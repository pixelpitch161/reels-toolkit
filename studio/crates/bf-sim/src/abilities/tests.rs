//! The abilities' unit tests. Every expected number is what `fight.py` gave on the reference PC (CPython 3.13, numpy
//! 2.5) for the same hand-built state and the same calls; the Python script that prints them is
//! `tools/studio_export/unit_checks/abilities_unit_vectors.py`.

use std::collections::BTreeMap;

use bf_core::event::{Event, Value};

use super::*;
use crate::fight::{Key, Setup, Slam, Wave};
use crate::geom::{self, V2};
use crate::kinds::striker;
use crate::roster::{FighterCfg, SlowCfg};
use crate::schedule::Schedule;
use crate::testing;

// The hand-built fight of every test below: `Fight('FLUTE', 'TROMBONE', seed=392)` under the episode's rules (the
// FLUTE's hits take twice their damage, the TROMBONE's 0.4 of it) with `striker_setup(0)` called, the balls and the
// clock set as the test sets them, then the same methods called: `trigger(k, slot)`, `abilities_step(on_beat)`, or
// `effects_step(dt)` once per step at `T0 + i * dt`.

/// The episode's beat, `60 / bpm`.
const BEAT: f64 = 0.44852941176470584;
/// `DT`.
const DT: f64 = 0.008333333333333333;
/// The time the hand-built cases start at.
const T0: f64 = 12.5;

use Value::{Float as F, Int as I};

fn s(x: &str) -> Value {
    Value::Str(x.into())
}

fn ev(t: f64, kind: &str, fields: Vec<Value>) -> Event {
    Event {
        t,
        kind: kind.into(),
        fields,
    }
}

/// FLUTE (0) against TROMBONE (1) under the episode's rules at `T0`, the FLUTE set up as a striker.
fn duel(roster: &[FighterCfg]) -> Fight<'_> {
    let mut fight = Fight::new(&testing::episode_setup(roster, 392), false);
    striker::setup(&mut fight, 0);
    fight.t = T0;
    assert_eq!((fight.rules.beat, fight.rules.dt), (BEAT, DT));
    fight
}

/// Puts a ball somewhere, flying some way.
fn place(fight: &mut Fight, k: usize, p: V2, v: V2) {
    fight.balls[k].p = p;
    fight.balls[k].v = v;
}

/// Steps in which something happened, each with a count.
type Steps = Vec<(usize, usize)>;

/// `effects_step` once per step at `T0 + i * dt` for `i` in 1 to `steps`. Returns, per step that made events, the
/// step and how many, and per step that changed the number of rings, the step and the new number.
fn run_effects(fight: &mut Fight, steps: usize) -> (Steps, Steps) {
    let (mut made, mut rings) = (Vec::new(), Vec::new());
    for i in 1..=steps {
        fight.t = T0 + i as f64 * DT;
        let (n0, w0) = (fight.events.len(), fight.waves.len());
        effects_step(fight, DT);
        if fight.events.len() > n0 {
            made.push((i, fight.events.len() - n0));
        }
        if fight.waves.len() != w0 {
            rings.push((i, fight.waves.len()));
        }
    }
    (made, rings)
}

#[track_caller]
fn bits(got: f64, want: f64) {
    assert_eq!(got.to_bits(), want.to_bits(), "{got:?} vs Python's {want:?}");
}

#[track_caller]
fn bits2(got: V2, want: V2) {
    assert_eq!(
        got.map(f64::to_bits),
        want.map(f64::to_bits),
        "{got:?} vs Python's {want:?}"
    );
}

/// `ab_dur`: beats at the fight's tempo, or seconds when the entry gives `dur`.
#[test]
fn a_duration_is_in_beats_or_in_seconds() {
    let roster = testing::roster();
    let fight = duel(&roster);
    let (flute, trombone) = (fight.balls[0].cfg, fight.balls[1].cfg);
    bits(ab_dur(&fight, &flute.unlock1), 3.5882352941176467); // FLUTTER TONGUE, 8 beats
    bits(ab_dur(&fight, &flute.unlock2), 1.7941176470588234); // SNAKE DANCE, 4 beats
    bits(ab_dur(&fight, &trombone.unlock2), 3.5882352941176467); // POWER SLIDE, 8 beats
    let mut both = flute.unlock1.clone();
    both.params.insert("dur".into(), 1.25);
    bits(ab_dur(&fight, &both), 1.25);
}

/// `trigger`: the `ability` event (name, kind, 1 the first time a slot fires and 0 after), the slot marked used,
/// and what each of the six abilities sets.
#[test]
fn trigger_announces_the_ability_and_its_kind_acts() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, 0, [-1.0, 0.5], [1.0, 2.4]);
    place(&mut fight, 1, [1.5, -0.25], [-2.4, 1.0]);
    let calls = [
        (0, Slot::Unlock1),
        (0, Slot::Unlock1),
        (0, Slot::Unlock2),
        (0, Slot::Low),
        (1, Slot::Unlock1),
        (1, Slot::Unlock2),
        (1, Slot::Low),
        (1, Slot::Low),
    ];
    // (haste, haste_mult, dance_until of the FLUTE, rings, slams) after each call
    let after = [
        (3.5882352941176467, 2.0, -1.0, 0, 0),
        (3.5882352941176467, 2.0, -1.0, 0, 0),
        (3.5882352941176467, 2.0, 14.319117647058821, 0, 0),
        (3.5882352941176467, 2.0, 14.319117647058821, 1, 0),
        (0.0, 1.0, 14.319117647058821, 2, 0),
        (3.5882352941176467, 2.0, 14.319117647058821, 2, 0),
        (3.5882352941176467, 2.0, 14.319117647058821, 2, 1),
        (3.5882352941176467, 2.0, 14.319117647058821, 2, 2),
    ];
    for ((k, slot), (haste, mult, dance, rings, slams)) in calls.into_iter().zip(after) {
        fight.t += DT;
        trigger(&mut fight, k, slot);
        let b = &fight.balls[k];
        assert!(b.used[slot.index()]);
        bits(b.haste, haste);
        bits(b.haste_mult, mult);
        bits(fight.striker[0].as_ref().expect("set up").dance_until, dance);
        assert_eq!((fight.waves.len(), fight.slams.len()), (rings, slams), "{k} {slot:?}");
    }
    assert_eq!((fight.balls[0].used, fight.balls[1].used), ([true; 3], [true; 3]));
    assert_eq!(
        fight.events,
        vec![
            ev(12.50833, "ability", vec![I(0), s("FLUTTER TONGUE"), s("haste"), I(1)]),
            ev(12.51667, "ability", vec![I(0), s("FLUTTER TONGUE"), s("haste"), I(0)]),
            ev(12.525, "ability", vec![I(0), s("SNAKE DANCE"), s("dance"), I(1)]),
            ev(12.53333, "ability", vec![I(0), s("PICCOLO"), s("wave"), I(1)]),
            ev(12.54167, "ability", vec![I(1), s("SAD TROMBONE"), s("shockwave"), I(1)]),
            ev(12.55, "ability", vec![I(1), s("POWER SLIDE"), s("haste"), I(1)]),
            ev(12.55833, "ability", vec![I(1), s("PEDAL TONE"), s("slam"), I(1)]),
            ev(12.56667, "ability", vec![I(1), s("PEDAL TONE"), s("slam"), I(0)]),
        ]
    );
    assert_eq!(
        fight.waves,
        vec![
            Wave {
                owner: 0,
                c: [-1.0, 0.5],
                r: 0.6,
                speed: 6.0,
                rmax: 9.0,
                dmg: 8.0,
                knock: 1.5,
                kind: "wave".into(),
                hit: false,
                name: "PICCOLO".into(),
                t0: 12.533333333333331,
                slow: None,
            },
            Wave {
                owner: 1,
                c: [1.5, -0.25],
                r: 0.6,
                speed: 6.0,
                rmax: 3.4,
                dmg: 3.0,
                knock: 1.0,
                kind: "shockwave".into(),
                hit: false,
                name: "SAD TROMBONE".into(),
                t0: 12.541666666666664,
                slow: Some(SlowCfg { mult: 0.6, dur: 1.0 }),
            },
        ]
    );
    let slam = |t0: f64, t_slam: f64| Slam {
        owner: 1,
        c: [-0.46176470588235297, 1.791764705882353],
        t0,
        t_slam,
        radius: 1.35,
        dmg: 24.0,
        knock: 1.6,
        done: false,
        name: "PEDAL TONE".into(),
    };
    assert_eq!(
        fight.slams,
        vec![
            slam(12.55833333333333, 13.455392156862741),
            slam(12.566666666666663, 13.463725490196074)
        ]
    );
    // the ring's centre is where its owner WAS: the owner moves on, the centre stays
    fight.balls[0].p = [0.0, 1.5];
    assert_eq!(fight.waves[0].c, [-1.0, 0.5]);
}

/// A slam is called on `O.p + O.v * lead * 0.6` with `lead = tele` beats, each coordinate held inside the ropes less
/// a ball radius (2.4), and lands `lead` seconds later.
#[test]
fn a_slam_is_called_on_where_the_enemy_is_heading() {
    let roster = testing::roster();
    for (p, v, c) in [
        ([0.2, -0.4], [1.5, -2.1], [1.0073529411764706, -1.5302941176470588]),
        ([2.2, -2.3], [2.6, 0.0], [2.4, -2.3]),
        ([-2.3, 2.39], [-1.0, 2.4], [-2.4, 2.4]),
        ([0.0, 0.0], [0.0, 0.0], [0.0, 0.0]),
    ] {
        let mut fight = duel(&roster);
        place(&mut fight, 0, p, v);
        trigger(&mut fight, 1, Slot::Low);
        let slam = &fight.slams[0];
        bits2(slam.c, c);
        bits(slam.t0, 12.5);
        bits(slam.t_slam, 13.397058823529411);
        assert_eq!((slam.owner, slam.done), (1, false));
    }
}

/// `abilities_step`: an unlock fires on the first beat at or after its level-up, one ability per fighter per beat,
/// `unlock1` before `unlock2`, and is ready again `every_bars` bars later (to 1e-6 s); the low-HP ability fires
/// once, on a beat, and takes that beat's turn.
#[test]
fn unlocks_fire_on_the_beat_one_at_a_time() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    let dance = |fight: &Fight| fight.striker[0].as_ref().expect("set up").dance_until;
    let step = |fight: &mut Fight, t: f64, on_beat: bool| -> Vec<Event> {
        let n0 = fight.events.len();
        fight.t = t;
        fight.abilities_step(on_beat);
        fight.events[n0..].to_vec()
    };
    let t0 = 10.0;
    fight.balls[0].dealt = 40.0;
    // both levels at once, between beats: ready, but nothing fires
    assert_eq!(
        step(&mut fight, t0, false),
        vec![ev(10.0, "level", vec![I(0), I(2)]), ev(10.0, "level", vec![I(0), I(3)])]
    );
    assert_eq!(fight.balls[0].ready, [Some(10.0), Some(10.0)]);
    assert_eq!(step(&mut fight, t0 + DT, false), vec![]);
    // a beat: the first unlock, ready again 4 bars on
    assert_eq!(
        step(&mut fight, t0 + 2.0 * DT, true),
        vec![ev(
            10.01667,
            "ability",
            vec![I(0), s("FLUTTER TONGUE"), s("haste"), I(1)]
        )]
    );
    let r1 = 17.193137254901963;
    assert_eq!(fight.balls[0].ready, [Some(r1), Some(10.0)]);
    bits(fight.balls[0].haste, 3.5882352941176467);
    bits(dance(&fight), -1.0);
    // the next beat: the second unlock, ready again 5 bars on
    assert_eq!(
        step(&mut fight, t0 + 3.0 * DT, true),
        vec![ev(10.025, "ability", vec![I(0), s("SNAKE DANCE"), s("dance"), I(1)])]
    );
    let r2 = 18.995588235294115;
    assert_eq!(fight.balls[0].ready, [Some(r1), Some(r2)]);
    bits(dance(&fight), 11.819117647058825);
    // a beat with nothing ready
    assert_eq!(step(&mut fight, t0 + 4.0 * DT, true), vec![]);
    // ready means `t >= ready - 1e-6`
    assert_eq!(step(&mut fight, r1 - 2e-6, true), vec![]);
    assert_eq!(
        step(&mut fight, r1 - 5e-7, true),
        vec![ev(
            17.19314,
            "ability",
            vec![I(0), s("FLUTTER TONGUE"), s("haste"), I(0)]
        )]
    );
    assert_eq!(fight.balls[0].ready, [Some(24.369607343137258), Some(r2)]);
    // low HP: nothing between beats; on a beat the low ability fires and takes the turn, though unlock2 is ready
    fight.balls[0].hp = 24.9;
    assert_eq!(step(&mut fight, r2 + 0.5, false), vec![]);
    assert!(!fight.balls[0].low_done);
    assert_eq!(
        step(&mut fight, r2 + 0.5 + DT, true),
        vec![ev(19.50392, "ability", vec![I(0), s("PICCOLO"), s("wave"), I(1)])]
    );
    assert!(fight.balls[0].low_done);
    assert_eq!(
        (fight.balls[0].ready, fight.waves.len()),
        ([Some(24.369607343137258), Some(r2)], 1)
    );
    // the next beat: unlock2
    assert_eq!(
        step(&mut fight, r2 + 0.5 + 2.0 * DT, true),
        vec![ev(19.51225, "ability", vec![I(0), s("SNAKE DANCE"), s("dance"), I(0)])]
    );
    assert_eq!(
        fight.balls[0].ready,
        [Some(24.369607343137258), Some(28.482843137254896)]
    );
    bits(dance(&fight), 21.306372549019603);
    // still low: the low ability does not fire again
    fight.balls[0].hp = 10.0;
    assert_eq!(step(&mut fight, r2 + 0.5 + 3.0 * DT, true), vec![]);
    assert_eq!(fight.waves.len(), 1);
    // the other fighter did nothing all along
    assert_eq!((fight.balls[1].level, fight.balls[1].used), (1, [false; 3]));
}

/// In one step fighter 0 has its whole turn (level, then ability) before fighter 1.
#[test]
fn fighter_0_takes_its_turn_before_fighter_1() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    fight.balls[0].dealt = 14.0;
    fight.balls[1].dealt = 14.0;
    fight.t = 11.0;
    fight.abilities_step(true);
    assert_eq!(
        fight.events,
        vec![
            ev(11.0, "level", vec![I(0), I(2)]),
            ev(11.0, "ability", vec![I(0), s("FLUTTER TONGUE"), s("haste"), I(1)]),
            ev(11.0, "level", vec![I(1), I(2)]),
            ev(11.0, "ability", vec![I(1), s("SAD TROMBONE"), s("shockwave"), I(1)]),
        ]
    );
}

/// A kind the engine has not ported is announced, marked used and timed like any other, and does nothing else (in
/// Python: a fighter whose three abilities are of a kind `trigger` has no branch for).
#[test]
fn a_kind_not_ported_is_announced_and_does_nothing() {
    let roster = testing::roster();
    let mut nobody = testing::fighter(&roster, "FLUTE").clone();
    nobody.name = "NOBODY".into();
    for (ab, name, every_bars) in [
        (&mut nobody.unlock1, "A", Some(2.0)),
        (&mut nobody.unlock2, "B", Some(3.0)),
        (&mut nobody.low, "C", None),
    ] {
        ab.name = name.into();
        ab.kind = AbilityKind::Unsupported("none".into());
        ab.slow = None;
        ab.params = every_bars
            .map(|n| ("every_bars".to_string(), n))
            .into_iter()
            .collect::<BTreeMap<_, _>>();
    }
    let setup = Setup {
        fighters: [&nobody, testing::fighter(&roster, "FLUTE")],
        rules: testing::episode_rules(),
        seed: 392,
    };
    let mut fight = Fight::new(&setup, false);
    striker::setup(&mut fight, 0);
    fight.balls[0].dealt = 40.0;
    fight.balls[0].hp = 10.0;
    for i in 0..4 {
        fight.t = 10.0 + i as f64 * DT;
        fight.abilities_step(true);
    }
    assert_eq!(
        fight.events,
        vec![
            ev(10.0, "level", vec![I(0), I(2)]),
            ev(10.0, "level", vec![I(0), I(3)]),
            ev(10.0, "ability", vec![I(0), s("C"), s("none"), I(1)]),
            ev(10.00833, "ability", vec![I(0), s("A"), s("none"), I(1)]),
            ev(10.01667, "ability", vec![I(0), s("B"), s("none"), I(1)]),
        ]
    );
    let b = &fight.balls[0];
    assert_eq!(b.ready, [Some(13.59656862745098), Some(15.399019607843137)]);
    assert_eq!((b.used, b.low_done), ([true; 3], true));
    assert_eq!((b.haste, b.haste_mult, b.shield, b.slow), (0.0, 1.0, 0.0, 0.0));
    assert!(fight.waves.is_empty() && fight.slams.is_empty());
    assert_eq!(fight.striker[0].as_ref().expect("set up").dance_until, -1.0);
}

/// `haste`: the time and the factor; the fight loop runs the time down a step at a time, and the weapon grid runs
/// `mult` times as fast while it lasts: 8 beats are 430.6 steps, so the factor counts for 430 steps after the one
/// that fired it.
#[test]
fn haste_lasts_its_beats_in_the_fight_loop() {
    let roster = testing::roster();
    let bare = testing::bare_pair();
    let mut rules = testing::episode_rules();
    rules.lv_dmg = (1e9, 1e9); // no level, so nothing fires but what the test fires
    let setup = Setup {
        fighters: [testing::fighter(&roster, "FLUTE"), &bare[1]],
        rules,
        seed: 392,
    };
    let mut fight = Fight::new(&setup, false);
    let mut sched = Schedule::new(&mut fight);
    sched.step(&mut fight);
    trigger(&mut fight, 0, Slot::Unlock1);
    bits(fight.balls[0].haste, 3.5882352941176467);
    assert_eq!((fight.ab_mult(0), fight.ab_mult(1)), (2.0, 1.0));
    let mut left = Vec::new();
    for _ in 0..431 {
        sched.step(&mut fight);
        left.push((fight.balls[0].haste, fight.ab_mult(0)));
    }
    for (step, haste, mult) in [
        (1, 3.5799019607843134, 2.0),
        (2, 3.57156862745098, 2.0),
        (429, 0.013235294117656974, 2.0),
        (430, 0.004901960784323641, 2.0),
        (431, 0.0, 1.0),
    ] {
        bits(left[step - 1].0, haste);
        assert_eq!(left[step - 1].1, mult, "step {step}");
    }
    // a haste that fires while one runs starts the time again
    trigger(&mut fight, 0, Slot::Unlock1);
    sched.step(&mut fight);
    trigger(&mut fight, 0, Slot::Unlock1);
    bits(fight.balls[0].haste, 3.5882352941176467);
    let again = fight.events.iter().filter(|e| e.kind == "ability").map(|e| e.i(5));
    assert_eq!(again.collect::<Vec<_>>(), [1, 0, 0]);
}

/// `dance` on a fighter that is no striker sets nothing the engine reads, and does not fail.
#[test]
fn a_dance_without_cobras_does_nothing() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    let ab = fight.balls[0].cfg.unlock2.clone();
    dance::fire(&mut fight, 1, &ab);
    assert!(fight.striker[1].is_none());
    bits(fight.striker[0].as_ref().expect("set up").dance_until, -1.0);
    dance::fire(&mut fight, 0, &ab);
    bits(
        fight.striker[0].as_ref().expect("set up").dance_until,
        14.294117647058822,
    );
}

/// PICCOLO: the ring grows by `speed * dt` a step from the ball's radius, hits the enemy 2.5 away in the step its
/// front reaches 2.2 (half a ball radius short of the enemy's centre), once, and is gone when it reaches its final
/// radius, 168 steps after it was fired.
#[test]
fn a_wave_spreads_hits_once_and_fades() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, 0, [-1.0, 0.5], [1.0, 2.4]);
    place(&mut fight, 1, [1.5, 0.5], [-2.4, 1.0]);
    trigger(&mut fight, 0, Slot::Low);
    let mut radius = BTreeMap::new();
    let (mut made, mut rings) = (Vec::new(), Vec::new());
    for i in 1..=170 {
        fight.t = T0 + i as f64 * DT;
        let (n0, w0) = (fight.events.len(), fight.waves.len());
        effects_step(&mut fight, DT);
        if let Some(w) = fight.waves.first() {
            radius.insert(i, w.r);
        }
        if fight.events.len() > n0 {
            made.push(i);
        }
        if fight.waves.len() != w0 {
            rings.push((i, fight.waves.len()));
        }
    }
    for (step, r) in [
        (1, 0.65),
        (2, 0.7000000000000001),
        (3, 0.7500000000000001),
        (31, 2.1500000000000004),
        (32, 2.2),
        (33, 2.25),
        (34, 2.3),
        (100, 5.599999999999988),
        (166, 8.899999999999991),
        (167, 8.949999999999992),
    ] {
        bits(radius[&step], r);
    }
    assert_eq!((made, rings), (vec![32], vec![(168, 0)]));
    assert_eq!(
        fight.events[1..],
        [ev(
            12.76667,
            "hit",
            vec![I(0), I(1), F(16.0), F(0.8999999999999999), F(0.5), s("wave"), I(0)]
        )]
    );
    let (a, b) = (&fight.balls[0], &fight.balls[1]);
    bits2(b.v, [2.579922479451396, 0.3224903099312632]);
    assert_eq!((b.boost, b.hp, b.slow, b.slow_mult), (1.5, 84.0, 0.0, 1.0));
    assert_eq!((a.dealt, a.hp, a.v), (16.0, 100.0, [1.0, 2.4]));
}

/// SAD TROMBONE: the shockwave hits the enemy its front passes and slows it: `slow` and `slow_mult` from the
/// ability's `slow`, and a `slow` event after the `hit`. The knock is straight away from the ring's centre and never
/// lowers a larger boost.
#[test]
fn a_shockwave_slows_the_enemy_it_hits() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, 1, [1.0, -0.5], [-2.4, 1.0]);
    place(&mut fight, 0, [-0.8, 0.3], [1.0, 2.4]);
    fight.balls[0].boost = 0.25;
    trigger(&mut fight, 1, Slot::Unlock1);
    let mut radius = BTreeMap::new();
    let (mut made, mut rings) = (Vec::new(), Vec::new());
    for i in 1..=60 {
        fight.t = T0 + i as f64 * DT;
        let (n0, w0) = (fight.events.len(), fight.waves.len());
        effects_step(&mut fight, DT);
        if let Some(w) = fight.waves.first() {
            radius.insert(i, (w.r, w.hit));
        }
        if fight.events.len() > n0 {
            made.push(i);
        }
        if fight.waves.len() != w0 {
            rings.push((i, fight.waves.len()));
        }
    }
    for (step, r, hit) in [
        (1, 0.65, false),
        (21, 1.6500000000000008, false),
        (22, 1.7000000000000008, true),
        (23, 1.7500000000000009, true),
        (24, 1.800000000000001, true),
        (55, 3.349999999999996, true),
    ] {
        bits(radius[&step].0, r);
        assert_eq!(radius[&step].1, hit, "step {step}");
    }
    assert_eq!((made, rings), (vec![22], vec![(56, 0)]));
    assert_eq!(
        fight.events[1..],
        [
            ev(
                12.68333,
                "hit",
                vec![
                    I(1),
                    I(0),
                    F(1.2000000000000002),
                    F(-0.25171307082784566),
                    F(0.056316920367931345),
                    s("shockwave"),
                    I(0)
                ]
            ),
            ev(12.68333, "slow", vec![I(0), I(1), F(1.0)]),
        ]
    );
    let (a, b) = (&fight.balls[0], &fight.balls[1]);
    bits2(a.v, [-2.051158241295944, 1.5977327276949125]);
    assert_eq!((a.boost, a.hp, a.slow, a.slow_mult), (1.0, 98.8, 1.0, 0.6));
    assert_eq!((b.dealt, b.slow, b.slow_mult), (1.2000000000000002, 0.0, 1.0));
}

/// A shielded enemy blocks the ring (a `block` event, no damage), is still knocked, and is not slowed.
#[test]
fn a_shield_blocks_a_ring_but_not_its_knock() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, 1, [1.0, -0.5], [-2.4, 1.0]);
    place(&mut fight, 0, [-0.8, 0.3], [1.0, 2.4]);
    fight.balls[0].shield = 5.0;
    trigger(&mut fight, 1, Slot::Unlock1);
    assert_eq!(run_effects(&mut fight, 60), (vec![(22, 1)], vec![(56, 0)]));
    assert_eq!(
        fight.events[1..],
        [ev(
            12.68333,
            "block",
            vec![
                I(0),
                I(1),
                F(-0.25171307082784566),
                F(0.056316920367931345),
                s("shockwave")
            ]
        )]
    );
    let a = &fight.balls[0];
    bits2(a.v, [-2.051158241295944, 1.5977327276949125]);
    assert_eq!((a.boost, a.hp, a.slow, a.slow_mult), (1.0, 100.0, 0.0, 1.0));
    assert_eq!(fight.balls[1].dealt, 0.0);
}

/// A ring hits only an enemy its front passes: not one beyond its final radius, not one that is inside it from the
/// start, but one that was inside and then gets ahead of the front.
#[test]
fn a_ring_hits_only_an_enemy_its_front_passes() {
    let roster = testing::roster();
    // 4.5 away: the front stops at 3.4
    let mut fight = duel(&roster);
    fight.balls[1].p = [2.0, -0.5];
    place(&mut fight, 0, [-2.0, 1.5617376914898995], [1.0, 2.4]);
    trigger(&mut fight, 1, Slot::Unlock1);
    assert_eq!(run_effects(&mut fight, 60), (vec![], vec![(56, 0)]));
    assert_eq!(fight.balls[0].hp, 100.0);
    // 0.8 away: `dist - R / 2` is 0.5, behind the front from the start
    let mut fight = duel(&roster);
    fight.balls[1].p = [1.0, -0.5];
    place(&mut fight, 0, [0.2, -0.5], [1.0, 2.4]);
    trigger(&mut fight, 1, Slot::Unlock1);
    assert_eq!(run_effects(&mut fight, 60), (vec![], vec![(56, 0)]));
    // the same, but at step 10 the enemy is 2.0 away, ahead of the front: it is hit when the front gets there
    let mut fight = duel(&roster);
    fight.balls[1].p = [1.0, -0.5];
    place(&mut fight, 0, [0.2, -0.5], [1.0, 2.4]);
    trigger(&mut fight, 1, Slot::Unlock1);
    assert_eq!(run_effects(&mut fight, 9), (vec![], vec![]));
    fight.balls[0].p = [-1.0, -0.5];
    let mut made = Vec::new();
    for i in 10..=60 {
        fight.t = T0 + i as f64 * DT;
        let n0 = fight.events.len();
        effects_step(&mut fight, DT);
        if fight.events.len() > n0 {
            made.push(i);
        }
    }
    assert_eq!(made, [22]);
    assert_eq!(
        fight.events[1..],
        [
            ev(
                12.68333,
                "hit",
                vec![
                    I(1),
                    I(0),
                    F(1.2000000000000002),
                    F(-0.3999999999999999),
                    F(-0.5),
                    s("shockwave"),
                    I(0)
                ]
            ),
            ev(12.68333, "slow", vec![I(0), I(1), F(1.0)]),
        ]
    );
    bits2(fight.balls[0].v, [-2.519186047620294, 0.6431964376900141]);
}

/// The hit test is `r0 < dist - R / 2 <= r + 1e-9` with `r0` the radius before the step and `r` after it: the step
/// each enemy distance is hit in, with the radii Python had there.
#[test]
fn the_front_passes_the_enemy_in_one_step_only() {
    let roster = testing::roster();
    for (x, step, r0, r) in [
        (2.2, 32, 2.1500000000000004, 2.2),
        (2.2 + 1e-12, 32, 2.1500000000000004, 2.2),
        (2.25, 33, 2.2, 2.25),
        (2.2 - 1e-12, 32, 2.1500000000000004, 2.2),
        (2.15, 31, 2.1000000000000005, 2.1500000000000004),
    ] {
        let mut fight = duel(&roster);
        fight.balls[1].p = [0.0, 0.0];
        fight.balls[0].p = [-(x + 0.3), 0.0];
        trigger(&mut fight, 1, Slot::Unlock1);
        let mut hit = Vec::new();
        for i in 1..60 {
            fight.t = T0 + i as f64 * DT;
            let before = fight.waves.first().map(|w| w.r);
            let n0 = fight.events.len();
            effects_step(&mut fight, DT);
            if fight.events.len() > n0 {
                hit.push((i, before.expect("the ring is there"), fight.waves[0].r));
            }
        }
        assert_eq!(hit.len(), 1, "front target {x}");
        assert_eq!(hit[0].0, step, "front target {x}");
        bits(hit[0].1, r0);
        bits(hit[0].2, r);
    }
}

/// The edges of that test, on floats that sit exactly on them (found in Python by trying the neighbours of the
/// decimal). The upper edge counts: an enemy whose `dist - R / 2` is exactly `r + 1e-9` after the first step is hit
/// in that step, not the next. The lower edge does not: `r0 < dist - R / 2`, so an enemy one float past the
/// starting front (0.6) is hit in the first step. (No float sits exactly ON the starting front with a ball radius
/// of 0.6: `0.6 + 0.3` is not a double.)
#[test]
fn the_edges_of_the_hit_test() {
    let roster = testing::roster();
    for (x, reach) in [(-0.9500000009999999, 0.650000001), (-0.9, 0.6000000000000001)] {
        let mut fight = duel(&roster);
        fight.balls[1].p = [0.0, 0.0];
        place(&mut fight, 0, [x, 0.0], [1.0, 2.4]);
        bits(geom::hypot(fight.balls[0].p) - 0.6 * 0.5, reach);
        trigger(&mut fight, 1, Slot::Unlock1);
        let (made, _) = run_effects(&mut fight, 5);
        assert_eq!(made, [(1, 2)], "an enemy at x = {x}: the hit and the slow, in step 1");
    }
    bits(0.65 + 1e-9, 0.650000001);
}

/// A ring whose final radius is not one of its steps (0.6 + n x 0.05) stops AT the radius in its last step: a
/// shockwave of radius 3.42 reaches 3.399999999999996 in step 56 and 3.42, not 3.45, in step 57, where it is gone.
/// An enemy with `dist - R / 2` = 3.41 is hit in that last step; one at 3.44 is beyond the ring.
#[test]
fn a_ring_stops_at_its_final_radius() {
    let roster = testing::roster();
    let mut off_grid = testing::fighter(&roster, "TROMBONE").clone();
    off_grid.unlock1.params.insert("radius".into(), 3.42);
    for (reach, hit) in [(3.41, true), (3.44, false)] {
        let setup = Setup {
            fighters: [testing::fighter(&roster, "FLUTE"), &off_grid],
            rules: testing::episode_rules(),
            seed: 392,
        };
        let mut fight = Fight::new(&setup, false);
        fight.t = T0;
        fight.balls[1].p = [0.0, 0.0];
        place(&mut fight, 0, [-(reach + 0.3), 0.0], [1.0, 2.4]);
        trigger(&mut fight, 1, Slot::Unlock1);
        assert_eq!(fight.waves[0].rmax, 3.42);
        assert_eq!(run_effects(&mut fight, 55), (vec![], vec![]));
        bits(fight.waves[0].r, 3.349999999999996);
        fight.t = T0 + 56.0 * DT;
        effects_step(&mut fight, DT);
        bits(fight.waves[0].r, 3.399999999999996);
        assert_eq!(fight.events.len(), 1, "nothing but the ability so far");
        fight.t = T0 + 57.0 * DT;
        effects_step(&mut fight, DT);
        assert!(fight.waves.is_empty(), "gone in the step it reaches its radius");
        let kinds: Vec<&str> = fight.events.iter().map(|e| e.kind.as_str()).collect();
        if hit {
            assert_eq!(kinds, ["ability", "hit", "slow"], "dist - R / 2 = {reach}");
        } else {
            assert_eq!(kinds, ["ability"], "dist - R / 2 = {reach}");
        }
    }
}

/// Two rings in flight are stepped in the order they were fired, each against its owner's enemy; one that fades
/// leaves the other in place.
#[test]
fn two_rings_keep_their_order() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, 0, [-0.9, 0.2], [1.0, 2.4]);
    place(&mut fight, 1, [0.9, -0.2], [-2.4, 1.0]);
    trigger(&mut fight, 1, Slot::Unlock1);
    trigger(&mut fight, 0, Slot::Low);
    assert_eq!(run_effects(&mut fight, 60), (vec![(19, 3)], vec![(56, 1)]));
    assert_eq!(
        fight.events[2..],
        [
            ev(
                12.65833,
                "hit",
                vec![
                    I(1),
                    I(0),
                    F(1.2000000000000002),
                    F(-0.3142877638896283),
                    F(0.06984172530880628),
                    s("shockwave"),
                    I(0)
                ]
            ),
            ev(12.65833, "slow", vec![I(0), I(1), F(1.0)]),
            ev(
                12.65833,
                "hit",
                vec![
                    I(0),
                    I(1),
                    F(16.0),
                    F(0.3142877638896283),
                    F(-0.06984172530880628),
                    s("wave"),
                    I(0)
                ]
            ),
        ]
    );
    let (a, b) = (&fight.balls[0], &fight.balls[1]);
    bits2(a.v, [-2.3173617324406384, 1.1789124653690828]);
    bits2(b.v, [2.5665296861110836, -0.415842963494832]);
    assert_eq!(
        (a.boost, a.hp, a.dealt, a.slow, a.slow_mult),
        (1.0, 98.8, 16.0, 1.0, 0.6)
    );
    assert_eq!((b.boost, b.hp, b.dealt, b.slow), (1.5, 84.0, 1.2000000000000002, 0.0));
    // the shockwave is gone, the PICCOLO ring goes on
    assert_eq!(fight.waves.len(), 1);
    assert_eq!((fight.waves[0].owner, fight.waves[0].kind.as_str()), (0, "wave"));
    bits(fight.waves[0].r, 3.599999999999995);
}

/// A ring clears the enemy's projectiles in the band its front sweeps in a step, `r0 - 0.15 < d <= r + 0.05` from
/// its centre, and leaves its owner's alone. Event `cleared`: the projectile's owner, the ring's owner, where the
/// projectile was and its id. (The projectiles are put in by hand, as Python's `spawn_key` would: no ported kind
/// fires one.)
#[test]
fn a_ring_clears_the_enemy_projectiles_its_front_passes() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    fight.balls[1].p = [1.0, -0.5];
    place(&mut fight, 0, [-2.0, 2.0], [1.0, 2.4]);
    trigger(&mut fight, 1, Slot::Unlock1);
    let spots = [
        (0, 1.0),
        (1, 1.0),
        (0, 0.95),
        (0, 0.9501),
        (0, 1.2),
        (0, 1.2001),
        (0, 0.5),
        (0, 3.0),
    ];
    for (id, (own, d)) in spots.into_iter().enumerate() {
        fight.keys.push(Key {
            p: [1.0 - d, -0.5],
            v: [0.0, 0.0],
            own,
            id: id as i64,
            vol: 0,
            idx: 0,
            dmg: 2.0,
        });
    }
    assert_eq!(
        run_effects(&mut fight, 11),
        (vec![(1, 1), (6, 1), (7, 2), (11, 1)], vec![])
    );
    let cleared = |t: f64, x: f64, id: i64| ev(t, "cleared", vec![I(0), I(1), F(x), F(-0.5), I(id)]);
    assert_eq!(
        fight.events[1..],
        [
            cleared(12.50833, 0.5, 6),
            cleared(12.55, 0.050000000000000044, 2),
            cleared(12.55833, 0.0, 0),
            cleared(12.55833, 0.049900000000000055, 3),
            cleared(12.59167, -0.19999999999999996, 4),
        ]
    );
    let left: Vec<(usize, i64)> = fight.keys.iter().map(|key| (key.own, key.id)).collect();
    assert_eq!(left, [(1, 1), (0, 5), (0, 7)]);
    bits(fight.waves[0].r, 1.1500000000000004);
}

/// The inner edge of that band is outside it (`d > r0 - 0.15`): a projectile that turns up exactly there, behind
/// the front, is not cleared, in that step or later; one a float further out is.
#[test]
fn a_projectile_on_the_inner_edge_of_the_band_stays() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    fight.balls[1].p = [0.0, 0.0];
    place(&mut fight, 0, [-2.0, 2.5], [1.0, 2.4]);
    trigger(&mut fight, 1, Slot::Unlock1);
    assert_eq!(run_effects(&mut fight, 10), (vec![], vec![]));
    let r0 = fight.waves[0].r;
    bits(r0, 1.1000000000000003);
    let inner = r0 - 0.15;
    bits(inner, 0.9500000000000003);
    for (id, p) in [[inner, 0.0], [0.9500000000000004, 0.0], [0.0, -inner]]
        .into_iter()
        .enumerate()
    {
        fight.keys.push(Key {
            p,
            v: [0.0, 0.0],
            own: 0,
            id: id as i64,
            vol: 0,
            idx: id as i64,
            dmg: 2.0,
        });
    }
    let mut made = Vec::new();
    for i in 11..=15 {
        fight.t = T0 + i as f64 * DT;
        let n0 = fight.events.len();
        effects_step(&mut fight, DT);
        if fight.events.len() > n0 {
            made.push(i);
        }
    }
    assert_eq!(made, [11]);
    assert_eq!(
        fight.events[1..],
        [ev(
            12.59167,
            "cleared",
            vec![I(0), I(1), F(0.9500000000000004), F(0.0), I(1)]
        )]
    );
    let left: Vec<(i64, V2)> = fight.keys.iter().map(|key| (key.id, key.p)).collect();
    assert_eq!(left, [(0, [0.9500000000000003, 0.0]), (2, [0.0, -0.9500000000000003])]);
}

/// The hand-built slam: the TROMBONE calls PEDAL TONE at `T0` on an enemy at (0.2, -0.4) flying (1.5, -2.1); just
/// before it lands the enemy is put at `at`. Returns the fight after 112 steps and the steps that made events.
fn slam_on<'a>(roster: &'a [FighterCfg], at: V2, boost: f64, shield: f64) -> (Fight<'a>, Vec<usize>) {
    let mut fight = duel(roster);
    place(&mut fight, 0, [0.2, -0.4], [1.5, -2.1]);
    fight.balls[0].boost = boost;
    fight.balls[0].shield = shield;
    trigger(&mut fight, 1, Slot::Low);
    let t_slam = fight.slams[0].t_slam;
    let mut made = Vec::new();
    for i in 1..=112 {
        fight.t = T0 + i as f64 * DT;
        if fight.t >= t_slam && !fight.slams[0].done {
            fight.balls[0].p = at;
        }
        let n0 = fight.events.len();
        effects_step(&mut fight, DT);
        if fight.events.len() > n0 {
            made.push(i);
        }
    }
    (fight, made)
}

/// The slam's centre in the hand-built cases.
const C: V2 = [1.0073529411764706, -1.5302941176470588];

/// PEDAL TONE lands in the first step at or after its time (2 beats are 107.6 steps: step 108), once. An enemy
/// within `radius + 0.6 R` of the centre takes the damage at its own centre and is knocked away from the slam's
/// centre, its boost raised to the slam's `knock`.
#[test]
fn a_slam_lands_once_two_beats_later() {
    let roster = testing::roster();
    let (fight, made) = slam_on(&roster, [C[0] + 0.3, C[1] + 0.4], 0.1, 0.0);
    assert_eq!(made, [108]);
    let hit_at = [1.3073529411764706, -1.130294117647059];
    assert_eq!(
        fight.events[1..],
        [
            ev(13.4, "slam", vec![I(1), F(C[0]), F(C[1]), I(1)]),
            ev(
                13.4,
                "hit",
                vec![
                    I(1),
                    I(0),
                    F(9.600000000000001),
                    F(hit_at[0]),
                    F(hit_at[1]),
                    s("slam"),
                    I(0)
                ]
            ),
        ]
    );
    let (a, b) = (&fight.balls[0], &fight.balls[1]);
    bits2(a.p, hit_at);
    bits2(a.v, [2.0298350442023927, 1.6247368073968684]);
    assert_eq!((a.boost, a.hp), (1.6, 90.4));
    assert_eq!(b.dealt, 9.600000000000001);
    // it has landed and stays listed
    assert_eq!(fight.slams.len(), 1);
    assert!(fight.slams[0].done);
    bits(fight.slams[0].t_slam, 13.397058823529411);
}

/// The slam's reach is `radius + 0.6 R` = 1.71 from its centre, strictly; a miss still makes the `slam` event, with
/// a 0. An enemy standing on the centre (within 1e-3) is knocked along its own heading, else away from the centre.
#[test]
fn a_slam_reaches_its_radius_and_a_bit() {
    let roster = testing::roster();
    // 1.70 away: inside
    let (fight, made) = slam_on(&roster, [C[0], C[1] + 1.70], 0.0, 0.0);
    assert_eq!(made, [108]);
    assert_eq!(fight.events[1], ev(13.4, "slam", vec![I(1), F(C[0]), F(C[1]), I(1)]));
    assert_eq!(
        fight.events[2],
        ev(
            13.4,
            "hit",
            vec![
                I(1),
                I(0),
                F(9.600000000000001),
                F(1.0073529411764706),
                F(0.16970588235294115),
                s("slam"),
                I(0)
            ]
        )
    );
    bits2(fight.balls[0].v, [0.4665917022470295, 2.5577904885610447]);
    // 1.72 away: a miss
    let (fight, made) = slam_on(&roster, [C[0], C[1] + 1.72], 0.0, 0.0);
    assert_eq!(made, [108]);
    assert_eq!(
        fight.events[1..],
        [ev(13.4, "slam", vec![I(1), F(C[0]), F(C[1]), I(0)])]
    );
    let a = &fight.balls[0];
    assert_eq!((a.v, a.boost, a.hp), ([1.5, -2.1], 0.0, 100.0));
    assert!(fight.slams[0].done);
    // 5e-4 from the centre: knocked along its own heading
    let (fight, _) = slam_on(&roster, [C[0] + 5e-4, C[1]], 0.0, 0.0);
    assert_eq!(fight.events.len(), 3);
    bits2(fight.balls[0].v, [1.5112193036681392, -2.115707025135395]);
    assert_eq!((fight.balls[0].boost, fight.balls[0].hp), (1.6, 90.4));
    // 2e-3 from the centre: knocked away from it
    let (fight, _) = slam_on(&roster, [C[0] + 2e-3, C[1]], 0.0, 0.0);
    bits2(fight.balls[0].v, [2.559930753388882, -0.4547026917000837]);
}

/// A shielded enemy blocks the slam (a `block` event at its own centre, no damage) and is still knocked.
#[test]
fn a_shield_blocks_a_slam_but_not_its_knock() {
    let roster = testing::roster();
    let (fight, made) = slam_on(&roster, [C[0] + 0.3, C[1] + 0.4], 0.1, 5.0);
    assert_eq!(made, [108]);
    assert_eq!(
        fight.events[1..],
        [
            ev(13.4, "slam", vec![I(1), F(C[0]), F(C[1]), I(1)]),
            ev(
                13.4,
                "block",
                vec![I(0), I(1), F(1.3073529411764706), F(-1.130294117647059), s("slam")]
            ),
        ]
    );
    let a = &fight.balls[0];
    bits2(a.v, [2.0298350442023927, 1.6247368073968684]);
    assert_eq!((a.boost, a.hp, fight.balls[1].dealt), (1.6, 100.0, 0.0));
}

/// A `slow` that is an empty table is no slow, as in Python (`if W.get('slow')`): the ring lands without a `slow`
/// event, and the fight does not record `EXT`, `SLW` and `CRV` for it. A wave with a full `slow` slows, like a
/// shockwave.
#[test]
fn an_empty_slow_is_no_slow() {
    let roster = testing::roster();
    let flute = testing::fighter(&roster, "FLUTE");
    let with_slow = |slow: serde_json::Value| {
        let mut raw = flute.raw.clone();
        raw["abilities"]["low"]["slow"] = slow;
        FighterCfg::from_json(&raw).expect("loads")
    };
    let (empty, full) = (
        with_slow(serde_json::json!({})),
        with_slow(serde_json::json!({"mult": 0.5, "dur": 0.75})),
    );
    let fight_of = |a: &'_ FighterCfg, b: &'_ FighterCfg| {
        let setup = Setup {
            fighters: [a, b],
            rules: testing::episode_rules(),
            seed: 392,
        };
        Fight::new(&setup, false).uses_ext
    };
    assert!(!fight_of(flute, flute));
    assert!(!fight_of(&empty, flute));
    assert!(!fight_of(flute, &empty));
    assert!(fight_of(&full, flute));
    assert!(fight_of(flute, testing::fighter(&roster, "TROMBONE")));

    for (cfg, slowed) in [(&empty, false), (&full, true)] {
        let setup = Setup {
            fighters: [cfg, flute],
            rules: testing::episode_rules(),
            seed: 392,
        };
        let mut fight = Fight::new(&setup, false);
        fight.t = T0;
        place(&mut fight, 0, [-1.0, 0.5], [1.0, 2.4]);
        place(&mut fight, 1, [1.5, 0.5], [-2.4, 1.0]);
        trigger(&mut fight, 0, Slot::Low);
        assert_eq!(fight.waves[0].slow.is_some(), slowed);
        let (made, _) = run_effects(&mut fight, 40);
        let hit = ev(
            12.76667,
            "hit",
            vec![I(0), I(1), F(16.0), F(0.8999999999999999), F(0.5), s("wave"), I(0)],
        );
        let b = &fight.balls[1];
        if slowed {
            assert_eq!(made, [(32, 2)]);
            assert_eq!(
                fight.events[1..],
                [hit, ev(12.76667, "slow", vec![I(1), I(0), F(0.75)])]
            );
            assert_eq!((b.slow, b.slow_mult), (0.75, 0.5));
        } else {
            assert_eq!(made, [(32, 1)]);
            assert_eq!(fight.events[1..], [hit]);
            assert_eq!((b.slow, b.slow_mult), (0.0, 1.0));
        }
        bits2(b.v, [2.579922479451396, 0.3224903099312632]);
    }
}
