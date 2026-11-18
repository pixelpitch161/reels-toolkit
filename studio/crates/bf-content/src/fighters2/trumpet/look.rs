//! The TRUMPET's Engine 2 look, drawn the way the current engine draws it (Aedan, 2026-11-17: everything faithful
//! to the original style, no 3D model).
//!
//! - The trumpet in the ball is the current engine's own sprite (`render_sprite` of `data/models/TRUMPET.json` at
//!   its view's yaw, rolled in 10 degree steps so the valves face the aim), laid in the glass as a flat
//!   camera-facing card of one quad per art pixel, unlit, at the current engine's `lit = sprite * 1.06`.
//! - Every weapon is the current engine's flat drawing: polygons in the floor plane at the ball's centre height, in
//!   its colours. The pistons are `draw_piston`'s (a brass collar, the silver rod with its glint line, the pearl
//!   button in a rim of the fighter's colour, a pale streak while they shoot out). A piston stops on the foe's glass
//!   instead of passing through it (Aedan, 2026-11-18): its button squashes against the glass with a small ring, and
//!   the trumpet recoils a touch in its ball.
//! - The specials are new, in the same vocabulary: HERALD LINE's silver poles with gold pennants, DOUBLE TONGUE+'s
//!   gold ring and the haste's glow under the ball, PISTON ROCKETS' flying pistons with puffs of steam behind them.
//!   Nothing pops: every part rises, folds, fades or shrinks out.
//! - Effects: the shared hit presets turned a notch below the dummy's, with no scorch marks.

use std::sync::Arc;

use bf_core::dmath::{atan2, cosf, sin, sinf};
use bf2_sim::HASTE_STEPS;

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
    DOUBLE_TONGUE, HERALD_LINE, HORN, P1, P2, P3, PISTON_ROCKETS, POLE_FOLD, POLE_LIFE, POLE_RISE, ROCKET_LIFE,
    VALVE_COMBO_TK,
};

/// The current engine's TRUMPET colours (`fighters/trumpet/look.rs`).
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
        // (pushed once the pistons are known: it recoils when they meet the foe)
        let card_at = |off: V3| {
            Instance::new(
                m.cards[idx],
                Material::Unlit,
                basis(
                    maths::scale(right, px),
                    maths::scale(up, px),
                    toward,
                    maths::add(at, off),
                ),
            )
            .coloured([1.0; 3])
        };

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
        let fstep = f.step.min(f.rec.steps.len().saturating_sub(1));
        let now = f.step as f64 + f.alpha;
        let now_mv = f.rec.steps.get(fstep).and_then(|st| st.mv[k]).map(|m| m.0 as usize);
        let foe = flat(ball_centre(f.snap.pos[1 - k]));
        let cam_right = right;
        let glow = col(mix(fighter, 0.5, 120.0));
        let rod = col(PISTON.map(|v| v * 0.82));
        let shine = col([250.0, 252.0, 255.0]);
        // a flat oval in the floor plane: `along` half-width along `d`, `across` across it
        let oval = |out: &mut Scene, p: V3, d: V3, along: f32, across: f32, colour: [f32; 3]| {
            let n = maths::normalize(maths::cross(d, UP));
            out.instances.push(
                Instance::new(
                    m.disc,
                    Material::Unlit,
                    basis(maths::scale(d, along), UP, maths::scale(n, across), p),
                )
                .coloured(colour),
            );
        };

        // The pistons: `draw_piston`, at the valve's lateral offset, the rod's base 0.2 of the radius inside the
        // glass. A piston that would pass into the foe stops on its glass: the tip is cut back to the glass, the
        // button squashes flat against it, and a small ring marks the contact.
        let mut recoil = 0.0f32;
        let mut recoil_dir = [0.0f32; 3];
        let taka = now_mv == Some(VALVE_COMBO_TK);
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
            let full = maths::add(tip, off);
            // where the rod meets the foe's glass (the button's rim stops on it)
            let reach = maths::length(maths::sub(full, a0));
            let stop = glass_hit(a0, d, foe, BALL_R + 0.1).filter(|&t| t < reach);
            let (tip, squash) = match stop {
                Some(t) => {
                    let over = reach - t;
                    recoil = recoil.max((over * 0.06).min(0.05));
                    recoil_dir = d;
                    (maths::add(a0, maths::scale(d, t.max(0.05))), (over / 0.25).min(1.0))
                }
                None => (full, 0.0),
            };
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
                    glow,
                    Material::Additive { strength: 0.3 },
                );
            }
            // TA-KA COMBO: the second tongue's after-image, a pale rod and button just behind the live one
            if taka && growing {
                let back = maths::sub(tip, maths::scale(d, 0.22));
                strip(
                    out,
                    lift(a0),
                    lift(back),
                    0.1,
                    glow,
                    Material::Additive { strength: 0.25 },
                );
                disc(out, lift(back), 0.13, glow, Material::Additive { strength: 0.3 });
            }
            strip(out, lift(a0), lift(tip), 0.1, rod, Material::Unlit);
            strip(
                out,
                lift(maths::add(a0, maths::scale(n, 0.018))),
                lift(maths::add(tip, maths::scale(n, 0.018))),
                0.022,
                shine,
                Material::Unlit,
            );
            disc(out, lift(a0), 0.095, col(COLLAR), Material::Unlit);
            // the button: squashed along the stroke against the glass
            let sq = 1.0 - 0.45 * squash;
            oval(
                out,
                lift(tip),
                d,
                0.16 * sq,
                0.16 * (1.0 + 0.15 * squash),
                col(mix(fighter, 0.8, 30.0)),
            );
            oval(
                out,
                lift(maths::add(tip, maths::scale(d, 0.015))),
                d,
                0.125 * sq,
                0.125 * (1.0 + 0.15 * squash),
                col(PEARL),
            );
            if squash > 0.0 {
                // the contact ring on the glass
                let at = maths::add(tip, maths::scale(d, 0.1));
                disc(
                    out,
                    lift(at),
                    0.12 + 0.12 * squash,
                    glow,
                    Material::Additive {
                        strength: 0.35 * squash,
                    },
                );
            }
        }
        out.instances.push(card_at(maths::scale(recoil_dir, -recoil)));

        // My projectiles: which move threw each (HERALD LINE's poles, PISTON ROCKETS' rockets), when, and which one
        // of its cast.
        let mut mine: Vec<(u32, usize, u32, usize)> = Vec::new();
        let mut pole_hits: Vec<(u32, u32)> = Vec::new();
        let mut active_at: [Option<u32>; 5] = [None; 5];
        for e in &f.rec.events {
            if e.step as usize > fstep {
                break;
            }
            if e.who != k {
                continue;
            }
            match e.kind {
                EvKind::Active { mv } if mv < active_at.len() => active_at[mv] = Some(e.step),
                EvKind::Spawn { proj } => {
                    let mv = f
                        .rec
                        .steps
                        .get(e.step as usize)
                        .and_then(|st| st.mv[k])
                        .map_or(usize::MAX, |m| m.0 as usize);
                    let idx = mine
                        .iter()
                        .filter(|m| m.1 == mv && e.step.saturating_sub(m.2) < 90)
                        .count();
                    mine.push((proj as u32, mv, e.step, idx));
                }
                EvKind::ProjHit { proj, .. } => pole_hits.push((proj as u32, e.step)),
                _ => {}
            }
        }
        let mut drawn = Vec::new();
        for &(id, pos3) in &f.snap.projs {
            let Some(&(_, mv, at, idx)) = mine.iter().find(|m| m.0 == id) else {
                continue;
            };
            let age = now - f64::from(at);
            let p = to_render(pos3);
            if mv == HERALD_LINE {
                drawn.push(id);
                let hit = pole_hits.iter().rev().find(|h| h.0 == id).map(|h| now - f64::from(h.1));
                self.pole(out, m, p, foe, age, idx, hit, cam_right, fighter);
            } else if mv == PISTON_ROCKETS {
                drawn.push(id);
                // heading: from where it was a step back
                let back = f
                    .rec
                    .steps
                    .get(fstep.saturating_sub(1))
                    .and_then(|st| st.projs.iter().find(|q| q.0 == id))
                    .map(|q| flat(to_render(q.1)));
                let pf = flat(p);
                let dir = back
                    .map(|b| maths::sub(pf, b))
                    .filter(|v| maths::length(*v) > 1e-4)
                    .map_or(floor_dir(facing), maths::normalize);
                // it sets off small out of the casing and grows to full size over 6 steps
                let grow = smooth01((age / 6.0) as f32);
                self.rocket(out, m, pf, dir, grow, age, fighter);
            }
        }

        // The rockets' steam: puffs left along each rocket's path, each one swelling and fading over 18 steps, so
        // the trail lingers and dies away after the rocket has hit or gone.
        for &(id, mv, at, _) in &mine {
            if mv != PISTON_ROCKETS {
                continue;
            }
            let last = (at as usize + ROCKET_LIFE as usize).min(fstep);
            let first = (at as usize + 1).max(fstep.saturating_sub(18));
            for st in (first..=last).step_by(2) {
                let Some(q) = f.rec.steps.get(st).and_then(|s| s.projs.iter().find(|q| q.0 == id)) else {
                    continue;
                };
                let u = ((now - st as f64) / 18.0).clamp(0.0, 1.0) as f32;
                let fade = 1.0 - u;
                if fade <= 0.02 {
                    continue;
                }
                let qp = flat(to_render(q.1));
                let wob = sinf(st as f32 * 1.3 + id as f32) * 0.03;
                disc(
                    out,
                    lift(maths::add(qp, [wob, 0.0, -wob])),
                    0.07 + 0.13 * u,
                    col([236.0, 236.0, 242.0]),
                    Material::Additive { strength: 0.32 * fade },
                );
            }
        }

        // DOUBLE TONGUE+: on its Active start a gold ring flashes out round the ball; through the haste a soft gold
        // glow pulses under the ball on every eighth note, and fades out over its last half second.
        if let Some(a0) = active_at[DOUBLE_TONGUE] {
            let since = now - f64::from(a0);
            if (0.0..18.0).contains(&since) {
                let u = (since / 18.0) as f32;
                let r = BALL_R * (1.05 + 0.9 * smooth01(u));
                let fade = 1.0 - smooth01(u);
                for j in 0..20 {
                    let a = std::f32::consts::TAU * j as f32 / 20.0;
                    let b = std::f32::consts::TAU * (j as f32 + 0.6) / 20.0;
                    let pa = maths::add(c, [cosf(a) * r, 0.0, sinf(a) * r]);
                    let pb = maths::add(c, [cosf(b) * r, 0.0, sinf(b) * r]);
                    strip(
                        out,
                        lift(flat(pa)),
                        lift(flat(pb)),
                        0.05,
                        col(fighter),
                        Material::Additive { strength: 0.7 * fade },
                    );
                }
            }
            let haste = f64::from(HASTE_STEPS);
            if (0.0..haste).contains(&since) {
                let fade_in = smooth01((since / 10.0) as f32);
                let fade_out = 1.0 - smooth01(((since - (haste - 60.0)) / 60.0) as f32);
                let pulse = 0.5 + 0.5 * cosf((since * std::f64::consts::TAU / 30.0) as f32);
                let floor = [c[0], 0.012, c[2]];
                disc(
                    out,
                    floor,
                    BALL_R * 1.15,
                    col(fighter),
                    Material::Additive {
                        strength: (0.08 + 0.1 * pulse) * fade_in * fade_out,
                    },
                );
            }
        }
        drawn
    }
}

/// The distance along the ray from `a` in direction `d` (a unit vector in the floor plane) to where it first
/// meets a circle of radius `r` round `c`, when it does ahead of `a`.
fn glass_hit(a: V3, d: V3, c: V3, r: f32) -> Option<f32> {
    let oc = maths::sub(a, c);
    let b = maths::dot(oc, d);
    let q = maths::dot(oc, oc) - r * r;
    let disc = b * b - q;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t > 0.0).then_some(t)
}

fn smooth01(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl TrumpetLook {
    /// One HERALD LINE pole at `p` (on the canvas), `age` steps after it rose: a silver pole that shoots up with a
    /// little overshoot, a gold pennant streaming away from the foe it fences, a wobble for a few steps after the
    /// foe bounces off it (`hit` steps ago), and at the end of its life it folds down flat outward, one pole after
    /// another, then sinks into the canvas.
    #[allow(clippy::too_many_arguments)]
    fn pole(
        &self,
        out: &mut Scene,
        m: &Meshes,
        p: V3,
        foe: V3,
        age: f64,
        idx: usize,
        hit: Option<f64>,
        cam_right: V3,
        fighter: [f64; 3],
    ) {
        let life = f64::from(POLE_LIFE);
        let rise = (age / POLE_RISE).clamp(0.0, 1.0);
        let up = 1.0 + 2.6 * cube(rise - 1.0) + 1.6 * (rise - 1.0) * (rise - 1.0);
        let fold_at = life - 40.0 + 3.0 * idx as f64;
        let fold = smooth01(((age - fold_at) / (POLE_FOLD * 0.7)) as f32);
        let sink =
            1.0 - smooth01(((age - fold_at - POLE_FOLD * 0.7) / (life - fold_at - POLE_FOLD * 0.7 - 1.0)) as f32);
        let h = 1.05 * up.max(0.0) as f32 * sink;
        if h < 0.02 {
            return;
        }
        let base = [p[0], 0.0, p[2]];
        let out_dir = {
            let v = [p[0] - foe[0], 0.0, p[2] - foe[2]];
            if maths::length(v) > 1e-4 {
                maths::normalize(v)
            } else {
                [1.0, 0.0, 0.0]
            }
        };
        // the wobble after a bounce: a tilt away from the foe that rings down
        let wob = hit
            .filter(|&s| s < 30.0)
            .map_or(0.0, |s| (0.35 * (1.0 - s / 30.0) * sin(s * 0.9)) as f32);
        let tilt = fold * std::f32::consts::FRAC_PI_2 + wob;
        let axis = maths::add(maths::scale(UP, cosf(tilt)), maths::scale(out_dir, sinf(tilt)));
        let top = maths::add(base, maths::scale(axis, h));
        let put = |out: &mut Scene, mesh: MeshId, x: V3, z: V3, o: V3, colour: [f32; 3]| {
            out.instances
                .push(Instance::new(mesh, Material::Unlit, facing_cam(x, z, o)).coloured(colour));
        };
        let disc = |out: &mut Scene, at: V3, r: f32, colour: [f32; 3]| {
            out.instances.push(
                Instance::new(m.disc, Material::Unlit, basis([r, 0.0, 0.0], UP, [0.0, 0.0, r], at)).coloured(colour),
            );
        };
        disc(out, [base[0], 0.012, base[2]], 0.12 * sink.max(0.3), col(COLLAR));
        disc(out, [base[0], 0.016, base[2]], 0.07 * sink.max(0.3), col(BRASS_DK));
        put(
            out,
            m.strip,
            maths::scale(axis, h),
            maths::scale(cam_right, 0.065),
            base,
            col(PISTON.map(|v| v * 0.82)),
        );
        put(
            out,
            m.strip,
            maths::scale(axis, h),
            maths::scale(cam_right, 0.02),
            maths::add(base, maths::scale(cam_right, 0.014)),
            col([250.0, 252.0, 255.0]),
        );
        // the pennant: a long gold triangle with a dark edge, streaming away from the foe and fluttering
        let flut = sin(age * 0.45 + idx as f64 * 1.3) as f32;
        let side = maths::normalize(maths::add(out_dir, maths::scale(axis, -0.15 * fold)));
        let tail = maths::add(maths::scale(side, 0.52 * sink), maths::scale(axis, 0.05 * flut));
        let o = maths::sub(top, maths::scale(axis, 0.16));
        let width = maths::scale(axis, 0.36);
        put(
            out,
            m.tri,
            maths::scale(tail, 1.1),
            maths::scale(width, 1.15),
            o,
            col(BRASS_DK),
        );
        put(out, m.tri, tail, width, maths::add(o, [0.0, 0.004, 0.0]), col(fighter));
        // a stripe of the brass highlight down the pennant's middle
        put(
            out,
            m.tri,
            maths::scale(tail, 0.7),
            maths::scale(width, 0.3),
            maths::add(o, [0.0, 0.008, 0.0]),
            col(BRASS_HI),
        );
        disc(out, maths::add(top, [0.0, 0.01, 0.0]), 0.05, col(BRASS_HI));
    }

    /// One PISTON ROCKET at `p` heading `dir`: the piston flying button-first, its rod and collar behind, a flicker
    /// of the fighter's colour from the casing end, all `grow` of full size.
    #[allow(clippy::too_many_arguments)]
    fn rocket(&self, out: &mut Scene, m: &Meshes, p: V3, dir: V3, grow: f32, age: f64, fighter: [f64; 3]) {
        let n = maths::normalize(maths::cross(UP, dir));
        let s = 0.5 + 0.5 * grow;
        let nose = maths::add(p, maths::scale(dir, 0.12 * s));
        let tail = maths::sub(p, maths::scale(dir, 0.34 * s));
        let strip = |out: &mut Scene, a: V3, b: V3, w: f32, colour: [f32; 3], mat: Material, y: f32| {
            let d = maths::sub(b, a);
            let len = maths::length(d).max(1e-5);
            let nn = maths::normalize(maths::cross(maths::scale(d, 1.0 / len), UP));
            let a = [a[0], a[1] + y, a[2]];
            out.instances
                .push(Instance::new(m.strip, mat, basis(d, UP, maths::scale(nn, w), a)).coloured(colour));
        };
        let disc = |out: &mut Scene, at: V3, r: f32, colour: [f32; 3], mat: Material, y: f32| {
            out.instances.push(
                Instance::new(
                    m.disc,
                    mat,
                    basis([r, 0.0, 0.0], UP, [0.0, 0.0, r], [at[0], at[1] + y, at[2]]),
                )
                .coloured(colour),
            );
        };
        // the exhaust: a flickering flame of the fighter's colour out of the casing end
        let flick = 0.75 + 0.25 * sin(age * 2.3) as f32;
        let flame = maths::sub(tail, maths::scale(dir, 0.22 * s * flick));
        strip(
            out,
            flame,
            tail,
            0.16 * s,
            col(mix(fighter, 0.6, 100.0)),
            Material::Additive { strength: 0.6 },
            0.001,
        );
        strip(
            out,
            tail,
            nose,
            0.1 * s,
            col(PISTON.map(|v| v * 0.82)),
            Material::Unlit,
            0.004,
        );
        strip(
            out,
            maths::add(tail, maths::scale(n, 0.018)),
            maths::add(nose, maths::scale(n, 0.018)),
            0.022 * s,
            col([250.0, 252.0, 255.0]),
            Material::Unlit,
            0.007,
        );
        disc(out, tail, 0.095 * s, col(COLLAR), Material::Unlit, 0.01);
        disc(
            out,
            nose,
            0.16 * s,
            col(mix(fighter, 0.8, 30.0)),
            Material::Unlit,
            0.013,
        );
        disc(
            out,
            maths::add(nose, maths::scale(dir, 0.015)),
            0.125 * s,
            col(PEARL),
            Material::Unlit,
            0.016,
        );
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
        if matches!(e, EvKind::ProjHit { .. }) {
            // confetti and herald poles: many small hits, so each barely sparks
            return Restraint {
                particles: 0.03,
                glow: 0.06,
                life: 0.4,
                size: 0.25,
                marks: 0.0,
                screen: 0.0,
                lens: false,
            };
        }
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
