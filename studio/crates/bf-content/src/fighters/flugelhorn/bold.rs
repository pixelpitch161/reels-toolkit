//! FLUGELHORN with the bold effects (`picture.effects = "bold"`; the owner's picks 32a to 32d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **WINGBEAT** (32a): two big outlined wings rise out of the ball, clap shut in front of it and stay shut for a
//!   moment; a gust front of white arcs runs out from the clap (further when it landed, with speed lines behind
//!   the shoved enemy) and loose feathers flutter down.
//! - **FLANKING CALL** (32b): dots on the floor show the swoop round to the enemy's side and a mark where it will
//!   clap; the ball swoops with its wings spread, claps from there, and the enemy flies off toward the rope with
//!   speed lines behind it.
//! - **FEATHER SHOWER** (32c): the wings flick forward as the volley starts and shiver while it lasts; the shots
//!   are big fluttering feathers with a quill; one that touches stings with a little green star and splinters.
//! - **TAKE WING** (32d): the wings spread across the whole ring over a ring of light that shows the reach, clap
//!   shut once with gust arcs and puffs of air running out all round and a burst of feathers; then the ball
//!   glides with its wings swept back (the ring's afterimages follow it by themselves).

use bf_core::dmath;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, WHITE};
use bf_picture::camera::{Cam, V3};
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::R_BALL as R;

use super::{FEATHER, FlugelLook, world};
use bf_core::roster::FighterCfg;

/// The shaded feathers of a wing, between the white ones.
const SHADE: V3 = [214.0, 220.0, 226.0];
/// Seconds the wings take to rise out of the ball before a clap (half of it while the ball glides).
const RISE: f64 = 0.36;
/// The last of those seconds, in which they sweep shut.
const SWEEP: f64 = 0.12;
/// Seconds the wings stay shut after a clap.
const HOLD: f64 = 0.1;
/// Seconds they take to fold away after that.
const FOLD: f64 = 0.16;
/// Seconds a gust front runs.
const GUST: f64 = 0.34;
/// Seconds a shed feather flutters.
const SHED: f64 = 0.75;
/// Seconds before a FLANKING CALL its dots start to show, and after it they take to fade.
const WARN: f64 = 0.3;
/// Seconds an enemy blown away trails speed lines.
const BLOWN: f64 = 0.4;
/// Seconds the wings take to rise before a FEATHER SHOWER, and to flick forward as it starts.
const FLICK: f64 = 0.1;

/// One painter item of the bold look.
enum BItem {
    /// A wing: its feathers back to front, each with its colour.
    Wing(Vec<(Vec<V3>, V3)>),
    /// A FEATHER SHOWER feather: x, z, the angle it lies along, its height, how wide it shows (it flutters).
    Feather(f64, f64, f64, f64, f64),
}

/// How the wings are held at one moment.
#[derive(Clone, Copy)]
struct Pose {
    /// The direction they clap toward.
    aim: f64,
    /// How far each is opened off the aim, radians.
    phi: f64,
    /// Their length.
    span: f64,
    /// How far the tips are lifted over the roots.
    lift: f64,
    /// The feathers of each.
    n: usize,
    /// Drawn after the grade (TAKE WING, bright under the low-HP spotlight) instead of in the painter pass.
    crisp: bool,
}

/// A bold feather: a leaf with a pointed tip from `root` along angle `a`, rising from height `y0` to `y1`.
fn leaf(root: [f64; 2], a: f64, len: f64, wd: f64, y0: f64, y1: f64) -> Vec<V3> {
    let d = [dmath::cos(a), dmath::sin(a)];
    let n = [-d[1], d[0]];
    let at = |s: f64, w: f64| {
        world(
            [
                root[0] + d[0] * len * s + n[0] * wd * w,
                root[1] + d[1] * len * s + n[1] * wd * w,
            ],
            y0 + (y1 - y0) * s,
        )
    };
    vec![
        at(0.0, 0.0),
        at(0.22, 0.5),
        at(0.68, 0.46),
        at(0.92, 0.2),
        at(1.0, 0.0),
        at(0.92, -0.2),
        at(0.68, -0.46),
        at(0.22, -0.5),
    ]
}

/// One wing from the ball's side `s` (+1 left, -1 right of the aim): the long feathers from the trailing one to
/// the leading one, then the short ones over their roots in the fighter's colour.
fn wing(c: [f64; 2], s: f64, pose: Pose, col: V3) -> Vec<(Vec<V3>, V3)> {
    let y = R * 0.9;
    let root_a = pose.aim + s * dmath::PI * 0.5;
    let root = [
        c[0] + R * 0.75 * dmath::cos(root_a),
        c[1] + R * 0.75 * dmath::sin(root_a),
    ];
    let wd = 0.1 + 0.11 * pose.span;
    let step = 0.9 / pose.n as f64;
    let mut out = Vec::with_capacity(pose.n + 3);
    for j in (0..pose.n).rev() {
        let fa = pose.aim + s * (pose.phi + step * j as f64);
        let len = pose.span * (1.0 - 0.55 * j as f64 / pose.n as f64);
        let shade = if j == 0 {
            bold::pale(col)
        } else if j % 2 == 0 {
            SHADE
        } else {
            FEATHER
        };
        out.push((
            leaf(root, fa, len, wd, y, y + pose.lift * len / pose.span.max(1e-6)),
            shade,
        ));
    }
    for j in (0..3).rev() {
        let fa = pose.aim + s * (pose.phi + 0.12 + 0.3 * j as f64);
        let len = pose.span * (0.42 - 0.07 * j as f64);
        out.push((leaf(root, fa, len, wd * 1.1, y + 0.02, y + 0.02 + pose.lift * 0.4), col));
    }
    out
}

/// A feather on the finished picture, with the dark outline and its quill.
#[allow(clippy::too_many_arguments)]
fn px_feather(
    img: &mut Pic,
    cam: &Cam,
    q: [f64; 2],
    a: f64,
    len: f64,
    wd: f64,
    y: f64,
    col: V3,
    quill: V3,
    alpha: f64,
) {
    let pts = cam.wp(&leaf(q, a, len, wd, y, y));
    px::ink_poly(img, &pts, col, alpha);
    px::line(img, pts[0], pts[4], 1.0, quill, alpha);
}

/// A gust front on the finished picture: three arcs `half` radians either side of `dir` round `c`, running out from
/// `r0` to `r1`, `u` (0 to 1) through their life.
#[allow(clippy::too_many_arguments)]
fn gust(img: &mut Pic, cam: &Cam, c: [f64; 2], dir: f64, half: f64, r0: f64, r1: f64, u: f64, col: V3) {
    for n in 0..3 {
        let un = u * 1.25 - 0.14 * n as f64;
        if !(0.0..1.0).contains(&un) {
            continue;
        }
        let r = r0 + (r1 - r0) * bold::ease_out(un);
        let h = half * (1.0 - 0.18 * n as f64);
        let steps = ((h * r * 5.0).ceil() as usize).clamp(6, 40);
        let pts: Vec<(f64, f64)> = (0..=steps)
            .map(|i| {
                let a = dir - h + 2.0 * h * i as f64 / steps as f64;
                bold::at(cam, c[0] + r * dmath::cos(a), c[1] + r * dmath::sin(a), R * 0.6)
            })
            .collect();
        let shade = if n == 0 { WHITE } else { bold::pale(col) };
        px::ink_polyline(img, &pts, 3.2 - 1.6 * un, shade, 1.0 - un * un);
    }
}

/// Something that sheds feathers and sends a gust: a clap of any kind.
struct Burst {
    /// When.
    t0: f64,
    /// Where the ball was.
    c: [f64; 2],
    /// Where the gust goes, and how wide either side (pi: all round).
    dir: f64,
    half: f64,
    /// Where the gust starts and how far it runs.
    r0: f64,
    r1: f64,
    /// The feathers it sheds.
    feathers: u64,
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// FEATHER SHOWER's beats.
    volley_beats: f64,
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        Nums {
            volley_beats: cfg.raw["abilities"]["unlock2"]["beats"].as_f64().unwrap_or(2.0),
        }
    }
}

impl FlugelLook {
    /// The ball's heading at recording frame `i`, or `or` when it is hardly moving.
    fn heading(p: &PictureCtx, i: usize, k: usize, or: f64) -> f64 {
        let (a, b) = (p.run.p(i.saturating_sub(2), k), p.run.p(i, k));
        let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
        if dmath::hypot(dx, dz) < 1e-3 {
            or
        } else {
            dmath::atan2(dz, dx)
        }
    }

    /// The angle from the ball to its enemy at recording frame `i`.
    fn toward(p: &PictureCtx, i: usize, k: usize) -> f64 {
        let (a, b) = (p.run.p(i, k), p.run.p(i, 1 - k));
        dmath::atan2(b[1] - a[1], b[0] - a[0])
    }

    /// The landed FLANKING CALL clap that is shut or folding at `t`, or about to land within `ahead`: its time.
    fn flank_hit(p: &PictureCtx, k: usize, t: f64, ahead: f64) -> Option<f64> {
        bold::since(p, k, &["hit:flank"], t, HOLD + FOLD)
            .map(|(_, e)| e.t)
            .or_else(|| bold::until(p, k, &["hit:flank"], t, ahead).map(|(_, e)| e.t))
    }

    /// TAKE WING's glide is on at `t`.
    fn gliding(&self, f: &FrameCtx<'_>) -> bool {
        f.fs[self.k].haste > 0.0 && self.takes.iter().any(|tk| tk.t_clap <= f.t)
    }

    /// The wings of a clap at `dt` seconds from it: rising wide, sweeping shut, held, folding away.
    fn clap_pose(dt: f64, aim: f64, span: f64, wide: f64) -> Pose {
        let shut = 0.1;
        let (phi, grow, lift) = if dt < -SWEEP {
            let w = bold::ease_out((dt + RISE) / (RISE - SWEEP));
            (wide * (0.55 + 0.45 * w), 0.25 + 0.75 * w, 0.55 * w)
        } else if dt < 0.0 {
            let w = bold::ease_in((dt + SWEEP) / SWEEP);
            (wide + (shut - wide) * w, 1.0, 0.55 - 0.45 * w)
        } else if dt < HOLD {
            (shut, 1.0, 0.1)
        } else {
            let w = ((dt - HOLD) / FOLD).clamp(0.0, 1.0);
            (shut + 0.5 * w, 1.0 - 0.8 * w, 0.1)
        };
        Pose {
            aim,
            phi,
            span: span * grow,
            lift,
            n: 6,
            crisp: false,
        }
    }

    /// How the wings are held at this frame, if they are out.
    fn pose(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Option<Pose> {
        let (k, t) = (self.k, f.t);
        let st = &f.fs[k];
        // TAKE WING: spread across the ring, one sweep shut, held, folded
        if let Some(tk) = self.takes.iter().find(|tk| tk.t0 <= t && t < tk.t_clap + HOLD + FOLD) {
            let full = R + tk.radius;
            let dt = t - tk.t_clap;
            let (phi, span, lift) = if dt < -SWEEP {
                let u = bold::ease_out((t - tk.t0) / (tk.t_clap - SWEEP - tk.t0).max(1e-3));
                (2.1 - 0.55 * u, full * (0.3 + 0.7 * u), 0.7 * u)
            } else if dt < 0.0 {
                let w = bold::ease_in((dt + SWEEP) / SWEEP);
                (1.55 - 1.45 * w, full, 0.7 - 0.5 * w)
            } else if dt < HOLD {
                (0.1, full, 0.2)
            } else {
                let w = (dt - HOLD) / FOLD;
                (0.1 + 0.6 * w, full * (1.0 - 0.8 * w), 0.2)
            };
            return Some(Pose {
                aim: st.th,
                phi,
                span,
                lift,
                n: 9,
                crisp: true,
            });
        }
        // FLANKING CALL: spread while it swoops, clapped shut toward the enemy as it gets there
        if let Some(th) = Self::flank_hit(p, k, t, SWEEP) {
            let aim = Self::toward(p, p.sim_i(th), k);
            return Some(Self::clap_pose(t - th, aim, R + 1.3, 1.9));
        }
        let glide = |aim: f64| Pose {
            aim,
            phi: 2.15 + 0.22 * dmath::sin(t * 14.0),
            span: R + 1.0,
            lift: 0.35,
            n: 6,
            crisp: false,
        };
        if f.in_fight && self.flanks.iter().any(|fl| fl.t0 <= t && t < fl.t_end) {
            return Some(glide(Self::heading(p, f.i, k, st.th)));
        }
        // FEATHER SHOWER: up and back, a flick forward, shivering while the feathers leave
        let vol = self.volley(p);
        if let Some((u, _)) = bold::until(p, k, &["ability:arc"], t, 3.0 * FLICK) {
            return Some(Pose {
                aim: st.th,
                phi: 1.6 + 0.8 * bold::ease_out(u),
                span: (R + 1.1) * (0.4 + 0.6 * bold::ease_out(u)),
                lift: 0.6 * u,
                n: 6,
                crisp: false,
            });
        }
        if let Some((age, _)) = bold::since(p, k, &["ability:arc"], t, vol + FOLD) {
            let (phi, grow) = if age < FLICK {
                (2.4 - 1.7 * bold::ease_in(age / FLICK), 1.0)
            } else if age < vol {
                (0.75 + if f.f.is_multiple_of(2) { 0.14 } else { 0.0 }, 1.0)
            } else {
                (0.75, 1.0 - 0.8 * (age - vol) / FOLD)
            };
            return Some(Pose {
                aim: st.th,
                phi,
                span: (R + 1.1) * grow,
                lift: 0.3,
                n: 6,
                crisp: false,
            });
        }
        // WINGBEAT; a gliding ball flaps: its claps rise quicker and go straight back to the glide
        let gliding = self.gliding(f);
        let (rise, after) = if gliding {
            (RISE * 0.5, HOLD)
        } else {
            (RISE, HOLD + FOLD)
        };
        if let Some(cl) = self.claps.iter().find(|c| c.t0 - rise <= t && t < c.t0 + after) {
            let dt = t - cl.t0;
            let aim = if dt < 0.0 { st.th } else { cl.aim };
            let dt = if gliding && dt < 0.0 { dt * 2.0 } else { dt };
            return Some(Self::clap_pose(dt, aim, R + cl.reach + 0.25, cl.half.max(1.2) + 0.6));
        }
        gliding.then(|| glide(Self::heading(p, f.i, k, st.th)))
    }

    /// Seconds a FEATHER SHOWER takes to throw its feathers.
    fn volley(&self, p: &PictureCtx) -> f64 {
        self.nums.volley_beats * p.show.grid.beat
    }

    /// Every clap from `t - back` to `t`, as what sheds feathers and sends a gust.
    fn bursts(&self, p: &PictureCtx, t: f64, back: f64) -> Vec<Burst> {
        let k = self.k;
        let mut out = Vec::new();
        for cl in self.claps.iter().filter(|c| t - back <= c.t0 && c.t0 <= t) {
            let landed = bold::events_in(p, k, &["hit:wingbeat"], cl.t0 - 1e-6, cl.t0 + 1e-6)
                .next()
                .is_some();
            out.push(Burst {
                t0: cl.t0,
                c: p.run.p(p.sim_i(cl.t0), k),
                dir: cl.aim,
                half: cl.half * 0.8,
                r0: R + cl.reach * 0.5,
                r1: R + cl.reach + if landed { 1.5 } else { 0.7 },
                feathers: if landed { 7 } else { 4 },
            });
        }
        for e in bold::events_in(p, k, &["hit:flank"], t - back, t) {
            let i = p.sim_i(e.t);
            out.push(Burst {
                t0: e.t,
                c: p.run.p(i, k),
                dir: Self::toward(p, i, k),
                half: 0.9,
                r0: R + 0.3,
                r1: R + 2.4,
                feathers: 7,
            });
        }
        for tk in self.takes.iter().filter(|tk| t - back <= tk.t_clap && tk.t_clap <= t) {
            let i = p.sim_i(tk.t_clap);
            out.push(Burst {
                t0: tk.t_clap,
                c: p.run.p(i, k),
                dir: Self::toward(p, i, k),
                half: dmath::PI,
                r0: R + 0.4,
                r1: tk.radius + 0.3,
                feathers: 16,
            });
        }
        out
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let tint = bold::light(p.col[k]);
        // FLANKING CALL: dots along the way the ball will swoop, and a mark where it claps
        for fl in self
            .flanks
            .iter()
            .filter(|fl| fl.t0 - WARN <= c.t && c.t < fl.t_end + WARN)
        {
            let (i0, i1) = (p.sim_i(fl.t0), p.sim_i(fl.t_end));
            let path: Vec<[f64; 2]> = (i0..=i1).map(|i| p.run.p(i, k)).collect();
            let len: f64 = path
                .windows(2)
                .map(|w| dmath::hypot(w[1][0] - w[0][0], w[1][1] - w[0][1]))
                .sum();
            let gap = (len / 12.0).max(0.3);
            let shown = if c.t < fl.t0 {
                len * (c.t - (fl.t0 - WARN)) / WARN
            } else {
                len
            };
            let fade = (1.0 - (c.t - fl.t_end) / WARN).clamp(0.0, 1.0);
            let (mut run, mut next) = (0.0, gap * 0.5);
            for w in path.windows(2) {
                run += dmath::hypot(w[1][0] - w[0][0], w[1][1] - w[0][1]);
                if run >= next && run <= shown {
                    next += gap;
                    floor::disc(ic, c, w[1][0], w[1][1], 0.15, tint, 2.2 * fade);
                }
            }
            if let (Some(end), true) = (path.last(), c.t < fl.t_end) {
                let u = ((c.t - (fl.t0 - WARN)) / (fl.t_end - fl.t0 + WARN).max(1e-6)).clamp(0.0, 1.0);
                floor::target(ic, c, end[0], end[1], R * 1.1, u, tint);
            }
        }
        for tk in &self.takes {
            let me = c.fs[k].p;
            if tk.t0 <= c.t && c.t < tk.t_clap {
                // TAKE WING: the reach of the giant clap, brightening as it gathers
                let u = (c.t - tk.t0) / (tk.t_clap - tk.t0).max(1e-3);
                floor::ring(ic, c, me[0], me[1], tk.radius, 0.07, tint, 0.4 + 0.8 * u);
                floor::band(
                    ic,
                    c,
                    Family::Air,
                    me[0],
                    me[1],
                    tk.radius * (1.25 - 0.25 * u),
                    u,
                    c.t,
                    tint,
                    0.8,
                );
            } else if tk.t_clap <= c.t && c.t < tk.t_clap + 0.5 {
                // the clap: puffs of air run out all round
                let at = p.run.p(p.sim_i(tk.t_clap), k);
                let u = (c.t - tk.t_clap) / 0.5;
                for (lag, strength) in [(0.0, 1.5), (0.22, 1.0)] {
                    let un = u - lag;
                    if un > 0.0 {
                        let r = R + (tk.radius - R) * bold::ease_out(un / (1.0 - lag));
                        floor::band(ic, c, Family::Air, at[0], at[1], r, 1.0 - un, c.t, tint, strength);
                    }
                }
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let k = self.k;
        let mut out: Vec<(f64, Payload)> = Vec::new();
        // FEATHER SHOWER: every shot a big fluttering feather
        if f.in_fight
            && let Some(rows) = p.run.frame_values("K", f.i)
        {
            for (n, row) in rows.as_chunks::<7>().0.iter().enumerate() {
                if row[0].is_nan() || row[3].round() as i64 != k as i64 {
                    continue;
                }
                let flut = f.t * 11.0 + n as f64 * 2.3;
                let a = f.t * 2.2 + n as f64 * 1.9 + 0.5 * dmath::sin(flut);
                let y = 0.5 + 0.12 * dmath::sin(flut * 0.7);
                let show = 0.65 + 0.35 * dmath::cos(flut).abs();
                let depth = f.cam.depth(&[world([row[0], row[1]], y)])[0];
                out.push((depth, Box::new(BItem::Feather(row[0], row[1], a, y, show))));
            }
        }
        let st = &f.fs[k];
        if !st.vis || st.shattered || f.t > p.t_ko {
            return out;
        }
        if let Some(pose) = self.pose(p, f).filter(|pose| !pose.crisp) {
            for s in [1.0, -1.0] {
                // sorted by the middle of the wing, so a wing behind a ball is hidden by it
                let mid_a = pose.aim + s * (pose.phi + 0.4);
                let mid = [
                    st.p[0] + (R + pose.span * 0.4) * dmath::cos(mid_a),
                    st.p[1] + (R + pose.span * 0.4) * dmath::sin(mid_a),
                ];
                let depth = f.cam.depth(&[world(mid, R)])[0];
                out.push((depth, Box::new(BItem::Wing(wing(st.p, s, pose, p.col[k])))));
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        match payload.downcast_ref::<BItem>() {
            Some(BItem::Wing(feathers)) => {
                for (pts, col) in feathers {
                    bold::outlined_poly(fr, f.cam, pts, *col);
                }
            }
            Some(&BItem::Feather(x, z, a, y, show)) => {
                let (len, wd) = (1.0, 0.4 * show);
                let root = [x - 0.5 * dmath::cos(a), z - 0.5 * dmath::sin(a)];
                bold::outlined_poly(fr, f.cam, &leaf(root, a, len, wd, y, y), FEATHER);
                // the tip in the fighter's colour and the quill down the middle
                let tip = [x + 0.16 * dmath::cos(a), z + 0.16 * dmath::sin(a)];
                bold::outlined_poly(
                    fr,
                    f.cam,
                    &leaf(tip, a, 0.34, wd * 0.8, y + 0.01, y + 0.01),
                    p.col[self.k],
                );
                let end = [x + 0.5 * dmath::cos(a), z + 0.5 * dmath::sin(a)];
                fr.line(
                    f.cam,
                    &[world(root, y + 0.02), world(end, y + 0.02)],
                    bold::deep(p.col[self.k]),
                    1,
                );
            }
            None => {}
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        let (k, t, cam) = (self.k, f.t, f.cam);
        let o = 1 - k;
        let col = p.col[k];
        let st = &f.fs[k];
        let alive = st.vis && !st.shattered && t <= p.t_ko;
        // TAKE WING's wings, bright over the dimmed ring
        if alive && let Some(pose) = self.pose(p, f).filter(|pose| pose.crisp) {
            for s in [1.0, -1.0] {
                for (pts, shade) in wing(st.p, s, pose, col) {
                    px::ink_poly(img, &cam.wp(&pts), shade, 1.0);
                }
            }
        }
        if !f.in_fight {
            return;
        }
        // FLANKING CALL: loose feathers where the ball has just been
        if alive && self.flanks.iter().any(|fl| fl.t0 <= t && t < fl.t_end) {
            for j in 1..=5usize {
                let Some(i) = f.i.checked_sub(j * 3) else { break };
                let q = p.run.p(i, k);
                let back = Self::heading(p, i, k, st.th) + dmath::PI;
                let wob = 0.6 * dmath::sin(t * 9.0 + j as f64 * 1.7);
                let a = 1.0 - j as f64 / 6.0;
                px_feather(img, cam, q, back + wob, 0.5, 0.18, R * 0.5, FEATHER, col, a);
            }
        }
        for b in self.bursts(p, t, SHED.max(GUST)) {
            let age = t - b.t0;
            // the gust front
            if age < GUST {
                gust(img, cam, b.c, b.dir, b.half, b.r0, b.r1, age / GUST, col);
            }
            // the feathers it shed flutter out and down
            let u = age / SHED;
            let seed = (b.t0 * 1000.0) as u64;
            for n in 0..b.feathers {
                let all_round = b.half >= dmath::PI;
                let spread = if all_round { dmath::PI } else { 1.3 };
                let a = b.dir + spread * (2.0 * px::rnd(seed, n) - 1.0);
                let far = if all_round { b.r1 * 0.85 } else { b.r1 * 0.55 + 0.5 };
                let r = b.r0 * 0.8 + (far - b.r0 * 0.8) * (0.35 + 0.65 * px::rnd(seed, n + 40)) * bold::ease_out(u);
                let sway = 0.25 * dmath::sin(t * 7.0 + n as f64 * 2.1);
                let q = [
                    b.c[0] + r * dmath::cos(a) - sway * dmath::sin(a),
                    b.c[1] + r * dmath::sin(a) + sway * dmath::cos(a),
                ];
                let turn = a + 1.2 + 1.1 * dmath::sin(t * 6.0 + n as f64 * 1.3);
                let y = R * 1.1 * (1.0 - u) + 0.05;
                let len = if all_round { 0.62 } else { 0.46 };
                let shade = if n % 3 == 0 { bold::pale(col) } else { FEATHER };
                px_feather(
                    img,
                    cam,
                    q,
                    turn,
                    len,
                    len * 0.36,
                    y,
                    shade,
                    col,
                    1.0 - bold::ease_in((u - 0.6) / 0.4),
                );
            }
        }
        // FEATHER SHOWER: a feather that touches stings: a little green star and white splinters
        for e in bold::events_in(p, k, &["hit:key"], t - 0.24, t) {
            let age = t - e.t;
            let at = bold::at(cam, e.f(5), e.f(6), R);
            let u = age / 0.24;
            px::ink_star(
                img,
                at,
                5.0 + 9.0 * bold::ease_out(u),
                2.2,
                4,
                0.4,
                bold::pale(col),
                1.0 - u,
            );
            px::chips(img, at, age, 0.24, 4, 16.0, (e.t * 1000.0) as u64, FEATHER);
        }
        // an enemy a clap sent flying trails speed lines
        if let Some((age, _)) = bold::since(p, k, &["hit:wingbeat", "hit:flank", "hit:take_wing"], t, BLOWN)
            && let Some(d) = drawn.iter().find(|d| d.k == o)
            && !f.fs[o].shattered
        {
            let (a, b) = (p.run.p(f.i.saturating_sub(2), o), p.run.p(f.i, o));
            let (s0, s1) = (bold::at(cam, a[0], a[1], R), bold::at(cam, b[0], b[1], R));
            let dir = (s1.0 - s0.0, s1.1 - s0.1);
            if dmath::hypot(dir.0, dir.1) > 1.5 {
                let u = age / BLOWN;
                px::speed_lines(img, d.s, d.rp, dir, d.rp * (1.6 - 0.6 * u), 6, WHITE, 1.0 - u);
            }
        }
    }
}
