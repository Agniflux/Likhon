#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Writes Product/Linux/SHA256SUMS.txt for the ibus-likhon packages that build-deb.sh and the RPM build produce.
#   ./write-sums.sh           run after a build; lists the .deb and .rpm files present, sorted by file name
# Safe to run again: the file is rebuilt from what is in the folder, and never lists itself.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
dir="$here/../Product/Linux"
shopt -s nullglob
files=("$dir"/ibus-likhon*.deb "$dir"/ibus-likhon*.rpm)
[ ${#files[@]} -gt 0 ] || exit 0
cd "$dir"
sha256sum -- "${files[@]##*/}" | LC_ALL=C sort -k2 > SHA256SUMS.txt
echo "wrote $dir/SHA256SUMS.txt (${#files[@]} files)"
