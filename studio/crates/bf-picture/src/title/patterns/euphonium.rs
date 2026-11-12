//! EUPHONIUM's panel on the VS card: `title.py` `_euphonium`, a big upright bell and four valves.
//!
//! The bell's mouth, seen from just above in the far corner: a darker inside shading to its middle, a bright brass
//! rim, the bell's flare widening up to it; four valve caps in a row along the divider on the left fighter's side,
//! each a brass cap, a bright ring and a pearl button on a stem.

use bf_core::dmath;

use crate::config::W;
use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_euphonium`.
pub const PORTED: bool = true;

/// `np.where(mask[..., None], out * keep + col * k, out)` on one pixel.
fn tint(px: &mut [f64; 3], on: bool, keep: f64, col: [f64; 3], k: f64) {
    if on {
        for (v, c) in px.iter_mut().zip(col) {
            *v = *v * keep + c * k;
        }
    }
}

/// `_euphonium(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`; this pattern reads none of it).
pub fn pattern(base: &mut RgbF, _card: &Layout) {
    let side = layout::side();
    let dd = layout::dd();
    let (cx, cy) = (300.0, 560.0);
    for (i, (px, &s)) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data).enumerate() {
        let (xx, yy) = ((i % W) as f64, (i / W) as f64);
        // e = ((XX - cx) / 120.0) ** 2 + ((YY - cy) / 46.0) ** 2 (numpy squares an array by multiplying)
        let (a, b) = ((xx - cx) / 120.0, (yy - cy) / 46.0);
        let e = a * a + b * b;
        let rim = (e - 1.0).abs() < 0.10;
        let inner = e < 0.9;
        if inner {
            let k = 0.80 + 0.12 * np::clip(1.0 - e, 0.0, 1.0);
            for v in px.iter_mut() {
                *v *= k;
            }
        }
        tint(px, rim, 0.45, [255.0, 222.0, 140.0], 0.55);
        // flare = (|XX - cx| < 26 + 90 * clip((YY - 330) / 230, 0, 1) ** 3) & (YY > 330) & (YY < cy) & ~inner
        let wide = 26.0 + 90.0 * dmath::pow(np::clip((yy - 330.0) / 230.0, 0.0, 1.0), 3.0);
        let flare = (xx - cx).abs() < wide && yy > 330.0 && yy < cy && !inner;
        if flare {
            for v in px.iter_mut() {
                *v = *v * 1.10 + 6.0;
            }
        }
        // four valve caps in a row along the divider
        let along = xx * dd[0] + yy * dd[1];
        let u = -s;
        for j in 0..4 {
            let a0 = -150.0 + 46.0 * f64::from(j);
            let r = np::hypot(along - a0, u - 64.0);
            tint(px, r < 13.0, 0.7, [255.0, 214.0, 120.0], 0.3);
            tint(px, (r - 13.0).abs() < 1.6, 0.45, [255.0, 236.0, 170.0], 0.55);
            tint(px, r < 6.0, 0.4, [250.0, 244.0, 226.0], 0.6);
            let stem = (along - a0).abs() < 2.0 && u > 77.0 && u < 112.0;
            tint(px, stem, 0.6, [255.0, 214.0, 120.0], 0.4);
        }
    }
}
