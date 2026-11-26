//! BARI SAX's panel on the VS card: `title.py` `_barisax`, pearl key touches and the pigtail loop.
//!
//! Pixel by pixel in the order numpy's whole-array expressions run. The distances are `np.hypot` (the plain root),
//! which only decide on which side of a threshold a pixel lies.

use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_barisax`.
pub const PORTED: bool = true;

/// `out * a + col * b` on one pixel.
fn mix(px: &mut [f64; 3], a: f64, col: [f64; 3], b: f64) {
    for (v, c) in px.iter_mut().zip(col) {
        *v = *v * a + c * b;
    }
}

/// `_barisax(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: the main stack's cups and pearl
/// touches along the divider (it works below the divider, the right fighter's side) and the pigtail, the tube coiling
/// once in the far corner, with its highlight. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    let side = layout::side();
    let dd = layout::dd();
    let (cx, cy) = (92.0, 560.0);
    for (i, (px, &s)) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data).enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // out = base * 1.0: the same values
        let along = xx * dd[0] + yy * dd[1];
        let u = -s;
        for j in 0..6 {
            let a0 = -250.0 + 62.0 * f64::from(j);
            let r = np::hypot(along - a0, u - 58.0);
            if r < 15.0 {
                mix(px, 0.72, [255.0, 214.0, 150.0], 0.28);
            }
            if (r - 15.0).abs() < 1.6 {
                mix(px, 0.45, [255.0, 226.0, 170.0], 0.55);
            }
            if np::hypot(along - a0, u - 96.0) < 7.0 {
                mix(px, 0.35, [250.0, 244.0, 232.0], 0.65);
            }
            if (along - a0).abs() < 1.4 && u > 73.0 && u < 90.0 {
                mix(px, 0.6, [255.0, 226.0, 170.0], 0.4);
            }
        }
        // the pigtail: the tube coiling once in the far corner
        let r = np::hypot(xx - cx, yy - cy);
        let looped = (r - 34.0).abs() < 6.0;
        let neck = (yy - (cy - 34.0) + (xx - cx) * 0.18).abs() < 5.0 && xx < cx && xx > cx - 92.0;
        if looped || neck {
            mix(px, 0.45, [255.0, 206.0, 120.0], 0.55);
        }
        if (r - 31.0).abs() < 1.2 && yy < cy {
            mix(px, 0.3, [255.0, 250.0, 230.0], 0.7);
        }
    }
}
