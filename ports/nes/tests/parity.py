#!/usr/bin/env python3
"""Run the NES 16-bit physics and the Mega Drive 32-bit physics (itself checked
against Rust) on the same long input script and compare every shared field.
The NES file is built with its host RNG switch so both use SplitMix64."""
from pathlib import Path
import subprocess, tempfile, sys
PORT = Path(__file__).resolve().parents[1]
MD = PORT.parent / 'megadrive'
FIELDS = ['x','y','y_velocity','is_collision_with_ball_happened','state','frame_number',
 'normal_status_arm_swing_direction','delay_before_next_frame','computer_boldness','diving_direction',
 'lying_down_duration_left','is_winner','game_ended','computer_where_to_stand_by',
 'sound.pipikachu','sound.pika','sound.chu']
BALL = ['x','y','x_velocity','y_velocity','punch_effect_radius','is_power_hit','rotation','fine_rotation',
 'punch_effect_x','punch_effect_y','sound.power_hit','sound.ball_touches_ground']
def driver(nes):
    names = ['p.player1.'+f for f in FIELDS]+['p.player2.'+f for f in FIELDS]+['p.ball.'+f for f in BALL]
    cast = '(long)'
    pr = ''.join(f'printf("%ld ",{cast}{n});' for n in names)
    return f'''#include "physics.h"
#include <stdio.h>
int main(void){{
 for(int mode=0;mode<3;mode++){{
  PikaPhysics p; physics_init(&p,mode==2,mode!=0,1234);
  for(int tick=0;tick<60000;tick++){{
   if(tick%157==156) physics_round(&p,tick%2==0);
   PikaUserInput in[2];
   for(int i=0;i<2;i++){{in[i].x_direction=((tick+i*5)/13)%3-1;in[i].y_direction=(tick%43<2)?-1:((tick+i)/17)%2;in[i].power_hit=((tick+i*3)%19==0);}}
   int hit=physics_tick(&p,in);
   printf("%d ",hit);{pr}
   if(mode!=0||1) printf("%d",{'0' if nes else 'p.ball.expected_landing_point_x*0'});
   puts("");
   p.player1.sound=(PlayerSound){{0}};p.player2.sound=(PlayerSound){{0}};p.ball.sound=(BallSound){{0}};
   if(tick%509==508){{p.player1.game_ended=1;p.player1.is_winner=1;p.player2.game_ended=1;}}
   if(tick%509==0){{p.player1.game_ended=0;p.player2.game_ended=0;}}
  }}
 }}
}}
'''
def run(name, src, inc, defs):
    with tempfile.TemporaryDirectory() as t:
        t = Path(t); (t/'d.c').write_text(driver(name == 'nes'))
        subprocess.run(['gcc','-O1','-fsanitize=undefined,address','-fno-sanitize-recover=undefined','-I',inc,*defs,'-o',t/'d',t/'d.c',src],check=True)
        return subprocess.run([t/'d'],check=True,capture_output=True,text=True).stdout.splitlines()
a = run('nes', PORT/'src/physics.c', str(PORT/'src'), ['-DPIKA_HOST','-DPIKA_REF_RNG'])
b = run('md', MD/'src/physics.c', str(MD/'src'), [])
assert len(a) == len(b) == 180000, (len(a), len(b))
bad = [i for i,(x,y) in enumerate(zip(a,b)) if x != y]
if bad:
    i = bad[0]; print('first mismatch at line', i, '\nNES', a[i], '\nMD ', b[i]); sys.exit(1)
print('OK', len(a), 'ticks identical (NES 16-bit == MD 32-bit)')
