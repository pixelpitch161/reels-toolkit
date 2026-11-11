//! The `baroque_storm` theme's classic arrangement, the `Bed` class of `kit/moods/baroque_storm.py`: not ported yet.

use bf_sound::traits::Bed;

use crate::error::ContentError;
use crate::themes::{ThemeOptions, not_ported};

/// The classic bed for the fight's options (its port makes it).
pub(super) fn classic(options: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
    let _ = options;
    Err(not_ported("baroque_storm"))
}
