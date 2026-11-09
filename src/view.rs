//! Port `view.js` (część „View” z MVC).
//!
//! W oryginale PixiJS trzyma stan obiektów wyświetlanych (pozycja, alpha, widoczność…),
//! a renderer rysuje je co klatkę. Tu robimy to samo: metody `draw_*` zmieniają stan
//! sprite'ów (tak jak w JS), a `draw()` rysuje je w kolejności z drzewa kontenerów Pixi.

use crate::ball_effects::{particle_random, BallImpact, BallTrail, SandBurst};
use crate::clouds::{advance_clouds, Cloud, NUM_OF_CLOUDS};
use crate::gfx::{Assets, Sprite};
use crate::physics::PikaPhysics;
use crate::scene::{CENTER_X, HEIGHT, WIDTH};
use crate::water::Wave;
use macroquad::prelude::{draw_rectangle, vec2, Color, FilterMode, Texture2D};

pub const GAME_START_SPEED: f64 = 1.5;
const MENU_LEFT_X: f64 = CENTER_X - 108.0;
const MENU_RIGHT_X: f64 = CENTER_X + 52.0;
const MENU_LOGO_SCALE: f64 = 244.0 / 276.0;

// MenuView

pub struct MenuView {
    visible: bool,
    background: Texture2D,
    grass: crate::grass::GrassBackground,
    sitting_pikachu: Sprite,

    subtitle: Sprite,
    pikachu_volleyball: Sprite,
    with_who: [Sprite; 2],
    mobile_mode: [Sprite; 2],
    mobile_quit: Sprite,
    mobile_right_handed: bool,
    fight: Sprite,

    selected_with_who: i32,
    selected_with_who_message_size_increment: i32,
}

impl MenuView {
    pub fn new(assets: &Assets) -> Self {
        let mut sitting_pikachu = Sprite::new(assets, "sitting_pikachu.png", 0.5, 0.0);
        sitting_pikachu.x = MENU_LEFT_X;
        sitting_pikachu.y = 162.0;
        sitting_pikachu.set_scale(0.90);

        // FUN_00405b70
        let mut pikachu_volleyball =
            Sprite::new(assets, "messages/ja/pikachu_volleyball.png", 0.5, 0.0);
        pikachu_volleyball.x = MENU_RIGHT_X;
        pikachu_volleyball.y = 88.0;
        pikachu_volleyball.set_scale(MENU_LOGO_SCALE);
        let mut subtitle = Sprite::new(assets, "messages/ja/pokemon.png", 0.5, 0.0);
        subtitle.x = MENU_RIGHT_X;
        subtitle.y = 56.0;

        let mut view = Self {
            visible: false,
            background: assets.menu_background(),
            grass: crate::grass::GrassBackground::new(),
            sitting_pikachu,
            subtitle,
            pikachu_volleyball,
            with_who: [
                Sprite::new(
                    assets,
                    if cfg!(target_os = "android") {
                        "messages/ja/start_game.png"
                    } else {
                        "messages/ja/with_computer.png"
                    },
                    0.5,
                    0.5,
                ),
                Sprite::new(assets, "messages/ja/with_friend.png", 0.5, 0.5),
            ],
            mobile_mode: [
                Sprite::new(assets, "messages/ja/mode_right.png", 0.5, 0.5),
                Sprite::new(assets, "messages/ja/mode_left.png", 0.5, 0.5),
            ],
            mobile_quit: Sprite::new(assets, "messages/ja/quit.png", 0.5, 0.5),
            mobile_right_handed: true,
            fight: Sprite::new(assets, "messages/ja/fight.png", 0.5, 0.5),
            selected_with_who: -1,
            selected_with_who_message_size_increment: 2,
        };
        view.initialize_visibles();
        view
    }

    pub fn set_visible(&mut self, b: bool) {
        self.visible = b;
        if !b {
            self.initialize_visibles();
        }
    }

    fn initialize_visibles(&mut self) {
        self.sitting_pikachu.alpha = 0.0;
        self.sitting_pikachu.begin_tick();
        self.subtitle.visible = false;
        self.pikachu_volleyball.visible = false;
        self.fight.visible = false;
        for s in &mut self.with_who {
            s.visible = false;
        }
        for s in &mut self.mobile_mode {
            s.visible = false;
        }
        self.mobile_quit.visible = false;
    }

    /// FUN_00405d50 – napis „fight!” raz większy, raz mniejszy
    pub fn draw_fight_message(&mut self, frame_counter: i32) {
        const SIZE_ARRAY: [i32; 9] = [20, 22, 25, 27, 30, 27, 25, 22, 20];
        let fight = &mut self.fight;
        fight.visible = true;
        let scale = if frame_counter < 30 {
            frame_counter as f64 / 30.0
        } else {
            SIZE_ARRAY[((frame_counter + 1) % 9) as usize] as f64 / 30.0
        };
        fight.set_scale(scale * 0.8);
        fight.x = MENU_LEFT_X;
        fight.y = 96.0;
    }

    pub fn update_entrance_fade(&mut self, frame_counter: i32) {
        self.sitting_pikachu.visible = true;
        self.sitting_pikachu.alpha = ((frame_counter + 1) as f64 * 0.04).min(1.0);
    }

    /// FUN_00405b70
    pub fn draw_pikachu_volleyball_message(&mut self, frame_counter: i32) {
        let progress = ((frame_counter - 30) as f64 / 41.0).clamp(0.0, 1.0);
        let remaining = (1.0 - progress).powi(3);
        let m = &mut self.pikachu_volleyball;
        m.visible = frame_counter > 30;
        m.set_scale(MENU_LOGO_SCALE * (1.0 - remaining * 0.14));
        m.x = MENU_RIGHT_X + remaining * 80.0;
        m.y = 88.0;
    }

    pub fn draw_subtitle(&mut self, frame_counter: i32) {
        if frame_counter == 0 {
            self.subtitle.visible = false;
            return;
        }
        if frame_counter > 71 {
            self.subtitle.visible = true;
        }
    }

    /// Wybór graczy: proporcjonalne powiększenie wokół środka pod tytułem.
    pub fn draw_with_who_messages(&mut self, frame_counter: i32) {
        if frame_counter > 70 {
            self.selected_with_who_message_size_increment =
                (self.selected_with_who_message_size_increment + 1).min(10);
        }
        let center_x = MENU_RIGHT_X;
        for (i, s) in self.with_who.iter_mut().enumerate() {
            s.visible = frame_counter > 70 && (!cfg!(target_os = "android") || i == 0);
            let scale = if self.selected_with_who == i as i32 {
                1.0 + self.selected_with_who_message_size_increment as f64 * 0.02
            } else {
                1.0
            };
            s.set_scale(scale);
            s.x = center_x;
            s.y = if cfg!(target_os = "android") {
                184.0
            } else {
                184.0 + 36.0 * i as f64
            };
        }
        for (i, s) in self.mobile_mode.iter_mut().enumerate() {
            s.visible = cfg!(target_os = "android")
                && frame_counter > 70
                && (i == 0) == self.mobile_right_handed;
            s.x = center_x;
            s.y = 216.0;
        }
        self.mobile_quit.visible = cfg!(target_os = "android") && frame_counter > 70;
        self.mobile_quit.x = center_x;
        self.mobile_quit.y = 246.0;
    }

    pub fn set_mobile_mode(&mut self, right_handed: bool) {
        self.mobile_right_handed = right_handed;
    }

    pub fn begin_tick(&mut self) {
        for s in [
            &mut self.sitting_pikachu,
            &mut self.subtitle,
            &mut self.pikachu_volleyball,
            &mut self.fight,
        ] {
            s.begin_tick();
        }
        for s in &mut self.with_who {
            s.begin_tick();
        }
        for s in &mut self.mobile_mode {
            s.begin_tick();
        }
        self.mobile_quit.begin_tick();
    }

    /// Zaznacza opcję (0: z komputerem, 1: z kolegą) – zaznaczony napis rośnie.
    pub fn select_with_who(&mut self, i: i32) {
        self.selected_with_who = i;
        self.selected_with_who_message_size_increment = 2;
    }

    pub fn draw(&self, t: f64) {
        if !self.visible {
            return;
        }
        self.grass.draw(&self.background);
        self.sitting_pikachu.draw_lerp(0.0, 0.0, 1.0, t);
        for s in [
            &self.subtitle,
            &self.pikachu_volleyball,
            &self.with_who[0],
            &self.with_who[1],
            &self.mobile_mode[0],
            &self.mobile_mode[1],
            &self.mobile_quit,
            &self.fight,
        ] {
            s.draw_lerp(0.0, 0.0, 1.0, t);
        }
    }
}

// ---------------------------------------------------------------------------
// GameView
// ---------------------------------------------------------------------------

const SCORE_DIGIT_WIDTH: f64 = 32.0;
const SCORE_EDGE_MARGIN: f64 = 14.0;

// The left score grows inward; the right score is anchored to the right edge.
fn score_units_x(board_x: f64, left_side: bool, has_tens: bool) -> f64 {
    board_x
        + if left_side && !has_tens {
            0.0
        } else {
            SCORE_DIGIT_WIDTH
        }
}

struct ScoreBoard {
    x: f64,
    y: f64,
    units: Sprite, // dzieści: child 0, przesunięcie (32, 0)
    tens: Sprite,  // child 1, przesunięcie (0, 0)
}

#[derive(Default)]
struct PlayerSand {
    previous: [Option<(i32, i32)>; 2],
    cooldown: [u8; 2],
    bursts: Vec<SandBurst>,
}

impl PlayerSand {
    fn advance(&mut self, players: [&crate::physics::Player; 2]) {
        for burst in &mut self.bursts {
            burst.advance();
        }
        self.bursts.retain(|burst| !burst.finished());
        let ground = crate::physics::PLAYER_TOUCHING_GROUND_Y_COORD;
        for (i, player) in players.into_iter().enumerate() {
            self.cooldown[i] = self.cooldown[i].saturating_sub(1);
            if let Some((old_x, old_y)) = self.previous[i] {
                let landed = old_y < ground && player.y == ground;
                let walking = player.y == ground && player.state == 0 && player.x != old_x;
                let scale = if landed {
                    0.5
                } else if walking && self.cooldown[i] == 0 {
                    0.2
                } else {
                    0.0
                };
                if scale > 0.0 {
                    self.bursts.push(SandBurst::new(
                        vec2(player.x as f32, (ground + 32) as f32),
                        scale,
                    ));
                    self.cooldown[i] = 5;
                }
            }
            self.previous[i] = Some((player.x, player.y));
        }
    }

    fn draw(&self, t: f32) {
        for burst in &self.bursts {
            burst.draw(t);
        }
    }
}

fn game_over_scale(frame: i32) -> f64 {
    2.0 + 3.0 * (1.0 - frame as f64 / 50.0).max(0.0)
}

fn game_over_shake(frame: i32) -> (f64, f64) {
    if frame < 50 {
        return (0.0, 0.0);
    }
    let i = frame as u32;
    (
        (particle_random(i, 11) as f64 * 2.0 - 1.0) * 4.0,
        (particle_random(i, 29) as f64 * 2.0 - 1.0) * 4.0,
    )
}

pub struct GameView {
    visible: bool,

    bg_tex: Texture2D,
    cloud_sprites: Vec<Sprite>,
    cloud_renderer: crate::cloud_render::CloudRenderer,
    light: macroquad::material::Material,

    player_shadows: [Sprite; 2],
    shadow_for_ball: Sprite,
    players: [Sprite; 2],
    ball_trail: BallTrail,
    ball: Sprite,
    impact: BallImpact,
    player_sand: PlayerSand,
    game_over_trail: BallTrail,
    score_boards: [ScoreBoard; 2],

    game_start: Sprite,
    ready: Sprite,
    game_end: Sprite,

    cloud_array: Vec<Cloud>,
    wave: Wave,
}

impl GameView {
    pub fn new(assets: &Assets) -> Self {
        // --- tło ---
        let bg_bytes = include_bytes!("../assets/images/background-wide.webp");
        let bg_tex = crate::texture::load(bg_bytes);
        bg_tex.set_filter(FilterMode::Linear);

        // --- chmury i fala ---
        let cloud_sprites: Vec<Sprite> = (0..NUM_OF_CLOUDS)
            .map(|_| Sprite::new(assets, "objects/cloud.png", 0.0, 0.0))
            .collect();
        // --- gracze: 28 klatek (stany 0..6) ---
        let mut player_names: Vec<String> = Vec::new();
        for i in 0..7 {
            if i == 3 {
                player_names.push(format!("pikachu/pikachu_{}_{}.png", i, 0));
                player_names.push(format!("pikachu/pikachu_{}_{}.png", i, 1));
            } else if i == 4 {
                player_names.push(format!("pikachu/pikachu_{}_{}.png", i, 0));
            } else {
                for j in 0..5 {
                    player_names.push(format!("pikachu/pikachu_{}_{}.png", i, j));
                }
            }
        }
        let players = std::array::from_fn(|_| Sprite::animated(assets, &player_names, 0.5, 0.5));

        // --- piłka ---
        let ball = Sprite::new(assets, "ball/ball_0.png", 0.5, 0.5);
        let ball_trail = BallTrail::default();
        let impact = BallImpact::default();

        // --- tablice wyników ---
        let number_names: Vec<String> = (0..10)
            .map(|n| format!("number/number_{}.png", n))
            .collect();
        let make_board = |x: f64, y: f64| ScoreBoard {
            x,
            y,
            units: Sprite::animated(assets, &number_names, 0.0, 0.0),
            tens: Sprite::animated(assets, &number_names, 0.0, 0.0),
        };
        let score_boards = [
            make_board(SCORE_EDGE_MARGIN, 10.0),
            make_board(
                WIDTH as f64 - 2.0 * SCORE_DIGIT_WIDTH - SCORE_EDGE_MARGIN,
                10.0,
            ),
        ];

        let make_shadow = |width, height| {
            let mut shadow = Sprite::new(assets, "objects/shadow.png", 0.5, 0.5);
            shadow.y = 273.0;
            shadow.width = width;
            shadow.height = height;
            shadow
        };
        let player_shadows = std::array::from_fn(|_| make_shadow(52.0, 11.0));
        let shadow_for_ball = make_shadow(32.0, 8.0);

        let game_start = Sprite::new(assets, "messages/ja/game_start.png", 0.0, 0.0);
        let mut ready = Sprite::new(assets, "messages/common/ready.png", 0.0, 0.0);
        let game_end = Sprite::new(assets, "messages/common/game_end.png", 0.0, 0.0);
        ready.x = CENTER_X - ready.width / 2.0;
        ready.y = 38.0;

        let mut cloud_array = Vec::new();
        for _ in 0..NUM_OF_CLOUDS {
            cloud_array.push(Cloud::new());
        }

        let mut view = Self {
            visible: false,
            bg_tex,
            cloud_sprites,
            cloud_renderer: crate::cloud_render::CloudRenderer::new(assets.cloud_texture()),
            light: assets.light(),
            player_shadows,
            shadow_for_ball,
            players,
            ball_trail,
            ball,
            impact,
            player_sand: PlayerSand::default(),
            game_over_trail: BallTrail::default(),
            score_boards,
            game_start,
            ready,
            game_end,
            cloud_array,
            wave: Wave::new(),
        };
        view.initialize_visibles();
        view.sync_cloud_sprites();
        view.begin_tick();
        view
    }

    pub fn set_visible(&mut self, b: bool) {
        self.visible = b;
        if !b {
            self.initialize_visibles();
        }
    }

    fn initialize_visibles(&mut self) {
        self.game_start.visible = false;
        self.ready.visible = false;
        self.game_end.visible = false;
    }

    /// Updates player/ball visuals and feedback from the simulation step.
    pub fn update_players_and_ball(
        &mut self,
        physics: &mut PikaPhysics,
        ground: bool,
        power_hit: bool,
    ) {
        for (i, player) in [&physics.player1, &physics.player2].into_iter().enumerate() {
            let sprite = &mut self.players[i];
            sprite.x = player.x as f64;
            sprite.y = player.y as f64;
            sprite.flip_x = if player.state == 3 || player.state == 4 {
                if i == 0 {
                    player.diving_direction == -1
                } else {
                    player.diving_direction != 1
                }
            } else {
                player.is_player2
            };
            let frame = frame_number_for_player_sprite(player.state, player.frame_number);
            sprite.goto_and_stop(frame);
            self.player_shadows[i].x = sprite.x;
        }
        self.player_sand
            .advance([&physics.player1, &physics.player2]);
        let ball = &mut physics.ball;

        self.ball.x = ball.x as f64;
        self.ball.y = ball.y as f64;
        self.shadow_for_ball.x = ball.x as f64;
        // 50 jednostek fine_rotation odpowiada dawnym pięciu klatkom pełnego obrotu.
        self.ball.rotation = ball.fine_rotation as f64 / 50.0 * std::f64::consts::TAU;
        self.ball.brightness = if ball.is_power_hit { 1.3 } else { 1.0 };

        let contacts = [
            physics.player1.is_collision_with_ball_happened,
            physics.player2.is_collision_with_ball_happened,
        ];
        let ground = ground.then(|| vec2(ball.punch_effect_x as f32, ball.punch_effect_y as f32));
        let squash = self
            .impact
            .advance(contacts, ball.world_bounce, power_hit, ground);
        self.ball.width = 40.0 * squash.x as f64;
        self.ball.height = 40.0 * squash.y as f64;
        ball.punch_effect_radius = (ball.punch_effect_radius - 2).max(0);

        self.ball_trail.advance(
            vec2(ball.x as f32, ball.y as f32),
            self.ball.rotation as f32,
            ball.is_power_hit,
        );
    }

    /// Początek kroku symulacji: obecne pozycje stają się „poprzednimi”.
    /// Wołane po zmianie stanu „teleportującej” sprite'y (nowa runda) działa jak snap.
    pub fn begin_tick(&mut self) {
        self.game_start.begin_tick();
        self.game_end.begin_tick();
        for s in self.players.iter_mut().chain(&mut self.player_shadows) {
            s.begin_tick();
        }
        self.shadow_for_ball.begin_tick();
        self.ball.begin_tick();
        for c in self.cloud_sprites.iter_mut() {
            c.begin_tick();
        }
    }

    pub fn reset_round_visuals(&mut self) {
        self.ball_trail.clear();
        self.impact = BallImpact::default();
        self.player_sand = PlayerSand::default();
        self.game_over_trail.clear();
        self.ball.width = 40.0;
        self.ball.height = 40.0;
        self.begin_tick();
    }

    pub fn update_scores(&mut self, scores: &[i32; 2]) {
        for (&score, board) in scores.iter().zip(&mut self.score_boards) {
            board.units.goto_and_stop((score % 10) as usize);
            board.tens.goto_and_stop(((score / 10) % 10) as usize);
            board.tens.visible = score >= 10;
        }
    }

    pub fn update_environment(&mut self) {
        advance_clouds(&mut self.cloud_array);
        self.wave.advance();

        self.sync_cloud_sprites();
    }

    fn sync_cloud_sprites(&mut self) {
        for (cloud, s) in self.cloud_array.iter().zip(&mut self.cloud_sprites) {
            s.x = cloud.sprite_top_left_point_x();
            s.y = cloud.sprite_top_left_point_y();
            s.width = cloud.sprite_width();
            s.height = cloud.sprite_height();
        }
    }

    /// FUN_00403f20 – napis „game start” rośnie
    pub fn draw_game_start_message(&mut self, frame_counter: i32, frame_total: i32) {
        if frame_counter == 0 {
            self.game_start.visible = true;
        } else if frame_counter >= frame_total - 1 {
            self.game_start.visible = false;
            return;
        }

        let m = &mut self.game_start;
        m.set_scale(frame_counter as f64 * GAME_START_SPEED / 25.0);
        m.x = CENTER_X - m.width / 2.0;
        m.y = 50.0 + m.height;
        if frame_counter == 0 {
            m.begin_tick();
        }
    }

    pub fn draw_ready_message(&mut self, b: bool) {
        self.ready.visible = b;
    }

    pub fn toggle_ready_message(&mut self) {
        self.ready.visible = !self.ready.visible;
    }

    /// FUN_00404070 – napis „game end”
    pub fn draw_game_end_message(&mut self, frame_counter: i32) {
        let m = &mut self.game_end;
        m.visible = true;
        m.set_scale(game_over_scale(frame_counter));
        let (dx, dy) = game_over_shake(frame_counter);
        m.x = CENTER_X - m.width / 2.0 + dx;
        m.y = 50.0 - (m.height - m.base_height()) / 2.0 + dy;
        self.game_over_trail.advance(
            vec2((m.x + m.width / 2.0) as f32, (m.y + m.height / 2.0) as f32),
            0.0,
            frame_counter >= 50,
        );
        if frame_counter == 0 {
            m.begin_tick();
        }
    }

    /// `t` ∈ [0, 1] – ułamek kroku symulacji (25 Hz), do płynnego rysowania przy 60+ FPS.
    pub fn draw(&self, t: f64, prediction: Option<(usize, f64, f64)>) {
        if !self.visible {
            return;
        }
        // kolejność dodawania do kontenera w oryginale = kolejność rysowania
        crate::background::draw(&self.bg_tex);
        self.cloud_renderer.draw(&self.cloud_sprites, t);
        self.wave.draw(t);
        self.player_sand.draw(t as f32);
        for (i, shadow) in self.player_shadows.iter().enumerate() {
            let ox = prediction
                .filter(|p| p.0 == i)
                .map_or(0.0, |p| p.1 - shadow.interpolated_transform(t).0);
            shadow.draw_lerp(ox, 0.0, 1.0, t);
        }
        self.shadow_for_ball.draw_lerp(0.0, 0.0, 1.0, t);
        for (i, player) in self.players.iter().enumerate() {
            let (ox, oy) = prediction.filter(|p| p.0 == i).map_or((0.0, 0.0), |p| {
                let (x, y, _, _) = player.interpolated_transform(t);
                (p.1 - x, p.2 - y)
            });
            player.draw_lerp(ox, oy, 1.0, t);
        }
        let (x, y, _, _) = self.ball.interpolated_transform(t);
        self.ball_trail
            .draw(vec2(x as f32, y as f32), 20.0, t as f32, &self.light);
        self.ball.draw_lerp(0.0, 0.0, 1.0, t);
        self.impact.draw_sand(t as f32);
        for (i, b) in self.score_boards.iter().enumerate() {
            b.units
                .draw(score_units_x(b.x, i == 0, b.tens.visible), b.y, 1.0);
            b.tens.draw(b.x, b.y, 1.0);
        }
        self.game_start.draw_lerp(0.0, 0.0, 1.0, t);
        self.ready.draw(0.0, 0.0, 1.0);
        if self.game_end.visible {
            let (x, y, w, h) = self.game_end.interpolated_transform(t);
            self.game_over_trail
                .draw_smoke_cloud(vec2((x + w / 2.0) as f32, (y + h / 2.0) as f32), t as f32);
        }
        self.game_end.draw_lerp(0.0, 0.0, 1.0, t);
    }
}

/// Numer klatki animacji gracza dla danego stanu (stany 0-2: po 5 klatek,
/// stan 3: 2 klatki, stan 4: 1 klatka, stany 5-6: po 5 klatek).
pub(crate) fn frame_number_for_player_sprite(state: i32, frame_number: i32) -> usize {
    let n = if state < 4 {
        5 * state + frame_number
    } else if state == 4 {
        17 + frame_number
    } else {
        18 + 5 * (state - 5) + frame_number
    };
    n as usize
}

// ---------------------------------------------------------------------------
// FadeInOut
// ---------------------------------------------------------------------------

pub struct FadeInOut {
    alpha: f64,
}

impl FadeInOut {
    pub fn new() -> Self {
        Self { alpha: 0.0 }
    }

    pub fn set_black_alpha_to(&mut self, alpha: f64) {
        self.alpha = alpha.clamp(0.0, 1.0);
    }

    /// alpha_increment > 0: ściemnianie, < 0: rozjaśnianie
    pub fn change_black_alpha_by(&mut self, alpha_increment: f64) {
        self.set_black_alpha_to(self.alpha + alpha_increment);
    }

    pub fn draw(&self) {
        if self.alpha > 0.0 {
            // Jednolity fade całego obszaru gry, bez tekstury i krawędzi atlasu.
            draw_rectangle(
                0.0,
                0.0,
                WIDTH,
                HEIGHT,
                Color::new(0.0, 0.0, 0.0, self.alpha as f32),
            );
        }
    }
}

#[cfg(test)]
mod score_layout_tests {
    use super::*;

    #[test]
    fn player_sand_requires_real_walking_or_landing() {
        use crate::physics::{Player, PLAYER_TOUCHING_GROUND_Y_COORD as GROUND};
        let mut effects = PlayerSand::default();
        let mut player = Player::new(false, false);
        let other = Player::new(true, false);
        effects.advance([&player, &other]);
        effects.advance([&player, &other]);
        assert!(effects.bursts.is_empty());
        player.x += 3;
        effects.advance([&player, &other]);
        assert_eq!(effects.bursts.len(), 1);
        assert_eq!(effects.bursts[0].scale(), 0.2);
        player.y = GROUND - 10;
        player.state = 1;
        effects.advance([&player, &other]);
        player.y = GROUND;
        player.state = 0;
        effects.advance([&player, &other]);
        assert_eq!(effects.bursts.len(), 2);
        assert_eq!(effects.bursts[1].scale(), 0.5);
        for _ in 0..12 {
            effects.advance([&player, &other]);
        }
        assert!(effects.bursts.is_empty());
    }

    #[test]
    fn game_over_finishes_double_size_and_shakes_in_both_axes() {
        assert_eq!(game_over_scale(0), 5.0);
        assert_eq!(game_over_scale(50), 2.0);
        assert_eq!(game_over_scale(200), 2.0);
        assert_eq!(game_over_shake(49), (0.0, 0.0));
        let offsets: Vec<_> = (50..100).map(game_over_shake).collect();
        assert!(offsets.iter().any(|(x, _)| *x < 0.0));
        assert!(offsets.iter().any(|(x, _)| *x > 0.0));
        assert!(offsets.iter().any(|(_, y)| *y < 0.0));
        assert!(offsets.iter().any(|(_, y)| *y > 0.0));
    }

    #[test]
    fn score_edges_remain_symmetric_with_one_or_two_digits() {
        let left_board = SCORE_EDGE_MARGIN;
        let right_board = WIDTH as f64 - 2.0 * SCORE_DIGIT_WIDTH - SCORE_EDGE_MARGIN;
        for left_has_tens in [false, true] {
            for right_has_tens in [false, true] {
                let left_edge = if left_has_tens {
                    left_board
                } else {
                    score_units_x(left_board, true, false)
                };
                let right_edge =
                    score_units_x(right_board, false, right_has_tens) + SCORE_DIGIT_WIDTH;
                assert_eq!(left_edge, WIDTH as f64 - right_edge);
                if left_has_tens {
                    assert_eq!(
                        score_units_x(left_board, true, true),
                        left_board + SCORE_DIGIT_WIDTH
                    );
                }
                if right_has_tens {
                    assert_eq!(
                        score_units_x(right_board, false, true),
                        right_board + SCORE_DIGIT_WIDTH
                    );
                }
            }
        }
    }
}
