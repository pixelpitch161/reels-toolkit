//! What CELLO's look and its part both derive from the recording: nothing shared. The look reads its own events
//! and the record's `GRAB` rows (`look.rs`); the part reads its own events (`sound.rs`); neither reads the other's
//! state (ruling M3-R15 needs no recomputation here).
