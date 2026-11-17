//! The content registry (Task 7): the roster and which fighters the app offers.
//!
//! The roster is `data/roster.json` (exported by `tools/studio_export/roster.py`), compiled into the app and read
//! once. It holds the kit's 20 fighters and the ten older fighters of milestone 4; the app offers only those in
//! [`PORTED`] ([`available`]), so an older fighter is in the roster (its port can load its entry) long before it is
//! offered. The retired fighters (`RETIRED`) are never in it, never offered and refused in a fight file.

use std::sync::OnceLock;

use bf_core::Engine;
use bf_core::fightfile::NameStatus;
use bf_core::roster::{FighterCfg, parse_roster};

pub use bf_core::fightfile::RETIRED;

/// The exported roster.
const ROSTER_JSON: &str = include_str!("../data/roster.json");

/// The fighters made from scratch in the Studio (no Python kit reference), appended to the exported roster.
const ROSTER_NEW_JSON: &str = include_str!("../data/roster_new.json");

/// The fighters whose weapon and abilities the engine has ported, in no particular order (grows as fighters are
/// ported).
pub const PORTED: [&str; 40] = [
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
    "BARI SAX",
    "TURNTABLES",
    "SYNTH",
    "BAGPIPES",
    "VIOLA",
    "UKULELE",
    "VIOLIN",
    "HARP",
    "ACCORDION",
    "DRUMS",
    "CORNET",
    "CONTRABASS CLARINET",
    "MARIMBA",
    "PICCOLO",
    "SOPRANO SAX",
    "MELLOPHONE",
    "TIMPANI",
    "CONTRABASSOON",
    "FLUGELHORN",
    "SNARE DRUM",
];

/// Every roster entry (the kit's 20 and the older ten, offered or not), in `fighters.py`'s order, every number as
/// exported. What the app offers is [`available`].
///
/// # Panics
/// When the compiled-in roster cannot be read (a broken export; the tests read it on every build).
pub fn roster() -> &'static [FighterCfg] {
    static ROSTER: OnceLock<Vec<FighterCfg>> = OnceLock::new();
    ROSTER.get_or_init(|| {
        let mut all = parse_roster(ROSTER_JSON).unwrap_or_else(|e| panic!("the compiled-in {e}"));
        all.extend(parse_roster(ROSTER_NEW_JSON).unwrap_or_else(|e| panic!("the compiled-in new {e}")));
        all
    })
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
/// roster fighter (ported or not: [`available`] is what the app offers) or an Engine 2 fighter
/// ([`crate::fighters2::NAMES`], never offered), `Unknown` for anything else.
pub fn name_status(name: &str) -> NameStatus {
    if RETIRED.contains(&name) {
        NameStatus::Retired
    } else if roster().iter().any(|f| f.name == name) || crate::fighters2::NAMES.contains(&name) {
        NameStatus::Known
    } else {
        NameStatus::Unknown
    }
}

/// The engines a fighter can fight on, in order (Engine 2 design section 6). Every offered fighter has the current
/// engine; an unknown or retired name has none.
///
/// A fighter session that moves a fighter to Engine 2 adds [`Engine::Two`] here for it and never takes
/// [`Engine::One`] away: both versions stay until every fighter has moved (Aedan, 2026-11-17).
pub fn engines(name: &str) -> &'static [Engine] {
    const CURRENT: &[Engine] = &[Engine::One];
    if PORTED.contains(&name) && !RETIRED.contains(&name) {
        CURRENT
    } else {
        &[]
    }
}

/// The engines two fighters can fight each other on: the ones both have, in order (no mixed-engine matchups).
pub fn pair_engines(a: &str, b: &str) -> Vec<Engine> {
    let theirs = engines(b);
    engines(a).iter().copied().filter(|e| theirs.contains(e)).collect()
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
    fn every_offered_fighter_has_the_current_engine_only() {
        for n in available() {
            assert_eq!(engines(n), [Engine::One], "{n}");
        }
    }

    #[test]
    fn unknown_and_retired_names_have_no_engine() {
        for n in ["GUITAR", "SAX", "NOPE"] {
            assert!(engines(n).is_empty(), "{n}");
        }
    }

    #[test]
    fn pairs_share_engines() {
        assert_eq!(pair_engines("FLUTE", "TROMBONE"), vec![Engine::One]);
        assert!(pair_engines("FLUTE", "NOPE").is_empty());
    }

    #[test]
    fn roster_has_the_kit_20_the_older_10_the_studio_ones_and_no_retired() {
        let r = roster();
        assert_eq!(r.len(), 30 + crate::fighters::NEW.len());
        // fighters.py's order, which starts with the first fighters the kit had, then the fighters made from scratch
        let names: Vec<&str> = r.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(&names[30..], crate::fighters::NEW);
        let names = &names[..30];
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
        // every kit fighter, TURNTABLES, BARI SAX, SYNTH, BAGPIPES, VIOLA, UKULELE, VIOLIN, HARP,
        // ACCORDION and DRUMS, in fighters.py's order whatever the order of `PORTED`; no fighter twice
        assert_eq!(
            available(),
            vec![
                "PIANO",
                "DRUMS",
                "VIOLIN",
                "BAGPIPES",
                "ACCORDION",
                "TUBA",
                "FLUTE",
                "TROMBONE",
                "HORN",
                "ORGAN",
                "HARP",
                "UKULELE",
                "ACOUSTIC",
                "TRUMPET",
                "TURNTABLES",
                "SYNTH",
                "CLARINET",
                "OBOE",
                "BASS CLARINET",
                "EUPHONIUM",
                "BASSOON",
                "BARI SAX",
                "VIOLA",
                "ELECTRIC",
                "CELLO",
                "CONTRABASS",
                "ALTO SAX",
                "TENOR SAX",
                "SOUSAPHONE",
                "BASS DRUM",
                "CORNET",
                "CONTRABASS CLARINET",
                "MARIMBA",
                "PICCOLO",
                "SOPRANO SAX",
                "MELLOPHONE",
                "TIMPANI",
                "CONTRABASSOON",
                "FLUGELHORN",
                "SNARE DRUM",
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
