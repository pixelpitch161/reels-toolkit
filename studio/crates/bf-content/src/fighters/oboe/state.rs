//! What OBOE's look and its part both derive from the recording (ruling M3-R15): nothing. `oboe.py`'s `Sound` reads
//! none of its `Look`'s state (no `M.P.LOOKS[...]`): its part works from the run's events alone, and the one thing it
//! takes from the other side is the victim's own part, which the score hands to `play_with`.
