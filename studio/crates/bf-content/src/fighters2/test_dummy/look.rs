//! TEST DUMMY's look: a grey toon "fist on an arm" (each rig segment a capsule: a cylinder between two spheres,
//! the fist a bigger ball), a metronome made of boxes inside the glass, its pendulum ticking, and orange glowing
//! projectiles, each with a point light.

use std::sync::Arc;

use bf2_render::maths::{self, M4, V3};
use bf2_render::stage::ball_centre;
use bf2_render::{FightFrame, Gpu, Instance, Light, Look2, Material, MeshId, Renderer, Scene, geom, to_render};
use bf2_sim::{EvKind, FighterDef, Pose, Segment, solve};

/// The dummy's grey and its orange accent (linear).
pub const GREY: [f32; 3] = [0.55, 0.56, 0.58];
pub const ORANGE: [f32; 3] = [1.0, 0.42, 0.08];

struct Meshes {
    cyl: MeshId,
    sphere: MeshId,
    cube: MeshId,
}

pub struct DummyLook {
    def: Arc<FighterDef>,
    meshes: Option<Meshes>,
}

impl DummyLook {
    pub fn new(def: Arc<FighterDef>) -> DummyLook {
        DummyLook { def, meshes: None }
    }
}

/// Column-major: `x`, `y`, `z` axes then the origin.
fn basis(x: V3, y: V3, z: V3, o: V3) -> M4 {
    [
        [x[0], x[1], x[2], 0.0],
        [y[0], y[1], y[2], 0.0],
        [z[0], z[1], z[2], 0.0],
        [o[0], o[1], o[2], 1.0],
    ]
}

/// A unit cylinder (radius 1, height 1, along Y, centred) stretched over `a`..`b` with radius `r`.
fn along(a: V3, b: V3, r: f32) -> M4 {
    let d = maths::sub(b, a);
    let len = maths::length(d).max(1e-5);
    let y = maths::scale(d, 1.0 / len);
    let helper = if y[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let x = maths::normalize(maths::cross(helper, y));
    let z = maths::cross(x, y);
    basis(
        maths::scale(x, r),
        d,
        maths::scale(z, r),
        maths::scale(maths::add(a, b), 0.5),
    )
}

/// Turn by `yaw` about Y, then by `roll` about the turned Z, then box `size`, about `o` (render space).
fn boxed(o: V3, yaw: f32, roll: f32, offset: V3, size: V3) -> M4 {
    let (sy, cy) = yaw.sin_cos();
    let (sr, cr) = roll.sin_cos();
    // The facing frame: X toward the facing, Z across it.
    let fx = [cy, 0.0, -sy];
    let fz = [sy, 0.0, cy];
    let up = [0.0, 1.0, 0.0];
    // Roll in the facing frame's Z-Y plane (a pendulum swinging across the facing).
    let ry = maths::add(maths::scale(up, cr), maths::scale(fz, -sr));
    let rz = maths::add(maths::scale(up, sr), maths::scale(fz, cr));
    let p = maths::add(
        o,
        maths::add(
            maths::scale(fx, offset[0]),
            maths::add(maths::scale(ry, offset[1]), maths::scale(rz, offset[2])),
        ),
    );
    basis(
        maths::scale(fx, size[0]),
        maths::scale(ry, size[1]),
        maths::scale(rz, size[2]),
        p,
    )
}

/// Who threw projectile `id`.
fn owner(f: &FightFrame<'_>, id: u32) -> Option<usize> {
    f.rec.events.iter().find_map(|e| match e.kind {
        EvKind::Spawn { proj } if proj == id as usize => Some(e.who),
        _ => None,
    })
}

impl Look2 for DummyLook {
    fn setup(&mut self, r: &mut Renderer, gpu: &Gpu) {
        self.meshes = Some(Meshes {
            cyl: r.meshes.add(gpu, geom::cylinder(1.0, 1.0, 20)),
            sphere: r.meshes.add(gpu, geom::icosphere(1.0, 2)),
            cube: r.meshes.add(gpu, geom::box_([1.0, 1.0, 1.0])),
        });
    }

    fn draw(&self, f: &FightFrame<'_>, k: usize, out: &mut Scene) -> Vec<u32> {
        let Some(m) = &self.meshes else {
            return Vec::new();
        };
        let toon = Material::toon;
        let pos = f.snap.pos[k];
        let facing = f.snap.facing[k];
        let r_ball = self.def.r;

        // The arm.
        let mut segs: Vec<Segment> = Vec::new();
        let centre = [pos[0], pos[1], pos[2] + r_ball];
        let pose = Pose {
            joints: f.snap.joints[k].clone(),
        };
        if pose.joints.len() == self.def.rig.bones.len() {
            solve(&self.def.rig, &pose, centre, r_ball, facing, &mut segs);
        }
        let last = segs.len().saturating_sub(1);
        for (i, s) in segs.iter().enumerate() {
            let (a, b, r) = (to_render(s.a), to_render(s.b), s.radius as f32);
            out.instances
                .push(Instance::new(m.cyl, toon(), along(a, b, r)).coloured(GREY));
            out.instances
                .push(Instance::new(m.sphere, toon(), maths::at_scaled(a, r * 1.05)).coloured(GREY));
            if i == last {
                // The fist: a bigger ball with an orange knuckle band.
                out.instances
                    .push(Instance::new(m.sphere, toon(), maths::at_scaled(b, r * 1.55)).coloured([0.7, 0.71, 0.73]));
                let d = maths::normalize(maths::sub(b, a));
                out.instances.push(
                    Instance::new(
                        m.cyl,
                        toon(),
                        along(b, maths::add(b, maths::scale(d, r * 0.5)), r * 1.45),
                    )
                    .coloured(ORANGE),
                );
            }
        }

        // The metronome inside the glass.
        let c = ball_centre(pos);
        let yaw = facing as f32;
        let wood = [0.32, 0.27, 0.24];
        let base = [c[0], c[1] - 0.42, c[2]];
        out.instances.push(
            Instance::new(
                m.cube,
                toon(),
                boxed(base, yaw, 0.0, [0.0, 0.03, 0.0], [0.34, 0.08, 0.36]),
            )
            .coloured(wood),
        );
        // A tapering body: three stacked boxes.
        for (i, (w, h)) in [(0.3, 0.2), (0.24, 0.2), (0.17, 0.18)].iter().enumerate() {
            let y = 0.17 + 0.2 * i as f32;
            out.instances.push(
                Instance::new(m.cube, toon(), boxed(base, yaw, 0.0, [-0.04, y, 0.0], [0.18, *h, *w])).coloured(wood),
            );
        }
        // The face plate and the ticking pendulum, toward the facing.
        out.instances.push(
            Instance::new(
                m.cube,
                toon(),
                boxed(base, yaw, 0.0, [0.05, 0.36, 0.0], [0.02, 0.5, 0.12]),
            )
            .coloured([0.85, 0.82, 0.74]),
        );
        let tick = (std::f64::consts::TAU * 1.1 * f.t_video).sin() as f32 * 0.45;
        let pivot = [base[0], base[1] + 0.12, base[2]];
        out.instances.push(
            Instance::new(
                m.cube,
                toon(),
                boxed(pivot, yaw, tick, [0.08, 0.25, 0.0], [0.02, 0.5, 0.025]),
            )
            .coloured([0.85, 0.86, 0.9]),
        );
        out.instances.push(
            Instance::new(
                m.cube,
                Material::Emissive { strength: 2.2 },
                boxed(pivot, yaw, tick, [0.09, 0.36, 0.0], [0.035, 0.07, 0.07]),
            )
            .coloured(ORANGE),
        );

        // Projectiles: orange glowing balls with a light.
        let mut drawn = Vec::new();
        for &(id, p) in &f.snap.projs {
            if owner(f, id) != Some(k) {
                continue;
            }
            let p = to_render(p);
            out.instances.push(
                Instance::new(m.sphere, Material::Emissive { strength: 4.0 }, maths::at_scaled(p, 0.2))
                    .coloured(ORANGE),
            );
            out.lights.push(Light {
                pos: p,
                colour: [3.0, 1.3, 0.3],
                range: 3.0,
                priority: 0,
            });
            drawn.push(id);
        }
        drawn
    }
}

