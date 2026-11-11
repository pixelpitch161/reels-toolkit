//! The thrust blade's unit tests. Every expected number is what `fight.py` gave on the reference PC (CPython 3.13,
//! numpy 2.5): see the note below the imports and the tests that drive the fight loop.

use bf_core::event::{Event, Value};

use super::*;
use crate::fight::{Key, Setup};
use crate::geom::{self, V2};
use crate::kinds::striker::{self, Basket};
use crate::roster::{FighterCfg, WeaponCfg};
use crate::schedule::Schedule;
use crate::testing;

// Every expected number below is what `fight.py` gave on the reference PC for the same hand-built state: a
// `Fight('FLUTE', 'TROMBONE', seed=392)` under the episode's rules (the TROMBONE's hits take 3.5 x 0.4 = 1.4) with
// `striker_setup(0)` called, its balls, clock, baskets and projectiles set as the test sets them, then the same
// methods called (`start_thrust(1)`, then `thrust_step(1, dt)` once per step at `T0 + i * dt`). The tests that
// drive the fight loop say how Python was driven.

/// The episode's beat, `60 / bpm`.
const BEAT: f64 = 0.44852941176470584;
/// `DT`.
const DT: f64 = 0.008333333333333333;
/// The time the hand-built thrusts start at.
const T0: f64 = 12.5;
/// The lance's owner in the hand-built fights: the TROMBONE, in the right corner.
const K: usize = 1;

use Value::{Float as F, Int as I};

fn s(x: &str) -> Value {
    Value::Str(x.into())
}

/// FLUTE (0) against TROMBONE (1) under the episode's rules, the FLUTE set up as a striker.
fn duel(roster: &[FighterCfg]) -> Fight<'_> {
    let mut fight = Fight::new(&testing::episode_setup(roster, 392), false);
    striker::setup(&mut fight, 0);
    fight
}

/// Puts the lance's owner and its enemy where the test wants them.
fn place(fight: &mut Fight, lance: V2, enemy: V2) {
    fight.balls[K].p = lance;
    fight.balls[1 - K].p = enemy;
}

/// Starts a thrust at `T0`.
fn thrust(fight: &mut Fight) {
    fight.t = T0;
    start_thrust(fight, K);
}

/// Step `i` of a thrust started at `T0`: the lance's step at `T0 + i * dt` (step 0 is the step that started it).
fn lance_step(fight: &mut Fight, i: usize) {
    fight.t = T0 + i as f64 * DT;
    thrust_step(fight, K, DT);
}

/// Puts a basket of the FLUTE's on the floor by hand, its cobra on it; returns its index.
fn basket(fight: &mut Fight, home: V2, t_drop: f64) -> usize {
    let st = fight.striker[0].as_mut().expect("set up as a striker");
    let id = st.next_bk;
    st.next_bk += 1;
    st.baskets.push(Basket {
        id,
        home,
        cp: home,
        t_drop,
        strike: None,
        down: None,
    });
    st.baskets.len() - 1
}

/// The FLUTE's basket `i`.
fn cobra<'f>(fight: &'f Fight, i: usize) -> &'f Basket {
    &fight.striker[0].as_ref().expect("set up as a striker").baskets[i]
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

fn kinds<'f>(fight: &'f Fight) -> Vec<&'f str> {
    fight.events.iter().map(|e| e.kind.as_str()).collect()
}

/// The lance's length beyond the ball at every step of an un-hasted thrust nobody is in reach of (`B.ext` after
/// `thrust_step`, steps 0 to 40): out over 0.08 s (ease-out), held 0.06 s, back over 0.18 s (smoothstep), then at
/// rest.
const EXT_PLAIN: [f64; 41] = [
    0.3,
    0.42836371527777084,
    0.5426215277777884,
    0.6427734375000039,
    0.7288194444444434,
    0.8007595486111063,
    0.8585937500000043,
    0.9023220486111121,
    0.9319444444444438,
    0.9474609374999992,
    0.95,
    0.95,
    0.95,
    0.95,
    0.95,
    0.95,
    0.95,
    0.9498338509119543,
    0.9442043895747595,
    0.9311446584616169,
    0.9114286440075208,
    0.8858303326474598,
    0.8551237108164396,
    0.8200827649494498,
    0.7814814814814849,
    0.7400938468475304,
    0.6966938474825985,
    0.6520554698216754,
    0.6069527002997557,
    0.5621595253518239,
    0.5184499314128945,
    0.47659790491795184,
    0.4373774323019828,
    0.4015624999999986,
    0.36992709444698524,
    0.34324520207793696,
    0.32229080932784493,
    0.3078379026317124,
    0.300660468424529,
    0.3,
    0.3,
];

/// The same under haste x 2 (`thr_k = 0.5`), steps 0 to 21: the whole thrust in 0.16 s.
const EXT_HASTE: [f64; 22] = [
    0.3,
    0.5426215277777655,
    0.7288194444444601,
    0.8585937500000043,
    0.9319444444444438,
    0.95,
    0.95,
    0.95,
    0.95,
    0.9442043895747615,
    0.9114286440075159,
    0.8551237108164396,
    0.7814814814814849,
    0.6966938474826081,
    0.606952700299746,
    0.5184499314128945,
    0.4373774323019909,
    0.3699270944469789,
    0.32229080932784493,
    0.300660468424529,
    0.3,
    0.3,
];

/// The same un-hasted with the lance grown to 2.47, steps 0 to 12 and 17 to 20.
const EXT_GROWN: [(usize, f64); 17] = [
    (0, 0.3),
    (1, 0.7285373263888659),
    (2, 1.1099826388889245),
    (3, 1.4443359375000135),
    (4, 1.731597222222219),
    (5, 1.9717664930555405),
    (6, 2.164843750000015),
    (7, 2.310828993055559),
    (8, 2.40972222222222),
    (9, 2.4615234374999977),
    (10, 2.47),
    (11, 2.47),
    (12, 2.47),
    (17, 2.4694453176599094),
    (18, 2.4506515775034283),
    (19, 2.4070521674795518),
    (20, 2.3412310115328006),
];

#[test]
fn the_closed_lance_starts_aimed_at_the_enemy() {
    let roster = testing::roster();
    let fight = duel(&roster);
    assert!(is_blade(&fight, 1) && !is_blade(&fight, 0));
    assert_eq!(main16(&fight, 1), 4, "every = 1 beat is 4 sixteenths");
    let b = &fight.balls[1];
    close(b.th, 2.356194490192345);
    assert_eq!((b.ext, b.thr, b.thr_hit, b.thr_k), (0.3, None, false, 1.0));
    assert_eq!((b.blade_len, b.frets), (0.95, 1));
    // in the left corner it aims the other way
    let ep = testing::episode_setup(&roster, 392);
    let swapped = Setup {
        fighters: [ep.fighters[1], ep.fighters[0]],
        ..ep
    };
    let fight = Fight::new(&swapped, false);
    assert!(is_blade(&fight, 0) && !is_blade(&fight, 1));
    close(fight.balls[0].th, 5.497787143782138);
    assert_eq!(fight.balls[0].ext, 0.3);
    // a bare fighter has no blade
    let bare = testing::bare_pair();
    let fight = Fight::new(&testing::bare_setup(&bare, 1), false);
    assert!(!is_blade(&fight, 0) && !is_blade(&fight, 1));
}

/// `start_thrust`: the lance locks on where the enemy is now (an angle from 0 to 2 pi), the thrust's clock starts,
/// its hit is armed again and its time scale is 1 over the haste factor; event `thrust` with the reach.
#[test]
fn start_thrust_locks_the_aim_and_starts_the_clock() {
    let roster = testing::roster();
    for (lance, enemy, haste, reach, th, thr_k) in [
        ([1.0, -0.5], [-1.2, 0.7], 0.0, 0.95, 2.642245931909663, 1.0),
        // the enemy below: atan2 is negative, the lance's angle is that plus 2 pi
        ([1.0, 0.5], [1.6, -1.0], 0.0, 0.95, 5.092895357497055, 1.0),
        // haste x 2: the whole thrust runs at half the time; the event carries the grown reach
        ([1.0, -0.5], [-1.2, 0.7], 1.0, 1.35, 2.642245931909663, 0.5),
        // a haste that has run out counts for nothing, whatever factor it left behind
        ([1.0, -0.5], [-1.2, 0.7], 0.0, 1.35, 2.642245931909663, 1.0),
    ] {
        let mut fight = duel(&roster);
        place(&mut fight, lance, enemy);
        let b = &mut fight.balls[K];
        (b.haste, b.haste_mult) = (haste, 2.0);
        b.thr_hit = true;
        b.blade_len = reach;
        thrust(&mut fight);
        let b = &fight.balls[K];
        close(b.th, th);
        assert!((0.0..2.0 * bf_core::dmath::PI).contains(&b.th));
        assert_eq!((b.thr, b.thr_hit, b.thr_k), (Some(T0), false, thr_k));
        assert_eq!((b.ext, b.blade_len, b.frets), (0.3, reach, 1), "nothing else moves");
        assert_eq!(fight.events.len(), 1);
        assert_event(&fight.events[0], 12.5, "thrust", &[I(1), F(reach)]);
    }
}

/// Idle, the closed lance turns toward the enemy at `aim_rate = 7.0` rad/s at most (0.0583 rad a step), the short
/// way round, and stays at `rest`. No event.
#[test]
fn an_idle_lance_turns_toward_the_enemy_at_aim_rate() {
    let roster = testing::roster();
    for (th0, lance, enemy, want) in [
        // within one step's turn: it lands on the aim and stays
        (
            2.62,
            [1.0, -0.5],
            [-1.2, 0.7],
            [
                2.642245931909663,
                2.642245931909663,
                2.642245931909663,
                2.642245931909663,
            ],
        ),
        (
            2.7,
            [1.0, -0.5],
            [-1.2, 0.7],
            [
                2.642245931909663,
                2.642245931909663,
                2.642245931909663,
                2.642245931909663,
            ],
        ),
        // far off: 7 / 120 rad a step, up or down
        (
            1.0,
            [1.0, -0.5],
            [-1.2, 0.7],
            [1.0583333333333333, 1.1166666666666667, 1.175, 1.2333333333333334],
        ),
        (
            5.0,
            [1.0, -0.5],
            [-1.2, 0.7],
            [
                4.941666666666666,
                4.883333333333333,
                4.824999999999999,
                4.766666666666666,
            ],
        ),
        // the short way goes through 0: the angle wraps
        (
            6.27,
            [0.0, 0.0],
            [1.0, 0.5],
            [
                0.04514802615374691,
                0.10348135948708025,
                0.16181469282041358,
                0.2201480261537469,
            ],
        ),
        (
            0.01,
            [0.0, 0.0],
            [1.0, -0.5],
            [
                6.234851973846253,
                6.17651864051292,
                6.118185307179586,
                6.059851973846253,
            ],
        ),
        // exactly opposite: `_wrap` gives -pi, so it turns downward
        (
            0.0,
            [0.0, 0.0],
            [-1.0, 0.0],
            [
                6.224851973846253,
                6.166518640512919,
                6.1081853071795855,
                6.049851973846252,
            ],
        ),
    ] {
        let mut fight = duel(&roster);
        place(&mut fight, lance, enemy);
        fight.balls[K].th = th0;
        fight.balls[K].ext = 0.9;
        for (i, want) in want.into_iter().enumerate() {
            lance_step(&mut fight, i);
            close(fight.balls[K].th, want);
            assert_eq!(fight.balls[K].ext, 0.3, "the idle lance is closed");
        }
        assert_eq!(fight.balls[K].thr, None);
        assert!(fight.events.is_empty());
    }
}

/// Brief: an un-hasted thrust takes `t_out + t_hold + t_back` = 0.08 + 0.06 + 0.18 = 0.32 s. Started at step 0, it
/// is on through step 38 (0.3167 s) and over at step 39 (0.325 s), the first step at or past 0.32 s; the lance's
/// length follows Python's at every step, and no event but the `thrust` is made when nobody is in reach.
#[test]
fn an_unhasted_thrust_takes_0_32_s() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    fight.rules.swat_cobras = false;
    place(&mut fight, [2.0, -2.0], [-2.0, 2.0]);
    thrust(&mut fight);
    for (i, want) in EXT_PLAIN.into_iter().enumerate() {
        lance_step(&mut fight, i);
        let b = &fight.balls[K];
        close(b.ext, want);
        assert_eq!(b.thr.is_some(), i <= 38, "step {i}");
        if b.thr.is_some() {
            assert_eq!(b.thr, Some(T0), "step {i}: the clock is not touched");
        }
        // on exactly while less than 0.32 s have gone by
        assert_eq!(b.thr.is_some(), (i as f64) * DT < 0.32, "step {i}");
    }
    close(fight.balls[K].th, 2.356194490192345);
    assert_eq!(kinds(&fight), ["thrust"]);
    assert!(!fight.balls[K].thr_hit);
    // the three phases: out through step 9 (u < 0.08), held through step 16 (u < 0.14), then back
    assert!(EXT_PLAIN[..10].windows(2).all(|w| w[0] < w[1]));
    assert!(EXT_PLAIN[10..17].iter().all(|&e| e == 0.95));
    assert!(EXT_PLAIN[16..40].windows(2).all(|w| w[0] > w[1]));
    // a grown lance goes further in the same time
    let mut fight = duel(&roster);
    fight.rules.swat_cobras = false;
    place(&mut fight, [2.0, -2.0], [-2.0, 2.0]);
    fight.balls[K].blade_len = 2.47;
    thrust(&mut fight);
    let mut grown = EXT_GROWN.into_iter().peekable();
    for i in 0..41 {
        lance_step(&mut fight, i);
        if let Some((_, want)) = grown.next_if(|&(step, _)| step == i) {
            close(fight.balls[K].ext, want);
        }
        assert_eq!(fight.balls[K].thr.is_some(), i <= 38, "step {i}");
    }
    assert!(grown.next().is_none());
}

/// Under haste the whole thrust runs at `1 / mult` of the time: 0.16 s with FLUTTER TONGUE's or POWER SLIDE's x 2
/// (on through step 19, over at step 20).
#[test]
fn a_hasted_thrust_takes_half_the_time() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    fight.rules.swat_cobras = false;
    place(&mut fight, [2.0, -2.0], [-2.0, 2.0]);
    (fight.balls[K].haste, fight.balls[K].haste_mult) = (1.0, 2.0);
    thrust(&mut fight);
    assert_eq!(fight.balls[K].thr_k, 0.5);
    for (i, want) in EXT_HASTE.into_iter().enumerate() {
        // the thrust keeps the time scale it started with, whatever the haste does meanwhile
        fight.balls[K].haste = 0.0;
        lance_step(&mut fight, i);
        close(fight.balls[K].ext, want);
        assert_eq!(fight.balls[K].thr.is_some(), i <= 19, "step {i}");
    }
    assert_eq!(kinds(&fight), ["thrust"]);
}

/// Brief: a thrust hits at most once. The enemy stands in the lance's way for the whole thrust: one `hit` (3.5 x the
/// episode's 0.4 for the right corner, sub = the fret count before the hit), on the first step the lance reaches
/// it; the enemy is knocked along the lance (`knock = 0.9`), and the lance grows by 0.08 and a fret.
#[test]
fn a_thrust_hits_once_knocks_and_grows() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-0.35, 0.3]);
    // the enemy flies along (2, 1) at the shared speed
    fight.balls[0].v = geom::scale(geom::div([2.0, 1.0], geom::norm([2.0, 1.0])), 2.6);
    thrust(&mut fight);
    close(fight.balls[K].th, 2.6066375798036967);
    let mut hits = Vec::new();
    for i in 0..42 {
        let n = fight.events.len();
        lance_step(&mut fight, i);
        if fight.events.len() > n {
            hits.push(i);
        }
    }
    // 1.569 apart: out of the closed lance's reach at step 0, in reach one step later
    assert_eq!(hits, [1], "one hit in the whole thrust");
    assert_eq!(kinds(&fight), ["thrust", "hit"]);
    assert_event(
        &fight.events[1],
        12.50833,
        "hit",
        &[
            I(1),
            I(0),
            F(1.4000000000000001),
            F(0.16617492018050461),
            F(-0.005881434181039591),
            s("blade"),
            I(1),
        ],
    );
    let (b, o) = (&fight.balls[K], &fight.balls[0]);
    close(o.hp, 98.6);
    close(b.dealt, 1.4000000000000001);
    close2(o.v, [-1.860375774333933, 1.8163154952409637]);
    assert_eq!(o.boost, 0.9);
    assert_eq!((b.blade_len, b.frets), (1.03, 2));
    assert_eq!(
        (b.thr, b.thr_hit, b.ext),
        (None, true, 0.3),
        "over, and its one hit spent"
    );
    close(b.th, 2.6066375798036967);
}

/// The lance is live while it goes out AND while it is held: an enemy that walks into the held lance is hit. While
/// the lance pulls back it hits nothing.
#[test]
fn a_thrust_hits_while_held_but_not_while_it_pulls_back() {
    let roster = testing::roster();
    // held: the enemy steps across the lance, near its tip, at step 12
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-1.2, 0.7]);
    thrust(&mut fight);
    let mut hits = Vec::new();
    for i in 0..42 {
        if i == 12 {
            fight.balls[0].p = [-0.7198772360277258, 0.21029732717618366];
        }
        let n = fight.events.len();
        lance_step(&mut fight, i);
        if fight.events.len() > n {
            hits.push(i);
            assert_eq!(fight.balls[K].ext, 0.95, "held at full length");
        }
    }
    assert_eq!(hits, [12]);
    assert_event(
        &fight.events[1],
        12.6,
        "hit",
        &[
            I(1),
            I(0),
            F(1.4000000000000001),
            F(-0.12223365161211397),
            F(0.26342120134646024),
            s("blade"),
            I(1),
        ],
    );
    assert_eq!((fight.balls[K].blade_len, fight.balls[K].frets), (1.03, 2));
    // the lance is idle again and has turned after the enemy for 3 steps
    close(fight.balls[K].th, 2.749935802742484);

    // pulling back: the enemy stands on the lance from step 17 on (u = 0.1417 >= t_out + t_hold): nothing
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-1.2, 0.7]);
    thrust(&mut fight);
    for i in 0..42 {
        if i == 17 {
            // 1.0 along the lance from its owner: inside the ball-to-tip span
            let th = fight.balls[K].th;
            let d = [bf_core::dmath::cos(th), bf_core::dmath::sin(th)];
            fight.balls[0].p = geom::add(fight.balls[K].p, d);
        }
        lance_step(&mut fight, i);
        if i == 16 {
            assert_eq!(fight.balls[K].ext, 0.95);
        }
        if i == 17 {
            close(fight.balls[K].ext, 0.9498338509119543);
        }
    }
    assert_eq!(kinds(&fight), ["thrust"]);
    let b = &fight.balls[K];
    assert_eq!((b.thr, b.thr_hit, b.blade_len, b.frets), (None, false, 0.95, 1));
    assert_eq!(fight.balls[0].hp, 100.0);
}

/// Brief: a thrust that meets a shield emits `block` (the shielded fighter first, then the lance's owner, the
/// contact point and `blade`) and pulls straight back: its clock is set to the start of the pull-back, so the lance
/// is home 0.18 s later (0.09 s under haste). No damage, no knock, no growth, and no second try.
#[test]
fn a_thrust_that_meets_a_shield_is_blocked_and_pulls_straight_back() {
    let roster = testing::roster();
    // (haste, thr after the block, the block's y, last step of the thrust, ext per step)
    let plain: &[f64] = &[
        0.3,
        0.42836371527777084,
        0.9459494709901928,
        0.9343138749174389,
        0.9158671982167328,
        0.8913834273230692,
        0.8616365486714356,
        0.8274005486968394,
        0.7894494138342694,
        0.7485571305187202,
        0.7054976851851766,
        0.6610450642686521,
        0.6159732542041318,
        0.5710562414266107,
        0.5270680123710738,
        0.48478255347253457,
        0.4449738511659782,
        0.4084158918863917,
        0.3758826620687853,
        0.3481481481481453,
        0.3259863365594666,
        0.3101712137377406,
        0.30147676611796914,
        0.3,
        0.3,
    ];
    let hasted: &[f64] = &[
        0.3,
        0.5426215277777655,
        0.9343138749174357,
        0.8913834273230633,
        0.8274005486968394,
        0.7485571305187202,
        0.6610450642686424,
        0.5710562414266012,
        0.48478255347253457,
        0.40841589188639915,
        0.3481481481481401,
        0.3101712137377406,
        0.3,
        0.3,
    ];
    for (haste, thr, hy, last, ext) in [
        (0.0, 12.368333333333332, -0.005881434181039591, 22, plain),
        (1.0, 12.438333333333333, -0.00588143418103948, 11, hasted),
    ] {
        let mut fight = duel(&roster);
        place(&mut fight, [1.0, -0.5], [-0.35, 0.3]);
        let v0 = geom::scale(geom::div([2.0, 1.0], geom::norm([2.0, 1.0])), 2.6);
        fight.balls[0].v = v0;
        fight.balls[0].shield = 1.0;
        (fight.balls[K].haste, fight.balls[K].haste_mult) = (haste, 2.0);
        thrust(&mut fight);
        for (i, want) in ext.iter().enumerate() {
            lance_step(&mut fight, i);
            let b = &fight.balls[K];
            close(b.ext, *want);
            assert_eq!(b.thr.is_some(), i <= last, "haste {haste}: step {i}");
            if i == 1 {
                // blocked on the step the lance reaches the enemy: B.thr = t - (t_out + t_hold) * thr_k
                close(b.thr.expect("still pulling back"), thr);
                assert!(b.thr_hit);
            }
        }
        assert_eq!(kinds(&fight), ["thrust", "block"], "haste {haste}");
        assert_event(
            &fight.events[1],
            12.50833,
            "block",
            &[I(0), I(1), F(0.16617492018050461), F(hy), s("blade")],
        );
        let (b, o) = (&fight.balls[K], &fight.balls[0]);
        assert_eq!((o.hp, o.v, o.boost), (100.0, v0, 0.0), "the shield takes it all");
        assert_eq!(
            (b.dealt, b.blade_len, b.frets),
            (0.0, 0.95, 1),
            "a blocked thrust grows nothing"
        );
        assert_eq!((b.thr, b.thr_hit, b.ext), (None, true, 0.3));
    }
}

/// Fret growth: a landed thrust adds `grow = 0.08` and a fret while the result stays within `max_len = 2.5` (plus
/// 1e-9): from `base = 0.95` and `frets0 = 1` that is 19 times, to 2.47 and 20 frets; then the lance stays as it is
/// while its thrusts still land. The hit's sub is the fret count before the hit. The lengths are Python's, bit for
/// bit (the sum is taken hit by hit).
#[test]
fn the_lance_grows_a_fret_a_hit_up_to_max_len() {
    let roster = testing::roster();
    let mut setup = testing::episode_setup(&roster, 392);
    // so that 24 hits do not knock the FLUTE out
    setup.rules.dmg_side = Some([1.0, 0.01]);
    let mut fight = Fight::new(&setup, false);
    let want = [
        (1.03, 2),
        (1.11, 3),
        (1.1900000000000002, 4),
        (1.2700000000000002, 5),
        (1.3500000000000003, 6),
        (1.4300000000000004, 7),
        (1.5100000000000005, 8),
        (1.5900000000000005, 9),
        (1.6700000000000006, 10),
        (1.7500000000000007, 11),
        (1.8300000000000007, 12),
        (1.9100000000000008, 13),
        (1.9900000000000009, 14),
        (2.0700000000000007, 15),
        (2.150000000000001, 16),
        (2.230000000000001, 17),
        (2.310000000000001, 18),
        (2.390000000000001, 19),
        (2.470000000000001, 20),
        (2.470000000000001, 20),
        (2.470000000000001, 20),
        (2.470000000000001, 20),
        (2.470000000000001, 20),
        (2.470000000000001, 20),
    ];
    for (n, (reach, frets)) in want.into_iter().enumerate() {
        // the enemy stands in the closed lance: every thrust lands on its first step
        place(&mut fight, [1.0, -0.5], [0.2, -0.5]);
        let frets_before = fight.balls[K].frets;
        fight.t = T0 + n as f64;
        start_thrust(&mut fight, K);
        thrust_step(&mut fight, K, DT);
        let b = &fight.balls[K];
        assert_eq!(
            (b.blade_len.to_bits(), b.frets),
            (f64::to_bits(reach), frets),
            "hit {n}"
        );
        let e = fight.events.last().expect("the thrust landed");
        assert_eq!(
            (e.kind.as_str(), e.s(7), e.i(8)),
            ("hit", "blade", frets_before),
            "hit {n}"
        );
        fight.balls[K].thr = None;
    }
    assert_eq!(fight.events.iter().filter(|e| e.kind == "hit").count(), 24);
    close(fight.balls[0].hp, 99.16000000000008);
    // the cap is `blade_len + grow <= max_len + 1e-9`: a step that lands on 2.5 (or a hair past it) is taken, a step
    // past it is not
    for (from, to, frets) in [(2.42, 2.5, 2), (2.4200000001, 2.5000000001, 2), (2.43, 2.43, 1)] {
        let mut fight = duel(&roster);
        place(&mut fight, [1.0, -0.5], [0.2, -0.5]);
        fight.balls[K].blade_len = from;
        thrust(&mut fight);
        lance_step(&mut fight, 0);
        assert_eq!(kinds(&fight), ["thrust", "hit"]);
        let b = &fight.balls[K];
        assert_eq!(
            (b.blade_len.to_bits(), b.frets),
            (f64::to_bits(to), frets),
            "from {from}"
        );
    }
}

/// Brief: a live lance through a risen cobra emits `swat` (the lance's owner, then the cobra's owner, the basket id
/// and where the lance touched), on the first step the lance comes within `head_r + 0.05` = 0.19 of the cobra. Only
/// under the rule `swat_cobras`, only a cobra that has risen, and only as far as the lance reaches.
#[test]
fn a_live_lance_through_a_risen_cobra_swats_it() {
    let roster = testing::roster();
    // The lance points along -x from (0.46, -0.5); its tip is at 0.4 - ext. (rule, cobra, t_drop, the step of the
    // swat with `ext` and `down` then)
    type Swat = Option<(usize, f64, f64)>;
    let cases: [(bool, V2, f64, Swat); 7] = [
        // on the line, 0.76 from the base: reached on the way out
        (
            true,
            [-0.3, -0.5],
            10.0,
            Some((2, 0.5426215277777884, 13.413725490196079)),
        ),
        // the rule is off
        (false, [-0.3, -0.5], 10.0, None),
        // dropped at 12.3: it rises until 12.7485, after the lance has gone back
        (true, [-0.3, -0.5], 12.3, None),
        // 0.2 beside the line: out of the 0.19
        (true, [-0.3, -0.3], 10.0, None),
        // 0.18 beside it: swatted when the tip has passed
        (
            true,
            [-0.3, -0.32],
            10.0,
            Some((3, 0.6427734375000039, 13.422058823529412)),
        ),
        // 0.2 beyond the tip of the full lance
        (true, [-0.75, -0.5], 10.0, None),
        // 0.18 beyond it: swatted as the lance reaches its full length
        (
            true,
            [-0.73, -0.5],
            10.0,
            Some((9, 0.9474609374999992, 13.47205882352941)),
        ),
    ];
    for (rule, at, t_drop, swat) in cases {
        let mut fight = duel(&roster);
        fight.rules.swat_cobras = rule;
        place(&mut fight, [1.0, -0.5], [-2.5, -0.5]);
        let bk = basket(&mut fight, at, t_drop);
        thrust(&mut fight);
        close(fight.balls[K].th, bf_core::dmath::PI);
        let mut seen = None;
        for i in 0..42 {
            let n = fight.events.len();
            lance_step(&mut fight, i);
            if fight.events.len() > n {
                assert_eq!(seen, None, "{at:?}: swatted once");
                seen = Some((i, fight.balls[K].ext, cobra(&fight, bk).down.expect("knocked flat")));
            }
        }
        match (seen, swat) {
            (Some((i, ext, down)), Some((want_i, want_ext, want_down))) => {
                assert_eq!(i, want_i, "{at:?}");
                close(ext, want_ext);
                // flat for 2 beats from the step of the swat
                close(down, want_down);
                close(down, T0 + i as f64 * DT + 2.0 * BEAT);
                assert_eq!(kinds(&fight), ["thrust", "swat"]);
                let t = bf_core::py::round_nd(T0 + i as f64 * DT, 5);
                assert_event(&fight.events[1], t, "swat", &[I(1), I(0), I(0), F(at[0]), F(at[1])]);
            }
            (None, None) => {
                assert_eq!(kinds(&fight), ["thrust"], "{at:?}");
                assert_eq!(cobra(&fight, bk).down, None);
            }
            other => panic!("{at:?} (rule {rule}, dropped at {t_drop}): {other:?}"),
        }
        // a swat is no hit: nothing is hurt and the lance does not grow
        assert_eq!(
            (fight.balls[0].hp, fight.balls[K].blade_len, fight.balls[K].thr_hit),
            (100.0, 0.95, false)
        );
    }
}

/// Brief: a swatted cobra cannot strike for 2 beats (`swat_beats`, the Python default): the strike grid passes it
/// over until `down`, then it lunges again.
#[test]
fn a_swatted_cobra_cannot_strike_for_2_beats() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-2.5, -0.5]);
    let bk = basket(&mut fight, [-0.3, -0.5], 10.0);
    thrust(&mut fight);
    for i in 0..5 {
        lance_step(&mut fight, i);
    }
    assert_eq!(kinds(&fight), ["thrust", "swat"]);
    assert_event(
        &fight.events[1],
        12.51667,
        "swat",
        &[I(1), I(0), I(0), F(-0.3), F(-0.5)],
    );
    let down = cobra(&fight, bk).down.expect("knocked flat");
    close(down, 13.413725490196079);
    close(down - (T0 + 2.0 * DT), 2.0 * BEAT);
    // the TROMBONE comes into the cobra's reach
    fight.balls[K].p = [0.6, -0.1];
    for (off, lunges) in [(-0.2, false), (-2e-9, false), (0.0, true)] {
        fight.t = down + off;
        let n = fight.events.len();
        striker::fire(&mut fight, 0);
        assert_eq!(fight.events.len() - n, usize::from(lunges), "{off} s from `down`");
        assert_eq!(cobra(&fight, bk).strike.is_some(), lunges);
    }
    assert_eq!(fight.events.last().expect("it lunged").kind, "strike");
}

/// The swat comes before the lance's own hit test (`fight.py` 753 to 764): a lance that reaches a cobra and the
/// enemy ball on the same step emits `swat`, then `hit`. A cobra swatted in mid-lunge has its bite spoiled: the
/// lunge runs on and bites nothing.
#[test]
fn a_swat_comes_before_the_lances_own_hit() {
    let roster = testing::roster();
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-0.4, -0.5]);
    let bk = basket(&mut fight, [0.1, -0.62], 10.0);
    // the cobra lunges at the TROMBONE two steps before the thrust
    fight.t = T0 - 2.0 * DT;
    striker::fire(&mut fight, 0);
    let lunge = cobra(&fight, bk).strike.clone().expect("in reach");
    close(lunge.t0, 12.483333333333333);
    close(lunge.t_hit, 12.603333333333332);
    close(lunge.aim, 0.13255153229667402);
    close(lunge.len, 0.5479647570252933);
    thrust(&mut fight);
    for i in 0..20 {
        let n = fight.events.len();
        lance_step(&mut fight, i);
        striker::step(&mut fight, 0, DT);
        assert_eq!(fight.events.len() - n, if i == 0 { 2 } else { 0 }, "step {i}");
    }
    assert_eq!(kinds(&fight), ["strike", "thrust", "swat", "hit"]);
    // the head was 0.1 along its lunge when the closed lance touched it
    assert_event(
        &fight.events[2],
        12.5,
        "swat",
        &[I(1), I(0), I(0), F(0.1754386049658725), F(-0.6099415193378837)],
    );
    assert_event(
        &fight.events[3],
        12.5,
        "hit",
        &[
            I(1),
            I(0),
            F(1.4000000000000001),
            F(0.19999999999999996),
            F(-0.4999999999999999),
            s("blade"),
            I(1),
        ],
    );
    // the lunge is still in the air at step 19 (it ends at 12.7953), its bite spent by the swat: the TROMBONE, well
    // inside the fang line at 12.6033, was not bitten
    let c = cobra(&fight, bk);
    assert!(c.strike.as_ref().is_some_and(|s| s.done));
    close(c.down.expect("knocked flat"), 13.397058823529411);
    assert_eq!((fight.balls[K].hp, fight.balls[0].blade_len), (100.0, 1.4));
}

/// A projectile at `p`, fired by `own`, numbered `id`.
fn key(own: usize, id: i64, p: V2) -> Key {
    Key {
        p,
        v: [0.0, 0.0],
        own,
        id,
        vol: 7,
        idx: id,
        dmg: 2.0,
    }
}

/// The parry (`parry = True`): the lance, while it goes out or is held, kills every ENEMY projectile within
/// `KEY_R + 0.04` = 0.15 of its segment (from 0.9 R out of the ball's centre to its tip): event `parry` per
/// projectile, in the order they were fired (the projectile's owner, then the lance's owner, where it was, its id).
/// No ported kind fires a projectile, so the projectiles are put in the fight by hand, as `spawn_key` would list
/// them.
#[test]
fn a_live_lance_parries_enemy_projectiles() {
    let roster = testing::roster();
    // the lance points along th = 2.44685437739309 from (1.0, -0.5); at step 4 its segment is 0.7888 long
    let on_the_line = [0.3546941251382042, 0.037754895718162956];
    let around: [(usize, V2); 9] = [
        // 0.3 along the lance: on it
        (0, on_the_line),
        // 0.149 beside it: inside the 0.15
        (0, [0.2593066495881967, -0.07671007494184606]),
        // 0.151 beside it: outside
        (0, [0.25802628078886775, -0.07824651750104081]),
        // on it, but the lance's own
        (1, on_the_line),
        // 0.149 on the other side
        (0, [0.4500816006882117, 0.15221986637817198]),
        // 0.1 behind the segment's start
        (0, [0.6619826369771546, -0.2183188641476289]),
        // 0.16 behind it
        (0, [0.7080759137529972, -0.25672992812749773]),
        // 0.6 and 0.7 along: before the tip (0.7888)
        (0, [0.12422774125899144, 0.22981021561750686]),
        (0, [0.04740561329925386, 0.2938286555839548]),
    ];
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-2.0, 2.0]);
    thrust(&mut fight);
    for i in 0..5 {
        if i == 4 {
            fight.keys = (0..).zip(around).map(|(id, (own, p))| key(own, id, p)).collect();
        }
        lance_step(&mut fight, i);
    }
    close(fight.balls[K].th, 2.44685437739309);
    close(fight.balls[K].ext, 0.7288194444444434);
    assert_eq!(
        kinds(&fight),
        ["thrust", "parry", "parry", "parry", "parry", "parry", "parry"]
    );
    for (e, id) in fight.events[1..].iter().zip([0usize, 1, 4, 5, 7, 8]) {
        let p = around[id].1;
        assert_event(e, 12.53333, "parry", &[I(0), I(1), F(p[0]), F(p[1]), I(id as i64)]);
    }
    // the rest fly on, in their order, untouched
    assert_eq!(
        fight.keys,
        vec![key(0, 2, around[2].1), key(1, 3, around[3].1), key(0, 6, around[6].1)]
    );

    // held (step 12, the segment 1.01 long): on the line, 0.09 beyond the tip (killed), 0.16 beyond it (not)
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-2.0, 2.0]);
    thrust(&mut fight);
    let beyond = [
        on_the_line,
        [-0.2598828985396966, 0.5499024154497468],
        [-0.3136583881115128, 0.5947153234262602],
    ];
    for i in 0..13 {
        if i == 12 {
            fight.keys = (0..).zip(beyond).map(|(id, p)| key(0, id, p)).collect();
        }
        lance_step(&mut fight, i);
    }
    assert_eq!(fight.balls[K].ext, 0.95);
    assert_eq!(kinds(&fight), ["thrust", "parry", "parry"]);
    assert_event(
        &fight.events[2],
        12.6,
        "parry",
        &[I(0), I(1), F(beyond[1][0]), F(beyond[1][1]), I(1)],
    );
    assert_eq!(fight.keys, vec![key(0, 2, beyond[2])]);

    // pulling back (step 20) the lance parries nothing; nor does a lance without the option; nor is its owner's
    // own projectile ever touched
    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-2.0, 2.0]);
    thrust(&mut fight);
    for i in 0..21 {
        if i == 20 {
            fight.keys = vec![key(0, 0, on_the_line)];
        }
        lance_step(&mut fight, i);
    }
    close(fight.balls[K].ext, 0.9114286440075208);
    assert_eq!((kinds(&fight), fight.keys.len()), (vec!["thrust"], 1));

    let mut plain = testing::fighter(&roster, "TROMBONE").clone();
    let WeaponCfg::Blade(w) = &mut plain.weapon else {
        panic!("the TROMBONE has a blade")
    };
    w.parry = false;
    let mut setup = testing::episode_setup(&roster, 392);
    setup.fighters[1] = &plain;
    let mut fight = Fight::new(&setup, false);
    place(&mut fight, [1.0, -0.5], [-2.0, 2.0]);
    thrust(&mut fight);
    for i in 0..5 {
        if i == 4 {
            fight.keys = vec![key(0, 0, on_the_line)];
        }
        lance_step(&mut fight, i);
    }
    assert_eq!((kinds(&fight), fight.keys.len()), (vec!["thrust"], 1));

    let mut fight = duel(&roster);
    place(&mut fight, [1.0, -0.5], [-2.0, 2.0]);
    thrust(&mut fight);
    for i in 0..5 {
        if i == 4 {
            fight.keys = vec![
                key(1, 0, on_the_line),
                key(1, 1, [0.39950703311471786, -0.0646746082281538]),
            ];
        }
        lance_step(&mut fight, i);
    }
    assert_eq!((kinds(&fight), fight.keys.len()), (vec!["thrust"], 2));
}

/// The TROMBONE with its lance changed.
fn trombone_with(roster: &[FighterCfg], change: impl FnOnce(&mut crate::roster::BladeCfg)) -> FighterCfg {
    let mut cfg = testing::fighter(roster, "TROMBONE").clone();
    let WeaponCfg::Blade(w) = &mut cfg.weapon else {
        panic!("the TROMBONE has a blade")
    };
    change(w);
    cfg
}

/// `rest` has two defaults in `fight.py`: a lance whose entry gives none is 0.0 long on the new ball
/// (`Ball.__init__`) and closes to 0.25 from its first step on (`thrust_step`): idle, at the start of a thrust and
/// after it.
#[test]
fn a_lance_without_rest_closes_to_a_quarter() {
    let roster = testing::roster();
    let no_rest = trombone_with(&roster, |w| w.rest = None);
    let mut setup = testing::episode_setup(&roster, 392);
    setup.fighters[1] = &no_rest;
    setup.rules.swat_cobras = false;
    let mut fight = Fight::new(&setup, false);
    assert_eq!(fight.balls[K].ext, 0.0, "before its first step");
    place(&mut fight, [2.0, -2.0], [-2.0, 2.0]);
    lance_step(&mut fight, 0);
    assert_eq!(fight.balls[K].ext, 0.25, "idle");
    thrust(&mut fight);
    let out = [
        0.25,
        0.38823784722221477,
        0.5112847222222336,
        0.6191406250000042,
        0.7118055555555544,
        0.7892795138888838,
        0.8515625000000047,
        0.89865451388889,
        0.9305555555555548,
        0.9472656249999992,
        0.95,
        0.95,
    ];
    let back = [
        (17, 0.9498210702128739),
        (18, 0.9437585733882026),
        (38, 0.2507112736879542),
    ];
    for i in 0..41 {
        lance_step(&mut fight, i);
        let ext = fight.balls[K].ext;
        if let Some(want) = out.get(i) {
            close(ext, *want);
        }
        if let Some((_, want)) = back.iter().find(|(step, _)| *step == i) {
            close(ext, *want);
        }
    }
    assert_eq!(
        (fight.balls[K].ext, fight.balls[K].thr),
        (0.25, None),
        "after the thrust"
    );
}

/// The phases are half-open: a step that falls exactly on the end of the hold is already in the pull-back (the
/// lance at full length, but no longer live: an enemy standing on it is not hit), and a step exactly on the end of
/// the thrust is idle again. With `t_out` = `t_hold` = 2 steps and `t_back` = 4 steps, started at t = 0, the steps
/// fall on the boundaries bit for bit. (The step exactly on the end of `t_out` is the first held one: full length
/// either way.)
#[test]
fn a_step_on_a_phase_boundary_belongs_to_the_later_phase() {
    let roster = testing::roster();
    let quick = trombone_with(&roster, |w| {
        (w.t_out, w.t_hold, w.t_back) = (2.0 * DT, 2.0 * DT, 4.0 * DT)
    });
    let mut setup = testing::episode_setup(&roster, 392);
    setup.fighters[1] = &quick;
    setup.rules.swat_cobras = false;
    let mut fight = Fight::new(&setup, false);
    place(&mut fight, [2.0, -2.0], [-2.0, 2.0]);
    fight.t = 0.0;
    start_thrust(&mut fight, K);
    let want = [
        0.3,
        0.7874999999999999,
        0.95,
        0.95,
        0.95,
        0.8484375,
        0.6249999999999998,
        0.40156249999999993,
        0.3,
        0.3,
    ];
    for (i, want) in want.into_iter().enumerate() {
        fight.t = i as f64 * DT;
        // on steps 4 (the hold just over) and 8 (the thrust just over) the enemy stands on the lance, 1.0 from
        // its owner
        fight.balls[0].p = if i == 4 || i == 8 {
            let th = fight.balls[K].th;
            geom::add(fight.balls[K].p, [bf_core::dmath::cos(th), bf_core::dmath::sin(th)])
        } else {
            [-2.0, 2.0]
        };
        thrust_step(&mut fight, K, DT);
        close(fight.balls[K].ext, want);
        assert_eq!(fight.balls[K].thr.is_some(), i < 8, "step {i}");
    }
    assert_eq!(kinds(&fight), ["thrust"], "nothing was hit");
    assert!(!fight.balls[K].thr_hit);
}

/// The lance at (1.0, -0.5) thrusts along -x at an enemy at (-2.5, -0.5) that flies along (1.0, 2.4); at step `at`
/// the enemy is put at `enemy`. Returns the fight after the thrust, and the steps on which an event was made.
fn thrust_along_x(roster: &[FighterCfg], at: usize, enemy: V2) -> (Fight<'_>, Vec<usize>) {
    let mut fight = duel(roster);
    fight.rules.swat_cobras = false;
    place(&mut fight, [1.0, -0.5], [-2.5, -0.5]);
    fight.balls[0].v = [1.0, 2.4];
    thrust(&mut fight);
    let mut steps = Vec::new();
    for i in 0..42 {
        if i == at {
            fight.balls[0].p = enemy;
        }
        let n = fight.events.len();
        lance_step(&mut fight, i);
        if fight.events.len() > n {
            steps.push(i);
        }
    }
    (fight, steps)
}

/// The hit test is the enemy's centre against the lance SEGMENT, within `R` = 0.6: beside it, beyond its tip, but
/// not behind its start (an enemy behind the lance's owner, on the line of the lance, is not hit). The contact point
/// is `R` from the enemy's centre toward the nearest point of the lance (the distance floored at 1e-6, for a centre
/// that lies on the lance), and the knock is along the lance, wherever the enemy stands.
#[test]
fn the_lance_hits_a_ball_within_r_of_its_segment() {
    let roster = testing::roster();
    // The held lance (step 12) lies on y = -0.5 from x = 0.46 to x = -0.55. (the enemy, the contact point or none)
    let cases: [(V2, Option<V2>); 9] = [
        // 0.599 and 0.601 beside it
        ([-0.2, -1.099], Some([-0.19999999999999984, -0.499])),
        ([-0.2, -1.101], None),
        // 0.599 and 0.601 beyond its tip
        ([-1.149, -0.5], Some([-0.549, -0.49999999999999983])),
        ([-1.151, -0.5], None),
        // beside the tip: the nearest point is the tip
        ([-0.5, -1.0], Some([-0.4999999999999999, -0.4])),
        // the centre 1e-4 from the lance: the contact point is a whole R from it
        ([-0.2, -0.5001], Some([-0.19999999999966694, 0.09989999999999999])),
        // the centre 1e-8 from the lance: the distance is taken as 1e-6, so the point is 0.006 from the centre
        ([-0.2, -0.50000001], Some([-0.19999999996669332, -0.4940000099032381])),
        // behind the owner, on the line of the lance and beside it
        ([2.3, -0.5], None),
        ([2.3, -0.2], None),
    ];
    for (enemy, contact) in cases {
        let at = if enemy[0] > 1.0 { 3 } else { 12 };
        let (fight, steps) = thrust_along_x(&roster, at, enemy);
        let (b, o) = (&fight.balls[K], &fight.balls[0]);
        match contact {
            Some(h) => {
                assert_eq!(steps, [12], "{enemy:?}");
                assert_event(
                    &fight.events[1],
                    12.6,
                    "hit",
                    &[I(1), I(0), F(1.4000000000000001), F(h[0]), F(h[1]), s("blade"), I(1)],
                );
                // knocked along the lance (-x), from a heading of (1.0, 2.4): the same wherever it stood
                close2(o.v, [-2.519186047620294, 0.6431964376900144]);
                assert_eq!((o.boost, b.thr_hit, b.frets), (0.9, true, 2));
            }
            None => {
                assert!(steps.is_empty(), "{enemy:?}: {:?}", fight.events);
                assert_eq!(
                    (o.hp, o.v, b.thr_hit, b.frets),
                    (100.0, [1.0, 2.4], false, 1),
                    "{enemy:?}"
                );
            }
        }
    }
}

/// `step` is `blade_step`: for a blade in thrust mode it is the thrust step.
#[test]
fn the_blade_step_of_a_thrust_blade_is_the_thrust_step() {
    let roster = testing::roster();
    let (mut by_step, mut by_thrust_step) = (duel(&roster), duel(&roster));
    for fight in [&mut by_step, &mut by_thrust_step] {
        place(fight, [1.0, -0.5], [-0.35, 0.3]);
        thrust(fight);
    }
    for i in 0..42 {
        by_step.t = T0 + i as f64 * DT;
        by_thrust_step.t = by_step.t;
        step(&mut by_step, K, DT);
        thrust_step(&mut by_thrust_step, K, DT);
        let (a, b) = (&by_step.balls[K], &by_thrust_step.balls[K]);
        assert_eq!(
            (a.ext, a.thr, a.thr_hit, a.th),
            (b.ext, b.thr, b.thr_hit, b.th),
            "step {i}"
        );
    }
    assert_eq!(by_step.events, by_thrust_step.events);
    assert_eq!(kinds(&by_step), ["thrust", "hit"]);
}

/// The steps of the `thrust` events of a fight loop in which both balls are put back in their corners after every
/// step (nobody is ever in reach) and the lance's owner `k` has haste x 2 switched on by hand after each step `n`
/// with `haste_from <= n < haste_to`. Python: `run()` on a `Fight` of the TROMBONE against a bag (weapon and
/// abilities of kind `none`, added to the roster table for the run), with the same two lines at the end of every
/// `abilities_step`.
fn thrust_steps(trombone_side: usize, steps: usize, haste: Option<(usize, usize)>) -> (Vec<i64>, Vec<Event>) {
    let roster = testing::roster();
    let bare = testing::bare_pair();
    let mut fighters = [&bare[0], &bare[1]];
    fighters[trombone_side] = testing::fighter(&roster, "TROMBONE");
    let setup = Setup {
        fighters,
        rules: testing::episode_rules(),
        seed: 392,
    };
    let mut fight = Fight::new(&setup, false);
    let mut sched = Schedule::new(&mut fight);
    let hold = |fight: &mut Fight, n: usize| {
        fight.balls[0].p = [-1.85, 1.85];
        fight.balls[1].p = [1.85, -1.85];
        if haste.is_some_and(|(from, to)| from <= n && n < to) {
            fight.balls[trombone_side].haste = 1.0;
            fight.balls[trombone_side].haste_mult = 2.0;
        }
    };
    hold(&mut fight, 0);
    let mut out = Vec::new();
    for n in 1..=steps {
        let seen = fight.events.len();
        sched.step(&mut fight);
        if fight.events[seen..].iter().any(|e| e.kind == "thrust") {
            out.push(sched.step);
        }
        hold(&mut fight, n);
    }
    (out, fight.events)
}

/// In the fight loop a thrust starts on the lance's own 16th-note grid, every `every = 1` beat: on the fight's first
/// step (16th 0), then on the first step at or after each beat. Haste divides the grid: x 2 is every 8th note.
#[test]
fn thrusts_start_on_the_16th_grid() {
    // no haste, 300 steps: the six beats
    let (steps, events) = thrust_steps(0, 300, None);
    assert_eq!(steps, [1, 54, 108, 162, 216, 270]);
    assert!(
        events.iter().all(|e| e.kind == "thrust"),
        "nobody is in reach: {events:?}"
    );
    let times: Vec<f64> = events.iter().map(|e| e.t).collect();
    assert_eq!(times, [3.59657, 4.03824, 4.48824, 4.93824, 5.38824, 5.83824]);
    for e in &events {
        assert_eq!(e.fields, [I(0), F(0.95)]);
    }
    // haste x 2 from step 150 on, the TROMBONE in the right corner: the 8th notes between the beats too
    let (steps, events) = thrust_steps(1, 330, Some((150, usize::MAX)));
    assert_eq!(steps, [1, 54, 108, 162, 189, 216, 243, 270, 297, 323]);
    assert!(events.iter().all(|e| e.fields == [I(1), F(0.95)]));
    // haste switched on after steps 100 to 199: its last second runs out at step 319, and beat 6 (step 323) is a
    // plain beat again
    let (steps, _) = thrust_steps(0, 330, Some((100, 200)));
    assert_eq!(steps, [1, 54, 108, 135, 162, 189, 216, 243, 270, 297, 323]);
}

/// The fight loop runs the lance's step after the balls have moved, in ball order: at the end of a 300-step loop
/// with nobody in reach the lance is where Python's is (mid pull-back of the thrust of step 270, still aimed along
/// the diagonal it locked on).
#[test]
fn the_fight_loop_steps_the_lance() {
    let roster = testing::roster();
    let bare = testing::bare_pair();
    let setup = Setup {
        fighters: [testing::fighter(&roster, "TROMBONE"), &bare[1]],
        rules: testing::episode_rules(),
        seed: 392,
    };
    let mut fight = Fight::new(&setup, false);
    let mut sched = Schedule::new(&mut fight);
    assert_eq!(
        sched.next16t, sched.k16_0,
        "the thrusters' counter starts on the fight's first 16th"
    );
    for _ in 0..300 {
        fight.balls[0].p = [-1.85, 1.85];
        fight.balls[1].p = [1.85, -1.85];
        sched.step(&mut fight);
    }
    let b = &fight.balls[0];
    close(b.th, 5.497787143782138);
    close(b.ext, 0.5184499314128945);
    close(b.thr.expect("mid thrust"), 5.838235294117647);
    // 2.5 s of 16ths (0.1121 s each) have gone by: numbers 0 to 22
    assert_eq!(sched.next16t - sched.k16_0, 23);
    // a fight without a thruster leaves the counter alone
    let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
    let mut sched = Schedule::new(&mut fight);
    for _ in 0..60 {
        sched.step(&mut fight);
    }
    assert_eq!(sched.next16t, sched.k16_0);
}
