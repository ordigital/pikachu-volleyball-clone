//! Trawa przesuwana po skosie jednym quadem, bez dużego mesha źdźbeł.
use macroquad::miniquad::{UniformDesc, UniformType};
use macroquad::prelude::*;

pub struct GrassBackground {
    material: Material,
}

impl GrassBackground {
    pub fn new() -> Self {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: crate::rendering::VERTEX,
                fragment: include_str!("../assets/shaders/grass.frag"),
            },
            MaterialParams {
                pipeline_params: crate::rendering::source_over(),
                uniforms: vec![
                    UniformDesc::new("Time", UniformType::Float1),
                    UniformDesc::new("UvScale", UniformType::Float2),
                ],
                ..Default::default()
            },
        )
        .expect("scrolling grass material");
        Self { material }
    }

    pub fn draw(&self, texture: &Texture2D) {
        // Oba przesunięcia mają wspólny okres 500 s, więc reset nie robi skoku.
        self.material
            .set_uniform("Time", (get_time() % 500.0) as f32);
        self.material.set_uniform(
            "UvScale",
            vec2(crate::scene::WIDTH / 432.0, crate::scene::HEIGHT / 304.0),
        );
        gl_use_material(&self.material);
        draw_texture_ex(
            texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(crate::scene::WIDTH, crate::scene::HEIGHT)),
                ..Default::default()
            },
        );
        gl_use_default_material();
    }
}
