#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Writes Product/Linux/SHA256SUMS.txt for the ibus-likhon packages that build-deb.sh and the RPM build produce.
#   ./write-sums.sh [VERSION] run after a build; lists the .deb and .rpm files present, sorted by file name
#                             VERSION limits the list to that version
# Safe to run again: the file is rebuilt from what is in the folder, and never lists itself.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
dir="$here/../Product/Linux"
version="${1:-}"
shopt -s nullglob
if [ -n "$version" ]; then
  rpm_version="${version//-/\~}"
  files=("$dir/ibus-likhon_${version}_"*.deb "$dir/ibus-likhon-${rpm_version}-"*.rpm)
else
  files=("$dir"/ibus-likhon*.deb "$dir"/ibus-likhon*.rpm)
fi
[ ${#files[@]} -gt 0 ] || { echo "no ibus-likhon packages in $dir" >&2; exit 1; }
cd "$dir"
sha256sum -- "${files[@]##*/}" | LC_ALL=C sort -k2 > SHA256SUMS.txt
echo "wrote $dir/SHA256SUMS.txt (${#files[@]} files)"
