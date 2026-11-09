#!/usr/bin/env python3
"""Compare every simulation field and RNG state against the actual Rust sources."""
from pathlib import Path
import re
import os
import subprocess
import tempfile

PORT = Path(__file__).resolve().parents[1]
ROOT = PORT.parents[1]
width=int(re.search(r'^#define MD_WORLD_WIDTH (\d+)$',(PORT/'src/scene.h').read_text(),re.M)[1])
source = (ROOT / 'src/physics.rs').read_text()
schemas = {}
for name in ['PikaUserInput', 'PlayerSound', 'BallSound', 'Player', 'Ball']:
    body = re.search(r'pub struct ' + name + r' \{(.*?)\n\}', source, re.S)[1]
    schemas[name] = re.findall(r'pub (\w+): (\w+),', body)

def fields(name, prefix):
    result = []
    for field, typ in schemas[name]:
        path = prefix + '.' + field
        result += fields(typ, path) if typ in schemas else [(path, typ)]
    return result

all_fields = fields('Player', 'p.player1') + fields('Player', 'p.player2') + fields('Ball', 'p.ball')
with tempfile.TemporaryDirectory(prefix='pika-md-parity-') as temp:
    work = Path(temp)
    # Private seed/state hooks in this temporary test adapter, never production Rust.
    rand = (ROOT / 'src/rand.rs').read_text()
    rand += '\npub fn seed(v:u64){STATE.with(|s|s.set(v));}\npub fn state()->u64{STATE.with(|s|s.get())}\n'
    (work / 'rand.rs').write_text(rand)
    rust = '''mod scene { pub const WIDTH:f32=544.0; }
mod rand;
#[path="PHYSICS"] mod physics;
use physics::*;
fn main() {
 for mode in 0..3 {
 rand::seed(1234); let mut p=PikaPhysics::new(mode==2,mode!=0);
 for tick in 0..20000 {
 if tick%157==156 { p.player1.initialize_for_new_round();p.player2.initialize_for_new_round();p.ball.initialize_for_new_round(tick%2==0); }
 let mut input=[PikaUserInput {x_direction:(tick/13)%3-1,y_direction:if tick%43<2 {-1} else { (tick/17)%2 },power_hit:if tick%19==0 {1}else{0}};2];
 let hit=p.run_engine_for_next_frame(&mut input);
 println!("FORMAT",ARGS,hit as i32,rand::state());
 // Match controller consumption; exercise winner poses independently too.
 p.player1.sound=PlayerSound::default();p.player2.sound=PlayerSound::default();p.ball.sound=BallSound::default();
 if tick%509==508 { p.player1.game_ended=true;p.player1.is_winner=true;p.player2.game_ended=true; }
 if tick%509==0 { p.player1.game_ended=false;p.player2.game_ended=false; }
 }
 }
}
'''.replace('PHYSICS', str(ROOT / 'src/physics.rs')).replace('WIDTH:f32=544.0',f'WIDTH:f32={width}.0')
    rust = rust.replace('FORMAT', ' '.join(['{}'] * (len(all_fields) + 2)))
    rust = rust.replace('ARGS', ','.join(path + ' as i32' for path, _ in all_fields))
    (work / 'main.rs').write_text(rust)
    c = '''#include "physics.h"
#include <stdio.h>
#include <inttypes.h>
int main(void) {
 for(int mode=0;mode<3;mode++) {
 PikaPhysics p; physics_init(&p,mode==2,mode!=0,1234);
 for(int tick=0;tick<20000;tick++) {
 if(tick%157==156) physics_round(&p,tick%2==0);
 PikaUserInput value={(tick/13)%3-1,tick%43<2?-1:(tick/17)%2,tick%19==0};
 PikaUserInput in[2]={value,value}; bool hit=physics_tick(&p,in);
 printf("FORMAT%d %" PRIu64 "\\n",ARGS,hit,physics_rng_state());
 p.player1.sound=(PlayerSound){0};p.player2.sound=(PlayerSound){0};p.ball.sound=(BallSound){0};
 if(tick%509==508) {p.player1.game_ended=true;p.player1.is_winner=true;p.player2.game_ended=true;}
 if(tick%509==0) {p.player1.game_ended=false;p.player2.game_ended=false;}
 }
 }
}
'''.replace('FORMAT', '%d ' * len(all_fields)).replace('ARGS', ','.join('(int)' + path for path, _ in all_fields))
    (work / 'main.c').write_text(c)
    subprocess.run(['rustc', '-Awarnings', '-O', str(work / 'main.rs'), '-o', str(work / 'reference')], check=True)
    subprocess.run(['cc', '-std=c99', '-Wall', '-Wextra', '-Werror', '-fsanitize=undefined,address', '-O2', '-I', str(PORT / 'src'), str(work / 'main.c'), str(PORT / 'src/physics.c'), '-o', str(work / 'port')], check=True)
    expected = subprocess.check_output([str(work / 'reference')]).splitlines()
    actual = subprocess.check_output([str(work / 'port')], env={**os.environ, 'ASAN_OPTIONS': 'detect_leaks=0'}).splitlines()
    assert len(expected) == len(actual)
    for tick, (a, b) in enumerate(zip(expected, actual)):
        if a != b:
            for index, (av, bv) in enumerate(zip(a.split(), b.split())):
                if av != bv:
                    label = all_fields[index][0] if index < len(all_fields) else ['ground', 'rng'][index-len(all_fields)]
                    raise AssertionError(f'mode {tick//20000}, tick {tick%20000}, {label}: Rust={av}, C={bv}')
    print(f'PASS: {len(actual)} ticks, all {len(all_fields)} fields + ground event + RNG; human/AI/AI-vs-AI; sanitizers enabled.')
