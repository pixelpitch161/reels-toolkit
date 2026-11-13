//! SYNTH's panel on the VS card: ballfight_turntables_vs_synth (#31 v3), `render.py` `_circuit` (92): circuit traces
//! (right-angle runs) and solder pads, on a 24 px grid.
//!
//! The episode's own pattern, which the episode registers into `title.PATTERNS`. It is not one of `title.py`'s
//! `PATTERNS` (the kit has no SYNTH), so it is not in this folder's table: SYNTH's look draws it through
//! [`crate::look::Look::title_pattern_with`] by calling [`pattern`] (the reference kit module registers it as its
//! look's `title_pattern`, which `kit.picture` adds with `PATTERNS.setdefault`).
//!
//! Pixel by pixel in the order numpy's whole-array expressions run; the one function is `np.hypot`
//! (`dmath::np_hypot`).

use bf_core::{dmath, py};

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the episode's.
pub const PORTED: bool = true;

/// `_circuit(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place. Each 24 px cell runs a trace
/// across its middle, along the row or the column by a hash of the cell (`(gx 7.13 + gy 3.71) % 1`), 1.22 x the
/// panel; a cell whose hash is above 0.72 has a pad of radius 2.6 px at its centre, 1.4 x. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        let (gx, gy) = ((xx / 24.0).floor(), (yy / 24.0).floor());
        let h = py::fmod_py(gx * 7.13 + gy * 3.71, 1.0);
        let (fx, fy) = (py::fmod_py(xx, 24.0), py::fmod_py(yy, 24.0));
        let trace = (h < 0.5 && (fy - 12.0).abs() < 1.0) || (h >= 0.5 && (fx - 12.0).abs() < 1.0);
        let pad = dmath::np_hypot(fx - 12.0, fy - 12.0) < 2.6 && h > 0.72;
        for v in px {
            let b = if trace { *v * 1.22 } else { *v };
            *v = if pad { b * 1.4 } else { b };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traces_and_pads_lie_on_the_grid() {
        let card = Layout::plain("TURNTABLES", "SYNTH");
        let (w, h) = (48usize, 48usize);
        let mut base = RgbF {
            w,
            h,
            c: 3,
            data: vec![100.0; w * h * 3],
        };
        pattern(&mut base, &card);
        let at = |x: usize, y: usize| base.data[(y * w + x) * 3];
        // cell (0, 0): h = 0 -> a row trace at y 11..13, no pad
        assert_eq!((at(5, 12), at(5, 5)), (122.0, 100.0));
        assert!(base.data.iter().all(|v| [100.0, 122.0, 140.0, 122.0 * 1.4].contains(v)));
    }
}
