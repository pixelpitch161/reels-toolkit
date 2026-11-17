use super::*;
use crate::post::Post;
use crate::scene::MAX_LIGHTS;

const TW: u32 = 270;
const TH: u32 = 480;
const COLOURS: [[f32; 3]; 2] = [[0.9, 0.2, 0.1], [0.1, 0.4, 0.95]];

fn frame() -> FightFrame {
    FightFrame {
        t_video: 12.5,
        pos: [[-1.6, -1.6, 0.0], [1.6, 1.4, 0.8]],
        facing: [0.8, -2.3],
        hp: [80.0, 55.0],
        projs: vec![(7, [0.0, 0.0, 1.0])],
    }
}

#[test]
fn stage_draws_both_balls_where_the_sim_says() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    let f = frame();
    let mut full = Scene::default();
    stage.draw(&f, COLOURS, &mut full);
    let mut empty = Scene::default();
    stage.draw_arena(&f, COLOURS, &mut empty);
    let a = r.render_sized(&gpu, &full, &Post::default(), TW, TH);
    let b = r.render_sized(&gpu, &empty, &Post::default(), TW, TH);
    for k in 0..2 {
        let c = ball_centre(f.pos[k]);
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
    let mut f = frame();
    f.projs = (0..20).map(|i| (i, [i as f64 * 0.1, 0.0, 1.0])).collect();
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
