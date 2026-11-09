#!/usr/bin/env python3
"""AI-vs-AI soak run: counts main-loop iterations that took more than one frame."""
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from emu import *
frames = int(sys.argv[2]) if len(sys.argv) > 2 else 20000
e = Emu(PORT / 'out/demo.nes', sys.argv[1] if len(sys.argv) > 1 else 'nestopia')
e.symbols = {}
for line in (PORT / 'out/demo.labels').read_text().splitlines():
    p = line.split()
    if len(p) == 3: e.symbols[p[2].lstrip('.').lstrip('_')] = int(p[1], 16)
seen = set()
for k in range(frames // 1000):
    e.run(1000); seen.add(e.sym('screen'))
    print(e.frames, 'screen', e.sym('screen'), 'late', e.sym('late_frames'), 'max_gap', e.sym('max_gap'), 'score', list(e.wram(e.symbols['score'], 2)))
import struct
h = struct.unpack('<6H', e.wram(e.symbols['gap_hist'], 12))
print('screens seen', sorted(seen), '| iterations by frames taken [1,2,3,4,5+]:', h[1:])
