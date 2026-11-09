#ifndef NES_SCENE_H
#define NES_SCENE_H
/* Shared by the 6502 code and the offline asset converter (tools parse the
 * numeric #defines).  Physics keeps the 432x304 world; the screen mapping is
 * x*19/32 (432 -> 256) and y*19/32 + NES_Y_OFFSET, computed with shifts. */
#define NES_WORLD_WIDTH 432
#define NES_WORLD_CENTER 216
#define NES_SCALE_NUM 19
#define NES_Y_OFFSET 32
/* Player art: 4x4 tiles (32px), feet aligned to the world ground line. */
#define NES_PLAYER_SIZE 32
#define NES_PLAYER_FOOT 3
/* Ball art: 3x3 tiles (24px) centred on the ball. */
#define NES_BALL_SIZE 24
#define NES_POLE_X 125
#define NES_POLE_W 6
#if NES_WORLD_CENTER * 2 != NES_WORLD_WIDTH
#error World midpoint must match the court width
#endif
#endif
