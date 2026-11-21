#!/usr/bin/env bash
# Builds the Likhon IBus engine on Linux: the Rust typing layer (likhon-ffi) and the C shell around it.
#   ./build.sh            builds into Desktop/Linux/out/
# Needs: a C compiler, pkg-config, libibus-1.0-dev, and Rust (rustup). The Rust target dir is kept out of the repo.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
src="$here/../Source"
out="$here/out"
target="${CARGO_TARGET_DIR:-$src/target}"
mkdir -p "$out"

( cd "$src" && cargo build --release -p likhon-ffi )

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$src/Cargo.toml" | head -1 | tr -d '\r')"
cc -O2 -Wall -Wextra -I"$src/crates/likhon-ffi/include" "$here/ibus-engine/likhon-ibus.c" \
   "$target/release/liblikhon_ffi.a" \
   $(pkg-config --cflags --libs ibus-1.0) -lm -ldl -lpthread \
   -o "$out/ibus-engine-likhon"

# The component file for running straight from the build folder (development; the .deb installs its own copy).
sed -e "s|@LIBEXEC@|$out|" -e "s|@VERSION@|$version|" "$here/ibus-engine/likhon.xml.in" > "$out/likhon.xml"
echo "built $out/ibus-engine-likhon ($version), $(stat -c %s "$out/ibus-engine-likhon") bytes"
