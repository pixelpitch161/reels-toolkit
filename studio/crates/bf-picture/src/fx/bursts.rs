//! Impact bursts: `starburst` (an ink-outlined spiky star) and `Bursts` (a filled star on the frame of the hit,
//! a larger hollow one on the next, then gone).

use std::sync::OnceLock;

use crate::camera::{Cam, V3};
use crate::config::FPS;
use crate::img::Img;
use crate::np;
use crate::pil;
use bf_core::dmath;
use bf_core::py::{py_min, round_half_even};
use bf_core::rng::Generator;

use super::timing::FlashBudget;
use super::{Px, Timeline, byte, ink};

/// `INK`: the outline colour of a star.
pub const INK: V3 = [18.0, 10.0, 26.0];

/// `np.clip(c * k + add, 0, 255)` of a colour.
fn tone(c: V3, k: f64, add: f64) -> V3 {
    c.map(|v| np::clip(v * k + add, 0.0, 255.0))
}

/// `_blend_patch(img, patch, x0, y0)`: alpha-composites an RGBA patch onto the float picture at `(x0, y0)`,
/// cut to the picture.
fn blend_patch<T: Px>(img: &mut Img<T>, patch: &Img<u8>, x0: i64, y0: i64) {
    let (w, h) = (patch.w as i64, patch.h as i64);
    let (xa, ya) = (x0.max(0), y0.max(0));
    let (xb, yb) = (
        (img.w as i64).min(x0.saturating_add(w)),
        (img.h as i64).min(y0.saturating_add(h)),
    );
    if xb <= xa || yb <= ya {
        return;
    }
    for y in ya..yb {
        for x in xa..xb {
            let src = patch.px((x - x0) as usize, (y - y0) as usize);
            // sub = patch.astype(np.float32); a = sub[..., 3:4] / 255.0
            let a = f32::from(src[3]) / 255.0;
            let dst = img.px_mut(x as usize, y as usize);
            for c in 0..3 {
                dst[c] = dst[c].over(a, f32::from(src[c]));
            }
        }
    }
}

/// `starburst(img, x, y, r, rgb, k, spikes, seed, hollow, bright)`: an ink-outlined star of `spikes` spikes
/// centred at `(x, y)`, radius `r` pixels, mixed into the float picture with opacity `k`. `bright`: a hot core
/// in the colour; not bright: a darker ink star (a hit past the flash budget); `hollow`: the outline alone (the
/// second frame). The Python defaults are `k=1.0`, `spikes=9`, `seed=0`, `hollow=False`, `bright=True`.
///
/// The star's shape comes from `np.random.default_rng(seed)`: one draw for the turn, then `2 * spikes` each for
/// the outer radii, the inner radii and the jitter of the angles.
///
/// # Panics
/// Where Python raises: `r` is not finite, or there are no spikes.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn starburst<T: Px>(
    img: &mut Img<T>,
    x: f64,
    y: f64,
    r: f64,
    rgb: V3,
    k: f64,
    spikes: usize,
    seed: u64,
    hollow: bool,
    bright: bool,
) {
    assert!(r.is_finite(), "starburst: a radius of {r}");
    assert!(spikes > 0, "starburst: no spikes");
    assert!(x.is_finite() && y.is_finite(), "starburst: a centre of ({x}, {y})");
    let mut rng = Generator::new(seed);
    let n = 2 * spikes;
    let rot = rng.uniform(0.0, 2.0 * dmath::PI);
    let mut outer = vec![0.0; n];
    let mut inner = vec![0.0; n];
    let mut jitter = vec![0.0; n];
    rng.fill_uniform(0.75, 1.0, &mut outer);
    rng.fill_uniform(0.32, 0.45, &mut inner);
    rng.fill_uniform(-0.12, 0.12, &mut jitter);
    let m = r.ceil() as i64 + 3;
    let mf = m as f64;
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let rad = if i % 2 == 0 { r * outer[i] } else { r * inner[i] };
            // rot + np.arange(n) * 2 * math.pi / n + jitter
            let ang = rot + (i * 2) as f64 * dmath::PI / n as f64 + jitter[i];
            (mf + rad * dmath::cos(ang), mf + rad * dmath::sin(ang))
        })
        .collect();
    let side = usize::try_from(2 * m + 1).expect("starburst: a radius below -3");
    let mut patch: Img<u8> = Img::new(side, side, 4);
    let a = byte((255.0 * py_min(1.0, k)) as i64);
    if hollow {
        pil::polygon_outline(&mut patch, &pts, None, &ink(tone(rgb, 0.6, 100.0), a));
    } else {
        let core = if bright {
            tone(rgb, 0.35, 190.0)
        } else {
            tone(rgb, 0.55, 30.0)
        };
        pil::polygon_outline(&mut patch, &pts, Some(&ink(core, a)[..]), &ink(INK, a));
        let small: Vec<(f64, f64)> = pts
            .iter()
            .map(|&(px, py)| (mf + (px - mf) * 0.45, mf + (py - mf) * 0.45))
            .collect();
        let hot = if bright {
            [255, 252, 240, a]
        } else {
            ink(tone(rgb, 0.8, 60.0), a)
        };
        pil::polygon(&mut patch, &small, &hot);
    }
    blend_patch(
        img,
        &patch,
        round_half_even(x) as i64 - m,
        round_half_even(y) as i64 - m,
    );
}

/// One burst as `add` keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Burst {
    t0: f64,
    pos: V3,
    rgb: V3,
    s: f64,
    bright: bool,
    seed: u64,
}

/// `Bursts`: impact bursts. On the frame of the hit a filled star, on the next a larger hollow one, then gone.
/// Bright cores are budgeted ([`FlashBudget`]); the rest draw as dark ink stars, so dense hits never strobe.
#[derive(Clone, Debug)]
pub struct Bursts {
    tl: Timeline<usize>,
    budget: FlashBudget,
    base: f64,
    per: f64,
    frames: i64,
    ev: Vec<Burst>,
    /// Which bursts are lit, worked out at the first look after an `add`.
    lit: OnceLock<Vec<bool>>,
}

impl Bursts {
    /// `Bursts(max_per_s, base_px, per_strength, frames)`. The Python defaults are 3, 7.0, 9.0 and 2.
    pub fn new(max_per_s: usize, base_px: f64, per_strength: f64, frames: u32) -> Bursts {
        Bursts {
            tl: Timeline::new(f64::from(frames) / f64::from(FPS) + 1e-6),
            budget: FlashBudget::new(max_per_s, 1.0),
            base: base_px,
            per: per_strength,
            frames: i64::from(frames),
            ev: Vec::new(),
            lit: OnceLock::new(),
        }
    }

    /// `add(t0, pos, rgb, strength, seed, bright)`: a burst at the world point `pos`. `strength` (0 to 2) is its
    /// size; `bright = false` is never a bright core (light taps); `seed = None` is Python's default, a seed made
    /// from the number of bursts before this one. The Python defaults are `strength=1.0`, `seed=None`,
    /// `bright=True`.
    pub fn add(&mut self, t0: f64, pos: V3, rgb: V3, strength: f64, seed: Option<u64>, bright: bool) {
        let seed = seed.unwrap_or(self.ev.len() as u64 * 7919 + 13);
        self.tl.add(t0, self.ev.len());
        self.ev.push(Burst {
            t0,
            pos,
            rgb,
            s: strength,
            bright,
            seed,
        });
        self.lit = OnceLock::new();
    }

    /// The number of bursts added.
    pub fn len(&self) -> usize {
        self.ev.len()
    }

    /// Whether no burst was added.
    pub fn is_empty(&self) -> bool {
        self.ev.is_empty()
    }

    /// `_assign()`: for every burst, in the order added, whether its core is bright.
    fn lit(&self) -> &[bool] {
        self.lit.get_or_init(|| {
            let cand: Vec<usize> = (0..self.ev.len()).filter(|&i| self.ev[i].bright).collect();
            let times: Vec<f64> = cand.iter().map(|&i| self.ev[i].t0).collect();
            let strengths: Vec<f64> = cand.iter().map(|&i| self.ev[i].s).collect();
            let ok = self.budget.assign(&times, Some(&strengths));
            let mut lit = vec![false; self.ev.len()];
            for (&i, o) in cand.iter().zip(ok) {
                lit[i] = o;
            }
            lit
        })
    }

    /// `draw(img, cam, t)`: mixes the bursts alive at `t` into the float picture.
    pub fn draw<T: Px>(&self, img: &mut Img<T>, cam: &Cam, t: f64) {
        let lit = self.lit();
        for (t0, _, &i) in self.tl.alive(t) {
            let e = &self.ev[i];
            // int((t - t0) * FPS + 1e-6)
            let fi = ((t - t0) * f64::from(FPS) + 1e-6) as i64;
            if fi >= self.frames {
                continue;
            }
            let s = cam.wp(&[e.pos])[0];
            let r = self.base + self.per * e.s;
            if fi == 0 {
                starburst(img, s.0, s.1, r, e.rgb, 1.0, 9, e.seed, false, lit[i]);
            } else {
                starburst(img, s.0, s.1, r * 1.3, e.rgb, 0.75, 9, e.seed + 1, true, true);
            }
        }
    }

    /// `bright_times()`: the times of the bursts whose core is bright, in the order added.
    pub fn bright_times(&self) -> Vec<f64> {
        let lit = self.lit();
        (0..self.ev.len()).filter(|&i| lit[i]).map(|i| self.ev[i].t0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_outside_the_picture_changes_nothing() {
        let mut img: Img<f64> = Img::filled(20, 12, &[50.0, 60.0, 70.0]);
        let before = img.clone();
        starburst(&mut img, -40.0, 5.0, 6.0, [200.0; 3], 1.0, 9, 3, false, true);
        starburst(&mut img, 10.0, 80.0, 6.0, [200.0; 3], 1.0, 9, 3, false, true);
        assert_eq!(img, before);
        starburst(&mut img, 10.0, 6.0, 6.0, [200.0; 3], 1.0, 9, 3, false, true);
        assert_ne!(img, before);
        // the hot core at the centre
        assert_eq!(img.px(10, 6), [255.0, 252.0, 240.0]);
    }

    #[test]
    fn the_fourth_bright_burst_of_a_second_is_dark() {
        let mut b = Bursts::new(3, 7.0, 9.0, 2);
        assert!(b.is_empty());
        for i in 0..5 {
            b.add(0.1 * f64::from(i), [0.0; 3], [200.0; 3], 1.0, None, i != 1);
        }
        assert_eq!(b.len(), 5);
        assert_eq!(b.bright_times(), [0.0, 0.2, 0.30000000000000004]);
        // a later add is counted at the next look
        b.add(1.05, [0.0; 3], [200.0; 3], 1.0, None, true);
        assert_eq!(b.bright_times().len(), 4);
        assert_eq!(b.ev[5].seed, 5 * 7919 + 13);
    }
}
