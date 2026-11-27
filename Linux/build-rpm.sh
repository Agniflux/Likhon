#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Builds the ibus-likhon RPM (Release 1%{?dist}) from the engine built by ./build.sh.
#   ./build-rpm.sh            output: Product/Linux/
# Needs: rpm-build and redhat-rpm-config (rpmbuild). Run ./build.sh first. The package installs only system files
# under /usr (the engine,
# its IBus component file, the licence notices); it writes nothing to a person's home folder.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../Source"
out="$here/out"
product="$here/../Product/Linux"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$src/Cargo.toml" | head -1 | tr -d '\r')"
# RPM Version cannot contain a hyphen: 0.2.1-alpha.1 becomes 0.2.1~alpha.1, which RPM sorts before 0.2.1.
rpm_version="${version//-/\~}"
case "$rpm_version" in
  ''|*[!0-9A-Za-z.+~]*) echo "unsupported version: $version" >&2; exit 1 ;;
esac
top="$(mktemp -d)"
trap 'rm -rf "$top"' EXIT

[ -x "$out/ibus-engine-likhon" ] || { echo "run ./build.sh first" >&2; exit 1; }
command -v rpmbuild >/dev/null 2>&1 || { echo "install rpm-build" >&2; exit 1; }

mkdir -p "$top"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
root="$top/SOURCES/tree"
mkdir -p "$root"
"$here/stage-tree.sh" "$root"
# The deb keeps the notices in /usr/share/doc; the RPM keeps the licence texts in /usr/share/licenses and the
# source offer in /usr/share/doc.
install -d "$root/usr/share/licenses/ibus-likhon" "$root/usr/share/doc/ibus-likhon"
for f in NOTICE NOTICES.md; do install -m 0644 "$here/../$f" "$root/usr/share/licenses/ibus-likhon/$f"; done
install -m 0644 "$here/../LICENSE" "$root/usr/share/licenses/ibus-likhon/LICENSE"
for f in NOTICE.txt avro-keyboard-MPL-2.0.txt khipro-MIT.txt unicode-cldr.txt rust-crates.txt; do install -m 0644 "$here/../Source/assets/licenses/$f" "$root/usr/share/licenses/ibus-likhon/$f"; done
install -m 0644 "$here/../SOURCE-OFFER.txt" "$root/usr/share/doc/ibus-likhon/SOURCE-OFFER.txt"

sed "s/@RPM_VERSION@/$rpm_version/g" "$here/ibus-likhon.spec.in" > "$top/SPECS/ibus-likhon.spec"
rpmbuild -bb --define "_topdir $top" "$top/SPECS/ibus-likhon.spec"

shopt -s nullglob
rpms=("$top"/RPMS/*/*.rpm)
[ ${#rpms[@]} -gt 0 ] || { echo "rpmbuild produced no rpm" >&2; exit 1; }
mkdir -p "$product"
cp "${rpms[@]}" "$product"/
"$here/write-sums.sh" "$version"
rpm="$product/${rpms[0]##*/}"
echo "built $rpm ($(stat -c %s "$rpm") bytes)"
rpm -qip "$rpm" | sed -n '1,12p'
