#!/usr/bin/env python3
"""Check real linked ROM/header/resource/VRAM budgets (standard library only)."""
from pathlib import Path
import json
import re
import struct
PORT=Path(__file__).resolve().parents[1]
reservation=int(re.search(r'^#define MD_SPRITE_TILES (\d+)$',(PORT/'src/scene.h').read_text(),re.M)[1])
rom=(PORT/'out/rom.bin').read_bytes()
symbols={parts[2]:int(parts[0],16) for line in (PORT/'out/symbol.txt').read_text().splitlines() if len(parts:=line.split())>=3}
def word(address):return struct.unpack_from('>H',rom,address)[0]
def tiles(name):return word(symbols[name]+2)
def sprite_tiles(name):return word(symbols[name+'_sprite']+14)
assert len(rom)<=4*1024*1024, 'ROM exceeds 4 MiB; mapper not supported'
assert rom[0x100:0x110]==b'SEGA MEGA DRIVE '
assert rom[0x120:0x150].rstrip()==b'PIKACHU VOLLEYBALL CLONE'
assert rom[0x1B0:0x1B2]!=b'RA','SRAM must not overlap music ROM'
assert word(0x18E)==sum(struct.unpack('>'+str((len(rom)-512)//2)+'H',rom[512:]))&65535,'checksum mismatch'
# Runtime VRAM: maps start 0xC000 (1536 tiles), font96, shared sprite reservation, system16.
court=tiles('court_tileset');menu=tiles('menu_tileset')+tiles('menu_art_tileset');water=tiles('water_tiles')
pause=tiles('pause_panel_tileset')+tiles('pause_cursor_tiles')
assert court+water+pause<=1536-96-reservation-16,'court/water/pause overlaps sprite VRAM'
assert menu<=1536-96-16,'menu overlaps font VRAM'
sprites=2*sprite_tiles('players')+sprite_tiles('ball')+4*sprite_tiles('digits')+2*sprite_tiles('cloud')+4*sprite_tiles('smoke')+3*sprite_tiles('ghost')+2*sprite_tiles('dust')+16*sprite_tiles('sand')+3*sprite_tiles('shadow')
peak_message=max(sprite_tiles(n) for n in ['start','ready','gameover'])
assert sprites+peak_message<=reservation,'sprite allocation overflow'
header=(PORT/'res/resources.h').read_text()
for name,length in re.findall(r'extern const u8 (\w+_sample)\[(\d+)\]',header):
    assert symbols[name]%256==0 and int(length)%256==0,'PCM4 alignment invalid'
    assert symbols[name]+int(length)<=len(rom),'audio outside ROM'
# Check the actual linked cloud texels against its shared palette. Neutral
# shades and blue/cyan are allowed; lavender (R=G<B) caused the pink artifact.
cloud_tiles=symbols['cloud_sprite_animation0_frame0_tileset']
assert word(cloud_tiles)==0,'cloud must remain uncompressed'
cloud_data=struct.unpack_from('>I',rom,cloud_tiles+4)[0]
used=set()
for byte in rom[cloud_data:cloud_data+word(cloud_tiles+2)*32]:
    used.update((byte>>4,byte&15))
for index in used-{0}:
    color=word(symbols['cloud_sprite_palette_data']+index*2)
    red,green,blue=color&14,(color>>4)&14,(color>>8)&14
    assert red==green==blue or red<green<=blue,'cloud contains a lavender/warm palette artifact'
assert symbols['drv_pcm4']+0xA6D<=len(rom),'PCM4 driver outside ROM'
print(json.dumps({'rom_bytes':len(rom),'cartridge_limit_bytes':4194304,'court_tiles':court,'water_tiles':water,'pause_tiles':pause,'menu_tiles':menu,'sprite_peak_tiles':sprites+peak_message,'sprite_reserved_tiles':reservation,'result':'PASS'},indent=2))
