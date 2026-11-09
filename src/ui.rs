//! Własne (rysowane w macroquad) menu u góry okna i okno „Controls”.
//! Menu: Game, Controls, Music, About.

use crate::input::{key_name, Controls, PadBtn, ACTION_LABELS};
use crate::pad::Pads;
use macroquad::prelude::*;

const SECONDARY_TEXT: Color = Color::new(0.76, 0.79, 0.84, 1.0);

const MENUS: &[(&str, &[&str])] = &[
    ("Game", &["Restart", "Full screen", "Exit"]),
    ("Controls", &[]),
    ("Music", &["On", "Off"]),
    ("About", &[]),
];

#[derive(Debug, PartialEq)]
pub enum UiCmd {
    Restart,
    Exit,
    MusicOn,
    MusicOff,
    ReturnToMenu,
    Resume,
    ToggleFullscreen,
}

#[derive(Clone, Copy)]
pub enum ExitTarget {
    Menu,
    System,
}

impl ExitTarget {
    fn answer(self, yes: bool) -> Option<UiCmd> {
        match (self, yes) {
            (Self::Menu, true) => Some(UiCmd::ReturnToMenu),
            (Self::Menu, false) => Some(UiCmd::Resume),
            (Self::System, true) => Some(UiCmd::Exit),
            (Self::System, false) => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Listen {
    player: usize,
    action: usize,
    pad: bool,
}

pub struct Ui {
    scale: f32,
    menu_open: Option<usize>,
    dialog: bool,
    exit_dialog: Option<ExitTarget>,
    about_dialog: bool,
    font: Option<Font>,
    listening: Option<Listen>,
    bar: bool,
}

fn text_w(font: Option<&Font>, t: &str, fs: f32) -> f32 {
    measure_text(t, font, fs.max(1.0).round() as u16, 1.0).width
}

fn ui_text(font: Option<&Font>, text: impl AsRef<str>, x: f32, y: f32, fs: f32, color: Color) {
    draw_text_ex(
        text.as_ref(),
        x,
        y,
        TextParams {
            font,
            font_size: fs.max(1.0).round() as u16,
            color,
            ..Default::default()
        },
    );
}

fn draw_label(
    font: Option<&Font>,
    t: &str,
    x: f32,
    rect_y: f32,
    rect_h: f32,
    fs: f32,
    color: Color,
) {
    ui_text(font, t, x, rect_y + rect_h * 0.5 + fs * 0.32, fs, color);
}

/// Przycisk; zwraca true, jeśli został kliknięty.
fn button(
    font: Option<&Font>,
    r: Rect,
    label: &str,
    fs: f32,
    mouse: Vec2,
    clicked: bool,
    highlight: bool,
) -> bool {
    let hover = r.contains(mouse);
    let bg = if highlight {
        Color::new(0.75, 0.55, 0.1, 1.0)
    } else if hover {
        Color::new(0.33, 0.36, 0.45, 1.0)
    } else {
        Color::new(0.22, 0.24, 0.30, 1.0)
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.5, 0.52, 0.6, 1.0));
    let tw = text_w(font, label, fs);
    draw_label(font, label, r.x + (r.w - tw) / 2.0, r.y, r.h, fs, WHITE);
    hover && clicked
}

#[cfg(not(target_os = "android"))]
const SUPPORT_URL: &str = "https://buymeacoffee.com/adamstrad";
const SUPPORT_LABEL: &str = "buy me a coffee";
const SUPPORT_COLOR: Color = Color::new(1.0, 0.44, 0.68, 1.0);

fn support_width(font: Option<&Font>, size: f32) -> f32 {
    measure_text(SUPPORT_LABEL, font, size.max(1.0) as u16, 1.0).width + size * 1.15
}

fn support_link(
    rect: Rect,
    font: Option<&Font>,
    size: f32,
    mouse: Vec2,
    clicked: bool,
    color: Color,
) -> bool {
    // A drawn outline heart avoids requiring a font with U+2661.
    let center = vec2(rect.x + size * 0.45, rect.y + rect.h * 0.50);
    let point = |t: f32| {
        let x = 16.0 * t.sin().powi(3);
        let y = -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos());
        center + vec2(x, y) * (size / 38.0)
    };
    for i in 0..32 {
        let a = point(i as f32 * std::f32::consts::TAU / 32.0);
        let b = point((i + 1) as f32 * std::f32::consts::TAU / 32.0);
        draw_line(a.x, a.y, b.x, b.y, (size * 0.065).max(1.0), color);
    }
    draw_text_ex(
        SUPPORT_LABEL,
        rect.x + size * 1.15,
        rect.y + rect.h * 0.5 + size * 0.32,
        TextParams {
            font,
            font_size: size.max(1.0) as u16,
            color,
            ..Default::default()
        },
    );
    rect.contains(mouse) && clicked
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn pika_open_url(ptr: *const u8, len: u32);
}

fn open_support() {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        pika_open_url(SUPPORT_URL.as_ptr(), SUPPORT_URL.len() as u32);
    }
    #[cfg(target_os = "windows")]
    let result = {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", SUPPORT_URL])
            .creation_flags(0x08000000)
            .spawn()
    };
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(SUPPORT_URL).spawn();
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "android"))))]
    let result = std::process::Command::new("xdg-open")
        .env_remove("LD_LIBRARY_PATH")
        .arg(SUPPORT_URL)
        .spawn();
    #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
    if let Err(error) = result {
        eprintln!("Could not open support link: {error}");
    }
}

impl Ui {
    pub fn android_support(&self, click: Option<Vec2>, scale: f32, left: f32, top: f32) -> bool {
        let size = (11.0 * scale).clamp(10.0, 24.0);
        let width = support_width(self.font.as_ref(), size);
        let rect = Rect::new(
            left + crate::scene::WIDTH * scale - width - 8.0 * scale,
            top + crate::scene::HEIGHT * scale - 25.0 * scale,
            width,
            18.0 * scale,
        );
        support_link(
            rect,
            self.font.as_ref(),
            size,
            click.unwrap_or(vec2(-1.0, -1.0)),
            click.is_some(),
            WHITE,
        )
    }
    /// About dialog of the menu-less (browser) build: controls, author and the support link.
    fn web_about(&mut self, mouse: Vec2, clicked: bool, sw: f32, sh: f32) {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
        let scale = self
            .scale
            .min((sw - 20.0).max(1.0) / 540.0)
            .min((sh - 20.0).max(1.0) / 330.0);
        let (dw, dh) = (540.0 * scale, 330.0 * scale);
        let (dx, dy) = ((sw - dw) / 2.0, (sh - dh) / 2.0);
        draw_rectangle(dx, dy, dw, dh, Color::new(0.13, 0.14, 0.18, 1.0));
        draw_rectangle_lines(dx, dy, dw, dh, scale, Color::new(0.5, 0.52, 0.6, 1.0));
        let font = self.font.as_ref();
        let centered = |text: &str, y: f32, size: f32, color: Color| {
            let fs = size * scale;
            ui_text(font, text, dx + (dw - text_w(font, text, fs)) / 2.0, dy + y * scale, fs, color);
        };
        centered("Pikachu Volleyball Clone", 38.0, 22.0, WHITE);
        centered("Controls", 66.0, 16.0, Color::new(1.0, 0.85, 0.2, 1.0));
        for (i, line) in [
            "Player 1:  D / G move,  R jump,  F aim down,  Z hit or dive",
            "Player 2:  arrow keys move / jump / aim down,  Enter hit or dive",
            "P pause,  Esc menu,  F1 restart",
            "F11 or Alt+Enter: full screen",
            "Touch: drag to move and jump, tap the other half to hit or dive",
        ]
        .iter()
        .enumerate()
        {
            centered(line, 92.0 + 21.0 * i as f32, 14.0, WHITE);
        }
        centered("by Adam Mateusz Brożyński", 214.0, 17.0, WHITE);
        let link_size = 16.0 * scale;
        let link_width = support_width(font, link_size);
        let link = Rect::new(
            dx + (dw - link_width) / 2.0,
            dy + 226.0 * scale,
            link_width,
            24.0 * scale,
        );
        if support_link(link, font, link_size, mouse, clicked, SUPPORT_COLOR) {
            open_support();
        }
        let close = Rect::new(dx + (dw - 120.0 * scale) / 2.0, dy + 272.0 * scale, 120.0 * scale, 30.0 * scale);
        if button(font, close, "Close", 18.0 * scale, mouse, clicked, false)
            || is_key_pressed(KeyCode::Escape)
            || is_key_pressed(KeyCode::Enter)
        {
            self.about_dialog = false;
        }
    }

    pub fn new(scale: f32) -> Self {
        Self {
            scale,
            menu_open: None,
            dialog: false,
            exit_dialog: None,
            about_dialog: false,
            font: None,
            listening: None,
            bar: true,
        }
    }

    pub fn set_font(&mut self, font: Font) {
        self.font = Some(font);
    }

    /// The browser build has no menu bar (title-screen About entry instead).
    pub fn set_bar(&mut self, visible: bool) {
        self.bar = visible;
    }

    pub fn menu_height(&self, fullscreen: bool) -> f32 {
        if fullscreen || !self.bar {
            0.0
        } else {
            32.0 * self.scale
        }
    }

    /// Czy okno dialogowe blokuje grę.
    pub fn modal(&self) -> bool {
        self.dialog || self.exit_dialog.is_some() || self.about_dialog
    }

    pub fn confirm_exit(&mut self, target: ExitTarget) {
        self.menu_open = None;
        self.exit_dialog = Some(target);
    }

    /// Obsługa i rysowanie menu / okna. Wołać po narysowaniu gry (domyślna kamera).
    pub fn frame(
        &mut self,
        music_on: bool,
        fullscreen: bool,
        in_game_menu: bool,
        controls: &mut Controls,
        pads: &Pads,
    ) -> Option<UiCmd> {
        let s = self.scale;
        let menu_fs = 15.0 * s;
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        let sw = screen_width();
        let sh = screen_height();
        let bar_h = self.menu_height(false);
        if in_game_menu {
            let menu_h = self.menu_height(fullscreen);
            let fit = (sw / crate::scene::WIDTH).min((sh - menu_h).max(1.0) / crate::scene::HEIGHT);
            let right = (sw + crate::scene::WIDTH * fit) * 0.5;
            let bottom = menu_h + ((sh - menu_h) + crate::scene::HEIGHT * fit) * 0.5;
            let size = (11.0 * fit).clamp(10.0, 24.0);
            let label_width = support_width(self.font.as_ref(), size);
            let rect = Rect::new(
                right - label_width - 8.0 * fit,
                bottom - 25.0 * fit,
                label_width,
                18.0 * fit,
            );
            if support_link(
                rect,
                self.font.as_ref(),
                size,
                mouse,
                clicked && !self.modal() && self.menu_open.is_none(),
                WHITE,
            ) {
                open_support();
            }
            if !self.bar {
                let label = "About / controls";
                let left = (sw - crate::scene::WIDTH * fit) * 0.5;
                let lw = text_w(self.font.as_ref(), label, size);
                let rect = Rect::new(left + 8.0 * fit, bottom - 25.0 * fit, lw, 18.0 * fit);
                let hover = rect.contains(mouse);
                draw_label(
                    self.font.as_ref(),
                    label,
                    rect.x,
                    rect.y,
                    rect.h,
                    size,
                    if hover { Color::new(1.0, 0.85, 0.2, 1.0) } else { WHITE },
                );
                draw_line(rect.x, rect.y + rect.h * 0.9, rect.x + lw, rect.y + rect.h * 0.9, 1.0, WHITE);
                if hover && clicked && !self.modal() {
                    self.about_dialog = true;
                }
            }
        }
        // W pełnym ekranie pasek nakłada się na grę tylko przy górnej krawędzi.
        if fullscreen && self.menu_open.is_none() && !self.modal() && my > bar_h {
            return None;
        }
        let row_h = 30.0 * s;
        let pad = 10.0 * s;
        let mut cmd = None;

        // --- układ tytułów menu ---
        let mut title_rects: Vec<Rect> = Vec::new();
        let mut x = 4.0 * s;
        for (t, _) in MENUS.iter() {
            let w = text_w(self.font.as_ref(), t, menu_fs) + 2.0 * pad;
            title_rects.push(Rect::new(x, 0.0, w, bar_h));
            x += w;
        }
        let item_rect = |m: usize, i: usize| -> Rect {
            let mut w = 120.0 * s;
            for it in MENUS[m].1.iter() {
                w = w.max(text_w(self.font.as_ref(), it, menu_fs) + 3.0 * pad + 8.0 * s);
            }
            Rect::new(title_rects[m].x, bar_h + row_h * i as f32, w, row_h)
        };

        // --- logika menu (tylko gdy nie ma okna dialogowego) ---
        if self.bar && !self.modal() {
            let over_title = title_rects.iter().position(|r| r.contains(mouse));
            let was_open = self.menu_open;
            if was_open.is_some() {
                if let Some(t) = over_title {
                    if !MENUS[t].1.is_empty() {
                        self.menu_open = Some(t);
                    }
                }
            }
            if clicked {
                if let Some(t) = over_title {
                    if t == 1 {
                        self.menu_open = None;
                        self.dialog = true;
                        self.listening = None;
                    } else if t == 3 {
                        self.menu_open = None;
                        self.about_dialog = true;
                    } else {
                        self.menu_open = if was_open == Some(t) { None } else { Some(t) };
                    }
                } else if let Some(m) = was_open {
                    let mut hit = None;
                    for i in 0..MENUS[m].1.len() {
                        if item_rect(m, i).contains(mouse) {
                            hit = Some(i);
                        }
                    }
                    self.menu_open = None;
                    if let Some(i) = hit {
                        match (m, i) {
                            (0, 0) => cmd = Some(UiCmd::Restart),
                            (0, 1) => cmd = Some(UiCmd::ToggleFullscreen),
                            (0, 2) => cmd = Some(UiCmd::Exit),
                            (2, 0) => cmd = Some(UiCmd::MusicOn),
                            (2, 1) => cmd = Some(UiCmd::MusicOff),
                            _ => {}
                        }
                    }
                }
            }
        }

        if self.bar {
            // --- rysowanie paska menu ---
            let bar_bg = Color::new(0.14, 0.15, 0.19, 1.0);
            draw_rectangle(0.0, 0.0, sw, bar_h, bar_bg);
            draw_line(0.0, bar_h, sw, bar_h, 1.0, Color::new(0.4, 0.42, 0.5, 1.0));
            for (i, (t, _)) in MENUS.iter().enumerate() {
                let r = title_rects[i];
                let active = self.menu_open == Some(i);
                if active || (self.menu_open.is_none() && !self.modal() && r.contains(mouse)) {
                    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.30, 0.33, 0.42, 1.0));
                }
                draw_label(self.font.as_ref(), t, r.x + pad, r.y, r.h, menu_fs, WHITE);
            }
            if let Some(m) = self.menu_open {
                let n = MENUS[m].1.len();
                let first = item_rect(m, 0);
                draw_rectangle(
                    first.x,
                    first.y,
                    first.w,
                    row_h * n as f32,
                    Color::new(0.18, 0.19, 0.25, 1.0),
                );
                draw_rectangle_lines(
                    first.x,
                    first.y,
                    first.w,
                    row_h * n as f32,
                    1.0,
                    Color::new(0.4, 0.42, 0.5, 1.0),
                );
                for (i, it) in MENUS[m].1.iter().enumerate() {
                    let r = item_rect(m, i);
                    if r.contains(mouse) {
                        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.30, 0.33, 0.42, 1.0));
                    }
                    draw_label(
                        self.font.as_ref(),
                        it,
                        r.x + pad + 8.0 * s,
                        r.y,
                        r.h,
                        menu_fs,
                        WHITE,
                    );
                    // znacznik aktualnego wyboru w menu Music
                    let selected = (m == 0 && i == 1 && fullscreen)
                        || (m == 2 && i == 0 && music_on)
                        || (m == 2 && i == 1 && !music_on);
                    if selected {
                        draw_circle(
                            r.x + pad * 0.9,
                            r.y + r.h / 2.0,
                            3.0 * s,
                            Color::new(1.0, 0.85, 0.2, 1.0),
                        );
                    }
                }
            }

        }

        let fs = 20.0 * s;
        if self.dialog {
            self.controls_dialog(controls, pads, mouse, clicked, sw, sh);
        }
        if self.about_dialog && !self.bar {
            self.web_about(mouse, clicked, sw, sh);
        }
        if self.about_dialog && self.bar {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
            let scale = s
                .min((sw - 20.0).max(1.0) / 360.0)
                .min((sh - 20.0).max(1.0) / 170.0);
            let (dw, dh) = (360.0 * scale, 170.0 * scale);
            let (dx, dy) = ((sw - dw) / 2.0, (sh - dh) / 2.0);
            draw_rectangle(dx, dy, dw, dh, Color::new(0.13, 0.14, 0.18, 1.0));
            draw_rectangle_lines(dx, dy, dw, dh, scale, Color::new(0.5, 0.52, 0.6, 1.0));
            for (text, y, size) in [
                ("Pikachu Volleyball Clone", 46.0, 22.0),
                ("by Adam Mateusz Brożyński", 76.0, 17.0),
            ] {
                let font_size = size * scale;
                draw_text_ex(
                    text,
                    dx + (dw - measure_text(text, self.font.as_ref(), font_size as u16, 1.0).width)
                        / 2.0,
                    dy + y * scale,
                    TextParams {
                        font: self.font.as_ref(),
                        font_size: font_size.max(1.0) as u16,
                        color: WHITE,
                        ..Default::default()
                    },
                );
            }
            let link_size = 16.0 * scale;
            let link_width = support_width(self.font.as_ref(), link_size);
            let link = Rect::new(
                dx + (dw - link_width) / 2.0,
                dy + 88.0 * scale,
                link_width,
                24.0 * scale,
            );
            if support_link(
                link,
                self.font.as_ref(),
                link_size,
                mouse,
                clicked,
                SUPPORT_COLOR,
            ) {
                open_support();
            }
            let close = Rect::new(
                dx + (dw - 120.0 * scale) / 2.0,
                dy + 120.0 * scale,
                120.0 * scale,
                30.0 * scale,
            );
            if button(
                self.font.as_ref(),
                close,
                "Close",
                18.0 * scale,
                mouse,
                clicked,
                false,
            ) || is_key_pressed(KeyCode::Escape)
                || is_key_pressed(KeyCode::Enter)
            {
                self.about_dialog = false;
            }
        }
        if let Some(target) = self.exit_dialog {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
            let dw = (360.0 * s).min((sw - 20.0).max(1.0));
            let dh = 140.0 * s;
            let dx = (sw - dw) / 2.0;
            let dy = (sh - dh) / 2.0;
            draw_rectangle(dx, dy, dw, dh, Color::new(0.13, 0.14, 0.18, 1.0));
            let title = match target {
                ExitTarget::Menu => "Return to menu?",
                ExitTarget::System => "Exit to desktop?",
            };
            draw_label(
                self.font.as_ref(),
                title,
                dx + (dw - text_w(self.font.as_ref(), title, fs)) / 2.0,
                dy,
                dh * 0.5,
                fs,
                WHITE,
            );
            let bw = (dw - 3.0 * pad) / 2.0;
            let yes = Rect::new(dx + pad, dy + dh * 0.6, bw, 32.0 * s);
            let no = Rect::new(dx + 2.0 * pad + bw, yes.y, bw, yes.h);
            let yes_clicked = button(self.font.as_ref(), yes, "Yes", fs, mouse, clicked, false);
            let no_clicked = button(self.font.as_ref(), no, "No", fs, mouse, clicked, false);
            if yes_clicked || is_key_pressed(KeyCode::Y) {
                self.exit_dialog = None;
                cmd = target.answer(true);
            } else if no_clicked || is_key_pressed(KeyCode::N) {
                self.exit_dialog = None;
                cmd = target.answer(false);
            }
        }
        cmd
    }

    fn controls_dialog(
        &mut self,
        controls: &mut Controls,
        pads: &Pads,
        mouse: Vec2,
        clicked: bool,
        sw: f32,
        sh: f32,
    ) {
        let s = controls_dialog_scale(self.scale, sw, sh);
        let fs = 20.0 * s;
        let small = 17.0 * s;

        // ---- przypisywanie (nasłuch klawisza / przycisku pada) ----
        let last_key = get_last_key_pressed();
        if let Some(l) = self.listening {
            if is_key_pressed(KeyCode::Escape) {
                self.listening = None;
            } else if last_key == Some(KeyCode::Backspace) {
                let bd = &mut controls.players[l.player].actions[l.action];
                if l.pad {
                    bd.pad = None;
                } else {
                    bd.key = None;
                }
                self.listening = None;
            } else if l.pad {
                if let Some(p) = pads.pressed() {
                    controls.players[l.player].actions[l.action].pad = Some(p);
                    self.listening = None;
                }
            } else if let Some(k) = last_key {
                if key_name(k).is_some() {
                    controls.players[l.player].actions[l.action].key = Some(k);
                    self.listening = None;
                }
            }
        } else if is_key_pressed(KeyCode::Escape) {
            controls.save();
            self.dialog = false;
            return;
        }

        // ---- tło i ramka ----
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
        let dw = CONTROLS_DIALOG_WIDTH * s;
        let dh = CONTROLS_DIALOG_HEIGHT * s;
        let dx = (sw - dw) / 2.0;
        let dy = (sh - dh) / 2.0;
        draw_rectangle(dx, dy, dw, dh, Color::new(0.13, 0.14, 0.18, 1.0));
        draw_rectangle_lines(dx, dy, dw, dh, 2.0, Color::new(0.5, 0.52, 0.6, 1.0));
        ui_text(
            self.font.as_ref(),
            "Controls",
            dx + 16.0 * s,
            dy + 28.0 * s,
            fs * 1.2,
            WHITE,
        );

        // klik w trakcie nasłuchu anuluje go; w przeciwnym razie obsługujemy przyciski
        let click_ok = clicked && self.listening.is_none();
        if clicked && self.listening.is_some() {
            self.listening = None;
        }

        let margin = 16.0 * s;
        let col_w = (dw - 3.0 * margin) / 2.0;
        let label_w = 136.0 * s;
        let btn_w = (col_w - label_w - 6.0 * s) / 2.0;
        let row_h = 32.0 * s;
        let top = dy + 46.0 * s;

        for p in 0..2usize {
            let cx = dx + margin + (col_w + margin) * p as f32;

            // nagłówek gracza + wybór pada
            draw_label(
                self.font.as_ref(),
                &format!("Player {}", p + 1),
                cx,
                top,
                28.0 * s,
                fs,
                Color::new(1.0, 0.85, 0.2, 1.0),
            );
            let pad_label = match controls.players[p].pad_index {
                Some(i) => format!("Gamepad: #{}", i + 1),
                None => "Gamepad: off".to_string(),
            };
            let pr = Rect::new(cx + col_w - 150.0 * s, top, 150.0 * s, 28.0 * s);
            if button(
                self.font.as_ref(),
                pr,
                &pad_label,
                small,
                mouse,
                click_ok,
                false,
            ) {
                controls.players[p].pad_index = match controls.players[p].pad_index {
                    None => Some(0),
                    Some(3) => None,
                    Some(n) => Some(n + 1),
                };
            }

            // nagłówki kolumn
            let cap_y = top + 36.0 * s;
            ui_text(
                self.font.as_ref(),
                "Keyboard",
                cx + label_w + 4.0 * s,
                cap_y + 12.0 * s,
                small,
                SECONDARY_TEXT,
            );
            ui_text(
                self.font.as_ref(),
                "Gamepad",
                cx + label_w + btn_w + 10.0 * s,
                cap_y + 12.0 * s,
                small,
                SECONDARY_TEXT,
            );

            for (a, label) in ACTION_LABELS.iter().enumerate() {
                let ry = cap_y + 18.0 * s + row_h * a as f32;
                draw_label(
                    self.font.as_ref(),
                    label,
                    cx,
                    ry,
                    row_h - 4.0 * s,
                    small,
                    WHITE,
                );

                let bd = controls.players[p].actions[a];
                let listening_key = self.listening
                    == Some(Listen {
                        player: p,
                        action: a,
                        pad: false,
                    });
                let listening_pad = self.listening
                    == Some(Listen {
                        player: p,
                        action: a,
                        pad: true,
                    });

                let key_label = if listening_key {
                    "press key...".to_string()
                } else {
                    bd.key.and_then(key_name).unwrap_or("-").to_string()
                };
                let kr = Rect::new(cx + label_w, ry, btn_w, row_h - 4.0 * s);
                if button(
                    self.font.as_ref(),
                    kr,
                    &key_label,
                    small,
                    mouse,
                    click_ok,
                    listening_key,
                ) {
                    self.listening = Some(Listen {
                        player: p,
                        action: a,
                        pad: false,
                    });
                }

                let pad_btn_label = if listening_pad {
                    "press button...".to_string()
                } else {
                    bd.pad.map(PadBtn::label).unwrap_or("-").to_string()
                };
                let br = Rect::new(cx + label_w + btn_w + 6.0 * s, ry, btn_w, row_h - 4.0 * s);
                if button(
                    self.font.as_ref(),
                    br,
                    &pad_btn_label,
                    small,
                    mouse,
                    click_ok,
                    listening_pad,
                ) {
                    self.listening = Some(Listen {
                        player: p,
                        action: a,
                        pad: true,
                    });
                }
            }
        }

        // Global shortcuts are fixed; player bindings above can be changed.
        let shortcuts_y = dy + 283.0 * s;
        ui_text(
            self.font.as_ref(),
            "Keyboard shortcuts",
            dx + margin,
            shortcuts_y,
            fs,
            WHITE,
        );
        let shortcuts = [
            [
                "P  Pause / resume",
                "Esc  Exit / return to menu",
                "F1  Restart",
            ],
            ["Alt+Enter / F11  Full screen", "", ""],
        ];
        for (column, entries) in shortcuts.iter().enumerate() {
            for (row, entry) in entries.iter().enumerate() {
                ui_text(
                    self.font.as_ref(),
                    *entry,
                    dx + margin + column as f32 * (col_w + margin),
                    shortcuts_y + (22.0 + CONTROLS_LINE_HEIGHT * row as f32) * s,
                    small,
                    SECONDARY_TEXT,
                );
            }
        }

        ui_text(
            self.font.as_ref(),
            "Touch controls (invisible; enabled by default)",
            dx + margin,
            dy + 385.0 * s,
            fs,
            WHITE,
        );
        for (row, line) in [
            "Drag on the left half to move; drag up to jump.",
            "Tap the right half: power hit in the air, dive while moving on the ground.",
            "Android: Lefthanded mode mirrors the controls and player side.",
            "Keyboard and gamepad bindings stay active alongside touch controls.",
        ]
        .iter()
        .enumerate()
        {
            ui_text(
                self.font.as_ref(),
                *line,
                dx + margin,
                dy + (411.0 + row as f32 * CONTROLS_LINE_HEIGHT) * s,
                small,
                SECONDARY_TEXT,
            );
        }

        // ---- stopka ----
        let foot_y = dy + dh - 64.0 * s;
        let hint =
            "Click a button, then press a key / gamepad button.  Backspace = clear,  Esc = cancel.";
        ui_text(self.font.as_ref(), hint, dx + margin, foot_y, small, GRAY);
        ui_text(
            self.font.as_ref(),
            format!("Connected gamepads: {}", pads.count()),
            dx + margin,
            foot_y + CONTROLS_LINE_HEIGHT * s,
            small,
            SECONDARY_TEXT,
        );

        let bw = 150.0 * s;
        let bh = 30.0 * s;
        let by = dy + dh - bh - 12.0 * s;
        let reset = Rect::new(dx + dw - 2.0 * bw - 2.0 * margin, by, bw, bh);
        let close = Rect::new(dx + dw - bw - margin, by, bw, bh);
        if button(
            self.font.as_ref(),
            reset,
            "Reset defaults",
            small,
            mouse,
            click_ok,
            false,
        ) {
            *controls = Controls::default();
        }
        if button(
            self.font.as_ref(),
            close,
            "Close",
            small,
            mouse,
            click_ok,
            false,
        ) {
            controls.save();
            self.dialog = false;
            self.listening = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_menu_overlays_the_scene_instead_of_reserving_height() {
        let ui = Ui::new(1.0);
        assert_eq!(ui.menu_height(false), 32.0);
        assert_eq!(ui.menu_height(true), 0.0);
        assert_eq!(MENUS[0].1, ["Restart", "Full screen", "Exit"]);
        assert_eq!(MENUS[3].0, "About");
    }

    #[test]
    fn about_dialog_is_modal() {
        let mut ui = Ui::new(1.0);
        assert!(!ui.modal());
        ui.about_dialog = true;
        assert!(ui.modal());
        ui.about_dialog = false;
        assert!(!ui.modal());
    }

    #[test]
    fn exit_confirmation_distinguishes_system_exit_and_match_resume() {
        assert_eq!(ExitTarget::System.answer(true), Some(UiCmd::Exit));
        assert_eq!(ExitTarget::System.answer(false), None);
        assert_eq!(ExitTarget::Menu.answer(true), Some(UiCmd::ReturnToMenu));
        assert_eq!(ExitTarget::Menu.answer(false), Some(UiCmd::Resume));
    }
}

/// Scale every control together, rather than clipping only the dialog frame.
fn controls_dialog_scale(preferred: f32, width: f32, height: f32) -> f32 {
    preferred
        .min((width - 20.0).max(1.0) / CONTROLS_DIALOG_WIDTH)
        .min((height - 20.0).max(1.0) / CONTROLS_DIALOG_HEIGHT)
}

const CONTROLS_DIALOG_WIDTH: f32 = 880.0;
const CONTROLS_DIALOG_HEIGHT: f32 = 580.0;
const CONTROLS_LINE_HEIGHT: f32 = 24.0;

#[cfg(test)]
mod controls_layout_tests {
    use super::*;

    #[test]
    fn controls_fit_small_and_high_dpi_windows() {
        for (width, height, preferred) in [
            (544.0, 306.0, 1.0),
            (1088.0, 612.0, 2.0),
            (320.0, 240.0, 2.0),
            (1920.0, 1080.0, 1.0),
        ] {
            let s = controls_dialog_scale(preferred, width, height);
            assert!(s > 0.0 && s <= preferred);
            assert!(CONTROLS_DIALOG_WIDTH * s <= width - 19.99);
            assert!(CONTROLS_DIALOG_HEIGHT * s <= height - 19.99);
            let last_row_bottom =
                (46.0 + 36.0 + 18.0 + (ACTION_LABELS.len() - 1) as f32 * 32.0 + 28.0) * s;
            let shortcuts_top = (283.0 - 18.0) * s;
            let shortcuts_bottom = (283.0 + 22.0 + 2.0 * CONTROLS_LINE_HEIGHT) * s;
            let footer_top = (CONTROLS_DIALOG_HEIGHT - 64.0 - 15.0) * s;
            assert!(last_row_bottom < shortcuts_top);
            assert!(shortcuts_bottom < (385.0 - 20.0) * s);
            assert!((411.0 + 3.0 * CONTROLS_LINE_HEIGHT) * s < footer_top);
        }
    }
}
