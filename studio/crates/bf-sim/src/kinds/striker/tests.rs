//! The striker's unit tests. Every expected number is what `fight.py` gave on the reference PC (CPython 3.13, numpy
//! 2.5): see the note below the imports and the tests that drive the fight loop. The Python scripts that print them
//! are in `tools/studio_export/unit_checks/` (`striker_unit_vectors.py`; `striker_grid_vectors.py` for the fight loop).

use bf_core::event::{Event, Value};

use super::*;
use crate::roster::FighterCfg;
use crate::schedule::Schedule;
use crate::testing;

// Every expected number below is what `fight.py` gave on the reference PC (CPython 3.13, numpy 2.5) for the same
// hand-built state: a `Fight('FLUTE', 'TROMBONE', seed=392)` under the episode's rules with `striker_setup(0)`
// called (and `t_strike` set, as `run()` does), its baskets, balls and clock set as the test sets them, then the
// same methods called. The tests that drive the fight loop say how Python was driven.

/// The episode's beat, `60 / bpm`.
const BEAT: f64 = 0.44852941176470584;
/// `DT`.
const DT: f64 = 0.008333333333333333;
/// The time the hand-built cobras have just risen at: dropped at 10.0, one beat later.
const T_RISEN: f64 = 10.448529411764707;

use Value::{Float as F, Int as I};

fn s(x: &str) -> Value {
    Value::Str(x.into())
}

/// FLUTE (0) against TROMBONE (1) under the episode's rules, the FLUTE set up as a striker.
fn charmer(roster: &[FighterCfg]) -> Fight<'_> {
    let mut fight = testing::fight(&testing::episode_setup(roster, 392), false);
    setup(&mut fight, 0);
    fight
}

/// The striker's state, to change by hand.
fn state<'f>(fight: &'f mut Fight, k: usize) -> &'f mut Striker {
    fight.striker[k].as_mut().expect("set up as a striker")
}

/// The striker's state, to look at.
fn st<'f>(fight: &'f Fight, k: usize) -> &'f Striker {
    fight.striker[k].as_ref().expect("set up as a striker")
}

/// Puts a basket on the floor by hand, its cobra on it (or at `cp`); returns its index.
fn basket(fight: &mut Fight, k: usize, home: V2, t_drop: f64, cp: Option<V2>) -> usize {
    let st = state(fight, k);
    let id = st.next_bk;
    st.next_bk += 1;
    st.baskets.push(Basket {
        id,
        home,
        cp: cp.unwrap_or(home),
        t_drop,
        strike: None,
        down: None,
    });
    st.baskets.len() - 1
}

/// An event equals `(t, kind, fields)`: the time and every whole number and text exactly, floats to 1e-12.
#[track_caller]
fn assert_event(e: &Event, t: f64, kind: &str, fields: &[Value]) {
    assert_eq!((e.t, e.kind.as_str()), (t, kind), "{e:?}");
    assert_eq!(e.fields.len(), fields.len(), "{e:?}");
    for (got, want) in e.fields.iter().zip(fields) {
        match (got, want) {
            (Value::Float(x), Value::Float(y)) => assert!((x - y).abs() <= 1e-12, "{x} vs {y} in {e:?}"),
            _ => assert_eq!(got, want, "{e:?}"),
        }
    }
}

#[track_caller]
fn close(got: f64, want: f64) {
    assert!((got - want).abs() <= 1e-12, "{got} vs Python's {want}");
}

#[track_caller]
fn close2(got: V2, want: V2) {
    assert!(
        (got[0] - want[0]).abs() <= 1e-12 && (got[1] - want[1]).abs() <= 1e-12,
        "{got:?} vs Python's {want:?}"
    );
}

#[test]
fn setup_gives_the_striker_its_start_state() {
    let roster = testing::roster();
    let mut fight = testing::fight(&testing::episode_setup(&roster, 392), false);
    assert_eq!(fight.striker, [None, None], "Fight::new sets no kind up");
    assert!(is_striker(&fight, 0) && !is_striker(&fight, 1));
    setup(&mut fight, 0);
    assert_eq!(
        fight.striker[0],
        Some(Striker {
            baskets: vec![],
            next_bk: 0,
            dance_until: -1.0,
            t_strike: 0.12,
        })
    );
    assert_eq!(fight.striker[1], None, "the TROMBONE has no baskets");
    // the reach is the ball's blade_len: w['base']
    assert_eq!(fight.balls[0].blade_len, 1.4);
    // the fight loop sets up whoever is a striker, on either side
    let ep = testing::episode_setup(&roster, 1);
    let swapped = crate::Setup {
        fighters: [ep.fighters[1], ep.fighters[0]],
        ..ep
    };
    let mut fight = testing::fight(&swapped, false);
    Schedule::new(&mut fight);
    assert!(fight.striker[0].is_none() && fight.striker[1].is_some());
    let bare = testing::bare_pair();
    let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
    Schedule::new(&mut fight);
    assert_eq!(fight.striker, [None, None]);
}

/// Brief: a basket drops every `drop_every = 4` beats and the oldest is taken back beyond `max_baskets = 2`
/// (the `basket` event carries the removed id, else -1).
#[test]
fn a_basket_drops_every_4_beats_and_the_oldest_is_taken_back() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let mut ids = Vec::new();
    for kb in 0..14 {
        fight.t = 3.5 + kb as f64 * 0.5;
        fight.balls[0].p = [-1.85 + 0.25 * kb as f64, 1.85 - 0.125 * kb as f64];
        beat(&mut fight, 0, kb);
        let striker = st(&fight, 0);
        ids.push((
            striker.baskets.iter().map(|b| b.id).collect::<Vec<_>>(),
            striker.next_bk,
        ));
    }
    let want: [(&[i64], i64); 14] = [
        (&[0], 1),
        (&[0], 1),
        (&[0], 1),
        (&[0], 1),
        (&[0, 1], 2),
        (&[0, 1], 2),
        (&[0, 1], 2),
        (&[0, 1], 2),
        (&[1, 2], 3),
        (&[1, 2], 3),
        (&[1, 2], 3),
        (&[1, 2], 3),
        (&[2, 3], 4),
        (&[2, 3], 4),
    ];
    for (kb, (got, want)) in ids.iter().zip(want).enumerate() {
        assert_eq!((got.0.as_slice(), got.1), want, "after beat {kb}");
    }
    // the event: the new id, where the fighter stood, and the id taken back (-1 for none)
    assert_eq!(fight.events.len(), 4);
    assert_event(&fight.events[0], 3.5, "basket", &[I(0), I(0), F(-1.85), F(1.85), I(-1)]);
    assert_event(
        &fight.events[1],
        5.5,
        "basket",
        &[I(0), I(1), F(-0.8500000000000001), F(1.35), I(-1)],
    );
    assert_event(
        &fight.events[2],
        7.5,
        "basket",
        &[I(0), I(2), F(0.1499999999999999), F(0.8500000000000001), I(0)],
    );
    assert_event(
        &fight.events[3],
        9.5,
        "basket",
        &[I(0), I(3), F(1.15), F(0.3500000000000001), I(1)],
    );
    // the baskets left: where the fighter stood, the cobra on its basket, dropped at the step's time, at rest
    let striker = st(&fight, 0);
    assert_eq!(
        striker.baskets,
        vec![
            Basket {
                id: 2,
                home: [0.1499999999999999, 0.8500000000000001],
                cp: [0.1499999999999999, 0.8500000000000001],
                t_drop: 7.5,
                strike: None,
                down: None,
            },
            Basket {
                id: 3,
                home: [1.15, 0.3500000000000001],
                cp: [1.15, 0.3500000000000001],
                t_drop: 9.5,
                strike: None,
                down: None,
            },
        ]
    );
}

/// In the fight loop the drops fall on the beat grid: beats 0, 4, 8 and 12 of the fight, at the time of the
/// step that reached the beat, where the FLUTE is before that step moves it. The Python recording of the same
/// seed has its first four `basket` events at those times, with the same ids and the same baskets taken back.
#[test]
fn baskets_drop_on_the_beat_grid() {
    let roster = testing::roster();
    let mut fight = testing::fight(&testing::episode_setup(&roster, 392), false);
    let mut sched = Schedule::new(&mut fight);
    let py = testing::python_recording("ep392.bfrec");
    let mut py_drops = py.events.iter().filter(|e| e.kind == "basket");
    let mut drops = Vec::new();
    while sched.kb < 12 {
        let before = fight.balls[0].p;
        let n = fight.events.len();
        sched.step(&mut fight);
        let made: Vec<&Event> = fight.events[n..].iter().filter(|e| e.kind == "basket").collect();
        assert_eq!(
            made.len(),
            usize::from(sched.on_beat && sched.kb % 4 == 0),
            "step {}",
            sched.step
        );
        if let Some(&e) = made.first() {
            assert_eq!(e.t, bf_core::py::round_nd(fight.t, 5));
            let taken = if sched.kb >= 8 { sched.kb / 4 - 2 } else { -1 };
            assert_eq!(
                e.fields,
                vec![I(0), I(sched.kb / 4), F(before[0]), F(before[1]), I(taken)]
            );
            // Python's: the same time, id and basket taken back (where it stood depends on the lance by then)
            let p = py_drops.next().expect("Python dropped one too");
            assert_eq!((p.t, p.i(3), p.i(6)), (e.t, e.i(3), e.i(6)), "{p:?}");
            drops.push((sched.step, sched.kb));
        }
    }
    assert_eq!(drops, vec![(1, 0), (216, 4), (431, 8), (646, 12)]);
    assert_eq!(st(&fight, 0).baskets.len(), 2);
}

/// Brief: a cobra cannot strike before `rise = 1.0` beat after its drop. (The test is `t < t_drop + rise * beat -
/// 1e-9`, so 0.5 ns early still counts as risen and 2 ns early does not.)
#[test]
fn a_cobra_cannot_strike_before_it_has_risen() {
    let roster = testing::roster();
    for (t_off, risen) in [
        (-BEAT, false),
        (-0.2, false),
        (-2e-9, false),
        (-5e-10, true),
        (0.0, true),
    ] {
        let mut fight = charmer(&roster);
        let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
        fight.balls[1].p = [1.9, 0.6];
        fight.t = 10.0 + BEAT + t_off;
        fire(&mut fight, 0);
        let strike = st(&fight, 0).baskets[b].strike.clone();
        assert_eq!(strike.is_some(), risen, "{t_off} s from one beat after the drop");
        assert_eq!(fight.events.len(), usize::from(risen));
    }
    // Python's numbers for the strike 0.5 ns inside the slack: t = 10.448529411264706
    let mut fight = charmer(&roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    fight.balls[1].p = [1.9, 0.6];
    fight.t = 10.0 + BEAT - 5e-10;
    assert_eq!(fight.t, 10.448529411264706);
    fire(&mut fight, 0);
    assert_event(
        &fight.events[0],
        10.44853,
        "strike",
        &[I(0), I(0), F(0.5456550545269917), F(1.27783393541592), F(0.12)],
    );
    let strike = st(&fight, 0).baskets[b].strike.clone().expect("it lunges");
    assert_eq!(
        (strike.t0, strike.t_hit, strike.t_end, strike.done),
        (10.448529411264706, 10.568529411264706, 10.760529411264706, false)
    );
}

/// The fight loop with the FLUTE held in its corner and a bag held 1.485 away, in reach of every basket: both
/// are put back before every step, and from step `haste_from` on the FLUTE is kept hasted (x 2). Returns the
/// lunges as (step, 16th of the fight, basket, seconds to the bite) and the strikers' 16th counter at the end.
///
/// The Python numbers the two tests below expect come from `Fight('FLUTE', 'TROMBONE', seed=392).run()` under
/// the episode's rules, driven the same way: the two balls put back (and the haste set) at the end of every
/// `abilities_step`, the lance kept idle by replacing `start_thrust` and `blade_step` on that one fight.
fn held(steps: i64, haste_from: Option<i64>) -> (Vec<(i64, i64, i64, f64)>, i64) {
    let roster = testing::roster();
    let bare = testing::bare_pair();
    let setup = crate::Setup {
        fighters: [testing::fighter(&roster, "FLUTE"), &bare[1]],
        rules: testing::episode_rules(),
        seed: 392,
    };
    let mut fight = testing::fight(&setup, false);
    let mut sched = Schedule::new(&mut fight);
    assert_eq!(sched.next16k, sched.k16_0);
    let (mut lunges, mut sixteenths) = (Vec::new(), 0);
    while sched.step < steps {
        fight.balls[0].p = [-1.85, 1.85];
        fight.balls[1].p = [-0.8, 0.8];
        if haste_from.is_some_and(|from| sched.step >= from) {
            fight.balls[0].haste = 1.0;
            fight.balls[0].haste_mult = 2.0;
        }
        let (n, n16) = (fight.events.len(), sched.next16k);
        sched.step(&mut fight);
        sixteenths += sched.next16k - n16;
        for e in &fight.events[n..] {
            if e.kind == "strike" {
                lunges.push((sched.step, sixteenths - 1, e.i(3), e.f(6)));
            }
        }
    }
    (lunges, sched.next16k - sched.k16_0)
}

/// In the fight loop the lunges start on the strike grid: every beat (`every = 1` is every 4th 16th; a lunge is
/// over well inside a beat), counted on the strikers' own 16th counter. A basket is dropped at the time of the
/// step that reached its beat, and that step is up to one step late against the beat itself; so whether a cobra
/// has risen one beat later depends on how late the next beat's step is. The first basket's step is a whole
/// step late (the fight's first step), so its cobra has not risen on beat 1 and first lunges on beat 2; the
/// basket of beat 4 has risen by beat 5. Each landed bite takes 0.003 s off the next lunge.
#[test]
fn lunges_start_on_the_beat_once_the_cobra_has_risen() {
    let (lunges, sixteenths) = held(300, None);
    // beats fall on steps 1, 54, 108, 162, 216 and 270: 16ths 0, 4, 8, ...
    assert_eq!(
        lunges,
        vec![
            (108, 8, 0, 0.12),
            (162, 12, 0, 0.11699999999999999),
            (216, 16, 0, 0.11399999999999999),
            (270, 20, 0, 0.11099999999999999),
            (270, 20, 1, 0.11099999999999999),
        ]
    );
    assert_eq!(sixteenths, 23, "the strikers' own counter: 23 sixteenths in 300 steps");
}

/// With haste the strike grid is divided: `every = 1` beat is 4 sixteenths, FLUTTER TONGUE's `mult = 2.0` makes
/// it 2 (every 8th note), and the lunge itself takes half as long.
#[test]
fn haste_divides_the_strike_grid() {
    let (lunges, _) = held(330, Some(150));
    assert_eq!(
        lunges,
        vec![
            // beat 2, before the haste; 16th 10 then passes without a lunge
            (108, 8, 0, 0.12),
            (162, 12, 0, 0.058499999999999996),
            (189, 14, 0, 0.056999999999999995),
            (216, 16, 0, 0.055499999999999994),
            (243, 18, 0, 0.05399999999999999),
            (270, 20, 0, 0.05249999999999999),
            (270, 20, 1, 0.05249999999999999),
            (297, 22, 0, 0.04949999999999999),
            (297, 22, 1, 0.04949999999999999),
            (323, 24, 0, 0.046499999999999986),
            (323, 24, 1, 0.046499999999999986),
        ]
    );
}

/// `striker_fire` against Python: who lunges, at what angle, how far, how fast.
#[test]
fn fire_matches_python() {
    let roster = testing::roster();
    // a change to the fight (and the basket of that index) before the call
    type Pre = fn(&mut Fight, usize);
    // (the enemy, the cobra, the change) -> the strike event's (aim, length, seconds), if any
    type Case = (&'static str, V2, V2, Pre, Option<[f64; 3]>);
    let none: Pre = |_, _| {};
    let cases: [Case; 8] = [
        (
            "in reach",
            [1.9, 0.6],
            [0.5, -0.25],
            none,
            Some([0.5456550545269917, 1.27783393541592, 0.12]),
        ),
        ("far", [-2.3, 2.3], [2.0, -2.0], none, None),
        (
            "haste halves the time",
            [1.9, 0.6],
            [0.5, -0.25],
            |f, _| {
                f.balls[0].haste = 1.0;
                f.balls[0].haste_mult = 2.0;
            },
            Some([0.5456550545269917, 1.27783393541592, 0.06]),
        ),
        (
            "near: the lunge is at least 0.2 long",
            [0.9, 0.05],
            [0.5, -0.25],
            none,
            Some([0.6435011087932844, 0.2, 0.12]),
        ),
        (
            "swatted flat",
            [1.9, 0.6],
            [0.5, -0.25],
            |f, b| state(f, 0).baskets[b].down = Some(T_RISEN + 0.25),
            None,
        ),
        (
            "up again",
            [1.9, 0.6],
            [0.5, -0.25],
            |f, b| state(f, 0).baskets[b].down = Some(T_RISEN),
            Some([0.5456550545269917, 1.27783393541592, 0.12]),
        ),
        (
            "grown: the far end of the reach, the lunge capped at blade_len + 0.4 R",
            [2.9, 1.9],
            [0.5, -0.25],
            |f, _| {
                f.balls[0].blade_len = 2.4;
                state(f, 0).t_strike = 0.07;
            },
            Some([0.7305083008032945, 2.6399999999999997, 0.07]),
        ),
        (
            "the same enemy is out of the starting reach",
            [2.9, 1.9],
            [0.5, -0.25],
            none,
            None,
        ),
    ];
    for (name, enemy, cobra, pre, want) in cases {
        let mut fight = charmer(&roster);
        let b = basket(&mut fight, 0, cobra, 10.0, None);
        fight.balls[1].p = enemy;
        fight.t = 10.0 + BEAT;
        assert_eq!(fight.t, T_RISEN);
        pre(&mut fight, b);
        fire(&mut fight, 0);
        let strike = st(&fight, 0).baskets[b].strike.clone();
        match want {
            None => {
                assert!(fight.events.is_empty() && strike.is_none(), "{name}");
            }
            Some([aim, len, tk]) => {
                assert_eq!(fight.events.len(), 1, "{name}");
                assert_event(
                    &fight.events[0],
                    10.44853,
                    "strike",
                    &[I(0), I(0), F(aim), F(len), F(tk)],
                );
                let s = strike.unwrap_or_else(|| panic!("{name}: it lunges"));
                close(s.aim, aim);
                close(s.len, len);
                // t_hit = t + tk, t_end = t + tk * 2.6
                assert_eq!(
                    (s.t0, s.t_hit, s.t_end, s.done),
                    (T_RISEN, T_RISEN + tk, T_RISEN + tk * 2.6, false)
                );
            }
        }
    }
    // Python's own t_hit and t_end for the plain, the hasted and the grown lunge
    assert_eq!(
        (T_RISEN + 0.12, T_RISEN + 0.12 * 2.6),
        (10.568529411764706, 10.760529411764706)
    );
    assert_eq!(
        (T_RISEN + 0.06, T_RISEN + 0.06 * 2.6),
        (10.508529411764707, 10.604529411764707)
    );
    assert_eq!(
        (T_RISEN + 0.07, T_RISEN + 0.07 * 2.6),
        (10.518529411764707, 10.630529411764707)
    );
}

/// The reach test is `dist - R > blade_len + slack`: the enemy's surface exactly `blade_len + slack` from the
/// cobra is still in reach, the next float out is not.
#[test]
fn the_reach_ends_at_blade_len_plus_slack_from_the_surface() {
    let roster = testing::roster();
    for (x, struck) in [
        (2.249999999, true),
        (2.25, true),
        (2.2500000000000004, false),
        (2.250000001, false),
    ] {
        let mut fight = charmer(&roster);
        let b = basket(&mut fight, 0, [0.0, 0.0], 10.0, None);
        fight.balls[1].p = [x, 0.0];
        fight.t = T_RISEN;
        fire(&mut fight, 0);
        assert_eq!(st(&fight, 0).baskets[b].strike.is_some(), struck, "enemy at x = {x}");
        if struck {
            // straight along x, the full lunge: blade_len + 0.4 R
            assert_event(
                &fight.events[0],
                10.44853,
                "strike",
                &[I(0), I(0), F(0.0), F(1.64), F(0.12)],
            );
        }
    }
}

/// Each cobra decides for itself, in basket order; one that is lunging (even with its bite done) is passed over.
#[test]
fn every_free_cobra_in_reach_lunges() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let b0 = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    let b1 = basket(&mut fight, 0, [-0.75, 1.0], 10.2, None);
    let b2 = basket(&mut fight, 0, [1.0, 1.0], 10.3, None);
    fight.balls[1].p = [0.4, 0.9];
    fight.t = 10.3 + BEAT;
    assert_eq!(fight.t, 10.748529411764707);
    let busy = Strike {
        t0: fight.t - 0.2,
        t_hit: fight.t - 0.08,
        t_end: fight.t + 0.112,
        aim: 0.5,
        len: 1.0,
        done: true,
    };
    state(&mut fight, 0).baskets[b1].strike = Some(busy.clone());
    fire(&mut fight, 0);
    assert_eq!(fight.events.len(), 2);
    assert_event(
        &fight.events[0],
        10.74853,
        "strike",
        &[I(0), I(0), F(1.6575346654708818), F(0.7943396380615194), F(0.12)],
    );
    assert_event(
        &fight.events[1],
        10.74853,
        "strike",
        &[I(0), I(2), F(-2.9764439761751667), F(0.24827625302982193), F(0.12)],
    );
    let striker = st(&fight, 0);
    assert_eq!(striker.baskets[b1].strike, Some(busy));
    assert!(striker.baskets[b0].strike.is_some() && striker.baskets[b2].strike.is_some());
}

/// What one hand-driven lunge did: `(step, done before, done after, events made)` for every step of `step`
/// that changed the lunge or made an event (`None` = no lunge), as the Python run logged it.
type Log = Vec<(usize, Option<bool>, Option<bool>, usize)>;

/// A cobra at (0.5, -0.25) lunges at an enemy at (1.9, 0.6) flying at (1.0, -2.4); then `striker_step` runs
/// once per step for 45 steps, `mover` moving the enemy by hand before step 5.
fn lunge(roster: &[FighterCfg], pre: fn(&mut Fight), enemy_at_5: Option<V2>) -> (Fight<'_>, Log) {
    let mut fight = charmer(roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    fight.balls[1].p = [1.9, 0.6];
    fight.balls[1].v = [1.0, -2.4];
    fight.t = T_RISEN;
    pre(&mut fight);
    fire(&mut fight, 0);
    let mut log = Log::new();
    for i in 1..=45 {
        fight.t = T_RISEN + i as f64 * DT;
        if let (5, Some(p)) = (i, enemy_at_5) {
            fight.balls[1].p = p;
        }
        let n = fight.events.len();
        let was = st(&fight, 0).baskets[b].strike.as_ref().map(|s| s.done);
        step(&mut fight, 0, DT);
        let now = st(&fight, 0).baskets[b].strike.as_ref().map(|s| s.done);
        if was != now || fight.events.len() != n {
            log.push((i, was, now, fight.events.len() - n));
        }
    }
    (fight, log)
}

/// `striker_step` against Python: the bite is tested once, on the first step at or after `t_hit` (step 15 of a
/// 0.12 s lunge), and the lunge is cleared at `t_end` (step 38).
#[test]
fn a_lunge_bites_when_it_is_out_and_ends_later() {
    let roster = testing::roster();
    let strike = [I(0), I(0), F(0.5456550545269917), F(1.27783393541592), F(0.12)];
    let bitten: Log = vec![(15, Some(false), Some(true), 1), (38, Some(true), None, 0)];
    let missed: Log = vec![(15, Some(false), Some(true), 0), (38, Some(true), None, 0)];

    // it lands: damage (1.08 x the FLUTE's handicap 2.0), the contact point on the enemy's surface, the knock
    // along the lunge, the reach and the speed grown
    let (fight, log) = lunge(&roster, |_| {}, None);
    assert_eq!(log, bitten);
    assert_eq!(fight.events.len(), 2);
    assert_event(&fight.events[0], 10.44853, "strike", &strike);
    assert_event(
        &fight.events[1],
        10.57353,
        "hit",
        &[
            I(0),
            I(1),
            F(2.16),
            F(1.3871274908669626),
            F(0.28861311945494134),
            s("fang"),
            I(0),
        ],
    );
    let (flute, enemy) = (&fight.balls[0], &fight.balls[1]);
    assert_eq!((enemy.hp, flute.dealt, enemy.boost), (97.84, 2.16, 0.5));
    close2(enemy.v, [2.4882308748541426, 0.7541267223786599]);
    assert_eq!(flute.blade_len, 1.47);
    assert_eq!(st(&fight, 0).t_strike, 0.11699999999999999);
    assert_eq!(st(&fight, 0).baskets[0].cp, [0.5, -0.25], "a cobra at home stays");

    // the enemy is gone when the lunge is out: a miss, nothing changes
    let (fight, log) = lunge(&roster, |_| {}, Some([2.3, 2.3]));
    assert_eq!(log, missed);
    assert_eq!(fight.events.len(), 1);
    let (flute, enemy) = (&fight.balls[0], &fight.balls[1]);
    assert_eq!(
        (enemy.hp, flute.dealt, enemy.boost, enemy.v),
        (100.0, 0.0, 0.0, [1.0, -2.4])
    );
    assert_eq!((flute.blade_len, st(&fight, 0).t_strike), (1.4, 0.12));

    // a shield blocks it: a `block` event at the contact point, no damage, no knock, no growth
    let (fight, log) = lunge(&roster, |f| f.balls[1].shield = 0.5, None);
    assert_eq!(log, bitten);
    assert_event(
        &fight.events[1],
        10.57353,
        "block",
        &[I(1), I(0), F(1.3871274908669626), F(0.28861311945494134), s("fang")],
    );
    let (flute, enemy) = (&fight.balls[0], &fight.balls[1]);
    assert_eq!(
        (enemy.hp, flute.dealt, enemy.boost, enemy.v),
        (100.0, 0.0, 0.0, [1.0, -2.4])
    );
    assert_eq!((flute.blade_len, st(&fight, 0).t_strike), (1.4, 0.12));

    // with haste the lunge takes half as long: out on step 8, over on step 19
    let (fight, log) = lunge(
        &roster,
        |f| {
            f.balls[0].haste = 1.0;
            f.balls[0].haste_mult = 2.0;
        },
        None,
    );
    assert_eq!(log, vec![(8, Some(false), Some(true), 1), (19, Some(true), None, 0)]);
    assert_event(
        &fight.events[1],
        10.5152,
        "hit",
        &[
            I(0),
            I(1),
            F(2.16),
            F(1.3871274908669626),
            F(0.28861311945494134),
            s("fang"),
            I(0),
        ],
    );

    // the fang line is a capsule of radius head_r = 0.14 around the lunge, from the cobra to the lunge's end:
    // an enemy beside its end is bitten at 0.733 from it and missed at 0.753 (R + head_r = 0.74)
    let (fight, log) = lunge(&roster, |_| {}, Some([1.5, 1.14]));
    assert_eq!(log, bitten);
    assert_event(
        &fight.events[1],
        10.57353,
        "hit",
        &[
            I(0),
            I(1),
            F(2.16),
            F(1.575567686653193),
            F(0.5447777518154375),
            s("fang"),
            I(0),
        ],
    );
    let (fight, log) = lunge(&roster, |_| {}, Some([1.5, 1.16]));
    assert_eq!(log, missed);
    assert_eq!(fight.events.len(), 1);
    // an enemy behind the cobra is bitten from the cobra's own spot
    let (fight, log) = lunge(&roster, |_| {}, Some([0.1, -0.7]));
    assert_eq!(log, bitten);
    assert_event(
        &fight.events[1],
        10.57353,
        "hit",
        &[
            I(0),
            I(1),
            F(2.16),
            F(0.49861830329795187),
            F(-0.25155440878980423),
            s("fang"),
            I(0),
        ],
    );
    // the knock is along the lunge wherever the enemy is
    close2(fight.balls[1].v, [2.4882308748541426, 0.7541267223786599]);
}

/// Brief: each landed bite grows the reach by `grow = 0.07` up to `max_reach = 2.4` and shortens the strike by
/// `speed_grow = 0.003` down to `t_min = 0.07`. The sequences are Python's, bit for bit.
#[test]
fn each_landed_bite_grows_the_reach_and_shortens_the_strike() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    fight.t = 20.0;
    let (mut reach, mut speed) = (Vec::new(), Vec::new());
    for _ in 0..20 {
        fight.balls[1].p = [1.2, 0.1];
        state(&mut fight, 0).baskets[b].strike = Some(Strike {
            t0: fight.t - 0.1,
            t_hit: fight.t,
            t_end: fight.t + 0.2,
            aim: 0.4,
            len: 1.0,
            done: false,
        });
        step(&mut fight, 0, DT);
        reach.push(fight.balls[0].blade_len);
        speed.push(st(&fight, 0).t_strike);
    }
    assert_eq!(
        reach,
        [
            1.47,
            1.54,
            1.61,
            1.6800000000000002,
            1.7500000000000002,
            1.8200000000000003,
            1.8900000000000003,
            1.9600000000000004,
            2.0300000000000002,
            2.1,
            2.17,
            2.2399999999999998,
            2.3099999999999996,
            2.3799999999999994,
            2.4,
            2.4,
            2.4,
            2.4,
            2.4,
            2.4
        ]
    );
    assert_eq!(
        speed,
        [
            0.11699999999999999,
            0.11399999999999999,
            0.11099999999999999,
            0.10799999999999998,
            0.10499999999999998,
            0.10199999999999998,
            0.09899999999999998,
            0.09599999999999997,
            0.09299999999999997,
            0.08999999999999997,
            0.08699999999999997,
            0.08399999999999996,
            0.08099999999999996,
            0.07799999999999996,
            0.07499999999999996,
            0.07199999999999995,
            0.07,
            0.07,
            0.07,
            0.07
        ]
    );
    assert_eq!(fight.events.iter().filter(|e| e.kind == "hit").count(), 20);
    assert_eq!(
        (fight.balls[1].hp, fight.balls[0].dealt),
        (56.80000000000007, 43.19999999999999)
    );
    // a bite that does not land grows nothing
    let before = (fight.balls[0].blade_len, st(&fight, 0).t_strike);
    fight.balls[1].p = [-2.0, -2.0];
    state(&mut fight, 0).baskets[b].strike = Some(Strike {
        t0: fight.t - 0.1,
        t_hit: fight.t,
        t_end: fight.t + 0.2,
        aim: 0.4,
        len: 1.0,
        done: false,
    });
    step(&mut fight, 0, DT);
    assert_eq!((fight.balls[0].blade_len, st(&fight, 0).t_strike), before);
    assert_eq!(fight.events.len(), 20);
}

/// A cobra off its basket slithers home at `slither * 1.6` units a second and stops exactly there.
#[test]
fn a_cobra_slithers_home() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, Some([0.56, -0.2]));
    fight.balls[1].p = [2.0, 2.0];
    let want = [
        [0.5374655091318103, -0.21877874239015807],
        [0.5149310182636206, -0.23755748478031613],
        [0.5, -0.25],
        [0.5, -0.25],
        [0.5, -0.25],
    ];
    for (i, want) in want.into_iter().enumerate() {
        fight.t = 12.0 + (i + 1) as f64 * DT;
        step(&mut fight, 0, DT);
        close2(st(&fight, 0).baskets[b].cp, want);
    }
    assert_eq!(st(&fight, 0).baskets[b].home, [0.5, -0.25]);
    assert!(fight.events.is_empty());
}

/// SNAKE DANCE: while `t < dance_until` the cobra slithers after the enemy at `slither` units a second and
/// stops `2 R + 0.1` from its centre; when the dance is over it slithers home.
#[test]
fn during_the_dance_a_cobra_chases_the_enemy() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    fight.balls[1].p = [1.9, 0.6];
    // the dance covers steps 1 to 20
    state(&mut fight, 0).dance_until = 12.0 + 20.5 * DT;
    let want: [(usize, V2); 10] = [
        (1, [0.5156711044457317, -0.24048540087223433]),
        (2, [0.5313422088914634, -0.23097080174446866]),
        (18, [0.782079880023171, -0.07873721570021791]),
        // arrived: 1.3 from the enemy's centre
        (19, [0.7887762302117524, -0.07467157451429349]),
        (20, [0.7887762302117524, -0.07467157451429349]),
        // the dance is over: home, 1.6 times as fast
        (21, [0.7637024630985816, -0.08989493311871855]),
        (22, [0.7386286959854108, -0.1051182917231436]),
        (23, [0.71355492887224, -0.12034165032756866]),
        (24, [0.6884811617590693, -0.13556500893199375]),
        (30, [0.5380385590800447, -0.22690516055854426]),
    ];
    let mut compared = 0;
    for i in 1..=30 {
        fight.t = 12.0 + i as f64 * DT;
        if i == 24 {
            // where the enemy is no longer matters
            fight.balls[1].p = [-1.0, 1.0];
        }
        step(&mut fight, 0, DT);
        let cp = st(&fight, 0).baskets[b].cp;
        if i == 20 {
            let d = crate::geom::hypot(crate::geom::sub([1.9, 0.6], cp));
            assert!((d - 1.3).abs() <= 1e-12, "{d}");
        }
        if let Some((_, w)) = want.iter().find(|(n, _)| *n == i) {
            close2(cp, *w);
            compared += 1;
        }
    }
    assert_eq!(compared, want.len(), "every Python sample was compared");
    assert_eq!(
        st(&fight, 0).baskets[b].home,
        [0.5, -0.25],
        "the basket itself never moves"
    );
}

/// A lunging cobra stays where it is, dancing or not.
#[test]
fn a_lunging_cobra_does_not_slither() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, Some([0.8, 0.1]));
    fight.balls[1].p = [2.3, 2.3];
    fight.t = 12.0;
    state(&mut fight, 0).baskets[b].strike = Some(Strike {
        t0: 11.95,
        t_hit: 12.07,
        t_end: 12.262,
        aim: 0.5,
        len: 1.0,
        done: false,
    });
    step(&mut fight, 0, DT);
    assert_eq!(st(&fight, 0).baskets[b].cp, [0.8, 0.1]);
    state(&mut fight, 0).dance_until = 13.0;
    step(&mut fight, 0, DT);
    assert_eq!(st(&fight, 0).baskets[b].cp, [0.8, 0.1]);
    assert!(st(&fight, 0).baskets[b].strike.is_some());
}

// The swat rule. The lance points along D (the angle 0.7); its base is one unit behind the cobra at (0.5, -0.25)
// and the given distance to its left, so the lance passes the cobra at that distance.

/// `[cos(0.7), sin(0.7)]`, as Python had it.
const D: V2 = [0.7648421872844885, 0.644217687237691];
/// The lance's base when it passes 0.1 beside the cobra.
const A_01: V2 = [-0.3292639560082576, -0.8177334685092421];
/// ... 0.2 beside it.
const A_02: V2 = [-0.3936857247320267, -0.7412492497807933];
/// ... 0.5 beside it, on the side a lunge aimed at 2.2 goes.
const A_05: V2 = [-0.586951030903334, -0.5117965935954467];
/// ... 1.2 beside it.
const A_12: V2 = [-1.0379034119697177, 0.023592937503695177];
/// When a cobra swatted at `T_RISEN` may lunge again: two beats later.
const T_UP: f64 = 11.345588235294118;

fn in_the_air(t0: f64, t_hit: f64, t_end: f64, done: bool) -> Option<Strike> {
    Some(Strike {
        t0,
        t_hit,
        t_end,
        aim: 2.2,
        len: 1.5,
        done,
    })
}

/// One cobra risen at `T_RISEN`, one call of `swat_cobras` by the TROMBONE's lance.
fn swat(roster: &[FighterCfg], a: V2, l: f64, t_off: f64, pre: fn(&mut Basket)) -> Fight<'_> {
    let mut fight = charmer(roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    fight.t = 10.0 + BEAT + t_off;
    pre(&mut state(&mut fight, 0).baskets[b]);
    swat_cobras(&mut fight, 1, a, D, l);
    fight
}

/// `swat_cobras` against Python: a lance through a risen cobra's body knocks it flat for two beats.
#[test]
fn a_lance_through_a_risen_cobra_swats_it() {
    let roster = testing::roster();
    let body = [I(1), I(0), I(0), F(0.5), F(-0.25)];
    // the lance passes 0.1 beside the cobra (head_r + 0.05 = 0.19 is the touch distance)
    let fight = swat(&roster, A_01, 1.5, 0.0, |_| {});
    assert_eq!(fight.events.len(), 1);
    assert_event(&fight.events[0], 10.44853, "swat", &body);
    let bk = st(&fight, 0).baskets[0].clone();
    assert_eq!((bk.down, bk.strike), (Some(T_UP), None));
    assert_eq!(T_RISEN + SWAT_BEATS * BEAT, T_UP);
    // nothing else changes: no damage, no growth
    assert_eq!(
        (fight.balls[0].hp, fight.balls[1].dealt, fight.balls[0].blade_len),
        (100.0, 0.0, 1.4)
    );

    // 0.2 beside: no touch
    let fight = swat(&roster, A_02, 1.5, 0.0, |_| {});
    assert!(fight.events.is_empty());
    assert_eq!(st(&fight, 0).baskets[0].down, None);
    // a lance that ends 0.25 short of the cobra does not reach it; one 0.15 short touches it with its tip
    assert!(swat(&roster, A_01, 0.75, 0.0, |_| {}).events.is_empty());
    let fight = swat(&roster, A_01, 0.85, 0.0, |_| {});
    assert_event(&fight.events[0], 10.44853, "swat", &body);

    // a cobra still rising out of its basket cannot be swatted (the same 1e-9 slack as for its lunge)
    assert!(swat(&roster, A_01, 1.5, -2e-9, |_| {}).events.is_empty());
    let fight = swat(&roster, A_01, 1.5, -5e-10, |_| {});
    assert_event(&fight.events[0], 10.44853, "swat", &body);
    assert_eq!(st(&fight, 0).baskets[0].down, Some(11.345588234794118));

    // one already flat is left alone; once it is up it can be swatted again
    let fight = swat(&roster, A_01, 1.5, 0.0, |b| b.down = Some(T_RISEN + 0.3));
    assert!(fight.events.is_empty());
    assert_eq!(st(&fight, 0).baskets[0].down, Some(10.748529411764707));
    let fight = swat(&roster, A_01, 1.5, 0.0, |b| b.down = Some(T_RISEN));
    assert_event(&fight.events[0], 10.44853, "swat", &body);
    assert_eq!(st(&fight, 0).baskets[0].down, Some(T_UP));
}

/// A lunge still on its way out is a segment from the cobra to its head; a lance through it swats the cobra
/// where they cross, and the lunge misses (`done`). A lunge that has bitten is only the body again.
#[test]
fn a_lance_through_a_lunge_in_the_air_makes_it_miss() {
    let roster = testing::roster();
    // half way out (0.75 of 1.5), the lance crossing 0.5 from the cobra
    let fight = swat(&roster, A_05, 2.5, 0.0, |b| {
        b.strike = in_the_air(T_RISEN - 0.06, T_RISEN + 0.06, T_RISEN + 0.252, false);
    });
    assert_eq!(fight.events.len(), 1);
    assert_event(
        &fight.events[0],
        10.44853,
        "swat",
        &[I(1), I(0), I(0), F(0.20501048869484412), F(0.15526339213598223)],
    );
    let bk = st(&fight, 0).baskets[0].clone();
    assert_eq!(bk.down, Some(T_UP));
    let strike = bk.strike.expect("the lunge runs on to its end");
    assert!(strike.done, "but it will not bite");
    assert_eq!(
        (strike.t0, strike.t_hit, strike.t_end),
        (T_RISEN - 0.06, T_RISEN + 0.06, T_RISEN + 0.252)
    );

    // the lance beyond the head: no touch, the lunge goes on
    let fight = swat(&roster, A_12, 2.5, 0.0, |b| {
        b.strike = in_the_air(T_RISEN - 0.06, T_RISEN + 0.06, T_RISEN + 0.252, false);
    });
    assert!(fight.events.is_empty());
    let bk = st(&fight, 0).baskets[0].clone();
    assert_eq!((bk.down, bk.strike.map(|s| s.done)), (None, Some(false)));

    // a quarter out (0.375), the lance 0.5 away: the head is 0.125 from it, a touch at the head
    let fight = swat(&roster, A_05, 2.5, 0.0, |b| {
        b.strike = in_the_air(T_RISEN - 0.03, T_RISEN + 0.09, T_RISEN + 0.282, false);
    });
    assert_event(
        &fight.events[0],
        10.44853,
        "swat",
        &[I(1), I(0), I(0), F(0.27931208102924854), F(0.05318615143234179)],
    );
    assert_eq!(st(&fight, 0).baskets[0].strike.as_ref().map(|s| s.done), Some(true));

    // a lunge whose bite is done is the body only: the lance 0.5 away misses it, the one 0.1 away swats it
    let done = |b: &mut Basket| b.strike = in_the_air(T_RISEN - 0.15, T_RISEN - 0.03, T_RISEN + 0.162, true);
    let fight = swat(&roster, A_05, 2.5, 0.0, done);
    assert!(fight.events.is_empty());
    assert_eq!(st(&fight, 0).baskets[0].down, None);
    let fight = swat(&roster, A_01, 1.5, 0.0, done);
    assert_event(
        &fight.events[0],
        10.44853,
        "swat",
        &[I(1), I(0), I(0), F(0.5), F(-0.25)],
    );
    assert_eq!(st(&fight, 0).baskets[0].down, Some(T_UP));

    // a lunge with no flight time (t_hit == t0) has not left the basket yet
    let fight = swat(&roster, A_05, 2.5, 0.0, |b| {
        b.strike = in_the_air(T_RISEN, T_RISEN, T_RISEN + 0.2, false);
    });
    assert!(fight.events.is_empty());
    assert_eq!(st(&fight, 0).baskets[0].strike.as_ref().map(|s| s.done), Some(false));
}

/// Every cobra is tested, in basket order; the event names the lance's owner first, then the cobra's. A fighter
/// without baskets is nothing to swat.
#[test]
fn swat_cobras_tests_every_cobra_of_the_enemy() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    // 0.4 further along the lance and 0.05 to the side; and one far away
    basket(&mut fight, 0, [0.8559368749137954, 0.007687074895076418], 10.1, None);
    basket(&mut fight, 0, [-2.0, 2.0], 10.1, None);
    fight.t = 10.1 + BEAT;
    swat_cobras(&mut fight, 1, A_01, D, 1.5);
    assert_eq!(fight.events.len(), 2);
    assert_event(
        &fight.events[0],
        10.54853,
        "swat",
        &[I(1), I(0), I(0), F(0.5), F(-0.25)],
    );
    assert_event(
        &fight.events[1],
        10.54853,
        "swat",
        &[I(1), I(0), I(1), F(0.8559368749137954), F(0.007687074895076418)],
    );
    let down: Vec<Option<f64>> = st(&fight, 0).baskets.iter().map(|b| b.down).collect();
    assert_eq!(down, [Some(11.445588235294117), Some(11.445588235294117), None]);

    // the FLUTE's own "lance" call finds no enemy baskets, and a striker with none on the floor has none to swat
    swat_cobras(&mut fight, 0, A_01, D, 1.5);
    assert_eq!(fight.events.len(), 2);
    let mut fight = charmer(&roster);
    fight.t = 12.0;
    swat_cobras(&mut fight, 1, [0.0, 0.0], D, 1.5);
    assert!(fight.events.is_empty());

    // sides swapped: the lance is fighter 0, the cobras are fighter 1's
    let ep = testing::episode_setup(&roster, 392);
    let swapped = crate::Setup {
        fighters: [ep.fighters[1], ep.fighters[0]],
        ..ep
    };
    let mut fight = testing::fight(&swapped, false);
    setup(&mut fight, 1);
    basket(&mut fight, 1, [0.5, -0.25], 10.0, None);
    fight.t = T_RISEN;
    swat_cobras(&mut fight, 0, A_01, D, 1.5);
    assert_event(
        &fight.events[0],
        10.44853,
        "swat",
        &[I(0), I(1), I(0), F(0.5), F(-0.25)],
    );
    assert_eq!(st(&fight, 1).baskets[0].down, Some(T_UP));
}

/// What a swat costs, through the striker's own steps: the cobra cannot lunge until it is up again, and the
/// lunge that was in the air ends without a bite. The same calls in Python gave the same: the swat at
/// (0.6709575030443459, -0.1462043731516472), flat until 11.378921568627451, no bite, a lunge again from then.
#[test]
fn a_swatted_cobra_misses_and_waits() {
    let roster = testing::roster();
    let mut fight = charmer(&roster);
    let b = basket(&mut fight, 0, [0.5, -0.25], 10.0, None);
    fight.balls[1].p = [1.9, 0.6];
    fight.t = T_RISEN;
    fire(&mut fight, 0);
    // 4 steps into the lunge the lance crosses it
    for i in 1..=4 {
        fight.t = T_RISEN + i as f64 * DT;
        step(&mut fight, 0, DT);
    }
    let aim = st(&fight, 0).baskets[b].strike.as_ref().expect("lunging").aim;
    // a lance from beside the cobra straight across the lunge, 0.2 out from the cobra
    let across = [-bf_core::dmath::sin(aim), bf_core::dmath::cos(aim)];
    let along = [bf_core::dmath::cos(aim), bf_core::dmath::sin(aim)];
    let a = [
        0.5 + along[0] * 0.2 - across[0] * 0.5,
        -0.25 + along[1] * 0.2 - across[1] * 0.5,
    ];
    swat_cobras(&mut fight, 1, a, across, 1.0);
    assert_eq!(fight.events.len(), 2, "{:?}", fight.events);
    assert_event(
        &fight.events[1],
        10.48186,
        "swat",
        &[I(1), I(0), I(0), F(0.6709575030443459), F(-0.1462043731516472)],
    );
    let down = st(&fight, 0).baskets[b].down.expect("flat");
    assert_eq!(down, fight.t + 2.0 * BEAT);
    assert_eq!(down, 11.378921568627451);
    // the lunge runs out without a bite although the enemy never moved
    for i in 5..=45 {
        fight.t = T_RISEN + i as f64 * DT;
        step(&mut fight, 0, DT);
    }
    assert_eq!(fight.events.len(), 2, "no bite: {:?}", fight.events);
    assert_eq!(fight.balls[1].hp, 100.0);
    assert!(st(&fight, 0).baskets[b].strike.is_none(), "the lunge is over");
    // on the strike grid it stays down until `down`, then lunges again
    fight.t = down - DT;
    fire(&mut fight, 0);
    assert_eq!(fight.events.len(), 2);
    fight.t = down;
    fire(&mut fight, 0);
    assert_eq!(fight.events.len(), 3);
    assert_eq!(fight.events[2].kind, "strike");
}
