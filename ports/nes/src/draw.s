; Hot sprite emitters.  cc65 code is too slow for 40+ OAM entries per frame, so
; the player and ball compositions are written in assembly.  Inputs are zero-page
; variables set by main.c; output goes to the OAM shadow at oam (index oi).
.include "chr_layout.inc"
.export _draw_player, _draw_ball, _trim_rows
.import _pl_y, _ball_top, _pl_nib
.export _oi, _dr_x, _dr_y, _dr_tile, _dr_flip, _dr_ptr, _dr_bx, _dr_shy
.import _oam

.segment "ZEROPAGE"
_oi:      .res 1     ; byte index into oam (multiple of 4)
_dr_x:    .res 1     ; player: left x
_dr_y:    .res 1     ; player: top y; ball: centre y
_dr_tile: .res 1     ; first tile (player pose base / ball frame base)
_dr_flip: .res 1     ; 0 or 1 (horizontal flip)
_dr_ptr:  .res 2     ; -> 4 row nibbles of the pose mask
_dr_bx:   .res 2     ; ball centre x (signed 16)
_dr_shy:  .res 1     ; shadow top y
ry:       .res 1
rt:       .res 1
rrow:     .res 1
nib:      .res 1
k:        .res 1
cx:       .res 2

.segment "RODATA"
col_order: .byte 1, 2, 0, 3
bit_tbl:   .byte 1, 2, 4, 8
off_n:     .byte 0, 8, 16, 24
off_f:     .byte 24, 16, 8, 0

.segment "CODE"
; 16 tiles at most; columns visited 1,2,0,3 so edge tiles are dropped first.
_draw_player:
    lda _dr_y
    sec
    sbc #1
    sta ry
    lda _dr_tile
    sta rt
    lda #0
    sta rrow
@row:
    ldy rrow
    lda (_dr_ptr),y
    sta nib
    beq @next_row
    lda #0
    sta k
@col:
    ldx k
    ldy col_order,x
    lda bit_tbl,y
    and nib
    beq @skip
    ldx _oi
    lda ry
    sta _oam,x
    inx
    tya
    clc
    adc rt
    sta _oam,x
    inx
    lda _dr_flip
    beq @noflip
    lda #$40
    sta _oam,x
    inx
    lda _dr_x
    clc
    adc off_f,y
    sta _oam,x
    inx
    jmp @emitted
@noflip:
    lda #0
    sta _oam,x
    inx
    lda _dr_x
    clc
    adc off_n,y
    sta _oam,x
    inx
@emitted:
    stx _oi
@skip:
    inc k
    lda k
    cmp #4
    bne @col
@next_row:
    lda ry
    clc
    adc #8
    sta ry
    lda rt
    clc
    adc #4
    sta rt
    inc rrow
    lda rrow
    cmp #4
    bne @row
    rts

; Ball 3x3 tiles (attr 1) at centre (dr_bx, dr_y) + 4-tile shadow (attr 2).
; Tiles whose x leaves 0..255 are skipped instead of wrapping.
_draw_ball:
    lda _dr_y
    sec
    sbc #13
    sta ry
    lda _dr_tile
    sta rt
    lda #0
    sta rrow
@brow:
    lda _dr_bx
    sec
    sbc #12
    sta cx
    lda _dr_bx+1
    sbc #0
    sta cx+1
    ldy #0
@bcol:
    lda cx+1
    bne @bskip
    ldx _oi
    lda ry
    sta _oam,x
    inx
    tya
    clc
    adc rt
    sta _oam,x
    inx
    lda #1
    sta _oam,x
    inx
    lda cx
    sta _oam,x
    inx
    stx _oi
@bskip:
    lda cx
    clc
    adc #8
    sta cx
    bcc @b1
    inc cx+1
@b1:
    iny
    cpy #3
    bne @bcol
    lda ry
    clc
    adc #8
    sta ry
    lda rt
    clc
    adc #3
    sta rt
    inc rrow
    lda rrow
    cmp #3
    bne @brow
    ; shadow
    lda _dr_bx
    sec
    sbc #16
    sta cx
    lda _dr_bx+1
    sbc #0
    sta cx+1
    ldy #0
@scol:
    lda cx+1
    bne @sskip
    ldx _oi
    lda _dr_shy
    sec
    sbc #1
    sta _oam,x
    inx
    tya
    clc
    adc #TILE_SHADOW
    sta _oam,x
    inx
    lda #2
    sta _oam,x
    inx
    lda cx
    sta _oam,x
    inx
    stx _oi
@sskip:
    lda cx
    clc
    adc #8
    sta cx
    bcc @s1
    inc cx+1
@s1:
    iny
    cpy #4
    bne @scol
    rts

; ---------------------------------------------------------------------------
; trim_rows: the PPU draws 8 sprites per scanline.  Each player tile row (index x = player*4 + row)
; shares its scanlines with the pole's sprites (1 extra, lines 176..194) and the ball (3 extra);
; the remaining budget 8 - extra is split between two players that stand at similar heights
; (player 1 gets the odd one).  Tail/edge columns are dropped first (drop1 table).
.segment "RODATA"
pop4:  .byte 0,1,1,2,1,2,2,3,1,2,2,3,2,3,3,4
drop1: .byte 0,0,0,2,0,4,4,6,0,8,2,10,4,12,6,14
.segment "ZEROPAGE"
t_y:     .res 1
t_cap:   .res 1
t_share: .res 1
t_tmp:   .res 1
.segment "CODE"
_trim_rows:
    lda _pl_y
    sec
    sbc _pl_y+1
    bcs @abs
    eor #$FF
    adc #1
@abs:
    ldy #0
    cmp #32
    bcs @noshare
    iny
@noshare:
    sty t_share
    ldx #0
@next:
    txa
    lsr
    lsr
    tay
    lda _pl_y,y
    sta t_y
    txa
    and #3
    asl
    asl
    asl
    clc
    adc t_y
    sta t_y                 ; y of this tile row
    ldy #0                  ; extra sprites on its scanlines
    lda t_y
    cmp #169                ; pole lines 176..194: row overlaps when 168 < y < 195
    bcc @nopole
    cmp #195
    bcs @nopole
    iny
@nopole:
    lda _ball_top
    clc
    adc #24
    sta t_tmp               ; ball lines [ball_top, ball_top+24)
    lda t_y
    cmp t_tmp
    bcs @noball
    clc
    adc #8
    cmp _ball_top
    beq @noball
    bcc @noball
    iny
    iny
    iny
@noball:
    cpy #0
    beq @skip
    sty t_tmp
    lda #8
    sec
    sbc t_tmp
    sta t_cap
    lda t_share
    beq @trim
    txa
    and #4
    bne @second
    lda t_cap
    clc
    adc #1
    lsr
    sta t_cap
    jmp @trim
@second:
    lsr t_cap
@trim:
    lda _pl_nib,x
@again:
    tay
    lda pop4,y
    cmp t_cap
    beq @store
    bcc @store
    lda drop1,y
    jmp @again
@store:
    tya
    sta _pl_nib,x
@skip:
    inx
    cpx #8
    bne @next
    rts
