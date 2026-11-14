//! Pikachu Volleyball – port gry z JavaScript (gorisanson/pikachu-volleyball) do Rusta.
//!
//! Oryginalna gra: (C) SACHI SOFT / SAWAYAKAN Programmers, (C) Satoshi Takenouchi, 1997.
//!
//! Menu u góry okna: Game (Restart, Full screen, Exit), Controls, Music, About.
//! Domyślne sterowanie:
//!   Gracz 1: D (lewo), G (prawo), R (góra), F (dół), Z (uderzenie / wybór)
//!   Gracz 2: strzałki, Enter (uderzenie / wybór)
//!   Pad: D-pad + przycisk A (przypisanie pada do gracza w Controls)
//! Skróty: P – pauza, F1 – restart, Backspace – skórki graczy,
//!   Alt+Enter / F11 – pełny ekran

mod audio;
mod background;
mod ball_effects;
mod cloud_render;
mod clouds;
mod gfx;
mod grass;
mod input;
mod keyboard;
mod menu_badge;
mod pad;
mod physics;
mod pikavolley;
mod rand;
mod rendering;
mod scene;
mod texture;
mod text_label;
mod touch;
mod ui;
mod view;
mod water;

use macroquad::prelude::*;

/// Logiczna rozdzielczość gry (współrzędne i fizyka); grafika HD jest rysowana do bufora
/// dopasowanego do fizycznego rozmiaru okna; współrzędne symulacji zostają stałe.
use crate::scene::{HEIGHT as SCREEN_H, WIDTH as SCREEN_W};
/// Wysokość paska menu w oknie desktopowym (musi się zgadzać z `Ui::menu_height`).
const MENU_H_DESKTOP: i32 = 32;

fn window_conf() -> Conf {
    Conf {
        window_title: "Pikachu Volleyball Clone".to_owned(),
        icon: Some(window_icon()),
        window_width: (SCREEN_W as i32) * 2,
        window_height: (SCREEN_H as i32) * 2 + MENU_H_DESKTOP,
        window_resizable: true,
        high_dpi: true,
        fullscreen: cfg!(target_os = "android"),
        sample_count: 0,
        platform: macroquad::miniquad::conf::Platform {
            // GLX can terminate XWayland with GLXBadFBConfig before fallback.
            linux_x11_gl: macroquad::miniquad::conf::LinuxX11Gl::EGLOnly,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Three pre-rendered player icons arranged horizontally in a single PNG.
fn window_icon() -> macroquad::miniquad::conf::Icon {
    let image = Image::from_file_with_format(include_bytes!("../assets/images/icon.png"), None)
        .expect("embedded window icon");
    assert_eq!((image.width, image.height), (112, 64));
    fn extract<const N: usize>(image: &Image, x: usize, size: usize) -> [u8; N] {
        let mut pixels = [0; N];
        for y in 0..size {
            let start = (y * image.width as usize + x) * 4;
            pixels[y * size * 4..(y + 1) * size * 4]
                .copy_from_slice(&image.bytes[start..start + size * 4]);
        }
        pixels
    }
    macroquad::miniquad::conf::Icon {
        small: extract(&image, 0, 16),
        medium: extract(&image, 16, 32),
        big: extract(&image, 48, 64),
    }
}

fn toggle_fullscreen(fullscreen: &mut bool) {
    *fullscreen = !*fullscreen;
    set_fullscreen(*fullscreen);
}

#[cfg(target_os = "android")]
fn android_call(method: &str) {
    unsafe {
        let raw = macroquad::miniquad::native::android::attach_jni_env();
        if let Ok(mut env) = jni::JNIEnv::from_raw(raw.cast()) {
            let activity = jni::objects::JObject::from_raw(
                macroquad::miniquad::native::android::ACTIVITY.cast(),
            );
            let _ = env.call_method(activity, method, "()V", &[]);
        }
    }
}

/// Compute where the game scene rectangle goes in the window/screen and the fitting scale.
/// Returns (scene_x, scene_y, scene_w, scene_h, scale).
fn scene_placement(screen_w: f32, screen_h: f32, menu_h: f32, portrait: bool) -> (f32, f32, f32, f32, f32) {
    if portrait {
        // Portrait: game fills the width, sits at the top.
        let scale = screen_w / SCREEN_W;
        let w = screen_w;
        let h = SCREEN_H * scale;
        // Cap game scene to at most 55% of screen height so controls have room.
        let max_h = screen_h * 0.55;
        if h > max_h {
            let capped_scale = max_h / SCREEN_H;
            let cw = SCREEN_W * capped_scale;
            (
                (screen_w - cw) * 0.5,
                0.0,
                cw,
                max_h,
                capped_scale,
            )
        } else {
            (0.0, 0.0, w, h, scale)
        }
    } else {
        // Landscape / desktop: centered with aspect ratio preserved.
        let avail_h = (screen_h - menu_h).max(1.0);
        let scale = f32::min(screen_w / SCREEN_W, avail_h / SCREEN_H);
        let w = SCREEN_W * scale;
        let h = SCREEN_H * scale;
        (
            (screen_w - w) * 0.5,
            menu_h + (avail_h - h) * 0.5,
            w,
            h,
            scale,
        )
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    #[cfg(target_os = "android")]
    unsafe {
        let raw = macroquad::miniquad::native::android::attach_jni_env();
        let env = jni::JNIEnv::from_raw(raw.cast()).expect("Android JNI environment");
        let vm = env.get_java_vm().expect("Android Java VM");
        let activity = macroquad::miniquad::native::android::ACTIVITY;
        ndk_context::initialize_android_context(vm.get_java_vm_pointer().cast(), activity.cast());
    }
    let on_android = cfg!(target_os = "android");
    let web = cfg!(target_arch = "wasm32");
    let presentation = rendering::opaque_presentation();
    crate::texture::init_device_profile();

    let assets = gfx::Assets::load();
    let mut game = pikavolley::PikachuVolleyball::new(&assets);
    let mut controls = if on_android {
        input::Controls::default()
    } else {
        input::Controls::load()
    };
    let mut pads = pad::Pads::new();
    let mut touch = touch::TouchControls::new();
    let mut ui = ui::Ui::new(if on_android { 2.0 } else { 1.0 });
    ui.set_bar(!web);
    ui.set_font(
        load_ttf_font_from_bytes(include_bytes!("../assets/fonts/NotoSans-Regular.ttf"))
            .expect("Noto Sans"),
    );

    // Minimalne supersampling 3×; większy bufor dobieramy do okna i DPI.
    // Na starszych urządzeniach mobilnych używamy stałego 2× (1088×612), co daje
    // idealną ostrość na ekranach qHD (960×540) i 720p bez nadmiarowego 3× (1632×918).
    let low_end_mobile = on_android && crate::texture::is_low_end_device();
    let compute_target_size = |scale: f32, dpi: f32| -> (u32, u32) {
        let s = if low_end_mobile {
            2
        } else {
            rendering::scene_scale(scale, dpi)
        };
        (SCREEN_W as u32 * s, SCREEN_H as u32 * s)
    };
    let mut target_size = compute_target_size(0.0, 1.0);
    let mut target = render_target(target_size.0, target_size.1);
    target.texture.set_filter(FilterMode::Linear);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, SCREEN_W, SCREEN_H));
    cam.render_target = Some(target.clone());

    // Symulacja zawsze w stałym kroku 25 Hz (jak w oryginale); render co klatkę monitora
    // (np. 60 FPS) z interpolacją pozycji – animacje klatkowe zostają w tempie oryginału.
    let step = 1.0 / game.normal_fps as f64;
    let mut acc = 0.0f64;
    let mut pending_hit = false;
    let mut fullscreen = on_android;

    'main: loop {
        // touches() are physical pixels, screen_width() logical (browser zoom can change this).
        let touch_dpi = if web { screen_dpi_scale() } else { 1.0 };
        pads.update();
        game.update_enter_release(is_key_down(KeyCode::Enter) || is_key_down(KeyCode::KpEnter));
        let mut modal = ui.modal();

        // Detect portrait orientation (Android only; desktop always landscape layout).
        let portrait = on_android && screen_height() > screen_width();

        // Compute portrait layout once per frame (only if portrait).
        let portrait_layout = if portrait {
            Some(touch::PortraitLayout::compute(screen_width(), screen_height(), touch.right_handed))
        } else {
            None
        };

        #[cfg(target_os = "android")]
        if pad::take_back() {
            if game.is_menu() {
                android_call("quitGame");
                break 'main;
            }
            game.restart();
        }

        if on_android && game.is_menu() {
            let menu_h = 0.0f32;
            let (sx, sy, _sw, sh, scale) = scene_placement(screen_width(), screen_height(), menu_h, portrait);
            // Wyłącznie kliknięcia w obrębie samej sceny gry (nie w panelu kontrolek poniżej).
            let android_menu_click = touches()
                .iter()
                .find(|t| t.phase == TouchPhase::Ended && t.position.y <= sy + sh)
                .map(|t| t.position);
            if let Some(point) = android_menu_click {
                let logical = (point - vec2(sx, sy)) / scale;
                // Przycisk wsparcia jest wyłącznie w prawym dolnym rogu sceny gry.
                if (410.0..=544.0).contains(&logical.x) && (270.0..=306.0).contains(&logical.y) {
                    #[cfg(target_os = "android")]
                    android_call("openSupport");
                } else if (200.0..=440.0).contains(&logical.x) {
                    if (153.0..183.0).contains(&logical.y) {
                        game.request_touch_start(false);
                    } else if (183.0..213.0).contains(&logical.y) {
                        let new_mode = !game.is_android_right_handed();
                        game.set_android_mode(new_mode);
                        touch.right_handed = new_mode;
                    } else if (213.0..243.0).contains(&logical.y) {
                        game.cycle_skins_mode();
                    } else if (243.0..275.0).contains(&logical.y) {
                        #[cfg(target_os = "android")]
                        android_call("quitGame");
                        break 'main;
                    }
                }
            }
        }
        if !on_android && game.is_menu() && !modal {
            let mut click_point = touches()
                .iter()
                .find(|t| t.phase == TouchPhase::Ended)
                .map(|t| t.position / touch_dpi);
            if click_point.is_none() && is_mouse_button_pressed(MouseButton::Left) {
                click_point = Some(mouse_position().into());
            }
            if let Some(point) = click_point {
                let menu_h = ui.menu_height(fullscreen);
                let available_h = screen_height() - menu_h;
                let scale = (screen_width() / SCREEN_W).min(available_h / SCREEN_H);
                let logical = (point
                    - vec2(
                        (screen_width() - SCREEN_W * scale) * 0.5,
                        menu_h + (available_h - SCREEN_H * scale) * 0.5,
                    ))
                    / scale;
                if (200.0..=440.0).contains(&logical.x) {
                    if (160.0..191.0).contains(&logical.y) {
                        game.request_touch_start(false);
                    } else if (191.0..223.0).contains(&logical.y) {
                        game.request_touch_start(true);
                    } else if (223.0..255.0).contains(&logical.y) {
                        game.cycle_skins_mode();
                    }
                }
            }
        }
        if on_android {
            if game.quit_requested {
                #[cfg(target_os = "android")]
                android_call("quitGame");
                break 'main;
            }
            touch.right_handed = game.is_android_right_handed();
            controls.players[0].pad_index = if touch.right_handed { None } else { Some(0) };
            controls.players[1].pad_index = if touch.right_handed { Some(0) } else { None };
        }

        // --- skróty klawiszowe (nieaktywne w oknie przypisywania klawiszy) ---
        if !modal && !on_android {
            if is_key_pressed(KeyCode::Escape) {
                if game.can_pause() {
                    game.set_paused(true);
                    ui.confirm_exit(ui::ExitTarget::Menu);
                    modal = true;
                } else if game.is_menu() && !web {
                    ui.confirm_exit(ui::ExitTarget::System);
                    modal = true;
                }
            }
            if !modal && is_key_pressed(KeyCode::P) {
                game.set_paused(!game.is_paused());
            }
            if !modal && is_key_pressed(KeyCode::F1) {
                game.restart();
            }
            let alt_enter = (is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt))
                && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter));
            if !modal && (is_key_pressed(KeyCode::F11) || alt_enter) {
                if alt_enter {
                    game.suppress_fullscreen_enter();
                }
                toggle_fullscreen(&mut fullscreen);
            }
        }

        // --- stały krok symulacji 25 Hz ---
        // W trybie portretowym kontrolki ekranowe są aktywne także w menu (D-pad nawiguje, A/B zatwierdza).
        let touch_blocked = (!portrait && game.is_menu()) || modal || game.is_paused() || (on_android && pads.count() > 0);
        let touch_state = if let Some(ref layout) = portrait_layout {
            touch.update_portrait(touch_blocked, layout, game.is_menu())
        } else {
            touch.update(touch_dpi, touch_blocked)
        };
        if game.is_menu() || modal || game.is_paused() {
            pending_hit = false;
        } else {
            pending_hit |= touch_state.action;
        }
        if modal || game.is_paused() {
            acc = 0.0;
        } else {
            acc += get_frame_time() as f64;
            if acc > 0.25 {
                acc = 0.25;
            }
            while acc >= step {
                let mut tick_touch = touch_state;
                tick_touch.action |= pending_hit;
                pending_hit = false;
                game.game_loop(&controls, &pads, &tick_touch);
                acc -= step;
            }
        }
        let t = if modal {
            1.0
        } else {
            game.render_t(acc / step)
        };

        let menu_h = if on_android {
            0.0
        } else {
            ui.menu_height(fullscreen)
        };
        let (scene_x, scene_y, w, h, scale) = scene_placement(screen_width(), screen_height(), menu_h, portrait);

        // Na starszych urządzeniach mobilnych dopasowujemy bufor dokładnie do fizycznych
        // pikseli ekranu (np. 960×540 qHD), co daje 1:1 ostrość bez nadmiarowego 3× supersamplingu.
        let next_size = if low_end_mobile {
            (w.round() as u32, h.round() as u32)
        } else {
            compute_target_size(scale, screen_dpi_scale())
        };
        if next_size != target_size {
            target_size = next_size;
            target = render_target(target_size.0, target_size.1);
            target.texture.set_filter(FilterMode::Linear);
            cam.render_target = Some(target.clone());
        }

        // --- rysowanie w logicznych współrzędnych do bufora o fizycznej rozdzielczości ---
        set_camera(&cam);
        clear_background(BLACK);
        game.draw(t, &touch_state, &pads);

        // --- bufor -> okno (pod paskiem menu, z zachowaniem proporcji) ---
        set_default_camera();
        clear_background(BLACK);
        gl_use_material(&presentation);
        draw_texture_ex(
            &target.texture,
            scene_x,
            scene_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                flip_y: true,
                ..Default::default()
            },
        );

        gl_use_default_material();

        // --- portrait on-screen controls (drawn below the game scene) ---
        if let Some(ref layout) = portrait_layout {
            layout.draw();
        }

        // --- menu i okna (na wierzchu); sterowanie dotykowe jest niewidoczne ---
        if on_android && game.is_menu() {
            ui.android_support(None, scale, scene_x, scene_y);
        }
        let music_on = game.audio_mut().is_bgm_on();
        match if on_android {
            None
        } else {
            ui.frame(music_on, fullscreen, game.is_menu(), &mut controls, &pads)
        } {
            Some(ui::UiCmd::Restart | ui::UiCmd::ReturnToMenu) => game.restart(),
            Some(ui::UiCmd::Resume) => game.set_paused(false),
            Some(ui::UiCmd::Exit) => break 'main,
            Some(ui::UiCmd::MusicOn) => game.audio_mut().turn_bgm_volume(true),
            Some(ui::UiCmd::MusicOff) => game.audio_mut().turn_bgm_volume(false),
            Some(ui::UiCmd::ToggleFullscreen) => toggle_fullscreen(&mut fullscreen),
            None => {}
        }

        if on_android {
            let fps = get_fps();
            let fps_str = format!("{fps} FPS");
            let fps_y = screen_height() - 8.0;
            draw_rectangle(6.0, fps_y - 14.0, 64.0, 18.0, Color::new(0.0, 0.0, 0.0, 0.65));
            draw_text(&fps_str, 10.0, fps_y, 16.0, WHITE);
        }

        next_frame().await;
    }
}

#[cfg(test)]
mod window_tests {
    #[test]
    fn window_icon_has_visible_player_at_every_size() {
        let icon = super::window_icon();
        for pixels in [&icon.small[..], &icon.medium[..], &icon.big[..]] {
            assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0));
            assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 0));
        }
    }
}
