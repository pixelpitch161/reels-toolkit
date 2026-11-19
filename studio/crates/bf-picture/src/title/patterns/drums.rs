//! DRUMS's panel on the VS card: ballfight_drums_vs_violin (#28).
//!
//! No pattern of its own was found in the episode (its VS card predates the panel patterns): the plain panel unless
//! the port finds one. It is not one of `title.py`'s `PATTERNS` (the kit has no DRUMS), so it is not in this
//! folder's table: once ported, DRUMS's look draws it through [`crate::look::Look::title_pattern_with`] by calling
//! [`pattern`] (the reference kit module registers it as its look's `title_pattern`, which `kit.picture` adds with
//! `PATTERNS.setdefault`).
//!
//! Not ported yet: DRUMS is one of the older fighters (milestone 4), and this stays a stub until its port. Porting
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
