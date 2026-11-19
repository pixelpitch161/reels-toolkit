//! TURNTABLES's panel on the VS card: ballfight_turntables_vs_synth (#31 v3), `render.py` `_grooves` (81).
//!
//! The episode's own pattern, a record's grooves, which the episode registers into `title.PATTERNS`. It is not one
//! of `title.py`'s `PATTERNS` (the kit has no TURNTABLES), so it is not in this folder's table: TURNTABLES's look
//! draws it through [`crate::look::Look::title_pattern_with`] by calling [`pattern`] with its colour (the reference
//! kit module registers it as its look's `title_pattern`, which `kit.picture` adds with `PATTERNS.setdefault`).
//!
//! Pixel by pixel in the order numpy's whole-array expressions run: `r = np.hypot(XX, YY)` from the frame's top-left
//! corner, the grooves a sine of it, the track gaps every 84 px, the label inside 46 px.

use bf_core::{dmath, py};

use crate::img::RgbF;
use crate::title::layout::Layout;

/// The roster's TURNTABLES colour (lime), the label's when no look gives its own.
pub const LIME: [f64; 3] = [190.0, 245.0, 60.0];

/// The pattern as the card's table calls it: [`pattern_in`] with the roster's lime (its look calls [`pattern_in`]
/// with its own colour, Python's `COL[DJ]`). `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    pattern_in(base, LIME);
}

/// Whether [`pattern`] is the episode's.
pub const PORTED: bool = true;

/// `_grooves(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: record grooves round the far
/// corner (`0.88 + 0.14 g`, `g` a sine of the distance every 6 px), track gaps (x 1.2 within 1.6 px of every 42nd of
/// 84), a label arc inside 46 px in `col` (`base * 0.55 + clip(col) * 0.45`).
pub fn pattern_in(base: &mut RgbF, col: [f64; 3]) {
    let w = base.w;
    let lc = col.map(|c| crate::np::clip(c, 0.0, 255.0));
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        let r = crate::np::hypot(xx, yy);
        // g = 0.5 + 0.5 * np.sin(r * 2 * math.pi / 6.0)
        let g = 0.5 + 0.5 * dmath::sin(r * 2.0 * dmath::PI / 6.0);
        let gap = (py::fmod_py(r, 84.0) - 42.0).abs() < 1.6;
        let lab = r < 46.0;
        for (v, &c) in px.iter_mut().zip(&lc) {
            let mut b = *v * (0.88 + 0.14 * g);
            if gap {
                b *= 1.2;
            }
            *v = if lab { b * 0.55 + c * 0.45 } else { b };
        }
    }
}
