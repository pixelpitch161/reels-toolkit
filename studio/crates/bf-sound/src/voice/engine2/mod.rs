//! Voice Engine 2: notes rebuilt from recordings of the real instrument, with the instrument's working parts
//! modelled (the Composed arrangement only; design: `docs/studio/2026-11-26-voice-engine-2-design.md`).
//!
//! Each recorded note of a free (CC0) recording set is measured once, by the `recipes` tool in `bf-harness`, into a
//! recipe: how loud each overtone is at each moment, how far off pitch it sits, and how loud the noise between the
//! overtones is in 24 bands ([`recipe`]). A note is rebuilt from the recipes of the recorded notes nearest it in
//! pitch and strength. What a recording can't hold, the instrument's working parts give: a piano's dampers and pedal,
//! an organ's wind and its stops coming in.
//!
//! - [`recipe`]: the recipe format.
//! - [`player`]: a note rebuilt from a recipe.
//! - [`play`]: how a note is played (accents, slurs, swells, the pedal...), for the parts and the composition system.

pub mod play;
pub mod player;
pub mod recipe;
