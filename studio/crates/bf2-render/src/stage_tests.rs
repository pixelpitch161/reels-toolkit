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
