#!/bin/sh
# play.sh: Beargarden sk8r on this Mac, in one go. Starts the world (mull with this realm) if
# nothing listens on its port, builds the app (app/: the lens with Beargarden's game modes) if it
# is not built, makes the folder it runs from, and runs it. Quitting it stops the world this
# script started.
#
#   tools/play.sh                 play
#   LENS_HUD_LIVE=1 tools/play.sh edit world/hud/*.json and see it within a second
#   REBUILD=1 tools/play.sh       build the app again first
#
# Where things are (defaults for this workspace; override any):
#   ENGINE   a mull checkout          ~/src/eg/rig-wt/engine-main, else repos/small-forest/repos/engine
#   LENS     a lens checkout, for its catalogue and fixtures   ~/src/eg/lens
#   TARGET   the app's cargo target   ~/src/eg/rig-wt/cargo-target/beargarden
#   PORT     the world's telnet port  4061 (its web port is PORT + 1)
set -e
HERE="$(cd "$(dirname "$0")/.." && pwd)"
if [ -z "$ENGINE" ]; then
  ENGINE="$HOME/src/eg/rig-wt/engine-main"
  [ -f "$ENGINE/mix.exs" ] || ENGINE="$HOME/src/eg/repos/small-forest/repos/engine"
fi
LENS="${LENS:-$HOME/src/eg/lens}"
TARGET="${TARGET:-$HOME/src/eg/rig-wt/cargo-target/beargarden}"
PORT="${PORT:-4061}"
WEB=$((PORT + 1))
RUN="$HOME/.beargarden/lens-run"

# The app: the lens (a git dependency) with sk8 registered, and this realm compiled in as its
# charts' home.
BIN="$TARGET/release/beargarden"
if [ ! -x "$BIN" ] || [ -n "$REBUILD" ]; then
  echo "building the app (a few minutes the first time)"
  (cd "$HERE/app" && CARGO_TARGET_DIR="$TARGET" LENS_REALM="$HERE/world" cargo build --release)
fi

# The skaters' bodies, made once from the band's and CharGen's (tools/make_skaters.sh).
[ -d "$HERE/world/scenes/skaters" ] || echo "note: no skaters yet (world/scenes/skaters): run tools/make_skaters.sh"

# The folder the lens runs from: the lens's own meshes, and Beargarden's spots beside them.
mkdir -p "$RUN/conditioned"
for f in "$HOME/src/eg/lens/conditioned/"*; do
  [ -e "$f" ] && ln -sfn "$f" "$RUN/conditioned/$(basename "$f")"
done
ln -sfn "$HERE/conditioned/beargarden" "$RUN/conditioned/beargarden"
ln -sfn "$LENS/catalogue.json" "$RUN/catalogue.json"
ln -sfn "$LENS/fixtures" "$RUN/fixtures"

# The world, unless one already listens.
WORLD=""
if ! lsof -tiTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  echo "starting the world on $PORT (log: $RUN/world.log)"
  (cd "$ENGINE" && PORT="$PORT" WEB_PORT="$WEB" elixir --no-halt -S mix run --no-start "$HERE/run-realm.exs" > "$RUN/world.log" 2>&1) &
  for i in $(seq 1 90); do lsof -tiTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1 && break; sleep 1; done
  WORLD=$(lsof -tiTCP:"$PORT" -sTCP:LISTEN)
  trap '[ -n "$WORLD" ] && kill "$WORLD" 2>/dev/null' EXIT
fi

cd "$RUN"
LENS_LOOK=folder LENS_ADDR="127.0.0.1:$PORT" LENS_WEB="127.0.0.1:$WEB" "$BIN"
