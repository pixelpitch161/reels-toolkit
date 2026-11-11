//! The fight engine: rules, balls, weapon kinds, abilities and the roster numbers.
//!
//! A rule-for-rule port of `projects/ballfight/fight.py`: an honest, deterministic 2D fight between two balls in a
//! square ring, 120 steps a second. [`run`] plays one fight and returns its recording.
//!
//! - [`rules`]: the rules of a fight (`class Rules`).
//! - [`geom`]: plane geometry with numpy's order of operations (the helpers above `class Ball`).
//! - [`ball`]: one fighter in the ring (`class Ball`).
//! - [`fight`]: the fight's core (`class Fight`: the launch, events, damage, knockback, motion, levels).
//! - [`schedule`]: the order of a step (`Fight.run`), the one place that calls the weapon kinds and abilities.
//! - [`kinds`]: the weapon kinds, one module each (`striker`: the FLUTE's baskets and cobras, and the swat rule;
//!   `blade`: the TROMBONE's thrust lance; and a module for each other weapon kind of the kit, a stub until its
//!   fighter's port, already called wherever `fight.py` calls it).
//! - [`abilities`]: the abilities, one module per kind (`haste`, `dance`, `wave`, `shockwave` with its slow, `slam`,
//!   `shield`, and the kit's other kinds, stubs until their ports), with `trigger` and `effects_step`.
//! - `record`: what is written down at every frame (`Fight.snap`).
//! - [`parity`]: how long a fight of this engine agrees with the Python recording of the same fight, and what ends
//!   the agreement (the parity suites' measure).
//! - [`roster`]: the fighters' numbers, exported from `fighters.py` (the types live in `bf_core::roster`).

pub mod abilities;
pub mod ball;
pub mod fight;
pub mod geom;
pub mod kinds;
pub mod parity;
mod record;
pub mod roster;
pub mod rules;
pub mod schedule;

pub use fight::{Fight, Setup, lead_changes, run, setup_from, stunned};
#[cfg(any(test, feature = "testing"))]
pub use fight::{run_traced, run_with_unported};
pub use rules::{Rules, SimError};

/// What the unit tests share: the roster, the Python reference recordings and ready-made setups.
#[cfg(test)]
pub(crate) mod testing {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use bf_core::fightfile::FightFile;
    use bf_core::recording::{Array, Recording};

    use crate::fight::{Fight, Setup};
    use crate::roster::{AbilityCfg, AbilityKind, AbilitySettings, FighterCfg, WeaponCfg, parse_roster};
    use crate::rules::Rules;

    /// A fight of a setup, through the test entry `Fight::with_unported`: the bare fighters below have no weapon
    /// and abilities of a kind not ported, which `Fight::new` refuses. The setup's rules and every ported kind are
    /// still checked.
    pub(crate) fn fight<'a>(setup: &Setup<'a>, record: bool) -> Fight<'a> {
        Fight::with_unported(setup, record).unwrap_or_else(|e| panic!("the engine cannot run this setup: {e}"))
    }

    /// Every committed Python recording of FLUTE against TROMBONE, relative to the reference fights folder: the five
    /// reference fights, the fight with no K.O. and the scenarios.
    pub(crate) const ALL_RECORDINGS: [&str; 21] = [
        "ep392.bfrec",
        "seed1.bfrec",
        "seed2.bfrec",
        "seed3.bfrec",
        "seed500.bfrec",
        "no_ko.bfrec",
        "scenarios/early_ko_s1.bfrec",
        "scenarios/early_ko_s2.bfrec",
        "scenarios/early_ko_s3.bfrec",
        "scenarios/early_levels_s1.bfrec",
        "scenarios/early_levels_s2.bfrec",
        "scenarios/early_levels_s3.bfrec",
        "scenarios/early_low_s1.bfrec",
        "scenarios/early_low_s2.bfrec",
        "scenarios/early_low_s3.bfrec",
        "scenarios/plain_s7.bfrec",
        "scenarios/plain_s8.bfrec",
        "scenarios/plain_s9.bfrec",
        "scenarios/swapped_s1.bfrec",
        "scenarios/swapped_s2.bfrec",
        "scenarios/swapped_s3.bfrec",
    ];

    fn reference(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference/fights/flute_vs_trombone")
            .join(rel)
    }

    /// The exported roster.
    pub(crate) fn roster() -> Vec<FighterCfg> {
        parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
    }

    /// One fighter of a roster.
    pub(crate) fn fighter<'a>(roster: &'a [FighterCfg], name: &str) -> &'a FighterCfg {
        roster
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the roster"))
    }

    /// A Python reference recording.
    pub(crate) fn python_recording(rel: &str) -> Recording {
        Recording::read(&reference(rel)).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The kit's stories of the reference recordings (`stories.json`).
    pub(crate) fn stories() -> serde_json::Value {
        let path = reference("stories.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// The example fight file (FLUTE against TROMBONE, the episode the reference recordings come from).
    pub(crate) fn example_fight_file() -> FightFile {
        FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the example parses")
    }

    /// A float array of a recording, flat in C order.
    pub(crate) fn f64s(rec: &Recording, key: &str) -> Vec<f64> {
        match rec.arrays.get(key) {
            Some(Array::F64 { data, .. }) => data.clone(),
            other => panic!("{key} is not a float array: {other:?}"),
        }
    }

    /// The rules the example episode's Python fights ran under (those of `ep392.bfrec`).
    pub(crate) fn episode_rules() -> Rules {
        let py = python_recording("ep392.bfrec");
        let mut rules = Rules::new(1.0, 0.0);
        rules.apply(&py.rules).expect("the episode's rules apply");
        rules
    }

    /// FLUTE (left) against TROMBONE under the episode's rules.
    pub(crate) fn episode_setup(roster: &[FighterCfg], seed: u64) -> Setup<'_> {
        Setup {
            fighters: [fighter(roster, "FLUTE"), fighter(roster, "TROMBONE")],
            rules: episode_rules(),
            seed,
        }
    }

    /// A fighter with no weapon and no abilities the engine knows: only the core acts on it, whatever kinds are
    /// ported later. Its abilities are the bags' of the reference fights (`dict(name='NOTHING', kind='none',
    /// every_bars=4)`), a kind not ported: should one come up (on a beat below `LOW_HP`, or after a level), it is
    /// announced by an `ability` event and does nothing.
    fn bare(name: &str) -> FighterCfg {
        let nothing = || AbilityCfg {
            name: "NOTHING".into(),
            kind: AbilityKind::Unsupported("none".into()),
            params: BTreeMap::from([("every_bars".to_string(), 4.0)]),
            slow: None,
            detune: None,
            settings: AbilitySettings::Plain,
        };
        FighterCfg {
            name: name.into(),
            color: [255, 255, 255],
            dark: [0, 0, 0],
            weapon: WeaponCfg::Unsupported("none".into()),
            unlock1: nothing(),
            unlock2: nothing(),
            low: nothing(),
            register: (60, 72),
            idiom: String::new(),
            pp: String::new(),
            raw: serde_json::Value::Null,
        }
    }

    /// FLUTE's entry as `KAZOO BALL`, with a weapon of a made-up kind (`kazoo`): a fighter the engine refuses whatever
    /// the kit's kinds ported so far.
    pub(crate) fn kazoo(roster: &[FighterCfg]) -> FighterCfg {
        FighterCfg {
            name: "KAZOO BALL".into(),
            weapon: WeaponCfg::Unsupported("kazoo".into()),
            ..fighter(roster, "FLUTE").clone()
        }
    }

    /// Two bare fighters, `LEFT` and `RIGHT`.
    pub(crate) fn bare_pair() -> [FighterCfg; 2] {
        [bare("LEFT"), bare("RIGHT")]
    }

    /// The bare pair at the episode's tempo and start time, with the engine's default rules (no handicap).
    pub(crate) fn bare_setup(pair: &[FighterCfg; 2], seed: u64) -> Setup<'_> {
        let ep = episode_rules();
        Setup {
            fighters: [&pair[0], &pair[1]],
            rules: Rules::new(ep.bpm, ep.t_fight),
            seed,
        }
    }
}
