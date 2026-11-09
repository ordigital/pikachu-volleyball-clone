.export _game_nt, _game_pal, _menu_nt, _menu_pal, _pose_mask
.segment "CHARS"
    .incbin "res/chr.bin"
.segment "RODATA"
_game_nt:   .incbin "res/game_nt.bin"
_game_pal:  .incbin "res/pal_game.bin"
_menu_nt:   .incbin "res/menu_nt.bin"
_menu_pal:  .incbin "res/pal_menu.bin"
_pose_mask: .incbin "res/pose_mask.bin"
