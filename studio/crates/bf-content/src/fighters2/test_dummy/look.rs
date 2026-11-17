//! TEST DUMMY's look: a grey toon "fist on an arm" (each rig segment a capsule: a cylinder between two spheres,
//! the fist a bigger ball), a metronome made of boxes inside the glass, its pendulum ticking, and orange glowing
//! projectiles, each with a point light. Its effects ([`LookFx`]): a smear on the fist through HAYMAKER, a
//! crackling lightning trail through DUMMY BUSTER and a dome and a beam on its final hit, a ribbon behind every
//! TOSS projectile, and smouldering marks where GRAB SLAM lands. All in the dummy's grey with an orange accent.

use std::sync::Arc;

use bf2_fx::decal::{DECAL_LIFE, DecalDef, DecalKind, Surface};
use bf2_fx::frame::LookFx;
use bf2_fx::meshfx::{BeamDef, DomeDef};
use bf2_fx::tone::Restraint;
use bf2_fx::trail::{PROJ, TrailDef, TrailStyle};
use bf2_fx::{Anchor, Env, Exit, FxError, FxKind, FxRecord, FxTimeline};
use bf2_render::maths::{self, M4, V3};
use bf2_render::stage::ball_centre;
use bf2_render::{FightFrame, Gpu, Instance, Light, Look2, Material, MeshId, Renderer, Scene, geom, to_render};
use bf2_sim::{EvKind, FighterDef, Pose, Recording2, Segment, solve};

use super::{DUMMY_BUSTER, FIST, GRAB_SLAM, HAYMAKER, TOSS};

/// The dummy's grey and its orange accent (linear).
pub const GREY: [f32; 3] = [0.55, 0.56, 0.58];
pub const ORANGE: [f32; 3] = [1.0, 0.42, 0.08];

struct Meshes {
    cyl: MeshId,
    sphere: MeshId,
    cube: MeshId,
    /// The limb and joint inside out: the arm's ink outline.
    cyl_ink: MeshId,
    sphere_ink: MeshId,
}

/// The ink the current engine outlines its models with (`sprite`'s `INK` over the edge), linear.
const INK: [f32; 3] = [0.008, 0.004, 0.012];
/// How far the ink shell stands out round a limb: about one art pixel at the base camera (36 art pixels a unit).
const INK_W: f32 = 0.03;

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
    let (sy, cy) = (bf_core::dmath::sinf(yaw), bf_core::dmath::cosf(yaw));
    let (sr, cr) = (bf_core::dmath::sinf(roll), bf_core::dmath::cosf(roll));
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
            // low-poly and faceted, as the current engine's models are: eight flat sides to a limb, an 80-face
            // ball to a joint and the fist
            cyl: r.meshes.add(gpu, geom::faceted(&geom::cylinder(1.0, 1.0, 8))),
            sphere: r.meshes.add(gpu, geom::faceted(&geom::icosphere(1.0, 1))),
            cube: r.meshes.add(gpu, geom::box_([1.0, 1.0, 1.0])),
            cyl_ink: r.meshes.add(gpu, geom::ink_shell(&geom::cylinder(1.0, 1.0, 8))),
            sphere_ink: r.meshes.add(gpu, geom::ink_shell(&geom::icosphere(1.0, 1))),
        });
    }

    fn draw(&self, f: &FightFrame<'_>, k: usize, out: &mut Scene) -> Vec<u32> {
        let Some(m) = &self.meshes else {
            return Vec::new();
        };
        // flat faces in seven bands, as the current engine shades its models
        let toon = Material::low_poly;
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
            let ink = |mesh, at| Instance::new(mesh, Material::Unlit, at).coloured(INK);
            out.instances.push(ink(m.cyl_ink, along(a, b, r + INK_W)));
            out.instances
                .push(ink(m.sphere_ink, maths::at_scaled(a, r * 1.05 + INK_W)));
            if i == last {
                out.instances
                    .push(ink(m.sphere_ink, maths::at_scaled(b, r * 1.55 + INK_W)));
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
        let tick = bf_core::dmath::sin(std::f64::consts::TAU * 1.1 * f.t_video) as f32 * 0.45;
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
            // inked, as the current engine's projectiles are
            out.instances
                .push(Instance::new(m.sphere_ink, Material::Unlit, maths::at_scaled(p, 0.2 + INK_W)).coloured(INK));
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

/// The dummy's grey as HDR light, and its orange accent, hot.
/// (E7: Aedan asked for the dummy's attack effects toned down by a lot: these are half what they were.)
const GREY_HOT: [f32; 3] = [1.0, 1.02, 1.1];
const ORANGE_HOT: [f32; 3] = [4.5, 1.8, 0.35];

fn record(t0: f64, life: f32, exit: Exit, env: Env, seed: u32, anchor: Anchor, kind: FxKind) -> FxRecord {
    FxRecord {
        t0,
        life,
        exit,
        env,
        seed,
        anchor,
        kind,
    }
}

impl LookFx for DummyLook {
    fn fx(
        &self,
        k: usize,
        rec: &Recording2,
        step_to_t: &dyn Fn(u32) -> f64,
        out: &mut FxTimeline,
    ) -> Result<(), FxError> {
        let dt = rec.dt as f32;
        let mv = |i: usize| &self.def.moves[i];
        for (i, e) in rec.events.iter().enumerate() {
            if e.who != k {
                continue;
            }
            let t0 = step_to_t(e.step);
            let seed = 0xD0_0000 ^ (e.step.wrapping_mul(97) + i as u32);
            match &e.kind {
                EvKind::Active { mv: m } if *m == HAYMAKER => {
                    let life = (mv(HAYMAKER).active + mv(HAYMAKER).recovery / 2) as f32 * dt + 0.15;
                    out.push(record(
                        t0 - 0.03,
                        life,
                        Exit::Fade,
                        Env::new(0.03, 0.05, 0.18),
                        seed,
                        Anchor::Bone { k, bone: FIST },
                        FxKind::Trail(TrailDef {
                            k,
                            bone: FIST,
                            width: (0.2, 0.04),
                            length_s: 0.1,
                            colour: GREY_HOT,
                            core: ORANGE_HOT,
                            style: TrailStyle::Smear,
                        }),
                    ))?;
                }
                EvKind::Windup { mv: m } if *m == DUMMY_BUSTER => {
                    let b = mv(DUMMY_BUSTER);
                    let life = (b.windup + b.active + b.recovery / 3) as f32 * dt;
                    out.push(record(
                        t0,
                        life,
                        Exit::Fade,
                        Env::new(0.1, 0.1, 0.2),
                        seed,
                        Anchor::Bone { k, bone: FIST },
                        FxKind::Trail(TrailDef {
                            k,
                            bone: FIST,
                            width: (0.15, 0.03),
                            length_s: 0.18,
                            colour: [2.0, 1.1, 0.4],
                            core: [2.5, 1.8, 1.0],
                            style: TrailStyle::Lightning,
                        }),
                    ))?;
                }
                EvKind::Contact {
                    mv: m, target, point, ..
                } if *m == DUMMY_BUSTER => {
                    // Only the final hit: no later BUSTER contact of this fighter within half a second.
                    let later = rec.events[i + 1..].iter().any(|f| {
                        f.who == k
                            && f.step <= e.step + 60
                            && matches!(f.kind, EvKind::Contact { mv: m2, .. } if m2 == DUMMY_BUSTER)
                    });
                    if later {
                        continue;
                    }
                    out.push(record(
                        t0,
                        0.6,
                        Exit::Burst,
                        Env::new(0.04, 0.15, 0.5),
                        seed,
                        Anchor::World(*point),
                        FxKind::Dome(DomeDef {
                            r: (0.3, 1.3),
                            colour: [1.0, 0.45, 0.1],
                            fresnel: 0.85,
                        }),
                    ))?;
                    out.push(record(
                        t0,
                        0.4,
                        Exit::Fade,
                        Env::new(0.02, 0.08, 0.25),
                        seed ^ 1,
                        Anchor::Bone { k, bone: FIST },
                        FxKind::Beam(BeamDef {
                            from: Anchor::Bone { k, bone: FIST },
                            to: Anchor::Body {
                                k: (*target).min(1),
                                offset: [0.0; 3],
                            },
                            width: 0.2,
                            colour: [2.0, 0.85, 0.2],
                            core: [3.0, 2.2, 1.2],
                            jitter: 0.12,
                        }),
                    ))?;
                }
                // TOSS is the dummy's only throw.
                EvKind::Spawn { proj } => {
                    let _ = TOSS;
                    let gone = rec.events[i..]
                        .iter()
                        .find(|f| matches!(f.kind, EvKind::Expire { proj: p } if p == *proj))
                        .map_or(t0 + 2.0, |f| step_to_t(f.step));
                    let life = ((gone - t0) as f32 + 0.25).max(0.3);
                    out.push(record(
                        t0,
                        life,
                        Exit::Fade,
                        Env::new(0.03, 0.0, 0.25),
                        seed,
                        Anchor::Body { k, offset: [0.0; 3] },
                        FxKind::Trail(TrailDef {
                            k,
                            bone: PROJ + *proj,
                            width: (0.2, 0.02),
                            length_s: 0.2,
                            colour: [2.5, 1.0, 0.2],
                            core: [4.5, 4.0, 3.0],
                            style: TrailStyle::Ribbon,
                        }),
                    ))?;
                }
                EvKind::Slam { target } => {
                    // GRAB SLAM is the dummy's only slam: smouldering marks where the target hits the canvas.
                    let _ = GRAB_SLAM;
                    let Some(st) = rec.steps.get(e.step as usize) else {
                        continue;
                    };
                    let p = st.pos[(*target).min(1)];
                    for (j, (kind, r, colour)) in [
                        (DecalKind::Scorch, 1.1, [0.02, 0.015, 0.012]),
                        (DecalKind::Glow, 0.6, [3.0, 1.0, 0.15]),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        out.push(record(
                            t0,
                            DECAL_LIFE - 1.5 * j as f32,
                            Exit::Ember,
                            Env::new(0.03, 0.0, 2.0),
                            seed ^ (2 + j as u32),
                            Anchor::World([p[0], p[1], 0.0]),
                            FxKind::Decal(DecalDef {
                                r,
                                kind,
                                colour,
                                on: Surface::Canvas,
                            }),
                        ))?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The shared presets turned well down for the dummy's events (Aedan, E7: "tone down their attack effects by
    /// a lot ... very subtle for now"): a fifth of the sparks, a third of the glow and flash, shorter rings and
    /// lights, smaller marks on the canvas, and no lens effects (speed lines, radial blur, chroma, shockwaves) on
    /// its hits. The K.O. stays readable: more sparks and longer than a hit, but no lens effects either, its dome and ring
    /// under half their size and its flash and bars turned down.
    fn tone(&self, e: &EvKind) -> Restraint {
        if matches!(e, EvKind::Ko { .. }) {
            Restraint {
                particles: 0.35,
                glow: 0.3,
                life: 0.8,
                size: 0.4,
                marks: 0.8,
                screen: 0.45,
                lens: false,
            }
        } else {
            Restraint {
                particles: 0.2,
                glow: 0.35,
                life: 0.6,
                size: 0.5,
                marks: 0.55,
                screen: 0.3,
                lens: false,
            }
        }
    }
}
