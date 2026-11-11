//! The fighters: one folder per ported fighter. So far each holds its instrument voice (`voice.rs`) and its
//! part in the score (`sound.rs`); its look joins them as it is ported.

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_sound::traits::Sound;
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

/// The part in the score of the fighter `fighter` on side `k` (0 is the left corner) of the fight `ff`: the
/// kit's `fighters.sound(name, M, k)`. `None` for a fighter whose part is not ported. Every fighter the app
/// offers ([`crate::PORTED`]) has one.
pub fn sound(fighter: &str, k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    match fighter {
        "FLUTE" => Some(Box::new(flute::FluteSound::new(k))),
        "TROMBONE" => {
            // Its role (`T.ROLES.get(name, 'melody')`) sets the band it is checked in.
            let role = ff.music.roles.get(fighter).map_or("melody", String::as_str);
            Some(Box::new(trombone::TromboneSound::new(k, role)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PORTED, RETIRED, roster};

    #[test]
    fn every_ported_fighter_has_a_part() {
        let ff = FightFile::parse(include_str!("../../../../fights/flute_vs_trombone.toml")).expect("parses");
        for f in roster() {
            for k in 0..2 {
                let part = sound(&f.name, k, &ff);
                assert_eq!(part.is_some(), PORTED.contains(&f.name.as_str()), "{}", f.name);
                if let Some(part) = part {
                    assert_eq!(part.k(), k, "{}", f.name);
                }
            }
        }
        for name in RETIRED.iter().chain(&["flute", "", "KAZOO"]) {
            assert!(sound(name, 0, &ff).is_none(), "{name:?}");
        }
        // The class attributes of the two parts (kit/fighters/flute.py and trombone.py, `class Sound`).
        let flute = sound("FLUTE", 0, &ff).unwrap();
        assert_eq!(
            (flute.bus_gain(), flute.send(), flute.band()),
            (1.7, 0.45, (600.0, 3200.0))
        );
        let trombone = sound("TROMBONE", 1, &ff).unwrap();
        assert_eq!(
            (trombone.bus_gain(), trombone.send(), trombone.band()),
            (1.8, 0.2, (70.0, 700.0))
        );
        // #48: as a riff over someone else's bass the trombone is checked in the riff's band.
        let mut riff = ff.clone();
        riff.music.roles.insert("TROMBONE".into(), "riff".into());
        assert_eq!(sound("TROMBONE", 1, &riff).unwrap().band(), (120.0, 1500.0));
        riff.music.roles.clear();
        assert_eq!(sound("TROMBONE", 1, &riff).unwrap().band(), (70.0, 700.0));
    }

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
            assert_eq!(voice(name).expect("a voice").name(), cfg.pp);
        }
    }
}
