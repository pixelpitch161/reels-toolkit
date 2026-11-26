//! UKULELE's look, ported close enough for the eye (ruling M4-R16) from #33's `render.py` and `styles.py`
//! (`projects/ballfight_ukulele_vs_acoustic/`): SURF'S UP's rolling low-poly wave (`wave_quads`, `draw_wave`: lit
//! water faces, foam streaks, the uke's pink on the crest, a foam lip) that lifts and lights the uke as it rides it,
//! with foam flying off the lip, flares along its path, a splash and a wet mark where it crashes, and its crest's
//! shadow under the spot; UKE-NADO's spinning funnel of water (`draw_spout`) chasing the enemy, foam off its base,
//! water trails and a shimmer round it; nylon strings, koa chips and frets in the shatter.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_core::roster::{FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::lighting::shade;
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{PictureCtx, layer_blend, screen};
use bf_picture::scene::{self, R_BALL as R};
use bf_picture::shadows::proj_point;
use bf_picture::{np, pil};

use super::state::{self, Spout, Surf};

/// The same moves with the bold effects.
#[path = "bold.rs"]
mod bold;

/// `WATER`: deep, mid, lite, foam, streak.
const DEEP: V3 = [40.0, 128.0, 170.0];
const MID: V3 = [72.0, 182.0, 212.0];
const LITE: V3 = [150.0, 230.0, 242.0];
const FOAM: V3 = [252.0, 250.0, 252.0];
const STREAK: V3 = [206.0, 244.0, 250.0];
/// `PROFILE`: the wave's cross-section, front line at x = 0, x running back, y up.
const PROFILE: [(f64, f64); 9] = [
    (1.35, 0.0),
    (0.95, 0.30),
    (0.62, 0.62),
    (0.32, 0.88),
    (0.12, 1.0),
    (-0.12, 0.92),
    (-0.08, 0.6),
    (0.02, 0.28),
    (0.06, 0.0),
];
/// `PROF_COL`: 0 deep, 1 mid, 2 lite, 3 foam.
const PROF_COL: [u8; 8] = [1, 1, 2, 3, 3, 2, 1, 0];
/// `SPOUT_H`.
const SPOUT_H: f64 = 2.5;
/// Nylon strings (shards).
const STRING_COL: [u8; 3] = [240, 236, 226];
/// `WOOD_COL` of the uke: koa top and sides.
const WOOD_TOP: V3 = [196.0, 112.0, 62.0];
const WOOD_SIDE: V3 = [120.0, 60.0, 36.0];

fn water(key: u8) -> V3 {
    match key {
        0 => DEEP,
        1 => MID,
        2 => LITE,
        3 => FOAM,
        _ => STREAK,
    }
}

fn add3(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn mul3(a: V3, k: f64) -> V3 {
    a.map(|v| v * k)
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn mean(q: &[V3]) -> V3 {
    let n = q.len() as f64;
    q.iter().fold([0.0; 3], |s, p| add3(s, *p)).map(|v| v / n)
}

fn world(q: [f64; 2], y: f64) -> V3 {
    scene::to_world(q[0], q[1], y)
}

/// `np.linspace(a, b, n)`.
fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            if i + 1 == n {
                b
            } else {
                a + (b - a) * i as f64 / (n - 1) as f64
            }
        })
        .collect()
}

/// `wave_quads(c, u, hw, Hh, t, L_back=0.95, M=7, lip)`: world quads `(Q, colour key, face id)` of a curling wave
/// whose front line passes through `c`, rolling along `u`.
fn wave_quads(c: [f64; 2], u: [f64; 2], hw: f64, hh: f64, t: f64, lip: f64) -> Vec<([V3; 4], u8, i64)> {
    let (l_back, m) = (0.95, 7);
    let n = [-u[1], u[0]];
    let grid: Vec<Vec<V3>> = linspace(-1.0, 1.0, m + 1)
        .into_iter()
        .map(|v| {
            let hj = hh * dmath::pow(py_max(0.0, 1.0 - dmath::pow(v.abs(), 3.0)), 0.6);
            let wob = 0.05 * dmath::sin(3.1 * v + t * 9.0);
            let bow = 0.28 * (1.0 - v * v);
            PROFILE
                .iter()
                .enumerate()
                .map(|(i, &(px, py))| {
                    let mut x = px * l_back - bow;
                    if (4..=6).contains(&i) {
                        x += wob - lip * 0.35 * py;
                    }
                    let q = [c[0] - u[0] * x + n[0] * v * hw, c[1] - u[1] * x + n[1] * v * hw];
                    world(q, py_max(0.0, py * hj))
                })
                .collect()
        })
        .collect();
    let mut out = Vec::new();
    for j in 0..m {
        for i in 0..PROFILE.len() - 1 {
            let q = [grid[j][i], grid[j + 1][i], grid[j + 1][i + 1], grid[j][i + 1]];
            out.push((q, PROF_COL[i], (j * 13 + i) as i64));
        }
    }
    out
}

/// `draw_wave(fr, cam, L, shade, c, u, hw, Hh, t, rim_col, lip)`: the lit low-poly wave, far faces first, and its
/// foam lip.
#[allow(clippy::too_many_arguments)] // the Python signature
fn draw_wave(fr: &mut Frame, f: &FrameCtx<'_>, c: [f64; 2], u: [f64; 2], hw: f64, hh: f64, t: f64, rim: V3, lip: f64) {
    let cam = f.cam;
    let quads = wave_quads(c, u, hw, hh, t, lip);
    let ctrs: Vec<V3> = quads.iter().map(|(q, _, _)| mean(q)).collect();
    let dep = cam.depth(&ctrs);
    let mut order: Vec<usize> = (0..quads.len()).collect();
    order.sort_by(|&a, &b| dep[b].total_cmp(&dep[a]));
    for idx in order {
        let (q, key, fid) = &quads[idx];
        let mut nrm = cross(sub3(q[1], q[0]), sub3(q[3], q[0]));
        let nn = dot3(nrm, nrm).sqrt();
        if nn < 1e-9 {
            continue;
        }
        nrm = mul3(nrm, 1.0 / nn);
        let ctr = ctrs[idx];
        if dot3(nrm, sub3(cam.pos, ctr)) < 0.0 {
            nrm = mul3(nrm, -1.0);
        }
        let jit = if *key == 3 {
            1.0
        } else {
            1.0 + 0.07 * dmath::sin(*fid as f64 * 12.9898)
        };
        let mut key = *key;
        if key == 1 && (fid * 7 + (t * 6.0) as i64).rem_euclid(5) == 0 {
            key = 4;
        }
        let base = mul3(water(key), jit);
        let lit = shade(f.l, &[nrm], &[ctr], &[base], None, 0.5, 0.0, 5.0)[0];
        let mut col = add3(mul3(lit, 0.82), mul3(water(key), 0.18));
        if key == 2 || key == 3 {
            col = add3(mul3(col, 0.85), mul3(rim, 0.15));
        }
        fr.poly(cam, q, col.map(|v| np::clip(v, 0.0, 255.0)), true);
    }
    let n = [-u[1], u[0]];
    let pts: Vec<V3> = linspace(-0.92, 0.92, 11)
        .into_iter()
        .map(|v| {
            let hj = hh * dmath::pow(py_max(0.0, 1.0 - dmath::pow(v.abs(), 3.0)), 0.6);
            let x = PROFILE[5].0 * 0.95 - 0.28 * (1.0 - v * v) + 0.05 * dmath::sin(3.1 * v + t * 9.0)
                - lip * 0.35 * PROFILE[5].1;
            let q = [c[0] - u[0] * x + n[0] * v * hw, c[1] - u[1] * x + n[1] * v * hw];
            world(q, PROFILE[5].1 * hj + 0.02)
        })
        .collect();
    fr.line(cam, &pts, rim.map(|v| np::clip(v * 0.5 + 128.0, 0.0, 255.0)), 2);
}

/// One polygon of the overlay: `(screen pts, rgb, alpha)`.
type OverlayPoly = (Vec<(f64, f64)>, V3, f64);

/// `overlay_polys(arr, polys)`: the polygons drawn onto one RGBA overlay in order, then composited once.
fn overlay_polys(arr: &mut Img<f32>, polys: &[OverlayPoly]) {
    if polys.is_empty() {
        return;
    }
    let mut ov: Img<u8> = Img::new(W, H, 4);
    for (pts, rgb, a) in polys {
        if pts.len() >= 3 {
            let c = rgb.map(|v| np::clip(v, 0.0, 255.0) as u8);
            pil::polygon(&mut ov, pts, &[c[0], c[1], c[2], (255.0 * a) as u8]);
        }
    }
    for (px, o) in arr
        .data
        .as_chunks_mut::<3>()
        .0
        .iter_mut()
        .zip(ov.data.as_chunks::<4>().0)
    {
        let k = f32::from(o[3]) / 255.0f32;
        for (v, &c) in px.iter_mut().zip(&o[..3]) {
            *v = *v * (1.0f32 - k) + f32::from(c) * k;
        }
    }
}

/// `spout_radius(y)`: a wide splash at the base, a pinched waist, flaring at the top.
fn spout_radius(y: f64) -> f64 {
    let v = y / SPOUT_H;
    0.62 * dmath::pow(1.0 - v, 3.0) + 0.22 + 0.55 * v * v
}

/// `draw_spout(arr, cam, p, t, k, rim_col)`: a spinning funnel of water, back half first, then the front half.
fn draw_spout(arr: &mut Img<f32>, cam: &Cam, p: [f64; 2], t: f64, k: f64, rim: V3) {
    if k <= 0.02 {
        return;
    }
    let ys = linspace(0.0, SPOUT_H * (0.35 + 0.65 * py_min(1.0, k * 1.4)), 12);
    let na = 20;
    let angs = linspace(0.0, 2.0 * dmath::PI, na + 1);
    let cdep = cam.depth(&[world(p, 0.8)])[0];
    let (mut back, mut front) = (Vec::new(), Vec::new());
    for i in 0..ys.len() - 1 {
        let (r0, r1) = (spout_radius(ys[i]), spout_radius(ys[i + 1]));
        let sw = t * (9.0 + 3.0 * i as f64) + i as f64 * 0.7;
        for a in 0..na {
            let (a0, a1) = (angs[a] + sw, angs[a + 1] + sw);
            let at = |r: f64, an: f64, y: f64| world([p[0] + r * dmath::cos(an), p[1] + r * dmath::sin(an)], y);
            let q = [
                at(r0, a0, ys[i]),
                at(r0, a1, ys[i]),
                at(r1, a1, ys[i + 1]),
                at(r1, a0, ys[i + 1]),
            ];
            let stripe = ((a * 2 + i * 3) / 4) % 4;
            let col = [FOAM, LITE, MID, LITE][stripe];
            let col = add3(mul3(col, 0.85), mul3(rim, 0.15));
            let al = if stripe == 0 { 0.8 } else { 0.55 }
                * py_min(1.0, k)
                * (0.6 + 0.4 * (1.0 - i as f64 / ys.len() as f64));
            let s = cam.wp(&q);
            if cam.depth(&[mean(&q)])[0] > cdep {
                back.push((s, mul3(col, 0.8), al * 0.8));
            } else {
                front.push((s, col, al));
            }
        }
    }
    back.extend(front);
    overlay_polys(arr, &back);
}

/// `e[7]` of a hit, or `""`.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// What a painter item draws.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Item {
    /// A wave (an index into `surfs`).
    Wave(usize),
    /// A waterspout (an index into `spouts`).
    Spout(usize),
}

/// UKULELE's look.
#[derive(Clone, Debug, Default)]
pub struct UkuleleLook {
    /// The roster numbers its bold moves read, from `setup_as`'s `cfg`.
    nums: bold::Nums,
    k: usize,
    surfs: Vec<Surf>,
    spouts: Vec<Spout>,
    path: Vec<[f64; 4]>,
    /// `V`, `KNOCK_T` as the recording has them, through [`state::dist`].
    rec: Option<std::sync::Arc<bf_core::recording::Recording>>,
}

impl UkuleleLook {
    fn ta(p: &PictureCtx, t: f64) -> f64 {
        py_min(t, p.t_ko)
    }

    fn show(p: &PictureCtx, t: f64) -> bool {
        p.show.grid.t_fight <= t && t < p.t_hs + 0.12
    }

    fn dist(&self, tau: f64, power: f64) -> f64 {
        self.rec.as_ref().map_or(0.0, |r| state::dist(r, tau, power))
    }

    /// `surf_height(w, t)`: rises over 0.12 s, rolls, collapses over 0.25 s after it ends.
    fn height(w: &Surf, t: f64) -> f64 {
        if t < w.t0 {
            return 0.0;
        }
        (0.32 + 0.45 * w.h) * ss(w.t0, w.t0 + 0.12, t) * (1.0 - ss(w.t_end, w.t_end + 0.25, t))
    }

    /// `surf_front(w, t)`.
    fn front(&self, w: &Surf, t: f64) -> [f64; 2] {
        let tau = py_min(t, w.t_end) - w.t0;
        let d = self.dist(py_max(0.0, tau), w.power);
        let mut c = [w.o[0] + w.u[0] * d, w.o[1] + w.u[1] * d];
        if t > w.t_end {
            let s = 0.6 * ss(w.t_end, w.t_end + 0.25, t);
            c = [c[0] + w.u[0] * s, c[1] + w.u[1] * s];
        }
        c
    }

    /// `spout_at(t)`: the index, place and strength of the live waterspout.
    fn spout_at(&self, t: f64) -> Option<(usize, [f64; 2], f64)> {
        let (i, sp) = self
            .spouts
            .iter()
            .enumerate()
            .find(|(_, sp)| sp.t0 <= t && t < sp.t_end + 0.4)?;
        let p = state::spout_pos(sp, &self.path, t);
        let k = ss(sp.t0, sp.t0 + 0.25, t) * (1.0 - ss(sp.t_end, sp.t_end + 0.4, t));
        Some((i, p, k))
    }
}

impl Look for UkuleleLook {
    fn shards(&self) -> Vec<String> {
        [("string", 4), ("top", 6), ("side", 2), ("fret", 3)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &[]
    }

    fn flash_hits(&self) -> &[&str] {
        &[]
    }

    fn fringe_hits(&self) -> &[&str] {
        &["surf", "spout"]
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[]
    }

    /// The little brother: the uke sits smaller in its ball.
    fn sprite_k(&self) -> f64 {
        0.70
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.nums = bold::Nums::of(cfg);
        let life = match &cfg.weapon {
            WeaponCfg::Surf(w) => w.life,
            _ => 1.1,
        };
        self.surfs = state::surfs(&p.rec, k, life, p.t_ko);
        self.spouts = state::spouts(&p.rec, k, p.t_ko);
        self.path = state::spout_path(&p.rec)
            .into_iter()
            .filter(|q| q[1] as usize == k)
            .collect();
        self.rec = Some(p.rec.clone());
    }

    /// The uke rides its wave: lifted on the crest, glowing.
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if p.bold {
            return self.bold_state(p, kk, st, t, phase);
        }
        if phase != Phase::Fight || kk != self.k {
            return;
        }
        let tc = py_min(t, p.t_ko);
        for w in &self.surfs {
            if w.t0 <= tc && tc < w.t_end + 0.25 {
                let ride = 1.0 - ss(w.t0 + 0.55, w.t0 + 0.9, tc);
                st.lift = py_max(st.lift, 0.45 * Self::height(w, tc) * ride);
                st.glow = py_max(st.glow, 0.45 * dmath::exp(-(tc - w.t0) / 0.14));
                st.glow_np = false;
            }
        }
    }

    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        match how(e) {
            "surf" => Some(1.2),
            "spout" => Some(1.0),
            "shockwave" => Some(1.4),
            _ => DefaultLook.shake(p, e),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "surf" => Some((10.0, 0.3)),
            "spout" => Some((9.0, 0.26)),
            _ => None,
        }
    }

    /// SURF'S UP: the crest lights the ring as it rolls, foam flies off the lip, the crash leaves water on the
    /// canvas. UKE-NADO: foam spins off its base, water trails on the canvas.
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let k = self.k;
        for w in &self.surfs {
            let dur = w.t_end - w.t0;
            let hh = 0.32 + 0.45 * w.h;
            let n = [-w.u[1], w.u[0]];
            let mut tau = 0.0;
            while tau <= dur {
                let d = self.dist(tau, w.power);
                let c = [w.o[0] + w.u[0] * d, w.o[1] + w.u[1] * d];
                for side in [-0.8, 0.0, 0.8] {
                    let q = [c[0] + n[0] * side * w.hw, c[1] + n[1] * side * w.hw];
                    let emit = Emit {
                        n: Some(1),
                        dir: [w.u[0], 0.6, w.u[1]],
                        vel: scene::to_world(w.u[0] * 1.2, w.u[1] * 1.2, 0.0),
                        speed: Some((0.4, 1.0)),
                        ..Emit::default()
                    };
                    fx.pa.emit(w.t0 + tau, world(q, hh * 0.9), "foam", &emit);
                }
                if tau == 0.0 || (tau - 0.3).abs() < 0.05 {
                    fx.fl
                        .add(w.t0 + tau, world(c, 0.8), p.col[k], 1.6, 0.45, 0.03, 0.22, 0.0);
                }
                tau += 0.08;
            }
            let d = self.dist(dur, w.power);
            let cend = [w.o[0] + w.u[0] * d, w.o[1] + w.u[1] * d];
            if w.crashed {
                let emit = Emit {
                    n: Some(14),
                    dir: [w.u[0], 1.0, w.u[1]],
                    speed: Some((0.8, 2.2)),
                    spread: Some(80.0),
                    ..Emit::default()
                };
                fx.pa.emit(w.t_end, world(cend, hh * 0.7), "foam", &emit);
            }
            if w.t_end < p.t_ko {
                fx.mk.add(
                    w.t_end,
                    Mark {
                        size: 0.3 + 0.3 * w.hw,
                        life: 2.5,
                        seed: (w.t0 * 97.0) as u64,
                        strength: 0.6,
                        ..Mark::new(MarkKind::Water, cend[0], cend[1])
                    },
                );
            }
        }
        for sp in &self.spouts {
            let (mut tt, mut n) = (sp.t0, 0u64);
            while tt < sp.t_end {
                let q = state::spout_pos(sp, &self.path, tt);
                let emit = Emit {
                    n: Some(2),
                    spread: Some(180.0),
                    speed: Some((0.6, 1.6)),
                    ..Emit::default()
                };
                fx.pa.emit(tt, world(q, 0.1), "foam", &emit);
                if n % 4 == 0 {
                    fx.mk.add(
                        tt,
                        Mark {
                            size: 0.4,
                            life: 2.5,
                            seed: n + 500,
                            strength: 0.6,
                            ..Mark::new(MarkKind::Water, q[0], q[1])
                        },
                    );
                }
                if n % 5 == 0 {
                    fx.fl.add(tt, world(q, 1.2), p.col[sp.owner], 1.6, 0.4, 0.03, 0.3, 0.0);
                }
                tt += 0.06;
                n += 1;
            }
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind != "hit" {
            return;
        }
        let Some(att) = e.who().filter(|&a| a == self.k) else {
            return;
        };
        let (t, x, z) = (e.t, e.f(5), e.f(6));
        let pos = scene::to_world(x, z, R);
        match how(e) {
            "surf" => {
                fx.bu.add(t, pos, p.col[att], 1.5, None, true);
                fx.fl.add(t, pos, p.col[att], 1.8, 0.85, 0.03, 0.28, 0.0);
                let emit = Emit {
                    n: Some(14),
                    speed: Some((1.0, 2.4)),
                    spread: Some(110.0),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "foam", &emit);
                fx.mk.add(
                    t,
                    Mark {
                        size: 0.45,
                        life: 2.5,
                        seed: (t * 100.0) as u64,
                        strength: 0.7,
                        ..Mark::new(MarkKind::Water, x, z)
                    },
                );
            }
            "spout" => {
                fx.bu.add(t, pos, p.col[att], 1.1, None, true);
                fx.fl.add(t, pos, p.col[att], 1.8, 0.8, 0.03, 0.3, 0.0);
                let emit = Emit {
                    n: Some(12),
                    speed: Some((1.0, 2.6)),
                    spread: Some(150.0),
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "foam", &emit);
            }
            _ => {}
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

    /// The waves' crests cast the spot's shadow too.
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        let mut out = Vec::new();
        for w in self.surfs.iter().filter(|w| w.t0 <= f.t && f.t < w.t_end + 0.25) {
            let hh = Self::height(w, f.t);
            if hh > 0.05 && Self::show(p, f.t) {
                let c = self.front(w, f.t);
                for (q, key, _) in wave_quads(c, w.u, w.hw, hh, 0.0, 0.0) {
                    if key >= 1 {
                        out.push(proj_point(&q, spot, 0.93));
                    }
                }
            }
        }
        out
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if p.bold {
            return self.bold_items_free(p, f);
        }
        if !Self::show(p, f.t) {
            return Vec::new();
        }
        let ta = Self::ta(p, f.t);
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for (i, w) in self.surfs.iter().enumerate() {
            if w.t0 <= ta && ta < w.t_end + 0.25 {
                let c = self.front(w, ta);
                let at = scene::to_world(c[0] - w.u[0] * 0.3, c[1] - w.u[1] * 0.3, 0.3);
                out.push((f.cam.depth(&[at])[0], Box::new(Item::Wave(i))));
            }
        }
        if let Some((i, q, _)) = self.spout_at(ta) {
            out.push((f.cam.depth(&[world(q, 0.6)])[0], Box::new(Item::Spout(i))));
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        if p.bold {
            return self.bold_draw_item(p, f, fr, payload);
        }
        let Some(item) = payload.downcast_ref::<Item>() else {
            return;
        };
        let ta = Self::ta(p, f.t);
        match *item {
            Item::Wave(i) => {
                let w = &self.surfs[i];
                let hh = Self::height(w, if f.t < p.t_hs { ta } else { p.t_ko });
                if hh > 0.03 {
                    let c = self.front(w, ta);
                    let lip = ss(w.t_end, w.t_end + 0.25, ta);
                    draw_wave(fr, f, c, w.u, w.hw + 0.3 * R, hh, ta, p.col[w.owner], lip);
                }
            }
            Item::Spout(_) => {
                if let Some((_, q, k)) = self.spout_at(ta) {
                    let (mut arr, depth) = (fr.array(), fr.depth());
                    draw_spout(&mut arr, f.cam, q, f.t, k, p.col[self.k]);
                    fr.set_array(&arr, Some(&depth));
                }
            }
        }
    }

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
        match kind {
            "string" => {
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
                let bytes = arr.data.iter().map(|&v| np::clip_u8_f32(v)).collect();
                let mut im: Img<u8> = Img::from_vec(arr.w, arr.h, arr.c, bytes);
                pil::line(&mut im, &sp, &STRING_COL, 1);
                for (v, &b) in arr.data.iter_mut().zip(&im.data) {
                    *v = f32::from(b);
                }
            }
            "fret" => {
                let (lk, wk) = (0.30, 0.035);
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
            }
            "top" | "side" => {
                let (col, s) = if kind == "top" {
                    (WOOD_TOP, size * 1.3)
                } else {
                    (WOOD_SIDE, size)
                };
                let tri = [
                    [sx + d[0] * s, sz + d[1] * s],
                    [
                        sx - d[0] * s * 0.7 + nn[0] * s * 0.8,
                        sz - d[1] * s * 0.7 + nn[1] * s * 0.8,
                    ],
                    [
                        sx - d[0] * s * 0.4 - nn[0] * s * 0.7,
                        sz - d[1] * s * 0.4 - nn[1] * s * 0.7,
                    ],
                ];
                let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
                layer_blend(
                    arr,
                    &[on_screen(&tri)],
                    col.map(|c| np::clip(c * shade_k, 0.0, 255.0)),
                    1.0,
                    None,
                );
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ the bold effects
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if p.bold {
            self.bold_canvas(p, ic, c);
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if p.bold {
            self.bold_canvas_wave(p, ic, c, w);
        }
    }

    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if p.bold {
            self.bold_after_grade(p, img, f, drawn);
        }
    }

    /// The waterspout ripples the air round it.
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        if !Self::show(p, t) {
            return;
        }
        if let Some((_, q, k)) = self.spout_at(Self::ta(p, t)) {
            img.shimmer_world(cam, world(q, 0.05), 0.9, 0.3, 1.8, t, k, 0.0);
        }
    }

    /// The wave's front while one rolls.
    fn focus_points(&self, p: &PictureCtx, t: f64, _st: &BallState) -> Vec<(V3, String)> {
        if !Self::show(p, t) {
            return Vec::new();
        }
        let ta = Self::ta(p, t);
        self.surfs
            .iter()
            .filter(|w| w.t0 <= ta && ta < w.t_end)
            .map(|w| (world(self.front(w, ta), 0.3), "surf".to_string()))
            .collect()
    }
}
