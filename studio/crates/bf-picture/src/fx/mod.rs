//! The effects kit: the port of `reelkit/fx.py`.
//!
//! Everything is built once from the fight's events, in time order, and then asked for its state at any time
//! `t`, so frames can be drawn in any order and on any number of threads:
//!
//! | Python | here |
//! |---|---|
//! | `env_ad`, `FlashBudget` | [`timing`] |
//! | `Flares` (attack lights) | [`flares`] |
//! | `starburst`, `Bursts` (impact bursts) | [`bursts`] |
//! | `Shake` (camera shake) | [`shake`] |
//! | `shimmer`, `ellipse_of`, `shimmer_world`, `fringe` (local warps) | [`warp`] |
//! | `PRESETS`, `Particles` | [`particles`] |
//! | `Marks` (marks that fade on the floor) | [`marks`] |
//! | `Moments` (the ability dim and the speed lines) | [`moments`] |
//!
//! Only `_demo` (the kit's self-test sheet) is not ported.
//!
//! # The picture's dtype
//!
//! In Python the picture changes its dtype on the way through the effects, and the arithmetic follows it: after
//! the grade it is `float64`; `Particles.draw` and `Moments.speed_lines` hand back a new `float32` picture of
//! whole numbers when they draw and the very array they were given when they do not; `starburst` then mixes in
//! `float32` or in `float64` depending on which it got. Here a function that mixes takes either picture
//! ([`Px`]) and works in its dtype, and the two that draw return `Option<Img<f32>>`: `None` is "the picture you
//! gave, untouched".
//!
//! # How close to Python
//!
//! Random numbers come from `bf_core::rng` in Python's order, so every draw is the same. The maths functions are
//! `bf_core::dmath`, where Python has the C runtime (`math.sin`) or numpy's loops (`np.sin`, `**`): values can
//! differ in the last bit, and numpy's fused dot products (`np.linalg.norm` of one vector, the camera's `@`) are
//! plain sums here. So float results are equal to about 1e-15, not to the bit, and every picture is equal byte
//! for byte unless such a last-bit difference lands exactly on a rounding step, which the reference cases
//! (`tests/kit_fx_vectors.rs`) never meet.
//!
//! Sizes come from the picture, where Python reads the kit's `W` and `H`.

pub mod bursts;
pub mod flares;
pub mod marks;
pub mod moments;
pub mod particles;
pub mod shake;
pub mod timing;
pub mod warp;

pub use self::bursts::{Bursts, starburst};
pub use self::flares::Flares;
pub use self::marks::{Mark, MarkKind, Marks};
pub use self::moments::Moments;
pub use self::particles::{Emit, Particles};
pub use self::shake::Shake;
pub use self::timing::{FlashBudget, env_ad};
pub use self::warp::{ellipse_of, fringe, shimmer, shimmer_world};

use crate::img::Img;
use crate::np;

mod sealed {
    pub trait Sealed {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

/// A sample of a float picture the effects draw on: `float32` or `float64`, each with numpy's arithmetic for it.
pub trait Px: Copy + PartialEq + sealed::Sealed {
    /// `np.clip(v, 0, 255).astype(np.uint8)`.
    fn to_byte(self) -> u8;

    /// `v * (1 - a) + src * a` as `_blend_patch` computes it: `a` and `src` are `float32`, so `1 - a` and
    /// `src * a` are `float32` results; on a `float64` picture the product with `v` and the sum are `float64`.
    fn over(self, a: f32, src: f32) -> Self;
}

impl Px for f32 {
    fn to_byte(self) -> u8 {
        np::clip_u8_f32(self)
    }

    fn over(self, a: f32, src: f32) -> f32 {
        self * (1.0 - a) + src * a
    }
}

impl Px for f64 {
    fn to_byte(self) -> u8 {
        np::clip_u8(self)
    }

    fn over(self, a: f32, src: f32) -> f64 {
        self * f64::from(1.0 - a) + f64::from(src * a)
    }
}

/// `Image.fromarray(np.clip(img, 0, 255).astype(np.uint8))`: the picture as bytes, to draw on.
pub(crate) fn bytes_of<T: Px>(img: &Img<T>) -> Img<u8> {
    Img::from_vec(img.w, img.h, img.c, img.data.iter().map(|v| v.to_byte()).collect())
}

/// `np.asarray(base).astype(np.float32)`: the drawn bytes as the `float32` picture the draw returns.
pub(crate) fn floats_of(img: &Img<u8>) -> Img<f32> {
    Img::from_vec(img.w, img.h, img.c, img.data.iter().map(|&b| f32::from(b)).collect())
}

/// `tuple(int(v) for v in col)` as a Pillow ink: each channel cut toward zero and held to 0..=255.
pub(crate) fn ink(col: [f64; 3], a: u8) -> [u8; 4] {
    [col[0] as u8, col[1] as u8, col[2] as u8, a]
}

/// A whole number Python computed (an alpha) as the byte Pillow makes of it: held to 0..=255.
pub(crate) fn byte(v: i64) -> u8 {
    v.clamp(0, 255) as u8
}

/// `_bbox(cx, cy, rx, ry)`: the pixel box around an ellipse, cut to a picture of `w` x `h`, as
/// `(x0, x1, y0, y1)` with the ends exclusive. `None` where the box is empty, and where Python's `int()` raises
/// (a centre or radius that is not finite).
pub(crate) fn bbox(cx: f64, cy: f64, rx: f64, ry: f64, w: usize, h: usize) -> Option<(usize, usize, usize, usize)> {
    let ends = [cx - rx, cx + rx, cy - ry, cy + ry];
    if ends.iter().any(|v| !v.is_finite()) {
        return None;
    }
    // int() cuts toward zero; a value beyond i64 is far outside any picture either way
    let [xa, xb, ya, yb] = ends.map(|v| v as i64);
    let x0 = xa.saturating_sub(1).max(0);
    let x1 = xb.saturating_add(2).min(w as i64);
    let y0 = ya.saturating_sub(1).max(0);
    let y1 = yb.saturating_add(2).min(h as i64);
    (x1 > x0 && y1 > y0).then_some((x0 as usize, x1 as usize, y0 as usize, y1 as usize))
}

/// `_Timeline`: events sorted by start time (those with the same time in the order they were added), with a
/// fixed look-back, so the ones alive at `t` are found quickly.
#[derive(Clone, Debug)]
pub(crate) struct Timeline<T> {
    span: f64,
    t0: Vec<f64>,
    /// Each event with the number of the `add` call that made it (0, 1, ...).
    items: Vec<(usize, T)>,
}

impl<T> Timeline<T> {
    pub(crate) fn new(span: f64) -> Timeline<T> {
        Timeline {
            span,
            t0: Vec::new(),
            items: Vec::new(),
        }
    }

    /// `add(t0, item)`. Python appends and sorts by `(t0, index)` before the first look; placing the event after
    /// every one that does not start later gives the same order.
    pub(crate) fn add(&mut self, t0: f64, item: T) {
        let at = self.t0.partition_point(|&x| x <= t0);
        let which = self.items.len();
        self.t0.insert(at, t0);
        self.items.insert(at, (which, item));
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    /// `alive(t)`: the events with `t - span <= t0 <= t`, as `(t0, number of the add call, item)`, in time order.
    pub(crate) fn alive(&self, t: f64) -> impl Iterator<Item = (f64, usize, &T)> {
        // bisect_left(t0, t - span) and bisect_right(t0, t)
        let lo = t - self.span;
        let i0 = self.t0.partition_point(|&x| x < lo);
        let i1 = self.t0.partition_point(|&x| x <= t);
        (i0..i1.max(i0)).map(move |i| (self.t0[i], self.items[i].0, &self.items[i].1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timeline_keeps_time_order_and_then_the_order_added() {
        let mut tl = Timeline::new(1.0);
        for (t0, name) in [(2.0, "a"), (1.0, "b"), (2.0, "c"), (0.5, "d"), (1.0, "e")] {
            tl.add(t0, name);
        }
        assert_eq!(tl.len(), 5);
        let all: Vec<_> = tl.alive(2.0).map(|(t0, which, name)| (t0, which, *name)).collect();
        // the look-back is closed at both ends: 1.0 = 2.0 - span is alive, 0.5 is not
        assert_eq!(all, [(1.0, 1, "b"), (1.0, 4, "e"), (2.0, 0, "a"), (2.0, 2, "c")]);
        assert_eq!(tl.alive(0.4).count(), 0);
        assert_eq!(tl.alive(1.6).map(|q| *q.2).collect::<Vec<_>>(), ["b", "e"]);
        assert_eq!(tl.alive(9.0).count(), 0);
    }

    #[test]
    fn mixing_follows_the_dtype_of_the_picture() {
        // float32: everything in float32; float64: 1 - a and src * a are float32 values, widened
        let (a, src) = (0.3f32, 200.0f32);
        assert_eq!(100.5f32.over(a, src), 100.5f32 * (1.0f32 - a) + src * a);
        assert_eq!(
            100.5f64.over(a, src),
            100.5f64 * f64::from(1.0f32 - a) + f64::from(src * a)
        );
        assert_ne!(100.5f64.over(a, src), 100.5 * (1.0 - 0.3) + 200.0 * 0.3);
        assert_eq!(7.25f64.over(0.0, 99.0), 7.25);
        assert_eq!((-3.0f32).to_byte(), 0);
        assert_eq!(300.7f64.to_byte(), 255);
        assert_eq!(41.9f64.to_byte(), 41);
        assert_eq!(ink([300.5, -20.0, 128.9], 7), [255, 0, 128, 7]);
        assert_eq!((byte(-4), byte(300), byte(77)), (0, 255, 77));
    }

    #[test]
    fn the_box_is_cut_to_the_picture() {
        // int(cx - rx) - 1 and int(cx + rx) + 2, int() cutting toward zero
        assert_eq!(bbox(50.5, 40.2, 10.0, 5.0, 360, 640), Some((39, 62, 34, 47)));
        assert_eq!(bbox(3.0, 3.0, 10.0, 10.0, 20, 12), Some((0, 15, 0, 12)));
        assert_eq!(bbox(-30.0, 5.0, 10.0, 10.0, 20, 12), None);
        assert_eq!(bbox(f64::NAN, 5.0, 10.0, 10.0, 20, 12), None);
        assert_eq!(bbox(5.0, 5.0, f64::INFINITY, 10.0, 20, 12), None);
        assert_eq!(bbox(5.0, 5.0, 1e30, 1e30, 20, 12), Some((0, 20, 0, 12)));
    }

    /// Frames are drawn on several threads from one set of effects.
    #[test]
    fn the_effects_can_be_shared_between_threads() {
        fn shared<T: Send + Sync>() {}
        shared::<Flares>();
        shared::<Bursts>();
        shared::<Shake>();
        shared::<Particles>();
        shared::<Marks>();
        shared::<Moments>();
    }
}
