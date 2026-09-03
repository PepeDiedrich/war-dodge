#!/data/data/com.termux/files/usr/bin/sh
set -eu
APP=${APP:-termux-poller}
STATE_DIR=${HOME}/.local/state/termux-poller
mkdir -p "$STATE_DIR"
echo "== release benchmark =="
cargo bench
echo "== one execution (elapsed/max RSS) =="
/data/data/com.termux/files/usr/bin/time -f 'elapsed=%e s max_rss=%M KB' "$APP" once 2>>"$STATE_DIR/profile.log"
tail -n 1 "$STATE_DIR/profile.log"
