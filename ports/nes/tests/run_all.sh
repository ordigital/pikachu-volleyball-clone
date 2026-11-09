#!/bin/sh
# Full NES port check. Builds every ROM variant, then runs all tests.
set -eu
cd "$(dirname "$0")/.."
./build.sh >/dev/null 2>&1
./build.sh phystest >/dev/null 2>&1
EXTRA="-DPIKA_WIN=2" ./build.sh demo >/dev/null 2>&1
python3 tests/parity.py
python3 tests/physics_rom.py
python3 tests/smoke.py nestopia
python3 tests/smoke.py quicknes
python3 tests/audio.py
python3 tests/demo_lag.py nestopia 20000 | tail -2
