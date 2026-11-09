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
