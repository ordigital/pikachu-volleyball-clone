/* C adaptation of the Rust simulation. Coordinates and arithmetic stay 32-bit.
 * Original reconstruction: gorisanson/pikachu-volleyball; see root credits. */
#include "physics.h"
#include "scene.h"
#ifndef SGDK_GCC
#include <string.h>
#endif
#define WIDTH MD_WORLD_WIDTH
#define HALF MD_WORLD_CENTER
#define GROUND 244
#define RADIUS 20
#define LIMIT 1000
static uint64_t rng;
static int32_t magnitude(int32_t x) { return x < 0 ? -x : x; }
static int32_t bounded(int32_t x, int32_t a, int32_t b) {
  return x < a ? a : x > b ? b : x;
}
static int32_t random15(void) {
  uint64_t z = (rng += UINT64_C(0x9E3779B97F4A7C15));
  z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
  z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
  z ^= z >> 31;
  return (int32_t)((z >> 33) % 32768);
}
uint64_t physics_rng_state(void) { return rng; }
static void player_round(Player *p) {
  p->x = p->is_player2 ? WIDTH - 36 : 36;
  p->y = GROUND;
  p->y_velocity = 0;
  p->is_collision_with_ball_happened = false;
  p->state = p->frame_number = p->delay_before_next_frame = 0;
  p->normal_status_arm_swing_direction = 1;
  p->computer_boldness = random15() % 5;
}
static void ball_round(Ball *b, bool right) {
  b->x = right ? WIDTH - 56 : 56;
  b->y = 0;
  b->x_velocity = 0;
  b->y_velocity = 1;
  b->punch_effect_radius = 0;
  b->is_power_hit = false;
  b->world_bounce = false;
}
void physics_round(PikaPhysics *p, bool right) {
  player_round(&p->player1);
  player_round(&p->player2);
  ball_round(&p->ball, right);
}
void physics_init(PikaPhysics *p, bool ai1, bool ai2, uint64_t seed) {
  memset(p, 0, sizeof(*p));
  rng = seed;
  p->player1.is_computer = ai1;
  p->player2.is_computer = ai2;
  p->player2.is_player2 = true;
  p->player1.lying_down_duration_left = p->player2.lying_down_duration_left =
      -1;
  physics_round(p, false);
}
static bool world(Ball *b) {
  int32_t vx = b->x_velocity, vy = b->y_velocity;
  b->world_bounce = false;
  b->previous_previous_x = b->previous_x;
  b->previous_previous_y = b->previous_y;
  b->previous_x = b->x;
  b->previous_y = b->y;
  int32_t rotation = b->fine_rotation + b->x_velocity / 2;
  if (rotation < 0)
    rotation += 50;
  else if (rotation > 50)
    rotation -= 50;
  b->fine_rotation = rotation;
  b->rotation = rotation / 10;
  int32_t x = b->x + b->x_velocity;
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
  b->world_bounce = b->x_velocity != vx || b->y_velocity != vy;
  int32_t y = b->y + b->y_velocity;
  if (y > 252) {
    b->sound.ball_touches_ground = true;
    b->world_bounce = true;
    b->y_velocity = -b->y_velocity;
    b->punch_effect_x = b->x;
    b->y = 252;
    b->punch_effect_radius = RADIUS;
    b->punch_effect_y = 272;
    return true;
  }
  b->y = y;
  b->x += b->x_velocity;
  b->y_velocity++;
  return false;
}
static void predict(Ball *b) {
  int32_t x = b->x, y = b->y, vx = b->x_velocity, vy = b->y_velocity;
  for (int32_t n = 1;; n++) {
    int32_t next = x + vx;
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
static int32_t predict_power(int32_t dx, int32_t dy, const Ball *b) {
  int32_t x = b->x, y = b->y, vy = magnitude(b->y_velocity) * dy * 2;
  int32_t vx = (magnitude(dx) + 1) * 10;
  if (x >= HALF)
    vx = -vx;
  for (int32_t n = 1;; n++) {
    int32_t next = x + vx;
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
static bool choose_power(const Player *p, const Ball *b, const Player *other,
                         PikaUserInput *in) {
  int32_t side = p->is_player2;
  bool descending = random15() % 2 == 0;
  for (int32_t dx = 1; dx >= 0; dx--) {
    for (int32_t n = 0; n < 3; n++) {
      int32_t dy = descending ? n - 1 : 1 - n;
      int32_t landing = predict_power(dx, dy, b);
      if ((landing <= side * HALF || landing >= side * WIDTH + HALF) &&
          magnitude(landing - other->x) > 64) {
        in->x_direction = dx;
        in->y_direction = dy;
        return true;
      }
    }
  }
  return false;
}
static void ai(Player *p, const Ball *b, const Player *other,
               PikaUserInput *in) {
  in->x_direction = in->y_direction = in->power_hit = 0;
  int32_t side = p->is_player2, landing = b->expected_landing_point_x,
          bold = p->computer_boldness;
  if (magnitude(b->x - p->x) > 100 && magnitude(b->x_velocity) < bold + 5) {
    int32_t left = side * HALF;
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
    int32_t left = side * HALF, right = (side + 1) * HALF;
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
  if (p->is_computer)
    ai(p, b, other, in);
  if (p->state == 4) {
    if (--p->lying_down_duration_left < -1)
      p->state = 0;
    return;
  }
  int32_t previous = p->x, vx = 0;
  if (p->state < 5)
    vx = p->state < 3 ? in->x_direction * 6 : p->diving_direction * 8;
  p->x = bounded(p->x + vx, p->is_player2 ? HALF + 32 : 32,
                 p->is_player2 ? WIDTH - 32 : HALF - 32);
  if (p->state < 3 && in->y_direction == -1 && p->y == GROUND) {
    p->y_velocity = -16;
    p->state = 1;
    p->frame_number = 0;
    p->sound.chu = true;
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
      p->sound.pika = true;
    } else if (p->state == 0 && in->x_direction != 0) {
      p->state = 3;
      p->frame_number = 0;
      p->diving_direction = in->x_direction;
      p->y_velocity = -5;
      p->sound.chu = true;
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
      int32_t frame = p->frame_number + p->normal_status_arm_swing_direction;
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
        p->sound.pipikachu = true;
      p->delay_before_next_frame = p->frame_number = 0;
    }
    if (p->frame_number < 4 && ++p->delay_before_next_frame > 4) {
      p->delay_before_next_frame = 0;
      p->frame_number++;
    }
  }
}
static void collide(Ball *b, int32_t x, const PikaUserInput *in,
                    int32_t state) {
  if (b->x < x)
    b->x_velocity = -(magnitude(b->x - x) / 3);
  else if (b->x > x)
    b->x_velocity = magnitude(b->x - x) / 3;
  if (b->x_velocity == 0)
    b->x_velocity = random15() % 3 - 1;
  int32_t vy = magnitude(b->y_velocity);
  b->y_velocity = -(vy < 15 ? 15 : vy);
  if (state == 2) {
    b->x_velocity = (magnitude(in->x_direction) + 1) * 10;
    if (b->x >= HALF)
      b->x_velocity = -b->x_velocity;
    b->punch_effect_x = b->x;
    b->punch_effect_y = b->y;
    b->y_velocity = magnitude(b->y_velocity) * in->y_direction * 2;
    b->punch_effect_radius = RADIUS;
    b->sound.power_hit = true;
    b->is_power_hit = true;
  } else
    b->is_power_hit = false;
  predict(b);
}
bool physics_tick(PikaPhysics *p, PikaUserInput in[2]) {
  bool ground = world(&p->ball);
  predict(&p->ball);
  move_player(&p->player1, &in[0], &p->player2, &p->ball);
  predict(&p->ball);
  move_player(&p->player2, &in[1], &p->player1, &p->ball);
  Player *players[2] = {&p->player1, &p->player2};
  for (int i = 0; i < 2; i++) {
    Player *pl = players[i];
    if (magnitude(p->ball.x - pl->x) <= 32 &&
        magnitude(p->ball.y - pl->y) <= 32) {
      if (!pl->is_collision_with_ball_happened) {
        collide(&p->ball, pl->x, &in[i], pl->state);
        pl->is_collision_with_ball_happened = true;
      }
    } else
      pl->is_collision_with_ball_happened = false;
  }
  return ground;
}
