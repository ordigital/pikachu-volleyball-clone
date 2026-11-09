//! Invisible touch controls shared by desktop and Android.
//! Drag from either movement half; drag up to jump; touch the opposite half to hit / dive.
//! Mouse clicks stay reserved for the desktop UI.

use macroquad::prelude::*;

#[derive(Clone, Copy, Default)]
pub struct TouchState {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub action: bool,
}

pub struct TouchControls {
    pub right_handed: bool,
    stick: Option<(u64, Vec2)>,
}

impl TouchControls {
    pub fn new() -> Self {
        Self {
            right_handed: true,
            stick: None,
        }
    }

    /// `dpi`: touches() reports physical pixels while screen_width() is logical (browser high-DPI).
    pub fn update(&mut self, dpi: f32, blocked: bool) -> TouchState {
        mobile_input(
            &mut self.stick,
            &touches(),
            screen_width() * dpi,
            screen_height() * dpi,
            blocked,
            self.right_handed,
        )
    }
}

fn mobile_input(
    stick: &mut Option<(u64, Vec2)>,
    touches: &[Touch],
    width: f32,
    height: f32,
    blocked: bool,
    right_handed: bool,
) -> TouchState {
    let active = |t: &&Touch| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
    if blocked {
        *stick = None;
        return TouchState::default();
    }
    if stick.is_some_and(|(id, _)| !touches.iter().filter(active).any(|t| t.id == id)) {
        *stick = None;
    }
    if stick.is_none() {
        if let Some(touch) = touches.iter().find(|t| {
            t.phase == TouchPhase::Started && (t.position.x < width * 0.5) == right_handed
        }) {
            *stick = Some((touch.id, touch.position));
        }
    }
    let mut state = TouchState::default();
    if let Some((id, origin)) = *stick {
        if let Some(touch) = touches.iter().filter(active).find(|t| t.id == id) {
            let delta = touch.position - origin;
            let dead_zone = width.min(height) * 0.015;
            state.left = delta.x < -dead_zone;
            state.right = delta.x > dead_zone;
            state.up = delta.y < -dead_zone;
            state.down = delta.y > dead_zone;
        }
    }
    state.action = touches.iter().filter(active).any(|t| {
        (t.position.x < width * 0.5) != right_handed && !stick.is_some_and(|(id, _)| t.id == id)
    });
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    fn finger(id: u64, phase: TouchPhase, x: f32, y: f32) -> Touch {
        Touch {
            id,
            phase,
            position: vec2(x, y),
        }
    }
    #[test]
    fn mobile_stick_follows_the_first_left_finger_and_supports_simultaneous_hit() {
        let mut stick = None;
        let state = mobile_input(
            &mut stick,
            &[finger(4, TouchPhase::Started, 100.0, 200.0)],
            1000.0,
            500.0,
            false,
            true,
        );
        assert!(!state.left && !state.right && !state.action);
        let state = mobile_input(
            &mut stick,
            &[
                finger(4, TouchPhase::Moved, 140.0, 160.0),
                finger(9, TouchPhase::Started, 800.0, 300.0),
            ],
            1000.0,
            500.0,
            false,
            true,
        );
        assert!(state.right && state.up && !state.down && state.action);
        let state = mobile_input(
            &mut stick,
            &[finger(4, TouchPhase::Moved, 700.0, 200.0)],
            1000.0,
            500.0,
            false,
            true,
        );
        assert!(state.right && !state.action);
        mobile_input(
            &mut stick,
            &[finger(4, TouchPhase::Ended, 700.0, 200.0)],
            1000.0,
            500.0,
            false,
            true,
        );
        assert!(stick.is_none());
    }
    #[test]
    fn menu_touch_does_not_trigger_a_hit_and_resets_the_stick() {
        let mut stick = Some((1, vec2(30.0, 20.0)));
        let state = mobile_input(
            &mut stick,
            &[finger(2, TouchPhase::Started, 10.0, 10.0)],
            1000.0,
            500.0,
            true,
            true,
        );
        assert!(!state.action && stick.is_none());
    }

    #[test]
    fn upward_drag_jumps_and_action_with_sideways_drag_dives_or_hits_in_air() {
        use crate::physics::{PikaPhysics, PikaUserInput, PLAYER_TOUCHING_GROUND_Y_COORD};
        for right_handed in [true, false] {
            let origin_x = if right_handed { 100.0 } else { 800.0 };
            let action_x = if right_handed { 800.0 } else { 100.0 };
            let mut stick = Some((1, vec2(origin_x, 200.0)));
            let movement = |y| finger(1, TouchPhase::Moved, origin_x + 40.0, y);
            let action = finger(2, TouchPhase::Started, action_x, 200.0);
            let input = |state: TouchState| PikaUserInput {
                x_direction: i32::from(state.right) - i32::from(state.left),
                y_direction: i32::from(state.down) - i32::from(state.up),
                power_hit: i32::from(state.action),
            };
            let state = mobile_input(
                &mut stick,
                &[movement(200.0), action.clone()],
                1000.0,
                500.0,
                false,
                right_handed,
            );
            let mut physics = PikaPhysics::new(false, false);
            physics.run_engine_for_next_frame(&mut [input(state), PikaUserInput::default()]);
            assert_eq!(physics.player1.state, 3); // Ground action dives, never jumps.
            assert_eq!(physics.player1.diving_direction, 1);

            let state = mobile_input(
                &mut stick,
                &[movement(160.0)],
                1000.0,
                500.0,
                false,
                right_handed,
            );
            let mut physics = PikaPhysics::new(false, false);
            physics.run_engine_for_next_frame(&mut [input(state), PikaUserInput::default()]);
            assert_eq!(physics.player1.state, 1);
            assert!(physics.player1.y < PLAYER_TOUCHING_GROUND_Y_COORD);
            let state = mobile_input(
                &mut stick,
                &[movement(160.0), action],
                1000.0,
                500.0,
                false,
                right_handed,
            );
            physics.run_engine_for_next_frame(&mut [input(state), PikaUserInput::default()]);
            assert_eq!(physics.player1.state, 2); // Action in the air smashes.
        }
    }

    #[test]
    fn left_handed_mode_uses_right_stick_and_left_hit() {
        let mut stick = None;
        mobile_input(
            &mut stick,
            &[finger(1, TouchPhase::Started, 800.0, 300.0)],
            1000.0,
            500.0,
            false,
            false,
        );
        let state = mobile_input(
            &mut stick,
            &[
                finger(1, TouchPhase::Moved, 750.0, 260.0),
                finger(2, TouchPhase::Started, 100.0, 300.0),
            ],
            1000.0,
            500.0,
            false,
            false,
        );
        assert!(state.left && state.up && !state.down && state.action);
    }
}
