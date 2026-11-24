//! EUPHONIUM with the bold effects (`picture.effects = "bold"`; the owner's picks 25a to 25d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs. Less clutter than classic: no smoke trails, dust or motes, one clear mark for each thing.
//!
//! - **BELL MORTAR** (25a): the bell tilts and thumps out a smoke ring; each shell is a big outlined brass slug
//!   spinning on its arc; its shadow grows inside a landing mark that closes in; it lands in a brass boom
//!   (stacked gold rings, a star, chips) and leaves a crater for two beats.
//! - **MARCHING ORDERS** (25b): chevrons march round the ball, a step on every 8th; the ring's speed-up look does
//!   the rest.
//! - **IDENTITY CRISIS** (25c): a dart in the enemy's colour flies into the ball, a mirror line sweeps across it
//!   as it turns into the amber stranger (a broken amber ring marks it for the bar), a bigger amber dart flies
//!   back at the enemy, and at the end of the bar the mirror sweeps back. The copied move's own look draws its
//!   status marks and its marks after the grade too.
//! - **NOT A TUBA** (25d): the ball thumps once for every shell and a column of smoke rings goes up, bright
//!   landing marks cover the ring, and the booms go off one after another.

use bf_core::dmath::{self, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize, Status};
use bf_picture::camera::{Cam, V3};
use bf_picture::fx::{Mark, MarkKind};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Fx, Pic};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::effects::up;
use bf_picture::ring::state::BallState;
use bf_picture::scene::R_BALL as R;

use super::state::Land;
use super::{BRASS, BRASS_HI, BRASS_SPARK, EuphoniumLook, SMOKE, SWAT_T, ShellRow, draw_swatted, shell_pos};

/// Seconds a smoke ring lasts.
const RING_S: f64 = 0.45;
/// Seconds between two thumps of NOT A TUBA.
const THUMP_GAP: f64 = 0.03;
/// Seconds a boom lasts, and seconds its crater shows (two beats at most).
const BOOM_S: f64 = 0.4;
const CRATER_S: f64 = 0.8;
/// Seconds the enemy's dart flies before IDENTITY CRISIS, a mirror sweep takes, and the copy's dart flies back.
const DART_S: f64 = 0.28;
const SWEEP_S: f64 = 0.22;
const BACK_S: f64 = 0.32;

/// A flat ring of radius `r` round the ring-frame point `(x, z)` at height `y`, on the screen.
fn world_ring(cam: &Cam, x: f64, z: f64, y: f64, r: f64) -> Vec<(f64, f64)> {
    (0..=24)
        .map(|i| {
            let a = f64::from(i) * TAU / 24.0;
            bold::at(cam, x + r * dmath::cos(a), z + r * dmath::sin(a), y)
        })
        .collect()
}

/// A dart at `tip`, flying along `dir` (a screen vector): a shaft, a head and two tail feathers, `size` times
/// the small one.
fn dart(img: &mut Pic, tip: (f64, f64), dir: (f64, f64), size: f64, col: V3, alpha: f64) {
    let n = dmath::hypot(dir.0, dir.1);
    if n.is_nan() || n < 1e-6 {
        return;
    }
    let (ux, uy) = (dir.0 / n, dir.1 / n);
    let tail = (tip.0 - ux * 13.0 * size, tip.1 - uy * 13.0 * size);
    for sgn in [-1.0, 1.0] {
        let feather = (
            tail.0 - ux * 4.0 * size - uy * 4.0 * size * sgn,
            tail.1 - uy * 4.0 * size + ux * 4.0 * size * sgn,
        );
        px::ink_line(img, tail, feather, 1.5 * size, bold::pale(col), alpha);
    }
    px::ink_arrow(img, tail, tip, 2.0 * size, 6.0 * size, col, alpha);
}

/// The mirror line across a ball: an upright white chord at `u` of the way across it (0 the left edge, 1 the
/// right), amber either side.
fn mirror(img: &mut Pic, d: &DrawnBall, u: f64, col: V3) {
    let r = d.rp * 1.12;
    let dx = (2.0 * u - 1.0) * r;
    let h = (r * r - dx * dx).max(0.0).sqrt();
    let x = d.s.0 + dx;
    for (off, c, w) in [(-3.0, col, 1.5), (3.0, col, 1.5), (0.0, bold::WHITE, 3.0)] {
        px::ink_line(img, (x + off, d.s.1 - h), (x + off, d.s.1 + h), w, c, 1.0);
    }
    px::glint(img, (x, d.s.1 - h), 5.0, bold::WHITE, 1.0);
}

/// The shells of one salvo taken together, so that a salvo of six is one mark and one boom, not six: the
/// middle of where they land (shell 0's spot), the radius that takes in every shell's splash, a number of the
/// caller's (how far along, or when they landed) and whether any of them hit.
struct Group {
    salvo: i64,
    c: [f64; 2],
    r: f64,
    v: f64,
    hit: bool,
}

/// Groups `(salvo, index, x, z, v, hit)` rows by their salvo, in the order the salvos first show.
fn groups(rows: impl Iterator<Item = (i64, i64, f64, f64, f64, bool)>, splash: f64) -> Vec<Group> {
    let mut out: Vec<(Group, Vec<[f64; 2]>)> = Vec::new();
    for (salvo, j, x, z, v, hit) in rows {
        let at = match out.iter().position(|(g, _)| g.salvo == salvo) {
            Some(i) => i,
            None => {
                let g = Group {
                    salvo,
                    c: [x, z],
                    r: splash,
                    v,
                    hit: false,
                };
                out.push((g, Vec::new()));
                out.len() - 1
            }
        };
        let (g, spots) = &mut out[at];
        if j == 0 {
            g.c = [x, z];
        }
        g.hit |= hit;
        spots.push([x, z]);
    }
    out.into_iter()
        .map(|(mut g, spots)| {
            let far = spots
                .iter()
                .map(|q| dmath::hypot(q[0] - g.c[0], q[1] - g.c[1]))
                .fold(0.0, f64::max);
            g.r = far + splash;
            g
        })
        .collect()
}

impl EuphoniumLook {
    /// Whether a shell is one of NOT A TUBA's.
    fn of_barrage(&self, sh: &ShellRow) -> bool {
        self.barrage_salvos.contains(&sh.salvo)
    }

    /// Its landings from `span` seconds ago up to `t` (and the K.O.).
    fn lands_within(&self, p: &PictureCtx, t: f64, span: f64) -> impl Iterator<Item = &Land> {
        let i0 = self.lands.partition_point(|l| l.t < t - span);
        let i1 = self.lands.partition_point(|l| l.t <= t);
        let t_ko = p.t_ko;
        self.lands[i0..i1.max(i0)].iter().filter(move |l| l.t <= t_ko)
    }

    /// The booms going off at `t`: a salvo's landings as one (its `v` is when it landed), each shell of NOT A
    /// TUBA by itself.
    fn booms(&self, p: &PictureCtx, t: f64) -> Vec<Group> {
        let recent = || self.lands_within(p, t, BOOM_S);
        let salvos = recent()
            .filter(|l| l.kind != "barrage")
            .map(|l| (l.salvo, l.j, l.x, l.z, l.t, l.hit != 0));
        let mut out = groups(salvos, self.splash("mortar"));
        let splash = self.splash("barrage");
        out.extend(recent().filter(|l| l.kind == "barrage").map(|l| Group {
            salvo: l.salvo,
            c: [l.x, l.z],
            r: splash,
            v: l.t,
            hit: l.hit != 0,
        }));
        out
    }

    /// How far the bell is tilted at `t` (0 to 1): back before a salvo, let go after it.
    fn tilt(&self, p: &PictureCtx, t: f64) -> f64 {
        let tc = t.min(p.t_ko);
        let i = self.salvos.partition_point(|s| s.t <= tc);
        let before = self
            .salvos
            .get(i)
            .map_or(0.0, |next| bold::ease_in(1.0 - (next.t - tc) / 0.16));
        let after = match i.checked_sub(1) {
            Some(j) => (1.0 - (tc - self.salvos[j].t) / 0.2).max(0.0),
            None => 0.0,
        };
        before.max(after)
    }

    /// The thump: the ball swells as a salvo leaves, and once for every shell of NOT A TUBA.
    pub(super) fn bold_state(&self, p: &PictureCtx, st: &mut BallState, t: f64) {
        let tc = t.min(p.t_ko);
        let i = self.salvos.partition_point(|s| s.t <= tc);
        if let Some(j) = i.checked_sub(1) {
            let age = tc - self.salvos[j].t;
            if age < 0.12 {
                st.scale *= 1.0 + 0.14 * (1.0 - age / 0.12);
            }
        }
        if let Some((age, e)) = bold::since(p, self.k, &["barrage"], tc, 1.0)
            && age < e.i(3) as f64 * THUMP_GAP
        {
            let beat = (age / THUMP_GAP).fract();
            st.scale *= 1.0 + 0.13 * (1.0 - beat);
        }
    }

    /// The bell tilts back before a salvo.
    pub(super) fn bold_roll(&self, p: &PictureCtx, t: f64, roll: f64) -> f64 {
        roll - 0.45 * self.tilt(p, t)
    }

    /// The effects without the clutter: the flares, the scorch of a hit (its crater), a swat's sparks; no
    /// smoke puffs, dust or motes (the smoke ring, the boom and the mirror sweep say it).
    pub(super) fn bold_fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        for s in self.salvos.iter().take_while(|s| s.t <= p.t_ko) {
            fx.fl.add(s.t, self.top(p, s.t), col, 1.2, 0.35, 0.03, 0.2, 0.0);
        }
        for l in self.lands.iter().take_while(|l| l.t <= p.t_ko) {
            if l.hit != 0 {
                fx.mk.add(
                    l.t,
                    Mark {
                        size: 0.44,
                        col: [44.0, 34.0, 30.0],
                        life: 2.5,
                        n: 1,
                        seed: (np::f64_to_i64(l.t * 100.0) + l.j).max(0) as u64,
                        strength: 0.85,
                        ..Mark::new(MarkKind::Scorch, l.x, l.z)
                    },
                );
                fx.fl.add(l.t, up(l.x, l.z, 0.4), col, 1.6, 0.7, 0.03, 0.25, 0.0);
            } else if l.j == 0 {
                fx.fl.add(l.t, up(l.x, l.z, 0.3), col, 1.2, 0.35, 0.03, 0.2, 0.0);
            }
        }
        for c in self.copies.iter().take_while(|c| c.t <= p.t_ko) {
            fx.fl.add(c.t, self.top(p, c.t), col, 1.8, 0.7, 0.03, 0.35, 0.0);
        }
        for &(t, by, x, z) in self.swats.iter().take_while(|s| s.0 <= p.t_ko) {
            let pos = up(x, z, R);
            fx.bu.add(t, pos, p.col[by].map(|c| c * 0.5 + 120.0), 1.0, None, true);
            fx.fl.add(t, pos, p.fl_col[by], 1.4, 0.6, 0.03, 0.2, 0.0);
            let sparks = bf_picture::fx::Emit {
                n: Some(12),
                col: Some(BRASS_SPARK.to_vec()),
                ..bf_picture::fx::Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
        if let Some(d) = &self.dup {
            d.fx_setup(p, fx);
        }
    }

    /// The landing marks that close in, the booms' brass rings, the craters, and last the shells' growing
    /// shadows.
    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if let Some(d) = &self.dup {
            d.canvas_after(p, ic, c);
        }
        if !c.in_fight {
            return;
        }
        let t = c.t;
        let tint = bold::light(p.col[self.k]);
        let flying: Vec<(ShellRow, f64)> = self
            .live_shells(p, t)
            .into_iter()
            .filter(|sh| !sh.ts.is_some_and(|ts| t >= ts))
            .map(|sh| (sh, (t - sh.t0) / (sh.t1 - sh.t0).max(1e-6)))
            .filter(|(_, s)| (0.0..1.0).contains(s))
            .collect();
        // one landing mark for a whole salvo, closing in on where it falls
        let salvos = flying
            .iter()
            .filter(|(sh, _)| !self.of_barrage(sh))
            .map(|(sh, s)| (sh.salvo, sh.j, sh.x1, sh.z1, *s, false));
        for g in groups(salvos, self.splash("mortar")) {
            floor::target(ic, c, g.c[0], g.c[1], g.r, g.v, tint);
        }
        for (sh, s) in flying.iter().filter(|(sh, _)| self.of_barrage(sh)) {
            // (its bright mark is drawn after the grade, over the dimmed ring)
            floor::ring(ic, c, sh.x1, sh.z1, self.splash("barrage"), 0.035, tint, 0.3 + 0.6 * s);
        }
        // one brass boom for a salvo; every shell of NOT A TUBA booms by itself
        for g in self.booms(p, t) {
            let age = t - g.v;
            let r = g.r * (0.3 + 1.0 * bold::ease_out(age / 0.22));
            let gain = if g.hit { 1.3 } else { 0.7 };
            floor::band(
                ic,
                c,
                Family::Brass,
                g.c[0],
                g.c[1],
                r,
                1.0 - age / BOOM_S,
                t,
                tint,
                gain,
            );
        }
        for l in self.lands_within(p, t, CRATER_S) {
            let age = t - l.t;
            let splash = self.splash(&l.kind);
            if l.hit != 0 {
                // the crater: a dark dent with a lit rim, gone in two beats
                let left = 1.0 - age / CRATER_S;
                floor::shade(ic, c, l.x, l.z, splash * 0.62, 0.6 * left);
                floor::ring(ic, c, l.x, l.z, splash * 0.66, 0.04, tint, 0.9 * left);
            }
        }
        for (sh, s) in &flying {
            let splash = self.splash(if self.of_barrage(sh) { "barrage" } else { "mortar" });
            floor::shade(ic, c, sh.x1, sh.z1, 0.12 + (splash * 0.85 - 0.12) * s, 0.3 + 0.45 * s);
        }
    }

    pub(super) fn bold_over_ring(
        &self,
        p: &PictureCtx,
        f: &FrameCtx<'_>,
        fr: &mut Frame,
        arr: &mut Img<f32>,
        depth: &mut Img<f32>,
    ) {
        if let Some(d) = &self.dup {
            d.over_ring(p, f, fr, arr, depth);
        }
        if !f.in_fight {
            return;
        }
        // a swatted shell tumbles away (as classic); the shells in flight are drawn after the grade
        let tt = f.t.min(p.t_ko);
        for sh in self.live_shells(p, f.t) {
            if let Some(ts) = sh.ts.filter(|&ts| tt >= ts)
                && tt - ts < SWAT_T
            {
                draw_swatted(arr, f.cam, &sh, ts, tt, p.col[sh.owner]);
            }
        }
    }

    /// One shell in flight: a brass slug, outlined, spinning end over end, with two puffs behind a salvo's.
    fn shell(&self, img: &mut Pic, cam: &Cam, sh: &ShellRow, t: f64) {
        let (x, z, y, s) = shell_pos(sh, t);
        if s >= 1.0 {
            return;
        }
        let c = bold::at(cam, x, z, y);
        let rp = (0.17 * bold::px_per_unit(cam, x, z, y)).max(3.2);
        if !self.of_barrage(sh) {
            for (n, dt) in [0.07, 0.14].into_iter().enumerate() {
                let q = shell_pos(sh, t - dt);
                if q.3 > 0.0 {
                    px::disc(
                        img,
                        bold::at(cam, q.0, q.1, q.2),
                        rp * (0.55 - 0.2 * n as f64),
                        SMOKE,
                        0.7,
                    );
                }
            }
        }
        let spin = t * 17.0 + sh.j as f64 * 1.3;
        let (ca, sa) = (dmath::cos(spin), dmath::sin(spin));
        let slug: Vec<(f64, f64)> = (0..12)
            .map(|i| {
                let a = f64::from(i) * TAU / 12.0;
                let (ex, ey) = (dmath::cos(a) * rp * 1.5, dmath::sin(a) * rp * 0.95);
                (c.0 + ex * ca - ey * sa, c.1 + ex * sa + ey * ca)
            })
            .collect();
        px::ink_poly(img, &slug, BRASS, 1.0);
        // the band round its middle turns with it; the highlight stays up and to the left
        let band = (-sa * rp * 0.9, ca * rp * 0.9);
        px::line(
            img,
            (c.0 - band.0, c.1 - band.1),
            (c.0 + band.0, c.1 + band.1),
            1.6,
            bold::deep(BRASS),
            1.0,
        );
        px::disc(img, (c.0 - rp * 0.35, c.1 - rp * 0.35), rp * 0.32, BRASS_HI, 1.0);
    }

    /// A smoke ring born at `t0` over the bell (where the ball was then), `age` seconds old: it rises and widens.
    fn smoke_ring(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t0: f64, age: f64, big: f64) {
        if !(0.0..RING_S).contains(&age) {
            return;
        }
        let u = age / RING_S;
        let q = p.run.p(p.sim_i(t0), self.k);
        let ring = world_ring(
            cam,
            q[0],
            q[1],
            R * 2.0 + 0.9 * bold::ease_out(u),
            (0.16 + 0.42 * bold::ease_out(u)) * big,
        );
        let alpha = (2.0 * (1.0 - u)).min(1.0);
        px::ink_polyline(img, &ring, 3.0 - 1.5 * u, SMOKE, alpha);
        px::polyline(img, &ring, 1.0, bold::WHITE, alpha * 0.7);
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        // the copied move's own marks, for this side
        if let Some(d) = &self.dup {
            d.after_grade(p, img, f, drawn);
        }
        if !f.in_fight {
            return;
        }
        let (k, cam, t) = (self.k, f.cam, f.t);
        let tt = t.min(p.t_ko);
        let col = p.col[k];
        let gold = bold::pale(col);
        // NOT A TUBA: a bright landing mark for every shell (the ring is dimmed under them)
        let shells = self.live_shells(p, t);
        for sh in shells.iter().filter(|sh| self.of_barrage(sh)) {
            let s = (tt - sh.t0) / (sh.t1 - sh.t0).max(1e-6);
            if !(0.0..1.0).contains(&s) || sh.ts.is_some_and(|ts| tt >= ts) {
                continue;
            }
            let splash = self.splash("barrage");
            let near = bold::ease_in(s);
            px::ink_polyline(
                img,
                &world_ring(cam, sh.x1, sh.z1, 0.02, splash),
                1.5,
                gold,
                0.5 + 0.5 * s,
            );
            let closing = world_ring(cam, sh.x1, sh.z1, 0.02, splash * (2.3 - 1.3 * near));
            px::polyline(img, &closing, 1.5, bold::WHITE, 0.35 + 0.65 * near);
            for i in 0..4 {
                let a = TAU / 8.0 + f64::from(i) * TAU / 4.0;
                let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                let q0 = bold::at(cam, sh.x1 + ca * splash * 0.55, sh.z1 + sa * splash * 0.55, 0.02);
                let q1 = bold::at(cam, sh.x1 + ca * splash * 0.95, sh.z1 + sa * splash * 0.95, 0.02);
                px::line(img, q0, q1, 1.5, gold, 0.5 + 0.5 * s);
            }
        }
        // the booms: two stacked brass rings race out from a salvo's middle (from each shell of NOT A TUBA)...
        for g in self.booms(p, tt) {
            let age = tt - g.v;
            let reach = g.r * if g.hit { 1.15 } else { 0.8 };
            for (n, back) in [0.0, 0.2].into_iter().enumerate() {
                let r = reach * (0.3 + 0.9 * bold::ease_out((age / 0.24).min(1.0))) - back;
                if r > 0.05 {
                    let ring = world_ring(cam, g.c[0], g.c[1], 0.03, r);
                    let fade = 1.0 - age / BOOM_S;
                    px::ink_polyline(img, &ring, 2.5 - n as f64, if n == 0 { gold } else { col }, fade);
                }
            }
        }
        // ...and a star and chips where each shell came down
        for l in self.lands_within(p, tt, BOOM_S) {
            let age = tt - l.t;
            let u = age / BOOM_S;
            let at = bold::at(cam, l.x, l.z, 0.12);
            let size = if l.hit != 0 { 1.0 } else { 0.65 };
            let r = (8.0 + 9.0 * bold::ease_out(u / 0.5)) * size;
            if u < 0.5 {
                px::ink_star(img, at, r, 0.3 * r, 8, l.t * 7.0, bold::WHITE, 1.0);
            } else {
                px::star(img, at, r, 0.3 * r, 8, l.t * 7.0, gold, 2.0 * (1.0 - u));
            }
            let seed = (l.t * 1000.0) as u64 + l.j as u64;
            px::chips(img, at, age, BOOM_S, 4, 20.0 * size, seed, BRASS);
        }
        // the shells in the air
        for sh in &shells {
            if !sh.ts.is_some_and(|ts| tt >= ts) {
                self.shell(img, cam, sh, tt);
            }
        }
        // a smoke ring out of the bell for every salvo, and a column of them for NOT A TUBA
        let i0 = self.salvos.partition_point(|s| s.t < tt - RING_S);
        for s in self.salvos[i0..].iter().take_while(|s| s.t <= tt) {
            self.smoke_ring(p, img, cam, s.t, tt - s.t, 1.0);
        }
        if let Some((age, e)) = bold::since(p, k, &["barrage"], tt, 1.0) {
            for n in 0..e.i(3).clamp(0, 24) {
                let born = n as f64 * THUMP_GAP;
                self.smoke_ring(p, img, cam, e.t, age - born, 0.8 + 0.06 * n as f64);
            }
        }
        let me = drawn.iter().find(|d| d.k == k);
        let foe = drawn.iter().find(|d| d.k == 1 - k);
        // MARCHING ORDERS: chevrons march round the ball, one step on every 8th
        if let (Some(_), Some(d)) = (p.bold_haste_left(k, t), me)
            && t < p.t_ko
        {
            let grid = (t - p.show.grid.t_fight) / (p.show.grid.beat / 2.0);
            let (step, into) = (grid.floor(), grid.fract());
            let hop = (1.0 - into / 0.3).max(0.0);
            let fresh = bold::since(p, k, &["ability:haste"], t, 0.5).map_or(0.0, |(age, _)| 1.0 - age / 0.5);
            let size = 1.0 + 0.6 * fresh;
            for i in 0..4 {
                let a = (step + bold::ease_out(into / 0.3) - 1.0) * TAU / 8.0 + f64::from(i) * TAU / 4.0;
                let (ca, sa) = (dmath::cos(a), dmath::sin(a));
                let r = d.rp * 1.75 + 3.0 * hop;
                let c = (d.s.0 + ca * r, d.s.1 + sa * r * 0.92);
                // pointing the way they march (clockwise on the screen)
                let (tx, ty) = (-sa, ca);
                let tip = (c.0 + tx * 4.5 * size, c.1 + ty * 4.5 * size);
                let arm = |sgn: f64| {
                    (
                        c.0 - tx * 3.0 * size + ca * 4.5 * size * sgn,
                        c.1 - ty * 3.0 * size + sa * 4.5 * size * sgn,
                    )
                };
                px::ink_polyline(img, &[arm(-1.0), tip, arm(1.0)], 2.5, gold, 1.0);
            }
        }
        // IDENTITY CRISIS
        let bar = p.show.grid.bar;
        for c in self.copies.iter().filter(|c| c.t < p.t_ko) {
            let age = t - c.t;
            let (Some(d), Some(e)) = (me, foe) else { break };
            let way = (d.s.0 - e.s.0, d.s.1 - e.s.1);
            if (-DART_S..0.0).contains(&age) {
                // the enemy's dart flies into the ball
                let u = bold::ease_in(1.0 + age / DART_S);
                let tip = (e.s.0 + way.0 * u, e.s.1 + way.1 * u);
                dart(img, tip, way, 1.0, p.col[1 - k], 1.0);
            }
            if (0.0..0.12).contains(&age) {
                px::ink_star(
                    img,
                    d.s,
                    6.0 + 60.0 * age,
                    3.0,
                    6,
                    0.3,
                    bold::pale(p.col[1 - k]),
                    1.0 - age / 0.12,
                );
            }
            if (0.0..bar).contains(&age) {
                // not itself for a bar: a broken amber ring that turns, its gaps widening as the bar runs out
                let left = 1.0 - age / bar;
                for i in 0..4 {
                    let a0 = t * 2.5 + f64::from(i) * TAU / 4.0;
                    px::ink_arc(img, d.s, d.rp * 1.14, 2.0, a0, a0 + (0.35 + 0.9 * left), col, 1.0);
                }
            }
            if (0.0..SWEEP_S).contains(&age) {
                mirror(img, d, age / SWEEP_S, col);
            }
            let fired = age - 0.1;
            if (0.0..BACK_S).contains(&fired) {
                // the copy goes back: a bigger amber dart
                let u = bold::ease_out(fired / BACK_S);
                let tip = (d.s.0 - way.0 * u, d.s.1 - way.1 * u);
                dart(
                    img,
                    tip,
                    (-way.0, -way.1),
                    1.7,
                    col,
                    (3.0 * (1.0 - fired / BACK_S)).min(1.0),
                );
            }
            let back = age - (bar - 0.15 - SWEEP_S / 2.0);
            if (0.0..SWEEP_S).contains(&back) {
                // ...and it sweeps back to itself
                mirror(img, d, 1.0 - back / SWEEP_S, col);
            }
        }
    }

    /// A salvo that lands on the enemy pops once, as big as all its shells together, not once a shell.
    pub(super) fn bold_hit(&self, p: &PictureCtx, e: &Event) -> Option<HitSize> {
        let mut same = bold::events_in(p, self.k, &["hit:shell"], e.t - 1e-6, e.t + 1e-6).peekable();
        if !same.peek().is_some_and(|first| std::ptr::eq(*first, e)) {
            return None;
        }
        Some(HitSize::of_damage(same.map(|h| h.f(4)).sum()))
    }

    /// The statuses a copied move puts on a ball (a copied SQUEAK's stun), from the copy's look.
    pub(super) fn bold_statuses(&self, p: &PictureCtx, t: f64) -> Vec<Status> {
        self.dup.as_ref().map_or_else(Vec::new, |d| d.bold_status(p, t))
    }
}
