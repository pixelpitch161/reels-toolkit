//! The UI matte and its composite: on the CPU, and the GPU pass against the CPU (lavapipe in the cloud).

use bf_picture::font;
use bf_picture::img::{Img, Rgb8};
use bf_picture::ring::hud::big_text;
use bf_picture::ui::{DrawLog, label};

use super::*;

/// A layer drawn by `draw` over black and over white, matted.
fn matted(w: usize, h: usize, draw: &dyn Fn(&mut Rgb8)) -> Overlay {
    let mut b: Rgb8 = Img::filled(w, h, &[0, 0, 0]);
    let mut wt: Rgb8 = Img::filled(w, h, &[255, 255, 255]);
    draw(&mut b);
    draw(&mut wt);
    matte(&b, &wt)
}

/// Ink at `a` over a block, as the UI's blended draws put it (`p·(1 − a) + c·a`, truncated).
fn ink(img: &mut Rgb8, x0: usize, y0: usize, n: usize, col: [u8; 3], a: f64) {
    for y in y0..y0 + n {
        for x in x0..x0 + n {
            for (p, &c) in img.px_mut(x, y).iter_mut().zip(&col) {
                *p = (f64::from(*p) * (1.0 - a) + f64::from(c) * a) as u8;
            }
        }
    }
}

#[test]
fn matte_of_opaque_is_opaque() {
    let ov = matted(20, 10, &|img| ink(img, 0, 0, 10, [200, 30, 40], 1.0));
    for y in 0..10 {
        for x in 0..20 {
            let p = ov.rgba[y * 20 + x];
            if x < 10 {
                assert_eq!(p, [200, 30, 40, 255]);
            } else {
                assert_eq!(p, [0; 4]);
            }
        }
    }
    let img: Rgb8 = Img::filled(4, 4, &[9, 8, 7]);
    assert!(opaque(&img).rgba.iter().all(|p| *p == [9, 8, 7, 255]));
    assert!(Overlay::empty(4, 4).is_clear());
}

#[test]
fn matte_of_half_ink_is_half() {
    let ov = matted(10, 10, &|img| ink(img, 0, 0, 10, [240, 200, 60], 0.5));
    for p in &ov.rgba {
        assert!((127..=128).contains(&p[3]), "{p:?}");
        assert_eq!([p[0], p[1], p[2]], [120, 100, 30]);
    }
    // and the count-in's own half-strength text: its letters are partly transparent
    let ov = matted(360, 640, &|img| {
        big_text(img, &mut DrawLog::new(), "3", 180.0, 320.0, 120.0, [255, 220, 90], 0.5)
    });
    let part = ov.rgba.iter().filter(|p| p[3] > 0 && p[3] < 255).count();
    assert!(part > 100 && ov.rgba.iter().all(|p| p[3] < 255), "{part}");
}

/// The largest channel difference of two pictures.
fn worst(a: &Rgb8, b: &Rgb8) -> i32 {
    a.data
        .iter()
        .zip(&b.data)
        .map(|(x, y)| (i32::from(*x) - i32::from(*y)).abs())
        .max()
        .unwrap_or(0)
}

#[test]
fn composite_cpu_reproduces_direct_draw() {
    let draw = |img: &mut Rgb8| {
        label(
            img,
            &mut DrawLog::new(),
            "SECOND WIND",
            20,
            30,
            [250, 240, 90],
            2,
            Some(font::OUTLINE),
        );
    };
    let mut direct: Rgb8 = Img::filled(200, 80, &[96, 110, 128]);
    draw(&mut direct);
    let mut comp: Rgb8 = Img::filled(200, 80, &[96, 110, 128]);
    composite_cpu(&mut comp, &matted(200, 80, &draw));
    assert!(worst(&direct, &comp) <= 1, "{}", worst(&direct, &comp));
    // a partly transparent draw too, within the matte's ±2
    let soft = |img: &mut Rgb8| {
        big_text(
            img,
            &mut DrawLog::new(),
            "FIGHT!",
            180.0,
            320.0,
            90.0,
            [255, 90, 60],
            0.6,
        )
    };
    let mut direct: Rgb8 = Img::filled(360, 640, &[40, 160, 90]);
    soft(&mut direct);
    let mut comp: Rgb8 = Img::filled(360, 640, &[40, 160, 90]);
    composite_cpu(&mut comp, &matted(360, 640, &soft));
    assert!(worst(&direct, &comp) <= 2, "{}", worst(&direct, &comp));
}

/// A small test layer: an opaque label and soft ink.
fn layer() -> Overlay {
    matted(90, 160, &|img| {
        label(
            img,
            &mut DrawLog::new(),
            "K.O.",
            10,
            20,
            [255, 255, 255],
            2,
            Some(font::OUTLINE),
        );
        ink(img, 40, 90, 30, [255, 60, 30], 0.45);
    })
}

#[test]
fn pixel_font_stays_crisp() {
    let ov = layer();
    let mut frame: Rgb8 = Img::filled(270, 480, &[50, 60, 70]);
    composite_cpu(&mut frame, &ov);
    crisp(&frame, &ov);
}

/// Every 3 x 3 block under an opaque UI pixel is uniform.
fn crisp(frame: &Rgb8, ov: &Overlay) {
    let s = frame.w / ov.w as usize;
    let mut n = 0;
    for y in 0..ov.h as usize {
        for x in 0..ov.w as usize {
            if ov.rgba[y * ov.w as usize + x][3] != 255 {
                continue;
            }
            n += 1;
            let first = frame.px(x * s, y * s).to_vec();
            for dy in 0..s {
                for dx in 0..s {
                    assert_eq!(frame.px(x * s + dx, y * s + dy), &first[..], "block ({x}, {y})");
                }
            }
        }
    }
    assert!(n > 50, "{n} opaque pixels");
}

fn floor_scene(r: &mut crate::Renderer, gpu: &Gpu) -> crate::Scene {
    let floor = r.meshes.add(gpu, crate::geom::plane(40.0, 40.0));
    crate::Scene {
        beam: None,
        camera: crate::Camera::default_ring(),
        ambient: [0.3; 3],
        instances: vec![crate::Instance::new(
            floor,
            crate::Material::toon(),
            crate::maths::IDENTITY,
        )],
        lights: vec![crate::Light {
            pos: [0.0, 2.0, 0.0],
            colour: [2.0, 1.0, 0.5],
            range: 8.0,
            priority: 0,
        }],
        ..crate::Scene::default()
    }
}

#[test]
fn gpu_composite_matches_cpu() {
    let gpu = Gpu::headless().expect("a GPU");
    let mut r = crate::Renderer::new(&gpu);
    let scene = floor_scene(&mut r, &gpu);
    let post = crate::Post::default();
    let base = r.render_sized(&gpu, &scene, &post, 270, 480);
    let ov = layer();
    r.set_overlay(&gpu, Some(&ov));
    let gpu_out = r.render_sized(&gpu, &scene, &post, 270, 480);
    let mut cpu_out = base.clone();
    composite_cpu(&mut cpu_out, &ov);
    assert!(worst(&gpu_out, &cpu_out) <= 1, "{}", worst(&gpu_out, &cpu_out));
    crisp(&gpu_out, &ov);
    // through the accumulation path too, once per output frame
    let two = [scene.clone(), scene.clone()];
    let acc = r.render_accum_sized(&gpu, &two, &[1.0, 1.0], &post, &mut crate::NoFx, None, 270, 480);
    r.set_overlay(&gpu, None);
    let mut acc_cpu = r.render_accum_sized(&gpu, &two, &[1.0, 1.0], &post, &mut crate::NoFx, None, 270, 480);
    composite_cpu(&mut acc_cpu, &ov);
    assert!(worst(&acc, &acc_cpu) <= 1, "{}", worst(&acc, &acc_cpu));
    // None clears it
    assert_eq!(worst(&r.render_sized(&gpu, &scene, &post, 270, 480), &base), 0);
}
