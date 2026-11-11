//! `reelkit/config.py`: the frame constants, the safe zone, the Bayer dither tables and the small scalar helpers
//! every picture module imports.
//!
//! Each helper keeps Python's operation order and Python's `min` / `max` (which keep the first argument unless
//! the second is smaller / greater); `studio/reference/picture_kit/config.npz` checks them bit for bit.
//!
//! Not ported, because the renderer never calls them: `nrm` and `dirv`. (`nrm` goes through `np.linalg.norm`,
//! which is OpenBLAS's `dot`, a fused multiply-add chain on the reference PC: see [`crate::np`].)

use crate::img::Img;
use bf_core::py;
use std::sync::OnceLock;

/// The internal render width in pixels.
pub const W: usize = 360;
/// The internal render height in pixels (portrait 9:16).
pub const H: usize = 640;
/// The nearest-neighbour upscale to the 1080 x 1920 output.
pub const UP: usize = 3;
/// Frames per second.
pub const FPS: u32 = 30;
/// The default length of a video in seconds.
pub const DUR: f64 = 61.0;
/// `int(DUR * FPS)`: 1,830 frames.
pub const NF: usize = 1830;

/// The safe zone in internal pixels, `(x0, y0, x1, y1)`: the part of the frame the apps leave visible.
pub const SAFE: (i32, i32, i32, i32) = (20, 96, 340, 500);
/// The right-hand button column the apps cover: `x >= 300` from `y >= 290` down.
pub const SAFE_BUTTONS: (i32, i32, i32, i32) = (300, 290, W as i32, H as i32);
/// `W // 2`: captions are centred on the frame.
pub const SAFE_CX: i32 = (W / 2) as i32;

/// `BAYER`: the 4 x 4 ordered-dither matrix, `[[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]] / 16`
/// (rows top to bottom). Sixteenths are exact in binary.
pub const BAYER: [[f64; 4]; 4] = [
    [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
    [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
    [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
];

/// `BAYER_FULL`: [`BAYER`] tiled over the frame, `np.tile(BAYER, (H // 4, W // 4))`: one `float64` per pixel,
/// `bayer_full().px(x, y)[0] == BAYER[y % 4][x % 4]`.
pub fn bayer_full() -> &'static Img<f64> {
    static TABLE: OnceLock<Img<f64>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut data = Vec::with_capacity(W * H);
        for y in 0..H {
            for x in 0..W {
                data.push(BAYER[y % 4][x % 4]);
            }
        }
        Img::from_vec(W, H, 1, data)
    })
}

/// `XX` of `YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)`: each pixel's column as a `float32`.
pub fn xx() -> &'static Img<f32> {
    static TABLE: OnceLock<Img<f32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut data = Vec::with_capacity(W * H);
        for _ in 0..H {
            data.extend((0..W).map(|x| x as f32));
        }
        Img::from_vec(W, H, 1, data)
    })
}

/// `YY` of `YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)`: each pixel's row as a `float32`.
pub fn yy() -> &'static Img<f32> {
    static TABLE: OnceLock<Img<f32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut data = Vec::with_capacity(W * H);
        for y in 0..H {
            data.extend(std::iter::repeat_n(y as f32, W));
        }
        Img::from_vec(W, H, 1, data)
    })
}

/// `safe_x1(y0, y1)`: the right-hand limit for something spanning rows `y0..y1`: the safe box's edge, or the
/// button column's edge if it reaches down into the column. (`y0` is not used, as in the Python.)
pub fn safe_x1(_y0: i32, y1: i32) -> i32 {
    if y1 > SAFE_BUTTONS.1 { SAFE_BUTTONS.0 } else { SAFE.2 }
}

/// `safe_overshoot(x0, y0, x1, y1)`: the pixels by which a box sticks out of the safe zone (0: fully inside):
/// outside the outer box, or into the button column.
pub fn safe_overshoot(x0: i32, y0: i32, x1: i32, y1: i32) -> i32 {
    let mut over = (SAFE.0 - x0).max(SAFE.1 - y0).max(x1 - SAFE.2).max(y1 - SAFE.3).max(0);
    let (bx0, by0, bx1, by1) = SAFE_BUTTONS;
    if x1 > bx0 && y1 > by0 && x0 < bx1 && y0 < by1 {
        over = over.max((x1 - bx0).min(y1 - by0));
    }
    over
}

/// Python's `max(a, b)` on floats: `a` unless `b` is greater.
fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// Python's `min(a, b)` on floats: `a` unless `b` is smaller.
fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// `whole_period(period, dur)`: the nearest period that completes a whole number of cycles (at least one) in
/// `dur` seconds, so anything moving with it is back where it started at the loop point. Python's `round` ties to
/// even. The Python default for `dur` is [`DUR`].
pub fn whole_period(period: f64, dur: f64) -> f64 {
    // n = max(1, round(dur / period)); return dur / n
    let n = py_max(1.0, py::round_half_even(dur / period));
    dur / n
}

/// `loop_bpm(bpm, dur, beats_per_bar)`: the nearest tempo at which `dur` seconds hold a whole number of bars. The
/// port lives with the beat grid in `bf_core::grid`.
pub use bf_core::grid::loop_bpm;

/// `ss(a, b, x)`: smoothstep from `a` to `b`. `a == b` divides by zero (Python raises there; this returns 0 or 1).
pub fn ss(a: f64, b: f64, x: f64) -> f64 {
    let x = py_min(1.0, py_max(0.0, (x - a) / (b - a)));
    x * x * (3.0 - 2.0 * x)
}

/// `ease(x)`: quintic ease-in-out on 0..1.
pub fn ease(x: f64) -> f64 {
    let x = py_min(1.0, py_max(0.0, x));
    x * x * x * (x * (6.0 * x - 15.0) + 10.0)
}

/// `lerp(a, b, u)`: `a + (b - a) * u`.
pub fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

/// `mixc(a, b, u)`: two colours mixed channel by channel, `a * (1 - u) + b * u` (as `float64`, not rounded).
pub fn mixc(a: [f64; 3], b: [f64; 3], u: f64) -> [f64; 3] {
    let k = 1.0 - u;
    [a[0] * k + b[0] * u, a[1] * k + b[1] * u, a[2] * k + b[2] * u]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_is_portrait_1080_by_1920() {
        assert_eq!((W * UP, H * UP), (1080, 1920));
        assert_eq!(NF, (DUR * f64::from(FPS)) as usize);
        assert_eq!(SAFE_BUTTONS, (300, 290, 360, 640));
        assert_eq!(SAFE_CX, 180);
    }

    #[test]
    fn bayer_tables() {
        let t = bayer_full();
        assert_eq!((t.w, t.h, t.c), (W, H, 1));
        assert_eq!(t.px(0, 0), &[0.0]);
        assert_eq!(t.px(1, 0), &[0.5]);
        assert_eq!(t.px(0, 1), &[0.75]);
        assert_eq!(t.px(359, 639), &[BAYER[3][3]]);
        assert_eq!(t.px(359, 639), &[0.3125]);
        let mut seen: Vec<u32> = BAYER.concat().iter().map(|v| (v * 16.0) as u32).collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..16).collect::<Vec<_>>());
        assert_eq!(xx().px(17, 400), &[17.0]);
        assert_eq!(yy().px(17, 400), &[400.0]);
    }

    #[test]
    fn smoothstep_and_ease_clamp() {
        assert_eq!(ss(0.0, 1.0, -3.0), 0.0);
        assert_eq!(ss(0.0, 1.0, 0.5), 0.5);
        assert_eq!(ss(0.0, 1.0, 9.0), 1.0);
        assert_eq!(ss(1.0, 0.0, 0.25), ss(0.0, 1.0, 0.75));
        assert_eq!((ease(-1.0), ease(0.5), ease(2.0)), (0.0, 0.5, 1.0));
        assert_eq!(lerp(2.0, 4.0, 0.25), 2.5);
        assert_eq!(mixc([0.0, 10.0, 255.0], [100.0, 20.0, 0.0], 0.5), [50.0, 15.0, 127.5]);
    }

    #[test]
    fn whole_periods_and_the_safe_zone() {
        assert_eq!(whole_period(7.3, 61.0), 61.0 / 8.0);
        assert_eq!(whole_period(200.0, 61.0), 61.0);
        // 5 / 2 is the tie 2.5: Python rounds it to 2.
        assert_eq!(whole_period(2.0, 5.0), 2.5);
        assert_eq!(safe_x1(100, 290), 340);
        assert_eq!(safe_x1(100, 291), 300);
        assert_eq!(safe_overshoot(20, 96, 340, 290), 0);
        assert_eq!(safe_overshoot(18, 96, 340, 290), 2);
        assert_eq!(safe_overshoot(250, 280, 310, 300), 10);
    }
}
