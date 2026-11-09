/* Integer simulation shared in behavior with src/physics.rs. Keep parity tests
 * passing. */
#ifndef PIKA_PHYSICS_H
#define PIKA_PHYSICS_H
#ifdef SGDK_GCC
#include <genesis.h>
typedef unsigned long long uint64_t;
#define UINT64_C(x) x##ULL
#else
#include <stdbool.h>
#include <stdint.h>
#endif
typedef struct {
  int32_t x_direction;
  int32_t y_direction;
  int32_t power_hit;
} PikaUserInput;
typedef struct {
  bool pipikachu;
  bool pika;
  bool chu;
} PlayerSound;
typedef struct {
  bool power_hit;
  bool ball_touches_ground;
} BallSound;
typedef struct {
  bool is_player2;
  bool is_computer;
  int32_t x;
  int32_t y;
  int32_t y_velocity;
  bool is_collision_with_ball_happened;
  int32_t state;
  int32_t frame_number;
  int32_t normal_status_arm_swing_direction;
  int32_t delay_before_next_frame;
  int32_t computer_boldness;
  int32_t diving_direction;
  int32_t lying_down_duration_left;
  bool is_winner;
  bool game_ended;
  int32_t computer_where_to_stand_by;
  PlayerSound sound;
} Player;
typedef struct {
  int32_t x;
  int32_t y;
  int32_t x_velocity;
  int32_t y_velocity;
  int32_t punch_effect_radius;
  bool is_power_hit;
  bool world_bounce;
  int32_t expected_landing_point_x;
  int32_t rotation;
  int32_t fine_rotation;
  int32_t punch_effect_x;
  int32_t punch_effect_y;
  int32_t previous_x;
  int32_t previous_previous_x;
  int32_t previous_y;
  int32_t previous_previous_y;
  BallSound sound;
} Ball;
typedef struct {
  Player player1, player2;
  Ball ball;
} PikaPhysics;
void physics_init(PikaPhysics *p, bool ai1, bool ai2, uint64_t seed);
void physics_round(PikaPhysics *p, bool serve_right);
bool physics_tick(PikaPhysics *p, PikaUserInput input[2]);
uint64_t physics_rng_state(void);
#endif
