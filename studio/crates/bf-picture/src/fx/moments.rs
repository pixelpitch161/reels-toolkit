//! `Moments`: ability moments. As an ability's name punches in, the background dims (30 % at most, eased) except
//! soft discs around the fighters, and speed lines stream behind the fighter whose moment it is.

use crate::camera::V3;
use crate::img::Img;
use crate::np;
use crate::pil::blend;
use bf_core::dmath;
use bf_core::py::{fmod_py, py_max, py_min};
use bf_core::rng::Generator;

use super::timing::smooth;
use super::{Px, Timeline, byte, bytes_of, floats_of, ink};

/// One moment as `add` keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moment {
    /// The fighter it belongs to (the ring uses 0 and 1).
    pub key: i64,
    /// The fighter's colour.
    pub rgb: V3,
    /// Its strength.
    pub k: f64,
}

/// A moment at work at some time: what `Moments.active` lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Active<'a> {
    /// The moment.
    pub moment: &'a Moment,
    /// The number of the `add` call that made it (0, 1, ...).
    pub which: usize,
    /// Its envelope times its strength.
    pub e: f64,
    /// Its age in seconds.
    pub u: f64,
}

/// `Moments`.
#[derive(Clone, Debug)]
pub struct Moments {
    tl: Timeline<Moment>,
    max_dim: f64,
    hold: f64,
    t_in: f64,
    t_out: f64,
}

impl Moments {
    /// `Moments(max_dim, hold, t_in, t_out)`: the dim is `max_dim` at most (never above 0.3), eased in over
    /// `t_in`, held for `hold` and eased out over `t_out`. The Python defaults are 0.3, 0.55, 0.14 and 0.4.
    pub fn new(max_dim: f64, hold: f64, t_in: f64, t_out: f64) -> Moments {
        Moments {
            tl: Timeline::new(t_in + hold + t_out + 0.1),
            max_dim: py_min(0.3, max_dim),
            hold,
            t_in,
            t_out,
        }
    }

    /// `add(t0, key, rgb, k)`: a moment of the fighter `key` at `t0`. Python's default `k` is 1.0.
    pub fn add(&mut self, t0: f64, key: i64, rgb: V3, k: f64) {
        self.tl.add(t0, Moment { key, rgb, k });
    }

    /// The number of moments added.
    pub fn len(&self) -> usize {
        self.tl.len()
    }

    /// Whether no moment was added.
    pub fn is_empty(&self) -> bool {
        self.tl.len() == 0
    }

    /// `env(t, t0)`: the envelope at `t` of a moment that began at `t0`.
    pub fn env(&self, t: f64, t0: f64) -> f64 {
        let u = t - t0;
        if u < 0.0 {
            return 0.0;
        }
        smooth(u / self.t_in) * (1.0 - smooth((u - self.t_in - self.hold) / self.t_out))
    }

    /// `active(t)`: the moments whose envelope is above 0.01 at `t`, in time order.
    pub fn active(&self, t: f64) -> Vec<Active<'_>> {
        self.tl
            .alive(t)
            .filter_map(|(t0, which, moment)| {
                let env = self.env(t, t0);
                (env > 0.01).then_some(Active {
                    moment,
                    which,
                    e: env * moment.k,
                    u: t - t0,
                })
            })
            .collect()
    }

    /// `dim_k(t)`: how much the background is dimmed at `t` (0 to `max_dim`).
    pub fn dim_k(&self, t: f64) -> f64 {
        // max([e for m, e, u in self.active(t)] + [0.0])
        let mut top: Option<f64> = None;
        for e in self.active(t).iter().map(|a| a.e).chain([0.0]) {
            top = Some(match top {
                None => e,
                Some(m) => py_max(m, e),
            });
        }
        py_min(self.max_dim, top.unwrap_or(0.0) * self.max_dim)
    }

    /// `apply_dim(img, t, keep)`: darkens the picture by [`Moments::dim_k`], except soft discs
    /// `keep = [(x, y, r)]` (the fighters stay lit). Returns whether it did (not for a dim of 0.005 or less:
    /// Python then returns the picture it was given).
    ///
    /// The picture is the `float64` one the ring has after the grade, and the discs' centres are `float64` too
    /// (`cam.wp`), which is what makes numpy compute the discs in `float64`. Without discs the factor is a
    /// `float32` value, as numpy has it then.
    pub fn apply_dim(&self, img: &mut Img<f64>, t: f64, keep: &[(f64, f64, f64)]) -> bool {
        let k = self.dim_k(t);
        if k <= 0.005 {
            return false;
        }
        for yi in 0..img.h {
            for xi in 0..img.w {
                let factor = if keep.is_empty() {
                    // lit stays a float32 array of zeros, and a Python float does not widen it
                    f64::from(1.0f32 - k as f32 * (1.0f32 - 0.0f32))
                } else {
                    let (xx, yy) = (xi as f64, yi as f64);
                    let mut lit = 0.0f64;
                    for &(x, y, r) in keep {
                        // np.hypot is the C runtime's, which is the plain root of the sum of the squares
                        let (dx, dy) = (xx - x, yy - y);
                        let d = (dx * dx + dy * dy).sqrt();
                        lit = np::maximum(lit, np::clip(1.6 - d / py_max(r, 1.0), 0.0, 1.0));
                    }
                    1.0 - k * (1.0 - lit)
                };
                for v in img.px_mut(xi, yi) {
                    *v *= factor;
                }
            }
        }
        true
    }

    /// `speed_lines(img, t, key, x, y, r, direction, n, length, seed)`: speed lines streaming behind the fighter
    /// `key` (screen centre `x`, `y`, radius `r`), parallel to `direction` (the way it moves; they stream the
    /// other way), spread across a band 3.2 `r` wide. The Python defaults are `direction=(0.0, 1.0)`, `n=16`,
    /// `length=(16, 46)`, `seed=0`.
    ///
    /// Returns the new picture, `float32` holding whole numbers, or `None` when the fighter has no moment at
    /// `t`: Python then returns the picture it was given.
    ///
    /// The lines come from `np.random.default_rng(seed + key * 101)`: per line its offset across the band, its
    /// length, its speed and its phase.
    ///
    /// # Panics
    /// Where numpy raises: `seed + key * 101` is negative.
    #[allow(clippy::too_many_arguments)] // the Python signature
    pub fn speed_lines<T: Px>(
        &self,
        img: &Img<T>,
        t: f64,
        key: i64,
        x: f64,
        y: f64,
        r: f64,
        direction: (f64, f64),
        n: usize,
        length: (f64, f64),
        seed: u64,
    ) -> Option<Img<f32>> {
        // max(act, key=lambda q: q[1]): the first of the strongest
        let mut best: Option<Active<'_>> = None;
        for a in self.active(t).into_iter().filter(|a| a.moment.key == key) {
            if best.is_none_or(|b| a.e > b.e) {
                best = Some(a);
            }
        }
        let Active { moment, e, u, .. } = best?;
        let (mut dx, mut dy) = direction;
        let mut len = dmath::hypot(dx, dy);
        if len < 1e-6 {
            (dx, dy, len) = (0.0, 1.0, 1.0);
        }
        let (dx, dy) = (dx / len, dy / len);
        let (px, py) = (-dy, dx);
        let seed = i64::try_from(seed)
            .ok()
            .and_then(|s| key.checked_mul(101).and_then(|k| s.checked_add(k)))
            .and_then(|s| u64::try_from(s).ok())
            .unwrap_or_else(|| panic!("speed_lines: the seed {seed} + {key} * 101 is not a seed"));
        let mut rng = Generator::new(seed);
        let mut base = bytes_of(img);
        let col = moment.rgb.map(|v| np::clip(v * 0.6 + 110.0, 0.0, 255.0));
        for i in 0..n {
            let off = rng.uniform(-1.6, 1.6) * r;
            let ln = rng.uniform(length.0, length.1);
            // pixels per second the line streams back
            let sp = rng.uniform(260.0, 420.0);
            let ph = rng.uniform(0.0, 1.0);
            let s0 = r * 0.7 + fmod_py(u * sp / 60.0 + ph, 1.0) * 30.0;
            let (ax, ay) = (x - dx * s0 + px * off, y - dy * s0 + py * off);
            let (bx, by) = (ax - dx * ln, ay - dy * ln);
            let a = byte((255.0 * e * (0.5 + 0.5 * (1.0 - off.abs() / (1.6 * r)))) as i64);
            let wd = if off.abs() < 0.6 * r && i % 3 == 0 { 2 } else { 1 };
            blend::line(&mut base, &[(ax, ay), (bx, by)], ink(col, a), wd);
            blend::point(&mut base, &[(ax, ay)], [255, 255, 255, a]);
        }
        Some(floats_of(&base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one() -> Moments {
        let mut mo = Moments::new(0.3, 0.55, 0.14, 0.4);
        mo.add(1.0, 0, [222.0, 232.0, 246.0], 1.0);
        mo
    }

    #[test]
    fn the_dim_eases_in_holds_and_eases_out() {
        let mo = one();
        assert_eq!(mo.len(), 1);
        assert!(!mo.is_empty());
        assert_eq!(mo.env(0.9, 1.0), 0.0);
        assert_eq!(mo.dim_k(0.9), 0.0);
        assert_eq!(mo.env(1.07, 1.0), mo.active(1.07)[0].e);
        assert!((mo.env(1.07, 1.0) - 0.5).abs() < 1e-9);
        assert_eq!(mo.dim_k(1.4), 0.3);
        assert!((mo.dim_k(1.89) - 0.15).abs() < 1e-9);
        assert_eq!(mo.dim_k(2.2), 0.0);
        // the dim never passes 30 %, whatever is asked for or however strong the moment
        let mut deep = Moments::new(0.8, 0.55, 0.14, 0.4);
        deep.add(1.0, 0, [0.0; 3], 5.0);
        assert_eq!(deep.dim_k(1.4), 0.3);
    }

    #[test]
    fn the_fighters_stay_lit() {
        let mo = one();
        let mut img: Img<f64> = Img::filled(40, 30, &[100.0, 50.0, 200.0]);
        assert!(!mo.apply_dim(&mut img, 0.5, &[(10.0, 10.0, 4.0)]));
        assert_eq!(img.px(0, 0), [100.0, 50.0, 200.0]);
        assert!(mo.apply_dim(&mut img, 1.4, &[(10.0, 10.0, 4.0)]));
        // inside 0.6 r the disc is fully lit; far away the picture loses 30 %
        assert_eq!(img.px(10, 10), [100.0, 50.0, 200.0]);
        assert_eq!(img.px(12, 10), [100.0, 50.0, 200.0]);
        assert_eq!(img.px(39, 29), [100.0 * 0.7, 50.0 * 0.7, 200.0 * 0.7]);
        // without discs the factor is the float32 value of 1 - 0.3
        let mut flat: Img<f64> = Img::filled(2, 2, &[100.0, 50.0, 200.0]);
        assert!(mo.apply_dim(&mut flat, 1.4, &[]));
        assert_eq!(flat.px(0, 0)[0], 100.0 * f64::from(1.0f32 - 0.3f32));
        assert_ne!(flat.px(0, 0)[0], 100.0 * 0.7);
    }

    #[test]
    fn speed_lines_are_drawn_for_the_fighter_of_the_moment_only() {
        let mo = one();
        let img: Img<f64> = Img::filled(120, 100, &[40.2, 50.7, 60.9]);
        assert!(
            mo.speed_lines(&img, 1.4, 1, 60.0, 50.0, 12.0, (1.0, 0.0), 16, (16.0, 46.0), 0)
                .is_none()
        );
        assert!(
            mo.speed_lines(&img, 0.5, 0, 60.0, 50.0, 12.0, (1.0, 0.0), 16, (16.0, 46.0), 0)
                .is_none()
        );
        let out = mo
            .speed_lines(&img, 1.4, 0, 60.0, 50.0, 12.0, (1.0, 0.0), 16, (16.0, 46.0), 0)
            .expect("the moment is on");
        // the picture comes back as bytes; the lines are behind the fighter (it moves to the right)
        assert_eq!(out.px(119, 0), [40.0, 50.0, 60.0]);
        let drawn: Vec<usize> = (0..120)
            .filter(|&x| (0..100).any(|y| out.px(x, y) != [40.0, 50.0, 60.0]))
            .collect();
        assert!(!drawn.is_empty() && drawn.iter().all(|&x| x < 60), "{drawn:?}");
    }
}
