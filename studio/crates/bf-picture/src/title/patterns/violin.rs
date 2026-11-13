//! VIOLIN's panel on the VS card: `title.py` `_violin`, scrolls, f-holes and flamed maple.
//!
//! Not ported yet. VIOLIN is one of the older fighters (milestone 4): this stays a stub until then.
//! While [`PORTED`] is false a card for VIOLIN is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_violin` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_violin`.
pub const PORTED: bool = false;

/// `_violin(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
