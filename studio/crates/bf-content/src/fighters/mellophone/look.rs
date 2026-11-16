//! MELLOPHONE's look (a Studio fighter: no kit look to match; design
//! `docs/superpowers/specs/2026-11-16-mellophone-fighter-design.md`). The ball is its model (a silver marching
//! mellophone built from web photos).
//!
//! What it draws, from the recorded `SLAP` and `SLAPQ`: every SLAPBACK wave as a bright arc of sound bowed the way it
//! flies, wider and hotter each echo generation, with a soft band of light under it on the canvas; a wave that lands
//! on a rope flashes there, and while its echo waits a ring pulses on the rope, tightening until the echo leaves;
//! HORNS UP's thin stabs; PRESS BOX lights all four ropes like a stadium wall; STADIUM ECHO's giant arc. It shatters
//! into silver tubing, valves and bell chips.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, py_min};
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::fx::Emit;
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, FrameCtx, Fx, Look, Payload, Spark};
use bf_picture::np;
use bf_picture::raster::Frame;
use bf_picture::ring::PictureCtx;
use bf_picture::ring::effects::up;
use bf_picture::scene::{self, A, R_BALL as R};

use crate::fighters::trumpet::TrumpetLook;

/// The hot white-gold a wave warms to with its generations.
const HOT: V3 = [255.0, 226.0, 160.0];
/// The silver of the horn (rope-flash sparks).
const SILVER: V3 = [226.0, 230.0, 238.0];
/// The height the waves fly at.
const FLY_Y: f64 = R * 0.8;

/// A hit's kind (`e[7]`), or `""`.
fn how(e: &Event) -> &str {
    match (e.kind.as_str(), e.fields.get(5)) {
        ("hit", Some(Value::Str(s))) => s,
        _ => "",
    }
}

/// The event's fighter as a side.
fn owner(e: &Event) -> usize {
    e.who().filter(|&k| k < 2).unwrap_or(0)
}

/// `BAYER_FULL[y, x]`.
fn bayer(x: usize, y: usize) -> f64 {
    BAYER[y % 4][x % 4]
}

/// A number of a recorded row.
fn num(r: &[Value], i: usize) -> Option<f64> {
    match r.get(i) {
        Some(Value::Float(x)) => Some(*x),
        Some(Value::Int(n)) => Some(*n as f64),
        _ => None,
    }
}

/// One recorded wave: `(owner, id, x, z, vx, vz, generation, half_w, kind)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WaveRow {
    owner: usize,
    x: f64,
    z: f64,
    vx: f64,
    vz: f64,
    generation: f64,
    half_w: f64,
    kind: i64,
}

impl WaveRow {
    fn of(v: &Value) -> Option<WaveRow> {
        let Value::List(r) = v else { return None };
        Some(WaveRow {
            owner: num(r, 0)? as usize,
            x: num(r, 2)?,
            z: num(r, 3)?,
            vx: num(r, 4)?,
            vz: num(r, 5)?,
            generation: num(r, 6)?,
            half_w: num(r, 7)?,
            kind: num(r, 8)? as i64,
        })
    }
}

/// One recorded echo waiting on a rope: `(owner, x, z, seconds left, generation, kind)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct EchoRow {
    owner: usize,
    x: f64,
    z: f64,
    left: f64,
}

impl EchoRow {
    fn of(v: &Value) -> Option<EchoRow> {
        let Value::List(r) = v else { return None };
        Some(EchoRow {
            owner: num(r, 0)? as usize,
            x: num(r, 1)?,
            z: num(r, 2)?,
            left: num(r, 3)?,
        })
    }
}

/// A wave's colour: its fighter's colour lifted to glow, warming toward white-gold each generation.
fn glow(col: V3, generation: f64) -> V3 {
    let base = col.map(|c| c * 0.45 + 140.0);
    let k = py_min(1.0, 0.25 + generation / 3.0);
    std::array::from_fn(|c| np::clip(base[c] * (1.0 - k) + HOT[c] * k, 0.0, 255.0))
}

/// The arc of a wave in the ring frame: `n + 1` points across its width, bowed forward along its flight.
fn arc(w: &WaveRow, n: usize) -> Vec<(f64, f64)> {
    let sp = py_max(1e-9, dmath::py_hypot(w.vx, w.vz));
    let (ux, uz) = (w.vx / sp, w.vz / sp);
    let (nx, nz) = (-uz, ux);
    (0..=n)
        .map(|j| {
            let s = 2.0 * j as f64 / n as f64 - 1.0;
            let back = w.half_w * 0.55 * s * s;
            (w.x + nx * s * w.half_w - ux * back, w.z + nz * s * w.half_w - uz * back)
        })
        .collect()
}

/// One painter item.
enum Item {
    /// A wave in flight.
    Wave(WaveRow),
    /// An echo waiting on a rope.
    Echo(EchoRow),
}

/// MELLOPHONE's look.
#[derive(Clone, Debug, Default)]
pub struct MellophoneLook {
    /// Its side.
    k: usize,
    /// Its rope landings: `(t, x, z, echoes)`.
    ropes: Vec<(f64, f64, f64, bool)>,
    /// PRESS BOX windows: `(start, end)`.
    press: Vec<(f64, f64)>,
    /// STADIUM ECHO's start, if it fired.
    stadium: Option<f64>,
    /// HORNS UP's starts.
    horns: Vec<f64>,
}

impl MellophoneLook {
    /// Its rows of `key` showing at `t`.
    fn rows(&self, p: &PictureCtx, key: &str, t: f64) -> Vec<Value> {
        if !(p.show.grid.t_fight <= t && t < p.t_hs) || !p.run.has(key) {
            return Vec::new();
        }
        let i = p.sim_i(py_min(t, p.t_ko));
        match p.run.nested(key, i) {
            Some(Value::List(rows)) => rows.clone(),
            _ => Vec::new(),
        }
    }

    fn waves(&self, p: &PictureCtx, t: f64) -> Vec<WaveRow> {
        self.rows(p, "SLAP", t)
            .iter()
            .filter_map(WaveRow::of)
            .filter(|w| w.owner == self.k)
            .collect()
    }

    fn echoes(&self, p: &PictureCtx, t: f64) -> Vec<EchoRow> {
        self.rows(p, "SLAPQ", t)
            .iter()
            .filter_map(EchoRow::of)
            .filter(|e| e.owner == self.k)
            .collect()
    }

    /// Where ball `kk` is at `t`, in the world, `y` up.
    fn at(&self, p: &PictureCtx, kk: usize, t: f64, y: f64) -> V3 {
        let q = p.run.p(p.sim_i(py_min(t, p.t_ko)), kk);
        scene::to_world(q[0], q[1], y)
    }
}

impl Look for MellophoneLook {
    /// Its shatter parts: silver tubing, the three valve caps and chips of the bell (drawn as TRUMPET's).
    fn shards(&self) -> Vec<String> {
        [("tube", 6), ("valve", 3), ("brass", 5)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shard(
        &self,
        p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        curl: f64,
    ) {
        TrumpetLook::default().draw_shard(p, arr, cam, kind, pos, rot, size, curl);
    }

    fn spark_big(&self) -> &[&str] {
        &["stadium"]
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn cfg_summary(&self) -> Option<String> {
        Some(String::new())
    }

    fn setup_as(&mut self, p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
        let mine = |e: &&Event| e.who() == Some(k);
        let ev = &p.rec.events;
        self.ropes = ev
            .iter()
            .filter(mine)
            .filter(|e| e.kind == "slap_rope")
            .map(|e| (e.t, e.f(4), e.f(5), e.i(6) != 0))
            .collect();
        self.press = ev
            .iter()
            .filter(mine)
            .filter(|e| e.kind == "press_box")
            .map(|e| (e.t, e.f(3)))
            .collect();
        self.stadium = ev.iter().filter(mine).find(|e| e.kind == "stadium_echo").map(|e| e.t);
        self.horns = ev
            .iter()
            .filter(mine)
            .filter(|e| e.kind == "horns_up")
            .map(|e| e.t)
            .collect();
    }

    fn shake(&self, _p: &PictureCtx, e: &Event) -> Option<f64> {
        match (e.kind.as_str(), how(e)) {
            ("hit", "stadium") if owner(e) == self.k => Some(0.7),
            ("hit", "slapback") if owner(e) == self.k && e.i(8) > 0 => Some(0.3),
            ("slap_rope", _) if e.who() == Some(self.k) && self.stadium.is_some_and(|t| e.t >= t) => Some(0.35),
            ("horns_stun", _) if e.i(3) == self.k as i64 => Some(0.5),
            _ => None,
        }
    }

    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let col = p.col[owner(e)];
        let g = e.i(8) as f64;
        match how(e) {
            "slapback" => (glow(col, g).map(|c| c * 0.7 + 60.0), 5 + g as usize, 1.8, 0.18),
            "stab" => (glow(col, 1.0), 4, 1.5, 0.14),
            "stadium" => (HOT, 10, 2.4, 0.24),
            _ => (col.map(|c| c * 0.6 + 100.0), 5, 1.8, 0.18),
        }
    }

    fn hit_glow(&self, _p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        match how(e) {
            "slapback" => Some((5.0 + e.i(8) as f64, 0.16)),
            "stadium" => Some((9.0, 0.24)),
            _ => None,
        }
    }

    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        let col = p.col[self.k];
        // a wave landing on a rope: a flash where it meets it, silver sparks if it will echo
        for &(t, x, z, echo) in &self.ropes {
            if t > p.t_ko {
                break;
            }
            let pos = up(x, z, FLY_Y);
            fx.fl.add(
                t,
                pos,
                glow(col, 1.0),
                if echo { 1.0 } else { 0.6 },
                0.6,
                0.02,
                0.18,
                0.0,
            );
            if echo {
                let sparks = Emit {
                    n: Some(4),
                    col: Some(vec![SILVER, glow(col, 2.0)]),
                    scale: 0.7,
                    ..Emit::default()
                };
                fx.pa.emit(t, pos, "sparks", &sparks);
            }
        }
        // HORNS UP: a bright flare at the bell
        for &t in &self.horns {
            if t > p.t_ko {
                break;
            }
            fx.fl.add(t, self.at(p, self.k, t, R), HOT, 1.4, 0.7, 0.02, 0.25, 0.0);
        }
        // STADIUM ECHO: a big flare and a burst of light
        if let Some(t) = self.stadium.filter(|&t| t <= p.t_ko) {
            let pos = self.at(p, self.k, t, R);
            fx.fl.add(t, pos, HOT, 2.6, 0.9, 0.03, 0.5, 0.0);
            let sparks = Emit {
                n: Some(24),
                spread: Some(180.0),
                col: Some(vec![HOT, SILVER, glow(col, 0.0)]),
                ..Emit::default()
            };
            fx.pa.emit(t, pos, "sparks", &sparks);
        }
    }

    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        if e.kind == "hit" && owner(e) == self.k && matches!(how(e), "slapback" | "stadium") {
            fx.bu.add(
                e.t,
                up(e.f(5), e.f(6), R * 0.6),
                glow(p.col[self.k], e.i(8) as f64),
                0.7,
                None,
                true,
            );
        }
    }

    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        let sparks = Emit {
            n: Some(26),
            spread: Some(180.0),
            speed: Some((0.8, 2.2)),
            col: Some(vec![SILVER, HOT, p.col[self.k]]),
            ..Emit::default()
        };
        fx.pa.emit(p.t_ko, pos, "sparks", &sparks);
    }

    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        if !c.in_fight {
            return;
        }
        let col = p.col[self.k];
        // a soft band of light under every wave
        for w in self.waves(p, c.t) {
            let tint = glow(col, w.generation).map(|v| v / 255.0);
            let pts = arc(&w, 6);
            let reach = w.half_w + 0.4;
            for (j, light) in ic.iter_mut().enumerate() {
                let (x, z) = (c.sxp[j], c.szp[j]);
                if (x - w.x).abs() > reach || (z - w.z).abs() > reach {
                    continue;
                }
                let d = pts
                    .iter()
                    .map(|q| np::hypot(x - q.0, z - q.1))
                    .fold(f64::INFINITY, f64::min);
                let band = np::clip(1.0 - d / 0.22, 0.0, 1.0);
                let band = (band * 3.0 + bayer(c.xs[j], c.ys[j]) * 0.9).floor() / 3.0;
                let v = band * if w.kind == 2 { 0.9 } else { 0.55 };
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
        // PRESS BOX: all four ropes lit like a stadium wall
        if let Some(&(t0, t1)) = self.press.iter().find(|&&(a, b)| a <= c.t && c.t < b) {
            let fade = np::clip(py_min((c.t - t0) / 0.2, (t1 - c.t) / 0.4), 0.0, 1.0);
            let tint = glow(col, 2.0).map(|v| v / 255.0);
            for (j, light) in ic.iter_mut().enumerate() {
                let edge = A - c.sxp[j].abs().max(c.szp[j].abs());
                if !(0.0..0.35).contains(&edge) {
                    continue;
                }
                let stripe = 0.6 + 0.4 * dmath::sin((c.sxp[j] + c.szp[j]) * 9.0 - c.t * 12.0);
                let v = (1.0 - edge / 0.35) * stripe * fade * 0.9;
                for ch in 0..3 {
                    light[ch] += v * tint[ch];
                }
            }
        }
    }

    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        if !f.in_fight {
            return Vec::new();
        }
        let mut out: Vec<(f64, Payload)> = Vec::new();
        for w in self.waves(p, f.t) {
            let depth = f.cam.depth(&[scene::to_world(w.x, w.z, FLY_Y)])[0];
            out.push((depth, Box::new(Item::Wave(w))));
        }
        for e in self.echoes(p, f.t) {
            let depth = f.cam.depth(&[scene::to_world(e.x, e.z, FLY_Y)])[0];
            out.push((depth, Box::new(Item::Echo(e))));
        }
        out
    }

    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        let col = p.col[self.k];
        match payload.downcast_ref::<Item>() {
            Some(Item::Wave(w)) => {
                // the wave and two ripples behind it, drawn a little wider than it hits so it reads at video size
                let hot = glow(col, w.generation + 1.0);
                let (outer, inner) = match w.kind {
                    1 => (3, 1),
                    2 => (8, 4),
                    _ => (5 + w.generation.min(3.0) as u32, 3),
                };
                let sp = py_max(1e-9, dmath::py_hypot(w.vx, w.vz));
                for (j, lag) in [0.0, 0.16, 0.30].iter().enumerate() {
                    if w.kind == 1 && j > 0 {
                        break;
                    }
                    let ripple = WaveRow {
                        x: w.x - w.vx / sp * lag,
                        z: w.z - w.vz / sp * lag,
                        half_w: py_max(w.half_w, 0.42) * (1.0 - 0.18 * j as f64),
                        ..*w
                    };
                    let pts: Vec<V3> = arc(&ripple, 8)
                        .iter()
                        .map(|q| scene::to_world(q.0, q.1, FLY_Y))
                        .collect();
                    let fade = 1.0 - 0.3 * j as f64;
                    if j == 0 {
                        fr.line(f.cam, &pts, col.map(|c| c * 0.6 + 70.0), outer);
                    }
                    let c = std::array::from_fn(|i| hot[i] * fade + (1.0 - fade) * col[i] * 0.5);
                    fr.line(f.cam, &pts, c, (inner as f64 * fade).max(1.0) as u32);
                }
            }
            Some(Item::Echo(e)) => {
                // a ring on the rope that tightens until the echo leaves
                let r = 0.12 + 0.5 * np::clip(e.left / 0.5, 0.0, 1.0);
                let pts: Vec<V3> = (0..=12)
                    .map(|j| {
                        let a = j as f64 * std::f64::consts::TAU / 12.0;
                        scene::to_world(e.x + r * dmath::cos(a), e.z + r * dmath::sin(a), FLY_Y)
                    })
                    .collect();
                fr.line(f.cam, &pts, glow(col, 2.0), 2);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wave_warms_with_its_generations() {
        let col = [40.0, 24.0, 244.0];
        let (g0, g3) = (glow(col, 0.0), glow(col, 3.0));
        assert!(g3[1] > g0[1]);
        assert_eq!(g3, HOT);
    }

    #[test]
    fn a_wave_arc_spans_its_width_and_bows_forward() {
        let w = WaveRow {
            owner: 0,
            x: 0.0,
            z: 0.0,
            vx: 1.0,
            vz: 0.0,
            generation: 0.0,
            half_w: 0.5,
            kind: 0,
        };
        let pts = arc(&w, 8);
        assert_eq!(pts.len(), 9);
        assert!((pts[0].1 + 0.5).abs() < 1e-9 && (pts[8].1 - 0.5).abs() < 1e-9);
        assert!(pts[4].0 > pts[0].0, "the middle leads");
    }

    #[test]
    fn recorded_rows_read() {
        let v = Value::List(vec![
            Value::Int(1),
            Value::Int(7),
            Value::Float(0.5),
            Value::Float(-1.0),
            Value::Float(3.0),
            Value::Float(4.0),
            Value::Int(2),
            Value::Float(0.32),
            Value::Int(0),
        ]);
        let w = WaveRow::of(&v).expect("a wave");
        assert_eq!((w.owner, w.generation, w.kind), (1, 2.0, 0));
        let q = Value::List(vec![
            Value::Int(0),
            Value::Float(3.0),
            Value::Float(1.0),
            Value::Float(0.2),
            Value::Int(1),
            Value::Int(0),
        ]);
        let e = EchoRow::of(&q).expect("an echo");
        assert_eq!((e.owner, e.x, e.left), (0, 3.0, 0.2));
        assert!(WaveRow::of(&Value::Int(3)).is_none());
    }
}
