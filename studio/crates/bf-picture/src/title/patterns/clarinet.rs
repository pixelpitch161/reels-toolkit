//! CLARINET's panel on the VS card: `title.py` `_clarinet_keys`, ebony grain and rows of ring keys.
//!
//! The grain is a wavy sine across the panel that lightens or darkens it a little; three rows of keys run parallel to
//! the divider (30 px in, then 64 and 128 px further): every 26 px a nickel ring round a darker cup, and a thin nickel
//! rod beside each row.

use bf_core::{dmath, py};

use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_clarinet_keys`.
pub const PORTED: bool = true;

/// `_clarinet_keys(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`: this pattern reads none of it).
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let dd = layout::dd();
    let side = layout::side();
    for y in 0..base.h {
        for x in 0..base.w {
            let (xx, yy) = (x as f64, y as f64);
            // g = np.sin((XX * 0.95 - YY * 0.30) / 4.5 + 1.6 * np.sin(XX / 41.0 + YY / 77.0))
            let g = dmath::sin((xx * 0.95 - yy * 0.30) / 4.5 + 1.6 * dmath::sin(xx / 41.0 + yy / 77.0));
            // base * (0.93 + 0.06 * (g > 0.6) - 0.06 * (g < -0.75))
            let grain = 0.93 + 0.06 * f64::from(u8::from(g > 0.6)) - 0.06 * f64::from(u8::from(g < -0.75));
            // u = SIDE + 30.0: key rows run parallel to the divider
            let u = side.px(x, y)[0] + 30.0;
            let px = base.px_mut(x, y);
            for v in px.iter_mut() {
                *v *= grain;
            }
            for (j, off) in [0.0, 64.0, 128.0].into_iter().enumerate() {
                let along = (xx * dd[0] + yy * dd[1]) / 26.0 + 0.5 * j as f64;
                let c = (u - off).abs() < 8.0;
                let q = (py::fmod_py(along, 1.0) - 0.5) * 26.0;
                let r = np::hypot(q, u - off);
                let ring = c && (r - 6.0).abs() < 1.3;
                let cup = c && r < 3.2;
                let rod = (u - off - 9.5).abs() < 0.8;
                for (v, (key, bar)) in px.iter_mut().zip([(236.0, 226.0), (236.0, 226.0), (248.0, 240.0)]) {
                    if ring {
                        *v = *v * 0.55 + key * 0.45;
                    }
                    if cup {
                        *v *= 0.78;
                    }
                    if rod {
                        *v = *v * 0.7 + bar * 0.3;
                    }
                }
            }
        }
    }
}
