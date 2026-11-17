//! An [`Engine`] in the app's own files as the fight file writes it: the number 1 or 2 (Engine 2 E6, ruling R6).
//! Used with `#[serde(default, skip_serializing_if = "Engine::is_one", with = "crate::model::engine_number")]`, so a
//! current-engine row has no `engine` key and every file written before Engine 2 reads and writes as it did.

use bf_core::Engine;
use serde::{Deserialize, Deserializer, Serializer, de};

/// Writes the engine's number.
pub fn serialize<S: Serializer>(engine: &Engine, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u32(engine.number())
}

/// Reads 1 or 2; any other number is refused in words.
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Engine, D::Error> {
    let n = u32::deserialize(d)?;
    Engine::from_number(n).ok_or_else(|| {
        de::Error::custom(format!(
            "engine must be 1 (the current engine) or 2 (Engine 2), found {n}"
        ))
    })
}
