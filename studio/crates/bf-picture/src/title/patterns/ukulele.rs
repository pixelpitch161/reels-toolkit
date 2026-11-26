//! UKULELE's panel on the VS card: `title.py` `_surf`, rolling wave crests, foam lines.
//!
//! Wavy bands run across the panel 24 px apart: the top 12 % of each band is a foam crest (the panel x 0.72 plus a
//! pale foam x 0.28), a band from 55 % to 75 % a darker trough (x 0.88).

use bf_core::{dmath, py};

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_surf`.
pub const PORTED: bool = true;

/// The foam, `np.array([250, 246, 252])`.
const FOAM: [f64; 3] = [250.0, 246.0, 252.0];

/// `_surf(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // y = YY + 14.0 * np.sin(XX / 30.0) + 6.0 * np.sin(XX / 11.0 + YY / 40.0); s = (y / 24.0) % 1.0
        let y = yy + 14.0 * dmath::sin(xx / 30.0) + 6.0 * dmath::sin(xx / 11.0 + yy / 40.0);
        let s = py::fmod_py(y / 24.0, 1.0);
        if s > 0.55 && s < 0.75 {
            px.iter_mut().for_each(|v| *v *= 0.88);
        }
        if s < 0.12 {
            for (v, &c) in px.iter_mut().zip(&FOAM) {
                *v = *v * 0.72 + c * 0.28;
            }
        }
    }
}
