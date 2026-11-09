//! Port `keyboard.js` – wejście gracza (klawiatura + pad + dotyk) zamrożone na czas klatki.

use crate::input::{Action, PlayerControls};
use crate::pad::Pads;
use crate::physics::PikaUserInput;
use crate::touch::TouchState;
use macroquad::prelude::{is_key_down, KeyCode};

pub struct PikaKeyboard {
    pub input: PikaUserInput,
    power_hit_key_is_down_previous: bool,
    suppress_enter_until_release: bool,
}

impl PikaKeyboard {
    pub fn new() -> Self {
        Self {
            input: PikaUserInput::default(),
            power_hit_key_is_down_previous: false,
            suppress_enter_until_release: false,
        }
    }

    pub fn suppress_fullscreen_enter(&mut self) {
        self.suppress_enter_until_release = true;
    }

    /// Called each rendered frame, including while simulation is paused.
    pub fn update_enter_release(&mut self, enter_down: bool) {
        self.suppress_enter_until_release &= enter_down;
    }

    /// Zamraża stan sterowania na czas jednej klatki gry.
    pub fn get_input(&mut self, c: &PlayerControls, pads: &Pads, touch: Option<&TouchState>) {
        let held = |a: Action| -> bool {
            let bd = c.actions[a as usize];
            let key = bd.key.is_some_and(|key| {
                is_key_down(key)
                    && !(self.suppress_enter_until_release
                        && matches!(key, KeyCode::Enter | KeyCode::KpEnter))
            });
            let pad = match (c.pad_index, bd.pad) {
                (Some(i), Some(p)) => pads.is_down(i, p),
                _ => false,
            };
            key || pad
        };
        let (sx, sy) = c.pad_index.map(|i| pads.stick(i)).unwrap_or((0.0, 0.0));
        let t = touch.copied().unwrap_or_default();

        let left = held(Action::Left) || sx < -0.5 || t.left;
        let right = held(Action::Right) || sx > 0.5 || t.right;
        let up = held(Action::Up) || sy < -0.5 || t.up;
        let down = held(Action::Down) || sy > 0.5 || t.down;

        self.input.x_direction = if left {
            -1
        } else if right {
            1
        } else {
            0
        };
        self.input.y_direction = if up {
            -1
        } else if down {
            1
        } else {
            0
        };

        let is_down = held(Action::PowerHit) || t.action;
        self.input.power_hit = if !self.power_hit_key_is_down_previous && is_down {
            1
        } else {
            0
        };
        self.power_hit_key_is_down_previous = is_down;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_enter_release_is_observed_without_simulation_ticks() {
        let mut keyboard = PikaKeyboard::new();
        keyboard.suppress_fullscreen_enter();
        keyboard.update_enter_release(true);
        assert!(keyboard.suppress_enter_until_release);
        // Release during pause, then press again before the next simulation tick.
        keyboard.update_enter_release(false);
        keyboard.update_enter_release(true);
        assert!(!keyboard.suppress_enter_until_release);
    }
}
