//! Port `audio.js`: efekty dźwiękowe z panoramą stereo i muzyka w tle.
//!
//! Oryginał używa pixi-sound ze `StereoFilter` (pan -0.75 / 0 / 0.75).
//! Tu to samo robimy ręcznie: dla źródła mono stosujemy wzór StereoPannerNode
//! z Web Audio (równomocny), po czym odtwarzamy gotowy bufor przez rodio.
//! Głośności: muzyka meczu 0.2, menu 0.45, SFX 0.35.

use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;

const SFX_VOLUME: f32 = 0.35;

const BGM_BYTES: &[u8] = include_bytes!("../assets/sounds/bgm.ogg");
const MENU_BGM_BYTES: &[u8] = include_bytes!("../assets/sounds/menu.ogg");

#[derive(Clone, Copy)]
enum MusicTrack {
    Menu,
    Game,
}

impl MusicTrack {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Menu => MENU_BGM_BYTES,
            Self::Game => BGM_BYTES,
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

use super::Snd;

/// Jeden dźwięk w trzech wariantach panoramy.
struct StereoSound {
    rate: u32,
    left: Vec<f32>,
    center: Vec<f32>,
    right: Vec<f32>,
}

impl StereoSound {
    fn from_wav(bytes: &[u8]) -> Self {
        let (rate, mono) = parse_wav(bytes);
        Self {
            rate,
            left: pan(&mono, -0.75),
            center: pan(&mono, 0.0),
            right: pan(&mono, 0.75),
        }
    }
}

/// Wzór StereoPannerNode dla źródła mono; zwraca bufor przeplatany L,R.
fn pan(mono: &[f32], pan: f32) -> Vec<f32> {
    let x = (pan + 1.0) / 2.0;
    let gl = (x * std::f32::consts::FRAC_PI_2).cos();
    let gr = (x * std::f32::consts::FRAC_PI_2).sin();
    let mut out = Vec::with_capacity(mono.len() * 2);
    for &s in mono {
        out.push(s * gl);
        out.push(s * gr);
    }
    out
}

/// Minimalny parser WAV (PCM 8/16 bit); bierze pierwszy kanał.
fn parse_wav(bytes: &[u8]) -> (u32, Vec<f32>) {
    let mut rate = 22050u32;
    let mut bits = 8u16;
    let mut channels = 1u16;
    let mut samples: Vec<f32> = Vec::new();
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        let start = pos + 8;
        let end = (start + size).min(bytes.len());
        if id == b"fmt " && end - start >= 16 {
            channels = u16::from_le_bytes([bytes[start + 2], bytes[start + 3]]).max(1);
            rate = u32::from_le_bytes([
                bytes[start + 4],
                bytes[start + 5],
                bytes[start + 6],
                bytes[start + 7],
            ]);
            bits = u16::from_le_bytes([bytes[start + 14], bytes[start + 15]]);
        } else if id == b"data" {
            let data = &bytes[start..end];
            if bits == 8 {
                samples = data
                    .chunks(channels as usize)
                    .map(|c| (c[0] as f32 - 128.0) / 128.0)
                    .collect();
            } else {
                let frame = 2 * channels as usize;
                samples = data
                    .chunks(frame)
                    .filter(|c| c.len() >= 2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                    .collect();
            }
        }
        pos = start + size + (size & 1);
    }
    (rate, samples)
}

pub struct PikaAudio {
    // _stream musi żyć tak długo jak odtwarzamy dźwięk
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,

    pipikachu: [StereoSound; 2],
    pika: [StereoSound; 2],
    chu: [StereoSound; 2],
    pi: [StereoSound; 2],
    pikachu: [StereoSound; 2],
    power_hit: StereoSound,
    ball_touches_ground: StereoSound,

    bgm: Option<Sink>,
    bgm_track: Option<MusicTrack>,
    bgm_on: bool,
    bgm_gain: f32,
}

impl PikaAudio {
    pub fn new() -> Self {
        let (stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("Brak urządzenia audio ({e}) – gra bez dźwięku.");
                (None, None)
            }
        };
        Self {
            _stream: stream,
            handle,
            pipikachu: [
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE140_1.wav")),
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE140_2.wav")),
            ],
            pika: [
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE141_1.wav")),
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE141_2.wav")),
            ],
            chu: [
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE142_1.wav")),
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE142_2.wav")),
            ],
            pi: [
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE143_1.wav")),
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE143_2.wav")),
            ],
            pikachu: [
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE144_1.wav")),
                StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE144_2.wav")),
            ],
            power_hit: StereoSound::from_wav(include_bytes!("../assets/sounds/WAVE145_1.wav")),
            ball_touches_ground: StereoSound::from_wav(include_bytes!(
                "../assets/sounds/WAVE146_1.wav"
            )),
            bgm: None,
            bgm_track: None,
            bgm_on: true,
            bgm_gain: 1.0,
        }
    }

    /// `left_or_center_or_right`: -1 lewy, 0 środek, 1 prawy kanał
    pub fn play(&self, snd: Snd, left_or_center_or_right: i32) {
        self.play_alt(snd, left_or_center_or_right, false);
    }

    pub fn play_alt(&self, snd: Snd, left_or_center_or_right: i32, alt: bool) {
        let handle = match &self.handle {
            Some(h) => h,
            None => return,
        };
        let idx = usize::from(alt);
        let s = match snd {
            Snd::Pipikachu => &self.pipikachu[idx],
            Snd::Pika => &self.pika[idx],
            Snd::Chu => &self.chu[idx],
            Snd::Pi => &self.pi[idx],
            Snd::Pikachu => &self.pikachu[idx],
            Snd::PowerHit => &self.power_hit,
            Snd::BallTouchesGround => &self.ball_touches_ground,
        };
        let data = match left_or_center_or_right {
            -1 => &s.left,
            1 => &s.right,
            _ => &s.center,
        };
        if let Ok(sink) = Sink::try_new(handle) {
            sink.set_volume(SFX_VOLUME);
            sink.append(SamplesBuffer::new(2, s.rate, data.clone()));
            sink.detach();
        }
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
        let handle = match &self.handle {
            Some(h) => h,
            None => return,
        };
        if let Ok(sink) = Sink::try_new(handle) {
            if let Ok(decoder) = Decoder::new_looped(Cursor::new(track.bytes())) {
                sink.set_volume(track.volume(self.bgm_on, self.bgm_gain));
                sink.append(decoder);
                self.bgm = Some(sink);
            }
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if let Some(sink) = &self.bgm {
            if paused {
                sink.pause();
            } else {
                sink.play();
            }
        }
    }

    pub fn stop_bgm(&mut self) {
        self.bgm_track = None;
        if let Some(sink) = self.bgm.take() {
            sink.stop();
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
        if let (Some(sink), Some(track)) = (&self.bgm, self.bgm_track) {
            sink.set_volume(track.volume(self.bgm_on, self.bgm_gain));
        }
    }

    pub fn is_bgm_on(&self) -> bool {
        self.bgm_on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_tracks_decode_and_keep_their_volume_when_unmuted() {
        for (track, volume) in [(MusicTrack::Menu, 0.45), (MusicTrack::Game, 0.2)] {
            assert!(Decoder::new_looped(Cursor::new(track.bytes())).is_ok());
            assert_eq!(track.volume(true, 1.0), volume);
            assert_eq!(track.volume(false, 0.5), 0.0);
            assert_eq!(track.volume(true, 0.5), volume * 0.5);
            assert_eq!(track.volume(true, 0.0), 0.0);
        }
    }

    #[test]
    fn all_sound_effects_decode_correctly() {
        for bytes in [
            include_bytes!("../assets/sounds/WAVE140_1.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE140_2.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE141_1.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE141_2.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE142_1.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE142_2.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE143_1.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE143_2.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE144_1.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE144_2.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE145_1.wav").as_slice(),
            include_bytes!("../assets/sounds/WAVE146_1.wav").as_slice(),
        ] {
            let (rate, samples) = parse_wav(bytes);
            assert_eq!(rate, 22050);
            assert!(!samples.is_empty());
        }
    }
}
