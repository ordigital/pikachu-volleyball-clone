#ifndef MUSIC_H
#define MUSIC_H
#include <genesis.h>
void music_init(void);
void music_start(void);
/* Screen transition: silence native music and clear pending/buffered SFX. */
void music_stop(void);
/* Freeze MIDI position + PCM4 SFX; native held notes retrigger on resume. */
void music_pause(bool value);
#endif
