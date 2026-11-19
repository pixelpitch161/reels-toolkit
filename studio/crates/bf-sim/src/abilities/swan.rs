//! Ability kind `swan` (CELLO's SWAN SONG): `fight.py` `trigger` 2951 to 2952; 4244 to 4301 (`start_swan`,
//! `swan_step`).
//!
//! The fighter rises for `rise_beats` (letting go of anything it holds), glides for `glide_beats` along a quadratic
//! Bezier from where it rose to the enemy's spot predicted `lead` x 0.6 of its motion to the landing (the curve bows
//! `bow` toward the ring's centre), then dives for `dive_beats` and crashes there: an enemy within `radius` (+0.6 R)
//! takes `dmg` and is knocked away (`knock`). From the rise to the landing it is airborne (`Ball::swan`): it can't be
//! hit or knocked, doesn't clash, and its weapon rests. A copied ability (IDENTITY CRISIS) runs this code for the
//! copier's ball, so nothing here assumes which side has the kind.

use bf_core::dmath;
use bf_core::event::Value;
use bf_core::py;

use crate::abilities::param;
use crate::fight::Fight;
use crate::geom::{self, V2};
use crate::kinds::grapple;
use crate::roster::AbilityCfg;

/// This ability's per-fight state (`Fight::swan`): nothing beyond the ball's [`Swan`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {}

/// `B.swan` while SWAN SONG flies (`start_swan` 4251 to 4273): `Ball::swan`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Swan {
    /// When it rose.
    pub t0: f64,
    /// When the glide starts.
    pub tg: f64,
    /// When the dive starts.
    pub td: f64,
    /// When it lands.
    pub tl: f64,
    /// Where it rose.
    pub p0: V2,
    /// The curve's control point.
    pub c: V2,
    /// Where it lands.
    pub p1: V2,
    /// `ab['radius']`, `ab['dmg']`, `ab['knock']` (read at the landing).
    pub radius: f64,
    /// See `radius`.
    pub dmg: f64,
    /// See `radius`.
    pub knock: f64,
}

/// `np.clip(p, lo, hi)` on a 2-vector.
fn clip(p: V2, lo: f64, hi: f64) -> V2 {
    [geom::np_clip(p[0], lo, hi), geom::np_clip(p[1], lo, hi)]
}

/// `trigger`, kind `swan`: `trigger` 2951 to 2952, `start_swan` (4251 to 4272). Event `swan`: now, the glide's, the
/// dive's and the landing's times, the curve's three points and `height` (default 1.5).
pub fn start(f: &mut Fight, k: usize, ab: &AbilityCfg) {
    // `if getattr(B, 'gr', None) is not None`: only a grappler holds or rushes
    if grapple::is(f, k) && f.grapple.gr[k].is_some() {
        grapple::release(f, k, "swan");
    }
    let bt = f.rules.beat;
    let tg = f.t + param(ab, "rise_beats") * bt;
    let td = tg + param(ab, "glide_beats") * bt;
    let tl = td + param(ab, "dive_beats") * bt;
    let lim = f.rules.a - f.rules.r;
    let p0 = f.balls[k].p;
    let o = &f.balls[1 - k];
    let lead = ab.get("lead").unwrap_or(1.0);
    // p1 = np.clip(O.p + O.v * (tl - self.t) * ab.get('lead', 1.0) * 0.6, -lim, lim)
    let ahead = geom::scale(geom::scale(geom::scale(o.v, tl - f.t), lead), 0.6);
    let p1 = clip(geom::add(o.p, ahead), -lim, lim);
    let d = geom::sub(p1, p0);
    let nd = geom::hypot(d);
    let mut nrm = if nd > 1e-6 {
        geom::div([-d[1], d[0]], nd)
    } else {
        [0.0, 1.0]
    };
    let mid = geom::div(geom::add(p0, p1), 2.0);
    // bow toward the centre (the curve stays inside)
    if geom::dot(nrm, [-mid[0], -mid[1]]) < 0.0 {
        nrm = [-nrm[0], -nrm[1]];
    }
    let c = clip(geom::add(mid, geom::scale(nrm, param(ab, "bow"))), -lim, lim);
    let t0 = f.t;
    let b = &mut f.balls[k];
    b.vmul = None;
    b.swan = Some(Swan {
        t0,
        tg,
        td,
        tl,
        p0,
        c,
        p1,
        radius: param(ab, "radius"),
        dmg: param(ab, "dmg"),
        knock: param(ab, "knock"),
    });
    let data = [
        t0,
        tg,
        td,
        tl,
        p0[0],
        p0[1],
        c[0],
        c[1],
        p1[0],
        p1[1],
        ab.get("height").unwrap_or(1.5),
    ];
    f.ev("swan", k as i64, data.iter().map(|&x| Value::Float(x)).collect());
}

/// `swan_step(k)` (4274 to 4301), from `run()` 6179 to 6180: the rise (it stays where it rose), the glide along the
/// eased curve (its heading along it), the dive (over the landing spot) and the landing. Event `swan_land`: where,
/// and 1 if it hit.
pub fn swan_step(f: &mut Fight, k: usize) {
    let Some(sw) = f.balls[k].swan.clone() else {
        return;
    };
    let t = f.t;
    if t < sw.tg {
        f.balls[k].p = sw.p0;
    } else if t < sw.td {
        let u = (t - sw.tg) / py::py_max(1e-9, sw.td - sw.tg);
        let u = u * u * (3.0 - 2.0 * u);
        // (1 - u) ** 2 * p0 + 2 * u * (1 - u) * c + u * u * p1
        let a = dmath::pow(1.0 - u, 2.0);
        let m = 2.0 * u * (1.0 - u);
        let e = u * u;
        let p = geom::add(
            geom::add(geom::scale(sw.p0, a), geom::scale(sw.c, m)),
            geom::scale(sw.p1, e),
        );
        f.balls[k].p = p;
        // dv = 2 * (1 - u) * (c - p0) + 2 * u * (p1 - c)
        let dv = geom::add(
            geom::scale(geom::sub(sw.c, sw.p0), 2.0 * (1.0 - u)),
            geom::scale(geom::sub(sw.p1, sw.c), 2.0 * u),
        );
        if geom::hypot(dv) > 1e-9 {
            // its heading along the glide
            let v_shared = f.rules.v;
            f.balls[k].v = geom::scale(geom::div(dv, geom::hypot(dv)), v_shared);
        }
    } else if t < sw.tl {
        f.balls[k].p = sw.p1;
    } else {
        f.balls[k].p = sw.p1;
        f.balls[k].swan = None;
        let op = f.balls[1 - k].p;
        let rel = geom::sub(op, sw.p1);
        let dist = geom::hypot(rel);
        let hit = dist < sw.radius + f.rules.r * 0.6;
        if hit {
            f.damage(k, 1 - k, sw.dmg, "swan", op, 0);
            let away = if dist > 1e-3 { rel } else { f.balls[1 - k].v };
            f.knock(1 - k, away, sw.knock);
        }
        let data = vec![
            Value::Float(sw.p1[0]),
            Value::Float(sw.p1[1]),
            Value::Int(i64::from(hit)),
        ];
        f.ev("swan_land", k as i64, data);
    }
}
