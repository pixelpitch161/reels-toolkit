//! `Flares`: attacks light the scene. Short point lights in a fighter's colour.

use std::cmp::Ordering;

use crate::camera::V3;
use crate::lighting::Lights;
use crate::np;
use crate::v3;
use bf_core::py::py_max;

use super::Timeline;
use super::timing::env_ad;

/// One flare as `add` keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Flare {
    pos: V3,
    /// `_norm_rgb(rgb) * peak`.
    rgb: V3,
    radius: f64,
    attack: f64,
    decay: f64,
    hold: f64,
}

/// `Flares`: short-lived point lights. Each rises over `attack`, holds for `hold` and decays with `decay`
/// ([`env_ad`]); [`Flares::apply`] adds the strongest `max_n` live ones to the scene's lights.
#[derive(Clone, Debug)]
pub struct Flares {
    tl: Timeline<Flare>,
    /// The most lights given at one time.
    pub max_n: usize,
}

/// `c.max()` of a colour.
fn top(c: V3) -> f64 {
    np::maximum(np::maximum(c[0], c[1]), c[2])
}

/// `_norm_rgb(rgb)`: the colour divided by its largest channel, or by 1 if that is smaller.
fn norm_rgb(rgb: V3) -> V3 {
    v3::div(rgb, py_max(1.0, top(rgb)))
}

impl Flares {
    /// `Flares(max_n, span)`. The Python defaults are 6 and 3.0.
    pub fn new(max_n: usize, span: f64) -> Flares {
        Flares {
            tl: Timeline::new(span),
            max_n,
        }
    }

    /// `add(t0, pos, rgb, radius, peak, attack, decay, hold)`. The Python defaults are `radius=2.6`, `peak=1.0`,
    /// `attack=0.03`, `decay=0.25`, `hold=0.0`.
    #[allow(clippy::too_many_arguments)] // the Python signature
    pub fn add(&mut self, t0: f64, pos: V3, rgb: V3, radius: f64, peak: f64, attack: f64, decay: f64, hold: f64) {
        self.tl.add(
            t0,
            Flare {
                pos,
                rgb: v3::scale(norm_rgb(rgb), peak),
                radius,
                attack,
                decay,
                hold,
            },
        );
    }

    /// The number of flares added.
    pub fn len(&self) -> usize {
        self.tl.len()
    }

    /// Whether no flare was added.
    pub fn is_empty(&self) -> bool {
        self.tl.len() == 0
    }

    /// `at(t)`: the lights at `t` as `(position, rgb, radius)`, the strongest first (by its largest channel;
    /// equal ones in time order), at most `max_n`. A flare at 2 % of its peak or less is left out.
    pub fn at(&self, t: f64) -> Vec<(V3, V3, f64)> {
        let mut out = Vec::new();
        for (t0, _, f) in self.tl.alive(t) {
            let k = env_ad(t - t0, f.attack, f.decay, f.hold);
            if k > 0.02 {
                out.push((f.pos, v3::scale(f.rgb, k), f.radius));
            }
        }
        // out.sort(key=lambda q: -float(q[1].max())): a stable sort
        out.sort_by(|a, b| (-top(a.1)).partial_cmp(&-top(b.1)).unwrap_or(Ordering::Equal));
        out.truncate(self.max_n);
        out
    }

    /// `apply(L, t)`: adds the lights of [`Flares::at`] to `l` as point lights.
    pub fn apply(&self, l: &mut Lights, t: f64) {
        for (p, c, r) in self.at(t) {
            l.point(p, c, r);
        }
    }

    /// `tint(P, t, gain)`: the RGB light the live flares cast on each point of `p`, with the falloff
    /// `1 / (1 + (d / R)^2)`, for what is not shaded through [`Lights`] (the crowd, the particles). Python's
    /// default gain is 1.0.
    pub fn tint(&self, p: &[V3], t: f64, gain: f64) -> Vec<V3> {
        let mut out = vec![[0.0; 3]; p.len()];
        for (pos, c, r) in self.at(t) {
            for (o, &q) in out.iter_mut().zip(p) {
                let d = v3::norm(v3::sub(q, pos)) / r;
                let w = 1.0 / (1.0 + d * d);
                *o = v3::add(*o, v3::scale(c, w));
            }
        }
        out.iter().map(|&o| v3::scale(o, gain)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_is_normalised_by_its_largest_channel() {
        assert_eq!(norm_rgb([200.0, 100.0, 50.0]), [1.0, 0.5, 0.25]);
        assert_eq!(norm_rgb([0.5, 0.25, 0.0]), [0.5, 0.25, 0.0]);
    }

    #[test]
    fn the_strongest_lights_come_first_and_only_max_n() {
        let mut f = Flares::new(2, 3.0);
        assert!(f.is_empty());
        f.add(0.0, [0.0; 3], [255.0, 0.0, 0.0], 2.0, 0.5, 0.03, 0.25, 1.0);
        f.add(0.0, [1.0, 0.0, 0.0], [0.0, 255.0, 0.0], 2.0, 1.0, 0.03, 0.25, 1.0);
        f.add(0.0, [2.0, 0.0, 0.0], [0.0, 0.0, 255.0], 2.0, 1.0, 0.03, 0.25, 1.0);
        f.add(5.0, [3.0, 0.0, 0.0], [9.0, 9.0, 9.0], 2.0, 1.0, 0.03, 0.25, 0.0);
        assert_eq!(f.len(), 4);
        let at = f.at(0.5);
        assert_eq!(at.len(), 2);
        assert_eq!((at[0].0, at[1].0), ([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]));
        assert_eq!(at[0].1, [0.0, 1.0, 0.0]);
        assert_eq!(Flares::new(6, 3.0).tint(&[[0.0; 3]], 0.5, 1.0), [[0.0; 3]]);
        // at the light itself the tint is its colour; at one radius, half of it
        let tint = f.tint(&[[1.0, 0.0, 0.0], [1.0, 2.0, 0.0]], 0.5, 2.0);
        assert_eq!(tint[0][1], 2.0);
        assert_eq!(tint[1][1], 1.0);
        let mut l = Lights::new([0.3; 3], [0.1; 3]);
        f.apply(&mut l, 0.5);
        assert_eq!(l.points.len(), 2);
    }
}
