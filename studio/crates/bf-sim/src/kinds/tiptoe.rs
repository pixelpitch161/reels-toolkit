//! Weapon kind `tiptoe`, BASSOON's TIPTOE: `fight.py` 3280 to 3429 (`tiptoe_setup`, `_behind`, `_hop_launch`,
//! `_hop_land`, `tiptoe_tick`, `tiptoe_step`, `start_mountain`), and the ability `mountain` (`trigger` 2917 to 2918).
//!
//! The bassoon does not glide: it stands still between staccato hops. Every `every` beats (a haste divides it) it
//! hops for `air16` 16ths (a haste divides that too, at least 1) to a spot `gap` beyond contact behind the enemy
//! (behind = against the enemy's direction of travel), aimed where the enemy will be at the landing (`lead` x its
//! motion over the air time), turned by up to `jitter` degrees (seeded), at most `blade_len` away (`base`, + `grow`
//! per landed stomp, up to `max_reach`). In the air it cannot clash. Each landing is a stomp: an enemy within
//! `stomp_r` beyond contact (`stomp`, + `stomp_grow` per landed stomp, up to `stomp_max`) takes `dmg` (x `back`
//! when the bassoon landed within `back_half` degrees of straight behind it) and `knock`. A hopper moves only when
//! knocked (`Fight::move_step`), and cannot hop while its boost is above `stagger`.
//!
//! MOUNTAIN KING (ability kind `mountain`): hops launched `seq16` 16ths apart, circling the enemy `step` degrees per
//! hop from straight behind at `gap`, at most `reach` away, each landing a stomp (`dmg`, x the weapon's `back` when
//! behind, `knock`); the ordinary hops pause meanwhile, and it ignores the stagger.
//!
//! Events: `hop` (the start, the landing spot, the landing time, kind 0 hop / 1 follow-up / 2 mountain), `stomp`
//! (where, hit 0 / 1, back 0 / 1, the stomp radius, the hop's kind), `mountain` (the number of hops), `mountain_end`.
//! Recorded as `HOP`.
//!
//! Python hangs `hop`, `mk`, `mk_ab`, `hop_bonus` and `stomp_r` on the ball; here they are [`State`],
//! `Fight::tiptoe`, indexed by fighter (`Ball::air`, `Ball::hopper` and `Ball::blade_len` are the ball's, the shared
//! engine reads them). The generator is drawn once per ordinary hop launched (the jitter), nowhere else.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::{Fight, stunned};
use crate::geom::{self, V2};
use crate::record::Frames;
use crate::roster::{AbilityCfg, TiptoeCfg, WeaponCfg};

/// A hop in the air (`B.hop`, the dict `_hop_launch` makes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hop {
    /// Where it left the floor.
    pub p0: V2,
    /// Where it lands.
    pub p1: V2,
    /// When it left.
    pub t0: f64,
    /// When it lands.
    pub t1: f64,
    /// 0 an ordinary hop, 1 the hook's follow-up, 2 a hop of MOUNTAIN KING.
    pub kind: i64,
}

/// What a MOUNTAIN KING's stomp deals (`B.mk_ab`: the ability's `dmg` and `knock`; it outlives the dance).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MkAb {
    /// Damage per landing.
    pub dmg: f64,
    /// Knockback per landing.
    pub knock: f64,
}

/// A MOUNTAIN KING under way (`B.mk`, the dict `start_mountain` makes; `ab`'s numbers are copied in).
#[derive(Clone, Debug, PartialEq)]
pub struct Mk {
    /// `ab['seq16']`: the 16ths from each hop's launch to the next.
    pub seq16: Vec<i64>,
    /// `ab['step']`: degrees round the enemy per hop.
    pub step: f64,
    /// `ab['gap']`: the landing spots' distance beyond contact.
    pub gap: f64,
    /// `ab['reach']`: the longest hop.
    pub reach: f64,
    /// The next hop's number.
    pub i: usize,
    /// The 16th the next hop is launched on (`None` before the first).
    pub next: Option<i64>,
    /// The angle of the first landing spot round the enemy (`None` before the first hop).
    pub ang: Option<f64>,
}

/// Weapon kind `tiptoe`'s per-fight state (`Fight::tiptoe`), indexed by fighter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// `B.stomp_r`: the stomp's radius beyond contact.
    pub stomp_r: [f64; 2],
    /// `B.hop`: the hop in the air.
    pub hop: [Option<Hop>; 2],
    /// `B.mk`: MOUNTAIN KING's dance.
    pub mk: [Option<Mk>; 2],
    /// `B.mk_ab`: the last MOUNTAIN KING's stomp numbers.
    pub mk_ab: [Option<MkAb>; 2],
    /// `B.hop_bonus`: a follow-up hop is due (BOCAL HOOK landed).
    pub hop_bonus: [bool; 2],
}

/// `B.w['kind'] == 'tiptoe'`: fighter `k`'s weapon is this kind.
pub fn is(f: &Fight, k: usize) -> bool {
    matches!(f.balls[k].cfg.weapon, WeaponCfg::Tiptoe(_))
}

/// The tiptoe's weapon settings (`B.w`).
///
/// # Panics
/// When fighter `k`'s weapon is not a tiptoe.
fn weapon<'a>(f: &Fight<'a>, k: usize) -> &'a TiptoeCfg {
    let cfg = f.balls[k].cfg;
    match &cfg.weapon {
        WeaponCfg::Tiptoe(w) => w,
        other => panic!("fighter {k} ({}) has no tiptoe weapon: {other:?}", cfg.name),
    }
}

/// `tiptoe_setup(k)` (3294 to 3303), from `run()` 5830 to 5832
pub fn setup(f: &mut Fight, k: usize) {
    let w = weapon(f, k);
    let b = &mut f.balls[k];
    b.hopper = true;
    b.blade_len = w.base;
    b.air = false;
    f.tiptoe.stomp_r[k] = w.stomp;
    f.tiptoe.hop[k] = None;
    f.tiptoe.mk[k] = None;
    f.tiptoe.hop_bonus[k] = false;
}

/// `_behind(k)` (3305 to 3313): the unit vector from the enemy to the spot straight behind it (against its direction
/// of travel); for an enemy standing still, from the enemy to fighter `k`, or `[1, 0]` when they coincide.
pub fn behind(f: &Fight, k: usize) -> V2 {
    let (b, o) = (&f.balls[k], &f.balls[1 - k]);
    let sp = geom::hypot(o.v);
    if sp > 1e-6 {
        // -O.v / sp
        return [-o.v[0] / sp, -o.v[1] / sp];
    }
    let d = geom::sub(b.p, o.p);
    let n = geom::hypot(d);
    if n > 1e-6 { geom::div(d, n) } else { [1.0, 0.0] }
}

/// `_hop_launch(k, air16, kind, tgt, reach)` (3315 to 3330): the hop toward `tgt`, at most `reach` long, landing
/// inside the ropes `air16` 16ths from now. The hopper's heading turns to it. Event `hop`.
fn hop_launch(f: &mut Fight, k: usize, air16: i64, kind: i64, tgt: V2, reach: f64) {
    let r = f.rules.r;
    let air = air16 as f64 * f.rules.beat / 4.0;
    let p0 = f.balls[k].p;
    let mut j = geom::sub(tgt, p0);
    let l = geom::hypot(j);
    if l > reach {
        j = geom::scale(j, reach / l);
    }
    let lim = f.rules.a - r;
    let p1 = [
        geom::np_clip(p0[0] + j[0], -lim, lim),
        geom::np_clip(p0[1] + j[1], -lim, lim),
    ];
    if l > 1e-6 {
        // B.v = J / L * self.R.V: the hopper's heading
        let v = f.rules.v;
        f.balls[k].v = [j[0] / l * v, j[1] / l * v];
    }
    let t1 = f.t + air;
    f.tiptoe.hop[k] = Some(Hop {
        p0,
        p1,
        t0: f.t,
        t1,
        kind,
    });
    f.balls[k].air = true;
    let data = vec![
        Value::Float(p0[0]),
        Value::Float(p0[1]),
        Value::Float(p1[0]),
        Value::Float(p1[1]),
        Value::Float(t1),
        Value::Int(kind),
    ];
    f.ev("hop", k as i64, data);
}

/// `_hop_land(k)` (3332 to 3363): the hopper stands where the hop ends (stepped out past the enemy if that is
/// inside it) and stomps: an enemy within the stomp radius takes the hop's damage (x `back` from behind) and knock;
/// a landed stomp of an ordinary or follow-up hop grows the reach and the radius. Event `stomp`.
fn hop_land(f: &mut Fight, k: usize) {
    let r = f.rules.r;
    let h = f.tiptoe.hop[k].take().expect("a hop to land");
    f.balls[k].p = h.p1;
    f.balls[k].air = false;
    let o_p = f.balls[1 - k].p;
    let rel = geom::sub(f.balls[k].p, o_p);
    let dr = geom::hypot(rel);
    if dr < 2.0 * r + 0.02 {
        // never land inside the enemy: step out past it
        let lim = f.rules.a - r;
        let u = if dr > 1e-6 { geom::div(rel, dr) } else { behind(f, k) };
        let s = 2.0 * r + 0.02;
        f.balls[k].p = [
            geom::np_clip(o_p[0] + u[0] * s, -lim, lim),
            geom::np_clip(o_p[1] + u[1] * s, -lim, lim),
        ];
    }
    let w = weapon(f, k);
    let (dmg, knock) = if h.kind == 2 {
        let ab = f.tiptoe.mk_ab[k].expect("a mountain hop has its ability");
        (ab.dmg, ab.knock)
    } else {
        (w.dmg, w.knock)
    };
    let rel = geom::sub(o_p, f.balls[k].p);
    let dist = geom::hypot(rel);
    let hit = dist - 2.0 * r <= f.tiptoe.stomp_r[k];
    let mut back = false;
    if hit {
        let n = if dist > 1e-6 {
            geom::div(rel, dist)
        } else {
            let u = behind(f, k);
            [-u[0], -u[1]]
        };
        let o_v = f.balls[1 - k].v;
        let sp = geom::hypot(o_v);
        if sp > 1e-6 {
            // float(n @ (O.v / sp)) >= math.cos(math.radians(w['back_half']))
            back = geom::dot(n, geom::div(o_v, sp)) >= dmath::cos(geom::radians(w.back_half));
        }
        // hx, hy = (O.p - n * R).tolist()
        let at = geom::sub(o_p, geom::scale(n, r));
        let landed = f.damage(
            k,
            1 - k,
            dmg * if back { w.back } else { 1.0 },
            "stomp",
            at,
            i64::from(back),
        );
        f.knock(1 - k, n, knock);
        if landed && h.kind != 2 {
            let b = &mut f.balls[k];
            b.blade_len = py::py_min(w.max_reach, b.blade_len + w.grow);
            f.tiptoe.stomp_r[k] = py::py_min(w.stomp_max, f.tiptoe.stomp_r[k] + w.stomp_grow);
        }
    }
    let p = f.balls[k].p;
    let data = vec![
        Value::Float(p[0]),
        Value::Float(p[1]),
        Value::Int(i64::from(hit)),
        Value::Int(i64::from(back)),
        Value::Float(f.tiptoe.stomp_r[k]),
        Value::Int(h.kind),
    ];
    f.ev("stomp", k as i64, data);
}

/// `O.p + O.v * air * lead + u * (2 * R + gap)`: a hop's target, the spot `gap` beyond contact from where the enemy
/// will be, in the direction `u` from it.
fn target(f: &Fight, k: usize, air: f64, lead: f64, u: V2, gap: f64) -> V2 {
    let o = &f.balls[1 - k];
    let s = 2.0 * f.rules.r + gap;
    [
        o.p[0] + o.v[0] * air * lead + u[0] * s,
        o.p[1] + o.v[1] * air * lead + u[1] * s,
    ]
}

/// `tiptoe_tick(k, k16)` (3365 to 3411), from `run()` 6055 to 6059: on 16th `k16` of the fight. A hop due within a
/// substep lands on the grid; then, on the ground and not stunned: MOUNTAIN KING's next hop when its 16th has come
/// (the dance ends, `mountain_end`, when its hops are used up); else, unless tumbling, the hook's follow-up hop at
/// once, or an ordinary hop every `max(1, int(round(main16 / ab_mult)))` 16ths (one draw: the jitter).
pub fn tick(f: &mut Fight, k: usize, k16: i64) {
    let w = weapon(f, k);
    if let Some(h) = f.tiptoe.hop[k]
        && f.t + f.rules.dt + 1e-9 >= h.t1
    {
        // due within a substep: land on the grid (MOUNTAIN KING's hops keep their schedule)
        hop_land(f, k);
    }
    if f.tiptoe.hop[k].is_some() || stunned(f, k) {
        return;
    }
    if let Some(mk) = &f.tiptoe.mk[k] {
        // MOUNTAIN KING: the accelerating dance
        if mk.next.is_none_or(|next| k16 >= next) {
            if mk.i >= mk.seq16.len() {
                f.tiptoe.mk[k] = None;
                f.ev("mountain_end", k as i64, vec![]);
                return;
            }
            let iv = mk.seq16[mk.i];
            // min(int(w['air16']), iv): `as` truncates toward zero, as int() does
            let air16 = (w.air16 as i64).min(iv);
            let air = air16 as f64 * f.rules.beat / 4.0;
            let ang = match mk.ang {
                Some(ang) => ang,
                None => {
                    let u = behind(f, k);
                    dmath::atan2(u[1], u[0])
                }
            };
            let a = ang + geom::radians(mk.step) * mk.i as f64;
            let (gap, reach) = (mk.gap, mk.reach);
            let tgt = target(f, k, air, w.lead, [dmath::cos(a), dmath::sin(a)], gap);
            let mk = f.tiptoe.mk[k].as_mut().expect("the dance");
            mk.next = Some(k16 + iv);
            mk.ang = Some(ang);
            mk.i += 1;
            hop_launch(f, k, air16, 2, tgt, reach);
        }
        return;
    }
    if f.balls[k].boost > w.stagger {
        // tumbling from a knock: no hop
        return;
    }
    if f.tiptoe.hop_bonus[k] {
        // (BOCAL HOOK) the follow-up hop, right away
        f.tiptoe.hop_bonus[k] = false;
        let air = f.rules.beat / 4.0;
        let tgt = target(f, k, air, w.lead, behind(f, k), w.gap);
        hop_launch(f, k, 1, 1, tgt, w.max_reach);
        return;
    }
    // int(round(x)): half to even; a quotient too large for an i64 saturates
    let main16 = py::round_half_even(4.0 * w.every);
    let ev16 = (py::round_half_even(main16 / f.ab_mult(k)) as i64).max(1);
    if k16.rem_euclid(ev16) == 0 {
        let air16 = (py::round_half_even(w.air16 / f.ab_mult(k)) as i64).max(1);
        let air = air16 as f64 * f.rules.beat / 4.0;
        let u = behind(f, k);
        let j = geom::radians(f.rng.uniform(-1.0, 1.0) * w.jitter);
        let (c, s) = (dmath::cos(j), dmath::sin(j));
        let ub = [u[0] * c - u[1] * s, u[0] * s + u[1] * c];
        let tgt = target(f, k, air, w.lead, ub, w.gap);
        let reach = f.balls[k].blade_len;
        hop_launch(f, k, air16, 0, tgt, reach);
    }
}

/// `tiptoe_step(k, dt)` (3412 to 3422), from `run()` 6171 to 6172: a hop whose time has come lands; one in the air
/// moves the ball along the straight line from its start to its landing spot.
pub fn step(f: &mut Fight, k: usize, _dt: f64) {
    let Some(h) = f.tiptoe.hop[k] else {
        return;
    };
    if f.t + 1e-9 >= h.t1 {
        hop_land(f, k);
        return;
    }
    let u = (f.t - h.t0) / py::py_max(1e-9, h.t1 - h.t0);
    f.balls[k].p = [h.p0[0] + (h.p1[0] - h.p0[0]) * u, h.p0[1] + (h.p1[1] - h.p0[1]) * u];
}

/// `trigger`, kind `mountain` (2917 to 2918): `start_mountain(k, ab)` (3423 to 3428). Event `mountain`: the number of
/// hops. (On a ball that is no hopper, a copy's, the dance is set and never danced, as in Python.)
pub fn start_mountain(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    let seq16 = crate::roster::abilities::mountain::seq16(ab).to_vec();
    let n = seq16.len() as i64;
    f.tiptoe.mk[k] = Some(Mk {
        seq16,
        step: param(ab, "step"),
        gap: param(ab, "gap"),
        reach: param(ab, "reach"),
        i: 0,
        next: None,
        ang: None,
    });
    f.tiptoe.mk_ab[k] = Some(MkAb {
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
    });
    f.tiptoe.hop_bonus[k] = false;
    f.ev("mountain", k as i64, vec![Value::Int(n)]);
}

/// The record keys this kind adds (Python's `rec` gets them at setup): `run()` 5834 to 5835: `HOP`, when a fighter
/// is a hopper.
pub fn record_keys(f: &Fight) -> &'static [&'static str] {
    if (0..2).any(|k| is(f, k)) { &["HOP"] } else { &[] }
}

/// This kind's rows of one frame: `snap`, `HOP` (5609 to 5615): per ball, a hop's progress (0 to 1), its start time
/// and its landing time, or three times -1 on the ground.
pub(crate) fn snap(f: &Fight, out: &mut Frames) {
    let mut row = [-1.0; 6];
    for (j, hop) in f.tiptoe.hop.iter().enumerate() {
        if let Some(h) = hop {
            row[3 * j] = (f.t - h.t0) / py::py_max(1e-9, h.t1 - h.t0);
            row[3 * j + 1] = h.t0;
            row[3 * j + 2] = h.t1;
        }
    }
    out.push_f64("HOP", &[2, 3], &row);
}
