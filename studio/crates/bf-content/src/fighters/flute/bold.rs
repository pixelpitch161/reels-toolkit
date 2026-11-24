//! FLUTE with the bold effects (`picture.effects = "bold"`; the owner's picks 10a to 10d of the 2026-11-24
//! review). The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here
//! runs.
//!
//! - **Snake charmer** (10a): bigger outlined baskets whose lids pop off as they land; bigger cobras that sway;
//!   before a strike the hood flares, the head draws back and glints and a lane shows the lunge; the lunge is
//!   held on the enemy for three frames, then the cobra pulls back into its sway.
//! - **FLUTTER TONGUE** (10b): the ring marks the haste itself; here two more notes flutter round the ball, and
//!   the cobras sway at double pace and flick red forked tongues.
//! - **SNAKE DANCE** (10c): a wavy charm line with notes on it plays out of the flute, a ribbon up and then a
//!   line to each cobra; the cobras leave their baskets, bigger on the floor, each with a winding track back to
//!   its basket.
//! - **PICCOLO** (10d): the ball goes thin and tall with shrill ticks over it as a warning; three thin shivering
//!   rings cross the ring; the glass cracks they leave glint.

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::rng::Generator;
use bf_picture::bold::floor::{self, Family};
use bf_picture::bold::px::{self, Surface};
use bf_picture::bold::{self, HitSize};
use bf_picture::camera::V3;
use bf_picture::config::bayer_full;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::scene::{AP, R_BALL as R, to_world};

use super::{
    BASKET_H, BASKET_R, Cobra, FluteLook, INK, SCALE_DK, SILVER, WICKER, WICKER_DK, WICKER_HI, cobra_pose, draw_basket,
    draw_cobra, int, piccolo_rd, side_of, text,
};

/// How much bigger the bold baskets and cobras are.
const BIG: f64 = 1.4;
/// Seconds before a strike its cobra winds up.
const WIND_S: f64 = 0.24;
/// Seconds a basket's lid flies after it lands.
const LID_S: f64 = 0.55;
/// Seconds before PICCOLO the flute goes thin, and seconds it takes to spring back.
const SHRILL_S: f64 = 0.4;
const SPRING_S: f64 = 0.15;
/// The charm line's and the tongues' colours.
const CHARM: V3 = [150.0, 255.0, 225.0];
const TONGUE: V3 = [255.0, 70.0, 110.0];

/// One painter item of the bold look.
enum BItem {
    Basket(Cobra),
    Cobra(Cobra),
    /// A popped lid: x, z, height, radius, how flat it is seen (1 round, 0 edge on).
    Lid(f64, f64, f64, f64, f64),
}

/// A cobra as the bold look poses it.
struct BPose {
    pts: Vec<V3>,
    head: V3,
    face: [f64; 2],
    /// Its size for `draw_cobra` (above 1: a thicker body and a wider hood).
    size: f64,
    glow: f64,
    /// The wind-up before its next strike, 0 to 1.
    wind: f64,
    /// It is in a strike.
    striking: bool,
}

/// A little note: a head, a stem and a flag, `s` the head's radius.
fn note<S: Surface + ?Sized>(img: &mut S, c: (f64, f64), s: f64, col: V3, a: f64) {
    let top = (c.0 + s * 0.8, c.1 - s * 3.2);
    px::ink_line(img, (c.0 + s * 0.8, c.1), top, 1.0, col, a);
    px::ink_line(img, top, (top.0 + s * 1.3, top.1 + s * 1.1), 1.2, col, a);
    px::ink_disc(img, c, s, col, a);
}

impl FluteLook {
    /// Whether the flute is hasted at recorded frame `i`.
    fn hasted(&self, p: &PictureCtx, in_fight: bool, i: usize, t: f64) -> bool {
        in_fight && t < p.t_ko && p.run.f("HA", i, self.k).unwrap_or(0.0) > 0.0
    }

    /// PICCOLO's warning at `t`: 0 to 1 as it nears, then back to 0 just after it fires.
    fn shrill(&self, p: &PictureCtx, t: f64) -> f64 {
        let fired =
            |e: &&Event| e.kind == "ability" && int(e, 2).and_then(side_of) == Some(self.k) && text(e, 3) == self.low.0;
        let i0 = p.ev_t.partition_point(|&te| te < t - SPRING_S);
        let i1 = p.ev_t.partition_point(|&te| te <= t + SHRILL_S);
        p.rec.events[i0..i1.max(i0)]
            .iter()
            .filter(fired)
            .map(|e| {
                if e.t > t {
                    bold::ease_in(1.0 - (e.t - t) / SHRILL_S)
                } else {
                    1.0 - (t - e.t) / SPRING_S
                }
            })
            .fold(0.0, f64::max)
    }

    /// The glass cracks PICCOLO has left that still show at `t`: where, and each one's age (the same draw as
    /// the look's `fx_event`, so the glints sit on the cracks).
    fn cracks(&self, p: &PictureCtx, t: f64) -> Vec<([f64; 2], f64)> {
        let life = 2.0 * self.beat;
        let mut out = Vec::new();
        for e in p.rec.events.iter().filter(|e| {
            e.kind == "ability" && text(e, 3) == self.low.0 && e.t <= t && t - e.t < life + 4.0 / self.low.1.max(0.1)
        }) {
            let Some(k) = int(e, 2).and_then(side_of) else {
                continue;
            };
            let at = p.run.p(p.sim_i(e.t), k);
            let mut rng = Generator::new((e.t * 10.0) as u64);
            for _ in 0..9 {
                let r = rng.uniform(1.0, 4.0);
                let a = rng.uniform(0.0, 2.0 * PI);
                let q = [at[0] + r * dmath::cos(a), at[1] + r * dmath::sin(a)];
                if q[0].abs().max(q[1].abs()) > AP - 0.1 {
                    continue;
                }
                let age = t - (e.t + (r - R) / self.low.1);
                if (0.0..life).contains(&age) {
                    out.push((q, age));
                }
            }
        }
        // the ring keeps only the newest six marks
        out.sort_by(|a, b| a.1.total_cmp(&b.1));
        out.truncate(6);
        out
    }

    /// The bold pose of cobra `c`: the look's own pose, bigger, with the wind-up before a strike and the strike
    /// held where it touched.
    fn bold_pose(&self, p: &PictureCtx, f: &FrameCtx<'_>, c: &Cobra) -> BPose {
        let (kf, t) = (self.k, f.t);
        let tk = t.min(p.t_ko);
        let dk = if t < p.t_ko { self.dancing_k(t) } else { 0.0 };
        let rise_t = self.beat * self.rise;
        let mut t_drop = c.t_drop;
        // a basket being taken back: the cobra sinks in
        if c.bk < 0.98 && t > t_drop + rise_t {
            t_drop = t - rise_t * c.bk.max(0.0);
        }
        let lv = if c.bid < 100 { self.swat_level(c.bid, tk) } else { 1.0 };
        if lv < 1.0 && t > t_drop + rise_t {
            t_drop = t - rise_t * lv;
        }
        let sw = if c.bid < 100 { self.swat_lean(c.bid, tk) } else { None };
        let mut aim = c.aim;
        if let Some((_, strength)) = sw {
            aim += PI * 0.55 * strength;
        }
        let hasted = self.hasted(p, f.in_fight, f.i, t);
        // S1: a strike that has just bitten is held where it touched
        let t_pose = if c.strike.is_some() {
            bold::hold_time(p, kf, &["fang"], t)
        } else {
            t
        };
        // hasted, it sways at double pace
        let beat = if hasted { self.beat / 2.0 } else { self.beat };
        let pose = cobra_pose(c.home, c.cp, t_pose, t_drop, rise_t, aim, c.strike, dk, beat, c.bid);
        // how far out the lunge is (the pose's own measure): a lunge is not made longer, it must end on the enemy
        let out = c.strike.map_or(0.0, |(t0, t_hit, t_end, _, _)| {
            if t_pose < t_hit {
                bold::ease_out((t_pose - t0) / (t_hit - t0).max(1e-6))
            } else {
                (1.0 - (t_pose - t_hit) / (t_end - t_hit).max(1e-6)).max(0.0)
            }
        });
        // S3: the wind-up before its next strike
        let wind = if c.strike.is_none() && sw.is_none() && t < p.t_ko {
            self.strikes
                .get(&c.bid)
                .and_then(|v| v.iter().find(|q| q.0 > tk && q.0 <= tk + WIND_S))
                .map_or(0.0, |q| 1.0 - (q.0 - tk) / WIND_S)
        } else {
            0.0
        };
        let away = dmath::hypot(c.cp[0] - c.home[0], c.cp[1] - c.home[1]) > 0.05;
        let y_base = if away { 0.04 } else { BASKET_H * 0.9 };
        let base = to_world(c.cp[0], c.cp[1], y_base);
        // out of a bigger basket, its mouth is higher
        let lift = if away {
            0.0
        } else {
            BASKET_H * 0.9 * (BIG - 1.0) * c.bk.min(1.0)
        };
        let grow = 1.0 + (BIG - 1.0) * (1.0 - out);
        let back = sub(to_world(pose.face[0], pose.face[1], 0.0), to_world(0.0, 0.0, 0.0));
        let n = pose.pts.len();
        let bigger = |q: V3, s: f64| -> V3 {
            let lean = 0.22 * wind * s * s;
            [
                base[0] + (q[0] - base[0]) * grow - back[0] * lean,
                base[1] + (q[1] - base[1]) * grow + lift + 0.06 * wind * s,
                base[2] + (q[2] - base[2]) * grow - back[2] * lean,
            ]
        };
        let pts: Vec<V3> = pose
            .pts
            .iter()
            .enumerate()
            .map(|(j, &q)| bigger(q, (j + 10).saturating_sub(n) as f64 / 9.0))
            .collect();
        let mut glow = match c.strike {
            Some(q) if t < q.1 + 0.12 => 1.0,
            _ if hasted => 0.35,
            _ => 0.5 * wind,
        };
        if let Some((since, _)) = sw
            && since < 0.12
        {
            glow = glow.max(0.8 * (1.0 - since / 0.12));
        }
        if t >= p.t_solo - 0.8 {
            glow = glow.max(0.6 * p.solo_env(t));
        }
        BPose {
            pts,
            head: bigger(pose.head, 1.0),
            face: pose.face,
            size: pose.e * c.bk.min(1.0) * (1.0 + 0.2 * (1.0 - 0.5 * out)) * (1.0 + 0.4 * wind),
            glow,
            wind,
            striking: c.strike.is_some(),
        }
    }

    /// The ball goes thin and tall as PICCOLO nears.
    pub(super) fn bold_state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64) {
        if kk != self.k || !(p.show.grid.t_fight <= t && t < p.t_ko) {
            return;
        }
        let u = self.shrill(p, t);
        if u > 0.0 {
            let shiver = 0.03 * u * dmath::sin(t * 150.0);
            st.sx = Some(1.0 - 0.3 * u + shiver);
            st.sy = Some(1.0 + 0.24 * u);
        }
    }

    /// The light on the canvas: a lane where a cobra that is winding up will lunge.
    pub(super) fn bold_canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight || c.t >= p.t_ko {
            return;
        }
        let tint = bold::light(p.col[self.k]);
        for (bid, b) in &self.baskets {
            let Some(q) = self
                .strikes
                .get(bid)
                .and_then(|v| v.iter().find(|q| q.0 > c.t && q.0 <= c.t + WIND_S))
            else {
                continue;
            };
            // from where the cobra is (its basket, or on the floor in SNAKE DANCE)
            let from = self.cobra_spot(p, c.i, *bid).unwrap_or(b.home);
            let u = 1.0 - (q.0 - c.t) / WIND_S;
            let to = [from[0] + dmath::cos(q.3) * q.4, from[1] + dmath::sin(q.3) * q.4];
            floor::lane(ic, c, from, to, 0.11, tint, 0.25 + 0.5 * u);
        }
    }

    /// Where basket `bid`'s cobra is at recorded frame `i` (`COB`), when the recording says.
    fn cobra_spot(&self, p: &PictureCtx, i: usize, bid: i64) -> Option<[f64; 2]> {
        use bf_core::event::Value;
        let Value::List(sides) = p.run.nested("COB", i)? else {
            return None;
        };
        let Value::List(mine) = sides.get(self.k)? else {
            return None;
        };
        mine.iter().find_map(|c| {
            let Value::List(c) = c else { return None };
            let num = |v: &Value| match v {
                Value::Float(x) => Some(*x),
                Value::Int(n) => Some(*n as f64),
                _ => None,
            };
            match (c.first(), c.get(3).and_then(num), c.get(4).and_then(num)) {
                (Some(Value::Int(id)), Some(x), Some(z)) if *id == bid => Some([x, z]),
                _ => None,
            }
        })
    }

    /// PICCOLO: three thin shivering rings, one after another, over a soft puff of air.
    pub(super) fn bold_canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        if w.kind != "wave" {
            return;
        }
        let rmax = if w.rmax > 0.0 { w.rmax } else { 1.0 };
        let tint = bold::light(p.col[w.owner]);
        floor::wave(ic, c, Family::Air, w, c.t, tint, 0.45);
        let bayer = bayer_full();
        for (j, light) in ic.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - w.cx, c.szp[j] - w.cz);
            let rd = piccolo_rd(dmath::hypot(dx, dz), dmath::atan2(dz, dx), c.t);
            let mut amount: f64 = 0.0;
            for (n, back) in [0.0, 0.5, 1.0].into_iter().enumerate() {
                let r = w.r - back;
                if r <= 0.1 {
                    continue;
                }
                let band = (1.0 - (rd - r).abs() / 0.13).clamp(0.0, 1.0);
                let band = (band * 3.0 + bayer.px(c.xs[j], c.ys[j])[0] * 0.9).floor() / 3.0;
                amount = amount.max(band * (1.0 - r / rmax) * (1.3 - 0.25 * n as f64));
            }
            if amount > 0.0 {
                for ch in 0..3 {
                    light[ch] += amount * tint[ch];
                }
            }
        }
    }

    /// Crisp marks on the canvas, under the balls: PICCOLO's cracks and the winding tracks of SNAKE DANCE's cobras.
    pub(super) fn bold_canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        if !f.in_fight || f.t >= p.t_ko {
            return;
        }
        let at = |q: [f64; 2]| bold::at(f.cam, q[0], q[1], 0.01);
        // PICCOLO's cracks: thin kinked lines of glass that grow as the ring passes and fade within two beats
        let life = 2.0 * self.beat;
        for (q, age) in self.cracks(p, f.t) {
            let fade = (1.0 - age / life).clamp(0.0, 1.0);
            let grow = bold::ease_out(age / 0.12);
            let seed = q[0] * 37.0 + q[1] * 91.0;
            for b in 0..5 {
                let bf = f64::from(b);
                let a = seed + bf * 1.257 + 0.4 * dmath::sin(seed * 3.0 + bf);
                let l = (0.2 + 0.18 * dmath::sin(seed * 5.0 + bf * 2.3).abs()) * grow;
                let mid = [q[0] + dmath::cos(a) * l * 0.5, q[1] + dmath::sin(a) * l * 0.5];
                let end = [q[0] + dmath::cos(a + 0.4) * l, q[1] + dmath::sin(a + 0.4) * l];
                let line = [at(q), at(mid), at(end)];
                let under: Vec<(f64, f64)> = line.iter().map(|s| (s.0, s.1 + 1.0)).collect();
                px::polyline(arr, &under, 1.0, INK, 0.4 * fade);
                px::polyline(arr, &line, 1.0, bold::WHITE, 0.9 * fade);
            }
        }
        for c in self.cobs(p, f) {
            let (dx, dz) = (c.cp[0] - c.home[0], c.cp[1] - c.home[1]);
            let far = dmath::hypot(dx, dz);
            if far > 0.2 {
                // the track it has slithered, winding back to its basket
                let (nx, nz) = (-dz / far, dx / far);
                let track: Vec<(f64, f64)> = (0..=20)
                    .map(|j| {
                        let s = j as f64 / 20.0;
                        let wv = 0.09 * dmath::sin(s * far * 7.0) * (s * (1.0 - s) * 4.0).min(1.0);
                        at([c.home[0] + dx * s + nx * wv, c.home[1] + dz * s + nz * wv])
                    })
                    .collect();
                px::polyline(arr, &track, 2.4, INK, 0.4);
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for c in self.cobs(p, f) {
            let dzb = f.cam.depth(&[to_world(c.home[0], c.home[1], 0.12)])[0];
            let basket: Payload = Box::new(BItem::Basket(*c));
            out.push((dzb, basket));
            let dzc = (dzb - 0.01).min(f.cam.depth(&[to_world(c.cp[0], c.cp[1], 0.5)])[0]);
            let cobra: Payload = Box::new(BItem::Cobra(*c));
            out.push((dzc, cobra));
            // the lid pops off as the basket lands
            let u = (f.t.min(p.t_ko) - c.t_drop - 0.06) / LID_S;
            if (0.0..1.0).contains(&u) && c.bid < 100 {
                let side = if c.bid % 2 == 0 { 1.0 } else { -1.0 };
                let (x, z) = (c.home[0] + side * 1.0 * u, c.home[1] - side * 1.0 * u);
                let y = BASKET_H * BIG + 1.5 * u * (1.0 - u) * 2.0 - 0.3 * u;
                let lid: Payload = Box::new(BItem::Lid(
                    x,
                    z,
                    y.max(0.03),
                    BASKET_R * BIG * 0.95 * (1.0 - 0.5 * bold::ease_in(u)),
                    dmath::cos(u * 9.0).abs().max(0.25),
                ));
                out.push((dzb - 0.005, lid));
            }
        }
        out
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let ngon = |cx: f64, cz: f64, y: f64, r: f64, flat: f64| -> Vec<V3> {
            (0..14)
                .map(|j| {
                    let a = 2.0 * PI * j as f64 / 14.0;
                    to_world(cx + r * dmath::cos(a), cz + r * flat * dmath::sin(a), y)
                })
                .collect()
        };
        let c = match payload.downcast_ref::<BItem>() {
            Some(BItem::Basket(c)) => {
                if c.bk > 0.02 {
                    // the outline: a dark foot and a dark rim under the bigger basket
                    let k = c.bk * BIG;
                    let (r, h) = (BASKET_R * k, BASKET_H * k);
                    fr.poly(
                        f.cam,
                        &ngon(c.home[0], c.home[1], 0.004, r * 0.86 + 0.045, 1.0),
                        INK,
                        true,
                    );
                    fr.poly(f.cam, &ngon(c.home[0], c.home[1], h - 0.004, r + 0.045, 1.0), INK, true);
                    draw_basket(fr, f.cam, f.l, c.home[0], c.home[1], k);
                }
                return;
            }
            Some(&BItem::Lid(x, z, y, r, flat)) => {
                bold::outlined_poly(fr, f.cam, &ngon(x, z, y, r, flat), WICKER_HI);
                fr.poly(f.cam, &ngon(x, z, y + 0.004, r * 0.7, flat), WICKER, true);
                fr.poly(f.cam, &ngon(x, z, y + 0.008, r * 0.4, flat), WICKER_DK, true);
                bold::blob(fr, f.cam, to_world(x, z, y + 0.05), 0.05, WICKER_HI);
                return;
            }
            Some(BItem::Cobra(c)) => c,
            None => return,
        };
        let pose = self.bold_pose(p, f, c);
        draw_cobra(
            fr,
            f.cam,
            f.l,
            &pose.pts,
            pose.head,
            pose.face,
            pose.size,
            p.col[self.k],
            pose.glow,
        );
        // a thicker body over the look's own, banded the same way, up to the neck
        let nseg = pose.pts.len() - 1;
        if pose.size > 0.3 && nseg >= 3 {
            let w = 0.19 * bold::px_per_unit(f.cam, c.cp[0], c.cp[1], 0.5) * pose.size.min(1.0);
            let width = |j: usize| ((w * (0.6 + 0.4 * j as f64 / nseg as f64)).round() as u32).clamp(3, 12);
            for j in 0..nseg - 1 {
                fr.line(f.cam, &pose.pts[j..j + 2], INK, width(j) + 2);
            }
            for j in 0..nseg - 1 {
                let base = if j % 2 == 0 { SILVER } else { SCALE_DK.map(|v| v * 1.45) };
                let lit = base.map(|v| (v * (1.0 + 0.25 * pose.glow) + 30.0 * pose.glow).min(255.0));
                fr.line(f.cam, &pose.pts[j..j + 2], lit, width(j));
            }
        }
        // FLUTTER TONGUE: a forked red tongue flicks out
        if pose.size > 0.6
            && !pose.striking
            && self.hasted(p, f.in_fight, f.i, f.t)
            && (f.t * 9.0 + c.bid as f64 * 0.37).rem_euclid(1.0) < 0.6
        {
            let fa = sub(to_world(pose.face[0], pose.face[1], 0.0), to_world(0.0, 0.0, 0.0));
            let sd = [-fa[2], 0.0, fa[0]];
            let along = |d: f64, s: f64| -> V3 {
                [
                    pose.head[0] + fa[0] * d + sd[0] * s,
                    pose.head[1] - 0.02,
                    pose.head[2] + fa[2] * d + sd[2] * s,
                ]
            };
            fr.line(f.cam, &[along(0.14, 0.0), along(0.34, 0.0)], TONGUE, 2);
            fr.line(f.cam, &[along(0.34, 0.0), along(0.44, 0.07)], TONGUE, 1);
            fr.line(f.cam, &[along(0.34, 0.0), along(0.44, -0.07)], TONGUE, 1);
        }
    }

    /// On the finished picture: the wind-up's glint on a hood, FLUTTER TONGUE's notes, PICCOLO's shrill ticks
    /// and the glints on its cracks.
    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t >= p.t_ko {
            return;
        }
        let k = self.k;
        let me = drawn.iter().find(|d| d.k == k);
        // SNAKE DANCE: the charm line plays out of the flute, a travelling wave with notes on it
        let dk = self.dancing_k(f.t);
        let dance = self.dances.iter().rev().find(|d| d.0 <= f.t).map(|d| f.t - d.0);
        let wavy = |from: (f64, f64), to: (f64, f64), amp: f64, shown: f64| -> Vec<(f64, f64)> {
            let (dx, dy) = (to.0 - from.0, to.1 - from.1);
            let len = dmath::hypot(dx, dy).max(1e-9);
            let n = ((len / 3.0) as usize).clamp(6, 60);
            (0..=((n as f64 * shown.clamp(0.0, 1.0)) as usize))
                .map(|j| {
                    let s = j as f64 / n as f64;
                    let wv = amp * dmath::sin(s * len * 0.28 - f.t * 13.0) * (s * 5.0).min(1.0);
                    (from.0 + dx * s - dy / len * wv, from.1 + dy * s + dx / len * wv)
                })
                .collect()
        };
        if let (Some(d), Some(age)) = (me, dance)
            && dk > 0.02
        {
            let top = (d.s.0, d.s.1 - d.rp * 0.9);
            // the opening flourish: a ribbon rising out of the flute
            if age < 0.8 {
                let a = (1.0 - (age - 0.55) / 0.25).clamp(0.0, 1.0);
                for side in [-1.0, 1.0] {
                    let to = (top.0 + side * d.rp * 2.0, top.1 - d.rp * 2.1);
                    let line = wavy(top, to, 4.0, bold::ease_out(age / 0.3));
                    px::ink_polyline(img, &line, 1.6, CHARM, a);
                    if let Some(&tip) = line.last() {
                        note(img, (tip.0, tip.1 - 4.0), 2.2, bold::WHITE, a);
                    }
                }
            }
            // and a line to each cobra it is charming across the floor
            for c in self.cobs(p, f) {
                if dmath::hypot(c.cp[0] - c.home[0], c.cp[1] - c.home[1]) < 0.15 {
                    continue;
                }
                let pose = self.bold_pose(p, f, c);
                let head = f.cam.wp(&[pose.head])[0];
                let line = wavy(top, head, 3.0, bold::ease_out(age / 0.35));
                px::ink_polyline(img, &line, 1.4, CHARM, dk.min(1.0));
                let s = (f.t * 0.9 + 0.31 * c.bid as f64).rem_euclid(1.0);
                if let Some(&at) = line.get((s * line.len() as f64) as usize) {
                    note(img, (at.0, at.1 - 4.0), 1.9, bold::WHITE, dk.min(1.0));
                }
            }
        }
        // a cobra about to strike: a glint on its flared hood
        for c in self.cobs(p, f) {
            let pose = self.bold_pose(p, f, c);
            if pose.wind > 0.3 && pose.size > 0.5 {
                let s = f.cam.wp(&[pose.head])[0];
                let g = dmath::sin((pose.wind - 0.3) / 0.7 * PI);
                px::glint(img, (s.0, s.1 - 3.0), 3.0 + 7.0 * g, bold::WHITE, 1.0);
            }
        }
        // FLUTTER TONGUE: two more notes flutter round the ball, opposite each other
        if let Some(d) = me
            && self.hasted(p, f.in_fight, f.i, f.t)
        {
            for q in 0..2 {
                let a = -f.t * 5.0 + PI * q as f64 + 1.0;
                let flutter = 2.5 * dmath::sin(f.t * 38.0 + 2.0 * q as f64);
                let c = (
                    d.s.0 + dmath::cos(a) * d.rp * 1.75,
                    d.s.1 + dmath::sin(a) * d.rp * 0.8 - d.rp * 0.2 + flutter,
                );
                note(img, c, 1.9, bold::pale(p.col[k]), 1.0);
            }
        }
        // PICCOLO's warning: shrill ticks stand over the thin ball
        let u = self.shrill(p, f.t);
        if let Some(d) = me
            && u > 0.05
        {
            for q in -2..=2 {
                let qf = f64::from(q);
                let x = d.s.0 + qf * d.rp * 0.34 + 1.2 * dmath::sin(f.t * 120.0 + qf);
                let top = d.s.1 - d.rp * (1.55 + 0.75 * u - 0.12 * qf.abs());
                let len = d.rp * (0.35 + 0.35 * u) * (1.0 - 0.2 * qf.abs());
                px::ink_line(img, (x, top), (x, top - len), 1.0, bold::WHITE, u.min(1.0));
            }
            px::glint(
                img,
                (d.s.0, d.s.1 - d.rp * 1.35),
                3.0 + 5.0 * u,
                bold::WHITE,
                u.min(1.0),
            );
        }
        // the cracks glint, each in its own time
        let life = 2.0 * self.beat;
        for (j, (q, age)) in self.cracks(p, f.t).into_iter().enumerate() {
            let tw = dmath::sin(age * 13.0 + j as f64 * 2.1);
            let fade = (1.0 - age / life).clamp(0.0, 1.0);
            if tw > 0.0 || age < 0.12 {
                let r = if age < 0.12 { 7.0 } else { 2.0 + 4.5 * tw };
                px::glint(
                    img,
                    bold::at(f.cam, q[0], q[1], 0.02),
                    r,
                    bold::WHITE,
                    (0.4 + 0.6 * fade).min(1.0),
                );
            }
        }
    }

    /// A bite pops as a solid hit; PICCOLO's wave is heavy whatever the handicap.
    pub(super) fn bold_hit(e: &Event, by_damage: HitSize) -> Option<HitSize> {
        match text(e, 7) {
            "wave" => Some(HitSize::Heavy),
            "fang" => Some(by_damage.max(HitSize::Solid)),
            _ => Some(by_damage),
        }
    }
}

/// `a - b`.
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
