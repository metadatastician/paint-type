// SPDX-License-Identifier: AGPL-3.0-or-later
//
// codec -- PNG encode/decode for straight-alpha RGBA8 buffers. Decode is
// bounded by MAX_DIM and supports 1/2/4/8/16-bit RGB, RGBA, Grayscale,
// Grayscale+Alpha, Indexed Palette, and tRNS transparency chunks.

/// Maximum width or height accepted on decode (guards against
/// decompression-bomb dimensions).
pub const MAX_DIM: u32 = 16_384;

fn validate_io_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains('\0') {
        return Err("invalid empty or NUL-containing file path".to_string());
    }
    Ok(())
}

/// Encode `rgba` (length `w * h * 4`, straight alpha) to a PNG file.
pub fn save_png(path: &str, rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    validate_io_path(path)?;
    if w == 0 || h == 0 || w > MAX_DIM || h > MAX_DIM {
        return Err(format!("invalid PNG dimensions {w}x{h}"));
    }
    let expected = (w as usize)
        .checked_mul(h as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| format!("image dimensions {w}x{h} overflow buffer size"))?;
    if rgba.len() != expected {
        return Err(format!(
            "buffer length {} does not match {}x{}x4 = {}",
            rgba.len(),
            w,
            h,
            expected
        ));
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    Ok(())
}

/// Decode a PNG file into `(rgba, w, h)` straight-alpha RGBA8, expanding
/// sub-byte / paletted / tRNS images and stripping 16-bit channels cleanly.
pub fn load_png(path: &str) -> Result<(Vec<u8>, u32, u32), String> {
    validate_io_path(path)?;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    if info.width == 0 || info.height == 0 || info.width > MAX_DIM || info.height > MAX_DIM {
        return Err(format!(
            "image {}x{} outside allowed bounds 1..={}",
            info.width, info.height, MAX_DIM
        ));
    }

    let alloc_len = reader.output_buffer_size();
    let mut buf = vec![0u8; alloc_len];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let w = frame.width;
    let h = frame.height;
    let out_len = (w as usize)
        .checked_mul(h as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| format!("decoded frame {w}x{h} overflows RGBA8 buffer"))?;
    buf.truncate(frame.buffer_size());

    let rgba = match frame.color_type {
        png::ColorType::Rgba => {
            if buf.len() != out_len {
                return Err(format!(
                    "decoded RGBA frame size {} != expected {}",
                    buf.len(),
                    out_len
                ));
            }
            buf
        }
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(out_len);
            for px in buf.chunks_exact(3) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity(out_len);
            for px in buf.chunks_exact(2) {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity(out_len);
            for &g in &buf {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        other => return Err(format!("unsupported colour type {other:?}")),
    };
    Ok((rgba, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_pixels() {
        let w = 3;
        let h = 2;
        let src: Vec<u8> = (0..(w * h * 4) as u8).collect();
        let path = std::env::temp_dir().join("pt_codec_roundtrip.png");
        let path = path.to_str().unwrap();

        save_png(path, &src, w, h).expect("save");
        let (out, ow, oh) = load_png(path).expect("load");
        assert_eq!((ow, oh), (w, h));
        assert_eq!(out, src);
    }

    #[test]
    fn load_png_decodes_16bit_and_grayscale_pngs() {
        let path16 = std::env::temp_dir().join("pt_codec_16bit.png");
        {
            let f = std::fs::File::create(&path16).expect("create 16-bit png");
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), 2, 1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Sixteen);
            let mut writer = enc.write_header().expect("header");
            // Two 16-bit big-endian RGBA pixels: pure red opaque, pure blue half-alpha.
            let raw16: [u8; 16] = [
                0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, // (255, 0, 0, 255)
                0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x80, 0x80, // (0, 0, 255, 128)
            ];
            writer.write_image_data(&raw16).expect("write 16-bit data");
        }
        let (rgba16, w16, h16) = load_png(path16.to_str().unwrap()).expect("decode 16-bit");
        assert_eq!((w16, h16), (2, 1));
        assert_eq!(&rgba16[0..4], &[255, 0, 0, 255]);
        assert_eq!(&rgba16[4..8], &[0, 0, 255, 128]);

        let path_gray = std::env::temp_dir().join("pt_codec_gray.png");
        {
            let f = std::fs::File::create(&path_gray).expect("create gray png");
            let mut enc = png::Encoder::new(std::io::BufWriter::new(f), 2, 1);
            enc.set_color(png::ColorType::Grayscale);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().expect("header");
            writer.write_image_data(&[64u8, 192u8]).expect("write gray");
        }
        let (rgba_g, wg, hg) = load_png(path_gray.to_str().unwrap()).expect("decode gray");
        assert_eq!((wg, hg), (2, 1));
        assert_eq!(rgba_g, vec![64, 64, 64, 255, 192, 192, 192, 255]);
    }

    #[test]
    fn save_rejects_wrong_length() {
        let r = save_png("/tmp/unused.png", &[0, 0, 0], 10, 10);
        assert!(r.is_err());
    }
}
