#!/data/data/com.termux/files/usr/bin/sh
set -eu
APP=${APP:-war-dodger}
STATE_DIR=${HOME}/.local/state/war-dodger
TIME_BIN=/data/data/com.termux/files/usr/bin/time
mkdir -p "$STATE_DIR"
if [ ! -x "$TIME_BIN" ]; then
  echo "Install the profiler first: pkg install time" >&2
  exit 2
fi
echo "== release benchmark =="
cargo bench
echo "== one execution (elapsed/max RSS) =="
"$TIME_BIN" -f 'elapsed=%e s max_rss=%M KB' "$APP" once 2>>"$STATE_DIR/profile.log"
tail -n 1 "$STATE_DIR/profile.log"
