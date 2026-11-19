//! BAGPIPES's panel on the VS card: `title.py` `_tartan`, a large tartan sett, low contrast.
//!
//! Pixel by pixel in the order numpy's whole-array expressions run; no transcendental function.

use bf_core::py;

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_tartan`.
pub const PORTED: bool = true;

/// The over-check's red.
const RED: [f64; 3] = [200.0, 50.0, 46.0];

/// `_tartan(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: a checker of 46 px diagonal blocks,
/// a little lighter on alternate blocks, with a red over-check line through the middle of each. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // u, v = (XX + YY) / 46.0, (XX - YY) / 46.0
        let (u, v) = ((xx + yy) / 46.0, (xx - yy) / 46.0);
        // chk = ((np.floor(u) + np.floor(v)) % 2)
        let chk = py::fmod_py(u.floor() + v.floor(), 2.0);
        // line = (np.abs((u % 1) - 0.5) < 0.05) | (np.abs((v % 1) - 0.5) < 0.05)
        let line = (py::fmod_py(u, 1.0) - 0.5).abs() < 0.05 || (py::fmod_py(v, 1.0) - 0.5).abs() < 0.05;
        let k = 0.92 + 0.12 * chk;
        for (c, red) in px.iter_mut().zip(RED) {
            // base = base * (0.92 + 0.12 * chk); np.where(line, base * 0.7 + RED * 0.3, base)
            let b = *c * k;
            *c = if line { b * 0.7 + red * 0.3 } else { b };
        }
    }
}
