//! OBOE's panel on the VS card: `title.py` `_oboe_reeds`, tuning forks, reed blades and A waves.
//!
//! The A is a sine running along the divider, 40 px from it; five tuning forks stand with their prongs toward the
//! divider, every 120 px along it; a double-reed blade stands between each pair of forks, wide and flat at its tip,
//! narrow at its green thread. All of it lies below the divider (`u = -SIDE`): the pattern was drawn for the right
//! fighter's half, and on the left half, where `u` is below zero, it draws nothing.

use bf_core::dmath;

use crate::config::W;
use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_oboe_reeds`.
pub const PORTED: bool = true;

/// `np.where(mask[..., None], out * keep + col * (1 - keep), out)` on one pixel.
fn tint(px: &mut [f64; 3], on: bool, keep: f64, col: [f64; 3], k: f64) {
    if on {
        for (v, c) in px.iter_mut().zip(col) {
            *v = *v * keep + c * k;
        }
    }
}

/// `_oboe_reeds(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`; this pattern reads none of it).
pub fn pattern(base: &mut RgbF, _card: &Layout) {
    let side = layout::side();
    let dd = layout::dd();
    for (i, (px, &s)) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data).enumerate() {
        let (xx, yy) = ((i % W) as f64, (i / W) as f64);
        // along = (XX * DD[0] + YY * DD[1]); u = -SIDE; out = base * 1.0
        let along = xx * dd[0] + yy * dd[1];
        let u = -s;
        for v in px.iter_mut() {
            *v *= 1.0;
        }
        // the A: a sine running along the divider
        let w = (u - 40.0 - 6.0 * dmath::sin(along / 9.0)).abs() < 1.2;
        tint(px, w, 0.6, [250.0, 240.0, 170.0], 0.4);
        // tuning forks, prongs toward the divider
        for j in 0..5 {
            let a0 = -390.0 + 120.0 * f64::from(j);
            let q = along - a0;
            let stem = q.abs() < 1.6 && u > 96.0 && u < 150.0;
            let prong = ((q - 5.0).abs() < 1.6 || (q + 5.0).abs() < 1.6) && u > 64.0 && u <= 98.0;
            let bow = q.abs() < 6.6 && (u - 97.0).abs() < 1.6;
            tint(px, stem || prong || bow, 0.58, [236.0, 236.0, 214.0], 0.42);
        }
        // double-reed blades between the forks
        for j in 0..5 {
            let q = along - (-330.0 + 120.0 * f64::from(j));
            // wide, flat tip; narrow at the thread
            let hw = (1.0 - np::clip((u - 96.0) / 46.0, 0.0, 1.0)) * 6.5 + 2.0;
            let blade = q.abs() < hw && u > 96.0 && u < 142.0;
            let thread = q.abs() < 2.6 && (142.0..160.0).contains(&u);
            tint(px, blade, 0.5, [232.0, 208.0, 150.0], 0.5);
            tint(px, thread, 0.5, [60.0, 150.0, 86.0], 0.5);
        }
    }
}
