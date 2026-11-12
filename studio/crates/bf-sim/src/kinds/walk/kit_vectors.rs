//! The kind and its two abilities against the kit on hand-built state: every float as `float.hex()`, the numbers
//! `tools/studio_export/unit_checks/walk_unit_vectors.py` printed (its sections A to J).

use bf_core::event::{Event, Value};

use super::*;
use crate::abilities::{self, slap, stampede};
use crate::fight::{Fight, Setup, Wave};
use crate::geom::V2;
use crate::kinds::striker;
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

/// How many representable steps `x` is from the number Python printed as `hex` (both finite, same sign).
fn ulps(x: f64, hex: &str) -> u64 {
    let want = hex.parse::<HexF>().expect("a float.hex()").0;
    x.to_bits().abs_diff(want.to_bits())
}

/// The lines `got` against the script's `want`, word by word: every number (a `float.hex()`, with or without the
/// brackets and commas of a vector) within one representable step of the kit's, every other word equal. Only for
/// THE ELEPHANT's turn, whose `math.atan2`, `math.cos` and `math.sin` are the C runtime's in the kit and `dmath`'s
/// here (ruling M3-R38; the audit that makes them the same is not merged yet).
fn near<S: AsRef<str>, W: AsRef<str>>(got: &[S], want: impl AsRef<[W]>) {
    let want = want.as_ref();
    assert_eq!(got.len(), want.len(), "lines");
    let words = |l: &str| -> Vec<String> {
        l.split([' ', ',', '[', ']'])
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect()
    };
    for (g, w) in got.iter().zip(want) {
        let (g, w) = (g.as_ref(), w.as_ref());
        let (gw, ww) = (words(g), words(w));
        assert_eq!(gw.len(), ww.len(), "{g} against the kit's {w}");
        for (a, b) in gw.iter().zip(&ww) {
            match (a.parse::<HexF>(), b.parse::<HexF>()) {
                (Ok(x), Ok(_)) => assert!(ulps(x.0, b) <= 1, "{g} against the kit's {w}"),
                _ => assert_eq!(a, b, "{g} against the kit's {w}"),
            }
        }
    }
}

/// A number read back from Python's `float.hex()` of a normal number (the only ones the script prints here).
struct HexF(f64);

impl std::str::FromStr for HexF {
    type Err = String;
    fn from_str(t: &str) -> Result<HexF, String> {
        let (neg, t) = t.strip_prefix('-').map_or((false, t), |r| (true, r));
        let body = t
            .strip_prefix("0x1.")
            .ok_or_else(|| format!("{t}: not a normal number"))?;
        let (mant, exp) = body.split_once('p').ok_or_else(|| format!("{t}: no exponent"))?;
        let mant = u64::from_str_radix(mant, 16).map_err(|e| e.to_string())?;
        let exp: i64 = exp.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
        let bits = (((exp + 1023) as u64) << 52) | mant | (u64::from(neg) << 63);
        Ok(HexF(f64::from_bits(bits)))
    }
}

fn vec(v: &[f64]) -> String {
    let parts: Vec<String> = v.iter().map(|&x| hx(x)).collect();
    format!("[{}]", parts.join(", "))
}

/// An event as the script prints it: the time, the kind, then every field.
fn line(e: &Event) -> String {
    let mut out = vec![hx(e.t), e.kind.clone()];
    for v in &e.fields {
        out.push(match v {
            Value::Int(n) => n.to_string(),
            Value::Float(x) => hx(*x),
            Value::Str(s) => s.clone(),
            other => format!("{other:?}"),
        });
    }
    out.join(" ")
}

fn events(f: &Fight, from: usize) -> Vec<String> {
    f.events[from..].iter().map(line).collect()
}

/// `new()`: CONTRABASS (left) against FLUTE, seed 392, the example episode's rules, both kinds set up, at 12.5 s.
fn new<'a>(roster: &'a [FighterCfg]) -> Fight<'a> {
    let mut f = fight(
        &Setup {
            fighters: [fighter(roster, "CONTRABASS"), fighter(roster, "FLUTE")],
            rules: episode_rules(),
            seed: 392,
        },
        false,
    );
    setup(&mut f, 0);
    striker::setup(&mut f, 1);
    f.t = 12.5;
    f
}

fn place(f: &mut Fight, bp: V2, op: V2, ov: V2) {
    f.balls[0].p = bp;
    f.balls[1].p = op;
    f.balls[1].v = ov;
}

fn wstep(f: &Fight) -> Step {
    f.walk.wstep[0].clone().expect("a step under way")
}

fn py(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

/// `show_step`: the step's ends, times and kind, the heading and the walking flag.
fn show_step(f: &Fight) -> String {
    let s = wstep(f);
    format!(
        "p0 {} p1 {} t0 {} t1 {} kind {} v {} walking {}",
        vec(&s.p0),
        vec(&s.p1),
        hx(s.t0),
        hx(s.t1),
        s.kind,
        vec(&f.balls[0].v),
        py(f.balls[0].walking)
    )
}

fn show_wave(w: &Wave) -> String {
    format!(
        "c {} r {} speed {} rmax {} dmg {} knock {} kind {} name {} grow {} clears {}",
        vec(&w.c),
        hx(w.r),
        hx(w.speed),
        hx(w.rmax),
        hx(w.dmg),
        hx(w.knock),
        w.kind,
        w.name,
        py(w.grow),
        py(w.clears)
    )
}

fn s(x: &str) -> String {
    x.to_string()
}

const STEP_A: &str = "p0 [-0x1.0000000000000p+0, 0x1.0000000000000p-1] p1 [-0x1.686c17c75ed76p-2, \
                      0x1.f573342a2409cp-4] t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0 v \
                      [0x1.1f8ce4a112a2bp+1, -0x1.4f15f8f6de978p+0] walking True";
const STEP_A_EV: &str = "0x1.9000000000000p+3 step 0 -0x1.0000000000000p+0 0x1.0000000000000p-1 \
                         -0x1.686c17c75ed76p-2 0x1.f573342a2409cp-4 0x1.972d2d2d2d2d3p+3 0";

#[test]
fn a_step_starts_on_its_grid_with_the_kits_numbers() {
    let roster = roster();
    // A: off the grid nothing; on it the step, the heading, one draw
    let mut f = new(&roster);
    place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    tick(&mut f, 0, 1);
    assert!(f.walk.wstep[0].is_none() && f.events.is_empty());
    tick(&mut f, 0, 2);
    assert_eq!(show_step(&f), STEP_A);
    assert_eq!(events(&f, 0), [STEP_A_EV]);
    assert_eq!(hx(f.rng.uniform(0.0, 1.0)), "0x1.9ae5e6ad6ce20p-4");
    // B: under haste a step every 8th, of one 16th
    let mut f = new(&roster);
    place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    f.balls[0].haste = 1.0;
    f.balls[0].haste_mult = 2.0;
    tick(&mut f, 0, 1);
    assert_eq!(
        show_step(&f),
        "p0 [-0x1.0000000000000p+0, 0x1.0000000000000p-1] p1 [-0x1.5ea793eb769f2p-2, 0x1.1d457cb962e3ep-3] t0 \
         0x1.9000000000000p+3 t1 0x1.9396969696969p+3 kind 1 v [0x1.23c87319f7439p+1, -0x1.401d9f494414cp+0] \
         walking True"
    );
    assert_eq!(
        events(&f, 0),
        [
            "0x1.9000000000000p+3 step 0 -0x1.0000000000000p+0 0x1.0000000000000p-1 -0x1.5ea793eb769f2p-2 \
          0x1.1d457cb962e3ep-3 0x1.9396969696969p+3 1"
        ]
    );
}

#[test]
fn no_step_while_staggered_held_or_stampeding() {
    let roster = roster();
    // C
    let mut f = new(&roster);
    place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    f.balls[0].boost = 0.61;
    tick(&mut f, 0, 2);
    f.balls[0].boost = 0.6;
    f.balls[0].grabbed = true;
    tick(&mut f, 0, 2);
    f.balls[0].grabbed = false;
    f.stampede.stamp[0] = Some(stampede::Stamp {
        t1: 99.0,
        next: 0.0,
        last: -9.0,
        speed: 1.0,
        turn: 1.0,
        dmg: 0.0,
        knock: 0.0,
        cool: 0.0,
        stomp16: 2.0,
        stomp_r: 1.0,
        stomp_speed: 9.0,
        stomp_dmg: 0.0,
        stomp_knock: 0.0,
        name: String::new(),
    });
    tick(&mut f, 0, 2);
    assert!(f.events.is_empty() && f.walk.wstep[0].is_none());
}

#[test]
fn a_step_stops_short_of_contact_and_of_the_ropes() {
    let roster = roster();
    // D
    let mut f = new(&roster);
    place(&mut f, [0.0, 0.0], [0.55, 0.2], [0.0, 0.0]);
    tick(&mut f, 0, 2);
    assert_eq!(
        show_step(&f),
        "p0 [0x0.0p+0, 0x0.0p+0] p1 [0x0.0p+0, 0x0.0p+0] t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0 v \
         [0x1.35a69472516dfp+1, 0x1.e7ced08f98d30p-1] walking True"
    );
    let mut f = new(&roster);
    place(&mut f, [2.3, 2.3], [2.5, 2.4], [0.0, 0.0]);
    tick(&mut f, 0, 2);
    assert_eq!(
        show_step(&f),
        "p0 [0x1.2666666666666p+1, 0x1.2666666666666p+1] p1 [0x1.2666666666666p+1, 0x1.2666666666666p+1] t0 \
         0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0 v [0x1.25a1925b82e47p+1, 0x1.39488114a9e68p+0] \
         walking True"
    );
    let mut f = new(&roster);
    place(&mut f, [0.3, 0.3], [0.3, 0.3], [0.0, 0.0]);
    tick(&mut f, 0, 2);
    assert_eq!(
        show_step(&f),
        "p0 [0x1.3333333333333p-2, 0x1.3333333333333p-2] p1 [0x1.3333333333333p-2, 0x1.3333333333333p-2] t0 \
         0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0 v [0x1.4caf163877c05p+1, 0x1.193cd869fadd4p-4] \
         walking True"
    );
}

#[test]
fn the_swing_the_landing_and_its_quake() {
    let roster = roster();
    // E
    let mut f = new(&roster);
    let dt = f.rules.dt;
    place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    tick(&mut f, 0, 2);
    let st = wstep(&f);
    let n = f.events.len();
    f.t = st.t0 + 0.1;
    step(&mut f, 0, dt);
    assert_eq!(vec(&f.balls[0].p), "[-0x1.74eca22dcbff2p-1, 0x1.5deefa244bf43p-2]");
    f.t = st.t1;
    step(&mut f, 0, dt);
    assert_eq!(vec(&f.balls[0].p), "[-0x1.686c17c75ed76p-2, 0x1.f573342a2409cp-4]");
    assert!(!f.balls[0].walking && f.walk.wstep[0].is_none());
    assert_eq!(
        events(&f, n),
        ["0x1.972d234eb9a17p+3 step_land 0 -0x1.686c17c75ed76p-2 0x1.f573342a2409cp-4 0x1.51eb851eb851fp-1 0"]
    );
    assert_eq!(
        show_wave(f.waves.last().expect("a ring")),
        "c [-0x1.686c17c75ed76p-2, 0x1.f573342a2409cp-4] r 0x1.3333333333333p-1 speed 0x1.2000000000000p+3 rmax \
         0x1.428f5c28f5c29p+0 dmg 0x1.999999999999ap+1 knock 0x1.999999999999ap-3 kind step name WALKING BASS grow \
         True clears False"
    );
    let n = f.events.len();
    let bp = f.balls[0].p;
    f.balls[1].p = [bp[0] + 0.95, bp[1] + 0.3];
    f.balls[1].v = [0.5, 2.4];
    for _ in 0..12 {
        f.t += dt;
        abilities::effects_step(&mut f, dt);
    }
    assert_eq!(
        (hx(f.balls[1].hp), vec(&f.balls[1].v), hx(f.balls[1].boost)),
        (
            s("0x1.7666666666666p+6"),
            s("[0x1.246ab09efee8ep+1, 0x1.3dcb4d32b4c2fp+0]"),
            s("0x1.999999999999ap-3")
        )
    );
    assert_eq!(
        (hx(f.balls[0].blade_len), f.waves.len()),
        (s("0x1.6b851eb851eb9p-1"), 0)
    );
    assert_eq!(
        events(&f, n),
        ["0x1.97b5b2d4d4025p+3 hit 0 1 0x1.999999999999ap+2 0x1.a7f1f638c7ab0p-6 0x1.ef1843a970bf7p-3 step 0"]
    );
    // F: a step due within a substep lands from walk_tick
    let mut f = new(&roster);
    place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    tick(&mut f, 0, 2);
    f.t = wstep(&f).t1 - dt * 0.5;
    let n = f.events.len();
    tick(&mut f, 0, 4);
    assert_eq!(vec(&f.balls[0].p), "[-0x1.686c17c75ed76p-2, 0x1.f573342a2409cp-4]");
    assert!(f.walk.wstep[0].is_none());
    assert_eq!(
        events(&f, n),
        ["0x1.970b0f27bb2ffp+3 step_land 0 -0x1.686c17c75ed76p-2 0x1.f573342a2409cp-4 0x1.51eb851eb851fp-1 0"]
    );
}

#[test]
fn the_reach_grows_to_its_cap() {
    let roster = roster();
    // G
    let mut f = new(&roster);
    let dt = f.rules.dt;
    f.balls[0].blade_len = 1.48;
    for _ in 0..2 {
        f.waves = vec![Wave {
            owner: 0,
            c: [0.0, 0.0],
            r: f.rules.r,
            speed: 9.0,
            rmax: 3.0,
            dmg: 1.0,
            knock: 0.1,
            kind: "step".to_string(),
            hit: false,
            name: "WALKING BASS".to_string(),
            t0: f.t,
            slow: None,
            stun: None,
            detune: None,
            grow: true,
            clears: false,
        }];
        f.balls[1].p = [0.92, 0.0];
        abilities::effects_step(&mut f, dt);
        assert_eq!(hx(f.balls[0].blade_len), "0x1.8000000000000p+0");
    }
}

#[test]
fn a_pinned_walker_restarts_its_step_where_it_stands() {
    let roster = roster();
    // H: with a step under way it restarts where the walker is; planted (no step), it has none to restart
    let expect_w = "[0x1.3aba9a3f9293ap+0, 0x0.0p+0]";
    let expect_o = "[0x1.3333333333333p+1, 0x1.0e6142c89108ep-2]";
    for with_step in [true, false] {
        let mut f = new(&roster);
        let dt = f.rules.dt;
        f.balls[0].p = [2.0, 0.0];
        f.balls[1].p = [2.2, 0.05];
        f.balls[0].v = [1.0, 0.0];
        f.balls[1].v = [1.0, 0.0];
        if with_step {
            f.walk.wstep[0] = Some(Step {
                p0: [1.5, 0.0],
                p1: [2.2, 0.0],
                t0: 12.4,
                t1: 12.75,
                kind: 0,
            });
            f.balls[0].walking = true;
        }
        f.move_step(dt);
        assert_eq!((vec(&f.balls[0].p), vec(&f.balls[1].p)), (s(expect_w), s(expect_o)));
        match &f.walk.wstep[0] {
            Some(st) => {
                assert!(with_step);
                assert_eq!((vec(&st.p0), vec(&st.p1)), (s(expect_w), s(expect_w)));
            }
            None => assert!(!with_step),
        }
    }
}

#[test]
fn slap_bass_waits_for_the_enemy_then_rings() {
    let roster = roster();
    // I
    let mut f = new(&roster);
    let cfg = f.balls[0].cfg;
    let ab = &cfg.unlock1;
    place(&mut f, [0.0, 0.0], [2.0, 0.0], [0.0, 0.0]);
    assert!(!slap::ready(&f, 0, ab));
    f.balls[1].p = [1.6, 0.0];
    assert!(slap::ready(&f, 0, ab));
    f.balls[1].p = [2.0, 0.0];
    f.balls[0].grabbed = true;
    assert!(slap::ready(&f, 0, ab));
    f.balls[0].grabbed = false;
    slap::start(&mut f, 0, ab);
    assert_eq!(
        show_wave(f.waves.last().expect("a ring")),
        "c [0x0.0p+0, 0x0.0p+0] r 0x1.3333333333333p-1 speed 0x1.4000000000000p+3 rmax 0x1.0000000000000p+1 dmg \
         0x1.8000000000000p+1 knock 0x1.999999999999ap+0 kind slap name SLAP BASS grow False clears True"
    );
    assert_eq!(events(&f, 0), ["0x1.9000000000000p+3 slap 0 0"]);
}

#[test]
fn the_elephant_charges_tramples_stomps_and_ends() {
    let roster = roster();
    // J
    let mut f = new(&roster);
    let dt = f.rules.dt;
    let cfg = f.balls[0].cfg;
    let ab = &cfg.low;
    place(&mut f, [-1.0, 0.5], [1.2, -0.7], [1.3, -2.0]);
    tick(&mut f, 0, 2);
    f.balls[0].boost = 0.3;
    stampede::start(&mut f, 0, ab);
    let st = |f: &Fight| f.stampede.stamp[0].clone().expect("charging");
    assert_eq!(vec(&f.balls[0].v), "[0x1.2429e4bf762bbp+1, -0x1.3eb956a2525e2p+0]");
    assert!(f.balls[0].walking && f.balls[0].boost == 0.0 && f.walk.wstep[0].is_none());
    assert_eq!(
        (hx(st(&f).t1), hx(st(&f).next)),
        (s("0x1.e61e1e1e1e1e2p+3"), s("0x1.9000000000000p+3"))
    );
    assert_eq!(
        events(&f, 0),
        [STEP_A_EV, "0x1.9000000000000p+3 stampede 0 0x1.e61e1e1e1e1e2p+3"]
    );
    let n = f.events.len();
    stampede::stampede_step(&mut f, 0, dt);
    assert_eq!(
        (vec(&f.balls[0].p), hx(st(&f).next)),
        (
            s("[-0x1.f1644e29a07ddp-1, 0x1.f0105544af14ep-2]"),
            s("0x1.972d2d2d2d2d3p+3")
        )
    );
    // the heading after the turn: `math.atan2`, `math.cos` and `math.sin` are the C runtime's in the kit and
    // `dmath`'s here, one step apart on this value (ruling M3-R38: the audit that makes `dmath` the C runtime's is
    // not merged yet). Held to one step; exact once it is.
    for (got, want) in f.balls[0]
        .v
        .iter()
        .zip(["0x1.b63ed71f31418p+1", "-0x1.de1601f37b8d3p+0"])
    {
        assert!(ulps(*got, want) <= 1, "{} against the kit's {want}", hx(*got));
    }
    assert_eq!(
        show_wave(f.waves.last().expect("a stomp")),
        "c [-0x1.f1644e29a07ddp-1, 0x1.f0105544af14ep-2] r 0x1.3333333333333p-1 speed 0x1.2000000000000p+3 rmax \
         0x1.b333333333334p+0 dmg 0x1.999999999999ap-1 knock 0x1.999999999999ap-2 kind stampede name THE ELEPHANT \
         grow False clears False"
    );
    assert_eq!(
        events(&f, n),
        ["0x1.9000000000000p+3 stampede_stomp 0 -0x1.f1644e29a07ddp-1 0x1.f0105544af14ep-2 0x1.b333333333334p+0"]
    );
    // the rope (from here on the turn's atan2, cos and sin are one step apart now and then: `near`)
    let n = f.events.len();
    f.t += dt;
    f.balls[0].p = [2.395, 0.0];
    f.balls[0].v = [3.0, 0.4];
    stampede::stampede_step(&mut f, 0, dt);
    near(
        &[vec(&f.balls[0].p), vec(&f.balls[0].v)],
        &[
            s("[0x1.2fb504e23ede4p+1, 0x1.e57e790974be7p-9]"),
            s("[-0x1.eff282bf54981p+1, 0x1.c7269178dd729p-2]"),
        ],
    );
    near(
        &events(&f, n),
        ["0x1.90443d46b26c0p+3 rope 0 0 1 0x1.2fb504e23ede4p+1 0x1.e57e790974be7p-9 0x1.f333333333334p+1"],
    );
    // a trample: the hit, the knock, the heading reflected off the contact
    let n = f.events.len();
    f.t += dt;
    f.balls[0].p = [0.0, 0.0];
    f.balls[0].v = [3.0, 0.0];
    f.balls[1].p = [0.45, 0.1];
    f.balls[1].v = [-1.0, 0.0];
    stampede::stampede_step(&mut f, 0, dt);
    near(
        &[hx(f.balls[1].hp), vec(&f.balls[1].v), hx(f.balls[1].boost)],
        &[
            s("0x1.7000000000000p+6"),
            s("[0x1.3ced1e319d1a7p+1, 0x1.9639391b47354p-1]"),
            s("0x1.ccccccccccccdp+0"),
        ],
    );
    near(
        &[vec(&f.balls[0].v), hx(st(&f).last)],
        &[
            s("[-0x1.c1b0532c7aff9p+1, -0x1.b1801abbd68ccp+0]"),
            s("0x1.9088888888888p+3"),
        ],
    );
    near(
        &events(&f, n),
        [
            "0x1.90888f861a60dp+3 hit 0 1 0x1.0000000000000p+3 0x1.3b7bcb72e751cp-1 0x1.1dd498c9f9c10p-3 trample 0",
            "0x1.90888f861a60dp+3 trample 0 0x1.3b7bcb72e751cp-1 0x1.1dd498c9f9c10p-3",
        ],
    );
    // within `cool`: no second trample
    let n = f.events.len();
    f.t += dt;
    stampede::stampede_step(&mut f, 0, dt);
    assert_eq!((hx(f.balls[1].hp), f.events.len()), (s("0x1.7000000000000p+6"), n));
    // a stomp when it is due
    f.t = st(&f).next;
    stampede::stampede_step(&mut f, 0, dt);
    assert_eq!(hx(st(&f).next), "0x1.9e5a5a5a5a5a6p+3");
    near(
        &events(&f, n),
        ["0x1.972d234eb9a17p+3 stampede_stomp 0 -0x1.9dd421520cc68p-6 -0x1.ded2683445394p-6 0x1.b333333333334p+0"],
    );
    // the end
    let n = f.events.len();
    f.t = st(&f).t1;
    stampede::stampede_step(&mut f, 0, dt);
    assert!(f.stampede.stamp[0].is_none() && !f.balls[0].walking);
    assert_eq!(events(&f, n), ["0x1.e61e2584f4c6ep+3 stampede_end 0"]);
}
