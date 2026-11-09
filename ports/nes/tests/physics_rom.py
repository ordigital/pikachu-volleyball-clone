#!/usr/bin/env python3
"""cc65-compiled physics == host-compiled physics (same driver, same xorshift RNG)."""
import subprocess, sys, tempfile, struct
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from emu import *
host = subprocess.run
with tempfile.TemporaryDirectory() as t:
    t = Path(t)
    (t / 'h.c').write_text('#include <stdio.h>\n#include "phys_driver.h"\nint main(void){uint16_t o[3*PH_CHECKS];for(int m=0;m<3;m++)ph_run(m,o+m*PH_CHECKS);for(int i=0;i<3*PH_CHECKS;i++)printf("%u\\n",o[i]);}\n')
    subprocess.run(['gcc', '-O1', '-DPIKA_HOST', '-I', str(PORT / 'src'), '-I', str(PORT / 'tests'), '-o', t / 'h', t / 'h.c', PORT / 'src/physics.c'], check=True)
    expected = [int(x) for x in subprocess.run([t / 'h'], capture_output=True, text=True, check=True).stdout.split()]
e = Emu(PORT / 'out/phystest.nes', sys.argv[1] if len(sys.argv) > 1 else 'nestopia')
e.symbols = {}
for line in (PORT / 'out/phystest.labels').read_text().splitlines():
    p = line.split()
    if len(p) == 3: e.symbols[p[2].lstrip('.').lstrip('_')] = int(p[1], 16)
for _ in range(400):
    e.run(100)
    if e.sym('ph_done'): break
assert e.sym('ph_done'), 'ROM physics did not finish'
got = list(struct.unpack('<%dH' % len(expected), e.wram(e.symbols['ph_out'], 2 * len(expected))))
bad = [i for i, (a, b) in enumerate(zip(got, expected)) if a != b]
assert not bad, f'mismatch at checkpoints {bad[:5]} (first: rom {got[bad[0]]} host {expected[bad[0]]})'
print('OK: cc65 physics matches host for', len(expected) * 100, 'ticks; ROM frames used:', e.frames)
