#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Stages the files every Linux package installs under /usr: the engine built by ./build.sh, its IBus component file,
# the first-run agreement with its text, the command that deletes learned words, and the dictionary with its licence.
#   ./stage-tree.sh ROOT      fills ROOT/usr, for build-deb.sh and the RPM build. Run ./build.sh first.
# The engine is left unstripped (the caller strips it); ROOT/DEBIAN and the other packaging files stay with the caller.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../Source"
out="$here/out"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$src/Cargo.toml" | head -1 | tr -d '\r')"
root="${1:?usage: stage-tree.sh ROOT}"

[ -x "$out/ibus-engine-likhon" ] || { echo "run ./build.sh first" >&2; exit 1; }

install -d "$root/usr/bin" "$root/usr/lib/ibus-likhon" "$root/usr/share/ibus-likhon" "$root/usr/share/ibus/component"
install -m 0755 "$out/ibus-engine-likhon" "$root/usr/lib/ibus-likhon/ibus-engine-likhon"
# The first-run agreement (shown once, by the engine), its text, and the command that deletes learned words.
install -m 0755 "$here/ibus-engine/likhon-agreement" "$root/usr/lib/ibus-likhon/likhon-agreement"
install -m 0755 "$here/ibus-engine/likhon-delete-words" "$root/usr/bin/likhon-delete-words"
for l in en bn; do sed -e '1s/^\xEF\xBB\xBF//' -e 's/\r$//' "$src/assets/legal/eula.$l.txt" > "$root/usr/share/ibus-likhon/agreement.$l.txt"; chmod 0644 "$root/usr/share/ibus-likhon/agreement.$l.txt"; done
# The dictionary (read-only SQLite, MPL-2.0, from Avro Keyboard's word list and ours) with its licence and source note.
dict="$src/assets/dict/mpl"
# words.full.sqlite (the enlarged list, in git since 2026-11-21) wins over the Avro-only words.sqlite when it exists.
words="$dict/words.sqlite"; [ -f "$dict/words.full.sqlite" ] && words="$dict/words.full.sqlite"
install -m 0644 "$words" "$root/usr/share/ibus-likhon/words.sqlite"
install -m 0644 "$dict/LICENSE" "$root/usr/share/ibus-likhon/LICENSE"
install -m 0644 "$dict/SOURCE.md" "$root/usr/share/ibus-likhon/SOURCE.md"
sed -e "s|@LIBEXEC@|/usr/lib/ibus-likhon|" -e "s|@VERSION@|$version|" "$here/ibus-engine/likhon.xml.in" > "$root/usr/share/ibus/component/likhon.xml"
chmod 0644 "$root/usr/share/ibus/component/likhon.xml"
