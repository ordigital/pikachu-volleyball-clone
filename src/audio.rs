//! Port `audio.js`. Wspólny enum dźwięków i implementacja rodio.

#[derive(Clone, Copy)]
pub enum Snd {
    Pipikachu,
    Pika,
    Chu,
    Pi,
    Pikachu,
    PowerHit,
    BallTouchesGround,
}

#[cfg(not(target_arch = "wasm32"))]
#[path = "audio_rodio.rs"]
mod imp;
#[cfg(target_arch = "wasm32")]
#[path = "audio_web.rs"]
mod imp;

pub use imp::PikaAudio;
