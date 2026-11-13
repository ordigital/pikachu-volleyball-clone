//! Port `pikavolley.js` – Controller: maszyna stanów gry, punktacja, dźwięki.
//! Gra jest wołana `game_loop()` 25 razy na sekundę (jak w oryginale).

use crate::audio::{PikaAudio, Snd};
use crate::gfx::Assets;
use crate::input::{Controls, PadBtn};
use crate::keyboard::PikaKeyboard;
use crate::pad::Pads;
use crate::physics::{
    PikaPhysics, Player, GROUND_HALF_WIDTH, GROUND_WIDTH, PLAYER_HALF_LENGTH, PLAYER_MOVE_SPEED,
    PLAYER_TOUCHING_GROUND_Y_COORD,
};
use crate::touch::TouchState;
use crate::view::{FadeInOut, GameView, MenuView, GAME_START_SPEED};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Menu,
    AfterMenuSelection,
    BeforeStartOfNewGame,
    StartOfNewGame,
    Round,
    AfterEndOfRound,
    BeforeStartOfNextRound,
}

// liczba klatek dla poszczególnych stanów (frameTotal w JS)
const FRAME_TOTAL_AFTER_MENU_SELECTION: i32 = 15;
const FRAME_TOTAL_BEFORE_START_OF_NEW_GAME: i32 = 15;
const FRAME_TOTAL_START_OF_NEW_GAME: i32 = 71;
const FRAME_TOTAL_AFTER_END_OF_ROUND: i32 = 5;
const FRAME_TOTAL_BEFORE_START_OF_NEXT_ROUND: i32 = 30;
const FRAME_TOTAL_GAME_END: i32 = 211;
const NO_INPUT_FRAME_TOTAL_MENU: i32 = 30 * 25;
const MENU_MUSIC_FADE_FRAMES: i32 = 3 * 25;

fn menu_music_gain(remaining_frames: i32) -> f32 {
    let remaining = (remaining_frames as f32 / MENU_MUSIC_FADE_FRAMES as f32).clamp(0.0, 1.0);
    remaining * remaining
}

/// Visual-only interpolation toward the next likely position; physics stays at 25 Hz.
fn predicted_player_position(player: &Player, direction: i32, t: f64) -> (f64, f64) {
    let t = t.clamp(0.0, 1.0);
    let (min_x, max_x) = if player.is_player2 {
        (
            GROUND_HALF_WIDTH + PLAYER_HALF_LENGTH,
            GROUND_WIDTH - PLAYER_HALF_LENGTH,
        )
    } else {
        (PLAYER_HALF_LENGTH, GROUND_HALF_WIDTH - PLAYER_HALF_LENGTH)
    };
    let x = (player.x as f64 + direction as f64 * PLAYER_MOVE_SPEED as f64 * t)
        .clamp(min_x as f64, max_x as f64);
    let y = if player.y < PLAYER_TOUCHING_GROUND_Y_COORD {
        (player.y as f64 + player.y_velocity as f64 * t).min(PLAYER_TOUCHING_GROUND_Y_COORD as f64)
    } else {
        player.y as f64
    };
    (x, y)
}

pub struct PikachuVolleyball {
    menu: MenuView,
    game: GameView,
    fade: FadeInOut,

    audio: PikaAudio,
    physics: PikaPhysics,
    keyboard_array: [PikaKeyboard; 2],

    /// fps gry
    pub normal_fps: i32,
    /// fps w zwolnionym tempie
    slow_motion_fps: i32,
    slow_motion_frames_num: i32,
    slow_motion_frames_left: i32,
    slow_motion_num_of_skipped_frames: i32,

    /// Do interpolacji przy renderowaniu >25 FPS: ile kroków 25 Hz minęło od ostatniej
    /// zmiany stanu gry i ile kroków trwał poprzedni odstęp (w zwolnionym tempie = 5).
    ticks_since_advance: i32,
    advance_interval: i32,

    /// 0: z komputerem, 1: z kolegą
    selected_with_who: i32,
    menu_y_direction_previous: i32,
    android_right_handed: bool,
    touch_start_requested: bool,
    scores: [i32; 2],
    /// wygrywa ten, kto pierwszy dobije do tej liczby punktów
    pub winning_score: i32,

    game_ended: bool,
    round_ended: bool,
    is_player2_serve: bool,

    frame_counter: i32,
    no_input_frame_counter: i32,
    fade_menu_music: bool,

    paused: bool,

    state: State,
}

impl PikachuVolleyball {
    pub fn new(assets: &Assets) -> Self {
        // kolejność jak w JS: widoki (chmury losują liczby), audio, fizyka
        let menu = MenuView::new(assets);
        let game = GameView::new(assets);
        let fade = FadeInOut::new();
        let audio = PikaAudio::new();
        let physics = PikaPhysics::new(true, true);
        let keyboard_array = [PikaKeyboard::new(), PikaKeyboard::new()];

        Self {
            menu,
            game,
            fade,
            audio,
            physics,
            keyboard_array,
            normal_fps: 25,
            slow_motion_fps: 5,
            slow_motion_frames_num: 6,
            slow_motion_frames_left: 0,
            slow_motion_num_of_skipped_frames: 0,
            ticks_since_advance: 0,
            advance_interval: 1,
            selected_with_who: 0,
            menu_y_direction_previous: 0,
            android_right_handed: true,
            touch_start_requested: false,
            scores: [0, 0],
            winning_score: 15,
            game_ended: false,
            round_ended: false,
            is_player2_serve: false,
            frame_counter: 0,
            no_input_frame_counter: 0,
            fade_menu_music: false,
            paused: false,
            state: State::Menu,
        }
    }

    /// Pętla gry – wołać dokładnie `normal_fps` (25) razy na sekundę.
    /// Fizyka, animacje klatkowe i dźwięk zawsze chodzą w tym stałym kroku;
    /// płynność przy 60+ FPS zapewnia interpolacja pozycji w `draw`.
    pub fn game_loop(&mut self, controls: &Controls, pads: &Pads, touch: &TouchState) {
        if self.paused {
            return;
        }
        if self.slow_motion_frames_left > 0 {
            self.slow_motion_num_of_skipped_frames += 1;
            let every = ((self.normal_fps as f64) / (self.slow_motion_fps as f64)).round() as i32;
            if self.slow_motion_num_of_skipped_frames % every != 0 {
                self.ticks_since_advance += 1;
                return;
            }
            self.slow_motion_frames_left -= 1;
            self.slow_motion_num_of_skipped_frames = 0;
        }
        self.advance_interval = (self.ticks_since_advance + 1).max(1);
        self.ticks_since_advance = 0;
        // obecne pozycje stają się „poprzednimi” (punkt wyjścia interpolacji)
        self.game.begin_tick();
        self.menu.begin_tick();

        // złap wejście (klawiatura, pad, dotyk) i zamroź je na czas klatki
        let touch_right = if cfg!(target_os = "android") {
            self.android_right_handed
        } else {
            self.physics.player1.is_computer || !self.physics.player2.is_computer
        };
        self.keyboard_array[0].get_input(
            &controls.players[0],
            pads,
            if touch_right { None } else { Some(touch) },
        );
        self.keyboard_array[1].get_input(
            &controls.players[1],
            pads,
            if touch_right { Some(touch) } else { None },
        );
        match self.state {
            State::Menu => self.menu_state(),
            State::AfterMenuSelection => self.after_menu_selection(),
            State::BeforeStartOfNewGame => self.before_start_of_new_game(),
            State::StartOfNewGame => self.start_of_new_game(),
            State::Round => self.round(),
            State::AfterEndOfRound => self.after_end_of_round(),
            State::BeforeStartOfNextRound => self.before_start_of_next_round(),
        }
    }

    /// Ułamek bieżącego kroku symulacji ∈ [0, 1] (`frac` – ułamek kroku 1/25 s, który
    /// upłynął od ostatniego `game_loop`).
    pub fn render_t(&self, frac: f64) -> f64 {
        if self.paused {
            return 1.0;
        }
        ((self.ticks_since_advance as f64 + frac) / self.advance_interval as f64).clamp(0.0, 1.0)
    }

    pub fn draw(&self, t: f64, touch: &TouchState, pads: &Pads) {
        self.menu.draw(t);
        let prediction = if cfg!(target_os = "android")
            && self.state == State::Round
            && !self.round_ended
            && !self.game_ended
            && !self.paused
        {
            let index = usize::from(self.android_right_handed);
            let player = if index == 0 {
                &self.physics.player1
            } else {
                &self.physics.player2
            };
            if player.state < 3 {
                let (sx, _) = pads.stick(0);
                let left = touch.left || pads.is_down(0, PadBtn::DPadLeft) || sx < -0.5;
                let right = touch.right || pads.is_down(0, PadBtn::DPadRight) || sx > 0.5;
                let direction = if left {
                    -1
                } else if right {
                    1
                } else {
                    0
                };
                let (x, y) = predicted_player_position(player, direction, t);
                Some((index, x, y))
            } else {
                None
            }
        } else {
            None
        };
        self.game.draw(t, prediction);
        self.fade.draw();
    }

    fn power_hit_pressed(&self) -> bool {
        self.keyboard_array[0].input.power_hit == 1 || self.keyboard_array[1].input.power_hit == 1
    }

    /// Menu: z kim grasz? Z komputerem czy z kolegą?
    fn menu_state(&mut self) {
        if self.frame_counter == 0 {
            self.audio.play_menu_bgm();
            self.menu.set_visible(true);
            self.fade.set_black_alpha_to(0.0);
            self.selected_with_who = 0;
            self.menu.select_with_who(self.selected_with_who);
        }
        let k0 = self.keyboard_array[0].input;
        let k1 = self.keyboard_array[1].input;
        if [k0, k1]
            .iter()
            .any(|k| k.x_direction != 0 || k.y_direction != 0 || k.power_hit == 1)
        {
            self.no_input_frame_counter = 0;
        } else {
            self.no_input_frame_counter += 1;
        }
        if !cfg!(target_os = "android") {
            self.audio.set_bgm_gain(menu_music_gain(
                NO_INPUT_FRAME_TOTAL_MENU - self.no_input_frame_counter,
            ));
        }
        // Dwa kroki animacji menu na jeden tick: czas wejścia skrócony o połowę.
        let animation_steps = if self.frame_counter < 71 { 2 } else { 1 };
        for _ in 0..animation_steps {
            let fc = self.frame_counter;
            self.menu.draw_fight_message(fc);
            self.menu.update_entrance_fade(fc);
            self.menu.draw_pikachu_volleyball_message(fc);
            self.menu.draw_subtitle(fc);
            self.menu.draw_with_who_messages(fc);
            self.frame_counter += 1;
        }

        if !cfg!(target_os = "android") && self.frame_counter < 71 && self.power_hit_pressed() {
            self.frame_counter = 71;
            return;
        }

        if self.frame_counter <= 71
            && !(cfg!(target_os = "android")
                && (self.power_hit_pressed() || self.touch_start_requested))
        {
            return;
        }

        let y_dir = if k0.y_direction != 0 {
            k0.y_direction
        } else {
            k1.y_direction
        };
        let y_edge = if self.menu_y_direction_previous == 0 {
            y_dir
        } else {
            0
        };
        self.menu_y_direction_previous = y_dir;

        if !cfg!(target_os = "android") && y_edge == -1 && self.selected_with_who > 0 {
            self.selected_with_who -= 1;
            self.menu.select_with_who(self.selected_with_who);
            self.audio.play(Snd::Pi, 0);
        } else if !cfg!(target_os = "android") && y_edge == 1 && self.selected_with_who < 2 {
            self.selected_with_who += 1;
            self.menu.select_with_who(self.selected_with_who);
            self.audio.play(Snd::Pi, 0);
        }

        if !cfg!(target_os = "android")
            && self.selected_with_who == 2
            && (k0.power_hit == 1 || k1.power_hit == 1)
        {
            self.cycle_skins_mode();
            return;
        }

        if k0.power_hit == 1 || k1.power_hit == 1 || self.touch_start_requested {
            self.game.set_player_skins_mode(self.menu.skins_mode());
            let touch_start = std::mem::take(&mut self.touch_start_requested);
            if cfg!(target_os = "android") {
                self.physics.player1.is_computer = self.android_right_handed;
                self.physics.player2.is_computer = !self.android_right_handed;
            } else if self.selected_with_who == 1 {
                self.physics.player1.is_computer = false;
                self.physics.player2.is_computer = false;
            } else if touch_start {
                self.physics.player1.is_computer = true;
                self.physics.player2.is_computer = false;
            } else if k0.power_hit == 1 {
                self.physics.player1.is_computer = false;
                self.physics.player2.is_computer = true;
            } else if k1.power_hit == 1 {
                self.physics.player1.is_computer = true;
                self.physics.player2.is_computer = false;
            }
            self.audio.play(Snd::Pikachu, 0);
            self.fade_menu_music = true;
            self.frame_counter = 0;
            self.no_input_frame_counter = 0;
            self.state = State::AfterMenuSelection;
            return;
        }

        if !cfg!(target_os = "android") && self.no_input_frame_counter >= NO_INPUT_FRAME_TOTAL_MENU
        {
            self.game.set_player_skins_mode(self.menu.skins_mode());
            self.audio.stop_bgm();
            self.fade_menu_music = false;
            self.physics.player1.is_computer = true;
            self.physics.player2.is_computer = true;
            self.frame_counter = 0;
            self.no_input_frame_counter = 0;
            self.state = State::AfterMenuSelection;
        }
    }

    /// Wygaszanie po wyborze w menu
    fn after_menu_selection(&mut self) {
        let duration = if self.fade_menu_music {
            MENU_MUSIC_FADE_FRAMES
        } else {
            FRAME_TOTAL_AFTER_MENU_SELECTION
        };
        self.fade.change_black_alpha_by(1.0 / duration as f64);
        self.frame_counter += 1;
        if self.fade_menu_music {
            self.audio
                .set_bgm_gain(menu_music_gain(duration - self.frame_counter));
        }
        if self.frame_counter >= duration {
            if self.fade_menu_music {
                self.audio.stop_bgm();
                self.fade_menu_music = false;
            }
            self.frame_counter = 0;
            self.state = State::BeforeStartOfNewGame;
        }
    }

    /// Opóźnienie przed nową grą (istnieje w oryginale)
    fn before_start_of_new_game(&mut self) {
        self.frame_counter += 1;
        if self.frame_counter >= FRAME_TOTAL_BEFORE_START_OF_NEW_GAME {
            self.frame_counter = 0;
            self.menu.set_visible(false);
            self.state = State::StartOfNewGame;
        }
    }

    /// Początek nowej gry: ustawienie piłki i graczy, napis „game start”
    fn start_of_new_game(&mut self) {
        if self.frame_counter == 0 {
            self.game.set_visible(true);
            self.game_ended = false;
            self.round_ended = false;
            self.is_player2_serve = false;
            self.physics.player1.game_ended = false;
            self.physics.player1.is_winner = false;
            self.physics.player2.game_ended = false;
            self.physics.player2.is_winner = false;

            self.scores[0] = 0;
            self.scores[1] = 0;
            self.game.update_scores(&self.scores);

            self.initialize_round();

            self.fade.set_black_alpha_to(1.0); // czarny ekran
            self.audio.play_bgm();
        }

        let frame_total = (FRAME_TOTAL_START_OF_NEW_GAME as f64 / GAME_START_SPEED).ceil() as i32;
        self.game
            .draw_game_start_message(self.frame_counter, frame_total);
        self.game.update_environment();
        self.fade.change_black_alpha_by(-(1.0 / 17.0)); // rozjaśnianie
        self.frame_counter += 1;

        if self.frame_counter >= frame_total {
            self.frame_counter = 0;
            self.fade.set_black_alpha_to(0.0);
            self.state = State::Round;
        }
    }

    fn initialize_round(&mut self) {
        self.physics.player1.initialize_for_new_round();
        self.physics.player2.initialize_for_new_round();
        self.physics
            .ball
            .initialize_for_new_round(self.is_player2_serve);
        self.game
            .update_players_and_ball(&mut self.physics, false, false);
        self.game.reset_round_visuals(); // bez interpolacji i smugi po teleportacji
    }

    /// Runda: gracze grają w siatkówkę
    fn round(&mut self) {
        let pressed_power_hit = self.power_hit_pressed();

        if self.physics.player1.is_computer && self.physics.player2.is_computer && pressed_power_hit
        {
            self.restart();
            return;
        }

        let mut inputs = [self.keyboard_array[0].input, self.keyboard_array[1].input];
        let is_ball_touching_ground = self.physics.run_engine_for_next_frame(&mut inputs);
        self.keyboard_array[0].input = inputs[0];
        self.keyboard_array[1].input = inputs[1];

        let power_hit = self.physics.ball.sound.power_hit;
        self.play_sound_effect();
        self.game
            .update_players_and_ball(&mut self.physics, is_ball_touching_ground, power_hit);
        self.game.update_environment();

        if self.game_ended {
            self.game.draw_game_end_message(self.frame_counter);
            self.frame_counter += 1;
            if self.frame_counter >= FRAME_TOTAL_GAME_END
                || (self.frame_counter >= 70 && pressed_power_hit)
            {
                self.restart();
            }
            return;
        }

        if is_ball_touching_ground && !self.round_ended && !self.game_ended {
            if self.physics.ball.punch_effect_x < GROUND_HALF_WIDTH {
                self.is_player2_serve = true;
                self.scores[1] += 1;
                if self.scores[1] >= self.winning_score {
                    self.game_ended = true;
                    self.physics.player1.is_winner = false;
                    self.physics.player2.is_winner = true;
                    self.physics.player1.game_ended = true;
                    self.physics.player2.game_ended = true;
                }
            } else {
                self.is_player2_serve = false;
                self.scores[0] += 1;
                if self.scores[0] >= self.winning_score {
                    self.game_ended = true;
                    self.physics.player1.is_winner = true;
                    self.physics.player2.is_winner = false;
                    self.physics.player1.game_ended = true;
                    self.physics.player2.game_ended = true;
                }
            }
            self.game.update_scores(&self.scores);
            if !self.round_ended && !self.game_ended {
                self.slow_motion_frames_left = self.slow_motion_frames_num;
            }
            self.round_ended = true;
        }

        if self.round_ended && !self.game_ended {
            // ostatnia klatka rundy – zaczynamy wygaszanie
            if self.slow_motion_frames_left == 0 {
                self.fade.change_black_alpha_by(1.0 / 16.0);
                self.state = State::AfterEndOfRound;
            }
        }
    }

    /// Wygaszanie po końcu rundy
    fn after_end_of_round(&mut self) {
        self.fade.change_black_alpha_by(1.0 / 16.0);
        self.frame_counter += 1;
        if self.frame_counter >= FRAME_TOTAL_AFTER_END_OF_ROUND {
            self.frame_counter = 0;
            self.state = State::BeforeStartOfNextRound;
        }
    }

    /// Przed następną rundą: ustawienie piłki i graczy, napis „Ready?”
    fn before_start_of_next_round(&mut self) {
        if self.frame_counter == 0 {
            self.fade.set_black_alpha_to(1.0);
            self.game.draw_ready_message(false);

            self.initialize_round();
        }

        self.game.update_environment();
        self.fade.change_black_alpha_by(-(1.0 / 16.0));

        self.frame_counter += 1;
        if self.frame_counter % 5 == 0 {
            self.game.toggle_ready_message();
        }

        if self.frame_counter >= FRAME_TOTAL_BEFORE_START_OF_NEXT_ROUND {
            self.frame_counter = 0;
            self.game.draw_ready_message(false);
            self.fade.set_black_alpha_to(0.0);
            self.round_ended = false;
            self.state = State::Round;
        }
    }

    /// Efekty dźwiękowe w trakcie rundy
    fn play_sound_effect(&mut self) {
        let (left_alt, right_alt) = self.game.skins_mode().alt_flags();
        for i in 0..2 {
            let player = if i == 0 {
                &mut self.physics.player1
            } else {
                &mut self.physics.player2
            };
            let alt = if i == 0 { left_alt } else { right_alt };
            let pan = if i == 0 { -1 } else { 1 };
            if player.sound.pipikachu {
                self.audio.play_alt(Snd::Pipikachu, pan, alt);
                player.sound.pipikachu = false;
            }
            if player.sound.pika {
                self.audio.play_alt(Snd::Pika, pan, alt);
                player.sound.pika = false;
            }
            if player.sound.chu {
                self.audio.play_alt(Snd::Chu, pan, alt);
                player.sound.chu = false;
            }
        }

        let ball = &mut self.physics.ball;
        let pan = (ball.punch_effect_x - GROUND_HALF_WIDTH).signum();
        if ball.sound.power_hit {
            self.audio.play(Snd::PowerHit, pan);
            ball.sound.power_hit = false;
        }
        if ball.sound.ball_touches_ground {
            self.audio.play(Snd::BallTouchesGround, pan);
            ball.sound.ball_touches_ground = false;
        }
    }

    // ---- sterowanie spoza gry (odpowiedniki przycisków z ui.js) ----

    /// Przycisk „restart”
    pub fn restart(&mut self) {
        self.frame_counter = 0;
        self.no_input_frame_counter = 0;
        self.touch_start_requested = false;
        self.fade_menu_music = false;
        self.slow_motion_frames_left = 0;
        self.slow_motion_num_of_skipped_frames = 0;
        self.set_paused(false);
        self.audio.stop_bgm();
        self.ticks_since_advance = 0;
        self.advance_interval = 1;
        self.fade.set_black_alpha_to(0.0);
        self.menu.set_visible(false);
        self.game.set_visible(false);
        self.game.set_player_skins_mode(self.menu.skins_mode());
        self.menu_y_direction_previous = 0;
        self.state = State::Menu;
    }

    pub fn cycle_skins_mode(&mut self) -> crate::view::PlayerSkinsMode {
        let next = self.menu.skins_mode().next();
        self.menu.set_skins_mode(next);
        self.game.set_player_skins_mode(next);
        self.audio.play(Snd::Pi, 0);
        next
    }

    pub fn update_enter_release(&mut self, enter_down: bool) {
        for keyboard in &mut self.keyboard_array {
            keyboard.update_enter_release(enter_down);
        }
    }

    pub fn suppress_fullscreen_enter(&mut self) {
        for keyboard in &mut self.keyboard_array {
            keyboard.suppress_fullscreen_enter();
        }
    }

    pub fn is_menu(&self) -> bool {
        matches!(self.state, State::Menu)
    }

    pub fn set_android_mode(&mut self, right_handed: bool) {
        self.android_right_handed = right_handed;
        self.menu.set_mobile_mode(right_handed);
    }

    pub fn request_touch_start(&mut self, two_players: bool) {
        self.selected_with_who = i32::from(two_players);
        self.menu.select_with_who(self.selected_with_who);
        self.touch_start_requested = true;
    }

    pub fn can_pause(&self) -> bool {
        matches!(
            self.state,
            State::StartOfNewGame
                | State::Round
                | State::AfterEndOfRound
                | State::BeforeStartOfNextRound
        )
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused && self.can_pause();
        self.audio.set_paused(self.paused);
    }

    pub fn audio_mut(&mut self) -> &mut PikaAudio {
        &mut self.audio
    }
}

#[cfg(test)]
mod menu_timing_tests {
    use super::*;

    #[test]
    fn demo_waits_30_seconds_and_music_fades_during_the_last_three() {
        assert_eq!(NO_INPUT_FRAME_TOTAL_MENU, 30 * 25);
        assert_eq!(menu_music_gain(NO_INPUT_FRAME_TOTAL_MENU), 1.0);
        assert_eq!(menu_music_gain(3 * 25), 1.0);
        assert!((menu_music_gain(2 * 25) - 4.0 / 9.0).abs() < 0.0001);
        assert_eq!(menu_music_gain(0), 0.0);
        assert_eq!(menu_music_gain(-25), 0.0);
    }

    #[test]
    fn player_selection_fades_for_three_seconds() {
        assert_eq!(MENU_MUSIC_FADE_FRAMES, 3 * 25);
        assert_eq!(menu_music_gain(MENU_MUSIC_FADE_FRAMES), 1.0);
        assert!(menu_music_gain(MENU_MUSIC_FADE_FRAMES - 1) < 1.0);
        assert_eq!(
            menu_music_gain(MENU_MUSIC_FADE_FRAMES - MENU_MUSIC_FADE_FRAMES),
            0.0
        );
    }

    #[test]
    fn visual_prediction_is_smooth_and_stays_within_court_and_ground() {
        let mut player = Player::new(false, false);
        player.x = 100;
        assert_eq!(predicted_player_position(&player, 1, 0.0), (100.0, 244.0));
        assert_eq!(predicted_player_position(&player, 1, 0.5), (103.0, 244.0));
        assert_eq!(predicted_player_position(&player, 1, 1.0), (106.0, 244.0));
        player.x = GROUND_HALF_WIDTH - PLAYER_HALF_LENGTH;
        assert_eq!(
            predicted_player_position(&player, 1, 1.0).0,
            player.x as f64
        );
        player.y = 240;
        player.y_velocity = 12;
        assert_eq!(predicted_player_position(&player, 0, 1.0).1, 244.0);
    }
}
