#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Types into a GTK text box through a real ibus-daemon with the Likhon engine, on a virtual display.
#   ./tests/headless-typing.sh       (run after ./build.sh)
#   LIKHON_INSTALLED=1 ./tests/headless-typing.sh      tests the engine installed from the .deb or .rpm instead
# No admin rights and nothing in your home folder: the settings and the words database live in a temporary folder.
set -uo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
out="$here/out"
work="$(mktemp -d)"
trap 'kill $(jobs -p) 2>/dev/null || true; rm -rf "$work"' EXIT

# The engine reads ~/.config/likhon/settings.json and writes ~/.local/share/likhon/user.sqlite; point both here.
export XDG_CONFIG_HOME="$work/config" XDG_DATA_HOME="$work/data"
mkdir -p "$XDG_CONFIG_HOME/likhon" "$XDG_DATA_HOME"
# The first-run agreement (likhon-agreement) writes this file when the person accepts; the test plays that part.
printf '2026-11-21' > "$XDG_CONFIG_HOME/likhon/agreement"

export DISPLAY=:97
Xvfb :97 -screen 0 800x600x24 >/dev/null 2>&1 &
sleep 1
eval "$(dbus-launch --sh-syntax)"
if [ "${LIKHON_INSTALLED:-0}" != 1 ]; then
  export IBUS_COMPONENT_PATH="$out"
  export LIKHON_DICTIONARY="$here/../Source/assets/dict/mpl/words.sqlite"
fi
export GTK_IM_MODULE=ibus XMODIFIERS=@im=ibus QT_IM_MODULE=ibus
# Fedora does not build ibus-memconf: use it when the host provides it, and run without a config module otherwise.
if [ -x /usr/libexec/ibus-memconf ]; then
  config=(--config /usr/libexec/ibus-memconf)
else
  config=(--config disable)
fi
ibus-daemon --daemonize --replace --xim --panel disable --cache none "${config[@]}" >"$work/ibus.log" 2>&1
sleep 2
ibus engine likhon
echo "engine: $(ibus engine)"

norm() { python3 -c 'import sys,unicodedata; print(unicodedata.normalize("NFC", sys.argv[1]), end="")' "$1"; }
fail=0
check() { # name, got, want
  if [ "$(norm "$2")" = "$(norm "$3")" ]; then echo "PASS  $1"; else echo "FAIL  $1: got [$2] wanted [$3]"; fail=1; fi
}

# A new text box is a new input context, so the engine starts fresh and reads the settings file again.
entry_pid=""
start_entry() { # extra arguments for typing-entry.py
  python3 "$here/tests/typing-entry.py" "$work/text.txt" 40 "$@" &
  entry_pid=$!
  sleep 3
  xdotool search --name likhon-test windowfocus 2>/dev/null || true
  sleep 1
}
stop_entry() { kill "$entry_pid" 2>/dev/null; wait "$entry_pid" 2>/dev/null; entry_pid=""; }
typed() { xdotool type --delay 120 "$1"; sleep 1; cat "$work/text.txt"; }
learned() { python3 -c 'import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(",".join(r[0] for r in c.execute("select key from learned")))' "$XDG_DATA_HOME/likhon/user.sqlite" 2>/dev/null; }

# 1. Defaults: phonetic, suggestions, autocorrect, learning.
start_entry
check "phonetic sentence" "$(typed "ami banglay gan gai ")" "আমি বাংলায় গান গাই "
xdotool key ctrl+a BackSpace; sleep 0.5
check "dictionary autocorrect" "$(typed "account ")" "অ্যাকাউন্ট "
xdotool key ctrl+a BackSpace; sleep 0.5
xdotool type --delay 120 "bangla"; sleep 0.5; xdotool key Down; sleep 0.5; xdotool key space; sleep 1
got="$(cat "$work/text.txt")"
if [ -n "$got" ] && [ "$(norm "$got")" != "$(norm "বাংলা ")" ]; then echo "PASS  Down then Space picks another row: [$got]"; else echo "FAIL  Down then Space: [$got]"; fail=1; fi
xdotool key ctrl+a BackSpace; sleep 0.5
typed "zzqxz " >/dev/null
stop_entry
if [ -n "$(learned)" ]; then echo "PASS  a finished word is learned: [$(learned)]"; else echo "FAIL  nothing learned"; fail=1; fi

# 2. Settings file: colon stays a colon, Bangla digits on.
printf '{"typing.visarga": false, "typing.bangla_digits": true}' > "$XDG_CONFIG_HOME/likhon/settings.json"
start_entry
check "settings file: colon and Bangla digits" "$(typed "ami 12: ")" "আমি ১২: "
stop_entry

# 3. Settings file: Probhat layout types by key position (kaTimR is কাঠিমড় there, and something else in phonetic).
printf '{"layout": "probhat"}' > "$XDG_CONFIG_HOME/likhon/settings.json"
start_entry
check "settings file: Probhat layout" "$(typed "kaTimR ")" "কাঠিমড় "
stop_entry

# 4. A password field: nothing is learned.
printf '{}' > "$XDG_CONFIG_HOME/likhon/settings.json"
rm -f "$XDG_DATA_HOME/likhon/user.sqlite"*
start_entry --password
typed "ami tumi " >/dev/null
stop_entry
if [ -z "$(learned)" ]; then echo "PASS  nothing learned in a password field"; else echo "FAIL  learned in a password field: [$(learned)]"; fail=1; fi

# 5. Agreement not accepted (declined, or not yet shown): the engine types nothing and keys pass straight through.
rm -f "$XDG_CONFIG_HOME/likhon/agreement"
export LIKHON_LIBEXEC=/nonexistent   # so no dialog opens in this headless session
start_entry
check "no agreement: keys pass through" "$(typed "ami ")" "ami "
stop_entry
printf '2026-11-21' > "$XDG_CONFIG_HOME/likhon/agreement"

[ "$fail" = 0 ] && echo "PASS" || { echo "FAILED"; exit 1; }
