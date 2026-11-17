use bf_picture::img::Rgb8;
use bf2_render::{Instance, Material, NoFx, Post, Renderer, Scene, geom, maths};

use super::*;
use crate::record::{Env, Exit};
use crate::testkit::{arm, canvas_scene, dump, lum, step_to_t, swing};
use crate::trail::TrailSource;

const TW: u32 = 270;
const TH: u32 = 480;

fn rec(kind: FxKind, t0: f64, life: f32) -> FxRecord {
    FxRecord {
        t0,
        life,
        exit: Exit::Fade,
        env: Env::new(0.03, 0.0, 0.3),
        seed: 7,
        anchor: Anchor::World([0.0; 3]),
        kind,
    }
}

fn screen_rec(effect: ScreenFx, amount: f32, t0: f64, life: f32) -> FxRecord {
    rec(FxKind::Screen(ScreenDef { effect, amount }), t0, life)
}

/// No vignette, grain or lens dirt: only what a test switches on.
fn plain(gpu: &Gpu) -> Screen {
    let mut s = Screen::new(gpu);
    s.look.vignette = 0.0;
    s.look.grain = 0.0;
    s.look.dirt = 0.0;
    s
}

fn resolve(a: &Anchor) -> [f64; 3] {
    match a {
        Anchor::World(p) => *p,
        _ => [0.0; 3],
    }
}

fn render(r: &mut Renderer, gpu: &Gpu, s: &mut Screen, scene: &Scene, recs: &[&FxRecord], t: f64) -> Rgb8 {
    let list: Vec<(&FxRecord, [f64; 3])> = recs.iter().map(|&r| (r, resolve(&r.anchor))).collect();
    s.set(&list, &resolve, t);
    r.render_accum_sized(
        gpu,
        std::slice::from_ref(scene),
        &[1.0],
        &Post::default(),
        &mut NoFx,
        Some(s),
        TW,
        TH,
    )
}

fn no_chain(r: &mut Renderer, gpu: &Gpu, scene: &Scene) -> Rgb8 {
    r.render_accum_sized(
        gpu,
        std::slice::from_ref(scene),
        &[1.0],
        &Post::default(),
        &mut NoFx,
        None,
        TW,
        TH,
    )
}

/// An emissive ball of radius `rad` at sim `p`.
fn glow(r: &mut Renderer, gpu: &Gpu, scene: &mut Scene, p: [f64; 3], rad: f32, strength: f32) {
    let m = r.meshes.add(gpu, geom::icosphere(rad, 3));
    scene.instances.push(
        Instance::new(m, Material::Emissive { strength }, maths::translation(to_render(p))).coloured([1.0, 0.9, 0.8]),
    );
}

fn at_px(scene: &Scene, p: [f64; 3]) -> [f32; 2] {
    scene.camera.project(to_render(p), TW, TH).unwrap()
}

/// Mean luminance of the pixels whose distance from `c` is in `r0..r1`.
fn ring_mean(img: &Rgb8, c: [f32; 2], r0: f32, r1: f32) -> f32 {
    let (mut sum, mut n) = (0.0, 0);
    for y in 0..img.h {
        for x in 0..img.w {
            let d = ((x as f32 - c[0]).powi(2) + (y as f32 - c[1]).powi(2)).sqrt();
            if d >= r0 && d < r1 {
                sum += lum(img.px(x, y));
                n += 1;
            }
        }
    }
    sum / n.max(1) as f32
}

fn mean_diff_where(a: &Rgb8, b: &Rgb8, keep: impl Fn(f32, f32) -> bool) -> f32 {
    let (mut sum, mut n) = (0.0, 0);
    for y in 0..a.h {
        for x in 0..a.w {
            if keep(x as f32, y as f32) {
                sum += (lum(a.px(x, y)) - lum(b.px(x, y))).abs();
                n += 1;
            }
        }
    }
    sum / n.max(1) as f32
}

#[test]
fn bloom_spreads_bright_pixels() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut s = plain(&gpu);
    let mut scene = Scene::default();
    glow(&mut r, &gpu, &mut scene, [0.0; 3], 0.3, 40.0);
    let c = at_px(&scene, [0.0; 3]);
    let bare = no_chain(&mut r, &gpu, &scene);
    let img = render(&mut r, &gpu, &mut s, &scene, &[], 10.0);
    dump(&img, "bloom");
    let (near_bare, near) = (ring_mean(&bare, c, 14.0, 30.0), ring_mean(&img, c, 14.0, 30.0));
    let far = ring_mean(&img, c, 30.0, 60.0);
    assert!(near_bare < 1.0, "the ball alone lights nothing around it: {near_bare}");
    assert!(near > 12.0, "bloom spreads around the ball: {near}");
    assert!(far > 2.0 && far < near, "and falls off with distance: {far} vs {near}");
    // A dim scene does not bloom.
    let mut dim = Scene::default();
    glow(&mut r, &gpu, &mut dim, [0.0; 3], 0.3, 0.6);
    let img = render(&mut r, &gpu, &mut s, &dim, &[], 10.0);
    assert!(
        ring_mean(&img, c, 14.0, 30.0) < 1.0,
        "below the threshold nothing blooms"
    );
}

#[test]
fn radial_blur_streaks_toward_centre() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut s = plain(&gpu);
    s.look.bloom = 0.0;
    let mut scene = Scene::default();
    let p = [3.0, 0.0, 0.0];
    glow(&mut r, &gpu, &mut scene, p, 0.35, 3.0);
    let c = at_px(&scene, p);
    let centre = [TW as f32 / 2.0, TH as f32 / 2.0];
    let blur = screen_rec(ScreenFx::RadialBlur { centre: Anchor::Screen }, 1.0, 10.0, 1.0);
    let bare = render(&mut r, &gpu, &mut s, &scene, &[], 10.3);
    let img = render(&mut r, &gpu, &mut s, &scene, &[&blur], 10.3);
    dump(&img, "radial");
    // Along the line through the centre the ball smears; across it, it doesn't.
    let (dx, dy) = (c[0] - centre[0], c[1] - centre[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = (dx / len, dy / len);
    let probe = |img: &Rgb8, along: f32, across: f32| {
        let x = c[0] + ux * along - uy * across;
        let y = c[1] + uy * along + ux * across;
        lum(img.px(x.round() as usize, y.round() as usize))
    };
    let reach = 18.0;
    assert!(probe(&bare, reach, 0.0) < 5.0 && probe(&bare, 0.0, reach) < 5.0);
    let (along, across) = (probe(&img, reach, 0.0), probe(&img, 0.0, reach));
    assert!(along > 30.0, "the streak runs along the radius: {along}");
    assert!(across < 5.0, "and not across it: {across}");
}

#[test]
fn shockwave_displaces_a_grid_pattern() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut s = plain(&gpu);
    s.look.bloom = 0.0;
    let mut scene = canvas_scene(&mut r, &gpu);
    for i in -4..=4 {
        for j in -6..=6 {
            glow(
                &mut r,
                &gpu,
                &mut scene,
                [i as f64 * 0.7, j as f64 * 0.7, 0.0],
                0.12,
                2.0,
            );
        }
    }
    let c = at_px(&scene, [0.0; 3]);
    let wave = screen_rec(
        ScreenFx::Shockwave {
            centre: Anchor::World([0.0; 3]),
            speed: 0.5,
        },
        1.0,
        10.0,
        1.0,
    );
    let t = 10.4;
    let radius = 0.5 * 0.4 * TH as f32;
    let bare = render(&mut r, &gpu, &mut s, &scene, &[], t);
    let img = render(&mut r, &gpu, &mut s, &scene, &[&wave], t);
    dump(&img, "shock");
    let dist = |x: f32, y: f32| ((x - c[0]).powi(2) + (y - c[1]).powi(2)).sqrt();
    let on_front = mean_diff_where(&bare, &img, |x, y| (dist(x, y) - radius).abs() < 20.0);
    let outside = mean_diff_where(&bare, &img, |x, y| dist(x, y) > radius + 90.0);
    let inside = mean_diff_where(&bare, &img, |x, y| dist(x, y) < radius - 90.0);
    assert!(on_front > 3.0, "the front moves the grid: {on_front}");
    assert!(
        outside < 0.3 && inside < 0.3,
        "far from it nothing moves: {inside} / {outside}"
    );
}

#[test]
fn flash_reaches_full_then_fades() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut s = plain(&gpu);
    let scene = canvas_scene(&mut r, &gpu);
    let flash = screen_rec(ScreenFx::Flash { colour: [6.0; 3] }, 1.0, 10.0, 0.4);
    let bare = render(&mut r, &gpu, &mut s, &scene, &[], 10.0);
    let mean = |img: &Rgb8| img.data.chunks(3).map(lum).sum::<f32>() / (img.w * img.h) as f32;
    let peak = render(&mut r, &gpu, &mut s, &scene, &[&flash], 10.05);
    assert!(mean(&peak) > 245.0, "the flash fills the picture: {}", mean(&peak));
    let mut last = mean(&peak);
    for k in 1..=10 {
        let t = 10.05 + 0.035 * k as f64;
        let m = mean(&render(&mut r, &gpu, &mut s, &scene, &[&flash], t));
        assert!(m <= last + 0.5, "it only fades: {m} after {last}");
        last = m;
    }
    let end = render(&mut r, &gpu, &mut s, &scene, &[&flash], 10.0 + 0.4 - 1.0 / 240.0);
    assert!(
        mean_diff_where(&end, &bare, |_, _| true) < 2.0,
        "its last frame is back to the scene"
    );
}

#[test]
fn stacked_screen_effects_stay_finite() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut s = Screen::new(&gpu);
    let mut scene = canvas_scene(&mut r, &gpu);
    glow(&mut r, &gpu, &mut scene, [0.0, 0.0, 0.5], 0.4, 5.0e4);
    glow(&mut r, &gpu, &mut scene, [1.5, 1.0, 0.3], 0.2, 300.0);
    let centre = Anchor::World([0.0, 0.0, 0.5]);
    let all = [
        screen_rec(
            ScreenFx::Flash {
                colour: [40.0, 30.0, 20.0],
            },
            3.0,
            10.0,
            0.3,
        ),
        screen_rec(ScreenFx::Invert, 1.0, 10.0, 1.0),
        screen_rec(ScreenFx::RadialBlur { centre }, 2.0, 10.0, 1.0),
        screen_rec(ScreenFx::Shockwave { centre, speed: 1.5 }, 3.0, 10.0, 1.0),
        screen_rec(ScreenFx::Shockwave { centre, speed: 0.7 }, 3.0, 10.0, 1.0),
        screen_rec(ScreenFx::Chroma { centre }, 3.0, 10.0, 1.0),
        screen_rec(
            ScreenFx::SpeedLines {
                centre,
                colour: [30.0; 3],
            },
            2.0,
            10.0,
            1.0,
        ),
        screen_rec(ScreenFx::Letterbox, 1.0, 10.0, 1.0),
        screen_rec(
            ScreenFx::Grade {
                lift: [0.4, 0.0, -0.3],
                gamma: [0.5, 1.0, 2.0],
                gain: [3.0, 1.0, 0.5],
                saturation: 2.5,
            },
            1.0,
            10.0,
            1.0,
        ),
        screen_rec(ScreenFx::Desaturate { keep: [1.0, 0.0, 0.0] }, 1.0, 10.0, 1.0),
        screen_rec(ScreenFx::Bloom, 4.0, 10.0, 1.0),
    ];
    let recs: Vec<&FxRecord> = all.iter().collect();
    for f in 0..33 {
        let t = 10.0 + f as f64 / 30.0;
        let img = render(&mut r, &gpu, &mut s, &scene, &recs, t);
        let hdr = r.read_hdr(&gpu).unwrap();
        assert!(
            hdr.iter().all(|p| p.iter().all(|c| c.is_finite() && *c >= 0.0)),
            "frame {f} carries a NaN, an infinity or negative light"
        );
        let white = (0..img.w * img.h)
            .filter(|i| lum(&img.data[i * 3..i * 3 + 3]) >= 250.0)
            .count();
        if t >= 10.3 {
            assert!(
                (white as f32) < 0.95 * (img.w * img.h) as f32,
                "frame {f} is white outside the flash's life ({white} px)"
            );
        }
        if f == 12 {
            dump(&img, "stacked");
        }
    }
}

#[test]
fn post_chain_is_stateless() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut s = Screen::new(&gpu);
    let mut scene = canvas_scene(&mut r, &gpu);
    glow(&mut r, &gpu, &mut scene, [0.0, 0.0, 0.5], 0.4, 30.0);
    let centre = Anchor::World([0.0, 0.0, 0.5]);
    let a = [
        screen_rec(ScreenFx::Shockwave { centre, speed: 1.0 }, 1.0, 10.0, 1.0),
        screen_rec(ScreenFx::Chroma { centre }, 1.0, 10.0, 1.0),
    ];
    let b = [
        screen_rec(ScreenFx::Flash { colour: [5.0; 3] }, 1.0, 20.0, 0.5),
        screen_rec(ScreenFx::Letterbox, 1.0, 20.0, 1.0),
    ];
    let first = render(&mut r, &gpu, &mut s, &scene, &a.iter().collect::<Vec<_>>(), 10.2);
    render(&mut r, &gpu, &mut s, &scene, &b.iter().collect::<Vec<_>>(), 20.1);
    let again = render(&mut r, &gpu, &mut s, &scene, &a.iter().collect::<Vec<_>>(), 10.2);
    assert!(first.data == again.data, "frame t after frame t+10 s equals frame t");
}

#[test]
fn subframes_rise_with_speed() {
    let rig = arm(1.0);
    let tl = FxTimeline::default();
    let n_for = |rec: &bf2_sim::Recording2, step: u32, tl: &FxTimeline| {
        let src = TrailSource {
            rec,
            rigs: [&rig, &rig],
            r_ball: [0.6; 2],
            step_to_t: &step_to_t,
        };
        subframes(&src, step_to_t(step), tl)
    };
    let still = swing(10, 0.0, 40);
    assert_eq!(n_for(&still, 30, &tl), 1);
    let slow = swing(200, 0.3, 10);
    assert_eq!(n_for(&slow, 100, &tl), 1);
    let fast = swing(12, std::f64::consts::PI, 20);
    assert_eq!(n_for(&fast, 8, &tl), 8);
    let mut last = 0;
    for steps in [400, 120, 60, 30, 12] {
        let rec = swing(steps, std::f64::consts::PI, 20);
        let n = n_for(&rec, (steps as u32 / 2).max(5), &tl);
        assert!(n >= last, "faster swings never get fewer sub-frames: {n} after {last}");
        last = n;
    }
    // A cue asks for all of them.
    let mut cued = FxTimeline::default();
    cued.push(screen_rec(ScreenFx::MotionBlur, 1.0, step_to_t(25) - 0.1, 0.3))
        .unwrap();
    assert_eq!(n_for(&still, 25, &cued), 8);
    // The shutter: n times inside half a frame, centred on t.
    let ts = shutter_times(10.0, 8);
    assert_eq!(ts.len(), 8);
    let mean = ts.iter().sum::<f64>() / 8.0;
    assert!((mean - 10.0).abs() < 1e-9);
    assert!(ts.windows(2).all(|w| w[1] > w[0]));
    assert!(ts[7] - ts[0] < 1.0 / 60.0 && ts[7] - ts[0] > 0.8 / 60.0);
    assert_eq!(shutter_times(10.0, 1), vec![10.0]);
}

#[test]
fn accum_blurs_a_fast_ball() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let ball = r.meshes.add(&gpu, geom::icosphere(0.5, 3));
    let t = 10.0;
    let scene_at = |ts: f64| {
        // 1 unit per frame along sim x.
        let x = (ts - t) * 30.0;
        Scene {
            instances: vec![
                Instance::new(
                    ball,
                    Material::Emissive { strength: 2.0 },
                    maths::translation(to_render([x, 0.0, 0.5])),
                )
                .coloured([1.0; 3]),
            ],
            ..Scene::default()
        }
    };
    let extent = |img: &Rgb8| {
        let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0, usize::MAX, 0);
        for y in 0..img.h {
            for x in 0..img.w {
                if lum(img.px(x, y)) > 20.0 {
                    (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
                }
            }
        }
        ((x1 - x0) as f32, (y1 - y0) as f32)
    };
    let one = r.render_accum_sized(&gpu, &[scene_at(t)], &[1.0], &Post::default(), &mut NoFx, None, TW, TH);
    let times = shutter_times(t, 8);
    let scenes: Vec<Scene> = times.iter().map(|&ts| scene_at(ts)).collect();
    let blurred = r.render_accum_sized(&gpu, &scenes, &[1.0; 8], &Post::default(), &mut NoFx, None, TW, TH);
    dump(&blurred, "accum");
    let ((w1, h1), (w8, h8)) = (extent(&one), extent(&blurred));
    assert!(w8 > 1.3 * w1, "stretched along its motion: {w8} vs {w1}");
    assert!((h8 - h1).abs() <= 0.1 * h1 + 2.0, "and not across it: {h8} vs {h1}");
}
