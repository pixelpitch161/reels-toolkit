use bf2_render::{Camera, Light, Post, Renderer, Scene};

use super::*;
use crate::lights::{LightDef, lights_for};
use crate::record::Env;
use crate::testkit::{FnPass, canvas_scene, dump, lum};

const TW: u32 = 270;
const TH: u32 = 480;

fn rec(kind: FxKind, life: f32) -> FxRecord {
    FxRecord {
        t0: 10.0,
        life,
        exit: Exit::Fade,
        env: Env::new(0.03, 0.0, 0.5),
        seed: 21,
        anchor: Anchor::World([0.0; 3]),
        kind,
    }
}

fn render(
    r: &mut Renderer,
    gpu: &Gpu,
    fx: &mut MeshFx,
    scene: &Scene,
    recs: &[&FxRecord],
    t: f64,
) -> bf_picture::img::Rgb8 {
    let list: Vec<(&FxRecord, [f64; 3])> = recs.iter().map(|&r| (r, [0.0; 3])).collect();
    let resolve = |a: &Anchor| match a {
        Anchor::World(p) => *p,
        _ => [0.0; 3],
    };
    let mut pass = FnPass(|ctx: &mut FxCtx<'_>| fx.draw(ctx, &list, &resolve, t));
    r.render_sized_with(gpu, scene, &Post::default(), TW, TH, Some(&mut pass))
}

/// The farthest lit pixel from `c`.
fn reach(img: &bf_picture::img::Rgb8, c: [f32; 2]) -> f32 {
    let mut best = 0.0f32;
    for y in 0..img.h {
        for x in 0..img.w {
            if lum(img.px(x, y)) > 40.0 {
                best = best.max(((x as f32 - c[0]).powi(2) + (y as f32 - c[1]).powi(2)).sqrt());
            }
        }
    }
    best
}

#[test]
fn ring_expands_over_life() {
    let d = RingDef::default();
    let mut last = 0.0;
    for k in 0..=20 {
        let r = grow(d.r, 1.0, k as f32 / 20.0, Exit::Fade, 1.0);
        assert!(r >= last);
        last = r;
    }
    assert!((last - d.r.1).abs() < 1e-5);
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut fx = MeshFx::new(&gpu);
    let ring = rec(FxKind::Ring(d), 1.0);
    let c = Camera::default_ring().project([0.0; 3], TW, TH).unwrap();
    let early = render(&mut r, &gpu, &mut fx, &Scene::default(), &[&ring], 10.1);
    let late = render(&mut r, &gpu, &mut fx, &Scene::default(), &[&ring], 10.5);
    dump(&late, "ring");
    let (a, b) = (reach(&early, c), reach(&late, c));
    assert!(a > 5.0 && b > a + 10.0, "ring reach {a} px early, {b} px later");
}

#[test]
fn effect_light_brightens_canvas() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut scene = canvas_scene(&mut r, &gpu);
    let at = [0.0, 0.0, 1.0];
    let light = FxRecord {
        anchor: Anchor::World(at),
        ..rec(
            FxKind::Light(LightDef {
                colour: [6.0, 3.0, 1.0],
                range: 4.0,
                flicker: 0.3,
            }),
            1.0,
        )
    };
    let c = scene.camera.project([0.0; 3], TW, TH).unwrap();
    let dark = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    lights_for(10.2, &[(&light, at)], &mut scene.lights);
    assert_eq!(scene.lights.len(), 1);
    let lit = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let (x, y) = (c[0] as usize, c[1] as usize);
    assert!(
        lum(lit.px(x, y)) > lum(dark.px(x, y)) + 40.0,
        "{:?} vs {:?}",
        lit.px(x, y),
        dark.px(x, y)
    );
    // Gone when the record is.
    let mut none = Vec::new();
    lights_for(11.0, &[(&light, at)], &mut none);
    lights_for(9.9, &[(&light, at)], &mut none);
    assert!(none.is_empty());
}

#[test]
fn beam_connects_its_anchors() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut fx = MeshFx::new(&gpu);
    let (a, b) = ([-2.0, 0.0, 1.0], [2.0, 1.0, 1.0]);
    let beam = rec(
        FxKind::Beam(BeamDef {
            from: Anchor::World(a),
            to: Anchor::World(b),
            ..BeamDef::default()
        }),
        1.0,
    );
    let img = render(&mut r, &gpu, &mut fx, &Scene::default(), &[&beam], 10.3);
    dump(&img, "beam");
    let cam = Camera::default_ring();
    let p = |q: [f64; 3]| cam.project(to_render(q), TW, TH).unwrap();
    let lit_near = |s: [f32; 2]| {
        let mut best = 0.0f32;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let (x, y) = ((s[0] as i32 + dx) as usize, (s[1] as i32 + dy) as usize);
                best = best.max(lum(img.px(x, y)));
            }
        }
        best
    };
    let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, 1.0];
    for (name, q) in [
        ("near the start", [-1.8, 0.05, 1.0]),
        ("the middle", mid),
        ("near the end", [1.8, 0.95, 1.0]),
    ] {
        assert!(lit_near(p(q)) > 100.0, "{name} is dark");
    }
    // Off the line, nothing.
    assert!(lit_near(p([0.0, -2.0, 1.0])) < 5.0);
}

#[test]
fn effect_lights_outrank_stage_lights_at_the_cap() {
    let mut scene = Scene::default();
    for i in 0..70 {
        scene.lights.push(Light {
            pos: [i as f32, 0.0, 0.0],
            colour: [5.0; 3],
            range: 3.0,
            priority: 0,
        });
    }
    let light = rec(
        FxKind::Light(LightDef {
            colour: [0.2, 0.1, 0.0],
            range: 2.0,
            flicker: 0.0,
        }),
        1.0,
    );
    lights_for(10.5, &[(&light, [0.0, 0.0, 1.0])], &mut scene.lights);
    let kept = scene.lights_for_gpu();
    assert_eq!(kept.len(), 64);
    assert!(
        kept.iter().any(|l| l.priority == crate::lights::EFFECT_PRIORITY),
        "the dim effect light was dropped"
    );
}
