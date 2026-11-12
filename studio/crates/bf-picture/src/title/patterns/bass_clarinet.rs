//! BASS CLARINET's panel on the VS card: `title.py` `_bassclarinet`, grenadilla, pad cups and the neck.
//!
//! The grain of the black wood runs over the whole panel as lighter and darker streaks; two rows of big pad cups
//! (a silver cup, its ring and the rod they sit on) run along the divider, 36 and 114 px from it on the left
//! fighter's side; the silver gooseneck loops over the top of the panel with a white highlight inside it.

use bf_core::{dmath, py};

use crate::config::W;
use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_bassclarinet`.
pub const PORTED: bool = true;

/// `np.where(mask[..., None], out * keep + col * k, out)` on one pixel.
fn tint(px: &mut [f64; 3], on: bool, keep: f64, col: [f64; 3], k: f64) {
    if on {
        for (v, c) in px.iter_mut().zip(col) {
            *v = *v * keep + c * k;
        }
    }
}

/// `_bassclarinet(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`; this pattern reads none of it).
pub fn pattern(base: &mut RgbF, _card: &Layout) {
    let side = layout::side();
    let dd = layout::dd();
    for (i, (px, &s)) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data).enumerate() {
        let (xx, yy) = ((i % W) as f64, (i / W) as f64);
        // g = np.sin((XX * 0.95 - YY * 0.30) / 5.0 + 1.4 * np.sin(XX / 47.0 + YY / 83.0))
        let g = dmath::sin((xx * 0.95 - yy * 0.30) / 5.0 + 1.4 * dmath::sin(xx / 47.0 + yy / 83.0));
        // out = base * (0.93 + 0.06 * (g > 0.6) - 0.06 * (g < -0.75))
        let light = if g > 0.6 { 1.0 } else { 0.0 };
        let dark = if g < -0.75 { 1.0 } else { 0.0 };
        let grain = 0.93 + 0.06 * light - 0.06 * dark;
        for v in px.iter_mut() {
            *v *= grain;
        }
        // two rows of big pad cups along the divider
        let u = s + 36.0;
        for (j, off) in [0.0, 78.0].into_iter().enumerate() {
            let along = (xx * dd[0] + yy * dd[1]) / 34.0 + 0.5 * j as f64;
            let q = (py::fmod_py(along, 1.0) - 0.5) * 34.0;
            let r = np::hypot(q, u - off);
            let ring = (r - 9.5).abs() < 1.6;
            let cup = r < 7.0;
            tint(px, cup, 0.62, [214.0, 218.0, 232.0], 0.38);
            tint(px, ring, 0.5, [236.0, 238.0, 248.0], 0.5);
            let rod = (u - off - 13.0).abs() < 0.9;
            tint(px, rod, 0.65, [226.0, 228.0, 240.0], 0.35);
        }
        // the silver gooseneck looping over the top
        let r = np::hypot(xx - 268.0, yy - 66.0);
        let neck = (r - 50.0).abs() < 6.5 && (yy < 66.0 || xx > 268.0);
        let hl = (r - 47.0).abs() < 1.5 && neck;
        tint(px, neck, 0.45, [212.0, 216.0, 228.0], 0.55);
        tint(px, hl, 0.3, [255.0, 255.0, 255.0], 0.7);
    }
}
