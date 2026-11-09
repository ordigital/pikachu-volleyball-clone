#include "physics.h"
#include "music.h"
#include "scene.h"
#include "resources.h"
#include <genesis.h>

/* H40, 64x32 maps, no unused desktop UI or configurable input layer. */
#define HZ_SIM 25
#define FX_COUNT 16
static PikaPhysics game, previous;
static u16 hz, accumulator, held[2], edges[2], queued[2];
static u16 score[2], timer, menu_choice, pause_choice, pause_owner;
static bool two_players, paused, serve_right, round_scored;
static enum { MENU, STARTING, PLAYING, POINT, READY, FINISHED } screen;
static Sprite *player_sp[2], *ball_sp, *digits_sp[4], *shadow_sp[3];
static Sprite *cloud_sp[2], *smoke_sp[4], *ghost_sp[3], *dust_sp[2], *message;
static u16 water_index, pause_index, pause_cursor_index;
#define PAUSE_X 10
#define PAUSE_Y 10
#define PAUSE_W 20
#define PAUSE_H 8
#define SPRITE_BIAS 128
static u32 visual_tick;
static bool slow_segment;
static u16 slow_phase;
static volatile u32 physics_steps;
static s16 squash_ticks;
static bool squash_power;
static u16 rng_fx = 0x8421;
static volatile u32 worst_tick_us, missed_frames, max_render_subticks,
    max_water_subticks, sim_steps;
static u32 last_vblank;
static u16 water_map[120];
static volatile u16 max_line_sprites, max_line_pixels, dropped_effects;
static u8 line_sprites[28];
static u16 line_pixels[28];
static s16 trail_x[8], trail_y[8];
static u16 trail_count;
typedef struct {
  Sprite *sprite;
  s16 x, y, previous_x, previous_y, vx, vy, gravity, age, life;
} Grain;
static Grain grains[FX_COUNT];
static struct {
  s16 x, y, age, scale;
} dust[2];
static u16 particle_cursor;

static s16 map_x(s32 x) { return divs(muls((s16)x, MD_SCALE_NUM), MD_SCALE_DEN); }
static s16 map_y(s32 y) { return MD_Y_OFFSET + map_x(y); }
static s16 blend(s32 a, s32 b) {
  u16 numerator = slow_segment ? slow_phase * hz + accumulator : accumulator;
  u16 denominator = slow_segment ? hz * 5 : hz;
  return (s16)(a + divs(muls((s16)(b - a), numerator), denominator));
}
static u16 random_fx(void) {
  rng_fx ^= rng_fx << 7;
  rng_fx ^= rng_fx >> 9;
  rng_fx ^= rng_fx << 8;
  return rng_fx;
}
static void hide(Sprite *s) {
  if (s)
    SPR_setVisibility(s, HIDDEN);
}
static void place(Sprite *s, s16 x, s16 y) {
  if (s) {
    SPR_setPosition(s, x, y);
    SPR_setVisibility(s, VISIBLE);
  }
}
static Sprite *add(const SpriteDefinition *def, u16 pal) {
  Sprite *s = SPR_addSprite(def, 0, 0, TILE_ATTR(pal, TRUE, FALSE, FALSE));
  if (!s) {
    VDP_drawText("SPRITE MEMORY FULL", 1, 1);
    while (TRUE)
      SYS_doVBlankProcess();
  }
  hide(s);
  return s;
}
static void sfx(u16 i) {
  static const u8 *const data[] = {sfx0_sample, sfx1_sample, sfx2_sample,
                                   sfx3_sample, sfx4_sample, sfx5_sample,
                                   sfx6_sample};
  static const u32 size[] = {sizeof(sfx0_sample), sizeof(sfx1_sample),
                             sizeof(sfx2_sample), sizeof(sfx3_sample),
                             sizeof(sfx4_sample), sizeof(sfx5_sample),
                             sizeof(sfx6_sample)};
  static const u8 priority[] = {4, 2, 1, 0, 4, 3, 5};
  static u16 current;
  SYS_disableInts();
  if (!SND_PCM4_isPlaying(SOUND_PCM_CH2_MSK) ||
      priority[i] >= priority[current]) {
    SND_PCM4_startPlay(data[i], size[i], SOUND_PCM_CH2, FALSE);
    current = i;
  }
  SYS_enableInts();
}
static void input_poll(void) {
  for (u16 i = 0; i < 2; i++) {
    u16 now = JOY_readJoypad(i == 0 ? JOY_1 : JOY_2);
    edges[i] = now & ~held[i];
    held[i] = now;
    queued[i] |= edges[i] & BUTTON_B;
  }
}
static void clear_actions(void) { queued[0] = queued[1] = 0; }
static void show_pause(void) {
  VDP_setTileMapEx(BG_A, pause_panel.tilemap,
                  TILE_ATTR_FULL(PAL3, TRUE, FALSE, FALSE, pause_index),
                  PAUSE_X, PAUSE_Y, 0, 0, PAUSE_W, PAUSE_H, CPU);
  for (u16 option = 0; option < 2; option++)
    VDP_setTileMapXY(BG_A,
                    TILE_ATTR_FULL(PAL3, TRUE, FALSE, FALSE,
                                   pause_cursor_index + (pause_choice == option)),
                    PAUSE_X + 5, PAUSE_Y + 3 + option * 2);
}
static void set_pause(bool value) {
  paused = value;
  clear_actions();
  music_pause(value);
  if (value) {
    /* High-priority sprites beat high-priority planes: hide only those over
     * the dialog. draw_game restores their current visibility on resume. */
    for (Sprite *s = firstSprite; s; s = s->next)
      if (s->x < SPRITE_BIAS + (PAUSE_X + PAUSE_W) * 8 &&
          s->x + s->definition->w > SPRITE_BIAS + PAUSE_X * 8 &&
          s->y < SPRITE_BIAS + (PAUSE_Y + PAUSE_H) * 8 &&
          s->y + s->definition->h > SPRITE_BIAS + PAUSE_Y * 8)
        hide(s);
    SPR_update();
    pause_choice = 0;
    show_pause();
  } else
    VDP_clearTileMapRect(BG_A, PAUSE_X, PAUSE_Y, PAUSE_W, PAUSE_H);
}
static void reset_fx(void) {
  slow_segment = false;
  slow_phase = 0;
  trail_count = 0;
  squash_ticks = 0;
  visual_tick = 0;
  for (u16 i = 0; i < FX_COUNT; i++)
    grains[i].life = 0;
  for (u16 i = 0; i < 2; i++)
    dust[i].age = 30;
}
static void load_image(const Image *image) {
  VDP_setEnable(FALSE);
  VDP_clearPlane(BG_A, TRUE);
  VDP_clearPlane(BG_B, TRUE);
  if (image->tileset->numTile > TILE_USER_LENGTH) {
    VDP_setEnable(TRUE);
    VDP_drawText("BACKGROUND VRAM OVERFLOW", 1, 1);
    while (TRUE)
      SYS_doVBlankProcess();
  }
  VDP_drawImageEx(BG_B, image,
                  TILE_ATTR_FULL(PAL0, FALSE, FALSE, FALSE, TILE_USER_INDEX), 0,
                  0, TRUE, FALSE);
  VDP_setEnable(TRUE);
}
static void menu_open(void) {
  music_stop();
  paused = false;
  SPR_end();
  memset(player_sp, 0, sizeof(player_sp));
  message = NULL;
  screen = MENU;
  timer = 0;
  menu_choice = 0;
  accumulator = 0;
  clear_actions();
  load_image(&menu);
  VDP_drawImageEx(BG_A, &menu_art,
                  TILE_ATTR_FULL(PAL2, FALSE, FALSE, FALSE,
                                 TILE_USER_INDEX + menu.tileset->numTile),
                  0, 0, FALSE, FALSE);
  /* IMAGE exports 64 colors: loading at PAL2 would wrap and erase PAL0. */
  PAL_setPalette(PAL2, menu_art.palette->data, CPU);
  PAL_setPalette(PAL3, menu_art.palette->data + 16, CPU);
  VDP_drawText(">", 18, 17);

}
static void new_round(void) {
  physics_round(&game, serve_right);
  previous = game;
  round_scored = false;
  reset_fx();
  clear_actions();
}
static void game_open(void) {
  screen = STARTING;
  timer = 0;
  paused = false;
  serve_right = false;
  score[0] = score[1] = 0;
  round_scored = false;
  clear_actions();
  accumulator = 0;
  physics_init(&game, false, !two_players,
               UINT64_C(0x53C012341234) + getTick());
  previous = game;
  SPR_initEx(MD_SPRITE_TILES);
  load_image(&court);
  PAL_setPalette(PAL1, dust_sprite.palette->data, CPU);
  PAL_setPalette(PAL2, players_sprite.palette->data, CPU);
  PAL_setPalette(PAL3, smoke_sprite.palette->data, CPU);
  water_index = TILE_USER_INDEX + court.tileset->numTile;
  pause_index = water_index + water_tiles.numTile;
  pause_cursor_index = pause_index + pause_panel.tileset->numTile;
  if (pause_cursor_index + pause_cursor_tiles.numTile > TILE_SPRITE_INDEX) {
    VDP_drawText("BACKGROUND VRAM OVERFLOW", 1, 1);
    while (TRUE)
      SYS_doVBlankProcess();
  }
  VDP_loadTileSet(&water_tiles, water_index, CPU);
  VDP_loadTileSet(pause_panel.tileset, pause_index, CPU);
  VDP_loadTileSet(&pause_cursor_tiles, pause_cursor_index, CPU);
  for (u16 i = 0; i < 2; i++) {
    player_sp[i] = add(&players_sprite, PAL2);
    SPR_setHFlip(player_sp[i], i == 1);
    cloud_sp[i] = add(&cloud_sprite, PAL3);
    SPR_setPriority(cloud_sp[i], FALSE);
    dust_sp[i] = add(&dust_sprite, PAL1);
    SPR_setDepth(dust_sp[i], 100);
    dust[i].age = 30;
  }
  ball_sp = add(&ball_sprite, PAL2);
  SPR_setDepth(ball_sp, -100);
  for (u16 i = 0; i < 4; i++) {
    digits_sp[i] = add(&digits_sprite, PAL3);
    SPR_setDepth(digits_sp[i], -200);
    smoke_sp[i] = add(&smoke_sprite, PAL3);
    SPR_setDepth(smoke_sp[i], 150);
  }
  for (u16 i = 0; i < 3; i++) {
    ghost_sp[i] = add(&ghost_sprite, PAL2);
    SPR_setFrame(ghost_sp[i], i);
    SPR_setDepth(ghost_sp[i], 120);
    shadow_sp[i] = add(&shadow_sprite, PAL1);
    SPR_setDepth(shadow_sp[i], 180);
  }
  for (u16 i = 0; i < FX_COUNT; i++) {
    grains[i].sprite = add(&sand_sprite, PAL1);
    SPR_setDepth(grains[i].sprite, 90);
  }
  message = add(&start_sprite, PAL3);
  SPR_setDepth(message, -150);
  reset_fx();
  music_start();
  last_vblank = vtimer;
}
static void burst(s16 x, s16 y, u16 scale, u16 count) {
  /* Fixed-point fountain over the ball width; separate visual RNG. */
  for (u16 i = 0; i < count; i++) {
    Grain *g = &grains[particle_cursor++ % FX_COUNT];
    g->x = x * 16 + ((s16)(random_fx() % (MD_BALL_DIAMETER * 16)) - MD_BALL_DIAMETER * 8) * (s16)scale / 100;
    g->y = y * 16;
    g->previous_x = g->x;
    g->previous_y = g->y;
    g->vx = ((s16)(random_fx() % 65) - 32) * (s16)scale / 100;
    g->vy = -(30 + (s16)(random_fx() % 55)) * (s16)scale / 100;
    g->gravity = scale / 20;
    g->age = 0;
    g->life = 18;
    SPR_setFrame(g->sprite, scale >= 50 ? 1 : 0);
  }
  u16 d = dust[0].age > dust[1].age ? 0 : 1;
  dust[d].x = x;
  dust[d].y = y;
  dust[d].age = 0;
  dust[d].scale = scale == 100 ? 0 : scale == 50 ? 1 : 2;
}
static void sounds(void) {
  Player *p[2] = {&game.player1, &game.player2};
  for (u16 i = 0; i < 2; i++) {
    if (p[i]->sound.pipikachu)
      sfx(0);
    if (p[i]->sound.pika)
      sfx(1);
    if (p[i]->sound.chu)
      sfx(2);
    p[i]->sound = (PlayerSound){0};
  }
  if (game.ball.sound.power_hit)
    sfx(5);
  if (game.ball.sound.ball_touches_ground)
    sfx(6);
  game.ball.sound = (BallSound){0};
}
static void effects_tick(bool ground) {
  visual_tick++;
  if (squash_ticks > 0)
    squash_ticks--;
  bool collide = game.ball.world_bounce ||
                 (!previous.player1.is_collision_with_ball_happened &&
                  game.player1.is_collision_with_ball_happened) ||
                 (!previous.player2.is_collision_with_ball_happened &&
                  game.player2.is_collision_with_ball_happened);
  if (collide || game.ball.sound.power_hit) {
    squash_ticks = 6;
    squash_power = game.ball.sound.power_hit;
  }
  if (ground)
    burst(map_x(game.ball.x), map_y(272), 100, 12);
  Player *now[2] = {&game.player1, &game.player2},
         *old[2] = {&previous.player1, &previous.player2};
  for (u16 i = 0; i < 2; i++) {
    if (now[i]->y == 244 && old[i]->y < 244)
      burst(map_x(now[i]->x), map_y(273), 50, 5);
    else if (now[i]->y == 244 && now[i]->state == 0 && now[i]->x != old[i]->x &&
             visual_tick % 4 == 0)
      burst(map_x(now[i]->x), map_y(273), 20, 2);
  }
  for (u16 i = 0; i < FX_COUNT; i++) {
    Grain *g = &grains[i];
    if (g->life > g->age) {
      g->previous_x = g->x;
      g->previous_y = g->y;
      g->x += g->vx;
      g->y += g->vy;
      g->vy += g->gravity;
      g->age++;
    }
  }
  for (u16 i = 0; i < 2; i++)
    if (dust[i].age < 8)
      dust[i].age++;
  for (u16 i = 7; i > 0; i--) {
    trail_x[i] = trail_x[i - 1];
    trail_y[i] = trail_y[i - 1];
  }
  trail_x[0] = map_x(game.ball.x);
  trail_y[0] = map_y(game.ball.y);
  if (game.ball.is_power_hit) {
    if (trail_count < 8)
      trail_count++;
  } else if (trail_count)
    trail_count--;
}
static u16 player_frame(const Player *p) {
  return p->state < 4    ? 5 * p->state + p->frame_number
         : p->state == 4 ? 17 + p->frame_number
                         : 18 + 5 * (p->state - 5) + p->frame_number;
}
static void draw_water(void) {
  u32 started = getSubTick();
  /* 7.2 s tide, foam travels sideways; only the bottom three tile rows change.
   */
  u16 phase = visual_tick % 180;
  s16 crest = phase < 90 ? 224 - (s16)phase * (224 - MD_WATER_CREST) / 90
                         : MD_WATER_CREST + (s16)(phase - 90) * (224 - MD_WATER_CREST) / 90;
  for (u16 row = 25; row < 28; row++)
    for (u16 x = 0; x < 40; x++) {
      s16 y = row * 8;
      u16 tile = 0;
      if (y + 8 > crest)
        tile = TILE_ATTR_FULL(PAL0, FALSE, FALSE, FALSE,
                              water_index + (y < crest
                                                 ? ((u32)(crest - y) * 8 +
                                                    ((visual_tick / 2 + x) & 7))
                                                 : (u32)(64 + row - 25)));
      water_map[(row - 25) * 40 + x] = tile;
    }
  VDP_setTileMapDataRect(BG_A, water_map, 0, 25, 40, 3, 40, DMA_QUEUE);
  u32 elapsed = getSubTick() - started;
  if (elapsed > max_water_subticks)
    max_water_subticks = elapsed;
}
/* Conservative scanline admission for decorative sprites. Main actors/UI
 * reserve first; effects disappear by priority before the VDP could drop a
 * player/ball. */
static bool reserve_rect(s16 y, u16 w, u16 h, bool critical) {
  s16 top = y < 0 ? 0 : y / 8, bottom = (y + (s16)h + 7) / 8;
  if (bottom > 28)
    bottom = 28;
  u16 count = (w + 31) / 32;
  if (!critical)
    for (s16 row = top; row < bottom; row++) {
      if (line_sprites[row] + count > 19 || line_pixels[row] + w > 312) {
        dropped_effects++;
        return FALSE;
      }
    }
  for (s16 row = top; row < bottom; row++) {
    line_sprites[row] += count;
    line_pixels[row] += w;
  }
  return TRUE;
}
static void effect_place(Sprite *s, s16 x, s16 y, u16 w, u16 h) {
  if (reserve_rect(y, w, h, FALSE))
    place(s, x, y);
  else
    hide(s);
}
static void draw_game(void) {
  memset(line_sprites, 0, sizeof(line_sprites));
  memset(line_pixels, 0, sizeof(line_pixels));
  const Player *players[2] = {&game.player1, &game.player2},
               *old[2] = {&previous.player1, &previous.player2};
  s16 px[2], py[2];
  for (u16 i = 0; i < 2; i++) {
    px[i] = map_x(blend(old[i]->x, players[i]->x));
    py[i] = map_y(blend(old[i]->y, players[i]->y));
    reserve_rect(py[i] - MD_PLAYER_CELL / 2, MD_PLAYER_CELL, MD_PLAYER_CELL, TRUE);
    bool flip = (players[i]->state == 3 || players[i]->state == 4)
                    ? (i == 0 ? players[i]->diving_direction == -1
                              : players[i]->diving_direction != 1)
                    : i == 1;
    SPR_setHFlip(player_sp[i], flip);
    SPR_setFrame(player_sp[i], player_frame(players[i]));
    place(player_sp[i], px[i] - MD_PLAYER_CELL / 2, py[i] - MD_PLAYER_CELL / 2);
  }
  s16 bx = map_x(blend(previous.ball.x, game.ball.x)),
      by = map_y(blend(previous.ball.y, game.ball.y));
  u16 row = 0;
  if (squash_ticks) {
    if (squash_power)
      row = squash_ticks >= 5   ? 4
            : squash_ticks >= 3 ? 3
            : squash_ticks == 2 ? 2
                                : 5;
    else
      row = squash_ticks > 2 ? 1 : 5;
  }
  SPR_setAnimAndFrame(ball_sp, row, (game.ball.fine_rotation * 16 / 50) % 16);
  place(ball_sp, bx - MD_BALL_WIDTH / 2, by - MD_BALL_HEIGHT / 2);
  reserve_rect(by - MD_BALL_HEIGHT / 2, MD_BALL_WIDTH, MD_BALL_HEIGHT, TRUE);
  for (u16 side = 0; side < 2; side++) {
    u16 value = score[side];
    Sprite *t = digits_sp[side * 2], *u = digits_sp[side * 2 + 1];
    if (value >= 10) {
      SPR_setFrame(t, value / 10);
      place(t, side ? 248 : 8, 8);
      reserve_rect(8, 32, 48, TRUE);
    } else
      hide(t);
    SPR_setFrame(u, value % 10);
    place(u, side ? 280 : value >= 10 ? 40 : 8, 8);
    reserve_rect(8, 32, 48, TRUE);
  }
  if (screen == STARTING) {
    SPR_setAnim(message, timer < 24 ? timer / 3 : 8);
    place(message, 96, 101);
    reserve_rect(101, 128, 32, TRUE);
  } else if (screen == READY && (timer / 5) % 2 == 0) {
    place(message, 96, 101);
    reserve_rect(101, 128, 32, TRUE);
  } else if (screen == FINISHED) {
    s16 jitter = timer > 50 ? (s16)(random_fx() % 5) - 2 : 0;
    SPR_setAnim(message, timer < 15 ? 0 : timer < 35 ? 1 : 2);
    s16 message_y = 87 + (timer > 50 ? (s16)(random_fx() % 5) - 2 : 0);
    place(message, 40 + jitter, message_y);
    reserve_rect(message_y, gameover_sprite.w, gameover_sprite.h, TRUE);
  } else
    hide(message);
  for (u16 i = 0; i < 3; i++)
    effect_place(shadow_sp[i], (i < 2 ? px[i] : bx) - MD_SHADOW_WIDTH / 2, map_y(273) - 4, MD_SHADOW_WIDTH,
                 8);
  for (u16 i = 0; i < 2; i++) {
    if (dust[i].age < 8) {
      SPR_setAnimAndFrame(dust_sp[i], dust[i].scale, dust[i].age / 2);
      effect_place(dust_sp[i], dust[i].x - 16, dust[i].y - 18, 32, 24);
    } else
      hide(dust_sp[i]);
  }
  for (u16 i = 0; i < 4; i++) {
    u16 at = i + 1;
    if (screen == FINISHED && timer > 50) {
      SPR_setFrame(smoke_sp[i], i);
      effect_place(smoke_sp[i], 60 + i * 56 + (s16)(random_fx() % 7),
                   95 + (s16)(random_fx() % 7), 32, 24);
    } else if (trail_count > at) {
      SPR_setFrame(smoke_sp[i], i);
      effect_place(smoke_sp[i], trail_x[at] - 16, trail_y[at] - 12, 32, 24);
    } else
      hide(smoke_sp[i]);
  }
  for (u16 i = 0; i < 3; i++) {
    u16 at = i * 2 + 2;
    if (trail_count > at)
      effect_place(ghost_sp[i], trail_x[at] - MD_BALL_WIDTH / 2,
                   trail_y[at] - MD_BALL_HEIGHT / 2, MD_BALL_WIDTH, MD_BALL_HEIGHT);
    else
      hide(ghost_sp[i]);
  }
  for (u16 n = 0; n < FX_COUNT; n++) {
    u16 i = (particle_cursor + FX_COUNT - 1 - n) % FX_COUNT;
    Grain *g = &grains[i];
    if (g->age < g->life)
      effect_place(g->sprite,
                   blend(g->previous_x, g->x) / 16 - 4,
                   blend(g->previous_y, g->y) / 16 - 4, 8, 8);
    else
      hide(g->sprite);
  }
  for (u16 i = 0; i < 2; i++)
    effect_place(cloud_sp[i],
                 (s16)((visual_tick / (i + 2) + i * 170) % 416) - 48,
                 54 + i * 24, 48, 24);
  for (u16 band = 0; band < 28; band++) {
    if (line_sprites[band] > max_line_sprites)
      max_line_sprites = line_sprites[band];
    if (line_pixels[band] > max_line_pixels)
      max_line_pixels = line_pixels[band];
  }
  SPR_update();
}
static void tick(void) {
  sim_steps++;
  if (screen == POINT) {
    /* The point phase lasts 45 normal ticks; its world advances at one fifth
     * speed. Finish the landing's normal interpolation before stretching steps. */
    if (++timer >= 45) {
      new_round();
      screen = READY;
      timer = 0;
      SPR_setDefinition(message, &ready_sprite);
      draw_water();
      return;
    }
    if (!slow_segment) {
      slow_segment = true;
      slow_phase = 0;
    } else if (++slow_phase < 5)
      return;
    else
      slow_phase = 0;
  }
  previous = game;
  if (screen == STARTING || screen == READY) {
    visual_tick++;
    timer++;
    if (timer >= (screen == STARTING ? 48 : 30)) {
      screen = PLAYING;
      timer = 0;
      clear_actions();
    }
  } else {
    PikaUserInput in[2];
    for (u16 i = 0; i < 2; i++) {
      in[i].x_direction = (held[i] & BUTTON_LEFT)    ? -1
                          : (held[i] & BUTTON_RIGHT) ? 1
                                                     : 0;
      in[i].y_direction = (held[i] & BUTTON_UP)     ? -1
                          : (held[i] & BUTTON_DOWN) ? 1
                                                    : 0;
      in[i].power_hit = queued[i] != 0;
    }
    clear_actions();
    u32 start = getSubTick();
    physics_steps++;
    bool ground = physics_tick(&game, in);
    u32 elapsed = (getSubTick() - start) * 10000 / 768;
    if (elapsed > worst_tick_us)
      worst_tick_us = elapsed;
    effects_tick(ground);
    sounds();
    if (screen == PLAYING && ground && !round_scored) {
      round_scored = true;
      serve_right = game.ball.punch_effect_x < MD_WORLD_CENTER;
      u16 winner = serve_right ? 1 : 0;
      score[winner]++;
      timer = 0;
      if (score[winner] >= 15) {
        game.player1.game_ended = game.player2.game_ended = true;
        game.player1.is_winner = winner == 0;
        game.player2.is_winner = winner == 1;
        screen = FINISHED;
        SPR_setDefinition(message, &gameover_sprite);
      } else
        screen = POINT;
    } else if (screen == FINISHED && ++timer >= 200) {
      menu_open();
      return;
    }
  }
  draw_water();
}
int main(bool hardReset) {
  VDP_setScreenWidth320();
  VDP_setScreenHeight224();
  VDP_setPlaneSize(64, 32, FALSE);
  VDP_setBGAAddress(0xC000);
  VDP_setWindowAddress(0xD000);
  VDP_setBGBAddress(0xE000);
  VDP_setSpriteListAddress(0xF000);
  VDP_setHScrollTableAddress(0xF400);
  VDP_setTextPlane(BG_A);
  VDP_setTextPalette(PAL3);
  VDP_setTextPriority(TRUE);
  hz = SYS_isPAL() ? 50 : 60;
  JOY_init();
  music_init();
  menu_open();
  while (TRUE) {
    SYS_doVBlankProcess();
    u32 elapsed_frames = vtimer - last_vblank;
    last_vblank = vtimer;
    input_poll();
    if (screen == MENU) {
      if ((edges[0] | edges[1]) & (BUTTON_UP | BUTTON_DOWN)) {
        menu_choice ^= 1;
        sfx(3);
      }
      VDP_drawText(menu_choice ? " " : ">", 18, 17);
      VDP_drawText(menu_choice ? ">" : " ", 18, 21);
      if ((edges[0] | edges[1]) & BUTTON_B) {
        two_players = menu_choice != 0;
        sfx(4);
        game_open();
      }
      clear_actions();
      continue;
    }
    u16 active = two_players ? 2 : 1;
    bool start_edge = false;
    u16 owner = 0;
    for (u16 i = 0; i < active; i++)
      if (edges[i] & BUTTON_START) {
        start_edge = true;
        owner = i;
        break;
      }
    if (start_edge) {
      if (!paused)
        pause_owner = owner;
      set_pause(!paused);
      continue;
    }
    if (paused) {
      if (edges[pause_owner] & (BUTTON_UP | BUTTON_DOWN)) {
        pause_choice ^= 1;
        show_pause();
      }
      if (edges[pause_owner] & BUTTON_B) {
        if (pause_choice)
          menu_open();
        else
          set_pause(false);
      }
      clear_actions();
      continue;
    }
    accumulator += (u16)(elapsed_frames > 5 ? 5 : elapsed_frames) * HZ_SIM;
    while (accumulator >= hz && screen != MENU) {
      accumulator -= hz;
      tick();
    }
    if (screen != MENU) {
      u32 before_frame = vtimer, started = getSubTick();
      draw_game();
      u32 elapsed = getSubTick() - started;
      if (elapsed > max_render_subticks)
        max_render_subticks = elapsed;
      if (vtimer != before_frame)
        missed_frames++;
    }
  }
  return 0;
}
