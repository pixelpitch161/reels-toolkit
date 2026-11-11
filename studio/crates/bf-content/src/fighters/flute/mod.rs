//! FLUTE: the fighter's instrument voice, its part in the score and its look.

pub mod look;
pub mod sound;
pub mod voice;

pub use look::{FluteLook, flute_keys, flute_sprite};
pub use sound::FluteSound;
pub use voice::FluteVoice;
