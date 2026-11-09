#!/usr/bin/env python3
"""Music/voice data checks, independent of tools/: decodes res/music_*.bin and the
DPCM samples inside the ROM, compares the score with the original MIDI and, with
Nestopia, checks that the pulse pitches that come out of the APU are the scored notes."""
import struct, sys
from pathlib import Path
import numpy as np
PORT = Path(__file__).resolve().parents[1]
RES = PORT / 'res'
sys.path.insert(0, str(Path(__file__).parent))
def midi_events(path):
    d = path.read_bytes(); assert d[:4] == b'MThd'
    div = struct.unpack('>H', d[12:14])[0]; assert div == 48
    p = 14; assert d[p:p+4] == b'MTrk'; end = p + 8 + struct.unpack('>I', d[p+4:p+8])[0]; p += 8
    t = 0; status = 0; ev = []
    while p < end:
        v = 0
        while True:
            b = d[p]; p += 1; v = v << 7 | b & 127
            if not b & 128: break
        t += v; b = d[p]
        if b == 0xFF:
            ln = d[p+2]; p += 3 + ln; continue
        if b & 128: status = b; p += 1
        n = 1 if status & 0xF0 in (0xC0, 0xD0) else 2
        ev.append((t, status & 0xF0, status & 15, d[p], d[p+1] if n == 2 else 0)); p += n
    return ev, t
ev, last = midi_events(PORT.parent / 'megadrive/art/original-bgm.mid')
FRAMES = round(last / 2)
def decode(name):
    b = (RES / f'music_{name}.bin').read_bytes(); assert b[-2:] == b'\xff\x00'
    out = []; f = 0
    for i in range(0, len(b) - 2, 2):
        n, l = b[i], b[i + 1]; assert l >= 1; out.append((f, n, l)); f += l
    return out, f
streams = {k: decode(k) for k in ('p1', 'p2', 'tri', 'noise')}
for k, (_, total) in streams.items():
    assert total == FRAMES, f'{k} stream is {total} frames, the song is {FRAMES}'
# P1 carries the sax melody (MIDI channel 2): every onset must be scored on its frame
onsets = {}
for t, k, ch, a, b in ev:
    if k == 0x90 and ch == 1 and b > 0: onsets.setdefault(round(t / 2), []).append(a)
p1 = {f: n for f, n, l in streams['p1'][0] if n}
miss = [f for f, notes in onsets.items() if p1.get(f) != max(notes)]
assert len(miss) <= 0.02 * len(onsets), f'{len(miss)} of {len(onsets)} melody onsets differ'
# triangle bass stays inside the triangle's range, drums use the 4 defined types
lo = np.frombuffer((RES / 'note_lo.bin').read_bytes(), np.uint8); hi = np.frombuffer((RES / 'note_hi.bin').read_bytes(), np.uint8)
period = lo.astype(int) | hi.astype(int) << 8
for _, n, _ in streams['tri'][0]:
    assert n == 0 or (period[n] >> 1) < 2048
assert {n for _, n, _ in streams['noise'][0]} <= {0, 1, 2, 3, 4}
# DPCM samples: DMC limits and placement in the ROM (64-byte aligned, $C000-$FFFF)
rom = (PORT / 'out/pikachu-volleyball.nes').read_bytes()
names = ['pipikachu', 'pika', 'chu', 'pi', 'pikachu', 'power_hit', 'ball_touches_ground', 'hit']
for i, name in enumerate(names):
    d = (RES / f'dpcm{i}.bin').read_bytes(); assert len(d) % 16 == 0 and len(d) <= 4081, name
    at = rom.find(d); assert at >= 16, f'{name} missing from the ROM'
    cpu = 0x8000 + at - 16
    assert cpu % 64 == 0 and cpu >= 0xC000 and cpu + len(d) < 0x10000, f'{name} at ${cpu:04X}'
print('score data OK:', {k: (len(v[0]), v[1]) for k, v in streams.items()}, '| melody onsets', len(onsets), 'missed', len(miss))
# --- emulated output: pulse pitches match the score -----------------------------
from emu import Emu, START
e = Emu(PORT / 'out/pikachu-volleyball.nes', 'nestopia'); e.run(30); e.audio.clear(); e.tap(START); e.run(60 * 12)
a = np.frombuffer(bytes(e.audio), '<i2').reshape(-1, 2).astype(float).mean(1); sr = int(e.rate)
freqs = sorted({round(440 * 2 ** ((n - 69) / 12)) for _, n, _ in streams['p1'][0] + streams['p2'][0] if n})
def dominant(seg):
    sp = np.abs(np.fft.rfft(seg * np.hanning(len(seg)), 1 << 15)); fr = np.fft.rfftfreq(1 << 15, 1 / sr)
    m = (fr > 200) & (fr < 2200); return fr[m][np.argmax(sp[m])]
hits = tot = 0
for i in range(sr // 2, len(a) - sr // 10, sr // 10):
    seg = a[i:i + sr // 10]
    if np.abs(seg).max() < 2000: continue
    f = dominant(seg); tot += 1
    hits += any(abs(f - x) / x < 0.03 for x in freqs)
assert tot > 50 and hits / tot > 0.6, f'only {hits}/{tot} windows match a scored pitch'
print(f'APU output: {hits}/{tot} analysis windows carry a scored pitch')
print('OK audio')
