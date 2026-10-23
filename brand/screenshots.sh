#!/usr/bin/env bash
# Fork: screenshots of the running app on macOS — the whole screen, so the system menu bar, the
# traffic lights and open menus are in them (the app's own ui_shot draws only the window's
# content). Runs on a Mac with a logged-in session: a GitHub macOS runner (workflow
# screenshots.yml) or your own Mac.
#
#   brand/screenshots.sh "<path to Name.app>" <out dir>
#
# The app runs with its control channel (127.0.0.1:7981) to change what's on screen; menus are
# opened with System Events, which needs Accessibility access for the shell (granted on GitHub's
# runners; on your Mac System Settings › Privacy & Security asks once).
set -uo pipefail
APP="$1"
OUT="$2"
PORT=7981
mkdir -p "$OUT"
NAME="$(basename "$APP" .app)"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=brand.sh
. "$ROOT/brand/brand.sh"
KEYFILE="$HOME/Library/Application Support/$BRAND_DATA_DIR/control-key.local-$PORT"

shot() {
  sleep "${2:-1.5}"
  screencapture -x "$OUT/$1.png" && echo "shot $1" || echo "screencapture failed for $1"
}

# One control-channel request: method and params as JSON.
send() {
  python3 - "$PORT" "$KEYFILE" "$1" "$2" <<'EOF'
import json, socket, sys
port, keyfile, method, params = int(sys.argv[1]), sys.argv[2], sys.argv[3], json.loads(sys.argv[4])
key = open(keyfile).read().strip()
s = socket.create_connection(("127.0.0.1", port), timeout=10)
s.sendall((json.dumps({"id": 1, "key": key, "method": method, "params": params}) + "\n").encode())
print(s.makefile().readline().strip()[:200])
EOF
}

run() {
  local params="${2:-}"
  [ -n "$params" ] || params='{}'
  send engine.execute "{\"command\": \"$1\", \"params\": $params}"
}

menu() {
  osascript -e "tell application \"System Events\" to tell process \"$NAME\" to click menu bar item \"$1\" of menu bar 1" >/dev/null 2>&1 \
    || { echo "couldn't open the $1 menu (no Accessibility access?)"; return 1; }
}

close_menu() { osascript -e 'tell application "System Events" to key code 53' >/dev/null 2>&1 || true; }

open -n "$APP" --args --sample --control "$PORT"
for _ in $(seq 1 60); do [ -f "$KEYFILE" ] && break; sleep 1; done
sleep 6
osascript -e "tell application \"$NAME\" to activate" >/dev/null 2>&1 || true
shot 01-window 2

run ui.tab '{"tab": "Insert"}'
shot 02-insert-tab
run ui.tab '{"tab": "Home"}'
menu "Format" && shot 02-menu-format && close_menu
menu "View" && shot 03-menu-view && close_menu
menu "$NAME" && shot 04-menu-app && close_menu

run select.text '{"text": "Painters"}'
shot 05-selection

run ui.theme '{"value": "dark"}'
shot 06-dark 2
run ui.theme '{"value": "light"}'

run ui.language '{"value": "cs"}'
shot 07-czech 2
menu "Formát" && shot 08-menu-format-cs && close_menu
run ui.language '{"value": "en"}'

run ui.backstage '{"value": true, "page": "home"}'
shot 09-start 2
run ui.backstage '{"value": true, "page": "options"}'
shot 10-settings 2

osascript -e "tell application \"$NAME\" to quit" >/dev/null 2>&1 || pkill -x "$NAME" || true
ls -la "$OUT"
