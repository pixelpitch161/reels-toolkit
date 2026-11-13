//! Weapon kind `rests`, VIOLA's COUNTING RESTS: `fight.py` 5021 to 5107 (`rests_setup`, `fire_note`, `rests_tick`,
//! `_crush`), 5137 to 5181 (`notes_step`), and the abilities `double_stop` (5108 to 5117) and `melody` (`trigger` 2923
//! to 2927). The ball's `resting` and `guard` are what `damage` reads (342 to 343); `cur_kb`, `melody_until`,
//! `melody_k0` and `melody_rest` are on the ball too; `rest_count` and `note_dmg` are here, per ball.
//!
//! On every beat of its own 16th counter the fighter rests (`int(every) - 1` counted beats, taking x `guard` damage),
//! then fires a whole note: aimed ahead of the enemy (its predicted spot after the note's travel x `lead`, plus a
//! seeded wobble of up to `jitter` degrees: the kind's one draw from the fight's generator), it rolls at `roll` for
//! `hold` beats, bouncing off the ropes. It crushes the enemy once on contact and flattens the enemy's projectiles it
//! rolls over. A note may drop from above first (VIOLA JOKE): it lands on its spot `t_land`, crushes there, then rolls.
//!
//! Events: `note` (fighter, id, x, y, aim, radius, kind 0 / 1 / 2, t_land, t1), `rest` (fighter, count, 1 under MELODY
//! AT LAST), `crush` (fighter, id, where), hits of kind `note` whose sub-number is the note's kind, `crushed` (a
//! flattened projectile: its owner, the note's owner, where, its id), `double` (DOUBLE STOP), `melody` (fighter, until).
//! Recorded as `REST` (per ball: resting, count, MELODY AT LAST on) and `NOTES` (owner, x, y, radius, t0, t1, t_land,
//! id, kind, hit), both ragged JSON lists.

use bf_core::dmath;
use bf_core::event::Value;

use super::mortar;
use crate::abilities::ab_dur;
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2, np_clip, py_max, py_min};
use crate::record::Frames;
use crate::roster::{AbilityCfg, RestsCfg, WeaponCfg};

/// One entry of `self.notes`: `fire_note`'s dict.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Note {
    /// `id`: its number, counting up over the fight (`self.next_note`).
    pub id: i64,
    /// `owner`: the fighter that fired it.
    pub owner: usize,
    /// `c`: where it is.
    pub c: V2,
    /// `v`: its velocity (zero for a dropped note until it lands, then it rolls on with it).
    pub v: V2,
    /// `r`: its radius.
    pub r: f64,
    /// `t0`: when it appeared.
    pub t0: f64,
    /// `t_land`: when it lands (`t0` for a note that rolls at once).
    pub t_land: f64,
    /// `t1`: when it is gone (`t_land + hold` beats).
    pub t1: f64,
    /// `hit`: whether it has crushed the enemy.
    pub hit: bool,
    /// `landed`: whether it is on the canvas.
    pub landed: bool,
    /// `dmg`: its crush's damage.
    pub dmg: f64,
    /// `knock`: its crush's knockback.
    pub knock: f64,
    /// `kind`: 0 the weapon's own, 1 DOUBLE STOP's, 2 VIOLA JOKE's.
    pub kind: i64,
}

/// This kind's per-fight state (`Fight::rests`): the notes and what `rests_setup` hangs on a ball besides the ball's
/// own fields, indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `self.notes`: the whole notes rolling or dropping, in the order they were fired.
    pub notes: Vec<Note>,
    /// `self.next_note`: the next note's id.
    pub next_note: i64,
    /// `B.rest_count`: the rest being counted (0 when not resting; `getattr(B, 'rest_count', 0)` for a ball that never
    /// rests, the advisory of the pre-flight scan).
    pub rest_count: [i64; 2],
    /// `B.note_dmg`: the next note's damage.
    pub note_dmg: [f64; 2],
}

/// `B.w['kind'] == 'rests'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Rests(_))
}

/// Whether `self.notes` holds anything (Python's truth test of the list).
pub fn live(f: &Fight) -> bool {
    !f.rests.notes.is_empty()
}

/// Fighter `k`'s `rests` weapon (`B.w`).
///
/// # Panics
/// When its weapon is another kind: callers ask [`is`] first (a note is only ever fired by a rests fighter).
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a RestsCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Rests(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not rests", other.kind()),
    }
}

/// `rests_setup(k)` (5021 to 5033), from `run()` 5858 to 5860. The ball's `guard` is its weapon's (what `damage` reads
/// as `V.w.get('guard', 1.0)`).
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let b = &mut f.balls[k];
    b.blade_len = w.base;
    b.guard = w.guard;
    b.resting = false;
    b.cur_kb = -1;
    b.melody_until = -1.0;
    b.melody_k0 = 0;
    b.melody_rest = 1;
    f.rests.note_dmg[k] = w.dmg;
    f.rests.rest_count[k] = 0;
}

/// What `fire_note` is given besides the fighter: Python's keyword arguments, `None` for each left out.
#[derive(Clone, Copy, Debug, Default)]
struct Fire {
    tgt: Option<V2>,
    kind: i64,
    r: Option<f64>,
    dmg: Option<f64>,
    knock: Option<f64>,
    t_land: Option<f64>,
    v: Option<V2>,
}

/// `fire_note(k, tgt, kind, r, dmg, knock, t_land, v)` (5034 to 5062): spawns a whole note; by default the weapon's
/// own, aimed ahead of the enemy and rolling at once. Returns the new note's index in `notes`.
fn fire_note(f: &mut Fight, k: usize, a: Fire) -> usize {
    let w = weapon(f, k);
    let (bp, op) = (f.balls[k].p, f.balls[1 - k].p);
    let (rr, aa) = (f.rules.r, f.rules.a);
    let r = a.r.unwrap_or(f.balls[k].blade_len);
    let lim = aa - r;
    let clip = |p: V2| [np_clip(p[0], -lim, lim), np_clip(p[1], -lim, lim)];
    let (c, v, aim, t_land) = match a.t_land {
        None => {
            let tgt = match a.tgt {
                Some(t) => t,
                None => {
                    // tl = max(0.0, float(np.hypot(*d0)) - R - r) / w['roll']
                    let tl = py_max(0.0, geom::hypot(geom::sub(op, bp)) - rr - r) / w.roll;
                    mortar::predict(f, 1 - k, tl * w.lead)
                }
            };
            let d = geom::sub(tgt, bp);
            let aim = dmath::atan2(d[1], d[0]) + geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
            let u = [dmath::cos(aim), dmath::sin(aim)];
            // np.clip(B.p + u * (R + 0.6 * r), -lim, lim)
            let c = clip(geom::add(bp, geom::scale(u, rr + 0.6 * r)));
            (c, geom::scale(u, w.roll), aim, f.t)
        }
        Some(t_land) => {
            let c = clip(a.tgt.expect("a dropped note has its target"));
            let v = a.v.unwrap_or([0.0, 0.0]);
            let aim = if geom::hypot(v) > 1e-9 { dmath::atan2(v[1], v[0]) } else { 0.0 };
            (c, v, aim, t_land)
        }
    };
    let note = Note {
        id: f.rests.next_note,
        owner: k,
        c,
        v,
        r,
        t0: f.t,
        t_land,
        t1: t_land + w.hold * f.rules.beat,
        hit: false,
        landed: t_land <= f.t,
        dmg: a.dmg.unwrap_or(f.rests.note_dmg[k]),
        knock: a.knock.unwrap_or(w.knock),
        kind: a.kind,
    };
    f.rests.next_note += 1;
    let data = vec![
        Value::Int(note.id),
        Value::Float(c[0]),
        Value::Float(c[1]),
        Value::Float(aim),
        Value::Float(note.r),
        Value::Int(note.kind),
        Value::Float(note.t_land),
        Value::Float(note.t1),
    ];
    f.rests.notes.push(note);
    f.ev("note", k as i64, data);
    f.rests.notes.len() - 1
}

/// `rests_tick(k, k16)` (5064 to 5086), from `run()` 6045 to 6049 (its own counter, `n16v`): on every beat, a rest or
/// a note.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if k16 % 4 != 0 {
        return;
    }
    let kb = k16.div_euclid(4);
    f.balls[k].cur_kb = kb;
    if stunned(f, k) {
        f.balls[k].resting = false;
        f.rests.rest_count[k] = 0;
        return;
    }
    let b = &f.balls[k];
    let mel = f.t < b.melody_until;
    let (cyc, j) = if mel {
        // MELODY AT LAST: shorter rests
        let cyc = b.melody_rest + 1;
        (cyc, (kb - b.melody_k0).rem_euclid(cyc))
    } else {
        let cyc = weapon(f, k).every.trunc() as i64;
        (cyc, kb.rem_euclid(cyc))
    };
    if j < cyc - 1 {
        f.balls[k].resting = true;
        f.rests.rest_count[k] = j + 1;
        f.ev("rest", k as i64, vec![Value::Int(j + 1), Value::Int(i64::from(mel))]);
    } else {
        f.balls[k].resting = false;
        f.rests.rest_count[k] = 0;
        fire_note(f, k, Fire::default());
    }
}

/// `_crush(N, landed_now)` (5087 to 5107): note `i` crushes the enemy if it touches it (once): the hit, a knock along
/// its roll, and a landed hit of the weapon's own grows the next notes.
fn crush(f: &mut Fight, i: usize) {
    let n = f.rests.notes[i].clone();
    let (k, o) = (n.owner, 1 - n.owner);
    let rr = f.rules.r;
    let rel = geom::sub(f.balls[o].p, n.c);
    let dist = geom::hypot(rel);
    if dist >= n.r + rr * 0.9 {
        return;
    }
    f.rests.notes[i].hit = true;
    let u = geom::div(rel, py_max(dist, 1e-6));
    let [hx, hy] = geom::add(n.c, geom::scale(u, n.r));
    f.ev(
        "crush",
        k as i64,
        vec![Value::Int(n.id), Value::Float(hx), Value::Float(hy)],
    );
    let landed = f.damage(k, o, n.dmg, "note", [hx, hy], n.kind);
    let vn = geom::hypot(n.v);
    // away = u * 0.6 + (N['v'] / vn if vn > 1e-9 else u) * 0.4
    let along = if vn > 1e-9 { geom::div(n.v, vn) } else { u };
    let away = geom::add(geom::scale(u, 0.6), geom::scale(along, 0.4));
    f.knock(o, away, n.knock);
    if landed && is(f, k) {
        // every landed note grows the next ones
        let w = weapon(f, k);
        let b = &mut f.balls[k];
        b.blade_len = py_min(w.max_reach, b.blade_len + w.grow);
        f.rests.note_dmg[k] = py_min(w.dmg_max, f.rests.note_dmg[k] + w.dmg_grow);
    }
}

/// `notes_step(dt)` (5137 to 5181), from `run()` 6191 to 6192 (`if getattr(self, 'notes', None)`): a dropped note
/// lands and crushes on its spot; a rolling one rolls, bounces off the ropes, crushes, and flattens the enemy's
/// projectiles under it; a note past its `t1` is gone.
pub fn notes_step(f: &mut Fight, dt: f64) {
    if f.rests.notes.is_empty() {
        return;
    }
    let aa = f.rules.a;
    for i in 0..f.rests.notes.len() {
        let n = &mut f.rests.notes[i];
        if !n.landed {
            if f.t + 1e-9 < n.t_land {
                continue;
            }
            // a dropped note lands: crush on the spot
            n.landed = true;
            if !n.hit {
                crush(f, i);
            }
            continue;
        }
        n.c = geom::add(n.c, geom::scale(n.v, dt));
        let lim = aa - n.r;
        for ax in 0..2 {
            if n.c[ax].abs() > lim {
                n.c[ax] = lim.copysign(n.c[ax]) * 2.0 - n.c[ax];
                n.v[ax] = -n.v[ax];
            }
        }
        if !n.hit {
            crush(f, i);
        }
        if !f.keys.is_empty() {
            // it rolls over the enemy's projectiles
            let n = &f.rests.notes[i];
            let mask: Vec<bool> = f
                .keys
                .iter()
                .map(|key| key.own != n.owner && geom::hypot(geom::sub(key.p, n.c)) < n.r)
                .collect();
            let by = n.owner as i64;
            if mask.contains(&true) {
                f.kill_keys(&mask, "crushed", by);
            }
        }
    }
    let t = f.t;
    f.rests.notes.retain(|n| t < n.t1);
}

/// `trigger`, kind `double_stop` (2928 to 2929): `start_double_stop` (5108 to 5117): two whole notes at once instead
/// of this beat's rest, one at the enemy, one where it is heading.
pub fn start_double_stop(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let o = 1 - k;
    f.balls[k].resting = false;
    f.rests.rest_count[k] = 0;
    let w = weapon(f, k);
    let (bp, op) = (f.balls[k].p, f.balls[o].p);
    let tl = py_max(0.0, geom::hypot(geom::sub(op, bp)) - f.rules.r - f.balls[k].blade_len) / w.roll;
    fire_note(f, k, Fire { tgt: Some(op), kind: 1, ..Fire::default() });
    // O.p + O.v * tl * ab.get('lead2', 1.6)
    let (op, ov) = (f.balls[o].p, f.balls[o].v);
    let lead2 = ab.get("lead2").unwrap_or(1.6);
    let tgt = geom::add(op, geom::scale(geom::scale(ov, tl), lead2));
    fire_note(f, k, Fire { tgt: Some(tgt), kind: 1, ..Fire::default() });
    f.ev("double", k as i64, Vec::new());
}

/// `trigger`, kind `melody` (2923 to 2927): for its duration and half a beat the rests shrink (`melody_until`,
/// counted from the beat after it fires, `melody_k0`; `int(ab.get('rest', 1))` rests a note), and the `melody` event.
pub fn start_melody(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let until = f.t + ab_dur(f, ab) + 0.5 * f.rules.beat;
    let b = &mut f.balls[k];
    b.melody_until = until;
    b.melody_k0 = b.cur_kb + 1;
    b.melody_rest = ab.get("rest").unwrap_or(1.0).trunc() as i64;
    f.ev("melody", k as i64, vec![Value::Float(until)]);
}

/// `start_joke`'s note (5131 to 5136, for `abilities::joke`): the biggest whole note dropped on the enemy's spot,
/// landing `tele` beats later, then rolling on along the enemy's old heading for a whole note; and the `joke` event.
pub(crate) fn joke_note(f: &mut Fight, k: usize, param: impl Fn(&str) -> f64) {
    let o = 1 - k;
    let w = weapon(f, k);
    let ov = f.balls[o].v;
    let vn = geom::hypot(ov);
    // v = O.v / vn * B.w['roll'] if vn > 1e-9 else np.zeros(2)
    let v = if vn > 1e-9 { geom::scale(geom::div(ov, vn), w.roll) } else { [0.0, 0.0] };
    let r = py_max(f.balls[k].blade_len, w.base) * param("size");
    let fire = Fire {
        tgt: Some(f.balls[o].p),
        kind: 2,
        r: Some(r),
        dmg: Some(param("dmg")),
        knock: Some(param("knock")),
        t_land: Some(f.t + param("tele") * f.rules.beat),
        v: Some(v),
    };
    let i = fire_note(f, k, fire);
    let n = &f.rests.notes[i];
    let data = vec![Value::Int(o as i64), Value::Float(n.t_land), Value::Float(n.c[0]), Value::Float(n.c[1])];
    f.ev("joke", k as i64, data);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5862 to 5863: `REST` and `NOTES` when
/// a fighter rests.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) {
        &["REST", "NOTES"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `REST` and `NOTES` (5616 to 5620), into `out`.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let rest = (0..2).map(|k| {
        let b = &f.balls[k];
        Value::List(vec![
            Value::Float(if b.resting { 1.0 } else { 0.0 }),
            Value::Int(f.rests.rest_count[k]),
            Value::Float(if f.t < b.melody_until { 1.0 } else { 0.0 }),
        ])
    });
    out.push_nested("REST", Value::List(rest.collect()));
    let notes = f.rests.notes.iter().map(|n| {
        Value::List(vec![
            Value::Int(n.owner as i64),
            Value::Float(n.c[0]),
            Value::Float(n.c[1]),
            Value::Float(n.r),
            Value::Float(n.t0),
            Value::Float(n.t1),
            Value::Float(n.t_land),
            Value::Int(n.id),
            Value::Int(n.kind),
            Value::Int(i64::from(n.hit)),
        ])
    });
    out.push_nested("NOTES", Value::List(notes.collect()));
}
