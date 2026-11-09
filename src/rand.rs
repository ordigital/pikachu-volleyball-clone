//! Odpowiednik `rand.js`: `rand()` zwraca losową liczbę całkowitą z [0, 32767].
//! (W wersji JS jest to `Math.floor(32768 * Math.random())`.)

use std::cell::Cell;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static STATE: Cell<u64> = Cell::new(initial_seed());
}

#[cfg(target_arch = "wasm32")]
fn initial_seed() -> u64 {
    // SystemTime panics in the browser; the miniquad clock is Date.now() based.
    macroquad::miniquad::date::now().to_bits() ^ 0x9E37_79B9_7F4A_7C15
}

#[cfg(not(target_arch = "wasm32"))]
fn initial_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    nanos ^ 0x9E37_79B9_7F4A_7C15
}

/// splitmix64
fn next_u64() -> u64 {
    STATE.with(|c| {
        let mut z = c.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
        c.set(z);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    })
}

/// Losowa liczba całkowita z [0, 32767].
pub fn rand() -> i32 {
    ((next_u64() >> 33) % 32768) as i32
}
