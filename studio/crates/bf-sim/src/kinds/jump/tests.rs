//! The jump's unit tests, with RHAPSODY's and SQUEAK's (`abilities/stun.rs`: the stun is proven here, beside the
//! jumper it stops). Every expected number is what `fight.py` gave on the reference PC (CPython 3.13, numpy 2.5) for
//! the same hand-built state: a `Fight('CLARINET', 'FLUTE', seed=392)` under the example episode's rules with
//! `jump_setup` called and the clock at 12.5 s, its balls set as the test sets them, then the same methods called.
//! The script that prints them is `tools/studio_export/unit_checks/jump_unit_vectors.py`; the functions from
//! [`cases`] to the end of the file are its output, pasted (and wrapped by `cargo fmt`).
//!
//! Tolerances, as the striker's tests: every number that passes through `hypot`, `cos` or `sin` (a jump's spots,
//! headings and contact points) is held to 1e-12, because numpy's `hypot` is the C runtime's and `geom::hypot` is
//! `dmath`'s (they differ in the last bit for about one pair in six: `lane-fight-notes.md`, "Numeric facts"); whole
//! numbers, texts, times, damage, health, boosts, the reach and the generator's draws are compared exactly.

use bf_core::event::Value::{self, Float as F, Int as I, Str as S};

use super::*;
use crate::abilities::{effects_step, trigger, trigger_quiet};
use crate::ball::Slot;
use crate::fight::Setup;
use crate::roster::FighterCfg;
use crate::schedule::Schedule;
use crate::testing;

/// The clock of every hand-built fight.
const T: f64 = 12.5;

/// An event as the script prints it: time, kind, fields (the first is `who`).
type Ev = (f64, &'static str, Vec<Value>);

/// One `_jump` call: the state before, the arguments, the state after.
struct Case {
    name: &'static str,
    k: usize,
    p0: V2,
    v0: V2,
    op: V2,
    ov: V2,
    shield: f64,
    blade: f64,
    j: Jump,
    p1: V2,
    v1: V2,
    ov1: V2,
    oboost: f64,
    ohp: f64,
    dealt: f64,
    blade1: f64,
    events: Vec<Ev>,
}

/// The jumper after one `jump_tick`: its locked target, spot, velocity, reach, chain `(n, side, next)` and the number
/// of events so far.
struct Row {
    k16: i64,
    tgt: Option<V2>,
    p: V2,
    v: V2,
    blade: f64,
    rhap: Option<(f64, f64, Option<f64>)>,
    events: usize,
}

/// A run of `jump_tick` over some 16ths: a row a tick, every event, and the fight generator's next draw afterwards.
struct Run {
    rows: Vec<Row>,
    events: Vec<Ev>,
    next_draw: f64,
}

/// A SQUEAK ring followed until its front passes the enemy: the step and time of the hit, its events, and the enemy
/// afterwards.
struct Squeak {
    step: usize,
    t: f64,
    events: Vec<Ev>,
    stun: f64,
    v: V2,
    boost: f64,
    hp: f64,
    waves: usize,
    stunned: bool,
}

/// The roster with CLARINET's jump changed.
fn roster_with(change: impl Fn(&mut JumpCfg)) -> Vec<FighterCfg> {
    let mut roster = testing::roster();
    for f in &mut roster {
        if let WeaponCfg::Jump(w) = &mut f.weapon {
            change(w);
        }
    }
    roster
}

/// `left` against `right` under the episode's rules at 12.5 s, each jumper set up.
fn fight<'a>(roster: &'a [FighterCfg], left: &str, right: &str) -> Fight<'a> {
    let s = Setup {
        fighters: [testing::fighter(roster, left), testing::fighter(roster, right)],
        rules: testing::episode_rules(),
        seed: 392,
    };
    let mut f = testing::fight(&s, false);
    f.t = T;
    for k in 0..2 {
        if is(&f, k) {
            setup(&mut f, k);
        }
    }
    f
}

/// A float within 1e-12 of Python's.
#[track_caller]
fn close(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 1e-12, "{what}: {got} vs Python's {want}");
}

/// A 2-vector within 1e-12 of Python's.
#[track_caller]
fn close2(got: V2, want: V2, what: &str) {
    close(got[0], want[0], what);
    close(got[1], want[1], what);
}

/// The events equal Python's: the time, the kind and every whole number and text exactly, floats to 1e-12.
#[track_caller]
fn assert_events(got: &[bf_core::event::Event], want: &[Ev], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?}");
    for (g, (t, kind, fields)) in got.iter().zip(want) {
        assert_eq!((g.t, g.kind.as_str()), (*t, *kind), "{what}");
        assert_eq!(g.fields.len(), fields.len(), "{what}: {g:?}");
        for (a, b) in g.fields.iter().zip(fields) {
            match (a, b) {
                (Value::Float(x), Value::Float(y)) => close(*x, *y, what),
                _ => assert_eq!(a, b, "{what}: {g:?}"),
            }
        }
    }
}

/// `jump_setup` (2349 to 2354): the reach is the weapon's `base`, there is no target and no chain; and the roster's
/// CLARINET is a jumper on either side, FLUTE is not.
#[test]
fn setup_starts_at_the_base_reach() {
    let roster = testing::roster();
    for (pair, k) in [(("CLARINET", "FLUTE"), 0), (("FLUTE", "CLARINET"), 1)] {
        let mut f = fight(&roster, pair.0, pair.1);
        assert!(is(&f, k) && !is(&f, 1 - k));
        f.balls[k].blade_len = 9.0;
        f.jump.jump_tgt[k] = Some([1.0, 1.0]);
        start_rhapsody(&mut f, k, &testing::fighter(&roster, "CLARINET").low);
        setup(&mut f, k);
        assert_eq!(f.balls[k].blade_len, 2.6);
        assert_eq!(f.jump, State::default());
    }
}

/// `_jump` (2359 to 2394), every branch: the ball's new spot and heading, the enemy's knock, the damage with the
/// episode's handicap, the reach's growth and cap, and the events (`hit` of kind `cut` before `jump`; the jump's
/// last two fields are hit and kind).
#[test]
fn a_jump_lands_where_pythons_does() {
    let roster = testing::roster();
    let all = cases();
    assert_eq!(all.len(), 18);
    for c in &all {
        let mut f = if c.k == 0 {
            fight(&roster, "CLARINET", "FLUTE")
        } else {
            fight(&roster, "FLUTE", "CLARINET")
        };
        let (k, o) = (c.k, 1 - c.k);
        f.balls[k].p = c.p0;
        f.balls[k].v = c.v0;
        f.balls[o].p = c.op;
        f.balls[o].v = c.ov;
        f.balls[o].shield = c.shield;
        f.balls[k].blade_len = c.blade;
        jump(&mut f, k, c.j);
        close2(f.balls[k].p, c.p1, c.name);
        close2(f.balls[k].v, c.v1, c.name);
        assert_eq!(f.balls[o].p, c.op, "{}: the enemy stays", c.name);
        close2(f.balls[o].v, c.ov1, c.name);
        assert_eq!(
            (f.balls[o].boost, f.balls[o].hp, f.balls[k].dealt, f.balls[k].blade_len),
            (c.oboost, c.ohp, c.dealt, c.blade1),
            "{}",
            c.name
        );
        assert_events(&f.events, &c.events, c.name);
        // a jump draws nothing itself, and touches neither the lock nor a chain
        assert_eq!(f.jump, State::default(), "{}", c.name);
    }
    // each branch is in the table: a miss, a cut, a block, the growth, the cap, a chain jump
    let kinds = |c: &Case| c.events.iter().map(|e| e.1).collect::<Vec<_>>();
    assert!(all.iter().any(|c| kinds(c) == ["jump"]));
    assert!(all.iter().any(|c| kinds(c) == ["hit", "jump"] && c.blade1 > c.blade));
    assert!(all.iter().any(|c| kinds(c) == ["block", "jump"] && c.blade1 == c.blade));
    assert!(all.iter().any(|c| c.blade1 == 4.2 && c.blade < 4.2));
    assert!(
        all.iter()
            .any(|c| c.j.kind == 1 && kinds(c) == ["hit", "jump"] && c.blade1 == c.blade)
    );
}

/// Drives `jump_tick` over a run's 16ths as the script did: the jumper starts at a fixed spot, the enemy walks a
/// fixed path between 16ths (so a lock and a jump see different spots), `before` changes the fight before a tick.
fn drive<'a>(run: &Run, f: &mut Fight<'a>, k: usize, before: impl Fn(&mut Fight<'a>, i64)) {
    f.balls[k].p = [-1.2, 0.4];
    f.balls[k].v = [2.4, 1.0];
    for row in &run.rows {
        let k16 = row.k16;
        f.balls[1 - k].p = [0.6 + 0.11 * k16 as f64, -0.9 + 0.07 * k16 as f64];
        f.balls[1 - k].v = [1.0, 2.4];
        before(f, k16);
        tick(f, k, k16);
        // a lock is a sum and two products of the enemy's own numbers: exact
        assert_eq!(f.jump.jump_tgt[k], row.tgt, "16th {k16}: the locked target");
        close2(f.balls[k].p, row.p, &format!("16th {k16}: its spot"));
        close2(f.balls[k].v, row.v, &format!("16th {k16}: its heading"));
        assert_eq!(f.balls[k].blade_len, row.blade, "16th {k16}: the reach");
        let rhap = f.jump.rhap[k].as_ref().map(|r| (r.n, r.side, r.next));
        assert_eq!(rhap, row.rhap, "16th {k16}: the chain");
        assert_eq!(f.events.len(), row.events, "16th {k16}: events so far");
    }
    assert_events(&f.events, &run.events, "the run's events");
    // one draw from the fight's generator for every regular jump, none for anything else
    assert_eq!(
        f.rng.random(),
        run.next_draw,
        "the generator stands where Python's does"
    );
}

/// `jump_tick` (2396 to 2424): the target locks `lock16` 16ths before a jump (the enemy's spot then, plus `lead` x
/// its motion until the jump), the jump comes every `every` beats from the second beat on and uses the lock.
#[test]
fn the_grid_the_lock_and_the_lead() {
    let roster = roster_with(|w| w.lead = 0.5);
    let mut f = fight(&roster, "CLARINET", "FLUTE");
    drive(&run_grid(), &mut f, 0, |_, _| {});
}

/// Haste divides the grid (`ev16 = max(1, int(round(main16 / ab_mult)))`), and the lock is at most `ev16 - 1` 16ths
/// before.
#[test]
fn haste_halves_the_grid() {
    let roster = testing::roster();
    let mut f = fight(&roster, "CLARINET", "FLUTE");
    f.balls[0].haste = 1.0;
    f.balls[0].haste_mult = 2.0;
    drive(&run_haste(), &mut f, 0, |_, _| {});
}

/// With `lock16 = 0` nothing locks and the jump goes for the enemy as it is; without jitter the draw is still made.
#[test]
fn no_lock_jumps_at_the_enemy_as_it_is() {
    let roster = roster_with(|w| {
        w.lock16 = 0;
        w.jitter = 0.0;
    });
    let mut f = fight(&roster, "CLARINET", "FLUTE");
    drive(&run_no_lock(), &mut f, 0, |_, _| {});
}

/// A stunned jumper (or one sunk in shadow) loses its lock, makes no jump and draws nothing.
#[test]
fn a_stunned_jumper_loses_its_lock_and_stays() {
    let roster = testing::roster();
    let mut f = fight(&roster, "CLARINET", "FLUTE");
    drive(&run_dazed(), &mut f, 0, |f, k16| {
        f.balls[0].stun = if k16 == 2 || k16 == 3 { 0.3 } else { 0.0 };
        f.balls[0].shadow = if k16 == 8 { 0.2 } else { 0.0 };
    });
}

/// RHAPSODY (`trigger` 2930 to 2932, the chain in `jump_tick`): five jumps two 16ths apart from the next 16th on,
/// the side flipping, each a cut of kind 1 that grows nothing; regular jumps pause and no draw is made meanwhile;
/// then the grid goes on.
#[test]
fn rhapsody_is_a_chain_of_five() {
    let roster = testing::roster();
    let mut f = fight(&roster, "CLARINET", "FLUTE");
    drive(&run_chain(), &mut f, 0, |f, k16| {
        if k16 == 5 {
            trigger(f, 0, Slot::Low);
        }
    });
}

/// A chain fired quietly with other numbers and no `cut` of its own: it takes the weapon's (`ab.get('cut',
/// w['cut'])`).
#[test]
fn a_chain_without_a_cut_takes_the_weapons() {
    let roster = testing::roster();
    let mut ab = testing::fighter(&roster, "CLARINET").low.clone();
    ab.params.remove("cut");
    ab.params.insert("n".into(), 2.0);
    ab.params.insert("step16".into(), 3.0);
    let mut f = fight(&roster, "CLARINET", "FLUTE");
    drive(&run_chain_plain(), &mut f, 0, |f, k16| {
        if k16 == 5 {
            trigger_quiet(f, 0, &ab);
        }
    });
}

/// The same code for the right corner's ball.
#[test]
fn the_right_corner_jumps_too() {
    let roster = testing::roster();
    let mut f = fight(&roster, "FLUTE", "CLARINET");
    drive(&run_right(), &mut f, 1, |_, _| {});
}

/// Fires SQUEAK for fighter `k` (quietly with `stun` changed, when given) at an enemy 1.58 away and steps the
/// effects until an event comes.
fn squeak<'a>(
    roster: &'a [FighterCfg],
    pair: (&str, &str),
    k: usize,
    shield: f64,
    stun0: f64,
    ab: Option<&'a AbilityCfg>,
) -> (Fight<'a>, usize, usize) {
    let mut f = fight(roster, pair.0, pair.1);
    let dt = f.rules.dt;
    f.balls[k].p = [0.0, 0.0];
    f.balls[1 - k].p = [1.5, 0.5];
    f.balls[1 - k].v = [0.0, 2.6];
    f.balls[1 - k].shield = shield;
    f.balls[1 - k].stun = stun0;
    match ab {
        Some(ab) => trigger_quiet(&mut f, k, ab),
        None => trigger(&mut f, k, Slot::Unlock1),
    }
    // the ring `trigger` makes (2889 to 2892)
    let w = f.waves.last().expect("a ring");
    assert_eq!((w.owner, w.c, w.r, w.speed, w.rmax), (k, [0.0, 0.0], 0.6, 9.0, 2.4));
    assert_eq!(
        (w.dmg, w.knock, w.kind.as_str(), w.hit, w.name.as_str()),
        (2.0, 0.4, "stun", false, "SQUEAK")
    );
    assert_eq!((w.t0, w.slow, w.grow, w.clears), (T, None, false, true));
    assert!(w.detune.is_none());
    let n0 = f.events.len();
    let mut step = 0;
    for i in 0..60 {
        f.t = T + (i + 1) as f64 * dt;
        effects_step(&mut f, dt);
        if f.events.len() > n0 {
            step = i + 1;
            break;
        }
    }
    (f, step, n0)
}

#[track_caller]
fn assert_squeak(f: &Fight, step: usize, n0: usize, k: usize, want: &Squeak) {
    let o = &f.balls[1 - k];
    assert_eq!((step, f.t), (want.step, want.t));
    assert_events(&f.events[n0..], &want.events, "the ring's events");
    assert_eq!((o.stun, o.boost, o.hp), (want.stun, want.boost, want.hp));
    close2(o.v, want.v, "the knocked enemy's heading");
    assert_eq!((f.waves.len(), stunned(f, 1 - k)), (want.waves, want.stunned));
}

/// What every SQUEAK case shares: the ring passes the enemy on step 10, knocks it and is still spreading.
fn landed(events: Vec<Ev>, stun: f64, hp: f64, stunned: bool) -> Squeak {
    Squeak {
        step: 10,
        t: 12.583333333333334,
        events,
        stun,
        v: [2.2325718785852158, 1.3325249742256937],
        boost: 0.4,
        hp,
        waves: 1,
        stunned,
    }
}

/// The ring's hit on fighter `vic` by `att` for `d` damage.
fn ring_hit(att: i64, vic: i64, d: f64) -> Ev {
    let at = [F(0.9307900211696918), F(0.3102633403898973)];
    let [x, y] = at;
    (
        12.58333,
        "hit",
        vec![I(att), I(vic), F(d), x, y, S("stun".into()), I(0)],
    )
}

/// SQUEAK (`trigger` 2889 to 2892, `effects_step` 3072 to 3074): the ring's hit is of kind `stun`, and the fighter it
/// lands on is stunned for the ability's `stun` seconds (event `stun`: the victim, the owner, the seconds).
#[test]
fn squeak_stuns_the_fighter_its_ring_lands_on() {
    let roster = testing::roster();
    let (f, step, n0) = squeak(&roster, ("CLARINET", "FLUTE"), 0, 0.0, 0.0, None);
    assert_eq!(n0, 1, "the ability event");
    let want = landed(
        vec![ring_hit(0, 1, 4.0), (12.58333, "stun", vec![I(1), I(0), F(0.5)])],
        0.5,
        96.0,
        true,
    );
    assert_squeak(&f, step, n0, 0, &want);
    // a longer stun already running is kept (`max`)
    let (f, step, n0) = squeak(&roster, ("CLARINET", "FLUTE"), 0, 0.0, 0.75, None);
    let want = landed(
        vec![ring_hit(0, 1, 4.0), (12.58333, "stun", vec![I(1), I(0), F(0.5)])],
        0.75,
        96.0,
        true,
    );
    assert_squeak(&f, step, n0, 0, &want);
    // from the right corner (a copied SQUEAK runs for whichever ball fires it)
    let (f, step, n0) = squeak(&roster, ("FLUTE", "CLARINET"), 1, 0.0, 0.0, None);
    let want = landed(
        vec![ring_hit(1, 0, 0.8), (12.58333, "stun", vec![I(0), I(1), F(0.5)])],
        0.5,
        99.2,
        true,
    );
    assert_squeak(&f, step, n0, 1, &want);
}

/// A shielded enemy blocks the ring and is not stunned (the stun needs a landed hit); a `stun` of 0 stuns nobody
/// (`W.get('stun')` is false).
#[test]
fn a_blocked_or_empty_squeak_stuns_nobody() {
    let roster = testing::roster();
    let (f, step, n0) = squeak(&roster, ("CLARINET", "FLUTE"), 0, 1.0, 0.0, None);
    let at = vec![
        I(1),
        I(0),
        F(0.9307900211696918),
        F(0.3102633403898973),
        S("stun".into()),
    ];
    assert_squeak(
        &f,
        step,
        n0,
        0,
        &landed(vec![(12.58333, "block", at)], 0.0, 100.0, false),
    );
    let mut ab = testing::fighter(&roster, "CLARINET").unlock1.clone();
    ab.params.insert("stun".into(), 0.0);
    let (f, step, n0) = squeak(&roster, ("CLARINET", "FLUTE"), 0, 0.0, 0.0, Some(&ab));
    assert_eq!(n0, 0, "fired quietly");
    assert_squeak(&f, step, n0, 0, &landed(vec![ring_hit(0, 1, 4.0)], 0.0, 96.0, false));
}

/// `move` 3135 and the timers (6100 to 6101): a stunned ball holds still while the stun runs down a step at a time.
#[test]
fn a_stunned_ball_holds_still() {
    let roster = testing::roster();
    let (mut f, _, _) = squeak(&roster, ("CLARINET", "FLUTE"), 0, 0.0, 0.0, None);
    let dt = f.rules.dt;
    let jumper = f.balls[0].p;
    for stun in [0.49166666666666664, 0.4833333333333333, 0.4749999999999999] {
        f.move_step(dt);
        f.balls[1].stun = geom::py_max(0.0, f.balls[1].stun - dt);
        assert_eq!(f.balls[1].p, [1.5, 0.5], "the stunned FLUTE stays");
        close2(f.balls[1].v, [2.2325718785852158, 1.3325249742256937], "heading kept");
        assert_eq!(f.balls[1].stun, stun);
    }
    assert_ne!(f.balls[0].p, jumper, "the CLARINET moves on");
}

/// The fight loop itself: a FLUTE stunned for two seconds in a running fight does not move, drops no basket and
/// starts no strike until the stun has run out, while the CLARINET jumps on; afterwards it moves and fights again.
/// A twin fight without the stun shows what the FLUTE would have done in the same steps: a basket and strikes.
#[test]
fn a_stunned_flute_starts_nothing_in_the_fight_loop() {
    let roster = testing::roster();
    let s = Setup {
        fighters: [
            testing::fighter(&roster, "CLARINET"),
            testing::fighter(&roster, "FLUTE"),
        ],
        rules: testing::episode_rules(),
        seed: 392,
    };
    let count = |f: &Fight, from: usize, kind: &str, who: i64| {
        f.events[from..]
            .iter()
            .filter(|e| e.kind == kind && e.fields[0] == I(who))
            .count()
    };
    let mut f = testing::fight(&s, false);
    let mut sched = Schedule::new(&mut f);
    f.t = f.rules.t_fight;
    let dt = f.rules.dt;
    // four beats of plain fighting
    let steps_a_beat = (f.rules.beat / dt).round() as usize;
    for _ in 0..4 * steps_a_beat {
        sched.step(&mut f);
    }
    assert!(count(&f, 0, "basket", 1) >= 1, "{:?}", f.events);
    let (mut twin, mut twin_sched) = (f.clone(), sched.clone());
    // stunned for two seconds: more than the four beats between two baskets
    f.balls[1].stun = 2.0;
    let (p0, n0) = (f.balls[1].p, f.events.len());
    let (mut p, mut steps, mut still) = (p0, 0, 0);
    while f.balls[1].stun > 0.0 {
        assert!(stunned(&f, 1));
        let n = f.events.len();
        sched.step(&mut f);
        twin_sched.step(&mut twin);
        steps += 1;
        // only a clash (the two balls pushed apart, `move` 3209 to 3226) may shift a stunned ball; in the step
        // its stun runs out (the timers come before `move`) it moves on
        if count(&f, n, "clash", -1) == 0 && f.balls[1].stun > 0.0 {
            assert_eq!(f.balls[1].p, p, "step {steps}: a stunned FLUTE does not move");
            still += 1;
        }
        p = f.balls[1].p;
    }
    assert!(still > 100, "still on {still} of {steps} steps");
    assert!((239..=241).contains(&steps), "two seconds of steps, not {steps}");
    assert_eq!(count(&f, n0, "basket", 1), 0, "no basket while stunned");
    assert_eq!(count(&f, n0, "strike", 1), 0, "no strike while stunned");
    assert!(count(&f, n0, "jump", 0) >= 4, "the CLARINET jumped on");
    // the same steps without the stun: it moved, dropped a basket and struck
    assert_ne!(twin.balls[1].p, p0);
    assert!(count(&twin, n0, "basket", 1) >= 1, "{:?}", &twin.events[n0..]);
    assert!(count(&twin, n0, "strike", 1) >= 1, "{:?}", &twin.events[n0..]);
    // and then it goes on
    let n1 = f.events.len();
    for _ in 0..8 * steps_a_beat {
        sched.step(&mut f);
    }
    assert_ne!(f.balls[1].p, p, "it moves again");
    assert!(count(&f, n1, "basket", 1) >= 1, "{:?}", &f.events[n1..]);
}

// ---------------------------------------------------------------------------------------------------------------
// The output of `tools/studio_export/unit_checks/jump_unit_vectors.py`, pasted.

/// `_jump` on hand-built state: what `fight.py` gave.
fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "a cut through the enemy",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.7329924047901797, 0.7054987341316967],
            v1: [2.3006462076931715, 0.38344103461552875],
            ov1: [0.6753842248988692, 2.510748125309307],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 2.66,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(7.2),
                        F(0.5986393923831788),
                        F(-0.09183635429907255),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.7329924047901797),
                        F(0.7054987341316967),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
        Case {
            name: "turned off the line",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 33.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.361265279407181, 1.4059371493032342],
            v1: [2.0948347407727863, 1.025508366056276],
            ov1: [2.3844790152090445, -1.0364650626089198],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 2.66,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(7.2),
                        F(0.23619014925431403),
                        F(1.0388917912248608),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.361265279407181),
                        F(1.4059371493032342),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
        Case {
            name: "an enemy that moved on is missed",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.4, 2.1],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.564624201963574, 0.6774373669939291],
            v1: [2.3006462076931715, 0.38344103461552864],
            ov1: [0.3, 2.5],
            oboost: 0.0,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![(
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-1.0),
                    F(0.25),
                    F(1.564624201963574),
                    F(0.6774373669939291),
                    I(0),
                    I(0),
                ],
            )],
        },
        Case {
            name: "turned the other way",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: -17.5,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.7267902529404835, 0.2602382947795367],
            v1: [2.3323643173762627, 0.008757341489254715],
            ov1: [0.9943423913361278, 2.4023495184461203],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 2.66,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(7.2),
                        F(0.5022528075125258),
                        F(-0.0999957707003537),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.7267902529404835),
                        F(0.2602382947795367),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
        Case {
            name: "the reach caps a long jump",
            k: 0,
            p0: [-2.2, -2.0],
            v0: [0.0, 2.6],
            op: [2.0, 2.2],
            ov: [-2.6, 0.0],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [2.0, 2.2],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [-0.36152236891497647, -0.1615223689149763],
            v1: [1.8384776310850237, 1.8384776310850237],
            ov1: [-2.6, 0.0],
            oboost: 0.0,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![(
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-2.2),
                    F(-2.0),
                    F(-0.36152236891497647),
                    F(-0.1615223689149763),
                    I(0),
                    I(0),
                ],
            )],
        },
        Case {
            name: "the ropes stop it",
            k: 0,
            p0: [0.5, 1.0],
            v0: [2.0, 1.0],
            op: [1.9, 2.1],
            ov: [0.0, -2.6],
            shield: 0.0,
            blade: 4.2,
            j: Jump {
                tgt: [1.9, 2.1],
                theta: 0.0,
                land: 1.5,
                reach: 4.2,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [2.4, 2.4],
            v1: [1.8001595780051451, 1.326433373266949],
            ov1: [-0.5873981906655946, 2.5327777963296345],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 4.2,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(7.2),
                        F(2.2559194228299075),
                        F(1.6169664975879774),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (12.5, "jump", vec![I(0), F(0.5), F(1.0), F(2.4), F(2.4), I(1), I(0)]),
            ],
        },
        Case {
            name: "never inside the enemy",
            k: 0,
            p0: [0.0, 0.0],
            v0: [2.6, 0.0],
            op: [1.4, 0.3],
            ov: [0.0, 2.6],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.2, 0.1],
                theta: 0.0,
                land: 1.0,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [0.42820309397358014, 1.0862002120563679],
            v1: [0.9535536784480139, 2.41882934129679],
            ov1: [0.0, 2.6],
            oboost: 0.0,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![(
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.0),
                    F(0.0),
                    F(0.42820309397358014),
                    F(1.0862002120563679),
                    I(0),
                    I(0),
                ],
            )],
        },
        Case {
            name: "away from the enemy",
            k: 0,
            p0: [0.0, 0.0],
            v0: [2.6, 0.0],
            op: [1.4, 0.3],
            ov: [0.0, 2.6],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [-0.2, 0.1],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [-1.5416407864998738, 0.7708203932499369],
            v1: [-2.3255106965997814, 1.1627553482998907],
            ov1: [0.0, 2.6],
            oboost: 0.0,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![(
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.0),
                    F(0.0),
                    F(-1.5416407864998738),
                    F(0.7708203932499369),
                    I(0),
                    I(0),
                ],
            )],
        },
        Case {
            name: "landing on the enemy steps out along the jump",
            k: 0,
            p0: [0.0, 0.0],
            v0: [2.6, 0.0],
            op: [2.0, 0.0],
            ov: [0.0, 2.6],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [1.0, 0.0],
                theta: 0.0,
                land: 1.0,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [2.4, 0.0],
            v1: [2.6, 0.0],
            ov1: [0.9463266328395895, 2.421665935665927],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 2.66,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![I(0), I(1), F(7.2), F(2.0), F(-0.59999999999975), S("cut".into()), I(0)],
                ),
                (12.5, "jump", vec![I(0), F(0.0), F(0.0), F(2.4), F(0.0), I(1), I(0)]),
            ],
        },
        Case {
            name: "a target on the ball jumps along its heading",
            k: 0,
            p0: [0.5, -0.5],
            v0: [1.5, 2.0],
            op: [-1.5, 1.0],
            ov: [1.0, 1.0],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, -0.5],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.39999999999964, 0.6999999999995199],
            v1: [1.5000000000000002, 2.0],
            ov1: [1.0, 1.0],
            oboost: 0.0,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![(
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.5),
                    F(-0.5),
                    F(1.39999999999964),
                    F(0.6999999999995199),
                    I(0),
                    I(0),
                ],
            )],
        },
        Case {
            name: "the enemy on the line is cut from its middle",
            k: 0,
            p0: [0.0, 0.0],
            v0: [0.0, 2.6],
            op: [1.0, 0.0],
            ov: [0.0, -2.6],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [1.0, 0.0],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [2.4, 0.0],
            v1: [2.6, 0.0],
            ov1: [1.4823463431644377, 2.1360358889507163],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 2.66,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![I(0), I(1), F(7.2), F(1.0), F(-0.59999999999975), S("cut".into()), I(0)],
                ),
                (12.5, "jump", vec![I(0), F(0.0), F(0.0), F(2.4), F(0.0), I(1), I(0)]),
            ],
        },
        Case {
            name: "a jump of no length",
            k: 0,
            p0: [2.4, 2.4],
            v0: [2.6, 0.0],
            op: [0.0, 0.0],
            ov: [0.0, 2.6],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [3.4, 3.4],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [2.4, 2.4],
            v1: [2.6, 0.0],
            ov1: [0.0, 2.6],
            oboost: 0.0,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![(12.5, "jump", vec![I(0), F(2.4), F(2.4), F(2.4), F(2.4), I(0), I(0)])],
        },
        Case {
            name: "a shielded enemy blocks the cut, is knocked, grows nothing",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.8,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 0.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.7329924047901797, 0.7054987341316967],
            v1: [2.3006462076931715, 0.38344103461552875],
            ov1: [0.6753842248988692, 2.510748125309307],
            oboost: 0.6,
            ohp: 100.0,
            dealt: 0.0,
            blade1: 2.6,
            events: vec![
                (
                    12.5,
                    "block",
                    vec![
                        I(1),
                        I(0),
                        F(0.5986393923831788),
                        F(-0.09183635429907255),
                        S("cut".into()),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.7329924047901797),
                        F(0.7054987341316967),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
        Case {
            name: "the reach grows to its cap",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 4.17,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 0.0,
                land: 1.5,
                reach: 4.17,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [1.9795908857482156, 0.7465984809580359],
            v1: [2.300646207693172, 0.38344103461552864],
            ov1: [0.6753842248988692, 2.510748125309307],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 4.2,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(7.2),
                        F(0.5986393923831818),
                        F(-0.09183635429909032),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.9795908857482156),
                        F(0.7465984809580359),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
        Case {
            name: "a wide cut",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 33.0,
                land: 1.5,
                reach: 2.6,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.4,
                kind: 0,
                grow: true,
            },
            p1: [1.361265279407181, 1.4059371493032342],
            v1: [2.0948347407727863, 1.025508366056276],
            ov1: [2.3844790152090445, -1.0364650626089198],
            oboost: 0.6,
            ohp: 92.8,
            dealt: 7.2,
            blade1: 2.66,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(7.2),
                        F(0.23619014925431403),
                        F(1.0388917912248608),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.361265279407181),
                        F(1.4059371493032342),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
        Case {
            name: "a chain jump grows nothing",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: 35.0,
                land: 1.4,
                reach: 5.5,
                dmg: 3.2,
                knock: 0.8,
                cut: 0.25,
                kind: 1,
                grow: false,
            },
            p1: [1.499195699464218, 1.4806161094802652],
            v1: [2.0924630459438416, 1.0303389740077868],
            ov1: [2.3870526622949417, -1.030523938301217],
            oboost: 0.8,
            ohp: 93.6,
            dealt: 6.4,
            blade1: 2.6,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(6.4),
                        F(0.23494748561072037),
                        F(1.03828167776353),
                        S("cut".into()),
                        I(1),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.499195699464218),
                        F(1.4806161094802652),
                        I(1),
                        I(1),
                    ],
                ),
            ],
        },
        Case {
            name: "a chain jump the other way",
            k: 0,
            p0: [-1.0, 0.25],
            v0: [1.2, -2.0],
            op: [0.5, 0.5],
            ov: [0.3, 2.5],
            shield: 0.0,
            blade: 2.6,
            j: Jump {
                tgt: [0.5, 0.5],
                theta: -35.0,
                land: 1.4,
                reach: 5.5,
                dmg: 3.2,
                knock: 0.8,
                cut: 0.25,
                kind: 1,
                grow: false,
            },
            p1: [1.7632227782435357, -0.1035463631956397],
            v1: [2.3135209269223758, -0.2960083118634175],
            ov1: [1.24263137089182, 2.2838273306145003],
            oboost: 0.8,
            ohp: 93.6,
            dealt: 6.4,
            blade1: 2.6,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(0),
                        I(1),
                        F(6.4),
                        F(0.4238524899875018),
                        F(-0.09514834849715947),
                        S("cut".into()),
                        I(1),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(0),
                        F(-1.0),
                        F(0.25),
                        F(1.7632227782435357),
                        F(-0.1035463631956397),
                        I(1),
                        I(1),
                    ],
                ),
            ],
        },
        Case {
            name: "from the right corner",
            k: 1,
            p0: [1.5, -1.0],
            v0: [-2.0, 1.5],
            op: [-0.5, 0.2],
            ov: [2.6, 0.0],
            shield: 0.0,
            blade: 3.1,
            j: Jump {
                tgt: [-0.5, 0.2],
                theta: -8.0,
                land: 1.5,
                reach: 3.1,
                dmg: 3.6,
                knock: 0.6,
                cut: 0.0,
                kind: 0,
                grow: true,
            },
            p1: [-1.4071483424833084, 1.0599894678016621],
            v1: [-2.0398098487501795, 1.4453981392480653],
            ov1: [-2.084402635347776, -1.5541124971294107],
            oboost: 0.6,
            ohp: 98.56,
            dealt: 1.4400000000000002,
            blade1: 3.16,
            events: vec![
                (
                    12.5,
                    "hit",
                    vec![
                        I(1),
                        I(0),
                        F(1.4400000000000002),
                        F(-0.15310444658046357),
                        F(0.6895543637000425),
                        S("cut".into()),
                        I(0),
                    ],
                ),
                (
                    12.5,
                    "jump",
                    vec![
                        I(1),
                        F(1.5),
                        F(-1.0),
                        F(-1.4071483424833084),
                        F(1.0599894678016621),
                        I(1),
                        I(0),
                    ],
                ),
            ],
        },
    ]
}

/// `jump_tick`: the grid, the lock and the lead.
fn run_grid() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 0,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 1,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 2,
                tgt: Some([0.9321323529411765, -0.4908823529411765]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 3,
                tgt: Some([0.9321323529411765, -0.4908823529411765]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [2.068479091918759, 0.0904440565491168],
                v: [2.5884170151453056, -0.2451476202337415],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 5,
                tgt: None,
                p: [2.068479091918759, 0.0904440565491168],
                v: [2.5884170151453056, -0.2451476202337415],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 6,
                tgt: Some([1.3721323529411764, -0.21088235294117647]),
                p: [2.068479091918759, 0.0904440565491168],
                v: [2.5884170151453056, -0.2451476202337415],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 7,
                tgt: Some([1.3721323529411764, -0.21088235294117647]),
                p: [2.068479091918759, 0.0904440565491168],
                v: [2.5884170151453056, -0.2451476202337415],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 8,
                tgt: None,
                p: [-0.11109869130343597, 0.012781835942598713],
                v: [-2.598351059770895, -0.09258385489632957],
                blade: 2.66,
                rhap: None,
                events: 3,
            },
            Row {
                k16: 9,
                tgt: None,
                p: [-0.11109869130343597, 0.012781835942598713],
                v: [-2.598351059770895, -0.09258385489632957],
                blade: 2.66,
                rhap: None,
                events: 3,
            },
        ],
        events: vec![
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-1.2),
                    F(0.4),
                    F(2.068479091918759),
                    F(0.0904440565491168),
                    I(0),
                    I(0),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.458634495023924),
                    F(0.2596194753317451),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.068479091918759),
                    F(0.0904440565491168),
                    F(-0.11109869130343597),
                    F(0.012781835942598713),
                    I(1),
                    I(0),
                ],
            ),
        ],
        next_draw: 0.6221133620055489,
    }
}

/// `jump_tick`: haste halves the grid; the lock is at most one 16th before.
fn run_haste() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 0,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 1,
                tgt: Some([0.71, -0.8300000000000001]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 2,
                tgt: Some([0.71, -0.8300000000000001]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 3,
                tgt: Some([0.9299999999999999, -0.69]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [2.2223014652868596, -1.0257871919868167],
                v: [2.4000427606680734, -0.9998973682157453],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 5,
                tgt: Some([1.15, -0.55]),
                p: [2.2223014652868596, -1.0257871919868167],
                v: [2.4000427606680734, -0.9998973682157453],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 6,
                tgt: None,
                p: [0.30908102012918537, 0.6921172526348021],
                v: [-1.9345715999437199, 1.7370759121843797],
                blade: 2.72,
                rhap: None,
                events: 4,
            },
            Row {
                k16: 7,
                tgt: Some([1.37, -0.41]),
                p: [0.30908102012918537, 0.6921172526348021],
                v: [-1.9345715999437199, 1.7370759121843797],
                blade: 2.72,
                rhap: None,
                events: 4,
            },
            Row {
                k16: 8,
                tgt: None,
                p: [2.4, -1.161160784408662],
                v: [1.945718907890297, -1.7245805088421327],
                blade: 2.7800000000000002,
                rhap: None,
                events: 6,
            },
        ],
        events: vec![
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.2707455465113255),
                    F(-0.06614397830736762),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-1.2),
                    F(0.4),
                    F(2.2223014652868596),
                    F(-1.0257871919868167),
                    I(1),
                    I(0),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.6608636720425491),
                    F(-0.03356040001298771),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.2223014652868596),
                    F(-1.0257871919868167),
                    F(0.30908102012918537),
                    F(0.6921172526348021),
                    I(1),
                    I(0),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.082019882574865),
                    F(-0.7890120556669673),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.30908102012918537),
                    F(0.6921172526348021),
                    F(2.4),
                    F(-1.161160784408662),
                    I(1),
                    I(0),
                ],
            ),
        ],
        next_draw: 0.8002469913554168,
    }
}

/// `jump_tick`: no lock, no jitter.
fn run_no_lock() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 0,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 1,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 2,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 3,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [2.177609989095356, -1.1380188343202045],
                v: [2.36622877731834, -1.077479175386028],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 5,
                tgt: None,
                p: [2.177609989095356, -1.1380188343202045],
                v: [2.36622877731834, -1.077479175386028],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 6,
                tgt: None,
                p: [2.177609989095356, -1.1380188343202045],
                v: [2.36622877731834, -1.077479175386028],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 7,
                tgt: None,
                p: [2.177609989095356, -1.1380188343202045],
                v: [2.36622877731834, -1.077479175386028],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 8,
                tgt: None,
                p: [0.49276937774652607, 0.7893253288955402],
                v: [-1.7111997452393544, 1.9574972367522692],
                blade: 2.72,
                rhap: None,
                events: 4,
            },
        ],
        events: vec![
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(0.7913509595263682),
                    F(-1.1660527947656236),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-1.2),
                    F(0.4),
                    F(2.177609989095356),
                    F(-1.1380188343202045),
                    I(1),
                    I(0),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.9317301315580395),
                    F(0.054892248901235174),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.177609989095356),
                    F(-1.1380188343202045),
                    F(0.49276937774652607),
                    F(0.7893253288955402),
                    I(1),
                    I(0),
                ],
            ),
        ],
        next_draw: 0.6221133620055489,
    }
}

/// `jump_tick`: stunned on 16ths 2 and 3, in shadow on 8.
fn run_dazed() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 0,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 1,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 2,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 3,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 5,
                tgt: None,
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 6,
                tgt: Some([1.26, -0.48]),
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 7,
                tgt: Some([1.26, -0.48]),
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 8,
                tgt: None,
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 9,
                tgt: None,
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
        ],
        events: vec![(
            12.5,
            "jump",
            vec![
                I(0),
                F(-1.2),
                F(0.4),
                F(2.219612330948775),
                F(-0.20646070480110668),
                I(0),
                I(0),
            ],
        )],
        next_draw: 0.10031690700889628,
    }
}

/// `jump_tick`: RHAPSODY from 16th 5.
fn run_chain() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 3,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 5,
                tgt: None,
                p: [0.3036787635091325, -1.6652310812852054],
                v: [-2.0686368804156543, -1.5750369700372715],
                blade: 2.6,
                rhap: Some((4.0, -1.0, Some(7.0))),
                events: 4,
            },
            Row {
                k16: 6,
                tgt: None,
                p: [0.3036787635091325, -1.6652310812852054],
                v: [-2.0686368804156543, -1.5750369700372715],
                blade: 2.6,
                rhap: Some((4.0, -1.0, Some(7.0))),
                events: 4,
            },
            Row {
                k16: 7,
                tgt: None,
                p: [2.4, -0.0035837346802360392],
                v: [2.037544503514229, 1.615058016356983],
                blade: 2.6,
                rhap: Some((3.0, 1.0, Some(9.0))),
                events: 6,
            },
            Row {
                k16: 8,
                tgt: None,
                p: [2.4, -0.0035837346802360392],
                v: [2.037544503514229, 1.615058016356983],
                blade: 2.6,
                rhap: Some((3.0, 1.0, Some(9.0))),
                events: 6,
            },
            Row {
                k16: 9,
                tgt: None,
                p: [0.7514941274217786, -1.3911190399113895],
                v: [-1.989173433487367, -1.6742726932934424],
                blade: 2.6,
                rhap: Some((2.0, -1.0, Some(11.0))),
                events: 8,
            },
            Row {
                k16: 10,
                tgt: None,
                p: [0.7514941274217786, -1.3911190399113895],
                v: [-1.989173433487367, -1.6742726932934424],
                blade: 2.6,
                rhap: Some((2.0, -1.0, Some(11.0))),
                events: 8,
            },
            Row {
                k16: 11,
                tgt: None,
                p: [2.4, 0.5239166564624144],
                v: [1.6962336055708764, 1.9704800317009108],
                blade: 2.6,
                rhap: Some((1.0, 1.0, Some(13.0))),
                events: 10,
            },
            Row {
                k16: 12,
                tgt: None,
                p: [2.4, 0.5239166564624144],
                v: [1.6962336055708764, 1.9704800317009108],
                blade: 2.6,
                rhap: Some((1.0, 1.0, Some(13.0))),
                events: 10,
            },
            Row {
                k16: 13,
                tgt: None,
                p: [2.011615457423446, -1.389879283579213],
                v: [-0.5171015413459737, -2.5480592606800996],
                blade: 2.6,
                rhap: None,
                events: 12,
            },
            Row {
                k16: 14,
                tgt: Some([2.14, 0.08000000000000007]),
                p: [2.011615457423446, -1.389879283579213],
                v: [-0.5171015413459737, -2.5480592606800996],
                blade: 2.6,
                rhap: None,
                events: 12,
            },
            Row {
                k16: 15,
                tgt: Some([2.14, 0.08000000000000007]),
                p: [2.011615457423446, -1.389879283579213],
                v: [-0.5171015413459737, -2.5480592606800996],
                blade: 2.6,
                rhap: None,
                events: 12,
            },
            Row {
                k16: 16,
                tgt: None,
                p: [2.4, 1.468499278156453],
                v: [0.35006047127569423, 2.5763263897360202],
                blade: 2.66,
                rhap: None,
                events: 14,
            },
            Row {
                k16: 17,
                tgt: None,
                p: [2.4, 1.468499278156453],
                v: [0.35006047127569423, 2.5763263897360202],
                blade: 2.66,
                rhap: None,
                events: 14,
            },
        ],
        events: vec![
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-1.2),
                    F(0.4),
                    F(2.219612330948775),
                    F(-0.20646070480110668),
                    I(0),
                    I(0),
                ],
            ),
            (
                12.5,
                "ability",
                vec![I(0), S("RHAPSODY".into()), S("rhapsody".into()), I(1)],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(1.5134700700086012),
                    F(-1.0273777416343817),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.219612330948775),
                    F(-0.20646070480110668),
                    F(0.3036787635091325),
                    F(-1.6652310812852054),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(1.742705696082381),
                    F(-0.8802025777340527),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.3036787635091325),
                    F(-1.6652310812852054),
                    F(2.4),
                    F(-0.0035837346802360392),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(1.9763706215292556),
                    F(-0.7290400231124694),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.4),
                    F(-0.0035837346802360392),
                    F(0.7514941274217786),
                    F(-1.3911190399113895),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(2.264726161161748),
                    F(-0.5214385243625108),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.7514941274217786),
                    F(-1.3911190399113895),
                    F(2.4),
                    F(0.5239166564624144),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(2.6180136755415613),
                    F(-0.10933112492599359),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.4),
                    F(0.5239166564624144),
                    F(2.011615457423446),
                    F(-1.389879283579213),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.765463140830149),
                    F(0.3007831856790069),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.011615457423446),
                    F(-1.389879283579213),
                    F(2.4),
                    F(1.468499278156453),
                    I(1),
                    I(0),
                ],
            ),
        ],
        next_draw: 0.6221133620055489,
    }
}

/// `jump_tick`: a quiet chain of 2, 3 16ths apart, with the cut of the weapon.
fn run_chain_plain() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 3,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [2.219612330948775, -0.20646070480110668],
                v: [2.560051996590423, -0.4540195752975736],
                blade: 2.6,
                rhap: None,
                events: 1,
            },
            Row {
                k16: 5,
                tgt: None,
                p: [0.3036787635091325, -1.6652310812852054],
                v: [-2.0686368804156543, -1.5750369700372715],
                blade: 2.6,
                rhap: Some((1.0, -1.0, Some(8.0))),
                events: 3,
            },
            Row {
                k16: 6,
                tgt: None,
                p: [0.3036787635091325, -1.6652310812852054],
                v: [-2.0686368804156543, -1.5750369700372715],
                blade: 2.6,
                rhap: Some((1.0, -1.0, Some(8.0))),
                events: 3,
            },
            Row {
                k16: 7,
                tgt: None,
                p: [0.3036787635091325, -1.6652310812852054],
                v: [-2.0686368804156543, -1.5750369700372715],
                blade: 2.6,
                rhap: Some((1.0, -1.0, Some(8.0))),
                events: 3,
            },
            Row {
                k16: 8,
                tgt: None,
                p: [2.4, 0.07590966213747918],
                v: [2.0000911884300603, 1.6612149884841594],
                blade: 2.6,
                rhap: None,
                events: 5,
            },
            Row {
                k16: 9,
                tgt: None,
                p: [2.4, 0.07590966213747918],
                v: [2.0000911884300603, 1.6612149884841594],
                blade: 2.6,
                rhap: None,
                events: 5,
            },
            Row {
                k16: 10,
                tgt: Some([1.7000000000000002, -0.19999999999999996]),
                p: [2.4, 0.07590966213747918],
                v: [2.0000911884300603, 1.6612149884841594],
                blade: 2.6,
                rhap: None,
                events: 5,
            },
            Row {
                k16: 11,
                tgt: Some([1.7000000000000002, -0.19999999999999996]),
                p: [2.4, 0.07590966213747918],
                v: [2.0000911884300603, 1.6612149884841594],
                blade: 2.6,
                rhap: None,
                events: 5,
            },
            Row {
                k16: 12,
                tgt: None,
                p: [0.22493874955387216, 0.07238632020038105],
                v: [-2.5999965887849332, -0.0042116869194828875],
                blade: 2.66,
                rhap: None,
                events: 7,
            },
            Row {
                k16: 13,
                tgt: None,
                p: [0.22493874955387216, 0.07238632020038105],
                v: [-2.5999965887849332, -0.0042116869194828875],
                blade: 2.66,
                rhap: None,
                events: 7,
            },
        ],
        events: vec![
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(-1.2),
                    F(0.4),
                    F(2.219612330948775),
                    F(-0.20646070480110668),
                    I(0),
                    I(0),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(1.5134700700086012),
                    F(-1.0273777416343817),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.219612330948775),
                    F(-0.20646070480110668),
                    F(0.3036787635091325),
                    F(-1.6652310812852054),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(6.4),
                    F(1.8633573050348058),
                    F(-0.8015595050223219),
                    S("cut".into()),
                    I(1),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(0.3036787635091325),
                    F(-1.6652310812852054),
                    F(2.4),
                    F(0.07590966213747918),
                    I(1),
                    I(1),
                ],
            ),
            (
                12.5,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(7.2),
                    F(1.91902807224935),
                    F(0.5399992127965232),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(0),
                    F(2.4),
                    F(0.07590966213747918),
                    F(0.22493874955387216),
                    F(0.07238632020038105),
                    I(1),
                    I(0),
                ],
            ),
        ],
        next_draw: 0.6221133620055489,
    }
}

/// `jump_tick`: from the right corner.
fn run_right() -> Run {
    Run {
        rows: vec![
            Row {
                k16: 0,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 1,
                tgt: None,
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 2,
                tgt: Some([0.82, -0.76]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 3,
                tgt: Some([0.82, -0.76]),
                p: [-1.2, 0.4],
                v: [2.4, 1.0],
                blade: 2.6,
                rhap: None,
                events: 0,
            },
            Row {
                k16: 4,
                tgt: None,
                p: [1.6009254344335129, -1.7370777309594776],
                v: [2.067040729918734, -1.5771311362271143],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
            Row {
                k16: 5,
                tgt: None,
                p: [1.6009254344335129, -1.7370777309594776],
                v: [2.067040729918734, -1.5771311362271143],
                blade: 2.66,
                rhap: None,
                events: 2,
            },
        ],
        events: vec![
            (
                12.5,
                "hit",
                vec![
                    I(1),
                    I(0),
                    F(1.4400000000000002),
                    F(0.6760466608706661),
                    F(-1.0970093992120156),
                    S("cut".into()),
                    I(0),
                ],
            ),
            (
                12.5,
                "jump",
                vec![
                    I(1),
                    F(-1.2),
                    F(0.4),
                    F(1.6009254344335129),
                    F(-1.7370777309594776),
                    I(1),
                    I(0),
                ],
            ),
        ],
        next_draw: 0.10031690700889628,
    }
}
