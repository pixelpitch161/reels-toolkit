//! The hook's own cases, read from whole fights of BASSOON against FLUTE.

use crate::fight::{Setup, run};
use crate::testing;

/// BOCAL HOOK in whole fights: it opens with the enemy within `ready` beyond contact, hooks `open16` 16ths later, a
/// landed hook is a `hit` of 4.0 and sets up the hopper's follow-up hop (kind 1) on the next
/// 16th, and `HOOK` holds the crook from the opening to 0.3 s after the hook.
#[test]
fn the_hook_waits_opens_hooks_and_sets_up_a_hop() {
    let roster = testing::roster();
    let mut rules = testing::episode_rules();
    // no handicap: the hopper levels up early and hooks often
    rules.dmg_side = None;
    let (s16, r) = (rules.beat / 4.0, rules.r);
    let (mut opened, mut hooked, mut followed) = (0, 0, 0);
    for seed in 1..=40 {
        let setup = Setup {
            fighters: [testing::fighter(&roster, "BASSOON"), testing::fighter(&roster, "FLUTE")],
            rules: rules.clone(),
            seed,
        };
        let rec = run(&setup, true).expect("the engine runs BASSOON");
        let frames = rec.frames();
        let frame = |t: f64| (((t - rules.t_fight) * f64::from(rules.fps)).round() as usize).min(frames - 1);
        let p = testing::f64s(&rec, "P");
        let hook = testing::f64s(&rec, "HOOK");
        assert_eq!(hook.len(), frames * 6);
        // FLUTE never has a crook out
        assert!(hook.chunks(6).all(|row| row[3..].iter().all(|v| v.is_nan())));
        let opens: Vec<_> = rec.events.iter().filter(|e| e.kind == "hook_open").collect();
        let hooks: Vec<_> = rec.events.iter().filter(|e| e.kind == "hook").collect();
        opened += opens.len();
        assert!(hooks.len() == opens.len() || hooks.len() + 1 == opens.len());
        for (o, h) in opens.iter().zip(&hooks) {
            // the frame at the opening holds the positions it opened on (to within a frame's travel)
            let i = frame(o.t);
            let d = ((p[i * 4 + 2] - p[i * 4]).powi(2) + (p[i * 4 + 3] - p[i * 4 + 1]).powi(2)).sqrt();
            assert!(
                d - 2.0 * r <= 1.0 + 0.3,
                "seed {seed}: opened at {} s with the enemy {d} away",
                o.t
            );
            assert!((o.f(3) - (o.t + 2.0 * s16)).abs() < 1e-4);
            assert!((h.t - o.f(3)).abs() < rules.dt + 2e-5);
            assert_eq!(h.f(5), 1.1);
            let landed = rec
                .events
                .iter()
                .find(|e| e.kind == "hit" && e.t == h.t && e.s(7) == "hook");
            if let Some(hit) = landed {
                assert_eq!(h.i(3), 1);
                assert!((hit.f(4) - 4.0).abs() < 1e-12);
                // the follow-up hop comes on the next 16th the hopper can hop (not while it tumbles; MOUNTAIN KING
                // cancels it)
                let soon = |e: &&bf_core::event::Event| e.kind == "hop" && e.t >= h.t && e.t - h.t < 4.0 * s16;
                if rec.events.iter().find(soon).is_some_and(|e| e.i(8) == 1) {
                    followed += 1;
                }
                hooked += 1;
            }
            // the record: swinging out at the frame after it opens
            let at = (frame(o.t) + 1).min(frames - 1) * 6;
            assert_eq!((hook[at], hook[at + 2]), (1.0, o.f(3)), "seed {seed}");
        }
    }
    assert!(
        opened >= 3 && hooked >= 2 && followed >= 1,
        "{opened} hooks, {hooked} landed, {followed} followed up"
    );
}
