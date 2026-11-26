//! PIANO with the bold effects (`picture.effects = "bold"`; the owner's picks 26a to 26d of the 2026-11-24 review).
//! The hooks of `look.rs` hand over to these when [`PictureCtx::bold`] is set; without it nothing here runs.
//!
//! - **PIANO ROLL** (26a): the paper unrolls from its spool as the fight starts, an inked strip with sprocket
//!   holes down both edges and big punch holes scrolling to a thick brass bar; a fired hole flashes a short chute
//!   of light into the ring. The shots are outlined keys, an ivory one with its notch and a bigger ebony one with
//!   a light rim, on a streak; a black key's hit pops a size bigger.
//! - **SUSTAIN** (26b): three brass pedals show under the ball and the right one goes down and lights gold; the
//!   keys fly out, lift and hang turning inside gold rings (one on the floor, one round the key); before they go
//!   each turns to the enemy along a dashed gold line, and they dart in on gold streaks.
//! - **GLISSANDO** (26c): a fan of keyboard opens round the ball toward the enemy and a lit key sweeps along it
//!   as each of the 15 keys leaves; the ones that land pop with a ring and ivory chips.
//! - **88 KEYS** (26d): a ring of 40 keys builds round the ball in a run, bursts outward as the roll fires, and a
//!   bar of light sweeps the ring with the row of keys.

use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_picture::bold::px;
use bf_picture::bold::{self, GOLD, HitSize, INK, WHITE, floor};
use bf_picture::camera::{Cam, V3};
use bf_picture::config::bayer_full;
use bf_picture::look::{CanvasCtx, DrawnBall, FrameCtx, Payload, Pic};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::scene::{A, CANVAS_RIM, R_BALL as R, to_world};

use super::{BAR, BRASS, HOLE, KEY_B, KEY_W, Kind, LAT, PAPER, PAPER_DK, PianoLook, Shot, WOOD, int, is_black, num};
use bf_core::roster::FighterCfg;

/// The bold paper is wider: it runs from the bar out to a spool at the apron's edge.
const DEPTH: f64 = 0.49;
const SPOOL: f64 = BAR + DEPTH + 0.035;
/// Paper between two rows of the bold roll (the holes are bigger, so the rows sit further apart).
const ROWGAP: f64 = 0.2;
/// Half a bold hole along the scroll, and across.
const HOLE_L: f64 = 0.064;
const HOLE_W: f64 = 0.088;
/// Seconds the paper takes to unroll before the fight starts.
const UNROLL_S: f64 = 0.6;
/// Seconds a fired hole's chute of light lasts.
const CHUTE_S: f64 = 0.16;
/// Seconds before a hanging key flies on that it turns to the enemy.
const AIM_S: f64 = 0.2;
/// The height a hanging key lifts to.
const HANG_Y: f64 = 0.62;
/// Seconds GLISSANDO's keyboard takes to open before the first key leaves, and to fold afterwards.
const FAN_IN_S: f64 = 0.22;
const FAN_OUT_S: f64 = 0.18;
/// Seconds 88 KEYS' ring of keys takes to build, and to burst.
const RING_IN_S: f64 = 0.55;
const RING_OUT_S: f64 = 0.32;
/// Seconds an arc key's pop lasts.
const POP_S: f64 = 0.28;

/// One painter item of the bold look: a key, with whether it darts (a released SUSTAIN mine) and, for a hanging
/// key about to go, the angle to the enemy.
struct BKey {
    s: Shot,
    dart: bool,
    aim: Option<f64>,
}

/// A key-sized quad in the world: centred on `(x, z)` at height `y`, `len` long along `ang` and `wid` across.
fn quad(x: f64, z: f64, ang: f64, len: f64, wid: f64, y: f64) -> [V3; 4] {
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    let pt = |sd: f64, sn: f64| {
        to_world(
            x + sd * d[0] * len / 2.0 + sn * n[0] * wid / 2.0,
            z + sd * d[1] * len / 2.0 + sn * n[1] * wid / 2.0,
            y,
        )
    };
    [pt(1.0, 1.0), pt(1.0, -1.0), pt(-1.0, -1.0), pt(-1.0, 1.0)]
}

/// A flying key that reads as a key: an ivory one with the notch a black key sits in, or a bigger ebony one with
/// a light rim in `edge` and a gloss line.
#[allow(clippy::too_many_arguments)]
fn draw_key(fr: &mut Frame, cam: &Cam, x: f64, z: f64, ang: f64, y: f64, black: bool, edge: V3) {
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    if black {
        let (len, wid) = (0.56, 0.23);
        fr.poly(cam, &quad(x, z, ang, len + 0.09, wid + 0.09, y), edge, true);
        fr.poly(cam, &quad(x, z, ang, len, wid, y), KEY_B, true);
        let gloss = [
            to_world(x - d[0] * len * 0.3, z - d[1] * len * 0.3, y),
            to_world(x + d[0] * len * 0.3, z + d[1] * len * 0.3, y),
        ];
        fr.line(cam, &gloss, [150.0, 150.0, 176.0], 1);
    } else {
        let (len, wid) = (0.46, 0.19);
        bold::outlined_poly(fr, cam, &quad(x, z, ang, len, wid, y), KEY_W);
        // the notch at the back where a black key sits
        let (cx, cz) = (
            x - d[0] * len * 0.24 + n[0] * wid * 0.27,
            z - d[1] * len * 0.24 + n[1] * wid * 0.27,
        );
        fr.poly(cam, &quad(cx, cz, ang, len * 0.5, wid * 0.44, y), INK, true);
    }
}

/// The four screen corners of a key of a keyboard bent round `(x, z)` at height `y`: from radius `r0` to `r1`,
/// `half` radians to either side of `ang`.
#[allow(clippy::too_many_arguments)]
fn fan_key(cam: &Cam, x: f64, z: f64, y: f64, ang: f64, half: f64, r0: f64, r1: f64) -> [(f64, f64); 4] {
    let at = |r: f64, a: f64| bold::at(cam, x + r * dmath::cos(a), z + r * dmath::sin(a), y);
    [
        at(r0, ang - half),
        at(r1, ang - half),
        at(r1, ang + half),
        at(r0, ang + half),
    ]
}

/// The roster numbers the bold moves read, taken in `setup_as` from its `cfg`: for its own side that is
/// `p.cfg[k]`, and EUPHONIUM's copy of this look gets the copied fighter's numbers.
#[derive(Clone, Debug, Default)]
pub(super) struct Nums {
    /// GLISSANDO: its keys, its sweep in degrees and its beats.
    gliss: (f64, f64, f64),
}

impl Nums {
    /// The numbers of the roster entry `cfg`.
    pub(super) fn of(cfg: &FighterCfg) -> Nums {
        let ab = &cfg.unlock2;
        Nums {
            gliss: (
                ab.get("n").unwrap_or(15.0).max(2.0),
                ab.get("sweep").unwrap_or(150.0),
                ab.get("beats").unwrap_or(1.0),
            ),
        }
    }
}

impl PianoLook {
    /// The paper's leading edge and whether the spool is bare, as [`PianoLook::paper`], with the bold unrolling
    /// before the fight.
    fn bold_paper(&self, p: &PictureCtx, t: f64) -> (f64, bool) {
        let tf = p.show.grid.t_fight;
        if t < tf - UNROLL_S {
            return (DEPTH + 0.2, true);
        }
        if t < tf {
            return (DEPTH * (1.0 - bold::ease_out((t - (tf - UNROLL_S)) / UNROLL_S)), false);
        }
        // the classic edge is in classic paper: the same share of the wider strip
        let (edge, gone) = self.paper(p, t);
        (edge * DEPTH / super::DEPTH, gone)
    }

    /// The rows on the bold paper at the effects clock `tc`: how far out each is and its holes' lanes.
    fn bold_rows(&self, tc: f64) -> Vec<(f64, Vec<f64>)> {
        let t8 = self.t8;
        if t8 <= 0.0 {
            return Vec::new();
        }
        let j0 = (tc / t8).floor() as i64 - 1;
        let mut out = Vec::new();
        for j in j0..j0 + (DEPTH / ROWGAP) as i64 + 3 {
            let d = BAR + (j as f64 * t8 - tc) / t8 * ROWGAP;
            if d < BAR - ROWGAP * 0.4 || d > BAR + DEPTH - HOLE_L {
                continue;
            }
            if let Some(r) = self.rows.get(&j) {
                out.push((d, r.pitches.iter().map(|&m| self.lane(m as f64)).collect()));
            }
        }
        out
    }

    /// SUSTAIN's hanging shots at recording frame `i`: id -> when it flies on.
    fn holds(p: &PictureCtx, i: usize) -> BTreeMap<i64, f64> {
        let mut out = BTreeMap::new();
        if let Some(Value::List(list)) = p.run.nested("HOLD", i) {
            for h in list {
                if let Value::List(h) = h
                    && let (Some(Value::Int(id)), Some(Value::Float(rel))) = (h.first(), h.get(2))
                {
                    out.insert(*id, *rel);
                }
            }
        }
        out
    }

    /// The volley of each of this side's live shots at recording frame `i`: id -> volley.
    fn volleys(&self, p: &PictureCtx, i: usize) -> BTreeMap<i64, i64> {
        let mut out = BTreeMap::new();
        if let Some(rows) = p.run.frame_values("K", i) {
            for row in rows.as_chunks::<7>().0 {
                if !row[0].is_nan() && np::f64_to_i64(row[3]) == self.k as i64 {
                    out.insert(np::f64_to_i64(row[4]), np::f64_to_i64(row[6]));
                }
            }
        }
        out
    }

    /// Whether the row of volley `vol` fired while SUSTAIN held (its `roll` event's flag).
    fn sustained(&self, p: &PictureCtx, vol: i64) -> bool {
        let Some(&(t0, Kind::Row)) = self.vols.get(&vol) else {
            return false;
        };
        bold::events_in(p, self.k, &["roll"], t0 - 1e-6, t0 + 1e-6)
            .any(|e| int(e, 3) == Some(vol) && int(e, 5) == Some(1))
    }

    /// The live shots with what the bold look adds: whether each darts and, when it is about to fly on, when.
    fn bold_shots(&self, p: &PictureCtx, t: f64, i: usize) -> Vec<(Shot, bool, Option<f64>)> {
        let shots = self.shots(p, t, Some(i));
        if shots.is_empty() {
            return Vec::new();
        }
        let holds = Self::holds(p, i);
        let vols = self.volleys(p, i);
        shots
            .into_iter()
            .map(|s| {
                let rel = holds.get(&s.sid).copied();
                let dart =
                    s.kind == Kind::Row && rel.is_none() && vols.get(&s.sid).is_some_and(|&vol| self.sustained(p, vol));
                (s, dart, rel)
            })
            .collect()
    }

    /// The height a bold key is drawn at: a hanging one lifts and bobs.
    fn bold_y(&self, s: &Shot) -> f64 {
        match s.hang {
            Some(hang) => {
                let base = self.shot_y(s);
                base + (HANG_Y - base) * bold::ease_out(hang / 0.12) + 0.03 * dmath::sin(dmath::TAU * 2.0 * hang)
            }
            None => self.shot_y(s),
        }
    }

    pub(super) fn bold_canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        let k = self.k;
        let t = c.t;
        let rim = CANVAS_RIM;
        let col = p.col[k].map(|v| v / 255.0);
        let gold = bold::light(GOLD);
        let tc = p.fx_clock(t);
        let (edge, gone) = self.bold_paper(p, t);
        let rows = self.bold_rows(tc);
        let near = |out: f64, lat: f64| out > -0.02 && out < SPOOL + 0.06 && lat.abs() < LAT + 0.12;
        // the rows whose fire still lights the bar: (strength, their lanes)
        let lit: Vec<(f64, Vec<f64>)> = self
            .fired
            .iter()
            .map(|abs8| &self.rows[abs8])
            .filter(|r| (0.0..0.2).contains(&(tc - r.t)))
            .map(|r| {
                (
                    dmath::exp(-(tc - r.t) / 0.07),
                    r.pitches.iter().map(|&m| self.lane(m as f64)).collect(),
                )
            })
            .collect();
        let flash: f64 = self
            .runoffs
            .iter()
            .filter(|&&(t0, _)| (0.0..0.5).contains(&(tc - t0)))
            .map(|&(t0, _)| dmath::exp(-(tc - t0) / 0.18))
            .sum();
        let sus = if c.in_fight && p.run.has("ROLL") {
            p.run.row("ROLL", c.i, k).and_then(|r| r.get(1).copied()).unwrap_or(0.0)
        } else {
            0.0
        };
        let sk = (sus / 0.25).min(1.0);
        let bayer = bayer_full();
        let paint = |light: &mut V3, colr: V3, k_col: f64| {
            for ch in 0..3 {
                light[ch] *= colr[ch] / rim[ch] * k_col;
            }
        };
        for (j, light) in ic.iter_mut().enumerate() {
            let (o, la) = self.local(c.sxp[j], c.szp[j]);
            if !near(o, la) {
                continue;
            }
            let bay = bayer.px(c.xs[j], c.ys[j])[0];
            let mut add = [0.0; 3];
            let on_paper = !gone && o >= BAR + edge && o <= BAR + DEPTH && la.abs() <= LAT;
            if on_paper {
                // the row coordinate: whole numbers scroll with the rows
                let ph = (o - BAR) / ROWGAP + tc / self.t8.max(1e-6);
                let mut colr = PAPER;
                let mut k_col = if ph.rem_euclid(2.0) < 1.0 { 0.84 } else { 0.74 };
                if la.abs() > LAT - 0.035 || o < BAR + edge + 0.022 {
                    // the inked edges of the strip
                    (colr, k_col) = (INK, 1.0);
                } else if (la.abs() - (LAT - 0.12)).abs() < 0.035 {
                    // sprocket holes down both edges
                    if (ph * 2.0).rem_euclid(1.0) < 0.5 {
                        (colr, k_col) = (HOLE, 0.9);
                    }
                } else if (la.abs() - (LAT - 0.2)).abs() < 0.012 {
                    (colr, k_col) = (PAPER_DK, 0.7);
                }
                for (d, lanes) in &rows {
                    if (o - d).abs() >= HOLE_L {
                        continue;
                    }
                    if lanes.iter().any(|&ln| (la - ln).abs() < HOLE_W) {
                        (colr, k_col) = (HOLE, 0.85);
                        // passing the bar: lit
                        if *d < BAR + 0.08 {
                            for ch in 0..3 {
                                add[ch] += col[ch] * 1.5 + 0.2;
                            }
                        }
                    }
                }
                paint(light, colr, k_col);
                for ch in 0..3 {
                    add[ch] += sk * 0.22 * gold[ch];
                }
            }
            // the brass read bar, thick, between two inked lines
            let bar_d = (o - (BAR - 0.022)).abs();
            if la.abs() <= LAT + 0.05 {
                if bar_d < 0.03 {
                    paint(light, BRASS, 1.2);
                    for ch in 0..3 {
                        add[ch] += sk * 0.5 * gold[ch];
                    }
                } else if bar_d < 0.045 {
                    paint(light, INK, 1.0);
                }
            }
            let mut gl = 0.0;
            for (a, lanes) in &lit {
                for &ln in lanes {
                    gl += a
                        * np::clip(1.0 - (la - ln).abs() / 0.16, 0.0, 1.0)
                        * np::clip(1.0 - (o - BAR).abs() / 0.2, 0.0, 1.0);
                }
            }
            if la.abs() <= LAT {
                gl += flash * 1.6 * np::clip(1.0 - (o - BAR).abs() / 0.3, 0.0, 1.0);
            }
            let g = (np::clip(gl, 0.0, 1.6) * 3.0 + bay * 0.9).floor() / 3.0;
            for ch in 0..3 {
                add[ch] += g * (col[ch] * 1.3 + 0.2);
            }
            // the supply spool: a wooden roller wound with paper, brass caps, turning with the scroll
            let ph = (o - BAR) / ROWGAP + tc / self.t8.max(1e-6);
            let sp_d = (o - SPOOL).abs();
            if sp_d < 0.035 && la.abs() <= LAT + 0.02 {
                if gone || sp_d > 0.02 {
                    paint(light, WOOD, if (ph * 2.0).rem_euclid(1.0) < 0.5 { 1.15 } else { 0.8 });
                } else {
                    paint(light, PAPER_DK, 0.8);
                }
            }
            if sp_d < 0.04 && (la.abs() - (LAT + 0.06)).abs() < 0.045 {
                paint(light, BRASS, 1.0);
            }
            for ch in 0..3 {
                light[ch] += add[ch];
            }
        }
        if !c.in_fight {
            return;
        }
        let tint = bold::light(p.col[k]);
        // a fired hole throws a short chute of light into the ring
        for r in self.fired.iter().map(|abs8| &self.rows[abs8]) {
            let age = tc - r.t;
            if !(0.0..CHUTE_S).contains(&age) {
                continue;
            }
            for &m in &r.pitches {
                let ln = self.lane(m as f64);
                let (a, b) = (self.ring_pt(0.0, ln), self.ring_pt(-0.9, ln));
                floor::lane(ic, c, [a.0, a.1], [b.0, b.1], 0.09, tint, 0.9 * (1.0 - age / CHUTE_S));
            }
        }
        let foe = c.fs[1 - k].p;
        let shots = self.bold_shots(p, t, c.i);
        // 88 KEYS: a bar of light sweeps the ring with the row of keys, and a wake behind it
        let run: Vec<f64> = shots
            .iter()
            .filter(|(s, ..)| s.kind == Kind::Runoff)
            .map(|(s, ..)| -self.local(s.x, s.z).0)
            .collect();
        if run.len() >= 3 {
            let front = run.iter().sum::<f64>() / run.len() as f64;
            let fade = (1.0 - front / (2.0 * A)).clamp(0.25, 1.0);
            let line = |inside: f64| {
                let (a, b) = (self.ring_pt(-inside, -A), self.ring_pt(-inside, A));
                ([a.0, a.1], [b.0, b.1])
            };
            let (a, b) = line((front + 0.3).min(2.0 * A));
            floor::lane(ic, c, a, b, 0.16, tint, 1.1 * fade);
            let (a, b) = line((front - 0.45).max(0.0));
            floor::lane(ic, c, a, b, 0.55, tint, 0.3 * fade);
        }
        for (s, _, rel) in &shots {
            match s.hang {
                Some(hang) => {
                    // a hanging key: a gold ring on the floor under it
                    let pulse = 0.75 + 0.25 * dmath::sin(dmath::TAU * 4.0 * hang);
                    floor::ring(ic, c, s.x, s.z, 0.27, 0.055, gold, pulse);
                    floor::disc(ic, c, s.x, s.z, 0.2, gold, 0.25);
                    // about to go: a dashed line to the enemy
                    if let Some(rel) = rel
                        && rel - t < AIM_S
                    {
                        let u = 1.0 - ((rel - t) / AIM_S).clamp(0.0, 1.0);
                        floor::dashes(ic, c, [s.x, s.z], foe, 0.05, 0.3, t * 8.0, gold, 0.4 + 0.5 * u);
                    }
                }
                None if s.kind != Kind::Runoff => {
                    for (j, light) in ic.iter_mut().enumerate() {
                        let rd = np::hypot(c.sxp[j] - s.x, c.szp[j] - s.z);
                        let spot = np::clip(1.0 - rd / 0.3, 0.0, 1.0);
                        let spot = spot * spot;
                        for ch in 0..3 {
                            light[ch] += spot * 0.35 * col[ch];
                        }
                    }
                }
                None => {}
            }
        }
    }

    pub(super) fn bold_items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let foe = f.fs[1 - self.k].p;
        self.bold_shots(p, f.t, f.i)
            .into_iter()
            // 88 KEYS' row is drawn after the grade, so the low-HP spotlight does not grey it
            .filter(|(s, ..)| s.kind != Kind::Runoff)
            .map(|(s, dart, rel)| {
                let aim = rel
                    .filter(|rel| s.hang.is_some() && rel - f.t < AIM_S)
                    .map(|_| dmath::atan2(foe[1] - s.z, foe[0] - s.x));
                let depth = f.cam.depth(&[to_world(s.x, s.z, self.bold_y(&s))])[0];
                let payload: Payload = Box::new(BKey { s, dart, aim });
                (depth, payload)
            })
            .collect()
    }

    pub(super) fn bold_draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let Some(BKey { s, dart, aim }) = payload.downcast_ref::<BKey>() else {
            return;
        };
        let k = self.k;
        let y = self.bold_y(s);
        let ang = match (s.hang, aim) {
            // about to go: it points at the enemy
            (Some(_), Some(aim)) => *aim,
            // a hanging key turns slowly
            (Some(hang), None) => hang * 3.0 + s.sid as f64,
            _ => s.ang,
        };
        // the streak back along its flight: longer and gold for a key darting in
        if s.hang.is_none() {
            let mut pts = vec![(s.x, s.z)];
            for b in 1..=(if *dart { 4 } else { 3 }) {
                if let Some(q) = self.trail_pos(p, f.t, s.sid, b) {
                    let last = pts[pts.len() - 1];
                    if dmath::hypot(q.0 - last.0, q.1 - last.1) < 0.6 {
                        pts.push(q);
                    } else {
                        break;
                    }
                }
            }
            if pts.len() > 1 {
                let world: Vec<V3> = pts.iter().map(|q| to_world(q.0, q.1, y)).collect();
                if *dart {
                    bold::thick_line(fr, f.cam, &world, GOLD, 3);
                } else {
                    fr.line(f.cam, &world, bold::pale(p.col[k]), 2);
                }
            }
        }
        let black = s.m.is_some_and(is_black);
        let edge = if s.hang.is_some() || *dart {
            GOLD
        } else {
            bold::pale(p.col[k])
        };
        draw_key(fr, f.cam, s.x, s.z, ang, y, black, edge);
    }

    pub(super) fn bold_after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        if !f.in_fight || f.t > p.t_ko {
            return;
        }
        let k = self.k;
        let t = f.t;
        let me = &f.fs[k];
        let own = p.col[k];
        // SUSTAIN: the keys hang inside gold rings, and glint as they turn to go
        for (s, _, rel) in self.bold_shots(p, t, f.i) {
            if s.kind == Kind::Runoff {
                // 88 KEYS: the row of keys sweeping the ring, slim so 40 of them stand side by side
                let y = self.shot_y(&s);
                let black = s.m.is_some_and(is_black);
                let (d0, d1) = (dmath::cos(s.ang), dmath::sin(s.ang));
                let tail = bold::at(f.cam, s.x - d0 * 0.2, s.z - d1 * 0.2, y);
                let far = bold::at(f.cam, s.x - d0 * 0.75, s.z - d1 * 0.75, y);
                px::line(img, far, tail, 1.0, bold::pale(own), 0.7);
                let body = f
                    .cam
                    .wp(&quad(s.x, s.z, s.ang, if black { 0.56 } else { 0.46 }, 0.105, y));
                if black {
                    px::polyline(
                        img,
                        &[body[0], body[1], body[2], body[3], body[0]],
                        2.6,
                        bold::pale(own),
                        1.0,
                    );
                    px::poly(img, &body, KEY_B, 1.0);
                } else {
                    px::ink_poly(img, &body, KEY_W, 1.0);
                }
                continue;
            }
            let Some(hang) = s.hang else {
                continue;
            };
            let sp = bold::at(f.cam, s.x, s.z, self.bold_y(&s));
            let go = rel.map_or(0.0, |rel| 1.0 - ((rel - t) / AIM_S).clamp(0.0, 1.0));
            let r = 12.0 + 1.5 * dmath::sin(dmath::TAU * 4.0 * hang) - 4.0 * go;
            px::ink_ring(img, sp, r, 1.6, GOLD, 1.0);
            if go > 0.0 {
                px::glint(img, (sp.0, sp.1 - r), 3.0 + 4.0 * go, WHITE, 1.0);
            }
        }
        if me.shattered {
            return;
        }
        let Some(d) = drawn.iter().find(|d| d.k == k) else {
            return;
        };
        // SUSTAIN: three pedals under the ball; the right one goes down and lights up
        let sus = if p.run.has("ROLL") {
            p.run.row("ROLL", f.i, k).and_then(|r| r.get(1).copied()).unwrap_or(0.0)
        } else {
            0.0
        };
        let wind = bold::until(p, k, &["sustain"], t, 0.25).map(|(u, _)| u);
        if sus > 0.0 || wind.is_some() {
            let a = if sus > 0.0 { 1.0 } else { wind.unwrap_or(0.0) };
            let press = bold::since(p, k, &["sustain"], t, 0.3).map_or(0.0, |(age, _)| 1.0 - age / 0.3);
            let y0 = d.s.1 + d.rp + 2.0;
            for n in 0..3 {
                let x = d.s.0 + (f64::from(n) - 1.0) * 12.0;
                let lit = n == 2 && sus > 0.0;
                let dy = if lit { 3.0 } else { 0.0 };
                let col = if lit {
                    [255.0, 236.0, 150.0]
                } else {
                    BRASS.map(|v| v * 0.7)
                };
                let tongue = [
                    (x - 2.0, y0 + dy),
                    (x + 2.0, y0 + dy),
                    (x + 5.0, y0 + 10.0 + dy),
                    (x + 3.0, y0 + 14.0 + dy),
                    (x - 3.0, y0 + 14.0 + dy),
                    (x - 5.0, y0 + 10.0 + dy),
                ];
                px::ink_poly(img, &tongue, col, a);
            }
            if sus > 0.0 {
                let c = (d.s.0 + 12.0, y0 + 12.0);
                let beat = p.show.grid.beat_pulse(t, 0.25, 0.08).clamp(0.0, 1.0);
                px::ring(img, c, 9.0 + 12.0 * (1.0 - press), 1.6, GOLD, press);
                px::ring(img, c, 9.5 + 1.5 * beat, 1.2, GOLD, 0.8);
                px::glint(img, (c.0 + 3.0, c.1 - 5.0), 4.0 + 4.0 * press + 2.0 * beat, WHITE, 1.0);
            }
        }
        self.bold_fan(p, img, f);
        self.bold_ring_of_keys(p, img, f);
        // GLISSANDO: the keys that land pop
        for e in bold::events_in(p, k, &["hit:key"], t - POP_S, t) {
            let (Some(x), Some(z)) = (num(e, 5), num(e, 6)) else {
                continue;
            };
            if !Self::arc_hit(e) {
                continue;
            }
            let u = (t - e.t) / POP_S;
            let sp = bold::at(f.cam, x, z, 0.4);
            px::ink_ring(img, sp, 4.0 + 12.0 * bold::ease_out(u), 1.6, bold::pale(own), 1.0 - u);
            px::chips(img, sp, t - e.t, POP_S, 5, 16.0, (e.t * 1000.0) as u64, KEY_W);
        }
    }

    /// GLISSANDO's keyboard: a fan of keys round the ball toward the enemy, a lit key sweeping along it as each
    /// key leaves.
    fn bold_fan(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>) {
        let k = self.k;
        let t = f.t;
        let (n, sweep, beats) = self.nums.gliss;
        let sweep = sweep * dmath::PI / 180.0;
        let dur = beats * p.show.grid.beat;
        let me = f.fs[k].p;
        // open: how far the fan has unfolded; age: seconds since the first key left; aim: its middle
        let (open, age, aim) = if let Some((age, e)) = bold::since(p, k, &["ability:arc"], t, dur + FAN_OUT_S) {
            let i0 = p.sim_i(e.t);
            let (a, b) = (p.run.p(i0, k), p.run.p(i0, 1 - k));
            let fold = ((age - dur) / FAN_OUT_S).clamp(0.0, 1.0);
            (1.0 - fold, age, dmath::atan2(b[1] - a[1], b[0] - a[0]))
        } else if let Some((u, _)) = bold::until(p, k, &["ability:arc"], t, FAN_IN_S) {
            let b = f.fs[1 - k].p;
            (bold::ease_out(u), -1.0, dmath::atan2(b[1] - me[1], b[0] - me[0]))
        } else {
            return;
        };
        let steps = n - 1.0;
        let step = sweep / steps;
        let (r0, r1) = (R + 0.14, R + 0.14 + 0.5 * open);
        for j in 0..n as usize {
            let ang = aim - sweep / 2.0 + step * j as f64;
            // seconds since this key left (negative before)
            let gone = age - dur * j as f64 / steps;
            let lit = (0.0..0.09).contains(&gone);
            let (col, push) = if lit {
                (WHITE, 0.12)
            } else if gone >= 0.0 {
                (bold::pale(p.col[k]), 0.0)
            } else {
                (KEY_W, 0.0)
            };
            let q = fan_key(f.cam, me[0], me[1], R, ang, step * 0.44, r0 + push, r1 + push);
            px::ink_poly(img, &q, col, 1.0);
            if lit {
                let tip = bold::at(
                    f.cam,
                    me[0] + (r1 + 0.2) * dmath::cos(ang),
                    me[1] + (r1 + 0.2) * dmath::sin(ang),
                    R,
                );
                px::glint(img, tip, 6.0, WHITE, 1.0);
            }
        }
        // the black keys between them
        for j in 0..n as usize - 1 {
            if [2, 6].contains(&(j % 7)) {
                continue;
            }
            let ang = aim - sweep / 2.0 + step * (j as f64 + 0.5);
            let q = fan_key(f.cam, me[0], me[1], R, ang, step * 0.24, r0, r0 + (r1 - r0) * 0.6);
            px::ink_poly(img, &q, KEY_B, 1.0);
        }
    }

    /// 88 KEYS: a ring of 40 keys builds round the ball in a run, then bursts outward as the roll fires.
    fn bold_ring_of_keys(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>) {
        let k = self.k;
        let t = f.t;
        let me = f.fs[k].p;
        let n = 40usize;
        // built: how many keys stand; out: how far the burst has gone (0 to 1)
        let (built, out) = if let Some((age, _)) = bold::since(p, k, &["runoff"], t, RING_OUT_S) {
            (n as f64, age / RING_OUT_S)
        } else if let Some((u, _)) = bold::until(p, k, &["runoff"], t, RING_IN_S) {
            (u * 1.15 * n as f64, 0.0)
        } else {
            return;
        };
        let step = dmath::TAU / n as f64;
        let fly = 2.6 * bold::ease_out(out);
        let a = 1.0 - bold::ease_in(out);
        for j in 0..n {
            if j as f64 > built {
                break;
            }
            let ang = step * j as f64 - dmath::PI / 2.0;
            let black = is_black(j as i64);
            let newest = out == 0.0 && built - (j as f64) < 2.0;
            let (r0, len) = (R + 0.2 + fly, if black { 0.3 } else { 0.42 });
            let col = if newest {
                WHITE
            } else if black {
                KEY_B
            } else {
                KEY_W
            };
            let q = fan_key(f.cam, me[0], me[1], R, ang, step * 0.4, r0, r0 + len);
            px::ink_poly(img, &q, col, a);
            if out > 0.0 && j % 2 == 0 {
                // a streak behind each key as it flies out
                let at = |r: f64| bold::at(f.cam, me[0] + r * dmath::cos(ang), me[1] + r * dmath::sin(ang), R);
                px::line(img, at(r0 - 0.5 * fly), at(r0), 1.0, bold::pale(p.col[k]), a);
            }
        }
    }

    /// Whether a `key` hit was one of GLISSANDO's: its sub-number is its step along the fan, where a row's or
    /// the runoff's is a pitch of the roll's range.
    fn arc_hit(e: &Event) -> bool {
        int(e, 8).is_some_and(|sub| (0..24).contains(&sub))
    }

    /// A black key hits a size harder than an ivory one, and GLISSANDO's keys pop.
    pub(super) fn bold_hit(&self, e: &Event, by_damage: HitSize) -> Option<HitSize> {
        if bold::how(e) != "key" {
            return Some(by_damage);
        }
        let sub = int(e, 8);
        if Self::arc_hit(e) || sub.is_some_and(|m| m >= self.reg.0 && is_black(m)) {
            Some(by_damage.max(HitSize::Solid))
        } else {
            Some(by_damage)
        }
    }
}
