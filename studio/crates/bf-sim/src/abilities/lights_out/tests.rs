//! LIGHTS OUT's unit tests. Every expected number is what `fight.py` gave on the reference PC for the same
//! hand-built state (`tools/studio_export/unit_checks/anchor_unit_vectors.py`, its output pasted): a fight under the
//! example episode's rules, the clock at 12.5 s.
//!
//! What is compared how. Everything is compared to the bit but the shadow's heading while it homes and what follows
//! from it (the knock of the eruption along the line between the balls is not affected): `lights_out_step` takes
//! `math.atan2` of the ball's velocity every step, and `dmath::atan2` is one step from CPython's on some inputs
//! (ruling M3-R38; for example `atan2(-0.19484431151375917, 2.592688892688579)` is `0xbfb333e262cfe8eb` here and
//! `...8ec` there, the first step of the first test). Those velocities are held to 1e-12 until the shared function
//! is CPython's; then [`steered`] becomes `assert_eq!`.

use bf_core::event::Value::{Float as F, Int as I, Str as S};
use bf_core::event::{Event, Value};

use super::*;
use crate::abilities::trigger;
use crate::ball::Slot;
use crate::fight::Setup;
use crate::geom::V2;
use crate::roster::{FighterCfg, WeaponCfg};
use crate::rules::Rules;
use crate::testing;

const BCL: &str = "BASS CLARINET";
const T: f64 = 12.5;

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
    let [p, v, op, ov] = at;
    f.balls[k].p = p;
    f.balls[k].v = v;
    f.balls[1 - k].p = op;
    f.balls[1 - k].v = ov;
    f.t = T;
    f
}

/// A steered velocity against the kit's (see the module's note: `math.atan2`, ruling M3-R38).
fn steered(got: (V2, f64, f64), want: (V2, f64, f64)) {
    assert!(
        (got.0[0] - want.0[0]).abs() <= 1e-12 && (got.0[1] - want.0[1]).abs() <= 1e-12,
        "{got:?} is not {want:?}"
    );
    assert_eq!((got.1, got.2), (want.1, want.2));
}

/// `lights_out_step` once per step from the clock until the eruption, the enemy drifting by `drift` a step: each
/// step's `(v, boost, shadow)`.
fn sink(f: &mut Fight, k: usize, drift: V2) -> Vec<(V2, f64, f64)> {
    let dt = f.rules.dt;
    let t0 = f.t;
    let mut seen = Vec::new();
    for i in 1..400 {
        f.t = t0 + i as f64 * dt;
        let o = &mut f.balls[1 - k];
        o.p = [o.p[0] + drift[0], o.p[1] + drift[1]];
        lights_out_step(f, k, dt);
        let b = &f.balls[k];
        seen.push((b.v, b.boost, b.shadow));
        if f.lights_out.balls[k].is_none() {
            break;
        }
    }
    seen
}

/// `start_lights_out` on a whirling ball pulls the peg first; the shadow cannot be hit and starts no attack; it
/// homes on the enemy at `turn` radians a second and erupts on it after its 4 beats.
#[test]
fn it_pulls_its_peg_sinks_homes_and_erupts() {
    let roster = testing::roster();
    let at = [[-1.0, 0.5], [1.0, -2.0], [2.4, 2.4], [-0.5, 2.0]];
    let mut f = duel(&roster, [BCL, "FLUTE"], 0, at);
    let WeaponCfg::Anchor(w) = &f.balls[0].cfg.weapon else {
        panic!("an anchor")
    };
    anchor::anchor_plant(&mut f, 0, w, 0.95);
    let dt = f.rules.dt;
    for i in 1..=10 {
        f.t = T + i as f64 * dt;
        anchor::step(&mut f, 0, dt);
    }
    f.balls[1].p = [0.5, -0.1];
    trigger(&mut f, 0, Slot::Low);
    let (p, v) = (
        [-0.29815215682044494, 0.10485582697703966],
        [2.592688892688579, -0.19484431151375917],
    );
    let t_end = 14.377450980392158;
    assert_eq!(
        f.events[1..],
        [
            ev(
                12.58333,
                "ability",
                vec![I(0), S("LIGHTS OUT".into()), S("lights_out".into()), I(1)]
            ),
            ev(12.58333, "pull", vec![I(0), F(p[0]), F(p[1]), F(v[0]), F(v[1])]),
            ev(12.58333, "lights_out", vec![I(0), F(t_end)]),
        ]
    );
    let b = &f.balls[0];
    assert!(b.anc.is_none());
    assert_eq!((b.p, b.v, b.boost, b.shadow), (p, v, 0.8, 1.7941176470588243));
    let lo = f.lights_out.balls[0].as_ref().expect("in shadow");
    assert_eq!((lo.t0, lo.t_end), (12.583333333333334, t_end));
    assert!(crate::fight::stunned(&f, 0));
    assert!(!f.damage(1, 0, 5.0, "strike", [0.0, 0.0], 0));
    assert_eq!(f.balls[0].hp, 100.0);
    let n = f.events.len();
    let seen = sink(&mut f, 0, [0.0, 0.0]);
    let want = [
        ([2.5880673006815655, -0.24881247384894925], 0.8, 1.7857843137254914),
        ([2.5823224562030775, -0.3026726485649182], 0.8, 1.7774509803921568),
        ([2.5754568525850146, -0.3564014596952267], 0.8, 1.769117647058824),
    ];
    for (got, want) in seen.iter().zip(want) {
        steered(*got, want);
    }
    assert_eq!(seen.len(), 216);
    steered(seen[215], ([2.5183730712973817, -0.6463722408675918], 0.8, 0.0));
    let hit = vec![I(0), I(1), F(12.0), F(0.5), F(-0.1), S("erupt".into()), I(0)];
    assert_eq!(
        f.events[n..],
        [
            ev(14.38333, "hit", hit),
            ev(14.38333, "erupt", vec![I(0), F(p[0]), F(p[1]), I(1)]),
        ]
    );
    let o = &f.balls[1];
    assert_eq!(
        (o.hp, o.v, o.boost, f.balls[0].dealt),
        (88.0, [2.599941919424951, -0.01737859598154429], 1.8, 12.0)
    );
}

/// The eruption misses an enemy beyond `radius` + 0.6 R; on top of the enemy it knocks along the enemy's heading.
#[test]
fn the_eruption_misses_far_and_knocks_along_the_heading_on_top() {
    let roster = testing::roster();
    for (op, hit) in [([2.0, 2.0], false), ([-1.0, 0.5], true)] {
        let at = [[-1.0, 0.5], [0.0, 0.0], op, [0.3, -0.4]];
        let mut f = duel(&roster, [BCL, "FLUTE"], 0, at);
        trigger(&mut f, 0, Slot::Low);
        f.t = f.lights_out.balls[0].as_ref().expect("in shadow").t_end;
        let dt = f.rules.dt;
        lights_out_step(&mut f, 0, dt);
        assert!(f.lights_out.balls[0].is_none());
        assert_eq!(f.balls[0].shadow, 0.0);
        let erupt = ev(14.29412, "erupt", vec![I(0), F(-1.0), F(0.5), I(i64::from(hit))]);
        let o = &f.balls[1];
        if hit {
            let h = vec![I(0), I(1), F(12.0), F(-1.0), F(0.5), S("erupt".into()), I(0)];
            assert_eq!(f.events[2..], [ev(14.29412, "hit", h), erupt]);
            assert_eq!(
                (o.hp, o.v, o.boost),
                (88.0, [1.5599999999984397, -2.07999999999792], 1.8)
            );
        } else {
            assert_eq!(f.events[2..], [erupt]);
            assert_eq!((o.hp, o.v, o.boost), (100.0, [0.3, -0.4], 0.0));
        }
    }
}

/// A copied LIGHTS OUT (what IDENTITY CRISIS does): BASS CLARINET's ability started on FLUTE's ball in the right
/// corner of a fight against TROMBONE. It runs on the ability it was given; no `ability` event, no record key.
#[test]
fn a_copied_one_runs_on_a_ball_whose_fighter_has_none() {
    let roster = testing::roster();
    let lo = &testing::fighter(&roster, BCL).low;
    let at = [[1.0, 0.5], [-1.0, 2.0], [-0.5, 0.1], [0.5, -2.0]];
    let mut f = duel(&roster, ["TROMBONE", "FLUTE"], 1, at);
    start(&mut f, 1, lo);
    assert_eq!(f.events, [ev(12.5, "lights_out", vec![I(1), F(14.294117647058822)])]);
    assert_eq!(f.balls[1].used, [false; 3]);
    assert!(record_keys(&f).is_empty());
    let seen = sink(&mut f, 1, [0.004, 0.002]);
    let want = [
        ([-1.0414466466104548, 1.9787341615951948], 0.25, 1.7857843137254896),
        ([-1.0824412927959568, 1.956609528661817], 0.25, 1.777450980392155),
        ([-1.1229661463848604, 1.9336357035578178], 0.25, 1.7691176470588221),
    ];
    for (got, want) in seen.iter().zip(want) {
        steered(*got, want);
    }
    assert_eq!(seen.len(), 216);
    steered(seen[215], ([-2.2336154008312827, 0.104700721913968], 0.25, 0.0));
    let hit = vec![
        I(1),
        I(0),
        F(12.0),
        F(0.3640000000000007),
        F(0.5320000000000004),
        S("erupt".into()),
        I(0),
    ];
    assert_eq!(
        f.events[1..],
        [
            ev(14.3, "hit", hit),
            ev(14.3, "erupt", vec![I(1), F(1.0), F(0.5), I(1)]),
        ]
    );
    let o = &f.balls[0];
    assert_eq!(
        (o.hp, o.v, o.boost, f.balls[1].dealt),
        (88.0, [-2.5470453449491908, -0.522072802159824], 1.8, 12.0)
    );
    // the record has `LO` exactly when a fighter has a LIGHTS OUT of its own
    let g = duel(&roster, ["FLUTE", BCL], 1, at);
    assert_eq!(record_keys(&g), ["LO"]);
}
