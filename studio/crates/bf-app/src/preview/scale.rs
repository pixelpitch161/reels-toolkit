//! The preview's size rule and the safe-zone geometry, in the internal 360 x 640 picture's pixels.

/// The internal picture's width.
const W: i32 = 360;
/// The internal picture's height.
const H: i32 = 640;

/// The safe zone: x0, y0, x1, y1 (ends excluded). Captions and HUD stay inside it.
pub const SAFE: [i32; 4] = [20, 96, 340, 500];
/// The corner the apps' buttons cover, taken out of the safe zone: x0, y0, x1, y1 (ends excluded).
pub const SAFE_BUTTONS: [i32; 4] = [300, 290, 360, 640];

/// Whether internal pixel (x, y) is in the safe zone: inside `SAFE` and not inside `SAFE_BUTTONS`.
pub fn in_safe_zone(x: i32, y: i32) -> bool {
    holds(&SAFE, x, y) && !holds(&SAFE_BUTTONS, x, y)
}

/// Whether rectangle `r` (x0, y0, x1, y1, ends excluded) holds pixel (x, y).
fn holds(r: &[i32; 4], x: i32, y: i32) -> bool {
    (r[0]..r[2]).contains(&x) && (r[1]..r[3]).contains(&y)
}

/// The rectangles outside the safe zone (x0, y0, x1, y1, ends excluded), not overlapping, covering the rest of the
/// 360 x 640 picture: the band above, the band below, the strip on the left, the strip on the right above the
/// buttons, and the buttons' corner within the safe zone's rows.
pub fn shade() -> [[i32; 4]; 5] {
    let [x0, y0, x1, y1] = SAFE;
    let [bx0, by0, _, _] = SAFE_BUTTONS;
    [
        [0, 0, W, y0],
        [0, y1, W, H],
        [0, y0, x0, y1],
        [x1, y0, W, by0],
        [bx0, by0, W, y1],
    ]
}

/// The preview's size in physical pixels for an available area `avail`: 360n x 640n for the largest whole n that
/// fits, else the largest 9:16 picture that fits, rounded down to whole pixels.
pub fn preview_size(avail: (f32, f32)) -> (f32, f32) {
    // Whole pixels first (`as` saturates: NaN and negative sizes become 0, huge ones u32::MAX, which keeps the u64
    // products below from overflowing), then integer maths only, so a fit such as 297 x 16 / 9 = 528 cannot come
    // out as 527.99 and lose a pixel.
    let (aw, ah) = (u64::from(avail.0.floor() as u32), u64::from(avail.1.floor() as u32));
    let (w, h) = (W as u64, H as u64);
    let n = (aw / w).min(ah / h);
    let (pw, ph) = if n >= 1 {
        (w * n, h * n)
    } else {
        (aw.min(ah * w / h), ah.min(aw * h / w))
    };
    (pw as f32, ph as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shade_is_exactly_the_outside() {
        let shade = shade();
        for y in 0..H {
            for x in 0..W {
                let n = shade.iter().filter(|r| holds(r, x, y)).count();
                let want = if in_safe_zone(x, y) { 0 } else { 1 };
                assert_eq!(n, want, "pixel ({x}, {y})");
            }
        }
    }

    #[test]
    fn the_safe_zone_edges() {
        assert!(in_safe_zone(20, 96));
        assert!(!in_safe_zone(340, 96));
        assert!(in_safe_zone(299, 400));
        assert!(!in_safe_zone(300, 290));
        assert!(in_safe_zone(339, 289));
        assert!(!in_safe_zone(19, 200));
        assert!(!in_safe_zone(100, 95));
        assert!(!in_safe_zone(100, 500));
        assert!(in_safe_zone(100, 499));
        assert!(in_safe_zone(299, 499));
        assert!(!in_safe_zone(300, 499));
        assert!(!in_safe_zone(-1, -1));
        assert!(!in_safe_zone(360, 640));
    }

    #[test]
    fn preview_size_takes_the_largest_whole_scale() {
        assert_eq!(preview_size((410.0, 652.0)), (360.0, 640.0));
        assert_eq!(preview_size((820.0, 1300.0)), (720.0, 1280.0));
        assert_eq!(preview_size((360.0, 640.0)), (360.0, 640.0));
        assert_eq!(preview_size((719.9, 1300.0)), (360.0, 640.0));
        assert_eq!(preview_size((1440.0, 2560.0)), (1440.0, 2560.0));
    }

    #[test]
    fn preview_size_fits_9_16_when_no_whole_scale_fits() {
        assert_eq!(preview_size((410.0, 552.0)), (310.0, 552.0));
        assert_eq!(preview_size((300.0, 652.0)), (300.0, 533.0));
        // 297 x 16 / 9 is exactly 528: integer maths cannot round it down to 527.
        assert_eq!(preview_size((297.0, 652.0)), (297.0, 528.0));
        assert_eq!(preview_size((359.0, 639.0)), (359.0, 638.0));
        assert_eq!(preview_size((300.7, 652.0)), (300.0, 533.0));
        assert_eq!(preview_size((0.0, 0.0)), (0.0, 0.0));
        assert_eq!(preview_size((-5.0, 100.0)), (0.0, 0.0));
        assert_eq!(preview_size((f32::NAN, 100.0)), (0.0, 0.0));
        assert_eq!(preview_size((100.0, f32::INFINITY)), (100.0, 177.0));
    }
}
