#!/usr/bin/env python3
"""Produce a standard 4 MiB cartridge image, then checksum the final padded bytes."""
from pathlib import Path
import struct
path=Path(__file__).resolve().parent/'out/rom.bin'
rom=bytearray(path.read_bytes())
assert len(rom)<=0x400000,'ROM exceeds mapper-free 4 MiB budget'
rom.extend(b'\xff'*(0x400000-len(rom)))
struct.pack_into('>I',rom,0x1A4,len(rom)-1)
checksum=sum(value[0] for value in struct.iter_unpack('>H',memoryview(rom)[512:]))&65535
struct.pack_into('>H',rom,0x18E,checksum)
path.write_bytes(rom)

# SGDK emits optional sprite-layout PNGs.
for preview in (path.parents[1]/'res').glob('*_opt.png'):
    preview.unlink()
