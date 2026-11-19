//! The shards: the loser shatters at the K.O. `_shards`, `part_time` and `parts_at` of `kit/picture.py`, and the
//! drawing of the glass shards in `render` (a look draws its own parts).

use bf_core::dmath;
use bf_core::rng::Generator;

use super::{PictureCtx, layer_blend, screen};
use crate::camera::{Cam, V3};
use crate::config::{H, W, ease};
use crate::img::Img;
use crate::np;
use crate::scene::{self, A, R_BALL as R};

/// `T.SLOWMO`: the seconds of slow motion after the hit-stop.
pub const SLOWMO: f64 = 0.7;
/// `T.SLOWMO_RATE`: the particle clock's rate during it.
pub const SLOWMO_RATE: f64 = 0.25;

/// One part of the shatter as `_shards` makes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// `'glass'`, or one of the loser's look's kinds.
    pub kind: String,
    /// Where it starts: ring-frame x, height, ring-frame z.
    pub p: V3,
    pub v: V3,
    pub rot: f64,
    pub w: f64,
    pub size: f64,
    pub curl: f64,
}

/// One part at a time, a row of `parts_at(t)`: `(kind, pos, rot, size, curl, alpha)`.
#[derive(Clone, Debug, PartialEq)]
pub struct PartAt<'a> {
    pub kind: &'a str,
    /// Ring-frame x, height, ring-frame z.
    pub p: V3,
    pub rot: f64,
    pub size: f64,
    pub curl: f64,
    pub alpha: f64,
}

/// `_shards()`: the parts (24 glass shards, then the loser's look's kinds) drawn from
/// `np.random.default_rng(shard_seed)`, and each part's flight integrated at 120 Hz over 7 s of particle time
/// (gravity, the canvas's bounce, friction, the ropes as soft walls): `traj[j][i] = (x, y, z, rot)`.
pub(super) fn shards(shard_seed: u64, p0: [f64; 2], look_kinds: &[String]) -> (Vec<Part>, Vec<Vec<[f64; 4]>>) {
    let mut rng = Generator::new(shard_seed);
    let kinds = std::iter::repeat_n("glass", 24).chain(look_kinds.iter().map(String::as_str));
    let mut parts = Vec::new();
    for kind in kinds {
        let a = rng.uniform(0.0, 2.0 * dmath::PI);
        let sp = if kind == "glass" {
            rng.uniform(1.6, 4.2)
        } else {
            rng.uniform(1.0, 3.0)
        };
        // p = np.array([p0[0], R, p0[1]], float) + rng.normal(0, 0.12, 3) * [1, 0.6, 1]
        let mut jit = [0.0; 3];
        rng.fill_normal(0.0, 0.12, &mut jit);
        let p = [p0[0] + jit[0] * 1.0, R + jit[1] * 0.6, p0[1] + jit[2] * 1.0];
        // v = np.array([sp * math.cos(a), rng.uniform(1.5, 3.8), sp * math.sin(a)])
        let vx = sp * dmath::cos(a);
        let vy = rng.uniform(1.5, 3.8);
        let vz = sp * dmath::sin(a);
        let rot = rng.uniform(0.0, 2.0 * dmath::PI);
        let w = rng.uniform(-9.0, 9.0);
        let size = rng.uniform(0.10, 0.22);
        let curl = rng.uniform(-1.0, 1.0);
        parts.push(Part {
            kind: kind.to_string(),
            p,
            v: [vx, vy, vz],
            rot,
            w,
            size,
            curl,
        });
    }
    let dt = 1.0 / 120.0;
    let n = (7.0 / dt) as usize + 1;
    let wall = A - 0.05;
    let traj = parts
        .iter()
        .map(|q| {
            let (mut p, mut v, mut rot, mut w) = (q.p, q.v, q.rot, q.w);
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                out.push([p[0], p[1], p[2], rot]);
                v[1] -= 9.0 * dt;
                for c in 0..3 {
                    p[c] += v[c] * dt;
                }
                rot += w * dt;
                if p[1] < 0.02 {
                    p[1] = 0.02;
                    v[1] = -v[1] * 0.3;
                    v[0] *= 0.7;
                    v[2] *= 0.7;
                    w *= 0.6;
                }
                for ax in [0, 2] {
                    if p[ax].abs() > wall {
                        p[ax] = wall.copysign(p[ax]);
                        v[ax] = -v[ax] * 0.4;
                    }
                }
            }
            out
        })
        .collect();
    (parts, traj)
}

impl PictureCtx {
    /// `part_time(t)`: the particle clock: slow motion for the first [`SLOWMO`] seconds after the hit-stop, then
    /// real time.
    pub fn part_time(&self, t: f64) -> f64 {
        let u = t - self.t_hs;
        if u < 0.0 {
            0.0
        } else if u < SLOWMO {
            u * SLOWMO_RATE
        } else {
            SLOWMO * SLOWMO_RATE + (u - SLOWMO)
        }
    }

    /// `parts_at(t)`: every part of the shatter at `t` (none before the end of the hit-stop or from the glide
    /// home on). From the reassembly the parts fly back along a little arc, spinning home.
    pub fn parts_at(&self, t: f64) -> Vec<PartAt<'_>> {
        let g = &self.show.grid;
        if t < self.t_hs || t >= g.t_glide {
            return Vec::new();
        }
        let pt = self.part_time(t);
        let back = ease((t - g.t_reassemble) / (g.t_glide - 0.1 - g.t_reassemble));
        let tgt = [self.p_ko[self.lose][0], R, self.p_ko[self.lose][1]];
        self.parts
            .iter()
            .zip(&self.traj)
            .filter(|(_, traj)| !traj.is_empty())
            .map(|(q, traj)| {
                // i = min(TRAJ.shape[1] - 1, int(pt * 120))
                let i = (np::f64_to_i64(pt * 120.0).max(0) as usize).min(traj.len() - 1);
                let [x, y, z, mut rot] = traj[i];
                let mut p = [x, y, z];
                if back > 0.0 {
                    let arc = [0.0, 0.9 * dmath::sin(dmath::PI * back), 0.0];
                    p = std::array::from_fn(|c| p[c] + (tgt[c] - p[c]) * back + arc[c]);
                    rot += 6.0 * back;
                }
                PartAt {
                    kind: &q.kind,
                    p,
                    rot,
                    size: q.size * (1.0 - 0.5 * back),
                    curl: q.curl,
                    alpha: 1.0,
                }
            })
            .collect()
    }

    /// The shards of a frame, drawn into the `float32` picture: a glass shard is a triangle in the loser's
    /// lightened colour with bright corners; every other kind is the loser's look's (`draw_shard`).
    pub(super) fn draw_shards(&self, arr: &mut Img<f32>, cam: &Cam, t: f64) {
        let col = self.col[self.lose];
        for part in self.parts_at(t) {
            let (sx, sz) = (part.p[0], part.p[2]);
            if part.kind != "glass" {
                self.looks[self.lose].draw_shard(self, arr, cam, part.kind, part.p, part.rot, part.size, part.curl);
                continue;
            }
            let d = [dmath::cos(part.rot), dmath::sin(part.rot)];
            let nn = [-d[1], d[0]];
            let size = part.size;
            // tri = [c + d * size, c - d * size * 0.6 + nn * size * 0.7, c - d * size * 0.5 - nn * size * 0.6]
            let tri = [
                [sx + d[0] * size, sz + d[1] * size],
                [
                    sx - d[0] * size * 0.6 + nn[0] * size * 0.7,
                    sz - d[1] * size * 0.6 + nn[1] * size * 0.7,
                ],
                [
                    sx - d[0] * size * 0.5 - nn[0] * size * 0.6,
                    sz - d[1] * size * 0.5 - nn[1] * size * 0.6,
                ],
            ];
            let world: Vec<V3> = tri.iter().map(|q| scene::to_world(q[0], q[1], part.p[1])).collect();
            let sp = screen(cam, &world);
            layer_blend(
                arr,
                std::slice::from_ref(&sp),
                col.map(|c| c * 0.35 + 170.0),
                0.85,
                None,
            );
            // for (xq, yq) in sp.astype(int): arr[yq, xq] = COL[LOSE] * 0.6 + 100
            for &(x, y) in &sp {
                let (xq, yq) = (np::f64_to_i64(x), np::f64_to_i64(y));
                if 0 <= xq && xq < W as i64 && 0 <= yq && yq < H as i64 {
                    let px = arr.px_mut(xq as usize, yq as usize);
                    for c in 0..3 {
                        px[c] = (col[c] * 0.6 + 100.0) as f32;
                    }
                }
            }
        }
    }
}
