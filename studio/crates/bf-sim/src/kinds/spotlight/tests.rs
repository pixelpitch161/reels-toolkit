//! The spotlight weapon's unit tests (and the abilities `octave`, `altissimo` and `encore`). Every expected line is
//! what `fight.py` gave on the reference PC (CPython 3.13, numpy 2.5) for the same hand-built state and the same
//! calls: `vectors.txt`, the output of `tools/studio_export/unit_checks/spotlight_unit_vectors.py`.
//!
//! The hand-built fight: `Fight('ALTO SAX', 'FLUTE', seed=392)` (or the two the other way round) under the engine's
//! default rules at the example episode's tempo, the alto at (-1, 0.5) moving (1, -2), the enemy at (1.2, -0.7)
//! moving (-0.5, 2), the clock at 12.5 s, `spot_setup` called; then what each test sets, and the same calls. After
//! every call the whole state of the kind is one line, compared with Python's bit for bit: the light's radius and
//! the burn, the abilities' ends, every light (its numbers, where it is now, its line), the phone lights, the
//! enemy's HP, the damage dealt and the events since the line before.
//!
//! The enemy's heading after a knock (made in the shared `Fight::knock`, `np.linalg.norm` in Python, whose sum is
//! fused on the reference PC, as `geom::norm` is) is compared bit for bit too, on its own line after each state
//! line. The tests put the heading back by hand before a call that reads it.

use bf_core::event::Value;

use super::*;
use crate::abilities::trigger;
use crate::ball::Slot;
use crate::fight::Setup;
use crate::roster::FighterCfg;
use crate::rules::Rules;
use crate::testing;

/// Python's lines.
const VECTORS: &str = include_str!("vectors.txt");
/// The time the hand-built cases start at.
const T: f64 = 12.5;

fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn opt(x: Option<f64>) -> String {
    x.map_or("none".to_string(), bits)
}

fn tok(v: &Value) -> String {
    match v {
        Value::Int(n) => format!("i{n}"),
        Value::Float(x) => format!("f{}", bits(*x)),
        Value::Str(s) => format!("s{s}"),
        other => panic!("an event field the vectors have no word for: {other:?}"),
    }
}

/// The lines of one case of `vectors.txt`: `(label, state, [vx, vy, boost])`.
fn lines(name: &str) -> Vec<(String, String, [f64; 3])> {
    let mut out = Vec::new();
    let mut on = false;
    let mut state: Option<(String, String)> = None;
    for line in VECTORS.lines() {
        if let Some(case) = line.strip_prefix("== ") {
            on = case == name;
        } else if !on {
        } else if let Some((label, text)) = line.split_once(" | ") {
            state = Some((label.to_string(), text.to_string()));
        } else if let Some((label, text)) = line.split_once(" ~ ") {
            let (was, text_was) = state.take().expect("a state line before its heading line");
            assert_eq!(was, label, "{name}: the two lines of a step");
            let v: Vec<f64> = text.split(' ').map(|x| x.parse().expect("a number")).collect();
            out.push((was, text_was, [v[0], v[1], v[2]]));
        } else {
            panic!("{name}: a line that is neither a state nor a heading: {line}");
        }
    }
    assert!(!out.is_empty(), "vectors.txt has no case {name:?}");
    out
}

/// A hand-built fight and Python's lines for it.
struct Case<'a> {
    name: &'static str,
    f: Fight<'a>,
    /// The alto's side.
    k: usize,
    /// The events already compared.
    seen: usize,
    want: Vec<(String, String, [f64; 3])>,
    /// The next line of `want`.
    at: usize,
}

impl<'a> Case<'a> {
    /// ALTO SAX against FLUTE (`alto_left`) or FLUTE against ALTO SAX, the balls where the tests put them, the alto
    /// set up.
    fn new(roster: &'a [FighterCfg], name: &'static str, alto_left: bool) -> Case<'a> {
        let (left, right) = if alto_left {
            ("ALTO SAX", "FLUTE")
        } else {
            ("FLUTE", "ALTO SAX")
        };
        let ep = testing::episode_rules();
        let pair = Setup {
            fighters: [testing::fighter(roster, left), testing::fighter(roster, right)],
            rules: Rules::new(ep.bpm, ep.t_fight),
            seed: 392,
        };
        let mut f = testing::fight(&pair, false);
        let k = usize::from(!alto_left);
        f.balls[k].p = [-1.0, 0.5];
        f.balls[1 - k].p = [1.2, -0.7];
        f.balls[k].v = [1.0, -2.0];
        f.balls[1 - k].v = [-0.5, 2.0];
        f.t = T;
        setup(&mut f, k);
        let seen = f.events.len();
        Case {
            name,
            f,
            k,
            seen,
            want: lines(name),
            at: 0,
        }
    }

    /// The kind's whole state as the Python script writes it.
    fn state(&mut self) -> String {
        let (f, k) = (&self.f, self.k);
        let (b, o) = (&f.balls[k], &f.balls[1 - k]);
        let s = &f.spotlight;
        let spots: Vec<String> = s
            .spots
            .iter()
            .map(|sp| {
                let c = spot_pos(sp, f.t);
                [
                    sp.id.to_string(),
                    sp.owner.to_string(),
                    sp.kind.to_string(),
                    sp.hot.to_string(),
                    sp.passes.to_string(),
                    i64::from(sp.grow).to_string(),
                    i64::from(sp.hit).to_string(),
                    bits(sp.t0),
                    bits(sp.t1),
                    bits(sp.r),
                    bits(sp.dmg),
                    bits(c[0]),
                    bits(c[1]),
                    bits(sp.a[0]),
                    bits(sp.a[1]),
                    bits(sp.b[0]),
                    bits(sp.b[1]),
                ]
                .join(":")
            })
            .collect();
        let phones: Vec<String> = s
            .phones
            .iter()
            .map(|&(t, q)| [bits(t), bits(q[0]), bits(q[1])].join(":"))
            .collect();
        let events: Vec<String> = f.events[self.seen..]
            .iter()
            .map(|e| {
                let fields: Vec<String> = e.fields.iter().map(tok).collect();
                format!("{}:{}:{}", bits(e.t), e.kind, fields.join(","))
            })
            .collect();
        self.seen = f.events.len();
        format!(
            "bl={} burn={} oct={} alt={} enc={} spots=[{}] phones=[{}] hp={} dealt={} events=[{}]",
            bits(b.blade_len),
            bits(s.burn[k]),
            opt(s.oct_until[k]),
            opt(s.alt[k].map(|a| a.until)),
            opt(s.encore[k].map(|e| e.t1)),
            spots.join(";"),
            phones.join(";"),
            bits(o.hp),
            bits(b.dealt),
            events.join(";")
        )
    }

    /// The state now is Python's next line, and the enemy's heading and boost are its next heading line's.
    #[track_caller]
    fn step(&mut self, label: &str) {
        let got = self.state();
        let (name, at) = (self.name, self.at);
        let (want_label, want, v) = self
            .want
            .get(at)
            .unwrap_or_else(|| panic!("{name}: Python has no line {at} ({label})"));
        assert_eq!(want_label, label, "{name}: line {at}");
        assert_eq!(&got, want, "{name}: {label}");
        let o = &self.f.balls[1 - self.k];
        for (got, want) in [o.v[0], o.v[1], o.boost].into_iter().zip(v) {
            assert!(
                got.to_bits() == want.to_bits(),
                "{name}: {label}: the enemy's heading or boost is {got:?}, Python's {want:?}"
            );
        }
        self.at += 1;
    }

    /// Every line of the case was compared.
    #[track_caller]
    fn done(self) {
        assert_eq!(self.at, self.want.len(), "{}: lines compared", self.name);
    }

    /// The enemy as the tests leave it before a call that reads its heading: `O.v = (-0.5, 2.0)`, `O.boost = 0.0`.
    fn rest_enemy(&mut self) {
        let o = &mut self.f.balls[1 - self.k];
        o.v = [-0.5, 2.0];
        o.boost = 0.0;
    }

    /// `O.p = spot_pos(spots[0], t) + off`.
    fn enemy_on_light(&mut self, off: V2) {
        let c = spot_pos(&self.f.spotlight.spots[0], self.f.t);
        self.f.balls[1 - self.k].p = geom::add(c, off);
    }
}

/// `spot_setup` (4387 to 4393): the light starts `base` wide and burns `dmg`, on either side.
#[test]
fn setup_gives_the_light_its_radius_and_its_burn() {
    let roster = testing::roster();
    for (name, left) in [("setup", true), ("setup, the right corner", false)] {
        let mut c = Case::new(&roster, name, left);
        assert_eq!((c.f.balls[c.k].blade_len, c.f.spotlight.burn[c.k]), (0.42, 1.3));
        c.step("set up");
        c.done();
    }
}

/// `spot_tick` and `spot_step` over one sweep (4399 to 4500): the line on the grid with its jitter draw, a burn and
/// its knock, a burn at the light's centre (the knock along the enemy's own heading), a miss, a shielded enemy
/// (`block`, then `burn`), and the sweep's end with the growth of the light and the burn.
#[test]
fn a_sweep_burns_on_the_16ths_and_grows_when_it_ends() {
    let roster = testing::roster();
    let mut c = Case::new(&roster, "sweep", true);
    let k = c.k;
    let s16 = c.f.rules.beat / 4.0;
    let dt = c.f.rules.dt;
    tick(&mut c.f, k, 0);
    c.step("a sweep on the grid");
    c.f.t = T + s16;
    tick(&mut c.f, k, 1);
    c.step("the next 16th");
    c.f.t = T + 3.0 * s16;
    c.enemy_on_light([0.1, 0.05]);
    tick(&mut c.f, k, 3);
    c.step("a burn");
    c.f.t = T + 4.0 * s16;
    c.rest_enemy();
    // `O.p = spot_pos(...).copy()`: the light's own place, not that place plus zero
    c.f.balls[1 - k].p = spot_pos(&c.f.spotlight.spots[0], c.f.t);
    tick(&mut c.f, k, 4);
    c.step("a burn at the centre");
    c.f.t = T + 5.0 * s16;
    c.f.balls[1 - k].p = [3.0, 3.0];
    tick(&mut c.f, k, 5);
    c.step("a miss");
    c.f.t = T + 6.0 * s16;
    c.f.balls[1 - k].shield = 1.0;
    c.enemy_on_light([0.1, 0.05]);
    tick(&mut c.f, k, 6);
    c.step("a shielded enemy");
    step(&mut c.f, k, dt);
    c.step("a step while it runs");
    c.f.t = c.f.spotlight.spots[0].t1 + dt;
    step(&mut c.f, k, dt);
    c.step("its end");
    assert!(c.f.spotlight.spots.is_empty());
    c.done();
}

/// The growth rule and its caps (4476 to 4479): 14 sweeps that landed take the radius from 0.42 to `max_r` and the
/// burn from 1.3 to `dmg_max`; a sweep that missed grows nothing.
#[test]
fn the_light_and_the_burn_grow_to_their_caps() {
    let roster = testing::roster();
    let mut c = Case::new(&roster, "growth", true);
    let k = c.k;
    let dt = c.f.rules.dt;
    for j in 0..14 {
        c.f.t = T + f64::from(j) * 2.0;
        tick(&mut c.f, k, 0);
        c.f.spotlight.spots.last_mut().expect("the sweep").hit = true;
        c.f.t += 1.0;
        step(&mut c.f, k, dt);
        c.step(&format!("sweep {j} landed"));
    }
    assert_eq!((c.f.balls[k].blade_len, c.f.spotlight.burn[k]), (0.75, 2.1));
    c.f.t = T + 40.0;
    tick(&mut c.f, k, 0);
    c.f.t += 1.0;
    step(&mut c.f, k, dt);
    c.step("a sweep that missed");
    c.done();
}

/// When `spot_tick` starts no sweep (4428 to 4436): off the weapon's grid, stunned, smashed; and a haste's grid.
#[test]
fn no_sweep_off_the_grid_or_for_a_fighter_that_cannot_attack() {
    let roster = testing::roster();
    let mut c = Case::new(&roster, "no sweep", true);
    let k = c.k;
    tick(&mut c.f, k, 8);
    c.step("off the grid");
    c.f.balls[k].stun = 1.0;
    tick(&mut c.f, k, 0);
    c.step("stunned");
    c.f.balls[k].stun = 0.0;
    c.f.balls[k].smashed = 1.0;
    tick(&mut c.f, k, 0);
    c.step("smashed");
    assert!(c.f.spotlight.spots.is_empty());
    c.f.balls[k].smashed = 0.0;
    c.f.balls[k].haste = 1.0;
    c.f.balls[k].haste_mult = 2.0;
    tick(&mut c.f, k, 8);
    c.step("a haste halves the grid");
    tick(&mut c.f, k, 4);
    c.step("and no further");
    assert_eq!(c.f.spotlight.spots.len(), 1);
    c.done();
}

/// ALTISSIMO (`trigger` 2961 to 2963, `spot_tick` 4443 to 4446): a narrow, hot light on a shorter line, its burn, and
/// a plain sweep once it has run out.
#[test]
fn altissimo_narrows_the_light_and_heats_the_burn() {
    let roster = testing::roster();
    let mut c = Case::new(&roster, "altissimo", true);
    let k = c.k;
    let s16 = c.f.rules.beat / 4.0;
    let dt = c.f.rules.dt;
    trigger(&mut c.f, k, Slot::Unlock2);
    c.step("fired");
    tick(&mut c.f, k, 0);
    c.step("a hot sweep");
    c.f.t = T + 3.0 * s16;
    c.enemy_on_light([0.05, 0.02]);
    tick(&mut c.f, k, 3);
    c.step("a hot burn");
    c.rest_enemy();
    c.f.t = c.f.spotlight.alt[k].expect("ALTISSIMO was fired").until;
    step(&mut c.f, k, dt);
    tick(&mut c.f, k, 0);
    c.step("a sweep when it has run out");
    c.done();
}

/// OCTAVE KEY (`trigger` 2958 to 2960, `spot_tick` 4453 to 4459): a second light, square to the first, there and
/// back in the same time (`spot_pos`); neither light's end grows anything unless the sweep itself burned.
#[test]
fn the_octave_key_adds_a_light_that_comes_back() {
    let roster = testing::roster();
    let mut c = Case::new(&roster, "octave", true);
    let k = c.k;
    let s16 = c.f.rules.beat / 4.0;
    let dt = c.f.rules.dt;
    trigger(&mut c.f, k, Slot::Unlock1);
    c.step("fired");
    tick(&mut c.f, k, 0);
    c.step("two lights");
    for j in 1..7 {
        c.f.t = T + f64::from(j) * s16;
        c.step(&format!("16th {j}"));
    }
    c.f.t = c.f.spotlight.spots[0].t1 + dt;
    c.f.spotlight.spots[1].hit = true;
    step(&mut c.f, k, dt);
    c.step("their end");
    c.f.t = c.f.spotlight.oct_until[k].expect("OCTAVE KEY was fired");
    tick(&mut c.f, k, 0);
    c.step("a sweep when it has run out");
    c.done();
}

/// ENCORE (`start_encore` 4501 to 4512, `spot_step` 4474 to 4488, `spot_tick` 4419 to 4427), on either side: the
/// sweep gives way to the chase light, which closes in step by step and burns; the phone lights come on the 8ths
/// (two draws each) and burn; no sweep starts; at its end the phones go out, and the light a step later; then the
/// sweeps start again.
#[test]
fn encore_chases_the_enemy_and_the_phones_join_in() {
    let roster = testing::roster();
    for (name, left) in [("encore", true), ("encore, the right corner", false)] {
        let mut c = Case::new(&roster, name, left);
        let k = c.k;
        let dt = c.f.rules.dt;
        tick(&mut c.f, k, 0);
        c.step("a sweep");
        trigger(&mut c.f, k, Slot::Low);
        c.step("fired");
        for j in 0..5 {
            c.f.t += dt;
            step(&mut c.f, k, dt);
            c.step(&format!("chase step {j}"));
        }
        tick(&mut c.f, k, 1);
        c.step("an odd 16th");
        tick(&mut c.f, k, 2);
        c.step("an 8th: the phones");
        // O.p = f.spots[0]['c'] + np.array([0.05, 0.0])
        c.f.balls[1 - k].p = geom::add(c.f.spotlight.spots[0].c, [0.05, 0.0]);
        tick(&mut c.f, k, 3);
        c.step("the chase light burns");
        c.rest_enemy();
        tick(&mut c.f, k, 4);
        c.step("the phones again");
        c.rest_enemy();
        tick(&mut c.f, k, 16);
        c.step("no sweep on the grid");
        c.f.t += dt;
        step(&mut c.f, k, dt);
        c.step("the light closes in");
        c.f.t = c.f.spotlight.encore[k].expect("ENCORE runs").t1;
        step(&mut c.f, k, dt);
        c.step("its end");
        assert!(c.f.spotlight.encore[k].is_none() && c.f.spotlight.phones.is_empty());
        c.f.t += dt;
        step(&mut c.f, k, dt);
        c.step("the step after");
        assert!(c.f.spotlight.spots.is_empty());
        c.rest_enemy();
        tick(&mut c.f, k, 0);
        c.step("sweeps again");
        c.done();
    }
}

fn sweep(passes: i64) -> Spot {
    Spot {
        owner: 0,
        id: 0,
        a: [-1.0, 0.5],
        b: [1.0, -0.5],
        c: [9.0, 9.0],
        t0: 10.0,
        t1: 12.0,
        r: 0.42,
        dmg: 1.3,
        hit: false,
        kind: 0,
        hot: 0,
        passes,
        grow: true,
    }
}

#[test]
fn a_sweep_is_eased_along_its_line_and_the_octave_light_comes_back() {
    let one = sweep(1);
    assert_eq!(spot_pos(&one, 9.0), [-1.0, 0.5]);
    assert_eq!(spot_pos(&one, 11.0), [0.0, 0.0]);
    assert_eq!(spot_pos(&one, 12.5), [1.0, -0.5]);
    // a quarter of the way: u = 0.25, eased 0.25 * 0.25 * 2.5 = 0.15625
    assert_eq!(spot_pos(&one, 10.5), [-1.0 + 2.0 * 0.15625, 0.5 - 0.15625]);
    let two = sweep(2);
    assert_eq!(spot_pos(&two, 11.0), [1.0, -0.5]);
    assert_eq!(spot_pos(&two, 12.0), [-1.0, 0.5]);
    assert_eq!(spot_pos(&two, 10.5), [0.0, 0.0]);
}

#[test]
fn the_chase_light_is_where_it_stands_and_a_light_with_no_time_is_at_its_end() {
    let mut chase = sweep(1);
    chase.kind = 2;
    assert_eq!(spot_pos(&chase, 11.0), [9.0, 9.0]);
    let mut flash = sweep(1);
    flash.t1 = flash.t0;
    assert_eq!(spot_pos(&flash, 10.0), [-1.0, 0.5]);
    assert_eq!(spot_pos(&flash, 10.0 + 1e-9), [1.0, -0.5]);
}

#[test]
fn a_line_is_held_inside_the_ropes() {
    let (a, b) = spot_line(3.5, [0.0, 0.0], [1.0, 0.0], 2.6);
    assert_eq!((a, b), ([-1.3, 0.0], [1.3, 0.0]));
    let (a, b) = spot_line(3.5, [3.0, -3.2], [0.6, 0.8], 2.0);
    assert_eq!(a, [2.4, -3.25]);
    assert_eq!(b, [3.25, -3.2 + 0.8 * 2.0 / 2.0]);
}
