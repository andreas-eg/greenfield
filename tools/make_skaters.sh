#!/bin/sh
# The skaters: the band's and three CharGen bodies (Söder's rider-s4 packages, native rigs,
# not in git) with Beargarden's skate clips (art/actionforge, authored in ActionForge by
# tools/actionforge.py from tools/skate_clips.json) retargeted onto each, into
# world/scenes/skaters/ where the catalogue's skater_* bodies point.
#
#   SKATER_SOURCES=<soder>/world/scenes/rider-s4-20261030 tools/make_skaters.sh
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
src=${SKATER_SOURCES:-$here/../rig-wt/soder-main/world/scenes/rider-s4-20261030}
out=$here/world/scenes/skaters
mkdir -p "$out"
for body in hugo erik zikai npc06 npc09 npc20; do
    python3 "$here/tools/retarget_skate.py" "$src/$body.glb" "$out/$body.glb" "$here/art/actionforge" "$here/tools/skate_clips.json" | tail -1
done
