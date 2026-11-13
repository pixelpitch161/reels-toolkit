//! Milestone 4, Task 3: the engine's sockets for the older ten fighters' kinds. Every place `fight.py` calls the code
//! of an older kind (`pulse`, `inflate`, `bite`, `trap`, `surf`, `scratch_floor`, `tether`, `wrecking`, `rests`, the
//! spinning blade, and the abilities `clamp`, `rewind`, `lash`, `spout`, `offbeat`, `snap`, `double_stop`, `melody`,
//! `joke`), or reads the per-ball radius, mass or rewind it brings, has its call in the shared engine files, in
//! Python's order, calling stubs; and none of them changes a fight.
//!
//! The older ten fighters' entries are `tests/older_roster.json` (exported from `fighters.py` as `roster.json` is).
//! Their kinds are stubs, so they load as `Unsupported` and fight only through the test entry `with_unported`: an
//! unported weapon does nothing, an unported ability is announced and does nothing, and every kind already ported
//! (their waves, hastes, shields, slams and arcs) runs. With `Fight::trace_sockets` on, each of them fights FLUTE
//! (both corners, two seeds) and its trace is checked against `tests/sockets_order.txt` as `tests/sockets.rs` checks
//! FLUTE against TROMBONE.

use std::collections::{BTreeMap, BTreeSet};

use bf_core::recording::Recording;
use bf_sim::roster::{
    AbilityKind, FighterCfg, OLDER_ABILITY_KINDS, OLDER_WEAPON_KINDS, WeaponCfg, blade, parse_roster,
};
use bf_sim::{Fight, Rules, Setup, mass, radius, rewinding, run_traced_with_unported, run_with_unported};

/// One line of `sockets_order.txt`.
#[derive(Clone, Debug)]
struct Socket {
    name: String,
    lines: String,
    when: String,
}

fn order() -> Vec<Socket> {
    include_str!("sockets_order.txt")
        .lines()
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

fn of(order: &[Socket], when: &[&str]) -> Vec<String> {
    order
        .iter()
        .filter(|s| when.contains(&s.when.as_str()))
        .map(|s| s.name.clone())
        .collect()
}

fn kit() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

fn older() -> Vec<FighterCfg> {
    parse_roster(include_str!("older_roster.json")).expect("the older roster loads")
}

/// The older ten, in `fighters.py`'s order.
const OLDER: [&str; 10] = [
    "DRUMS",
    "VIOLIN",
    "BAGPIPES",
    "ACCORDION",
    "HARP",
    "UKULELE",
    "TURNTABLES",
    "SYNTH",
    "BARI SAX",
    "VIOLA",
];

/// The sockets this task adds, each with its `fight.py` lines (the pre-flight scan's list).
const NEW: [(&str, &str); 25] = [
    ("setup.inflate", "5761-5762"),
    ("setup.wrecking", "5842-5846"),
    ("setup.rests", "5858-5863"),
    ("snap.RAD", "5520-5521"),
    ("snap.WRECK", "5577-5587"),
    ("snap.SNAP", "5588-5594"),
    ("snap.REST", "5616-5620"),
    ("beat.scratch_floor", "5942-5943"),
    ("beat.tether", "5944-5945"),
    ("tick.pulse", "5946-5963"),
    ("tick.grid16", "5975-5980"),
    ("tick.tether", "5981-5986"),
    ("tick.wrecking", "6040-6044"),
    ("tick.rests", "6045-6049"),
    ("move.clash_hooks", "3233-3234"),
    ("rewind.step", "6116-6117"),
    ("weapon.scratch_floor", "6127-6128"),
    ("weapon.tether", "6129-6130"),
    ("weapon.inflate", "6131-6132"),
    ("stage9.lashes", "6137-6138"),
    ("later.wrecking", "6167-6168"),
    ("later.snap", "6169-6170"),
    ("later.notes", "6191-6192"),
    ("later.clamps", "6201-6202"),
    ("damage.guard", "342-343"),
];

fn count(rec: &Recording, kind: &str) -> usize {
    rec.events.iter().filter(|e| e.kind == kind).count()
}

/// Every entry `block[0]` is followed directly by the rest of `block`, and those occur nowhere else.
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
fn the_order_file_lists_the_new_sockets_at_their_lines() {
    let order = order();
    for (name, lines) in NEW {
        let s = order.iter().filter(|s| s.name == name).collect::<Vec<_>>();
        assert!(!s.is_empty(), "{name} is not in sockets_order.txt");
        assert!(s.iter().all(|s| s.lines == lines), "{name}: {s:?}");
    }
    // per-fighter places twice, fighter 0's first
    for name in [
        "beat.scratch_floor",
        "beat.tether",
        "weapon.scratch_floor",
        "weapon.tether",
        "weapon.inflate",
    ] {
        assert_eq!(order.iter().filter(|s| s.name == name).count(), 2, "{name}");
    }
    let names: BTreeSet<&str> = order.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names.len(),
        BTreeSet::from_iter(order.iter().map(|s| (&s.name, &s.lines))).len()
    );
}

/// Each older fighter against FLUTE, both corners, seeds 1 and 2: the trace follows `sockets_order.txt` exactly (the
/// setup, frame `f0`, every step's unconditional sockets, every frame's), the conditional ones come in their blocks
/// and as often as the events say, no stub is ever reached, and tracing changes nothing.
#[test]
fn each_older_fighter_against_flute_reaches_the_sockets_in_fight_py_order() {
    let order = order();
    let known: BTreeSet<String> = order.iter().map(|s| s.name.clone()).collect();
    let setup_frame: Vec<String> = [of(&order, &["setup"]), of(&order, &["frame"])].concat();
    let frame = of(&order, &["frame"]);
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
    let (kit, older) = (kit(), older());
    let flute = kit.iter().find(|f| f.name == "FLUTE").expect("FLUTE");
    let mut seen = BTreeSet::new();
    let mut fights = 0;
    for name in OLDER {
        let x = older.iter().find(|f| f.name == name).expect("in older_roster.json");
        for k in 0..2 {
            for seed in [1, 2] {
                let mut fighters = [flute, flute];
                fighters[k] = x;
                let setup = Setup {
                    fighters,
                    rules: Rules::new(132.0, 3.5),
                    seed,
                };
                let what = format!("{} against {}, seed {seed}", fighters[0].name, fighters[1].name);
                let plain = run_with_unported(&setup, true).expect("runs");
                let (rec, trace) = run_traced_with_unported(&setup, true).expect("runs");
                assert_eq!(rec.hash(), plain.hash(), "{what}: tracing is only looking");
                assert_eq!(rec.events, plain.events, "{what}");
                for t in &trace {
                    assert!(!t.starts_with("stub:"), "{what}: {t} was reached");
                    assert!(known.contains(*t), "{what}: {t} is not in sockets_order.txt");
                    seen.insert(t.to_string());
                }
                let first = trace.iter().position(|t| *t == "clock").expect("a step");
                assert_eq!(trace[..first], setup_frame, "{what}: the setup and frame f0");
                let (mut at, mut frames_seen, mut step) = (first, 0, 0);
                while at < trace.len() {
                    let end = trace[at + 1..]
                        .iter()
                        .position(|t| *t == "clock")
                        .map_or(trace.len(), |i| at + 1 + i);
                    let seg = &trace[at..end];
                    let snap = seg.iter().position(|t| t.starts_with("snap.")).unwrap_or(seg.len());
                    if snap < seg.len() {
                        assert_eq!(seg[snap..], frame, "{what}: a frame after step {}", step + 1);
                        frames_seen += 1;
                    }
                    let got: Vec<&str> = seg[..snap].iter().copied().filter(|t| !is_conditional(t)).collect();
                    let want = if got.contains(&"beat.stunned") {
                        &step_with_beat
                    } else {
                        &step_alone
                    };
                    assert_eq!(got, *want, "{what}: step {}", step + 1);
                    step += 1;
                    at = end;
                }
                assert_eq!(frames_seen + 1, rec.frames(), "{what}");
                // the conditional blocks
                let n: BTreeMap<&str, usize> = trace.iter().fold(BTreeMap::new(), |mut m, t| {
                    *m.entry(*t).or_insert(0) += 1;
                    m
                });
                let get = |s: &str| n.get(s).copied().unwrap_or(0);
                blocks(&trace, &["damage.skips", "damage.guard", "damage.detuned"], &what);
                // a call of `damage` lands (`hit`) or meets a shield (`block`); FLUTE's fangs write their own blocks
                assert!(get("damage.skips") >= count(&rec, "hit"), "{what}");
                assert!(
                    get("damage.skips") <= count(&rec, "hit") + count(&rec, "block"),
                    "{what}"
                );
                // no walker here: every overlap is a clash, and the inflaters' hooks follow it
                blocks(&trace, &["move.walker", "move.wclash", "move.clash_hooks"], &what);
                assert_eq!(get("move.walker"), count(&rec, "clash"), "{what}");
                blocks(
                    &trace,
                    &["trigger.ability", "trigger.last_ab", "trigger.dispatch"],
                    &what,
                );
                assert_eq!(get("trigger.ability"), count(&rec, "ability"), "{what}");
                for (i, t) in trace.iter().enumerate() {
                    if *t == "move.walker" {
                        assert_eq!(trace[i - 1], "move.clash", "{what}: at trace entry {i}");
                    }
                }
                // the older kinds' sockets are reached and do nothing: no event of theirs
                for kind in [
                    "inflate",
                    "bite",
                    "pulse",
                    "string",
                    "surf",
                    "scratch",
                    "plug_fire",
                    "note",
                    "heal",
                ] {
                    assert_eq!(count(&rec, kind), 0, "{what}: {kind}");
                }
                assert!(
                    !rec.arrays.contains_key("RAD") && !rec.arrays.contains_key("X33_SPOUT"),
                    "{what}"
                );
                fights += 1;
            }
        }
    }
    assert_eq!(fights, 40);
    // every new socket that is not conditional was reached; the clash hooks were too (balls clash in every fight)
    for (name, _) in NEW {
        if name != "damage.guard" {
            assert!(seen.contains(name), "{name} was never reached");
        }
    }
    assert!(seen.contains("damage.guard"), "a hit was made");
}

/// A roster entry of each of the nine older weapon kinds, and the spinning blade, loads as `Unsupported` with its
/// kind's name exactly while its module's `PORTED` is false; the nine older ability kinds the same. Each kind of the
/// tables is some older fighter's.
#[test]
fn the_older_kinds_load_as_unsupported_until_ported() {
    let (mut weapons, mut abilities) = (BTreeSet::new(), BTreeSet::new());
    for f in older() {
        let kind = f.raw["weapon"]["kind"].as_str().expect("a kind").to_string();
        match OLDER_WEAPON_KINDS.iter().find(|(k, _)| *k == kind) {
            Some((_, ported)) => {
                assert_eq!(matches!(f.weapon, WeaponCfg::Unsupported(_)), !ported, "{}", f.name);
                if !ported {
                    assert_eq!(f.weapon, WeaponCfg::Unsupported(kind.clone()), "{}", f.name);
                }
                weapons.insert(kind);
            }
            None => {
                // VIOLIN's bow: a blade with no `mode`
                assert_eq!((f.name.as_str(), kind.as_str()), ("VIOLIN", "blade"));
                assert!(f.raw["weapon"].get("mode").is_none());
                assert_eq!(
                    matches!(f.weapon, WeaponCfg::Unsupported(_)),
                    !blade::SPIN_PORTED,
                    "{}",
                    f.name
                );
                if !blade::SPIN_PORTED {
                    assert_eq!(f.weapon, WeaponCfg::Unsupported("blade".into()));
                }
            }
        }
        for (ab, slot) in [(&f.unlock1, "unlock1"), (&f.unlock2, "unlock2"), (&f.low, "low")] {
            let kind = f.raw["abilities"][slot]["kind"].as_str().expect("a kind");
            assert_eq!(ab.kind.name(), kind, "{} {slot}", f.name);
            if let Some((_, ported)) = OLDER_ABILITY_KINDS.iter().find(|(k, _)| *k == kind) {
                assert_eq!(!ab.kind.is_ported(), !ported, "{} {slot}", f.name);
                if !ported {
                    assert_eq!(ab.kind, AbilityKind::Unsupported(kind.to_string()));
                }
                abilities.insert(kind.to_string());
            }
        }
    }
    let table = |t: &[(&str, bool)]| t.iter().map(|(k, _)| k.to_string()).collect::<BTreeSet<_>>();
    assert_eq!(
        weapons,
        table(&OLDER_WEAPON_KINDS),
        "one older fighter for each weapon kind"
    );
    assert_eq!(
        abilities,
        table(&OLDER_ABILITY_KINDS),
        "one older fighter for each ability kind"
    );
    // and the engine refuses each older fighter outside the test entry, naming it
    let kit = kit();
    let flute = kit.iter().find(|f| f.name == "FLUTE").expect("FLUTE");
    for f in older() {
        let setup = Setup {
            fighters: [&f, flute],
            rules: Rules::new(132.0, 3.5),
            seed: 1,
        };
        let err = setup.check().expect_err("refused").to_string();
        assert!(
            err.starts_with(&format!("fighters[0]: the engine has not ported {}'s", f.name)),
            "{err}"
        );
    }
}

/// `radius`, `mass` and `rewinding` give the shared `Rules::r`, 1.0 and false for every ball of the 30, all fight
/// long: no kind the engine runs sets a ball's own radius, mass or rewind, so no fight of milestone 1 or 3 changes.
#[test]
fn every_ball_has_the_shared_radius_and_mass_and_never_rewinds() {
    let (kit, older) = (kit(), older());
    let flute = kit.iter().find(|f| f.name == "FLUTE").expect("FLUTE");
    for f in kit.iter().chain(older.iter()) {
        let setup = Setup {
            fighters: [f, flute],
            rules: Rules::new(132.0, 3.5),
            seed: 3,
        };
        let mut fight = Fight::with_unported(&setup, false).expect("runs");
        let r = fight.rules.r;
        let mut sched = bf_sim::schedule::Schedule::new(&mut fight);
        let mut steps = 0;
        while fight.t < fight.rules.t_max && fight.winner.is_none() && steps < 3000 {
            sched.step(&mut fight);
            for k in 0..2 {
                assert_eq!(radius(&fight, k).to_bits(), r.to_bits(), "{} {k}", f.name);
                assert_eq!(mass(&fight, k).to_bits(), 1.0f64.to_bits(), "{} {k}", f.name);
                assert!(!rewinding(&fight, k), "{} {k}", f.name);
                assert!(
                    fight.balls[k].rad.is_none() && fight.balls[k].mass.is_none(),
                    "{} {k}",
                    f.name
                );
                assert!(!fight.balls[k].resting, "{} {k}", f.name);
            }
            if fight.balls.iter().any(|b| b.hp <= 0.0) {
                break;
            }
            steps += 1;
        }
        assert!(steps > 500, "{}: {steps} steps", f.name);
    }
    // and what the helpers read: a ball's own values when it has them
    let setup = Setup {
        fighters: [flute, flute],
        rules: Rules::new(132.0, 3.5),
        seed: 1,
    };
    let mut fight = Fight::new(&setup, false).expect("runs");
    fight.balls[1].rad = Some(0.7);
    fight.balls[1].mass = Some(1.9);
    fight.balls[1].rw_left = 0.25;
    assert_eq!(
        (radius(&fight, 1), mass(&fight, 1), rewinding(&fight, 1)),
        (0.7, 1.9, true)
    );
    assert_eq!(
        (radius(&fight, 0), mass(&fight, 0), rewinding(&fight, 0)),
        (fight.rules.r, 1.0, false)
    );
}

/// A swollen ball (BAG INFLATE's `rad` and `mass`, set by hand here as `inflate_step` will) bounces off the ropes at
/// its own radius and clashes through `move`'s unequal-mass branch; a rewinding ball takes no hit and does not clash.
#[test]
fn the_per_ball_radius_mass_and_rewind_reach_move_and_damage() {
    let kit = kit();
    let flute = kit.iter().find(|f| f.name == "FLUTE").expect("FLUTE");
    let setup = Setup {
        fighters: [flute, flute],
        rules: Rules::new(132.0, 3.5),
        seed: 1,
    };
    // the rope: a ball of radius 0.9 at a - 0.8 has crossed it, one of the shared radius has not
    let mut fight = Fight::new(&setup, false).expect("runs");
    let (a, r) = (fight.rules.a, fight.rules.r);
    assert!(r < 0.8, "the shared radius {r}");
    fight.balls[0].p = [a - 0.8, 0.0];
    fight.balls[0].v = [1.0, 0.0];
    fight.balls[1].p = [-a + 2.0, -a + 2.0];
    fight.balls[0].rad = Some(0.9);
    fight.move_step(1e-6);
    assert!(fight.events.iter().any(|e| e.kind == "rope"), "{:?}", fight.events);
    assert!(fight.balls[0].v[0] < 0.0);
    // the clash: equal masses split the overlap evenly; a heavier ball gives less
    let clash = |m: Option<f64>| {
        let mut fight = Fight::new(&setup, false).expect("runs");
        fight.balls[0].p = [-0.2, 0.0];
        fight.balls[1].p = [0.2, 0.0];
        fight.balls[0].v = [1.0, 0.0];
        fight.balls[1].v = [-1.0, 0.0];
        fight.balls[0].mass = m;
        fight.move_step(1e-9);
        (
            fight.balls[0].p[0],
            fight.balls[1].p[0],
            fight.events.iter().filter(|e| e.kind == "clash").count(),
        )
    };
    let (p0, q0, n0) = clash(None);
    let (p1, q1, n1) = clash(Some(3.0));
    let (p2, q2, _) = clash(Some(1.0));
    assert_eq!((n0, n1), (1, 1));
    assert!(((p0 + 0.2) + (q0 - 0.2)).abs() < 1e-12, "even: {p0} {q0}");
    assert!(
        (p1 + 0.2).abs() < (q1 - 0.2).abs(),
        "the heavy ball gives less: {p1} {q1}"
    );
    // a mass of 1.0 takes the unequal-mass branch (Python's `hasattr`), the same push to the last bits or close
    assert!((p2 - p0).abs() < 1e-12 && (q2 - q0).abs() < 1e-12);
    // rewinding: no clash, no hit
    let mut fight = Fight::new(&setup, false).expect("runs");
    fight.balls[0].p = [-0.2, 0.0];
    fight.balls[1].p = [0.2, 0.0];
    fight.balls[1].rw_left = 0.5;
    fight.move_step(1e-9);
    assert!(fight.events.iter().all(|e| e.kind != "clash"));
    // `move` skips a rewinding ball (rewind_step drives it)
    assert_eq!(fight.balls[1].p, [0.2, 0.0]);
    let hp = fight.balls[1].hp;
    assert!(!fight.damage(0, 1, 5.0, "test", [0.0, 0.0], 0));
    assert_eq!(fight.balls[1].hp, hp);
    // resting: x guard
    fight.balls[1].rw_left = 0.0;
    fight.balls[1].resting = true;
    fight.balls[1].guard = 0.5;
    assert!(fight.damage(0, 1, 4.0, "test", [0.0, 0.0], 0));
    let d = hp - fight.balls[1].hp;
    assert!((d - 4.0 * fight.rules.dmg_scale * 0.5).abs() < 1e-12, "{d}");
}
