//! The weapon kinds, one module each: free functions taking `&mut Fight`, ports of the `fight.py` methods of the
//! same name. [`crate::schedule`] calls them, in the order `Fight.run()` does. A kind's own state hangs off
//! [`crate::Fight`] as an `Option` per fighter, or is on the ball where `Ball.__init__` puts it (the thrust blade's).
//!
//! `striker` (FLUTE) and `blade` (TROMBONE's thrust lance) are ported. Every other weapon kind of the kit has its
//! module here, owned by its fighter's port: a stub until then, already called from every place `fight.py` calls
//! it (`schedule.rs`, `fight.rs`, `record.rs`, `abilities/mod.rs`), behind its `is`, which no fighter passes before
//! the roster reads the kind.

pub mod anchor;
pub mod bite;
pub mod blade;
pub mod combo;
pub mod crowd;
pub mod drill;
pub mod echo;
pub mod grapple;
pub mod inflate;
pub mod jump;
pub mod lightning;
pub mod line;
pub mod mortar;
pub mod pulse;
pub mod rake;
pub mod rests;
pub mod roll;
pub mod roll_charge;
pub mod scratch_floor;
pub mod spotlight;
pub mod striker;
pub mod surf;
pub mod swallow;
pub mod tether;
pub mod tiptoe;
pub mod trap;
pub mod tune;
pub mod walk;
pub mod wrecking;
