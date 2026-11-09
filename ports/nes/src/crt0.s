; Start-up, vectors and NMI for the MMC3 (mapper 4) build.
; The NMI owns all PPU access while rendering is on: sprite DMA, the queued
; VRAM writes in vbuf, CHR bank registers from chr_bank[], scroll/control.
.export __STARTUP__ : absolute = 1
.exportzp _region_pal, _pad_cur, _pad_edge
.export _take_edges
.export _nmi_frame, _ppu_ctrl, _ppu_mask, _chr_bank, _chr_next, _vbuf, _vbuf_len, _oam_ready
.import _main, copydata, zerobss, music_tick
.importzp sp
.import __SRAM_START__, __SRAM_SIZE__

.segment "HEADER"
    .byte "NES", $1A
    .byte 2              ; 2 x 16 KiB PRG
    .byte 4              ; 4 x 8 KiB CHR
    .byte $41            ; mapper 4, vertical mirroring bit
    .byte $00
    .byte 1              ; 8 KiB PRG RAM
    .byte 0,0,0,0,0,0,0

.segment "ZEROPAGE"
_nmi_frame:   .res 1
_ppu_ctrl:    .res 1
_ppu_mask:    .res 1
_vbuf_len:    .res 1
_oam_ready:   .res 1
_chr_bank:    .res 6
_chr_next:    .res 6
_region_pal:  .res 1
_pad_cur:     .res 2     ; stable state of both pads (A=1 B=2 Sel=4 Start=8 U=16 D=32 L=64 R=128)
_pad_edge:    .res 2     ; rising edges accumulated since main last looked
pad_prev:     .res 2
pad_a:        .res 2
pad_b:        .res 2
pad_lock:     .res 1     ; set while main takes edges: the NMI then skips one poll
pal_tmp:      .res 1

.export _oam
.segment "OAMBUF"
_oam:         .res 256
.segment "BSS"
_vbuf:        .res 128

.segment "STARTUP"
reset:
    sei
    cld
    ldx #$40
    stx $4017
    ldx #$FF
    txs
    inx
    stx $2000
    stx $2001
    stx $4010
    stx $4015                     ; stop any DMC sample from before a soft reset
    bit $2002
:   bit $2002
    bpl :-
    ; clear internal RAM, park sprites
    lda #0
    tax
:   sta $0000,x
    sta $0100,x
    sta $0300,x
    sta $0400,x
    sta $0500,x
    sta $0600,x
    sta $0700,x
    inx
    bne :-
    lda #$F0
:   sta $0200,x
    inx
    bne :-
:   bit $2002
    bpl :-
    ; Region: count 12-cycle loop passes over one frame (NTSC ~2480, PAL/Dendy >2560)
    ldx #0
    ldy #0
:   inx
    bne :+
    iny
:   bit $2002
    bpl :--
    cpy #10
    lda #0
    rol
    sta pal_tmp
    ; MMC3: vertical mirroring, WRAM enabled, IRQ off, fixed 32 KiB PRG
    lda #0
    sta $A000
    lda #$80
    sta $A001
    sta $E000
    lda #6
    sta $8000
    lda #0
    sta $8001
    lda #7
    sta $8000
    lda #1
    sta $8001
    lda #<(__SRAM_START__ + __SRAM_SIZE__)
    sta sp
    lda #>(__SRAM_START__ + __SRAM_SIZE__)
    sta sp+1
    jsr zerobss
    jsr copydata
    lda pal_tmp
    sta _region_pal
    jmp _main

nmi:
    pha
    txa
    pha
    tya
    pha
    inc _nmi_frame
    lda _oam_ready
    beq @no_oam
    lda #0
    sta $2003
    lda #2
    sta $4014
    ldx #5
@cp:
    lda _chr_next,x
    sta _chr_bank,x
    dex
    bpl @cp
    lda #0
    sta _oam_ready
@no_oam:
    ldx #0
@next:
    cpx _vbuf_len
    bcs @done
    lda _vbuf,x
    sta $2006
    inx
    lda _vbuf,x
    sta $2006
    inx
    ldy _vbuf,x
    inx
@copy:
    lda _vbuf,x
    sta $2007
    inx
    dey
    bne @copy
    beq @next
@done:
    lda #0
    sta _vbuf_len
    ; CHR banks R0..R5
    ldx #0
@bank:
    stx $8000
    lda _chr_bank,x
    sta $8001
    inx
    cpx #6
    bne @bank
    lda #0
    sta $2005
    sta $2005
    lda _ppu_ctrl
    sta $2000
    lda _ppu_mask
    sta $2001
    jsr music_tick
    jsr read_pads
    pla
    tay
    pla
    tax
    pla
    rti

; Controllers are polled here every frame so a short tap is never lost while the
; main loop is busy.  A DMC fetch can swallow a bit: accept two equal reads.
read_pads:
    lda pad_lock
    bne @skip
    jsr read_once
    lda pad_a
    sta pad_b
    lda pad_a+1
    sta pad_b+1
    ldy #6                        ; give up after 6 mismatching reads
@again:
    jsr read_once
    lda pad_a
    cmp pad_b
    bne @differ
    lda pad_a+1
    cmp pad_b+1
    beq @stable
@differ:
    lda pad_a
    sta pad_b
    lda pad_a+1
    sta pad_b+1
    dey
    bne @again
@skip:
    rts                           ; no two reads agreed: keep the previous state
@stable:
    ldx #1
@each:
    lda pad_a,x
    tay
    eor pad_prev,x
    and pad_a,x                   ; newly pressed bits
    ora _pad_edge,x
    sta _pad_edge,x
    sty pad_prev,x
    sty _pad_cur,x
    dex
    bpl @each
    rts

read_once:
    lda #1
    sta $4016
    lda #0
    sta $4016
    ldx #8
@bit:
    lda $4016
    lsr
    ror pad_a
    lda $4017
    lsr
    ror pad_a+1
    dex
    bne @bit
    rts

; uint8_t take_edges(uint8_t pad): returns and clears the accumulated edges.  sei does
; not mask NMI, so a lock makes read_pads skip a poll instead (the edge is not lost:
; pad_prev is unchanged and the next poll sees it again).
_take_edges:
    tax
    lda #1
    sta pad_lock
    lda _pad_edge,x
    pha
    lda #0
    sta _pad_edge,x
    sta pad_lock
    pla
    ldx #0
    rts

irq:
    rti

.segment "VECTORS"
    .word nmi
    .word reset
    .word irq
