//! Engine 2's fighters, as `bf2-sim` data (Engine 2 design section 7). Each fighter lives in its own folder and
//! builds its [`FighterDef`] in Rust. A real fighter is offered on Engine 2 once a fighter session adds its name to
//! [`MOVED`]; the TEST DUMMY ([`DUMMIES`]) is offered only with the developer switch on ([`crate::offered`]).

use std::sync::{Arc, OnceLock};

use bf_core::fightfile::FightFile;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_picture::sprite::Mesh;
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;
use bf2_sim::FighterDef;

use crate::FighterCard;
use crate::personality::Personality;

mod test_dummy;
// SPIKE: the trumpet's candidate standard attacks (Aedan's round-1 picks), for in-engine example renders.
mod trumpet;

/// Every Engine 2 fighter, in order: the moved fighters ([`MOVED`]) and the dummy ([`DUMMIES`]). "TEST DUMMY 2" is
/// the same dummy under a second name in a warm colour: a fight file never names one fighter twice, so a dummy
/// mirror match is TEST DUMMY against TEST DUMMY 2.
pub const NAMES: [&str; 3] = ["TEST DUMMY", "TEST DUMMY 2", "TRUMPET"];

/// The real fighters with an Engine 2 version (each also keeps its current-engine version). A fighter session adds
/// its fighter's name here, and its name to [`NAMES`].
pub const MOVED: [&str; 0] = [];

/// The developer fighter's two names (TEST DUMMY and its mirror twin), offered only with the developer switch on.
pub const DUMMIES: [&str; 2] = ["TEST DUMMY", "TEST DUMMY 2"];

#[cfg(test)]
thread_local! {
    static TEST_MOVED: std::cell::Cell<Option<&'static [&'static str]>> = const { std::cell::Cell::new(None) };
}

/// The moved fighters: [`MOVED`] (a test may stand in a list of its own with [`with_moved`]).
pub(crate) fn moved() -> &'static [&'static str] {
    #[cfg(test)]
    if let Some(names) = TEST_MOVED.with(std::cell::Cell::get) {
        return names;
    }
    &MOVED
}

/// Runs `f` with `names` as the moved fighters on this thread (tests only).
#[cfg(test)]
pub(crate) fn with_moved<R>(names: &'static [&'static str], f: impl FnOnce() -> R) -> R {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_MOVED.with(|c| c.set(None));
        }
    }
    TEST_MOVED.with(|c| c.set(Some(names)));
    let _reset = Reset;
    f()
}

/// The VS card panel of an Engine 2 fighter: a moved fighter's current-engine card (until its folder writes a
/// `card.rs` of its own), the dummy's own card, `None` for any other name.
pub fn card(name: &str) -> Option<&'static FighterCard> {
    if DUMMIES.contains(&name) {
        Some(&test_dummy::card::CARD)
    } else if moved().contains(&name) {
        crate::card(name)
    } else {
        None
    }
}

/// The roster names of the three moves an Engine 2 card's ability lines describe, in the card's order, for a
/// fighter whose card is its own (the dummy); `None` for a moved fighter (its roster abilities name them) or any
/// other name.
pub fn card_moves(name: &str) -> Option<[&'static str; 3]> {
    DUMMIES.contains(&name).then_some(test_dummy::card::MOVES)
}

/// The reach an Engine 2 card shows for a fighter whose card is its own (the dummy), in words; `None` otherwise.
pub fn card_range(name: &str) -> Option<&'static str> {
    DUMMIES.contains(&name).then_some(test_dummy::card::RANGE)
}

/// The fight a new Engine 2 match-up with the dummy on either side begins from (see [`crate::starter_on`]).
/// A move's roster name in sentence case, as a card writes it: "GRAB SLAM" is "Grab slam".
#[cfg(test)]
pub(crate) fn sentence(name: &str) -> String {
    let lower = name.to_lowercase();
    let mut c = lower.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

pub(crate) fn dummy_starter(left: &str, right: &str) -> bf_core::fightfile::FightFile {
    test_dummy::starter::starter(left, right)
}

/// The Engine 2 look of the fighter called `name`: a new one each call (a look holds its renderer's meshes).
pub fn look(name: &str) -> Option<Box<dyn bf2_render::Look2>> {
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Some(Box::new(test_dummy::look::DummyLook::new(def(name)?))),
        "TRUMPET" => Some(Box::new(trumpet::look::TrumpetLook::new(def(name)?))),
        _ => None,
    }
}

/// The effects of the look of the fighter called `name` ([`bf2_fx::frame::LookFx`]).
pub fn look_fx(name: &str) -> Option<Box<dyn bf2_fx::frame::LookFx>> {
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Some(Box::new(test_dummy::look::DummyLook::new(def(name)?))),
        "TRUMPET" => Some(Box::new(trumpet::look::TrumpetLook::new(def(name)?))),
        _ => None,
    }
}

/// The Engine 2 definition of the fighter called `name`, built once and shared.
pub fn def(name: &str) -> Option<Arc<FighterDef>> {
    static DUMMY: OnceLock<Arc<FighterDef>> = OnceLock::new();
    static DUMMY_2: OnceLock<Arc<FighterDef>> = OnceLock::new();
    static TRUMPET: OnceLock<Arc<FighterDef>> = OnceLock::new();
    match name {
        "TRUMPET" => Some(TRUMPET.get_or_init(|| Arc::new(trumpet::def())).clone()),
        "TEST DUMMY" => Some(
            DUMMY
                .get_or_init(|| {
                    let mut d = test_dummy::def();
                    // SPIKE (trumpet demo clips): BF_DUMMY_PASSIVE=1 makes the dummy a punching bag that throws
                    // nothing and keeps its arm tucked away, so the attacker's moves read on their own.
                    if std::env::var("BF_DUMMY_PASSIVE").is_ok_and(|v| v == "1") {
                        d.brain.choices.clear();
                        d.brain.low = None;
                        d.brain.speed = 0.2;
                        for b in &mut d.rig.bones {
                            b.length = 0.01;
                        }
                    }
                    Arc::new(d)
                })
                .clone(),
        ),
        "TEST DUMMY 2" => Some(
            DUMMY_2
                .get_or_init(|| {
                    Arc::new(FighterDef {
                        name: "TEST DUMMY 2".into(),
                        colour: [230, 150, 90],
                        ..test_dummy::def()
                    })
                })
                .clone(),
        ),
        _ => None,
    }
}

/// The dummy's roster entries (`test_dummy/roster.json`, both names), read once. They are never appended to
/// [`crate::roster`]: the dummy stays hidden.
fn entries() -> &'static [FighterCfg] {
    static ENTRIES: OnceLock<Vec<FighterCfg>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        parse_roster(include_str!("test_dummy/roster.json")).unwrap_or_else(|e| panic!("the dummy's compiled-in {e}"))
    })
}

/// The roster entry of the Engine 2 fighter `name`, for its place in the score: TEST DUMMY plays MARIMBA's voice
/// in MARIMBA's register, in its own colour, and its abilities name its own moves (unlock1 and unlock2 its two
/// `Special` moves, low its `Low` move). `None` for any other name.
pub fn cfg(name: &str) -> Option<&'static FighterCfg> {
    if name == "TRUMPET" {
        return crate::roster().iter().find(|f| f.name == name);
    }
    entries().iter().find(|f| f.name == name)
}

/// The instrument voice of the Engine 2 fighter `name`: the dummy's is MARIMBA's.
pub fn voice(name: &str) -> Option<Arc<dyn Voice>> {
    cfg(name).and_then(|c| crate::fighters::voice(&c.pp))
}

/// The part in the score of the Engine 2 fighter `name` on side `k` of the fight `ff`.
pub fn sound(name: &str, k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    let _ = ff;
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Some(Box::new(test_dummy::sound::DummySound::new(k))),
        _ => None,
    }
}

/// The musical personality of the Engine 2 fighter `name` (the composer writes it a line): the dummy borrows
/// MARIMBA's.
pub fn personality(name: &str) -> Option<&'static Personality> {
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Some(&crate::fighters::marimba::personality::PERSONALITY),
        "TRUMPET" => Some(&crate::fighters::trumpet::personality::PERSONALITY),
        _ => None,
    }
}

/// The VS card's close-up model of the Engine 2 fighter `name`: the dummy's is a small metronome
/// (`test_dummy/metronome.json`).
pub fn card_mesh(name: &str) -> Option<Mesh> {
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Mesh::from_json(include_str!("test_dummy/metronome.json")).ok(),
        "TRUMPET" => crate::fighters::trumpet::MESH.and_then(|t| Mesh::from_json(t).ok()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_checks() {
        let d = def("TEST DUMMY").expect("the dummy is defined");
        assert_eq!(d.check(), Ok(()));
        assert_eq!(d.name, "TEST DUMMY");
        assert!(NAMES.iter().all(|n| def(n).is_some()));
        assert!(def("FLUTE").is_none());
    }

    #[test]
    fn dummy_is_not_offered() {
        for name in DUMMIES {
            assert!(crate::engines(name).is_empty());
            assert!(!crate::available().contains(&name));
            assert!(!crate::PORTED.contains(&name));
        }
    }

    #[test]
    fn dummy_card_names_its_moves() {
        let c = card("TEST DUMMY").expect("the dummy has a card");
        assert_eq!(c.weapon, "Fist on an arm");
        assert!(std::ptr::eq(c, card("TEST DUMMY 2").unwrap()));
        let d = def("TEST DUMMY").unwrap();
        let chosen: Vec<&str> = d.brain.choices.iter().map(|ch| d.moves[ch.mv].name).collect();
        assert_eq!(chosen.len(), 5);
        for name in &chosen {
            assert!(
                c.text.contains(&crate::fighters2::sentence(name)),
                "{name} in {:?}",
                c.text
            );
        }
        for m in card_moves("TEST DUMMY").unwrap() {
            assert!(d.moves.iter().any(|mv| mv.name == m), "{m}");
        }
        let near = d
            .brain
            .choices
            .iter()
            .map(|ch| ch.range.0)
            .fold(f64::INFINITY, f64::min);
        let far = d.brain.choices.iter().map(|ch| ch.range.1).fold(0.0, f64::max);
        assert_eq!(card_range("TEST DUMMY"), Some(format!("{near} to {far} m").as_str()));
        assert!(card("FLUTE").is_none() && card_moves("FLUTE").is_none());
        with_moved(&["FLUTE"], || {
            assert!(std::ptr::eq(card("FLUTE").unwrap(), crate::card("FLUTE").unwrap()))
        });
    }

    #[test]
    fn dummy_names_are_known_and_its_fight_file_validates() {
        use bf_core::fightfile::NameStatus;
        for name in NAMES {
            assert_eq!(crate::name_status(name), NameStatus::Known);
            assert!(look(name).is_some());
        }
        let ff = bf_core::fightfile::FightFile::parse(include_str!("../../../../fights/engine2/test_dummy.toml"))
            .expect("the dummy's fight file parses");
        assert_eq!(ff.engine, bf_core::Engine::Two);
        assert_eq!(crate::validate(&ff), Ok(()));
        assert!(look("FLUTE").is_none());
    }

    #[test]
    fn dummy_has_a_score_identity() {
        let ff = bf_core::fightfile::FightFile::parse(include_str!("../../../../fights/engine2/test_dummy.toml"))
            .expect("the dummy's fight file parses");
        let marimba = crate::roster().iter().find(|f| f.name == "MARIMBA").unwrap();
        for (k, name) in DUMMIES.into_iter().enumerate() {
            let c = cfg(name).expect("a roster entry");
            assert_eq!(c.name, name);
            assert_eq!((c.pp.as_str(), c.register), (marimba.pp.as_str(), marimba.register));
            let d = def(name).unwrap();
            assert_eq!(c.color, d.colour);
            let special: Vec<&str> = d
                .moves
                .iter()
                .filter(|m| m.has(bf2_sim::Tag::Special))
                .map(|m| m.name)
                .collect();
            assert_eq!([c.unlock1.name.as_str(), c.unlock2.name.as_str()], special[..]);
            let low = d.moves.iter().find(|m| m.has(bf2_sim::Tag::Low)).unwrap();
            assert_eq!(c.low.name, low.name);
            let v = voice(name).expect("a voice");
            assert_eq!(v.name(), c.pp);
            let part = sound(name, k, &ff).expect("a part");
            assert_eq!(part.k(), k);
            assert_eq!(part.band(), crate::fighters::sound("MARIMBA", k, &ff).unwrap().band());
            assert!(std::ptr::eq(
                personality(name).unwrap(),
                crate::personality("MARIMBA").unwrap()
            ));
            assert!(std::ptr::eq(
                crate::personality(name).unwrap(),
                crate::personality("MARIMBA").unwrap()
            ));
            assert_eq!(card_mesh(name).expect("a card model").faces.len(), 3);
        }
        assert!(cfg("MARIMBA").is_none() && voice("MARIMBA").is_none() && personality("FLUTE").is_none());
        assert!(sound("FLUTE", 0, &ff).is_none() && card_mesh("FLUTE").is_none());
        // parsed once
        assert!(std::ptr::eq(cfg("TEST DUMMY").unwrap(), cfg("TEST DUMMY").unwrap()));
    }

    #[test]
    fn dummy_still_hidden() {
        for name in DUMMIES {
            assert!(crate::roster().iter().all(|f| f.name != name), "{name}");
            assert!(crate::fighters::entry(name).is_none(), "{name}");
        }
    }

    #[test]
    fn dummy_fight_file_validates() {
        let ff = bf_core::fightfile::FightFile::parse(include_str!("../../../../fights/engine2/test_dummy.toml"))
            .expect("the dummy's fight file parses");
        assert_eq!(ff.music.arrangement, crate::themes::COMPOSED);
        assert_eq!(ff.music.title_hits.get("TEST DUMMY"), Some(&vec![0, 4, 7]));
        assert_eq!(crate::soundtrack::validate(&ff), Ok(()));
        // the music is checked now: a theme that does not exist is refused
        let mut bad = ff.clone();
        bad.music.theme = "polka".into();
        assert_eq!(
            crate::validate(&bad).unwrap_err().to_string(),
            r#"music.theme: "polka" is not a theme"#
        );
        // and a current-engine fighter in an Engine 2 file still is, first
        let mut bad = ff;
        bad.fighters[1] = "FLUTE".into();
        assert!(
            crate::validate(&bad)
                .unwrap_err()
                .to_string()
                .contains("no Engine 2 version yet")
        );
    }

    #[test]
    fn dummy_look_records_all_have_exits() {
        use bf2_fx::FxTimeline;
        use bf2_fx::presets::{MIN_FALL, brightness};
        let defs = [def("TEST DUMMY").unwrap(), def("TEST DUMMY 2").unwrap()];
        let fx = [look_fx("TEST DUMMY").unwrap(), look_fx("TEST DUMMY 2").unwrap()];
        let mut kinds = std::collections::HashSet::new();
        for seed in 1..=20 {
            let mut rules = bf2_sim::Rules2::new(4.5);
            rules.dmg_scale = 2.0;
            let rec = bf2_sim::run(
                &bf2_sim::Setup2 {
                    fighters: defs.clone(),
                    rules,
                    seed,
                },
                true,
            )
            .unwrap();
            let to_t = |s: u32| rec.t_fight + f64::from(s) * rec.dt;
            let mut tl = FxTimeline::new();
            for (k, f) in fx.iter().enumerate() {
                f.fx(k, &rec, &to_t, &mut tl).unwrap();
            }
            assert_eq!(tl.check_all(), Ok(()));
            for r in &tl.records {
                kinds.insert(std::mem::discriminant(&r.kind));
                assert!(r.life > 0.0, "seed {seed}: {r:?}");
                assert!(
                    r.env.release >= MIN_FALL,
                    "seed {seed}: a fall of {}: {r:?}",
                    r.env.release
                );
                let peak = (1..200)
                    .map(|i| brightness(r, r.t0 + f64::from(r.life) * f64::from(i) / 200.0))
                    .fold(0.0f32, f32::max);
                let last = brightness(r, r.t0 + f64::from(r.life) - 1.0 / 240.0);
                assert!(last < 0.02 * peak, "seed {seed}: ends at {last} of {peak}: {r:?}");
            }
        }
        // Smears and lightning (trails), the dome, the beam and the decals all appear.
        assert!(kinds.len() >= 4, "{} kinds of record", kinds.len());
    }

    /// Aedan, E7: the dummy's attack effects toned down by a lot. Wired through its restraint, a real fight keeps
    /// under a third of the presets' sparks, no lens effects outside the K.O., and every record still fades out.
    #[test]
    fn dummy_effects_are_toned_down() {
        use bf2_fx::presets::{MIN_FALL, particle_count};
        use bf2_fx::screen::ScreenFx;
        use bf2_fx::{FxKind, FxTimeline};
        let defs = [def("TEST DUMMY").unwrap(), def("TEST DUMMY 2").unwrap()];
        let fx = [look_fx("TEST DUMMY").unwrap(), look_fx("TEST DUMMY 2").unwrap()];
        let mut rules = bf2_sim::Rules2::new(4.5);
        rules.dmg_scale = 2.0;
        let rec = bf2_sim::run(
            &bf2_sim::Setup2 {
                fighters: defs.clone(),
                rules,
                seed: 576,
            },
            true,
        )
        .unwrap();
        let to_t = |s: u32| rec.t_fight + f64::from(s) * rec.dt;
        let colours = [[0.6; 3], [1.0, 0.4, 0.1]];
        let (mut full, mut toned) = (FxTimeline::new(), FxTimeline::new());
        bf2_fx::wire::wire(&rec, [&defs[0], &defs[1]], colours, &to_t, &mut full).unwrap();
        let (a, b) = (|e: &bf2_sim::EvKind| fx[0].tone(e), |e: &bf2_sim::EvKind| fx[1].tone(e));
        bf2_fx::wire::wire_toned(&rec, [&defs[0], &defs[1]], colours, &to_t, [&a, &b], &mut toned).unwrap();
        let (pf, pt) = (particle_count(&full.records), particle_count(&toned.records));
        assert!(pt * 3 < pf, "{pf} -> {pt}");
        let t_ko = rec
            .events
            .iter()
            .find(|e| matches!(e.kind, bf2_sim::EvKind::Ko { .. }))
            .map(|e| to_t(e.step));
        for r in &toned.records {
            assert!(r.env.release >= MIN_FALL, "{r:?}");
            if let FxKind::Screen(s) = &r.kind {
                let lens = matches!(
                    s.effect,
                    ScreenFx::SpeedLines { .. } | ScreenFx::RadialBlur { .. } | ScreenFx::Chroma { .. }
                );
                let at_ko = t_ko.is_some_and(|t| (r.t0 - t).abs() < 0.05);
                assert!(!lens || at_ko, "a lens effect off the K.O.: {r:?}");
            }
        }
    }
}
