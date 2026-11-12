//! EUPHONIUM's look: the `Look` half of `kit/fighters/euphonium.py` and its module helpers (`shell_pos`,
//! `draw_shell`, `target_light`, `splash_light`, `swat_pos`, `draw_swatted`), from #38's `styles.py` and #48.
//!
//! The shells on real arcs with smoke trails, the growing shadow circles and splash-radius rings, the amber
//! splashes, smoke out of the bell on every salvo, scorches only where a shell hits, IDENTITY CRISIS's flare and
//! motes and the amber stranger in its ball (the enemy's model, tinted), the shells a lance swats tumbling away, and
//! brass, valve and bell shards.
//!
//! IDENTITY CRISIS's copies are drawn by the enemy's own look (`dup`): the enemy's `Look` made for the euphonium's
//! side with the enemy's roster numbers ([`Look::setup_as`] with the enemy's entry, as `euphonium.py` 192 to 201
//! does), for the copied kinds only. Its hooks are forwarded where the kit forwards them and nowhere else.
//!
//! Its own roster numbers come from the `cfg` [`Look::setup_as`] is given: the weapon's and NOT A TUBA's `splash`.
//!
//! numpy's dtypes, kept: the frame of `over_ring` is `float32`, every mask `float32`, every alpha a Python float;
//! a pixel mixed in place is [`mix`]; the canvas light is `float64`. The amber stranger's tint is `lit * 0.55 +
//! col * 0.55` with `lit` the `float32` lit sprite and `col` a `float64` colour: a `float32` product, then a `float64`
//! sum, rounded to `float32` once (where the kit stores it in the frame; the euphonium's ball is never sunk, so
//! nothing comes between).

use std::sync::Arc;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{self, py_max, py_min};
use bf_core::roster::{AbilityKind, FighterCfg, WeaponCfg};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::{H, W, bayer_full, ss};
use bf_picture::fx::{Emit, Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{BallSprite, CanvasCtx, FrameCtx, Fx, Look, Phase, Pic, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::{R_BALL as R, to_world};

use super::state::{self, Land, Salvo};
use crate::fighters::shared::reeds::{self, Num, Px};

/// `INK`.
const INK: V3 = [18.0, 10.0, 26.0];
/// `WHITE`.
const WHITE: V3 = [255.0, 252.0, 240.0];
/// `BRASS`.
const BRASS: V3 = [236.0, 186.0, 84.0];
/// `BRASS_HI`.
const BRASS_HI: V3 = [255.0, 240.0, 190.0];
/// `SMOKE`.
const SMOKE: V3 = [196.0, 188.0, 176.0];
/// `SMOKE_C`: the bell's smoke.
const SMOKE_C: [V3; 3] = [[150.0, 144.0, 136.0], [190.0, 182.0, 170.0], [120.0, 116.0, 112.0]];
/// `BRASS_SPARK`.
const BRASS_SPARK: [V3; 3] = [[255.0, 214.0, 120.0], [240.0, 172.0, 32.0], [255.0, 246.0, 210.0]];
/// `SLATE_SPARK`.
const SLATE_SPARK: [V3; 3] = [[196.0, 210.0, 236.0], [110.0, 132.0, 170.0], [240.0, 244.0, 255.0]];
/// `SWAT_T`: seconds a swatted shell tumbles away before it is gone.
const SWAT_T: f64 = 0.30;
/// The kinds of its own the copy's look never draws (`own` in `setup`).
const OWN: [&str; 4] = ["mortar", "barrage", "haste", "copy"];

/// `e[7]` of a hit (what hit), or `""` for an event without one.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// `e[2]` as a side, for `P.COL[e[2]]`: the event's fighter.
fn owner(e: &Event) -> usize {
    e.who().filter(|&k| k < 2).unwrap_or(0)
}

/// `img[y, x] = img[y, x] * keep + add` on one pixel: `keep` a Python float (a `float32` pixel is multiplied in
/// `float32`), `add` a `float64` colour term, the sum stored in the picture's dtype.
fn mix<T: Px>(px: &mut [T], keep: f64, add: V3) {
    for (v, a) in px.iter_mut().zip(add) {
        *v = T::set(v.times(keep) + a);
    }
}

/// `img[y, x] = col`: a `float64` colour stored in the picture's dtype.
fn put<T: Px>(px: &mut [T], col: V3) {
    for (v, c) in px.iter_mut().zip(col) {
        *v = T::set(c);
    }
}

/// `wp(cam, x, z, y)`: ring-frame points to the screen (`cam.wp(S.to_world(x, z, y))`).
fn wp(cam: &Cam, pts: &[V3]) -> Vec<(f64, f64)> {
    let world: Vec<V3> = pts.iter().map(|q| to_world(q[0], q[1], q[2])).collect();
    cam.wp(&world)
}

/// `np.clip(BRASS * 0.85 + col * 0.15, 0, 255)`: a shell's body.
fn body(col: V3) -> V3 {
    std::array::from_fn(|c| np::clip(BRASS[c] * 0.85 + col[c] * 0.15, 0.0, 255.0))
}

/// A Python float (or numpy scalar) squared with `**` (ruling M3-R38's rule for ports).
fn sq(x: f64) -> f64 {
    dmath::pow(x, 2.0)
}

// ---------------------------------------------------------------- the shells, from the recorded SHELLS
/// A row of `RUN['SHELLS'][i]`: `(owner, x0, z0, x1, z1, t0, t1, salvo, j, landed[, ts])`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ShellRow {
    owner: usize,
    x0: f64,
    z0: f64,
    x1: f64,
    z1: f64,
    t0: f64,
    t1: f64,
    salvo: i64,
    j: i64,
    /// `sh[10]`: when a lance swatted it (`len(sh) > 10`).
    ts: Option<f64>,
}

impl ShellRow {
    /// One recorded row, or `None` for a row that is not one.
    fn of(v: &Value) -> Option<ShellRow> {
        let Value::List(row) = v else { return None };
        let f = |i: usize| match row.get(i) {
            Some(Value::Float(x)) => Some(*x),
            Some(Value::Int(n)) => Some(*n as f64),
            _ => None,
        };
        let n = |i: usize| match row.get(i) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        };
        Some(ShellRow {
            owner: usize::try_from(n(0)?).ok()?,
            x0: f(1)?,
            z0: f(2)?,
            x1: f(3)?,
            z1: f(4)?,
            t0: f(5)?,
            t1: f(6)?,
            salvo: n(7)?,
            j: n(8)?,
            ts: if row.len() > 10 { f(10) } else { None },
        })
    }
}

/// `shell_pos(sh, t, R)`: a shell's ring-frame position `(x, z, y)` at `t` and how far along it is: a straight line
/// over the canvas, a parabola up (the apex grows with the range), launched from the top of the euphonium's bell.
fn shell_pos(sh: &ShellRow, t: f64) -> (f64, f64, f64, f64) {
    let s = py_min(1.0, py_max(0.0, (t - sh.t0) / py_max(1e-6, sh.t1 - sh.t0)));
    let (x, z) = (sh.x0 + (sh.x1 - sh.x0) * s, sh.z0 + (sh.z1 - sh.z0) * s);
    let rng = dmath::py_hypot(sh.x1 - sh.x0, sh.z1 - sh.z0);
    let apex = 2.2 + 0.35 * rng;
    let y = R * 1.7 * (1.0 - s) + 0.05 * s + 4.0 * apex * s * (1.0 - s);
    (x, z, y, s)
}

/// `draw_shell(img, cam, sh, t, R, col)`: a brass shell in flight: an amber slug with a hot highlight and a short grey
/// smoke trail.
fn draw_shell(img: &mut Img<f32>, cam: &Cam, sh: &ShellRow, t: f64, col: V3) {
    let (x, z, y, s) = shell_pos(sh, t);
    if s >= 1.0 {
        return;
    }
    let pts: Vec<V3> = [0.0, 0.05, 0.10, 0.15]
        .iter()
        .map(|&dt| {
            let q = shell_pos(sh, t - dt);
            [q.0, q.1, q.2]
        })
        .collect();
    let sp = wp(cam, &pts);
    let trail = reeds::mask(&[], std::slice::from_ref(&sp), 1);
    reeds::blend(img, &trail, SMOKE, Num::Py(0.45));
    let c = sp[0];
    let rp = py_max(1.4, 0.10 * cam.f / py_max(1e-6, cam.depth(&[to_world(x, z, y)])[0]));
    // xi0, xi1 = int(c[0] - rp - 1), int(c[0] + rp + 2)
    let (xi0, xi1) = ((c.0 - rp - 1.0) as i64, (c.0 + rp + 2.0) as i64);
    let (yi0, yi1) = ((c.1 - rp - 1.0) as i64, (c.1 + rp + 2.0) as i64);
    let b = body(col);
    for yi in yi0.max(0)..yi1.min(H as i64) {
        for xi in xi0.max(0)..xi1.min(W as i64) {
            let (dx, dy) = (xi as f64 + 0.5 - c.0, yi as f64 + 0.5 - c.1);
            let dd = sq(dx / (rp * 1.25)) + sq(dy / rp);
            let px = img.px_mut(xi as usize, yi as usize);
            if dd <= 1.0 {
                let hi = dx < -0.2 * rp && dy < -0.1 * rp;
                put(px, if hi { BRASS_HI } else { b });
            } else if dd <= 1.6 {
                mix(px, 0.4, INK.map(|v| v * 0.6));
            }
        }
    }
}

/// `target_light(sxp, szp, sh, t, splash)` at one canvas pixel: `(shade, ring)`, the shadow circle growing and
/// darkening as the shell falls, and a thin ring in the owner's colour at the splash radius; `None` outside the
/// flight.
fn target_light(sx: f64, sz: f64, sh: &ShellRow, s: f64, splash: f64) -> (f64, f64) {
    let d = np::hypot(sx - sh.x1, sz - sh.z1);
    let rr = 0.12 + (splash * 0.85 - 0.12) * s;
    let shade = dmath::pow(np::clip(1.0 - d / rr, 0.0, 1.0), 0.6) * (0.25 + 0.6 * s);
    let ring = np::clip(1.0 - (d - splash).abs() / 0.035, 0.0, 1.0) * (0.3 + 0.6 * s);
    (shade, ring)
}

/// `splash_light(sxp, szp, x, z, age, splash)` at one canvas pixel (`age` in 0..0.4): a landed shell's splash, a ring
/// racing out to the splash radius, fading over 0.4 s.
fn splash_light(sx: f64, sz: f64, x: f64, z: f64, age: f64, splash: f64) -> f64 {
    let d = np::hypot(sx - x, sz - z);
    let r = splash * dmath::pow(py_min(1.0, age / 0.12), 0.5);
    np::clip(1.0 - (d - r).abs() / 0.10, 0.0, 1.0) * (1.0 - age / 0.4)
}

/// `swat_pos(sh, t, R)`: a swatted shell knocked off its arc: from where the lance met it, it flies on away from the
/// lance, up a little and falling, spinning out over [`SWAT_T`].
fn swat_pos(sh: &ShellRow, ts: f64, t: f64) -> (f64, f64, f64) {
    let (x, z, y, _) = shell_pos(sh, ts);
    let d = [sh.x1 - sh.x0, sh.z1 - sh.z0];
    let n = np::hypot(d[0], d[1]) + 1e-9;
    let d = [d[0] / n, d[1] / n];
    let u = t - ts;
    // (1 if sh[8] % 2 else -1): knocked sideways off its line
    let sgn = if py::imod_py(sh.j, 2) != 0 { 1.0 } else { -1.0 };
    let side = [-d[1] * sgn, d[0] * sgn];
    let px = x + (side[0] * 2.4 - d[0] * 0.8) * u;
    let pz = z + (side[1] * 2.4 - d[1] * 0.8) * u;
    (px, pz, y + 2.2 * u - 9.0 * u * u)
}

/// `draw_swatted(img, cam, sh, t, R, col)`: a swatted shell spinning away off its arc: the brass slug tumbling end over
/// end, a bright glint each half turn, a short spark trail back to where the lance met it.
fn draw_swatted(img: &mut Img<f32>, cam: &Cam, sh: &ShellRow, ts: f64, t: f64, col: V3) {
    let u = t - ts;
    let (x, z, y) = swat_pos(sh, ts, t);
    if y <= 0.02 {
        return;
    }
    let pts: Vec<V3> = [0.0, 0.04, 0.08]
        .iter()
        .map(|&dt| {
            let q = swat_pos(sh, ts, py_max(ts, t - dt));
            [q.0, q.1, q.2]
        })
        .collect();
    let sp = wp(cam, &pts);
    let trail = reeds::mask(&[], std::slice::from_ref(&sp), 1);
    reeds::blend(img, &trail, BRASS_HI, Num::Py(0.6 * (1.0 - u / SWAT_T)));
    let c = sp[0];
    let rp = py_max(1.6, 0.11 * cam.f / py_max(1e-6, cam.depth(&[to_world(x, z, y)])[0]));
    // tumbling: about 6 turns a second
    let a = u * 38.0;
    let (ca, sa) = (dmath::cos(a), dmath::sin(a));
    let glint = dmath::cos(2.0 * a) > 0.6;
    let b = body(col);
    let (y0, y1) = ((c.1 - rp - 2.0) as i64, (c.1 + rp + 3.0) as i64);
    let (x0, x1) = ((c.0 - rp - 2.0) as i64, (c.0 + rp + 3.0) as i64);
    for yi in y0.max(0)..y1.min(H as i64) {
        for xi in x0.max(0)..x1.min(W as i64) {
            let (dx, dy) = (xi as f64 + 0.5 - c.0, yi as f64 + 0.5 - c.1);
            let (ex, ey) = (dx * ca + dy * sa, -dx * sa + dy * ca);
            let dd = sq(ex / (rp * 1.35)) + sq(ey / (rp * 0.8));
            let px = img.px_mut(xi as usize, yi as usize);
            if dd <= 1.0 {
                let c = if glint && ex < 0.0 {
                    WHITE
                } else if ex < -0.3 * rp {
                    BRASS_HI
                } else {
                    b
                };
                put(px, c);
            } else if dd <= 1.7 {
                mix(px, 0.4, INK.map(|v| v * 0.6));
            }
        }
    }
}

// ---------------------------------------------------------------- the look
/// One of its IDENTITY CRISIS copies: `(t, name, kind)` of a `copy` event of its own.
#[derive(Clone, Debug, PartialEq)]
struct Copy {
    t: f64,
    kind: String,
}

/// The EUPHONIUM's look: `Look` of `kit/fighters/euphonium.py`.
#[derive(Default)]
pub struct EuphoniumLook {
    /// Its side.
    k: usize,
    /// `SPLASH['mortar']` (`None` for a weapon without one: Python raises where it reads it).
    splash_mortar: Option<f64>,
    /// `SPLASH['barrage']`.
    splash_barrage: Option<f64>,
    /// `SALVOS`.
    salvos: Vec<Salvo>,
    /// `LANDS`.
    lands: Vec<Land>,
    /// `BARRAGE_SALVOS`.
    barrage_salvos: Vec<i64>,
    /// `COPIES`.
    copies: Vec<Copy>,
    /// `SWATS`: `(t, lance owner, x, z)` of the swats of its shells.
    swats: Vec<(f64, usize, f64, f64)>,
    /// `self.dup`: the enemy's look made for this side with the enemy's numbers, when it copied a kind that look
    /// draws.
    dup: Option<Box<dyn Look>>,
    /// The enemy's view roll (`P.CFG[e]['view'][2]`): the amber stranger's.
    enemy_roll: f64,
    /// `self.cfg['view']`: its own view's yaw centre and swing (for `extra_sprites`).
    view: (f64, f64),
    /// `self.cfg['view']` as `cfg_summary` names it.
    view_text: String,
}

impl std::fmt::Debug for EuphoniumLook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EuphoniumLook")
            .field("k", &self.k)
            .field("copies", &self.copies)
            .field("dup", &self.dup.is_some())
            .finish()
    }
}

impl EuphoniumLook {
    /// `self.SPLASH[kind]`.
    fn splash(&self, kind: &str) -> f64 {
        let s = if kind == "barrage" {
            self.splash_barrage
        } else {
            self.splash_mortar
        };
        s.expect("the shells' splash reads the weapon's `splash` (Python raises KeyError)")
    }

    /// `_dup_kind(how)`: a hit the copy's look handles (any but a shell's).
    fn dup_kind(&self, how: &str) -> bool {
        self.dup.is_some() && how != "shell"
    }

    /// `crisis_at(t)`: for a bar after a copy the euphonium's ball shows the enemy's instrument, tinted (0..1).
    fn crisis_at(&self, p: &PictureCtx, t: f64) -> f64 {
        let bar = p.show.grid.bar;
        for c in &self.copies {
            if c.t <= t && t < c.t + bar && c.t < p.t_ko {
                return py_min(1.0, (t - c.t) / 0.08) * (1.0 - ss(c.t + bar - 0.3, c.t + bar, t));
            }
        }
        0.0
    }

    /// `_top(t_)`: the top of its bell at `t_` (in the world).
    fn top(&self, p: &PictureCtx, t: f64) -> V3 {
        let q = p.run.p(p.sim_i(t), self.k);
        to_world(q[0], q[1], R * 1.7)
    }

    /// `live_shells(t)`: its shells showing at `t` (in flight or landed less than 0.6 s ago), from the recorded
    /// `SHELLS`.
    fn live_shells(&self, p: &PictureCtx, t: f64) -> Vec<ShellRow> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has("SHELLS") {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        match p.run.nested("SHELLS", i) {
            Some(Value::List(rows)) => rows
                .iter()
                .filter_map(ShellRow::of)
                .filter(|sh| sh.owner == self.k)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// `recent_lands(t, span=0.4)`: its landings of the last 0.4 s (up to the K.O.).
    fn recent_lands(&self, p: &PictureCtx, t: f64) -> impl Iterator<Item = &Land> {
        // np.searchsorted(LAND_T, t - span), np.searchsorted(LAND_T, t, 'right')
        let i0 = self.lands.partition_point(|l| l.t < t - 0.4);
        let i1 = self.lands.partition_point(|l| l.t <= t);
        let t_ko = p.t_ko;
        self.lands[i0..i1.max(i0)].iter().filter(move |l| l.t <= t_ko)
    }
}

impl Look for EuphoniumLook {
    fn shards(&self) -> Vec<String> {
        // ['brass'] * 8 + ['valve'] * 4 + ['bell'] * 2
        [("brass", 8), ("valve", 4), ("bell", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn spark_big(&self) -> &[&str] {
        &[]
    }

    /// `flash_hits`: the copy's look's (set in `setup`), else none.
    fn flash_hits(&self) -> &[&str] {
        self.dup.as_ref().map_or(&[], |d| d.flash_hits())
    }

    /// `fringe_hits`: the copy's look's (set in `setup`), else none.
    fn fringe_hits(&self) -> &[&str] {
        self.dup.as_ref().map_or(&[], |d| d.fringe_hits())
    }

    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("barrage", 0.5)]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    /// What `setup_as` took from `cfg`, by the keys `euphonium.py` reads from `self.cfg`.
    fn cfg_summary(&self) -> Option<String> {
        Some(format!("view={}", self.view_text))
    }

    /// `Look(P, k)` with `self.cfg = cfg`, then `setup()`: the roster numbers from `cfg`, the events of side `k`, and
    /// the copy's look when it copied a kind that look draws.
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        let ev = &p.rec.events;
        let mine = |e: &&Event, kind: &str| e.kind == kind && e.who() == Some(k);
        // SPLASH = {'mortar': w['splash'], 'barrage': low['splash'] if low['kind'] == 'barrage' else w['splash']}
        self.splash_mortar = match &cfg.weapon {
            WeaponCfg::Mortar(w) => Some(w.splash),
            _ => None,
        };
        self.splash_barrage = if cfg.low.kind == AbilityKind::Barrage {
            cfg.low.get("splash")
        } else {
            self.splash_mortar
        };
        self.salvos = state::salvos(ev, k);
        self.lands = state::lands(ev, k);
        self.barrage_salvos = ev.iter().filter(|e| mine(e, "barrage")).map(|e| e.i(4)).collect();
        self.copies = ev
            .iter()
            .filter(|e| mine(e, "copy"))
            .map(|e| Copy {
                t: e.t,
                kind: e.s(4).to_string(),
            })
            .collect();
        self.swats = ev
            .iter()
            .filter(|e| e.kind == "swat_shell" && e.fields.get(1) == Some(&Value::Int(k as i64)))
            .map(|e| (e.t, owner(e), e.f(6), e.f(7)))
            .collect();
        let view = &cfg.raw["view"];
        let num = |i: usize| view.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
        self.view = (num(0), num(1));
        self.view_text = view.to_string();
        // IDENTITY CRISIS: the copied kinds the enemy's own look draws (for this side, in amber)
        let copied = self
            .copies
            .iter()
            .any(|c| c.t < p.t_ko && !OWN.contains(&c.kind.as_str()));
        let e = 1 - k;
        let enemy = &p.cfg[e];
        self.enemy_roll = enemy.raw["view"].get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);
        self.dup = None;
        if copied {
            // the copied abilities' numbers are the enemy's
            let mut dup = crate::fighters::look_for(&enemy.name);
            dup.setup_as(p, k, enemy);
            self.dup = Some(dup);
        }
    }

    // ---------------------------------------------------------------- IDENTITY CRISIS in the ball
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, _gl: f64) {
        if kk != self.k {
            return;
        }
        st.extra.insert("crisis".to_string(), 0.0);
        if phase == Phase::Fight {
            let crisis = self.crisis_at(p, t);
            st.extra.insert("crisis".to_string(), crisis);
            if crisis > 0.0 {
                st.glow = py_max(st.glow, 0.5 * crisis);
            }
        }
    }

    fn ball_sprite<'a>(
        &'a self,
        p: &'a PictureCtx,
        _kk: usize,
        st: &BallState,
        _t: f64,
        rp: f64,
        yaw: f64,
    ) -> Option<BallSprite<'a>> {
        if st.extra.get("crisis").copied().unwrap_or(0.0) <= 0.5 {
            return None;
        }
        let col = p.col[self.k];
        let sprite: Arc<Img<u8>> = p.sprite(1 - self.k, rp * 0.80, yaw, self.enemy_roll);
        // lambda lit: lit * 0.55 + col[None, None] * 0.55: an amber stranger (who am I?)
        let tint = move |lit: &mut Img<f32>| {
            for px in lit.data.as_chunks_mut::<3>().0 {
                for (v, c) in px.iter_mut().zip(col) {
                    *v = (f64::from(*v * 0.55f32) + c * 0.55) as f32;
                }
            }
        };
        Some(BallSprite {
            sprite,
            tint: Some(Box::new(tint)),
        })
    }

    fn extra_sprites(&self) -> Vec<(usize, f64, f64, f64)> {
        if self.copies.is_empty() {
            return Vec::new();
        }
        vec![(1 - self.k, self.view.0, self.view.1, self.enemy_roll)]
    }

    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        self.dup.as_ref().is_some_and(|d| d.hides_hp(p, kk, t))
    }

    // ---------------------------------------------------------------- events
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "hit" && how(e) == "shell" {
            return Some(0.5);
        }
        if let Some(d) = &self.dup
            && (e.kind != "hit" || self.dup_kind(how(e)))
        {
            return d.shake(p, e);
        }
        if e.kind == "slam" { Some(1.6) } else { None }
    }

    fn rope(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "copy"
            && let Some(d) = &self.dup
        {
            // self.dup.rope_kinds.get(e[4])
            let kind = e.s(4);
            return d.rope_kinds().iter().find(|(n, _)| *n == kind).map(|&(_, k)| k);
        }
        if e.kind == "ability" {
            let kind = e.s(4);
            return self.rope_kinds().iter().find(|(n, _)| *n == kind).map(|&(_, k)| k);
        }
        let _ = p;
        if e.kind == "slam" { Some(0.9) } else { None }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        if how(e) == "shell" {
            return (p.col[owner(e)].map(|c| c * 0.6 + 100.0), 7, 2.2, 0.22);
        }
        if let Some(d) = self.dup.as_ref().filter(|_| self.dup_kind(how(e))) {
            return d.spark(p, e);
        }
        (p.col[owner(e)].map(|c| c * 0.6 + 100.0), 5, 1.8, 0.18)
    }

    fn hit_glow(&self, p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        if how(e) == "shell" {
            return Some((8.0, 0.2));
        }
        match self.dup.as_ref().filter(|_| self.dup_kind(how(e))) {
            Some(d) => d.hit_glow(p, e),
            None => None,
        }
    }

    fn fringe(&self, _p: &PictureCtx, e: &Event) -> bool {
        self.fringe_hits().contains(&how(e))
    }

    fn fringe_size(&self, p: &PictureCtx, e: &Event) -> (f64, f64) {
        match &self.dup {
            Some(d) => d.fringe_size(p, e),
            None => (26.0, 2.5),
        }
    }

    // ---------------------------------------------------------------- effects (#38's _fx_build, in its order)
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // fx_salvos: BELL MORTAR: smoke out of the bell
        for s in &self.salvos {
            if s.t > p.t_ko {
                break;
            }
            let smoke = Emit {
                n: Some(3 + s.n.min(4)),
                col: Some(SMOKE_C.to_vec()),
                scale: 0.8,
                ..Emit::default()
            };
            fx.pa.emit(s.t, self.top(p, s.t), "smoke", &smoke);
            fx.fl.add(s.t, self.top(p, s.t), col, 1.2, 0.35, 0.03, 0.2, 0.0);
        }
        // fx_lands: the shells land
        for l in &self.lands {
            if l.t > p.t_ko {
                break;
            }
            let dust = Emit {
                n: Some(if l.kind == "barrage" { 2 } else { 3 }),
                col: Some(vec![[196.0, 178.0, 150.0], [150.0, 132.0, 110.0]]),
                ..Emit::default()
            };
            fx.pa.emit(l.t, up(l.x, l.z, 0.08), "dust", &dust);
            if l.hit != 0 {
                // only a hit scorches the canvas (misses just kick dust)
                fx.mk.add(
                    l.t,
                    Mark {
                        size: 0.30,
                        col: [44.0, 34.0, 30.0],
                        life: 2.5,
                        n: 1,
                        // seed=int(t_ * 100) + j
                        seed: (np::f64_to_i64(l.t * 100.0) + l.j).max(0) as u64,
                        strength: 0.5,
                        ..Mark::new(MarkKind::Scorch, l.x, l.z)
                    },
                );
                fx.fl.add(l.t, up(l.x, l.z, 0.4), col, 1.6, 0.7, 0.03, 0.25, 0.0);
                let sparks = Emit {
                    n: Some(8),
                    col: Some(BRASS_SPARK.to_vec()),
                    ..Emit::default()
                };
                fx.pa.emit(l.t, up(l.x, l.z, 0.3), "sparks", &sparks);
            } else if l.j == 0 {
                fx.fl.add(l.t, up(l.x, l.z, 0.3), col, 1.2, 0.35, 0.03, 0.2, 0.0);
            }
        }
        // fx_copies: IDENTITY CRISIS: an amber flare, question-mark motes
        for c in &self.copies {
            if c.t > p.t_ko {
                break;
            }
            fx.fl.add(c.t, self.top(p, c.t), col, 1.8, 0.7, 0.03, 0.35, 0.0);
            let motes = Emit {
                n: Some(10),
                col: Some(BRASS_SPARK.iter().chain(&SLATE_SPARK).copied().collect()),
                ..Emit::default()
            };
            fx.pa.emit(c.t, self.top(p, c.t), "motes", &motes);
        }
        // fx_swats (#48): a lance swats a shell away: sparks, a flare
        for &(t, by, x, z) in &self.swats {
            if t > p.t_ko {
                break;
            }
            let pos = up(x, z, R);
            fx.bu.add(t, pos, p.col[by].map(|c| c * 0.5 + 120.0), 1.0, None, true);
            fx.fl.add(t, pos, p.fl_col[by], 1.4, 0.6, 0.03, 0.2, 0.0);
            let sparks = Emit {
                n: Some(12),
                col: Some(BRASS_SPARK.to_vec()),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
        if let Some(d) = &self.dup {
            d.fx_setup(p, fx);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind == "hit" && how(e) == "shell" {
            fx.bu
                .add(e.t, up(e.f(5), e.f(6), R * 0.6), p.col[owner(e)], 0.8, None, true);
        } else if let Some(d) = &self.dup
            && (e.kind != "hit" || self.dup_kind(how(e)))
        {
            // a copied move's effects (the enemy's, in amber)
            d.fx_event(p, fx, e);
        }
    }

    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        if let Some(d) = &self.dup {
            d.fx_after(p, fx);
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![[255.0, 214.0, 120.0], [240.0, 172.0, 32.0], [214.0, 216.0, 226.0]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    // ---------------------------------------------------------------- the canvas
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if let Some(d) = &self.dup {
            d.canvas(p, ic, c);
        }
    }

    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if let Some(d) = &self.dup {
            d.canvas_wave(p, ic, c, w);
        }
    }

    /// The shells' growing shadows and splash-radius rings, the splashes, then the shadows darken it all (#38 did the
    /// darkening last, after every canvas light).
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if let Some(d) = &self.dup {
            d.canvas_after(p, ic, c);
        }
        if !c.in_fight {
            return;
        }
        let t = c.t;
        let bayer = bayer_full();
        let mut shade_c = vec![0.0f64; ic.len()];
        for sh in self.live_shells(p, t) {
            if sh.ts.is_some_and(|ts| t >= ts) {
                // #48: swatted: its target circle is gone
                continue;
            }
            let kind = if self.barrage_salvos.contains(&sh.salvo) {
                "barrage"
            } else {
                "mortar"
            };
            let s = (t - sh.t0) / py_max(1e-6, sh.t1 - sh.t0);
            if !(0.0..1.0).contains(&s) {
                continue;
            }
            let splash = self.splash(kind);
            let lit = p.col[sh.owner].map(|v| v / 255.0 * 1.2 + 0.2);
            for (j, light) in ic.iter_mut().enumerate() {
                let (sd, ring) = target_light(c.sxp[j], c.szp[j], &sh, s, splash);
                shade_c[j] = np::maximum(shade_c[j], sd * 0.7);
                let ring = (ring * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
                for ch in 0..3 {
                    light[ch] += ring * 0.9 * lit[ch];
                }
            }
        }
        for l in self.recent_lands(p, t) {
            let age = t - l.t;
            if !(0.0..0.4).contains(&age) {
                continue;
            }
            let splash = self.splash(&l.kind);
            let lit = p.col[self.k].map(|v| v / 255.0 * 1.2 + 0.25);
            let gain = if l.hit != 0 { 1.1 } else { 0.6 };
            for (j, light) in ic.iter_mut().enumerate() {
                let sp = splash_light(c.sxp[j], c.szp[j], l.x, l.z, age, splash) * gain;
                for ch in 0..3 {
                    light[ch] += sp * lit[ch];
                }
            }
        }
        for (light, s) in ic.iter_mut().zip(&shade_c) {
            for v in light.iter_mut() {
                *v *= 1.0 - s;
            }
        }
    }

    // ---------------------------------------------------------------- the shells in the air, shards
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, arr: &mut Img<f32>, depth: &mut Img<f32>) {
        if let Some(d) = &self.dup {
            d.over_ring(p, f, fr, arr, depth);
        }
        if !f.in_fight {
            return;
        }
        let tt = py_min(f.t, p.t_ko);
        for sh in self.live_shells(p, f.t) {
            let col = p.col[sh.owner];
            if let Some(ts) = sh.ts.filter(|&ts| tt >= ts) {
                // #48: swatted: it tumbles away and is gone
                if tt - ts < SWAT_T {
                    draw_swatted(arr, f.cam, &sh, ts, tt, col);
                }
                continue;
            }
            draw_shell(arr, f.cam, &sh, tt, col);
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
        _curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let at = |q: [f64; 2]| to_world(q[0], q[1], pos[1]);
        if kind == "brass" {
            // a length of tubing
            let (ln, wd) = (0.22, 0.045);
            let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .iter()
                .map(|&(a1, a2)| {
                    at([
                        sx + d[0] * ln * a1 + nn[0] * wd * a2,
                        sz + d[1] * ln * a1 + nn[1] * wd * a2,
                    ])
                })
                .collect();
            let sp = ring::screen(cam, &q4);
            ring::layer_blend(arr, &[sp], BRASS, 1.0, None);
            return;
        }
        // a valve (a chip), the bell (a big curved chip)
        let col = if kind == "valve" { BRASS_HI } else { BRASS };
        let sz_ = size * if kind == "valve" { 0.7 } else { 1.6 };
        let tri: Vec<V3> = [
            [sx + d[0] * sz_, sz + d[1] * sz_],
            [
                sx - d[0] * sz_ * 0.7 + nn[0] * sz_ * 0.8,
                sz - d[1] * sz_ * 0.7 + nn[1] * sz_ * 0.8,
            ],
            [
                sx - d[0] * sz_ * 0.4 - nn[0] * sz_ * 0.7,
                sz - d[1] * sz_ * 0.4 - nn[1] * sz_ * 0.7,
            ],
        ]
        .iter()
        .map(|&q| at(q))
        .collect();
        let sp = ring::screen(cam, &tri);
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        ring::layer_blend(arr, &[sp], col.map(|c| np::clip(c * shade_k, 0.0, 255.0)), 1.0, None);
    }

    // ---------------------------------------------------------------- lens
    fn shimmer(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        if let Some(d) = &self.dup {
            d.shimmer(p, img, cam, t, w);
        }
    }

    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        if let Some(d) = &self.dup {
            d.shimmer_after(p, img, cam, t);
        }
    }

    fn focus_points(&self, p: &PictureCtx, t: f64, st: &BallState) -> Vec<(V3, String)> {
        match &self.dup {
            Some(d) => d
                .focus_points(p, t, st)
                .into_iter()
                .filter(|(_, lab)| lab != "lance tip")
                .collect(),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ts: Option<f64>) -> ShellRow {
        ShellRow {
            owner: 0,
            x0: -1.0,
            z0: 0.5,
            x1: 1.5,
            z1: -0.5,
            t0: 10.0,
            t1: 11.0,
            salvo: 3,
            j: 1,
            ts,
        }
    }

    #[test]
    fn a_shell_arcs_from_the_bell_to_the_canvas() {
        let sh = row(None);
        let (x, z, y, s) = shell_pos(&sh, 10.0);
        assert_eq!((x, z, s), (-1.0, 0.5, 0.0));
        assert_eq!(y, R * 1.7);
        let (x, z, y, s) = shell_pos(&sh, 11.5);
        assert_eq!((x, z, s), (1.5, -0.5, 1.0));
        assert!((y - 0.05).abs() < 1e-12);
        // the apex is in the middle, higher than the launch
        assert!(shell_pos(&sh, 10.5).2 > R * 1.7);
    }

    #[test]
    fn a_recorded_row_reads_with_and_without_its_swat_time() {
        use Value::{Float as F, Int as I};
        let v = Value::List(vec![
            I(0),
            F(-1.0),
            F(0.5),
            F(1.5),
            F(-0.5),
            F(10.0),
            F(11.0),
            I(3),
            I(1),
            I(0),
        ]);
        assert_eq!(ShellRow::of(&v), Some(row(None)));
        let Value::List(mut l) = v else { unreachable!() };
        l[9] = I(3);
        l.push(F(10.6));
        assert_eq!(ShellRow::of(&Value::List(l)), Some(row(Some(10.6))));
        assert_eq!(ShellRow::of(&F(1.0)), None);
    }

    #[test]
    fn a_swatted_shell_flies_off_and_falls() {
        let sh = row(Some(10.6));
        let (x0, z0, y0) = swat_pos(&sh, 10.6, 10.6);
        let (x, z, _, _) = shell_pos(&sh, 10.6);
        assert_eq!((x0, z0), (x, z));
        assert!(y0 > 0.02);
        assert!(swat_pos(&sh, 10.6, 11.4).2 < 0.02);
    }

    #[test]
    fn the_splash_races_out_and_fades() {
        // at its centre right after the landing, on its ring at full radius later
        assert!(splash_light(0.0, 0.0, 0.0, 0.0, 0.0, 0.6) > 0.9);
        assert!(splash_light(0.6, 0.0, 0.0, 0.0, 0.2, 0.6) > 0.4);
        assert_eq!(splash_light(0.6, 0.0, 0.0, 0.0, 0.4, 0.6), 0.0);
    }

    #[test]
    fn the_amber_strangers_tint_is_a_float32_product_and_a_float64_sum() {
        let col = [240.0, 172.0, 32.0];
        let v = 200.25f32;
        let got = (f64::from(v * 0.55f32) + col[0] * 0.55) as f32;
        assert_eq!(got, (f64::from(v * 0.55f32) + 132.0) as f32);
        assert_ne!(f64::from(v * 0.55f32), f64::from(v) * 0.55);
    }
}
