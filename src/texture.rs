//! Texture loading: PNG through macroquad, lossy/lossless WebP through `image-webp`.
use macroquad::prelude::{Image, Texture2D};
use std::io::Cursor;

/// Decodes a WebP file to (width, height, RGBA bytes).
pub fn decode_webp(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).expect("embedded WebP");
    let (width, height) = decoder.dimensions();
    let has_alpha = decoder.has_alpha();
    let mut raw = vec![0u8; decoder.output_buffer_size().expect("WebP size")];
    decoder.read_image(&mut raw).expect("embedded WebP pixels");
    let rgba = if has_alpha {
        raw
    } else {
        raw.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect()
    };
    (width, height, rgba)
}

pub fn load(bytes: &[u8]) -> Texture2D {
    if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        let (width, height, rgba) = decode_webp(bytes);
        Texture2D::from_image(&Image {
            bytes: rgba,
            width: width as u16,
            height: height as u16,
        })
    } else {
        Texture2D::from_file_with_format(bytes, None)
    }
}

use std::sync::atomic::{AtomicBool, Ordering};

static IS_LOW_END: AtomicBool = AtomicBool::new(false);

pub fn init_device_profile() {
    #[allow(unused_mut)]
    let mut low_end = false;
    #[cfg(target_os = "android")]
    {
        let max_tex = max_texture_size();
        let renderer = gl_renderer().to_ascii_lowercase();
        if max_tex < 4452
            || renderer.contains("adreno (tm) 30")
            || renderer.contains("adreno 30")
            || renderer.contains("mali-400")
            || renderer.contains("mali-450")
            || renderer.contains("mali-t720")
        {
            low_end = true;
        }
    }
    IS_LOW_END.store(low_end, Ordering::Relaxed);
}

pub fn is_low_end_device() -> bool {
    IS_LOW_END.load(Ordering::Relaxed)
}

pub fn max_texture_size() -> i32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut size: i32 = 0;
        unsafe {
            macroquad::miniquad::native::gl::glGetIntegerv(
                macroquad::miniquad::native::gl::GL_MAX_TEXTURE_SIZE,
                &mut size,
            );
        }
        if size > 0 {
            return size;
        }
    }
    4096
}

#[allow(dead_code)]
pub fn gl_renderer() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        unsafe {
            let ptr = macroquad::miniquad::native::gl::glGetString(0x1F01);
            if !ptr.is_null() {
                if let Ok(s) = std::ffi::CStr::from_ptr(ptr.cast()).to_str() {
                    return s.to_owned();
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::decode_webp;

    #[test]
    fn embedded_webp_textures_decode_to_the_expected_size() {
        for (bytes, w, h) in [
            (include_bytes!("../assets/images/background-wide.webp").as_slice(), 1672, 941),
            (include_bytes!("../assets/images/menu_grass.webp").as_slice(), 1728, 1216),
        ] {
            let (width, height, rgba) = decode_webp(bytes);
            assert_eq!((width, height), (w, h));
            assert_eq!(rgba.len(), (w * h * 4) as usize);
            assert!(rgba.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
            assert!(rgba.as_chunks::<4>().0.iter().any(|p| p[1] > 60), "image should not be black");
        }
    }
}
