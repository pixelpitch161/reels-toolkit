//! The content registry (Task 7): the roster and which fighters the app offers.
//!
//! The roster is `data/roster.json` (exported by `tools/studio_export/roster.py`), compiled into the app and read
//! once. The retired fighters (`RETIRED`) are never in it, never offered and refused in a fight file.

use std::sync::OnceLock;

use bf_core::fightfile::NameStatus;
use bf_core::roster::{FighterCfg, parse_roster};

pub use bf_core::fightfile::RETIRED;

/// The exported roster.
const ROSTER_JSON: &str = include_str!("../data/roster.json");

/// The fighters whose weapon and abilities the engine has ported, in no particular order (grows as fighters are
/// ported).
pub const PORTED: [&str; 2] = ["FLUTE", "TROMBONE"];

/// Every kit fighter's roster entry, in `fighters.py`'s order, every number as exported.
///
/// # Panics
/// When the compiled-in roster cannot be read (a broken export; the tests read it on every build).
pub fn roster() -> &'static [FighterCfg] {
    static ROSTER: OnceLock<Vec<FighterCfg>> = OnceLock::new();
    ROSTER.get_or_init(|| parse_roster(ROSTER_JSON).unwrap_or_else(|e| panic!("the compiled-in {e}")))
}

/// The fighters the app offers: [`PORTED`], in roster order.
pub fn available() -> Vec<&'static str> {
    roster()
        .iter()
        .map(|f| f.name.as_str())
        .filter(|n| PORTED.contains(n))
        .collect()
}

/// Whether a fighter's music settings (key, chord loop, music seed, its line) feed its fight: true for a fighter
/// whose notes are its weapon (PIANO's roll, once it is ported). False for every fighter in Milestone 1.
pub fn music_feeds_fight(name: &str) -> bool {
    crate::fighters::music_feeds_fight(name)
}

/// What the registry says about a fighter name, for `FightFile::validate`: `Retired` for [`RETIRED`], `Known` for a
/// roster fighter (ported or not: [`available`] is what the app offers), `Unknown` for anything else.
pub fn name_status(name: &str) -> NameStatus {
    if RETIRED.contains(&name) {
        NameStatus::Retired
    } else if roster().iter().any(|f| f.name == name) {
        NameStatus::Known
    } else {
        NameStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_core::roster::{AbilityKind, WeaponCfg};

    /// The 20 kit fighters of spec section 10.
    const KIT: [&str; 20] = [
        "ACOUSTIC",
        "ALTO SAX",
        "BASS CLARINET",
        "BASS DRUM",
        "BASSOON",
        "CELLO",
        "CLARINET",
        "CONTRABASS",
        "ELECTRIC",
        "EUPHONIUM",
        "FLUTE",
        "HORN",
        "OBOE",
        "ORGAN",
        "PIANO",
        "SOUSAPHONE",
        "TENOR SAX",
        "TROMBONE",
        "TRUMPET",
        "TUBA",
    ];

    #[test]
    fn roster_has_20_and_no_retired() {
        let r = roster();
        assert_eq!(r.len(), 20);
        let mut names: Vec<&str> = r.iter().map(|f| f.name.as_str()).collect();
        // fighters.py's order, which starts with the first fighters the kit had
        assert_eq!(&names[..4], &["PIANO", "TUBA", "FLUTE", "TROMBONE"]);
        names.sort_unstable();
        assert_eq!(names, KIT);
        for f in r {
            assert!(!RETIRED.contains(&f.name.as_str()), "{} is retired", f.name);
        }
        assert_eq!(RETIRED, ["GUITAR", "SAX"]);
        // parsed once: the same slice every call
        assert!(std::ptr::eq(roster(), roster()));
    }

    #[test]
    fn available_is_flute_and_trombone() {
        // roster order: FLUTE comes before TROMBONE in fighters.py
        assert_eq!(available(), vec!["FLUTE", "TROMBONE"]);
    }

    #[test]
    fn ported_fighters_have_supported_kinds() {
        for name in PORTED {
            let f = roster()
                .iter()
                .find(|f| f.name == name)
                .expect("a ported fighter is in the roster");
            assert!(!matches!(f.weapon, WeaponCfg::Unsupported(_)), "{name}: {:?}", f.weapon);
            for ab in [&f.unlock1, &f.unlock2, &f.low] {
                assert!(
                    !matches!(ab.kind, AbilityKind::Unsupported(_)),
                    "{name}: {} {:?}",
                    ab.name,
                    ab.kind
                );
            }
        }
    }

    #[test]
    fn the_registry_names_fighters_for_fight_files() {
        assert_eq!(name_status("FLUTE"), NameStatus::Known);
        assert_eq!(name_status("TROMBONE"), NameStatus::Known);
        assert_eq!(name_status("KAZOO"), NameStatus::Unknown);
        assert_eq!(name_status("flute"), NameStatus::Unknown);
        // in the roster but not ported yet: a fight file may name it (the app does not offer it, see available())
        assert_eq!(name_status("PIANO"), NameStatus::Known);
        for r in RETIRED {
            assert_eq!(name_status(r), NameStatus::Retired);
        }
        // ALTO SAX, TENOR SAX and BARI SAX are different fighters
        assert_ne!(name_status("ALTO SAX"), NameStatus::Retired);
        assert_ne!(name_status("BARI SAX"), NameStatus::Retired);
    }

    #[test]
    fn music_feeds_no_fight_in_m1() {
        for f in roster() {
            assert!(!music_feeds_fight(&f.name), "{}", f.name);
        }
    }

    #[test]
    fn the_example_fight_file_validates_against_the_registry() {
        let text = include_str!("../../../fights/flute_vs_trombone.toml");
        let ff = bf_core::fightfile::FightFile::parse(text).expect("parses");
        assert_eq!(ff.validate(&name_status), Ok(()));
        let feeds: Vec<bool> = ff.fighters.iter().map(|n| music_feeds_fight(n)).collect();
        assert_eq!(feeds, [false, false]);
        assert_eq!(ff.pair_rules, crate::pair_rules(&ff.fighters[0], &ff.fighters[1]));
        let mut kazoo = ff.clone();
        kazoo.fighters[0] = "KAZOO".into();
        assert_eq!(
            kazoo.validate(&name_status).unwrap_err().to_string(),
            r#"fighters[0]: "KAZOO" is not in the roster"#
        );
    }
}
