//! What ORGAN's look and its part both derive from the recording (ruling M3-R15): nothing. The kit's `Sound` for
//! the ORGAN reads only the run's events (`line`, `hit`, `ability`, `slam`) and never its look's state; the look
//! rebuilds its pipe rows from the same events on its own (`look.rs`).
