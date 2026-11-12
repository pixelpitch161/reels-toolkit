//! The anchor's unit tests. Every expected number is what `fight.py` gave on the reference PC (CPython 3.13, numpy
//! 2.5) for the same hand-built state: a fight under the example episode's rules with `anchor_setup` called and the
//! clock at 12.5 s, its balls set as the test sets them, then the same methods called. The script that prints them
//! is `tools/studio_export/unit_checks/anchor_unit_vectors.py`; the numbers are its output, pasted. Everything is
//! compared to the bit.

use bf_core::event::Value::{Float as F, Int as I, Str as S};
use bf_core::event::{Event, Value};

use super::*;
use crate::abilities::trigger;
use crate::ball::Slot;
use crate::fight::Setup;
use crate::roster::FighterCfg;
use crate::rules::Rules;
use crate::testing;

const BCL: &str = "BASS CLARINET";
const T: f64 = 12.5;
const T_END: f64 = 13.397058823529411;
const OM: f64 = 10.50630985790685;
/// The peg of the plain case.
const C: V2 = [-0.0556078369859101, 0.39692991492402513];
/// Where the ball is, and how it moves, when the plain whirl's peg is pulled.
const PULL_P: V2 = [0.8915179304864358, 0.32308696949306503];
const PULL_V: V2 = [0.20209648223210142, 2.5921336793979997];

fn ev(t: f64, kind: &str, fields: Vec<Value>) -> Event {
    Event {
        t,
        kind: kind.into(),
        fields,
    }
}

/// `left` against `right`: ball `k` at `p` moving `v`, its enemy at `op` moving `ov`, the clock at [`T`].
fn duel<'a>(roster: &'a [FighterCfg], names: [&str; 2], k: usize, at: [V2; 4]) -> Fight<'a> {
    let ep = testing::episode_rules();
    let setup = Setup {
        fighters: names.map(|n| testing::fighter(roster, n)),
        rules: Rules::new(ep.bpm, ep.t_fight),
        seed: 392,
    };
    let mut f = testing::fight(&setup, false);
    for j in 0..2 {
        if is(&f, j) {
            super::setup(&mut f, j);
        }
    }
    let [p, v, op, ov] = at;
    f.balls[k].p = p;
    f.balls[k].v = v;
    f.balls[1 - k].p = op;
    f.balls[1 - k].v = ov;
    f.t = T;
    f
}

/// The plain state of the script: the anchorer at (-1, 0.5), the enemy at (0.5, -0.1).
const PLAIN: [V2; 4] = [[-1.0, 0.5], [1.0, -2.0], [0.5, -0.1], [-0.5, 2.0]];

/// The fighter's own weapon, as `anchor_tick` passes it.
fn own<'a>(f: &Fight<'a>, k: usize) -> &'a AnchorCfg {
    weapon(f, k)
}

/// `anchor_step` once per step from the clock until the peg is pulled (or `until` steps): each step's `(p, v)`.
fn whirl(f: &mut Fight, k: usize, until: Option<usize>) -> Vec<(V2, V2)> {
    let dt = f.rules.dt;
    let t0 = f.t;
    let mut seen = Vec::new();
    for i in 1..400 {
        f.t = t0 + i as f64 * dt;
        step(f, k, dt);
        seen.push((f.balls[k].p, f.balls[k].v));
        if f.balls[k].anc.is_none() || until.is_some_and(|n| i >= n) {
            break;
        }
    }
    seen
}

fn pull(k: i64, t: f64, p: V2, v: V2) -> Event {
    ev(t, "pull", vec![I(k), F(p[0]), F(p[1]), F(v[0]), F(v[1])])
}

/// `anchor_plant` (2584 to 2601): the peg an orbit ahead toward the led enemy, clipped inside the ropes; along the
/// ball's heading when it stands on its target; the angle from the heading when there is no orbit; the spin of the
/// corner.
#[test]
fn the_peg_goes_in_ahead_of_the_ball() {
    let roster = testing::roster();
    struct Case {
        names: [&'static str; 2],
        k: usize,
        at: [V2; 4],
        orbit: Option<f64>,
        c: V2,
        th: f64,
        r0: f64,
        om: f64,
    }
    let cases = [
        Case {
            names: [BCL, "FLUTE"],
            k: 0,
            at: PLAIN,
            orbit: None,
            c: C,
            th: 3.032883840686932,
            r0: 0.9500000000000001,
            om: OM,
        },
        // by the ropes
        Case {
            names: [BCL, "FLUTE"],
            k: 0,
            at: [[2.3, 2.2], [1.0, 2.0], [2.6, 2.6], [0.3, 0.2]],
            orbit: Some(1.8),
            c: [0.5799999999999998, 0.5799999999999998],
            th: 0.7554669850436401,
            r0: 2.3627949551325864,
            om: OM,
        },
        // on the enemy
        Case {
            names: [BCL, "FLUTE"],
            k: 0,
            at: [[0.4, -0.3], [1.5, -0.5], [0.4, -0.3], [0.0, 0.0]],
            orbit: None,
            c: [1.3012491331474179, -0.600416377715806],
            th: 2.819842099193151,
            r0: 0.9499999999993989,
            om: OM,
        },
        // from the right corner
        Case {
            names: ["FLUTE", BCL],
            k: 1,
            at: [[1.0, 0.5], [-1.0, 2.0], [-0.5, 0.1], [0.5, -2.0]],
            orbit: None,
            c: [0.18948311448860922, 0.004457490924426488],
            th: 0.5487529636982253,
            r0: 0.95,
            om: -OM,
        },
        // no orbit
        Case {
            names: [BCL, "FLUTE"],
            k: 0,
            at: PLAIN,
            orbit: Some(0.0),
            c: [-1.0, 0.5],
            th: 3.032883840686932,
            r0: 0.0,
            om: OM,
        },
    ];
    for (i, c) in cases.iter().enumerate() {
        let mut f = duel(&roster, c.names, c.k, c.at);
        let orbit = c.orbit.unwrap_or(f.balls[c.k].blade_len);
        let w = own(&f, c.k);
        anchor_plant(&mut f, c.k, w, orbit);
        let an = f.balls[c.k].anc.clone().expect("a whirl");
        assert_eq!(
            (an.c, an.th, an.r0, an.orbit, an.om),
            (c.c, c.th, c.r0, orbit, c.om),
            "case {i}"
        );
        assert_eq!(
            (an.t0, an.t_end, an.cool, an.own),
            (T, T_END, [0.0, 0.0], true),
            "case {i}"
        );
        assert_eq!(
            f.events,
            [ev(
                T,
                "anchor",
                vec![I(c.k as i64), F(c.c[0]), F(c.c[1]), F(orbit), F(T_END)]
            )],
            "case {i}"
        );
    }
}

/// `anchor_tick` (2611 to 2619): a peg every 16 16ths from the 16th (never on 0), every 8 under a haste of 2; none
/// while stunned, in shadow or whirling.
#[test]
fn the_grid_of_the_pegs() {
    let roster = testing::roster();
    let grid = |set: &dyn Fn(&mut Fight, i64), clear: bool| -> Vec<i64> {
        let mut f = duel(&roster, [BCL, "FLUTE"], 0, PLAIN);
        let mut out = Vec::new();
        for k16 in 0..41 {
            set(&mut f, k16);
            let had = f.balls[0].anc.is_some();
            tick(&mut f, 0, k16);
            if f.balls[0].anc.is_some() && !had {
                out.push(k16);
            }
            if clear {
                f.balls[0].anc = None;
            }
        }
        out
    };
    assert_eq!(grid(&|_, _| {}, true), [16, 32]);
    let hasted = |f: &mut Fight, k16: i64| {
        let on = (10..30).contains(&k16);
        f.balls[0].haste = if on { 1.0 } else { 0.0 };
        f.balls[0].haste_mult = if on { 2.0 } else { 1.0 };
    };
    assert_eq!(grid(&hasted, true), [16, 24, 32]);
    let stunned = |f: &mut Fight, k16: i64| {
        f.balls[0].stun = if k16 == 16 { 0.3 } else { 0.0 };
        f.balls[0].shadow = if k16 == 32 { 0.2 } else { 0.0 };
    };
    assert_eq!(grid(&stunned, true), [] as [i64; 0]);
    // a whirl left in place: the first peg, and no second while it whirls
    assert_eq!(grid(&|_, _| {}, false), [16]);
}

/// `anchor_step` (2620 to 2658) and `anchor_release`: the ball goes round the peg, smashes the enemy standing on the
/// circle (damage, a knock along the swing and outward, 0.8 s of rest for the arm, the orbit grown by 0.06), and is
/// flung off along the tangent when the whirl's time is up.
#[test]
fn a_whirl_smashes_grows_and_is_pulled() {
    let roster = testing::roster();
    let mut f = duel(&roster, [BCL, "FLUTE"], 0, PLAIN);
    let w = own(&f, 0);
    anchor_plant(&mut f, 0, w, 0.95);
    f.balls[1].p = [C[0], C[1] + 0.95];
    f.balls[1].v = [0.0, 0.0];
    let seen = whirl(&mut f, 0, None);
    assert_eq!(
        seen[0],
        (
            [-1.0053952415182943, 0.4170268348780225],
            [-0.21114446822624763, -9.978760771154347]
        )
    );
    assert_eq!(
        seen[1],
        (
            [-1.0035145803089842, 0.3338997161091427],
            [0.6622147991546281, -9.959001961751591]
        )
    );
    assert_eq!(
        seen[19],
        (
            [0.012303368047912863, -0.5506396435944267],
            [9.955459393714854, 0.7134961629091874]
        )
    );
    assert_eq!(
        seen[47],
        (
            [0.49527232753152417, 1.170900873255954],
            [-8.131578709256356, 5.787717702994867]
        )
    );
    assert_eq!((seen.len(), seen[107]), (108, (PULL_P, PULL_V)));
    let hit = vec![
        I(0),
        I(1),
        F(4.5),
        F(-0.0556078369859101),
        F(0.746929914924025),
        S("anchor".into()),
        I(0),
    ];
    assert_eq!(f.events[1..], [ev(12.9, "hit", hit), pull(0, 13.4, PULL_P, PULL_V)]);
    let (b, o) = (&f.balls[0], &f.balls[1]);
    assert!(b.anc.is_none());
    assert_eq!((b.boost, b.dealt, b.blade_len), (0.8, 4.5, 1.01));
    assert_eq!(
        (o.hp, o.v, o.boost),
        (95.5, [-1.3961703386397815, 2.193332711990039], 1.1)
    );
}

/// With a twin the ghost arm on the far side of the peg smashes too (sub-number 1, `twin_dmg` x the damage), each
/// arm rests on its own, and the orbit stops at `max_orbit`.
#[test]
fn a_twins_arm_smashes_from_the_far_side() {
    let roster = testing::roster();
    let mut f = duel(&roster, [BCL, "FLUTE"], 0, PLAIN);
    trigger(&mut f, 0, Slot::Unlock2);
    assert_eq!(f.twin.until[0], 16.088235294117645);
    let w = own(&f, 0);
    anchor_plant(&mut f, 0, w, 0.95);
    f.balls[1].p = [0.8770637492279614, 0.577550268896939];
    f.balls[1].v = [0.0, 0.0];
    f.balls[0].blade_len = 1.78;
    let dt = f.rules.dt;
    f.t = T + dt;
    step(&mut f, 0, dt);
    let at = |sub: i64, dmg: f64| {
        vec![
            I(0),
            I(1),
            F(dmg),
            F(0.2880080105665689),
            F(0.46347425586141444),
            S("anchor".into()),
            I(sub),
        ]
    };
    assert_eq!(f.events[3], ev(12.50833, "hit", at(1, 2.6999999999999997)));
    let an = f.balls[0].anc.as_ref().expect("still whirling");
    assert_eq!((an.cool, f.balls[0].blade_len), ([0.0, 0.8], 1.8));
    for i in 2..=32 {
        f.t = T + i as f64 * dt;
        step(&mut f, 0, dt);
    }
    assert_eq!(f.events[4], ev(12.76667, "hit", at(0, 4.5)));
    let an = f.balls[0].anc.as_ref().expect("still whirling");
    assert_eq!(an.cool, [0.8, 0.5416666666666676]);
    let mut i = 32;
    while f.balls[0].anc.is_some() {
        i += 1;
        f.t = T + i as f64 * dt;
        step(&mut f, 0, dt);
    }
    assert_eq!((i, f.events.len()), (108, 6));
    let (b, o) = (&f.balls[0], &f.balls[1]);
    assert_eq!((b.dealt, b.blade_len), (7.199999999999999, 1.8));
    assert_eq!(
        (o.hp, o.v, o.boost),
        (92.8, [1.8147944180347386, 1.8618596134661598], 1.1)
    );
}

/// An enemy sunk in shadow is not touched at all; a shielded one blocks: no growth, but the knock and the arm's rest
/// as for a smash.
#[test]
fn a_shadow_is_passed_over_and_a_shield_blocks() {
    let roster = testing::roster();
    for shield in [false, true] {
        let mut f = duel(&roster, [BCL, "FLUTE"], 0, PLAIN);
        let w = own(&f, 0);
        anchor_plant(&mut f, 0, w, 0.95);
        f.balls[1].p = [C[0], C[1] + 0.95];
        f.balls[1].v = [0.0, 0.0];
        if shield {
            f.balls[1].shield = 9.0;
        } else {
            f.balls[1].shadow = 9.0;
        }
        let seen = whirl(&mut f, 0, None);
        assert_eq!(seen.len(), 108);
        let (b, o) = (&f.balls[0], &f.balls[1]);
        assert_eq!((b.dealt, b.blade_len, o.hp), (0.0, 0.95, 100.0));
        if shield {
            let block = vec![
                I(1),
                I(0),
                F(-0.0556078369859101),
                F(0.746929914924025),
                S("anchor".into()),
            ];
            assert_eq!(f.events[1..], [ev(12.9, "block", block), pull(0, 13.4, PULL_P, PULL_V)]);
            assert_eq!((o.v, o.boost), ([-1.3961703386397815, 2.193332711990039], 1.1));
        } else {
            assert_eq!(f.events[1..], [pull(0, 13.4, PULL_P, PULL_V)]);
            assert_eq!((o.v, o.boost), ([0.0, 0.0], 0.0));
        }
    }
}

/// A copied whirl (what IDENTITY CRISIS does): BASS CLARINET's weapon and orbit planted on FLUTE's ball. It whirls,
/// smashes and is pulled on the settings it was given, and grows nobody's orbit.
#[test]
fn a_copied_whirl_runs_on_another_fighters_ball() {
    let roster = testing::roster();
    let mut f = duel(&roster, ["FLUTE", BCL], 0, PLAIN);
    f.balls[1].blade_len = 1.25;
    let flute_blade = f.balls[0].blade_len;
    let w = own(&f, 1);
    anchor_plant(&mut f, 0, w, 1.25);
    let c = [0.2426212671238026, 0.3643814670052963];
    let an = f.balls[0].anc.clone().expect("a whirl");
    assert_eq!(
        (an.c, an.th, an.r0, an.orbit, an.om, an.own),
        (c, 3.032883840686932, 1.2500000000000002, 1.25, OM, false)
    );
    assert_eq!(
        f.events,
        [ev(T, "anchor", vec![I(0), F(c[0]), F(c[1]), F(1.25), F(T_END)])]
    );
    f.balls[1].p = [c[0], c[1] + 1.25];
    f.balls[1].v = [0.0, 0.0];
    let seen = whirl(&mut f, 0, None);
    assert_eq!(
        seen[0],
        (
            [-1.0070990019977555, 0.3908247827342402],
            [-0.2778216687187469, -13.129948383097828]
        )
    );
    let hit = vec![
        I(0),
        I(1),
        F(4.5),
        F(0.2426212671238026),
        F(1.0143814670052964),
        S("anchor".into()),
        I(0),
    ];
    let (p, v) = ([1.4888393822189947, 0.2672196967014014], PULL_V);
    assert_eq!(f.events[1..], [ev(12.91667, "hit", hit), pull(0, 13.4, p, v)]);
    assert_eq!((seen.len(), seen[107]), (108, (p, v)));
    let (b, o) = (&f.balls[0], &f.balls[1]);
    assert_eq!(
        (b.boost, b.dealt, b.blade_len, o.blade_len),
        (0.8, 4.5, flute_blade, 1.25)
    );
    assert_eq!(
        (o.hp, o.v, o.boost),
        (95.5, [-1.6242165711715326, 2.0302513464876437], 1.1)
    );
    // a copied whirl alone adds no record key
    let g = duel(&roster, ["FLUTE", "TROMBONE"], 0, PLAIN);
    assert!(record_keys(&g).is_empty());
    assert_eq!(record_keys(&f), ["ANC"]);
}

/// `anchor_release` (2602 to 2609) mid-whirl: along the tangent in the whirl's direction at the shared speed; the
/// boost is the larger of the ball's and the weapon's `release`.
#[test]
fn a_pulled_peg_flings_the_ball_along_the_tangent() {
    let roster = testing::roster();
    let cases = [
        (
            0usize,
            [BCL, "FLUTE"],
            0.3,
            [-0.29815215682044494, 0.10485582697703966],
            [2.592688892688579, -0.19484431151375917],
            0.8,
        ),
        (
            1,
            ["FLUTE", BCL],
            1.1,
            [-1.1461754569952165, 1.2920618967113813],
            [0.6565066665843566, 2.515750185676292],
            1.1,
        ),
    ];
    for (k, names, boost, p, v, boost1) in cases {
        let mut f = duel(&roster, names, k, PLAIN);
        f.balls[1 - k].p = [2.4, 2.4];
        let w = own(&f, k);
        anchor_plant(&mut f, k, w, 0.95);
        f.balls[k].boost = boost;
        whirl(&mut f, k, Some(10));
        assert_eq!(f.t, 12.583333333333334);
        anchor_release(&mut f, k);
        let b = &f.balls[k];
        assert!(b.anc.is_none());
        assert_eq!((b.p, b.v, b.boost), (p, v, boost1));
        assert_eq!(f.events[1..], [pull(k as i64, 12.58333, p, v)]);
    }
}
