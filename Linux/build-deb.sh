#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Builds ibus-likhon_<version>_<arch>.deb from the engine built by ./build.sh.
#   ./build-deb.sh            output: Desktop/Product/Linux/
# Needs: dpkg-deb (dpkg-dev), strip. Run ./build.sh first. The package installs only system files under /usr
# (the engine, its IBus component file, the licence notices); it writes nothing to a person's home folder.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../Source"
out="$here/out"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$src/Cargo.toml" | head -1 | tr -d '\r')"
arch="$(dpkg --print-architecture)"
pkg="ibus-likhon"
root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT

[ -x "$out/ibus-engine-likhon" ] || { echo "run ./build.sh first" >&2; exit 1; }

install -d "$root/usr/bin" "$root/usr/lib/ibus-likhon" "$root/usr/share/ibus-likhon" "$root/usr/share/ibus/component" "$root/usr/share/doc/$pkg" "$root/DEBIAN"
install -m 0755 "$out/ibus-engine-likhon" "$root/usr/lib/ibus-likhon/ibus-engine-likhon"
strip --strip-unneeded "$root/usr/lib/ibus-likhon/ibus-engine-likhon"
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

cat > "$root/usr/share/doc/$pkg/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: Likhon
Source: https://likhon.agniflux.com

Files: *
Copyright: Agniflux Enterprise
License: MPL-2.0
 This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
 not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
 .
 Third-party notices (okkhor, MIT, and the other Rust crates linked in) are listed in Likhon's About page and at
 https://likhon.agniflux.com/licenses.
EOF
chmod 0644 "$root/usr/share/doc/$pkg/copyright"
for f in NOTICE NOTICES.md SOURCE-OFFER.txt; do install -m 0644 "$here/../$f" "$root/usr/share/doc/$pkg/$f"; done

size_kb="$(du -sk "$root/usr" | cut -f1)"
cat > "$root/DEBIAN/control" <<EOF
Package: $pkg
Version: $version
Section: utils
Priority: optional
Architecture: $arch
Depends: ibus (>= 1.5), libc6 (>= 2.31), libglib2.0-0
Recommends: fonts-noto-core, zenity | kdialog
Installed-Size: $size_kb
Maintainer: Agniflux Enterprise <info@agniflux.com>
Homepage: https://likhon.agniflux.com
Description: Likhon, free Bangla phonetic typing for IBus
 Type Bangla with English letters (ami -> আমি). Shows the word being typed with suggestions from a built-in
 dictionary, and commits it when it ends.
 Nothing you type is stored or sent anywhere.
 .
 After installing, add "Likhon (Bangla phonetic)" under Settings > Keyboard > Input Sources (GNOME), or run
 ibus-setup on other desktops. If it does not appear, run: ibus restart
EOF

cat > "$root/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
# Nothing to register system-wide: ibus-daemon reads /usr/share/ibus/component/ when a session starts or restarts.
exit 0
EOF
chmod 0755 "$root/DEBIAN/postinst"

mkdir -p "$here/../Product/Linux"
deb="$here/../Product/Linux/${pkg}_${version}_${arch}.deb"
dpkg-deb --build --root-owner-group "$root" "$deb" >/dev/null
echo "built $deb ($(stat -c %s "$deb") bytes)"
sha256sum "$deb" | sed "s|$here/../Product/Linux/||" > "$here/../Product/Linux/SHA256SUMS.txt"
dpkg-deb --info "$deb" | head -14
