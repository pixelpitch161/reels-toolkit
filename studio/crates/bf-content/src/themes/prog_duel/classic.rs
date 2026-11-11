//! The `prog_duel` theme's classic arrangement, the `Bed` class of `kit/moods/prog_duel.py`: not ported yet.

use bf_sound::traits::Bed;

use crate::error::ContentError;
use crate::themes::{ThemeOptions, not_ported};

/// The classic bed for the fight's options (its port makes it).
pub(super) fn classic(options: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
    let _ = options;
    Err(not_ported("prog_duel"))
}
