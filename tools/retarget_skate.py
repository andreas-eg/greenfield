#!/usr/bin/env python3
"""Put the skate clips authored in ActionForge (Mixamo skeleton, T-pose, nose at +X) onto one
of our bodies (the band's rider-s4 packages or a CharGen NPC: Mixamo-named joints, A-pose,
facing +Z), as new animations in a copy of that body's GLB.

Each joint takes the source joint's world rotation away from its rest, applied after a turn
that lines the target's rest bone up with the source's (so an A-pose arm follows a T-pose
arm), and a quarter turn about up, so the board's nose is the body's forward (+Z). The hips
follow the source's, scaled by hip height; then the hips are lowered or raised so the anchor
foot (or the lower foot) stands where the body's own rest foot stands: on the deck.

    python3 tools/retarget_skate.py target.glb out.glb clips_dir [skate_clips.json]
"""
import json, os, struct, sys
import numpy as np
from gltfpose import Rig, qmul, qinv, qrot, load

FPS = 30.0
YAW = np.array([0, np.sin(-np.pi / 4), 0, np.cos(-np.pi / 4)])  # -90 deg about +Y: +X -> +Z

# Source joint -> where its direction points (child) for the rest alignment.
AIM = {"Hips": "Spine", "Spine": "Spine1", "Spine1": "Spine2", "Spine2": "Neck", "Neck": "Head", "Head": "HeadTop_End",
       "LeftShoulder": "LeftArm", "LeftArm": "LeftForeArm", "LeftForeArm": "LeftHand", "LeftHand": "LeftHandMiddle1",
       "RightShoulder": "RightArm", "RightArm": "RightForeArm", "RightForeArm": "RightHand", "RightHand": "RightHandMiddle1",
       "LeftUpLeg": "LeftLeg", "LeftLeg": "LeftFoot", "LeftFoot": "LeftToeBase",
       "RightUpLeg": "RightLeg", "RightLeg": "RightFoot", "RightFoot": "RightToeBase"}


def short(name):
    return name.split(":")[-1]


def arc(a, b):
    """Shortest rotation taking direction a to direction b."""
    a, b = a / np.linalg.norm(a), b / np.linalg.norm(b)
    c = np.cross(a, b)
    d = float(np.dot(a, b))
    if d < -0.9999:
        ax = np.cross(a, [1, 0, 0]) if abs(a[0]) < 0.9 else np.cross(a, [0, 1, 0])
        ax /= np.linalg.norm(ax)
        return np.array([ax[0], ax[1], ax[2], 0.0])
    q = np.array([c[0], c[1], c[2], 1 + d])
    return q / np.linalg.norm(q)


def target_map(names):
    """Source short name -> target node name, for the chains both have."""
    by = {short(n): n for n in names}
    spines = sorted([s for s in by if s.startswith("Spine")], key=lambda s: (len(s), s))
    m = {}
    for s in AIM:
        if s in ("Spine", "Spine1", "Spine2"):
            continue
        if s in by:
            m[s] = by[s]
    # Three source spine joints onto however many the target has: first, middle, last.
    if spines:
        pick = [spines[0], spines[len(spines) // 2], spines[-1]] if len(spines) >= 3 else (spines + spines * 2)[:3]
        for s, t in zip(("Spine", "Spine1", "Spine2"), pick):
            m.setdefault(s, by[t])
    return m


def retarget(src, tgt, mapping, anchor, frames):
    """Per frame: {target node index: (translation or None, rotation)}."""
    s0 = src.world(None, 0)
    t0 = tgt.world(None, 0)
    sname = {short(n): n for n in s0}
    tidx = {n.get("name"): i for i, n in enumerate(tgt.nodes)}
    hip_s = s0[sname["Hips"]][0][1]
    hip_t = t0[mapping["Hips"]][0][1]
    k = hip_t / hip_s
    # Rest alignment per mapped joint: target rest dir -> source rest dir.
    align = {}
    for s, t in mapping.items():
        aim = AIM.get(s)
        ta = mapping.get(aim) if aim else None
        if aim and aim in sname and ta and ta in t0:
            ds = s0[sname[aim]][0] - s0[sname[s]][0]
            dt = t0[ta][0] - t0[t][0]
            align[s] = arc(dt, ds) if np.linalg.norm(ds) > 1e-4 and np.linalg.norm(dt) > 1e-4 else np.array([0, 0, 0, 1.0])
        else:
            align[s] = np.array([0, 0, 0, 1.0])
    rev = {t: s for s, t in mapping.items()}
    out = []
    dur = src.duration(None)
    for f in range(frames):
        tm = min(dur, f / FPS)
        sw = src.world("x", tm)
        world = {}
        local = {}

        def solve(i):
            if i in world:
                return world[i]
            n = tgt.nodes[i]
            name = n.get("name")
            pt, pr, ps = (np.zeros(3), np.array([0, 0, 0, 1.0]), np.ones(3))
            p = tgt.parent.get(i)
            if p is not None:
                pt, pr, ps = solve(p)
            lt, lr, ls = tgt.rest(i)
            if name in rev:
                s = rev[name]
                D = qmul(sw[sname[s]][1], qinv(s0[sname[s]][1]))
                W = qmul(YAW, qmul(D, qmul(align[s], t0[name][1])))
                lr = qmul(qinv(pr), W)
                lr /= np.linalg.norm(lr)
                if s == "Hips":
                    pos = qrot(YAW, t0[name][0] + (sw[sname["Hips"]][0] - s0[sname["Hips"]][0]) * k)
                    lt = qrot(qinv(pr), pos - pt) / ps
                local[i] = (lt if s == "Hips" else None, lr)
            wp = pt + qrot(pr, ps * lt)
            wr = qmul(pr, lr)
            world[i] = (wp, wr, ps * ls)
            return world[i]

        for i in range(len(tgt.nodes)):
            solve(i)
        # Stand the anchor foot where the body's rest foot stands.
        if anchor:
            feet = {"left": ["LeftFoot"], "right": ["RightFoot"], "both": ["LeftFoot", "RightFoot"]}[anchor]
            low = min(world[tidx[mapping[x]]][0][1] for x in feet)
            rest = min(t0[mapping[x]][0][1] for x in feet)
            dy = rest - low
            hi = tidx[mapping["Hips"]]
            p = tgt.parent.get(hi)
            pr = world[p][1] if p is not None else np.array([0, 0, 0, 1.0])
            ps = world[p][2] if p is not None else np.ones(3)
            lt, lr = local[hi]
            local[hi] = (lt + qrot(qinv(pr), np.array([0, dy, 0])) / ps, lr)
        out.append(local)
    return out


def append(j, bin_, data, kind, comp=5126):
    """Add an accessor for float data; return its index."""
    arr = np.asarray(data, dtype=np.float32)
    raw = arr.tobytes()
    off = len(bin_)
    pad = (-off) % 4
    bin_ += b"\x00" * pad
    off += pad
    bin_ += raw
    j.setdefault("bufferViews", []).append({"buffer": 0, "byteOffset": off, "byteLength": len(raw)})
    acc = {"bufferView": len(j["bufferViews"]) - 1, "componentType": comp, "count": len(arr),
           "type": {1: "SCALAR", 3: "VEC3", 4: "VEC4"}[arr.shape[1] if arr.ndim > 1 else 1]}
    if acc["type"] == "SCALAR":
        acc["min"], acc["max"] = [float(arr.min())], [float(arr.max())]
    j.setdefault("accessors", []).append(acc)
    return len(j["accessors"]) - 1, bin_


def save(path, j, bin_):
    bin_ += b"\x00" * ((-len(bin_)) % 4)
    j["buffers"][0]["byteLength"] = len(bin_)
    js = json.dumps(j, separators=(",", ":")).encode()
    js += b" " * ((-len(js)) % 4)
    total = 12 + 8 + len(js) + 8 + len(bin_)
    with open(path, "wb") as f:
        f.write(struct.pack("<4sII", b"glTF", 2, total))
        f.write(struct.pack("<I4s", len(js), b"JSON") + js)
        f.write(struct.pack("<I4s", len(bin_), b"BIN\x00") + bin_)


def main():
    target, out, clips_dir = sys.argv[1:4]
    spec = json.load(open(sys.argv[4] if len(sys.argv) > 4 else os.path.join(os.path.dirname(__file__), "skate_clips.json")))
    tgt = Rig(target)
    j, bin_ = load(target)
    j["animations"] = [a for a in j.get("animations", []) if not a["name"].startswith("skate_")]
    names = [n.get("name", "") for n in tgt.nodes]
    mapping = target_map(names)
    missing = [s for s in ("Hips", "LeftUpLeg", "LeftLeg", "LeftFoot", "RightFoot", "LeftArm", "Head") if s not in mapping]
    if missing:
        raise SystemExit(f"{target}: no {missing}")
    for clip in spec["clips"]:
        path = os.path.join(clips_dir, clip["name"] + ".glb")
        if not os.path.exists(path):
            print("  skip", clip["name"], "(not authored)")
            continue
        src = Rig(path)
        frames = clip["frames"]
        poses = retarget(src, tgt, mapping, clip.get("anchor"), frames)
        times = np.arange(frames, dtype=np.float32)[:, None] / FPS
        ti, bin_ = append(j, bin_, times, "SCALAR")
        anim = {"name": clip["name"], "samplers": [], "channels": []}
        for node in poses[0]:
            rots = [p[node][1] for p in poses]
            for i in range(1, len(rots)):
                if np.dot(rots[i], rots[i - 1]) < 0:
                    rots[i] = -rots[i]
            ri, bin_ = append(j, bin_, rots, "VEC4")
            anim["samplers"].append({"input": ti, "output": ri, "interpolation": "LINEAR"})
            anim["channels"].append({"sampler": len(anim["samplers"]) - 1, "target": {"node": node, "path": "rotation"}})
            if poses[0][node][0] is not None:
                li, bin_ = append(j, bin_, [p[node][0] for p in poses], "VEC3")
                anim["samplers"].append({"input": ti, "output": li, "interpolation": "LINEAR"})
                anim["channels"].append({"sampler": len(anim["samplers"]) - 1, "target": {"node": node, "path": "translation"}})
        j["animations"].append(anim)
        print(f"  {clip['name']}: {frames} frames, {len(poses[0])} joints")
    save(out, j, bin_)
    print(out, os.path.getsize(out))


if __name__ == "__main__":
    main()
