//! SPIKE: the TRUMPET's Engine 2 look, drawn the way the current engine draws it (Aedan, 2026-11-17: everything
//! faithful to the original style, no 3D model).
//!
//! - The trumpet in the ball is the current engine's own sprite (`render_sprite` of `data/models/TRUMPET.json` at
//!   its view's yaw, rolled in 10 degree steps so the valves face the aim), laid in the glass as a flat
//!   camera-facing card of one quad per art pixel, unlit, at the current engine's `lit = sprite * 1.06`.
//! - Every weapon is the current engine's flat drawing: polygons in the floor plane at the ball's centre height, in
//!   its colours. The pistons are `draw_piston`'s (a brass collar, the silver rod with its glint line, the pearl
//!   button in a rim of the fighter's colour, a pale streak while they shoot out). The lance and the valve caps are
//!   new, in the same vocabulary: flat brass strips with a dark edge and a glint, collar rings, flat discs.
//! - Effects: the shared hit presets turned a notch below the dummy's, with no scorch marks.

use std::sync::Arc;

use bf_core::dmath::{atan2, cosf, sin, sinf};

fn cube(x: f64) -> f64 {
    x * x * x
}

use bf2_fx::tone::Restraint;
use bf2_fx::{FxError, FxTimeline, frame::LookFx};
use bf2_render::maths::{self, M4, V3};
use bf2_render::mesh::FLAG_TWO_SIDED;
use bf2_render::renderer::{ART_H, ART_W};
use bf2_render::stage::{BALL_R, ball_centre, linear};
use bf2_render::{
    Camera, FightFrame, Gpu, Instance, Look2, Material, MeshData, MeshId, Renderer, Scene, Vertex, to_render,
};
use bf2_sim::{EvKind, FighterDef, Joint, Pose, Recording2, Segment, solve};

use super::{
    CAP1, CAP2, CAP3, CAVALRY_CHARGE, H1, H2, H3, HERALD_LINE, HORN, LANCE, P1, P2, P3, POLE_LIFE, ROYAL_HERALDS,
    TICKER_TAPE,
};

/// The current engine's TRUMPET colours (`fighters/trumpet/look.rs`).
const BRASS: [f64; 3] = [232.0, 184.0, 72.0];
const BRASS_HI: [f64; 3] = [255.0, 236.0, 160.0];
const BRASS_DK: [f64; 3] = [150.0, 104.0, 34.0];
const PEARL: [f64; 3] = [238.0, 236.0, 228.0];
const PISTON: [f64; 3] = [212.0, 218.0, 230.0];
const COLLAR: [f64; 3] = [214.0, 168.0, 62.0];

/// The roster's `view` yaw (20) on the current engine's 6 degree grid.
const YAW: f64 = 18.0;
/// The current engine's roll step.
const ROLL_STEP: i32 = 10;
/// The sprite's share of the ball's radius on the picture (`sprite_k`).
const SPRITE_K: f32 = 0.8;
/// How far behind the ball's centre the card sits, so the flat weapons at centre height always draw over it.
const CARD_BACK: f32 = 0.22;

fn col(c: [f64; 3]) -> [f32; 3] {
    linear(c.map(|v| v.clamp(0.0, 255.0)))
}

fn mix(c: [f64; 3], k: f64, add: f64) -> [f64; 3] {
    c.map(|v| (v * k + add).clamp(0.0, 255.0))
}

struct Meshes {
    /// A unit strip: x 0..1, z -0.5..0.5, in the floor plane, both sides drawn.
    strip: MeshId,
    /// A unit 12-gon (radius 1) in the floor plane, both sides drawn.
    disc: MeshId,
    /// A unit triangle in the floor plane: its base across z at x 0, its point at x 1.
    tri: MeshId,
    /// The sprite cards: `radii` x 36 rolls.
    cards: Vec<MeshId>,
    radii: Vec<u32>,
}

pub struct TrumpetLook {
    def: Arc<FighterDef>,
    meshes: Option<Meshes>,
}

impl TrumpetLook {
    pub fn new(def: Arc<FighterDef>) -> TrumpetLook {
        TrumpetLook { def, meshes: None }
    }
}

fn basis(x: V3, y: V3, z: V3, o: V3) -> M4 {
    [
        [x[0], x[1], x[2], 0.0],
        [y[0], y[1], y[2], 0.0],
        [z[0], z[1], z[2], 0.0],
        [o[0], o[1], o[2], 1.0],
    ]
}

const UP: V3 = [0.0, 1.0, 0.0];

fn vert(p: V3, n: V3, rgb: [f32; 3]) -> Vertex {
    let mut v = Vertex::new(p, n);
    v.rgb = rgb;
    v.flags = FLAG_TWO_SIDED;
    v
}

fn unit_strip() -> MeshData {
    let q = [[0.0, 0.0, -0.5], [1.0, 0.0, -0.5], [1.0, 0.0, 0.5], [0.0, 0.0, 0.5]];
    MeshData {
        verts: q.iter().map(|&p| vert(p, UP, [1.0; 3])).collect(),
        idx: vec![0, 2, 1, 0, 3, 2],
    }
}

fn unit_tri() -> MeshData {
    let q = [[0.0, 0.0, -0.5], [1.0, 0.0, 0.0], [0.0, 0.0, 0.5]];
    MeshData {
        verts: q.iter().map(|&p| vert(p, UP, [1.0; 3])).collect(),
        idx: vec![0, 2, 1],
    }
}

/// A flat shape's model: `x` and `z` span it, `o` places it. Its face turns toward the camera (z is flipped when
/// it would face away), and the basis stays right-handed, so its triangles are never culled.
fn facing_cam(x: V3, z: V3, o: V3) -> M4 {
    let eye = Camera::default_ring().eye;
    let mut z = z;
    let mut y = maths::cross(z, x);
    if maths::dot(y, maths::sub(eye, o)) < 0.0 {
        z = maths::scale(z, -1.0);
        y = maths::scale(y, -1.0);
    }
    let ly = maths::length(y).max(1e-6);
    basis(x, maths::scale(y, 1.0 / ly), z, o)
}

fn unit_disc() -> MeshData {
    let mut verts = vec![vert([0.0; 3], UP, [1.0; 3])];
    let mut idx = Vec::new();
    for i in 0..12u32 {
        let a = std::f32::consts::TAU * i as f32 / 12.0;
        verts.push(vert([cosf(a), 0.0, sinf(a)], UP, [1.0; 3]));
        idx.extend([0, 1 + (i + 1) % 12, 1 + i]);
    }
    MeshData { verts, idx }
}

/// The sprite as a flat card: one quad per run of same-coloured pixels on a row, in pixel units (x right, y up,
/// the sprite's centre at the origin), facing +z.
fn card(img: &bf_picture::Img<u8>) -> MeshData {
    let c = img.w as f32 / 2.0;
    let mut m = MeshData {
        verts: Vec::new(),
        idx: Vec::new(),
    };
    for y in 0..img.h {
        let mut x = 0;
        while x < img.w {
            let p = img.px(x, y);
            if p[3] == 0 {
                x += 1;
                continue;
            }
            let rgb = [p[0], p[1], p[2]];
            let mut e = x + 1;
            while e < img.w && img.px(e, y)[3] != 0 && img.px(e, y)[..3] == rgb[..] {
                e += 1;
            }
            let lit = col(rgb.map(|v| f64::from(v) * 1.06));
            let (x0, x1) = (x as f32 - c, e as f32 - c);
            let (y0, y1) = (c - (y + 1) as f32, c - y as f32);
            let base = m.verts.len() as u32;
            for q in [[x0, y0, 0.0], [x1, y0, 0.0], [x1, y1, 0.0], [x0, y1, 0.0]] {
                m.verts.push(vert(q, [0.0, 0.0, 1.0], lit));
            }
            m.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            x = e;
        }
    }
    m
}

/// The ball's radius on the art picture at `c`, through today's ring framing.
fn ball_px(cam: &Camera, c: V3) -> f32 {
    let (Some(a), Some(b)) = (
        cam.project(c, ART_W, ART_H),
        cam.project(maths::add(c, [BALL_R, 0.0, 0.0]), ART_W, ART_H),
    ) else {
        return 30.0;
    };
    (b[0] - a[0]).abs()
}

/// The floor direction at sim angle `a`, in render space.
fn floor_dir(a: f64) -> V3 {
    [bf_core::dmath::cos(a) as f32, 0.0, -bf_core::dmath::sin(a) as f32]
}

/// The screen angle (degrees, y down) of render-space direction `d` at `c`.
fn screen_angle(cam: &Camera, c: V3, d: V3) -> f32 {
    match (
        cam.project(c, ART_W, ART_H),
        cam.project(maths::add(c, d), ART_W, ART_H),
    ) {
        (Some(a), Some(b)) => (atan2(f64::from(b[1] - a[1]), f64::from(b[0] - a[0])) as f32).to_degrees(),
        _ => 0.0,
    }
}

impl Look2 for TrumpetLook {
    fn setup(&mut self, r: &mut Renderer, gpu: &Gpu) {
        let mesh = crate::fighters::trumpet::MESH
            .and_then(|t| bf_picture::sprite::Mesh::from_json(t).ok())
            .expect("the trumpet's model is compiled in");
        // the sprite radii the ring can need: the ball from the near ropes to the far ones
        let cam = Camera::default_ring();
        let rs: Vec<u32> = [-4.0f32, 4.0]
            .iter()
            .map(|&z| (ball_px(&cam, [0.0, BALL_R, z]) * SPRITE_K).round() as u32)
            .collect();
        let (lo, hi) = (rs[0].min(rs[1]).saturating_sub(1), rs[0].max(rs[1]) + 1);
        let radii: Vec<u32> = (lo..=hi.min(bf_picture::sprite::MAX_RADIUS)).collect();
        let mut cards = Vec::new();
        for &rad in &radii {
            for i in 0..36 {
                let roll = f64::from(i * ROLL_STEP - 180);
                let img = bf_picture::sprite::render_sprite(&mesh, rad, YAW, roll);
                cards.push(r.meshes.add(gpu, card(&img)));
            }
        }
        self.meshes = Some(Meshes {
            strip: r.meshes.add(gpu, unit_strip()),
            tri: r.meshes.add(gpu, unit_tri()),
            disc: r.meshes.add(gpu, unit_disc()),
            cards,
            radii,
        });
    }

    fn draw(&self, f: &FightFrame<'_>, k: usize, out: &mut Scene) -> Vec<u32> {
        let Some(m) = &self.meshes else {
            return Vec::new();
        };
        let pos = f.snap.pos[k];
        let facing = f.snap.facing[k];
        let joints = &f.snap.joints[k];
        if joints.len() != self.def.rig.bones.len() {
            return Vec::new();
        }
        let horn = joints.get(HORN).copied().unwrap_or(Joint::REST);
        let cam = Camera::default_ring();
        let c = ball_centre(pos);

        // The sprite card. Valves toward the aim (the current engine's `trumpet_alpha`), turned a quarter by the
        // lance (mouthpiece first), rocked by the combo.
        let alpha = screen_angle(&cam, c, floor_dir(facing)) + 90.0;
        // (a positive roll turns the sprite anticlockwise: a quarter turn back brings the mouthpiece to the aim)
        let extra = -(horn.yaw.to_degrees() as f32) + horn.pitch.to_degrees() as f32;
        let rl = ((-alpha + extra) / ROLL_STEP as f32).round() as i32 * ROLL_STEP;
        let rl = (rl + 180).rem_euclid(360);
        let rp = ball_px(&cam, c) * SPRITE_K;
        let ri = m
            .radii
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| ((**r as f32 - rp).abs() * 100.0) as i32)
            .map_or(0, |(i, _)| i);
        let rad = m.radii[ri] as f32;
        let toward = maths::normalize(maths::sub(cam.eye, c));
        let right = maths::normalize(maths::cross(UP, toward));
        let up = maths::cross(toward, right);
        let at = maths::sub(c, maths::scale(toward, CARD_BACK));
        // one sprite pixel: the sprite's radius covers SPRITE_K of the ball's (at the card's depth, a hair smaller)
        let px = BALL_R * SPRITE_K / rad * (ball_px(&cam, at) / ball_px(&cam, c)).min(1.0);
        let idx = ri * 36 + (rl / ROLL_STEP) as usize;
        out.instances.push(
            Instance::new(
                m.cards[idx],
                Material::Unlit,
                basis(maths::scale(right, px), maths::scale(up, px), toward, at),
            )
            .coloured([1.0; 3]),
        );

        // The rig's segments, now and a few steps back (the caps' motion ghosts, the pistons' streaks).
        let solve_at = |step: usize| -> Option<Vec<Segment>> {
            let st = f.rec.steps.get(step)?;
            let p = st.pos[k];
            let pose = Pose {
                joints: st.joints[k].clone(),
            };
            let mut segs = Vec::new();
            solve(
                &self.def.rig,
                &pose,
                [p[0], p[1], p[2] + self.def.r],
                self.def.r,
                st.facing[k],
                &mut segs,
            );
            Some(segs)
        };
        let mut segs: Vec<Segment> = Vec::new();
        let pose = Pose { joints: joints.clone() };
        solve(
            &self.def.rig,
            &pose,
            [pos[0], pos[1], pos[2] + self.def.r],
            self.def.r,
            facing,
            &mut segs,
        );
        let flat = |v: V3| [v[0], c[1], v[2]];
        let seg = |s: &[Segment], b: usize| (to_render(s[b].a), to_render(s[b].b));
        let mut layer = 0.0f32;
        let mut lift = |p: V3| {
            layer += 0.003;
            [p[0], p[1] + layer, p[2]]
        };
        let strip = |out: &mut Scene, a: V3, b: V3, w: f32, colour: [f32; 3], mat: Material| {
            // (x along the strip, y up, z = x cross y: right-handed, so its triangles stay front-facing)
            let d = maths::sub(b, a);
            let len = maths::length(d).max(1e-5);
            let n = maths::normalize(maths::cross(maths::scale(d, 1.0 / len), UP));
            out.instances
                .push(Instance::new(m.strip, mat, basis(d, UP, maths::scale(n, w), a)).coloured(colour));
        };
        let disc = |out: &mut Scene, p: V3, r: f32, colour: [f32; 3], mat: Material| {
            out.instances
                .push(Instance::new(m.disc, mat, basis([r, 0.0, 0.0], UP, [0.0, 0.0, r], p)).coloured(colour));
        };
        let fighter = self.def.colour.map(f64::from);
        let prev = f.step.checked_sub(2).and_then(solve_at);

        // The pistons: `draw_piston`, at the valve's lateral offset, the rod's base 0.8 of the radius out.
        for (i, b) in [P1, P2, P3].into_iter().enumerate() {
            let (a, tip) = seg(&segs, b);
            let (a, tip) = (flat(a), flat(tip));
            let len = maths::length(maths::sub(tip, a));
            if len < 0.04 {
                continue;
            }
            let d = maths::scale(maths::sub(tip, a), 1.0 / len);
            let n = maths::normalize(maths::cross(UP, d));
            let off = maths::scale(n, (i as f32 - 1.0) * 0.1);
            let a0 = maths::add(maths::sub(a, maths::scale(d, BALL_R * 0.2)), off);
            let tip = maths::add(tip, off);
            let growing = prev.as_ref().is_some_and(|p| {
                let (pa, pt) = seg(p, b);
                maths::length(maths::sub(pt, pa)) + 0.02 < len
            });
            if growing {
                strip(
                    out,
                    lift(a0),
                    lift(tip),
                    0.3,
                    col(mix(fighter, 0.5, 120.0)),
                    Material::Additive { strength: 0.3 },
                );
            }
            strip(
                out,
                lift(a0),
                lift(tip),
                0.1,
                col(PISTON.map(|v| v * 0.82)),
                Material::Unlit,
            );
            strip(
                out,
                lift(maths::add(a0, maths::scale(n, 0.018))),
                lift(maths::add(tip, maths::scale(n, 0.018))),
                0.022,
                col([250.0, 252.0, 255.0]),
                Material::Unlit,
            );
            disc(out, lift(a0), 0.095, col(COLLAR), Material::Unlit);
            disc(out, lift(tip), 0.16, col(mix(fighter, 0.8, 30.0)), Material::Unlit);
            disc(
                out,
                lift(maths::add(tip, maths::scale(d, 0.015))),
                0.125,
                col(PEARL),
                Material::Unlit,
            );
        }

        // The lance: the lead pipe in three telescoping flat brass sections, the mouthpiece on the end.
        {
            let (a, tip) = seg(&segs, LANCE);
            let (a, tip) = (flat(a), flat(tip));
            let len = maths::length(maths::sub(tip, a));
            if len > 0.06 {
                let d = maths::scale(maths::sub(tip, a), 1.0 / len);
                let n = maths::normalize(maths::cross(UP, d));
                let a0 = maths::sub(a, maths::scale(d, 0.18));
                let total = len + 0.18;
                let growing = prev.as_ref().is_some_and(|p| {
                    let (pa, pt) = seg(p, LANCE);
                    maths::length(maths::sub(pt, pa)) + 0.03 < len
                });
                if growing {
                    strip(
                        out,
                        lift(a0),
                        lift(tip),
                        0.4,
                        col(mix(fighter, 0.5, 120.0)),
                        Material::Additive { strength: 0.3 },
                    );
                }
                let halves = [0.11f32, 0.09, 0.072];
                for (i, w) in halves.into_iter().enumerate() {
                    let from = (total * i as f32 / 3.0 - 0.04).max(0.0);
                    let to = total * (i as f32 + 1.0) / 3.0;
                    let p0 = maths::add(a0, maths::scale(d, from));
                    let p1 = maths::add(a0, maths::scale(d, to));
                    strip(out, lift(p0), lift(p1), 2.0 * w + 0.03, col(BRASS_DK), Material::Unlit);
                    strip(out, lift(p0), lift(p1), 2.0 * w, col(BRASS), Material::Unlit);
                    let g = maths::scale(n, w * 0.45);
                    strip(
                        out,
                        lift(maths::add(p0, g)),
                        lift(maths::add(p1, g)),
                        0.022,
                        col(BRASS_HI),
                        Material::Unlit,
                    );
                    // a collar ring at the section's mouth
                    strip(
                        out,
                        lift(maths::sub(p1, maths::scale(d, 0.05))),
                        lift(p1),
                        2.0 * w + 0.05,
                        col(COLLAR),
                        Material::Unlit,
                    );
                }
                // the mouthpiece: a silver shank and cup
                let cup = maths::add(tip, maths::scale(d, 0.13));
                strip(
                    out,
                    lift(tip),
                    lift(cup),
                    0.08,
                    col(PISTON.map(|v| v * 0.82)),
                    Material::Unlit,
                );
                disc(out, lift(cup), 0.12, col(PISTON.map(|v| v * 0.82)), Material::Unlit);
                disc(out, lift(cup), 0.09, col(PISTON), Material::Unlit);
                disc(out, lift(cup), 0.045, col(PISTON.map(|v| v * 0.55)), Material::Unlit);
            }
        }

        // The valve caps on their cords, with pale ghosts of where they were.
        for b in [CAP1, CAP2, CAP3] {
            let (a, tip) = seg(&segs, b);
            let v = maths::sub(tip, a);
            let len = maths::length(v);
            if len < 0.64 {
                continue;
            }
            let dir = maths::scale(v, 1.0 / len);
            for (j, back) in [6usize, 4, 2].into_iter().enumerate() {
                if let Some(ps) = f.step.checked_sub(back).and_then(solve_at) {
                    let (_, gt) = seg(&ps, b);
                    if maths::length(maths::sub(gt, tip)) > 0.08 {
                        disc(
                            out,
                            lift(gt),
                            0.15,
                            col(mix(fighter, 0.5, 120.0)),
                            Material::Additive {
                                strength: 0.12 + 0.08 * j as f32,
                            },
                        );
                    }
                }
            }
            let from = maths::add(a, maths::scale(dir, BALL_R * 0.85));
            strip(
                out,
                lift(from),
                lift(tip),
                0.025,
                col(PISTON.map(|v| v * 0.82)),
                Material::Unlit,
            );
            disc(out, lift(tip), 0.16, col(BRASS_DK), Material::Unlit);
            disc(out, lift(tip), 0.135, col(COLLAR), Material::Unlit);
            disc(out, lift(tip), 0.095, col(BRASS_HI), Material::Unlit);
            disc(out, lift(tip), 0.05, col(PEARL), Material::Unlit);
        }
        // ---------------------------------------------------------------- round 2: the specials
        let fstep = f.step.min(f.rec.steps.len().saturating_sub(1));
        let now_mv = f.rec.steps.get(fstep).and_then(|st| st.mv[k]);
        let flat_put = |out: &mut Scene, mesh: MeshId, x: V3, z: V3, o: V3, colour: [f32; 3], mat: Material| {
            out.instances
                .push(Instance::new(mesh, mat, facing_cam(x, z, o)).coloured(colour));
        };
        let cam_right = right;
        let mut drawn = Vec::new();

        // My projectiles: which move threw each (HERALD LINE's poles, TICKER TAPE's flakes), and when.
        let mut mine: Vec<(u32, usize, u32, usize)> = Vec::new(); // (id, move, spawn step, index in its cast)
        for e in &f.rec.events {
            if e.step as usize > fstep {
                break;
            }
            if e.who != k {
                continue;
            }
            if let EvKind::Spawn { proj } = e.kind {
                let mv = f
                    .rec
                    .steps
                    .get(e.step as usize)
                    .and_then(|st| st.mv[k])
                    .map_or(usize::MAX, |m| m.0 as usize);
                let idx = mine
                    .iter()
                    .filter(|m| m.1 == mv && e.step.saturating_sub(m.2) < 40)
                    .count();
                mine.push((proj as u32, mv, e.step, idx));
            }
        }
        for &(id, pos3) in &f.snap.projs {
            let Some(&(_, mv, at, idx)) = mine.iter().find(|m| m.0 == id) else {
                continue;
            };
            let age = (f.step as f64 + f.alpha) - f64::from(at);
            let p = to_render(pos3);
            if mv == HERALD_LINE {
                drawn.push(id);
                let life = f64::from(POLE_LIFE);
                let up = (1.0 - cube(1.0 - (age / 10.0).clamp(0.0, 1.0)))
                    * (1.0 - ((age - (life - 24.0)) / 20.0).clamp(0.0, 1.0));
                let h = 0.95 * up as f32;
                if h < 0.02 {
                    continue;
                }
                let base = [p[0], 0.0, p[2]];
                let top = maths::add(base, [0.0, h, 0.0]);
                disc(out, [base[0], 0.012, base[2]], 0.11, col(COLLAR), Material::Unlit);
                flat_put(
                    out,
                    m.strip,
                    [0.0, h, 0.0],
                    maths::scale(cam_right, 0.06),
                    base,
                    col(PISTON.map(|v| v * 0.82)),
                    Material::Unlit,
                );
                flat_put(
                    out,
                    m.strip,
                    [0.0, h, 0.0],
                    maths::scale(cam_right, 0.018),
                    maths::add(base, maths::scale(cam_right, 0.012)),
                    col([250.0, 252.0, 255.0]),
                    Material::Unlit,
                );
                // the pennant streams away from the foe it fences, fluttering
                let tc = ball_centre(f.snap.pos[1 - k]);
                let out_dir = maths::normalize([p[0] - tc[0], 0.0, p[2] - tc[2]]);
                let flut = sin(age * 0.5 + idx as f64) as f32 * 0.05;
                let tip = maths::add(maths::scale(out_dir, 0.36), [0.0, flut, 0.0]);
                let o = maths::sub(top, [0.0, 0.11, 0.0]);
                flat_put(
                    out,
                    m.tri,
                    maths::scale(tip, 1.12),
                    [0.0, 0.26, 0.0],
                    o,
                    col(BRASS_DK),
                    Material::Unlit,
                );
                flat_put(
                    out,
                    m.tri,
                    tip,
                    [0.0, 0.2, 0.0],
                    maths::add(o, [0.0, 0.004, 0.0]),
                    col(fighter),
                    Material::Unlit,
                );
                disc(
                    out,
                    maths::add(top, [0.0, 0.01, 0.0]),
                    0.04,
                    col(BRASS_HI),
                    Material::Unlit,
                );
            } else if mv == TICKER_TAPE {
                drawn.push(id);
                let colours = [fighter, BRASS_HI, BRASS];
                for j in 0..3usize {
                    let a = (age * 0.31 + (idx * 3 + j) as f64 * 1.7) as f32;
                    let off = [
                        sinf((idx * 5 + j * 3) as f32) * 0.16,
                        cosf((j * 7) as f32) * 0.08,
                        cosf((idx * 3 + j * 5) as f32) * 0.16,
                    ];
                    let at = maths::add(p, if j == 0 { [0.0; 3] } else { off });
                    let landed = at[1] < 0.16;
                    let x = if landed {
                        [cosf(a) * 0.14, 0.0, sinf(a) * 0.14]
                    } else {
                        maths::scale(maths::normalize([cosf(a), 0.7 * sinf(1.3 * a), sinf(a)]), 0.15)
                    };
                    let z = maths::scale(
                        maths::normalize(maths::cross(x, if landed { [0.0, -1.0, 0.0] } else { [0.3, 1.0, 0.2] })),
                        0.08,
                    );
                    let at = if landed { [at[0], 0.015, at[2]] } else { at };
                    flat_put(
                        out,
                        m.strip,
                        x,
                        z,
                        maths::sub(at, maths::scale(x, 0.5)),
                        col(colours[(idx + j) % 3]),
                        Material::Unlit,
                    );
                }
            }
        }

        // CAVALRY CHARGE: a brass wake and a row of pennants left standing along the dash.
        let last_active = f
            .rec
            .events
            .iter()
            .rev()
            .filter(|e| e.who == k && e.step as usize <= fstep)
            .find_map(|e| match e.kind {
                EvKind::Active { mv } if mv == CAVALRY_CHARGE => Some(e.step),
                _ => None,
            });
        if let Some(a0) = last_active {
            let since = (f.step as f64 + f.alpha) - f64::from(a0);
            if (0.0..70.0).contains(&since) {
                let fade = (1.0 - ((since - 20.0) / 50.0).clamp(0.0, 1.0)) as f32;
                let start = f.rec.steps.get(a0 as usize).map(|st| ball_centre(st.pos[k]));
                if let Some(s0) = start {
                    let s0 = [s0[0], c[1], s0[2]];
                    let cc = [c[0], c[1], c[2]];
                    if maths::length(maths::sub(cc, s0)) > 0.3 {
                        strip(
                            out,
                            lift(s0),
                            lift(cc),
                            0.9,
                            col(mix(fighter, 0.5, 120.0)),
                            Material::Additive { strength: 0.28 * fade },
                        );
                    }
                    for i in 0..3u32 {
                        let st = f
                            .rec
                            .steps
                            .get((a0 + 3 + i * 4) as usize)
                            .map(|st| ball_centre(st.pos[k]));
                        let Some(q) = st else { continue };
                        if since < f64::from(3 + i * 4) {
                            continue;
                        }
                        let d = floor_dir(f.rec.steps[(a0 + 3 + i * 4) as usize].facing[k]);
                        let back = maths::scale(d, -1.0);
                        let base = maths::add([q[0], 0.0, q[2]], maths::scale(back, 0.7));
                        let h = 0.6 * fade;
                        if h < 0.03 {
                            continue;
                        }
                        flat_put(
                            out,
                            m.strip,
                            [0.0, h, 0.0],
                            maths::scale(cam_right, 0.05),
                            base,
                            col(PISTON.map(|v| v * 0.82)),
                            Material::Unlit,
                        );
                        let flut = sinf((since * 0.6) as f32 + i as f32) * 0.05;
                        let o = maths::add(base, [0.0, h - 0.09, 0.0]);
                        flat_put(
                            out,
                            m.tri,
                            maths::add(maths::scale(back, 0.32), [0.0, flut, 0.0]),
                            [0.0, 0.17, 0.0],
                            o,
                            col(fighter),
                            Material::Unlit,
                        );
                    }
                }
            }
        }

        // ROYAL HERALDS: three long herald trumpets with hanging banners, cones of air on the blast.
        let blast = match now_mv {
            Some((mv, 1, s)) if mv as usize == ROYAL_HERALDS => Some(f64::from(s) + f.alpha),
            Some((mv, 2, s)) if mv as usize == ROYAL_HERALDS && s < 8 => Some(8.0 + f64::from(s) + f.alpha),
            _ => None,
        };
        for b in [H1, H2, H3] {
            let (a, tip) = seg(&segs, b);
            let (a, tip) = (flat(a), flat(tip));
            let len = maths::length(maths::sub(tip, a));
            if len < 0.1 {
                continue;
            }
            let d = maths::scale(maths::sub(tip, a), 1.0 / len);
            let n = maths::normalize(maths::cross(d, UP));
            let a0 = maths::sub(a, maths::scale(d, 0.12));
            strip(out, lift(a0), lift(tip), 0.11, col(BRASS_DK), Material::Unlit);
            strip(out, lift(a0), lift(tip), 0.075, col(BRASS), Material::Unlit);
            strip(
                out,
                lift(maths::add(a0, maths::scale(n, 0.018))),
                lift(maths::add(tip, maths::scale(n, 0.018))),
                0.02,
                col(BRASS_HI),
                Material::Unlit,
            );
            // the bell, flaring out
            let bell = lift(maths::add(tip, maths::scale(d, 0.28)));
            flat_put(
                out,
                m.tri,
                maths::scale(d, -0.3),
                maths::scale(n, 0.4),
                bell,
                col(BRASS_DK),
                Material::Unlit,
            );
            flat_put(
                out,
                m.tri,
                maths::scale(d, -0.27),
                maths::scale(n, 0.33),
                lift(bell),
                col(BRASS),
                Material::Unlit,
            );
            // the banner, hanging from the middle of the tube
            let mid = maths::add(a0, maths::scale(maths::sub(tip, a0), 0.55));
            let bh = (0.32 * (len / 1.5)).min(0.32);
            flat_put(
                out,
                m.strip,
                [0.0, -bh, 0.0],
                maths::scale(d, 0.26),
                mid,
                col(BRASS_DK),
                Material::Unlit,
            );
            flat_put(
                out,
                m.strip,
                [0.0, -bh + 0.03, 0.0],
                maths::scale(d, 0.21),
                maths::add(mid, [0.0, -0.015, 0.0]),
                col(fighter),
                Material::Unlit,
            );
            if let Some(s) = blast {
                let kk = (s / 16.0).clamp(0.0, 1.0) as f32;
                let l = 0.4 + 1.1 * kk;
                flat_put(
                    out,
                    m.tri,
                    maths::scale(d, -l),
                    maths::scale(n, 0.25 + 0.9 * kk),
                    lift(maths::add(bell, maths::scale(d, l))),
                    col(mix(fighter, 0.4, 150.0)),
                    Material::Additive {
                        strength: 0.4 * (1.0 - kk),
                    },
                );
            }
        }
        drawn
    }
}

impl LookFx for TrumpetLook {
    fn fx(
        &self,
        _k: usize,
        _rec: &Recording2,
        _step_to_t: &dyn Fn(u32) -> f64,
        _out: &mut FxTimeline,
    ) -> Result<(), FxError> {
        Ok(())
    }

    /// A notch lighter than the dummy's (Aedan: fighters' hits land lighter than the dummy's presets), and no
    /// scorch marks (they blur, which the current engine never does).
    fn tone(&self, e: &EvKind) -> Restraint {
        if matches!(e, EvKind::Ko { .. }) {
            Restraint {
                particles: 0.35,
                glow: 0.3,
                life: 0.8,
                size: 0.4,
                marks: 0.0,
                screen: 0.45,
                lens: false,
            }
        } else {
            Restraint {
                particles: 0.14,
                glow: 0.28,
                life: 0.5,
                size: 0.42,
                marks: 0.0,
                screen: 0.22,
                lens: false,
            }
        }
    }
}
