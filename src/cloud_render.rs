//! Cienie chmur liczone w masce chmury-odbiorcy; nie rysują niczego na tle.
use crate::clouds::NUM_OF_CLOUDS;
use crate::gfx::Sprite;
use macroquad::miniquad::{UniformDesc, UniformType};
use macroquad::prelude::*;

pub struct CloudRenderer {
    texture: Texture2D,
    material: Material,
}

impl CloudRenderer {
    pub fn new(texture: Texture2D) -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: crate::rendering::VERTEX,
                fragment: include_str!("../assets/shaders/cloud.frag"),
            },
            MaterialParams {
                pipeline_params: crate::rendering::source_over(),
                uniforms: vec![
                    UniformDesc::new("CloudRects", UniformType::Float4).array(NUM_OF_CLOUDS),
                    UniformDesc::new("ReceiverIndex", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .expect("cloud receiver shadows material");
        Self { texture, material }
    }

    pub fn draw(&self, clouds: &[Sprite], t: f64) {
        let mut rects = [Vec4::ZERO; NUM_OF_CLOUDS];
        for (rect, cloud) in rects.iter_mut().zip(clouds) {
            let (x, y, w, h) = cloud.interpolated_transform(t);
            *rect = vec4(x as f32, y as f32, w as f32, h as f32);
        }
        self.material.set_uniform_array("CloudRects", &rects);
        gl_use_material(&self.material);
        for (index, rect) in rects.iter().enumerate() {
            if rect.z <= 0.0 || rect.w <= 0.0 {
                continue;
            }
            self.material.set_uniform("ReceiverIndex", index as f32);
            draw_texture_ex(
                &self.texture,
                rect.x,
                rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(rect.z, rect.w)),
                    ..Default::default()
                },
            );
        }
        gl_use_default_material();
    }
}
