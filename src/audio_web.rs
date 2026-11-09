//! Browser audio: the same `PikaAudio` API as audio_rodio.rs, backed by the Web Audio plugin in
//! ports/browser/pika_audio.js (decodeAudioData, StereoPannerNode, one looping music source).
use super::Snd;

const SFX_VOLUME: f32 = 0.35;

extern "C" {
    fn pika_audio_load(id: u32, ptr: *const u8, len: u32);
    fn pika_audio_sfx(id: u32, pan: f32, gain: f32);
    fn pika_audio_music(id: u32, gain: f32);
    fn pika_audio_music_stop();
    fn pika_audio_music_pause(paused: u32);
    fn pika_audio_music_gain(gain: f32);
}

/// Lets the JS side verify that the plugin matches this crate (miniquad plugin convention).
#[no_mangle]
pub extern "C" fn pika_audio_crate_version() -> u32 {
    1
}

#[derive(Clone, Copy)]
enum MusicTrack {
    Menu,
    Game,
}

impl MusicTrack {
    fn id(self) -> u32 {
        match self {
            Self::Menu => 7,
            Self::Game => 8,
        }
    }

    fn volume(self, enabled: bool, gain: f32) -> f32 {
        if !enabled {
            return 0.0;
        }
        gain * match self {
            Self::Menu => 0.45,
            Self::Game => 0.2,
        }
    }
}

pub struct PikaAudio {
    bgm_track: Option<MusicTrack>,
    bgm_on: bool,
    bgm_gain: f32,
}

impl PikaAudio {
    pub fn new() -> Self {
        let sounds: [&[u8]; 9] = [
            include_bytes!("../assets/sounds/WAVE140_1.wav"),
            include_bytes!("../assets/sounds/WAVE141_1.wav"),
            include_bytes!("../assets/sounds/WAVE142_1.wav"),
            include_bytes!("../assets/sounds/WAVE143_1.wav"),
            include_bytes!("../assets/sounds/WAVE144_1.wav"),
            include_bytes!("../assets/sounds/WAVE145_1.wav"),
            include_bytes!("../assets/sounds/WAVE146_1.wav"),
            include_bytes!("../assets/sounds/menu.ogg"),
            include_bytes!("../assets/sounds/bgm.ogg"),
        ];
        for (id, bytes) in sounds.iter().enumerate() {
            unsafe { pika_audio_load(id as u32, bytes.as_ptr(), bytes.len() as u32) };
        }
        Self {
            bgm_track: None,
            bgm_on: true,
            bgm_gain: 1.0,
        }
    }

    /// `left_or_center_or_right`: -1 left, 0 centre, 1 right channel.
    pub fn play(&self, snd: Snd, left_or_center_or_right: i32) {
        let id = match snd {
            Snd::Pipikachu => 0,
            Snd::Pika => 1,
            Snd::Chu => 2,
            Snd::Pi => 3,
            Snd::Pikachu => 4,
            Snd::PowerHit => 5,
            Snd::BallTouchesGround => 6,
        };
        let pan = match left_or_center_or_right {
            -1 => -0.75,
            1 => 0.75,
            _ => 0.0,
        };
        unsafe { pika_audio_sfx(id, pan, SFX_VOLUME) };
    }

    pub fn play_bgm(&mut self) {
        self.play_music(MusicTrack::Game);
    }

    pub fn play_menu_bgm(&mut self) {
        self.play_music(MusicTrack::Menu);
    }

    fn play_music(&mut self, track: MusicTrack) {
        self.stop_bgm();
        self.bgm_track = Some(track);
        self.bgm_gain = 1.0;
        unsafe { pika_audio_music(track.id(), track.volume(self.bgm_on, self.bgm_gain)) };
    }

    pub fn set_paused(&mut self, paused: bool) {
        if self.bgm_track.is_some() {
            unsafe { pika_audio_music_pause(paused as u32) };
        }
    }

    pub fn stop_bgm(&mut self) {
        if self.bgm_track.take().is_some() {
            unsafe { pika_audio_music_stop() };
        }
    }

    pub fn turn_bgm_volume(&mut self, turn_on: bool) {
        self.bgm_on = turn_on;
        self.update_bgm_volume();
    }

    pub fn set_bgm_gain(&mut self, gain: f32) {
        let gain = gain.clamp(0.0, 1.0);
        if gain != self.bgm_gain {
            self.bgm_gain = gain;
            self.update_bgm_volume();
        }
    }

    fn update_bgm_volume(&self) {
        if let Some(track) = self.bgm_track {
            unsafe { pika_audio_music_gain(track.volume(self.bgm_on, self.bgm_gain)) };
        }
    }

    pub fn is_bgm_on(&self) -> bool {
        self.bgm_on
    }
}
