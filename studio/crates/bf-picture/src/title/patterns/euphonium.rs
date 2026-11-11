//! EUPHONIUM's panel on the VS card: `title.py` `_euphonium`, a big upright bell and four valves.
//!
//! Not ported yet. It comes with EUPHONIUM's look (Task 26 of milestone 3), which fills this file and sets [`PORTED`].
//! While [`PORTED`] is false a card for EUPHONIUM is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_euphonium` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_euphonium`.
pub const PORTED: bool = false;

/// `_euphonium(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
