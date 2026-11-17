//! SPIKE: the TRUMPET's Engine 2 look, enough to judge the three candidate standard attacks in-engine.
//!
//! The model is the current engine's own (`data/models/TRUMPET.json`, the same faces the sprite renderer draws),
//! lifted to 3D and laid in the glass side-on as today: its long axis across the facing, its valves toward the foe.
//! `HORN`'s joint turns it (LEAD PIPE LANCE turns it mouthpiece-first) and rocks it (VALVE COMBO). The pistons are
//! today's: a brass collar, the nickel-silver rod, a pearl button in a brass rim. The lance is the lead pipe in three
//! telescoping brass sections with the silver mouthpiece on the end. The yo-yos are the valve caps on silver cords.
//! Effects: brass smears behind the pistons and the lance, ribbons behind the caps, a puff of valve steam after the
//! chord, and the shared hit presets turned down a notch below the dummy's.

use std::sync::Arc;

use bf2_fx::particles::{BurstDef, Shape};
use bf2_fx::tone::Restraint;
use bf2_fx::trail::{TrailDef, TrailStyle};
use bf2_fx::{Anchor, Env, Exit, FxError, FxKind, FxRecord, FxTimeline, frame::LookFx};
use bf2_render::maths::{self, M4, V3};
use bf2_render::stage::{BALL_R, ball_centre, linear};
use bf2_render::{FightFrame, Gpu, Instance, Look2, Material, MeshId, Renderer, Scene, geom, to_render};
use bf2_sim::{EvKind, FighterDef, Pose, Recording2, Segment, solve};

use super::{CAP1, CAP2, CAP3, HORN, LANCE, LEAD_PIPE_LANCE, P1, P2, P3, VALVE_CAP_YOYOS, VALVE_COMBO};

struct Meshes {
    model: MeshId,
    cyl: MeshId,
    sphere: MeshId,
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

/// A sphere squashed to `flat` along `axis` (unit), radius `r`, at `c`.
fn disc(c: V3, axis: V3, r: f32, flat: f32) -> M4 {
    let helper = if axis[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let x = maths::normalize(maths::cross(helper, axis));
    let z = maths::cross(x, axis);
    basis(maths::scale(x, r), maths::scale(axis, r * flat), maths::scale(z, r), c)
}

/// The sim's floor direction at angle `a`, in render space.
fn floor_dir(a: f64) -> V3 {
    [bf_core::dmath::cos(a) as f32, 0.0, -bf_core::dmath::sin(a) as f32]
}

const UP: V3 = [0.0, 1.0, 0.0];

fn srgb(c: [u8; 3]) -> [f32; 3] {
    linear(c.map(f64::from))
}

/// The model's brass (its main face colour), the fighter's yellow, nickel silver, pearl.
fn brass() -> [f32; 3] {
    srgb([228, 178, 72])
}
fn yellow() -> [f32; 3] {
    srgb(super::COLOUR)
}
fn silver() -> [f32; 3] {
    srgb([214, 218, 228])
}
fn pearl() -> [f32; 3] {
    srgb([252, 248, 238])
}

impl Look2 for TrumpetLook {
    fn setup(&mut self, r: &mut Renderer, gpu: &Gpu) {
        let mesh = crate::fighters::trumpet::MESH
            .and_then(|t| bf_picture::sprite::Mesh::from_json(t).ok())
            .expect("the trumpet's model is compiled in");
        self.meshes = Some(Meshes {
            model: r.meshes.add(gpu, geom::from_sprite_mesh(&mesh, 1.0)),
            cyl: r.meshes.add(gpu, geom::cylinder(1.0, 1.0, 16)),
            sphere: r.meshes.add(gpu, geom::icosphere(1.0, 2)),
        });
    }

    fn draw(&self, f: &FightFrame<'_>, k: usize, out: &mut Scene) -> Vec<u32> {
        let Some(m) = &self.meshes else {
            return Vec::new();
        };
        let toon = Material::toon;
        let pos = f.snap.pos[k];
        let facing = f.snap.facing[k];
        let joints = &f.snap.joints[k];
        let horn = joints.get(HORN).copied().unwrap_or(bf2_sim::Joint::REST);

        // The horn in the glass: model +y (valves) toward the facing, model +x (bell) across it, model +z up;
        // HORN's yaw turns it (minus: a quarter turn brings the mouthpiece, model -x, to the front), its pitch
        // rocks the valves up.
        let c = ball_centre(pos);
        let turn = facing - horn.yaw;
        let d = floor_dir(turn);
        let e1 = floor_dir(turn - std::f64::consts::FRAC_PI_2);
        let (sp, cp) = (
            bf_core::dmath::sinf(horn.pitch as f32),
            bf_core::dmath::cosf(horn.pitch as f32),
        );
        let dy = maths::add(maths::scale(d, cp), maths::scale(UP, sp));
        let up = maths::sub(maths::scale(UP, cp), maths::scale(d, sp));
        let s = BALL_R * 0.84;
        out.instances.push(Instance::new(
            m.model,
            toon(),
            basis(maths::scale(e1, s), maths::scale(dy, s), maths::scale(up, s), c),
        ));

        // The rig's segments.
        let mut segs: Vec<Segment> = Vec::new();
        let centre = [pos[0], pos[1], pos[2] + self.def.r];
        let pose = Pose { joints: joints.clone() };
        if pose.joints.len() == self.def.rig.bones.len() {
            solve(&self.def.rig, &pose, centre, self.def.r, facing, &mut segs);
        } else {
            return Vec::new();
        }
        let seg = |b: usize| (to_render(segs[b].a), to_render(segs[b].b));

        // The pistons.
        for b in [P1, P2, P3] {
            let (a, tip) = seg(b);
            let len = maths::length(maths::sub(tip, a));
            if len < 0.05 {
                continue;
            }
            let dir = maths::scale(maths::sub(tip, a), 1.0 / len);
            // starts a little inside the glass, at the valve
            let a0 = maths::sub(a, maths::scale(dir, 0.12));
            out.instances.push(
                Instance::new(m.cyl, toon(), along(a0, maths::add(a0, maths::scale(dir, 0.1)), 0.062))
                    .coloured(brass()),
            );
            out.instances
                .push(Instance::new(m.cyl, toon(), along(a0, tip, 0.032)).coloured(silver()));
            out.instances.push(
                Instance::new(
                    m.cyl,
                    toon(),
                    along(tip, maths::add(tip, maths::scale(dir, 0.045)), 0.09),
                )
                .coloured(yellow()),
            );
            out.instances.push(
                Instance::new(
                    m.sphere,
                    toon(),
                    disc(maths::add(tip, maths::scale(dir, 0.05)), dir, 0.07, 0.45),
                )
                .coloured(pearl()),
            );
        }

        // The lance: three telescoping sections and the mouthpiece.
        {
            let (a, tip) = seg(LANCE);
            let len = maths::length(maths::sub(tip, a));
            if len > 0.05 {
                let dir = maths::scale(maths::sub(tip, a), 1.0 / len);
                let a0 = maths::sub(a, maths::scale(dir, 0.15));
                let total = len + 0.15;
                let radii = [0.07f32, 0.056, 0.044];
                for (i, r) in radii.into_iter().enumerate() {
                    let from = (total * i as f32 / 3.0 - 0.04).max(0.0);
                    let to = total * (i as f32 + 1.0) / 3.0;
                    let p0 = maths::add(a0, maths::scale(dir, from));
                    let p1 = maths::add(a0, maths::scale(dir, to));
                    out.instances
                        .push(Instance::new(m.cyl, toon(), along(p0, p1, r)).coloured(brass()));
                    // a ferrule at the section's mouth
                    out.instances.push(
                        Instance::new(
                            m.cyl,
                            toon(),
                            along(maths::sub(p1, maths::scale(dir, 0.03)), p1, r + 0.014),
                        )
                        .coloured(yellow()),
                    );
                }
                // the mouthpiece: a silver shank and cup
                out.instances.push(
                    Instance::new(
                        m.cyl,
                        toon(),
                        along(tip, maths::add(tip, maths::scale(dir, 0.1)), 0.035),
                    )
                    .coloured(silver()),
                );
                out.instances.push(
                    Instance::new(
                        m.sphere,
                        toon(),
                        disc(maths::add(tip, maths::scale(dir, 0.13)), dir, 0.075, 0.6),
                    )
                    .coloured(silver()),
                );
            }
        }

        // The valve caps on their cords (from the glass out to the cap), when they are out.
        for b in [CAP1, CAP2, CAP3] {
            let (a, tip) = seg(b);
            let v = maths::sub(tip, a);
            let len = maths::length(v);
            if len < 0.62 {
                continue;
            }
            let dir = maths::scale(v, 1.0 / len);
            let from = maths::add(a, maths::scale(dir, BALL_R * 0.85));
            out.instances
                .push(Instance::new(m.cyl, toon(), along(from, tip, 0.012)).coloured(silver()));
            out.instances.push(
                Instance::new(
                    m.cyl,
                    toon(),
                    along(
                        maths::sub(tip, maths::scale(UP, 0.035)),
                        maths::add(tip, maths::scale(UP, 0.035)),
                        0.1,
                    ),
                )
                .coloured(brass()),
            );
            out.instances.push(
                Instance::new(
                    m.sphere,
                    toon(),
                    disc(maths::add(tip, maths::scale(UP, 0.04)), UP, 0.065, 0.5),
                )
                .coloured(pearl()),
            );
        }
        Vec::new()
    }
}

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

/// The yellow as HDR light (the bloom brings it down) and a hot core.
const YELLOW_HOT: [f32; 3] = [2.4, 1.7, 0.35];
const CORE_HOT: [f32; 3] = [3.2, 3.0, 2.2];

impl LookFx for TrumpetLook {
    fn fx(
        &self,
        k: usize,
        rec: &Recording2,
        step_to_t: &dyn Fn(u32) -> f64,
        out: &mut FxTimeline,
    ) -> Result<(), FxError> {
        let dt = rec.dt as f32;
        for (i, e) in rec.events.iter().enumerate() {
            if e.who != k {
                continue;
            }
            let t0 = step_to_t(e.step);
            let seed = 0x7B_0000 ^ (e.step.wrapping_mul(97) + i as u32);
            let trail = |bone, life: f32, width, length_s, style, j: u32| {
                record(
                    t0,
                    life,
                    Exit::Fade,
                    Env::new(0.02, 0.0, 0.12),
                    seed ^ j,
                    Anchor::Bone { k, bone },
                    FxKind::Trail(TrailDef {
                        k,
                        bone,
                        width,
                        length_s,
                        colour: YELLOW_HOT,
                        core: CORE_HOT,
                        style,
                    }),
                )
            };
            match &e.kind {
                EvKind::Active { mv } if *mv == VALVE_COMBO => {
                    let d = &self.def.moves[VALVE_COMBO];
                    let life = (d.active + 4) as f32 * dt;
                    for (j, b) in [P1, P2, P3].into_iter().enumerate() {
                        out.push(trail(b, life, (0.12, 0.02), 0.05, TrailStyle::Smear, j as u32))?;
                    }
                    // valve steam after the chord: a soft puff from the casings, drifting up
                    let Some(st) = rec.steps.get(e.step as usize) else {
                        continue;
                    };
                    let f = st.facing[k];
                    let off = [bf_core::dmath::cos(f) * 0.45, bf_core::dmath::sin(f) * 0.45, 0.15];
                    out.push(record(
                        t0 + f64::from(16.0 * dt),
                        0.9,
                        Exit::Dissolve,
                        Env::new(0.06, 0.1, 0.5),
                        seed ^ 9,
                        Anchor::Body { k, offset: off },
                        FxKind::Burst(BurstDef {
                            count: 40,
                            speed: (0.4, 1.2),
                            spread: 0.6,
                            dir: [0.0, 1.0, 0.0],
                            gravity: -0.6,
                            drag: 1.5,
                            size: (0.08, 0.28),
                            colour: [0.55, 0.55, 0.6],
                            colour_end: [0.3, 0.3, 0.34],
                            stretch: 0.0,
                            bounce: 0.0,
                            shape: Shape::Smoke,
                            emit_over: 0.25,
                        }),
                    ))?;
                }
                EvKind::Active { mv } if *mv == LEAD_PIPE_LANCE => {
                    let d = &self.def.moves[LEAD_PIPE_LANCE];
                    let life = (d.active + 6) as f32 * dt;
                    out.push(trail(LANCE, life, (0.16, 0.02), 0.07, TrailStyle::Smear, 1))?;
                }
                EvKind::Windup { mv } if *mv == VALVE_CAP_YOYOS => {
                    let d = &self.def.moves[VALVE_CAP_YOYOS];
                    let life = (d.windup + d.active + d.recovery / 2) as f32 * dt;
                    for (j, b) in [CAP1, CAP2, CAP3].into_iter().enumerate() {
                        out.push(trail(b, life, (0.1, 0.01), 0.12, TrailStyle::Ribbon, j as u32))?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A notch lighter than the dummy's (Aedan: fighters' hits land lighter than the dummy's presets).
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
                particles: 0.14,
                glow: 0.28,
                life: 0.5,
                size: 0.42,
                marks: 0.45,
                screen: 0.22,
                lens: false,
            }
        }
    }
}
