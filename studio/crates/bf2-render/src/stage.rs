//! The stage: the arena, the ring, the crowd, the LED boards, and each fighter's glass ball.
//!
//! Every mesh is made once in [`Stage::new`]; the crowd's places and colours are seeded there too, so a frame
//! only places instances. Sizes are the sim's (`bf2_sim::Rules2`): the ropes enclose a square of half-size
//! [`RING_A`], and a ball has radius [`BALL_R`].

use crate::geom;
use crate::gpu::Gpu;
use crate::look2::FightFrame;
use crate::maths::{self, V3};
use crate::mesh::{MeshData, MeshId};
use crate::renderer::Renderer;
use crate::scene::{Instance, Light, Material, Scene, Spot, to_render};

/// The ring's half-size inside the ropes (`Rules2::a`).
pub const RING_A: f32 = 3.0;
/// A ball's radius (`Rules2::r`).
pub const BALL_R: f32 = 0.6;
/// The ambient light of the arena.
pub const AMBIENT: f32 = 0.06;

/// The posts stand just outside the ropes' square; the canvas runs on past them as the apron.
const POST: f32 = RING_A + 0.25;
const APRON: f32 = POST + 0.45;
/// The arena floor, below the ring's platform.
const FLOOR_Y: f32 = -1.2;
const ROPE_HEIGHTS: [f32; 3] = [0.45, 0.85, 1.25];
const ROPE_R: f32 = 0.035;
const ROPE_SAG: f32 = 0.06;
const POST_H: f32 = 1.45;
/// The stands: tiers rising away from the ring on all four sides.
const TIERS: usize = 3;
const STAND_IN: f32 = 8.0;
const TIER_DEPTH: f32 = 1.6;
const TIER_RISE: f32 = 0.8;
const SEAT_GAP: f32 = 0.55;
/// LED board segments per side, on the front of the first tier.
const LEDS: usize = 16;
const PROJ_R: f32 = 0.15;

const CANVAS: [f32; 3] = [0.075, 0.08, 0.1];
const SKIRT: [f32; 3] = [0.05, 0.06, 0.1];
const FLOOR: [f32; 3] = [0.035, 0.035, 0.045];
const STANDS: [f32; 3] = [0.09, 0.09, 0.12];
const ROPE: [f32; 3] = [0.92, 0.92, 0.95];
const POST_NEUTRAL: [f32; 3] = [0.55, 0.56, 0.6];
const SPOT_COLOUR: [f32; 3] = [6.0, 5.7, 5.1];

/// A ball's centre in render space: the sim position is its bottom.
pub fn ball_centre(pos: [f64; 3]) -> V3 {
    maths::add(to_render(pos), [0.0, BALL_R, 0.0])
}

/// One seat of the crowd.
struct Fan {
    pos: V3,
    yaw: f32,
    phase: f32,
    colour: [f32; 3],
}

pub struct Stage {
    floor: MeshId,
    unit_box: MeshId,
    canvas: MeshId,
    post: MeshId,
    ropes: MeshId,
    fan: MeshId,
    ball: MeshId,
    disc: MeshId,
    proj: MeshId,
    crowd: Vec<Fan>,
}

/// A small, fixed sequence (splitmix64): the crowd is the same every run and on every PC.
struct Seq(u64);

impl Seq {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// The four sides of a square: (outward direction, along direction) in the floor plane of render space.
const SIDES: [(V3, V3); 4] = [
    ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
    ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
];

/// A rotation about Y taking +X to `along` (so a box's length runs along a side).
fn yaw_of(along: V3) -> f32 {
    (-along[2]).atan2(along[0])
}

/// Three ropes a side, each a chain of capsules sagging between the posts.
fn ropes_mesh() -> MeshData {
    const LINKS: usize = 8;
    let mut m = MeshData::default();
    for (out, along) in SIDES {
        for h in ROPE_HEIGHTS {
            let at = |s: f32| {
                let base = maths::add(maths::scale(out, POST), maths::scale(along, (2.0 * s - 1.0) * POST));
                maths::add(base, [0.0, h - ROPE_SAG * 4.0 * s * (1.0 - s), 0.0])
            };
            for i in 0..LINKS {
                let (a, b) = (at(i as f32 / LINKS as f32), at((i + 1) as f32 / LINKS as f32));
                m.append(&geom::capsule(a, b, ROPE_R, 8));
            }
        }
    }
    m.painted(ROPE, 0.3)
}

/// A low-poly fan: a body and a head, standing on y = 0.
fn fan_mesh() -> MeshData {
    let mut m = geom::capsule([0.0, 0.16, 0.0], [0.0, 0.42, 0.0], 0.16, 6);
    let mut head = geom::icosphere(0.13, 0);
    for v in &mut head.verts {
        v.pos[1] += 0.72;
    }
    m.append(&head);
    m
}

impl Stage {
    pub fn new(r: &mut Renderer, gpu: &Gpu) -> Stage {
        let mut add = |m: MeshData| r.meshes.add(gpu, m);
        let floor = add(geom::plane(120.0, 120.0));
        let unit_box = add(geom::box_([1.0, 1.0, 1.0]));
        let canvas = add(geom::plane(2.0 * APRON, 2.0 * APRON).painted([1.0; 3], 0.0));
        let post = add(geom::cylinder(0.11, POST_H, 12).painted([1.0; 3], 0.6));
        let ropes = add(ropes_mesh());
        let fan = add(fan_mesh());
        let ball = add(geom::icosphere(BALL_R, 3));
        let disc = add(geom::cylinder(1.0, 0.004, 32));
        let proj = add(geom::icosphere(PROJ_R, 2));

        let mut seq = Seq(0xBA11_F164_7000_0002);
        let mut crowd = Vec::new();
        for (out, along) in SIDES {
            for tier in 0..TIERS {
                let dist = STAND_IN + TIER_DEPTH * (tier as f32 + 0.5);
                let top = FLOOR_Y + TIER_RISE * (tier + 1) as f32;
                let half_len = STAND_IN + TIER_DEPTH * tier as f32;
                let seats = (2.0 * half_len / SEAT_GAP) as usize;
                for s in 0..seats {
                    // Some seats are empty.
                    if seq.next() < 0.12 {
                        continue;
                    }
                    let u = (s as f32 + 0.5) * SEAT_GAP - half_len + (seq.next() - 0.5) * 0.15;
                    let pos = maths::add(maths::scale(out, dist), maths::scale(along, u));
                    let hue = seq.next();
                    let light = 0.25 + 0.45 * seq.next();
                    crowd.push(Fan {
                        pos: [pos[0], top, pos[2]],
                        yaw: yaw_of(along),
                        phase: seq.next() * std::f32::consts::TAU,
                        colour: hue_colour(hue, light),
                    });
                }
            }
        }
        Stage {
            floor,
            unit_box,
            canvas,
            post,
            ropes,
            fan,
            ball,
            disc,
            proj,
            crowd,
        }
    }

    /// The whole stage for frame `f`: the arena and both balls. Projectiles are drawn by
    /// [`Stage::draw_projectiles`], once the looks have said which ones they drew.
    pub fn draw(&self, f: &FightFrame<'_>, colours: [[f32; 3]; 2], out: &mut Scene) {
        self.draw_arena(f, colours, out);
        self.draw_balls(f, colours, out);
    }

    /// Everything but the balls: the lights, the arena, the stands, the crowd, the LED boards and the ring.
    pub fn draw_arena(&self, f: &FightFrame<'_>, colours: [[f32; 3]; 2], out: &mut Scene) {
        out.ambient = [AMBIENT; 3];
        out.spot = Some(Spot {
            pos: [0.0, 14.0, 0.0],
            target: [0.0, 0.0, 0.0],
            colour: SPOT_COLOUR,
            angle_deg: 22.0,
            shadow: true,
        });
        let t = f.t_video as f32;
        let toon = Material::toon();
        let matte = Material::matte();
        let put = |out: &mut Scene, mesh, material, model, rgb| {
            out.instances.push(Instance::new(mesh, material, model).coloured(rgb));
        };

        put(out, self.floor, matte, maths::translation([0.0, FLOOR_Y, 0.0]), FLOOR);
        for (out_dir, along) in SIDES {
            let yaw = yaw_of(along);
            for tier in 0..TIERS {
                let dist = STAND_IN + TIER_DEPTH * (tier as f32 + 0.5);
                let height = TIER_RISE * (tier + 1) as f32;
                let len = 2.0 * (STAND_IN + TIER_DEPTH * (tier as f32 + 1.0));
                let c = maths::add(maths::scale(out_dir, dist), [0.0, FLOOR_Y + height / 2.0, 0.0]);
                put(
                    out,
                    self.unit_box,
                    matte,
                    maths::trs(c, yaw, [len, height, TIER_DEPTH]),
                    STANDS,
                );
            }
            // The LED boards: a chase of the leader's colour along the first tier's front.
            let leader = if f.snap.hp[0] >= f.snap.hp[1] {
                colours[0]
            } else {
                colours[1]
            };
            let seg = 2.0 * STAND_IN / LEDS as f32;
            for i in 0..LEDS {
                let u = (i as f32 + 0.5) * seg - STAND_IN;
                let wave = 0.5 + 0.5 * (t * 3.0 - i as f32 * 0.7).sin();
                let c = maths::add(
                    maths::add(maths::scale(out_dir, STAND_IN - 0.03), maths::scale(along, u)),
                    [0.0, FLOOR_Y + TIER_RISE * 0.55, 0.0],
                );
                put(
                    out,
                    self.unit_box,
                    Material::Emissive {
                        strength: 0.6 + 2.4 * wave,
                    },
                    maths::trs(c, yaw, [seg * 0.9, 0.3, 0.05]),
                    leader,
                );
            }
        }
        for fan in &self.crowd {
            let bob = 0.04 * (t * 2.2 + fan.phase).sin().max(0.0);
            let p = maths::add(fan.pos, [0.0, bob, 0.0]);
            put(out, self.fan, toon, maths::trs(p, fan.yaw, [1.0; 3]), fan.colour);
        }

        // The ring: the canvas, its skirt down to the floor, the posts and the ropes.
        put(out, self.canvas, matte, maths::IDENTITY, CANVAS);
        put(
            out,
            self.unit_box,
            matte,
            maths::trs(
                [0.0, (FLOOR_Y - 0.01) / 2.0, 0.0],
                0.0,
                [2.0 * APRON, -FLOOR_Y - 0.01, 2.0 * APRON],
            ),
            SKIRT,
        );
        for (k, corner) in [[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]].iter().enumerate() {
            // Sim (−x, −y) is fighter 0's corner and (+x, +y) fighter 1's; render z is −y.
            let colour = match k {
                0 => colours[0],
                2 => colours[1],
                _ => POST_NEUTRAL,
            };
            let p = [corner[0] * POST, POST_H / 2.0, corner[1] * POST];
            put(out, self.post, toon, maths::translation(p), colour);
        }
        put(out, self.ropes, toon, maths::IDENTITY, [1.0; 3]);
    }

    /// Each ball's glass shell, tinted in its fighter's colour, and its contact shadow on the canvas.
    pub fn draw_balls(&self, f: &FightFrame<'_>, colours: [[f32; 3]; 2], out: &mut Scene) {
        for (pos, colour) in f.snap.pos.iter().zip(colours) {
            let c = ball_centre(*pos);
            out.instances
                .push(Instance::new(self.ball, Material::glass(colour), maths::translation(c)));
            // The contact shadow shrinks and fades as the ball rises.
            let lift = (pos[2] as f32).max(0.0);
            let fade = (1.0 - lift / 2.5).clamp(0.0, 1.0);
            if fade > 0.0 {
                let r = BALL_R * (0.55 + 0.35 * fade);
                out.instances.push(Instance::new(
                    self.disc,
                    Material::Glass {
                        tint: [0.0; 3],
                        opacity: 0.55 * fade,
                        fresnel: 0.0,
                    },
                    maths::trs([c[0], 0.004, c[2]], 0.0, [r, 1.0, r]),
                ));
            }
        }
    }

    /// A glowing sphere and a small light for every live projectile whose id is not in `drawn`.
    pub fn draw_projectiles(&self, f: &FightFrame<'_>, drawn: &[u32], out: &mut Scene) {
        for &(id, pos) in &f.snap.projs {
            if drawn.contains(&id) {
                continue;
            }
            let p = to_render(pos);
            out.instances.push(
                Instance::new(self.proj, Material::Emissive { strength: 3.0 }, maths::translation(p))
                    .coloured([1.0, 0.75, 0.4]),
            );
            out.lights.push(Light {
                pos: p,
                colour: [1.2, 0.8, 0.4],
                range: 2.5,
            });
        }
    }

    /// The default projectile's mesh.
    pub fn proj_mesh(&self) -> MeshId {
        self.proj
    }
}

/// A crowd colour: hue 0..1 at a lightness, muted so the ring stays the brightest thing.
fn hue_colour(h: f32, light: f32) -> [f32; 3] {
    let k = |n: f32| {
        let x = (n + h * 6.0) % 6.0;
        (x - 3.0).abs().clamp(1.0, 2.0) - 1.0
    };
    let rgb = [k(5.0), k(3.0), k(1.0)];
    let grey = 0.55;
    rgb.map(|c| light * (grey + (1.0 - grey) * c))
}

#[cfg(test)]
#[path = "stage_tests.rs"]
mod tests;
