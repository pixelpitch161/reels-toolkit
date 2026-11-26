//! The tune weapon's unit tests (and the `endless` ability's and a wave's `detune`). Every expected number is what
//! `fight.py` gave on the reference PC (CPython 3.13, numpy 2.5) for the same hand-built state and the same calls;
//! the Python script that prints them is `tools/studio_export/unit_checks/tune_unit_vectors.py`.
//!
//! The hand-built fight: `Fight('OBOE', 'FLUTE', seed=392)` (or the two the other way round) under the engine's
//! default rules at the example episode's tempo, the oboe at (-1, 0.5) moving (1, -2), the enemy at (1.2, -0.7)
//! moving (-0.5, 2), the clock at 12.5 s, `tune_setup` called; then what each test sets, and the same calls.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py;

use super::*;
use crate::abilities::{effects_step, trigger};
use crate::ball::Slot;
use crate::fight::Setup;
use crate::roster::FighterCfg;
use crate::rules::Rules;
use crate::testing;

/// The episode's beat, `60 / bpm`.
const BEAT: f64 = 0.44852941176470584;
/// `DT`.
const DT: f64 = 0.008333333333333333;
/// A 16th note, `beat / 4`.
const S16: f64 = 0.11213235294117646;
/// The time the hand-built cases start at.
const T: f64 = 12.5;
/// `dur_beats * beat`: the 8 beats a beam detunes for.
const DETUNE_S: f64 = 3.5882352941176467;

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

/// One event or a list of them.
trait Events {
    fn list(self) -> Vec<Event>;
}

impl Events for Event {
    fn list(self) -> Vec<Event> {
        vec![self]
    }
}

impl Events for Vec<Event> {
    fn list(self) -> Vec<Event> {
        self
    }
}

impl<const N: usize> Events for [Event; N] {
    fn list(self) -> Vec<Event> {
        self.into()
    }
}

/// The events are Python's, bit for bit.
#[track_caller]
fn same(got: impl Events, want: impl Events) {
    assert_eq!(got.list(), want.list());
}

/// A heading is Python's to 1e-12: the one number of these tests that is not Python's to the bit, the enemy's heading
/// after ILL WIND's knock (`-0.7222199464008384` here, `...389` in Python). It is made in the shared `Fight::knock`
/// (two `np.linalg.norm` calls in Python, plain sums here, ruling R25), whose own test holds the same bar.
#[track_caller]
fn near(got: [f64; 2], want: [f64; 2]) {
    assert!(
        (got[0] - want[0]).abs() <= 1e-12 && (got[1] - want[1]).abs() <= 1e-12,
        "{got:?} vs Python's {want:?}"
    );
}

/// OBOE against FLUTE (`oboe_left`) or FLUTE against OBOE, the balls where the tests put them, the oboe set up.
/// Returns the fight and the oboe's side.
fn duel(roster: &[FighterCfg], oboe_left: bool) -> (Fight<'_>, usize) {
    let (left, right) = if oboe_left {
        ("OBOE", "FLUTE")
    } else {
        ("FLUTE", "OBOE")
    };
    let ep = testing::episode_rules();
    let pair = Setup {
        fighters: [testing::fighter(roster, left), testing::fighter(roster, right)],
        rules: Rules::new(ep.bpm, ep.t_fight),
        seed: 392,
    };
    let mut f = testing::fight(&pair, false);
    let k = usize::from(!oboe_left);
    f.balls[k].p = [-1.0, 0.5];
    f.balls[1 - k].p = [1.2, -0.7];
    f.balls[k].v = [1.0, -2.0];
    f.balls[1 - k].v = [-0.5, 2.0];
    f.t = T;
    setup(&mut f, k);
    (f, k)
}

/// The shot of the hand-built fight with no lock: straight at the enemy.
const SHOT: [f64; 3] = [-0.438146833334794, 0.19353463636443302, 5.783838585499456];
/// Where that beam lands on the enemy at (0.6, -0.3).
const LANDS: [f64; 2] = [0.3126887215916563, -0.8267373437486307];
/// The enemy's heading after that beam's knock.
const KNOCKED: [f64; 2] = [2.597986814640727, 0.10229619218002407];

/// The oboe shoots (no lock) and the enemy steps into the beam; one step of the beam.
fn beam_lands<'a>(roster: &'a [FighterCfg], before: impl Fn(&mut Fight<'a>)) -> Fight<'a> {
    let (mut f, k) = duel(roster, true);
    before(&mut f);
    tick(&mut f, k, 3);
    f.balls[1].p = [0.6, -0.3];
    step(&mut f, k, DT);
    f
}

/// `tune_setup` (2436 to 2446): the depth is `depth0`, the aim points at the enemy, nobody is out of tune; the same
/// from either corner.
#[test]
fn the_setup_aims_at_the_enemy_from_either_corner() {
    let roster = testing::roster();
    for oboe_left in [true, false] {
        let (f, k) = duel(&roster, oboe_left);
        assert_eq!(f.balls[k].th, 5.783838585499456);
        assert_eq!(
            f.tune.balls[k],
            BallTune {
                depth: 0.4,
                ..BallTune::default()
            }
        );
        assert_eq!(f.tune.balls[1 - k], BallTune::default());
        for b in &f.balls {
            assert_eq!((b.detune_t, b.detune_d, b.detune_w), (0.0, 0.0, 0.0));
        }
        assert!(is(&f, k) && !is(&f, 1 - k));
    }
}

/// `tune_tick` (2476 to 2492) over 40 16ths: the aim locks 3 16ths before each shot (where the enemy is, less the
/// oboe's own motion until the shot), the shot falls 3 16ths after each bar line and lives `hold` seconds.
#[test]
fn the_grid_locks_three_16ths_before_each_shot() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    let lock = 6.007493136202604;
    let o = [-0.3841683134813877, 0.3257836578284297];
    let mut seen = Vec::new();
    for k16 in 0..40 {
        f.t = T + k16 as f64 * S16;
        let n0 = f.events.len();
        tick(&mut f, k, k16);
        if f.events.len() > n0 {
            seen.push(k16);
        }
        let locked = matches!(k16, 0..=2 | 16..=18 | 32..=34);
        assert_eq!(f.tune.balls[k].tune_lock, locked.then_some(lock), "k16 {k16}");
    }
    assert_eq!(seen, [0, 3, 16, 19, 32, 35]);
    let shot = |t: f64| ev(t, "tune", vec![I(0), F(o[0]), F(o[1]), F(lock), F(6.0), F(0.12)]);
    same(
        f.events.clone(),
        [
            ev(12.5, "tune_lock", vec![I(0), F(lock)]),
            shot(12.8364),
            ev(14.29412, "tune_lock", vec![I(0), F(lock)]),
            shot(14.63051),
            ev(16.08824, "tune_lock", vec![I(0), F(lock)]),
            shot(16.42463),
        ],
    );
    assert_eq!(f.balls[k].th, lock);
    let beams = &f.tune.balls[k].tbeams;
    assert_eq!(
        beams.iter().map(|b| b.t1).collect::<Vec<_>>(),
        [13.016397058823529, 14.810514705882353, 16.604632352941177]
    );
    for b in beams {
        assert_eq!(
            (b.o, b.d, b.hit),
            (o, [0.9622370101853317, -0.27221303464307856], false)
        );
    }
}

/// `tune_tick` 2493 to 2494: every 8th while the enemy is out of tune stings it for `sting` x the depth, at the
/// enemy's centre; nothing on the 16ths between, nothing once it is in tune again.
#[test]
fn the_oboes_8ths_sting_a_detuned_enemy() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    (f.balls[1].detune_t, f.balls[1].detune_d, f.balls[1].detune_w) = (1.0, 0.5, 0.6);
    for k16 in [4, 5, 6] {
        f.t = T + k16 as f64 * S16;
        tick(&mut f, k, k16);
    }
    let sting = |t: f64| ev(t, "hit", vec![I(0), I(1), F(0.375), F(1.2), F(-0.7), s("sting"), I(0)]);
    same(f.events.clone(), [sting(12.94853), sting(13.17279)]);
    assert_eq!((f.balls[1].hp, f.balls[0].dealt), (99.25, 0.75));
    assert_eq!((f.balls[1].v, f.balls[1].boost), ([-0.5, 2.0], 0.0));
    f.balls[1].detune_t = 0.0;
    tick(&mut f, k, 8);
    assert_eq!(f.events.len(), 2);
}

/// `tune_tick` 2455 to 2457: a stunned oboe drops its lock and starts nothing.
#[test]
fn a_stunned_oboe_drops_its_lock() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    f.tune.balls[k].tune_lock = Some(1.0);
    f.balls[k].stun = 0.5;
    tick(&mut f, k, 3);
    assert!(f.events.is_empty());
    assert_eq!(f.tune.balls[k].tune_lock, None);
    assert!(f.tune.balls[k].tbeams.is_empty());
    assert_eq!(f.balls[k].th, 5.783838585499456);
}

/// `tune_step` 2500 to 2507: between shots the aim turns to the enemy at `track` (4 radians a second); a locked aim
/// is held.
#[test]
fn the_aim_turns_at_its_rate_and_holds_a_lock() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    f.balls[k].th = 3.0;
    for want in [3.033333333333333, 3.0666666666666664, 3.0999999999999996] {
        step(&mut f, k, DT);
        assert_eq!(f.balls[k].th, want);
    }
    f.tune.balls[k].tune_lock = Some(5.5);
    step(&mut f, k, DT);
    assert_eq!(f.balls[k].th, 5.5);
}

/// `tune_step` 2508 to 2524: a beam that reaches the enemy hits once (`dmg`), knocks it along the beam, puts it
/// out of tune at the oboe's depth for 8 beats and deepens the next detune; it goes out `hold` seconds after the
/// shot. Then `damage` 344 to 346: the detuned fighter's own hits land `weak` x depth weaker.
#[test]
fn a_beam_lands_once_knocks_detunes_and_deepens() {
    let roster = testing::roster();
    let mut f = beam_lands(&roster, |_| {});
    let hit = vec![I(1), F(2.0), F(LANDS[0]), F(LANDS[1]), s("tune"), I(0)];
    same(
        f.events.clone(),
        [
            ev(
                12.5,
                "tune",
                vec![I(0), F(SHOT[0]), F(SHOT[1]), F(SHOT[2]), F(6.0), F(0.12)],
            ),
            ev(12.5, "hit", [vec![I(0)], hit].concat()),
            ev(12.5, "detune", vec![I(1), I(0), F(0.4), F(DETUNE_S)]),
        ],
    );
    assert_eq!((f.balls[0].th, f.tune.balls[0].depth), (5.8171719188327895, 0.5));
    let beam = &f.tune.balls[0].tbeams[0];
    assert_eq!(
        (beam.o, beam.d, beam.t1, beam.hit),
        (
            [SHOT[0], SHOT[1]],
            [0.8778955729143844, -0.47885213068057336],
            12.68,
            true
        )
    );
    let o = &f.balls[1];
    assert_eq!((o.hp, o.boost, f.balls[0].dealt), (98.0, 0.35, 2.0));
    assert_eq!(o.v, KNOCKED);
    assert_eq!((o.detune_t, o.detune_d, o.detune_w), (DETUNE_S, 0.4, 0.6));
    // a beam hits once
    step(&mut f, 0, DT);
    assert_eq!(f.events.len(), 3);
    // and goes out `hold` s after the shot
    f.t = T + 0.18;
    step(&mut f, 0, DT);
    assert!(f.tune.balls[0].tbeams.is_empty());
    assert_eq!(f.balls[0].th, 5.81953769817878);
    // the detuned attacker: 5 x (1 - 0.6 x 0.4)
    assert!(f.damage(1, 0, 5.0, "fang", [0.25, -0.5], 2));
    assert_eq!((f.balls[0].hp, f.balls[1].dealt), (96.2, 3.8));
    same(
        f.events[3].clone(),
        ev(
            12.68,
            "hit",
            vec![I(1), I(0), F(3.8), F(0.25), F(-0.5), s("fang"), I(2)],
        ),
    );
}

/// `tune_step` 2524: the depth stops at `depth_max` (the detune itself is at the depth before the growth).
#[test]
fn the_depth_stops_at_its_cap() {
    let roster = testing::roster();
    let f = beam_lands(&roster, |f| f.tune.balls[0].depth = 0.95);
    same(
        f.events[2].clone(),
        ev(12.5, "detune", vec![I(1), I(0), F(0.95), F(DETUNE_S)]),
    );
    assert_eq!(f.tune.balls[0].depth, 1.0);
}

/// A beam on a shielded enemy is blocked: the knock is given all the same (`knock` follows `damage` whether or not
/// it landed), nobody is put out of tune and the depth does not grow.
#[test]
fn a_shielded_enemy_blocks_the_beam_and_is_still_knocked() {
    let roster = testing::roster();
    let f = beam_lands(&roster, |f| f.balls[1].shield = 1.0);
    assert_eq!(f.events.len(), 2);
    same(
        f.events[1].clone(),
        ev(12.5, "block", vec![I(1), I(0), F(LANDS[0]), F(LANDS[1]), s("tune")]),
    );
    assert_eq!((f.tune.balls[0].depth, f.tune.balls[0].tbeams[0].hit), (0.4, true));
    let o = &f.balls[1];
    assert_eq!((o.hp, o.boost, f.balls[0].dealt), (100.0, 0.35, 0.0));
    assert_eq!(o.v, KNOCKED);
    assert_eq!((o.detune_t, o.detune_d, o.detune_w), (0.0, 0.0, 0.0));
}

/// `tune_step` 2517: a beam through the enemy's very centre has no side to push from: the contact is the enemy's
/// far side along the beam.
#[test]
fn a_beam_through_the_centre_pushes_along_itself() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    f.balls[0].p = [0.0, 0.0];
    f.balls[1].p = [2.0, 0.0];
    f.tune.balls[k].tune_lock = Some(0.0);
    tick(&mut f, k, 3);
    step(&mut f, k, DT);
    same(
        f.events.clone(),
        [
            ev(12.5, "tune", vec![I(0), F(0.64), F(0.0), F(0.0), F(6.0), F(0.12)]),
            ev(12.5, "hit", vec![I(0), I(1), F(2.0), F(2.6), F(0.0), s("tune"), I(0)]),
            ev(12.5, "detune", vec![I(1), I(0), F(0.4), F(DETUNE_S)]),
        ],
    );
    assert_eq!(f.balls[1].v, [2.517444142412154, 0.6499807611132181]);
}

/// `trigger`, kind `endless` (2884 to 2888), and the held beam in `tune_tick` (2458 to 2475) and `tune_step` (2505):
/// the lock and the live beams are dropped, the beam is held for 4 beats, turns at its own `track` (1.4), bites on
/// the 8ths when the enemy touches it (`dmg`, and out of tune at the oboe's depth), and ends with `endless_end`.
#[test]
fn the_endless_note_is_held_bites_on_the_8ths_and_ends() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    f.tune.balls[k].tune_lock = Some(1.0);
    tick(&mut f, k, 3);
    f.events.clear();
    trigger(&mut f, k, Slot::Unlock2);
    let until = 14.294117647058822;
    same(
        f.events.clone(),
        [
            ev(12.5, "ability", vec![I(0), s("ENDLESS NOTE"), s("endless"), I(1)]),
            ev(12.5, "endless", vec![I(0), F(until)]),
        ],
    );
    let st = &f.tune.balls[k];
    assert_eq!((st.tune_lock, st.tbeams.len(), st.depth), (None, 0, 0.4));
    assert_eq!(
        st.endless.as_ref().map(|e| (e.until, e.ab.name.as_str())),
        Some((until, "ENDLESS NOTE"))
    );
    assert_eq!((f.balls[k].th, f.balls[k].used), (1.0, [false, true, false]));
    f.balls[k].th = 3.0;
    for want in [3.0116666666666667, 3.0233333333333334] {
        step(&mut f, k, DT);
        assert_eq!(f.balls[k].th, want);
    }
    // the held beam on the enemy
    f.balls[k].th = py::fmod_py(dmath::atan2(-1.2, 2.2), 2.0 * dmath::PI);
    f.events.clear();
    let mut seen = Vec::new();
    for k16 in [1, 2, 3, 4] {
        f.t = T + k16 as f64 * S16;
        let n0 = f.events.len();
        tick(&mut f, k, k16);
        seen.push(f.events.len() - n0);
    }
    assert_eq!(seen, [0, 2, 0, 2]);
    let bite = |t: f64| {
        let (x, y) = (0.6732626562513694, -0.4126887215916559);
        [
            ev(t, "hit", vec![I(0), I(1), F(0.9), F(x), F(y), s("endless"), I(0)]),
            ev(t, "detune", vec![I(1), I(0), F(0.4), F(DETUNE_S)]),
        ]
    };
    same(f.events.clone(), [bite(12.72426), bite(12.94853)].concat());
    let o = &f.balls[1];
    assert_eq!(
        (o.hp, o.v, o.boost, f.balls[0].dealt),
        (98.19999999999999, [-0.5, 2.0], 0.0, 1.8)
    );
    assert_eq!((o.detune_t, o.detune_d, o.detune_w), (DETUNE_S, 0.4, 0.6));
    // off the enemy: no bite
    f.balls[k].th = 1.0;
    tick(&mut f, k, 6);
    assert_eq!(f.events.len(), 4);
    // its time is up
    f.t = T + 4.0 * BEAT;
    tick(&mut f, k, 16);
    same(f.events[4].clone(), ev(14.29412, "endless_end", vec![I(0)]));
    assert_eq!(f.tune.balls[k].endless, None);
}

/// ILL WIND (`trigger` 2845 to 2846, `effects_step` 3069 to 3071): the wave's `detune` puts the enemy it lands on
/// out of tune, 8 beats deep 1.0, its hits 0.45 weaker.
#[test]
fn the_ill_wind_detunes_the_enemy_it_lands_on() {
    let roster = testing::roster();
    let (mut f, k) = duel(&roster, true);
    trigger(&mut f, k, Slot::Low);
    let mut landed = None;
    for i in 0..80 {
        f.t = T + (i + 1) as f64 * DT;
        let n0 = f.events.len();
        effects_step(&mut f, DT);
        if f.events.len() > n0 {
            landed = Some((i + 1, f.t, f.events[n0..].to_vec()));
            break;
        }
    }
    let (step, t, events) = landed.expect("the ring reaches the enemy");
    assert_eq!((step, t), (43, 12.858333333333333));
    let (x, y) = (0.6732626562513695, -0.4126887215916559);
    same(
        events.clone(),
        [
            ev(12.85833, "hit", vec![I(0), I(1), F(8.0), F(x), F(y), s("wave"), I(0)]),
            ev(12.85833, "detune", vec![I(1), I(0), F(1.0), F(DETUNE_S)]),
        ],
    );
    let o = &f.balls[1];
    assert_eq!((o.hp, o.boost), (92.0, 1.5));
    near(o.v, [2.4976785920131728, -0.7222199464008389]);
    assert_eq!((o.detune_t, o.detune_d, o.detune_w), (DETUNE_S, 1.0, 0.45));
}

/// The record has `DET` and `TUNE` exactly when a fighter is a tuner.
#[test]
fn the_record_keys_follow_the_weapon() {
    let roster = testing::roster();
    let (f, _) = duel(&roster, false);
    assert_eq!(record_keys(&f), ["DET", "TUNE"]);
    let plain = testing::fight(&testing::episode_setup(&roster, 392), false);
    assert!(record_keys(&plain).is_empty());
}
