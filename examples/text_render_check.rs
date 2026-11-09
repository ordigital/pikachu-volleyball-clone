//! GPU regression for font-atlas growth: cargo run --example text_render_check.
//! Uses the actual game label renderer; exits with failure if any mask is blank.
use macroquad::prelude::*;

// Only the style type is needed from gfx. Keep the harness free of game assets/AI.
mod gfx {
    #[derive(Clone, Copy)]
    pub struct TextStyle {
        pub color: macroquad::prelude::Color,
        pub outline: f32,
        pub rotation: f32,
    }
}
#[allow(dead_code)]
#[path = "../src/rendering.rs"]
mod rendering;
#[allow(dead_code)]
#[path = "../src/scene.rs"]
mod scene;
#[path = "../src/text_label.rs"]
mod text_label;

fn window_conf() -> Conf {
    Conf {
        window_title: "Font atlas regression".into(),
        platform: macroquad::miniquad::conf::Platform {
            linux_x11_gl: macroquad::miniquad::conf::LinuxX11Gl::EGLOnly,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Jersey10-Regular.ttf"))
        .expect("Jersey 10");
    let material = text_label::outline_material();
    // Adding these labels grows the atlas while its old texture is still bound.
    let texts = [
        "1 PLAYER",
        "2 PLAYERS",
        "START GAME",
        "MODE: RIGHTHANDED",
        "MODE: LEFTHANDED",
        "QUIT",
        "GAME OVER",
        "READY?",
        "START!",
        "0",
        "1",
        "9",
    ];
    let labels: Vec<_> = texts
        .iter()
        .map(|text| text_label::TextLabel::new(text, &font, &material))
        .collect();
    let target = render_target(1632, 918);
    let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 544., 306.));
    camera.render_target = Some(target.clone());
    for size in [vec2(120., 20.), vec2(240., 40.), vec2(480., 80.)] {
        for (text, label) in texts.iter().zip(&labels) {
            set_camera(&camera);
            clear_background(BLACK);
            label.draw(
                vec2(10., 10.),
                size,
                gfx::TextStyle {
                    color: WHITE,
                    outline: 0.0225,
                    rotation: 0.,
                },
                1.,
            );
            set_default_camera(); // Flush the label before reading its pixels.
            let pixels = target.texture.get_texture_data();
            let coverage = pixels
                .bytes
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[0] > 128)
                .count();
            assert!(
                coverage > 100,
                "blank text after atlas growth: {text}, size={size:?}"
            );
        }
    }
    println!(
        "PASS: all {} labels visible at three cache sizes after atlas growth",
        texts.len()
    );
    std::process::exit(0);
}
