//! The content registry (Task 7): the roster and which fighters the app offers.
//!
//! The roster is `data/roster.json` (exported by `tools/studio_export/roster.py`), compiled into the app and read
//! once. It holds the kit's 20 fighters and the ten older fighters of milestone 4; the app offers only those in
//! [`PORTED`] ([`available`]), so an older fighter is in the roster (its port can load its entry) long before it is
//! offered. The retired fighters (`RETIRED`) are never in it, never offered and refused in a fight file.

use std::sync::OnceLock;

use bf_core::fightfile::NameStatus;
use bf_core::roster::{FighterCfg, parse_roster};

pub use bf_core::fightfile::RETIRED;

/// The exported roster.
const ROSTER_JSON: &str = include_str!("../data/roster.json");

/// The fighters whose weapon and abilities the engine has ported, in no particular order (grows as fighters are
/// ported).
pub const PORTED: [&str; 20] = [
    "FLUTE",
    "TROMBONE",
    "PIANO",
    "TRUMPET",
    "ORGAN",
    "CLARINET",
    "OBOE",
    "SOUSAPHONE",
    "TENOR SAX",
    "ALTO SAX",
    "BASS CLARINET",
    "TUBA",
    "CONTRABASS",
    "BASSOON",
    "HORN",
    "CELLO",
    "ACOUSTIC",
    "ELECTRIC",
    "BASS DRUM",
    "EUPHONIUM",
];

/// Every roster entry (the kit's 20 and the older ten, offered or not), in `fighters.py`'s order, every number as
/// exported. What the app offers is [`available`].
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
/// whose notes are its weapon (PIANO's roll). False for every other fighter.
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
    fn roster_has_the_kit_20_and_the_older_10_and_no_retired() {
        let r = roster();
        assert_eq!(r.len(), 30);
        // fighters.py's order, which starts with the first fighters the kit had
        let names: Vec<&str> = r.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(&names[..4], &["PIANO", "DRUMS", "VIOLIN", "BAGPIPES"]);
        let (older, mut kit): (Vec<&str>, Vec<&str>) = names.iter().partition(|n| crate::fighters::OLDER.contains(n));
        assert_eq!(&kit[..4], &["PIANO", "TUBA", "FLUTE", "TROMBONE"]);
        kit.sort_unstable();
        assert_eq!(kit, KIT);
        let mut older_sorted = older.clone();
        older_sorted.sort_unstable();
        assert_eq!(older_sorted, crate::fighters::OLDER);
        // the older ten are in the roster for their ports, and offered only once a register task lists them
        for name in &older {
            assert_eq!(available().contains(name), PORTED.contains(name), "{name}");
        }
        for f in r {
            assert!(!RETIRED.contains(&f.name.as_str()), "{} is retired", f.name);
        }
        assert_eq!(RETIRED, ["GUITAR", "SAX"]);
        // parsed once: the same slice every call
        assert!(std::ptr::eq(roster(), roster()));
    }

    #[test]
    fn available_is_the_ported_fighters_in_roster_order() {
        // every kit fighter, in fighters.py's order whatever the order of `PORTED`; no fighter twice
        assert_eq!(
            available(),
            vec![
                "PIANO",
                "TUBA",
                "FLUTE",
                "TROMBONE",
                "HORN",
                "ORGAN",
                "ACOUSTIC",
                "TRUMPET",
                "CLARINET",
                "OBOE",
                "BASS CLARINET",
                "EUPHONIUM",
                "BASSOON",
                "ELECTRIC",
                "CELLO",
                "CONTRABASS",
                "ALTO SAX",
                "TENOR SAX",
                "SOUSAPHONE",
                "BASS DRUM",
            ]
        );
        assert_eq!(available().len(), PORTED.len());
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
        // every roster fighter, ported or not: a fight file may name it (what the app offers is available())
        for f in roster() {
            assert_eq!(name_status(&f.name), NameStatus::Known, "{}", f.name);
        }
        for r in RETIRED {
            assert_eq!(name_status(r), NameStatus::Retired);
        }
        // ALTO SAX, TENOR SAX and BARI SAX are different fighters
        assert_ne!(name_status("ALTO SAX"), NameStatus::Retired);
        assert_ne!(name_status("BARI SAX"), NameStatus::Retired);
    }

    #[test]
    fn only_pianos_music_can_feed_its_fight() {
        // PIANO's roll is its music (once its folder says so); no other fighter's music feeds its fight, and no
        // name outside the roster
        for f in roster() {
            if f.name != "PIANO" {
                assert!(!music_feeds_fight(&f.name), "{}", f.name);
            }
        }
        for name in RETIRED.iter().chain(&["KAZOO", "", "piano"]) {
            assert!(!music_feeds_fight(name), "{name:?}");
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
