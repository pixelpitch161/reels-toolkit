//! The bell on hand-built state. Every expected string is a line of
//! `tools/studio_export/unit_checks/swallow_unit_vectors.py` (the kit's `fight.py` on the same state, each float as
//! Python's `float.hex()`), pasted as printed: the comparisons are bit for bit.

use bf_core::event::{Event, Value};

use super::*;
use crate::Rules;
use crate::fight::Setup;
use crate::geom::V2;
use crate::roster::FighterCfg;
use crate::testing::{episode_rules, fight, fighter, roster};

/// Python's `float.hex()` of a normal number or zero.
fn hx(x: f64) -> String {
    if x == 0.0 {
        return if x.is_sign_negative() { "-0x0.0p+0" } else { "0x0.0p+0" }.to_string();
    }
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let sign = if x < 0.0 { "-" } else { "" };
    format!("{sign}0x1.{:013x}p{exp:+}", bits & ((1 << 52) - 1))
}

fn vec(v: V2) -> String {
    format!("[{}, {}]", hx(v[0]), hx(v[1]))
}

fn py_bool(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

/// An event as the script prints it: the time, the kind, then every field.
fn line(e: &Event) -> String {
    let mut out = vec![hx(e.t), e.kind.clone()];
    for v in &e.fields {
        out.push(match v {
            Value::Int(n) => n.to_string(),
            Value::Float(x) => hx(*x),
            Value::Str(s) => s.clone(),
            other => panic!("an event field the script does not print: {other:?}"),
        });
    }
    out.join(" ")
}

fn lines(f: &Fight, from: usize) -> Vec<String> {
    f.events[from..].iter().map(line).collect()
}

/// The script's `show`: the swallower's aim and reach, the enemy's state, then the oom-pah and the cannonball.
fn show(f: &Fight, k: usize) -> [String; 2] {
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    let first = format!(
        "th {} reach {} O.p {} O.v {} boost {} held {} hp {} dealt {}",
        hx(b.th),
        hx(b.blade_len),
        vec(o.p),
        vec(o.v),
        hx(o.boost),
        py_bool(o.held),
        hx(o.hp),
        hx(b.dealt)
    );
    let sw = match &f.swallow.sw[k] {
        None => "None".to_string(),
        Some(s) => format!(
            "t0={} t_in={} t_pah={} caught={} done={}",
            hx(s.t0),
            hx(s.t_in),
            hx(s.t_pah),
            py_bool(s.caught),
            py_bool(s.done)
        ),
    };
    let cannon = match &f.swallow.cannon[k] {
        None => "None".to_string(),
        Some(c) => format!("t_end={} ev_i={}", hx(c.t_end), c.ev_i),
    };
    [first, format!("sw {sw} cannon {cannon}")]
}

/// The script's `new`: TUBA as fighter `k` against FLUTE under `rules`, the bell set up, at t = 12.5.
fn new_with<'a>(roster: &'a [FighterCfg], seed: u64, k: usize, rules: Rules) -> Fight<'a> {
    let names = if k == 0 { ["TUBA", "FLUTE"] } else { ["FLUTE", "TUBA"] };
    let mut f = fight(
        &Setup {
            fighters: [fighter(roster, names[0]), fighter(roster, names[1])],
            rules,
            seed,
        },
        false,
    );
    setup(&mut f, k);
    f.t = 12.5;
    f
}

fn new<'a>(roster: &'a [FighterCfg]) -> Fight<'a> {
    new_with(roster, 392, 0, episode_rules())
}

fn place(f: &mut Fight, k: usize, bp: V2, op: V2, ov: V2) {
    f.balls[k].p = bp;
    f.balls[1 - k].p = op;
    f.balls[1 - k].v = ov;
}

/// The gulp of cases E, F, H and I: the enemy just inside the catch margin.
const CLOSE: (V2, V2, V2) = ([-1.0, 0.5], [-0.3, 0.1], [1.3, -2.0]);
const OOM: &str = "0x1.9000000000000p+3 oom 0 0x1.9e5a5a5a5a5a6p+3 0x1.acb4b4b4b4b4bp+3 0x1.ccccccccccccdp+0";
const GULP_HIT: &str =
    "0x1.90443d46b26c0p+3 hit 0 1 0x1.8000000000000p+2 -0x1.ea8d296f09baap-2 0x1.9e582f5a5442ep-3 gulp 0";
const GULP: &str = "0x1.90443d46b26c0p+3 gulp 0 -0x1.ea8d296f09baap-2 0x1.9e582f5a5442ep-3 0x1.eb851eb851eb8p+0";
const SW_OPEN: &str = "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=False \
                       done=False cannon None";
const SW_CAUGHT: &str = "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=True \
                         done=False cannon None";

/// Case A: `swallow_setup`: no oom-pah, no cannonball, the bell faces the enemy, on either side.
#[test]
fn the_bell_starts_facing_the_enemy() {
    let roster = roster();
    for (k, th) in [(0, "0x1.5fdbbe9bba775p+2"), (1, "0x1.2d97c7f3321d2p+1")] {
        let f = new_with(&roster, 392, k, episode_rules());
        assert_eq!(hx(f.balls[k].th), th);
        assert_eq!(hx(f.balls[k].blade_len), "0x1.ccccccccccccdp+0");
        assert_eq!((&f.swallow.sw[k], &f.swallow.cannon[k]), (&None, &None));
        assert!(is(&f, k) && !is(&f, 1 - k));
    }
    let mut f = new(&roster);
    place(&mut f, 0, [0.5, 1.0], [-1.25, -0.75], [1.0, 0.0]);
    setup(&mut f, 0);
    assert_eq!(hx(f.balls[0].th), "0x1.f6a7a2955385ep+1");
}

/// Case B: `swallow_beat`: an oom every 4 beats, none while one runs; under haste every 2 beats and twice as fast.
#[test]
fn an_oom_on_the_grid_and_none_while_one_runs() {
    let roster = roster();
    let mut f = new(&roster);
    beat(&mut f, 0, 5);
    assert!(f.swallow.sw[0].is_none() && f.events.is_empty());
    beat(&mut f, 0, 8);
    assert_eq!(show(&f, 0)[1], SW_OPEN);
    assert_eq!(lines(&f, 0), [OOM]);
    f.t = 13.0;
    beat(&mut f, 0, 12);
    assert_eq!(f.events.len(), 1, "one oom-pah at a time");
    assert_eq!(f.swallow.sw[0].as_ref().map(|s| s.t0), Some(12.5));
    f.swallow.sw[0].as_mut().expect("an oom").done = true;
    beat(&mut f, 0, 12);
    assert_eq!(
        show(&f, 0)[1],
        "sw t0=0x1.a000000000000p+3 t_in=0x1.ae5a5a5a5a5a6p+3 t_pah=0x1.bcb4b4b4b4b4bp+3 caught=False done=False \
         cannon None"
    );
    assert_eq!(
        lines(&f, 1),
        ["0x1.a000000000000p+3 oom 0 0x1.ae5a5a5a5a5a6p+3 0x1.bcb4b4b4b4b4bp+3 0x1.ccccccccccccdp+0"]
    );
    let mut f = new(&roster);
    (f.balls[0].haste, f.balls[0].haste_mult) = (1.0, 2.0);
    beat(&mut f, 0, 5);
    assert!(f.swallow.sw[0].is_none());
    beat(&mut f, 0, 6);
    assert_eq!(
        show(&f, 0)[1],
        "sw t0=0x1.9000000000000p+3 t_in=0x1.972d2d2d2d2d3p+3 t_pah=0x1.9e5a5a5a5a5a6p+3 caught=False done=False \
         cannon None"
    );
    assert_eq!(
        lines(&f, 0),
        ["0x1.9000000000000p+3 oom 0 0x1.972d2d2d2d2d3p+3 0x1.9e5a5a5a5a5a6p+3 0x1.ccccccccccccdp+0"]
    );
}

/// Case C: the aim turns at most `aim_rate * dt` a step, lands on the enemy when closer, and wraps through 0.
#[test]
fn the_aim_turns_at_its_rate() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    f.balls[0].th = 1.0;
    for th in ["0x1.e666666666666p-1", "0x1.cccccccccccccp-1", "0x1.b333333333332p-1"] {
        f.t += dt;
        step(&mut f, 0, dt);
        assert_eq!(hx(f.balls[0].th), th);
        assert_eq!((f.balls[1].p, f.balls[1].v), ([1.2, -0.7], [1.3, -2.0]));
        assert!(f.events.is_empty() && f.swallow.sw[0].is_none());
    }
    f.balls[0].th = 5.79;
    step(&mut f, 0, dt);
    assert_eq!(hx(f.balls[0].th), "0x1.722a695084094p+2");
    f.balls[0].th = 0.01;
    place(&mut f, 0, [-1.0, 0.5], [1.2, 0.45], [1.3, -2.0]);
    step(&mut f, 0, dt);
    assert_eq!(hx(f.balls[0].th), "0x1.90ab689557ef5p+2");
}

/// Case D: while the bell inhales, an enemy whose surface is within the reach is steered toward it, its speed
/// kept; one further out is left alone.
#[test]
fn the_suction_steers_an_enemy_in_reach() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.630ef1ceedaa8p+2 reach 0x1.ccccccccccccdp+0 O.p [0x1.3333333333333p+0, -0x1.6666666666666p-1] \
             O.v [0x1.34726641ceb1fp+0, -0x1.0783cf22ce1ecp+1] boost 0x0.0p+0 held False hp 0x1.9000000000000p+6 \
             dealt 0x0.0p+0",
            SW_OPEN
        ]
    );
    assert_eq!(hx(geom::norm(f.balls[1].v)), "0x1.3153df622de43p+1");
    let mut f = new(&roster);
    place(&mut f, 0, [-2.5, 0.5], [2.4, -0.7], [1.3, -2.0]);
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.balls[1].v, [1.3, -2.0]);
    assert_eq!(show(&f, 0)[1], SW_OPEN);
}

/// Case E: a gulp (the hit, the held enemy at the tuba's centre, the reach grown), the ride inside, the pah (one
/// draw from the generator: the spit's jitter), then the cannonball when the spat enemy hits a rope.
#[test]
fn a_gulp_the_ride_the_spit_and_the_cannonball() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, CLOSE.0, CLOSE.1, CLOSE.2);
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.630ef1ceedaa8p+2 reach 0x1.eb851eb851eb8p+0 O.p [-0x1.0000000000000p+0, 0x1.0000000000000p-1] \
             O.v [0x1.355586233b4a7p+0, -0x1.074137bfecd69p+1] boost 0x0.0p+0 held True hp 0x1.7800000000000p+6 \
             dealt 0x1.8000000000000p+2",
            SW_CAUGHT
        ]
    );
    assert_eq!(lines(&f, 0), [OOM, GULP_HIT, GULP]);
    // the tuba moves on: the enemy rides inside
    f.balls[0].p = [-0.9, 0.6];
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.balls[1].p, [-0.9, 0.6]);
    assert!(f.balls[1].held);
    assert_eq!(hx(f.balls[0].th), "0x1.5fdbbe9bba775p+2");
    assert_eq!(f.events.len(), 3);
    f.t = f.swallow.sw[0].as_ref().expect("an oom").t_pah - 0.5 * dt;
    step(&mut f, 0, dt);
    assert_eq!(show(&f, 0)[1], SW_CAUGHT);
    assert_eq!(hx(f.balls[0].th), "0x1.630ef1ceedaa8p+2");
    assert_eq!(f.events.len(), 3);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.6642250220ddbp+2 reach 0x1.eb851eb851eb8p+0 O.p [0x1.56e939a8b0844p-3, 0x1.2f4efadc7a640p-7] \
             O.v [0x1.232ecd7352440p+1, -0x1.424b573bac637p+0] boost 0x1.199999999999ap+1 held False hp \
             0x1.5000000000000p+6 dealt 0x1.0000000000000p+4",
            "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=True done=True \
             cannon t_end=0x1.c9a3a3a3a3a3ap+3 ev_i=5"
        ]
    );
    assert_eq!(
        lines(&f, 3),
        [
            "0x1.acd6e04c05921p+3 pah 0 1 -0x1.02cc95fa0418cp-1",
            "0x1.acd6e04c05921p+3 hit 0 1 0x1.4000000000000p+3 0x1.56e939a8b0844p-3 0x1.2f4efadc7a640p-7 spit 0"
        ]
    );
    // flying: nothing new, the cannonball waits
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(hx(f.balls[0].th), "0x1.697558355410ep+2");
    assert_eq!(f.swallow.cannon[0].as_ref().map(|c| c.ev_i), Some(5));
    // the tuba's own rope hit is not the cannonball
    let rope = |x: f64, y: f64, s: f64, ax: i64, sign: i64| {
        vec![
            Value::Int(ax),
            Value::Int(sign),
            Value::Float(x),
            Value::Float(y),
            Value::Float(s),
        ]
    };
    f.ev("rope", 0, rope(0.25, -3.0, 4.0, 1, -1));
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.swallow.cannon[0].as_ref().map(|c| c.ev_i), Some(6));
    assert_eq!(hx(f.balls[1].hp), "0x1.5000000000000p+6");
    f.ev("rope", 1, rope(3.25, -1.5, 5.0, 0, 1));
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.6fdbbe9bba774p+2 reach 0x1.eb851eb851eb8p+0 O.p [0x1.56e939a8b0844p-3, 0x1.2f4efadc7a640p-7] \
             O.v [0x1.232ecd7352440p+1, -0x1.424b573bac637p+0] boost 0x1.199999999999ap+1 held False hp \
             0x1.2800000000000p+6 dealt 0x1.a000000000000p+4",
            "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=True done=True \
             cannon None"
        ]
    );
    assert_eq!(
        lines(&f, 6),
        [
            "0x1.ad5f5ad96a6a0p+3 rope 1 0 1 0x1.a000000000000p+1 -0x1.8000000000000p+0 0x1.4000000000000p+2",
            "0x1.ada3ad18d25eep+3 hit 0 1 0x1.4000000000000p+3 0x1.a000000000000p+1 -0x1.8000000000000p+0 cannon 0"
        ]
    );
    // one draw was taken: the generator stands where a fresh one stands after its first draw
    assert_eq!(hx(f.rng.uniform(0.0, 1.0)), "0x1.9ae5e6ad6ce20p-4");
    let mut g = new(&roster);
    assert_eq!(hx(g.rng.uniform(-1.0, 1.0)), "0x1.8364f4f052cc8p-3");
    assert_eq!(hx(g.rng.uniform(0.0, 1.0)), "0x1.9ae5e6ad6ce20p-4");
}

/// Case F: the cannonball holds through `t_end` and is gone the step after; a later rope hit is only a rope hit.
#[test]
fn the_cannonball_times_out() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, CLOSE.0, CLOSE.1, CLOSE.2);
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    f.t = f.swallow.sw[0].as_ref().expect("an oom").t_pah;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.6642250220ddbp+2 reach 0x1.eb851eb851eb8p+0 O.p [0x1.0f4acd96a5388p-3, 0x1.7a9cb742af8c0p-5] \
             O.v [0x1.34ec0fd7bcd39p+1, -0x1.ef24d4b47ea69p-1] boost 0x1.199999999999ap+1 held False hp \
             0x1.5000000000000p+6 dealt 0x1.0000000000000p+4",
            "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=True done=True \
             cannon t_end=0x1.c981818181818p+3 ev_i=5"
        ]
    );
    let t_end = f.swallow.cannon[0].as_ref().expect("a cannonball").t_end;
    f.t = t_end;
    step(&mut f, 0, dt);
    assert!(f.swallow.cannon[0].is_some());
    f.t = t_end + dt;
    step(&mut f, 0, dt);
    assert!(f.swallow.cannon[0].is_none());
    let n = f.events.len();
    let data = vec![
        Value::Int(0),
        Value::Int(1),
        Value::Float(3.25),
        Value::Float(-1.5),
        Value::Float(5.0),
    ];
    f.ev("rope", 1, data);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.events.len(), n + 1);
    assert_eq!(hx(f.balls[1].hp), "0x1.5000000000000p+6");
}

/// Case G: a pah with nothing caught: the event carries 0 and the bell's aim, and no draw is taken.
#[test]
fn a_pah_with_nothing_caught() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, [-2.5, 0.5], [2.4, -0.7], [1.3, -2.0]);
    beat(&mut f, 0, 8);
    f.t = f.swallow.sw[0].as_ref().expect("an oom").t_pah;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0)[1],
        "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=False done=True \
         cannon None"
    );
    assert_eq!(lines(&f, 1), ["0x1.acb4b72c5197ap+3 pah 0 0 0x1.630ef1ceedaa8p+2"]);
    assert_eq!((f.balls[1].p, f.balls[1].v), ([2.4, -0.7], [1.3, -2.0]));
    assert_eq!(hx(f.rng.uniform(-1.0, 1.0)), "0x1.8364f4f052cc8p-3");
    // done: the next steps only turn the bell
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.events.len(), 2);
}

/// Case H: a gulp on a shielded enemy is a `block` (the enemy, the swallower, where, `swallow`) and ends the
/// inhale at once; nothing is caught and the enemy is still steered in that step.
#[test]
fn a_gulp_on_a_shield_is_blocked_and_ends_the_inhale() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, CLOSE.0, CLOSE.1, CLOSE.2);
    f.balls[1].shield = 0.75;
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    let blocked = "sw t0=0x1.9000000000000p+3 t_in=0x1.9044444444444p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=False \
                   done=False cannon None";
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.630ef1ceedaa8p+2 reach 0x1.ccccccccccccdp+0 O.p [-0x1.3333333333333p-2, 0x1.999999999999ap-4] \
             O.v [0x1.355586233b4a7p+0, -0x1.074137bfecd69p+1] boost 0x0.0p+0 held False hp 0x1.9000000000000p+6 \
             dealt 0x0.0p+0",
            blocked
        ]
    );
    assert_eq!(
        lines(&f, 1),
        ["0x1.90443d46b26c0p+3 block 1 0 -0x1.ea8d296f09baap-2 0x1.9e582f5a5442ep-3 swallow"]
    );
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.6642250220ddbp+2 reach 0x1.ccccccccccccdp+0 O.p [-0x1.3333333333333p-2, 0x1.999999999999ap-4] \
             O.v [0x1.355586233b4a7p+0, -0x1.074137bfecd69p+1] boost 0x0.0p+0 held False hp 0x1.9000000000000p+6 \
             dealt 0x0.0p+0",
            blocked
        ]
    );
    assert_eq!(f.events.len(), 2);
}

/// Case I (#56): a rolling enemy is steered but not gulped; with the rule `swallow_rollers` it is gulped like any
/// other.
#[test]
fn a_rolling_enemy_is_gulped_only_under_the_rule() {
    let roster = roster();
    for on in [false, true] {
        let mut rules = episode_rules();
        if on {
            rules.swallow_rollers = Some(true);
        }
        let mut f = new_with(&roster, 392, 0, rules);
        let dt = f.rules.dt;
        place(&mut f, 0, CLOSE.0, CLOSE.1, CLOSE.2);
        f.balls[1].rolling = true;
        beat(&mut f, 0, 8);
        f.t += dt;
        step(&mut f, 0, dt);
        assert_eq!(vec(f.balls[1].v), "[0x1.355586233b4a7p+0, -0x1.074137bfecd69p+1]");
        if on {
            assert_eq!(show(&f, 0)[1], SW_CAUGHT);
            assert_eq!(lines(&f, 1), [GULP_HIT, GULP]);
            assert!(f.balls[1].held);
        } else {
            assert_eq!(show(&f, 0)[1], SW_OPEN);
            assert_eq!(f.events.len(), 1);
            assert_eq!(f.balls[1].p, CLOSE.1);
            assert!(!f.balls[1].held);
        }
    }
    // a rule switched off is the default
    let mut rules = episode_rules();
    rules.swallow_rollers = Some(false);
    let mut f = new_with(&roster, 392, 0, rules);
    let dt = f.rules.dt;
    place(&mut f, 0, CLOSE.0, CLOSE.1, CLOSE.2);
    f.balls[1].rolling = true;
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.events.len(), 1);
}

/// Case J: a tuba within 0.3 of the ring's centre spits where the bell points (plus the jitter); and the whole
/// kind with the tuba as the right-hand fighter (the handicap of its side, the events' `who`).
#[test]
fn a_spit_from_the_middle_and_the_tuba_on_the_right() {
    let roster = roster();
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, 0, [0.1, -0.2], [0.6, 0.1], [1.3, -2.0]);
    beat(&mut f, 0, 8);
    f.t += dt;
    step(&mut f, 0, dt);
    assert_eq!(f.events.len(), 3);
    f.t = f.swallow.sw[0].as_ref().expect("an oom").t_pah;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f, 0),
        [
            "th 0x1.6642250220ddbp+2 reach 0x1.eb851eb851eb8p+0 O.p [0x1.1adce93265847p+0, -0x1.c89298d6742e8p-1] \
             O.v [0x1.1221f44aaf526p+1, -0x1.7965a70e7bccdp+0] boost 0x1.199999999999ap+1 held False hp \
             0x1.5000000000000p+6 dealt 0x1.0000000000000p+4",
            "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=True done=True \
             cannon t_end=0x1.c981818181818p+3 ev_i=5"
        ]
    );
    assert_eq!(
        lines(&f, 3),
        [
            "0x1.acb4b72c5197ap+3 pah 0 1 -0x1.34aa6aaaad7e1p-1",
            "0x1.acb4b72c5197ap+3 hit 0 1 0x1.4000000000000p+3 0x1.1adce93265847p+0 -0x1.c89298d6742e8p-1 spit 0"
        ]
    );
    let mut f = new_with(&roster, 7, 1, episode_rules());
    place(&mut f, 1, [3.0, -3.1], [2.6, -2.8], [1.3, -2.0]);
    beat(&mut f, 1, 8);
    f.t += dt;
    step(&mut f, 1, dt);
    assert_eq!(
        lines(&f, 0),
        [
            "0x1.9000000000000p+3 oom 1 0x1.9e5a5a5a5a5a6p+3 0x1.acb4b4b4b4b4bp+3 0x1.ccccccccccccdp+0",
            "0x1.90443d46b26c0p+3 hit 1 0 0x1.3333333333334p+0 0x1.428f5c28f5c2ap+1 -0x1.5eb851eb851ebp+1 gulp 0",
            "0x1.90443d46b26c0p+3 gulp 1 0x1.428f5c28f5c2ap+1 -0x1.5eb851eb851ebp+1 0x1.eb851eb851eb8p+0"
        ]
    );
    f.balls[1].p = [3.05, -3.1];
    f.t = f.swallow.sw[1].as_ref().expect("an oom").t_pah;
    step(&mut f, 1, dt);
    assert_eq!(
        show(&f, 1),
        [
            "th 0x1.3a6494bffee9ep+1 reach 0x1.eb851eb851eb8p+0 O.p [0x1.2ddac3fb6fa56p+1, -0x1.0c2b8c1fb046dp+1] \
             O.v [-0x1.796825958d0cdp+0, 0x1.12211885e8da7p+1] boost 0x1.199999999999ap+1 held False hp \
             0x1.8333333333333p+6 dealt 0x1.999999999999ap+1",
            "sw t0=0x1.9000000000000p+3 t_in=0x1.9e5a5a5a5a5a6p+3 t_pah=0x1.acb4b4b4b4b4bp+3 caught=True done=True \
             cannon t_end=0x1.c981818181818p+3 ev_i=5"
        ]
    );
    assert_eq!(
        lines(&f, 3),
        [
            "0x1.acb4b72c5197ap+3 pah 1 1 0x1.163b0a5fb6ecdp+1",
            "0x1.acb4b72c5197ap+3 hit 1 0 0x1.0000000000000p+1 0x1.2ddac3fb6fa56p+1 -0x1.0c2b8c1fb046dp+1 spit 0"
        ]
    );
}
