//! OBOE's panel on the VS card: `title.py` `_oboe_reeds`, tuning forks, reed blades and A waves.
//!
//! Not ported yet. It comes with OBOE's look (Task 11 of milestone 3), which fills this file and sets [`PORTED`].
//! While [`PORTED`] is false a card for OBOE is refused ([`crate::title::TitleError::PatternNotPorted`]) and
//! [`pattern`] is never called. Porting it: the body of `_oboe_reeds` here, `PORTED = true`, and nothing else (the
//! table in `patterns/mod.rs` already names this file).

use crate::img::RgbF;
use crate::title::layout::Layout;

/// Whether [`pattern`] is the kit's `_oboe_reeds`.
pub const PORTED: bool = false;

/// `_oboe_reeds(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`). Not ported: does nothing.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = (base, card);
}
