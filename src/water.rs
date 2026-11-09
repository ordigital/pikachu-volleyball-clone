//! Płynny przypływ przy dolnej krawędzi; tylko zegar symulacji, bez sprite'ów/RNG.
use macroquad::prelude::*;

const PERIOD: f64 = 7.2;
use crate::scene::{HEIGHT as BOTTOM, WIDTH};
const STEP: f32 = 4.0;

#[derive(Default)]
pub struct Wave {
    ticks: u64,
}

impl Wave {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn advance(&mut self) {
        self.ticks += 1;
    }

    pub fn draw(&self, t: f64) {
        let seconds = (self.ticks.saturating_sub(1) as f64 + t.clamp(0.0, 1.0)) / 25.0;
        let mut mesh = Mesh {
            vertices: Vec::with_capacity(440),
            indices: Vec::with_capacity(660),
            texture: None,
        };
        for i in 0..=(WIDTH / STEP) as usize {
            let x = i as f32 * STEP;
            let y = surface_y(seconds, x);
            mesh.vertices.push(Vertex::new(
                x,
                y,
                0.0,
                0.0,
                0.0,
                Color::new(0.35, 0.82, 0.96, 0.80),
            ));
            mesh.vertices.push(Vertex::new(
                x,
                BOTTOM + 14.0,
                0.0,
                0.0,
                0.0,
                Color::new(0.06, 0.48, 0.78, 0.94),
            ));
            if i > 0 {
                let n = (i * 2) as u16;
                mesh.indices
                    .extend_from_slice(&[n - 2, n, n - 1, n, n + 1, n - 1]);
            }
        }
        draw_mesh(&mesh);
        for i in 0..(WIDTH / STEP) as usize {
            let x = i as f32 * STEP;
            let y = surface_y(seconds, x);
            let next = surface_y(seconds, x + STEP);
            // Szerszy półprzezroczysty brzeg i cienka jasna piana.
            draw_line(
                x,
                y + 1.0,
                x + STEP,
                next + 1.0,
                5.0,
                Color::new(0.9, 1.0, 1.0, 0.18),
            );
            draw_line(x, y, x + STEP, next, 1.6, Color::new(0.95, 1.0, 1.0, 0.85));
            if i % 7 < 3 {
                draw_line(
                    x,
                    y + 7.0,
                    x + STEP,
                    next + 7.0,
                    0.7,
                    Color::new(0.8, 0.98, 1.0, 0.24),
                );
            }
        }
    }
}

fn surface_y(seconds: f64, x: f32) -> f32 {
    let phase = seconds * std::f64::consts::TAU / PERIOD;
    let surge = ((1.0 - phase.cos()) * 0.5).powi(2) as f32;
    let ripples = (x * 0.045 + seconds as f32 * 1.7).sin() * 1.4
        + (x * 0.09 - seconds as f32 * 1.2).sin() * 0.55;
    (BOTTOM + 11.0) - 24.0 * surge + ripples * surge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tide_enters_then_completely_retreats_without_covering_players() {
        for x in 0..=WIDTH as i32 {
            assert!(surface_y(0.0, x as f32) > BOTTOM + 5.0);
            let crest = surface_y(PERIOD / 2.0, x as f32);
            assert!(crest > BOTTOM - 15.0 && crest < BOTTOM);
            assert!(surface_y(PERIOD, x as f32) > BOTTOM + 5.0);
        }
    }

    #[test]
    fn tide_has_no_discontinuity_at_cycle_boundary() {
        for x in [0.0, WIDTH / 2.0, WIDTH] {
            assert!((surface_y(PERIOD - 0.001, x) - surface_y(PERIOD + 0.001, x)).abs() < 0.01);
        }
    }
}
