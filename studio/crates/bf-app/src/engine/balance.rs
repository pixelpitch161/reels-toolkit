//! Balancing: `AutoBalance`, `Measure` and `StopBalance`.
//!
//! The coordinator (`super`) routes these commands here. Task 29 fills this file; until then every command is
//! ignored.

use super::Ctx;
use crate::seam::Command;

/// What this handler keeps between commands (cancel flags, caches).
#[derive(Default)]
pub struct State;

/// Takes one command and returns at once; long work goes to its own thread (`Ctx::spawn`).
pub fn handle(state: &mut State, ctx: &Ctx, command: Command) {
    let _ = (state, ctx, command);
}
