//! ALTO SAX with the bold effects (`picture.effects = "bold"`; the owner's picks 19a to 19d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **SPOTLIGHT SOLO** (19a): a hard-edged cone and a bright, hard-rimmed pool sweep the canvas, the floor
//!   round it dimmed; while the enemy is in a light it wears a hot ring, sizzles and gives off wisps of smoke
//!   (each burn a small pop); behind the light a scorch trail with a glowing edge fades over two beats.
//! - **OCTAVE KEY** (19b): the second light is cyan, cone and pool; where the two pools cross the floor goes
//!   white, and with both on the enemy it goes white-hot: white rays round it and a double tick on each burn.
//! - **ALTISSIMO** (19c): a small white-hot spot with a hard core on a thin bright cone; it whips in a small
//!   figure of eight round its sweep and leaves a glowing line; each of its burns pops as a solid hit.
//! - **ENCORE** (19d): the house goes dark round the ring and phone lights twinkle in the stands; each phone
//!   light is a thin beam from the stands to a ringed pinprick; the big light chases the enemy; as it ends every
//!   phone flares and the light bursts in a finale flash (round the pool, not the whole screen).

use bf_core::dmath::{self, TAU};
use bf_core::event::Event;
use bf_picture::bold::floor;
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::V3;
use bf_picture::config::{self, H, W};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{self, R_BALL as R};

use super::{AltoSaxLook, HOT, Light, WARM, cone_points, how, hull_of, lights};
use crate::fighters::shared::reeds;

/// OCTAVE KEY's light, as canvas light and as paint.
const CYAN_L: V3 = [0.2, 0.9, 1.15];
/// See [`CYAN_L`].
const CYAN: V3 = [120.0, 236.0, 255.0];
/// A burning enemy's ring.
const HEAT: V3 = [255.0, 170.0, 84.0];
/// A wisp of smoke.
const SMOKE: V3 = [226.0, 220.0, 232.0];
/// How far ALTISSIMO's spot whips to either side of its sweep.
const WHIP: f64 = 0.3;
/// Seconds ENCORE's finale flash lasts.
const FINALE_S: f64 = 0.3;
/// How many phones twinkle in the stands.
const PHONES: u64 = 46;

impl AltoSaxLook {
    /// The line a sweep runs along, from its `spot` event: where it starts and where it ends.
    fn sweep_line(p: &PictureCtx, k: usize, s: &Light) -> Option<([f64; 2], [f64; 2])> {
        // (the event's own time may be rounded: its start is its element 8)
        bold::events_in(p, k, &["spot"], s.t0 - 0.05, s.t0 + 0.05)
            .find(|e| e.i(11) == s.kind && (e.f(8) - s.t0).abs() < 1e-6)
            .map(|e| ([e.f(4), e.f(5)], [e.f(6), e.f(7)]))
    }

    /// How far ALTISSIMO's spot is drawn from its place on the sweep `a` to `b` at `t`: a small figure of eight
    /// (across the sweep, and twice as fast along it). The burn stays the sweep's own.
    fn whip(s: &Light, a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
        let d = [b[0] - a[0], b[1] - a[1]];
        let n = dmath::hypot(d[0], d[1]);
        if n < 1e-6 {
            return [0.0, 0.0];
        }
        let (u, v) = ([d[0] / n, d[1] / n], [-d[1] / n, d[0] / n]);
        let ph = (t - s.t0) * TAU * 3.1;
        let amp = WHIP * s.fade(t);
        let (across, along) = (amp * dmath::sin(ph), 0.55 * amp * dmath::sin(2.0 * ph));
        [v[0] * across + u[0] * along, v[1] * across + u[1] * along]
    }

    /// Where ALTISSIMO's spot is drawn at `t`: its place on the sweep plus the whip.
    fn whipped(p: &PictureCtx, k: usize, mut s: Light, t: f64) -> Light {
        if !s.hot || s.kind == 2 || s.kind == 3 {
            return s;
        }
        if let Some((a, b)) = Self::sweep_line(p, k, &s) {
            let off = Self::whip(&s, a, b, t);
            s.cx += off[0];
            s.cz += off[1];
        }
        s
    }

    /// Its lights at `t` as the bold look draws them (ALTISSIMO's whipped), none outside the fight.
    fn shown(&self, p: &PictureCtx, t: f64) -> Vec<Light> {
        if !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return Vec::new();
        }
        lights(p, p.sim_i(t), self.k)
            .into_iter()
            .map(|s| Self::whipped(p, self.k, s, t))
            .collect()
    }

    /// How dark the house is for ENCORE at `t` (0 to 1): in over 0.3 s, out over the finale.
    fn house_dark(&self, p: &PictureCtx, t: f64) -> f64 {
        self.enc
            .iter()
            .filter(|e| e.0 < p.t_ko)
            .map(|&(a, b)| bold::ease_out((t - a) / 0.3) * (1.0 - bold::ease_in((t - b) / FINALE_S)))
            .fold(0.0, f64::max)
            * if t < p.t_ko { 1.0 } else { 0.0 }
    }

    /// ALTISSIMO's glowing line: the way the whipped spot came over the last 0.4 s, drawn over the scorch.
    fn glowing_line(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, ls: &[Light]) {
        for s in ls.iter().filter(|s| s.hot && s.kind != 2 && s.kind != 3) {
            let Some((a, b)) = Self::sweep_line(p, self.k, s) else {
                continue;
            };
            // the sweep's own place (eased along its line; OCTAVE KEY's light goes there and back), then the whip
            let at = |tp: f64| {
                let mut u = ((tp - s.t0) / (s.t1 - s.t0).max(1e-9)).clamp(0.0, 1.0);
                if s.kind == 1 {
                    u = if u < 0.5 { 2.0 * u } else { 2.0 - 2.0 * u };
                }
                let u = u * u * (3.0 - 2.0 * u);
                let off = Self::whip(s, a, b, tp);
                [a[0] + (b[0] - a[0]) * u + off[0], a[1] + (b[1] - a[1]) * u + off[1]]
            };
            const STEPS: u32 = 36;
            for n in 0..STEPS {
                let (t0, t1) = (c.t - f64::from(n) / 90.0, c.t - f64::from(n + 1) / 90.0);
                if t1 < s.t0 {
                    break;
                }
                let fade = 1.0 - f64::from(n) / f64::from(STEPS);
                floor::lane(ic, c, at(t0), at(t1), 0.06, [1.5, 1.5, 1.7], 1.6 * fade);
            }
        }
    }

    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let bayer = config::bayer_full();
        let own = p.col[k].map(|v| v / 255.0);
        let ls = self.shown(p, c.t);
        for s in &ls {
            if s.kind == 3 {
                // a phone light: a white pinprick in a thin ring
                floor::disc(ic, c, s.cx, s.cz, 0.16, [1.5, 1.5, 1.6], 1.0);
                floor::ring(ic, c, s.cx, s.cz, 0.32, 0.035, [1.2, 1.2, 1.3], 0.9);
                continue;
            }
            let fade = s.fade(c.t);
            let (tint, gain, rim): (V3, f64, V3) = if s.hot {
                (HOT, 1.15, [1.2, 1.2, 1.3])
            } else if s.kind == 1 {
                (CYAN_L, 0.72, CYAN_L)
            } else {
                (WARM, if s.kind == 0 { 0.72 } else { 0.8 }, own.map(|v| v + 0.25))
            };
            for (j, light) in ic.iter_mut().enumerate() {
                let rd = dmath::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz);
                if rd > s.r + 0.08 {
                    continue;
                }
                let b = bayer.px(c.xs[j], c.ys[j])[0];
                // a hard edge: two dithered pixels of it, no more
                let core = ((s.r - rd) / 0.03).clamp(0.0, 1.0);
                let mut m = (core * 2.0 + b * 0.9).floor() / 2.0;
                if s.hot && rd < s.r * 0.55 {
                    m += 0.9;
                }
                let v = m * gain * fade;
                // the rim, in the light's own colour
                let w = if (rd - s.r).abs() < 0.045 { 0.95 * fade } else { 0.0 };
                for ch in 0..3 {
                    light[ch] += v * tint[ch] + w * rim[ch];
                }
            }
        }
        // where two pools cross the floor goes white
        let big: Vec<&Light> = ls.iter().filter(|s| s.kind != 3).collect();
        if big.len() >= 2 {
            for (j, light) in ic.iter_mut().enumerate() {
                let inside = big
                    .iter()
                    .filter(|s| dmath::hypot(c.sxp[j] - s.cx, c.szp[j] - s.cz) < s.r)
                    .count();
                if inside >= 2 {
                    for ch in light.iter_mut() {
                        *ch += 0.85;
                    }
                }
            }
        }
    }

    /// The floor dims round each pool (and the whole canvas for ENCORE), and the scorch trail the lights left
    /// over the last two beats darkens it, glowing where it is fresh.
    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let k = self.k;
        let ls: Vec<Light> = self.shown(p, c.t).into_iter().filter(|s| s.kind != 3).collect();
        // the trail: where the lights stood at each earlier frame, newest first
        let life = 2.0 * p.show.grid.beat;
        let mut trail: Vec<(f64, f64, f64, f64)> = Vec::new();
        for n in 1..=(life * 30.0) as u32 {
            let age = f64::from(n) / 30.0;
            let tp = c.t - age;
            if tp < p.show.grid.t_fight {
                break;
            }
            for l in lights(p, p.sim_i(tp), k).into_iter().filter(|l| l.kind != 3) {
                let l = Self::whipped(p, k, l, tp);
                let r = l.r * if l.hot { 1.0 } else { 0.8 };
                trail.push((l.cx, l.cz, r * r, age));
            }
        }
        let dark = self.house_dark(p, c.t);
        if ls.is_empty() && trail.is_empty() && dark <= 0.0 {
            return;
        }
        self.scorch(p, ic, c, &ls, &trail, dark);
        self.glowing_line(p, ic, c, &ls);
    }

    /// The dimming and the scorch of [`Self::bold_canvas_after`], pixel by pixel.
    fn scorch(
        &self,
        p: &PictureCtx,
        ic: &mut [V3],
        c: &CanvasCtx<'_>,
        ls: &[Light],
        trail: &[(f64, f64, f64, f64)],
        dark: f64,
    ) {
        let bayer = config::bayer_full();
        let life = 2.0 * p.show.grid.beat;
        for (j, light) in ic.iter_mut().enumerate() {
            let (sx, sz) = (c.sxp[j], c.szp[j]);
            let b = bayer.px(c.xs[j], c.ys[j])[0];
            let mut dim: f64 = 1.0;
            let mut lit = false;
            for s in ls {
                let rd = dmath::hypot(sx - s.cx, sz - s.cz);
                if rd <= s.r {
                    lit = true;
                } else {
                    let ring = (1.0 - (rd - s.r) / 1.1).clamp(0.0, 1.0);
                    dim = dim.min(1.0 - 0.45 * (ring * 3.0 + b * 0.9).floor() / 3.0);
                }
            }
            if lit {
                continue;
            }
            dim = dim.min(1.0 - 0.5 * dark);
            // the scorch: the newest light that stood here
            let mut ember = 0.0;
            if let Some(&(_, _, _, age)) = trail
                .iter()
                .find(|&&(x, z, r2, _)| (sx - x) * (sx - x) + (sz - z) * (sz - z) < r2)
            {
                let v = 1.0 - age / life;
                dim *= 1.0 - 0.5 * ((v * 3.0 + b * 0.9).floor() / 3.0).min(1.0);
                ember = (1.0 - age / 0.22).max(0.0);
            }
            for ch in 0..3 {
                light[ch] = light[ch] * dim + ember * [0.55, 0.2, 0.04][ch];
            }
        }
    }

    /// The cones from the rafters: brighter, in three hard steps, with their edges drawn.
    pub(super) fn bold_over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, arr: &mut Img<f32>) {
        if !f.in_fight {
            return;
        }
        let cam = f.cam;
        let bayer = config::bayer_full();
        for s in self.shown(p, f.t) {
            if s.kind == 3 {
                continue;
            }
            let fade = s.fade(f.t);
            let pts = cone_points(cam, &s);
            let Some(outline) = hull_of(&pts) else { continue };
            let mk = reeds::mask(std::slice::from_ref(&outline), &[], 1);
            let y_max = pts[1..].iter().map(|q| q.1).fold(f64::NEG_INFINITY, f64::max);
            let mut lit: Img<f64> = Img::new(W, H, 1);
            for y in 0..H {
                let fall = ((y_max - y as f64 + 60.0) / 380.0).clamp(0.3, 1.0);
                for x in 0..W {
                    let m = f64::from(mk.px(x, y)[0]);
                    if m > 0.0 {
                        lit.px_mut(x, y)[0] = (m * fall * 3.0 + bayer.px(x, y)[0] * 0.9).floor() / 3.0;
                    }
                }
            }
            let col = if s.hot {
                [255.0, 255.0, 255.0]
            } else if s.kind == 1 {
                CYAN
            } else {
                [255.0, 236.0, 200.0]
            };
            reeds::add(arr, &lit, col, (if s.hot { 0.62 } else { 0.46 }) * fade);
            // the hard edge of the cone
            let mut closed = outline.clone();
            closed.push(outline[0]);
            px::polyline(arr, &closed, 1.6, col, 0.75 * fade);
            let g = pts[0];
            if -20.0 < g.1 && g.1 < H as f64 {
                fr.glow(g.0, g.1, 9.0, [255.0, 244.0, 220.0], 0.7 * fade);
            }
        }
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight {
            return;
        }
        let (k, o, t, cam) = (self.k, 1 - self.k, f.t, f.cam);
        let ls = self.shown(p, t);
        // ENCORE: the house goes dark round the ring and the phones twinkle in the stands
        let dark = self.house_dark(p, t);
        let finale = self
            .enc
            .iter()
            .filter(|e| e.1 < p.t_ko)
            .map(|e| t - e.1)
            .find(|a| (0.0..FINALE_S).contains(a));
        if dark > 0.0 {
            let bayer = config::bayer_full();
            let mid = bold::at(cam, 0.0, 0.0, 0.0);
            let corner = bold::at(cam, -scene::POST, scene::POST, 0.0);
            let r = dmath::hypot(corner.0 - mid.0, corner.1 - mid.1) * 0.92;
            px::dim_outside(img, mid, r, 70.0, 1.0 - 0.72 * dark, &|x, y| bayer.px(x, y)[0]);
            for j in 0..PHONES {
                // a seat in the stands: round the ring, a little way up
                let ang = TAU * (j as f64 + 0.8 * px::rnd(j, 1)) / PHONES as f64;
                let far = scene::STAND_R0 + 0.3 + 2.4 * px::rnd(j, 2);
                let y = 0.5 + (far - scene::STAND_R0) * 0.5;
                let s = cam.wp(&[[far * dmath::cos(ang), y, far * dmath::sin(ang)]])[0];
                // each blinks to its own time; all of them flare for the finale
                let on = px::rnd(j, (t * 5.0 + px::rnd(j, 3) * 7.0).floor() as u64 + 10) < 0.6;
                if finale.is_some() {
                    px::glint(img, s, 9.0, bold::WHITE, 1.0);
                } else if on {
                    px::disc(img, s, 2.2, bold::WHITE, dark);
                    px::glint(img, s, 3.0 + 3.0 * px::rnd(j, 4), bold::WHITE, 0.9 * dark);
                }
            }
        }
        // each phone light: a thin beam from the stands down to its pinprick
        for s in ls.iter().filter(|s| s.kind == 3) {
            let w = scene::to_world(s.cx, s.cz, 0.05);
            let n = dmath::hypot(w[0], w[2]).max(1e-9);
            let src = cam.wp(&[[w[0] / n * 7.0, 1.1, w[2] / n * 7.0]])[0];
            let end = cam.wp(&[w])[0];
            px::line(img, src, end, 3.0, [200.0, 220.0, 255.0], 0.3);
            px::line(img, src, end, 1.2, bold::WHITE, 0.95);
            px::glint(img, src, 8.0, bold::WHITE, 1.0);
            px::glint(img, end, 5.0, bold::WHITE, 1.0);
        }
        // the enemy in the light: a hot ring, sizzle and wisps of smoke
        let en = f.fs[o].p;
        let on_it: Vec<&Light> = ls
            .iter()
            .filter(|s| s.kind != 3 && dmath::hypot(en[0] - s.cx, en[1] - s.cz) < s.r + 0.6 * R)
            .collect();
        if let Some(b) = drawn.iter().find(|d| d.k == o).filter(|_| !on_it.is_empty()) {
            let white = on_it.len() >= 2 || on_it.iter().any(|s| s.hot);
            let tone = if white { bold::WHITE } else { HEAT };
            let flick = 0.75 + 0.25 * dmath::sin(t * 60.0);
            px::ring(img, b.s, b.rp + 3.0, 3.0, tone, 0.9 * flick);
            px::ring(img, b.s, b.rp + 5.5, 1.5, HEAT, 0.6 * flick);
            for w in 0..3u64 {
                let rise = (t * 1.7 + w as f64 * 0.37).rem_euclid(1.0);
                let x0 = b.s.0 + (w as f64 - 1.0) * b.rp * 0.55;
                let y0 = b.s.1 - b.rp * 0.95 - rise * b.rp * 1.3;
                let wisp: Vec<(f64, f64)> = (0..6)
                    .map(|i| {
                        let up = f64::from(i) * 4.5;
                        (x0 + 4.0 * dmath::sin(up * 0.35 + t * 9.0 + w as f64 * 2.0), y0 - up)
                    })
                    .collect();
                px::ink_polyline(img, &wisp, 1.8, SMOKE, (1.0 - rise).min(rise * 6.0).clamp(0.0, 1.0));
            }
            // the sizzle of each burn
            let burns: Vec<&Event> = bold::events_in(p, k, &["burn"], t - 0.16, t).collect();
            if let Some(e) = burns.last() {
                px::chips(img, b.s, t - e.t, 0.16, 6, b.rp * 1.5, (e.t * 1000.0) as u64, tone);
            }
            if on_it.len() >= 2 {
                // white-hot: rays stand round it, and a double tick on a double burn
                for j in 0..10 {
                    let a = TAU * f64::from(j) / 10.0 + t * 1.5;
                    let (r0, r1) = (b.rp * 1.35, b.rp * (1.75 + 0.3 * dmath::sin(t * 30.0 + f64::from(j))));
                    let at = |r: f64| (b.s.0 + dmath::cos(a) * r, b.s.1 + dmath::sin(a) * r);
                    px::ink_line(img, at(r0), at(r1), 2.5, bold::WHITE, 1.0);
                }
                if let Some(e) = burns
                    .last()
                    .filter(|e| burns.iter().filter(|x| x.t == e.t).count() >= 2)
                {
                    let u = (t - e.t) / 0.16;
                    for side in [-1.0, 1.0] {
                        let c = (b.s.0 + side * b.rp * 0.55, b.s.1 - b.rp * (1.3 + 0.5 * u));
                        px::ink_cross(img, c, 5.0, 2.2, CYAN, 1.0 - u);
                    }
                }
            }
        }
        // the finale: the chase light bursts where it stood
        if let Some(age) = finale {
            let u = age / FINALE_S;
            let at = lights(p, p.sim_i(t - age - 0.05), k)
                .into_iter()
                .find(|l| l.kind == 2)
                .map_or(en, |l| [l.cx, l.cz]);
            let s = bold::at(cam, at[0], at[1], 0.3);
            let ppu = bold::px_per_unit(cam, at[0], at[1], 0.3);
            // rays and a ring race out from the pool; its middle flashes white for a moment
            let grow = bold::ease_out(u);
            for j in 0..14 {
                let a = TAU * f64::from(j) / 14.0 + 0.2;
                let (r0, r1) = (ppu * (0.7 + 1.3 * grow), ppu * (1.2 + 2.2 * grow));
                let on = |r: f64| (s.0 + dmath::cos(a) * r, s.1 + dmath::sin(a) * r * 0.8);
                px::ink_line(img, on(r0), on(r1), 3.0, bold::WHITE, (2.0 * (1.0 - u)).min(1.0));
            }
            px::ink_ring(
                img,
                s,
                ppu * (0.6 + 2.8 * grow),
                3.0,
                bold::pale(p.col[k]),
                (2.0 * (1.0 - u)).min(1.0),
            );
            if age < 0.1 {
                px::disc(img, s, ppu * 0.75, bold::WHITE, 0.85 * (1.0 - age / 0.1));
            }
        }
    }

    /// A burn is a small pop on every 16th; ALTISSIMO's white-hot burns are solid hits.
    pub(super) fn bold_hit(&self, p: &PictureCtx, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "burn" => {
                let hot = lights(p, p.sim_i(e.t), self.k).iter().any(|l| l.hot && l.kind != 3);
                Some(if hot { HitSize::Solid } else { HitSize::Tap })
            }
            "phone" => Some(HitSize::Tap),
            _ => Some(by_damage),
        }
    }
}
