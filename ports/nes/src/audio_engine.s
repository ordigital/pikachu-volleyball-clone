; NES sound: 60 Hz music sequencer on pulse 1/2, triangle and noise, plus
; DPCM voice effects on the DMC channel.  music_tick runs from the NMI.
; Stream entry = (note, frames); note 0 = rest, $FF = loop.  The noise stream
; uses drum types 1..4 (kick, snare, hat, crash).
.include "audio_tables.inc"
.export _music_start, _music_stop, _music_pause, _sfx_play, music_tick
.importzp _region_pal
.import music_p1, music_p2, music_tri, music_noise, note_lo, note_hi
.import dpcm0, dpcm1, dpcm2, dpcm3, dpcm4, dpcm5, dpcm6, dpcm7

.segment "ZEROPAGE"
mus_on:     .res 1
mus_ptr_lo: .res 4
mus_ptr_hi: .res 4
mus_rem:    .res 4
mus_note:   .res 4
mus_age:    .res 4
mus_hi:     .res 3        ; last $4003/$4007/$400B high byte per tonal channel
mus_tmp:    .res 2
mus_reg:    .res 1
sfx_cur:    .res 1
pal_cnt:    .res 1
duck:       .res 1     ; DMC voice running: music is turned down
syn_n:      .res 1     ; noise effect frames left
syn_base:   .res 1
syn_per:    .res 1
syn_p:      .res 1     ; pulse chirp frames left

.segment "AUDIO"
stream_lo: .byte <music_p1, <music_p2, <music_tri, <music_noise
stream_hi: .byte >music_p1, >music_p2, >music_tri, >music_noise
; per-frame volume by note age (0..15)
env_p1: .byte 8,8,7,7,7,6,6,6,6,5,5,5,5,5,5,5
env_p2: .byte 6,6,5,5,5,4,4,4,4,3,3,3,3,3,3,3
; noise: types 1..4 (index 0 unused) - period/mode byte and 16-frame envelopes
noise_per: .byte 0, $0C, $07, $82, $05
noise_env:
    .byte 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0
    .byte 10,9,8,7,5,4,3,2,1,0,0,0,0,0,0,0       ; kick
    .byte  8,8,7,5,4,3,2,2,1,0,0,0,0,0,0,0       ; snare
    .byte  5,4,3,1,0,0,0,0,0,0,0,0,0,0,0,0       ; hat
    .byte  7,7,6,6,6,5,5,4,4,4,3,3,3,2,2,2       ; crash
sfx_prio: .byte 4, 2, 1, 0, 4, 3, 5, 2
; punchy APU layers on top of the (quiet 1-bit) DPCM voices: 1 hit, 2 power hit, 3 ground, 4 pika chirp
sfx_syn:  .byte 0, 4, 0, 0, 0, 2, 3, 1
syn_pers: .byte 0, $07, $0C, $0E
syn_env:                                          ; 12 frames per noise effect
    .byte 0,0,0,0,0,0,0,0,0,0,0,0
    .byte 14,11, 7, 4, 2, 1, 0, 0, 0, 0, 0, 0   ; hit
    .byte 15,15,14,12,10, 8, 6, 5, 4, 2, 1, 0   ; power hit
    .byte 12,10, 8, 6, 4, 3, 2, 1, 0, 0, 0, 0   ; ground
chirp:    .byte 70, 62, 55, 49, 44                ; "pik": five rising frames on pulse 2
sfx_addr: .byte <(dpcm0 >> 6), <(dpcm1 >> 6), <(dpcm2 >> 6), <(dpcm3 >> 6), <(dpcm4 >> 6), <(dpcm5 >> 6), <(dpcm6 >> 6), <(dpcm7 >> 6)
sfx_rate: .byte DPCM_RATES
sfx_len:  .byte DPCM0_LEN, DPCM1_LEN, DPCM2_LEN, DPCM3_LEN, DPCM4_LEN, DPCM5_LEN, DPCM6_LEN, DPCM7_LEN

synth_step:
    lda syn_n
    beq @pulse
    lda #12
    sec
    sbc syn_n
    clc
    adc syn_base
    tay
    lda syn_env,y
    ora #$30
    sta $400C
    lda syn_n
    cmp #12
    bne @nr
    lda syn_per
    sta $400E
    lda #$F8
    sta $400F
@nr:
    dec syn_n
@pulse:
    lda syn_p
    beq @done
    lda #5
    sec
    sbc syn_p
    tay
    lda #$BC
    sta $4004
    lda #$08
    sta $4005
    lda chirp,y
    sta $4006
    cpy #0
    bne @np
    lda #0
    sta $4007
@np:
    dec syn_p
    bne @done
    lda #$FF
    sta mus_hi+1                ; music rewrites pulse 2's high byte on its next note
@done:
    rts

_music_start:
    ldx #3
@init:
    lda stream_lo,x
    sta mus_ptr_lo,x
    lda stream_hi,x
    sta mus_ptr_hi,x
    lda #0
    sta mus_rem,x
    sta mus_note,x
    sta mus_age,x
    dex
    bpl @init
    lda #$FF
    sta mus_hi
    sta mus_hi+1
    sta mus_hi+2
    lda #1
    sta mus_on
    jsr enable_channels
    rts

_music_stop:
    lda #0
    sta mus_on
silence:
    lda #$30
    sta $4000
    sta $4004
    sta $400C
    lda #$80
    sta $4008
    rts

_music_pause:           ; A != 0: pause (silence), A == 0: resume
    cmp #0
    bne @pause
    lda #1
    sta mus_on
    lda #$FF
    sta mus_hi
    sta mus_hi+1
    sta mus_hi+2
    jsr enable_channels
    rts
@pause:
    lda #0
    sta mus_on
    jmp silence

; Pulse/triangle/noise on; DMC enable bit preserved (writing 0 would cut a voice).
enable_channels:
    lda $4015
    and #$10
    ora #$0F
    sta $4015
    rts

; The score is paced for 60 Hz: on 50 Hz consoles run a second step every 5th frame.
music_tick:
    jsr music_step
    lda _region_pal
    beq @out
    inc pal_cnt
    lda pal_cnt
    cmp #5
    bne @out
    lda #0
    sta pal_cnt
    jsr music_step
@out:
    jmp synth_step

music_step:
    lda mus_on
    bne @go
    rts
@go:
    lda $4015                   ; duck the music while a voice or synth effect plays
    and #$10
    ora syn_n
    ora syn_p
    sta duck
    ldx #0
@chan:
    lda mus_rem,x
    bne @have
@fetch:
    lda mus_ptr_lo,x
    sta mus_tmp
    lda mus_ptr_hi,x
    sta mus_tmp+1
    ldy #0
    lda (mus_tmp),y
    cmp #$FF
    bne @entry
    lda stream_lo,x
    sta mus_ptr_lo,x
    lda stream_hi,x
    sta mus_ptr_hi,x
    jmp @fetch
@entry:
    sta mus_note,x
    iny
    lda (mus_tmp),y
    sta mus_rem,x
    lda #0
    sta mus_age,x
    clc
    lda mus_ptr_lo,x
    adc #2
    sta mus_ptr_lo,x
    bcc @have
    inc mus_ptr_hi,x
@have:
    lda mus_age,x
    cmp #15
    bcc :+
    lda #15
:   sta mus_tmp+1            ; clamped age
    cpx #2
    bcc @pulse
    beq @tri_j
    jmp @noise
@tri_j:
    jmp @tri
@pulse:
    ; ---- pulse 1/2
    txa
    asl
    asl
    sta mus_reg
    lda mus_note,x
    beq @pulse_off
    tay
    lda note_lo,y
    sta mus_tmp
    lda note_hi,y
    pha
    ldy mus_tmp+1
    cpx #0
    bne @env2
    lda env_p1,y
    ora #$B0
    bne @vol
@env2:
    lda env_p2,y
    ora #$70
@vol:
    ldy duck
    beq @loud
    tay
    and #$F0
    sta mus_tmp+1
    tya
    and #$0F
    lsr
    ora mus_tmp+1
@loud:
    ldy mus_reg
    sta $4000,y
    lda #$08
    sta $4001,y
    lda mus_tmp
    sta $4002,y
    pla
    cmp mus_hi,x
    beq @pulse_done
    sta mus_hi,x
    sta $4003,y
    jmp @pulse_done
@pulse_off:
    ldy mus_reg
    lda #$30
    sta $4000,y
    lda #$FF
    sta mus_hi,x
@pulse_done:
    jmp @next
    ; ---- triangle
@tri:
    lda duck
    bne @tri_off
    lda mus_note,x
    beq @tri_off
    tay
    lda note_hi,y
    lsr
    sta mus_tmp+1
    lda note_lo,y
    ror
    sta $400A
    lda mus_tmp+1
    cmp mus_hi+2
    beq :+
    sta mus_hi+2
    sta $400B
:   lda #$FF
    sta $4008
    jmp @next
@tri_off:
    lda #$80
    sta $4008
    lda #$FF
    sta mus_hi+2
    jmp @next
    ; ---- noise
@noise:
    lda mus_note,x
    beq @noise_off
    asl
    asl
    asl
    asl                        ; type * 16
    clc
    adc mus_tmp+1              ; + clamped age
    tay
    lda noise_env,y
    ldy duck
    beq :+
    lsr
:   ora #$30
    sta $400C
    lda mus_age,x
    bne @next
    ldy mus_note,x
    lda noise_per,y
    sta $400E
    lda #$F8
    sta $400F
    jmp @next
@noise_off:
    lda #$30
    sta $400C
@next:
    dec mus_rem,x
    inc mus_age,x
    bne :+
    dec mus_age,x               ; saturate at 255
:   inx
    cpx #4
    bne @chan_jump
    rts
@chan_jump:
    jmp @chan

_sfx_play:                      ; A = effect id 0..6
    tax
    lda $4015
    and #$10
    beq @start                  ; DMC idle
    lda sfx_prio,x
    cmp sfx_cur
    bcc @ret                    ; weaker than the running effect
@start:
    lda sfx_prio,x
    sta sfx_cur
    lda #$0F                    ; $4015 reads report length counters, not the enables
    sta $4015                   ; stop the previous sample, keep the music channels on
    lda sfx_rate,x
    sta $4010
    lda #$40
    sta $4011
    lda sfx_addr,x
    sta $4012
    lda sfx_len,x
    sta $4013
    lda #$1F
    sta $4015
    lda sfx_syn,x
    beq @ret
    cmp #4
    bne @noise
    lda #5
    sta syn_p
    rts
@noise:
    tay
    lda syn_pers,y
    sta syn_per
    tya
    asl
    sta mus_tmp
    asl
    clc
    adc mus_tmp                 ; type * 6 ...
    asl                         ; ... * 12
    sta syn_base
    lda #12
    sta syn_n
@ret:
    rts
