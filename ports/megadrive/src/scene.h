#ifndef MD_SCENE_H
#define MD_SCENE_H
/* Shared by physics, rendering and the offline asset converter. Numeric values
 * are parsed by the Python tools; use C-style truncation for scene coordinates. */
#define MD_WORLD_WIDTH 432
#define MD_WORLD_CENTER 216
#define MD_SCALE_NUM 20
#define MD_SCALE_DEN 27
#define MD_Y_OFFSET -8
#define MD_PLAYER_CELL 48
#define MD_PLAYER_ART 46
#define MD_BALL_WIDTH 40
#define MD_BALL_HEIGHT 32
#define MD_BALL_DIAMETER 30
#define MD_SHADOW_WIDTH 40
#define MD_SPRITE_TILES 512
#define MD_BEACH_START 216
#define MD_WATER_CREST 220
#if MD_WORLD_CENTER * 2 != MD_WORLD_WIDTH
#error World midpoint must match the court width
#endif
#endif
