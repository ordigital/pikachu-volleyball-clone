#!/bin/sh
set -eu
cd "$(dirname "$0")"
# SGDK 2.11, immutable Linux toolchain image (GCC 13.2).
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/m68k" registry.gitlab.com/doragasu/docker-sgdk@sha256:327ab838fbdf6bc741c6a7a11ee3c937cf1aaf1dc07a475995e89b741b6a830d SRC_S= "$@"

case "$*" in
    ""|release) python3 finalize_rom.py; python3 tests/check_rom.py ;;
esac
