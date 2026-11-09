//! Wektorowa nieregularna gwiazdka Battle; bez tekstury i bez SVG rasteryzowanego w runtime.
use crate::gfx::TextStyle;
use crate::text_label::TextLabel;
use macroquad::prelude::*;

// Punkty obrysu w układzie oryginalnego znaczka 485×485; łatwe do ręcznej edycji.
const OUTLINE: [(f32, f32); 34] = [
    (269.0, 50.0),
    (281.0, 103.0),
    (332.0, 61.0),
    (329.0, 132.0),
    (393.0, 106.0),
    (367.0, 171.0),
    (405.0, 188.0),
    (375.0, 228.0),
    (407.0, 274.0),
    (369.0, 293.0),
    (378.0, 350.0),
    (329.0, 333.0),
    (326.0, 381.0),
    (273.0, 340.0),
    (245.0, 377.0),
    (217.0, 348.0),
    (187.0, 380.0),
    (175.0, 343.0),
    (117.0, 365.0),
    (137.0, 318.0),
    (79.0, 317.0),
    (116.0, 275.0),
    (69.0, 250.0),
    (111.0, 222.0),
    (67.0, 181.0),
    (128.0, 165.0),
    (106.0, 110.0),
    (162.0, 130.0),
    (172.0, 68.0),
    (223.0, 105.0),
    (237.0, 87.0),
    (249.0, 73.0),
    (258.0, 63.0),
    (264.0, 56.0),
];

pub fn draw_battle(center: Vec2, size: Vec2, label: &TextLabel, alpha: f32) {
    let points: [Vec2; 34] =
        OUTLINE.map(|(x, y)| center + (vec2(x, y) / 485.0 - Vec2::splat(0.5)) * size);
    let middle = center + vec2(0.0, -0.025) * size;
    let scale = size.x.min(size.y) / 160.0;
    for (offset, color) in [
        (
            vec2(2.5, 3.5) * scale,
            Color::new(0.03, 0.025, 0.025, alpha),
        ),
        (Vec2::ZERO, Color::new(1.0, 0.025, 0.015, alpha)),
    ] {
        for i in 0..points.len() {
            let a = points[i] + offset;
            let b = points[(i + 1) % points.len()] + offset;
            draw_triangle(middle + offset, a, b, color);
        }
    }
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        draw_line(
            a.x,
            a.y,
            b.x,
            b.y,
            0.8 * scale,
            Color::new(0.05, 0.035, 0.03, alpha),
        );
    }
    let text_size = vec2(size.x * 0.56, size.y * 0.19);
    let text_center = center + vec2(-0.005, -0.04) * size;
    label.draw(
        text_center - text_size / 2.0,
        text_size,
        TextStyle {
            color: Color::new(1.0, 0.91, 0.03, 1.0),
            outline: 0.065,
            rotation: -0.42,
        },
        alpha,
    );
}
