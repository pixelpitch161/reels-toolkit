//! The `dark_breakbeat` theme's classic arrangement, the `Bed` class of its reference mood: not ported yet (Task
//! 23).

use bf_sound::traits::Bed;

use crate::error::ContentError;
use crate::themes::{ThemeOptions, not_ported};

/// The classic bed for the fight's options (its port makes it).
pub(super) fn classic(options: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
    let _ = options;
    Err(not_ported("dark_breakbeat"))
}
