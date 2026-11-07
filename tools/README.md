# tools

How Greenfield's skaters are made: the band's and CharGen's bodies with skate animations
authored in [ActionForge](https://actionforge.app).

| File | What it does |
|---|---|
| `skate_clips.py` | The 13 skate clips as keyed poses: ride, crouch, push, ollie, air, grab, land, grind, manual, powerslide, bail, handplant, wallride. Writes `skate_clips.json`. |
| `actionforge.py` | Makes each clip in ActionForge (New animation, from the rest pose), keys its poses in Animate > Transform, and downloads it (GLB, Mixamo skeleton, no mesh) into `art/actionforge/`. It drives a Chrome over the DevTools protocol. |
| `retarget_skate.py` | Puts the ActionForge clips onto one of our bodies, as new animations in a copy of that body's GLB. |
| `make_skaters.sh` | Runs the retarget for all six skaters (Hugo, Erik, Zikai, NPC 06, 09 and 20) into `world/scenes/skaters/`. That folder is not in git, because the source bodies are not. |
| `gltfpose.py`, `stick.py` | Read a GLB's joints at a time in a clip, and draw them as a stick figure. They check a pose without the lens. |

## Making the clips again

```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new \
  --remote-debugging-port=9333 --user-data-dir=/tmp/af-profile https://actionforge.app/ &
python3 tools/skate_clips.py > tools/skate_clips.json
python3 tools/actionforge.py tools/skate_clips.json art/actionforge          # or name some clips
SKATER_SOURCES=<soder>/world/scenes/rider-s4-20261030 tools/make_skaters.sh
```

Nothing is uploaded: ActionForge runs in the browser. Its library animations are CC0, and
these clips are our own poses.

## ActionForge's Transform panel, as measured

Each probe keyed one bone at 20° on one axis and read the exported GLB back.
- **Spine and neck:** X leans forward, Y turns toward +X, Z tilts toward -X.
- **Legs:** X swings the knee back when positive (so negative is forward), Y and Z are mirrored. A negative Z on `thigh_l` takes the left foot out to +X.
- **Arms:** Z raises and lowers the arm (negative lowers the left arm, positive the right). Y swings it forward and back. X bends the forearm forward.
- **Pelvis location:** in cm, with Z up.
- **Rig:** the character faces +Z with its left at +X. It exports in a T-pose.

## The retarget

Our bodies use Mixamo joint names (CharGen's have five spine joints and an extra neck
joint) and stand in an A-pose. For each joint, the retarget:
1. takes the source joint's world rotation away from its rest;
2. applies it after the turn that lines the target's rest bone up with the source's, so the A-pose arms follow the T-pose ones;
3. applies a quarter turn about up, so the board's nose (+X in ActionForge) becomes the body's forward (+Z).

The hips follow the source's, scaled by hip height. Each frame the anchor foot (or both
feet) is stood on the deck at the body's own rest foot height. That plants the feet on the
board for every body, whatever its leg length.
