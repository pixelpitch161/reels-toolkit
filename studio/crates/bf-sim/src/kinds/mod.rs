//! The weapon kinds, one module each: free functions taking `&mut Fight`, ports of the `fight.py` methods of the
//! same name. [`crate::schedule`] calls them, in the order `Fight.run()` does. A kind's own state hangs off
//! [`crate::Fight`] as an `Option` per fighter, or is on the ball where `Ball.__init__` puts it (the thrust blade's).

pub mod blade;
pub mod striker;
