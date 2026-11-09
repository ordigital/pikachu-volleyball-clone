; Ball trajectory prediction used by the computer player (hot loop).
; Mirrors predict() / predict_power() in physics.c exactly:
;   _pr_land  - normal rules incl. the side-of-net bounce  (mode 0)
;   _pr_power - power-hit rules, vy flip over the net only (mode 1)
; In: _pr_x/_pr_y/_pr_vx/_pr_vy (signed 16).  Out: _pr_x = landing x.
HALF   = 216
WIDTH  = 432
RADIUS = 20
LIMIT  = 1000
.export _pr_land, _pr_power, _pr_x, _pr_y, _pr_vx, _pr_vy

.segment "ZEROPAGE"
_pr_x:  .res 2
_pr_y:  .res 2
_pr_vx: .res 2
_pr_vy: .res 2
pn:     .res 2
t:      .res 2
mode:   .res 1

.macro NEG16 v
    sec
    lda #0
    sbc v
    sta v
    lda #0
    sbc v+1
    sta v+1
.endmacro

.segment "CODE"
_pr_land:
    lda #0
    beq start
_pr_power:
    lda #1
start:
    sta mode
    lda #1
    sta pn
    lda #0
    sta pn+1
loop:
    ; wall: next = x + vx; flip vx when next < RADIUS or next > WIDTH
    clc
    lda _pr_x
    adc _pr_vx
    sta t
    lda _pr_x+1
    adc _pr_vx+1
    sta t+1
    bmi flip
    lda t+1
    bne chk_hi
    lda t
    cmp #RADIUS
    bcc flip
chk_hi:
    lda t+1
    cmp #>WIDTH
    bcc nowall
    bne flip
    lda t
    cmp #<WIDTH+1
    bcc nowall
flip:
    NEG16 _pr_vx
nowall:
    ; ceiling: y + vy < 0  ->  vy = 1
    clc
    lda _pr_y
    adc _pr_vy
    lda _pr_y+1
    adc _pr_vy+1
    bpl nofloor
    lda #1
    sta _pr_vy
    lda #0
    sta _pr_vy+1
nofloor:
    ; net zone: |x - HALF| < 25 and y > 176
    sec
    lda _pr_x
    sbc #HALF
    sta t
    lda _pr_x+1
    sbc #0
    sta t+1
    bpl dxpos
    lda t+1
    cmp #$FF
    bne nonet
    lda t
    cmp #$E8             ; dx >= -24
    bcc nonet
    bcs inzone
dxpos:
    lda t+1
    bne nonet
    lda t
    cmp #25
    bcs nonet
inzone:
    lda _pr_y+1
    bmi nonet
    bne yhigh
    lda _pr_y
    cmp #177
    bcc nonet
yhigh:
    lda mode
    bne power_rule
    lda _pr_y+1
    bne side
    lda _pr_y
    cmp #192
    bcs side
    ; 176 < y < 192: bounce off the net top when falling
    lda _pr_vy+1
    bmi nonet
    ora _pr_vy
    beq nonet
    NEG16 _pr_vy
    jmp nonet
side:
    lda _pr_vx+1
    bpl vxabs
    NEG16 _pr_vx
vxabs:
    lda t+1
    bpl nonet            ; x >= HALF: vx = +|vx|
    NEG16 _pr_vx         ; x <  HALF: vx = -|vx|
    jmp nonet
power_rule:
    lda _pr_vy+1
    bmi nonet
    ora _pr_vy
    beq nonet
    NEG16 _pr_vy
nonet:
    ; y += vy; stop when y > 252 or n >= LIMIT
    clc
    lda _pr_y
    adc _pr_vy
    sta _pr_y
    lda _pr_y+1
    adc _pr_vy+1
    sta _pr_y+1
    bmi cont
    bne done
    lda _pr_y
    cmp #253
    bcs done
cont:
    lda pn+1
    cmp #>LIMIT
    bcc go
    bne done
    lda pn
    cmp #<LIMIT
    bcs done
go:
    inc pn
    bne :+
    inc pn+1
:   clc
    lda _pr_x
    adc _pr_vx
    sta _pr_x
    lda _pr_x+1
    adc _pr_vx+1
    sta _pr_x+1
    inc _pr_vy
    bne :+
    inc _pr_vy+1
:   jmp loop
done:
    rts
