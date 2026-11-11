//! The app's icon, drawn by this code: the window's `ground` as a square with rounded corners, and on it two touching
//! discs, the series' two fighters meeting, in the interface's `text` grey (left) and `muted` grey (right). Colour
//! belongs to fighters, so the icon is greys only. The same pixels serve the window, the taskbar and the shortcuts
//! (through `assets/icon/ballfight.ico`, which `the_committed_icon_is_current` keeps equal to [`ico_bytes`]).

use crate::theme::tokens;

/// The sizes the icon file holds, smallest first.
pub const SIZES: [u32; 5] = [16, 24, 32, 48, 256];

/// Samples along each side of a pixel (so 64 a pixel), for smooth edges.
const SUB: i64 = 8;
/// The shapes, in 64ths of the icon's side: the ground's corner radius, and each disc's radius. The discs sit on the
/// middle line, one radius either side of the centre, so they touch there.
const CORNER: i64 = 10;
const DISC: i64 = 14;

/// The icon at `size` x `size` pixels, as straight (not premultiplied) RGBA, row by row from the top. Drawn with
/// integer maths only, so every PC makes the same bytes.
pub fn rgba(size: u32) -> Vec<u8> {
    let n = i64::from(size);
    // Fine units: 2 * SUB a pixel, so each sample's centre is a whole (odd) number.
    let side = 2 * SUB * n;
    let at = |sixty_fourths: i64| side * sixty_fourths / 64;
    let (corner, disc, mid) = (at(CORNER), at(DISC), side / 2);
    let left = (mid - disc, mid);
    let right = (mid + disc, mid);
    let within = |(x, y): (i64, i64), (cx, cy): (i64, i64), r: i64| (x - cx).pow(2) + (y - cy).pow(2) <= r * r;
    let grey = |c: egui::Color32| i64::from(c.r());
    let (text, muted, ground) = (grey(tokens::TEXT), grey(tokens::MUTED), grey(tokens::GROUND));

    let mut out = Vec::with_capacity((n * n * 4) as usize);
    for py in 0..n {
        for px in 0..n {
            let (mut sum, mut count) = (0i64, 0i64);
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let p = ((px * SUB + sx) * 2 + 1, (py * SUB + sy) * 2 + 1);
                    let nearest = (p.0.clamp(corner, side - corner), p.1.clamp(corner, side - corner));
                    let value = if within(p, left, disc) {
                        text
                    } else if within(p, right, disc) {
                        muted
                    } else if within(p, nearest, corner) {
                        ground
                    } else {
                        continue;
                    };
                    sum += value;
                    count += 1;
                }
            }
            let all = SUB * SUB;
            let value = if count == 0 { 0 } else { (sum + count / 2) / count } as u8;
            let alpha = ((count * 255 + all / 2) / all) as u8;
            out.extend_from_slice(&[value, value, value, alpha]);
        }
    }
    out
}

/// One picture as PNG bytes (eight bits a channel, with alpha).
fn png_bytes(size: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("a PNG header written to memory");
    writer
        .write_image_data(&rgba(size))
        .expect("a PNG picture written to memory");
    writer.finish().expect("a PNG finished in memory");
    bytes
}

/// An ICO file holding one PNG picture for each of [`SIZES`]: the 6-byte header, a 16-byte entry for each picture
/// (a width or height of 256 is written as 0), then the pictures in the same order.
pub fn ico_bytes() -> Vec<u8> {
    let pictures: Vec<Vec<u8>> = SIZES.iter().map(|&s| png_bytes(s)).collect();
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(SIZES.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * SIZES.len();
    for (&size, picture) in SIZES.iter().zip(&pictures) {
        let written = if size >= 256 { 0 } else { size as u8 };
        out.extend_from_slice(&[written, written, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(picture.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += picture.len();
    }
    for picture in &pictures {
        out.extend_from_slice(picture);
    }
    out
}

/// The 256 picture, for the window and the taskbar.
pub fn window_icon() -> egui::IconData {
    egui::IconData {
        rgba: rgba(256),
        width: 256,
        height: 256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn u16_at(b: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([b[at], b[at + 1]])
    }

    fn u32_at(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    /// Decodes one PNG to (width, height, RGBA bytes).
    fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().expect("a PNG header");
        let mut buf = vec![0; reader.output_buffer_size().expect("a buffer size")];
        let info = reader.next_frame(&mut buf).expect("a PNG frame");
        assert_eq!(
            info.color_type,
            png::ColorType::Rgba,
            "every picture has an alpha channel"
        );
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    }

    #[test]
    fn the_ico_holds_every_size() {
        let ico = ico_bytes();
        assert!(ico.len() > 6, "an ICO has a header");
        assert_eq!(u16_at(&ico, 0), 0, "reserved");
        assert_eq!(u16_at(&ico, 2), 1, "type 1 is an icon");
        assert_eq!(u16_at(&ico, 4) as usize, SIZES.len());
        for (i, &size) in SIZES.iter().enumerate() {
            let entry = 6 + 16 * i;
            let written = if size == 256 { 0 } else { size as u8 };
            assert_eq!(ico[entry], written, "width of {size}");
            assert_eq!(ico[entry + 1], written, "height of {size}");
            assert_eq!(ico[entry + 2], 0, "no palette");
            assert_eq!(ico[entry + 3], 0, "reserved");
            assert_eq!(u16_at(&ico, entry + 4), 1, "planes");
            assert_eq!(u16_at(&ico, entry + 6), 32, "bits per pixel");
            let bytes = u32_at(&ico, entry + 8) as usize;
            let offset = u32_at(&ico, entry + 12) as usize;
            assert!(
                offset + bytes <= ico.len(),
                "the picture of {size} lies inside the file"
            );
            let (w, h, pixels) = decode(&ico[offset..offset + bytes]);
            assert_eq!((w, h), (size, size));
            assert_eq!(pixels, rgba(size), "the file's picture of {size} is rgba({size})");
        }
    }

    #[test]
    fn the_committed_icon_is_current() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icon/ballfight.ico");
        let made = ico_bytes();
        if std::env::var_os("BF_BLESS").is_some_and(|v| v == "1") {
            std::fs::create_dir_all(path.parent().expect("a folder")).expect("make the icon folder");
            std::fs::write(&path, &made).expect("write the icon");
        }
        let committed = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e} (bless it with BF_BLESS=1)", path.display()));
        assert!(
            committed == made,
            "{} is not what icon::ico_bytes() makes: bless it with BF_BLESS=1",
            path.display()
        );
    }

    #[test]
    fn the_icon_is_greys_only() {
        for size in SIZES {
            let pixels = rgba(size);
            assert_eq!(pixels.len(), (size * size * 4) as usize, "{size}: four bytes a pixel");
            for p in pixels.as_chunks::<4>().0 {
                if p[3] > 0 {
                    assert!(p[0] == p[1] && p[1] == p[2], "{size}: a pixel that is not grey: {p:?}");
                }
            }
        }
    }

    #[test]
    fn the_two_discs_and_the_ground_are_there() {
        for size in SIZES {
            let pixels = rgba(size);
            assert_eq!(pixels.len(), (size * size * 4) as usize, "{size}: four bytes a pixel");
            let at = |x: u32, y: u32| {
                let i = ((y * size + x) * 4) as usize;
                [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
            };
            let [tr, tg, tb, _] = tokens::TEXT.to_array();
            let [mr, mg, mb, _] = tokens::MUTED.to_array();
            let [gr, gg, gb, _] = tokens::GROUND.to_array();
            let mid = size / 2;
            assert_eq!(
                at(size * 3 / 10, mid),
                [tr, tg, tb, 255],
                "{size}: the left disc is `text`"
            );
            assert_eq!(
                at(size * 7 / 10, mid),
                [mr, mg, mb, 255],
                "{size}: the right disc is `muted`"
            );
            assert_eq!(
                at(mid, size / 8),
                [gr, gg, gb, 255],
                "{size}: above the discs is `ground`"
            );
            assert!(at(0, 0)[3] < 128, "{size}: the corner is rounded off");
            assert_eq!(at(0, mid)[3], 255, "{size}: the ground reaches the edge");
        }
    }

    #[test]
    fn the_window_icon_is_the_256_picture() {
        let icon = window_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba, rgba(256));
    }

    /// Writes enlarged pictures of each size for a person to judge, only when `BF_ICON_PREVIEW` names a folder:
    /// each size scaled up by whole pixels to about 512, once on a dark and once on a light backdrop, side by side.
    #[test]
    fn previews_on_request() {
        let Some(folder) = std::env::var_os("BF_ICON_PREVIEW") else {
            return;
        };
        let folder = PathBuf::from(folder);
        std::fs::create_dir_all(&folder).expect("make the preview folder");
        let side = 512u32;
        for size in SIZES {
            let pixels = rgba(size);
            let scale = side / size;
            let shown = size * scale;
            let (w, h) = (shown * 2 + 48, shown + 32);
            let mut out = vec![0u8; (w * h * 3) as usize];
            for (i, px) in out.as_chunks_mut::<3>().0.iter_mut().enumerate() {
                let back = if (i as u32 % w) < w / 2 { 0x20 } else { 0xf3 };
                px.copy_from_slice(&[back, back, back]);
            }
            for half in 0..2u32 {
                let x0 = 16 + half * (shown + 16);
                let back = if half == 0 { 0x20u32 } else { 0xf3 };
                for y in 0..shown {
                    for x in 0..shown {
                        let i = (((y / scale) * size + x / scale) * 4) as usize;
                        let (v, a) = (pixels[i] as u32, pixels[i + 3] as u32);
                        let c = ((v * a + back * (255 - a) + 127) / 255) as u8;
                        let o = (((y + 16) * w + x0 + x) * 3) as usize;
                        out[o..o + 3].copy_from_slice(&[c, c, c]);
                    }
                }
            }
            let file = std::fs::File::create(folder.join(format!("icon-{size}.png"))).expect("a preview file");
            let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("a header")
                .write_image_data(&out)
                .expect("the preview");
        }
    }
}
