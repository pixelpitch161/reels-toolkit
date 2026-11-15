//! ACCORDION's panel on the VS card: `title.py` `_pleats`, pleats parallel to the divider.
//!
//! Bands 14 px wide running parallel to the divider, alternately lighter and darker.

use bf_core::py;

use crate::img::RgbF;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_pleats`.
pub const PORTED: bool = true;

/// `_pleats(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let side = layout::side();
    for y in 0..base.h {
        for x in 0..base.w {
            // s = (SIDE / 14.0) % 2; base * np.where(s < 1, 1.06, 0.84)
            let s = py::fmod_py(side.px(x, y)[0] / 14.0, 2.0);
            let k = if s < 1.0 { 1.06 } else { 0.84 };
            for v in base.px_mut(x, y).iter_mut() {
                *v *= k;
            }
        }
    }
}
