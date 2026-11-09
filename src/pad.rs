//! Obsługa padów przez gilrs na desktopie i Android InputDevice na telefonie.

#[cfg(target_os = "android")]
pub use imp::take_back;
pub use imp::Pads;

#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
mod imp {
    use crate::input::{PadBtn, ALL_PAD_BTNS};
    use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};

    fn to_gilrs(b: PadBtn) -> Button {
        match b {
            PadBtn::South => Button::South,
            PadBtn::East => Button::East,
            PadBtn::North => Button::North,
            PadBtn::West => Button::West,
            PadBtn::LeftTrigger => Button::LeftTrigger,
            PadBtn::LeftTrigger2 => Button::LeftTrigger2,
            PadBtn::RightTrigger => Button::RightTrigger,
            PadBtn::RightTrigger2 => Button::RightTrigger2,
            PadBtn::Select => Button::Select,
            PadBtn::Start => Button::Start,
            PadBtn::DPadUp => Button::DPadUp,
            PadBtn::DPadDown => Button::DPadDown,
            PadBtn::DPadLeft => Button::DPadLeft,
            PadBtn::DPadRight => Button::DPadRight,
            PadBtn::LeftThumb => Button::LeftThumb,
            PadBtn::RightThumb => Button::RightThumb,
        }
    }

    fn from_gilrs(b: Button) -> Option<PadBtn> {
        ALL_PAD_BTNS.into_iter().find(|&p| to_gilrs(p) == b)
    }

    pub struct Pads {
        gilrs: Option<Gilrs>,
        ids: Vec<GamepadId>,
        pressed: Option<PadBtn>,
    }

    impl Pads {
        pub fn new() -> Self {
            Self {
                gilrs: Gilrs::new().ok(),
                ids: Vec::new(),
                pressed: None,
            }
        }

        /// Wołać raz na klatkę renderowania.
        pub fn update(&mut self) {
            self.pressed = None;
            if let Some(g) = self.gilrs.as_mut() {
                while let Some(ev) = g.next_event() {
                    if let EventType::ButtonPressed(btn, _) = ev.event {
                        if let Some(p) = from_gilrs(btn) {
                            self.pressed = Some(p);
                        }
                    }
                }
                let mut ids: Vec<GamepadId> = g
                    .gamepads()
                    .filter(|(_, gp)| gp.is_connected())
                    .map(|(id, _)| id)
                    .collect();
                ids.sort_by_key(|id| usize::from(*id));
                self.ids = ids;
            }
        }

        /// Liczba podłączonych padów.
        pub fn count(&self) -> usize {
            self.ids.len()
        }

        /// Przycisk wciśnięty w tej klatce (do przypisywania klawiszy).
        pub fn pressed(&self) -> Option<PadBtn> {
            self.pressed
        }

        pub fn is_down(&self, index: usize, btn: PadBtn) -> bool {
            let (g, id) = match (self.gilrs.as_ref(), self.ids.get(index)) {
                (Some(g), Some(id)) => (g, *id),
                _ => return false,
            };
            g.gamepad(id).is_pressed(to_gilrs(btn))
        }

        /// Lewa gałka (x w prawo dodatnie, y w dół dodatnie).
        pub fn stick(&self, index: usize) -> (f32, f32) {
            let (g, id) = match (self.gilrs.as_ref(), self.ids.get(index)) {
                (Some(g), Some(id)) => (g, *id),
                _ => return (0.0, 0.0),
            };
            let gp = g.gamepad(id);
            (gp.value(Axis::LeftStickX), -gp.value(Axis::LeftStickY))
        }
    }
}

#[cfg(target_os = "android")]
mod imp {
    use crate::input::PadBtn;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CONNECTED: AtomicU32 = AtomicU32::new(0);
    static BUTTONS: AtomicU32 = AtomicU32::new(0);
    static AXIS_X: AtomicU32 = AtomicU32::new(0);
    static AXIS_Y: AtomicU32 = AtomicU32::new(0);
    static BACK: AtomicU32 = AtomicU32::new(0);

    #[no_mangle]
    pub extern "system" fn Java_com_ordigital_pikavolley_MainActivity_nativeBack(
        _env: *mut std::ffi::c_void,
        _activity: *mut std::ffi::c_void,
    ) {
        BACK.store(1, Ordering::Release);
    }

    pub fn take_back() -> bool {
        BACK.swap(0, Ordering::AcqRel) != 0
    }

    #[no_mangle]
    pub extern "system" fn Java_com_ordigital_pikavolley_MainActivity_nativePadState(
        _env: *mut std::ffi::c_void,
        _activity: *mut std::ffi::c_void,
        connected: i32,
        buttons: i32,
        x: f32,
        y: f32,
    ) {
        BUTTONS.store(buttons as u32, Ordering::Relaxed);
        AXIS_X.store(x.to_bits(), Ordering::Relaxed);
        AXIS_Y.store(y.to_bits(), Ordering::Relaxed);
        CONNECTED.store(connected as u32, Ordering::Release);
    }

    pub struct Pads;

    impl Pads {
        pub fn new() -> Self {
            Pads
        }
        pub fn update(&mut self) {}
        pub fn count(&self) -> usize {
            CONNECTED.load(Ordering::Acquire) as usize
        }
        pub fn pressed(&self) -> Option<PadBtn> {
            None
        }
        pub fn is_down(&self, index: usize, btn: PadBtn) -> bool {
            let bit = match btn {
                PadBtn::South => 1,
                PadBtn::DPadLeft => 2,
                PadBtn::DPadRight => 4,
                PadBtn::DPadUp => 8,
                PadBtn::DPadDown => 16,
                _ => 0,
            };
            index == 0 && self.count() > 0 && BUTTONS.load(Ordering::Relaxed) & bit != 0
        }
        pub fn stick(&self, index: usize) -> (f32, f32) {
            if index == 0 && self.count() > 0 {
                (
                    f32::from_bits(AXIS_X.load(Ordering::Relaxed)),
                    f32::from_bits(AXIS_Y.load(Ordering::Relaxed)),
                )
            } else {
                (0.0, 0.0)
            }
        }
    }
}

/// Browser: no gamepads for now (a Gamepad API bridge could be added later).
#[cfg(target_arch = "wasm32")]
mod imp {
    use crate::input::PadBtn;

    pub struct Pads;

    impl Pads {
        pub fn new() -> Self {
            Pads
        }
        pub fn update(&mut self) {}
        pub fn count(&self) -> usize {
            0
        }
        pub fn pressed(&self) -> Option<PadBtn> {
            None
        }
        pub fn is_down(&self, _index: usize, _btn: PadBtn) -> bool {
            false
        }
        pub fn stick(&self, _index: usize) -> (f32, f32) {
            (0.0, 0.0)
        }
    }
}
