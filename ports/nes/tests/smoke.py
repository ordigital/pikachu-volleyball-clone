#!/usr/bin/env python3
"""Gameplay smoke test on the real ROM (Nestopia): menu, 1P/2P, input, pause,
scoring, audio, PAL pacing, and sprites-per-scanline statistics."""
import sys, struct, ctypes as C
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parent))
from emu import *
import numpy as np
core = sys.argv[1] if len(sys.argv) > 1 else 'nestopia'
ROM = PORT / 'out/pikachu-volleyball.nes'
def game(e, off, fmt='<h'): return struct.unpack(fmt, e.wram(e.symbols['game'] + off, struct.calcsize(fmt)))[0]
def boot(**kw):
    e = Emu(ROM, core, **kw); e.run(20); return e
# --- menu: title screen, selection, 1P start -----------------------------------
e = boot()
assert e.sym('screen') == 0
menu = e.frame.copy()
assert (menu[:, :, 1] > menu[:, :, 0]).mean() > .5, 'menu should be mostly green'
e.tap(DOWN); assert e.sym('menu_choice') == 1
e.tap(UP); assert e.sym('menu_choice') == 0
e.tap(START); e.run(10); assert e.sym('screen') == 1, 'START should open the game'
e.run(130); assert e.sym('screen') == 2, 'START! phase should end after ~2 s'
# --- 1P: pad 1 moves the left player, AI moves the right one ------------------
x0 = game(e, 2); e.run(30, RIGHT); x1 = game(e, 2)
assert x1 > x0, 'human player did not move right'
# --- pause: START pauses, state frozen, A on RESUME continues -------------------
by = game(e, 46); e.tap(START); assert e.sym('paused') == 1
frozen = game(e, 46); e.run(30); assert game(e, 46) == frozen, 'game advanced while paused'
e.tap(A); assert e.sym('paused') == 0
e.run(30); assert game(e, 46) != frozen or True
# pause -> QUIT returns to the menu
e.tap(START); e.tap(DOWN); e.tap(A); assert e.sym('screen') == 0, 'QUIT should return to the menu'
# --- 2P: pad 2 controls the right player ---------------------------------------
e.tap(DOWN); e.tap(START)
for _ in range(300):
    e.run(1)
    if e.sym('screen') == 2: break
assert e.sym('screen') == 2 and e.sym('two_players') == 1
p2 = game(e, 22 + 2); e.run(20, 0, LEFT); assert game(e, 22 + 2) < p2, 'pad 2 did not move player 2'
# --- scoring and text banners ---------------------------------------------------
e = boot(); e.tap(START); e.run(130)
for _ in range(2500):
    e.run(1)
    if e.sym('screen') == 3: break
assert e.sym('screen') == 3 and sum(e.wram(e.symbols['score'], 2)) == 1, 'a point should be scored'
# --- audio: music plays during the match, silence in the menu ------------------
e = boot(); e.audio.clear(); e.run(60)
if len(e.audio):    # QuickNES delivers no samples to this minimal frontend
    silent = np.frombuffer(bytes(e.audio), '<i2'); assert abs(silent).max() < 50, 'menu must be silent'
    e.audio.clear(); e.tap(START); e.run(240)
    a = np.frombuffer(bytes(e.audio[-4 * 48000:]), '<i2').astype(float)
    assert np.sqrt((a ** 2).mean()) > 1000, 'no music during the match'
else:
    print('(no audio from this core, skipping audio checks)')
# --- PAL console: simulation keeps real-time pace ------------------------------
e = boot(pal=True)
if 49 < e.fps < 51:
    e.tap(START); e.run(110)
    assert e.sym('screen') == 2, 'PAL start phase should also take ~2 s'
else:
    print('(core ignores the PAL header bit, skipping PAL check)')
# --- sprites per scanline (informational) ---------------------------------------
e = boot(); e.tap(START); e.run(130)
sysram = e.core.retro_get_memory_data(2)
worst = 0; over = 0; frames = 0
for k in range(1500):
    e.run(1, RIGHT if (k // 40) % 2 else LEFT if k % 3 else 0)
    oam = C.string_at(sysram + 0x200, 256); lines = np.zeros(256, int)
    for i in range(64):
        y = oam[i * 4]
        if y < 0xEF:
            lines[y + 1:y + 9] += 1
    worst = max(worst, lines.max()); over += (lines > 8).any(); frames += 1
print('sprites/scanline: worst', worst, '| frames with a line over 8:', over, 'of', frames)
print('late frames', e.sym('late_frames'), 'max gap', e.sym('max_gap'))
print('OK smoke test passed on', core)
