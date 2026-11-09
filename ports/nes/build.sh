#!/bin/sh
# Build with cc65 inside the pika-nes-toolchain Docker image.
#   ./build.sh            -> out/pikachu-volleyball.nes   (the game)
#   ./build.sh demo       -> out/demo.nes                 (AI vs AI soak/lag test ROM)
#   ./build.sh phystest   -> out/phystest.nes             (physics checksum ROM)
set -eu
cd "$(dirname "$0")"
IMG=pika-nes-toolchain
docker image inspect $IMG >/dev/null 2>&1 || docker build -t $IMG .
mode=${1:-game}
case $mode in
  game) out=pikachu-volleyball; defs="${EXTRA:-}"; srcs="src/*.c" ;;
  demo) out=demo; defs="-DPIKA_DEMO ${EXTRA:-}"; srcs="src/*.c" ;;
  phystest) out=phystest; defs=""; srcs="$(ls src/*.c | grep -v main.c | tr '\n' ' ') tests/phystest_main.c" ;;
  timing) out=timing; defs=""; srcs="$(ls src/*.c | grep -v main.c | grep -v physics.c | tr '\n' ' ') tests/timing_main.c" ;;
  *) echo "unknown mode $mode" >&2; exit 2 ;;
esac
rm -rf out/$mode; mkdir -p out/$mode
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/w" -w /w $IMG sh -ec "
  CFLAGS='--cpu 6502 -t none -Oirs -Isrc -Itests $defs'
  for f in $srcs; do b=\$(basename \$f .c); cc65 \$CFLAGS \$f -o out/$mode/\$b.s; ca65 --cpu 6502 out/$mode/\$b.s -o out/$mode/\$b.o; done
  for f in src/*.s; do ca65 --cpu 6502 -Isrc \$f -o out/$mode/\$(basename \$f .s).o; done
  ld65 -C mmc3.cfg -m out/$mode/map.txt -Ln out/$mode/labels.txt -o out/$out.nes out/$mode/*.o none.lib -L /usr/share/cc65/lib
"
cp out/$mode/labels.txt out/$out.labels
ls -l out/$out.nes
