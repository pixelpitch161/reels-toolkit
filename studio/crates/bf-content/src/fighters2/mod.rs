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
}
