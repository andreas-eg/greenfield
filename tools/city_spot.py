#!/usr/bin/env python3
"""A box of the real island as an sk8 spot: the city spike (plan Phase 3, SODER-LENS-S272).

Reads what GeoGen made of Södermalm (an island folder: urban/ meshes, charts/ records) and
the realm's generated LPC tables (skate parks, benches), and writes an sk8 SpotFile, which
`cargo run --bin spots` turns into the lens's chart, mesh and walkmask like any spot.

    python3 tools/city_spot.py ~/.soder/islands/soder-67 ~/src/eg/soder/world sk8/spots/medborgarplatsen.json

What goes in, clipped to the box x 165..315, z 160..310 (Björns trädgård's bowls and
Medborgarplatsen), moved so the box's middle (240, 235) is the spot's origin:
- ground: the island's soder_ground_<surface>_<cx>_<cz>.glb chunks, the stairs and the skate
  park's bowls, each triangle kept when its middle is in the box; the surface by the mesh's
  name (asphalt, paving, concrete smooth; setts and yard, GeoGen's made ground, rough; gravel
  very slow; grass and woodland slow; rock and cut faces unrideable; stairs as stairs);
- buildings: soder.buildings.json rings in the box, from their ground to their top, unrideable;
- grindables: the park's coping (skateparks_soder.c) and the benches (seats_soder.c) as 1.8 m
  wooden ledges at seat height; soder.edges.json lines with a 0.3 to 1.2 m drop as ledges;
- a wall round the box, so a ride ends against it, never off the edge of the world.

The ground is drawn for looking, not skating (kerbs smoothed into ramps, seams between
surfaces): finding out how far that holds is the spike.
"""
import json
import math
import os
import re
import sys

import numpy as np
import trimesh

BOX = (165.0, 160.0, 315.0, 310.0)
ORIGIN = (240.0, 235.0)
SURFACE = {
    "asphalt": "smooth", "paving": "smooth", "concrete": "smooth",
    "setts": "rough", "cobbles": "rough",
    "gravel": "veryslow",
    # GeoGen's "yard" is made ground nothing else named (courtyards, an untagged square such as
    # Medborgarplatsen): hard, uneven, rideable. Grass and woods are not.
    "yard": "rough",
    "grass": "slow", "woodland": "slow",
    "rock": "unrideable", "face_cut": "unrideable",
    "stair": "stairs",
}


def inside(x, z, pad=0.0):
    return BOX[0] - pad <= x <= BOX[2] + pad and BOX[1] - pad <= z <= BOX[3] + pad


def local(x, y, z):
    return [round(x - ORIGIN[0], 3), round(y, 3), round(z - ORIGIN[1], 3)]


def triangles(path):
    """A GLB's triangles in world space, as an (n, 3, 3) array."""
    mesh = trimesh.load(path, force="mesh")
    return mesh.vertices[mesh.faces]


class Meshes:
    """Triangles by surface, the vertices shared within each."""

    def __init__(self):
        self.by = {}

    def add(self, surface, tris):
        verts, faces, index = self.by.setdefault(surface, ([], [], {}))
        for t in tris:
            ids = []
            for v in t:
                key = (round(float(v[0]), 3), round(float(v[1]), 3), round(float(v[2]), 3))
                if key not in index:
                    index[key] = len(verts)
                    verts.append(local(*key))
                ids.append(index[key])
            if len(set(ids)) == 3:
                faces.append(ids)

    def quad(self, surface, a, b, c, d):
        """A wall quad a-b-c-d, both faces."""
        tris = np.array([[a, b, c], [a, c, d], [c, b, a], [d, c, a]], dtype=float)
        self.add(surface, tris)

    def spot_meshes(self):
        return [{"surface": s, "vertices": v, "triangles": f} for s, (v, f, _) in sorted(self.by.items()) if f]


def lpc_table(path, name, kind="float"):
    """A generated LPC function `float name(int i)` (nested ifs on i) as a Python function."""
    text = open(path).read()
    m = re.search(r"^" + kind + " " + name + r"\(int i\) \{\n(.*?)^\}", text, re.S | re.M)
    body = m.group(1)
    py = []
    for line in body.splitlines():
        indent = len(line) - len(line.lstrip())
        s = line.strip()
        if not s or s == "}":
            continue
        s = re.sub(r"^if \((.*)\) \{$", r"if \1:", s)
        s = re.sub(r"^if \((.*)\) return (.*);$", r"if \1: return \2", s)
        s = re.sub(r"^return (.*);$", r"return \1", s)
        py.append(" " * indent + s)
    src = "def f(i):\n" + "\n".join(py) + "\n"
    scope = {}
    exec(src, scope)
    return scope["f"]


def lpc_int(path, name):
    m = re.search(r"int " + name + r"\(\) \{ return (\d+); \}", open(path).read())
    return int(m.group(1))


def main():
    island, realm, out = sys.argv[1:4]
    urban = os.path.join(island, "urban")
    charts = os.path.join(island, "charts")
    meshes = Meshes()
    kept = {}

    # Ground: the chunks under the box, and the stairs and the park's bowls in it.
    files = []
    for f in sorted(os.listdir(urban)):
        m = re.match(r"soder_ground_(.+)_(\d+)_(\d+)\.glb$", f)
        if m and 26 <= int(m.group(2)) <= 27 and 11 <= int(m.group(3)) <= 12:
            files.append((SURFACE.get(m.group(1)), f))
        elif re.match(r"soder_ground_stair_\d+\.glb$", f):
            files.append(("stairs", f))
        elif f == "soder_skatepark_38327852.glb":
            files.append(("smooth", f))
    for surface, f in files:
        if surface is None:
            print("no surface for", f)
            continue
        tris = triangles(os.path.join(urban, f))
        mid = tris.mean(axis=1)
        keep = tris[(mid[:, 0] >= BOX[0]) & (mid[:, 0] <= BOX[2]) & (mid[:, 2] >= BOX[1]) & (mid[:, 2] <= BOX[3])]
        if len(keep):
            meshes.add(surface, keep)
            kept[surface] = kept.get(surface, 0) + len(keep)
    ground = trimesh.Trimesh(*_merged(meshes))
    floor = float(ground.bounds[0][1])
    top = float(ground.bounds[1][1])
    print("ground: %s, y %.1f..%.1f" % (kept, floor, top))

    # Holes: where a structure GeoGen draws as its own mesh stands (a wall's footprint, the
    # T-bana pavilion), the ground has none, and a ride fell through. A 1 m cell with no ground
    # is a solid block a metre over the ground round it.
    holes = plug_holes(ground, meshes)
    print("holes plugged: %d cells" % holes)

    # Buildings: their rings, base to top, unrideable.
    blds = json.load(open(os.path.join(charts, "soder.buildings.json")))["buildings"]
    n_bld = 0
    for b in blds:
        ring, lod = b.get("ring"), b.get("lod1") or {}
        if not ring or not any(inside(x, z) for x, z in ring) or "ground_z" not in lod:
            continue
        lo, hi = lod["ground_z"] - 1.0, lod.get("top_z", lod["ground_z"] + 10.0)
        for (x0, z0), (x1, z1) in zip(ring, ring[1:] + ring[:1]):
            meshes.quad("unrideable", [x0, lo, z0], [x1, lo, z1], [x1, hi, z1], [x0, hi, z0])
        n_bld += 1

    # The wall round the box.
    lo, hi = floor - 1.0, top + 3.0
    corners = [(BOX[0], BOX[1]), (BOX[2], BOX[1]), (BOX[2], BOX[3]), (BOX[0], BOX[3])]
    for (x0, z0), (x1, z1) in zip(corners, corners[1:] + corners[:1]):
        meshes.quad("unrideable", [x0, lo, z0], [x1, lo, z1], [x1, hi, z1], [x0, hi, z0])

    rails = []
    # The park's coping: each pool's lip at its rim's height, all round.
    parks = os.path.join(realm, "things", "skateparks_soder.c")
    lx, lz, rim = lpc_table(parks, "lip_x"), lpc_table(parks, "lip_z"), lpc_table(parks, "pool_rim")
    for p in range(lpc_int(parks, "pool_count")):
        first, count = int(lpc_table(parks, "pool_lip_first", "int")(p)), int(lpc_table(parks, "pool_lip_count", "int")(p))
        pts = [local(lx(i), rim(p), lz(i)) for i in range(first, first + count) if inside(lx(i), lz(i))]
        if len(pts) > 2:
            rails.append({"kind": "coping", "material": "metal", "closed": True, "points": pts})
    # The benches: 1.8 m wooden ledges at seat height on the ground under them.
    seats = os.path.join(realm, "things", "seats_soder.c")
    sx, sz, syaw = lpc_table(seats, "seat_x"), lpc_table(seats, "seat_z"), lpc_table(seats, "seat_yaw")
    n_bench = 0
    for i in range(lpc_int(seats, "seat_count")):
        x, z = sx(i), sz(i)
        if not inside(x, z, pad=-2.0):
            continue
        y = ground_at(ground, x, z)
        if y is None:
            continue
        yaw = syaw(i)
        dx, dz = -math.sin(yaw) * 0.9, math.cos(yaw) * 0.9
        rails.append({"kind": "ledge", "material": "wood", "closed": False,
                      "points": [local(x - dx, y + 0.45, z - dz), local(x + dx, y + 0.45, z + dz)]})
        n_bench += 1
    # Edges (terrace walls, retaining walls): drawn as their own objects, so the ground has a
    # gap where they stand; a wall from their foot to their top closes it (a ride from below
    # fell through the gap at Medborgarplatsen's terrace). One with a ledge's drop is a ledge
    # on top as well.
    n_edge = n_wall = 0
    for e in json.load(open(os.path.join(charts, "soder.edges.json")))["edges"]:
        drop = e["top"] - e["foot"]
        pts = [p for p in e["line"] if inside(p[0], p[1])]
        if len(pts) < 2:
            continue
        lo, hi = e["foot"] - 1.0, e["top"] + 0.05
        for (x0, z0), (x1, z1) in zip(pts, pts[1:]):
            meshes.quad("unrideable", [x0, lo, z0], [x1, lo, z1], [x1, hi, z1], [x0, hi, z0])
        n_wall += 1
        if 0.3 <= drop <= 1.2:
            rails.append({"kind": "ledge", "material": "stone", "closed": False, "points": [local(p[0], e["top"], p[1]) for p in pts]})
            n_edge += 1

    # The start: by the park's board (GeoGen's skate spot), facing the way it does (lens yaw
    # -pi/2 is north, sk8's pi), on the first smooth ground ahead of it: the board's own spot is
    # on the grass, where a push does not get going.
    smooth = trimesh.Trimesh(*_merged_only(meshes, "smooth"))
    park_x, park_z = 243.0, 239.5
    for step in range(0, 21):
        z = park_z - step * 0.25
        top = ground_at(ground, park_x, z)
        if top is not None and abs((ground_at(smooth, park_x, z) or -99.0) - top) < 0.01:
            park_z = z + 0.0
            break
    spawn_y = ground_at(ground, park_x, park_z) or floor
    print("start: (%.2f, %.2f, %.2f) local (%.2f, %.2f)" % (park_x, spawn_y, park_z, park_x - ORIGIN[0], park_z - ORIGIN[1]))
    marks = []
    for name, (x, z, heading) in {"medborgarplatsen": (172.0, 205.0, math.pi / 2)}.items():
        y = ground_at(ground, x, z)
        if y is not None:
            marks.append({"name": name, "pos": local(x, y, z), "heading": heading})
    spot = {
        "name": "medborgarplatsen",
        "title": "Medborgarplatsen and Björns trädgård",
        "spawn": local(park_x, spawn_y, park_z),
        "spawn_heading": math.pi,
        "meshes": meshes.spot_meshes(),
        "rails": rails,
        "marks": marks,
    }
    os.makedirs(os.path.dirname(out) or ".", exist_ok=True)
    json.dump(spot, open(out, "w"))
    tris = sum(len(m["triangles"]) for m in spot["meshes"])
    print("%s: %d triangles, %d buildings, coping %d, %d benches, %d edge walls (%d ledges), %d marks" % (
        out, tris, n_bld, sum(r["kind"] == "coping" for r in rails), n_bench, n_wall, n_edge, len(marks)))


def plug_holes(ground, meshes):
    """Every 1 m cell of the box the ground does not cover, as an unrideable block."""
    t = ground.vertices[ground.faces]
    lo, hi = t.min(axis=1), t.max(axis=1)
    n = 0
    xs = np.arange(BOX[0] + 0.5, BOX[2], 1.0)
    zs = np.arange(BOX[1] + 0.5, BOX[3], 1.0)
    covered = np.zeros((len(xs), len(zs)), dtype=bool)
    tops = np.full((len(xs), len(zs)), np.nan)
    for i, x in enumerate(xs):
        near = (lo[:, 0] <= x) & (hi[:, 0] >= x)
        sub = trimesh.Trimesh(ground.vertices, ground.faces[near], process=False) if near.any() else None
        for j, z in enumerate(zs):
            y = ground_at(sub, x, z) if sub is not None else None
            if y is not None:
                covered[i, j] = True
                tops[i, j] = y
    for i, x in enumerate(xs):
        for j, z in enumerate(zs):
            if covered[i, j]:
                continue
            around = tops[max(0, i - 2):i + 3, max(0, j - 2):j + 3]
            top = (np.nanmax(around) if np.isfinite(around).any() else float(lo[:, 1].min())) + 1.0
            base = float(lo[:, 1].min()) - 1.0
            x0, x1, z0, z1 = x - 0.5, x + 0.5, z - 0.5, z + 0.5
            for (a0, b0), (a1, b1) in [((x0, z0), (x1, z0)), ((x1, z0), (x1, z1)), ((x1, z1), (x0, z1)), ((x0, z1), (x0, z0))]:
                meshes.quad("unrideable", [a0, base, b0], [a1, base, b1], [a1, top, b1], [a0, top, b0])
            meshes.quad("unrideable", [x0, top, z0], [x1, top, z0], [x1, top, z1], [x0, top, z1])
            n += 1
    return n


def _merged_only(meshes, surface):
    v, f, _ = meshes.by[surface]
    return np.array([[a[0] + ORIGIN[0], a[1], a[2] + ORIGIN[1]] for a in v]), np.array(f)


def _merged(meshes):
    verts, faces = [], []
    for s, (v, f, _) in meshes.by.items():
        base = len(verts)
        verts.extend([[a[0] + ORIGIN[0], a[1], a[2] + ORIGIN[1]] for a in v])
        faces.extend([[a + base, b + base, c + base] for a, b, c in f])
    return np.array(verts), np.array(faces)


def ground_at(ground, x, z):
    """The highest ground under x, z (world), or None: the triangles whose plan holds the
    point, at their height there."""
    t = ground.vertices[ground.faces]
    a, b, c = t[:, 0], t[:, 1], t[:, 2]
    v0, v1 = (b - a)[:, [0, 2]], (c - a)[:, [0, 2]]
    v2 = np.array([x, z]) - a[:, [0, 2]]
    den = v0[:, 0] * v1[:, 1] - v1[:, 0] * v0[:, 1]
    ok = np.abs(den) > 1e-9
    den = np.where(ok, den, 1.0)
    u = (v2[:, 0] * v1[:, 1] - v1[:, 0] * v2[:, 1]) / den
    v = (v0[:, 0] * v2[:, 1] - v2[:, 0] * v0[:, 1]) / den
    hit = ok & (u >= 0) & (v >= 0) & (u + v <= 1)
    if not hit.any():
        return None
    y = a[:, 1] + u * (b - a)[:, 1] + v * (c - a)[:, 1]
    return float(y[hit].max())


if __name__ == "__main__":
    main()
