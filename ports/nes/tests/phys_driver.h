/* Shared by the host and ROM physics checksum programs (identical inputs). */
#include "physics.h"
#define PH_TICKS 5000
#define PH_CHECKS (PH_TICKS / 100)
static uint16_t ph_h;
static void (*ph_hook)(uint16_t, PikaPhysics *);
static uint16_t *ph_trace; /* optional per-tick hashes of the first 100 ticks */
static void ph_mix(int16_t v) { ph_h = (uint16_t)(ph_h * 31u + (uint16_t)v + 7u); }
static void ph_player(const Player *p) {
  ph_mix(p->x); ph_mix(p->y); ph_mix(p->y_velocity); ph_mix(p->is_collision_with_ball_happened);
  ph_mix(p->state); ph_mix(p->frame_number); ph_mix(p->normal_status_arm_swing_direction);
  ph_mix(p->delay_before_next_frame); ph_mix(p->computer_boldness); ph_mix(p->diving_direction);
  ph_mix(p->lying_down_duration_left); ph_mix(p->is_winner); ph_mix(p->game_ended);
  ph_mix(p->computer_where_to_stand_by); ph_mix(p->sound.pipikachu); ph_mix(p->sound.pika); ph_mix(p->sound.chu);
}
static void ph_run(uint8_t mode, uint16_t *out) {
  PikaPhysics p;
  PikaUserInput in[2];
  uint16_t tick;
  uint8_t i, hit;
  physics_init(&p, mode == 2, mode != 0, 1234);
  ph_h = 0;
  for (tick = 0; tick < PH_TICKS; tick++) {
    if (tick % 157 == 156) physics_round(&p, tick % 2 == 0);
    for (i = 0; i < 2; i++) {
      in[i].x_direction = (int8_t)(((tick + i * 5) / 13) % 3 - 1);
      in[i].y_direction = (int8_t)((tick % 43 < 2) ? -1 : ((tick + i) / 17) % 2);
      in[i].power_hit = ((tick + i * 3) % 19 == 0);
    }
    hit = physics_tick(&p, in);
    if (ph_hook) ph_hook(tick, &p);
    ph_mix(hit); ph_player(&p.player1); ph_player(&p.player2);
    ph_mix(p.ball.x); ph_mix(p.ball.y); ph_mix(p.ball.x_velocity); ph_mix(p.ball.y_velocity);
    ph_mix(p.ball.punch_effect_radius); ph_mix(p.ball.is_power_hit); ph_mix(p.ball.rotation);
    ph_mix(p.ball.fine_rotation); ph_mix(p.ball.punch_effect_x); ph_mix(p.ball.punch_effect_y);
    ph_mix(p.ball.sound.power_hit); ph_mix(p.ball.sound.ball_touches_ground);
    p.player1.sound.pipikachu = p.player1.sound.pika = p.player1.sound.chu = 0;
    p.player2.sound.pipikachu = p.player2.sound.pika = p.player2.sound.chu = 0;
    p.ball.sound.power_hit = p.ball.sound.ball_touches_ground = 0;
    if (tick % 509 == 508) { p.player1.game_ended = 1; p.player1.is_winner = 1; p.player2.game_ended = 1; }
    if (tick % 509 == 0) { p.player1.game_ended = 0; p.player2.game_ended = 0; }
    if (ph_trace && tick < 100) ph_trace[tick] = ph_h;
    if (tick % 100 == 99) out[tick / 100] = ph_h;
  }
}
