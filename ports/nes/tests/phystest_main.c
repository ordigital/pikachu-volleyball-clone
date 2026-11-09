/* ROM-side physics checksum: runs the cc65-compiled simulation for 3 modes. */
#include "phys_driver.h"
/* draw.s links against these main.c globals */
uint8_t pl_nib[8], pl_y[2], ball_top;
uint16_t ph_out[3 * PH_CHECKS], ph_tr[100];
volatile uint8_t ph_done;
int16_t dbg[2][40];
static void dump(int16_t *d, const Player *p) {
  d[0] = p->x; d[1] = p->y; d[2] = p->y_velocity; d[3] = p->is_collision_with_ball_happened; d[4] = p->state;
  d[5] = p->frame_number; d[6] = p->normal_status_arm_swing_direction; d[7] = p->delay_before_next_frame;
  d[8] = p->computer_boldness; d[9] = p->diving_direction; d[10] = p->lying_down_duration_left; d[11] = p->is_winner;
  d[12] = p->game_ended; d[13] = p->computer_where_to_stand_by; d[14] = p->sound.pipikachu; d[15] = p->sound.pika; d[16] = p->sound.chu;
}
static void hook(uint16_t t, PikaPhysics *p) {
  if (t == 43 || t == 44) {
    int16_t *d = dbg[t - 43];
    dump(d, &p->player1); dump(d + 17, &p->player2);
    d[34] = p->ball.x; d[35] = p->ball.y; d[36] = p->ball.x_velocity; d[37] = p->ball.y_velocity; d[38] = p->ball.rotation; d[39] = p->ball.fine_rotation;
  }
}
int main(void) {
  uint8_t m;
  for (m = 0; m < 3; m++) {
    ph_trace = m == 0 ? ph_tr : 0;
    ph_hook = m == 0 ? hook : 0;
    ph_run(m, ph_out + m * PH_CHECKS);
  }
  ph_done = 1;
  for (;;) {
  }
}
