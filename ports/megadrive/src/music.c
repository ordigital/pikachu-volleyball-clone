#include "music.h"
#include "music_data.h"

/* PCM4 owns DAC/FM channel 6. Runtime Z80 writes only DAC data, so every
 * 68000 FM transaction MUST restore its bank-0 $2A latch before releasing bus. */
#define PAUSE_REQUEST ((vu8 *)(Z80_DRV_PARAMS + 0x2C))
#define PAUSE_ACK ((vu8 *)(Z80_DRV_PARAMS + 0x2D))
static volatile bool running, music_paused;
static u16 video_hz, clock_fraction;
static bool pal;
/* Kept as named state for emulator timing/pause regression checks. */
volatile u16 music_tick, music_cursor;
volatile u32 music_loops;
static u8 levels[5][2], keys[5], psg_levels[4];

static void release_audio_bus(void) {
  YM2612_write(0, 0x2A);
  Z80_releaseBus();
}
static void silence_native(void) {
  for (u16 v = 0; v < 5; v++) {
    YM2612_writeReg(v / 3, 0x48 + v % 3, 127);
    YM2612_writeReg(v / 3, 0x4C + v % 3, 127);
  }
  for (u16 v = 0; v < 4; v++)
    PSG_setEnvelope(v, 15);
}
static void write_score(const MusicWrite *w) {
  u8 value = pal ? w->pal : w->ntsc;
  if (w->port == 2) {
    if ((value & 0x90) == 0x90)
      psg_levels[(value >> 5) & 3] = value & 15;
    PSG_write(value);
  } else {
    if (w->reg == 0x28) {
      u16 v = value & 7;
      if (v >= 4)
        v--;
      if (v < 5)
        keys[v] = (value & 0xF0) != 0;
    } else if ((w->reg & 0xFC) == 0x48 ||
               (w->reg & 0xFC) == 0x4C) {
      u16 v = w->port * 3 + (w->reg & 3);
      levels[v][(w->reg >> 2) & 1] = value;
    }
    YM2612_writeReg(w->port, w->reg, value);
  }
}
static void music_vblank(void) {
  if (!running || music_paused)
    return;
  clock_fraction += 120;
  while (clock_fraction >= video_hz) {
    clock_fraction -= video_hz;
    music_tick++;
  }
  if (music_cursor < MUSIC_WRITE_COUNT &&
      music_score[music_cursor].tick <= music_tick) {
    Z80_requestBus(TRUE);
    do {
      write_score(&music_score[music_cursor++]);
    } while (music_cursor < MUSIC_WRITE_COUNT &&
             music_score[music_cursor].tick <= music_tick);
    release_audio_bus();
  }
  if (music_tick >= MUSIC_END_TICK) {
    music_tick -= MUSIC_END_TICK;
    music_cursor = 0;
    music_loops++;
  }
}

static void pcm_pause(bool value) {
  SYS_disableInts();
  Z80_requestBus(TRUE);
  *PAUSE_REQUEST = value;
  Z80_releaseBus();
  SYS_enableInts();
  /* Only main may wait for ACK. Never hold interrupts/bus during this wait. */
  for (u16 attempt = 0; attempt < 250; attempt++) {
    SYS_disableInts();
    Z80_requestBus(TRUE);
    bool acknowledged = *PAUSE_ACK == value;
    Z80_releaseBus();
    SYS_enableInts();
    if (acknowledged)
      return;
    waitMs(1);
  }
  /* A mismatched driver must fail visibly rather than hang without diagnosis. */
  VDP_drawText("PCM4 PAUSE TIMEOUT", 1, 1);
  while (TRUE)
    SYS_doVBlankProcess();
}
static void clear_sfx(void) {
  pcm_pause(true);
  SYS_disableInts();
  Z80_requestBus(TRUE);
  vu8 *params = (vu8 *)Z80_DRV_PARAMS;
  *((vu8 *)Z80_DRV_COMMAND) = 0;
  *((vu8 *)Z80_DRV_STATUS) &= 0xF0;
  *((vu8 *)(Z80_DRV_STATUS + 1)) &= 0xF0;
  for (u16 channel = 0; channel < 4; channel++)
    for (u16 byte = 0; byte < 4; byte++)
      params[0x10 + channel * 4 + byte] = params[0x24 + byte];
  for (u16 i = 0; i < 512; i++)
    *((vu8 *)(Z80_RAM + 0x0E00 + i)) = 0;
  Z80_releaseBus();
  SYS_enableInts();
  pcm_pause(false);
}

void music_init(void) {
  pal = SYS_isPAL();
  video_hz = pal ? 50 : 60;
  Z80_loadDriver(Z80_DRIVER_PCM4, TRUE);
  pcm_pause(true);
  SYS_disableInts();
  Z80_requestBus(TRUE);
  /* No broad YM reset: leave PCM4's DAC and channel-6 routing intact. */
  YM2612_writeReg(0, 0x22, 0); /* no LFO */
  YM2612_writeReg(0, 0x27, 0); /* ordinary channel-3 mode */
  for (u16 v = 0; v < 5; v++) {
    /* Algorithm 4: two independent modulator/carrier pairs. Mild reed/bass
     * patches use no random/noise source and stay within the five FM voices. */
    const u8 reed[4][7] = {{1, 30, 31, 8, 2, 0x58, 0},
                           {2, 38, 31, 7, 2, 0x58, 0},
                           {1, 40, 28, 5, 0, 0x28, 0},
                           {1, 40, 28, 5, 0, 0x28, 0}};
    const u8 bass[4][7] = {{1, 38, 31, 12, 4, 0x58, 0},
                           {1, 48, 31, 12, 4, 0x58, 0},
                           {1, 40, 31, 8, 2, 0x48, 0},
                           {1, 40, 31, 8, 2, 0x48, 0}};
    for (u16 slot = 0; slot < 4; slot++)
      for (u16 reg = 0; reg < 7; reg++)
        YM2612_writeReg(v / 3, 0x30 + 16 * reg + 4 * slot + v % 3,
                        v == 4 ? bass[slot][reg] : reed[slot][reg]);
    YM2612_writeReg(v / 3, 0xB0 + v % 3, 0x14); /* feedback2, algorithm4 */
    YM2612_writeReg(v / 3, 0xB4 + v % 3, 0xC0);
    YM2612_writeReg(0, 0x28, v < 3 ? v : v + 1);
  }
  PSG_reset();
  silence_native();
  release_audio_bus();
  SYS_enableInts();
  pcm_pause(false);
  SND_PCM4_setVolume(SOUND_PCM_CH2, 15);
  SYS_setVIntCallback(music_vblank);
}
void music_pause(bool value) {
  SYS_disableInts();
  music_paused = value;
  Z80_requestBus(TRUE);
  if (value) {
    silence_native();
  } else if (running) {
    for (u16 v = 0; v < 5; v++) {
      YM2612_writeReg(v / 3, 0x48 + v % 3, levels[v][0]);
      YM2612_writeReg(v / 3, 0x4C + v % 3, levels[v][1]);
      u8 channel = v < 3 ? v : v + 1;
      YM2612_writeReg(0, 0x28, channel);
      if (keys[v])
        YM2612_writeReg(0, 0x28, 0xF0 | channel);
    }
    for (u16 v = 0; v < 4; v++)
      PSG_setEnvelope(v, psg_levels[v]);
  }
  release_audio_bus();
  SYS_enableInts();
  pcm_pause(value);
}
void music_stop(void) {
  SYS_disableInts();
  running = FALSE;
  Z80_requestBus(TRUE);
  silence_native();
  for (u16 v = 0; v < 5; v++) {
    YM2612_writeReg(0, 0x28, v < 3 ? v : v + 1);
    keys[v] = 0;
    levels[v][0] = levels[v][1] = 127;
  }
  for (u16 v = 0; v < 4; v++)
    psg_levels[v] = 15;
  release_audio_bus();
  SYS_enableInts();
  clear_sfx(); /* A screen transition explicitly clears pending/buffered SFX. */
  music_paused = FALSE;
}
void music_start(void) {
  music_stop();
  SYS_disableInts();
  music_tick = music_cursor = clock_fraction = 0;
  music_loops = 0;
  running = TRUE;
  SYS_enableInts();
}
