#include <genesis.h>
/* Standard 4 MiB ROM, no mapper, no SRAM overlapping the audio data. */
__attribute__((externally_visible)) const ROMHeader rom_header = {
    "SEGA MEGA DRIVE ",
    "(C)A.M.B. 2026  ",
    "PIKACHU VOLLEYBALL CLONE                        ",
    "PIKACHU VOLLEYBALL CLONE                        ",
    "GM 00000000-00",
    0,
    "J               ",
    0x00000000,
    0x003FFFFF,
    0xE0FF0000,
    0xE0FFFFFF,
    "  ",
    0,
    0,
    0,
    "            ",
    "UNOFFICIAL FAN REMAKE                   ",
    "JUE             "};
