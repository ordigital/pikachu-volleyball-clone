//! Zasoby PNG/fonty i wspólne transformacje obrazów, tekstu i figur wektorowych.
use crate::text_label::TextLabel;
use macroquad::prelude::*;
use std::collections::HashMap;

enum Art {
    Image(Texture2D),
    Text {
        labels: Vec<TextLabel>,
        style: TextStyle,
    },
    Ball(Material),
    Shadow,
    Battle(TextLabel),
}

pub struct Assets {
    images: HashMap<&'static str, Texture2D>,
    labels: HashMap<String, TextLabel>,
    light: Material,
}

impl Assets {
    pub fn load() -> Self {
        let mut images = HashMap::new();
        for (name, bytes) in [
            (
                "players",
                include_bytes!("../assets/images/players.png").as_slice(),
            ),
            (
                "title_pikachu",
                include_bytes!("../assets/images/title_pikachu.png").as_slice(),
            ),
            (
                "cloud",
                include_bytes!("../assets/images/cloud.png").as_slice(),
            ),
            (
                "sitting_pikachu",
                include_bytes!("../assets/images/sitting_pikachu.png").as_slice(),
            ),
            (
                "menu_grass",
                include_bytes!("../assets/images/menu_grass.webp").as_slice(),
            ),
        ] {
            let texture = crate::texture::load(bytes);
            texture.set_filter(FilterMode::Linear);
            images.insert(name, texture);
        }
        let pixel =
            load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Jersey10-Regular.ttf"))
                .expect("Jersey 10 Regular");
        let title =
            load_ttf_font_from_bytes(include_bytes!("../assets/fonts/BalsamiqSans-Bold.ttf"))
                .expect("Balsamiq Sans 700");
        let outline = crate::text_label::outline_material();
        let mut labels = HashMap::new();
        for name in [
            "messages/ja/with_computer.png",
            "messages/ja/with_friend.png",
            "messages/ja/start_game.png",
            "messages/ja/mode_right.png",
            "messages/ja/mode_left.png",
            "messages/ja/quit.png",
            "messages/common/game_end.png",
            "messages/common/ready.png",
            "messages/ja/game_start.png",
            "messages/ja/pokemon.png",
        ] {
            let (text, _, _, is_title) = text_definition(name).unwrap();
            labels.insert(
                name.to_owned(),
                TextLabel::new(text, if is_title { &title } else { &pixel }, &outline),
            );
        }
        for n in 0..10 {
            let name = format!("number/number_{n}.png");
            labels.insert(
                name.clone(),
                TextLabel::new(text_definition(&name).unwrap().0, &pixel, &outline),
            );
        }
        labels.insert("battle".into(), TextLabel::new("BATTLE!", &title, &outline));
        Self {
            images,
            labels,
            light: crate::rendering::additive_light(),
        }
    }

    pub(crate) fn light(&self) -> Material {
        self.light.clone()
    }
    pub(crate) fn cloud_texture(&self) -> Texture2D {
        self.images["cloud"].clone()
    }

    pub(crate) fn menu_background(&self) -> Texture2D {
        self.images["menu_grass"].clone()
    }

    fn asset(&self, name: &str) -> (Art, Rect, (f32, f32)) {
        if let Some((_, size, color, title)) = text_definition(name) {
            return (
                Art::Text {
                    labels: vec![self.labels[name].clone()],
                    style: TextStyle {
                        color,
                        outline: if title { 0.045 } else { 0.0225 },
                        rotation: 0.0,
                    },
                },
                Rect::default(),
                size,
            );
        }
        let (art, size) = match name {
            "messages/ja/fight.png" => (Art::Battle(self.labels["battle"].clone()), (160.0, 160.0)),
            "ball/ball_0.png" => (Art::Ball(self.light.clone()), (40.0, 40.0)),
            "objects/shadow.png" => (Art::Shadow, (32.0, 8.0)),
            _ => {
                let (key, size, mut rect) = image_definition(name);
                let texture = self.images[key].clone();
                if key == "players" {
                    // Mapowanie klatek pozostaje w jednostkach oryginalnego atlasu.
                    let sx = texture.width() / 1484.0;
                    let sy = texture.height() / 832.0;
                    rect = Rect::new(rect.x * sx, rect.y * sy, rect.w * sx, rect.h * sy);
                } else {
                    rect = Rect::new(0.0, 0.0, texture.width(), texture.height());
                }
                return (Art::Image(texture), rect, size);
            }
        };
        (art, Rect::default(), size)
    }
}

fn image_definition(name: &str) -> (&'static str, (f32, f32), Rect) {
    if let Some(name) = name
        .strip_prefix("pikachu/pikachu_")
        .and_then(|n| n.strip_suffix(".png"))
    {
        let (state, frame) = name.split_once('_').expect("player frame");
        let index = crate::view::frame_number_for_player_sprite(
            state.parse().unwrap(),
            frame.parse().unwrap(),
        );
        return (
            "players",
            (64.0, 64.0),
            Rect::new(
                (index % 7 * 212 + 4) as f32,
                (index / 7 * 208 + 4) as f32,
                204.0,
                200.0,
            ),
        );
    }
    let (key, logical, pixels) = match name {
        "messages/ja/pikachu_volleyball.png" => ("title_pikachu", (276.0, 79.0), (908.0, 260.0)),
        "objects/cloud.png" => ("cloud", (48.0, 24.0), (143.0, 72.0)),
        "sitting_pikachu.png" => ("sitting_pikachu", (104.0, 104.0), (330.0, 320.0)),
        _ => panic!("Unknown asset: {name}"),
    };
    (key, logical, Rect::new(0.0, 0.0, pixels.0, pixels.1))
}

fn text_definition(name: &str) -> Option<(&'static str, (f32, f32), Color, bool)> {
    let orange = Color::from_rgba(255, 144, 0, 255);
    let blue = Color::from_rgba(82, 157, 255, 255);
    Some(match name {
        "messages/ja/with_computer.png" => ("1 PLAYER", (120.0, 20.0), orange, false),
        "messages/ja/with_friend.png" => ("2 PLAYERS", (120.0, 20.0), orange, false),
        "messages/ja/start_game.png" => ("START GAME", (120.0, 20.0), orange, false),
        "messages/ja/mode_right.png" => ("MODE: RIGHTHANDED", (185.0, 16.0), orange, false),
        "messages/ja/mode_left.png" => ("MODE: LEFTHANDED", (185.0, 16.0), orange, false),
        "messages/ja/quit.png" => ("QUIT", (75.0, 18.0), orange, false),
        "messages/common/game_end.png" => (
            "GAME OVER",
            (96.0, 24.0),
            Color::from_rgba(228, 229, 13, 255),
            false,
        ),
        "messages/common/ready.png" => (
            "READY?",
            (80.0, 24.0),
            Color::from_rgba(255, 61, 0, 255),
            false,
        ),
        "messages/ja/game_start.png" => (
            "START!",
            (96.0, 24.0),
            Color::from_rgba(255, 61, 0, 255),
            false,
        ),
        "messages/ja/pokemon.png" => ("BEACH VOLLEYBALL", (200.0, 32.0), blue, true),
        _ => {
            let digit = name.strip_prefix("number/number_")?.strip_suffix(".png")?;
            let label = match digit {
                "0" => "0",
                "1" => "1",
                "2" => "2",
                "3" => "3",
                "4" => "4",
                "5" => "5",
                "6" => "6",
                "7" => "7",
                "8" => "8",
                "9" => "9",
                _ => return None,
            };
            (label, (32.0, 32.0), Color::from_rgba(240, 5, 1, 255), false)
        }
    })
}

#[derive(Clone, Copy)]
pub(crate) struct TextStyle {
    pub color: Color,
    pub outline: f32,
    pub rotation: f32,
}

/// Odpowiednik `PIXI.Sprite` / `PIXI.AnimatedSprite`.
/// Wymiary `width`/`height` to aktualny rozmiar rysowania (jak `sprite.width` w Pixi),
/// `flip_x` odpowiada ujemnemu `scale.x`.
pub struct Sprite {
    frames: Vec<Rect>,
    art: Art,
    pub brightness: f32,
    /// rozmiar logiczny tekstury (pierwszej klatki) w pikselach gry
    base_w: f64,
    base_h: f64,
    pub frame: usize,
    pub x: f64,
    pub y: f64,
    /// pozycja z poprzedniego kroku symulacji (do interpolacji przy 60+ FPS)
    pub prev_x: f64,
    pub prev_y: f64,
    prev_width: f64,
    prev_height: f64,
    pub width: f64,
    pub height: f64,
    pub anchor_x: f64,
    pub anchor_y: f64,
    pub alpha: f64,
    prev_alpha: f64,
    pub rotation: f64,
    prev_rotation: f64,
    pub visible: bool,
    pub flip_x: bool,
}

impl Sprite {
    pub fn new(assets: &Assets, name: &str, anchor_x: f64, anchor_y: f64) -> Self {
        let (art, rect, size) = assets.asset(name);
        Self::from_frames(vec![rect], size, anchor_x, anchor_y, art)
    }

    pub fn animated(assets: &Assets, names: &[String], anchor_x: f64, anchor_y: f64) -> Self {
        let mut sprite = Self::new(assets, &names[0], anchor_x, anchor_y);
        sprite.frames = names.iter().map(|name| assets.asset(name).1).collect();
        if let Art::Text { labels, .. } = &mut sprite.art {
            *labels = names
                .iter()
                .map(|name| assets.labels[name].clone())
                .collect();
        }
        sprite
    }

    fn from_frames(
        frames: Vec<Rect>,
        size: (f32, f32),
        anchor_x: f64,
        anchor_y: f64,
        art: Art,
    ) -> Self {
        let w = size.0 as f64;
        let h = size.1 as f64;
        Self {
            frames,
            art,
            brightness: 1.0,
            base_w: w,
            base_h: h,
            frame: 0,
            x: 0.0,
            y: 0.0,
            prev_x: 0.0,
            prev_y: 0.0,
            prev_width: w,
            prev_height: h,
            width: w,
            height: h,
            anchor_x,
            anchor_y,
            alpha: 1.0,
            prev_alpha: 1.0,
            rotation: 0.0,
            prev_rotation: 0.0,
            visible: true,
            flip_x: false,
        }
    }

    pub fn base_height(&self) -> f64 {
        self.base_h
    }

    /// `AnimatedSprite.gotoAndStop`
    pub fn goto_and_stop(&mut self, frame: usize) {
        self.frame = frame;
    }

    /// Zapamiętuje bieżącą pozycję jako „poprzednią” (początek nowego kroku symulacji).
    /// Wołane też po „teleportacji” sprite'a, żeby nie był interpolowany.
    pub fn begin_tick(&mut self) {
        self.prev_x = self.x;
        self.prev_y = self.y;
        self.prev_width = self.width;
        self.prev_height = self.height;
        self.prev_alpha = self.alpha;
        self.prev_rotation = self.rotation;
    }

    /// Jeden współczynnik skali zachowuje proporcje niezależnie od rozmiaru atlasu.
    pub fn set_scale(&mut self, scale: f64) {
        self.width = self.base_w * scale;
        self.height = self.base_h * scale;
    }

    pub fn draw(&self, ox: f64, oy: f64, alpha_mul: f64) {
        self.draw_at(
            (self.x, self.y, self.width, self.height),
            (ox, oy),
            (self.alpha * alpha_mul, self.rotation),
        );
    }

    pub(crate) fn interpolated_transform(&self, t: f64) -> (f64, f64, f64, f64) {
        // Zawinięcie chmury / teleportacja nie mogą przejechać przez cały ekran.
        if (self.x - self.prev_x).abs() > 150.0 || (self.y - self.prev_y).abs() > 150.0 {
            return (self.x, self.y, self.width, self.height);
        }
        let t = t.clamp(0.0, 1.0);
        let lerp = |a, b| a + (b - a) * t;
        (
            lerp(self.prev_x, self.x),
            lerp(self.prev_y, self.y),
            lerp(self.prev_width, self.width),
            lerp(self.prev_height, self.height),
        )
    }

    /// Interpoluje pozycję i rozmiar; animacje klatkowe nadal działają przy 25 Hz.
    pub fn draw_lerp(&self, ox: f64, oy: f64, alpha_mul: f64, t: f64) {
        let t = t.clamp(0.0, 1.0);
        let alpha = self.prev_alpha + (self.alpha - self.prev_alpha) * t;
        self.draw_at(
            self.interpolated_transform(t),
            (ox, oy),
            (alpha * alpha_mul, self.interpolated_rotation(t)),
        );
    }

    fn interpolated_rotation(&self, t: f64) -> f64 {
        lerp_rotation(self.prev_rotation, self.rotation, t)
    }

    fn draw_at(
        &self,
        (x, y, width, height): (f64, f64, f64, f64),
        (ox, oy): (f64, f64),
        (alpha, rotation): (f64, f64),
    ) {
        let a = alpha.clamp(0.0, 1.0);
        if !self.visible || width <= 0.0 || height <= 0.0 || a <= 0.0 {
            return;
        }
        let left = (ox + x - self.anchor_x * width) as f32;
        let top = (oy + y - self.anchor_y * height) as f32;
        let size = vec2(width as f32, height as f32);
        let center = vec2(left, top) + size / 2.0;
        match &self.art {
            Art::Image(texture) => draw_texture_ex(
                texture,
                left,
                top,
                Color::new(1.0, 1.0, 1.0, a as f32),
                DrawTextureParams {
                    dest_size: Some(size),
                    source: Some(self.frames[self.frame]),
                    flip_x: self.flip_x,
                    rotation: rotation as f32,
                    ..Default::default()
                },
            ),
            Art::Text { labels, style } => {
                labels[self.frame].draw(vec2(left, top), size, *style, a as f32)
            }
            Art::Ball(light) => crate::ball_effects::draw_ball(
                center,
                size,
                rotation as f32,
                self.brightness,
                a as f32,
                light,
            ),
            Art::Battle(font) => crate::menu_badge::draw_battle(center, size, font, a as f32),
            Art::Shadow => crate::ball_effects::draw_shadow(center, size, a as f32),
        }
    }
}

pub(crate) fn lerp_rotation(from: f64, to: f64, t: f64) -> f64 {
    use std::f64::consts::{PI, TAU};
    let delta = (to - from + PI).rem_euclid(TAU) - PI;
    from + delta * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite() -> Sprite {
        Sprite::from_frames(
            vec![Rect::new(0.0, 0.0, 120.0, 20.0)],
            (120.0, 20.0),
            0.5,
            0.5,
            Art::Shadow,
        )
    }

    #[test]
    fn every_player_animation_maps_to_a_unique_cell_in_the_new_assets() {
        let mut cells = std::collections::HashSet::new();
        for (state, count) in [5, 5, 5, 2, 1, 5, 5].into_iter().enumerate() {
            for frame in 0..count {
                let name = format!("pikachu/pikachu_{state}_{frame}.png");
                let (key, size, rect) = image_definition(&name);
                assert_eq!(key, "players");
                assert_eq!(size, (64.0, 64.0));
                assert!(rect.x >= 4.0 && rect.y >= 4.0);
                assert!(rect.x + rect.w <= 1484.0 && rect.y + rect.h <= 832.0);
                assert!(cells.insert((rect.x as u32, rect.y as u32)));
            }
        }
        assert_eq!(cells.len(), 28);
    }

    #[test]
    fn scaling_and_interpolation_preserve_aspect_ratio() {
        let mut s = sprite();
        s.begin_tick();
        s.set_scale(1.2);
        s.x = 10.0;
        let (x, _, w, h) = s.interpolated_transform(0.5);
        assert_eq!(x, 5.0);
        assert_eq!((w, h), (132.0, 22.0));
        assert_eq!(w / h, s.base_w / s.base_h);
    }

    #[test]
    fn cloud_breathing_keeps_its_center_while_moving() {
        let mut s = sprite();
        s.x = 100.0;
        s.y = 50.0;
        s.width = 48.0;
        s.height = 24.0;
        s.begin_tick();
        // Ruch o 2 px oraz powiększenie chmury o 1 px z każdej strony.
        s.x += 1.0;
        s.y -= 1.0;
        s.width += 2.0;
        s.height += 2.0;
        let (x, y, w, h) = s.interpolated_transform(0.5);
        assert_eq!(x + w / 2.0, 125.0);
        assert_eq!(y + h / 2.0, 62.0);
    }

    #[test]
    fn rotation_crosses_wrap_without_reversing() {
        use std::f64::consts::TAU;
        let mut s = sprite();
        s.rotation = TAU * 0.98;
        s.begin_tick();
        s.rotation = TAU * 0.02;
        assert!((s.interpolated_rotation(0.5) - TAU).abs() < 1e-12);
        s.begin_tick();
        s.rotation = TAU * 0.98;
        assert!(s.interpolated_rotation(0.5).abs() < 1e-12);
    }

    #[test]
    fn wrapping_snaps_instead_of_crossing_the_screen() {
        let mut s = sprite();
        s.x = 432.0;
        s.begin_tick();
        s.x = -68.0;
        s.set_scale(1.2);
        assert_eq!(s.interpolated_transform(0.5), (-68.0, 0.0, 144.0, 24.0));
    }

    #[test]
    fn snapshot_prevents_interpolation_after_teleport() {
        let mut s = sprite();
        s.x = 50.0;
        s.set_scale(2.0);
        s.begin_tick();
        assert_eq!(s.interpolated_transform(0.0), (50.0, 0.0, 240.0, 40.0));
    }
}
