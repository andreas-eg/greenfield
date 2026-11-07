#!/usr/bin/env python3
"""Read a GLB's node tree and animations; evaluate world joint transforms at a time.

Used by retarget_skate.py and to check the skate clips authored in ActionForge.
"""
import json, struct
import numpy as np


def load(path):
    b = open(path, "rb").read()
    assert b[:4] == b"glTF"
    off, j, bin_ = 12, None, b""
    while off < len(b):
        ln, kind = struct.unpack("<I4s", b[off:off + 8])
        chunk = b[off + 8:off + 8 + ln]
        if kind == b"JSON":
            j = json.loads(chunk)
        elif kind == b"BIN\x00":
            bin_ = chunk
        off += 8 + ln
    return j, bin_


COMP = {5126: ("f", 4), 5123: ("H", 2), 5121: ("B", 1), 5125: ("I", 4), 5122: ("h", 2), 5120: ("b", 1)}
NUM = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


def accessor(j, bin_, i):
    a = j["accessors"][i]
    bv = j["bufferViews"][a["bufferView"]]
    fmt, size = COMP[a["componentType"]]
    n = NUM[a["type"]]
    stride = bv.get("byteStride", size * n)
    start = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
    out = np.zeros((a["count"], n), dtype=np.float64)
    for k in range(a["count"]):
        out[k] = struct.unpack_from("<" + fmt * n, bin_, start + k * stride)
    if a.get("normalized"):
        out /= {5121: 255.0, 5123: 65535.0, 5120: 127.0, 5122: 32767.0}[a["componentType"]]
    return out


def qmul(a, b):
    x1, y1, z1, w1 = a
    x2, y2, z2, w2 = b
    return np.array([w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
                     w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
                     w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
                     w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2])


def qinv(q):
    return np.array([-q[0], -q[1], -q[2], q[3]]) / np.dot(q, q)


def qrot(q, v):
    p = qmul(qmul(q, np.array([v[0], v[1], v[2], 0.0])), qinv(q))
    return p[:3]


def slerp(a, b, t):
    d = np.dot(a, b)
    if d < 0:
        b, d = -b, -d
    if d > 0.9995:
        r = a + t * (b - a)
        return r / np.linalg.norm(r)
    th = np.arccos(d)
    return (np.sin((1 - t) * th) * a + np.sin(t * th) * b) / np.sin(th)


class Rig:
    def __init__(self, path):
        self.j, self.bin = load(path)
        self.nodes = self.j["nodes"]
        self.parent = {}
        for i, n in enumerate(self.nodes):
            for c in n.get("children", []):
                self.parent[c] = i
        self.byname = {n.get("name", f"node{i}"): i for i, n in enumerate(self.nodes)}
        self.anims = {}
        for a in self.j.get("animations", []):
            ch = {}
            for c in a["channels"]:
                s = a["samplers"][c["sampler"]]
                t = accessor(self.j, self.bin, s["input"])[:, 0]
                v = accessor(self.j, self.bin, s["output"])
                ch[(c["target"]["node"], c["target"]["path"])] = (t, v, s.get("interpolation", "LINEAR"))
            self.anims[a["name"]] = ch

    def rest(self, i):
        n = self.nodes[i]
        return (np.array(n.get("translation", [0, 0, 0]), float), np.array(n.get("rotation", [0, 0, 0, 1]), float),
                np.array(n.get("scale", [1, 1, 1]), float))

    def duration(self, anim):
        ch = self.anims[anim] if anim in self.anims else next(iter(self.anims.values()))
        return max(float(t[-1]) for t, _, _ in ch.values())

    def local(self, i, anim=None, time=0.0):
        t, r, s = self.rest(i)
        if anim:
            ch = self.anims[anim] if anim in self.anims else next(iter(self.anims.values()))
            for path in ("translation", "rotation", "scale"):
                if (i, path) not in ch:
                    continue
                ts, vs, interp = ch[(i, path)]
                if interp == "CUBICSPLINE":
                    vs = vs[1::3]
                if time <= ts[0]:
                    v = vs[0]
                elif time >= ts[-1]:
                    v = vs[-1]
                else:
                    k = int(np.searchsorted(ts, time)) - 1
                    f = (time - ts[k]) / (ts[k + 1] - ts[k])
                    if interp == "STEP":
                        v = vs[k]
                    elif path == "rotation":
                        v = slerp(vs[k], vs[k + 1], f)
                    else:
                        v = vs[k] + f * (vs[k + 1] - vs[k])
                if path == "translation":
                    t = np.array(v)
                elif path == "rotation":
                    r = np.array(v) / np.linalg.norm(v)
                else:
                    s = np.array(v)
        return t, r, s

    def world(self, anim=None, time=0.0):
        """name -> (position, rotation quaternion), scale folded into positions."""
        out = {}

        def go(i):
            if i in out:
                return out[i]
            t, r, s = self.local(i, anim, time)
            if i in self.parent:
                pp, pr, ps = go(self.parent[i])
                p = pp + qrot(pr, ps * t)
                q = qmul(pr, r)
                sc = ps * s
            else:
                p, q, sc = t, r, s
            out[i] = (p, q, sc)
            return out[i]

        for i in range(len(self.nodes)):
            go(i)
        return {self.nodes[i].get("name", f"node{i}"): (v[0], v[1]) for i, v in out.items()}


if __name__ == "__main__":
    import sys
    r = Rig(sys.argv[1])
    anim = sys.argv[2] if len(sys.argv) > 2 else None
    tm = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0
    w = r.world(anim, tm)
    for k in sys.argv[4:] or [n for n in w if any(x in n for x in ("Hips", "Foot", "Hand", "Head", "Leg", "Arm"))]:
        if k in w:
            print(f"{k:32s} {np.round(w[k][0], 3)}")
