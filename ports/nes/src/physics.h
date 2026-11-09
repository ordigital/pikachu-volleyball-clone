/* 16-bit integer simulation, behavior-compatible with src/physics.rs and
 * ports/megadrive/src/physics.c.  Only the RNG differs (see physics.c). */
#ifndef PIKA_PHYSICS_H
#define PIKA_PHYSICS_H
#include <stdint.h>
typedef struct {
  int8_t x_direction, y_direction, power_hit;
} PikaUserInput;
typedef struct {
  uint8_t pipikachu, pika, chu;
} PlayerSound;
typedef struct {
  uint8_t power_hit, ball_touches_ground, hit; /* hit: ordinary (non-power) player hit */
} BallSound;
typedef struct {
  uint8_t is_player2, is_computer;
  int16_t x, y, y_velocity;
  uint8_t is_collision_with_ball_happened;
  uint8_t state, frame_number;
  int8_t normal_status_arm_swing_direction;
  uint8_t delay_before_next_frame, computer_boldness;
  int8_t diving_direction;
  int8_t lying_down_duration_left;
  uint8_t is_winner, game_ended, computer_where_to_stand_by;
  PlayerSound sound;
} Player;
typedef struct {
  int16_t x, y, x_velocity, y_velocity;
  int16_t punch_effect_radius;
  uint8_t is_power_hit;
  int16_t expected_landing_point_x;
  uint8_t rotation;
  int16_t fine_rotation;
  int16_t punch_effect_x, punch_effect_y;
  BallSound sound;
} Ball;
typedef struct {
  Player player1, player2;
  Ball ball;
} PikaPhysics;
void physics_init(PikaPhysics *p, uint8_t ai1, uint8_t ai2, uint16_t seed);
void physics_round(PikaPhysics *p, uint8_t serve_right);
uint8_t physics_tick(PikaPhysics *p, PikaUserInput input[2]);
#endif
