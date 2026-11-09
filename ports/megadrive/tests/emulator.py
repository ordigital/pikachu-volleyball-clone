#!/usr/bin/env python3
"""Tiny libretro test frontend. Tests ROM behavior without a desktop frontend.
Usage: python3 tests/emulator.py /usr/lib/libretro/picodrive_libretro.so [output-dir]
"""
import ctypes as C
from pathlib import Path
import sys
import struct
import re
import json
import wave
import numpy as np
from PIL import Image
PORT=Path(__file__).resolve().parents[1]
scene={k:int(v) for k,v in re.findall(r'^#define (MD_\w+) (-?\d+)$',(PORT/'src/scene.h').read_text(),re.M)}
# The 68000 ABI aligns struct members at two bytes. Build the RAM schema from
# the same C declarations, then validate it against a freshly initialized match.
types={'bool':C.c_bool,'int32_t':C.c_int32}
for body,name in re.findall(r'typedef struct \{([^{}]+)\} (\w+);',(PORT/'src/physics.h').read_text()):
    fields=[]
    for declaration in body.strip().split(';'):
        if not declaration.strip():continue
        typename,names=declaration.strip().split(None,1)
        fields.extend((field.strip(),types[typename]) for field in names.split(','))
    types[name]=type(name,(C.Structure,),{'_layout_':'ms','_pack_':2,'_fields_':fields})
OUT=Path(sys.argv[2] if len(sys.argv)>2 else '/tmp/pika-md-emulator')
OUT.mkdir(parents=True,exist_ok=True)
core=C.CDLL(sys.argv[1])
ENV=C.CFUNCTYPE(C.c_bool,C.c_uint,C.c_void_p)
VIDEO=C.CFUNCTYPE(None,C.c_void_p,C.c_uint,C.c_uint,C.c_size_t)
SAMPLE=C.CFUNCTYPE(None,C.c_int16,C.c_int16)
BATCH=C.CFUNCTYPE(C.c_size_t,C.POINTER(C.c_int16),C.c_size_t)
POLL=C.CFUNCTYPE(None)
INPUT=C.CFUNCTYPE(C.c_int16,C.c_uint,C.c_uint,C.c_uint,C.c_uint)
class GameInfo(C.Structure):_fields_=[('path',C.c_char_p),('data',C.c_void_p),('size',C.c_size_t),('meta',C.c_char_p)]
class Geometry(C.Structure):_fields_=[('w',C.c_uint),('h',C.c_uint),('mw',C.c_uint),('mh',C.c_uint),('aspect',C.c_float)]
class Timing(C.Structure):_fields_=[('fps',C.c_double),('rate',C.c_double)]
class AV(C.Structure):_fields_=[('geometry',Geometry),('timing',Timing)]
class Variable(C.Structure):_fields_=[('key',C.c_char_p),('value',C.c_char_p)]
strings=[str(OUT).encode()]
format=0
frame=None
audio=bytearray()
record=True
capture=True
buttons=[0,0]
@ENV
def env(command,data):
    global format
    if command==10:format=C.cast(data,C.POINTER(C.c_int))[0];return True
    if command in (9,30,31):C.cast(data,C.POINTER(C.c_char_p))[0]=strings[0];return True
    if command==15:
        v=C.cast(data,C.POINTER(Variable)).contents
        v.value=None  # Unknown options must not retain the previous option's string.
        if v.key in (b'genesis_plus_gx_region_detect',b'picodrive_region'):
            v.value=(b'Europe' if '--pal' in sys.argv else b'US') if v.key==b'picodrive_region' else (b'pal' if '--pal' in sys.argv else b'ntsc-u');return True
        return False
    if command==17:C.cast(data,C.POINTER(C.c_bool))[0]=False;return True
    if command==3:C.cast(data,C.POINTER(C.c_bool))[0]=False;return True
    if command==47:C.cast(data,C.POINTER(C.c_int))[0]=3;return True
    return command in (1,11,16,18,35,36,37,44,53,69)
@VIDEO
def video(data,w,h,pitch):
    global frame
    if not data or not capture:return
    pixel_size=4 if format==1 else 2
    raw=np.frombuffer(C.string_at(data,pitch*h),dtype=np.uint8).reshape(h,pitch)[:,:w*pixel_size].copy()
    if format==1:
        a=raw.view(np.uint32).reshape(h,w);r=(a>>16)&255;g=(a>>8)&255;b=a&255
    else:
        a=raw.view(np.uint16).reshape(h,w)
        r=((a>>11)&31)*255//31 if format==2 else ((a>>10)&31)*255//31
        g=((a>>5)&63)*255//63 if format==2 else ((a>>5)&31)*255//31
        b=(a&31)*255//31
    frame=np.stack([r,g,b],axis=-1).astype(np.uint8)
@SAMPLE
def sample(l,r):
    if record:audio.extend(struct.pack('<hh',l,r))
@BATCH
def batch(data,n):
    if record:audio.extend(C.string_at(data,n*4))
    return n
@POLL
def poll():pass
@INPUT
def inp(port,device,index,id):return int(port<2 and bool(buttons[port]&(1<<id)))
for name,cb in [('environment',env),('video_refresh',video),('audio_sample',sample),('audio_sample_batch',batch),('input_poll',poll),('input_state',inp)]:
    getattr(core,'retro_set_'+name).argtypes=[type(cb)];getattr(core,'retro_set_'+name)(cb)
core.retro_init()
rom=(PORT/'out/rom.bin').read_bytes();romdata=C.create_string_buffer(rom)
info=GameInfo(str(PORT/'out/rom.bin').encode(),C.cast(romdata,C.c_void_p),len(rom),None)
core.retro_load_game.argtypes=[C.POINTER(GameInfo)];core.retro_load_game.restype=C.c_bool
assert core.retro_load_game(C.byref(info)),'load ROM failed'
core.retro_get_system_av_info.argtypes=[C.POINTER(AV)];av=AV();core.retro_get_system_av_info(C.byref(av))
assert (49<av.timing.fps<51) if '--pal' in sys.argv else (59<av.timing.fps<61)
core.retro_get_memory_data.argtypes=[C.c_uint];core.retro_get_memory_data.restype=C.c_void_p
ram=core.retro_get_memory_data(2)
symbols={}
for line in (PORT/'out/symbol.txt').read_text().splitlines():
    parts=line.split()
    if len(parts)>=3:
        try:symbols[parts[2]]=int(parts[0],16)
        except ValueError:pass
# Libretro exposes native 68000 RAM: PicoDrive/GX store words byte-swapped on x86.
swapped='blastem' not in sys.argv[1]
def memory(name,size=2):
    address=symbols[name]&65535
    values=bytes(C.c_ubyte.from_address(ram+((address+i)^1 if swapped else address+i)).value for i in range(size))
    return int.from_bytes(values,'big')
core.retro_serialize_size.restype=C.c_size_t
core.retro_serialize.argtypes=[C.c_void_p,C.c_size_t];core.retro_serialize.restype=C.c_bool
def z80():
    size=core.retro_serialize_size(); state=C.create_string_buffer(size)
    assert core.retro_serialize(state,size)
    marker=rom[symbols['drv_pcm4']+512:symbols['drv_pcm4']+576]
    blob=state.raw;at=blob.find(marker)
    assert at>=512, 'Z80 RAM marker missing from savestate'
    return blob[at-512:at-512+8192]
def music_position():
    tick=memory('music_tick')
    assert 0<=tick<9990,'MIDI cursor outside original song'
    assert not z80()[0x102]&1,'sampled BGM unexpectedly playing'
    return memory('music_loops',4)*9990+tick

def assert_menu_silent():
    z=z80();assert not z[0x102]&15,'audio channel survives Quit'
    assert max(abs(int(v)) for v in np.frombuffer(audio[-round(av.timing.rate*.1)*4:],dtype='<i2'))<12,'buffered music leaks into menu'

def write_memory(name,value,size=2,offset=0):
    address=(symbols[name]&65535)+offset
    for i,b in enumerate(value.to_bytes(size,'big')):
        C.c_ubyte.from_address(ram+((address+i)^1 if swapped else address+i)).value=b
def run(n,mask=0,port=0):
    buttons[port]=mask
    for _ in range(n):core.retro_run()
    buttons[port]=0

def snap(name):
    assert frame is not None
    Image.fromarray(frame).save(OUT/(name+'.png'))
run(120);snap('menu')
# Transparent menu artwork must not overwrite the grass palette (CRAM wraps).
green=(frame[:,:,1].astype(int)>frame[:,:,0].astype(int)*1.1)&(frame[:,:,1]>frame[:,:,2])
assert green.sum()>25000, 'menu foreground erased the grass palette'
assert not z80()[0x102]&1, 'background music plays in menu'
assert np.max(np.abs(np.frombuffer(audio[-44100*4:],dtype='<i2').astype(int)))<=4,'menu music is not silent'
assert memory('screen',4)==0, 'did not reach menu'
# libretro B=0 (Genesis B), Start=3, Up=4, Down=5, Left=6, Right=7.
run(2,1);run(40)
# Check source-layout offsets while the START phase holds all actors still.
def game_value(structure,field,actor='ball'):
    offset=getattr(types['PikaPhysics'],actor).offset+getattr(types[structure],field).offset
    address=(symbols['game']&65535)+offset
    raw=bytes(C.c_ubyte.from_address(ram+((address+i)^1 if swapped else address+i)).value for i in range(4))
    return int.from_bytes(raw,'big',signed=True)
assert game_value('Player','x','player2')==scene['MD_WORLD_WIDTH']-36
assert game_value('Player','y','player1')==244
assert scene['MD_WORLD_CENTER']*scene['MD_SCALE_NUM']//scene['MD_SCALE_DEN']==160
snap('start');run(110);snap('match')
assert max(abs(int(v)) for v in np.frombuffer(audio[-round(av.timing.rate*.1)*4:],dtype='<i2'))>100,'native music is silent'
# An expired dust slot must not wrap negative and reappear after a long rally.
write_memory('dust',32767,offset=4);run(6)
# A fresh burst may legitimately reuse this slot; negative ages never are.
assert memory('dust',6)&65535<32768,'expired dust age overflowed into animation'
before_steps=memory('sim_steps',4);before_music=music_position();run(600);advanced=memory('sim_steps',4)-before_steps
assert abs(music_position()-before_music-600*120/av.timing.fps)<3,'MIDI clock drifts during busy rendering'
assert abs(advanced-600*25/av.timing.fps)<=2,'simulation drifts from 25 Hz'
assert memory('screen',4) in (2,3,4), f"match did not start: screen={memory('screen',4)} timer={memory('timer')} hz={memory('hz')} vtimer={memory('vtimer',4)}"
run(2,1<<3);run(10);snap('pause')
assert memory('paused',1)==1, 'Start did not pause'
# Every panel pixel behind white letters is opaque blue, with no BG_B holes.
panel=frame[84:140,84:236].astype(int)
blue=(panel[:,:,2]>panel[:,:,0]*2)&(panel[:,:,2]>panel[:,:,1]*1.3)
white=(panel.min(axis=2)>220)&(panel.max(axis=2)-panel.min(axis=2)<15)
assert (blue|white).all(), 'pause panel has transparent/black pixels under text'
assert white.sum()>200 and blue.mean()>.85,'pause panel text/background missing'
before=memory('game',256);music_before=music_position();z_before=z80();assert z_before[0x130:0x132]==bytes([1,1]),'PCM4 pause not acknowledged'
run(60);assert memory('game',256)==before,'physics advanced while paused'
assert music_position()==music_before,'MIDI advanced while paused'
z_after=z80();assert z_before[0x104:0x132]==z_after[0x104:0x132] and z_before[0xE00:0x1000]==z_after[0xE00:0x1000],'audio buffers or decoder advanced while paused'
run(2,1);run(10);assert memory('paused',1)==0,'Resume did not work'
assert 0<music_position()-music_before<30,'MIDI resumes at a different position'
run(2,1<<3);run(2);run(2,1<<5);run(2);run(2,1);run(10);snap('quit-menu')
assert memory('screen',4)==0,'Quit did not return to menu'
assert_menu_silent()
# Start 2 players, both can pause, only owner chooses menu actions.
run(2,1<<5);run(2);run(2,1);run(150)
assert memory('two_players',1)==1
run(2,1<<3,1);run(5);assert memory('paused',1)==1
run(2,1,0);run(5);assert memory('paused',1)==1,'non-owner consumed pause dialog'
run(2,1,1);run(5);assert memory('paused',1)==0
# A normal point slows the world while phase/music clocks continue normally.
for _ in range(2400):
    if memory('screen',4)==3 and memory('slow_segment',1):break
    run(1)
else:raise AssertionError('point slow motion not reached')
before_physics=memory('physics_steps',4);before_clock=memory('sim_steps',4);before_music=music_position()
run(round(av.timing.fps*.8));snap('point-slow-motion')
assert 3<=memory('physics_steps',4)-before_physics<=5,'point does not run at one-fifth speed'
assert abs(memory('sim_steps',4)-before_clock-20)<=1,'point timer slowed with physics'
assert abs(music_position()-before_music-96)<3,'music slowed with physics'
# Force maximum tide and verify it stays below the restored court border.
write_memory('visual_tick',90,4);run(15);snap('low-water')
beach=frame[scene['MD_BEACH_START']:scene['MD_WATER_CREST']].astype(int)
cyan=(beach[:,:,2]>beach[:,:,0]*1.5)&(beach[:,:,2]>beach[:,:,1]*1.1)
assert not cyan.any(),'water covers the court border or upper beach'
# Stress match including directional jumps, B taps and long autonomous physics.
for i in range(40):
    run(45,(1<<4)|(1<<(6+i%2)))
    run(2,1);run(10)
snap('gameplay')
assert memory('max_line_sprites')<=20 and memory('max_line_pixels')<=320,'sprite scanline budget exceeded'
# Exercise scoring to 15, enlarged GAME OVER and automatic return.
write_memory('score',14);write_memory('score',14,offset=2)
for _ in range(3000):
    run(1)
    if memory('screen',4)==5:break
else:raise AssertionError('GAME OVER not reached')
run(round(av.timing.fps*2.5));snap('game-over')
assert memory('max_line_sprites')<=20 and memory('max_line_pixels')<=320,'game over scanline budget exceeded'
run(round(av.timing.fps*7));assert memory('screen',4)==0,'game over did not return to menu'
# Regression: subsequent games must score, including Quit after a point.
for replay in range(2):
    run(2,1);run(40)
    assert memory('screen',4)==1,'missing START phase'
    run(2,1<<3);run(8);snap('pause-start')
    assert memory('paused',1)==1
    if frame is not None:
        region=frame[81:143,81:239].astype(np.int16)
        assert not ((region[:,:,0]>170)&(region[:,:,2]<100)).any(),'sprite covers pause choices'
    run(2,1);run(150)
    # Read both scoreboard cells explicitly; the first cell can stay zero.
    address=symbols['score']&65535
    def score_total():
        return sum(int.from_bytes(bytes(C.c_ubyte.from_address(ram+((address+i+j)^1 if swapped else address+i+j)).value for j in range(2)),'big') for i in (0,2))
    for _ in range(2400):
        if score_total()>0:break
        run(1)
    assert score_total()>0 and memory('round_scored',1)==1,'next match cannot score'
    run(2,1<<3);run(2);run(2,1<<5);run(2);run(2,1);run(10)
    assert memory('screen',4)==0

# Scoring integration at the new midpoint, including the old erroneous region.
run(2,1);run(150)
for landing in [scene['MD_WORLD_CENTER']-1,scene['MD_WORLD_CENTER'],scene['MD_WORLD_CENTER']+32]:
    for _ in range(600):
        if memory('screen',4)==2:break
        run(1)
    else:raise AssertionError('match not ready for midpoint scoring check')
    before=memory('score',4);scores=[before>>16,before&65535]
    for field,value in [('x',landing),('y',252),('x_velocity',0),('y_velocity',1)]:
        offset=types['PikaPhysics'].ball.offset+getattr(types['Ball'],field).offset
        write_memory('game',value,4,offset=offset)
    for _ in range(12):
        run(1)
        if memory('screen',4)==3:break
    assert memory('screen',4)==3,'forced landing did not score'
    winner=1 if landing<scene['MD_WORLD_CENTER'] else 0
    scores[winner]+=1
    assert memory('score',4)==(scores[0]<<16|scores[1]),'point awarded on wrong side of resized court'
run(2,1<<3);run(2);run(2,1<<5);run(2);run(2,1);run(10)
assert memory('screen',4)==0

if '--long' in sys.argv:
    record=False;capture=False
    run(2,1);run(10)
    assert memory('screen',4)==1
    previous=music_position();paused_at_end=False
    for _ in range(round(av.timing.fps*(83.25*2+3))):
        write_memory('timer',0);run(1)
        position=music_position()
        assert 0<=position-previous<=3,'MIDI skipped or lost time'
        previous=position
        if not paused_at_end and memory('music_tick')>9900:
            run(2,1<<3);run(3)
            frozen=z80();cursor=music_position()
            assert frozen[0x130:0x132]==bytes([1,1])
            run(60);assert music_position()==cursor
            assert z80()[0x104:0x132]==frozen[0x104:0x132]
            assert z80()[0xE00:0x1000]==frozen[0xE00:0x1000]
            run(2,1<<3);run(2);previous=music_position();paused_at_end=True
    assert memory('music_loops',4)>=2 and paused_at_end,'original MIDI failed full-loop/pause checks'
    print('PASS: original native MIDI looped twice and paused/resumed near the loop boundary.')

with wave.open(str(OUT/'audio.wav'),'wb') as out:
    out.setnchannels(2);out.setsampwidth(2);out.setframerate(round(av.timing.rate));out.writeframes(audio)
print(json.dumps({'core':sys.argv[1],'fps':av.timing.fps,'audio_rate':av.timing.rate,'worst_physics_us':memory('worst_tick_us',4),'render_overruns':memory('missed_frames',4),'render_max_subticks':memory('max_render_subticks',4),'water_max_subticks':memory('max_water_subticks',4),'max_line_sprites':memory('max_line_sprites'),'max_line_pixels':memory('max_line_pixels'),'dropped_effects':memory('dropped_effects'),'rom_bytes':len(rom),'screenshots':str(OUT),'result':'PASS'},indent=2))
core.retro_unload_game();core.retro_deinit()
