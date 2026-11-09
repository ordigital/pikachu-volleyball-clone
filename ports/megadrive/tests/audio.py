#!/usr/bin/env python3
"""Check original MIDI identity, lossless extraction and actual linked chip score."""
from pathlib import Path
import hashlib
import json
import math
import re
import struct
import wave
import numpy as np
PORT=Path(__file__).resolve().parents[1]

def midi_events(data):
    """Read SMF independently of the optional authoring converter."""
    assert data[:22]==b'MThd\0\0\0\6\0\0\0\1\0\x30MTrk'+(len(data)-22).to_bytes(4,'big')
    p=22;tick=0;events=[]
    def variable():
        nonlocal p
        value=0
        while True:
            byte=data[p];p+=1;value=(value<<7)|(byte&127)
            if byte<128:return value
    while p<len(data):
        tick+=variable();status=data[p];p+=1
        if status==255:
            kind=data[p];p+=1;size=variable();payload=data[p:p+size];p+=size
            if kind==47:break
            assert kind==81 and size==3
            events.append((tick,1,int.from_bytes(payload,'big')))
        else:
            assert 128<=status<240
            size=1 if status&240 in (192,208) else 2
            events.append((tick,0,int.from_bytes(bytes([status])+data[p:p+size],'little')));p+=size
    assert p==len(data)
    return events,tick

provenance=json.loads((PORT/'art/original-bgm-midi.provenance.json').read_text())
mids=(PORT/'art/original-bgm.mids').read_bytes();midi=(PORT/'art/original-bgm.mid').read_bytes()
assert hashlib.sha256(mids).hexdigest()==provenance['mids_sha256']
assert hashlib.sha256(midi).hexdigest()==provenance['midi_sha256']
assert mids[:4]==b'RIFF' and mids[8:12]==b'MIDS'
assert struct.unpack_from('<I',mids,4)[0]+8==len(mids)
chunks={};p=12
while p<len(mids):
    size=struct.unpack_from('<I',mids,p+4)[0]
    chunks[mids[p:p+4]]=mids[p+8:p+8+size];p+=8+size+(size&1)
assert struct.unpack('<III',chunks[b'fmt '])==(48,4096,1)
raw=chunks[b'data'];buffers=struct.unpack_from('<I',raw)[0];p=4;tick=0;original=[]
for _ in range(buffers):
    _,size=struct.unpack_from('<II',raw,p);p+=8;end=p+size
    while p<end:
        delta,event=struct.unpack_from('<II',raw,p);p+=8;tick+=delta
        kind=event>>24&0xbf;value=event&0xffffff
        assert kind in (0,1)
        if not kind and value&240 in (192,208):value&=65535
        original.append((tick,kind,value))
    assert p==end
assert p==len(raw)
parsed,end=midi_events(midi)
assert parsed==original,'MIDI lost/reordered stream events'
assert len(parsed)==4541 and end==9990 and end/120==83.25
rom=(PORT/'out/rom.bin').read_bytes()
symbols={v[2]:int(v[0],16) for line in (PORT/'out/symbol.txt').read_text().splitlines() if len(v:=line.split())>=3}
source=(PORT/'src/music_data.h').read_text()
commands=[tuple(int(v,0) for v in values) for values in re.findall(r'\{(\d+),(\d+),(0x\w+),(0x\w+),(0x\w+)\}',source)]
assert len(commands)==22357 and [w[0] for w in commands]==sorted(w[0] for w in commands)
packed=b''.join(struct.pack('>HBBBB',*w) for w in commands)
assert rom[symbols['music_score']:symbols['music_score']+len(packed)]==packed,'linked score differs from source'
assert 'music_sample' not in symbols,'obsolete sampled BGM remains linked'
# Independently check FM note-on counts and original pitches (including bends),
# by decoding the linked register writes back into frequencies for PAL/NTSC.
expected={};bends={c:8192 for c in range(16)}
for tick,kind,value in parsed:
    if kind:continue
    channel=value&15;status=value&240;a=value>>8&127;b=value>>16&127
    if status==0xe0:bends[channel]=a|b<<7
    if status==0x90 and b and channel in (1,2,3,4):
        pitch=a+(bends[channel]-8192)/8192*2
        expected.setdefault(tick,[]).append(440*2**((pitch-69)/12))
actual={};registers={};keys=[False]*5;max_fm=0
for tick,port,reg,ntsc,pal in commands:
    assert port in (0,1,2)
    if port==2:assert reg==0;continue
    assert reg not in (0x2a,0x2b),'score touches PCM4 DAC'
    if reg==0x28:
        assert port==0 and ntsc==pal
        channel=ntsc&7;v=channel if channel<3 else channel-1
        assert 0<=v<5,'score touches DAC voice'
        keys[v]=bool(ntsc&0xf0);max_fm=max(max_fm,sum(keys))
        if keys[v]:
            pitches=[]
            for variant,clock in enumerate((7670454,7600489)):
                hi=registers[(v//3,0xa4+v%3)][variant]
                lo=registers[(v//3,0xa0+v%3)][variant]
                fnum=((hi&7)<<8)|lo;block=hi>>3
                pitches.append(fnum*clock*2**(block-1)/(144*2**20))
            actual.setdefault(tick,[]).append(pitches)
    else:
        assert reg&3!=3 and port*3+(reg&3)<5
        registers[port,reg]=(ntsc,pal)
assert not any(keys) and max_fm==5
assert actual.keys()==expected.keys()
max_pitch_error=0
for tick in expected:
    target=expected[tick];found=actual[tick]
    assert len(target)==len(found)
    for hz,variants in zip(target,found):
        error=max(abs(1200*math.log2(v/hz)) for v in variants)
        max_pitch_error=max(max_pitch_error,error)
        assert error<2,'note pitch changed during MIDI-to-FM conversion'
# Final PCM4 SFX retain quantization headroom; no sampled music is mixed with them.
header=(PORT/'res/resources.h').read_text();peaks={}
for name in (f'sfx{i}' for i in range(7)):
    with wave.open(str(PORT/f'res/{name}.wav')) as wav:
        assert (wav.getnchannels(),wav.getsampwidth(),wav.getframerate())==(1,2,16000)
        frames=wav.getnframes()
    address=symbols[name+'_sample'];size=int(re.search(name+r'_sample\[(\d+)\]',header)[1])
    assert address%256==size%256==0 and frames<=size<frames+256
    peaks[name]=float(np.max(np.abs(np.frombuffer(rom[address:address+frames],dtype=np.int8).astype(float))))/128
    assert peaks[name]<.31
# Driver pause gate must fit the exact image size hardcoded in the SGDK loader.
driver_size=0xa6d  # Immutable SGDK 2.11 loader ABI; test the actual image.
code=rom[symbols['drv_pcm4']:symbols['drv_pcm4']+driver_size]
assert code[13]==0xc3 and 0x3a<=int.from_bytes(code[14:16],'little')<0x200
assert b'\x3a\x30\x01\xb7\xc2' in code[0x3a:0x200]
print(json.dumps({'result':'PASS','original_midi_events':len(parsed),'song_seconds':83.25,
    'linked_register_writes':len(commands),'max_fm_voices':max_fm,
    'max_note_error_cents':round(max_pitch_error,3),'sfx_peaks':peaks,'driver_bytes':driver_size},indent=2))
