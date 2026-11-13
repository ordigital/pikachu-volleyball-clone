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
    let mut target_scale = rendering::scene_scale(0.0, 1.0);
    let mut target = render_target(
        SCREEN_W as u32 * target_scale,
        SCREEN_H as u32 * target_scale,
    );
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

        #[cfg(target_os = "android")]
        if pad::take_back() {
            if game.is_menu() {
                android_call("quitGame");
                break 'main;
            }
            game.restart();
        }

        if on_android && game.is_menu() {
            let android_menu_click = touches()
                .iter()
                .find(|t| t.phase == TouchPhase::Ended)
                .map(|t| t.position);
            if let Some(point) = android_menu_click {
                let scale = (screen_width() / SCREEN_W).min(screen_height() / SCREEN_H);
                let logical = (point
                    - vec2(
                        (screen_width() - SCREEN_W * scale) * 0.5,
                        (screen_height() - SCREEN_H * scale) * 0.5,
                    ))
                    / scale;
                if logical.x >= 410.0 && logical.y >= 270.0 {
                    #[cfg(target_os = "android")]
                    android_call("openSupport");
                } else if (200.0..=440.0).contains(&logical.x) {
                    if (153.0..183.0).contains(&logical.y) {
                        game.request_touch_start(false);
                    } else if (183.0..213.0).contains(&logical.y) {
                        touch.right_handed = !touch.right_handed;
                        game.set_android_mode(touch.right_handed);
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
        let touch_state = touch.update(touch_dpi,
            game.is_menu() || modal || game.is_paused() || (on_android && pads.count() > 0),
        );
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
        let avail_h = (screen_height() - menu_h).max(1.0);
        let scale = f32::min(screen_width() / SCREEN_W, avail_h / SCREEN_H);
        let next_scale = rendering::scene_scale(scale, screen_dpi_scale());
        if next_scale != target_scale {
            target_scale = next_scale;
            target = render_target(
                SCREEN_W as u32 * target_scale,
                SCREEN_H as u32 * target_scale,
            );
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
        let w = SCREEN_W * scale;
        let h = SCREEN_H * scale;
        gl_use_material(&presentation);
        draw_texture_ex(
            &target.texture,
            (screen_width() - w) / 2.0,
            menu_h + (avail_h - h) / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                flip_y: true,
                ..Default::default()
            },
        );

        gl_use_default_material();

        // --- menu i okna (na wierzchu); sterowanie dotykowe jest niewidoczne ---
        if on_android && game.is_menu() {
            let left = (screen_width() - w) / 2.0;
            let top = (screen_height() - h) / 2.0;
            ui.android_support(None, scale, left, top);
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
