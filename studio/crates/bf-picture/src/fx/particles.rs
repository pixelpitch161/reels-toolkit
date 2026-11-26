//! `Particles`: deterministic particles with trails, and the table of presets they are drawn from.
//!
//! [`Particles::emit`] makes particles from a preset with the system's seeded generator (so the emits must come
//! in the same order every time: the ring makes them all from the fight's events when it loads). A particle's
//! position at any time is a closed form (linear drag, gravity, a wobble), its trail is the same particle a
//! little earlier. [`Particles::draw`] projects and paints them.

use crate::camera::{Cam, V3};
use crate::img::Img;
use crate::np;
use crate::pil::blend;
use crate::v3;
use bf_core::dmath;
use bf_core::py::{py_max, py_min, round_half_even};
use bf_core::rng::Generator;

use super::{Px, byte, bytes_of, floats_of, ink};

/// How a particle's head is painted (`_SHAPES`, in Python's order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A soft ellipse with a highlight when it is large.
    Blob,
    /// A disc, or one pixel when it is small.
    Dot,
    /// A turning triangle.
    Shard,
    /// A flat rectangle.
    Square,
    /// One bright pixel (the trail is the streak).
    Streak,
    /// The outline of an ellipse.
    Ring,
}

impl Shape {
    /// The shape's index in Python's `_SHAPES`.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// One entry of `PRESETS`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    /// The key in Python's table.
    pub name: &'static str,
    /// Particles per emit.
    pub n: i64,
    /// Speed in m/s, `(lo, hi)`.
    pub speed: (f64, f64),
    /// The vertical bias added to the direction.
    pub up: f64,
    /// The spread around the direction in degrees (180: all round).
    pub spread: f64,
    /// Gravity: above 0 falls, below 0 rises.
    pub g: f64,
    /// Drag in 1/s.
    pub drag: f64,
    /// Life in seconds, `(lo, hi)`.
    pub life: (f64, f64),
    /// Size in world units, `(lo, hi)`.
    pub size: (f64, f64),
    /// Growth: the size is multiplied by `1 + grow * age`.
    pub grow: f64,
    /// The trail: `(samples, seconds between them)`.
    pub trail: (u32, f64),
    /// The head's shape.
    pub shape: Shape,
    /// The colours a particle picks one of.
    pub cols: &'static [[f64; 3]],
    /// The opacity.
    pub alpha: f64,
    /// The wobble: `(amplitude in m, Hz)`.
    pub wobble: (f64, f64),
    /// The twinkle in Hz (0: none).
    pub twinkle: f64,
    /// Whether it settles on the floor (`y = 0.01`).
    pub ground: bool,
    /// Whether a soft halo is drawn under the head.
    pub glow: bool,
    /// The spin of a shard in rad/s (each particle takes a random share of it).
    pub spin: f64,
}

/// What a preset has when Python's table leaves a field out (`P.get(...)`).
const BASE: Preset = Preset {
    name: "",
    n: 0,
    speed: (0.0, 0.0),
    up: 0.0,
    spread: 0.0,
    g: 0.0,
    drag: 0.0,
    life: (0.0, 0.0),
    size: (0.0, 0.0),
    grow: 0.0,
    trail: (0, 0.0),
    shape: Shape::Blob,
    cols: &[],
    alpha: 1.0,
    wobble: (0.0, 1.0),
    twinkle: 0.0,
    ground: false,
    glow: false,
    spin: 0.0,
};

/// `PRESETS`, in Python's order.
#[rustfmt::skip]
pub static PRESETS: [Preset; 20] = [
    Preset { name: "dust", n: 8, speed: (0.3, 0.9), up: 0.8, spread: 70.0, g: -0.25, drag: 2.2, life: (0.9, 1.6),
             size: (0.06, 0.12), grow: 1.2, trail: (3, 0.05), shape: Shape::Blob,
             cols: &[[176.0, 160.0, 150.0], [150.0, 132.0, 124.0], [206.0, 192.0, 178.0]], alpha: 0.55,
             wobble: (0.08, 1.3), ..BASE },
    Preset { name: "motes", n: 6, speed: (0.2, 0.7), up: 1.0, spread: 80.0, g: -0.35, drag: 1.6, life: (1.0, 1.8),
             size: (0.018, 0.03), grow: 0.0, trail: (5, 0.04), shape: Shape::Dot,
             cols: &[[255.0, 246.0, 222.0], [255.0, 230.0, 170.0], [230.0, 236.0, 255.0]], alpha: 0.95,
             wobble: (0.12, 1.8), twinkle: 7.0, glow: true, ..BASE },
    Preset { name: "steam", n: 7, speed: (0.8, 1.8), up: 1.2, spread: 30.0, g: -0.6, drag: 2.6, life: (0.6, 1.2),
             size: (0.06, 0.1), grow: 1.8, trail: (3, 0.05), shape: Shape::Blob,
             cols: &[[236.0, 240.0, 246.0], [210.0, 218.0, 230.0]], alpha: 0.5, wobble: (0.06, 2.0), ..BASE },
    Preset { name: "smoke", n: 6, speed: (0.2, 0.6), up: 0.6, spread: 90.0, g: -0.2, drag: 1.8, life: (1.4, 2.4),
             size: (0.08, 0.14), grow: 1.0, trail: (3, 0.08), shape: Shape::Blob,
             cols: &[[110.0, 116.0, 126.0], [140.0, 146.0, 152.0], [90.0, 94.0, 104.0]], alpha: 0.45,
             wobble: (0.1, 0.9), ..BASE },
    Preset { name: "shreds", n: 10, speed: (1.5, 3.5), up: 0.8, spread: 180.0, g: 6.0, drag: 1.2, life: (0.8, 1.4),
             size: (0.05, 0.09), trail: (3, 0.03), shape: Shape::Shard,
             cols: &[[64.0, 196.0, 88.0], [190.0, 40.0, 50.0], [240.0, 230.0, 210.0], [30.0, 60.0, 120.0]],
             alpha: 1.0, ground: true, spin: 12.0, ..BASE },
    Preset { name: "puffs", n: 6, speed: (1.2, 2.4), up: 0.2, spread: 50.0, g: 0.0, drag: 3.5, life: (0.4, 0.8),
             size: (0.05, 0.08), grow: 2.2, trail: (4, 0.03), shape: Shape::Blob, cols: &[[240.0, 244.0, 250.0]],
             alpha: 0.4, ..BASE },
    Preset { name: "shards", n: 8, speed: (1.8, 3.6), up: 0.9, spread: 180.0, g: 7.0, drag: 0.8, life: (0.8, 1.3),
             size: (0.05, 0.1), trail: (3, 0.03), shape: Shape::Shard,
             cols: &[[24.0, 22.0, 30.0], [50.0, 46.0, 60.0], [190.0, 245.0, 60.0]], alpha: 1.0, ground: true,
             spin: 14.0, ..BASE },
    Preset { name: "sparks", n: 10, speed: (2.0, 4.5), up: 0.6, spread: 180.0, g: 6.5, drag: 1.0,
             life: (0.25, 0.55), size: (0.015, 0.025), trail: (5, 0.02), shape: Shape::Streak,
             cols: &[[255.0, 236.0, 160.0], [255.0, 200.0, 90.0]], alpha: 1.0, glow: true, ground: true, ..BASE },
    Preset { name: "glitch", n: 6, speed: (0.4, 1.2), up: 0.4, spread: 180.0, g: 0.0, drag: 3.0, life: (0.2, 0.5),
             size: (0.06, 0.14), trail: (0, 0.0), shape: Shape::Square,
             cols: &[[70.0, 228.0, 255.0], [255.0, 60.0, 200.0], [240.0, 255.0, 255.0]], alpha: 0.9,
             twinkle: 15.0, ..BASE },
    Preset { name: "streaks", n: 8, speed: (2.5, 4.0), up: 0.0, spread: 15.0, g: 0.0, drag: 0.6, life: (0.3, 0.6),
             size: (0.012, 0.02), trail: (6, 0.025), shape: Shape::Streak,
             cols: &[[220.0, 228.0, 255.0], [170.0, 180.0, 230.0]], alpha: 0.6, ..BASE },
    Preset { name: "scales", n: 8, speed: (1.0, 2.2), up: 1.0, spread: 120.0, g: 5.0, drag: 1.4, life: (0.7, 1.2),
             size: (0.03, 0.05), trail: (2, 0.03), shape: Shape::Shard,
             cols: &[[222.0, 232.0, 246.0], [180.0, 196.0, 220.0]], alpha: 1.0, spin: 10.0, twinkle: 9.0, ..BASE },
    Preset { name: "glass", n: 12, speed: (1.0, 3.0), up: 0.8, spread: 180.0, g: 6.0, drag: 1.2, life: (0.6, 1.1),
             size: (0.015, 0.03), trail: (3, 0.02), shape: Shape::Dot,
             cols: &[[230.0, 244.0, 255.0], [200.0, 220.0, 255.0]], alpha: 1.0, twinkle: 12.0, glow: true,
             ground: true, ..BASE },
    Preset { name: "foam", n: 10, speed: (0.8, 2.0), up: 1.2, spread: 90.0, g: 4.0, drag: 1.8, life: (0.6, 1.2),
             size: (0.03, 0.06), grow: 0.6, trail: (3, 0.03), shape: Shape::Blob,
             cols: &[[250.0, 252.0, 255.0], [210.0, 236.0, 250.0]], alpha: 0.85, ground: true, ..BASE },
    Preset { name: "chips", n: 8, speed: (1.5, 3.0), up: 0.9, spread: 150.0, g: 7.0, drag: 1.0, life: (0.6, 1.1),
             size: (0.03, 0.06), trail: (2, 0.03), shape: Shape::Shard,
             cols: &[[214.0, 160.0, 100.0], [170.0, 116.0, 64.0], [236.0, 196.0, 140.0]], alpha: 1.0, ground: true,
             spin: 15.0, ..BASE },
    Preset { name: "embers", n: 8, speed: (0.5, 1.5), up: 1.2, spread: 100.0, g: -0.4, drag: 1.5, life: (0.8, 1.5),
             size: (0.015, 0.025), trail: (5, 0.04), shape: Shape::Dot,
             cols: &[[255.0, 150.0, 70.0], [220.0, 110.0, 50.0], [255.0, 200.0, 120.0]], alpha: 1.0,
             wobble: (0.08, 2.5), twinkle: 6.0, glow: true, ..BASE },
    Preset { name: "brass", n: 10, speed: (2.0, 4.0), up: 0.7, spread: 160.0, g: 6.5, drag: 1.0, life: (0.3, 0.6),
             size: (0.015, 0.025), trail: (4, 0.02), shape: Shape::Streak,
             cols: &[[255.0, 214.0, 120.0], [230.0, 176.0, 70.0]], alpha: 1.0, glow: true, ground: true, ..BASE },
    Preset { name: "smear", n: 4, speed: (0.1, 0.3), up: 0.0, spread: 180.0, g: 0.0, drag: 4.0, life: (0.15, 0.3),
             size: (0.04, 0.07), trail: (6, 0.02), shape: Shape::Streak, cols: &[[255.0, 140.0, 120.0]],
             alpha: 0.6, ..BASE },
    Preset { name: "keyshards", n: 6, speed: (1.5, 3.0), up: 0.8, spread: 180.0, g: 7.0, drag: 1.0,
             life: (0.6, 1.0), size: (0.05, 0.08), trail: (2, 0.03), shape: Shape::Shard,
             cols: &[[246.0, 244.0, 236.0], [24.0, 22.0, 30.0]], alpha: 1.0, ground: true, spin: 12.0, ..BASE },
    Preset { name: "ripples", n: 1, speed: (0.0, 0.0), up: 0.0, spread: 0.0, g: 0.0, drag: 0.0, life: (0.5, 0.6),
             size: (0.2, 0.2), grow: 6.0, trail: (0, 0.0), shape: Shape::Ring, cols: &[[246.0, 238.0, 216.0]],
             alpha: 0.8, ..BASE },
    Preset { name: "rosin", n: 8, speed: (0.3, 0.9), up: 0.5, spread: 120.0, g: -0.1, drag: 2.0, life: (0.6, 1.2),
             size: (0.012, 0.02), trail: (6, 0.03), shape: Shape::Dot,
             cols: &[[250.0, 240.0, 210.0], [230.0, 210.0, 170.0]], alpha: 0.8, wobble: (0.1, 2.0), ..BASE },
];

/// `PRESETS[name]`, or `None` for a name the table does not have.
pub fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// The keyword arguments of `emit`. `Emit::default()` is Python's defaults; name the ones that differ:
/// `Emit { n: Some(30), spread: Some(180.0), ..Emit::default() }`.
#[derive(Clone, Debug, PartialEq)]
pub struct Emit {
    /// How many particles (`None`: the preset's count). Zero or less emits nothing.
    pub n: Option<i64>,
    /// The direction they are sprayed around (it need not be a unit vector). Python's default is straight up.
    pub dir: V3,
    /// A velocity added to every particle (the attacker's motion).
    pub vel: V3,
    /// Colours instead of the preset's. One colour (`col=(r, g, b)`) is a list of one.
    pub col: Option<Vec<V3>>,
    /// Scales the speed and the size.
    pub scale: f64,
    /// `(lo, hi)` instead of the preset's speed.
    pub speed: Option<(f64, f64)>,
    /// Degrees instead of the preset's spread.
    pub spread: Option<f64>,
    /// `(lo, hi)` instead of the preset's life.
    pub life: Option<(f64, f64)>,
}

impl Default for Emit {
    fn default() -> Emit {
        Emit {
            n: None,
            dir: [0.0, 1.0, 0.0],
            vel: [0.0; 3],
            col: None,
            scale: 1.0,
            speed: None,
            spread: None,
            life: None,
        }
    }
}

/// One particle: a row of Python's columns, under their names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// When it was emitted.
    pub t0: f64,
    /// Where.
    pub p: V3,
    /// Its first velocity.
    pub v: V3,
    /// How long it lives.
    pub life: f64,
    /// Its size in world units at birth.
    pub size: f64,
    /// Its growth per second.
    pub grow: f64,
    /// Its colour.
    pub col: V3,
    /// Gravity.
    pub g: f64,
    /// Drag (at least 1e-3).
    pub drag: f64,
    /// The wobble's amplitude.
    pub wa: f64,
    /// The wobble's frequency.
    pub wf: f64,
    /// Its phase.
    pub ph: f64,
    /// The twinkle in Hz.
    pub tw: f64,
    /// The head's shape.
    pub shape: Shape,
    /// The opacity.
    pub alpha: f64,
    /// Whether it settles on the floor.
    pub ground: bool,
    /// Its spin in rad/s.
    pub spin: f64,
    /// Whether it has a halo.
    pub glow: bool,
    /// The trail's samples.
    pub tn: u32,
    /// The seconds between them.
    pub tdt: f64,
}

/// The `light` of [`Particles::draw`]: for each world point an RGB gain (Python: `P -> (N, 3)`).
pub type Light<'a> = &'a dyn Fn(&[V3]) -> Vec<V3>;

/// `math.radians(x)`.
fn radians(x: f64) -> f64 {
    x * (dmath::PI / 180.0)
}

impl Particle {
    /// `_pos`: where the particle is at age `u`: drag and gravity, the wobble, and the floor it settles on.
    pub fn pos(&self, u: f64) -> V3 {
        let k = self.drag;
        // vinf = [0, -g, 0] / k;  e = (1 - exp(-k u)) / k;  p = p0 + vinf u + (v - vinf) e
        let vinf = [0.0 / k, -self.g / k, 0.0 / k];
        let e = (1.0 - dmath::exp(-k * u)) / k;
        let mut p = [0.0; 3];
        for c in 0..3 {
            p[c] = self.p[c] + vinf[c] * u + (self.v[c] - vinf[c]) * e;
        }
        if self.wa > 0.0 {
            let w = 2.0 * dmath::PI * self.wf * u + self.ph;
            let amp = self.wa * np::minimum(1.0, u * 3.0);
            p[0] += dmath::sin(w) * amp;
            p[1] += 0.5 * dmath::sin(1.7 * w + 1.0) * amp;
            p[2] += dmath::cos(1.3 * w) * amp;
        }
        if self.ground {
            p[1] = np::maximum(p[1], 0.01);
        }
        p
    }
}

/// `Particles`.
#[derive(Clone, Debug)]
pub struct Particles {
    rng: Generator,
    /// Sorted by `t0`, those of one time in the order emitted (Python's stable `argsort`).
    items: Vec<Particle>,
    max_alive: usize,
    /// The longest life.
    lmax: f64,
}

impl Particles {
    /// `Particles(seed, max_alive)`. The Python defaults are 0 and 600.
    pub fn new(seed: u64, max_alive: usize) -> Particles {
        Particles {
            rng: Generator::new(seed),
            items: Vec::new(),
            max_alive,
            lmax: 0.0,
        }
    }

    /// `emit(t0, pos, kind, **opts)`: particles of the preset `kind` at the world point `pos`, sprayed around
    /// `opts.dir` (plus the preset's upward bias) and carried by `opts.vel`.
    ///
    /// Per particle the generator gives, in this order: the spray's angle from the direction, its turn around it,
    /// the upward bias, three normal draws for the position, the speed, the life, the size, the colour (no draw
    /// when there is one colour), the wobble's frequency, the phase and the spin.
    ///
    /// # Panics
    /// Where Python raises: a `kind` the table does not have, or an empty colour list.
    pub fn emit(&mut self, t0: f64, pos: V3, kind: &str, opts: &Emit) {
        let preset = preset(kind).unwrap_or_else(|| panic!("emit: no preset named {kind:?}"));
        let n = opts.n.unwrap_or(preset.n);
        if n <= 0 {
            return;
        }
        let d0 = v3::div(opts.dir, v3::norm(opts.dir) + 1e-9);
        let sp = radians(opts.spread.unwrap_or(preset.spread));
        let (lo, hi) = opts.speed.unwrap_or(preset.speed);
        let cols: &[V3] = opts.col.as_deref().unwrap_or(preset.cols);
        let (life_lo, life_hi) = opts.life.unwrap_or(preset.life);
        for _ in 0..n {
            // a random direction within sp of d0 (uniform on the cap)
            let cz = dmath::cos(sp) + (1.0 - dmath::cos(sp)) * self.rng.random();
            let phi = self.rng.uniform(0.0, 2.0 * dmath::PI);
            let a = if d0[0].abs() < 0.9 {
                [1.0, 0.0, 0.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            let e1 = v3::cross(d0, a);
            let e1 = v3::div(e1, v3::norm(e1));
            let e2 = v3::cross(d0, e1);
            let sz = py_max(0.0, 1.0 - cz * cz).sqrt();
            let side = v3::add(v3::scale(e1, dmath::cos(phi)), v3::scale(e2, dmath::sin(phi)));
            let dv = v3::add(v3::scale(d0, cz), v3::scale(side, sz));
            let dv = v3::add(dv, v3::scale([0.0, preset.up, 0.0], self.rng.uniform(0.5, 1.0)));
            let dv = v3::div(dv, v3::norm(dv) + 1e-9);
            let mut jitter = [0.0; 3];
            self.rng.fill_normal(0.0, 0.03, &mut jitter);
            let p = v3::add(pos, jitter);
            let v = v3::add(v3::scale(v3::scale(dv, self.rng.uniform(lo, hi)), opts.scale), opts.vel);
            let life = self.rng.uniform(life_lo, life_hi);
            let size = self.rng.uniform(preset.size.0, preset.size.1) * opts.scale;
            assert!(!cols.is_empty(), "emit: no colours");
            let col = cols[self.rng.integers(cols.len() as u64) as usize];
            let wf = preset.wobble.1 * self.rng.uniform(0.7, 1.3);
            let ph = self.rng.uniform(0.0, 2.0 * dmath::PI);
            let spin = preset.spin * self.rng.uniform(-1.0, 1.0);
            let particle = Particle {
                t0,
                p,
                v,
                life,
                size,
                grow: preset.grow,
                col,
                g: preset.g,
                drag: py_max(1e-3, preset.drag),
                wa: preset.wobble.0,
                wf,
                ph,
                tw: preset.twinkle,
                shape: preset.shape,
                alpha: preset.alpha,
                ground: preset.ground,
                spin,
                glow: preset.glow,
                tn: preset.trail.0,
                tdt: preset.trail.1,
            };
            let at = self.items.partition_point(|q| q.t0 <= t0);
            self.items.insert(at, particle);
            // float(self._arr['life'].max())
            self.lmax = if self.items.len() == 1 {
                life
            } else {
                np::maximum(self.lmax, life)
            };
        }
    }

    /// The particles, as `_arrays()` holds them: sorted by `t0`, those of one time in the order emitted.
    pub fn particles(&self) -> &[Particle] {
        &self.items
    }

    /// `alive(t)[1]`: the indices (into [`Particles::particles`]) of the particles alive at `t`, the last
    /// `max_alive` of them.
    pub fn alive(&self, t: f64) -> Vec<usize> {
        if self.items.is_empty() {
            return Vec::new();
        }
        // np.searchsorted(t0, t - lmax - 1e-6) and np.searchsorted(t0, t, 'right')
        let lo = t - self.lmax - 1e-6;
        let i0 = self.items.partition_point(|q| q.t0 < lo);
        let i1 = self.items.partition_point(|q| q.t0 <= t);
        let idx: Vec<usize> = (i0..i1.max(i0))
            .filter(|&i| {
                let u = t - self.items[i].t0;
                u >= 0.0 && u < self.items[i].life
            })
            .collect();
        // idx[-self.max_alive:], and Python's -0 is 0: a limit of 0 keeps them all
        let skip = if self.max_alive == 0 {
            0
        } else {
            idx.len().saturating_sub(self.max_alive)
        };
        idx[skip..].to_vec()
    }

    /// `draw(img, cam, t, light)`: paints the particles alive at `t`, trails first, then heads. `light` gives an
    /// RGB gain for each world point (`lambda P: 1 + 0.6 * FL.tint(P, t)` in the ring), so attack lights colour
    /// the particles too.
    ///
    /// Returns the new picture, `float32` holding whole numbers (the drawing happens on bytes), or `None` when
    /// no particle is alive in front of the camera: Python then returns the picture it was given, unrounded.
    pub fn draw<T: Px>(&self, img: &Img<T>, cam: &Cam, t: f64, light: Option<Light<'_>>) -> Option<Img<f32>> {
        struct Live {
            i: usize,
            u: f64,
            fade: f64,
            p0: V3,
            z: f64,
        }
        let two_pi = 2.0 * dmath::PI;
        let mut live = Vec::new();
        for i in self.alive(t) {
            let q = &self.items[i];
            let u = t - q.t0;
            let f = u / q.life;
            let mut fade = np::clip(np::minimum(f / 0.12, (1.0 - f) / 0.35), 0.0, 1.0);
            if q.tw > 0.0 {
                fade *= 0.55 + 0.45 * dmath::sin(two_pi * q.tw * u + q.ph * 3.0);
            }
            let p0 = q.pos(u);
            let z = cam.to_cam(&[p0])[0][2];
            if z > cam.near {
                live.push(Live { i, u, fade, p0, z });
            }
        }
        if live.is_empty() {
            return None;
        }
        let p0: Vec<V3> = live.iter().map(|l| l.p0).collect();
        let s0 = cam.wp(&p0);
        let gains = light.map(|f| f(&p0));
        if let Some(g) = &gains {
            assert_eq!(
                g.len(),
                live.len(),
                "draw: the light gave {} gains for {} points",
                g.len(),
                live.len()
            );
        }
        let mut base = bytes_of(img);
        let col_of = |j: usize| -> V3 {
            let col = self.items[live[j].i].col;
            match &gains {
                Some(g) => [0, 1, 2].map(|c| np::clip(col[c] * g[j][c], 0.0, 255.0)),
                None => col,
            }
        };
        let rpx: Vec<f64> = live
            .iter()
            .map(|l| {
                let q = &self.items[l.i];
                let size = q.size * (1.0 + q.grow * l.u);
                np::maximum(0.5, size * cam.f / l.z)
            })
            .collect();
        let alpha: Vec<f64> = live.iter().map(|l| self.items[l.i].alpha * l.fade).collect();
        // trails
        for (j, l) in live.iter().enumerate() {
            let q = &self.items[l.i];
            if q.tn == 0 {
                continue;
            }
            let k = q.tn as usize;
            let world: Vec<V3> = (0..=k)
                .map(|s| q.pos(np::maximum(0.0, l.u - q.tdt * s as f64)))
                .collect();
            let pts = cam.wp(&world);
            let col = col_of(j);
            for s in 0..k {
                let a = alpha[j] * (1.0 - (s + 1) as f64 / (k + 1) as f64) * 0.7;
                let wide = round_half_even(py_min(rpx[j], 2.0) * (1.0 - s as f64 / (k + 1) as f64));
                let wd = (wide as i64).max(1) as u32;
                blend::line(&mut base, &[pts[s], pts[s + 1]], ink(col, byte((255.0 * a) as i64)), wd);
            }
        }
        // heads
        let (w, h) = (img.w as f64, img.h as f64);
        for (j, l) in live.iter().enumerate() {
            let (x, y) = s0[j];
            if !(-8.0 < x && x < w + 8.0 && -8.0 < y && y < h + 8.0) {
                continue;
            }
            let q = &self.items[l.i];
            let col = col_of(j);
            let a = (255.0 * alpha[j]) as i64;
            let part = |k: f64| byte((a as f64 * k) as i64);
            let c = ink(col, byte(a));
            let r = rpx[j];
            if q.glow && a > 40 {
                let gr = py_max(1.5, r * 2.6);
                blend::ellipse(
                    &mut base,
                    (x - gr, y - gr, x + gr, y + gr),
                    Some(ink(col, part(0.25))),
                    None,
                    1,
                );
            }
            match q.shape {
                Shape::Dot => {
                    if r < 1.0 {
                        blend::point(&mut base, &[(x, y)], c);
                    } else {
                        blend::ellipse(&mut base, (x - r, y - r, x + r, y + r), Some(c), None, 1);
                    }
                }
                Shape::Blob => {
                    let body = ink(col, part(0.8));
                    blend::ellipse(
                        &mut base,
                        (x - r, y - r * 0.85, x + r, y + r * 0.85),
                        Some(body),
                        None,
                        1,
                    );
                    if r > 2.0 {
                        // tuple(min(255, v + 25) for v in c)
                        let up = |v: f64| byte(((v as i64).saturating_add(25)).min(255));
                        let shine = [up(col[0]), up(col[1]), up(col[2]), part(0.5)];
                        blend::ellipse(
                            &mut base,
                            (x - r * 0.5, y - r * 0.6, x + r * 0.3, y),
                            Some(shine),
                            None,
                            1,
                        );
                    }
                }
                Shape::Shard => {
                    let ang = q.spin * l.u + q.ph;
                    let pts = [0.0, 1.0, 2.0].map(|s: f64| {
                        (
                            x + r * 1.4 * dmath::cos(ang + s * 2.3),
                            y + r * 0.9 * dmath::sin(ang + s * 2.3),
                        )
                    });
                    blend::polygon(&mut base, &pts, c);
                }
                Shape::Square => blend::rectangle(&mut base, (x - r, y - r * 0.6, x + r, y + r * 0.6), Some(c), None),
                Shape::Streak => blend::point(&mut base, &[(x, y)], [255, 255, 240, byte(a)]),
                Shape::Ring => blend::ellipse(&mut base, (x - r, y - r * 0.55, x + r, y + r * 0.55), None, Some(c), 1),
            }
        }
        Some(floats_of(&base))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_pythons() {
        let names: Vec<&str> = PRESETS.iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "dust",
                "motes",
                "steam",
                "smoke",
                "shreds",
                "puffs",
                "shards",
                "sparks",
                "glitch",
                "streaks",
                "scales",
                "glass",
                "foam",
                "chips",
                "embers",
                "brass",
                "smear",
                "keyshards",
                "ripples",
                "rosin"
            ]
        );
        assert_eq!(preset("brass").unwrap().trail, (4, 0.02));
        assert_eq!(preset("dust").unwrap().wobble, (0.08, 1.3));
        assert_eq!(preset("glass").unwrap().wobble, (0.0, 1.0));
        assert!(preset("sand").is_none());
        assert_eq!(Shape::Ring.index(), 5);
        assert!(PRESETS.iter().all(|p| !p.cols.is_empty() && p.n > 0));
    }

    #[test]
    fn emits_are_kept_in_time_order_and_live_for_their_life() {
        let mut pa = Particles::new(1, 600);
        assert!(pa.alive(0.5).is_empty());
        pa.emit(1.0, [0.0, 0.5, 0.0], "brass", &Emit::default());
        pa.emit(
            0.5,
            [0.0, 0.5, 0.0],
            "dust",
            &Emit {
                n: Some(3),
                ..Emit::default()
            },
        );
        pa.emit(
            0.7,
            [0.0, 0.5, 0.0],
            "dust",
            &Emit {
                n: Some(0),
                ..Emit::default()
            },
        );
        pa.emit(
            1.0,
            [0.0, 0.5, 0.0],
            "glass",
            &Emit {
                n: Some(2),
                ..Emit::default()
            },
        );
        let all = pa.particles();
        assert_eq!(all.len(), 15);
        assert!(all[..3].iter().all(|q| q.t0 == 0.5 && q.shape == Shape::Blob));
        assert!(all[3..13].iter().all(|q| q.shape == Shape::Streak));
        assert!(all[13..].iter().all(|q| q.shape == Shape::Dot));
        assert_eq!(pa.alive(0.49), Vec::<usize>::new());
        assert_eq!(pa.alive(0.5), [0, 1, 2]);
        assert_eq!(pa.alive(1.0).len(), 15);
        // brass lives 0.3 to 0.6 s, glass 0.6 to 1.1 s, dust 0.9 to 1.6 s
        assert!(pa.alive(1.61).iter().all(|&i| !(3..13).contains(&i)));
        assert!(pa.alive(1.59).contains(&13));
        assert!(pa.alive(2.2).is_empty());
        let few = Particles {
            max_alive: 4,
            ..pa.clone()
        };
        assert_eq!(few.alive(1.0), [11, 12, 13, 14]);
        let all_of_them = Particles { max_alive: 0, ..pa };
        assert_eq!(all_of_them.alive(1.0).len(), 15);
    }

    #[test]
    fn a_particle_starts_where_it_was_put_and_settles_on_the_floor() {
        let mut pa = Particles::new(5, 600);
        pa.emit(0.0, [1.0, 0.3, -2.0], "shards", &Emit::default());
        for q in pa.particles() {
            // no wobble: at age 0 the particle is at its start
            assert_eq!(q.pos(0.0), q.p);
            assert_eq!(q.pos(30.0)[1], 0.01);
        }
    }

    #[test]
    fn nothing_alive_gives_the_picture_back() {
        let img: Img<f32> = Img::filled(40, 30, &[10.5, 20.25, 300.0]);
        let cam = Cam::new([0.0, 0.5, 0.0], 6.0, 25.0, 35.0, 50.0, Some(15.0), 0.0, 0.1);
        let mut pa = Particles::new(2, 600);
        assert!(pa.draw(&img, &cam, 0.1, None).is_none());
        pa.emit(0.0, [0.0, 0.5, 0.0], "glitch", &Emit::default());
        assert!(pa.draw(&img, &cam, 5.0, None).is_none());
        // drawn or not, what comes back is the picture as bytes
        let out = pa.draw(&img, &cam, 0.1, None).expect("particles are alive");
        assert_eq!(out.px(0, 0), [10.0, 20.0, 255.0]);
    }
}
