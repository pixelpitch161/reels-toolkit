//! ORGAN's panel on the VS card: `title.py` `_organ_pipes`, a front of pipes, tall to short, with mouths.
//!
//! Not ported yet. It comes with ORGAN's look (Task 9 of milestone 3), which fills this file and sets [`PORTED`].
//! While [`PORTED`] is false a card for ORGAN is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_organ_pipes` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_organ_pipes`.
pub const PORTED: bool = false;

/// `_organ_pipes(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
