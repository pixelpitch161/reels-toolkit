//! The echo and FINAL FLICK against `fight.py` on hand-built state, in the branches no reference fight reaches: every
//! expected line is the output of `tools/studio_export/unit_checks/echo_unit_vectors.py` (floats as Python's
//! `float.hex()`), compared to the bit. The script's sections A to C are the tests' letters.

use bf_core::event::{Event, Value};

use super::*;
use crate::abilities::flick;
use crate::fight::Setup;
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

fn vec(v: &[f64]) -> String {
    let parts: Vec<String> = v.iter().map(|&x| hx(x)).collect();
    format!("[{}]", parts.join(", "))
}

/// An event as the script prints it after `ev`.
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

/// `new(w)`: HORN (left) against FLUTE, seed 9, the example episode's rules, both kinds set up, at 12.5 s.
fn new<'a>(horn: &'a FighterCfg, flute: &'a FighterCfg) -> Fight<'a> {
    let mut f = fight(
        &Setup {
            fighters: [horn, flute],
            rules: episode_rules(),
            seed: 9,
        },
        false,
    );
    setup(&mut f, 0);
    striker::setup(&mut f, 1);
    f.t = 12.5;
    f
}

/// The script's `show`.
fn show(f: &Fight) -> String {
    let o = &f.balls[1];
    format!(
        "n_echo {} blade_len {} O.p {} O.v {} O.hp {} O.boost {}",
        hx(f.echo.balls[0].n_echo),
        hx(f.balls[0].blade_len),
        vec(&o.p),
        vec(&o.v),
        hx(o.hp),
        hx(o.boost)
    )
}

/// A: a ring whose centre is within 1e-6 of the enemy knocks it along its own heading.
#[test]
fn a_ring_on_the_enemys_centre_knocks_along_its_heading() {
    let roster = roster();
    let mut f = new(fighter(&roster, "HORN"), fighter(&roster, "FLUTE"));
    f.balls[0].p = [-1.2, 0.4];
    f.balls[1].p = [0.7, -0.3];
    f.balls[1].v = [1.3, -2.1];
    let c = geom::add(f.balls[1].p, [4e-7, -3e-7]);
    blast(&mut f, 0, c, 0, 1);
    let dt = f.rules.dt;
    step(&mut f, 0, dt);
    assert_eq!(
        show(&f),
        "n_echo 0x1.0000000000000p+0 blade_len 0x1.ccccccccccccdp-1 O.p [0x1.6666666666666p-1, -0x1.3333333333333p-2] \
         O.v [0x1.5e5775294557cp+0, -0x1.1af7dea1558bdp+1] O.hp 0x1.6d9999999999ap+6 O.boost 0x1.0000000000000p-1"
    );
    assert_eq!(
        events(&f, 0),
        [
            "0x1.9000000000000p+3 echo 0 0 1 0x1.666673d25fb3cp-1 -0x1.3333475529274p-2 0x1.8000000000000p+0",
            "0x1.9000000000000p+3 hit 0 1 0x1.1333333333333p+3 0x1.666673d25fb3cp-1 -0x1.3333475529274p-2 echo 0",
            "0x1.9000000000000p+3 echo_grow 0 1",
        ]
    );
}

/// B: `reach_grow` lengthens the reach per landed blast, up to `max_reach`.
#[test]
fn a_growing_reach_stops_at_max_reach() {
    let roster = roster();
    let mut horn = fighter(&roster, "HORN").clone();
    let WeaponCfg::Echo(w) = &mut horn.weapon else {
        panic!("HORN's weapon is an echo")
    };
    w.reach_grow = 0.15;
    w.max_reach = Some(1.1);
    let mut f = new(&horn, fighter(&roster, "FLUTE"));
    f.balls[0].p = [-0.4, 0.2];
    f.balls[1].p = [0.5, 0.25];
    f.balls[1].v = [0.6, 0.9];
    let dt = f.rules.dt;
    let want = [
        (
            "n_echo 0x1.0000000000000p+0 blade_len 0x1.0cccccccccccdp+0 O.p [0x1.0000000000000p-1, \
             0x1.0000000000000p-2] O.v [0x1.4435eb7c6fe34p+1, 0x1.2c7ce6be4170dp-1] O.hp 0x1.6d9999999999ap+6 \
             O.boost 0x1.0000000000000p-1",
            [
                "0x1.9000000000000p+3 echo 0 0 1 -0x1.999999999999ap-2 0x1.999999999999ap-3 0x1.8000000000000p+0",
                "0x1.9000000000000p+3 hit 0 1 0x1.1333333333333p+3 -0x1.95d0f02059c18p-4 0x1.bbd6a3ff19f38p-3 echo 0",
                "0x1.9000000000000p+3 echo_grow 0 1",
            ],
        ),
        (
            "n_echo 0x1.0000000000000p+1 blade_len 0x1.199999999999ap+0 O.p [0x1.0000000000000p-1, \
             0x1.0000000000000p-2] O.v [0x1.4b7518a1fa321p+1, 0x1.ddd0cd6ce76a9p-3] O.hp 0x1.4b33333333334p+6 \
             O.boost 0x1.0000000000000p-1",
            [
                "0x1.9000000000000p+3 echo 0 0 2 -0x1.999999999999ap-2 0x1.999999999999ap-3 0x1.a666666666666p+0",
                "0x1.9000000000000p+3 hit 0 1 0x1.1333333333333p+3 -0x1.95d0f02059c18p-4 0x1.bbd6a3ff19f38p-3 echo 0",
                "0x1.9000000000000p+3 echo_grow 0 2",
            ],
        ),
    ];
    for (call, (ball, evs)) in (1..).zip(want) {
        let n = f.events.len();
        let p = f.balls[0].p;
        blast(&mut f, 0, p, 0, call);
        step(&mut f, 0, dt);
        assert_eq!(show(&f), ball, "call {call}");
        assert_eq!(events(&f, n), evs, "call {call}");
    }
}

/// C: FINAL FLICK out of reach misses at full reach and is put away `show` s after it flicked.
#[test]
fn a_flick_out_of_reach_misses_and_is_put_away() {
    let roster = roster();
    let mut f = new(fighter(&roster, "HORN"), fighter(&roster, "FLUTE"));
    let ab = f.balls[0].cfg.low.clone();
    f.balls[0].p = [-2.1, -1.5];
    f.balls[1].p = [2.0, 1.9];
    f.balls[1].v = [0.4, -0.2];
    flick::start(&mut f, 0, &ab);
    let g = f.flick.glove[0].expect("the glove winds up");
    assert_eq!(
        format!("wind t_hit {} aim {}", hx(g.t_hit), hx(g.aim)),
        "wind t_hit 0x1.9e5a5a5a5a5a6p+3 aim 0x1.6279a8b5a170dp-1"
    );
    f.balls[1].p = [2.3, 1.7];
    f.t = g.t_hit;
    let dt = f.rules.dt;
    flick::flick_step(&mut f, 0, dt);
    let g = f.flick.glove[0].expect("the glove is out");
    assert_eq!(
        format!(
            "out state {} aim {} ext {} hit {} t_out {}",
            if g.state == flick::Phase::Out { "out" } else { "wind" },
            hx(g.aim),
            hx(g.ext),
            g.hit,
            hx(g.t_out)
        ),
        "out state out aim 0x1.41f1963c0045fp-1 ext 0x1.3333333333333p+1 hit 0 t_out 0x1.9e5a5a5a5a5a6p+3"
    );
    assert!(f.flick.rope[0].is_none(), "a miss flings nobody");
    f.t = g.t_out + 0.6;
    flick::flick_step(&mut f, 0, dt);
    assert!(f.flick.glove[0].is_some(), "still out at the end of its show");
    f.t = g.t_out + 0.6 + dt;
    flick::flick_step(&mut f, 0, dt);
    assert!(f.flick.glove[0].is_none(), "put away after it");
    assert_eq!(
        show(&f),
        "n_echo 0x0.0p+0 blade_len 0x1.ccccccccccccdp-1 O.p [0x1.2666666666666p+1, 0x1.b333333333333p+0] \
         O.v [0x1.999999999999ap-2, -0x1.999999999999ap-3] O.hp 0x1.9000000000000p+6 O.boost 0x0.0p+0"
    );
    assert_eq!(
        events(&f, 0),
        [
            "0x1.9000000000000p+3 flick_wind 0 0x1.9e5a5a5a5a5a6p+3 0x1.6279a8b5a170dp-1",
            "0x1.9e5a5b9628cbdp+3 flick 0 0 0x1.41f1963c0045fp-1 0x1.3333333333333p+1 0",
        ]
    );
}
