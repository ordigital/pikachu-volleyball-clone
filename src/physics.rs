//! Port fizyki i AI z `src/resources/js/physics.js`
//! (gorisanson/pikachu-volleyball, rekonstrukcja oryginalnej gry z 1997 r.).
//!
//! Wszystko jest liczone na liczbach całkowitych, tak jak w oryginale
//! (dzielenie `(x / 3) | 0` w JS == dzielenie całkowite `x / 3` w Rust).
//!
//! Współrzędne: X w [0, GROUND_WIDTH] rosnące w prawo, Y w [0, scene::HEIGHT] rosnące w dół.

pub const GROUND_WIDTH: i32 = crate::scene::WIDTH as i32;
pub const GROUND_HALF_WIDTH: i32 = GROUND_WIDTH / 2; // to także x słupka siatki
const PLAYER_LENGTH: i32 = 64;
pub(crate) const PLAYER_HALF_LENGTH: i32 = PLAYER_LENGTH / 2;
pub(crate) const PLAYER_MOVE_SPEED: i32 = 6;
pub(crate) const PLAYER_TOUCHING_GROUND_Y_COORD: i32 = 244;
const BALL_RADIUS: i32 = 20;
const BALL_TOUCHING_GROUND_Y_COORD: i32 = 252;
const NET_PILLAR_HALF_WIDTH: i32 = 25;
const NET_PILLAR_TOP_TOP_Y_COORD: i32 = 176;
const NET_PILLAR_TOP_BOTTOM_Y_COORD: i32 = 192;
/// Zabezpieczenie dodane w wersji JS (oryginał nie ogranicza pętli).
const INFINITE_LOOP_LIMIT: i32 = 1000;

use crate::rand::rand;

// ---------------------------------------------------------------------------
// Typy
// ---------------------------------------------------------------------------

/// Wejście gracza (klawiatura lub AI).
#[derive(Clone, Copy, Default, Debug)]
pub struct PikaUserInput {
    /// -1 lewo, 0 brak, 1 prawo
    pub x_direction: i32,
    /// -1 góra, 0 brak, 1 dół
    pub y_direction: i32,
    /// 0 lub 1 (bez auto-repeat)
    pub power_hit: i32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct PlayerSound {
    pub pipikachu: bool,
    pub pika: bool,
    pub chu: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct BallSound {
    pub power_hit: bool,
    pub ball_touches_ground: bool,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub is_player2: bool,
    pub is_computer: bool,
    pub x: i32,
    pub y: i32,
    pub y_velocity: i32,
    pub is_collision_with_ball_happened: bool,
    /// 0 normal, 1 jumping, 2 jumping_and_power_hitting, 3 diving,
    /// 4 lying_down_after_diving, 5 win, 6 lost
    pub state: i32,
    pub frame_number: i32,
    pub normal_status_arm_swing_direction: i32,
    pub delay_before_next_frame: i32,
    /// 0..=4 – im większe, tym śmielszy komputer
    pub computer_boldness: i32,
    pub diving_direction: i32,
    pub lying_down_duration_left: i32,
    pub is_winner: bool,
    pub game_ended: bool,
    pub computer_where_to_stand_by: i32,
    pub sound: PlayerSound,
}

impl Player {
    pub fn new(is_player2: bool, is_computer: bool) -> Self {
        let mut p = Player {
            is_player2,
            is_computer,
            x: 0,
            y: 0,
            y_velocity: 0,
            is_collision_with_ball_happened: false,
            state: 0,
            frame_number: 0,
            normal_status_arm_swing_direction: 1,
            delay_before_next_frame: 0,
            computer_boldness: 0,
            diving_direction: 0,
            lying_down_duration_left: -1,
            is_winner: false,
            game_ended: false,
            computer_where_to_stand_by: 0,
            sound: PlayerSound::default(),
        };
        p.initialize_for_new_round();
        // jak w JS: te pola ustawiane są po initializeForNewRound
        p.diving_direction = 0;
        p.lying_down_duration_left = -1;
        p.is_winner = false;
        p.game_ended = false;
        p.computer_where_to_stand_by = 0;
        p
    }

    pub fn initialize_for_new_round(&mut self) {
        self.x = if self.is_player2 {
            GROUND_WIDTH - 36
        } else {
            36
        };
        self.y = PLAYER_TOUCHING_GROUND_Y_COORD;
        self.y_velocity = 0;
        self.is_collision_with_ball_happened = false;
        self.state = 0;
        self.frame_number = 0;
        self.normal_status_arm_swing_direction = 1;
        self.delay_before_next_frame = 0;
        self.computer_boldness = rand() % 5;
    }
}

#[derive(Clone, Debug)]
pub struct Ball {
    pub x: i32,
    pub y: i32,
    pub x_velocity: i32,
    pub y_velocity: i32,
    pub punch_effect_radius: i32,
    pub is_power_hit: bool,
    /// Visual-only notification when the ball bounces off the world this tick.
    pub world_bounce: bool,
    pub expected_landing_point_x: i32,
    /// 0..=5 (5 == "hyper ball" glitch)
    pub rotation: i32,
    pub fine_rotation: i32,
    pub punch_effect_x: i32,
    pub punch_effect_y: i32,
    pub previous_x: i32,
    pub previous_previous_x: i32,
    pub previous_y: i32,
    pub previous_previous_y: i32,
    pub sound: BallSound,
}

impl Ball {
    pub fn new(is_player2_serve: bool) -> Self {
        let mut b = Ball {
            x: 0,
            y: 0,
            x_velocity: 0,
            y_velocity: 1,
            punch_effect_radius: 0,
            is_power_hit: false,
            world_bounce: false,
            expected_landing_point_x: 0,
            rotation: 0,
            fine_rotation: 0,
            punch_effect_x: 0,
            punch_effect_y: 0,
            previous_x: 0,
            previous_previous_x: 0,
            previous_y: 0,
            previous_previous_y: 0,
            sound: BallSound::default(),
        };
        b.initialize_for_new_round(is_player2_serve);
        b
    }

    pub fn initialize_for_new_round(&mut self, is_player2_serve: bool) {
        self.x = if is_player2_serve {
            GROUND_WIDTH - 56
        } else {
            56
        };
        self.y = 0;
        self.x_velocity = 0;
        self.y_velocity = 1;
        self.punch_effect_radius = 0;
        self.is_power_hit = false;
        self.world_bounce = false;
    }
}

pub struct PikaPhysics {
    pub player1: Player,
    pub player2: Player,
    pub ball: Ball,
}

impl PikaPhysics {
    pub fn new(is_player1_computer: bool, is_player2_computer: bool) -> Self {
        let player1 = Player::new(false, is_player1_computer);
        let player2 = Player::new(true, is_player2_computer);
        let ball = Ball::new(false);
        Self {
            player1,
            player2,
            ball,
        }
    }

    /// Jedna klatka fizyki. Zwraca `true`, jeśli piłka dotknęła ziemi.
    /// `input[0]` – gracz 1, `input[1]` – gracz 2 (AI nadpisuje wejście).
    pub fn run_engine_for_next_frame(&mut self, input: &mut [PikaUserInput; 2]) -> bool {
        physics_engine(&mut self.player1, &mut self.player2, &mut self.ball, input)
    }
}

// ---------------------------------------------------------------------------
// Silnik (FUN_00403dd0)
// ---------------------------------------------------------------------------

fn physics_engine(
    player1: &mut Player,
    player2: &mut Player,
    ball: &mut Ball,
    user_input: &mut [PikaUserInput; 2],
) -> bool {
    let is_ball_touching_ground = process_collision_between_ball_and_world(ball);

    for i in 0..2 {
        calculate_expected_landing_point_x_for(ball);
        if i == 0 {
            process_player_movement(player1, &mut user_input[0], player2, ball);
        } else {
            process_player_movement(player2, &mut user_input[1], player1, ball);
        }
    }

    for (i, input) in user_input.iter().enumerate() {
        let player = if i == 0 { &mut *player1 } else { &mut *player2 };
        if is_collision_between_ball_and_player(ball, player.x, player.y) {
            if !player.is_collision_with_ball_happened {
                process_collision_between_ball_and_player(ball, player.x, input, player.state);
                player.is_collision_with_ball_happened = true;
            }
        } else {
            player.is_collision_with_ball_happened = false;
        }
    }

    is_ball_touching_ground
}

/// FUN_00403070
fn is_collision_between_ball_and_player(ball: &Ball, player_x: i32, player_y: i32) -> bool {
    (ball.x - player_x).abs() <= PLAYER_HALF_LENGTH
        && (ball.y - player_y).abs() <= PLAYER_HALF_LENGTH
}

/// FUN_00402dc0
fn process_collision_between_ball_and_world(ball: &mut Ball) -> bool {
    let initial_velocity = (ball.x_velocity, ball.y_velocity);
    ball.world_bounce = false;
    ball.previous_previous_x = ball.previous_x;
    ball.previous_previous_y = ball.previous_y;
    ball.previous_x = ball.x;
    ball.previous_y = ball.y;

    let mut future_fine_rotation = ball.fine_rotation + ball.x_velocity / 2;
    // przy == 50 powstaje słynny glitch "hyper ball" (rotation = 5)
    if future_fine_rotation < 0 {
        future_fine_rotation += 50;
    } else if future_fine_rotation > 50 {
        future_fine_rotation -= 50;
    }
    ball.fine_rotation = future_fine_rotation;
    ball.rotation = ball.fine_rotation / 10;

    let future_ball_x = ball.x + ball.x_velocity;
    if !(BALL_RADIUS..=GROUND_WIDTH).contains(&future_ball_x) {
        ball.x_velocity = -ball.x_velocity;
    }

    let future_ball_y = ball.y + ball.y_velocity;
    if future_ball_y < 0 {
        ball.y_velocity = 1;
    }

    // siatka
    if (ball.x - GROUND_HALF_WIDTH).abs() < NET_PILLAR_HALF_WIDTH
        && ball.y > NET_PILLAR_TOP_TOP_Y_COORD
    {
        if ball.y <= NET_PILLAR_TOP_BOTTOM_Y_COORD {
            if ball.y_velocity > 0 {
                ball.y_velocity = -ball.y_velocity;
            }
        } else if ball.x < GROUND_HALF_WIDTH {
            ball.x_velocity = -ball.x_velocity.abs();
        } else {
            ball.x_velocity = ball.x_velocity.abs();
        }
    }

    ball.world_bounce = (ball.x_velocity, ball.y_velocity) != initial_velocity;
    let future_ball_y = ball.y + ball.y_velocity;
    if future_ball_y > BALL_TOUCHING_GROUND_Y_COORD {
        ball.sound.ball_touches_ground = true;
        ball.world_bounce = true;
        ball.y_velocity = -ball.y_velocity;
        ball.punch_effect_x = ball.x;
        ball.y = BALL_TOUCHING_GROUND_Y_COORD;
        ball.punch_effect_radius = BALL_RADIUS;
        ball.punch_effect_y = BALL_TOUCHING_GROUND_Y_COORD + BALL_RADIUS;
        return true;
    }

    ball.y = future_ball_y;
    ball.x += ball.x_velocity;
    ball.y_velocity += 1;
    false
}

/// FUN_00401fc0
fn process_player_movement(
    player: &mut Player,
    user_input: &mut PikaUserInput,
    the_other_player: &Player,
    ball: &Ball,
) {
    if player.is_computer {
        let_computer_decide_user_input(player, ball, the_other_player, user_input);
    }

    // leży po nurkowaniu – nie rusza się
    if player.state == 4 {
        player.lying_down_duration_left -= 1;
        if player.lying_down_duration_left < -1 {
            player.state = 0;
        }
        return;
    }

    // Animacja chodzenia zależy od faktycznego przesunięcia, także przy ścianie.
    let previous_player_x = player.x;
    // ruch w poziomie
    let mut player_velocity_x = 0;
    if player.state < 5 {
        if player.state < 3 {
            player_velocity_x = user_input.x_direction * PLAYER_MOVE_SPEED;
        } else {
            player_velocity_x = player.diving_direction * 8;
        }
    }

    let future_player_x = player.x + player_velocity_x;
    player.x = if player.is_player2 {
        future_player_x.clamp(
            GROUND_HALF_WIDTH + PLAYER_HALF_LENGTH,
            GROUND_WIDTH - PLAYER_HALF_LENGTH,
        )
    } else {
        future_player_x.clamp(PLAYER_HALF_LENGTH, GROUND_HALF_WIDTH - PLAYER_HALF_LENGTH)
    };

    // skok
    if player.state < 3
        && user_input.y_direction == -1
        && player.y == PLAYER_TOUCHING_GROUND_Y_COORD
    {
        player.y_velocity = -16;
        player.state = 1;
        player.frame_number = 0;
        player.sound.chu = true;
    }

    // grawitacja
    let future_player_y = player.y + player.y_velocity;
    player.y = future_player_y;
    if future_player_y < PLAYER_TOUCHING_GROUND_Y_COORD {
        player.y_velocity += 1;
    } else if future_player_y > PLAYER_TOUCHING_GROUND_Y_COORD {
        player.y_velocity = 0;
        player.y = PLAYER_TOUCHING_GROUND_Y_COORD;
        player.frame_number = 0;
        if player.state == 3 {
            player.state = 4;
            player.frame_number = 0;
            player.lying_down_duration_left = 3;
        } else {
            player.state = 0;
        }
    }

    if user_input.power_hit == 1 {
        if player.state == 1 {
            // smash
            player.delay_before_next_frame = 5;
            player.frame_number = 0;
            player.state = 2;
            player.sound.pika = true;
        } else if player.state == 0 && user_input.x_direction != 0 {
            // nurkowanie
            player.state = 3;
            player.frame_number = 0;
            player.diving_direction = user_input.x_direction;
            player.y_velocity = -5;
            player.sound.chu = true;
        }
    }

    // animacja
    if player.state == 1 {
        player.frame_number = (player.frame_number + 1) % 3;
    } else if player.state == 2 {
        if player.delay_before_next_frame < 1 {
            player.frame_number += 1;
            if player.frame_number > 4 {
                player.frame_number = 0;
                player.state = 1;
            }
        } else {
            player.delay_before_next_frame -= 1;
        }
    } else if player.state == 0 && player.x == previous_player_x {
        player.frame_number = 0;
        player.delay_before_next_frame = 0;
        player.normal_status_arm_swing_direction = 1;
    } else if player.state == 0 {
        player.delay_before_next_frame += 1;
        if player.delay_before_next_frame > 3 {
            player.delay_before_next_frame = 0;
            let future_frame_number =
                player.frame_number + player.normal_status_arm_swing_direction;
            if !(0..=4).contains(&future_frame_number) {
                player.normal_status_arm_swing_direction =
                    -player.normal_status_arm_swing_direction;
            }
            player.frame_number += player.normal_status_arm_swing_direction;
        }
    }

    if player.game_ended {
        if player.state == 0 {
            if player.is_winner {
                player.state = 5;
                player.sound.pipikachu = true;
            } else {
                player.state = 6;
            }
            player.delay_before_next_frame = 0;
            player.frame_number = 0;
        }
        process_game_end_frame_for(player);
    }
}

/// FUN_004025e0
fn process_game_end_frame_for(player: &mut Player) {
    if player.game_ended && player.frame_number < 4 {
        player.delay_before_next_frame += 1;
        if player.delay_before_next_frame > 4 {
            player.delay_before_next_frame = 0;
            player.frame_number += 1;
        }
    }
}

/// FUN_004030a0 – ustawia tylko prędkość piłki i przewidywany punkt lądowania.
fn process_collision_between_ball_and_player(
    ball: &mut Ball,
    player_x: i32,
    user_input: &PikaUserInput,
    player_state: i32,
) {
    if ball.x < player_x {
        ball.x_velocity = -((ball.x - player_x).abs() / 3);
    } else if ball.x > player_x {
        ball.x_velocity = (ball.x - player_x).abs() / 3;
    }

    if ball.x_velocity == 0 {
        ball.x_velocity = (rand() % 3) - 1;
    }

    let ball_abs_y_velocity = ball.y_velocity.abs();
    ball.y_velocity = -ball_abs_y_velocity;
    if ball_abs_y_velocity < 15 {
        ball.y_velocity = -15;
    }

    if player_state == 2 {
        // smash
        if ball.x < GROUND_HALF_WIDTH {
            ball.x_velocity = (user_input.x_direction.abs() + 1) * 10;
        } else {
            ball.x_velocity = -(user_input.x_direction.abs() + 1) * 10;
        }
        ball.punch_effect_x = ball.x;
        ball.punch_effect_y = ball.y;
        ball.y_velocity = ball.y_velocity.abs() * user_input.y_direction * 2;
        ball.punch_effect_radius = BALL_RADIUS;
        ball.sound.power_hit = true;
        ball.is_power_hit = true;
    } else {
        ball.is_power_hit = false;
    }

    calculate_expected_landing_point_x_for(ball);
}

#[derive(Clone, Copy)]
struct CopyBall {
    x: i32,
    y: i32,
    x_velocity: i32,
    y_velocity: i32,
}

/// FUN_004031b0
fn calculate_expected_landing_point_x_for(ball: &mut Ball) {
    let mut c = CopyBall {
        x: ball.x,
        y: ball.y,
        x_velocity: ball.x_velocity,
        y_velocity: ball.y_velocity,
    };
    let mut loop_counter = 0;
    loop {
        loop_counter += 1;

        let future_x = c.x_velocity + c.x;
        if !(BALL_RADIUS..=GROUND_WIDTH).contains(&future_x) {
            c.x_velocity = -c.x_velocity;
        }
        if c.y + c.y_velocity < 0 {
            c.y_velocity = 1;
        }

        if (c.x - GROUND_HALF_WIDTH).abs() < NET_PILLAR_HALF_WIDTH
            && c.y > NET_PILLAR_TOP_TOP_Y_COORD
        {
            // (w oryginale tu jest `<`, nie `<=` jak w FUN_00402dc0)
            if c.y < NET_PILLAR_TOP_BOTTOM_Y_COORD {
                if c.y_velocity > 0 {
                    c.y_velocity = -c.y_velocity;
                }
            } else if c.x < GROUND_HALF_WIDTH {
                c.x_velocity = -c.x_velocity.abs();
            } else {
                c.x_velocity = c.x_velocity.abs();
            }
        }

        c.y += c.y_velocity;

        if c.y > BALL_TOUCHING_GROUND_Y_COORD || loop_counter >= INFINITE_LOOP_LIMIT {
            break;
        }
        c.x += c.x_velocity;
        c.y_velocity += 1;
    }
    ball.expected_landing_point_x = c.x;
}

/// FUN_00402360 – AI sterujące graczem komputerowym.
fn let_computer_decide_user_input(
    player: &mut Player,
    ball: &Ball,
    the_other_player: &Player,
    user_input: &mut PikaUserInput,
) {
    user_input.x_direction = 0;
    user_input.y_direction = 0;
    user_input.power_hit = 0;

    let p2 = player.is_player2 as i32;
    let mut virtual_expected_landing_point_x = ball.expected_landing_point_x;

    if (ball.x - player.x).abs() > 100 && ball.x_velocity.abs() < player.computer_boldness + 5 {
        let left_boundary = p2 * GROUND_HALF_WIDTH;
        if (ball.expected_landing_point_x <= left_boundary
            || ball.expected_landing_point_x >= p2 * GROUND_WIDTH + GROUND_HALF_WIDTH)
            && player.computer_where_to_stand_by == 0
        {
            virtual_expected_landing_point_x = left_boundary + GROUND_HALF_WIDTH / 2;
        }
    }

    if (virtual_expected_landing_point_x - player.x).abs() > player.computer_boldness + 8 {
        if player.x < virtual_expected_landing_point_x {
            user_input.x_direction = 1;
        } else {
            user_input.x_direction = -1;
        }
    } else if rand() % 20 == 0 {
        player.computer_where_to_stand_by = rand() % 2;
    }

    if player.state == 0 {
        if ball.x_velocity.abs() < player.computer_boldness + 3
            && (ball.x - player.x).abs() < PLAYER_HALF_LENGTH
            && ball.y > -36
            && ball.y < 10 * player.computer_boldness + 84
            && ball.y_velocity > 0
        {
            user_input.y_direction = -1;
        }

        let left_boundary = p2 * GROUND_HALF_WIDTH;
        let right_boundary = (p2 + 1) * GROUND_HALF_WIDTH;
        if ball.expected_landing_point_x > left_boundary
            && ball.expected_landing_point_x < right_boundary
            && (ball.x - player.x).abs() > player.computer_boldness * 5 + PLAYER_LENGTH
            && ball.x > left_boundary
            && ball.x < right_boundary
            && ball.y > 174
        {
            // komputer decyduje się nurkować
            user_input.power_hit = 1;
            if player.x < ball.x {
                user_input.x_direction = 1;
            } else {
                user_input.x_direction = -1;
            }
        }
    } else if player.state == 1 || player.state == 2 {
        if (ball.x - player.x).abs() > 8 {
            if player.x < ball.x {
                user_input.x_direction = 1;
            } else {
                user_input.x_direction = -1;
            }
        }
        if (ball.x - player.x).abs() < 48 && (ball.y - player.y).abs() < 48 {
            let will_input_power_hit =
                decide_whether_input_power_hit(player, ball, the_other_player, user_input);
            if will_input_power_hit {
                user_input.power_hit = 1;
                if (the_other_player.x - player.x).abs() < 80 && user_input.y_direction != -1 {
                    user_input.y_direction = -1;
                }
            }
        }
    }
}

/// FUN_00402630
fn decide_whether_input_power_hit(
    player: &Player,
    ball: &Ball,
    the_other_player: &Player,
    user_input: &mut PikaUserInput,
) -> bool {
    let p2 = player.is_player2 as i32;

    let check = |x_direction: i32, y_direction: i32, user_input: &mut PikaUserInput| -> bool {
        let landing = expected_landing_point_x_when_power_hit(x_direction, y_direction, ball);
        if (landing <= p2 * GROUND_HALF_WIDTH || landing >= p2 * GROUND_WIDTH + GROUND_HALF_WIDTH)
            && (landing - the_other_player.x).abs() > PLAYER_LENGTH
        {
            user_input.x_direction = x_direction;
            user_input.y_direction = y_direction;
            return true;
        }
        false
    };

    if rand() % 2 == 0 {
        for x_direction in [1, 0] {
            for y_direction in [-1, 0, 1] {
                if check(x_direction, y_direction, user_input) {
                    return true;
                }
            }
        }
    } else {
        for x_direction in [1, 0] {
            for y_direction in [1, 0, -1] {
                if check(x_direction, y_direction, user_input) {
                    return true;
                }
            }
        }
    }
    false
}

/// FUN_00402870
fn expected_landing_point_x_when_power_hit(
    user_input_x_direction: i32,
    user_input_y_direction: i32,
    ball: &Ball,
) -> i32 {
    let mut c = CopyBall {
        x: ball.x,
        y: ball.y,
        x_velocity: ball.x_velocity,
        y_velocity: ball.y_velocity,
    };

    if c.x < GROUND_HALF_WIDTH {
        c.x_velocity = (user_input_x_direction.abs() + 1) * 10;
    } else {
        c.x_velocity = -(user_input_x_direction.abs() + 1) * 10;
    }
    c.y_velocity = c.y_velocity.abs() * user_input_y_direction * 2;

    let mut loop_counter = 0;
    loop {
        loop_counter += 1;

        let future_x = c.x + c.x_velocity;
        if !(BALL_RADIUS..=GROUND_WIDTH).contains(&future_x) {
            c.x_velocity = -c.x_velocity;
        }
        if c.y + c.y_velocity < 0 {
            c.y_velocity = 1;
        }

        if (c.x - GROUND_HALF_WIDTH).abs() < NET_PILLAR_HALF_WIDTH
            && c.y > NET_PILLAR_TOP_TOP_Y_COORD
        {
            // Celowo (jak w oryginale) komputer nie przewiduje odbicia od słupka:
            // czasem uderza piłkę, która odbije się od siatki.
            if c.y_velocity > 0 {
                c.y_velocity = -c.y_velocity;
            }
        }

        c.y += c.y_velocity;

        if c.y > BALL_TOUCHING_GROUND_Y_COORD || loop_counter >= INFINITE_LOOP_LIMIT {
            return c.x;
        }
        c.x += c.x_velocity;
        c.y_velocity += 1;
    }
}

// ---------------------------------------------------------------------------
// Testy
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ball_falls_and_hits_ground() {
        let mut ph = PikaPhysics::new(false, false);
        let mut inp = [PikaUserInput::default(); 2];
        let mut touched = false;
        for _ in 0..200 {
            if ph.run_engine_for_next_frame(&mut inp) {
                touched = true;
                break;
            }
        }
        assert!(touched);
        assert_eq!(ph.ball.y, BALL_TOUCHING_GROUND_Y_COORD);
    }

    #[test]
    fn walking_animates_only_while_the_player_actually_moves() {
        let mut player = Player::new(false, false);
        let other = Player::new(true, false);
        let ball = Ball::new(false);
        let mut input = PikaUserInput::default();
        player.frame_number = 3;
        for _ in 0..20 {
            process_player_movement(&mut player, &mut input, &other, &ball);
            assert_eq!(player.frame_number, 0);
        }
        input.x_direction = 1;
        for _ in 0..8 {
            process_player_movement(&mut player, &mut input, &other, &ball);
        }
        assert!(player.frame_number > 0);
        input.x_direction = 0;
        let stopped_x = player.x;
        for _ in 0..20 {
            process_player_movement(&mut player, &mut input, &other, &ball);
            assert_eq!(player.frame_number, 0);
            assert_eq!(player.x, stopped_x);
        }
    }

    #[test]
    fn pressing_against_a_boundary_does_not_animate_walking() {
        let mut player = Player::new(false, false);
        let other = Player::new(true, false);
        let ball = Ball::new(false);
        player.x = GROUND_HALF_WIDTH - PLAYER_HALF_LENGTH;
        let mut input = PikaUserInput {
            x_direction: 1,
            ..Default::default()
        };
        for _ in 0..20 {
            process_player_movement(&mut player, &mut input, &other, &ball);
            assert_eq!(player.frame_number, 0);
        }
        input.y_direction = -1;
        process_player_movement(&mut player, &mut input, &other, &ball);
        assert_eq!(player.state, 1); // Skok nadal ma własną animację.
        assert_eq!(player.frame_number, 1);
    }

    #[test]
    fn world_bounce_notification_tracks_actual_collisions() {
        let mut ball = Ball::new(false);
        ball.x = 100;
        ball.y = 80;
        ball.x_velocity = 1;
        ball.y_velocity = 1;
        assert!(!process_collision_between_ball_and_world(&mut ball));
        assert!(!ball.world_bounce);

        ball.x = BALL_RADIUS;
        ball.x_velocity = -2;
        ball.y = 80;
        ball.y_velocity = 0;
        assert!(!process_collision_between_ball_and_world(&mut ball));
        assert!(ball.world_bounce);

        ball.x = 100;
        ball.y = 0;
        ball.x_velocity = 0;
        ball.y_velocity = -2;
        assert!(!process_collision_between_ball_and_world(&mut ball));
        assert!(ball.world_bounce);

        ball.x = GROUND_HALF_WIDTH;
        ball.y = NET_PILLAR_TOP_TOP_Y_COORD + 2;
        ball.x_velocity = 0;
        ball.y_velocity = 4;
        assert!(!process_collision_between_ball_and_world(&mut ball));
        assert!(ball.world_bounce);

        ball.x = 100;
        ball.y = BALL_TOUCHING_GROUND_Y_COORD;
        ball.x_velocity = 0;
        ball.y_velocity = 2;
        assert!(process_collision_between_ball_and_world(&mut ball));
        assert!(ball.world_bounce);
        ball.initialize_for_new_round(false);
        assert!(!ball.world_bounce);
    }

    #[test]
    fn widened_court_keeps_players_in_their_halves_and_ball_reaches_new_wall() {
        let ball = Ball::new(false);
        for second in [false, true] {
            let mut player = Player::new(second, false);
            let other = Player::new(!second, false);
            for direction in [-1, 1] {
                let mut input = PikaUserInput {
                    x_direction: direction,
                    ..Default::default()
                };
                for _ in 0..200 {
                    process_player_movement(&mut player, &mut input, &other, &ball);
                }
                let expected = match (second, direction) {
                    (false, -1) => PLAYER_HALF_LENGTH,
                    (false, 1) => GROUND_HALF_WIDTH - PLAYER_HALF_LENGTH,
                    (true, -1) => GROUND_HALF_WIDTH + PLAYER_HALF_LENGTH,
                    _ => GROUND_WIDTH - PLAYER_HALF_LENGTH,
                };
                assert_eq!(player.x, expected);
            }
        }
        let mut ball = Ball::new(false);
        ball.x = 440; // beyond the old right edge, now inside the court
        ball.y = 80;
        ball.x_velocity = 10;
        ball.y_velocity = 0;
        process_collision_between_ball_and_world(&mut ball);
        assert_eq!(ball.x, 450);
        assert_eq!(ball.x_velocity, 10);
        ball.x = GROUND_WIDTH - 1;
        process_collision_between_ball_and_world(&mut ball);
        assert!(ball.x < GROUND_WIDTH && ball.x > 432);
        assert_eq!(ball.x_velocity, -10);
    }

    #[test]
    fn ai_vs_ai_runs_without_panic() {
        let mut ph = PikaPhysics::new(true, true);
        let mut inp = [PikaUserInput::default(); 2];
        for _ in 0..5000 {
            ph.run_engine_for_next_frame(&mut inp);
        }
    }
}
