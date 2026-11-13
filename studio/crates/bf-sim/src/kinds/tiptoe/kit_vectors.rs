//! The tiptoe, MOUNTAIN KING and BOCAL HOOK against `fight.py` on hand-built state: every expected line is the output
//! of `tools/studio_export/unit_checks/tiptoe_unit_vectors.py` (floats as Python's `float.hex()`), compared to the
//! bit. The script's sections A to H are the tests' letters.

use bf_core::event::{Event, Value};

use super::*;
use crate::abilities::hook;
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

/// Python's `str` of a bool.
fn py(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

/// An event as the script prints it after `ev`: the time, the kind, then every field.
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

/// `new()`: BASSOON (left) against FLUTE, seed 9, the example episode's rules, both kinds set up, at 12.5 s.
fn new<'a>(roster: &'a [FighterCfg]) -> Fight<'a> {
    let mut f = fight(
        &Setup {
            fighters: [fighter(roster, "BASSOON"), fighter(roster, "FLUTE")],
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

fn place(f: &mut Fight, bp: V2, op: V2, ov: V2) {
    f.balls[0].p = bp;
    f.balls[1].p = op;
    f.balls[1].v = ov;
}

/// The script's `show_ball` without its indent.
fn ball(f: &Fight) -> String {
    let (b, o) = (&f.balls[0], &f.balls[1]);
    format!(
        "p {} v {} air {} blade_len {} stomp_r {} O.p {} O.v {} O.hp {} O.boost {}",
        vec(&b.p),
        vec(&b.v),
        py(b.air),
        hx(b.blade_len),
        hx(f.tiptoe.stomp_r[0]),
        vec(&o.p),
        vec(&o.v),
        hx(o.hp),
        hx(o.boost)
    )
}

/// The script's `show_hop` without its indent.
fn hop(f: &Fight) -> String {
    match f.tiptoe.hop[0] {
        None => "hop None".to_string(),
        Some(h) => format!(
            "hop p0 {} p1 {} t0 {} t1 {} kind {}",
            vec(&h.p0),
            vec(&h.p1),
            hx(h.t0),
            hx(h.t1),
            h.kind
        ),
    }
}

#[test]
fn the_rules_are_the_scripts() {
    let r = episode_rules();
    assert_eq!(
        (hx(r.beat), hx(r.dt)),
        ("0x1.cb4b4b4b4b4b4p-2".into(), "0x1.1111111111111p-7".into())
    );
    assert_eq!((r.dmg_scale, r.dmg_side), (1.0, Some([2.0, 0.4])));
}

#[test]
fn behind_is_the_kits() {
    // A
    let roster = roster();
    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9]);
    assert_eq!(vec(&behind(&f, 0)), "[0x1.a2df8d09424fdp-2, -0x1.d3346c142c6cbp-1]");
    f.balls[1].v = [0.0, 0.0];
    assert_eq!(vec(&behind(&f, 0)), "[-0x1.572fc2a9de409p-1, -0x1.7bf4e0a9c859dp-1]");
    f.balls[1].p = f.balls[0].p;
    assert_eq!(vec(&behind(&f, 0)), "[0x1.0000000000000p+0, 0x0.0p+0]");
}

#[test]
fn hops_launch_as_the_kits() {
    // B
    let o = "O.p [0x1.b333333333333p+0, 0x1.ccccccccccccdp-2] O.v [-0x1.4cccccccccccdp+0, 0x1.7333333333333p+1] \
             O.hp 0x1.9000000000000p+6 O.boost 0x0.0p+0";
    let cases: [(V2, V2, f64, &str, &str, &str); 3] = [
        (
            [0.3, -1.1],
            [1.25, 0.8],
            3.0,
            "hop p0 [0x1.3333333333333p-2, -0x1.199999999999ap+0] p1 [0x1.4000000000000p+0, 0x1.999999999999ap-1] \
             t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0",
            "p [0x1.3333333333333p-2, -0x1.199999999999ap+0] v [0x1.29aa55a232767p+0, 0x1.29aa55a232767p+1]",
            "0x1.9000000000000p+3 hop 0 0x1.3333333333333p-2 -0x1.199999999999ap+0 0x1.4000000000000p+0 \
             0x1.999999999999ap-1 0x1.972d2d2d2d2d3p+3 0",
        ),
        (
            [0.3, -1.1],
            [-2.2, 1.9],
            1.6,
            "hop p0 [0x1.3333333333333p-2, -0x1.199999999999ap+0] p1 [-0x1.72d6cca64c75cp-1, 0x1.0881eacc50498p-3] \
             t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0",
            "p [0x1.3333333333333p-2, -0x1.199999999999ap+0] v [-0x1.5d2ace7a8b54ep-1, 0x1.a3002af973ff7p-1]",
            "0x1.9000000000000p+3 hop 0 0x1.3333333333333p-2 -0x1.199999999999ap+0 -0x1.72d6cca64c75cp-1 \
             0x1.0881eacc50498p-3 0x1.972d2d2d2d2d3p+3 0",
        ),
        (
            [2.0, 1.7],
            [3.6, 2.9],
            3.0,
            "hop p0 [0x1.0000000000000p+1, 0x1.b333333333333p+0] p1 [0x1.3333333333333p+1, 0x1.3333333333333p+1] \
             t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0",
            "p [0x1.0000000000000p+1, 0x1.b333333333333p+0] v [0x1.0a3d70a3d70a4p+1, 0x1.8f5c28f5c28f6p+0]",
            "0x1.9000000000000p+3 hop 0 0x1.0000000000000p+1 0x1.b333333333333p+0 0x1.3333333333333p+1 \
             0x1.3333333333333p+1 0x1.972d2d2d2d2d3p+3 0",
        ),
    ];
    let roster = roster();
    for (bp, tgt, reach, want_hop, want_pv, want_ev) in cases {
        let mut f = new(&roster);
        place(&mut f, bp, [1.7, 0.45], [-1.3, 2.9]);
        hop_launch(&mut f, 0, 2, 0, tgt, reach);
        assert_eq!(hop(&f), want_hop);
        let rest = "air True blade_len 0x1.999999999999ap+0 stomp_r 0x1.999999999999ap-3";
        assert_eq!(ball(&f), format!("{want_pv} {rest} {o}"));
        assert_eq!(events(&f, 0), [want_ev]);
    }
}

#[test]
fn landings_stomp_as_the_kits() {
    // C
    let me =
        "p [0x1.d2eb2be106d91p-1, -0x1.ece21a9cfdc71p-2] v [0x1.3e7e44d4c4d88p+1, -0x1.821df59709152p-1] air False";
    let grown = "blade_len 0x1.a3d70a3d70a3ep+0 stomp_r 0x1.a9fbe76c8b43ap-3";
    let plain = "blade_len 0x1.999999999999ap+0 stomp_r 0x1.999999999999ap-3";
    let o_p = "O.p [0x1.b333333333333p+0, 0x1.ccccccccccccdp-2]";
    let at = "0x1.4ffbb7b1e0f31p+0 -0x1.072f615cd21c0p-7";
    let t = "0x1.9000000000000p+3";
    /// The case, the landing spot, the enemy's velocity, the hop's kind, `show_ball`, the events.
    type Landing<'s> = (&'s str, V2, V2, i64, String, Vec<String>);
    let cases: [Landing; 5] = [
        (
            "behind",
            [1.15, -0.2],
            [1.1, 1.3],
            0,
            format!(
                "{me} {grown} {o_p} O.v [0x1.adf06c85b7e7ep+0, 0x1.fc1c2329aacc3p+0] O.hp 0x1.7800000000000p+6 \
                 O.boost 0x1.ccccccccccccdp-2"
            ),
            vec![
                format!("{t} hit 0 1 0x1.8000000000000p+2 {at} stomp 1"),
                format!("{t} stomp 0 0x1.d2eb2be106d91p-1 -0x1.ece21a9cfdc71p-2 1 1 0x1.a9fbe76c8b43ap-3 0"),
            ],
        ),
        (
            "front",
            [1.15, -0.2],
            [-1.1, -1.3],
            0,
            format!(
                "{me} {grown} {o_p} O.v [0x1.adf06c85b6acbp+0, 0x1.fc1c2329a957cp+0] O.hp 0x1.8666666666666p+6 \
                 O.boost 0x1.ccccccccccccdp-2"
            ),
            vec![
                format!("{t} hit 0 1 0x1.3333333333333p+1 {at} stomp 0"),
                format!("{t} stomp 0 0x1.d2eb2be106d91p-1 -0x1.ece21a9cfdc71p-2 1 0 0x1.a9fbe76c8b43ap-3 0"),
            ],
        ),
        (
            "inside",
            [1.55, 0.3],
            [-1.3, 2.9],
            1,
            format!(
                "p [0x1.acb67b62755c7p-1, -0x1.a693093b1547bp-2] v [0x1.3e7e44d4c4d88p+1, -0x1.821df59709152p-1] \
                 air False {grown} {o_p} O.v [0x1.69754056aab52p+0, 0x1.1773754cde9fbp+1] O.hp 0x1.8666666666666p+6 \
                 O.boost 0x1.ccccccccccccdp-2"
            ),
            vec![
                format!("{t} hit 0 1 0x1.3333333333333p+1 0x1.4696a14736e10p+0 0x1.a5a851cdb83b0p-6 stomp 0"),
                format!("{t} stomp 0 0x1.acb67b62755c7p-1 -0x1.a693093b1547bp-2 1 0 0x1.a9fbe76c8b43ap-3 1"),
            ],
        ),
        (
            "miss",
            [-1.8, -2.0],
            [-1.3, 2.9],
            0,
            format!(
                "p [-0x1.ccccccccccccdp+0, -0x1.0000000000000p+1] v [0x1.3e7e44d4c4d88p+1, -0x1.821df59709152p-1] \
                 air False {plain} {o_p} O.v [-0x1.4cccccccccccdp+0, 0x1.7333333333333p+1] O.hp 0x1.9000000000000p+6 \
                 O.boost 0x0.0p+0"
            ),
            vec![format!(
                "{t} stomp 0 -0x1.ccccccccccccdp+0 -0x1.0000000000000p+1 0 0 0x1.999999999999ap-3 0"
            )],
        ),
        (
            "mountain",
            [1.15, -0.2],
            [1.1, 1.3],
            2,
            format!(
                "{me} {plain} {o_p} O.v [0x1.adf06c85b7e7ep+0, 0x1.fc1c2329aacc3p+0] O.hp 0x1.8800000000000p+6 \
                 O.boost 0x1.3333333333333p-2"
            ),
            vec![
                format!("{t} hit 0 1 0x1.0000000000000p+1 {at} stomp 1"),
                format!("{t} stomp 0 0x1.d2eb2be106d91p-1 -0x1.ece21a9cfdc71p-2 1 1 0x1.999999999999ap-3 2"),
            ],
        ),
    ];
    let roster = roster();
    let low = &fighter(&roster, "BASSOON").low;
    for (tag, p1, ov, kind, want_ball, want_events) in cases {
        let mut f = new(&roster);
        place(&mut f, [0.3, -1.1], [1.7, 0.45], ov);
        if kind == 2 {
            start_mountain(&mut f, 0, low);
            f.tiptoe.mk[0] = None;
        }
        let n = f.events.len();
        f.tiptoe.hop[0] = Some(Hop {
            p0: [0.3, -1.1],
            p1,
            t0: 12.4,
            t1: 12.5,
            kind,
        });
        f.balls[0].air = true;
        hop_land(&mut f, 0);
        assert_eq!(ball(&f), want_ball, "{tag}");
        assert_eq!(events(&f, n), want_events, "{tag}");
    }
}

#[test]
fn ticks_hop_as_the_kits() {
    // D
    let roster = roster();
    let o = "O.p [0x1.b333333333333p+0, 0x1.ccccccccccccdp-2] O.v [-0x1.4cccccccccccdp+0, 0x1.7333333333333p+1] \
             O.hp 0x1.9000000000000p+6 O.boost 0x0.0p+0";
    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9]);
    tick(&mut f, 0, 5);
    assert_eq!((f.tiptoe.hop[0], f.events.len()), (None, 0));
    tick(&mut f, 0, 8);
    assert_eq!(
        hop(&f),
        "hop p0 [0x1.3333333333333p-2, -0x1.199999999999ap+0] p1 [0x1.c3d7e78b2b05fp+0, -0x1.d3bd3e462a4cap-2] \
         t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 0"
    );
    assert_eq!(
        ball(&f),
        format!(
            "p [0x1.3333333333333p-2, -0x1.199999999999ap+0] v [0x1.b9ad320456702p+0, 0x1.83d7818f6674ap-1] \
             air True blade_len 0x1.999999999999ap+0 stomp_r 0x1.999999999999ap-3 {o}"
        )
    );
    assert_eq!(
        events(&f, 0),
        [
            "0x1.9000000000000p+3 hop 0 0x1.3333333333333p-2 -0x1.199999999999ap+0 0x1.c3d7e78b2b05fp+0 \
          -0x1.d3bd3e462a4cap-2 0x1.972d2d2d2d2d3p+3 0"
        ]
    );
    assert_eq!(hx(f.rng.uniform(0.0, 1.0)), "0x1.d4acb401d49e3p-1");

    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9]);
    f.balls[0].boost = 0.5;
    tick(&mut f, 0, 8);
    assert_eq!((f.tiptoe.hop[0], f.events.len()), (None, 0));
    f.balls[0].boost = 0.0;
    f.tiptoe.hop_bonus[0] = true;
    tick(&mut f, 0, 9);
    assert_eq!(
        hop(&f),
        "hop p0 [0x1.3333333333333p-2, -0x1.199999999999ap+0] p1 [0x1.1384cf84032fap+1, -0x1.1e69f0d8f8fbep-1] \
         t0 0x1.9000000000000p+3 t1 0x1.9396969696969p+3 kind 1"
    );
    assert_eq!(
        ball(&f),
        format!(
            "p [0x1.3333333333333p-2, -0x1.199999999999ap+0] v [0x1.3f7981ad54a4dp+1, 0x1.74eb5382e24dap-1] \
             air True blade_len 0x1.999999999999ap+0 stomp_r 0x1.999999999999ap-3 {o}"
        )
    );
    assert_eq!(
        events(&f, 0),
        [
            "0x1.9000000000000p+3 hop 0 0x1.3333333333333p-2 -0x1.199999999999ap+0 0x1.1384cf84032fap+1 \
          -0x1.1e69f0d8f8fbep-1 0x1.9396969696969p+3 1"
        ]
    );
    // the follow-up hop draws nothing
    assert!(!f.tiptoe.hop_bonus[0]);
    assert_eq!(hx(f.rng.uniform(0.0, 1.0)), "0x1.6ea1557749065p-1");
}

#[test]
fn the_mountain_king_hops_as_the_kits() {
    // E
    let roster = roster();
    let low = &fighter(&roster, "BASSOON").low;
    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9]);
    start_mountain(&mut f, 0, low);
    assert_eq!(events(&f, 0), ["0x1.9000000000000p+3 mountain 0 20"]);
    let mut n = f.events.len();
    let ang = "-0x1.263d9434c589bp+0";
    let first = "hop p0 [0x1.3333333333333p-2, -0x1.199999999999ap+0] p1 [0x1.0746bc871d2f6p+1, \
                 -0x1.6258b63810b80p-2] t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 2";
    let second = "hop p0 [0x1.3333333333333p-2, -0x1.199999999999ap+0] p1 [0x1.3333333333333p+1, \
                  0x1.0bfe1ed1d79fap+0] t0 0x1.9000000000000p+3 t1 0x1.972d2d2d2d2d3p+3 kind 2";
    let first_ev = "0x1.9000000000000p+3 hop 0 0x1.3333333333333p-2 -0x1.199999999999ap+0 0x1.0746bc871d2f6p+1 \
                    -0x1.6258b63810b80p-2 0x1.972d2d2d2d2d3p+3 2";
    let second_ev = "0x1.9000000000000p+3 hop 0 0x1.3333333333333p-2 -0x1.199999999999ap+0 0x1.3333333333333p+1 \
                     0x1.0bfe1ed1d79fap+0 0x1.972d2d2d2d2d3p+3 2";
    let cases = [
        (40, 1, 43, first, Some(first_ev)),
        (41, 1, 43, "hop None", None),
        (43, 2, 46, second, Some(second_ev)),
        (44, 2, 46, "hop None", None),
    ];
    for (k16, i, next, want, want_ev) in cases {
        f.tiptoe.hop[0] = None;
        f.balls[0].air = false;
        tick(&mut f, 0, k16);
        let mk = f.tiptoe.mk[0].clone().expect("the dance");
        assert_eq!(
            (mk.i, mk.next, mk.ang.map(hx)),
            (i, Some(next), Some(ang.to_string())),
            "k16 {k16}"
        );
        assert_eq!(hop(&f), want, "k16 {k16}");
        let want_ev: Vec<String> = want_ev.into_iter().map(String::from).collect();
        assert_eq!(events(&f, n), want_ev, "k16 {k16}");
        n = f.events.len();
    }
    f.tiptoe.hop[0] = None;
    f.balls[0].air = false;
    f.tiptoe.mk[0].as_mut().expect("the dance").i = 20;
    tick(&mut f, 0, 80);
    assert_eq!((&f.tiptoe.mk[0], f.tiptoe.hop[0]), (&None, None));
    assert_eq!(events(&f, n), ["0x1.9000000000000p+3 mountain_end 0"]);
}

#[test]
fn a_hop_moves_and_lands_as_the_kits() {
    // F
    let roster = roster();
    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [1.7, 0.45], [-1.3, 2.9]);
    hop_launch(&mut f, 0, 2, 0, [1.25, 0.8], 3.0);
    let n = f.events.len();
    let h = f.tiptoe.hop[0].expect("a hop");
    let dt = f.rules.dt;
    f.t = 12.5 + 0.37 * (h.t1 - h.t0);
    step(&mut f, 0, dt);
    assert_eq!(vec(&f.balls[0].p), "[0x1.4d916872b022ap-1, -0x1.96872b020c422p-2]");
    assert_eq!(f.events.len(), n);
    f.t = h.t1 - 5e-10;
    step(&mut f, 0, dt);
    assert_eq!(hop(&f), "hop None");
    assert_eq!(
        ball(&f),
        "p [0x1.7956cabb47437p-1, 0x1.32f228a069f9bp+0] v [0x1.29aa55a232767p+0, 0x1.29aa55a232767p+1] air False \
         blade_len 0x1.a3d70a3d70a3ep+0 stomp_r 0x1.a9fbe76c8b43ap-3 O.p [0x1.b333333333333p+0, 0x1.ccccccccccccdp-2] \
         O.v [0x1.222e8eaa4a7cap+1, -0x1.45e2aa6a5197bp+0] O.hp 0x1.8666666666666p+6 O.boost 0x1.ccccccccccccdp-2"
    );
    assert_eq!(
        events(&f, n),
        [
            "0x1.972d234eb9a17p+3 hit 0 1 0x1.3333333333333p+1 0x1.39f49b9c01938p+0 0x1.a300a78a7ab42p-1 stomp 0",
            "0x1.972d234eb9a17p+3 stomp 0 0x1.7956cabb47437p-1 0x1.32f228a069f9bp+0 1 0 0x1.a9fbe76c8b43ap-3 0"
        ]
    );
}

#[test]
fn the_hook_waits_turns_and_lands_as_the_kits() {
    // G
    let roster = roster();
    let ab = &fighter(&roster, "BASSOON").unlock2;
    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [2.2, 0.9], [-1.3, 2.9]);
    assert!(!hook::ready(&f, 0, ab));
    f.balls[1].p = [1.05, 0.15];
    assert!(hook::ready(&f, 0, ab));
    f.balls[0].air = true;
    assert!(!hook::ready(&f, 0, ab));
    f.balls[0].air = false;
    hook::start(&mut f, 0, ab);
    let hk = |f: &Fight| f.hook.hook[0].expect("a hook");
    assert_eq!(
        (hx(hk(&f).aim), hx(hk(&f).t_hook)),
        ("0x1.07c6c6947a6a8p+0".into(), "0x1.972d2d2d2d2d3p+3".into())
    );
    assert_eq!(
        events(&f, 0),
        ["0x1.9000000000000p+3 hook_open 0 0x1.972d2d2d2d2d3p+3 0x1.07c6c6947a6a8p+0"]
    );
    let n = f.events.len();
    let dt = f.rules.dt;
    for want in ["0x1.fe0798b9705f0p-1", "0x1.ec6c5f7123988p-1", "0x1.dac670561bb50p-1"] {
        let p = f.balls[1].p;
        f.balls[1].p = [p[0] + 0.04, p[1] + -0.03];
        f.t += dt;
        hook::hook_step(&mut f, 0, dt);
        assert_eq!((hx(hk(&f).aim), hk(&f).state), (want.to_string(), 1));
    }
    f.t = hk(&f).t_hook;
    hook::hook_step(&mut f, 0, dt);
    let h = hk(&f);
    assert_eq!(
        (hx(h.aim), h.state, h.t_end.map(hx), f.tiptoe.hop_bonus[0]),
        (
            "0x1.dac670561bb50p-1".into(),
            2,
            Some("0x1.a0c6c6c6c6c6dp+3".into()),
            true
        )
    );
    assert_eq!(
        ball(&f),
        "p [0x1.3333333333333p-2, -0x1.199999999999ap+0] v [0x1.3e7e44d4c4d88p+1, -0x1.821df59709152p-1] air False \
         blade_len 0x1.999999999999ap+0 stomp_r 0x1.999999999999ap-3 O.p [0x1.2b851eb851eb9p+0, 0x1.eb851eb851eb8p-5] \
         O.v [0x1.e825ffee3c8a4p+0, 0x1.456eaa9ed306ep+1] O.hp 0x1.7000000000000p+6 O.boost 0x1.999999999999ap-2"
    );
    assert_eq!(
        events(&f, n),
        [
            "0x1.972d234eb9a17p+3 hit 0 1 0x1.0000000000000p+3 0x1.9eb851eb851edp-1 -0x1.ae147ae147ae1p-2 hook 0",
            "0x1.972d234eb9a17p+3 hook 0 1 0x1.dac670561bb50p-1 0x1.199999999999ap+0"
        ]
    );
    let n = f.events.len();
    let t_end = h.t_end.expect("put away later");
    f.t = t_end - 1e-6;
    hook::hook_step(&mut f, 0, dt);
    assert!(f.hook.hook[0].is_some());
    f.t = t_end;
    hook::hook_step(&mut f, 0, dt);
    assert!(f.hook.hook[0].is_none());
    assert_eq!(f.events.len(), n);
}

#[test]
fn the_hook_misses_as_the_kits() {
    // H
    let roster = roster();
    let ab = &fighter(&roster, "BASSOON").unlock2;
    let mut f = new(&roster);
    place(&mut f, [0.3, -1.1], [1.05, 0.15], [-1.3, 2.9]);
    hook::start(&mut f, 0, ab);
    let n = f.events.len();
    f.balls[1].p = [-0.6, -0.4];
    f.t = f.hook.hook[0].expect("a hook").t_hook;
    let dt = f.rules.dt;
    hook::hook_step(&mut f, 0, dt);
    let h = f.hook.hook[0].expect("a hook");
    assert_eq!(
        (hx(h.aim), f.tiptoe.hop_bonus[0], hx(f.balls[1].hp)),
        ("0x1.1493936147374p+0".into(), false, "0x1.9000000000000p+6".into())
    );
    assert_eq!(
        events(&f, n),
        ["0x1.972d234eb9a17p+3 hook 0 0 0x1.1493936147374p+0 0x1.199999999999ap+0"]
    );
}
