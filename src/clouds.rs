//! Ruch i pulsowanie chmur (port części `cloud_and_wave.js`).
//! FUN_00404770 w oryginalnym kodzie maszynowym.

use crate::physics::GROUND_WIDTH;
use crate::rand::rand;

pub const NUM_OF_CLOUDS: usize = 10;

pub struct Cloud {
    pub top_left_point_x: i32,
    pub top_left_point_y: i32,
    pub top_left_point_x_velocity: i32,
    pub size_diff_turn_number: f64,
}

impl Cloud {
    pub fn new() -> Self {
        // kolejność wywołań rand() jak w JS
        let top_left_point_x = -68 + (rand() % (GROUND_WIDTH + 68));
        let top_left_point_y = rand() % 152;
        let top_left_point_x_velocity = 1 + (rand() % 2);
        let size_diff_turn_number = (rand() % 11) as f64;
        Self {
            top_left_point_x,
            top_left_point_y,
            top_left_point_x_velocity,
            size_diff_turn_number,
        }
    }

    /// [0, 1, 2, 3, 4, 5, 4, 3, 2, 1, 0][size_diff_turn_number]
    pub fn size_diff(&self) -> f64 {
        (5.0 - (self.size_diff_turn_number - 5.0).abs()).max(0.0)
    }
    pub fn sprite_top_left_point_x(&self) -> f64 {
        self.top_left_point_x as f64 - self.size_diff()
    }
    pub fn sprite_top_left_point_y(&self) -> f64 {
        self.top_left_point_y as f64 - self.size_diff()
    }
    pub fn sprite_width(&self) -> f64 {
        48.0 + 2.0 * self.size_diff()
    }
    pub fn sprite_height(&self) -> f64 {
        24.0 + 2.0 * self.size_diff()
    }
}

/// Przesuwa chmury i aktualizuje ich oddech o jeden tick.
pub fn advance_clouds(cloud_array: &mut [Cloud]) {
    for cloud in cloud_array.iter_mut().take(NUM_OF_CLOUDS) {
        cloud.top_left_point_x += cloud.top_left_point_x_velocity;
        if cloud.top_left_point_x > GROUND_WIDTH {
            cloud.top_left_point_x = -68;
            cloud.top_left_point_y = rand() % 152;
            cloud.top_left_point_x_velocity = 1 + (rand() % 2);
        }
        cloud.size_diff_turn_number = (cloud.size_diff_turn_number + 0.25) % 11.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breathing_takes_44_ticks_without_slowing_horizontal_motion() {
        let mut clouds = [Cloud {
            top_left_point_x: 0,
            top_left_point_y: 50,
            top_left_point_x_velocity: 2,
            size_diff_turn_number: 0.0,
        }];
        for tick in 1..=44 {
            advance_clouds(&mut clouds);
            assert_eq!(clouds[0].top_left_point_x, tick * 2);
            assert!(clouds[0].size_diff() >= 0.0);
            if tick == 20 {
                assert_eq!(clouds[0].size_diff(), 5.0);
            }
        }
        assert_eq!(clouds[0].size_diff_turn_number, 0.0);
        assert_eq!(clouds[0].size_diff(), 0.0);
    }
}
