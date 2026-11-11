//! BAGPIPES's panel on the VS card: `title.py` `_tartan`, a large tartan sett, low contrast.
//!
//! Not ported yet. BAGPIPES is one of the older fighters (milestone 4): this stays a stub until then.
//! While [`PORTED`] is false a card for BAGPIPES is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_tartan` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_tartan`.
pub const PORTED: bool = false;

/// `_tartan(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
