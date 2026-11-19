//! Ability kind `dance` (SNAKE DANCE): `fight.py` 2878 to 2879.
//!
//! For `dur` seconds (or `dur_beats` beats) the striker's cobras leave their baskets and slither after the enemy
//! (`striker_step` reads `dance_until`); then they slither home.

use crate::abilities::ab_dur;
use crate::fight::Fight;
use crate::roster::AbilityCfg;

/// `trigger`, kind `dance`: `B.dance_until = self.t + ab_dur(ab)`. Python sets it on any ball; only a striker reads
/// it, so for a fighter that is no striker this does nothing.
pub fn fire(fight: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = fight.t + ab_dur(fight, ab);
    if let Some(st) = &mut fight.striker[k] {
        st.dance_until = until;
    }
}
