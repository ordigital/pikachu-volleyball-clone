#!/bin/sh
# Browser build: ports/browser/dist/ (index.html, gl.js, pika_audio.js, pikachu-volleyball.wasm).
#   ./build.sh           build into dist/
#   ./build.sh publish   also copy dist/ to site/play/ (the website source)
# Needs: rustup target add wasm32-unknown-unknown (once). Works offline like the native build.
set -eu
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
cd "$root"
cargo build --release --offline --target wasm32-unknown-unknown
mkdir -p "$here/dist"
cp target/wasm32-unknown-unknown/release/pikachu-volleyball.wasm "$here/dist/"
cp "$here/gl.js" "$here/pika_audio.js" "$here/dist/"
# The progress bar needs the decoded size (Content-Length is the compressed size when the server gzips).
sed "s/__WASM_BYTES__/$(wc -c < "$here/dist/pikachu-volleyball.wasm")/" "$here/index.html" > "$here/dist/index.html"
ls -l "$here/dist"
if [ "${1:-}" = publish ]; then
  mkdir -p "$root/site/play"
  cp "$here/dist/"* "$root/site/play/"
  brotli -q 11 -f -o "$root/site/play/pikachu-volleyball.wasm.br" "$here/dist/pikachu-volleyball.wasm"
  echo "copied to site/play/"
fi
