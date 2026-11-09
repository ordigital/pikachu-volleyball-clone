/* Pikachu Volleyball - NES (MMC3).  Screen flow, input, interpolation and
 * sprite composition; physics.c is the shared integer simulation. */
#include <stdint.h>
#include "audio.h"
#include "chr_layout.h"
#include "physics.h"
#include "scene.h"

#define PPUCTRL (*(volatile uint8_t *)0x2000)
#define PPUMASK (*(volatile uint8_t *)0x2001)
#define PPUSTAT (*(volatile uint8_t *)0x2002)
#define PPUADDR (*(volatile uint8_t *)0x2006)
#define PPUDATA (*(volatile uint8_t *)0x2007)
#define MMC_SEL (*(volatile uint8_t *)0x8000)
#define MMC_DAT (*(volatile uint8_t *)0x8001)
#define PAD_A 1
#define PAD_B 2
#define PAD_SELECT 4
#define PAD_START 8
#define PAD_UP 16
#define PAD_DOWN 32
#define PAD_LEFT 64
#define PAD_RIGHT 128
#define SIM_HZ 25
#ifndef PIKA_WIN
#define PIKA_WIN 15
#endif
#define WIN_SCORE PIKA_WIN

extern volatile uint8_t nmi_frame, ppu_ctrl, ppu_mask, vbuf_len, oam_ready;
extern uint8_t region_pal;
#pragma zpsym("region_pal")
static uint8_t hz; /* 60 (NTSC) or 50 (PAL/Dendy) */
extern uint8_t chr_bank[6], chr_next[6], vbuf[128];
#pragma zpsym("nmi_frame")
#pragma zpsym("ppu_ctrl")
#pragma zpsym("ppu_mask")
#pragma zpsym("vbuf_len")
#pragma zpsym("oam_ready")
#pragma zpsym("chr_bank")
#pragma zpsym("chr_next")
extern uint8_t oi, dr_x, dr_y, dr_tile, dr_flip, dr_shy;
extern const uint8_t *dr_ptr;
extern int16_t dr_bx;
#pragma zpsym("oi")
#pragma zpsym("dr_x")
#pragma zpsym("dr_y")
#pragma zpsym("dr_tile")
#pragma zpsym("dr_flip")
#pragma zpsym("dr_shy")
#pragma zpsym("dr_ptr")
#pragma zpsym("dr_bx")
extern const uint8_t game_nt[], game_pal[], menu_nt[], menu_pal[], pose_mask[];
extern uint8_t oam[256];

/* Test-visible state (the Python tests read these from WRAM via out/labels.txt). */
/* Sprite tile indices (pattern table $1000 = four 1 KiB CHR windows); the
 * chr_layout.h values are generated.  Players: P1 window 0, P2 window 1. */
#define TILE_P1 0
#define TILE_P2 64
enum { MENU, STARTING, PLAYING, POINT, READY, FINISHED };
PikaPhysics game;
int16_t prev_px[2], prev_py[2], prev_bx, prev_by;
uint8_t held[2], edge[2], queued[2];
uint8_t screen, paused, two_players, serve_right, round_scored;
uint8_t score[2], menu_choice, pause_choice, slow_segment, slow_phase;
uint8_t acc, order, last_sprites;
uint16_t timer;
uint8_t late_frames, max_gap, last_frame;
uint16_t gap_hist[6]; /* iterations that took 1,2,3,4,5+ frames (diagnostics) */

/* ---------------------------------------------------------------- video -- */
static void wait_frame(void) {
  uint8_t f = nmi_frame;
  while (f == nmi_frame) {
  }
}
static void apply_banks(void) {
  uint8_t i;
  for (i = 0; i < 6; i++) {
    chr_bank[i] = chr_next[i];
    MMC_SEL = i;
    MMC_DAT = chr_bank[i];
  }
}
static void set_banks(uint8_t bg, uint8_t p1, uint8_t p2) {
  chr_next[0] = bg;
  chr_next[1] = bg + 2;
  chr_next[2] = p1;
  chr_next[3] = p2;
  chr_next[4] = BANK_BALL;
  chr_next[5] = BANK_EXTRA;
}
static void load_screen(const uint8_t *nt, const uint8_t *pal, uint8_t bg_bank,
                        uint8_t select) {
  uint16_t i;
  PPUCTRL = 0;
  PPUMASK = 0;
  ppu_ctrl = 0x88 | select;
  ppu_mask = 0;
  set_banks(bg_bank, BANK_PLAYERS, BANK_PLAYERS);
  apply_banks();
  (void)PPUSTAT;
  PPUADDR = 0x3F;
  PPUADDR = 0x00;
  for (i = 0; i < 32; i++)
    PPUDATA = pal[i];
  PPUADDR = 0x20 | (select << 2);
  PPUADDR = 0x00;
  for (i = 0; i < 1024; i++)
    PPUDATA = nt[i];
  for (i = 0; i < 64; i++)
    oam[i * 4] = 0xF0;
  last_sprites = 0;
  oi = 0;
  PPUCTRL = ppu_ctrl;
  ppu_mask = 0x1E;
}
static uint8_t vq_len;
static void vq_open(void) {
  while (vbuf_len)
    wait_frame();
  vq_len = 0;
}
static void vq_run(uint8_t row, uint8_t col, uint8_t count) {
  uint16_t a = 0x2000 + row * 32 + col;
  vbuf[vq_len++] = a >> 8;
  vbuf[vq_len++] = (uint8_t)a;
  vbuf[vq_len++] = count;
}
static void vq_commit(void) { vbuf_len = vq_len; }

/* Two-row text banner (glyph tiles over a solid red tile) in the sky. */
static uint8_t glyph_index(char c) {
  uint8_t i;
  static const char chars[] = GLYPH_CHARS;
  for (i = 0; i < GLYPH_COUNT; i++)
    if (chars[i] == c)
      return i;
  return 255;
}
static void banner(uint8_t row, const char *text) {
  uint8_t n = 0, col, i, k, g, half;
  while (text[n])
    n++;
  col = (32 - (n + 2)) >> 1;
  for (half = 0; half < 2; half++) {
    vq_run(row + half, col, n + 2);
    vbuf[vq_len++] = SOLID_RED;
    for (i = 0; i < n; i++) {
      g = glyph_index(text[i]);
      k = g == 255 ? SOLID_RED : glyph_tiles[g * 2 + half];
      vbuf[vq_len++] = k;
    }
    vbuf[vq_len++] = SOLID_RED;
  }
}
/* Restores the 16 tiles around the screen centre (every banner is narrower). */
static void unbanner(uint8_t row) {
  uint8_t half, i;
  for (half = 0; half < 2; half++) {
    vq_run(row + half, 8, 16);
    for (i = 8; i < 24; i++)
      vbuf[vq_len++] = game_nt[(row + half) * 32 + i];
  }
}
#define MSG_ROW 9
#define PAUSE_ROW 6
#define RESUME_ROW 9
#define QUIT_ROW 11
static void show_message(const char *text) {
  vq_open();
  banner(MSG_ROW, text);
  vq_commit();
}
static void hide_message(void) {
  vq_open();
  unbanner(MSG_ROW);
  vq_commit();
}

/* ---------------------------------------------------------------- input -- */
extern uint8_t pad_cur[2];
#pragma zpsym("pad_cur")
uint8_t take_edges(uint8_t pad); /* edges latched by the NMI poll, cleared on read */
static void poll_input(void) {
  uint8_t i;
  for (i = 0; i < 2; i++) {
    held[i] = pad_cur[i];
    edge[i] = take_edges(i);
    if (edge[i] & (PAD_A | PAD_B))
      queued[i] = 1;
  }
}
static void clear_actions(void) { queued[0] = queued[1] = 0; }

/* ------------------------------------------------------------- geometry -- */
static uint8_t alpha8(void) {
  uint16_t n = slow_segment ? (uint16_t)slow_phase * hz + acc : acc;
  uint16_t a = hz == 60 ? (n * 17) >> 2 : (n * 41) >> 3; /* n * 256 / hz */
  if (slow_segment)
    a /= 5;
  return a > 255 ? 255 : (uint8_t)a;
}
/* Unsigned magnitude math: cc65 shifts mixed signed products incorrectly. */
static int16_t blend(int16_t a, int16_t b, uint8_t t) {
  int16_t d = b - a;
  if (d >= 0)
    return a + (int16_t)(((uint16_t)d * t) >> 8);
  return a - (int16_t)(((uint16_t)(-d) * t) >> 8);
}
static uint8_t pose_of(const Player *p) {
  return p->state < 4 ? 5 * p->state + p->frame_number
         : p->state == 4 ? 17 + p->frame_number
                         : 18 + 5 * (p->state - 5) + p->frame_number;
}

/* -------------------------------------------------------------- sprites -- */
void draw_player(void);
void draw_ball(void);
#define DIGIT_SLOT 48   /* OAM slots 48..63 hold the score digits */
static uint8_t mx[NES_WORLD_WIDTH + 1], my[280]; /* world -> screen */
static void init_tables(void) {
  uint16_t i, v = 0; /* v = i * 19, built incrementally (no multiply) */
  for (i = 0; i <= NES_WORLD_WIDTH; i++) {
    mx[i] = (uint8_t)(v >> 5);
    if (i < sizeof(my))
      my[i] = (uint8_t)(NES_Y_OFFSET + (v >> 5));
    v += NES_SCALE_NUM;
  }
}
static int16_t bx_of(int16_t x) {
  return x < 0 ? 0 : x > NES_WORLD_WIDTH ? NES_WORLD_WIDTH : x;
}
static int16_t by_of(int16_t y) { return y < 0 ? 0 : y > 279 ? 279 : y; }
/* The PPU draws at most 8 sprites per scanline.  Two 4-tile-wide players plus the
 * 3-tile ball overflow when they overlap vertically, so the tiles that would be
 * dropped are chosen here (tail/edge columns first) instead of flickering. */
uint8_t pl_nib[8]; /* [player * 4 + row]: row column masks after trimming (draw.s) */
uint8_t pl_y[2], ball_top;
static uint8_t pl_x[2], pl_tile[2], pl_flip[2];
void trim_rows(void); /* draw.s */
static void prep_player(uint8_t i, uint8_t t) {
  const Player *p = i ? &game.player2 : &game.player1;
  uint8_t pose = pose_of(p), r;
  if (p->state == 3 || p->state == 4)
    pl_flip[i] = i == 0 ? p->diving_direction == -1 : p->diving_direction != 1;
  else
    pl_flip[i] = i;
  pl_x[i] = mx[bx_of(blend(prev_px[i], p->x, t))] - 16;
  pl_y[i] = my[by_of(blend(prev_py[i], p->y, t))] - 16 + NES_PLAYER_FOOT;
  pl_tile[i] = (i ? TILE_P2 : TILE_P1) + ((pose & 3) << 4);
  for (r = 0; r < 4; r++)
    pl_nib[i * 4 + r] = pose_mask[pose * 4 + r];
  chr_next[2 + i] = BANK_PLAYERS + (pose >> 2);
}
static void emit_player(uint8_t i) {
  dr_flip = pl_flip[i];
  dr_x = pl_x[i];
  dr_y = pl_y[i];
  dr_ptr = pl_nib + i * 4;
  dr_tile = pl_tile[i];
  draw_player();
}
static void put_ball(uint8_t t) {
  uint8_t f = game.ball.rotation;
  dr_bx = mx[bx_of(blend(prev_bx, game.ball.x, t))];
  if (dr_bx < 12) /* sprites cannot be placed at x < 0: keep the whole ball visible */
    dr_bx = 12;
  else if (dr_bx > 243)
    dr_bx = 243;
  dr_y = my[by_of(blend(prev_by, game.ball.y, t))];
  ball_top = (uint8_t)(dr_y - 12);
  dr_shy = my[273] + 1;
  dr_tile = TILE_BALL + (f > 4 ? 0 : f) * 9;
  draw_ball();
}
static void sprite_at(uint8_t slot, uint8_t x, uint8_t y, uint8_t tile, uint8_t attr) {
  uint8_t *o = oam + slot * 4;
  o[0] = y - 1;
  o[1] = tile;
  o[2] = attr;
  o[3] = x;
}
static void put_digit(uint8_t slot, uint8_t d, uint8_t x, uint8_t visible) {
  uint8_t k;
  for (k = 0; k < 4; k++) {
    if (visible)
      sprite_at(slot + k, x + (k & 1) * 8, 12 + (k >> 1) * 8, TILE_DIGITS + d * 4 + k, 2);
    else
      oam[(slot + k) * 4] = 0xF0;
  }
}
static void update_scores(void) {
  put_digit(DIGIT_SLOT, score[0] / 10, 8, score[0] >= 10);
  put_digit(DIGIT_SLOT + 4, score[0] % 10, score[0] >= 10 ? 24 : 8, 1);
  put_digit(DIGIT_SLOT + 8, score[1] / 10, 216, score[1] >= 10);
  put_digit(DIGIT_SLOT + 12, score[1] % 10, 232, 1);
}
/* Sand burst when the ball lands for a point: 8 particles, frame-driven. */
#define NPART 8
int16_t part_x[NPART], part_y[NPART]; /* screen position * 16 */
static int8_t part_vx[NPART], part_vy[NPART];
uint8_t part_life[NPART], burst_seed;
static void spawn_burst(int16_t wx) {
  uint8_t i;
  for (i = 0; i < NPART; i++) {
    burst_seed = burst_seed * 5 + 3 + nmi_frame;
    part_x[i] = (int16_t)mx[bx_of(wx)] * 16;
    part_y[i] = (int16_t)(my[273] - 6) * 16;
    part_vx[i] = (int8_t)((i - 4) * 5 + (burst_seed & 3));
    part_vy[i] = -(int8_t)(28 + (burst_seed & 15) + (i & 1) * 12);
    part_life[i] = 22;
  }
}
static void update_particles(void) {
  uint8_t i;
  for (i = 0; i < NPART; i++)
    if (part_life[i]) {
      part_x[i] += part_vx[i];
      part_y[i] += part_vy[i];
      part_vy[i] += 3;
      part_life[i]--;
    }
}
static void draw_particles(void) {
  uint8_t i;
  for (i = 0; i < NPART; i++)
    if (part_life[i] && oi < DIGIT_SLOT * 4) {
      sprite_at(oi >> 2, (part_x[i] >> 4) - 4, (part_y[i] >> 4) - 4, TILE_DUST, 3);
      oi += 4;
    }
}
static void draw_pole(void) {
  uint8_t i;
  for (i = 0; i < 3 && oi < DIGIT_SLOT * 4; i++) {
    sprite_at(oi >> 2, 124, 176 + i * 8, i == 2 ? TILE_POLE_END : TILE_POLE, 3);
    oi += 4;
  }
}
static void finish_sprites(void) {
  uint8_t i;
  for (i = oi; i < last_sprites; i += 4)
    oam[i] = 0xF0;
  last_sprites = oi;
  oam_ready = 1;
}
static void draw_game(void) {
  uint8_t t = alpha8();
  oi = 0;
  put_ball(t);
  prep_player(0, t);
  prep_player(1, t);
  trim_rows();
  emit_player(order);
  emit_player(order ^ 1);
  order ^= 1;
  draw_pole();
  draw_particles();
  if (paused && oi < DIGIT_SLOT * 4) {
    sprite_at(oi >> 2, 88, (pause_choice ? QUIT_ROW : RESUME_ROW) * 8 + 4, TILE_ARROW, 1);
    oi += 4;
  }
  finish_sprites();
}

/* ----------------------------------------------------------------- flow -- */
static void snapshot(void) {
  prev_px[0] = game.player1.x;
  prev_px[1] = game.player2.x;
  prev_py[0] = game.player1.y;
  prev_py[1] = game.player2.y;
  prev_bx = game.ball.x;
  prev_by = game.ball.y;
}
static void burst_off(void) {
  uint8_t i;
  for (i = 0; i < NPART; i++)
    part_life[i] = 0;
}
static void new_round(void) {
  physics_round(&game, serve_right);
  snapshot();
  round_scored = 0;
  slow_segment = 0;
  burst_off();
  clear_actions();
}
static void menu_open(void) {
  music_stop();
  screen = MENU;
  paused = 0;
  load_screen(menu_nt, menu_pal, BANK_MENU, 1);
  menu_choice = 0;
  clear_actions();
}
static void game_open(void) {
  load_screen(game_nt, game_pal, BANK_GAME, 0);
#ifdef PIKA_DEMO
  physics_init(&game, 1, 1, 777u); /* fixed: comparable lag runs */
#else
  physics_init(&game, 0, !two_players, nmi_frame * 257u + 12345u);
#endif
  screen = STARTING;
  timer = 0;
  paused = 0;
  serve_right = 0;
  score[0] = score[1] = 0;
  acc = 0;
  new_round();
  update_scores();
  show_message("START!");
#ifndef PIKA_NOMUSIC
  music_start();
#endif
}
uint8_t sfx_count[9]; /* diagnostics: effects requested, by id */
static void play(uint8_t id) {
  sfx_count[id]++;
  sfx_play(id);
}
static void sounds(void) {
  Player *p[2];
  uint8_t i;
  p[0] = &game.player1;
  p[1] = &game.player2;
  for (i = 0; i < 2; i++) {
    if (p[i]->sound.pipikachu)
      play(SFX_PIPIKACHU);
    if (p[i]->sound.pika)
      play(SFX_PIKA);
    if (p[i]->sound.chu)
      play(SFX_CHU);
    p[i]->sound.pipikachu = p[i]->sound.pika = p[i]->sound.chu = 0;
  }
  if (game.ball.sound.power_hit)
    play(SFX_POWER);
  if (game.ball.sound.ball_touches_ground)
    play(SFX_GROUND);
  if (game.ball.sound.hit)
    play(SFX_HIT);
  game.ball.sound.power_hit = game.ball.sound.ball_touches_ground = game.ball.sound.hit = 0;
}
static void tick(void) {
  PikaUserInput in[2];
  uint8_t i, ground, winner;
  if (screen == POINT) {
    if (++timer >= 45) {
      new_round();
      screen = READY;
      timer = 0;
      show_message("READY?");
      return;
    }
    if (!slow_segment) {
      slow_segment = 1;
      slow_phase = 0;
    } else if (++slow_phase < 5)
      return;
    else
      slow_phase = 0;
  }
  snapshot();
  if (screen == STARTING || screen == READY) {
    if (++timer >= (screen == STARTING ? 48u : 30u)) {
      screen = PLAYING;
      timer = 0;
      clear_actions();
      hide_message();
    }
    return;
  }
  for (i = 0; i < 2; i++) {
    in[i].x_direction = (held[i] & PAD_LEFT) ? -1 : (held[i] & PAD_RIGHT) ? 1 : 0;
    in[i].y_direction = (held[i] & PAD_UP) ? -1 : (held[i] & PAD_DOWN) ? 1 : 0;
    in[i].power_hit = queued[i];
  }
  clear_actions();
  ground = physics_tick(&game, in);
  sounds();
  if (screen == PLAYING && ground && !round_scored) {
    round_scored = 1;
    spawn_burst(game.ball.punch_effect_x);
    serve_right = game.ball.punch_effect_x < NES_WORLD_CENTER;
    winner = serve_right;
    score[winner]++;
    update_scores();
    timer = 0;
    if (score[winner] >= WIN_SCORE) {
      game.player1.game_ended = game.player2.game_ended = 1;
      game.player1.is_winner = winner == 0;
      game.player2.is_winner = winner == 1;
      screen = FINISHED;
      show_message("GAME OVER");
    } else
      screen = POINT;
  } else if (screen == FINISHED && ++timer >= 200)
    menu_open();
}
/* One banner per NMI batch: vbuf is small and the vblank budget is ~100 bytes. */
static void show_pause(void) {
  vq_open();
  banner(PAUSE_ROW, "PAUSE");
  vq_commit();
  vq_open();
  banner(RESUME_ROW, "RESUME");
  vq_commit();
  vq_open();
  banner(QUIT_ROW, "QUIT");
  vq_commit();
}
static void hide_pause(void) {
  vq_open();
  unbanner(PAUSE_ROW);
  vq_commit();
  vq_open();
  unbanner(RESUME_ROW);
  vq_commit();
  vq_open();
  unbanner(QUIT_ROW);
  vq_commit();
}
static void set_pause(uint8_t on) {
  paused = on;
  music_pause(on);
  if (on) {
    pause_choice = 0;
    show_pause();
  } else {
    hide_pause();
    /* the pause banners cover the centre text: restore the phase's message */
    if (screen == STARTING)
      show_message("START!");
    else if (screen == READY)
      show_message("READY?");
    else if (screen == FINISHED)
      show_message("GAME OVER");
  }
}
static void draw_menu(void) {
  oi = 0;
  sprite_at(0, 104, menu_choice ? 164 : 132, TILE_ARROW, 1);
  oi = 4;
  finish_sprites();
}


int main(void) {
  uint8_t i, gap;
  init_tables();
  hz = region_pal ? 50 : 60;
  for (i = 0; i < 64; i++)
    oam[i * 4] = 0xF0;
  menu_open();
  for (;;) {
    /* Frame lock-step: never re-sample the frame counter, so a long tick only
     * delays this frame's drawing instead of skipping a whole frame. */
    while (nmi_frame == last_frame) {
    }
    gap = nmi_frame - last_frame;
    last_frame = nmi_frame;
    if (gap > 1 && late_frames < 255)
      late_frames++;
    if (gap > max_gap)
      max_gap = gap;
    gap_hist[gap > 5 ? 5 : gap]++;
    if (gap > 4)
      gap = 4;
    poll_input();
#ifdef PIKA_DEMO
    if (screen == MENU && nmi_frame > 30)
      edge[0] |= PAD_START;
#endif
    if (screen == MENU) {
      if ((edge[0] | edge[1]) & (PAD_UP | PAD_DOWN)) {
        menu_choice ^= 1;
        sfx_play(SFX_MOVE);
      }
      if ((edge[0] | edge[1]) & (PAD_A | PAD_B | PAD_START)) {
        two_players = menu_choice;
        sfx_play(SFX_SELECT);
        game_open();
        continue;
      }
      draw_menu();
      continue;
    }
    if ((edge[0] | (two_players ? edge[1] : 0)) & PAD_START) {
      set_pause(!paused);
      clear_actions();
      continue;
    }
    if (paused) {
      if ((edge[0] | edge[1]) & (PAD_UP | PAD_DOWN))
        pause_choice ^= 1;
      if ((edge[0] | edge[1]) & (PAD_A | PAD_B)) {
        if (pause_choice)
          menu_open();
        else
          set_pause(0);
        continue;
      }
      draw_game();
      continue;
    }
    update_particles();
    draw_game(); /* first: the frame is ready long before the next vblank */
    acc += SIM_HZ * gap;
    while (acc >= hz && screen != MENU) {
      acc -= hz;
      tick();
    }
  }
}
