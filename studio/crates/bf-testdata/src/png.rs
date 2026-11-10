//! PNG in and out: golden frames are 8-bit PNGs written by Pillow; failure artefacts are written as RGB.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Reads a PNG as `(width, height, channels, pixels)`: 1 (grey), 3 (RGB) or 4 (RGBA) channels of 8 bits, rows
/// top to bottom. A palette image comes back as RGB (or RGBA with a transparency chunk) and low-bit grey as 8 bit;
/// a 16-bit image, a grey-with-alpha image or an unreadable file panics naming the path.
pub fn read_png(path: &Path) -> (usize, usize, usize, Vec<u8>) {
    let fail = |e: &dyn std::fmt::Display| -> ! { panic!("cannot read PNG {}: {e}", path.display()) };
    let file = File::open(path).unwrap_or_else(|e| fail(&e));
    let mut decoder = ::png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(::png::Transformations::EXPAND);
    let mut reader = decoder.read_info().unwrap_or_else(|e| fail(&e));
    let size = reader
        .output_buffer_size()
        .unwrap_or_else(|| fail(&"the image is too large"));
    let mut buf = vec![0; size];
    let out = reader.next_frame(&mut buf).unwrap_or_else(|e| fail(&e));
    if out.bit_depth != ::png::BitDepth::Eight {
        fail(&format!("{:?}-bit samples; only 8-bit images are read", out.bit_depth));
    }
    let channels = match out.color_type {
        ::png::ColorType::Grayscale => 1,
        ::png::ColorType::Rgb => 3,
        ::png::ColorType::Rgba => 4,
        other => fail(&format!("colour type {other:?}; only grey, RGB and RGBA are read")),
    };
    let (w, h) = (out.width as usize, out.height as usize);
    buf.truncate(out.buffer_size());
    assert_eq!(
        buf.len(),
        w * h * channels,
        "{}: unexpected row padding",
        path.display()
    );
    (w, h, channels, buf)
}

/// Writes `rgb` (`w * h * 3` bytes, rows top to bottom) as an 8-bit RGB PNG, making the folder if needed; for
/// failure artefacts under `studio/target`. Panics naming the path on any error.
pub fn write_png_rgb(path: &Path, w: usize, h: usize, rgb: &[u8]) {
    assert_eq!(
        rgb.len(),
        w * h * 3,
        "{}: {w} x {h} RGB wants {} bytes, got {}",
        path.display(),
        w * h * 3,
        rgb.len()
    );
    let fail = |e: &dyn std::fmt::Display| -> ! { panic!("cannot write PNG {}: {e}", path.display()) };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| fail(&e));
    }
    let (Ok(width), Ok(height)) = (u32::try_from(w), u32::try_from(h)) else {
        fail(&format!("{w} x {h} is too large"))
    };
    let file = File::create(path).unwrap_or_else(|e| fail(&e));
    let mut encoder = ::png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(::png::ColorType::Rgb);
    encoder.set_depth(::png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap_or_else(|e| fail(&e));
    writer.write_image_data(rgb).unwrap_or_else(|e| fail(&e));
    writer.finish().unwrap_or_else(|e| fail(&e));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/bf-testdata-unit");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn rgb_round_trips() {
        let path = scratch("round_trip.png");
        let rgb: Vec<u8> = (0..5 * 3 * 3).map(|i| (i * 17 % 256) as u8).collect();
        write_png_rgb(&path, 5, 3, &rgb);
        assert_eq!(read_png(&path), (5, 3, 3, rgb));
    }

    #[test]
    fn grey_and_palette_images_read_as_8_bit() {
        let path = scratch("grey.png");
        let mut e = ::png::Encoder::new(std::fs::File::create(&path).unwrap(), 2, 2);
        e.set_color(::png::ColorType::Grayscale);
        e.set_depth(::png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&[0, 50, 100, 255]).unwrap();
        assert_eq!(read_png(&path), (2, 2, 1, vec![0, 50, 100, 255]));

        let path = scratch("palette.png");
        let mut e = ::png::Encoder::new(std::fs::File::create(&path).unwrap(), 3, 1);
        e.set_color(::png::ColorType::Indexed);
        e.set_depth(::png::BitDepth::Eight);
        e.set_palette(vec![1, 2, 3, 200, 100, 0]);
        e.write_header().unwrap().write_image_data(&[1, 0, 1]).unwrap();
        assert_eq!(read_png(&path), (3, 1, 3, vec![200, 100, 0, 1, 2, 3, 200, 100, 0]));
    }

    #[test]
    #[should_panic(expected = "missing.png")]
    fn a_missing_file_is_named() {
        read_png(&scratch("missing.png"));
    }

    #[test]
    #[should_panic(expected = "2 x 2 RGB wants 12 bytes, got 11")]
    fn write_checks_the_size() {
        write_png_rgb(&scratch("bad.png"), 2, 2, &[0; 11]);
    }
}
