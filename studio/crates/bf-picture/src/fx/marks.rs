//! `Marks`: marks that fade on a floor (scorch, claw, crack, water, smoke ring, dust, glass), evaluated per
//! pixel in the caller's floor coordinates (the ring's frame).

use crate::camera::V3;
use crate::np;
use bf_core::dmath;
use bf_core::py::fmod_py;
use bf_core::rng::Generator;

use super::Timeline;
use super::timing::smooth;

/// The kinds of mark (`MARK_KINDS`, in Python's order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkKind {
    /// A dark, jagged blot with a warm rim (in the mark's colour).
    Scorch,
    /// `n` parallel gouges along the mark's angle, dark with a light lip.
    Claw,
    /// `n` jagged rays from the centre.
    Crack,
    /// A wet, darker blot with a pale rim.
    Water,
    /// A soft dark ring.
    SmokeRing,
    /// A pale powdery blot (in the mark's colour).
    Dust,
    /// Sparkly specks (in the mark's colour).
    Glass,
}

impl MarkKind {
    /// Python's names, in the order of `MARK_KINDS`.
    pub const NAMES: [&'static str; 7] = ["scorch", "claw", "crack", "water", "smoke_ring", "dust", "glass"];

    /// The kind Python names `name`.
    pub fn from_name(name: &str) -> Option<MarkKind> {
        use MarkKind::*;
        let kinds = [Scorch, Claw, Crack, Water, SmokeRing, Dust, Glass];
        Self::NAMES.iter().position(|&n| n == name).map(|i| kinds[i])
    }

    /// Whether the light part of the mark is drawn in the mark's own colour (else toward white).
    fn tinted(self) -> bool {
        matches!(self, MarkKind::Scorch | MarkKind::Dust | MarkKind::Glass)
    }
}

/// One mark: the arguments of `Marks.add` after the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    /// What it is.
    pub kind: MarkKind,
    /// Its place on the floor.
    pub u: f64,
    /// Its place on the floor.
    pub v: f64,
    /// Its size in floor units (it reaches 1.25 times as far). Python's default is 0.5.
    pub size: f64,
    /// Its turn in radians. Python's default is 0.
    pub angle: f64,
    /// Its colour. Python's default is `(40, 30, 34)`.
    pub col: V3,
    /// How long it lasts in seconds. Python's default is 5.
    pub life: f64,
    /// Gouges of a claw (at least 1) or rays of a crack (at least 3). Python's default is 3.
    pub n: i64,
    /// The seed of its shape. Python's default is 0.
    pub seed: u64,
    /// Its strength. Python's default is 1.
    pub strength: f64,
}

impl Mark {
    /// A mark with Python's defaults for everything but its kind and place.
    pub fn new(kind: MarkKind, u: f64, v: f64) -> Mark {
        Mark {
            kind,
            u,
            v,
            size: 0.5,
            angle: 0.0,
            col: [40.0, 30.0, 34.0],
            life: 5.0,
            n: 3,
            seed: 0,
            strength: 1.0,
        }
    }
}

/// A mark alive at some time: what `Marks.live` lists.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveMark<'a> {
    /// The mark.
    pub mark: &'a Mark,
    /// The number of the `add` call that made it (0, 1, ...).
    pub which: usize,
    /// Its fade times its strength.
    pub k: f64,
    /// Its age in seconds.
    pub age: f64,
}

/// `Marks`: a mark fades in over 0.06 s, holds, and fades out over the last 40 % of its life.
#[derive(Clone, Debug)]
pub struct Marks {
    tl: Timeline<Mark>,
    max_live: usize,
}

impl Marks {
    /// `Marks(span, max_live)`. The Python defaults are 12.0 and 40.
    pub fn new(span: f64, max_live: usize) -> Marks {
        Marks {
            tl: Timeline::new(span),
            max_live,
        }
    }

    /// `add(t0, kind, u, v, ...)`.
    pub fn add(&mut self, t0: f64, mark: Mark) {
        self.tl.add(t0, mark);
    }

    /// The number of marks added.
    pub fn len(&self) -> usize {
        self.tl.len()
    }

    /// Whether no mark was added.
    pub fn is_empty(&self) -> bool {
        self.tl.len() == 0
    }

    /// `live(t)`: the marks to draw at `t`, in time order, the last `max_live` of them.
    pub fn live(&self, t: f64) -> Vec<LiveMark<'_>> {
        let mut out = Vec::new();
        for (t0, which, m) in self.tl.alive(t) {
            let age = t - t0;
            if age < 0.0 || age >= m.life {
                continue;
            }
            let fade = smooth(age / 0.06) * (1.0 - smooth((age - 0.6 * m.life) / (0.4 * m.life)));
            if fade > 0.01 {
                out.push(LiveMark {
                    mark: m,
                    which,
                    k: fade * m.strength,
                    age,
                });
            }
        }
        // out[-self.max_live:], and Python's -0 is 0: a limit of 0 keeps them all
        let skip = if self.max_live == 0 {
            0
        } else {
            out.len().saturating_sub(self.max_live)
        };
        out.split_off(skip)
    }

    /// `apply(alb, U, V, t, dither)`: mixes the live marks into the albedo `alb` of the floor points
    /// `(u[i], v[i])`. `dither` (0..1 per point: the Bayer value of its pixel) cuts the strengths into
    /// pixel-art steps of a third. Returns whether a mark was alive (Python then returns a new array, else the
    /// one it was given).
    ///
    /// # Panics
    /// If the slices differ in length.
    pub fn apply(&self, alb: &mut [V3], u: &[f64], v: &[f64], t: f64, dither: Option<&[f64]>) -> bool {
        let n = alb.len();
        assert!(
            u.len() == n && v.len() == n,
            "apply: {n} colours, {} and {} coordinates",
            u.len(),
            v.len()
        );
        if let Some(d) = dither {
            assert_eq!(d.len(), n, "apply: {n} colours, {} dither values", d.len());
        }
        let live = self.live(t);
        if live.is_empty() {
            return false;
        }
        for l in &live {
            let ext = l.mark.size * 1.25;
            let sel: Vec<usize> = (0..n)
                .filter(|&i| (u[i] - l.mark.u).abs() < ext && (v[i] - l.mark.v).abs() < ext)
                .collect();
            if sel.is_empty() {
                continue;
            }
            let (dark, lite) = strengths(l.mark, &sel, u, v);
            for (j, &i) in sel.iter().enumerate() {
                let (mut dark, mut lite) = (dark[j], lite[j]);
                if let Some(dq) = dither {
                    dark = (dark * 3.0 + dq[i] * 0.9).floor() / 3.0;
                    lite = (lite * 3.0 + dq[i] * 0.9).floor() / 3.0;
                }
                let k = l.k;
                let a = &mut alb[i];
                for (v, col) in a.iter_mut().zip(l.mark.col) {
                    *v = if l.mark.kind.tinted() {
                        *v * (1.0 - dark * k) + (col - *v) * (lite * k)
                    } else {
                        *v * (1.0 - dark * k) + (255.0 - *v) * (lite * k * 0.6)
                    };
                }
                if l.mark.kind == MarkKind::Water {
                    let wet = if dark > 0.0 { 1.0 } else { 0.0 };
                    for (c, tint) in [60.0, 90.0, 140.0].into_iter().enumerate() {
                        a[c] = a[c] * (1.0 - 0.2 * k * wet) + tint * 0.2 * k * wet;
                    }
                }
            }
        }
        true
    }
}

/// How dark and how light the mark makes each selected point, before the dither: `(dark, lite)`.
fn strengths(m: &Mark, sel: &[usize], u: &[f64], v: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let s = m.size;
    let (ca, sa) = (dmath::cos(m.angle), dmath::sin(m.angle));
    let seed = m.seed as f64;
    let two_pi = 2.0 * dmath::PI;
    // the point in the mark's own frame: x along its angle, y across, r from its centre, th around it
    let local: Vec<(f64, f64, f64, f64)> = sel
        .iter()
        .map(|&i| {
            let (du, dv) = (u[i] - m.u, v[i] - m.v);
            let x = (du * ca + dv * sa) / s;
            let y = (-du * sa + dv * ca) / s;
            let r = (x * x + y * y).sqrt() + 1e-6;
            (x, y, r, dmath::atan2(y, x))
        })
        .collect();
    let unit = |v: f64| np::clip(v, 0.0, 1.0);
    let mut dark = vec![0.0; sel.len()];
    let mut lite = vec![0.0; sel.len()];
    // np.random.default_rng(m['seed']): made for every kind, drawn from by the crack alone
    let mut rng = Generator::new(m.seed);
    match m.kind {
        MarkKind::Scorch => {
            for (j, &(_, _, r, th)) in local.iter().enumerate() {
                let rr =
                    r * (1.0 + 0.2 * dmath::sin(7.0 * th + seed) + 0.1 * dmath::sin(13.0 * th + (2 * m.seed) as f64));
                dark[j] = dmath::pow(unit(1.0 - rr), 0.8) * 0.7;
                lite[j] = unit(1.0 - (rr - 1.0).abs() / 0.18) * 0.35;
            }
        }
        MarkKind::Claw => {
            let n = m.n.max(1);
            let sp = if n > 1 { 0.9 / (n - 1).max(1) as f64 } else { 0.0 };
            for i in 0..n {
                let yi = if n > 1 { -0.45 + i as f64 * sp } else { 0.0 };
                for (j, &(x, y, _, _)) in local.iter().enumerate() {
                    let taper = unit(1.0 - x.abs()).sqrt();
                    let dd = (y - yi - 0.06 * dmath::sin(x * 3.0 + i as f64)).abs();
                    dark[j] = np::maximum(dark[j], unit(1.0 - dd / 0.07) * taper * 0.75);
                    lite[j] = np::maximum(lite[j], unit(1.0 - (dd - 0.1).abs() / 0.035) * taper * 0.4);
                }
            }
        }
        MarkKind::Crack => {
            let n = m.n.max(3);
            for i in 0..n {
                let a0 = rng.uniform(0.0, two_pi);
                let reach = rng.uniform(0.6, 1.0);
                for (j, &(_, _, r, th)) in local.iter().enumerate() {
                    let ai = a0 + 0.22 * dmath::sin(r * 11.0 + i as f64 * 3.1);
                    let dth = (fmod_py(th - ai + dmath::PI, two_pi) - dmath::PI).abs();
                    let w = 0.03 / np::maximum(r, 0.05) * (1.2 - r);
                    let inside = if r < reach { 1.0 } else { 0.0 };
                    dark[j] = np::maximum(dark[j], unit(1.0 - dth / np::maximum(w, 1e-3)) * inside * 0.85);
                }
            }
            for (j, &(_, _, r, _)) in local.iter().enumerate() {
                dark[j] = np::maximum(dark[j], unit(1.0 - r / 0.15) * 0.6);
            }
        }
        MarkKind::Water => {
            for (j, &(_, _, r, th)) in local.iter().enumerate() {
                let rr = r * (1.0 + 0.12 * dmath::sin(5.0 * th + seed));
                dark[j] = dmath::pow(unit(1.0 - rr), 0.4) * 0.35;
                lite[j] = unit(1.0 - (rr - 0.92).abs() / 0.08) * 0.45;
            }
        }
        MarkKind::SmokeRing => {
            for (j, &(_, _, r, _)) in local.iter().enumerate() {
                dark[j] = dmath::pow(unit(1.0 - (r - 0.75).abs() / 0.2), 1.5) * 0.5;
            }
        }
        MarkKind::Dust => {
            for (j, &(_, _, r, th)) in local.iter().enumerate() {
                let rr = r * (1.0 + 0.25 * dmath::sin(6.0 * th + seed));
                lite[j] = dmath::pow(unit(1.0 - rr), 0.7) * 0.5;
            }
        }
        MarkKind::Glass => {
            for (j, &(x, y, r, _)) in local.iter().enumerate() {
                let h = dmath::sin(x * 91.7 + y * 47.3 + seed) * 43758.5453;
                let h = h - h.floor();
                lite[j] = if h > 0.965 && r < 1.0 { 0.9 } else { 0.0 };
            }
        }
    }
    (dark, lite)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_pythons() {
        assert_eq!(MarkKind::from_name("smoke_ring"), Some(MarkKind::SmokeRing));
        assert_eq!(MarkKind::from_name("glass"), Some(MarkKind::Glass));
        assert_eq!(MarkKind::from_name("scorch"), Some(MarkKind::Scorch));
        assert_eq!(MarkKind::from_name("burn"), None);
    }

    #[test]
    fn a_mark_fades_in_holds_and_fades_out() {
        let mut mk = Marks::new(12.0, 40);
        assert!(mk.is_empty());
        mk.add(
            1.0,
            Mark {
                life: 2.0,
                strength: 0.5,
                ..Mark::new(MarkKind::Scorch, 0.0, 0.0)
            },
        );
        assert_eq!(mk.len(), 1);
        assert!(mk.live(0.99).is_empty());
        assert!(mk.live(1.0).is_empty());
        let l = mk.live(1.03);
        assert_eq!(l[0].which, 0);
        assert!((l[0].k - 0.25).abs() < 1e-12 && (l[0].age - 0.03).abs() < 1e-12);
        assert_eq!(mk.live(1.5)[0].k, 0.5);
        // half-way through the fade-out, which starts at 60 % of the life
        assert!((mk.live(2.6)[0].k - 0.25).abs() < 1e-12);
        assert!(mk.live(3.0).is_empty());
    }

    #[test]
    fn only_the_last_max_live_marks_are_drawn() {
        let mut mk = Marks::new(12.0, 2);
        for i in 0..4 {
            mk.add(f64::from(i) * 0.1, Mark::new(MarkKind::Dust, 0.0, 0.0));
        }
        let which: Vec<usize> = mk.live(1.0).iter().map(|l| l.which).collect();
        assert_eq!(which, [2, 3]);
        assert_eq!(Marks { max_live: 0, ..mk }.live(1.0).len(), 4);
    }

    #[test]
    fn a_mark_darkens_its_own_patch_only() {
        let mut mk = Marks::new(12.0, 40);
        mk.add(
            0.0,
            Mark {
                size: 1.0,
                ..Mark::new(MarkKind::SmokeRing, 0.0, 0.0)
            },
        );
        let u = [0.75, 0.0, 3.0];
        let v = [0.0, 0.0, 0.0];
        let mut alb = [[200.0; 3]; 3];
        assert!(!mk.apply(&mut alb, &u, &v, -1.0, None));
        assert_eq!(alb, [[200.0; 3]; 3]);
        assert!(mk.apply(&mut alb, &u, &v, 1.0, None));
        // on the ring: half as bright (to within the 1e-6 added to the radius); at the centre and far away: untouched
        assert!((alb[0][0] - 100.0).abs() < 1e-2);
        assert_eq!(alb[1], [200.0; 3]);
        assert_eq!(alb[2], [200.0; 3]);
    }
}
