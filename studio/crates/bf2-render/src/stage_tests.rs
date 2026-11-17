use bf2_sim::{Outcome, Recording2, StepSnap};

use super::*;
use crate::fight_frame::frame_at_time;
use crate::post::Post;
use crate::scene::MAX_LIGHTS;

const TW: u32 = 270;
const TH: u32 = 480;
const COLOURS: [[f32; 3]; 2] = [[0.9, 0.2, 0.1], [0.1, 0.4, 0.95]];

/// The map with no slow-mo and no K.O.
fn plain(rec: &Recording2) -> bf2_direct::TimeMap {
    let cfg = bf2_direct::DirectCfg::default();
    bf2_direct::TimeMap::build(&[], rec.steps.len() as u32, rec.dt, rec.t_fight, None, &cfg)
}

/// A one-step recording: the balls where the test wants them.
fn recording(projs: Vec<(u32, [f64; 3])>) -> Recording2 {
    let snap = StepSnap {
        pos: [[-1.6, -1.6, 0.0], [1.6, 1.4, 0.8]],
        vel: [[0.0; 3]; 2],
        facing: [0.8, -2.3],
        hp: [80.0, 55.0],
        mv: [None, None],
        joints: [vec![], vec![]],
        hitstop: [0, 0],
        projs,
    };
    Recording2 {
        names: ["A".into(), "B".into()],
        seed: 1,
        dt: 1.0 / 120.0,
        t_fight: 4.5,
        steps: vec![snap],
        events: vec![],
        outcome: Outcome {
            winner: None,
            t_ko: None,
            hp_end: [80.0, 55.0],
            hits: [0, 0],
            damage: [0.0, 0.0],
            by_decision: true,
        },
    }
}

#[test]
fn stage_draws_both_balls_where_the_sim_says() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    let rec = recording(vec![(7, [0.0, 0.0, 1.0])]);
    let f = frame_at_time(&rec, &plain(&rec), 12.5);
    let mut full = Scene::default();
    stage.draw(&f, COLOURS, &mut full);
    let mut empty = Scene::default();
    stage.draw_arena(&f, COLOURS, &mut empty);
    let a = r.render_sized(&gpu, &full, &Post::default(), TW, TH);
    let b = r.render_sized(&gpu, &empty, &Post::default(), TW, TH);
    for k in 0..2 {
        let c = ball_centre(f.snap.pos[k]);
        let p = full.camera.project(c, TW, TH).expect("in front of the camera");
        let (x, y) = (p[0].round() as usize, p[1].round() as usize);
        assert!(x < a.w && y < a.h, "ball {k} at {p:?} is off the picture");
        assert_ne!(a.px(x, y), b.px(x, y), "ball {k} at ({x}, {y}) does not show");
    }
}

#[test]
fn stage_scene_is_bounded() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    let rec = recording((0..20).map(|i| (i, [i as f64 * 0.1, 0.0, 1.0])).collect());
    let f = frame_at_time(&rec, &plain(&rec), 12.5);
    let mut s = Scene::default();
    stage.draw(&f, COLOURS, &mut s);
    stage.draw_projectiles(&f, &[3, 4], &mut s);
    assert!(s.instances.len() < 2000, "{} instances", s.instances.len());
    assert!(s.lights.len() <= MAX_LIGHTS, "{} lights", s.lights.len());
    let spot = s.spot.expect("the spot");
    assert!(spot.shadow);
    assert_eq!(s.ambient, [AMBIENT; 3]);
    // 18 default projectiles, not the two the look drew.
    let glowing = s.instances.iter().filter(|i| i.mesh == stage.proj_mesh()).count();
    assert_eq!(glowing, 18);
}

/// Where each instance of `mesh` starts (its model's translation).
fn starts(s: &Scene, mesh: crate::mesh::MeshId) -> Vec<V3> {
    s.instances
        .iter()
        .filter(|i| i.mesh == mesh)
        .map(|i| [i.model[3][0], i.model[3][1], i.model[3][2]])
        .collect()
}

#[test]
fn stage_ropes_bend_with_wobble() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    let rec = recording(vec![]);
    let quiet = frame_at_time(&rec, &plain(&rec), 12.5);
    let mut still = Scene::default();
    stage.draw(&quiet, COLOURS, &mut still);
    assert!(
        starts(&still, stage.rope_link).is_empty(),
        "a quiet frame draws whole ropes"
    );
    assert_eq!(starts(&still, stage.ropes[0]).len(), 1);

    let mut f = quiet.clone();
    f.arena.rope_wobble = [0.2, 0.0, -0.1, 0.0];
    let mut bent = Scene::default();
    stage.draw(&f, COLOURS, &mut bent);
    // The far side (sim +y) bows out, the near side in; the others hang as they were.
    assert!(starts(&bent, stage.ropes[0]).is_empty() && starts(&bent, stage.ropes[2]).is_empty());
    assert_eq!(starts(&bent, stage.ropes[1]).len(), 1);
    assert_eq!(starts(&bent, stage.ropes[3]).len(), 1);
    let links = starts(&bent, stage.rope_link);
    assert_eq!(links.len(), 2 * 3 * 8, "eight links a rope, three ropes a side");
    let out = |side: usize| {
        let (dir, _) = SIDES[side];
        links
            .iter()
            .map(|p| maths::dot(*p, dir))
            .filter(|d| (d - POST).abs() < 0.5)
            .fold(f32::NAN, |a, d| {
                if (d - POST).abs() > (a - POST).abs() || a.is_nan() {
                    d
                } else {
                    a
                }
            })
    };
    assert!(
        (out(0) - POST - 0.2).abs() < 0.02,
        "the far side bows out to {}",
        out(0)
    );
    assert!(
        (out(2) - POST + 0.1).abs() < 0.02,
        "the near side bows in to {}",
        out(2)
    );
    // The posts hold the ends: a rope's first link starts at its post.
    assert!(links.iter().any(|p| (maths::dot(*p, SIDES[0].0) - POST).abs() < 1e-4));
}

#[test]
fn stage_crowd_leds_and_house_follow_the_arena() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    let rec = recording(vec![]);
    let quiet = frame_at_time(&rec, &plain(&rec), 12.5);
    let mut still = Scene::default();
    stage.draw(&quiet, COLOURS, &mut still);
    let mut f = quiet.clone();
    f.arena.crowd_jump = 1.0;
    f.arena.led_flash = Some((1, 1.0));
    f.arena.house = 0.35;
    f.arena.beat = 1.0;
    let mut live = Scene::default();
    stage.draw(&f, COLOURS, &mut live);
    let (a, b) = (starts(&still, stage.fan), starts(&live, stage.fan));
    assert_eq!(a.len(), b.len());
    assert!(
        a.iter().zip(&b).all(|(p, q)| (q[1] - p[1] - 0.25).abs() < 1e-5),
        "the crowd is up"
    );
    assert!((live.ambient[0] - AMBIENT * 0.35).abs() < 1e-7);
    // The leader is fighter 0 (more HP), but the flash is fighter 1's.
    let leds = |s: &Scene| -> Vec<([f32; 4], f32)> {
        s.instances
            .iter()
            .filter_map(|i| match i.material {
                Material::Emissive { strength } if i.mesh == stage.unit_box => Some((i.colour, strength)),
                _ => None,
            })
            .collect()
    };
    let (l0, l1) = (leds(&still), leds(&live));
    assert_eq!(l0.len(), l1.len());
    assert!(l0.iter().all(|(c, _)| c[..3] == COLOURS[0]));
    assert!(l1.iter().all(|(c, _)| c[..3] == COLOURS[1]));
    assert!(
        l0.iter().zip(&l1).all(|(a, b)| b.1 > a.1 + 1.0),
        "the flash and the beat brighten the boards"
    );
}
