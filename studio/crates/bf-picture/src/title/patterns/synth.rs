//! SYNTH's panel on the VS card: ballfight_turntables_vs_synth (#31 v3), `render.py` `_circuit` (92).
//!
//! The episode's own pattern, a circuit board, which the episode registers into `title.PATTERNS`. It is not one of
//! `title.py`'s `PATTERNS` (the kit has no SYNTH), so it is not in this folder's table: once ported, SYNTH's look
//! draws it through [`crate::look::Look::title_pattern_with`] by calling [`pattern`] (the reference kit module
//! registers it as its look's `title_pattern`, which `kit.picture` adds with `PATTERNS.setdefault`).
//!
//! Not ported yet: SYNTH is one of the older fighters (milestone 4), and this stays a stub until its port. Porting
//! it: the body here and `PORTED = true`, and nothing else.

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the episode's.
pub const PORTED: bool = false;

/// The panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout. Not ported: does
/// nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
