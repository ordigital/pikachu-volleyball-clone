"""Minimal libretro frontend for NES tests (video, audio, input, WRAM access)."""
import ctypes as C
from pathlib import Path
import struct
import numpy as np
from PIL import Image
PORT = Path(__file__).resolve().parents[1]
CORES = {'fceumm': '/usr/lib/libretro/fceumm_libretro.so', 'mesen': '/usr/lib/libretro/mesen_libretro.so',
         'nestopia': '/usr/lib/libretro/nestopia_libretro.so', 'quicknes': '/usr/lib/libretro/quicknes_libretro.so'}
ENV = C.CFUNCTYPE(C.c_bool, C.c_uint, C.c_void_p)
VIDEO = C.CFUNCTYPE(None, C.c_void_p, C.c_uint, C.c_uint, C.c_size_t)
SAMPLE = C.CFUNCTYPE(None, C.c_int16, C.c_int16)
BATCH = C.CFUNCTYPE(C.c_size_t, C.POINTER(C.c_int16), C.c_size_t)
POLL = C.CFUNCTYPE(None)
INPUT = C.CFUNCTYPE(C.c_int16, C.c_uint, C.c_uint, C.c_uint, C.c_uint)
LOG = C.CFUNCTYPE(None, C.c_int, C.c_char_p)
class LogCb(C.Structure): _fields_ = [('log', LOG)]
class GameInfo(C.Structure): _fields_ = [('path', C.c_char_p), ('data', C.c_void_p), ('size', C.c_size_t), ('meta', C.c_char_p)]
class Geometry(C.Structure): _fields_ = [('w', C.c_uint), ('h', C.c_uint), ('mw', C.c_uint), ('mh', C.c_uint), ('aspect', C.c_float)]
class Timing(C.Structure): _fields_ = [('fps', C.c_double), ('rate', C.c_double)]
class AV(C.Structure): _fields_ = [('geometry', Geometry), ('timing', Timing)]
class Variable(C.Structure): _fields_ = [('key', C.c_char_p), ('value', C.c_char_p)]
# libretro joypad ids as bit masks for Emu.pad
B, Y, SELECT, START, UP, DOWN, LEFT, RIGHT, A = (1 << i for i in range(9))
class Emu:
    def __init__(self, rom, core='fceumm', battery=True, out='/tmp/pika-nes-emu', verbose=False, options=None, pal=False):
        self.verbose = verbose; self.options = options or {}; self.keep = []
        self.out = Path(out); self.out.mkdir(parents=True, exist_ok=True)
        data = bytearray(Path(rom).read_bytes())
        if pal: data[9] |= 1         # iNES TV-system bit
        if battery: data[6] |= 2     # expose the 8 KiB WRAM through libretro SAVE_RAM in tests
        self.fmt = 0; self.frame = None; self.audio = bytearray(); self.record = True
        self.pad = [0, 0]
        self.core = C.CDLL(CORES.get(core, core))
        self.strings = [str(self.out).encode()]
        emu = self
        @ENV
        def env(cmd, data_):
            if cmd == 10: emu.fmt = C.cast(data_, C.POINTER(C.c_int))[0]; return True
            if cmd in (9, 30, 31): C.cast(data_, C.POINTER(C.c_char_p))[0] = emu.strings[0]; return True
            if cmd == 15:
                v = C.cast(data_, C.POINTER(Variable)).contents
                val = emu.options.get(v.key.decode())
                v.value = val.encode() if val else None
                emu.keep.append(v.value)
                return val is not None
            if cmd == 27: C.cast(data_, C.POINTER(LogCb))[0] = emu.log_struct; return True
            if cmd == 17: C.cast(data_, C.POINTER(C.c_bool))[0] = False; return True
            if cmd == 3: C.cast(data_, C.POINTER(C.c_bool))[0] = False; return True
            if cmd == 47: C.cast(data_, C.POINTER(C.c_int))[0] = 3; return True
            return cmd in (1, 11, 16, 18, 35, 36, 37, 44, 53, 69)
        @LOG
        def log(level, fmt):
            if emu.verbose: print('core:', fmt.decode(errors='replace').rstrip())
        self.log_cb = log; self.log_struct = LogCb(log)
        @VIDEO
        def video(d, w, h, pitch):
            if not d: return
            raw = np.frombuffer(C.string_at(d, pitch * h), np.uint8).reshape(h, pitch)
            if emu.fmt == 1:
                a = raw[:, :w * 4].copy().view(np.uint32).reshape(h, w)
                emu.frame = np.stack([(a >> 16) & 255, (a >> 8) & 255, a & 255], -1).astype(np.uint8)
            else:
                a = raw[:, :w * 2].copy().view(np.uint16).reshape(h, w)
                if emu.fmt == 2:
                    r, g, b = (a >> 11) * 255 // 31, ((a >> 5) & 63) * 255 // 63, (a & 31) * 255 // 31
                else:
                    r, g, b = ((a >> 10) & 31) * 255 // 31, ((a >> 5) & 31) * 255 // 31, (a & 31) * 255 // 31
                emu.frame = np.stack([r, g, b], -1).astype(np.uint8)
        @SAMPLE
        def sample(l, r):
            if emu.record: emu.audio.extend(struct.pack('<hh', l, r))
        @BATCH
        def batch(d, n):
            if emu.record: emu.audio.extend(C.string_at(d, n * 4))
            return n
        @POLL
        def poll(): pass
        @INPUT
        def inp(port, device, index, id_): return int(port < 2 and device == 1 and bool(emu.pad[port] >> id_ & 1))
        self.cb = (env, video, sample, batch, poll, inp)
        for name, cb in zip(['environment', 'video_refresh', 'audio_sample', 'audio_sample_batch', 'input_poll', 'input_state'], self.cb):
            getattr(self.core, 'retro_set_' + name).argtypes = [type(cb)]; getattr(self.core, 'retro_set_' + name)(cb)
        self.core.retro_init()
        self.rom = C.create_string_buffer(bytes(data))
        info = GameInfo(str(Path(rom).resolve()).encode(), C.cast(self.rom, C.c_void_p), len(data), None)
        self.core.retro_load_game.argtypes = [C.POINTER(GameInfo)]; self.core.retro_load_game.restype = C.c_bool
        assert self.core.retro_load_game(C.byref(info)), 'ROM load failed'
        av = AV(); self.core.retro_get_system_av_info.argtypes = [C.POINTER(AV)]; self.core.retro_get_system_av_info(C.byref(av))
        self.fps, self.rate = av.timing.fps, av.timing.rate
        self.core.retro_get_memory_data.argtypes = [C.c_uint]; self.core.retro_get_memory_data.restype = C.c_void_p
        self.core.retro_get_memory_size.argtypes = [C.c_uint]; self.core.retro_get_memory_size.restype = C.c_size_t
        self.symbols = {}
        lab = Path(rom).with_suffix('.labels')
        if lab.exists():
            for line in lab.read_text().splitlines():
                p = line.split()
                if len(p) == 3 and p[0] == 'al': self.symbols[p[2].lstrip('.').lstrip('_')] = int(p[1], 16)
        self.frames = 0
    def run(self, n=1, pad0=0, pad1=0):
        self.pad = [pad0, pad1]
        for _ in range(n): self.core.retro_run(); self.frames += 1
    def tap(self, mask, pad=0, hold=3, wait=3):
        self.pad = [0, 0]; self.pad[pad] = mask
        self.run(hold, *self.pad); self.run(wait)
    def wram(self, addr, size=1):
        base = self.core.retro_get_memory_data(0); assert base, 'no SAVE_RAM exposed'
        return bytes(C.string_at(base + addr - 0x6000, size))
    def sym(self, name, size=1):
        a = self.symbols[name]
        if a >= 0x6000: return int.from_bytes(self.wram(a, size), 'little')
        raise KeyError(name)
    def snap(self, name):
        Image.fromarray(self.frame).save(self.out / (name + '.png')); return self.out / (name + '.png')
