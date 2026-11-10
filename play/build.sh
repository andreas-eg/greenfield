#!/bin/sh
# Build the sk8 simulation for the browser and copy the spots beside it.
#   ./build.sh            then   python3 -m http.server 8088   and open http://localhost:8088
# A gamepad needs a secure context: localhost is one; another device needs HTTPS.
set -e
here=$(cd "$(dirname "$0")" && pwd)
# sk8 is the lens's (crates/sk8, SODER-WORLD-S167): the lens checkout beside this one, or LENS.
sk8=${LENS:-$here/../../lens}/crates/sk8
target=${CARGO_TARGET_DIR:-$sk8/../../target}
(cd "$sk8" && cargo build --release --target wasm32-unknown-unknown --lib)
cp "$target/wasm32-unknown-unknown/release/sk8.wasm" "$here/sk8.wasm"
mkdir -p "$here/spots"
cp "$here"/../world/charts/*.skate.json "$here/spots/"
echo "built $(du -h "$here/sk8.wasm" | cut -f1) sk8.wasm and $(ls "$here/spots" | wc -l | tr -d ' ') spots"
