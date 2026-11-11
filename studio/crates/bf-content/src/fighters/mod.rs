//! The fighters: one folder per ported fighter. So far each holds its instrument voice (`voice.rs`); its look
//! and its part in the score join it as they are ported.

use std::sync::Arc;

use bf_sound::voice::Voice;

pub mod flute;
pub mod trombone;

/// The instrument voice of the fighter `fighter` (its name in the roster, such as `FLUTE`), or `None` for a
/// fighter whose voice is not ported. Every fighter the app offers ([`crate::PORTED`]) has one.
pub fn voice(fighter: &str) -> Option<Arc<dyn Voice>> {
    match fighter {
        "FLUTE" => Some(Arc::new(flute::FluteVoice)),
        "TROMBONE" => Some(Arc::new(trombone::TromboneVoice)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PORTED, RETIRED, roster};

    #[test]
    fn every_ported_fighter_has_a_voice() {
        for name in PORTED {
            assert!(voice(name).is_some(), "{name} has no voice");
        }
        // The Pixel Pitch registers of pp_bridge's roster (not the fighters' own: TROMBONE fights from 40 to 65).
        assert_eq!(voice("FLUTE").unwrap().register(), (72, 93));
        assert_eq!(voice("TROMBONE").unwrap().register(), (45, 67));
        // Nothing else has one: not a fighter that is not ported yet, not a retired one, not another spelling.
        for f in roster() {
            assert_eq!(
                voice(&f.name).is_some(),
                PORTED.contains(&f.name.as_str()),
                "{}",
                f.name
            );
        }
        for name in RETIRED.iter().chain(&["flute", "", "KAZOO"]) {
            assert!(voice(name).is_none(), "{name:?}");
        }
    }

    #[test]
    fn a_ported_fighter_plays_its_own_roster_voice() {
        // `kit.music.note` is asked for the roster's `pp` name: for the ported fighters it is their own name.
        for name in PORTED {
            let cfg = roster().iter().find(|f| f.name == name).expect("in the roster");
            assert_eq!(cfg.pp, name);
        }
    }
}
