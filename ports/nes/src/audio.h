#ifndef AUDIO_H
#define AUDIO_H
#include <stdint.h>
enum { SFX_PIPIKACHU, SFX_PIKA, SFX_CHU, SFX_MOVE, SFX_SELECT, SFX_POWER, SFX_GROUND, SFX_HIT };
void music_start(void);
void music_stop(void);
void music_pause(uint8_t paused);
void sfx_play(uint8_t id);
#endif
