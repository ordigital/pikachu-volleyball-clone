/* 16-bit adaptation of ports/megadrive/src/physics.c (itself the C port of
 * src/physics.rs).  Update order, bounds and integer arithmetic are unchanged.
 * Differences, all behavior-neutral for the simulation state:
 *  - 16-bit fields (every value fits; verified by tests/parity.py),
 *  - a 16-bit xorshift RNG instead of SplitMix64 (PIKA_REF_RNG restores it for
 *    the host parity test),
 *  - the landing prediction runs once per tick (the second MD call recomputed
 *    an identical value) and only when a computer player exists. */
#include "physics.h"
#include "scene.h"
#ifdef PIKA_HOST
#include <string.h>
#endif
#define WIDTH NES_WORLD_WIDTH
#define HALF NES_WORLD_CENTER
#define GROUND 244
#define RADIUS 20
#define LIMIT 1000
#ifdef PIKA_REF_RNG
static uint64_t rng;
static int16_t random15(void) {
  uint64_t z = (rng += 0x9E3779B97F4A7C15ULL);
  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
  z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
  z ^= z >> 31;
  return (int16_t)((z >> 33) % 32768);
}
#else
static uint16_t rng = 1;
static int16_t random15(void) {
  rng ^= rng << 7;
  rng ^= rng >> 9;
  rng ^= rng << 8;
  return (int16_t)(rng & 0x7fff);
}
#endif
static uint8_t has_ai;
static int16_t magnitude(int16_t x) { return x < 0 ? -x : x; }
static int16_t bounded(int16_t x, int16_t a, int16_t b) {
  return x < a ? a : x > b ? b : x;
}
static void player_round(Player *p) {
  p->x = p->is_player2 ? WIDTH - 36 : 36;
  p->y = GROUND;
  p->y_velocity = 0;
  p->is_collision_with_ball_happened = 0;
  p->state = p->frame_number = p->delay_before_next_frame = 0;
  p->normal_status_arm_swing_direction = 1;
  p->computer_boldness = random15() % 5;
}
static void ball_round(Ball *b, uint8_t right) {
  b->x = right ? WIDTH - 56 : 56;
  b->y = 0;
  b->x_velocity = 0;
  b->y_velocity = 1;
  b->punch_effect_radius = 0;
  b->is_power_hit = 0;
}
void physics_round(PikaPhysics *p, uint8_t right) {
  player_round(&p->player1);
  player_round(&p->player2);
  ball_round(&p->ball, right);
}
void physics_init(PikaPhysics *p, uint8_t ai1, uint8_t ai2, uint16_t seed) {
#ifdef PIKA_HOST
  memset(p, 0, sizeof(*p));
#ifdef PIKA_REF_RNG
  rng = seed;
#else
  rng = seed ? seed : 1;
#endif
#else
  uint8_t *q = (uint8_t *)p, i;
  for (i = 0; i < sizeof(*p); i++)
    q[i] = 0;
  rng = seed ? seed : 1;
#endif
  has_ai = ai1 || ai2;
  p->player1.is_computer = ai1;
  p->player2.is_computer = ai2;
  p->player2.is_player2 = 1;
  p->player1.lying_down_duration_left = p->player2.lying_down_duration_left =
      -1;
  physics_round(p, 0);
}
static uint8_t world(Ball *b) {
  int16_t rotation, x, y;
  /* cc65 turns signed /2 into an arithmetic shift (floors); C truncates. */
  rotation = b->fine_rotation +
             (b->x_velocity < 0 ? -(-b->x_velocity >> 1) : b->x_velocity >> 1);
  if (rotation < 0)
    rotation += 50;
  else if (rotation > 50)
    rotation -= 50;
  b->fine_rotation = rotation;
  b->rotation = (uint8_t)(rotation / 10);
  x = b->x + b->x_velocity;
  if (x < RADIUS || x > WIDTH)
    b->x_velocity = -b->x_velocity;
  if (b->y + b->y_velocity < 0)
    b->y_velocity = 1;
  if (magnitude(b->x - HALF) < 25 && b->y > 176) {
    if (b->y <= 192) {
      if (b->y_velocity > 0)
        b->y_velocity = -b->y_velocity;
    } else
      b->x_velocity =
          b->x < HALF ? -magnitude(b->x_velocity) : magnitude(b->x_velocity);
  }
  y = b->y + b->y_velocity;
  if (y > 252) {
    b->sound.ball_touches_ground = 1;
    b->y_velocity = -b->y_velocity;
    b->punch_effect_x = b->x;
    b->y = 252;
    b->punch_effect_radius = RADIUS;
    b->punch_effect_y = 272;
    return 1;
  }
  b->y = y;
  b->x += b->x_velocity;
  b->y_velocity++;
  return 0;
}
/* Trajectory loops: C reference for the host tests, assembly (predict.s) in the
 * ROM.  Both are checked against each other by tests/physics_rom.py. */
#ifdef PIKA_HOST
static void predict(Ball *b) {
  int16_t x = b->x, y = b->y, vx = b->x_velocity, vy = b->y_velocity, n, next;
  for (n = 1;; n++) {
    next = x + vx;
    if (next < RADIUS || next > WIDTH)
      vx = -vx;
    if (y + vy < 0)
      vy = 1;
    if (magnitude(x - HALF) < 25 && y > 176) {
      if (y < 192) {
        if (vy > 0)
          vy = -vy;
      } else
        vx = x < HALF ? -magnitude(vx) : magnitude(vx);
    }
    y += vy;
    if (y > 252 || n >= LIMIT)
      break;
    x += vx;
    vy++;
  }
  b->expected_landing_point_x = x;
}
static int16_t predict_power(int16_t dx, int16_t dy, const Ball *b) {
  int16_t x = b->x, y = b->y, vy = magnitude(b->y_velocity) * dy * 2, n, next;
  int16_t vx = (magnitude(dx) + 1) * 10;
  if (x >= HALF)
    vx = -vx;
  for (n = 1;; n++) {
    next = x + vx;
    if (next < RADIUS || next > WIDTH)
      vx = -vx;
    if (y + vy < 0)
      vy = 1;
    /* Intentionally does not predict side-of-net collision. */
    if (magnitude(x - HALF) < 25 && y > 176 && vy > 0)
      vy = -vy;
    y += vy;
    if (y > 252 || n >= LIMIT)
      return x;
    x += vx;
    vy++;
  }
}
#else
extern int16_t pr_x, pr_y, pr_vx, pr_vy;
#pragma zpsym("pr_x")
#pragma zpsym("pr_y")
#pragma zpsym("pr_vx")
#pragma zpsym("pr_vy")
void pr_land(void);
void pr_power(void);
static void predict(Ball *b) {
  pr_x = b->x;
  pr_y = b->y;
  pr_vx = b->x_velocity;
  pr_vy = b->y_velocity;
  pr_land();
  b->expected_landing_point_x = pr_x;
}
static int16_t predict_power(int16_t dx, int16_t dy, const Ball *b) {
  pr_x = b->x;
  pr_y = b->y;
  pr_vy = magnitude(b->y_velocity) * dy * 2;
  pr_vx = (magnitude(dx) + 1) * 10;
  if (pr_x >= HALF)
    pr_vx = -pr_vx;
  pr_power();
  return pr_x;
}
#endif
static uint8_t choose_power(const Player *p, const Ball *b, const Player *other,
                            PikaUserInput *in) {
  int16_t side = p->is_player2, dx, n, dy, landing;
  uint8_t descending = random15() % 2 == 0;
  for (dx = 1; dx >= 0; dx--) {
    for (n = 0; n < 3; n++) {
      dy = descending ? n - 1 : 1 - n;
      landing = predict_power(dx, dy, b);
      if ((landing <= side * HALF || landing >= side * WIDTH + HALF) &&
          magnitude(landing - other->x) > 64) {
        in->x_direction = (int8_t)dx;
        in->y_direction = (int8_t)dy;
        return 1;
      }
    }
  }
  return 0;
}
static void ai(Player *p, const Ball *b, const Player *other,
               PikaUserInput *in) {
  int16_t side = p->is_player2, landing = b->expected_landing_point_x,
          bold = p->computer_boldness, left, right;
  in->x_direction = in->y_direction = in->power_hit = 0;
  if (magnitude(b->x - p->x) > 100 && magnitude(b->x_velocity) < bold + 5) {
    left = side * HALF;
    if ((landing <= left || landing >= side * WIDTH + HALF) &&
        p->computer_where_to_stand_by == 0)
      landing = left + HALF / 2;
  }
  if (magnitude(landing - p->x) > bold + 8)
    in->x_direction = p->x < landing ? 1 : -1;
  else if (random15() % 20 == 0)
    p->computer_where_to_stand_by = random15() % 2;
  if (p->state == 0) {
    if (magnitude(b->x_velocity) < bold + 3 && magnitude(b->x - p->x) < 32 &&
        b->y > -36 && b->y < 10 * bold + 84 && b->y_velocity > 0)
      in->y_direction = -1;
    left = side * HALF;
    right = (side + 1) * HALF;
    if (b->expected_landing_point_x > left &&
        b->expected_landing_point_x < right &&
        magnitude(b->x - p->x) > bold * 5 + 64 && b->x > left && b->x < right &&
        b->y > 174) {
      in->power_hit = 1;
      in->x_direction = p->x < b->x ? 1 : -1;
    }
  } else if (p->state == 1 || p->state == 2) {
    if (magnitude(b->x - p->x) > 8)
      in->x_direction = p->x < b->x ? 1 : -1;
    if (magnitude(b->x - p->x) < 48 && magnitude(b->y - p->y) < 48 &&
        choose_power(p, b, other, in)) {
      in->power_hit = 1;
      if (magnitude(other->x - p->x) < 80 && in->y_direction != -1)
        in->y_direction = -1;
    }
  }
}
static void move_player(Player *p, PikaUserInput *in, const Player *other,
                        const Ball *b) {
  int16_t previous, vx = 0, frame;
  if (p->is_computer)
    ai(p, b, other, in);
  if (p->state == 4) {
    if (--p->lying_down_duration_left < -1)
      p->state = 0;
    return;
  }
  previous = p->x;
  if (p->state < 5)
    vx = p->state < 3 ? in->x_direction * 6 : p->diving_direction * 8;
  p->x = bounded(p->x + vx, p->is_player2 ? HALF + 32 : 32,
                 p->is_player2 ? WIDTH - 32 : HALF - 32);
  if (p->state < 3 && in->y_direction == -1 && p->y == GROUND) {
    p->y_velocity = -16;
    p->state = 1;
    p->frame_number = 0;
    p->sound.chu = 1;
  }
  p->y += p->y_velocity;
  if (p->y < GROUND)
    p->y_velocity++;
  else if (p->y > GROUND) {
    p->y_velocity = 0;
    p->y = GROUND;
    p->frame_number = 0;
    if (p->state == 3) {
      p->state = 4;
      p->lying_down_duration_left = 3;
    } else
      p->state = 0;
  }
  if (in->power_hit == 1) {
    if (p->state == 1) {
      p->delay_before_next_frame = 5;
      p->frame_number = 0;
      p->state = 2;
      p->sound.pika = 1;
    } else if (p->state == 0 && in->x_direction != 0) {
      p->state = 3;
      p->frame_number = 0;
      p->diving_direction = in->x_direction;
      p->y_velocity = -5;
      p->sound.chu = 1;
    }
  }
  if (p->state == 1)
    p->frame_number = (p->frame_number + 1) % 3;
  else if (p->state == 2) {
    if (p->delay_before_next_frame < 1) {
      if (++p->frame_number > 4) {
        p->frame_number = 0;
        p->state = 1;
      }
    } else
      p->delay_before_next_frame--;
  } else if (p->state == 0 && p->x == previous) {
    p->frame_number = p->delay_before_next_frame = 0;
    p->normal_status_arm_swing_direction = 1;
  } else if (p->state == 0) {
    if (++p->delay_before_next_frame > 3) {
      p->delay_before_next_frame = 0;
      frame = (int16_t)p->frame_number + p->normal_status_arm_swing_direction;
      if (frame < 0 || frame > 4)
        p->normal_status_arm_swing_direction =
            -p->normal_status_arm_swing_direction;
      p->frame_number += p->normal_status_arm_swing_direction;
    }
  }
  if (p->game_ended) {
    if (p->state == 0) {
      p->state = p->is_winner ? 5 : 6;
      if (p->is_winner)
        p->sound.pipikachu = 1;
      p->delay_before_next_frame = p->frame_number = 0;
    }
    if (p->frame_number < 4 && ++p->delay_before_next_frame > 4) {
      p->delay_before_next_frame = 0;
      p->frame_number++;
    }
  }
}
static void collide(Ball *b, int16_t x, const PikaUserInput *in,
                    uint8_t state) {
  int16_t vy;
  if (b->x < x)
    b->x_velocity = -(magnitude(b->x - x) / 3);
  else if (b->x > x)
    b->x_velocity = magnitude(b->x - x) / 3;
  if (b->x_velocity == 0)
    b->x_velocity = random15() % 3 - 1;
  vy = magnitude(b->y_velocity);
  b->y_velocity = -(vy < 15 ? 15 : vy);
  if (state == 2) {
    b->x_velocity = (magnitude(in->x_direction) + 1) * 10;
    if (b->x >= HALF)
      b->x_velocity = -b->x_velocity;
    b->punch_effect_x = b->x;
    b->punch_effect_y = b->y;
    b->y_velocity = magnitude(b->y_velocity) * in->y_direction * 2;
    b->punch_effect_radius = RADIUS;
    b->sound.power_hit = 1;
    b->is_power_hit = 1;
  } else {
    b->is_power_hit = 0;
    b->sound.hit = 1;
  }
  if (has_ai)
    predict(b);
}
static void touch(PikaPhysics *p, Player *pl, const PikaUserInput *in) {
  if (magnitude(p->ball.x - pl->x) <= 32 &&
      magnitude(p->ball.y - pl->y) <= 32) {
    if (!pl->is_collision_with_ball_happened) {
      collide(&p->ball, pl->x, in, pl->state);
      pl->is_collision_with_ball_happened = 1;
    }
  } else
    pl->is_collision_with_ball_happened = 0;
}
uint8_t physics_tick(PikaPhysics *p, PikaUserInput in[2]) {
  uint8_t ground = world(&p->ball);
  if (has_ai)
    predict(&p->ball);
  move_player(&p->player1, &in[0], &p->player2, &p->ball);
  move_player(&p->player2, &in[1], &p->player1, &p->ball);
  touch(p, &p->player1, &in[0]);
  touch(p, &p->player2, &in[1]);
  return ground;
}
