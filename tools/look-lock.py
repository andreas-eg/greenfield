#!/usr/bin/env python3
"""Beargarden's own entries in its lock (world/look/assets.lock): the spot meshes as
kit/beargarden/<file> and the skaters as realm/scenes/skaters/<file>, by the hash and size of the
files as published to Studio (lens tools/kit.py publish). Everything else the lock pins stays as it
is: this is not Söder's look-lock.py, which writes all of Söder's.

    python3 tools/look-lock.py write

Publish first, from a lens checkout:

    python3 <lens>/tools/kit.py publish conditioned/beargarden kit/beargarden
    python3 <lens>/tools/kit.py publish world/scenes/skaters realm/scenes/skaters

then `python3 <lens>/tools/kit.py check world/look/assets.lock` says Studio has every file.
@req SODER-LENS-S271
"""
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent.parent
LOCK = HERE / "world" / "look" / "assets.lock"
# What Beargarden makes itself: Studio's prefix, and the folder it is published from.
OWN = {
    "kit/beargarden/": HERE / "conditioned" / "beargarden",
    "realm/scenes/skaters/": HERE / "world" / "scenes" / "skaters",
}


def write():
    lock = json.loads(LOCK.read_text())
    assets = {k: v for k, v in lock["assets"].items() if not any(k.startswith(p) for p in OWN)}
    for prefix, folder in OWN.items():
        files = sorted(p for p in folder.rglob("*") if p.is_file() and not p.name.startswith("."))
        if not files:
            sys.exit(f"{folder}: nothing to lock (make it first: sk8's spot builder, tools/make_skaters.sh)")
        for p in files:
            data = p.read_bytes()
            assets[prefix + p.relative_to(folder).as_posix()] = {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
    lock["assets"] = dict(sorted(assets.items()))
    LOCK.write_text(json.dumps(lock, indent=1) + "\n")
    print(f"{LOCK.relative_to(HERE)}: {len(assets)} files")


if __name__ == "__main__":
    if sys.argv[1:] != ["write"]:
        sys.exit(__doc__)
    write()
