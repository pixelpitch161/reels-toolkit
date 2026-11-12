//! ACOUSTIC's panel on the VS card: `title.py` `_woodgrain`, spruce grain and six steel strings.
//!
//! The grain is a wavy sine across the panel that lightens or darkens it a little; six thin strings run parallel to
//! the divider, 70 px in and 11 px apart, a light steel blended over the panel.

use bf_core::dmath;

use crate::img::RgbF;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_woodgrain`.
pub const PORTED: bool = true;

/// The strings' steel, `np.array([236, 226, 206])`.
const STEEL: [f64; 3] = [236.0, 226.0, 206.0];

/// `_woodgrain(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`: this pattern reads none of it).
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let side = layout::side();
    for y in 0..base.h {
        for x in 0..base.w {
            let (xx, yy) = (x as f64, y as f64);
            // g = np.sin((XX * 0.30 + YY * 0.95) / 5.5 + 2.2 * np.sin(XX / 37.0 + YY / 90.0))
            let g = dmath::sin((xx * 0.30 + yy * 0.95) / 5.5 + 2.2 * dmath::sin(xx / 37.0 + yy / 90.0));
            // base * (0.94 + 0.08 * (g > 0.55) - 0.05 * (g < -0.7))
            let grain = 0.94 + 0.08 * f64::from(u8::from(g > 0.55)) - 0.05 * f64::from(u8::from(g < -0.7));
            let s = side.px(x, y)[0];
            let px = base.px_mut(x, y);
            for v in px.iter_mut() {
                *v *= grain;
            }
            // strings parallel to the divider: np.abs(SIDE + 70.0 + 11.0 * j) < 0.9
            for j in 0..6 {
                if (s + 70.0 + 11.0 * f64::from(j)).abs() < 0.9 {
                    for (v, &c) in px.iter_mut().zip(&STEEL) {
                        *v = *v * 0.6 + c * 0.4;
                    }
                }
            }
        }
    }
}
