//! BASSOON's panel on the VS card: `title.py` `_bassoon`, flamed maple, twin bores and the crook.
//!
//! Not ported yet. It comes with BASSOON's look (Task 19 of milestone 3), which fills this file and sets [`PORTED`].
//! While [`PORTED`] is false a card for BASSOON is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_bassoon` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_bassoon`.
pub const PORTED: bool = false;

/// `_bassoon(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
