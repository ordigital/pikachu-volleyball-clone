# Pikachu Volleyball Clone — Mega Drive

An independent Sega Mega Drive / Genesis port of the HD remake in this repository,
written in C with SGDK 2.11. The desktop/Android Rust game remains a separate build.
The ROM targets standard 4 MiB cartridges without a mapper or save RAM, including
PicoDrive on the SF2000.

## Play

- Choose **1 PLAYER** (pad 1 versus the computer) or **2 PLAYERS** (pads 1 and 2).
- **Left / Right:** move. **Up:** jump. **Down:** aim a downward airborne hit.
- **B:** power hit in the air; diving save while moving sideways on the ground.
  A short press is retained until the next simulation tick; holding B does not repeat.
- **Start:** pause. Choose **Resume** or **Quit** using Up/Down and B.
  Quit returns to the title screen. A fresh Start press also resumes.
- In two-player mode either pad can pause; that pad operates the pause choices.
- First to **15 points** wins. The title screen returns after GAME OVER.

No keyboard, touch controls, binding settings or desktop menu bar are included.
The title screen has a static grass background, no background music, and the
white pixel credit **by A.M.Brożyński**.

## Build

Install Docker and Python 3, then run from the repository root:

```sh
./ports/megadrive/build.sh
```

The script uses the immutable `doragasu/docker-sgdk` image for SGDK **2.11**, pinned
by SHA-256 digest. It runs as your user, converts the final output to a standard
4 MiB image, fixes its checksum and checks linked resource/VRAM budgets.
The result is **`ports/megadrive/out/rom.bin`**. A prebuilt `.bin` is also
available on the [Releases page](https://github.com/ordigital/pikachu-volleyball-clone/releases). Copy it to your emulator's Mega
Drive/Genesis ROM directory. To remove build outputs:

```sh
./ports/megadrive/build.sh clean
```

The final images/SFX in `res/` and native music score in `src/music_data.h`
are committed build inputs. Normal builds use Docker and Python 3, without Rust,
asset conversion or private tools. Optional local authoring scripts live in
`tools/`, which is deliberately Git-ignored and absent from public checkouts.

Sitting Pikachu, players and the shaded cloud use the root HD artwork, converted
to a limited pixel palette. Menu grass uses mirrored patches; transparent artwork
has an explicit palette. The court retains the HD sand and wooden border, with
lower HSV beach noise, dithered hills/sky and a shaded white/blue post. START/READY,
menu choices and scores use rounded Balsamiq Sans. The blue rounded pause panel
has opaque white labels. Cloud shading uses only white/cyan/blue palette entries.

The original BGM is **MIDI extracted directly from `Pokemon_20.exe`**, not an
MP3 transcription. `art/original-bgm.mid` is standard MIDI; the original MIDS
resource and SHA-256 provenance are kept alongside it. The EXE is not bundled.
The optional local extraction script reads PE resources
without executing it; the public audio check independently verifies the MIDI
against the original MIDS resource.

The compiler preserves all 4,541 source events, notes, velocity, volume,
expression, pitch bends and the complete 83.25-second song. It generates native
FM/PSG register commands with absolute MIDI deadlines. Instrument sounds are a
console arrangement: four FM reed voices, one FM bass, PSG accordion and simplified
PSG percussion. GM effects/modulation are omitted; FM pan is quantized to left,
centre or right, while PSG is mono. Playback dispatches due events each VBlank,
so timing is frame-quantized without accumulating per-event rounding error.

Final sound effects are mono 16 kHz PCM4 with fixed gain and peaks at most .30.
There is no sampled BGM or menu music. The root desktop/Android
music remains unchanged. Font licenses remain under root `assets/fonts/`.

## Implementation

- `src/physics.c` / `.h`: 32-bit integer physics and AI, matching the Rust update
  order, RNG and bounds. Normal play advances at 25 Hz. After a non-winning
  point, world animation/physics/effects advance at one fifth speed while the
  1.8-second point timer and music keep normal time. Interpolation covers the
  longer steps; landing completes its normal step before slow motion begins.
  Rendering does not change collisions.
- `src/main.c`: screen flow, controller edges, audio, fixed effect pools and drawing.
  NTSC/PAL VBlank timing drives the accumulator; input is polled every rendered
  frame. The console court uses the original **432-unit width**. World coordinates map
  with uniform scale **20/27**, Y offset **−8 px**, into **320×224**. Geometry,
  anchors and VRAM reservation share `src/scene.h` with the asset converter.
  Player cells are **48×48**, ball diameter **30 px** (40×32 padded cells), and
  the net post is taller/wider. This increases artwork scale by about 25% while
  keeping collisions, AI and scoring aligned with the displayed court.
- `res/`: 28 player poses, original rotating Poké Ball with baked squash states,
  spatially dithered pale-blue smoke and ball echoes, sand particles/dust,
  cloud art, water tiles, menu/court and rasterized font artwork.
- Ball impacts preserve the strong **50% height** squash for power hits and **10%**
  for ordinary bounces. Sand fountains also appear while walking (20% scale)
  and landing (50%). A tiled foam edge advances and retreats on a 7.2-second tide, reaching y220
  at most. HSV beach noise starts at y216, below the retained wooden court edge.
- GAME OVER ends enlarged, shakes and emits pale-blue smoke. Optional decorative
  effects are admitted within a conservative scanline sprite/pixel budget, with
  players, ball and scores reserved first. Transparency/blur use spatial dithering;
  this hardware has no ordinary alpha blending or arbitrary sprite scaling.
- `src/music.c` plays the original MIDI through five **YM2612 FM** voices and
  PSG; channel 6/DAC remains exclusively with PCM4 sound effects on CH2. Music
  uses every VInt independently of the 25 Hz physics and point slow motion.
  `src/music_data.h` is generated final input, not regenerated during builds.
- Pause freezes the MIDI cursor, mutes native voices, and waits for PCM4 to
  acknowledge SFX pause. Resume keeps song position and retriggers held FM notes;
  oscillator phase/envelopes are not restored sample-exactly. Screen transitions
  also clear pending SFX commands and both PCM buffers for a silent menu.
- FM writes hold the Z80 bus briefly and restore its bank-0 DAC address latch to
  `$2A` before release. Main-thread sound operations mask interrupts; VInt never
  waits for PCM4 pause acknowledgement. Driver ACK waits have a visible timeout.
- `src/drv_pcm4.s80` is the SGDK 2.11 driver with a small pause gate. It occupies
  unused initial padding, preserving the loader's exact **0xA6D-byte** image size.
  The 37-cycle gate is compensated by `wait83 → wait46`, keeping running DAC
  timing unchanged. It uses PARAMS+2C/+2D; +2A/+2B belong to READBUF. The original
  MIT license remains in `SGDK-LICENSE.txt`.

VRAM: plane A `0xC000`, window `0xD000`, plane B `0xE000`, sprite list `0xF000`,
scroll table `0xF400`; maps are 64×32. The font reserves 96 tiles and the sprite
engine 512 tiles. Dynamic sprites are uncompressed to reduce 68000 work;
backgrounds remain compressed. Do not move these tables without rechecking budgets.

## Verification and limitations

```sh
python3 ports/megadrive/tests/parity.py
python3 ports/megadrive/tests/check_rom.py
python3 ports/megadrive/tests/audio.py
python3 ports/megadrive/tests/emulator.py /usr/lib/libretro/picodrive_libretro.so /tmp/pika-md-test --long
python3 ports/megadrive/tests/emulator.py /usr/lib/libretro/genesis_plus_gx_libretro.so /tmp/pika-md-gx
python3 ports/megadrive/tests/emulator.py /usr/lib/libretro/picodrive_libretro.so /tmp/pika-md-pal --pal
```

The differential test compiles the actual root Rust physics/RNG and compares all
state fields, ground events and RNG state over 60,000 ticks, including AI and
round/win resets; the C side runs address/undefined-behavior sanitizers. It needs
Rust and a host C compiler. Emulator tests require Python, Pillow, NumPy and a
libretro core. They cover menu selection, pause ownership, frozen simulation and
audio buffers, resume, quit, 15-point completion and replays; `--long` streams and
checks two complete MIDI loops, pause near the loop boundary and menu silence.
The audio check compares extracted MIDI events and actual linked FM pitches,
verifies five-voice polyphony and checks PCM4 driver size/SFX headroom. Screenshots and a short audio capture go to `/tmp`.

Tested in PicoDrive and Genesis Plus GX, including PAL and NTSC. This is an initial
console port: some busy frames can take longer, and the original HD graphics,
soft transparency and number of particles are approximations at console resolution.
No physical Mega Drive or SF2000 was available for controller, audio-quality or
performance verification. Use the SF2000's emulator mapping to assign Genesis B
and Start; no alternative console button scheme is assumed inside the ROM.

Before finishing changes, run the relevant port checks and **both** the ROM build
and the root `cargo build --release --offline`. This keeps the console port and the existing desktop/Android game independently
buildable.
