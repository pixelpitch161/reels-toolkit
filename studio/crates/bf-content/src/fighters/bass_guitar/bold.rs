//! BASS GUITAR with the bold effects (`picture.effects = "bold"`; the owner's picks 8a to 8d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **Pick toss** (8a): a dotted loop shows the path on the floor just before the throw; a big outlined pick
//!   spins out along it with a trail, eating the dots; it goes white as it clips the enemy and is caught with a
//!   glint.
//! - **FRET SLIDE** (8b): fret bars light up one by one along the lane, closer together toward the enemy; the
//!   ball slides over them stretched, with a streak; the frets it passed flash white behind it and fade.
//! - **HEADSTOCK SWING** (8c): a thick maple neck with frets, a dark headstock and four chrome pegs winds back
//!   with a glint, then swings with a pale smear (the fight stops it dead on the enemy); the enemy flies off
//!   with speed lines.
//! - **SLINGSHOT** (8d): two thick strings from the corner posts stretch to the ball while dashes run along the
//!   aim; the ball fires stretched with afterimages; the strings snap straight between the posts and twang.

use bf_core::dmath;
use bf_core::event::Event;
use bf_picture::bold::floor;
use bf_picture::bold::px;
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::V3;
use bf_picture::config::FPS;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::BallState;
use bf_picture::scene::{R_BALL as R, to_world};

use super::{BassGuitarLook, CHROME, DARK, FLY_Y, HOT, MAPLE, Row, how, pick_shape};
use bf_core::roster::FighterCfg;

/// A bold pick's size across (classic: 0.26).
const PICK_S: f64 = 0.58;
/// Seconds before a throw its loop shows.
const LOOP_S: f64 = 0.3;
/// Seconds the passed frets keep flashing after the slide has ended.
const FRET_GLOW_S: f64 = 0.4;
/// Seconds the slingshot's strings twang after the release.
const TWANG_S: f64 = 0.7;
/// Half the width of a fret bar.
const FRET_HALF: f64 = 0.36;

/// One painter item of the bold look.
enum BItem {
    /// A pick: x, z, spin, size across, whether it is white (it has just clipped the enemy).
    Pick(f64, f64, f64, f64, bool),
    /// A ghost of the pick's trail: x, z, spin, size across.
    Ghost(f64, f64, f64, f64),
    /// The neck: butt, tip, and the tips of the last frames (the smear), newest first.
    Neck([f64; 2], [f64; 2], Vec<[f64; 2]>),
    /// A slingshot string: from its post along these points.
    Str(Vec<[f64; 2]>),
    /// An afterimage of the fired ball: x, z, radius.
    After(f64, f64, f64),
}

/// A dash this look draws: its row, which move it is, and how many frames ago the row was last recorded (0
/// while it runs).
struct Dash {
    row: Row,
    sling: bool,
    ago: usize,
}

/// A fired slingshot: the two posts, where the ball let go, and the seconds since.
struct Fired {
    posts: [[f64; 2]; 2],
    from: [f64; 2],
    age: f64,
}

/// Where FRET SLIDE's frets sit along a lane of length `len`: closer together toward the enemy.
fn fret_spots(len: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let (mut s, mut j) = (0.35, 0);
    while s < len - 0.05 && j < 40 {
        out.push(s);
        s += (0.46 - 0.035 * j as f64).max(0.15);
        j += 1;
    }
    out
}

/// `pts` walked at even steps of `step` units.
fn resample(pts: &[[f64; 2]], step: f64) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    let mut need = step * 0.5;
    for w in pts.windows(2) {
        let (dx, dz) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        let len = dmath::hypot(dx, dz);
        if len < 1e-9 {
            continue;
        }
        let mut at = need;
        while at <= len {
            out.push([w[0][0] + dx * at / len, w[0][1] + dz * at / len]);
            at += step;
        }
        need = at - len;
    }
    out
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// The beats FRET SLIDE and HEADSTOCK wind up for.
    wind: (f64, f64),
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        let ab = &cfg.raw["abilities"];
        Nums {
            wind: (
                ab["unlock1"]["wind_beats"].as_f64().unwrap_or(0.5),
                ab["unlock2"]["wind_beats"].as_f64().unwrap_or(0.75),
            ),
        }
    }
}

impl BassGuitarLook {
    /// The path pick `idx` flies from recorded frame `i` on: its recorded spots, for as long as it stays in the
    /// air (its loop only runs forward).
    fn pick_path(&self, p: &PictureCtx, i: usize, idx: f64) -> Vec<[f64; 2]> {
        let mut out = Vec::new();
        let mut last_u = -1.0;
        for j in i..(i + 90).min(p.run.nsim) {
            let Some(r) = self.rows_at(p, j).into_iter().find(|r| r.ty == 0 && r.x1 == idx) else {
                if out.is_empty() && j < i + 3 {
                    continue;
                }
                break;
            };
            if r.z1 < last_u {
                break;
            }
            last_u = r.z1;
            out.push([r.x0, r.z0]);
        }
        out
    }

    /// The dash that is running at recorded frame `i`, or that ended within `back` frames.
    fn dash(&self, p: &PictureCtx, i: usize, t: f64, back: usize) -> Option<Dash> {
        let sling = bold::since(p, self.k, &["fret_slide", "slingshot"], t, 8.0).map(|(_, e)| e.kind == "slingshot")?;
        (0..=back.min(i)).find_map(|ago| {
            self.rows_at(p, i - ago)
                .into_iter()
                .find(|r| r.ty == 1)
                .map(|row| Dash { row, sling, ago })
        })
    }

    /// The slingshot that has let go less than [`TWANG_S`] ago.
    fn fired(&self, p: &PictureCtx, i: usize, t: f64) -> Option<Fired> {
        let start = self.slings.iter().rev().find(|&&(ts, landed)| !landed && ts <= t)?.0;
        if t - start > 6.0 {
            return None;
        }
        let anchors = |j: usize| self.rows_at(p, j).into_iter().find(|r| r.ty == 3);
        if anchors(i).is_some() {
            return None;
        }
        let i0 = p.sim_i(start);
        let last = (i0..i).rev().take(120).find(|&j| anchors(j).is_some())?;
        let r = anchors(last)?;
        let age = (i - last) as f64 / f64::from(FPS);
        (age < TWANG_S).then(|| Fired {
            posts: [[r.x0, r.z0], [r.x1, r.z1]],
            from: p.run.p(last, self.k),
            age,
        })
    }

    /// The ball stretches as it slides, and more as the slingshot fires it.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64) {
        if kk != self.k || !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return;
        }
        let i = p.sim_i(t);
        if let Some(d) = self.dash(p, i, t, 0)
            && d.row.a > 0.5
            && d.row.a < 1.5
        {
            let k = if d.sling { 0.3 } else { 0.2 };
            st.sx = Some(1.0 + k);
            st.sy = Some(1.0 - k * 0.7);
        }
    }

    /// The light on the canvas: the slide's lane and the slingshot's aim.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight || c.t >= p.t_hs {
            return;
        }
        let k = self.k;
        let Some(d) = self.dash(p, c.i, c.t, 0) else {
            return;
        };
        let tint = bold::light(p.col[k]);
        let me = c.fs[k].p;
        let foe = c.fs[1 - k].p;
        if d.row.a > 2.5 {
            // drawing back: dashes run from the ball along the aim
            floor::dashes(
                ic,
                c,
                me,
                foe,
                0.1,
                0.35,
                c.t * 6.0,
                bold::light(HOT),
                0.5 + 0.5 * self.tension(p, c.i),
            );
        } else if d.row.a < 1.5 {
            let (a, b) = ([d.row.x0, d.row.z0], [d.row.x1, d.row.z1]);
            floor::lane(ic, c, a, b, FRET_HALF + 0.08, tint, if d.sling { 0.35 } else { 0.5 });
        }
    }

    /// How taut the slingshot's strings are at recorded frame `i` (0 to 1).
    fn tension(&self, p: &PictureCtx, i: usize) -> f64 {
        self.rows_at(p, i).iter().find(|r| r.ty == 3).map_or(0.0, |r| r.a)
    }

    /// Crisp marks on the canvas, under the balls: the picks' dotted loops, the frets and the aim's arrow.
    pub(super) fn bold_canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight || f.t >= p.t_hs || !p.run.has("PICK") {
            return;
        }
        let k = self.k;
        let col = p.col[k];
        let dot = bold::pale(col);
        let at = |q: [f64; 2]| bold::at(f.cam, q[0], q[1], 0.01);
        // the loop of the throw that is coming, appearing along its length
        if let Some((u, e)) = bold::until(p, k, &["pick"], f.t, LOOP_S) {
            let i_e = p.sim_i(e.t);
            for idx in 0..e.i(3).clamp(0, 3) {
                let dots = resample(&self.pick_path(p, i_e, idx as f64), 0.24);
                // the whole loop of the first pick; of the others in a fan only how they leave
                let part = if idx == 0 { 1.0 } else { 0.3 };
                let shown = (dots.len() as f64 * part * bold::ease_out(u * 1.6)).ceil() as usize;
                for q in dots.iter().take(shown) {
                    px::ink_disc(arr, at(*q), 1.4, dot, 1.0);
                }
            }
        }
        // the next stretch of the loop ahead of each pick in the air (the first pick's whole loop)
        for r in self.rows_at(p, f.i).iter().filter(|r| r.ty == 0) {
            let ahead = if r.x1 == 0.0 { usize::MAX } else { 7 };
            for q in resample(&self.pick_path(p, f.i, r.x1), 0.24).iter().skip(1).take(ahead) {
                px::ink_disc(arr, at(*q), 1.4, dot, 1.0);
            }
        }
        let Some(d) = self.dash(p, f.i, f.t, (FRET_GLOW_S * f64::from(FPS)) as usize) else {
            return;
        };
        let me = f.fs[k].p;
        if d.row.a > 2.5 {
            // the slingshot's aim: an arrowhead short of the enemy
            if d.ago == 0 {
                let foe = f.fs[1 - k].p;
                let (dx, dz) = (foe[0] - me[0], foe[1] - me[1]);
                let n = dmath::hypot(dx, dz).max(1e-9);
                if n > R * 3.2 {
                    let (ux, uz) = (dx / n, dz / n);
                    let tip = [foe[0] - ux * R * 1.25, foe[1] - uz * R * 1.25];
                    let tail = [tip[0] - ux * 0.5, tip[1] - uz * 0.5];
                    px::ink_arrow(arr, at(tail), at(tip), 2.0, 6.0, HOT, 1.0);
                }
            }
            return;
        }
        if d.sling {
            return;
        }
        // FRET SLIDE's frets
        let (a, b) = ([d.row.x0, d.row.z0], [d.row.x1, d.row.z1]);
        let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
        let len = dmath::hypot(dx, dz);
        if len < 1e-6 {
            return;
        }
        let (ux, uz) = (dx / len, dz / len);
        let (nx, nz) = (-uz, ux);
        let done = ((me[0] - a[0]) * ux + (me[1] - a[1]) * uz).max(0.0);
        let holding = d.row.a < 0.5 && d.ago == 0;
        // while it holds, the frets light up one after another
        let wind = self.nums.wind.0 * p.show.grid.beat;
        let lit_to = match bold::since(p, k, &["fret_slide"], f.t, 4.0) {
            Some((age, e)) if holding && e.i(3) == 0 => len * ((age + 0.04) / wind.max(1e-6)).clamp(0.12, 1.0),
            _ => len,
        };
        let fade = 1.0 - d.ago as f64 / (FRET_GLOW_S * f64::from(FPS));
        if !holding && d.ago == 0 && d.row.a < 1.5 {
            // the dash's streak
            px::line(arr, at(a), at(me), 7.0, dot, 0.45);
        }
        for (j, s) in fret_spots(len).into_iter().enumerate() {
            if s > lit_to {
                break;
            }
            let passed = !holding && s <= done + 0.05;
            if d.ago > 0 && !passed {
                continue;
            }
            let c = [a[0] + ux * s, a[1] + uz * s];
            let half = FRET_HALF * if passed { 1.0 + 0.25 * fade } else { 1.0 };
            let (p0, p1) = (
                at([c[0] - nx * half, c[1] - nz * half]),
                at([c[0] + nx * half, c[1] + nz * half]),
            );
            if passed {
                // the passed frets flash white, one after another, and fade
                let blink = ((f.t * 14.0 - j as f64 * 0.6).rem_euclid(2.0) - 1.0).abs();
                let c_fl = std::array::from_fn(|ch| HOT[ch] + (255.0 - HOT[ch]) * blink);
                px::ink_line(arr, p0, p1, 2.6, c_fl, fade.clamp(0.0, 1.0));
            } else {
                px::ink_line(arr, p0, p1, 2.2, if holding { HOT } else { dot }, 1.0);
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        if !f.in_fight || f.t >= p.t_hs || !p.run.has("PICK") {
            return out;
        }
        let k = self.k;
        let i = p.sim_i(f.t.min(p.t_ko));
        let depth_at = |x: f64, z: f64, y: f64| f.cam.depth(&[to_world(x, z, y)])[0];
        let rows = self.rows_at(p, i);
        let me = f.fs[k].p;
        // a pick that has just clipped the enemy is white for a moment
        let clip = bold::since(p, k, &["hit:pick"], f.t, 0.1).map(|(_, e)| [e.f(5), e.f(6)]);
        for r in rows.iter().filter(|r| r.ty == 0) {
            for back in 1..=3usize.min(i) {
                if let Some(g) = self
                    .rows_at(p, i - back)
                    .into_iter()
                    .find(|g| g.ty == 0 && g.x1 == r.x1 && g.z1 <= r.z1)
                {
                    let s = PICK_S * (0.8 - 0.17 * back as f64);
                    out.push((
                        depth_at(g.x0, g.z0, FLY_Y) + 0.03,
                        Box::new(BItem::Ghost(g.x0, g.z0, g.a, s)),
                    ));
                }
            }
            let white = clip.is_some_and(|c| dmath::hypot(c[0] - r.x0, c[1] - r.z0) < 0.8);
            let s = PICK_S * if white { 1.35 } else { 1.0 };
            out.push((
                depth_at(r.x0, r.z0, FLY_Y),
                Box::new(BItem::Pick(r.x0, r.z0, r.a, s, white)),
            ));
        }
        // the neck, with the tips of the last frames while it sweeps
        if let Some(r) = rows.iter().find(|r| r.ty == 2) {
            let (butt, tip) = ([r.x0, r.z0], [r.x1, r.z1]);
            if dmath::hypot(tip[0] - butt[0], tip[1] - butt[1]) >= 0.05 {
                let mut smear = Vec::new();
                let mut last = tip;
                // only the sweep smears, not the wind-up
                let wind = self.nums.wind.1 * p.show.grid.beat;
                let sweeping = bold::since(p, k, &["headstock"], f.t, 3.0)
                    .is_some_and(|(age, e)| e.i(3) == 1 || age >= wind - 0.02);
                for back in 1..=if sweeping { 3usize.min(i) } else { 0 } {
                    let Some(g) = self.rows_at(p, i - back).into_iter().find(|g| g.ty == 2) else {
                        break;
                    };
                    let q = [g.x1, g.z1];
                    // the held stop leaves no smear
                    if dmath::hypot(q[0] - last[0], q[1] - last[1]) < 0.3 {
                        break;
                    }
                    // the old tip as it would be round where the ball is now
                    smear.push([q[0] - g.x0 + butt[0], q[1] - g.z0 + butt[1]]);
                    last = q;
                }
                let mid = [(butt[0] + tip[0]) / 2.0, (butt[1] + tip[1]) / 2.0];
                out.push((depth_at(mid[0], mid[1], FLY_Y), Box::new(BItem::Neck(butt, tip, smear))));
            }
        }
        // the slingshot's strings: taut to the ball while it draws back
        if let Some(r) = rows.iter().find(|r| r.ty == 3) {
            let hum = 0.05 * (1.0 - r.a) * dmath::sin(f.t * 60.0);
            for post in [[r.x0, r.z0], [r.x1, r.z1]] {
                let mid = [(post[0] + me[0]) / 2.0 + hum, (post[1] + me[1]) / 2.0 - hum];
                out.push((
                    depth_at(mid[0], mid[1], FLY_Y) + 0.05,
                    Box::new(BItem::Str(vec![post, mid, me])),
                ));
            }
        } else if let Some(fd) = self.fired(p, i, f.t) {
            // let go: each string whips back to its post, twanging
            let u = fd.age / TWANG_S;
            let left = 1.0 - bold::ease_out(u * 1.15);
            for (n, post) in fd.posts.into_iter().enumerate() {
                let (dx, dz) = ((fd.from[0] - post[0]) * left, (fd.from[1] - post[1]) * left);
                let len = dmath::hypot(dx, dz);
                if len < 0.08 {
                    continue;
                }
                let (nx, nz) = (-dz / len, dx / len);
                let side = if n == 0 { 1.0 } else { -1.0 };
                let amp = side * (0.1 + 0.16 * (1.0 - u)) * dmath::cos(fd.age * 38.0);
                let pts: Vec<[f64; 2]> = (0..=8)
                    .map(|j| {
                        let s = j as f64 / 8.0;
                        let w = amp * dmath::sin(s * dmath::PI * 2.0) * (0.4 + 0.6 * s);
                        [post[0] + dx * s + nx * w, post[1] + dz * s + nz * w]
                    })
                    .collect();
                let mid = pts[4];
                out.push((depth_at(mid[0], mid[1], FLY_Y) + 0.05, Box::new(BItem::Str(pts))));
            }
        }
        // the fired ball's afterimages
        if let Some(d) = self.dash(p, i, f.t, 0)
            && d.sling
            && d.row.a > 0.5
            && d.row.a < 1.5
        {
            for (n, back) in [2usize, 4, 6].into_iter().enumerate() {
                if back > i {
                    break;
                }
                let q = p.run.p(i - back, k);
                if dmath::hypot(q[0] - me[0], q[1] - me[1]) < 0.25 {
                    continue;
                }
                out.push((
                    depth_at(q[0], q[1], R) + 0.3,
                    Box::new(BItem::After(q[0], q[1], R * (0.85 - 0.18 * n as f64))),
                ));
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let col = p.col[self.k];
        let lift = |pts: &[(f64, f64)], y: f64| -> Vec<V3> { pts.iter().map(|q| to_world(q.0, q.1, y)).collect() };
        match payload.downcast_ref::<BItem>() {
            Some(&BItem::Pick(x, z, rot, s, white)) => {
                let body = if white { bold::WHITE } else { col };
                bold::outlined_poly(fr, f.cam, &lift(&pick_shape((x, z), rot, s), FLY_Y), body);
                if !white {
                    // the bevel: a lighter pick inside
                    fr.poly(
                        f.cam,
                        &lift(&pick_shape((x, z), rot, s * 0.55), FLY_Y + 0.005),
                        bold::pale(col),
                        true,
                    );
                }
            }
            Some(&BItem::Ghost(x, z, rot, s)) => {
                fr.poly(f.cam, &lift(&pick_shape((x, z), rot, s), FLY_Y), bold::pale(col), true);
            }
            Some(BItem::Neck(butt, tip, smear)) => self.draw_bold_neck(fr, f, *butt, *tip, smear, col),
            Some(BItem::Str(pts)) => {
                // tied high on the post, down to the ball's height
                let line: Vec<V3> = pts
                    .iter()
                    .enumerate()
                    .map(|(j, q)| to_world(q[0], q[1], if j == 0 { 0.7 } else { FLY_Y * 0.8 }))
                    .collect();
                bold::thick_line(fr, f.cam, &line, CHROME, 3);
            }
            Some(&BItem::After(x, z, r)) => bold::blob(fr, f.cam, to_world(x, z, R), r, bold::deep(col)),
            None => {}
        }
    }

    /// The thick neck: the smear behind it, the fingerboard with its frets, the headstock and its four pegs.
    fn draw_bold_neck(
        &self,
        fr: &mut Frame,
        f: &FrameCtx<'_>,
        butt: [f64; 2],
        tip: [f64; 2],
        smear: &[[f64; 2]],
        col: V3,
    ) {
        let (dx, dz) = (tip[0] - butt[0], tip[1] - butt[1]);
        let len = dmath::hypot(dx, dz);
        let (ux, uz) = (dx / len, dz / len);
        let (nx, nz) = (-uz, ux);
        let w3 = |q: [f64; 2]| to_world(q[0], q[1], FLY_Y);
        let off = |s: f64, side: f64| [butt[0] + ux * s + nx * side, butt[1] + uz * s + nz * side];
        // the smear: a pale fan from the butt through where the tip has just been
        if !smear.is_empty() {
            let head = len + 0.4;
            let mut fan = vec![w3(butt), w3(off(head, 0.0))];
            for q in smear {
                let (sx, sz) = (q[0] - butt[0], q[1] - butt[1]);
                let n = dmath::hypot(sx, sz).max(1e-9);
                fan.push(w3([butt[0] + sx / n * head, butt[1] + sz / n * head]));
            }
            fr.poly(f.cam, &fan, bold::pale(col), true);
            if fan.len() >= 4 {
                let inner: Vec<V3> = [fan[0], fan[1], fan[2]].to_vec();
                fr.poly(f.cam, &inner, bold::pale(bold::pale(col)), true);
            }
        }
        // the fingerboard, a little narrower toward the headstock
        let (w0, w1) = (0.14, 0.11);
        bold::outlined_poly(
            fr,
            f.cam,
            &[w3(off(0.0, -w0)), w3(off(len, -w1)), w3(off(len, w1)), w3(off(0.0, w0))],
            MAPLE,
        );
        // frets, closer together toward the headstock
        let (mut s, mut j) = (0.22, 0);
        while s < len - 0.08 {
            let w = w0 + (w1 - w0) * s / len;
            fr.line(f.cam, &[w3(off(s, -w)), w3(off(s, w))], DARK, 1);
            s += (0.26 - 0.02 * j as f64).max(0.1);
            j += 1;
        }
        // the strings
        fr.line(f.cam, &[w3(off(0.0, 0.0)), w3(off(len, 0.0))], CHROME, 1);
        // the headstock: a dark paddle, grown with the neck
        let h = (0.42 * len / 1.3).clamp(0.12, 0.42);
        bold::outlined_poly(
            fr,
            f.cam,
            &[
                w3(off(len - 0.02, -w1)),
                w3(off(len + h * 0.35, -w1 * 1.9)),
                w3(off(len + h, -w1 * 1.5)),
                w3(off(len + h, w1 * 0.9)),
                w3(off(len - 0.02, w1)),
            ],
            DARK,
        );
        // four chrome pegs in a line along its long side
        for n in 0..4 {
            let s = len + h * (0.28 + 0.22 * n as f64);
            bold::blob(fr, f.cam, w3(off(s, -w1 * 2.2)), 0.055, CHROME);
        }
    }

    /// On the finished picture: the catch's glint, the wind-up's glint on the headstock, the struck enemy's
    /// speed lines.
    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t >= p.t_hs || !p.run.has("PICK") {
            return;
        }
        let k = self.k;
        let i = p.sim_i(f.t.min(p.t_ko));
        // a pick that came home in the last frames: a glint on the ball
        if let Some(d) = drawn.iter().find(|d| d.k == k) {
            for back in 1..=4usize.min(i) {
                let gone = |j: usize, idx: f64| !self.rows_at(p, j).iter().any(|r| r.ty == 0 && r.x1 == idx);
                let caught = self
                    .rows_at(p, i - back)
                    .iter()
                    .any(|r| r.ty == 0 && r.z1 > 0.85 && gone(i - back + 1, r.x1));
                if caught {
                    let u = (back - 1) as f64 / 4.0;
                    let c = (d.s.0 + d.rp * 0.55, d.s.1 - d.rp * 0.75);
                    px::glint(img, c, d.rp * (0.45 + 0.5 * (1.0 - u)), HOT, 1.0 - 0.5 * u);
                    break;
                }
            }
        }
        // the wound-back neck glints on its headstock before it swings
        if let Some(r) = self.rows_at(p, i).iter().find(|r| r.ty == 2)
            && let Some((age, e)) = bold::since(p, k, &["headstock"], f.t, 3.0)
            && e.i(3) == 0
        {
            let wind = self.nums.wind.1 * p.show.grid.beat;
            let u = age / wind.max(1e-6);
            if (0.45..1.0).contains(&u) {
                let (dx, dz) = (r.x1 - r.x0, r.z1 - r.z0);
                let n = dmath::hypot(dx, dz).max(1e-9);
                let c = bold::at(f.cam, r.x1 + dx / n * 0.3, r.z1 + dz / n * 0.3, FLY_Y);
                let g = dmath::sin((u - 0.45) / 0.55 * dmath::PI);
                px::glint(img, c, 4.0 + 9.0 * g, bold::WHITE, 1.0);
            }
        }
        // the clubbed enemy flies off: speed lines behind it
        if let Some((age, _)) = bold::since(p, k, &["hit:headstock", "hit:slingshot"], f.t, 0.3)
            && i >= 3
            && let Some(d) = drawn.iter().find(|d| d.k == 1 - k)
        {
            let (now, was) = (p.run.p(i, 1 - k), p.run.p(i - 3, 1 - k));
            let (dx, dz) = (now[0] - was[0], now[1] - was[1]);
            let n = dmath::hypot(dx, dz);
            if n > 0.2 {
                let a = bold::at(f.cam, now[0], now[1], R);
                let b = bold::at(f.cam, now[0] - dx / n, now[1] - dz / n, R);
                let (sx, sy) = (b.0 - a.0, b.1 - a.1);
                let sn = dmath::hypot(sx, sy).max(1e-9);
                let (ux, uy) = (sx / sn, sy / sn);
                let fade = 1.0 - age / 0.3;
                for side in [-0.6, 0.0, 0.6] {
                    let o = (d.s.0 - uy * d.rp * side, d.s.1 + ux * d.rp * side);
                    let near = d.rp * (1.25 - 0.3 * side.abs());
                    let far = near + d.rp * (1.1 + 0.6 * (1.0 - side.abs())) * fade;
                    px::ink_line(
                        img,
                        (o.0 + ux * near, o.1 + uy * near),
                        (o.0 + ux * far, o.1 + uy * far),
                        1.6,
                        bold::WHITE,
                        fade,
                    );
                }
            }
        }
    }

    /// The club and the slingshot are heavy whatever the handicap; a landed slide is at least a solid hit.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match how(e) {
            "headstock" | "slingshot" => Some(HitSize::Heavy),
            "fret_slide" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}
