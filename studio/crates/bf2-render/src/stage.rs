//! The stage: the current engine's ring and arena (`bf_picture::scene`, `bf_picture::ring::{arena, canvas, ropes,
//! ball}`), lifted to 3D at a higher fidelity (E7, "same style, higher fidelity": the mapping is
//! `docs/studio/engine2/style-port.md`).
//!
//! Render space is the current engine's ring frame: render `(x, z)` is its `(sx, sz)` and render `y` its height,
//! so its world is render space turned 45 degrees about Y ([`from_world`]) and the base camera
//! (`bf2_direct::base`) sees the ring as the same diamond. Everything is the current scene's: the slate-purple
//! canvas with its rings, star and apron band ([`bf_picture::scene::canvas_albedo`], baked into an albedo map with
//! a woven normal map), the skirt and its band, the padded posts with each fighter's colour on its corner, the
//! ropes with the middle one in its side's fighter colour, the LED boards (each half in its fighter's colour, a
//! chase stepping on the beat), ten stepped tiers and the seeded crowd (`bf_picture::scene::Scene::crowd`: the
//! same people, shirts and phones), the dark floor with the spill under the ring, and the spot above.
//!
//! Colours: the current engine paints sRGB 0..255; here they are linear. What the current engine shades outside
//! the spot (stands, crowd, skirt, floor) gets the colour it ends up with there, divided by the stage's ambient,
//! so the ambient alone gives the same palette and the effects' lights still light it. What stands in the spot
//! (canvas, posts, ropes, balls) gets its albedo and is lit.
//!
//! Every mesh and map is made once in [`Stage::new`]; a frame only places instances.

use bf_picture::camera::interp_keys;
use bf_picture::lighting::{Lights, shade};
use bf_picture::ring::state::camera_keys;
use bf_picture::scene as cur;

use crate::geom;
use crate::gpu::Gpu;
use crate::look2::FightFrame;
use crate::maths::{self, V3};
use crate::mesh::{MeshData, MeshId, Vertex};
use crate::renderer::Renderer;
use crate::scene::{Beam, Instance, Light, Material, Scene, Spot, Surface, to_render};
use crate::texture::{TEX_SIZE, TexId, hammered_height, normals_from_height, weave_height};

/// The ring's half-size inside the ropes (`Rules2::a`, the current engine's `A`).
pub const RING_A: f32 = cur::A as f32;
/// A ball's radius (`Rules2::r`, `R_BALL`).
pub const BALL_R: f32 = cur::R_BALL as f32;
/// The arena's ambient light (linear): the current engine's ambient (`Lights::new([0.36, 0.33, 0.44], ..)`) in
/// linear light.
pub const AMBIENT: [f32; 3] = [0.106, 0.087, 0.164];

/// The corner posts (`POST`); the canvas runs on past them as the apron (`AP`).
pub const POST: f32 = cur::POST as f32;
pub const APRON: f32 = cur::AP as f32;
/// The arena floor, below the ring's platform (`FLOOR_Y`).
const FLOOR_Y: f32 = cur::FLOOR_Y as f32;
/// The three ropes' heights (`ROPE_Y`).
pub const ROPE_HEIGHTS: [f32; 3] = [cur::ROPE_Y[0] as f32, cur::ROPE_Y[1] as f32, cur::ROPE_Y[2] as f32];
/// A rope's radius: the current engine's 2 px line at the ring's distance.
pub const ROPE_R: f32 = 0.028;
/// How far a rope sags in the middle (`rope_points`: `0.035 sin(pi u)`).
pub const ROPE_SAG: f32 = 0.035;
const POST_H: f32 = cur::POST_H as f32;
const PROJ_R: f32 = 0.15;
/// The arena's reactions: how far the crowd hops at a full jump (the current engine's one-pixel hop, a little more), and how much brighter the LED boards are at
/// a full flash (added strength; on the beat they pulse as the current engine's do).
const CROWD_JUMP: f32 = 0.08;
const LED_FLASH: f32 = 0.8;
/// The LED boards round the ring floor.
const BOARDS: usize = 64;
/// The spot (`SPOT`, `SPOT_AIM`) and its cone (`SPOT_COS`: soft from 19 to 27 degrees).
const SPOT_ANGLE: f32 = 27.0;
const SPOT_COLOUR: [f32; 3] = [1.3, 1.22, 1.3];
/// The fill of the stands' and crowd's light the current engine's directional key gives them, in linear light.
const NEUTRAL: [f64; 3] = [236.0, 232.0, 222.0];
/// The current engine's ball glass (`title::GLASS`) and the ink round a ball.
const GLASS: [f64; 3] = [236.0, 232.0, 226.0];
const INK: [f64; 3] = [22.0, 14.0, 30.0];
/// The current engine's crowd: how much of its shirt colour reaches the picture (`spill`), and its body and
/// head shades.
const SPILL: f64 = 0.48;

/// An sRGB 0..255 colour in linear light.
pub fn linear(c: [f64; 3]) -> [f32; 3] {
    c.map(|v| bf_core::dmath::powf((v / 255.0).clamp(0.0, 4.0) as f32, 2.2))
}

/// A linear colour as sRGB 0..255 (the fighters' colours come linear).
fn srgb255(c: [f32; 3]) -> [f64; 3] {
    c.map(|v| 255.0 * f64::from(bf_core::dmath::powf(v.max(0.0), 1.0 / 2.2)))
}

/// A current-engine world point in render space.
pub fn from_world(p: [f64; 3]) -> V3 {
    let (x, z) = cur::to_ring(p[0], p[2]);
    [x as f32, p[1] as f32, z as f32]
}

/// What the ambient alone must be lit by to show the current engine's `target` (sRGB 0..255).
fn baked(target: [f64; 3]) -> [f32; 3] {
    let l = linear(target);
    [0, 1, 2].map(|c| l[c] / AMBIENT[c])
}

/// A ball's centre in render space: the sim position is its bottom.
pub fn ball_centre(pos: [f64; 3]) -> V3 {
    maths::add(to_render(pos), [0.0, BALL_R, 0.0])
}

/// The four sides of a square: (outward direction, along direction) in the floor plane of render space.
pub const SIDES: [(V3, V3); 4] = [
    ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
    ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
];

/// Whose colour each side's middle rope carries, in [`SIDES`] order: the current engine's rope sides touching the
/// red corner (render (−x, +z), fighter 0's) are fighter 0's, the others fighter 1's.
pub const SIDE_FIGHTER: [usize; 4] = [1, 1, 0, 0];

/// The current engine's lights for shading the set outside the spot (`state::lights` without the corner lights
/// and the flares): its ambient, sky and key, seen from the base camera.
fn current_lights() -> Lights {
    let mut l = Lights::new([0.36, 0.33, 0.44], [0.10, 0.08, 0.14]);
    let d = [
        cur::SPOT[0] - cur::SPOT_AIM[0],
        cur::SPOT[1] - cur::SPOT_AIM[1],
        cur::SPOT[2] - cur::SPOT_AIM[2],
    ];
    l.key("spot", d, [1.05, 1.0, 0.96]);
    // the base camera's view: from +Z, 58 degrees up
    let p = 58f64.to_radians();
    l.view = [0.0, bf_core::dmath::sin(p), bf_core::dmath::cos(p)];
    l
}

/// A rope's colour as the current engine draws it (`draw_rope_side`): its base shaded once on the normal
/// [0, 1, 0.3], in linear light (drawn unlit, so the rope stays a flat line).
fn rope_colour(base: [f64; 3]) -> [f32; 3] {
    let up = [0.0, 1.0 / 1.09f64.sqrt(), 0.3 / 1.09f64.sqrt()];
    linear(shade(&current_lights(), &[up], &[[0.0; 3]], &[base], None, 0.5, 0.0, 5.0)[0])
}

/// Capsules per rope.
const LINKS: usize = 8;

/// Where a rope of side `side` at height `h` is at share `s` (0 to 1) of its length, bowed out by `wobble` in the
/// middle (outward positive; the posts hold its ends).
fn rope_at(side: usize, h: f32, s: f32, wobble: f32) -> V3 {
    let (out, along) = SIDES[side];
    let bow = POST + wobble * (std::f32::consts::PI * s).sin();
    let base = maths::add(maths::scale(out, bow), maths::scale(along, (2.0 * s - 1.0) * POST));
    maths::add(base, [0.0, h - ROPE_SAG * (std::f32::consts::PI * s).sin(), 0.0])
}

/// One rope hanging still: a chain of capsules sagging between the posts, white (the instance colours it).
fn rope_mesh(side: usize, h: f32) -> MeshData {
    let mut m = MeshData::default();
    for i in 0..LINKS * 2 {
        let n = (LINKS * 2) as f32;
        let (a, b) = (
            rope_at(side, h, i as f32 / n, 0.0),
            rope_at(side, h, (i + 1) as f32 / n, 0.0),
        );
        m.append(&geom::capsule(a, b, ROPE_R, 10));
    }
    m.painted([1.0; 3], 0.35)
}

/// The model that takes the unit link (a capsule from the origin to +X) to the segment `a`→`b`: stretched along
/// its length, its radius kept.
fn link_model(a: V3, b: V3) -> maths::M4 {
    let d = maths::sub(b, a);
    let x = maths::normalize(d);
    // A rope runs mostly level, so its second axis is level too (or up, for a link along Z).
    let side = maths::cross([0.0, 1.0, 0.0], x);
    let y = if maths::length(side) < 1e-4 {
        [1.0, 0.0, 0.0]
    } else {
        maths::cross(x, maths::normalize(side))
    };
    let z = maths::cross(x, y);
    [
        [d[0], d[1], d[2], 0.0],
        [y[0], y[1], y[2], 0.0],
        [z[0], z[1], z[2], 0.0],
        [a[0], a[1], a[2], 1.0],
    ]
}

/// The canvas's albedo map: [`cur::canvas_albedo`] over the apron (u along +x, v along −z from the near edge),
/// sRGB.
pub fn canvas_map(size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    let ap = cur::AP;
    for j in 0..size {
        let z = ap - (f64::from(j) + 0.5) / f64::from(size) * 2.0 * ap;
        for i in 0..size {
            let x = -ap + (f64::from(i) + 0.5) / f64::from(size) * 2.0 * ap;
            let c = cur::canvas_albedo(x, z);
            out.extend(c.map(|v| v.round().clamp(0.0, 255.0) as u8));
            out.push(255);
        }
    }
    out
}

/// The skirt: the current platform's faces (`platform_faces`: three bands a side), each coloured as the
/// current engine shades it, baked for the ambient.
fn skirt_mesh(l: &Lights) -> MeshData {
    let sc = cur::Scene::new([[0.0; 3]; 2]);
    let mut m = MeshData::default();
    for f in &sc.skirt_faces {
        let ctr = [0, 1, 2].map(|k| f.q.iter().map(|p| p[k]).sum::<f64>() / f.q.len() as f64);
        let target = shade(l, &[f.n], &[ctr], &[f.col], None, 0.3, 0.0, 5.0)[0];
        let n = from_world(f.n);
        let base = m.verts.len() as u32;
        for p in &f.q {
            let mut v = Vertex::new(from_world(*p), n);
            v.rgb = baked(target);
            m.verts.push(v);
        }
        // wound to face `n`
        let (a, b, c) = (
            m.verts[base as usize].pos,
            m.verts[base as usize + 1].pos,
            m.verts[base as usize + 2].pos,
        );
        let g = maths::cross(maths::sub(b, a), maths::sub(c, a));
        if maths::dot(g, n) >= 0.0 {
            m.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        } else {
            m.idx.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
        }
    }
    m
}

/// Adds one flat face of the current engine (`q` and `n` in its world) to `m`, every corner `rgb`, wound to face `n`.
fn push_face(m: &mut MeshData, q: &[[f64; 3]], n: [f64; 3], rgb: [f32; 3]) {
    let n = from_world(n);
    let base = m.verts.len() as u32;
    for p in q {
        let mut v = Vertex::new(from_world(*p), n);
        v.rgb = rgb;
        m.verts.push(v);
    }
    let (a, b, c) = (
        m.verts[base as usize].pos,
        m.verts[base as usize + 1].pos,
        m.verts[base as usize + 2].pos,
    );
    let g = maths::cross(maths::sub(b, a), maths::sub(c, a));
    for k in 1..q.len() as u32 - 1 {
        if maths::dot(g, n) >= 0.0 {
            m.idx.extend([base, base + k, base + k + 1]);
        } else {
            m.idx.extend([base, base + k + 1, base + k]);
        }
    }
}

/// The grey every post face is shaded on to find how the current engine's light falls on it (kept under 255, so
/// nothing clips): a face's light is its shaded grey over this.
const PROBE: f64 = 128.0;

/// A corner post as the current engine draws it (`post_faces`, `draw_post`): six-sided prisms, flat faces, each
/// lit once by the current engine's `shade` and baked for the ambient, so the ambient alone shows the current
/// picture's flat tones. The steel pole and cap carry their colour; the three cushions ([`PostMeshes::cushions`])
/// carry only the light, and their instance gives them the corner's colour (the same in linear light, as
/// `(c f / 255)^2.2 = (c / 255)^2.2 f^2.2`).
fn post_meshes(l: &Lights, corner: usize) -> (MeshData, MeshData) {
    let sc = cur::Scene::new([[0.0; 3]; 2]);
    let w = cur::to_world(cur::CORNERS[corner][0], cur::CORNERS[corner][1], 0.0);
    let (mut steel, mut cushions) = (MeshData::default(), MeshData::default());
    for f in &sc.posts[corner] {
        let ctr = [0, 1, 2].map(|k| f.q.iter().map(|p| p[k]).sum::<f64>() / f.q.len() as f64);
        // the cushions are the r 0.16 prisms (the pole is 0.085, its cap 0.11)
        let r = f.q.iter().map(|p| (p[0] - w[0]).hypot(p[2] - w[2])).fold(0.0, f64::max);
        if r > 0.14 {
            let g = shade(l, &[f.n], &[ctr], &[[PROBE; 3]], None, 0.6, 0.0, 1.0)[0];
            push_face(&mut cushions, &f.q, f.n, baked(g.map(|v| v / PROBE * 255.0)));
        } else {
            let c = shade(l, &[f.n], &[ctr], &[f.col], None, 0.6, 0.0, 5.0)[0];
            push_face(&mut steel, &f.q, f.n, baked(c));
        }
    }
    (steel, cushions)
}

/// The stands: `N_ROWS` tiers from `STAND_R0`, each a riser and a top, `segs` segments round, coloured as the
/// current engine draws them (`shade`, then `* 0.55`), baked for the ambient.
fn stands_mesh(l: &Lights, segs: u32) -> MeshData {
    let mut m = MeshData::default();
    let p = |r: f64, a: f64, y: f64| from_world([r * a.sin(), y, -r * a.cos()]);
    for row in 0..cur::N_ROWS {
        let (r0, r1) = (
            cur::STAND_R0 + row as f64 * cur::ROW_W,
            cur::STAND_R0 + (row + 1) as f64 * cur::ROW_W,
        );
        let y = cur::FLOOR_Y + row as f64 * cur::ROW_H;
        for k in 0..segs {
            let (a0, a1) = (
                std::f64::consts::TAU * f64::from(k) / f64::from(segs),
                std::f64::consts::TAU * f64::from(k + 1) / f64::from(segs),
            );
            let am = (a0 + a1) / 2.0;
            let quads = [
                (
                    [
                        p(r0, a0, y + cur::ROW_H),
                        p(r1, a0, y + cur::ROW_H),
                        p(r1, a1, y + cur::ROW_H),
                        p(r0, a1, y + cur::ROW_H),
                    ],
                    [0.0, 1.0, 0.0],
                    [54.0, 46.0, 74.0],
                ),
                (
                    [
                        p(r0, a0, y),
                        p(r0, a0, y + cur::ROW_H),
                        p(r0, a1, y + cur::ROW_H),
                        p(r0, a1, y),
                    ],
                    [-am.sin(), 0.0, am.cos()],
                    [40.0, 34.0, 58.0],
                ),
            ];
            for (q, n, col) in quads {
                let ctr = [r0 * am.sin(), y + cur::ROW_H, -r0 * am.cos()];
                let target = shade(l, &[n], &[ctr], &[col], None, 0.2, 0.0, 5.0)[0].map(|c| c * 0.55);
                let nr = from_world(n);
                let base = m.verts.len() as u32;
                for pos in q {
                    let mut v = Vertex::new(pos, nr);
                    v.rgb = baked(target);
                    m.verts.push(v);
                }
                let g = maths::cross(maths::sub(q[1], q[0]), maths::sub(q[2], q[0]));
                if maths::dot(g, nr) >= 0.0 {
                    m.idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                } else {
                    m.idx.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
                }
            }
        }
    }
    m
}

/// The arena floor: a disc at `FLOOR_Y` coloured as the current engine lights it per pixel (`floor`: the spill
/// under the ring, the faint checks left out), drawn unlit.
fn floor_mesh() -> MeshData {
    let radii: Vec<f32> = (0..=40).map(|k| (k as f32 / 40.0).powi(2) * 70.0).collect();
    let segs = 96u32;
    let mut m = MeshData::default();
    for &r in &radii {
        let e = (-((f64::from(r) - 4.6).max(0.0)) / 3.0).exp() * 0.55;
        let light = [0.20 + e, 0.18 + e * 0.95, 0.26 + e * 1.05];
        let c = [0, 1, 2].map(|k| cur::ARENA_FLOOR[k] * light[k]);
        for j in 0..=segs {
            let a = j as f32 / segs as f32 * std::f32::consts::TAU;
            let mut v = Vertex::new([r * a.cos(), FLOOR_Y, -r * a.sin()], [0.0, 1.0, 0.0]);
            v.rgb = linear(c);
            m.verts.push(v);
        }
    }
    let row = segs + 1;
    for i in 0..radii.len() as u32 - 1 {
        for j in 0..segs {
            let (p, q) = (i * row + j, (i + 1) * row + j);
            m.idx.extend([p, q, p + 1, p + 1, q, q + 1]);
        }
    }
    m
}

/// How far a corner light's spill on the floor reaches.
const SPILL_R: f32 = 7.0;

/// A corner light's spill on the floor, centred under it, flat at y = 0: each vertex weighted as the current
/// engine's `floor` weights the light (`h / d / (1 + (d / (1.8 r))^2)`, the light `h` above the floor, `r` its
/// radius), less its weight at the edge so it fades to nothing there.
fn spill_mesh() -> MeshData {
    let h = f64::from(POST_H) + 0.35 - cur::FLOOR_Y;
    let w = |d: f64| {
        let dist = (h * h + d * d).sqrt();
        let q = dist / (2.6 * 1.8);
        h / dist / (1.0 + q * q)
    };
    let edge = w(f64::from(SPILL_R));
    let (rings, segs) = (24u32, 64u32);
    let mut m = MeshData::default();
    for i in 0..=rings {
        let r = SPILL_R * (i as f32 / rings as f32).powf(1.5);
        let k = (w(f64::from(r)) - edge).max(0.0) as f32;
        for j in 0..segs {
            let a = j as f32 / segs as f32 * std::f32::consts::TAU;
            let mut v = Vertex::new([r * a.cos(), 0.0, -r * a.sin()], [0.0, 1.0, 0.0]);
            v.rgb = [k; 3];
            m.verts.push(v);
        }
    }
    for i in 0..rings {
        for j in 0..segs {
            let (p, q) = (i * segs + j, (i + 1) * segs + j);
            let (p1, q1) = (i * segs + (j + 1) % segs, (i + 1) * segs + (j + 1) % segs);
            m.idx.extend([p, q, p1, p1, q, q1]);
        }
    }
    m
}

/// One of the crowd as the current engine draws it (`arena`): flat, a body ellipse (1.9 x 1.4 head radii, its
/// top 0.6 head radii under the head's centre) and a round head, standing on y = 0 and facing −Z (turned to the
/// camera, so it reads as the same flat figure, not a 3D one); the body's vertices at 0.8 and the head's at 0.62
/// of the shirt colour, as the current engine inks them. The head sits a hair nearer than the body.
fn person_mesh() -> MeshData {
    let disc = |cx: f32, cy: f32, rx: f32, ry: f32, z: f32, k: f32| {
        let segs = 20u32;
        let mut m = MeshData::default();
        let mut v = Vertex::new([cx, cy, z], [0.0, 0.0, -1.0]);
        v.rgb = [k.powf(2.2); 3];
        m.verts.push(v);
        for j in 0..segs {
            let a = j as f32 / segs as f32 * std::f32::consts::TAU;
            let mut v = Vertex::new([cx + rx * a.cos(), cy + ry * a.sin(), z], [0.0, 0.0, -1.0]);
            v.rgb = [k.powf(2.2); 3];
            m.verts.push(v);
        }
        for j in 0..segs {
            // wound to face −Z
            m.idx.extend([0, 1 + (j + 1) % segs, 1 + j]);
        }
        m
    };
    let mut body = disc(0.0, 0.25, 0.285, 0.21, 0.0, 0.8);
    body.append(&disc(0.0, 0.55, 0.15, 0.15, -0.01, 0.62));
    body.with_tangents()
}

/// The rim light the ring puts on a head (`arena`: a line across the head, 0.8 head radii up, 1.2 wide), facing
/// −Z in front of the head; the instance colours it.
fn rim_mesh() -> MeshData {
    let (y, hw, hh, z) = (0.55 + 0.12, 0.09, 0.012, -0.02);
    let mut m = MeshData::default();
    for (x, yy) in [(-hw, y - hh), (hw, y - hh), (hw, y + hh), (-hw, y + hh)] {
        m.verts.push(Vertex::new([x, yy, z], [0.0, 0.0, -1.0]));
    }
    m.idx.extend([0, 2, 1, 0, 3, 2]);
    m.painted([1.0; 3], 0.0)
}

/// The rotation turning the person mesh (front −Z, up +Y) at `pos` to face `eye`.
fn facing(pos: V3, eye: V3) -> maths::M4 {
    let back = maths::normalize(maths::sub(pos, eye));
    let x = maths::normalize(maths::cross([0.0, 1.0, 0.0], back));
    let y = maths::cross(back, x);
    [
        [x[0], x[1], x[2], 0.0],
        [y[0], y[1], y[2], 0.0],
        [back[0], back[1], back[2], 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// One seat of the crowd: where, which way, its shirt and when its phone twinkles.
struct Fan {
    pos: V3,
    /// Turned to face the base camera, as the current engine's flat figures do.
    turn: maths::M4,
    ht: f32,
    shirt: usize,
    /// The current engine's phase (`ph`): the phone, and the side of the ring its light comes from.
    ph: f32,
    /// −1: fighter 0's side of the screen, 1: fighter 1's, 0: the middle.
    side: i8,
}

pub struct Stage {
    floor: MeshId,
    unit_box: MeshId,
    phone: MeshId,
    canvas: MeshId,
    skirt: MeshId,
    stands: MeshId,
    /// Each corner's pole and cap, and its cushions ([`post_meshes`]).
    posts: [(MeshId, MeshId); 4],
    /// Each side's three ropes hanging still, in [`SIDES`] then [`ROPE_HEIGHTS`] order.
    ropes: [[MeshId; 3]; 4],
    /// One capsule of rope from the origin to +X, for ropes that wobble.
    rope_link: MeshId,
    person: MeshId,
    rim: MeshId,
    ball: MeshId,
    outline: MeshId,
    disc: MeshId,
    /// The corner lights' spill on the floor.
    spill: MeshId,
    proj: MeshId,
    glow: MeshId,
    crowd: Vec<Fan>,
    canvas_map: Option<TexId>,
    weave: Option<TexId>,
    dimples: Option<TexId>,
}

/// A rotation about Y taking +X to `along` (so a box's length runs along a side).
fn yaw_of(along: V3) -> f32 {
    (-along[2]).atan2(along[0])
}

impl Stage {
    pub fn new(r: &mut Renderer, gpu: &Gpu) -> Stage {
        let l = current_lights();
        let canvas_map = r.textures.add_albedo(gpu, &canvas_map(TEX_SIZE));
        let weave = r
            .textures
            .add_normal(gpu, &normals_from_height(&weave_height(TEX_SIZE, 64), TEX_SIZE, 1.2));
        let dimples = r.textures.add_normal(
            gpu,
            &normals_from_height(&hammered_height(TEX_SIZE, 12, 7), TEX_SIZE, 10.0),
        );
        let mut add = |m: MeshData| r.meshes.add(gpu, m);
        let floor = add(floor_mesh());
        let unit_box = add(geom::box_([1.0, 1.0, 1.0]));
        let phone = add(geom::box_([1.0, 1.0, 1.0]));
        let canvas = add(geom::plane(2.0 * APRON, 2.0 * APRON).painted([1.0; 3], 0.0));
        let skirt = add(skirt_mesh(&l));
        let stands = add(stands_mesh(&l, 160));
        let posts = [0, 1, 2, 3].map(|c| {
            let (steel, cushions) = post_meshes(&l, c);
            (add(steel), add(cushions))
        });
        let ropes = [0, 1, 2, 3].map(|side| ROPE_HEIGHTS.map(|h| add(rope_mesh(side, h))));
        let rope_link = add(geom::capsule([0.0; 3], [1.0, 0.0, 0.0], ROPE_R, 10).painted([1.0; 3], 0.35));
        let person = add(person_mesh());
        let rim = add(rim_mesh());
        let ball = add(geom::uv_sphere(BALL_R, 48, 24));
        // the ink outline: the far side of a slightly bigger ball (wound inside out, so only it shows)
        let mut shell = geom::icosphere(BALL_R * 1.045, 3);
        for t in shell.idx.as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
        for v in &mut shell.verts {
            v.nrm = maths::scale(v.nrm, -1.0);
        }
        let outline = add(shell);
        let disc = add(geom::cylinder(1.0, 0.004, 32));
        let spill = add(spill_mesh());
        let proj = add(geom::icosphere(PROJ_R, 2));
        let glow = add(geom::icosphere(1.0, 2));

        // The same people as the current engine (its seeded crowd), where it puts them, each turned to the
        // current engine's camera (`camera_keys` at the start) so it reads as the same figure.
        let sc = cur::Scene::new([[0.0; 3]; 2]);
        let eye = from_world(interp_keys(&camera_keys(55.0), 0.0, None).pos);
        let crowd = sc
            .crowd
            .iter()
            .map(|q| {
                let pos = from_world(q.p);
                Fan {
                    pos,
                    turn: facing(pos, eye),
                    ht: q.ht as f32,
                    shirt: q.ci,
                    ph: q.ph as f32,
                    side: if q.p[0] < -1.5 {
                        -1
                    } else if q.p[0] > 1.5 {
                        1
                    } else {
                        0
                    },
                }
            })
            .collect();
        Stage {
            floor,
            unit_box,
            phone,
            canvas,
            skirt,
            stands,
            posts,
            ropes,
            rope_link,
            person,
            rim,
            ball,
            outline,
            disc,
            spill,
            proj,
            glow,
            crowd,
            canvas_map,
            weave,
            dimples,
        }
    }

    /// The whole stage for frame `f`: the arena and both balls. Projectiles are drawn by
    /// [`Stage::draw_projectiles`], once the looks have said which ones they drew.
    pub fn draw(&self, f: &FightFrame<'_>, colours: [[f32; 3]; 2], out: &mut Scene) {
        self.draw_arena(f, colours, out);
        self.draw_balls(f, colours, out);
    }

    /// Everything but the balls: the lights, the arena, the stands, the crowd, the LED boards and the ring, with
    /// the arena's reactions of `f.arena` (a quiet arena draws as it always has): the house lights scale the
    /// ambient, the crowd hops by `crowd_jump` x [`CROWD_JUMP`], the LED boards pulse on the beat and flash in the
    /// flashing fighter's colour, and a wobbling side's ropes bow.
    pub fn draw_arena(&self, f: &FightFrame<'_>, colours: [[f32; 3]; 2], out: &mut Scene) {
        let arena = &f.arena;
        let house = arena.house as f32;
        out.ambient = AMBIENT.map(|a| a * house);
        out.spot = Some(Spot {
            pos: from_world(cur::SPOT),
            target: from_world(cur::SPOT_AIM),
            colour: SPOT_COLOUR,
            angle_deg: SPOT_ANGLE,
            shadow: true,
        });
        let t = f.t_video as f32;
        let beat = arena.beat as f32;
        let col255 = colours.map(srgb255);
        // LIGHT_RGB = COL / max(COL); CROWD_TINT = 0.78 + 0.42 LIGHT_RGB
        let light_rgb = col255.map(|c| {
            let top = c[0].max(c[1]).max(c[2]).max(1.0);
            c.map(|v| v / top)
        });
        let matte = Material::matte();
        let put = |out: &mut Scene, mesh, material, model, rgb| {
            out.instances.push(Instance::new(mesh, material, model).coloured(rgb));
        };

        // The floor, the stands and the crowd.
        put(out, self.floor, Material::Unlit, maths::IDENTITY, [house; 3]);
        put(out, self.stands, matte, maths::IDENTITY, [1.0; 3]);
        let wk = 0.0;
        for fan in &self.crowd {
            let tone = match fan.side {
                -1 => light_rgb[0].map(|v| 0.78 + 0.42 * v),
                1 => light_rgb[1].map(|v| 0.78 + 0.42 * v),
                _ => [1.0, 0.92, 1.0],
            };
            let shirt = cur::CROWD_COLS[fan.shirt];
            let body = baked([0, 1, 2].map(|c| (shirt[c] * tone[c] * (SPILL + 0.28 * wk)).clamp(0.0, 255.0)));
            let bob =
                0.02 * (t * 2.2 + fan.ph * std::f32::consts::TAU).sin().max(0.0) + CROWD_JUMP * arena.crowd_jump as f32;
            let p = maths::add(fan.pos, [0.0, bob, 0.0]);
            let mut model = fan.turn;
            for col in model.iter_mut().take(3) {
                for c in &mut col[..3] {
                    *c *= fan.ht;
                }
            }
            model[3] = [p[0], p[1], p[2], 1.0];
            put(out, self.person, matte, model, body);
            // the ring's rim light on the head (`body x 1.5 + 30`)
            let rim = baked([0, 1, 2].map(|c| {
                let b = (shirt[c] * tone[c] * (SPILL + 0.28 * wk)).clamp(0.0, 255.0);
                (b * 1.5 + 30.0).clamp(0.0, 255.0)
            }));
            put(out, self.rim, matte, model, rim);
            // phones up, twinkling (the current engine's `ph < 0.07`)
            if fan.ph < 0.07 {
                let tw = 0.65 + 0.35 * (std::f32::consts::TAU * (t / (61.0 / 137.0) + fan.ph * 11.0)).sin();
                let ph = maths::add(p, [0.0, 0.55 * fan.ht + 0.32, 0.0]);
                put(
                    out,
                    self.phone,
                    Material::Emissive { strength: 1.6 * tw },
                    maths::mul(
                        &maths::translation(ph),
                        &maths::mul(&fan.turn, &maths::trs([0.0; 3], 0.0, [0.06, 0.1, 0.02])),
                    ),
                    linear([255.0, 248.0, 226.0]),
                );
            }
        }

        // The LED boards: each fighter's colour on its own half, a chase stepping on the beat, brighter on the beat;
        // the flashing fighter's half brighter while it flashes.
        let step = (t * 2.2).floor();
        let k = 0.55 + 0.45 * beat;
        let flash = arena.led_flash.map(|(who, a)| (who.min(1), a as f32));
        for b in 0..BOARDS {
            let (a0, a1) = (
                std::f64::consts::TAU * b as f64 / BOARDS as f64,
                std::f64::consts::TAU * (b + 1) as f64 / BOARDS as f64,
            );
            let am = (a0 + a1) / 2.0;
            let side = if am.sin() < 0.0 { 0 } else { 1 };
            let u = (am / std::f64::consts::TAU * 16.0 + f64::from(step)).rem_euclid(4.0);
            let mut c = col255[side];
            if u >= 2.0 {
                c = c.map(|v| v * 0.42 + 40.0);
            }
            let mut strength = 1.0;
            // a flash brightens the flashing fighter's own half; each half keeps its colour, as the current
            // engine's boards do (E8b: a whole ring in one fighter's colour read as a white-out)
            if let Some((who, a)) = flash
                && who == side
            {
                strength += LED_FLASH * a;
            }
            // `led_colour`'s pulse, then the board's fill and its top edge as the current engine paints them
            let c = c.map(|v| v * f64::from(k));
            let fill = linear(c.map(|v| v * 0.55 + 18.0));
            let edge = linear(c.map(|v| (v * 0.9 + 60.0).min(255.0)));
            let r = cur::BARRIER_R;
            let centre = from_world([r * am.sin(), cur::FLOOR_Y + cur::BARRIER_H / 2.0, -r * am.cos()]);
            let tangent = from_world([am.cos(), 0.0, am.sin()]);
            let yaw = yaw_of(tangent);
            let w = (2.0 * r * (std::f64::consts::PI / BOARDS as f64).sin()) as f32 * 1.01;
            put(
                out,
                self.unit_box,
                Material::Emissive { strength },
                maths::trs(centre, yaw, [w, cur::BARRIER_H as f32, 0.05]),
                fill,
            );
            let top = maths::add(centre, [0.0, cur::BARRIER_H as f32 / 2.0, 0.0]);
            put(
                out,
                self.unit_box,
                Material::Emissive { strength },
                maths::trs(top, yaw, [w, 0.018, 0.06]),
                edge,
            );
        }

        // The ring: the canvas (its albedo map and woven normal map), the skirt, the posts and the ropes.
        out.instances.push(
            Instance::new(self.canvas, matte, maths::IDENTITY).with_surface(Surface {
                albedo: self.canvas_map,
                normal: self.weave,
                uv_scale: 2.0,
                normal_strength: 0.6,
                back: None,
            }),
        );
        put(out, self.skirt, matte, maths::IDENTITY, [1.0; 3]);
        for c in 0..4 {
            let (steel, cushions) = self.posts[c];
            put(out, steel, matte, maths::IDENTITY, [1.0; 3]);
            let colour = match c {
                cur::RED => col255[0].map(|v| (v * 0.9).min(255.0)),
                cur::BLUE => col255[1].map(|v| (v * 0.9).min(255.0)),
                _ => NEUTRAL,
            };
            put(out, cushions, matte, maths::IDENTITY, linear(colour));
        }
        // The corner lights over the red and blue posts, in each fighter's light colour, pulsing on the beat, and
        // their glow.
        let rk = 1.0 + 0.25 * beat;
        for (k, corner) in [cur::RED, cur::BLUE].into_iter().enumerate() {
            let at = [
                cur::CORNERS[corner][0] as f32,
                POST_H + 0.35,
                cur::CORNERS[corner][1] as f32,
            ];
            out.lights.push(Light {
                pos: at,
                colour: linear(light_rgb[k].map(|v| v * 255.0)).map(|v| v * 0.9 * rk),
                range: 4.2,
                priority: 0,
            });
            put(
                out,
                self.glow,
                Material::Emissive {
                    strength: 2.0 * (0.35 + 0.15 * beat),
                },
                maths::at_scaled(at, 0.07),
                colours[k],
            );
        }
        for (k, corner) in [cur::RED, cur::BLUE].into_iter().enumerate() {
            let c = cur::CORNERS[corner];
            // The light's spill on the floor, added as the current engine adds it (`floor`: `0.9 x` the light's
            // colour, `light_rgb x 1.1`, on the arena floor's colour), taken to linear light about the floor's
            // own light by the ring (0.75): d lin(a x) / dx = 2.2 lin(a x) / x.
            let colour = [0, 1, 2].map(|j| {
                let lit = f64::from(linear([cur::ARENA_FLOOR[j] * 0.75; 3])[0]);
                (2.2 * lit / 0.75 * 0.9 * 1.1 * light_rgb[k][j]) as f32
            });
            put(
                out,
                self.spill,
                Material::Additive { strength: rk * house },
                maths::translation([c[0] as f32, FLOOR_Y + 0.004, c[1] as f32]),
                colour,
            );
        }
        // The ropes as the current engine draws them (`draw_rope_side`): flat 2 px lines, each in one colour, its
        // base shaded once on the normal [0, 1, 0.3], unlit here so they stay flat lines.
        for (side, &w) in arena.rope_wobble.iter().enumerate() {
            let middle = rope_colour(col255[SIDE_FIGHTER[side]].map(|v| v * 0.92));
            let cream = rope_colour(cur::ROPE_COL[0]);
            for (j, &h) in ROPE_HEIGHTS.iter().enumerate() {
                let colour = if j == 1 { middle } else { cream };
                if w == 0.0 {
                    put(out, self.ropes[side][j], Material::Unlit, maths::IDENTITY, colour);
                    continue;
                }
                for i in 0..LINKS {
                    let a = rope_at(side, h, i as f32 / LINKS as f32, w as f32);
                    let b = rope_at(side, h, (i + 1) as f32 / LINKS as f32, w as f32);
                    put(out, self.rope_link, Material::Unlit, link_model(a, b), colour);
                }
            }
        }
        // The spot's beam: the current engine's dithered haze from the can down to the pool (`0.10 + 0.04 wk`).
        let pool_r = cur::SPOT[1] * cur::spot_cos().0.acos().tan() * 0.92;
        out.beam = Some(Beam {
            apex: from_world(cur::SPOT),
            centre: from_world(cur::SPOT_AIM),
            radius: pool_r as f32,
            k: 0.10 + 0.04 * wk as f32,
        });
        // The spot's can at the top of the frame.
        put(
            out,
            self.glow,
            Material::Emissive { strength: 6.0 },
            maths::at_scaled(maths::add(from_world(cur::SPOT), [0.0, 0.4, 0.0]), 0.35),
            linear([255.0, 244.0, 220.0]),
        );
    }

    /// Each fighter's ball as the current engine draws it, in glass: the fill behind what it holds (its colour
    /// 0.3 and the glass 0.7, in four bands of the spot), a near side of clear glass whose rim turns the fighter's
    /// colour, a faint hammered finish, the ink outline, and its contact shadow on the canvas.
    pub fn draw_balls(&self, f: &FightFrame<'_>, colours: [[f32; 3]; 2], out: &mut Scene) {
        for (pos, colour) in f.snap.pos.iter().zip(colours) {
            let c = ball_centre(*pos);
            let col = srgb255(colour);
            let tone = linear([0, 1, 2].map(|k| col[k] * 0.30 + GLASS[k] * 0.70));
            let rim = linear(col.map(|v| (v * 0.9).min(255.0)));
            out.instances.push(
                Instance::new(
                    self.ball,
                    Material::Glass {
                        tint: rim,
                        opacity: 0.06,
                        fresnel: 0.95,
                    },
                    maths::translation(c),
                )
                .with_surface(Surface {
                    normal: self.dimples,
                    uv_scale: 3.0,
                    normal_strength: 0.12,
                    back: Some((tone, 0.88)),
                    ..Surface::default()
                }),
            );
            out.instances
                .push(Instance::new(self.outline, Material::Unlit, maths::translation(c)).coloured(linear(INK)));
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
                priority: 0,
            });
        }
    }

    /// The default projectile's mesh.
    pub fn proj_mesh(&self) -> MeshId {
        self.proj
    }
}

#[cfg(test)]
#[path = "stage_tests.rs"]
mod tests;
