//! HORN's panel on the VS card: `title.py` `_horn_coils`, tubing coiled round the panel's big ball (it reads the card's balls).
//!
//! Not ported yet. It comes with HORN's look (Task 21 of milestone 3), which fills this file and sets [`PORTED`].
//! While [`PORTED`] is false a card for HORN is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_horn_coils` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_horn_coils`.
pub const PORTED: bool = false;

/// `_horn_coils(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
