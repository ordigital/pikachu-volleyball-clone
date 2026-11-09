#!/bin/sh
# Browser build: ports/browser/dist/ (index.html, gl.js, pika_audio.js, pikachu-volleyball.wasm).
#   ./build.sh           build into dist/
#   ./build.sh publish   also copy dist/ to docs/play/ (the GitHub Pages site)
# Needs: rustup target add wasm32-unknown-unknown (once). Works offline like the native build.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$root"
cargo build --release --offline --target wasm32-unknown-unknown
mkdir -p "$here/dist"
cp target/wasm32-unknown-unknown/release/pikachu-volleyball.wasm "$here/dist/"
cp "$here/index.html" "$here/gl.js" "$here/pika_audio.js" "$here/dist/"
ls -l "$here/dist"
if [ "${1:-}" = publish ]; then
  mkdir -p "$root/docs/play"
  cp "$here/dist/"* "$root/docs/play/"
  echo "copied to docs/play/"
fi
