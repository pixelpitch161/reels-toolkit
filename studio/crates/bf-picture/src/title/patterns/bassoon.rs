//! BASSOON's panel on the VS card: `title.py` `_bassoon`, flamed maple, twin bores and the crook.
//!
//! Pixel by pixel in the order numpy's whole-array expressions run. The transcendentals are the two sines of the
//! maple flame (`bf_core::dmath::sin` where Python has numpy's), which only decide on which side of 0.5 and -0.6 a
//! pixel's flame value lies, and the two distances (`np.hypot`, the plain root), which only meet 4.2 and 44 +- 4.5.

use bf_core::dmath;

use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_bassoon`.
pub const PORTED: bool = true;

/// `out * a + col * b` on one pixel.
fn mix(px: &mut [f64; 3], a: f64, col: [f64; 3], b: f64) {
    for (v, c) in px.iter_mut().zip(col) {
        *v = *v * a + c * b;
    }
}

/// `_bassoon(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: the maple's flame across the
/// grain, the twin bores along the divider (it works above the divider, the left fighter's side) with their
/// silver key rods and touches, and the bocal, a silver crook arcing over the top corner with its cane reed. `card`
/// is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    let side = layout::side();
    let dd = layout::dd();
    let (cx, cy) = (248.0, 96.0);
    for (i, (px, &u)) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data).enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // g = np.sin((XX * 0.25 + YY * 0.97) / 3.2 + 2.0 * np.sin(XX / 23.0 + YY / 61.0))
        let g = dmath::sin((xx * 0.25 + yy * 0.97) / 3.2 + 2.0 * dmath::sin(xx / 23.0 + yy / 61.0));
        // out = base * (0.92 + 0.10 * (g > 0.5) - 0.07 * (g < -0.6))
        let flame = 0.92 + 0.10 * f64::from(u8::from(g > 0.5)) - 0.07 * f64::from(u8::from(g < -0.6));
        for v in px.iter_mut() {
            *v *= flame;
        }
        let along = xx * dd[0] + yy * dd[1];
        // the twin bores (long joint + wing) along the divider
        for off in [54.0, 92.0] {
            if ((u - off).abs() - 16.0).abs() < 1.2 {
                for v in px.iter_mut() {
                    *v *= 0.72;
                }
            }
            if (u - off + 9.0).abs() < 1.0 {
                mix(px, 0.75, [255.0, 236.0, 220.0], 0.25);
            }
        }
        // silver key rods + touches on the bores
        for j in 0..7 {
            let a0 = -330.0 + 95.0 * f64::from(j);
            let rod = (u - 73.0).abs() < 0.9 && (along - a0).abs() < 36.0;
            let dot = np::hypot(along - a0, u - 54.0) < 4.2;
            if rod || dot {
                mix(px, 0.5, [226.0, 228.0, 240.0], 0.5);
            }
        }
        // the bocal: a silver crook arcing over the top corner
        let r = np::hypot(xx - cx, yy - cy);
        let arc = (r - 44.0).abs() < 4.5 && yy < cy + 8.0;
        let slope = (yy - cy) - (xx - cx - 44.0) * 0.55;
        let tail = slope.abs() < 4.5 && xx > cx + 40.0 && xx < cx + 110.0 && yy > cy - 2.0;
        let stem = (xx - (cx - 44.0)).abs() < 4.5 && yy > cy && yy < cy + 70.0;
        if arc || tail || stem {
            mix(px, 0.4, [216.0, 220.0, 232.0], 0.6);
        }
        let reed = slope.abs() < 6.0 && xx >= cx + 110.0 && xx < cx + 128.0;
        if reed {
            mix(px, 0.4, [232.0, 208.0, 150.0], 0.6);
        }
    }
}
