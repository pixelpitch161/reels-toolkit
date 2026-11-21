//! Weapon kind `pick`, BASS GUITAR's PICK TOSS, and its three abilities `fret_slide` (FRET SLIDE), `headstock`
//! (HEADSTOCK SWING) and `slingshot` (SLINGSHOT). A Studio kind: the Python kit never had it, so this module is the
//! source of truth (spec `docs/superpowers/specs/2026-11-21-bass-guitar-fighter-design.md`).
//!
//! - PICK TOSS: on its grid (`every` beats; haste divides it) a bass that is not stunned throws `int(n_vol)` picks.
//!   Each flies a loop over `flight_beats` beats: out to the enemy's spot (led by its velocity), bending sideways,
//!   and back to the ball wherever it now is. A pick hits once: `dmg`, a knock of `knock` along its flight. A landed
//!   pick grows `n_vol` by `grow` (up to `n_max`); extra picks fan out `fan` radians apart, bending to alternate
//!   sides.
//! - FRET SLIDE: the ball holds still for `wind_beats` beats, then dashes at the enemy at `speed` x the shared speed,
//!   up to `span` ball radii. It stops dead on contact (a hit of kind `fret_slide`) and stays put `settle_beats`.
//! - HEADSTOCK SWING: the neck grows to `len` ball radii beyond the ball's edge and swings back `sweep` degrees over
//!   `wind_beats` beats, then sweeps round to the enemy over `swing_beats` beats. It stops dead on the enemy (a hit
//!   of kind `headstock`, knocked out and along the swing), then shrinks back over one beat.
//! - SLINGSHOT: two strings tie to the two corner posts behind the ball; it draws back `back` ball radii over
//!   `pull_beats` beats, then fires itself at the enemy like FRET SLIDE (a hit of kind `slingshot`).
//!
//! While it holds, draws back, dashes or settles the ball is `walking`: this module moves it and nothing knocks it.
//! A dash ends early when the ball is stunned, held, grabbed, pinned, airborne or smashed, or reaches the ropes. The
//! moves keep their own numbers, so they run for whichever ball fires them (a copied one too).
//!
//! Events: `pick` (owner; picks thrown, throw number), `fret_slide`, `headstock`, `slingshot` (owner; 0 when it
//! starts, 1 when it lands). Recorded as `PICK`, rows `(owner, type, a, x0, z0, x1, z1)`: a pick in the air (type 0;
//! `a` its spin, `x0, z0` where it is, `x1` its index, `z1` how far through its loop), a dash (type 1; `a` its phase:
//! 0 holding, 1 dashing, 2 settled, 3 drawing back; `x0, z0` where the line starts, `x1, z1` where it ends), the neck
//! of a swing (type 2; `a` how far it is grown, butt and tip) and a slingshot's anchors (type 3; `a` the strings'
//! tension, the two corner posts).
//!
//! None of this draws from the generator, so a fight without a bass is unchanged.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, radius, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, AbilityKind, PickCfg, WeaponCfg};

/// One pick in the air.
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    /// When it was thrown.
    pub t0: f64,
    /// Seconds its loop takes.
    pub dur: f64,
    /// Where it was thrown from.
    pub p0: V2,
    /// The loop's far point, from `p0`.
    pub d: V2,
    /// The loop's sideways bend, from `p0` (at the loop's quarter points).
    pub side: V2,
    /// Its index in the throw.
    pub i: i64,
    /// Its damage and knock.
    pub dmg: f64,
    pub knock: f64,
    /// Its radius.
    pub r: f64,
    /// It has hit (a pick hits once).
    pub hit: bool,
    /// Where it was last step (its flight direction).
    pub last: V2,
}

/// Which move a dash is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// FRET SLIDE.
    Slide,
    /// SLINGSHOT.
    Sling,
}

/// A dash's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Holding still, the line lit (FRET SLIDE).
    Hold,
    /// Drawing back between the strings (SLINGSHOT).
    Pull,
    /// Dashing along the line.
    Run,
    /// Stopped dead after the hit.
    Settle,
}

/// A FRET SLIDE or SLINGSHOT in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Dash {
    pub what: Move,
    pub phase: Phase,
    /// When the phase ends.
    pub until: f64,
    /// When the phase started.
    pub t0: f64,
    /// Where the phase started.
    pub p0: V2,
    /// Where it ends (the draw-back's end, or the line's).
    pub p1: V2,
    /// The line's unit direction.
    pub u: V2,
    /// Its speed while dashing.
    pub speed: f64,
    /// Its reach while dashing.
    pub span: f64,
    pub dmg: f64,
    pub knock: f64,
    /// How long it stays put after a hit.
    pub settle: f64,
    /// SLINGSHOT's corner posts.
    pub anchors: Option<[V2; 2]>,
}

/// A HEADSTOCK SWING in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct Swing {
    pub t0: f64,
    /// When the wind-up ends and the sweep starts.
    pub t_swing: f64,
    /// When the sweep ends.
    pub t_end: f64,
    /// When the neck has shrunk back.
    pub t_gone: f64,
    /// The neck's reach beyond the ball's edge.
    pub len: f64,
    /// The sweep, radians, signed (+ swings back anticlockwise).
    pub sweep: f64,
    pub dmg: f64,
    pub knock: f64,
    /// Once it has hit: when, and the angle it stopped at.
    pub stop: Option<(f64, f64)>,
}

/// The kind's per-fight state (`Fight::pick`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// Throws so far.
    pub throws: [i64; 2],
    /// The picks in the air.
    pub picks: [Vec<Pick>; 2],
    /// A dash in progress.
    pub dash: [Option<Dash>; 2],
    /// A swing in progress.
    pub swing: [Option<Swing>; 2],
}

/// Fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Pick(_))
}

/// Picks or one of its moves can be in the ring.
pub fn live(f: &Fight) -> bool {
    (0..2).any(|k| {
        is(f, k)
            || crate::abilities::has(f, k, &AbilityKind::FretSlide)
            || crate::abilities::has(f, k, &AbilityKind::Headstock)
            || crate::abilities::has(f, k, &AbilityKind::Slingshot)
    })
}

fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a PickCfg {
    match &f.balls[k].cfg.weapon {
        WeaponCfg::Pick(w) => w,
        other => panic!("fighter {k}'s weapon is {}, not pick", other.kind()),
    }
}

/// Setup: the first pick count.
pub fn setup(f: &mut Fight, k: usize) {
    f.balls[k].n_vol = weapon(f, k).n0;
}

fn rot(d: V2, a: f64) -> V2 {
    let (c, s) = (dmath::cos(a), dmath::sin(a));
    [d[0] * c - d[1] * s, d[0] * s + d[1] * c]
}

fn unit(d: V2) -> V2 {
    let n = geom::norm(d);
    if n > 1e-9 { geom::div(d, n) } else { [1.0, 0.0] }
}

fn clamp_in(f: &Fight, p: V2, m: f64) -> V2 {
    let lim = f.rules.a - m;
    [geom::np_clip(p[0], -lim, lim), geom::np_clip(p[1], -lim, lim)]
}

/// Where pick `p` of ball `k` is at `t`: the loop from its throw point, drawn home to where the ball is now.
pub fn pick_at(f: &Fight, k: usize, p: &Pick, t: f64) -> V2 {
    let u = ((t - p.t0) / p.dur).clamp(0.0, 1.0);
    let a = 2.0 * dmath::PI * u;
    let out = 0.5 * (1.0 - dmath::cos(a));
    let side = dmath::sin(a);
    let home = geom::sub(f.balls[k].p, p.p0);
    let q = geom::add(
        geom::add(p.p0, geom::scale(p.d, out)),
        geom::add(geom::scale(p.side, side), geom::scale(home, u * u)),
    );
    clamp_in(f, q, p.r)
}

/// The grid tick: on its grid a bass that is not stunned throws its picks.
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    if stunned(f, k) {
        return;
    }
    let w = weapon(f, k).clone();
    let main16 = py::round_half_even(4.0 * w.every) as i64;
    let ev16 = (py::round_half_even(main16 as f64 / f.ab_mult(k)) as i64).max(1);
    if py::imod_py(k16, ev16) != 0 {
        return;
    }
    let (me, them) = (f.balls[k].p, &f.balls[1 - k]);
    let dur = w.flight_beats * f.rules.beat / f.ab_mult(k);
    let tgt = clamp_in(
        f,
        geom::add(them.p, geom::scale(them.v, w.lead * dur * 0.5)),
        radius(f, 1 - k),
    );
    let d0 = geom::sub(tgt, me);
    let dist = geom::norm(d0).max(radius(f, k) * 2.0);
    let n = ((f.balls[k].n_vol + 1e-9) as i64).max(1);
    let throw = f.pick.throws[k];
    f.pick.throws[k] += 1;
    let flip = if throw % 2 == 0 { 1.0 } else { -1.0 };
    let r = w.hit_r * f.rules.r;
    for i in 0..n {
        let off = (i as f64 - (n - 1) as f64 / 2.0) * w.fan;
        let d = rot(unit(d0), off);
        let sgn = flip * if i % 2 == 0 { 1.0 } else { -1.0 };
        let side = geom::scale([-d[1], d[0]], sgn * w.bend * dist * 0.5);
        f.pick.picks[k].push(Pick {
            t0: f.t,
            dur,
            p0: me,
            d: geom::scale(d, dist),
            side,
            i,
            dmg: w.dmg,
            knock: w.knock,
            r,
            hit: false,
            last: me,
        });
    }
    f.ev("pick", k as i64, vec![Value::Int(n), Value::Int(throw)]);
}

/// Whether ball `k` can be moved by a dash: not stunned, held, grabbed, pinned, airborne or smashed.
fn free(f: &Fight, k: usize) -> bool {
    let b = &f.balls[k];
    !(stunned(f, k) || b.held || b.grabbed || b.pinned || b.air || b.smashed > 0.0 || b.anc.is_some())
        && b.swan.is_none()
        && b.dive.is_none()
        && b.strut.is_none()
}

/// A dash may start: the ball is free, not already dashing, and not walking for another kind.
fn can_dash(f: &Fight, k: usize) -> bool {
    free(f, k) && f.pick.dash[k].is_none() && !f.balls[k].walking
}

/// `fret_slide`: hold still, the line lit, then dash.
pub fn start_fret_slide(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if !can_dash(f, k) {
        return;
    }
    let me = f.balls[k].p;
    let u = unit(geom::sub(f.balls[1 - k].p, me));
    let span = param(ab, "span") * f.rules.r;
    f.pick.dash[k] = Some(Dash {
        what: Move::Slide,
        phase: Phase::Hold,
        until: f.t + param(ab, "wind_beats") * f.rules.beat,
        t0: f.t,
        p0: me,
        p1: clamp_in(f, geom::add(me, geom::scale(u, span)), radius(f, k)),
        u,
        speed: param(ab, "speed") * f.rules.v,
        span,
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        settle: param(ab, "settle_beats") * f.rules.beat,
        anchors: None,
    });
    f.balls[k].walking = true;
    f.ev("fret_slide", k as i64, vec![Value::Int(0)]);
}

/// `slingshot`: tie the strings to the corner posts behind the ball, draw back, then dash.
pub fn start_slingshot(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if !can_dash(f, k) {
        return;
    }
    let me = f.balls[k].p;
    let back = unit(geom::sub(me, f.balls[1 - k].p));
    let a = f.rules.a;
    let mut corners = [[a, a], [-a, a], [-a, -a], [a, -a]];
    // the two posts furthest along the way back (stable order: by score, then corner order)
    corners.sort_by(|p, q| {
        let sp = geom::dot(geom::sub(*p, me), back);
        let sq = geom::dot(geom::sub(*q, me), back);
        sq.total_cmp(&sp)
    });
    let r = radius(f, k);
    let p1 = clamp_in(f, geom::add(me, geom::scale(back, param(ab, "back") * f.rules.r)), r);
    f.pick.dash[k] = Some(Dash {
        what: Move::Sling,
        phase: Phase::Pull,
        until: f.t + param(ab, "pull_beats") * f.rules.beat,
        t0: f.t,
        p0: me,
        p1,
        u: geom::scale(back, -1.0),
        speed: param(ab, "speed") * f.rules.v,
        span: param(ab, "span") * f.rules.r,
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        settle: 0.25 * f.rules.beat,
        anchors: Some([corners[0], corners[1]]),
    });
    f.balls[k].walking = true;
    f.ev("slingshot", k as i64, vec![Value::Int(0)]);
}

/// `headstock`: grow the neck and swing it back, then sweep round to the enemy.
pub fn start_headstock(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    if f.pick.swing[k].is_some() {
        return;
    }
    let beat = f.rules.beat;
    let t_swing = f.t + param(ab, "wind_beats") * beat;
    let t_end = t_swing + param(ab, "swing_beats") * beat;
    // swing back to the side the ball is heading, so the sweep comes round against its motion
    let to = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let v = f.balls[k].v;
    let cross = to[0] * v[1] - to[1] * v[0];
    let sgn = if cross >= 0.0 { 1.0 } else { -1.0 };
    f.pick.swing[k] = Some(Swing {
        t0: f.t,
        t_swing,
        t_end,
        t_gone: t_end + beat,
        len: param(ab, "len") * f.rules.r,
        sweep: sgn * param(ab, "sweep").to_radians(),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
        stop: None,
    });
    f.ev("headstock", k as i64, vec![Value::Int(0)]);
}

fn ease_out(x: f64) -> f64 {
    let y = 1.0 - x.clamp(0.0, 1.0);
    1.0 - y * y * y
}

fn ease_in(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x
}

/// Swing `s` of ball `k` at `t`: the neck's angle (world radians) and its reach beyond the ball's edge.
pub fn swing_at(f: &Fight, k: usize, s: &Swing, t: f64) -> (f64, f64) {
    let to = geom::sub(f.balls[1 - k].p, f.balls[k].p);
    let aim = dmath::atan2(to[1], to[0]);
    let grow = ease_out((t - s.t0) / (s.t_swing - s.t0).max(1e-9));
    let shrink = 1.0 - 0.85 * ease_out((t - s.t_end) / (s.t_gone - s.t_end).max(1e-9));
    let len = s.len * grow * if t > s.t_end { shrink } else { 1.0 };
    if let Some((th, a)) = s.stop {
        return (a, s.len * (1.0 - 0.85 * ease_out((t - th) / f.rules.beat)));
    }
    let a = if t < s.t_swing {
        aim + s.sweep * grow
    } else {
        aim + s.sweep * (1.0 - ease_in((t - s.t_swing) / (s.t_end - s.t_swing).max(1e-9)))
    };
    (a, len)
}

fn seg_dist(p: V2, a: V2, b: V2) -> f64 {
    geom::hypot(geom::sub(p, geom::seg_pt(p, a, b)))
}

fn neck_ends(f: &Fight, k: usize, a: f64, len: f64) -> (V2, V2) {
    let d = [dmath::cos(a), dmath::sin(a)];
    let r = radius(f, k);
    let me = f.balls[k].p;
    (
        geom::add(me, geom::scale(d, r * 0.6)),
        geom::add(me, geom::scale(d, r + len)),
    )
}

/// The later step: the picks fly and hit, the dashes move, the swings sweep.
pub fn step(f: &mut Fight, dt: f64) {
    for k in 0..2 {
        picks_step(f, k);
        dash_step(f, k, dt);
        swing_step(f, k);
    }
}

fn picks_step(f: &mut Fight, k: usize) {
    if f.pick.picks[k].is_empty() {
        return;
    }
    let o = 1 - k;
    let mut picks = std::mem::take(&mut f.pick.picks[k]);
    for p in &mut picks {
        let at = pick_at(f, k, p, f.t);
        let dir = unit(geom::sub(at, p.last));
        p.last = at;
        if p.hit || f.winner.is_some() {
            continue;
        }
        if geom::hypot(geom::sub(f.balls[o].p, at)) < radius(f, o) + p.r {
            p.hit = true;
            if f.damage(k, o, p.dmg, "pick", at, p.i) {
                f.knock(o, dir, p.knock);
                if is(f, k) {
                    let w = weapon(f, k);
                    let b = &mut f.balls[k];
                    b.n_vol = py::py_min(w.n_max, b.n_vol + w.grow);
                }
            }
        }
    }
    let t = f.t;
    picks.retain(|p| t < p.t0 + p.dur);
    f.pick.picks[k] = picks;
}

fn end_dash(f: &mut Fight, k: usize, back: Option<V2>) {
    f.pick.dash[k] = None;
    let b = &mut f.balls[k];
    b.walking = false;
    if let Some(u) = back {
        b.v = geom::scale(u, -f.rules.v);
        b.boost = 0.0;
    }
}

fn dash_step(f: &mut Fight, k: usize, dt: f64) {
    let Some(mut d) = f.pick.dash[k].clone() else {
        return;
    };
    if !free(f, k) && d.phase != Phase::Settle {
        end_dash(f, k, None);
        return;
    }
    let o = 1 - k;
    match d.phase {
        Phase::Hold => {
            // the line follows the enemy until the dash starts
            let me = f.balls[k].p;
            d.u = unit(geom::sub(f.balls[o].p, me));
            d.p0 = me;
            d.p1 = clamp_in(f, geom::add(me, geom::scale(d.u, d.span)), radius(f, k));
            if f.t + 1e-9 >= d.until {
                d.phase = Phase::Run;
                d.t0 = f.t;
            }
        }
        Phase::Pull => {
            let s = ease_out((f.t - d.t0) / (d.until - d.t0).max(1e-9));
            let q = geom::add(d.p0, geom::scale(geom::sub(d.p1, d.p0), s));
            f.balls[k].p = q;
            if f.t + 1e-9 >= d.until {
                let me = f.balls[k].p;
                d.u = unit(geom::sub(f.balls[o].p, me));
                d.p0 = me;
                d.p1 = clamp_in(f, geom::add(me, geom::scale(d.u, d.span)), radius(f, k));
                d.phase = Phase::Run;
                d.t0 = f.t;
            }
        }
        Phase::Run => {
            let me = f.balls[k].p;
            let left = geom::sub(d.p1, me);
            let step = (d.speed * dt).min(geom::norm(left));
            let q = geom::add(me, geom::scale(d.u, step));
            f.balls[k].p = q;
            let gap = geom::hypot(geom::sub(f.balls[o].p, q)) - radius(f, k) - radius(f, o);
            if gap < 0.02 && f.winner.is_none() {
                let at = geom::add(q, geom::scale(d.u, radius(f, k)));
                let (how, ev) = match d.what {
                    Move::Slide => ("fret_slide", "fret_slide"),
                    Move::Sling => ("slingshot", "slingshot"),
                };
                if f.damage(k, o, d.dmg, how, at, 0) {
                    f.knock(o, d.u, d.knock);
                }
                f.ev(ev, k as i64, vec![Value::Int(1)]);
                d.phase = Phase::Settle;
                d.t0 = f.t;
                d.until = f.t + d.settle;
                d.p1 = q;
            } else if step < 1e-9 || geom::norm(left) < 1e-6 {
                end_dash(f, k, Some(d.u));
                return;
            }
        }
        Phase::Settle => {
            if f.t + 1e-9 >= d.until {
                end_dash(f, k, Some(d.u));
                return;
            }
        }
    }
    f.pick.dash[k] = Some(d);
}

fn swing_step(f: &mut Fight, k: usize) {
    let Some(mut s) = f.pick.swing[k].clone() else {
        return;
    };
    let o = 1 - k;
    if s.stop.is_none() && f.t >= s.t_swing && f.t <= s.t_end + 1e-9 && f.winner.is_none() && !stunned(f, k) {
        let (a, len) = swing_at(f, k, &s, f.t);
        let (butt, tip) = neck_ends(f, k, a, len);
        let op = f.balls[o].p;
        if seg_dist(op, butt, tip) < radius(f, o) + 0.04 {
            let at = geom::seg_pt(op, butt, tip);
            // knocked out from the ball and along the sweep
            let out = unit(geom::sub(op, f.balls[k].p));
            let along = geom::scale([-out[1], out[0]], -s.sweep.signum());
            let dir = unit(geom::add(out, geom::scale(along, 0.7)));
            if f.damage(k, o, s.dmg, "headstock", at, 0) {
                f.knock(o, dir, s.knock);
            }
            f.ev("headstock", k as i64, vec![Value::Int(1)]);
            s.stop = Some((f.t, a));
        }
    }
    let gone = match s.stop {
        Some((th, _)) => f.t >= th + f.rules.beat,
        None => f.t >= s.t_gone,
    };
    f.pick.swing[k] = if gone { None } else { Some(s) };
}

/// The record keys this kind adds: `PICK` when picks or one of its moves can be in the ring.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if live(f) { &["PICK"] } else { &[] }
}

fn row(k: usize, ty: i64, a: f64, p: V2, q: V2) -> Value {
    Value::List(vec![
        Value::Int(k as i64),
        Value::Int(ty),
        Value::Float(a),
        Value::Float(p[0]),
        Value::Float(p[1]),
        Value::Float(q[0]),
        Value::Float(q[1]),
    ])
}

/// This kind's rows of one frame.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut rows = Vec::new();
    for k in 0..2 {
        for p in &f.pick.picks[k] {
            let at = pick_at(f, k, p, f.t);
            let u = ((f.t - p.t0) / p.dur).clamp(0.0, 1.0);
            rows.push(row(k, 0, (f.t - p.t0) * 18.0, at, [p.i as f64, u]));
        }
        if let Some(d) = &f.pick.dash[k] {
            let ph = match d.phase {
                Phase::Hold => 0.0,
                Phase::Run => 1.0,
                Phase::Settle => 2.0,
                Phase::Pull => 3.0,
            };
            rows.push(row(k, 1, ph, d.p0, d.p1));
            if let Some([a1, a2]) = d.anchors
                && d.phase == Phase::Pull
            {
                let s = ((f.t - d.t0) / (d.until - d.t0).max(1e-9)).clamp(0.0, 1.0);
                rows.push(row(k, 3, s, a1, a2));
            }
        }
        if let Some(s) = &f.pick.swing[k] {
            let (a, len) = swing_at(f, k, s, f.t);
            let (butt, tip) = neck_ends(f, k, a, len);
            rows.push(row(k, 2, len / s.len.max(1e-9), butt, tip));
        }
    }
    out.push_nested("PICK", Value::List(rows));
}
