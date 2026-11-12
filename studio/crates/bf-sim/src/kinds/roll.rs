//! Weapon kind `roll`, PIANO's PIANO ROLL: `fight.py` 5164 to 5283 (`ROLL_DEFAULT`, `roll_setup`, `roll_lane`,
//! `roll_tick`, `fire_row`, `start_runoff`, `hold_step`), the projectiles (`spawn_key` 370 to 378, their flight and
//! hits in `move` 3235 to 3278) and their `K` rows in `snap` (5509 to 5514).
//!
//! A player-piano roll lies along one side of the ring: the side through the fighter's own corner and the back corner
//! (axis 0: `sx = -A`, axis 1: `sz = -A`). Places along it are pitches (`reg`: low notes at its own corner's end, high
//! notes at the back corner's end, `margin` in from the posts). On the 8th grid every row of the roll passes the read
//! bar and every hole of that row fires a note-shot from its lane into the ring: straight across, turned toward the
//! enemy by `aim`, at `speed`, `dmg` each. The rows are the fighter's line in the music: the rules give them
//! (`Rules::roll_rows`, `Rules::roll_start8`: [`feed_rules`]); without them the roll plays [`ROLL_DEFAULT`]. A row
//! fires its first `int(n_vol)` holes (`n0`, `grow` more per landed row up to `n_max`).
//!
//! The shots are the fight's projectiles (`Fight::keys`): this module fires them ([`spawn_key`]), flies them and
//! lands them ([`keys_move`]) for every kind that has any (the roll's rows, the `runoff`, an `arc`), whichever ball
//! fired them. SUSTAIN's shots stop, hang and re-aim ([`hold_step`]).
//!
//! Events: `roll` (fighter, volley, absolute 8th, 1 while sustained else 0, the pitches fired), `wall` (a shot that
//! left the ring: its owner, -1, where, its id) and hits of kind `key` whose sub-number is the shot's index in its
//! volley (a row's or a runoff's pitch; an arc's step). Recorded as `ROLL` (per ball: `n_vol`, seconds of sustain
//! left, seconds of reload left) and `HOLD` (the hanging shots: id, when it stops, when it flies on).
//!
//! None of this code draws from the generator.

use std::collections::{BTreeMap, BTreeSet};

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::fightfile::FightFile;
use bf_core::harmony::{self, Harmony};
use bf_core::lines::RollRowsError;
use bf_core::py;

use crate::fight::{Fight, Key};
use crate::geom::{self, V2};
use crate::record::{Frames, KMAX};
use crate::roster::{FighterCfg, RollCfg, WeaponCfg};
use crate::rules::{RollRows, Rules, SimError};

/// `ROLL_DEFAULT` (5180): the rows a roll plays when the rules give it none, as steps from the middle of its range.
pub const ROLL_DEFAULT: [&[i64]; 16] = [
    &[0, -12],
    &[],
    &[7],
    &[3],
    &[12, 0],
    &[],
    &[10],
    &[7],
    &[3, -9],
    &[],
    &[5],
    &[7],
    &[15, 3],
    &[],
    &[12],
    &[10],
];

/// `LOOP_START`: the bar the chord loop (and so the roll) starts on. The Studio's fights all start it at FIGHT!, bar 2
/// (`kit/episode.py`'s default; no fight file moves it).
const LOOP_START: i64 = 2;

/// A hanging shot's entry of `khold` (`fire_row` 5242): `[t_stop, t_release, speed, state]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hold {
    /// When it stops.
    pub t_stop: f64,
    /// When it turns to the enemy and flies on.
    pub t_rel: f64,
    /// The speed it flies on at.
    pub sp: f64,
    /// Whether it has stopped (Python's state 1; 0 while it still flies out).
    pub stopped: bool,
}

/// What `roll_setup` hangs on a roller's ball.
#[derive(Clone, Debug, PartialEq)]
pub struct Roller {
    /// `roll_ax`: the axis whose -A side holds the roll.
    pub ax: usize,
    /// `roll_rows`: the pitches of each row, in the order they are fired.
    pub rows: Vec<Vec<i64>>,
    /// `roll_start8`: the absolute 8th the first row falls on.
    pub start8: i64,
}

/// Weapon kind `roll`'s per-fight state (`Fight::roll`), and the projectiles' counters, which every kind that fires
/// one shares (`spawn_key`).
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    /// `next_id`: the next projectile's number.
    pub next_id: i64,
    /// `next_vol`: the next volley's number (a row, a runoff and an arc are one volley each).
    pub next_vol: i64,
    /// `vol_landed`: the volleys that have landed a shot (a row grows the roll once).
    pub vol_landed: BTreeSet<i64>,
    /// `khold`: SUSTAIN's hanging shots by projectile id (Python's dict, filled in id order).
    pub khold: BTreeMap<i64, Hold>,
    /// Each roller's own state (`None` for a fighter that is not one, and before `roll_setup`).
    pub ball: [Option<Roller>; 2],
    /// `B.sustain_until` (`getattr(B, 'sustain_until', -1.0)`): until when the rows are sustained.
    pub sustain_until: [f64; 2],
    /// `B.reload_until` (`getattr(B, 'reload_until', -1.0)`): until when a new roll is threaded and no row fires.
    pub reload_until: [f64; 2],
}

impl Default for State {
    fn default() -> State {
        State {
            next_id: 0,
            next_vol: 0,
            vol_landed: BTreeSet::new(),
            khold: BTreeMap::new(),
            ball: [None, None],
            sustain_until: [-1.0, -1.0],
            reload_until: [-1.0, -1.0],
        }
    }
}

/// `B.w['kind'] == 'roll'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Roll(_))
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a RollCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Roll(w) => w,
        other => panic!("fighter {k} ({}) has no roll: {other:?}", cfg.name),
    }
}

/// `B.w.get('reg', B.cfg.get('register', (48, 84)))`: the roll's pitch range.
fn reg(cfg: &FighterCfg, w: &RollCfg) -> (i64, i64) {
    let (lo, hi) = w.reg.unwrap_or(cfg.register);
    (i64::from(lo), i64::from(hi))
}

/// One projectile to fire: `spawn_key`'s arguments after the owner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// Where it starts.
    pub pos: V2,
    /// The angle it flies along.
    pub ang: f64,
    /// Units a second.
    pub speed: f64,
    /// The damage it does.
    pub dmg: f64,
    /// The volley it belongs to.
    pub vol: i64,
    /// Its index in the volley.
    pub idx: i64,
}

/// `spawn_key(owner, pos, ang, speed, dmg, vol, idx)` (370 to 378): a projectile enters the fight at `pos`, flying at
/// `speed` along `ang`, with the next id.
pub fn spawn_key(f: &mut Fight, owner: usize, s: Shot) {
    let id = f.roll.next_id;
    f.keys.push(Key {
        p: s.pos,
        v: [s.speed * dmath::cos(s.ang), s.speed * dmath::sin(s.ang)],
        own: owner,
        id,
        vol: s.vol,
        idx: s.idx,
        dmg: s.dmg,
    });
    f.roll.next_id += 1;
}

/// `roll_setup(k)` (5182 to 5198), from `run()` 5882 to 5884
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    // B.roll_ax = 0 if self.corner[k][0] < 0 else 1
    let ax = if f.corner[k][0] < 0.0 { 0 } else { 1 };
    f.roll.sustain_until[k] = -1.0;
    f.roll.reload_until[k] = -1.0;
    let given = f.rules.roll_rows.as_ref().and_then(|rows| rows.get(&k));
    let (rows, start8) = match given {
        Some(rows) => (rows.clone(), f.rules.roll_start8.unwrap_or(0)),
        None => {
            let (lo, hi) = reg(f.balls[k].cfg, w);
            let mid = py::ifloor_div(lo + hi, 2);
            let rows = ROLL_DEFAULT.iter().map(|r| r.iter().map(|d| mid + d).collect());
            (rows.collect(), 0)
        }
    };
    f.roll.ball[k] = Some(Roller { ax, rows, start8 });
}

fn roller<'f>(f: &'f Fight, k: usize) -> &'f Roller {
    f.roll.ball[k].as_ref().unwrap_or_else(|| {
        panic!(
            "fighter {k} ({}) has no piano roll (Python: no `roll_ax` on a ball whose weapon is not a roll)",
            f.balls[k].name
        )
    })
}

/// `roll_lane(B, m, margin)` (5200 to 5211): the spawn point of pitch `m` on the roll (ring frame); the
/// straight-across direction is the unit vector of the roll's axis.
pub fn roll_lane(f: &Fight, k: usize, m: f64, margin: Option<f64>) -> V2 {
    let w = weapon(f, k);
    let a = f.rules.a;
    let (lo, hi) = reg(f.balls[k].cfg, w);
    let mg = margin.unwrap_or(w.margin);
    // u = (float(m) - lo) / max(1.0, float(hi - lo))
    let u = (m - lo as f64) / geom::py_max(1.0, (hi - lo) as f64);
    // low notes at the own corner's end (+), high at the back
    let lat = (a - mg) - 2.0 * (a - mg) * geom::py_min(1.0, geom::py_max(0.0, u));
    let ax = roller(f, k).ax;
    let mut p = [0.0, 0.0];
    p[ax] = -a + 0.1;
    p[1 - ax] = lat;
    p
}

/// `roll_tick(k, n16)` (5213 to 5225), from `run()` 6070 to 6075: `n16` is the ABSOLUTE 16th (`n16o`, not counted from
/// the fight's first), and a stunned roller is skipped by the caller. On every 8th the row under the read bar fires,
/// unless a new roll is being threaded.
pub fn tick(f: &mut Fight, k: usize, n16: i64) {
    if py::imod_py(n16, 2) != 0 {
        return;
    }
    if f.t + 1e-9 < f.roll.reload_until[k] {
        return;
    }
    let abs8 = py::ifloor_div(n16, 2);
    let r = roller(f, k);
    let row = &r.rows[py::imod_py(abs8 - r.start8, r.rows.len() as i64) as usize];
    if row.is_empty() {
        return;
    }
    // row[:max(1, int(B.n_vol + 1e-9))]
    let n = ((f.balls[k].n_vol + 1e-9) as i64).max(1) as usize;
    let pitches: Vec<i64> = row.iter().take(n).copied().collect();
    fire_row(f, k, &pitches, abs8);
}

/// `fire_row(k, pitches, abs8)` (5227 to 5243): each pitch fires a shot from its lane; while the fighter's SUSTAIN
/// holds, each shot will stop after its short flight and hang (`khold`).
pub fn fire_row(f: &mut Fight, k: usize, pitches: &[i64], abs8: i64) {
    let w = weapon(f, k);
    let cfg = f.balls[k].cfg;
    let vol = f.roll.next_vol;
    f.roll.next_vol += 1;
    let sus = f.t < f.roll.sustain_until[k];
    // ab = B.cfg['abilities']['unlock1'] if sus else None
    let ab = &cfg.unlock1;
    let ax = roller(f, k).ax;
    for (j, &m) in pitches.iter().enumerate() {
        let pos = roll_lane(f, k, m as f64, None);
        let mut d0 = [0.0, 0.0];
        d0[ax] = 1.0;
        let de = geom::sub(f.balls[1 - k].p, pos);
        let de = geom::div(de, geom::norm(de) + 1e-9);
        // d = (1 - w['aim']) * d0 + w['aim'] * de
        let d = geom::add(geom::scale(d0, 1.0 - w.aim), geom::scale(de, w.aim));
        let ang = dmath::atan2(d[1], d[0]);
        let shot = Shot {
            pos,
            ang,
            speed: w.speed,
            dmg: w.dmg,
            vol,
            idx: m,
        };
        spawn_key(f, k, shot);
        if sus {
            // a staggered field
            let out16 = ab.get("out16").unwrap_or(2.0);
            let step16 = ab.get("step16").unwrap_or(0.0);
            let t_stop = f.t + (out16 + j as f64 * step16) * f.rules.beat / 4.0;
            let hold = Hold {
                t_stop,
                t_rel: t_stop + ab.get("hang_beats").unwrap_or(1.0) * f.rules.beat,
                sp: w.speed,
                stopped: false,
            };
            f.roll.khold.insert(f.roll.next_id - 1, hold);
        }
    }
    let data = vec![
        Value::Int(vol),
        Value::Int(abs8),
        Value::Int(i64::from(sus)),
        Value::List(pitches.iter().map(|&m| Value::Int(m)).collect()),
    ];
    f.ev("roll", k as i64, data);
}

/// 88 KEYS's numbers, as `start_runoff` reads them from the ability.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Runoff {
    /// `ab['n']`: how many shots.
    pub n: f64,
    /// `ab['speed']`.
    pub speed: f64,
    /// `ab['dmg']`, each.
    pub dmg: f64,
    /// `ab.get('margin', 0.25)`: how far in from the posts the lanes end.
    pub margin: f64,
    /// `ab.get('reload_beats', 0)`: beats without a row afterwards.
    pub reload_beats: f64,
}

/// `start_runoff(k, ab)` (5245 to 5260), 88 KEYS: every hole of the roll fires at once, straight across the ring, then
/// the roll is re-threaded.
///
/// # Panics
/// For a fighter whose weapon is not a roll (Python raises `AttributeError`: the ball has no `roll_ax`).
pub fn start_runoff(f: &mut Fight, k: usize, ab: Runoff) {
    let ax = roller(f, k).ax;
    let (lo, hi) = reg(f.balls[k].cfg, weapon(f, k));
    // n = int(ab['n'])
    let n = ab.n as i64;
    let vol = f.roll.next_vol;
    f.roll.next_vol += 1;
    let mut ms = Vec::new();
    for j in 0..n {
        // m = lo + (hi - lo) * j / max(1, n - 1)
        let m = lo as f64 + ((hi - lo) * j) as f64 / (n - 1).max(1) as f64;
        let pos = roll_lane(f, k, m, Some(ab.margin));
        let mi = py::round_half_even(m) as i64;
        let shot = Shot {
            pos,
            ang: if ax == 0 { 0.0 } else { dmath::PI / 2.0 },
            speed: ab.speed,
            dmg: ab.dmg,
            vol,
            idx: mi,
        };
        spawn_key(f, k, shot);
        ms.push(Value::Int(mi));
    }
    f.roll.reload_until[k] = f.t + ab.reload_beats * f.rules.beat;
    f.ev(
        "runoff",
        k as i64,
        vec![Value::Int(vol), Value::Int(n), Value::List(ms)],
    );
}

/// `hold_step()` (5262 to 5282), from `run()` 6113 to 6114 (`if rollers`), before `move`: SUSTAIN's mines stop after
/// their short flight, hang, then re-aim at the enemy and fly.
pub fn hold_step(f: &mut Fight) {
    if f.roll.khold.is_empty() {
        return;
    }
    let ids: Vec<i64> = f.roll.khold.keys().copied().collect();
    for i in ids {
        // live = {int(i): j for j, i in enumerate(self.kid)}
        let Some(j) = f.keys.iter().position(|key| key.id == i) else {
            f.roll.khold.remove(&i);
            continue;
        };
        let h = f.roll.khold[&i];
        if !h.stopped && f.t + 1e-9 >= h.t_stop {
            f.keys[j].v = [0.0, 0.0];
            f.roll.khold.get_mut(&i).expect("the entry read above").stopped = true;
        } else if h.stopped && f.t + 1e-9 >= h.t_rel {
            let o_p = f.balls[1 - f.keys[j].own].p;
            let d = geom::sub(o_p, f.keys[j].p);
            let n = geom::hypot(d);
            let d = if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] };
            f.keys[j].v = geom::scale(d, h.sp);
            f.roll.khold.remove(&i);
        }
    }
}

/// `move`, the projectiles (3235 to 3278): the flight, `kill_keys(out, 'wall')`, the hits on each ball
/// (`damage(..., 'key', ...)`, the roll growth 3267 to 3268, the nudge), after the balls' clash. (`steer` and
/// `ricochet` are for curving and bouncing volleys, kinds older than the kit's: no projectile of the kit curves or
/// bounces. The volley growth 3263 to 3266 is the older kind `volley`'s.)
pub fn keys_move(f: &mut Fight, dt: f64) {
    let (a, r) = (f.rules.a, f.rules.r);
    if !f.keys.is_empty() {
        // self.kp = self.kp + self.kv * dt
        for key in &mut f.keys {
            key.p = geom::add(key.p, geom::scale(key.v, dt));
        }
        // out = (np.abs(self.kp) > A).any(1)
        let out: Vec<bool> = f
            .keys
            .iter()
            .map(|key| key.p[0].abs() > a || key.p[1].abs() > a)
            .collect();
        f.kill_keys(&out, "wall", -1);
    }
    if f.keys.is_empty() {
        return;
    }
    for k in 0..2 {
        // 3246 to 3247, `rw_left`: the kind `rewind`, older than the kit's
        if !f.keys.iter().any(|key| key.own != k) {
            continue;
        }
        let b_p = f.balls[k].p;
        // m = enemy & (dd < getattr(B, 'rad', R) + self.R.KEY_R)
        let reach = r + f.rules.key_r;
        let m: Vec<bool> = f
            .keys
            .iter()
            .map(|key| key.own != k && geom::hypot(geom::sub(key.p, b_p)) < reach)
            .collect();
        if !m.contains(&true) {
            continue;
        }
        for i in (0..m.len()).filter(|&i| m[i]) {
            let key = f.keys[i].clone();
            let att = key.own;
            let landed = f.damage(att, k, key.dmg, "key", key.p, key.idx);
            if landed {
                if f.roll.vol_landed.insert(key.vol) {
                    // #45: a landed row punches more holes per row
                    if let WeaponCfg::Roll(w) = &f.balls[att].cfg.weapon {
                        let b = &mut f.balls[att];
                        b.n_vol = geom::py_min(w.n_max, b.n_vol + w.grow);
                    }
                }
                // a light nudge along the key's flight
                let b = &mut f.balls[k];
                let kv = geom::div(key.v, geom::norm(key.v) + 1e-9);
                let cur = geom::div(b.v, geom::norm(b.v) + 1e-9);
                let nv = geom::add(geom::scale(cur, 0.88), geom::scale(kv, 0.12));
                b.v = geom::scale(geom::div(nv, geom::norm(nv) + 1e-9), geom::norm(b.v));
            }
        }
        let mut hit = m.iter();
        f.keys.retain(|_| !hit.next().expect("one mask entry per projectile"));
        if f.keys.is_empty() {
            break;
        }
    }
}

/// `snap`, `K` (5509 to 5514): the first `min(KMAX, len(kp))` rows of this frame's `K` (72 x 7, all NaN on entry) from
/// `fight.keys`: `[x, y, atan2(vy, vx), owner, id, idx, vol]`
pub(crate) fn k_rows(f: &Fight, k: &mut [f64]) {
    let (rows, _) = k.as_chunks_mut::<7>();
    for (key, row) in f.keys.iter().take(KMAX).zip(rows) {
        *row = [
            key.p[0],
            key.p[1],
            dmath::atan2(key.v[1], key.v[0]),
            key.own as f64,
            key.id as f64,
            key.idx as f64,
            key.vol as f64,
        ];
    }
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5886 to 5887: `ROLL`, `HOLD` when a
/// fighter is a roller.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) {
        &["ROLL", "HOLD"]
    } else {
        &[]
    }
}

/// This kind's rows of one frame: `snap`, `ROLL` and `HOLD` (5621 to 5624), into `out`.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut roll = Vec::with_capacity(6);
    for k in 0..2 {
        roll.push(f.balls[k].n_vol);
        roll.push(geom::py_max(0.0, f.roll.sustain_until[k] - f.t));
        roll.push(geom::py_max(0.0, f.roll.reload_until[k] - f.t));
    }
    out.push_f64("ROLL", &[2, 3], &roll);
    let hold = f
        .roll
        .khold
        .iter()
        .map(|(&i, h)| Value::List(vec![Value::Int(i), Value::Float(h.t_stop), Value::Float(h.t_rel)]));
    out.push_nested("HOLD", Value::List(hold.collect()));
}

/// `kit/story.py` 33 to 35 (`rules()`): a roll fighter's rows are its line, so the fight's rules carry `roll_rows`
/// (`roll_rows(name)`, 41 to 75, by fighter index: `bf_core`'s `Harmony::roll_rows`) and `roll_start8 = 8 *
/// LOOP_START` (`Rules::roll_rows`, `Rules::roll_start8`). Called by `setup_from` after the pair rules; nothing to do
/// for a fight without a roller.
///
/// # Errors
/// `music.lines.<NAME>` for a line that is not 8 (or 4) notes a bar over the loop (Python's `assert`); `music` where
/// the harmony itself cannot be built or a chord has no tone in the fighter's register.
pub fn feed_rules(ff: &FightFile, fighters: [&FighterCfg; 2], rules: &mut Rules) -> Result<(), SimError> {
    let mut rows = RollRows::new();
    for (j, cfg) in fighters.iter().enumerate() {
        let WeaponCfg::Roll(w) = &cfg.weapon else {
            continue;
        };
        let music = |e: &dyn std::fmt::Display| SimError::new("music", format!("the roll's rows: {e}"));
        let grid = ff.grid();
        let scale = harmony::scale(&ff.music.scale).map_err(|e| music(&e))?;
        let chords = ff.chords(grid.n_bars);
        let h = Harmony::new(
            &grid,
            &ff.music.key,
            scale,
            chords,
            Some(ff.music.chord_loop.clone()),
            LOOP_START as usize,
        )
        .map_err(|e| music(&e))?;
        let line = ff.music.lines.get(&cfg.name).map(Vec::as_slice);
        let got = h
            .roll_rows(line, w.reg.unwrap_or(cfg.register), cfg.register)
            .map_err(|e| match e {
                RollRowsError::PerBar { .. } => SimError::new(format!("music.lines.{}", cfg.name), e.to_string()),
                other => music(&other),
            })?;
        let got = got.into_iter().map(|r| r.into_iter().map(i64::from).collect());
        rows.insert(j, got.collect());
    }
    if !rows.is_empty() {
        rules.roll_rows = Some(rows);
        rules.roll_start8 = Some(8 * LOOP_START);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
