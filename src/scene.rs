//! Wspólne wymiary planszy; nie rozciągamy fizyki ani sprite'ów do proporcji okna.
pub const WIDTH: f32 = 544.0;
pub const HEIGHT: f32 = 306.0;
pub const CENTER_X: f64 = WIDTH as f64 / 2.0;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn court_is_exactly_widescreen_and_physics_has_matching_bounds() {
        assert_eq!(WIDTH * 9.0, HEIGHT * 16.0);
        assert_eq!(crate::physics::GROUND_WIDTH as f32, WIDTH);
        assert_eq!(crate::physics::GROUND_HALF_WIDTH as f64, CENTER_X);
    }
}
