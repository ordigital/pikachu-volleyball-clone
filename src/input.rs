//! Konfiguracja sterowania: przypisania klawiszy i przycisków pada dla obu graczy,
//! zapis/odczyt z pliku tekstowego.

use macroquad::prelude::KeyCode;

/// Klawisze, które można przypisać (nazwa służy też do zapisu w pliku).
/// Klawisze F1–F12, P i Escape są zarezerwowane dla skrótów programu.
pub const KEY_NAMES: &[(KeyCode, &str)] = &[
    (KeyCode::A, "A"),
    (KeyCode::B, "B"),
    (KeyCode::C, "C"),
    (KeyCode::D, "D"),
    (KeyCode::E, "E"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::I, "I"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::M, "M"),
    (KeyCode::N, "N"),
    (KeyCode::O, "O"),
    (KeyCode::Q, "Q"),
    (KeyCode::R, "R"),
    (KeyCode::S, "S"),
    (KeyCode::T, "T"),
    (KeyCode::U, "U"),
    (KeyCode::V, "V"),
    (KeyCode::W, "W"),
    (KeyCode::X, "X"),
    (KeyCode::Y, "Y"),
    (KeyCode::Z, "Z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::Space, "Space"),
    (KeyCode::Enter, "Enter"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::Left, "Left"),
    (KeyCode::Right, "Right"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"),
    (KeyCode::LeftShift, "L-Shift"),
    (KeyCode::RightShift, "R-Shift"),
    (KeyCode::LeftControl, "L-Ctrl"),
    (KeyCode::RightControl, "R-Ctrl"),
    (KeyCode::LeftAlt, "L-Alt"),
    (KeyCode::RightAlt, "R-Alt"),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Slash, "/"),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Minus, "Minus"),
    (KeyCode::Equal, "="),
    (KeyCode::Backslash, "\\"),
    (KeyCode::Insert, "Insert"),
    (KeyCode::Delete, "Delete"),
    (KeyCode::Home, "Home"),
    (KeyCode::End, "End"),
    (KeyCode::PageUp, "PgUp"),
    (KeyCode::PageDown, "PgDn"),
    (KeyCode::Kp0, "Num0"),
    (KeyCode::Kp1, "Num1"),
    (KeyCode::Kp2, "Num2"),
    (KeyCode::Kp3, "Num3"),
    (KeyCode::Kp4, "Num4"),
    (KeyCode::Kp5, "Num5"),
    (KeyCode::Kp6, "Num6"),
    (KeyCode::Kp7, "Num7"),
    (KeyCode::Kp8, "Num8"),
    (KeyCode::Kp9, "Num9"),
    (KeyCode::KpEnter, "NumEnter"),
    (KeyCode::KpAdd, "Num+"),
    (KeyCode::KpSubtract, "Num-"),
    (KeyCode::KpMultiply, "Num*"),
    (KeyCode::KpDivide, "Num/"),
];

pub fn key_name(k: KeyCode) -> Option<&'static str> {
    KEY_NAMES.iter().find(|(c, _)| *c == k).map(|(_, n)| *n)
}

pub fn key_from_name(name: &str) -> Option<KeyCode> {
    KEY_NAMES.iter().find(|(_, n)| *n == name).map(|(c, _)| *c)
}

/// Przyciski pada (niezależne od biblioteki gilrs).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadBtn {
    South,
    East,
    North,
    West,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    LeftThumb,
    RightThumb,
}

pub const ALL_PAD_BTNS: [PadBtn; 16] = [
    PadBtn::South,
    PadBtn::East,
    PadBtn::North,
    PadBtn::West,
    PadBtn::LeftTrigger,
    PadBtn::LeftTrigger2,
    PadBtn::RightTrigger,
    PadBtn::RightTrigger2,
    PadBtn::Select,
    PadBtn::Start,
    PadBtn::DPadUp,
    PadBtn::DPadDown,
    PadBtn::DPadLeft,
    PadBtn::DPadRight,
    PadBtn::LeftThumb,
    PadBtn::RightThumb,
];

impl PadBtn {
    /// Nazwa do wyświetlania i zapisu w pliku.
    pub fn label(self) -> &'static str {
        match self {
            PadBtn::South => "Pad A",
            PadBtn::East => "Pad B",
            PadBtn::North => "Pad Y",
            PadBtn::West => "Pad X",
            PadBtn::LeftTrigger => "LB",
            PadBtn::LeftTrigger2 => "LT",
            PadBtn::RightTrigger => "RB",
            PadBtn::RightTrigger2 => "RT",
            PadBtn::Select => "Select",
            PadBtn::Start => "Start",
            PadBtn::DPadUp => "D-Up",
            PadBtn::DPadDown => "D-Down",
            PadBtn::DPadLeft => "D-Left",
            PadBtn::DPadRight => "D-Right",
            PadBtn::LeftThumb => "L3",
            PadBtn::RightThumb => "R3",
        }
    }
    pub fn from_label(s: &str) -> Option<PadBtn> {
        ALL_PAD_BTNS.iter().copied().find(|b| b.label() == s)
    }
}

/// Akcje gracza (indeks w `PlayerControls::actions`).
#[derive(Clone, Copy)]
pub enum Action {
    Left = 0,
    Right = 1,
    Up = 2,
    Down = 3,
    PowerHit = 4,
}

pub const NUM_ACTIONS: usize = 5;
pub const ACTION_LABELS: [&str; NUM_ACTIONS] =
    ["Left", "Right", "Up (jump)", "Down", "Power hit / OK"];
const ACTION_IDS: [&str; NUM_ACTIONS] = ["left", "right", "up", "down", "power"];

#[derive(Clone, Copy, Default)]
pub struct Binding {
    pub key: Option<KeyCode>,
    pub pad: Option<PadBtn>,
}

#[derive(Clone)]
pub struct PlayerControls {
    pub actions: [Binding; NUM_ACTIONS],
    /// Który pad (0..3) steruje tym graczem; `None` – żaden.
    pub pad_index: Option<usize>,
}

#[derive(Clone)]
pub struct Controls {
    pub players: [PlayerControls; 2],
}

fn b(key: Option<KeyCode>, pad: Option<PadBtn>) -> Binding {
    Binding { key, pad }
}

impl Default for Controls {
    /// Domyślne sterowanie jak w oryginale.
    fn default() -> Self {
        let p1 = PlayerControls {
            actions: [
                b(Some(KeyCode::D), Some(PadBtn::DPadLeft)),
                b(Some(KeyCode::G), Some(PadBtn::DPadRight)),
                b(Some(KeyCode::R), Some(PadBtn::DPadUp)),
                b(Some(KeyCode::F), Some(PadBtn::DPadDown)),
                b(Some(KeyCode::Z), Some(PadBtn::South)),
            ],
            pad_index: Some(0),
        };
        let p2 = PlayerControls {
            actions: [
                b(Some(KeyCode::Left), Some(PadBtn::DPadLeft)),
                b(Some(KeyCode::Right), Some(PadBtn::DPadRight)),
                b(Some(KeyCode::Up), Some(PadBtn::DPadUp)),
                b(Some(KeyCode::Down), Some(PadBtn::DPadDown)),
                b(Some(KeyCode::Enter), Some(PadBtn::South)),
            ],
            pad_index: Some(1),
        };
        Controls { players: [p1, p2] }
    }
}

fn config_directory(
    appimage: Option<std::path::PathBuf>,
    executable: Option<std::path::PathBuf>,
) -> std::path::PathBuf {
    appimage
        .or(executable)
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn config_path() -> std::path::PathBuf {
    let appimage = std::env::var_os("APPIMAGE").map(std::path::PathBuf::from);
    config_directory(appimage, std::env::current_exe().ok()).join("controls.txt")
}

impl Controls {
    pub fn load() -> Self {
        let mut c = Controls::default();
        if let Ok(text) = std::fs::read_to_string(config_path()) {
            c.apply_text(&text);
        }
        c
    }

    pub fn save(&self) {
        if let Err(error) = std::fs::write(config_path(), self.to_text()) {
            eprintln!("Could not save controls: {error}");
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.players.iter().enumerate() {
            let n = i + 1;
            match p.pad_index {
                Some(v) => out.push_str(&format!("p{n}.pad={v}\n")),
                None => out.push_str(&format!("p{n}.pad=-\n")),
            }
            for (id, bd) in ACTION_IDS.iter().zip(&p.actions) {
                let k = bd.key.and_then(key_name).unwrap_or("-");
                let pd = bd.pad.map(|x| x.label()).unwrap_or("-");
                out.push_str(&format!("p{n}.{id}={k}|{pd}\n"));
            }
        }
        out
    }

    fn apply_text(&mut self, text: &str) {
        for line in text.lines() {
            let (name, value) = match line.split_once('=') {
                Some(x) => x,
                None => continue,
            };
            let (pl, what) = match name.split_once('.') {
                Some(x) => x,
                None => continue,
            };
            let pi = match pl {
                "p1" => 0,
                "p2" => 1,
                _ => continue,
            };
            if what == "pad" {
                self.players[pi].pad_index = value.trim().parse::<usize>().ok().filter(|v| *v < 4);
                continue;
            }
            if let Some(ai) = ACTION_IDS.iter().position(|id| *id == what) {
                let (k, pd) = value.split_once('|').unwrap_or((value, "-"));
                self.players[pi].actions[ai] = Binding {
                    key: key_from_name(k.trim()),
                    pad: PadBtn::from_label(pd.trim()),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_round_trip_without_confusing_minus_and_unbound() {
        let mut original = Controls::default();
        original.players[0].actions[0] = Binding::default();
        original.players[1].actions[1].key = Some(KeyCode::Minus);
        original.players[1].pad_index = None;
        let mut restored = Controls::default();
        restored.apply_text(&original.to_text());
        for (before, after) in original.players.iter().zip(&restored.players) {
            assert_eq!(before.pad_index, after.pad_index);
            for (before, after) in before.actions.iter().zip(&after.actions) {
                assert_eq!(before.key, after.key);
                assert_eq!(before.pad, after.pad);
            }
        }
        assert_eq!(key_from_name("-"), None);
        for &(key, name) in KEY_NAMES {
            assert_eq!(key_from_name(name), Some(key));
            assert_eq!(key_name(key), Some(name));
        }
    }

    #[test]
    fn config_uses_appimage_location_instead_of_read_only_mount() {
        use std::path::PathBuf;
        assert_eq!(
            config_directory(
                Some(PathBuf::from("/home/player/Game.AppImage")),
                Some(PathBuf::from("/tmp/.mount_game/usr/bin/game"))
            ),
            PathBuf::from("/home/player")
        );
        assert_eq!(
            config_directory(None, Some(PathBuf::from("/opt/game/game"))),
            PathBuf::from("/opt/game")
        );
    }

    #[test]
    fn obsolete_binding_is_ignored_without_losing_saved_controls() {
        let mut controls = Controls::default();
        controls.apply_text("p1.downright=F|-\np1.left=A|Pad X\np2.power=Space|Pad B\n");
        assert_eq!(
            controls.players[0].actions[Action::Left as usize].key,
            Some(KeyCode::A)
        );
        assert_eq!(
            controls.players[1].actions[Action::PowerHit as usize].key,
            Some(KeyCode::Space)
        );
        let saved = controls.to_text();
        assert!(!saved.contains("downright"));
        let mut restored = Controls::default();
        restored.apply_text(&saved);
        assert_eq!(restored.to_text(), saved);
    }

    #[test]
    fn default_controls_round_trip() {
        let original = Controls::default().to_text();
        let mut restored = Controls::default();
        restored.apply_text(&original);
        assert_eq!(restored.to_text(), original);
    }
}
