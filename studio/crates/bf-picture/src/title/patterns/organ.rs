//! ORGAN's panel on the VS card: `title.py` `_organ_pipes`, a front of pipes, tall to short, with mouths.
//!
//! Pixel by pixel in the order numpy's whole-array expressions run. The one transcendental is the sine that sets a
//! pipe's height (23 pipes across the frame): `bf_core::dmath::sin` where Python has numpy's. It only decides
//! which rows a pipe and its mouth cover: whole rows against a height that is a whole number only for the first
//! pipe, whose sine is exactly 0 on both sides. So a last bit there moves no pixel.

use bf_core::{dmath, py};

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_organ_pipes`.
pub const PORTED: bool = true;

/// `_organ_pipes(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: pipes 16 px apart and 12 px
/// wide, their tops between 60 and 138 px down (rounded: 8 px lower at a pipe's edges), lit from the left, each
/// with a dark mouth 70 px below its top; between the pipes the panel is darker. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // col = np.floor(XX / 16.0);  xc = (XX % 16.0) - 8.0
        let col = (xx / 16.0).floor();
        let xc = py::fmod_py(xx, 16.0) - 8.0;
        // top = 60.0 + 70.0 * np.abs(np.sin(col * 0.9)) + 2.0 * xc * xc / 8.0
        let top = 60.0 + 70.0 * dmath::sin(col * 0.9).abs() + 2.0 * xc * xc / 8.0;
        let body = xc.abs() < 6.0 && yy > top;
        let mouth = body && (yy - (top + 70.0)).abs() < 3.0 && xc.abs() < 3.5;
        // shade = 1.0 + 0.10 * np.where(xc < -2, 1.0, np.where(xc > 3, -1.0, 0.0))
        let lit = if xc < -2.0 {
            1.0
        } else if xc > 3.0 {
            -1.0
        } else {
            0.0
        };
        let shade = 1.0 + 0.10 * lit;
        for v in px {
            // base = np.where(body, base * (0.98 * shade) + 14, base * 0.80); np.where(mouth, base * 0.45, base)
            let b = if body { *v * (0.98 * shade) + 14.0 } else { *v * 0.80 };
            *v = if mouth { b * 0.45 } else { b };
        }
    }
}
