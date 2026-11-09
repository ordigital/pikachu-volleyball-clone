//! Jawne mieszanie RGB/alfa: światło dodaje jasność, prezentacja nie przyciemnia bufora.
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

pub const VERTEX: &str = include_str!("../assets/shaders/scene.vert");

pub fn source_over() -> PipelineParams {
    PipelineParams {
        color_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::SourceAlpha),
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
        )),
        alpha_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::One,
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
        )),
        ..Default::default()
    }
}

pub fn additive_light() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX,
            fragment: include_str!("../assets/shaders/texture.frag"),
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::One,
                )),
                alpha_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Zero,
                    BlendFactor::One,
                )),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("additive light material")
}

pub fn opaque_presentation() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX,
            fragment: include_str!("../assets/shaders/present.frag"),
        },
        MaterialParams::default(),
    )
    .expect("opaque presentation material")
}

/// Całkowite progi zapobiegają tworzeniu nowego bufora przy każdym pikselu resize.
/// Co najmniej 3× zachowuje wygładzanie także w małym oknie.
pub fn scene_scale(window_scale: f32, dpi: f32) -> u32 {
    (window_scale * dpi).ceil().max(3.0) as u32
}

#[cfg(test)]
mod tests {
    use super::scene_scale;

    #[test]
    fn scene_buffer_covers_window_pixels_including_hidpi() {
        for (window, dpi) in [(2.0, 1.0), (3.46, 1.0), (2.0, 2.0), (4.1, 2.0)] {
            let scale = scene_scale(window, dpi);
            assert!(scale as f32 >= window * dpi);
            assert!(scale >= 3);
        }
        assert_eq!(scene_scale(0.0, 1.0), 3);
        assert_eq!(scene_scale(3.1, 1.0), scene_scale(3.9, 1.0));
    }
}
