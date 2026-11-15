//! HARP's panel on the VS card: `title.py` `_harp_strings`, fine strings fanning across from below the panel, pearl
//! glints. Pixel by pixel; the one transcendental is the angle (`bf_core::dmath::atan2` where Python has numpy's).

use bf_core::{dmath, py};

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_harp_strings`.
pub const PORTED: bool = true;

/// `_harp_strings(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: strings fanning out from
/// (520, 900), 80 to the radian, each 0.12 of its spacing wide, mixed 22 % toward pearl; between them the panel is
/// 6 % darker. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    const PEARL: [f64; 3] = [255.0, 246.0, 222.0];
    let w = base.w;
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // a = np.arctan2(YY - 900.0, XX - 520.0);  s = (a * 80.0) % 1.0
        let s = py::fmod_py(dmath::atan2(yy - 900.0, xx - 520.0) * 80.0, 1.0);
        let line = s < 0.12;
        for (v, &p) in px.iter_mut().zip(&PEARL) {
            *v = if line { *v * 0.78 + p * 0.22 } else { *v * 0.94 };
        }
    }
}
