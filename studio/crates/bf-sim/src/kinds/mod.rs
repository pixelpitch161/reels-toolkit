//! The weapon kinds, one module each: free functions taking `&mut Fight`, ports of the `fight.py` methods of the
//! same name. [`crate::schedule`] calls them, in the order `Fight.run()` does; a kind's own state hangs off
//! [`crate::Fight`] as an `Option` per fighter.

pub mod striker;
