use bf2_render::{Post, Renderer, Scene};

use super::*;
use crate::record::{Anchor, Env, Exit};
use crate::testkit::{FnPass, canvas_scene, dump, lum};

const TW: u32 = 270;
const TH: u32 = 480;

fn rec(def: DecalDef, exit: Exit) -> FxRecord {
    FxRecord {
        t0: 10.0,
        life: DECAL_LIFE,
        exit,
        env: Env::new(0.05, 0.0, 2.0),
        seed: 1234,
        anchor: Anchor::World([0.0; 3]),
        kind: FxKind::Decal(def),
    }
}

fn lit_canvas(r: &mut Renderer, gpu: &Gpu) -> Scene {
    let mut s = canvas_scene(r, gpu);
    s.ambient = [1.2; 3];
    s
}

fn render(
    r: &mut Renderer,
    gpu: &Gpu,
    d: &mut Decals,
    scene: &Scene,
    recs: &[&FxRecord],
    t: f64,
) -> bf_picture::img::Rgb8 {
    let list: Vec<(&FxRecord, [f64; 3])> = recs.iter().map(|&r| (r, [0.0; 3])).collect();
    let mut pass = FnPass(|ctx: &mut FxCtx<'_>| d.draw(ctx, &list, t));
    r.render_sized_with(gpu, scene, &Post::default(), TW, TH, Some(&mut pass))
}

/// Mean luminance in a box of half-size `h` around `c`.
fn mean(img: &bf_picture::img::Rgb8, c: [f32; 2], h: i32) -> f32 {
    let mut s = 0.0;
    let mut n = 0.0;
    for dy in -h..=h {
        for dx in -h..=h {
            s += lum(img.px((c[0] as i32 + dx) as usize, (c[1] as i32 + dy) as usize));
            n += 1.0;
        }
    }
    s / n
}

#[test]
fn scorch_darkens_canvas_then_fades() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut d = Decals::new(&gpu);
    let scene = lit_canvas(&mut r, &gpu);
    let c = scene.camera.project([0.0; 3], TW, TH).unwrap();
    let scorch = rec(DecalDef::default(), Exit::Fade);
    let blank = render(&mut r, &gpu, &mut d, &scene, &[], 12.0);
    let mid = render(&mut r, &gpu, &mut d, &scene, &[&scorch], 13.0);
    dump(&mid, "scorch");
    let end = render(
        &mut r,
        &gpu,
        &mut d,
        &scene,
        &[&scorch],
        10.0 + f64::from(DECAL_LIFE) - 1.0 / 30.0,
    );
    let (b, m, e) = (mean(&blank, c, 4), mean(&mid, c, 4), mean(&end, c, 4));
    assert!(m < 0.6 * b, "the scorch does not darken: {m} against {b}");
    assert!(
        (e - b).abs() < 3.0,
        "the scorch has not faded at its end: {e} against {b}"
    );
    // Ember cools through red, and also ends faded.
    let ember = rec(DecalDef::default(), Exit::Ember);
    let end = render(
        &mut r,
        &gpu,
        &mut d,
        &scene,
        &[&ember],
        10.0 + f64::from(DECAL_LIFE) - 1.0 / 30.0,
    );
    assert!((mean(&end, c, 4) - b).abs() < 3.0);
}

#[test]
fn crack_has_lines() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut d = Decals::new(&gpu);
    let scene = lit_canvas(&mut r, &gpu);
    let c = scene.camera.project([0.0; 3], TW, TH).unwrap();
    let crack = rec(
        DecalDef {
            kind: DecalKind::Crack,
            r: 1.4,
            colour: [0.01; 3],
            ..DecalDef::default()
        },
        Exit::Fade,
    );
    let blank = render(&mut r, &gpu, &mut d, &scene, &[], 13.0);
    let img = render(&mut r, &gpu, &mut d, &scene, &[&crack], 13.0);
    dump(&img, "crack");
    // High-frequency energy: the summed absolute Laplacian of luminance in the decal's box.
    let hf = |im: &bf_picture::img::Rgb8| {
        let mut s = 0.0;
        for y in (c[1] as usize - 20)..(c[1] as usize + 20) {
            for x in (c[0] as usize - 30)..(c[0] as usize + 30) {
                let l = |dx: i32, dy: i32| lum(im.px((x as i32 + dx) as usize, (y as i32 + dy) as usize));
                s += (4.0 * l(0, 0) - l(1, 0) - l(-1, 0) - l(0, 1) - l(0, -1)).abs();
            }
        }
        s
    };
    let (a, b) = (hf(&img), hf(&blank));
    assert!(a > 5.0 * b + 1000.0, "crack energy {a} against blank {b}");
}

#[test]
fn rope_decal_marks_the_ropes() {
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut d = Decals::new(&gpu);
    let glow = FxRecord {
        anchor: Anchor::World([0.0, 3.25, 0.8]),
        ..rec(
            DecalDef {
                kind: DecalKind::Glow,
                colour: [6.0, 2.0, 0.5],
                on: Surface::Rope(0),
                r: 0.8,
            },
            Exit::Fade,
        )
    };
    let list = [(&glow, [0.0, 3.25, 0.8])];
    let mut pass = FnPass(|ctx: &mut FxCtx<'_>| d.draw(ctx, &list, 11.0));
    let img = r.render_sized_with(&gpu, &Scene::default(), &Post::default(), TW, TH, Some(&mut pass));
    // The far side's middle rope, at its middle.
    let cam = bf2_render::Camera::default_ring();
    let p = cam.project([0.0, ROPE_HEIGHTS[1] - ROPE_SAG, -POST], TW, TH).unwrap();
    assert!(mean(&img, p, 2) > 30.0, "no glow on the rope");
}
