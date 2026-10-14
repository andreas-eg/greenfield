# greenfield

A realm of its own, beside Söder (eldergrounds/hugo), for what does not
belong in Söder. First the walking castle (SODER-WORLD-S72): a castle on four
legs walking a gravel ring round a green field, with a yard inside, entered
through its gate and left over its drawbridge.

- `world/field.c` - the field, on the `greenfield` chart (geogen/model/greenfield.py).
- `world/castle/walking_castle.c` - the yard inside, on the `walking_castle` chart
  (geogen/model/walking_castle.py); at boot it makes the castle and sends it out.
- `world/vehicles/walking_castle.c` - the castle outside; `world/things/castle_gate.c` its gate.
- `world/look/` - what the lens draws: the catalogue and the lock, files by
  their hash in Studio (the SODER project's assets for now).

Run it with a driver release (`lens/mac/build-driver.sh`):

    SPAWN=field CHART_PATH=world/charts <release>/world . 4041 4001

Written in LPC for mull, the driver; the efun reference is `/efuns` on a
running world. Tracked in Studio as the GREENFIELD project.
