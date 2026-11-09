#!/bin/sh
# Usage: build-appimage.sh x86_64|aarch64 binary library-dir runtime output
set -eu
[ "$#" -eq 5 ] || { echo "usage: $0 arch binary library-dir runtime output" >&2; exit 2; }
arch=$1; binary=$2; libs=$3; runtime=$4; output=$5
case "$arch" in x86_64|aarch64) ;; *) echo "unsupported architecture: $arch" >&2; exit 2;; esac
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT HUP INT TERM
app="$dir/PikachuVolleyball.AppDir"
mkdir -p "$app/usr/bin" "$app/usr/lib" "$app/usr/share/licenses/pikachu-volleyball-clone"
cp "$binary" "$app/usr/bin/pikachu-volleyball"
cp -L "$libs/libudev.so.1" "$app/usr/lib/"
cp "$root/packaging/AppRun" "$root/packaging/pikachu-volleyball-clone.desktop" "$root/packaging/pikachu-volleyball-clone.png" "$app/"
cp "$root/assets/fonts/"*-OFL.txt "$app/usr/share/licenses/pikachu-volleyball-clone/"
ARCH="$arch" appimagetool --no-appstream --runtime-file "$runtime" "$app" "$output"
