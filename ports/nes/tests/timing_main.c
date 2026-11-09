/* Measures the cc65 cost of the physics pieces: cycles/call = frames*29780/N. */
#include "../src/physics.c"
/* draw.s links against these main.c globals */
uint8_t pl_nib[8], pl_y[2], ball_top;
extern volatile uint8_t nmi_frame, ppu_ctrl;
#pragma zpsym("nmi_frame")
#pragma zpsym("ppu_ctrl")
#define N 100
uint16_t t_frames[10];
volatile uint8_t t_done;
static PikaPhysics base, work;
static PikaUserInput in[2];
static uint16_t measure_n;
#define TIME(slot, stmt)                                                      \
  {                                                                           \
    uint16_t n, frames = 0;                                                   \
    uint8_t last;                                                             \
    while (nmi_frame == nmi_frame) { break; }                                 \
    last = nmi_frame;                                                         \
    while (last == nmi_frame) {}                                              \
    last = nmi_frame;                                                         \
    for (n = 0; n < N; n++) {                                                 \
      work = base;                                                            \
      stmt;                                                                   \
    }                                                                         \
    frames = (uint8_t)(nmi_frame - last);                                     \
    if (nmi_frame - last > 250) frames = 250;                                 \
    t_frames[slot] = frames;                                                  \
  }
int main(void) {
  uint8_t i;
  uint16_t tick;
  ppu_ctrl = 0x80;
  *(volatile uint8_t *)0x2000 = 0x80;
  physics_init(&base, 1, 1, 77);
  /* advance to a mid-rally state with the ball near player 2 */
  for (tick = 0; tick < 60; tick++) {
    in[0].x_direction = in[1].x_direction = 0; in[0].y_direction = in[1].y_direction = 0; in[0].power_hit = in[1].power_hit = 0;
    physics_tick(&base, in);
  }
  in[0].x_direction = 1; in[0].y_direction = 0; in[0].power_hit = 0; in[1] = in[0];
  TIME(0, (void)0);                         /* copy overhead */
  TIME(1, world(&work.ball));
  TIME(2, predict(&work.ball));
  TIME(3, move_player(&work.player1, &in[0], &work.player2, &work.ball));
  base.player1.is_computer = 1;
  work = base;
  TIME(4, work.player1.is_computer = 1; move_player(&work.player1, &in[0], &work.player2, &work.ball));
  TIME(5, touch(&work, &work.player1, &in[0]));
  TIME(6, physics_tick(&work, in));
  TIME(7, ai(&work.player1, &work.ball, &work.player2, &in[0]));
  work.player1.state = 1;
  TIME(8, work.player1.state = 1; work.ball.y = work.player1.y - 10; work.ball.x = work.player1.x + 10; choose_power(&work.player1, &work.ball, &work.player2, &in[0]));
  t_done = 1;
  for (;;) {}
}
