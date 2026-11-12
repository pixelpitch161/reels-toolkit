//! ACOUSTIC's look: the port of `Look` in `projects/ballfight/kit/fighters/acoustic.py`, hook by hook, with the
//! module's draw helpers (`overlay_polys`, `draw_capo`, `rake_segments`, `draw_strings`, `draw_whip`,
//! `clip_segment`, `sweep_segments`; `mask_of` and `blend` are `shared::reeds`').
//!
//! What it draws: the ball flashing on every stroke; the six strings of each stroke (wound bronze E A D, plain G B E)
//! under one warm glow, with a motion fan between frames and a lit band on the canvas under them; six-line claw marks
//! where every stroke ends; FINGERPICK's whips with a travelling wave and a bright tip; BIG STRUM's six thick
//! strings across the whole ring over a bright band, with a heat shimmer on its front; the dark capo bar while CAPO
//! lasts; strings, spruce top chips, dark side chips and frets in the shatter.
//!
//! The arithmetic keeps Python's order and dtypes: scalar maths through `bf_core::dmath`, the frame and the masks in
//! `float32` where numpy's are (a `float32` mask times a Python float stays `float32`; the overlay's composite is
//! all `float32`).

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::kinds::rake::{rake_lateral, sweep_front};
use bf_core::roster::{AbilityKind, FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{BAYER, FPS, H, W, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, FrameCtx, Fx, Look, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::{np, pil};

use super::state::{self, Rake, Sweep, Whip};
use crate::fighters::shared::reeds::{self, Num};

/// `CLAW_COL`: the claw marks where a stroke ends.
const CLAW_COL: V3 = [70.0, 44.0, 30.0];
/// `STRINGS6`: wound bronze E A D, plain G B E.
const STRINGS6: [V3; 6] = [
    [198.0, 150.0, 86.0],
    [206.0, 160.0, 96.0],
    [214.0, 172.0, 110.0],
    [226.0, 218.0, 204.0],
    [234.0, 228.0, 218.0],
    [242.0, 238.0, 230.0],
];
/// `CAPO_METAL`.
const CAPO_METAL: V3 = [44.0, 40.0, 52.0];
/// `STRING_COL`: bronze steel strings (shards).
const STRING_COL: [u8; 3] = [212, 176, 118];
/// `WOOD_COL['top']`: spruce.
const WOOD_TOP: V3 = [226.0, 190.0, 122.0];
/// `WOOD_COL['side']`: the dark sides.
const WOOD_SIDE: V3 = [112.0, 52.0, 30.0];
/// The strings' thickness of a stroke, `(2, 2, 2, 1, 1, 1)`.
const THICK_RAKE: [u32; 6] = [2, 2, 2, 1, 1, 1];
/// The strings' thickness of BIG STRUM, `(3, 3, 3, 2, 2, 2)`.
const THICK_SWEEP: [u32; 6] = [3, 3, 3, 2, 2, 2];

// ---------------------------------------------------------------- #33 styles.py helpers
/// ACOUSTIC's `blend(arr, m, col, k)`: `arr += (np.clip(col, 0, 255) - arr) * (m * k)[..., None]` on the `float32`
/// frame and mask (no early return).
fn blend(arr: &mut Img<f32>, m: &Img<f32>, col: V3, k: f64) {
    reeds::blend_into(arr, m, col.map(|c| np::clip(c, 0.0, 255.0)), Num::Py(k));
}

/// `S.to_world(x, z, y)` of a ring-frame point.
fn world(q: [f64; 2], y: f64) -> V3 {
    scene::to_world(q[0], q[1], y)
}

/// `a + u * s` for 2-vectors, numpy's element by element.
fn along(a: [f64; 2], u: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] + u[0] * s, a[1] + u[1] * s]
}

/// `overlay_polys(arr, polys, alpha_max=1.0)`: draws `[(screen pts, rgb, alpha)]` onto one RGBA overlay in order
/// (no blending: a later polygon replaces an earlier one), then composites it once, all in `float32`. Nothing when
/// `polys` is empty.
fn overlay_polys(arr: &mut Img<f32>, polys: &[(Vec<(f64, f64)>, V3, f64)]) {
    if polys.is_empty() {
        return;
    }
    let mut ov: Img<u8> = Img::new(W, H, 4);
    for (pts, rgb, a) in polys {
        if pts.len() >= 3 {
            // tuple(int(v) for v in np.clip(rgb, 0, 255)) + (int(255 * a),)
            let c = rgb.map(|v| np::clip(v, 0.0, 255.0) as u8);
            let fill = [c[0], c[1], c[2], (255.0 * a) as u8];
            pil::polygon(&mut ov, pts, &fill);
        }
    }
    for (px, o) in arr
        .data
        .as_chunks_mut::<3>()
        .0
        .iter_mut()
        .zip(ov.data.as_chunks::<4>().0)
    {
        // k = o[..., 3:4] / 255.0 * alpha_max; arr * (1 - k) + o[..., :3] * k
        let k = f32::from(o[3]) / 255.0f32 * 1.0f32;
        for (v, &c) in px.iter_mut().zip(&o[..3]) {
            *v = *v * (1.0f32 - k) + f32::from(c) * k;
        }
    }
}

/// `draw_capo(arr, s, rp, k, col, t)`: a capo clamped across the glass: a dark bar with a tension screw and a
/// highlight in the fighter's colour.
fn draw_capo(arr: &mut Img<f32>, s: (f64, f64), rp: f64, k: f64, col: V3) {
    if k <= 0.02 {
        return;
    }
    let (x, y) = s;
    let yb = y - rp * 0.30;
    let w = rp * 0.92;
    let m = reeds::mask_of(
        &[vec![
            (x - w, yb - 2.0),
            (x + w, yb - 2.0),
            (x + w, yb + 1.5),
            (x - w, yb + 1.5),
        ]],
        None,
    );
    blend(arr, &m, CAPO_METAL, 0.95 * k);
    let hl = reeds::mask_of(&[vec![(x - w, yb - 2.0), (x + w, yb - 2.0)]], Some(1));
    blend(arr, &hl, col.map(|c| np::clip(c * 0.9 + 60.0, 0.0, 255.0)), 0.9 * k);
    let sc = reeds::mask_of(
        &[vec![
            (x + w - 1.0, yb - 4.0),
            (x + w + 2.0, yb - 4.0),
            (x + w + 2.0, yb + 3.0),
            (x + w - 1.0, yb + 3.0),
        ]],
        None,
    );
    blend(arr, &sc, [210.0, 210.0, 222.0], 0.9 * k);
}

/// A string as two ring-frame points.
type Seg = ([f64; 2], [f64; 2]);

/// `rake_segments(c, u, n, s, L, gap)`: the six strings of a stroke whose bundle sits at lateral offset `s`.
fn rake_segments(r: &Rake, s: f64, gap: f64) -> Vec<Seg> {
    (0..6)
        .map(|j| {
            let o = (f64::from(j) - 2.5) * gap;
            // a = c + u * R0 + n * (s + o); b = c + u * (R0 + L) + n * (s + o)
            let a = along(along(r.c, r.u, R), r.n, s + o);
            let b = along(along(r.c, r.u, R + r.l), r.n, s + o);
            (a, b)
        })
        .collect()
}

/// `draw_strings(arr, cam, segs, glow_col, k, y, prev, thick)`: six strings, a glow in the fighter's colour under a
/// bright core; `prev` (the bundle a frame ago) adds a motion fan between the two places.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_strings(
    arr: &mut Img<f32>,
    cam: &Cam,
    segs: &[Seg],
    glow_col: V3,
    k: f64,
    y: f64,
    prev: Option<&[Seg]>,
    thick: &[u32],
) {
    if k <= 0.02 {
        return;
    }
    let mut polys = Vec::new();
    if let Some(prev) = prev {
        for (&(a, b), &(pa, pb)) in segs.iter().zip(prev) {
            let q = cam.wp(&[world(a, y), world(b, y), world(pb, y), world(pa, y)]);
            polys.push((q, glow_col.map(|c| c * 0.5 + 120.0), 0.22 * k));
        }
    }
    overlay_polys(arr, &polys);
    let lines: Vec<Vec<(f64, f64)>> = segs.iter().map(|&(a, b)| cam.wp(&[world(a, y), world(b, y)])).collect();
    let gm = reeds::mask_of(&lines, Some(5));
    // one warm glow under the bundle
    blend(
        arr,
        &gm,
        glow_col.map(|c| np::clip(c * 1.05 + 30.0, 0.0, 255.0)),
        0.55 * k,
    );
    for (j, line) in lines.into_iter().enumerate() {
        let col = STRINGS6[j].map(|c| np::clip(c * 0.6 + 110.0, 0.0, 255.0));
        blend(arr, &reeds::mask_of(&[line], Some(thick[j])), col, 0.98 * k);
    }
}

/// `draw_whip(arr, cam, p, ang, ln, tau, t_ext, glow_col, y=0.5, j=0)`: one FINGERPICK string flicking out like a
/// whip and snapping back: a travelling wave along it, a bright tip.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_whip(
    arr: &mut Img<f32>,
    cam: &Cam,
    p: [f64; 2],
    ang: f64,
    ln: f64,
    tau: f64,
    t_ext: f64,
    glow_col: V3,
    j: i64,
) {
    let y = 0.5;
    if tau < 0.0 || tau > 2.0 * t_ext {
        return;
    }
    let ext = if tau <= t_ext {
        tau / t_ext
    } else {
        1.0 - (tau - t_ext) / t_ext
    };
    let u = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-u[1], u[0]];
    let lc = ln * py_max(0.0, ext);
    if lc < 0.05 {
        return;
    }
    // np.linspace(0, 1, 12): arange(12) * (1 / 11), the last the end itself
    let step = 1.0 / 11.0;
    let pts: Vec<V3> = (0..12)
        .map(|i| {
            let x = if i == 11 { 1.0 } else { f64::from(i) * step + 0.0 };
            // lat = 0.09 * math.sin(math.pi * x * 2.5 - tau * 60 + j) * x * (1 - 0.5 * ext)
            let lat = 0.09 * dmath::sin(dmath::PI * x * 2.5 - tau * 60.0 + j as f64) * x * (1.0 - 0.5 * ext);
            // q = p + u * (R + Lc * x) + n * lat
            world(along(along(p, u, R + lc * x), n, lat), y)
        })
        .collect();
    let s = cam.wp(&pts);
    blend(
        arr,
        &reeds::mask_of(std::slice::from_ref(&s), Some(3)),
        glow_col.map(|c| np::clip(c * 0.8 + 50.0, 0.0, 255.0)),
        0.5,
    );
    let core = STRINGS6[py::imod_py(j, 6) as usize].map(|c| c * 1.05 + 20.0);
    blend(arr, &reeds::mask_of(std::slice::from_ref(&s), Some(1)), core, 0.95);
    let last = s[s.len() - 1];
    let (tx, ty) = (py::round_half_even(last.0) as i64, py::round_half_even(last.1) as i64);
    if 0 <= tx && tx < W as i64 && 0 <= ty && ty < H as i64 {
        // arr[max(0, ty - 1):ty + 2, max(0, tx - 1):tx + 2] = [255, 248, 230]
        for yy in (ty - 1).max(0)..(ty + 2).min(H as i64) {
            for xx in (tx - 1).max(0)..(tx + 2).min(W as i64) {
                arr.px_mut(xx as usize, yy as usize)
                    .copy_from_slice(&[255.0, 248.0, 230.0]);
            }
        }
    }
}

/// `clip_segment(a, b, lim)` (Liang-Barsky): the part of the segment `a`-`b` inside the square `|x|, |z| <= lim`, or
/// `None`.
fn clip_segment(a: [f64; 2], b: [f64; 2], lim: f64) -> Option<Seg> {
    let (mut t0, mut t1) = (0.0, 1.0);
    let d = [b[0] - a[0], b[1] - a[1]];
    for (p, q) in [
        (-d[0], a[0] + lim),
        (d[0], lim - a[0]),
        (-d[1], a[1] + lim),
        (d[1], lim - a[1]),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            t0 = py_max(t0, r);
        } else {
            t1 = py_min(t1, r);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((along(a, d, t0), along(a, d, t1)))
}

/// `sweep_segments(c, u, front, lim, gap=0.14)`: BIG STRUM's six strings lying across the whole ring (square to
/// `u`), the first at `front` from `c`, the rest trailing behind it, clipped to the canvas.
fn sweep_segments(c: [f64; 2], u: [f64; 2], front: f64, lim: f64) -> Vec<Seg> {
    let gap = 0.14;
    let n = [-u[1], u[0]];
    (0..6)
        .filter_map(|j| {
            let m = along(c, u, front - f64::from(j) * gap);
            // seg = clip_segment(m - n * 9.0, m + n * 9.0, lim)
            let a = [m[0] - n[0] * 9.0, m[1] - n[1] * 9.0];
            clip_segment(a, along(m, n, 9.0), lim)
        })
        .collect()
}

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// A stroke showing at a time: `(j, rake, s_now, s_prev, k)` of `rakes_at`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Showing {
    j: usize,
    sn: f64,
    sp: f64,
    k: f64,
}

/// What a painter item draws.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    /// CAPO's bar on the ball.
    Capo,
    /// A stroke's strings.
    Rake(Showing),
    /// A FINGERPICK string (an index into `whips`).
    Whip(usize),
    /// Every BIG STRUM showing (indices into `sweeps`).
    Sweeps(Vec<usize>),
}

// ---------------------------------------------------------------- the look
/// ACOUSTIC's look: `Look` of `kit/fighters/acoustic.py`.
#[derive(Clone, Debug, Default)]
pub struct AcousticLook {
    /// Its side.
    k: usize,
    /// `RAKES` (and `RAKE_T`, their `t0`s).
    rakes: Vec<Rake>,
    /// `RAKE_HIT`, by stroke.
    rake_hit: Vec<bool>,
    /// `GAP = self.cfg['weapon'].get('gap', 0.085)`.
    gap: f64,
    /// `self.cfg['weapon']`'s `gap`, if it has one.
    cfg_gap: Option<f64>,
    /// `self.cfg['abilities']['unlock2']`'s `t_ext`, if it has one.
    cfg_t_ext: Option<f64>,
    /// `WHIPS`.
    whips: Vec<Whip>,
    /// `T_EXT = self.cfg['abilities']['unlock2'].get('t_ext', 0.07)`.
    t_ext: f64,
    /// `SWEEPS`.
    sweeps: Vec<Sweep>,
    /// `CAPO_DUR`: the haste's `dur_beats` x the beat (the first of unlock1, unlock2, low that is a haste), 0 without.
    capo_dur: f64,
}

impl AcousticLook {
    /// A look not yet set up.
    pub fn new() -> AcousticLook {
        AcousticLook::default()
    }

    /// `ta(t)`: the attacks' clock, frozen at the K.O. (`min(t, P.T_KO)`).
    fn ta(&self, p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    /// `show(t)`: `T.T_FIGHT <= t < P.T_HS + 0.12`.
    fn show(&self, p: &PictureCtx, t: f64) -> bool {
        p.show.grid.t_fight <= t && t < p.t_hs + 0.12
    }

    /// `rakes_at(t)`: the strokes still showing (the stroke and a 0.2 s fade).
    fn rakes_at(&self, t: f64) -> Vec<Showing> {
        if self.rakes.is_empty() {
            return Vec::new();
        }
        // j0, j1 = np.searchsorted(RAKE_T, t - 0.4), np.searchsorted(RAKE_T, t, 'right')
        let lo = t - 0.4;
        let j0 = self.rakes.partition_point(|r| r.t0 < lo);
        let j1 = self.rakes.partition_point(|r| r.t0 <= t);
        let mut out = Vec::new();
        for j in j0..j1 {
            let r = &self.rakes[j];
            let tau = t - r.t0;
            if tau > r.dur + 0.2 {
                continue;
            }
            let sn = rake_lateral(tau, r.dur, r.s, r.dir);
            let sp = rake_lateral(py_max(0.0, tau - 1.0 / f64::from(FPS)), r.dur, r.s, r.dir);
            let k = if tau <= r.dur { 1.0 } else { 1.0 - (tau - r.dur) / 0.2 };
            out.push(Showing { j, sn, sp, k });
        }
        out
    }

    /// `sweep_front_at(sw, t)`.
    fn sweep_front_at(sw: &Sweep, t: f64) -> f64 {
        sweep_front(t - sw.t0, sw.dur, sw.reach, R)
    }

    /// `now(F)`: this frame's live strokes, whips (indices) and sweeps (indices), and `ta`.
    fn now(&self, p: &PictureCtx, t: f64) -> (Vec<Showing>, Vec<usize>, Vec<usize>, f64) {
        let ta = self.ta(p, t);
        if !self.show(p, t) {
            return (Vec::new(), Vec::new(), Vec::new(), ta);
        }
        let whips = (0..self.whips.len())
            .filter(|&i| {
                let d = ta - self.whips[i].t0;
                0.0 <= d && d <= 2.0 * self.t_ext
            })
            .collect();
        let sweeps = (0..self.sweeps.len())
            .filter(|&i| {
                let sw = &self.sweeps[i];
                let d = ta - sw.t0;
                0.0 <= d && d <= sw.dur + 0.2
            })
            .collect();
        (self.rakes_at(ta), whips, sweeps, ta)
    }
}

impl Look for AcousticLook {
    fn shards(&self) -> Vec<String> {
        // ['string'] * 6 + ['top'] * 5 + ['side'] * 3 + ['fret'] * 3
        [("string", 6), ("top", 5), ("side", 3), ("fret", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    /// #33: rake, whip and sweep sparks are the small spray.
    fn spark_big(&self) -> &[&str] {
        &[]
    }

    /// #33: none of its hits set off the flashbulbs (abilities do).
    fn flash_hits(&self) -> &[&str] {
        &[]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["sweep"]
    }

    /// No rope-shaking ability.
    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    /// The big brother: the instrument fills its ball.
    fn sprite_k(&self) -> f64 {
        0.86
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `acoustic.py` reads from `self.cfg` (look.json's `cfg_keys`): each
    /// as the entry holds it (`None` where it has none and the look takes Python's default). ACOUSTIC's own numbers
    /// are the defaults' values, so the values alone would not tell its numbers from another fighter's.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!(
            "abilities.unlock2.t_ext={:?}; weapon.gap={:?}",
            self.cfg_t_ext, self.cfg_gap
        ))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the strings' gap, FINGERPICK's reach-out time and CAPO's
    /// length come from `cfg`, the events from side `k`. For its own side `cfg` is `p.cfg[k]`, and this is `setup`.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        self.rakes = state::rakes(ev, k);
        self.rake_hit = state::rake_hits(ev, k, &self.rakes);
        // self.cfg['weapon'].get('gap', 0.085): a weapon of another kind has no gap
        self.cfg_gap = match &cfg.weapon {
            WeaponCfg::Rake(w) => Some(w.gap),
            _ => None,
        };
        self.gap = self.cfg_gap.unwrap_or(0.085);
        self.whips = state::whips(ev, k);
        self.cfg_t_ext = cfg.unlock2.get("t_ext");
        self.t_ext = self.cfg_t_ext.unwrap_or(0.07);
        self.sweeps = state::sweeps(ev, k);
        // capo = [s for s in ('unlock1', 'unlock2', 'low') if kind == 'haste']; CAPO_DUR = dur_beats * BEAT
        let capo = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
            .into_iter()
            .find(|ab| ab.kind == AbilityKind::Haste);
        self.capo_dur = match capo {
            // (Python raises for a haste without `dur_beats`)
            Some(ab) => ab.get("dur_beats").unwrap_or(0.0) * p.show.grid.beat,
            None => 0.0,
        };
    }

    // ------------------------------------------------------------ the ball flashes on every stroke
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if phase == Phase::Fight && kk == self.k && !self.rakes.is_empty() {
            let tc = py_min(t, p.t_ko);
            let after = self.rakes.partition_point(|r| r.t0 <= tc);
            if let Some(j) = after.checked_sub(1) {
                // 0.35 * math.exp(-(tc - RAKE_T[j]) / 0.09): a Python float
                st.glow = 0.35 * dmath::exp(-(tc - self.rakes[j].t0) / 0.09);
                st.glow_np = false;
            }
        }
    }

    // ------------------------------------------------------------ events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "rake" => Some(0.5),
            "sweep" => Some(2.5),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "sweep" => Some((12.0, 0.35)),
            "rake" => Some((7.0, 0.22)),
            _ => None,
        }
    }

    /// Every stroke lights the strings and leaves six-line claw marks where it ends.
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        for (j, r) in self.rakes.iter().enumerate() {
            let s_end = rake_lateral(r.dur, r.dur, r.s, r.dir);
            // mid = c + u * (R + L * 0.5) + n * s_end
            let mid = along(along(r.c, r.u, R + r.l * 0.5), r.n, s_end);
            let peak = if r.main { 0.35 } else { 0.22 };
            let col = p.fl_col[k].map(|c| c * 0.9);
            fx.fl
                .add(r.t0 + r.dur * 0.5, world(mid, 0.7), col, 1.3, peak, 0.03, 0.14, 0.0);
            if r.t0 + r.dur < p.t_ko {
                fx.mk.add(
                    r.t0 + r.dur,
                    Mark {
                        size: r.l * 0.5,
                        angle: r.aim,
                        n: 6,
                        col: CLAW_COL,
                        life: 2.6,
                        seed: j as u64,
                        strength: if self.rake_hit[j] { 0.95 } else { 0.5 },
                        ..Mark::new(MarkKind::Claw, mid[0], mid[1])
                    },
                );
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let Some(att) = e.who().filter(|&a| a < 2) else { return };
        let (t, x, z) = (e.t, e.f(5), e.f(6));
        let pos = scene::to_world(x, z, R);
        match how(e) {
            "rake" => {
                fx.bu.add(t, pos, p.col[att], 0.8, None, true);
                fx.fl
                    .add(t, pos, p.fl_col[att].map(|c| c * 0.95), 1.5, 0.65, 0.03, 0.22, 0.0);
                let emit = Emit {
                    n: Some(8),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "chips", &emit);
            }
            "whip" => {
                fx.bu.add(t, pos, p.col[att], 0.4, None, false);
                let emit = Emit {
                    n: Some(3),
                    speed: Some((1.0, 2.0)),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "chips", &emit);
            }
            "sweep" => {
                fx.bu.add(t, pos, p.col[att], 2.0, None, true);
                fx.fl.add(t, pos, p.fl_col[att], 2.4, 1.1, 0.03, 0.5, 0.0);
                let emit = Emit {
                    n: Some(24),
                    speed: Some((1.5, 3.2)),
                    scale: 1.2,
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "chips", &emit);
                // angle = math.atan2(*(SWEEPS[0]['u'][::-1])) + math.pi / 2 if self.SWEEPS else 0.0
                let angle = match self.sweeps.first() {
                    Some(sw) => dmath::atan2(sw.u[1], sw.u[0]) + dmath::PI / 2.0,
                    None => 0.0,
                };
                fx.mk.add(
                    t,
                    Mark {
                        size: 1.2,
                        angle,
                        n: 6,
                        col: CLAW_COL,
                        life: 6.0,
                        seed: 77,
                        ..Mark::new(MarkKind::Claw, x, z)
                    },
                );
            }
            _ => {}
        }
    }

    /// BIG STRUM: the sweep lights the ring as it crosses.
    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        for sw in &self.sweeps {
            for u_ in [0.2, 0.5, 0.8] {
                let tu = sw.t0 + sw.dur * u_;
                let f = Self::sweep_front_at(sw, tu);
                let c = along(sw.c, sw.u, f);
                fx.fl
                    .add(tu, world(c, 0.9), p.fl_col[self.k], 2.6, 0.6, 0.03, 0.25, 0.0);
            }
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let emit = Emit {
            n: Some(30),
            spread: Some(180.0),
            speed: Some((0.8, 2.4)),
            scale: 1.2,
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "chips", &emit);
    }

    // ------------------------------------------------------------ the canvas
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let (rakes_now, _, sweeps_now, ta) = self.now(p, c.t);
        for s in &rakes_now {
            // light under each stroke's six strings
            let r = &self.rakes[s.j];
            let f = if r.main { 0.45 } else { 0.3 };
            let tint = p.col[r.owner].map(|v| v / 255.0 * 0.9 + 0.25);
            for (i, light) in ic.iter_mut().enumerate() {
                let (dx, dz) = (c.sxp[i] - r.c[0], c.szp[i] - r.c[1]);
                let ra = dx * r.u[0] + dz * r.u[1];
                let rb = dx * r.n[0] + dz * r.n[1];
                // inl = np.clip(1 - np.maximum(R * 0.9 - ra, ra - (R + L)) / 0.15, 0, 1)
                let inl = np::clip(1.0 - np::maximum(R * 0.9 - ra, ra - (R + r.l)) / 0.15, 0.0, 1.0);
                let band = np::clip(1.0 - (rb - s.sn).abs() / 0.34, 0.0, 1.0) * inl;
                let band = (band * 3.0 + BAYER[c.ys[i] % 4][c.xs[i] % 4] * 0.9).floor() / 3.0;
                let v = band * s.k * f;
                for (l, &w) in light.iter_mut().zip(&tint) {
                    *l += v * w;
                }
            }
        }
        for &si in &sweeps_now {
            // BIG STRUM: a bright band under the strings
            let sw = &self.sweeps[si];
            let fpos = Self::sweep_front_at(sw, ta);
            let fade = 1.0 - ss(sw.dur, sw.dur + 0.2, ta - sw.t0);
            let tint = p.col[sw.owner].map(|v| v / 255.0 + 0.3);
            for (i, light) in ic.iter_mut().enumerate() {
                let ra = (c.sxp[i] - sw.c[0]) * sw.u[0] + (c.szp[i] - sw.c[1]) * sw.u[1] - fpos;
                let band = np::clip(1.0 - (ra + 0.35).abs() / 0.55, 0.0, 1.0) * fade;
                let band = (band * 3.0 + BAYER[c.ys[i] % 4][c.xs[i] % 4] * 0.9).floor() / 3.0;
                let v = band * 0.7;
                for (l, &w) in light.iter_mut().zip(&tint) {
                    *l += v * w;
                }
            }
        }
    }

    // ------------------------------------------------------------ the painter pass
    /// CAPO: clamped across the ball while it lasts (drawn right after the ball: the same depth).
    fn items_ball(&self, _p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        if self.capo_dur > 0.0 && st.haste > 0.0 && f.in_fight {
            let pw = world(st.p, R);
            let capo: Payload = Box::new(Item::Capo);
            return vec![(f.cam.depth(&[pw])[0], capo)];
        }
        Vec::new()
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let (rakes_now, whips_now, sweeps_now, _) = self.now(p, f.t);
        let cam = f.cam;
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for s in rakes_now {
            let r = &self.rakes[s.j];
            let mid = along(along(r.c, r.u, R + r.l * 0.5), r.n, s.sn);
            out.push((cam.depth(&[world(mid, 0.5)])[0], Box::new(Item::Rake(s))));
        }
        for w in whips_now {
            let at = f.fs[self.whips[w].owner].p;
            out.push((cam.depth(&[world(at, 0.5)])[0] - 0.02, Box::new(Item::Whip(w))));
        }
        if !sweeps_now.is_empty() {
            // #33 drew these after the whole painter pass
            out.push((-1e9, Box::new(Item::Sweeps(sweeps_now))));
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(item) = payload.downcast_ref::<Item>() else {
            return;
        };
        let cam = f.cam;
        let ta = self.ta(p, f.t);
        let (mut arr, depth) = (fr.array(), fr.depth());
        match item {
            Item::Capo => {
                let st = &f.fs[self.k];
                let pw = world(st.p, R);
                let pc = cam.to_cam(&[pw])[0];
                let s = cam.proj(&[pc])[0];
                let rp = R * cam.f / pc[2] * st.scale;
                // k_ = min(1.0, st['haste'] / 0.15, (CAPO_DUR - st['haste']) / 0.12)
                let k_ = py_min(py_min(1.0, st.haste / 0.15), (self.capo_dur - st.haste) / 0.12);
                draw_capo(&mut arr, s, rp, py_max(0.0, k_), p.col[self.k]);
            }
            Item::Rake(s) => {
                let r = &self.rakes[s.j];
                let segs = rake_segments(r, s.sn, self.gap);
                let prev = ((s.sn - s.sp).abs() > 0.01).then(|| rake_segments(r, s.sp, self.gap));
                let k = s.k * if r.main { 1.0 } else { 0.75 };
                draw_strings(
                    &mut arr,
                    cam,
                    &segs,
                    p.col[r.owner],
                    k,
                    0.5,
                    prev.as_deref(),
                    &THICK_RAKE,
                );
            }
            Item::Whip(w) => {
                let w = &self.whips[*w];
                let at = f.fs[w.owner].p;
                draw_whip(
                    &mut arr,
                    cam,
                    at,
                    w.ang,
                    w.len,
                    ta - w.t0,
                    self.t_ext,
                    p.col[w.owner],
                    w.j,
                );
            }
            Item::Sweeps(list) => {
                for &si in list {
                    let sw = &self.sweeps[si];
                    let tau = ta - sw.t0;
                    let kk = 1.0 - ss(sw.dur, sw.dur + 0.2, tau);
                    let f1 = Self::sweep_front_at(sw, ta);
                    let f0 = Self::sweep_front_at(sw, ta - 1.0 / f64::from(FPS));
                    let segs = sweep_segments(sw.c, sw.u, f1, scene::AP);
                    let prev = (f1 - f0 > 0.02 && !segs.is_empty())
                        .then(|| sweep_segments(sw.c, sw.u, f0, scene::AP))
                        .filter(|prev| prev.len() == segs.len());
                    let thick = &THICK_SWEEP[..segs.len()];
                    draw_strings(&mut arr, cam, &segs, p.col[sw.owner], kk, 0.55, prev.as_deref(), thick);
                }
            }
        }
        fr.set_array(&arr, Some(&depth));
    }

    // ------------------------------------------------------------ shards: strings, top, sides, frets
    fn draw_shard(
        &self,
        _p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let on_screen = |pts: &[[f64; 2]]| -> Vec<(f64, f64)> {
            let w: Vec<V3> = pts.iter().map(|&q| world(q, pos[1])).collect();
            screen(cam, &w)
        };
        if kind == "string" {
            // [s + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
            let pts: Vec<[f64; 2]> = (0..7)
                .map(|j| {
                    let a = f64::from(j - 3);
                    let b = dmath::sin(f64::from(j));
                    [
                        sx + d[0] * a * 0.16 + nn[0] * curl * 0.10 * b,
                        sz + d[1] * a * 0.16 + nn[1] * curl * 0.10 * b,
                    ]
                })
                .collect();
            let sp = on_screen(&pts);
            // Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)): the line, then back as float32
            let bytes = arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect();
            let mut im: Img<u8> = Img::from_vec(arr.w, arr.h, arr.c, bytes);
            pil::line(&mut im, &sp, &STRING_COL, 1);
            for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                *v = f32::from(b);
            }
            return;
        }
        if kind == "fret" {
            let (lk, wk) = (0.30, 0.035);
            // [s + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            let q4: Vec<[f64; 2]> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    [
                        sx + d[0] * lk / 2.0 * a1 + nn[0] * wk / 2.0 * a2,
                        sz + d[1] * lk / 2.0 * a1 + nn[1] * wk / 2.0 * a2,
                    ]
                })
                .collect();
            layer_blend(arr, &[on_screen(&q4)], [222.0, 224.0, 236.0], 1.0, None);
            return;
        }
        // top (spruce), side (dark): wooden chips (Python has no colour for another kind and raises)
        let (col, sz_) = match kind {
            "top" => (WOOD_TOP, size * 1.3),
            "side" => (WOOD_SIDE, size * 1.0),
            _ => return,
        };
        let tri = [
            [sx + d[0] * sz_, sz + d[1] * sz_],
            [
                sx - d[0] * sz_ * 0.7 + nn[0] * sz_ * 0.8,
                sz - d[1] * sz_ * 0.7 + nn[1] * sz_ * 0.8,
            ],
            [
                sx - d[0] * sz_ * 0.4 - nn[0] * sz_ * 0.7,
                sz - d[1] * sz_ * 0.4 - nn[1] * sz_ * 0.7,
            ],
        ];
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        let col = col.map(|c| np::clip(c * shade_k, 0.0, 255.0));
        layer_blend(arr, &[on_screen(&tri)], col, 1.0, None);
    }

    // ------------------------------------------------------------ lens
    /// BIG STRUM's front ripples the air.
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        if !self.show(p, t) {
            return;
        }
        let ta = self.ta(p, t);
        for sw in &self.sweeps {
            let tau = ta - sw.t0;
            if 0.0 <= tau && tau <= sw.dur {
                let f = Self::sweep_front_at(sw, ta);
                let c = along(sw.c, sw.u, f);
                img.shimmer_world(cam, world(c, 0.05), 1.6, 0.5, 2.6, t, 0.8, 0.0);
            }
        }
    }

    /// The strings' far ends while a stroke shows.
    fn focus_points(&self, p: &PictureCtx, t: f64, _st: &BallState) -> Vec<(V3, String)> {
        if !self.show(p, t) {
            return Vec::new();
        }
        self.rakes_at(self.ta(p, t))
            .iter()
            .map(|s| {
                let r = &self.rakes[s.j];
                let b = along(along(r.c, r.u, R + r.l), r.n, s.sn);
                (world(b, 0.5), "rake".to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Liang-Barsky: a segment across the square is cut at its sides; one outside is none; one inside is whole.
    #[test]
    fn a_segment_is_clipped_to_the_square() {
        let (a, b) = clip_segment([-9.0, 0.5], [9.0, 0.5], 3.62).expect("crosses");
        assert_eq!((a, b), ([-3.62, 0.5], [3.62, 0.5]));
        assert_eq!(clip_segment([-9.0, 5.0], [9.0, 5.0], 3.62), None);
        assert_eq!(
            clip_segment([0.1, 0.2], [0.3, 0.4], 3.62),
            Some(([0.1, 0.2], [0.3, 0.4]))
        );
    }

    /// The six strings of a stroke are `gap` apart round the bundle's middle and `L` long, from the ball's surface;
    /// BIG STRUM's trail 0.14 behind its front, square to the aim, across the whole canvas.
    #[test]
    fn the_strings_lie_where_the_kit_draws_them() {
        let r = Rake {
            t0: 1.0,
            owner: 0,
            main: true,
            l: 1.2,
            c: [0.0, 0.0],
            aim: 0.0,
            u: [1.0, 0.0],
            n: [-0.0, 1.0],
            dir: 1.0,
            s: 0.6,
            dur: 0.16,
        };
        let segs = rake_segments(&r, 0.0, 0.085);
        assert_eq!(segs.len(), 6);
        assert_eq!(segs[0].0, [R, -2.5 * 0.085]);
        assert_eq!(segs[5].1, [R + 1.2, 2.5 * 0.085]);
        let sw = sweep_segments([0.0, 0.0], [1.0, 0.0], 2.0, scene::AP);
        assert_eq!(sw.len(), 6);
        assert_eq!(sw[1].0, [2.0 - 0.14, -scene::AP]);
    }
}
