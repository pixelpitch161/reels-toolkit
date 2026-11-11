//! The effects of the fight: `_fx_build` (every `reelkit.fx` effect from the run's events, once, in time order),
//! the camera shake, `events_between`, `sparks` and `shimmers` of `kit/picture.py`.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::py::{py_max, py_min};
use bf_core::rng::Generator;

use super::{PictureCtx, owner};
use crate::camera::{Cam, V3};
use crate::config::{H, W};
use crate::fx::Shake;
use crate::img::Img;
use crate::look::{Fx, Look, Pic};
use crate::np;
use crate::np::index::py_slice;
use crate::scene::{self, R_BALL as R};

/// `SHK`: a little camera shake on heavy hits: each look's `shake(e)` for its own events, then 2.5 at the K.O.
pub(super) fn shake(p: &PictureCtx, looks: &[Box<dyn Look>]) -> Shake {
    let mut shk = Shake::new(2.5, 0.09, 0.45);
    for e in &p.rec.events {
        if let Some(a) = owner(e).and_then(|k| looks[k].shake(p, e)) {
            shk.add(e.t, a);
        }
    }
    shk.add(p.t_ko, 2.5);
    shk
}

/// `up(x, z, y)`: a ring-frame point at height `y` in the world.
pub fn up(x: f64, z: f64, y: f64) -> V3 {
    scene::to_world(x, z, y)
}

/// `_fx_build()`: every effect from the run's events, once, in time order. The looks' hooks draw from the one
/// particle generator, so their order here is part of the picture: every look's `fx_setup`; the events up to the
/// K.O. (a `block` adds a dark burst, then the event's owner's look gets `fx_event`); every look's `fx_after`;
/// the ability moments on the captioned abilities; the K.O.'s burst and flare; the loser's `ko_fx`; every
/// look's `fx_end`.
pub(super) fn fx_build(p: &PictureCtx, fx: &mut Fx) {
    for l in &p.looks {
        l.fx_setup(p, fx);
    }
    for e in &p.rec.events {
        if e.t > p.t_ko + 1e-6 {
            break;
        }
        let k = owner(e);
        if e.kind == "block" {
            let col = k.map_or([255.0, 226.0, 150.0], |k| p.looks[k].block_burst());
            fx.bu.add(e.t, up(e.f(4), e.f(5), R), col, 0.6, None, false);
        }
        if let Some(k) = k {
            p.looks[k].fx_event(p, fx, e);
        }
    }
    for l in &p.looks {
        l.fx_after(p, fx);
    }
    // ability moments on the captioned abilities: for t_, who, name in STORY['caps']
    let mut caps: Vec<(f64, usize)> = Vec::new();
    for (who, list) in p.show.caps_ability.iter().enumerate() {
        caps.extend(list.iter().map(|c| (c.t0, who)));
    }
    // (the story's list is in time order; the two fighters' captions never share a time: they are 4 s apart)
    caps.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (t, who) in caps {
        fx.mo.add(t, who as i64, p.col[who], 1.0);
    }
    let pk = p.p_ko[p.lose];
    let pos = scene::to_world(pk[0], pk[1], R);
    fx.bu.add(p.t_ko, pos, p.col[p.lose], 2.0, None, true);
    fx.fl
        .add(p.t_ko, pos, p.fl_col[p.lose].map(|c| c * 0.9), 2.4, 1.1, 0.03, 0.6, 0.0);
    p.looks[p.lose].ko_fx(p, fx, pos);
    for l in &p.looks {
        l.fx_end(p, fx);
    }
}

/// `SPARK_OTHER` and its fallback: `(colour, n, speed, life)` of the sparks of an event that is not a hit.
fn spark_other(kind: &str) -> (V3, usize, f64, f64) {
    match kind {
        "parry" => ([230.0, 236.0, 255.0], 6, 2.4, 0.2),
        "block" => ([160.0, 220.0, 255.0], 12, 2.6, 0.3),
        _ => ([200.0, 196.0, 210.0], 3, 1.0, 0.15),
    }
}

impl PictureCtx {
    /// `events_between(t0, t1, kinds)`: the indices of the events with `t0 <= t <= t1` whose kind is one of
    /// `kinds`, in order.
    pub fn events_between(&self, t0: f64, t1: f64, kinds: &[&str]) -> Vec<usize> {
        let i0 = self.ev_t.partition_point(|&t| t < t0);
        let i1 = self.ev_t.partition_point(|&t| t <= t1);
        (i0..i1.max(i0))
            .filter(|&i| kinds.contains(&self.rec.events[i].kind.as_str()))
            .collect()
    }

    /// The event at index `i` of the run.
    pub fn event(&self, i: usize) -> &Event {
        &self.rec.events[i]
    }

    /// `sparks(arr, cam, t)`: the spark sprays of the last 0.32 s, one pixel a spark, and the white impact point
    /// of a look's `spark_flash` hits.
    pub(super) fn sparks(&self, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        for i in self.events_between(t - 0.32, t, &["hit", "parry", "block", "wall", "cleared", "fade"]) {
            let e = &self.rec.events[i];
            let dt = t - e.t;
            let hit = e.kind == "hit";
            let (att, x, z, (col, n, sp, life)) = if hit {
                let att = owner(e).unwrap_or(0);
                (att, e.f(5), e.f(6), self.looks[att].spark(self, e))
            } else {
                let mut spark = spark_other(&e.kind);
                if e.kind == "block" {
                    // the blocker's own block spray
                    if let Some(k) = owner(e) {
                        spark = self.looks[k].block_spark();
                    }
                }
                (0, e.f(4), e.f(5), spark)
            };
            if dt > life {
                continue;
            }
            let mut rng = Generator::new(i as u64);
            let fade = 1.0 - dt / life;
            let keep = (1.0 - fade) as f32;
            for _ in 0..n {
                let a = rng.uniform(0.0, 2.0 * dmath::PI);
                let v = sp * rng.uniform(0.4, 1.0);
                let q = [x + v * dt * dmath::cos(a), z + v * dt * dmath::sin(a)];
                let hgt = R + rng.uniform(-0.2, 0.3) + 2.5 * dt - 6.0 * dt * dt;
                let s = cam.wp(&[scene::to_world(q[0], q[1], py_max(0.02, hgt))])[0];
                let (xi, yi) = (np::f64_to_i64(s.0), np::f64_to_i64(s.1));
                if 0 <= xi && xi < W as i64 - 1 && 0 <= yi && yi < H as i64 - 1 {
                    let px = arr.px_mut(xi as usize, yi as usize);
                    for c in 0..3 {
                        // np.clip(arr[yi, xi] * (1 - fade) + np.clip(col + 60, 0, 255) * fade, 0, 255)
                        let hot = np::clip(col[c] + 60.0, 0.0, 255.0) * fade;
                        px[c] = np::clip(f64::from(px[c] * keep) + hot, 0.0, 255.0) as f32;
                    }
                }
            }
            if hit && self.looks[att].spark_flash().contains(&e.s(7)) && dt < 0.08 {
                let s = cam.wp(&[scene::to_world(x, z, R)])[0];
                let (sx, sy) = (np::f64_to_i64(s.0), np::f64_to_i64(s.1));
                let i32_of = |v: i64| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
                // arr[max(0, int(s[1]) - 1):int(s[1]) + 2, max(0, int(s[0]) - 1):int(s[0]) + 2] = 255
                let rows = py_slice(i32_of(sy.saturating_sub(1).max(0)), i32_of(sy.saturating_add(2)), H);
                let cols = py_slice(i32_of(sx.saturating_sub(1).max(0)), i32_of(sx.saturating_add(2)), W);
                for y in rows {
                    for x in cols.clone() {
                        arr.px_mut(x, y).fill(255.0);
                    }
                }
            }
        }
    }

    /// `shimmers(img, cam, t)`: heat-shimmer ripples from the fighters' rings and stomps.
    pub(super) fn shimmers(&self, img: &mut Pic, cam: &Cam, t: f64) {
        if self.show.grid.t_fight <= t && t < self.t_hs {
            let i = self.sim_i(py_min(t, self.t_ko));
            for w in self.run.waves(i) {
                self.looks[w.owner].shimmer(self, img, cam, t, w);
            }
        }
        for l in &self.looks {
            l.shimmer_after(self, img, cam, t);
        }
    }
}
