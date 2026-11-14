//! Invisible touch controls shared by desktop and Android.
//! Drag from either movement half; drag up to jump; touch the opposite half to hit / dive.
//! Mouse clicks stay reserved for the desktop UI.
//!
//! In portrait orientation on Android, visible on-screen controls are drawn below the game
//! scene: a D-pad on one side and A (jump) / B (hit) buttons on the other. The side depends
//! on the righthanded/lefthanded setting.

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

    /// Portrait mode: process touches against visible on-screen button regions.
    pub fn update_portrait(&mut self, blocked: bool, layout: &PortraitLayout, in_menu: bool) -> TouchState {
        portrait_input(&touches(), blocked, layout, in_menu)
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

// ---------------------------------------------------------------------------
// Portrait on-screen controls
// ---------------------------------------------------------------------------

/// Precomputed layout of the on-screen controls for portrait orientation.
#[allow(dead_code)]
pub struct PortraitLayout {
    /// Where the game scene texture is blitted (top of screen).
    pub game_rect: Rect,
    // D-pad
    pub dpad_center: Vec2,
    pub dpad_radius: f32,
    // Action buttons
    pub btn_a_center: Vec2, // Jump (= Up)
    pub btn_b_center: Vec2, // Hit (PowerHit)
    pub btn_radius: f32,
    pub right_handed: bool,
}

impl PortraitLayout {
    /// Compute layout from current screen dimensions and handedness.
    pub fn compute(screen_w: f32, screen_h: f32, right_handed: bool) -> Self {
        // Game scene at top, filling width, maintaining 16:9 aspect.
        let game_w = screen_w;
        let game_h = screen_w * (306.0 / 544.0);
        let game_rect = Rect::new(0.0, 0.0, game_w, game_h.min(screen_h * 0.55));

        let controls_top = game_rect.h;
        let controls_h = screen_h - controls_top;

        // D-pad and button sizing proportional to available space.
        let dpad_radius = (screen_w * 0.18).min(controls_h * 0.22);
        let btn_radius = dpad_radius * 0.48;
        let btn_spacing = dpad_radius * 1.05;

        let margin_x = screen_w * 0.23;

        let (dpad_x, btn_base_x) = if right_handed {
            (margin_x, screen_w - margin_x)
        } else {
            (screen_w - margin_x, margin_x)
        };

        // Controls center: positioned comfortably in the thumb sweet spot (~58% down control area).
        let controls_cy = controls_top + controls_h * 0.58;

        // Buttons: A (jump) upper, B (hit) lower, slanted naturally for the thumb arc.
        let (btn_a_center, btn_b_center) = if right_handed {
            (
                Vec2::new(btn_base_x + btn_spacing * 0.42, controls_cy - btn_spacing * 0.35),
                Vec2::new(btn_base_x - btn_spacing * 0.42, controls_cy + btn_spacing * 0.35),
            )
        } else {
            (
                Vec2::new(btn_base_x - btn_spacing * 0.42, controls_cy - btn_spacing * 0.35),
                Vec2::new(btn_base_x + btn_spacing * 0.42, controls_cy + btn_spacing * 0.35),
            )
        };

        PortraitLayout {
            game_rect,
            dpad_center: Vec2::new(dpad_x, controls_cy),
            dpad_radius,
            btn_a_center,
            btn_b_center,
            btn_radius,
            right_handed,
        }
    }

    /// Draw the on-screen controls (outlines only, RetroArch style).
    pub fn draw(&self) {
        let outline = Color::new(1.0, 1.0, 1.0, 0.38);
        let fill = Color::new(1.0, 1.0, 1.0, 0.06);
        let pressed_fill = Color::new(1.0, 1.0, 1.0, 0.18);
        let label_color = Color::new(1.0, 1.0, 1.0, 0.50);

        // Which buttons/directions are currently pressed (for visual feedback).
        let touches = touches();
        let active = |t: &&Touch| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
        let active_touches: Vec<&Touch> = touches.iter().filter(active).collect();

        // --- D-pad ---
        self.draw_dpad(&active_touches, outline, fill, pressed_fill);

        // --- A button (jump) ---
        let a_pressed = active_touches.iter().any(|t| {
            (t.position - self.btn_a_center).length() < self.btn_radius * 1.4
        });
        let a_fill = if a_pressed { pressed_fill } else { fill };
        draw_circle(self.btn_a_center.x, self.btn_a_center.y, self.btn_radius, a_fill);
        draw_circle_lines(self.btn_a_center.x, self.btn_a_center.y, self.btn_radius, 2.0, outline);
        // "A" label
        let label_size = self.btn_radius * 0.9;
        draw_text("A", self.btn_a_center.x - label_size * 0.28, self.btn_a_center.y + label_size * 0.3, label_size, label_color);

        // --- B button (hit) ---
        let b_pressed = active_touches.iter().any(|t| {
            (t.position - self.btn_b_center).length() < self.btn_radius * 1.4
        });
        let b_fill = if b_pressed { pressed_fill } else { fill };
        draw_circle(self.btn_b_center.x, self.btn_b_center.y, self.btn_radius, b_fill);
        draw_circle_lines(self.btn_b_center.x, self.btn_b_center.y, self.btn_radius, 2.0, outline);
        let label_size = self.btn_radius * 0.9;
        draw_text("B", self.btn_b_center.x - label_size * 0.28, self.btn_b_center.y + label_size * 0.3, label_size, label_color);
    }

    fn draw_dpad(&self, active_touches: &[&Touch], outline: Color, fill: Color, pressed_fill: Color) {
        let cx = self.dpad_center.x;
        let cy = self.dpad_center.y;
        let r = self.dpad_radius;
        let arm_w = r * 0.48; // Wider arms (up/down wider, left/right taller)
        let arm_len = r * 0.94; // Length from center to tip
        let arrow_size = arm_w * 0.48;

        // Determine which directions are pressed.
        let (d_left, d_right, d_up, d_down) = self.dpad_directions(active_touches);

        // Draw the cross shape: four rectangular arms.
        // Up arm
        let up_fill = if d_up { pressed_fill } else { fill };
        draw_rectangle(cx - arm_w * 0.5, cy - arm_len, arm_w, arm_len - arm_w * 0.5, up_fill);
        draw_rectangle_lines(cx - arm_w * 0.5, cy - arm_len, arm_w, arm_len - arm_w * 0.5, 2.0, outline);
        // Up arrow
        draw_triangle(
            Vec2::new(cx, cy - arm_len + arrow_size * 0.3),
            Vec2::new(cx - arrow_size, cy - arm_len + arrow_size * 1.5),
            Vec2::new(cx + arrow_size, cy - arm_len + arrow_size * 1.5),
            outline,
        );

        // Down arm
        let down_fill = if d_down { pressed_fill } else { fill };
        draw_rectangle(cx - arm_w * 0.5, cy + arm_w * 0.5, arm_w, arm_len - arm_w * 0.5, down_fill);
        draw_rectangle_lines(cx - arm_w * 0.5, cy + arm_w * 0.5, arm_w, arm_len - arm_w * 0.5, 2.0, outline);
        // Down arrow
        draw_triangle(
            Vec2::new(cx, cy + arm_len - arrow_size * 0.3),
            Vec2::new(cx - arrow_size, cy + arm_len - arrow_size * 1.5),
            Vec2::new(cx + arrow_size, cy + arm_len - arrow_size * 1.5),
            outline,
        );

        // Left arm
        let left_fill = if d_left { pressed_fill } else { fill };
        draw_rectangle(cx - arm_len, cy - arm_w * 0.5, arm_len - arm_w * 0.5, arm_w, left_fill);
        draw_rectangle_lines(cx - arm_len, cy - arm_w * 0.5, arm_len - arm_w * 0.5, arm_w, 2.0, outline);
        // Left arrow
        draw_triangle(
            Vec2::new(cx - arm_len + arrow_size * 0.3, cy),
            Vec2::new(cx - arm_len + arrow_size * 1.5, cy - arrow_size),
            Vec2::new(cx - arm_len + arrow_size * 1.5, cy + arrow_size),
            outline,
        );

        // Right arm
        let right_fill = if d_right { pressed_fill } else { fill };
        draw_rectangle(cx + arm_w * 0.5, cy - arm_w * 0.5, arm_len - arm_w * 0.5, arm_w, right_fill);
        draw_rectangle_lines(cx + arm_w * 0.5, cy - arm_w * 0.5, arm_len - arm_w * 0.5, arm_w, 2.0, outline);
        // Right arrow
        draw_triangle(
            Vec2::new(cx + arm_len - arrow_size * 0.3, cy),
            Vec2::new(cx + arm_len - arrow_size * 1.5, cy - arrow_size),
            Vec2::new(cx + arm_len - arrow_size * 1.5, cy + arrow_size),
            outline,
        );

        // Center square
        draw_rectangle(cx - arm_w * 0.5, cy - arm_w * 0.5, arm_w, arm_w, fill);
        draw_rectangle_lines(cx - arm_w * 0.5, cy - arm_w * 0.5, arm_w, arm_w, 2.0, outline);
    }

    /// Determine which D-pad directions are pressed based on active touches.
    fn dpad_directions(&self, active_touches: &[&Touch]) -> (bool, bool, bool, bool) {
        let cx = self.dpad_center.x;
        let cy = self.dpad_center.y;
        let r = self.dpad_radius;
        let (mut left, mut right, mut up, mut down) = (false, false, false, false);

        for t in active_touches {
            let dx = t.position.x - cx;
            let dy = t.position.y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > r * 1.35 { continue; } // Outside D-pad zone

            let dead = r * 0.14;
            if dist < dead { continue; }

            if dx < -dead { left = true; }
            if dx > dead { right = true; }
            if dy < -dead { up = true; }
            if dy > dead { down = true; }
        }
        (left, right, up, down)
    }
}

/// Process touches in portrait mode against the on-screen control layout.
fn portrait_input(touches: &[Touch], blocked: bool, layout: &PortraitLayout, in_menu: bool) -> TouchState {
    if blocked {
        return TouchState::default();
    }
    let active = |t: &&Touch| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
    let active_touches: Vec<&Touch> = touches.iter().filter(active).collect();
    let mut state = TouchState::default();

    // D-pad directions
    let (d_left, d_right, d_up, d_down) = layout.dpad_directions(&active_touches);
    if !in_menu {
        state.left = d_left;
        state.right = d_right;
    }
    state.up = d_up;
    state.down = d_down;

    let hit_radius = layout.btn_radius * 1.4;
    let a_pressed = active_touches.iter().any(|t| {
        (t.position - layout.btn_a_center).length() < hit_radius
    });
    let b_pressed = active_touches.iter().any(|t| {
        (t.position - layout.btn_b_center).length() < hit_radius
    });

    if in_menu {
        // In menu: both A and B confirm the current option.
        state.action = a_pressed || b_pressed;
    } else {
        // In gameplay: A is Jump (Up), B is Hit / Dive (PowerHit / Action).
        if a_pressed {
            state.up = true;
        }
        state.action = b_pressed;
    }

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

    #[test]
    fn portrait_dpad_and_buttons_respond_to_touches() {
        let layout = PortraitLayout::compute(1080.0, 1920.0, true);
        // Touch the D-pad left zone.
        let left_touch = finger(1, TouchPhase::Moved,
            layout.dpad_center.x - layout.dpad_radius * 0.5,
            layout.dpad_center.y);
        let state = portrait_input(&[left_touch.clone()], false, &layout, false);
        assert!(state.left && !state.right && !state.up);

        // Touch the A button (jump in gameplay, confirm in menu).
        let a_touch = finger(2, TouchPhase::Started,
            layout.btn_a_center.x,
            layout.btn_a_center.y);
        let state = portrait_input(&[a_touch.clone()], false, &layout, false);
        assert!(state.up && !state.action); // A in game = jump = up
        let menu_state = portrait_input(&[a_touch], false, &layout, true);
        assert!(menu_state.action); // A in menu = confirm

        // Touch the B button (hit).
        let b_touch = finger(3, TouchPhase::Started,
            layout.btn_b_center.x,
            layout.btn_b_center.y);
        let state = portrait_input(&[b_touch.clone()], false, &layout, false);
        assert!(state.action);

        // Multi-touch: direction + B.
        let state = portrait_input(&[left_touch, b_touch], false, &layout, false);
        assert!(state.left && state.action);
    }
}
