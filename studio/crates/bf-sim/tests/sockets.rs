//! Task 3, the engine's sockets: every place `fight.py` calls the code of a kit kind, or tests a kit kind's state,
//! has its call in the shared engine files already, in Python's order, and none of them changes a fight of FLUTE
//! against TROMBONE.
//!
//! With `Fight::trace_sockets` on (`bf_sim::run_traced`) every socket logs its name. `tests/sockets_order.txt` is the
//! order transcribed from `fight.py`, each socket with its lines and when it is reached; these tests run FLUTE
//! against TROMBONE (the example fight file, both corners, four seeds) and check the trace against it:
//!
//! - the setup and frame `f0`, then every step's unconditional sockets, and every frame's, exactly in the file's
//!   order (the beat grid's on the steps that reach a beat);
//! - the conditional ones (a hit, a knock, a clash, an ability, a thrust, a ring or slam that lands) in their
//!   blocks' order, and exactly as often as the recording's events say they happened;
//! - no stub of a kind not ported is ever reached;
//! - the fight with the trace on is the fight without it, bit for bit.

use std::collections::BTreeMap;

use bf_core::event::Value;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{run, run_traced, setup_from};

/// One line of `sockets_order.txt`.
#[derive(Clone, Debug)]
struct Socket {
    name: String,
    lines: String,
    when: String,
}

fn order() -> Vec<Socket> {
    let text = include_str!("sockets_order.txt");
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            assert_eq!(f.len(), 3, "sockets_order.txt: {l}");
            Socket {
                name: f[0].to_string(),
                lines: f[1].to_string(),
                when: f[2].to_string(),
            }
        })
        .collect()
}

/// The names of the file's sockets reached at `when`, in the file's order.
fn of(order: &[Socket], when: &[&str]) -> Vec<String> {
    order
        .iter()
        .filter(|s| when.contains(&s.when.as_str()))
        .map(|s| s.name.clone())
        .collect()
}

fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

fn fight_files() -> [FightFile; 2] {
    let ff = FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the example parses");
    let mut swapped = ff.clone();
    swapped.fighters.reverse();
    swapped.handicap.reverse();
    [ff, swapped]
}

/// Events of this kind (and, for `hit`, of what hit), counted.
fn count(rec: &Recording, kind: &str, what: Option<&[&str]>) -> usize {
    rec.events
        .iter()
        .filter(|e| e.kind == kind)
        .filter(|e| match what {
            None => true,
            Some(names) => matches!(e.fields.get(5), Some(Value::Str(s)) if names.contains(&s.as_str())),
        })
        .count()
}

/// How often each name occurs.
fn tally(trace: &[&str]) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for t in trace {
        *m.entry(t.to_string()).or_insert(0) += 1;
    }
    m
}

/// Every entry `first` is followed directly by `then` (in order), and `then` occurs nowhere else.
#[track_caller]
fn blocks(trace: &[&str], block: &[&str], what: &str) {
    for (i, t) in trace.iter().enumerate() {
        if *t == block[0] {
            assert_eq!(&trace[i..i + block.len()], block, "{what}: at trace entry {i}");
        }
    }
    let n = trace.iter().filter(|t| **t == block[0]).count();
    for name in &block[1..] {
        assert_eq!(trace.iter().filter(|t| *t == name).count(), n, "{what}: {name}");
    }
}

#[test]
fn the_order_file_names_every_socket_once_per_place() {
    let order = order();
    assert!(order.len() > 120, "{} sockets", order.len());
    for s in &order {
        assert!(
            [
                "setup", "frame", "step", "beat", "unlock", "ability", "copy", "hit", "knock", "clash", "lance",
                "ringhit", "slamhit"
            ]
            .contains(&s.when.as_str()),
            "{s:?}"
        );
        let ok = s
            .lines
            .split('-')
            .all(|n| n.parse::<u32>().is_ok_and(|n| (300..=6300).contains(&n)));
        assert!(ok, "{s:?}: a fight.py line or range");
    }
    // per-fighter places are listed for fighter 0, then fighter 1
    let names = |when: &str| of(&order, &[when]);
    assert_eq!(names("setup").len(), 19);
    assert_eq!(names("frame").len(), 21);
    assert_eq!(names("beat").iter().filter(|n| *n == "beat.stunned").count(), 2);
}

#[test]
fn flute_against_trombone_reaches_the_sockets_in_fight_py_order() {
    let order = order();
    let setup_frame: Vec<String> = [of(&order, &["setup"]), of(&order, &["frame"])].concat();
    let frame = of(&order, &["frame"]);
    let known: Vec<String> = order.iter().map(|s| s.name.clone()).collect();
    let conditional = [
        "unlock", "ability", "copy", "hit", "knock", "clash", "lance", "ringhit", "slamhit",
    ];
    let is_conditional = |name: &str| {
        order
            .iter()
            .find(|s| s.name == name)
            .is_some_and(|s| conditional.contains(&s.when.as_str()))
    };
    let step_with_beat = of(&order, &["step", "beat"]);
    let step_alone = of(&order, &["step"]);
    let roster = roster();
    let (mut fights, mut steps, mut beats) = (0, 0, 0);
    for ff in fight_files() {
        for seed in [1, 2, 3, 392] {
            let setup = setup_from(&ff, &roster, seed).expect("the example fight file sets up");
            let plain = run(&setup, true).expect("runs");
            let (rec, trace) = run_traced(&setup, true).expect("runs");
            let what = format!("{} against {}, seed {seed}", ff.fighters[0], ff.fighters[1]);
            // tracing is only looking
            assert_eq!(rec.hash(), plain.hash(), "{what}");
            assert_eq!(rec.events, plain.events, "{what}");
            // no stub of a kind not ported is reached, and every socket is one the file lists
            for t in &trace {
                assert!(!t.starts_with("stub:"), "{what}: {t} was reached");
                assert!(known.iter().any(|k| k == t), "{what}: {t} is not in sockets_order.txt");
            }
            // the setup, then frame f0
            let first = trace.iter().position(|t| *t == "clock").expect("a step");
            assert_eq!(trace[..first], setup_frame, "{what}: the setup and frame f0");
            // each step: its unconditional sockets in the file's order, then its frame (if it records one)
            let (mut at, mut frames_seen) = (first, 0);
            while at < trace.len() {
                let end = trace[at + 1..]
                    .iter()
                    .position(|t| *t == "clock")
                    .map_or(trace.len(), |i| at + 1 + i);
                let seg = &trace[at..end];
                let snap = seg.iter().position(|t| t.starts_with("snap.")).unwrap_or(seg.len());
                if snap < seg.len() {
                    assert_eq!(seg[snap..], frame, "{what}: a frame after step {}", steps + 1);
                    frames_seen += 1;
                }
                let got: Vec<&str> = seg[..snap].iter().copied().filter(|t| !is_conditional(t)).collect();
                let on_beat = got.contains(&"beat.stunned");
                let want = if on_beat { &step_with_beat } else { &step_alone };
                assert_eq!(got, *want, "{what}: step {} (beat: {on_beat})", steps + 1);
                steps += 1;
                beats += usize::from(on_beat);
                at = end;
            }
            assert_eq!(
                frames_seen + 1,
                rec.frames(),
                "{what}: a frame's sockets with every frame but f0"
            );

            // the conditional sockets: in their blocks, as often as the events say
            let n = tally(&trace);
            let get = |name: &str| n.get(name).copied().unwrap_or(0);
            // `damage` on every hit (no shield, shadow, smash or flight in this pair: every call lands), then
            // `knock`; a ring that passes its enemy, then its detune and stun; a slam that lands on the enemy
            blocks(&trace, &["damage.skips", "damage.guard", "damage.detuned"], &what);
            assert_eq!(
                get("damage.skips"),
                count(&rec, "hit", None),
                "{what}: damage once per hit"
            );
            let ring_hits = count(&rec, "hit", Some(&["wave", "shockwave"]));
            blocks(
                &trace,
                &["effects.wave.detune", "effects.wave.stun", "effects.wave.grow"],
                &what,
            );
            assert_eq!(get("effects.wave.detune"), ring_hits, "{what}: a ring's hit");
            let slam_hits = count(&rec, "hit", Some(&["slam"]));
            for (i, t) in trace.iter().enumerate() {
                if *t == "effects.slam.how" {
                    assert_eq!(
                        trace[i + 1],
                        "damage.skips",
                        "{what}: a slam names its hit, then damages"
                    );
                }
            }
            assert_eq!(get("effects.slam.how"), slam_hits, "{what}: a slam's hit");
            // a clash: no walker in this pair, so every overlap is announced
            blocks(&trace, &["move.walker", "move.wclash", "move.clash_hooks"], &what);
            assert_eq!(
                get("move.walker"),
                count(&rec, "clash", None),
                "{what}: one clash per overlap"
            );
            assert_eq!(get("move.walker_pinned"), 0, "{what}");
            // an ability: announced, made the fighter's last, dispatched; no copy in this pair
            blocks(
                &trace,
                &["trigger.ability", "trigger.last_ab", "trigger.dispatch"],
                &what,
            );
            assert_eq!(get("trigger.ability"), count(&rec, "ability", None), "{what}");
            assert_eq!(get("trigger.quiet"), 0, "{what}");
            // the lance, on each step of a thrust in progress, right after its weapon step's socket
            for (i, t) in trace.iter().enumerate() {
                if *t == "lance.swat_shells" {
                    assert_eq!(trace[i - 1], "weapon.blade", "{what}: at trace entry {i}");
                }
                if *t == "move.walker" {
                    assert_eq!(trace[i - 1], "move.clash", "{what}: at trace entry {i}");
                }
                if *t == "effects.wave.detune" {
                    assert_eq!(trace[i - 1], "knock.skips", "{what}: a ring knocks, then detunes");
                }
            }
            assert!(get("lance.swat_shells") > 0, "{what}: the lance thrusts");
            fights += 1;
        }
    }
    assert_eq!(fights, 8);
    assert!(beats > 8 * 50 && steps > 8 * 2000, "{beats} beats in {steps} steps");
}

/// A kit fighter is refused before a fight exactly when its weapon or one of its abilities is of a kind the engine
/// has not ported (each kind's `PORTED`), in words naming the fighter and the kind, on either side, and through
/// `setup_from` as the app calls it: never let into a fight where a stub would do nothing. Every other fighter
/// passes.
#[test]
fn every_fighter_not_ported_is_refused_in_words() {
    let roster = roster();
    let flute = roster.iter().find(|f| f.name == "FLUTE").expect("in the roster");
    let ff = &fight_files()[0];
    let (mut refused, mut passed) = (0, 0);
    for f in &roster {
        let weapon = matches!(f.weapon, bf_sim::roster::WeaponCfg::Unsupported(_));
        let ability = [&f.unlock1, &f.unlock2, &f.low]
            .into_iter()
            .find(|ab| !ab.kind.is_ported());
        for k in 0..2 {
            let mut fighters = [flute, flute];
            fighters[k] = f;
            let setup = bf_sim::Setup {
                fighters,
                rules: bf_sim::Rules::new(132.0, 3.5),
                seed: 1,
            };
            let mut file = ff.clone();
            file.fighters[k] = f.name.clone();
            file.fighters[1 - k] = "FLUTE".into();
            let want = if weapon {
                Some(format!(
                    "fighters[{k}]: the engine has not ported {}'s weapon kind",
                    f.name
                ))
            } else {
                ability.map(|ab| {
                    format!(
                        "fighters[{k}]: the engine has not ported {}'s ability {}",
                        f.name, ab.name
                    )
                })
            };
            match want {
                Some(want) => {
                    let err = setup.check().expect_err("refused").to_string();
                    assert!(err.starts_with(&want), "{err}");
                    assert!(bf_sim::Fight::new(&setup, false).is_err(), "{}", f.name);
                    assert!(run(&setup, false).is_err(), "{}", f.name);
                    let err = setup_from(&file, &roster, 1).expect_err("refused").to_string();
                    assert!(err.starts_with(&want), "{err}");
                }
                None => {
                    assert_eq!(setup.check(), Ok(()), "{}", f.name);
                    assert!(setup_from(&file, &roster, 1).is_ok(), "{}", f.name);
                }
            }
        }
        if weapon || ability.is_some() {
            refused += 1;
        } else {
            passed += 1;
        }
    }
    assert_eq!(refused + passed, 20);
    assert!(passed >= 2, "FLUTE and TROMBONE at least");
}
