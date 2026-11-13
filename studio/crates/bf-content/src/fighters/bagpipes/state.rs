//! What BAGPIPES's look and its part both derive from the recording (ruling M3-R15): the BAG INFLATE cycles (#30
//! render.py `_cycles`, from the run's `inflate`, `release` and `land` events, a cycle the K.O. interrupted ending
//! 10 s after it) and the bag's radius (`RAD`). The kit module rebuilds them in its `Look.setup` and again in its
//! `Sound.play`; so do `look.rs` (`BagpipesLook::setup_as`) and `sound.rs` (`BagpipesSound::cycles`, `radii`), each
//! with the fields it reads.
