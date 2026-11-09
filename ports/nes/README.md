# Pikachu Volleyball Clone — NES

An independent Nintendo Entertainment System port of the HD remake in this repository,
written in C and 6502 assembly with cc65. The desktop/Android Rust game and the Mega
Drive port remain separate builds. The cartridge is an **MMC3 (mapper 4)** board with
32 KiB PRG, 32 KiB CHR-ROM and 8 KiB WRAM (no battery, nothing is saved). NTSC and PAL/Dendy
consoles are detected at start-up (timing, interpolation and music tempo follow).

## Play

- Choose **1 PLAYER** (pad 1 against the computer) or **2 PLAYERS** (pads 1 and 2).
- **Left / Right:** move. **Up:** jump. **Down:** aim a downward airborne hit.
- **A or B:** power hit in the air; diving save while moving sideways on the ground.
  A press is kept until the next 25 Hz simulation step; holding the button does not repeat.
- **Start:** pause, then Up/Down + A/B to **Resume** or **Quit** (back to the title screen).
- First to **15 points** wins, then the title screen returns.

## Build and test

Docker and Python 3 are enough (the build image installs cc65 from the Ubuntu archive):

```sh
./ports/nes/build.sh                # out/pikachu-volleyball.nes
./ports/nes/tests/run_all.sh        # parity, ROM physics, gameplay, audio, lag
```

`build.sh demo` and `build.sh phystest` build the AI-vs-AI soak ROM and the physics
checksum ROM used by the tests. Tests run the real ROM in the Nestopia and QuickNES
libretro cores (`/usr/lib/libretro`), through `tests/emu.py`. Optional authoring tools
(`tools/`, Git-ignored) regenerate `res/` from the root HD art, the original MIDI kept by
the Mega Drive port and its voice WAVs; the build needs only the committed `res/` files.

## What differs from the other ports

The simulation is the same integer physics and AI at 25 Hz (`src/physics.c`, 16-bit,
checked tick-for-tick against the 32-bit Mega Drive C port and, through it, the Rust
code). Only the random generator differs (a 16-bit xorshift). The 6502 hardware
forced these arrangements:

- **Players 32×32 and ball 24×24 px** — the largest that fit the PPU's eight sprites
  per scanline with both players and the ball overlapping. Where they still overlap,
  `trim_rows` (assembly) removes the tail/edge tiles deterministically instead of flickering; it also leaves room for the pole sprites.
- The world keeps the original 432-unit width, mapped by 19/32 onto 256 px. Positions
  are interpolated at 50/60 Hz between simulation steps.
- Static scenery: clouds and a strip of sea are painted into the background; there is
  no squash-and-stretch ball, moving water, cloud shadow or walking sand. A sand burst
  appears only when the ball lands for a point (at most eight sprites).
- Low green hills run unbroken behind the pole (its 32px column has its own palette: green,
  white, blue, so the hill shading fades out there); the post has a wooden base in the sand.
- Pikachu uses three colours (yellow, black, red) per sprite palette.
- Scores are sprites; READY?/START!/GAME OVER/pause text are background tiles patched
  through a small VRAM queue during vblank.

## Audio

Music is the original BGM (MIDI, 83.25 s) arranged for the APU at 60 frames per second:
pulse 1 melody (sax line), pulse 2 accordion/octave harmony, triangle bass and a
four-type noise drum kit. The score is data (`res/music_*.bin`, 4.4 KiB) played by
`src/audio_engine.s` from the NMI. The eight voice effects are the original samples
converted to 1-bit DPCM (11 or 16.9 kHz, 7.3 KiB; the "hit" is cut from the power-hit sample); the DMC never interferes with the music
channels, and the controllers are read twice because a DMC fetch can corrupt one read.
1-bit DPCM is quiet, so the effects are layered with APU synth: a rising "pik" chirp on
pulse 2 when a swing starts, noise thumps for hits, spikes and the ball landing. While an
effect plays the music is ducked (pulses halved, triangle bass muted). Ordinary (non-spike)
hits have their own short sound. PAL consoles get a second sequencer step every fifth
frame to keep the tempo.

## Layout

| File | Role |
|---|---|
| `src/main.c` | screens, input edges, interpolation, sprite composition, banners |
| `src/physics.c` / `.h` | integer physics and AI (host-testable) |
| `src/predict.s` | landing prediction hot loops (exact copy of the C reference) |
| `src/draw.s` | player/ball OAM emitters |
| `src/crt0.s` | header, reset (region detection), NMI (OAM DMA, VRAM queue, CHR banks, pad polling) |
| `src/audio_engine.s`, `audio_data.s` | sequencer, DPCM voices, music/sample data |
| `src/scene.h`, `chr_layout.*` | geometry shared with the converters; generated CHR layout |
| `res/` | CHR image, nametables, palettes, pose masks, music streams, DPCM samples |
| `mmc3.cfg` | linker map: game code `$8000`, data/audio/samples fixed at `$C000` |

CHR layout (1 KiB banks): 0-3 game background, 4-7 title screen, 8-14 the 28 player
poses (four per bank; each player maps its own bank in a 1 KiB sprite window), 15 ball
frames/shadow/arrow/dust, 16 score digits.

## Limits

Tested in emulators only (no real cartridge, flash cart, PAL console or hardware
controller). Under heavy AI load a handful of frames take two or more video frames
(about 3% of iterations in a worst-case computer-vs-computer run, 0.3% three or more); the diagnostics
are in `tests/demo_lag.py`. Pikachu's yellow is the closest NES yellow (`$28`) and
looks greenish on some palettes.

## Credits

Original game: Pikachu Volleyball (1997, (c) SACHI SOFT / SAWAYAKAN Programmers and
Satoshi Takenouchi). The starting point for the project was the files of the browser
implementation by gorisanson/pikachu-volleyball, a reconstruction of the original game's logic. Fonts (Jersey 10, Balsamiq
Sans) are used under the SIL OFL; the licenses are in the repository's `assets/fonts/`.
Port and artwork conversion: Adam Mateusz Brożyński.
