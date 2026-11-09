//! Cache fontu rosnący z rozmiarem okna/DPI, z zapasem na symetryczny obrys.
use crate::gfx::TextStyle;
use macroquad::miniquad::{UniformDesc, UniformType};
use macroquad::prelude::*;
use std::{cell::RefCell, rc::Rc};

const FONT_SIZE: u16 = 96;

pub fn outline_material() -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: crate::rendering::VERTEX,
            fragment: include_str!("../assets/shaders/outline.frag"),
        },
        MaterialParams {
            pipeline_params: crate::rendering::source_over(),
            uniforms: vec![UniformDesc::new("OutlineUv", UniformType::Float2)],
            ..Default::default()
        },
    )
    .expect("text outline material")
}

struct Raster {
    target: RenderTarget,
    glyph_size: Vec2,
    font_size: u16,
    padding: f32,
}

#[derive(Clone)]
pub struct TextLabel {
    raster: Rc<RefCell<Raster>>,
    text: Rc<str>,
    font: Font,
    material: Material,
}

impl Raster {
    fn new(text: &str, font: &Font, font_size: u16) -> Self {
        let padding = font_size as f32 / 8.0;
        let dimensions = measure_text(text, Some(font), font_size, 1.0);
        let glyph_size = vec2(dimensions.width, dimensions.height);
        let size = glyph_size.ceil() + Vec2::splat(padding * 2.0);
        let target = render_target(size.x as u32, size.y as u32);
        target.texture.set_filter(FilterMode::Linear);
        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, size.x, size.y));
        camera.render_target = Some(target.clone());
        push_camera_state();
        set_camera(&camera);
        clear_background(BLACK);
        // Czytamy kanał R maski (pokrycie), a nie podniesione do kwadratu alfa
        // domyślnego renderera glyphów na przezroczystym tle.
        draw_text_ex(
            text,
            padding,
            padding + dimensions.offset_y,
            TextParams {
                font: Some(font),
                font_size,
                color: WHITE,
                ..Default::default()
            },
        );
        pop_camera_state();
        Self {
            target,
            glyph_size,
            font_size,
            padding,
        }
    }
}

impl TextLabel {
    pub fn new(text: &str, font: &Font, material: &Material) -> Self {
        Self {
            raster: Rc::new(RefCell::new(Raster::new(text, font, FONT_SIZE))),
            text: Rc::from(text),
            font: font.clone(),
            material: material.clone(),
        }
    }

    pub fn draw(&self, origin: Vec2, size: Vec2, style: TextStyle, alpha: f32) {
        let mut raster = self.raster.borrow_mut();
        let physical_scale = crate::rendering::scene_scale(
            (screen_width() / crate::scene::WIDTH).min(screen_height() / crate::scene::HEIGHT),
            screen_dpi_scale(),
        ) as f32;
        let fit =
            (size.x / raster.glyph_size.x.max(1.0)).min(size.y / raster.glyph_size.y.max(1.0));
        let needed = raster.font_size as f32 * fit * physical_scale;
        let font_size = cache_font_size(raster.font_size, needed);
        if font_size != raster.font_size {
            *raster = Raster::new(&self.text, &self.font, font_size);
        }
        let scale =
            (size.x / raster.glyph_size.x.max(1.0)).min(size.y / raster.glyph_size.y.max(1.0));
        let texture_size = raster.target.texture.size();
        let position =
            origin + (size - raster.glyph_size * scale) / 2.0 - Vec2::splat(raster.padding * scale);
        self.material.set_uniform(
            "OutlineUv",
            vec2(
                raster.font_size as f32 * style.outline / texture_size.x,
                raster.font_size as f32 * style.outline / texture_size.y,
            ),
        );
        let mut color = style.color;
        color.a *= alpha;
        gl_use_material(&self.material);
        draw_texture_ex(
            &raster.target.texture,
            position.x,
            position.y,
            color,
            DrawTextureParams {
                dest_size: Some(texture_size * scale),
                flip_y: true,
                rotation: style.rotation,
                pivot: Some(origin + size / 2.0),
                ..Default::default()
            },
        );
        gl_use_default_material();
    }
}

// Cache tylko rośnie i używa progów 96/192/384/768/1536, zamiast co klatkę.
fn cache_font_size(current: u16, needed: f32) -> u16 {
    let mut size = current;
    while (size as f32) < needed && size < 1536 {
        size *= 2;
    }
    size
}

#[cfg(test)]
mod tests {
    use super::cache_font_size;

    #[test]
    fn font_cache_covers_physical_size_without_resize_churn() {
        assert_eq!(cache_font_size(96, 256.0), 384);
        assert_eq!(cache_font_size(384, 260.0), 384);
        assert_eq!(cache_font_size(384, 80.0), 384);
        assert_eq!(cache_font_size(96, 1100.0), 1536);
    }
}
