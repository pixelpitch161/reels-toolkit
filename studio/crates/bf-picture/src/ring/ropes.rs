//! The ropes: `ROPE_EV`, `WAVE_EV` and `rope_points` of `kit/picture.py`. A rope sags a little, gives where a
//! ball hits it, wobbles with a fighter's rings and thuds with the bass.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::py::py_min;

use super::PictureCtx;
use crate::camera::V3;
use crate::look::Look;
use crate::np;
use crate::scene::{self, CORNERS, ROPE_SIDES};

/// One entry of `ROPE_EV`: `(t, side, x, z, speed)` of a ball bouncing off a rope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RopeEv {
    pub t: f64,
    /// The rope's side, an index into [`ROPE_SIDES`].
    pub side: usize,
    pub x: f64,
    pub z: f64,
    pub spd: f64,
}

/// `SIDE_OF[(axis, sign)]`: the rope side of a `rope` event, or `None` where Python raises `KeyError`.
fn side_of(axis: i64, sign: i64) -> Option<usize> {
    match (axis, sign) {
        (1, 1) => Some(0),
        (0, 1) => Some(1),
        (1, -1) => Some(2),
        (0, -1) => Some(3),
        _ => None,
    }
}

/// `ROPE_EV`: `[(e[0], SIDE_OF[(e[3], e[4])], e[5], e[6], e[7]) for e in EV if e[1] == 'rope']`. The events are
/// checked before ([`super::check_events`]); one that still has no side is left out.
pub(super) fn rope_events(ev: &[Event]) -> Vec<RopeEv> {
    ev.iter()
        .filter(|e| e.kind == "rope")
        .filter_map(|e| {
            Some(RopeEv {
                t: e.t,
                side: side_of(e.i(3), e.i(4))?,
                x: e.f(5),
                z: e.f(6),
                spd: e.f(7),
            })
        })
        .collect()
}

/// `WAVE_EV`: `(time, strength)` of every event that shakes the ropes: the abilities first, then the others (the
/// order `kit/picture.py` builds the list in; the list is not sorted by time).
pub(super) fn wave_events(p: &PictureCtx, looks: &[Box<dyn Look>]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for abilities in [true, false] {
        for e in p.rec.events.iter().filter(|e| (e.kind == "ability") == abilities) {
            // LOOKS[e[2]].rope(e)
            if let Some(k) = super::owner(e).and_then(|k| looks[k].rope(p, e)) {
                out.push((e.t, k));
            }
        }
    }
    out
}

/// `np.linspace(0, 1, n)`.
fn linspace01(n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![0.0];
    }
    let step = 1.0 / (n - 1) as f64;
    (0..n)
        .map(|k| if k == n - 1 { 1.0 } else { k as f64 * step + 0.0 })
        .collect()
}

impl PictureCtx {
    /// `side_ends(s)`: the two corner posts a rope of side `s` (an index into [`ROPE_SIDES`]) runs between, as
    /// ring-frame `(x, z)`: `S.CORNERS[a], S.CORNERS[b]` for `a, b = S.ROPE_SIDES[s]`.
    ///
    /// # Panics
    /// For a side above 3 (Python's `IndexError`).
    pub fn side_ends(&self, s: usize) -> ([f64; 2], [f64; 2]) {
        let (ca, cb) = ROPE_SIDES[s];
        (CORNERS[ca], CORNERS[cb])
    }

    /// `rope_points(s, y, t, n=15)`: one rope (side `s`, height `y`) as `n` world points: a slight sag and the
    /// give where balls hit it, decaying. Python's default `n` is 15.
    pub fn rope_points(&self, s: usize, y: f64, t: f64, n: usize) -> Vec<V3> {
        let (a, b) = self.side_ends(s);
        let ab = [b[0] - a[0], b[1] - a[1]];
        let u = linspace01(n);
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        let mn = (mid[0] * mid[0] + mid[1] * mid[1]).sqrt();
        let outn = [mid[0] / mn, mid[1] / mn];
        let mut disp = vec![0.0f64; n];
        // i0 = np.searchsorted(ROPE_T, t - 0.9); i1 = np.searchsorted(ROPE_T, t, 'right')
        let i0 = self.rope_ev.partition_point(|r| r.t < t - 0.9);
        let i1 = self.rope_ev.partition_point(|r| r.t <= t);
        for r in self.rope_ev.get(i0..i1).unwrap_or(&[]) {
            if r.side != s {
                continue;
            }
            let dt = t - r.t;
            // uc = float(np.clip(((np.array([x, z]) - a) @ (b - a)) / ((b - a) @ (b - a)), 0, 1))
            let d = [r.x - a[0], r.z - a[1]];
            let uc = np::clip(
                (d[0] * ab[0] + d[1] * ab[1]) / (ab[0] * ab[0] + ab[1] * ab[1]),
                0.0,
                1.0,
            );
            let amp = 0.16 * py_min(1.6, r.spd / 2.6) * dmath::exp(-dt / 0.22) * dmath::cos(2.0 * dmath::PI * 5.0 * dt);
            let lift = 1.0 - (y - 0.4) * 0.5;
            for (dv, &uv) in disp.iter_mut().zip(&u) {
                // disp += amp * np.exp(-((u - uc) / 0.16) ** 2) * (1 - (y - 0.4) * 0.5)
                let q = (uv - uc) / 0.16;
                *dv += amp * dmath::exp(-(q * q)) * lift;
            }
        }
        for &(te, k) in &self.wave_ev {
            let dt = t - te;
            if 0.15 < dt && dt < 1.2 {
                let w = 0.07 * k * dmath::exp(-(dt - 0.15) / 0.3) * dmath::sin(2.0 * dmath::PI * 4.0 * (dt - 0.15));
                for (dv, &uv) in disp.iter_mut().zip(&u) {
                    *dv += w * dmath::sin(dmath::PI * uv);
                }
            }
        }
        // the bass thud
        let thud = 0.012 * self.show.grid.beat_pulse(t, 0.1, 0.12);
        u.iter()
            .zip(&disp)
            .map(|(&uv, &dv)| {
                let wave = dmath::sin(dmath::PI * uv);
                let dv = dv + thud * wave;
                let sag = 0.035 * wave;
                // P = a + (b - a) * u; P = P + outn * disp
                let px = a[0] + ab[0] * uv + outn[0] * dv;
                let pz = a[1] + ab[1] * uv + outn[1] * dv;
                scene::to_world(px, pz, y - sag)
            })
            .collect()
    }
}
