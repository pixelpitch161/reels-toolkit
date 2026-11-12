//! The combo's and the ability beam's unit tests. Every expected number is what `fight.py` gave on the reference PC
//! for the same hand-built state: `tools/studio_export/unit_checks/combo_unit_vectors.py` prints them (a
//! `Fight('TRUMPET', 'FLUTE', seed=392)` under the episode's rules, the balls placed as here, `combo_setup(0)`
//! called, then the same calls: `combo_beat(0, kb)`, `combo_step(0, dt)` once per step at `T + i * dt`; for HIGH C
//! `trigger(0, 'low')`, then `beams_step(dt)` and `abilities_step(False)` once per step).

use bf_core::event::{Event, Value};

use super::*;
use crate::abilities::{self, beam};
use crate::ball::Slot;
use crate::fight::{Key, Setup};
use crate::roster::FighterCfg;
use crate::testing;

use Value::{Float as F, Int as I};

/// The episode's beat, `60 / bpm`.
const BEAT: f64 = 0.44852941176470584;
/// `DT`.
const DT: f64 = 0.008333333333333333;
/// The time the hand-built combos start at.
const T: f64 = 12.5;
/// `atan2(0.75, 1.5)`: the aim from the TRUMPET at (-1, 0.25) to the enemy at (0.5, 1).
const AIM: f64 = 0.4636476090008061;

fn s(x: &str) -> Value {
    Value::Str(x.into())
}

/// TRUMPET (0) at (-1, 0.25) against FLUTE (1) at (0.5, 1), both still, at `T`, the combo set up.
fn duel(roster: &[FighterCfg]) -> Fight<'_> {
    let pair = Setup {
        fighters: [testing::fighter(roster, "TRUMPET"), testing::fighter(roster, "FLUTE")],
        rules: testing::episode_rules(),
        seed: 392,
    };
    let mut f = testing::fight(&pair, false);
    f.balls[0].p = [-1.0, 0.25];
    f.balls[1].p = [0.5, 1.0];
    f.balls[0].v = [0.0, 0.0];
    f.balls[1].v = [0.0, 0.0];
    f.t = T;
    setup(&mut f, 0);
    f
}

/// `combo_step(0, dt)` once per step at `T + i * dt`; the events of each step that had any, by step.
fn run_combo(f: &mut Fight, steps: usize) -> Vec<(usize, Vec<Event>)> {
    let mut out = Vec::new();
    for i in 0..steps {
        f.t = T + i as f64 * DT;
        let n0 = f.events.len();
        step(f, 0, DT);
        if f.events.len() > n0 {
            out.push((i, f.events[n0..].to_vec()));
        }
    }
    out
}

/// The steps' events equal Python's to the bit.
fn same(got: &[(usize, Vec<Event>)], want: &[(usize, Vec<Event>)]) {
    assert_eq!(got, want);
}

/// The one value here that is not Python's to the bit: the enemy's velocity after the blast's knock. The shared
/// `Fight::knock` takes the length of `d + 0.3 n` (not a unit vector) as a plain sum where `fight.py` has
/// `np.linalg.norm` (ruling R25), so the last bits can differ; 1e-12 holds it.
fn close(got: f64, want: f64) {
    assert!((got - want).abs() <= 1e-12, "{got} vs Python's {want}");
}

fn ev(t: f64, kind: &str, fields: Vec<Value>) -> Event {
    Event {
        t,
        kind: kind.into(),
        fields,
    }
}

fn punch(t: f64, i: i64, reach: f64) -> Event {
    ev(t, "punch", vec![I(0), I(i), F(AIM), F(reach), I(1)])
}

fn hit(t: f64, dmg: f64, x: f64, y: f64, i: i64) -> Event {
    ev(t, "hit", vec![I(0), I(1), F(dmg), F(x), F(y), s("piston"), I(i)])
}

#[test]
fn the_valves_start_facing_the_enemy() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    assert_eq!(f.balls[0].th, AIM);
    assert_eq!(f.balls[0].blade_len, 0.6);
    assert_eq!(f.combo.balls[0], BallState::default());
    assert!(is(&f, 0) && !is(&f, 1));
    // an enemy behind and below: the angle is kept in 0 to 2 pi
    f.balls[1].p = [-2.0, -1.0];
    setup(&mut f, 0);
    assert_eq!(f.balls[0].th, 4.037648038161137);
}

#[test]
fn a_combo_is_queued_every_two_beats_and_every_beat_under_haste() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    let queue = vec![12.5, 12.724264705882353, 12.948529411764707];
    for (kb, n) in [(0, 1), (1, 1), (2, 2), (3, 2)] {
        beat(&mut f, 0, kb);
        assert_eq!(f.combo.balls[0].punch_q, queue, "kb {kb}");
        assert_eq!(f.combo.balls[0].combo_n, n, "kb {kb}");
    }
    let combo = |n: i64, g: f64| ev(12.5, "combo", vec![I(0), I(n), I(3), F(g)]);
    assert_eq!(f.events, [combo(1, 0.22426470588235292), combo(2, 0.22426470588235292)]);
    // DOUBLE TONGUE: a combo on every beat, the punches half as far apart
    let mut f = duel(&roster);
    (f.balls[0].haste, f.balls[0].haste_mult) = (1.0, 2.0);
    for kb in 0..3 {
        beat(&mut f, 0, kb);
        assert_eq!(f.combo.balls[0].punch_q, [12.5, 12.612132352941176, 12.724264705882353]);
        assert_eq!(f.combo.balls[0].combo_n, kb + 1);
    }
    assert_eq!(f.events, [1, 2, 3].map(|n| combo(n, 0.11213235294117646)));
    assert_eq!(0.5 * BEAT / 2.0, 0.11213235294117646);
}

#[test]
fn three_punches_land_the_third_harder_and_the_reach_grows() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    f.balls[1].v = [0.0, 2.6];
    beat(&mut f, 0, 0);
    f.events.clear();
    let got = run_combo(&mut f, 80);
    // (the TRUMPET's hits take 1.2 x 2.0 = 2.4 and 2.0 x 2.0 = 4.0 under the episode's handicap for the left corner)
    let want = vec![
        (0, vec![punch(12.5, 0, 0.6)]),
        (3, vec![hit(12.525, 2.4, 0.04049619755342515, 0.614181058607617, 0)]),
        (27, vec![punch(12.725, 1, 0.65)]),
        (30, vec![hit(12.75, 2.4, -0.03665631459994945, 0.7316718427000253, 1)]),
        (54, vec![punch(12.95, 2, 0.7000000000000001)]),
        (57, vec![hit(12.975, 4.0, -0.08823714194638599, 0.8817753628267468, 2)]),
    ];
    same(&got, &want);
    let (b, o) = (&f.balls[0], &f.balls[1]);
    assert_eq!((b.th, b.blade_len), (AIM, 0.7500000000000001));
    let st = &f.combo.balls[0];
    assert!(st.punches.is_empty() && st.punch_q.is_empty());
    assert_eq!(st.punch_i, 3);
    assert_eq!((o.hp, o.boost, b.dealt), (91.19999999999999, 1.0, 8.8));
    assert_eq!(o.v, [2.316237261294855, 1.1811202095366027]);
}

#[test]
fn an_enemy_out_of_reach_is_missed() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    // further along the same line from the TRUMPET, so the aim is the same
    f.balls[1].p = [2.5, 2.0];
    beat(&mut f, 0, 0);
    f.events.clear();
    let got = run_combo(&mut f, 80);
    let want = vec![
        (0, vec![punch(12.5, 0, 0.6)]),
        (27, vec![punch(12.725, 1, 0.6)]),
        (54, vec![punch(12.95, 2, 0.6)]),
    ];
    same(&got, &want);
    assert_eq!((f.balls[0].blade_len, f.balls[0].th, f.balls[1].hp), (0.6, AIM, 100.0));
}

#[test]
fn a_shielded_enemy_blocks_and_the_piston_pulls_straight_back() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    f.balls[1].shield = 9.0;
    beat(&mut f, 0, 0);
    f.events.clear();
    let got = run_combo(&mut f, 10);
    let block = ev(
        12.525,
        "block",
        vec![I(1), I(0), F(0.04049619755342515), F(0.614181058607617), s("piston")],
    );
    same(&got, &[(0, vec![punch(12.5, 0, 0.6)]), (3, vec![block])]);
    let p = &f.combo.balls[0].punches[0];
    assert_eq!((p.t0, p.hit), (12.435, true));
    assert_eq!((f.balls[0].blade_len, f.balls[1].hp), (0.6, 100.0));
}

#[test]
fn the_reach_stops_at_its_longest() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    f.balls[0].blade_len = 1.68;
    beat(&mut f, 0, 0);
    f.events.clear();
    let got = run_combo(&mut f, 80);
    let want = vec![
        (0, vec![punch(12.5, 0, 1.68)]),
        (1, vec![hit(12.50833, 2.4, 0.05038831024681517, 0.602697434645476, 0)]),
        (27, vec![punch(12.725, 1, 1.7)]),
        (
            28,
            vec![hit(12.73333, 2.4, -0.03665631459994956, 0.7316718427000253, 1)],
        ),
        (54, vec![punch(12.95, 2, 1.7)]),
        (
            55,
            vec![hit(12.95833, 4.0, -0.08793701974514856, 0.8802917679806745, 2)],
        ),
    ];
    same(&got, &want);
    assert_eq!(f.balls[0].blade_len, 1.7);
}

#[test]
fn haste_runs_the_punches_twice_as_fast() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    (f.balls[0].haste, f.balls[0].haste_mult) = (5.0, 2.0);
    beat(&mut f, 0, 0);
    f.events.clear();
    let got = run_combo(&mut f, 40);
    let want = vec![
        (0, vec![punch(12.5, 0, 0.6)]),
        (2, vec![hit(12.51667, 2.4, 0.05402963433710534, 0.5986143588137414, 0)]),
        (14, vec![punch(12.61667, 1, 0.65)]),
        (
            16,
            vec![hit(12.63333, 2.4, -0.03665631459994945, 0.7316718427000252, 1)],
        ),
        (27, vec![punch(12.725, 2, 0.7000000000000001)]),
        (
            29,
            vec![hit(12.74167, 4.0, -0.09369613532259702, 0.9132538248508181, 2)],
        ),
    ];
    same(&got, &want);
    assert_eq!(f.balls[0].blade_len, 0.7500000000000001);
    assert!(f.combo.balls[0].punches.is_empty());
}

// ---------------------------------------------------------------- HIGH C (the ability beam)
/// `beams_step(dt)` (when a beam is live, as `run()` asks) and `abilities_step(False)` once per step from step 1;
/// the events of each step that had any.
fn run_beam(f: &mut Fight, steps: usize) -> Vec<(usize, Vec<Event>)> {
    let mut out = Vec::new();
    for i in 1..steps {
        f.t = T + i as f64 * DT;
        let n0 = f.events.len();
        if beam::live(f) {
            beam::beams_step(f, DT);
        }
        f.abilities_step(false);
        if f.events.len() > n0 {
            out.push((i, f.events[n0..].to_vec()));
        }
    }
    out
}

#[test]
fn high_c_locks_its_aim_on_where_the_enemy_will_be() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    f.balls[0].v = [1.5, -0.5];
    f.balls[1].v = [-0.8, 2.4];
    abilities::trigger(&mut f, 0, Slot::Low);
    let ch = f.beam.charge[0].clone().expect("a charge");
    assert_eq!((ch.aim, ch.t_fire), (1.3462506713228284, 12.948529411764707));
    assert_eq!(
        f.events,
        [
            ev(12.5, "ability", vec![I(0), s("HIGH C"), s("beam"), I(1)]),
            ev(
                12.5,
                "charge",
                vec![I(0), F(1.3462506713228284), F(BEAT), F(7.0), F(1.1)]
            ),
        ]
    );
    assert!(!beam::live(&f) && f.beam.charge[1].is_none());
}

#[test]
fn high_c_fires_on_its_beat_hits_once_and_clears_the_projectiles_in_its_path() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    abilities::trigger(&mut f, 0, Slot::Low);
    let key = |own: usize, p: [f64; 2], id: i64, vol: i64, idx: i64| Key {
        p,
        v: [0.0, 0.0],
        own,
        id,
        vol,
        idx,
        dmg: 1.0,
    };
    // an enemy projectile on the line, one far from it, and one of its own on the line
    f.keys = vec![
        key(1, [-0.2, 0.65], 0, 0, 0),
        key(1, [-1.0, -2.5], 1, 0, 1),
        key(0, [0.1, 0.8], 2, 1, 0),
    ];
    f.events.clear();
    let got = run_beam(&mut f, 120);
    let fired = ev(
        12.95,
        "beam",
        vec![
            I(0),
            F(-0.4275665977600538),
            F(0.5362167011199731),
            F(AIM),
            F(7.0),
            F(1.1),
        ],
    );
    // 9.0 x 2.0, the episode's handicap for the left corner
    let landed = ev(
        12.95833,
        "hit",
        vec![
            I(0),
            I(1),
            F(18.0),
            F(1.0366563145999494),
            F(1.2683281572999747),
            s("beam"),
            I(0),
        ],
    );
    let cleared = ev(12.95833, "cleared", vec![I(1), I(0), F(-0.2), F(0.65), I(0)]);
    let level = ev(12.95833, "level", vec![I(0), I(2)]);
    same(&got, &[(54, vec![fired]), (55, vec![landed, cleared, level])]);
    assert!(!beam::live(&f) && f.beam.charge[0].is_none());
    let (b, o) = (&f.balls[0], &f.balls[1]);
    // a beam grows no weapon
    assert_eq!((o.hp, o.boost, b.blade_len), (82.0, 1.6, 0.6));
    close(o.v[0], 2.3255106965968744);
    close(o.v[1], 1.1627553482984372);
    assert_eq!(f.keys.iter().map(|k| k.id).collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn the_blast_is_gone_after_its_hold() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    abilities::trigger(&mut f, 0, Slot::Low);
    let mut gone = None;
    for i in 1..120 {
        f.t = T + i as f64 * DT;
        if beam::live(&f) {
            beam::beams_step(&mut f, DT);
        }
        f.abilities_step(false);
        if gone.is_none() && f.beam.charge[0].is_none() && !beam::live(&f) {
            gone = Some(i);
        }
    }
    // fired at step 54, live for its 0.3 s
    assert_eq!(gone, Some(90));
}

#[test]
fn a_shielded_enemy_blocks_the_blast_and_is_still_knocked() {
    let roster = testing::roster();
    let mut f = duel(&roster);
    f.balls[1].shield = 9.0;
    abilities::trigger(&mut f, 0, Slot::Low);
    f.events.clear();
    let got = run_beam(&mut f, 120);
    let block = ev(
        12.95833,
        "block",
        vec![I(1), I(0), F(1.0366563145999494), F(1.2683281572999747), s("beam")],
    );
    assert_eq!(got.len(), 2);
    same(&got[1..], &[(55, vec![block])]);
    assert_eq!((f.balls[1].hp, f.balls[1].boost), (100.0, 1.6));
}
