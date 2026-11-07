#!/usr/bin/env python3
"""Beargarden's skate clips as poses keyed in ActionForge (tools/actionforge.py reads the JSON
this writes). The pose is ActionForge's own rig (UE names) facing +Z with the board under the
feet along X, nose at +X (regular: left foot forward). retarget_skate.py turns it a quarter so
the nose is the body's forward.

Panel values, as ActionForge's Animate > Transform takes them (degrees; the pelvis's location
in cm, its Z up). Measured by probing (tools/README.md): legs, X forward/back (negative swings
the knee forward), Z sideways (negative takes the left foot out to +X); spine, X leans forward,
Y turns toward the nose; arms, Z raises and lowers (left negative is down), X bends the
forearm forward.

    python3 tools/skate_clips.py > tools/skate_clips.json
"""
import copy, json


def stance(drop=11, bend=27, spread=13, lean=6, twist=12, arms=68, elbows=28, head=28):
    """Feet across the board's bolts, knees over the toes, eyes down the nose."""
    knee = 2 * bend - 4
    return {
        "pelvis": {"loc": [0, 0, -drop], "rot": [0, twist * 0.6, 0]},
        "spine_01": {"rot": [lean, twist * 0.5, 0]},
        "spine_02": {"rot": [lean * 0.6, twist * 0.5, 0]},
        "spine_03": {"rot": [0, twist * 0.3, 0]},
        "neck_01": {"rot": [0, head * 0.4, 0]},
        "Head": {"rot": [-6, head * 0.6, 0]},
        "thigh_l": {"rot": [-bend, 0, -spread]},
        "calf_l": {"rot": [knee, 0, 0]},
        "foot_l": {"rot": [-(knee - bend) * 0.85, 0, spread * 0.8]},
        "thigh_r": {"rot": [-bend, 0, spread]},
        "calf_r": {"rot": [knee, 0, 0]},
        "foot_r": {"rot": [-(knee - bend) * 0.85, 0, -spread * 0.8]},
        "upperarm_l": {"rot": [0, 8, -arms]},
        "lowerarm_l": {"rot": [elbows, 0, 0]},
        "upperarm_r": {"rot": [0, -8, arms]},
        "lowerarm_r": {"rot": [elbows, 0, 0]},
    }


def with_(pose, **bones):
    p = copy.deepcopy(pose)
    for b, spec in bones.items():
        p.setdefault(b, {}).update(spec)
    return p


RIDE = stance()
LOW = stance(drop=16, bend=34, lean=10)
CROUCH = stance(drop=30, bend=52, spread=12, lean=24, arms=58, elbows=40, head=22)
TUCK = with_(stance(drop=8, bend=60, spread=10, lean=16, arms=40, elbows=35, head=24),
             thigh_l={"rot": [-62, 0, -10]}, thigh_r={"rot": [-62, 0, 10]},
             calf_l={"rot": [100, 0, 0]}, calf_r={"rot": [100, 0, 0]},
             pelvis={"loc": [0, 0, 6], "rot": [0, 8, 0]})
POP = with_(CROUCH, thigh_r={"rot": [-6, 0, 10]}, calf_r={"rot": [8, 0, 0]}, foot_r={"rot": [30, 0, -8]},
            thigh_l={"rot": [-58, 0, -12]}, calf_l={"rot": [80, 0, 0]}, pelvis={"loc": [0, 0, 2], "rot": [0, 8, 0]},
            upperarm_l={"rot": [0, 8, -35]}, upperarm_r={"rot": [0, -8, 35]})
# Indy: the back hand to the toe edge between the feet.
GRAB = with_(TUCK, spine_01={"rot": [26, 6, 0]}, spine_02={"rot": [18, 6, 0]},
             upperarm_r={"rot": [-40, -55, 72]}, lowerarm_r={"rot": [10, 0, 0]},
             upperarm_l={"rot": [0, 30, -15]}, lowerarm_l={"rot": [20, 0, 0]})
GRIND = with_(stance(drop=14, bend=31, lean=8, arms=30, elbows=18, head=34),
              upperarm_l={"rot": [0, 15, -30]}, upperarm_r={"rot": [0, -15, 30]})
GRIND_SWAY = with_(GRIND, spine_01={"rot": [8, 6, 6]}, upperarm_l={"rot": [0, 15, -22]}, upperarm_r={"rot": [0, -15, 40]})
# Manual: the weight over the back foot (-X), the front leg long, arms out.
MANUAL = with_(stance(drop=12, bend=22, lean=2, arms=35, elbows=20, head=30),
               pelvis={"loc": [-9, 0, -12], "rot": [0, 6, 0]},
               spine_01={"rot": [2, 6, 12]}, spine_02={"rot": [0, 4, 8]},
               thigh_l={"rot": [-12, 0, -24]}, calf_l={"rot": [14, 0, 0]}, foot_l={"rot": [-2, 0, 18]},
               thigh_r={"rot": [-40, 0, 2]}, calf_r={"rot": [76, 0, 0]}, foot_r={"rot": [-30, 0, 0]})
# Powerslide: sat back on the heels (-Z), arms forward to the toe side.
SLIDE = with_(stance(drop=26, bend=40, spread=14, lean=-4, arms=40, elbows=30, head=10),
              pelvis={"loc": [0, -12, -26], "rot": [0, 30, 0]},
              spine_01={"rot": [16, 14, 0]}, spine_02={"rot": [10, 10, 0]},
              upperarm_l={"rot": [-30, 30, -40]}, upperarm_r={"rot": [30, -30, 40]})
# Push: hips square to the nose, the front leg holds the board, the back foot goes to the
# ground on the toe side (+Z), ahead, and drives back past the tail.
PUSH0 = with_(stance(drop=14, bend=24, spread=6, lean=10, twist=55, head=20),
              thigh_l={"rot": [-24, 0, -6]}, foot_l={"rot": [-14, 0, 0]})
PUSH_REACH = with_(PUSH0, thigh_r={"rot": [-46, 0, -6]}, calf_r={"rot": [40, 0, 0]}, foot_r={"rot": [0, 0, 0]},
                   pelvis={"loc": [0, 0, -18], "rot": [0, 34, 0]})
PUSH_PLANT = with_(PUSH0, thigh_r={"rot": [-14, 0, -12]}, calf_r={"rot": [6, 0, 0]}, foot_r={"rot": [6, 0, 0]},
                   pelvis={"loc": [0, 0, -20], "rot": [0, 34, 0]})
PUSH_DRIVE = with_(PUSH0, thigh_r={"rot": [26, 0, 4]}, calf_r={"rot": [24, 0, 0]}, foot_r={"rot": [30, 0, 0]},
                   pelvis={"loc": [0, 0, -16], "rot": [0, 34, 0]})
BAIL_A = with_(CROUCH, pelvis={"loc": [0, -10, -30], "rot": [-35, 10, 15]},
               upperarm_l={"rot": [0, 40, 10]}, upperarm_r={"rot": [0, -40, -10]},
               thigh_l={"rot": [-70, 0, -20]}, thigh_r={"rot": [-40, 0, 20]})
BAIL_B = with_(BAIL_A, pelvis={"loc": [0, -30, -60], "rot": [-70, 10, 20]},
               spine_01={"rot": [-10, 0, 10]}, Head={"rot": [-20, 0, 0]},
               upperarm_l={"rot": [0, 70, 30]}, upperarm_r={"rot": [0, -70, -30]},
               thigh_l={"rot": [-90, 0, -25]}, thigh_r={"rot": [-20, 0, 25]}, calf_l={"rot": [30, 0, 0]})
HANDPLANT = with_(TUCK, upperarm_l={"rot": [0, -60, -80]}, lowerarm_l={"rot": [0, 0, 0]},
                  upperarm_r={"rot": [-40, -55, 72]}, spine_01={"rot": [30, 0, 0]})
WALLRIDE = with_(GRIND, spine_01={"rot": [4, 8, -14]}, upperarm_l={"rot": [0, 10, -10]}, upperarm_r={"rot": [0, -10, 55]})


def clip(name, frames, *keys, anchor=None):
    """`anchor`: the foot (left, right, both) the retarget stands on the deck each frame; none in the air."""
    return {"name": name, "frames": frames, "anchor": anchor, "keys": [{"frame": f, "pose": p} for f, p in keys]}


CLIPS = [
    clip("skate_ride", 60, (1, RIDE), (31, LOW), (60, RIDE), anchor="both"),
    clip("skate_crouch", 20, (1, CROUCH), (20, CROUCH), anchor="both"),
    clip("skate_push", 36, (1, PUSH0), (9, PUSH_REACH), (16, PUSH_PLANT), (27, PUSH_DRIVE), (36, PUSH0), anchor="left"),
    clip("skate_ollie", 14, (1, CROUCH), (4, POP), (14, TUCK), anchor="both"),
    clip("skate_air", 20, (1, TUCK), (20, TUCK), anchor="both"),
    clip("skate_grab", 20, (1, TUCK), (6, GRAB), (20, GRAB), anchor="both"),
    clip("skate_land", 14, (1, CROUCH), (14, RIDE), anchor="both"),
    clip("skate_grind", 40, (1, GRIND), (21, GRIND_SWAY), (40, GRIND), anchor="both"),
    clip("skate_manual", 30, (1, MANUAL), (30, MANUAL), anchor="right"),
    clip("skate_powerslide", 30, (1, SLIDE), (30, SLIDE), anchor="both"),
    clip("skate_bail", 30, (1, CROUCH), (10, BAIL_A), (30, BAIL_B)),
    clip("skate_handplant", 20, (1, HANDPLANT), (20, HANDPLANT), anchor="both"),
    clip("skate_wallride", 20, (1, WALLRIDE), (20, WALLRIDE), anchor="both"),
]

if __name__ == "__main__":
    print(json.dumps({"clips": CLIPS}, indent=1))
