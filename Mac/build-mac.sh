#!/usr/bin/env bash
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of the MPL was
# not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
# Builds the Likhon macOS input method as one universal (Apple Silicon + Intel) app, an installer package, and a
# disk image:  Desktop/Product/Mac/Likhon-<version>-macOS.dmg
#   ./build-mac.sh
# Needs: a Mac with Xcode command line tools (swiftc, lipo, pkgbuild, hdiutil) and Rust (rustup).
# Not signed or notarized (owner decision 2026-11-21): the download page explains right-click Open.
# NOT YET RUN: written on Windows. Run it on a Mac and fix what breaks.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../Source"
build="$here/build"
product="$here/../Product/Mac"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$src/Cargo.toml" | head -1 | tr -d '\r')"
min_macos="13.0"

rm -rf "$build"
mkdir -p "$build" "$product"

# 1. The Rust typing layer, once per architecture, joined into one library.
rustup target add aarch64-apple-darwin x86_64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET="$min_macos"
( cd "$src" && cargo build --release -p likhon-ffi --target aarch64-apple-darwin )
( cd "$src" && cargo build --release -p likhon-ffi --target x86_64-apple-darwin )
target="${CARGO_TARGET_DIR:-$src/target}"
lipo -create "$target/aarch64-apple-darwin/release/liblikhon_ffi.a" \
             "$target/x86_64-apple-darwin/release/liblikhon_ffi.a" -output "$build/liblikhon_ffi.a"

# 2. The Swift app, once per architecture, joined into one executable.
for arch in arm64 x86_64; do
  swiftc -O -target "$arch-apple-macos$min_macos" \
    -import-objc-header "$here/Likhon-Bridging-Header.h" -Xcc "-I$src/crates/likhon-ffi/include" \
    "$here/Sources/main.swift" "$here/Sources/LikhonInputController.swift" "$here/Sources/Agreement.swift" \
    "$build/liblikhon_ffi.a" -framework Cocoa -framework InputMethodKit -liconv \
    -o "$build/LikhonInputMethod-$arch"
done
lipo -create "$build/LikhonInputMethod-arm64" "$build/LikhonInputMethod-x86_64" -output "$build/LikhonInputMethod"

# 3. The app bundle.
app="$build/pkgroot/Likhon.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$build/LikhonInputMethod" "$app/Contents/MacOS/LikhonInputMethod"
sed "s|@VERSION@|$version|g" "$here/Info.plist.in" > "$app/Contents/Info.plist"
dict="$src/assets/dict/mpl"
# words.full.sqlite (the enlarged list, in git since 2026-11-21) wins over the Avro-only words.sqlite when it exists.
words="$dict/words.sqlite"; [ -f "$dict/words.full.sqlite" ] && words="$dict/words.full.sqlite"
cp "$words" "$app/Contents/Resources/words.sqlite"
# The first-run agreement text (the same as the installer's and the website's; Desktop/Installer/license/make-eula.mjs; text in Source/assets/legal).
for l in en bn; do perl -pe 's/^\xEF\xBB\xBF//; s/\r$//' "$src/assets/legal/eula.$l.txt" > "$app/Contents/Resources/agreement.$l.txt"; done
cp "$dict/LICENSE" "$app/Contents/Resources/DICTIONARY-LICENSE.txt"
for f in NOTICE NOTICES.md SOURCE-OFFER.txt; do cp "$here/../$f" "$app/Contents/Resources/$f"; done
cp "$dict/SOURCE.md" "$app/Contents/Resources/DICTIONARY-SOURCE.md"
plutil -lint "$app/Contents/Info.plist"
# An ad-hoc signature (no Apple account): Apple Silicon refuses to run unsigned code at all.
codesign --force --deep --sign - "$app"

# 4. The installer package: puts the app in /Library/Input Methods (asks for the Mac password once).
pkgbuild --root "$build/pkgroot" --identifier app.likhon.inputmethod.pkg --version "$version" \
         --install-location "/Library/Input Methods" "$build/Install Likhon.pkg"

# 5. The disk image: the installer and a short how-to.
dmgroot="$build/dmgroot"
mkdir -p "$dmgroot"
cp "$build/Install Likhon.pkg" "$dmgroot/"
cp "$here/INSTALL.txt" "$dmgroot/Read me first.txt"
dmg="$product/Likhon-$version-macOS.dmg"
rm -f "$dmg"
hdiutil create -volname "Likhon $version" -srcfolder "$dmgroot" -ov -format UDZO "$dmg"
( cd "$product" && shasum -a 256 "$(basename "$dmg")" > SHA256SUMS.txt )
echo "built $dmg ($(stat -f %z "$dmg") bytes)"
lipo -info "$app/Contents/MacOS/LikhonInputMethod"
