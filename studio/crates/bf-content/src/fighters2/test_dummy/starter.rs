//! The fight a new Engine 2 match-up with the TEST DUMMY begins from (Engine 2 E6, Task 3).

use std::sync::OnceLock;

use bf_core::Engine;
use bf_core::fightfile::FightFile;

use crate::fighters2::DUMMIES;

/// The dummy's own fight file: the big top on its first chord loop, tuned for the dummy (damage scale 2).
const DUMMY_FILE: &str = include_str!("../../../../../fights/engine2/test_dummy.toml");

fn dummy_file() -> &'static FightFile {
    static FILE: OnceLock<FightFile> = OnceLock::new();
    FILE.get_or_init(|| {
        FightFile::parse(DUMMY_FILE).unwrap_or_else(|e| panic!("studio/fights/engine2/test_dummy.toml: {e}"))
    })
}

/// `left` vs `right` (two different names, at least one of them the dummy's) on Engine 2, before what every
/// starter sets (captions, version, no seed): the other fighter's home config with its theme when it has one, else
/// the dummy's own file (the big top on its first chord loop). The dummy plays the melody; the other fighter keeps
/// its home part, or takes the bass (or, without a bass part, a line) when its home part is the melody. The
/// handicap is even, the damage scale the dummy's (2), and there are no pair rules.
pub fn starter(left: &str, right: &str) -> FightFile {
    let other = [left, right].into_iter().find(|n| !DUMMIES.contains(n));
    let home = other.and_then(crate::starter::home_of);
    let base = home.unwrap_or_else(dummy_file);
    let theme = base.music.theme.clone();
    let part = |name: &str| -> String {
        if DUMMIES.contains(&name) && Some(name) != [left, right].into_iter().find(|n| DUMMIES.contains(n)) {
            // the second dummy of a mirror: the first one has the melody
            return "line".to_string();
        }
        if DUMMIES.contains(&name) {
            return "melody".to_string();
        }
        let own = home.and_then(|h| h.music.roles.get(name).cloned());
        match own.as_deref() {
            Some("melody") | None => if crate::card(name).is_some_and(|c| c.can_play_bass) {
                "bass"
            } else {
                "line"
            }
            .to_string(),
            Some(role) => role.to_string(),
        }
    };
    let mut ff = crate::starter::arrange(base.clone(), left, right, &theme, part(left), part(right));
    ff.engine = Engine::Two;
    ff.dmg_scale = dummy_file().dmg_scale;
    ff.pair_rules.clear();
    ff
}
