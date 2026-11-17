//! Engine 2's fighters, as `bf2-sim` data (Engine 2 design section 7). Each fighter lives in its own folder and
//! builds its [`FighterDef`] in Rust. None is offered yet: the registry's [`crate::engines`] and
//! [`crate::available`] know nothing of these names until a fighter session moves a fighter here.

use std::sync::{Arc, OnceLock};

use bf2_sim::FighterDef;

mod test_dummy;

/// Every Engine 2 fighter, in order. "TEST DUMMY 2" is the same dummy under a second name in a warm colour: a fight
/// file never names one fighter twice, so a dummy mirror match is TEST DUMMY against TEST DUMMY 2.
pub const NAMES: [&str; 2] = ["TEST DUMMY", "TEST DUMMY 2"];

/// The Engine 2 look of the fighter called `name`: a new one each call (a look holds its renderer's meshes).
pub fn look(name: &str) -> Option<Box<dyn bf2_render::Look2>> {
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Some(Box::new(test_dummy::look::DummyLook::new(def(name)?))),
        _ => None,
    }
}

/// The effects of the look of the fighter called `name` ([`bf2_fx::frame::LookFx`]).
pub fn look_fx(name: &str) -> Option<Box<dyn bf2_fx::frame::LookFx>> {
    match name {
        "TEST DUMMY" | "TEST DUMMY 2" => Some(Box::new(test_dummy::look::DummyLook::new(def(name)?))),
        _ => None,
    }
}

/// The Engine 2 definition of the fighter called `name`, built once and shared.
pub fn def(name: &str) -> Option<Arc<FighterDef>> {
    static DUMMY: OnceLock<Arc<FighterDef>> = OnceLock::new();
    static DUMMY_2: OnceLock<Arc<FighterDef>> = OnceLock::new();
    match name {
        "TEST DUMMY" => Some(DUMMY.get_or_init(|| Arc::new(test_dummy::def())).clone()),
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
        for name in NAMES {
            assert!(crate::engines(name).is_empty());
            assert!(!crate::available().contains(&name));
            assert!(!crate::PORTED.contains(&name));
        }
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
}
