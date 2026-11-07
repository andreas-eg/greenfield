#!/usr/bin/env python3
"""Draw a GLB's skeleton as a stick figure, front, side and top, at a few times of a clip:
a quick look at a pose without the lens.

    python3 tools/stick.py clip.glb out.png [anim] [times...]
"""
import sys
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from gltfpose import Rig


def main():
    r = Rig(sys.argv[1])
    anim = sys.argv[3] if len(sys.argv) > 3 else next(iter(r.anims), None)
    dur = r.duration(anim) if anim else 0
    times = [float(t) for t in sys.argv[4:]] or [0, dur / 2, dur]
    fig, axes = plt.subplots(len(times), 3, figsize=(9, 3 * len(times)))
    if len(times) == 1:
        axes = [axes]
    for row, t in zip(axes, times):
        w = r.world(anim, t)
        for ax, (a, b, title) in zip(row, ((0, 1, "front (x,y)"), (2, 1, "side (z,y)"), (0, 2, "top (x,z)"))):
            for i, n in enumerate(r.nodes):
                p = r.parent.get(i)
                name = n.get("name", "")
                if p is None or "Hand" in name and any(f in name for f in ("Index", "Middle", "Ring", "Pinky", "Thumb")):
                    continue
                pn = r.nodes[p].get("name", "")
                if pn not in w or name not in w:
                    continue
                x0, x1 = w[pn][0], w[name][0]
                col = "tab:red" if "Left" in name else "tab:blue" if "Right" in name else "k"
                ax.plot([x0[a], x1[a]], [x0[b], x1[b]], color=col, lw=2)
            ax.axhline(0, color="0.8", lw=0.5)
            ax.set_aspect("equal")
            ax.set_xlim(-1, 1)
            ax.set_ylim(-0.6 if b == 2 else -0.1, 0.6 if b == 2 else 1.9)
            ax.set_title(f"{title} t={t:.2f}", fontsize=8)
    plt.tight_layout()
    plt.savefig(sys.argv[2], dpi=70)


main()
