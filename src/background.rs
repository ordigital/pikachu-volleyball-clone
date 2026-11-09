//! Dopasowanie nowej ilustracji do wysokości piasku i kolizji słupka.
use crate::scene::{HEIGHT, WIDTH};
use macroquad::prelude::*;

// Wiersze w wygenerowanym obrazie 1672×941 → wysokości logiczne planszy.
// Wspólne wierzchołki/UV eliminują szwy; zachowujemy całe niebo i oba brzegi.
const ROWS: [(f32, f32); 8] = [
    (0.0, 0.0),
    (503.0, 176.0),
    (746.0, 254.0),
    (776.0, 262.0),
    (814.0, 273.0),
    (851.0, 286.0),
    (883.0, 295.0),
    (941.0, HEIGHT),
];

pub fn draw(texture: &Texture2D) {
    let mut mesh = Mesh {
        vertices: Vec::with_capacity(16),
        indices: Vec::with_capacity(42),
        texture: Some(texture.clone()),
    };
    for (i, &(source_y, y)) in ROWS.iter().enumerate() {
        for (x, u) in [(0.0, 0.0), (WIDTH, 1.0)] {
            mesh.vertices
                .push(Vertex::new(x, y, 0.0, u, source_y / 941.0, WHITE));
        }
        if i > 0 {
            let n = (i * 2) as u16;
            mesh.indices
                .extend_from_slice(&[n - 2, n - 1, n, n - 1, n + 1, n]);
        }
    }
    draw_mesh(&mesh);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_landmarks_cover_the_scene_without_gaps_or_reversals() {
        assert_eq!(ROWS[0], (0.0, 0.0));
        assert_eq!(ROWS.last().unwrap().1, HEIGHT);
        for rows in ROWS.windows(2) {
            assert!(rows[0].0 < rows[1].0 && rows[0].1 < rows[1].1);
        }
        assert_eq!(ROWS[1].1, 176.0); // top of collision pole
        assert_eq!(ROWS[4].1, 273.0); // sand contact / shadows
    }
}
