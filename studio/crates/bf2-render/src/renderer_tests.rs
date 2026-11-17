//! The renderer's picture tests, on small scenes at 270 x 480. Pictures are compared by property, never by
//! bytes across devices.

use super::*;
use crate::geom;
use crate::scene::{Camera, Instance, Light, Material, Scene, Spot};

const TW: u32 = 270;
const TH: u32 = 480;

fn setup() -> (Gpu, Renderer) {
    let gpu = Gpu::headless().expect("a GPU");
    let r = Renderer::new(&gpu);
    (gpu, r)
}

fn lum(px: &[u8]) -> f32 {
    0.2126 * px[0] as f32 + 0.7152 * px[1] as f32 + 0.0722 * px[2] as f32
}

fn at(img: &Rgb8, p: [f32; 2]) -> [u8; 3] {
    let (x, y) = (p[0].round() as usize, p[1].round() as usize);
    let q = img.px(x.min(img.w - 1), y.min(img.h - 1));
    [q[0], q[1], q[2]]
}

fn camera(eye: [f32; 3], target: [f32; 3]) -> Camera {
    Camera {
        eye,
        target,
        roll: 0.0,
        fov_deg: 36.0,
    }
}

#[test]
fn toon_sphere_has_banded_shading() {
    let (gpu, mut r) = setup();
    let ball = r.meshes.add(&gpu, geom::icosphere(1.0, 4));
    let scene = Scene {
        camera: camera([0.0, 0.0, 6.0], [0.0; 3]),
        ambient: [0.05; 3],
        spot: None,
        lights: vec![Light {
            pos: [3.0, 3.0, 3.0],
            colour: [1.0; 3],
            range: 40.0,
        }],
        instances: vec![Instance::new(ball, Material::toon(), maths::IDENTITY).coloured([0.8; 3])],
    };
    let img = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let inside = |x: usize, y: usize| img.px(x, y).iter().any(|&c| c > 0);
    let mut levels = std::collections::BTreeMap::new();
    for y in 2..img.h - 2 {
        for x in 2..img.w - 2 {
            // The core: the pixel and everything within 2 pixels of it is inside the silhouette.
            let core = (y - 2..=y + 2).all(|yy| (x - 2..=x + 2).all(|xx| inside(xx, yy)));
            if core {
                *levels.entry(img.px(x, y).to_vec()).or_insert(0usize) += 1;
            }
        }
    }
    let total: usize = levels.values().sum();
    assert!(total > 10_000, "the sphere covers {total} core pixels");
    // A pixel where a band edge crosses a triangle edge resolves the two triangles' samples to an in-between
    // grey; those are single pixels, so a level counts only if it covers at least 0.05% of the core.
    let counted = levels.values().filter(|&&n| n * 2000 >= total).count();
    assert!(counted <= 4 + 2, "{counted} grey levels inside the sphere: {levels:?}");
    assert!(counted >= 4, "too flat: {levels:?}");
}

fn floor_scene(r: &mut Renderer, gpu: &Gpu) -> Scene {
    let floor = r.meshes.add(gpu, geom::plane(40.0, 40.0));
    Scene {
        camera: Camera::default_ring(),
        ambient: [0.1; 3],
        instances: vec![Instance::new(floor, Material::toon(), maths::IDENTITY)],
        ..Scene::default()
    }
}

#[test]
fn point_light_tints_the_floor() {
    let (gpu, mut r) = setup();
    let mut scene = floor_scene(&mut r, &gpu);
    scene.lights.push(Light {
        pos: [0.0, 2.0, 0.0],
        colour: [3.0, 0.0, 0.0],
        range: 6.0,
    });
    let img = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let redness = |p: [u8; 3]| p[0] as i32 - p[1] as i32;
    let under = at(&img, scene.camera.project([0.0; 3], TW, TH).unwrap());
    for corner in [[1.0, 1.0], [268.0, 1.0], [1.0, 478.0], [268.0, 478.0]] {
        let c = at(&img, corner);
        assert!(
            redness(under) > redness(c) + 40,
            "under the light {under:?}, corner {c:?}"
        );
    }
}

#[test]
fn shadow_darkens_floor_under_ball() {
    let (gpu, mut r) = setup();
    let mut scene = floor_scene(&mut r, &gpu);
    let ball = r.meshes.add(&gpu, geom::icosphere(1.0, 3));
    scene.instances.push(Instance::new(
        ball,
        Material::toon(),
        maths::translation([0.0, 2.0, 0.0]),
    ));
    scene.spot = Some(Spot {
        pos: [0.0, 12.0, 0.0],
        target: [0.0; 3],
        colour: [4.0; 3],
        angle_deg: 30.0,
        shadow: true,
    });
    let img = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    // On the camera's side of the ball, so the ball itself does not hide the floor there.
    let under = at(&img, scene.camera.project([0.0, 0.0, 0.9], TW, TH).unwrap());
    let beside = at(&img, scene.camera.project([3.5, 0.0, 0.9], TW, TH).unwrap());
    assert!(
        lum(&under) < 0.6 * lum(&beside),
        "under the ball {under:?}, beside it {beside:?}"
    );
    // And without the shadow, the same spot is lit.
    scene.spot.as_mut().unwrap().shadow = false;
    let img = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let unshadowed = at(&img, scene.camera.project([0.0, 0.0, 0.9], TW, TH).unwrap());
    assert!(lum(&unshadowed) > 0.9 * lum(&beside), "{unshadowed:?} {beside:?}");
}

#[test]
fn glass_is_see_through() {
    let (gpu, mut r) = setup();
    let cube = r.meshes.add(&gpu, geom::box_([3.0, 3.0, 0.5]));
    let ball = r.meshes.add(&gpu, geom::icosphere(1.0, 3));
    let scene = Scene {
        camera: camera([0.0, 0.0, 8.0], [0.0; 3]),
        ambient: [0.2; 3],
        instances: vec![
            Instance::new(ball, Material::glass([0.7, 0.8, 1.0]), maths::IDENTITY),
            Instance::new(cube, Material::Unlit, maths::translation([0.0, 0.0, -3.0])).coloured([0.9, 0.05, 0.05]),
        ],
        ..Scene::default()
    };
    let img = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let c = at(&img, [TW as f32 / 2.0, TH as f32 / 2.0]);
    assert!(
        c[0] as i32 > c[1] as i32 + 60 && c[0] as i32 > c[2] as i32 + 60,
        "through the ball: {c:?}"
    );
    // And the ball is there: its centre is not the bare red.
    let bare = r.render_sized(
        &gpu,
        &Scene {
            instances: scene.instances[1..].to_vec(),
            ..scene.clone()
        },
        &Post::default(),
        TW,
        TH,
    );
    assert_ne!(c, at(&bare, [TW as f32 / 2.0, TH as f32 / 2.0]));
}

#[test]
fn glass_sorts_back_to_front() {
    let (gpu, mut r) = setup();
    let ball = r.meshes.add(&gpu, geom::icosphere(1.0, 3));
    let glass = |tint| Material::Glass {
        tint,
        opacity: 0.6,
        fresnel: 0.9,
    };
    let red = |p| Instance::new(ball, glass([1.0, 0.0, 0.0]), maths::translation(p));
    let blue = |p| Instance::new(ball, glass([0.0, 0.0, 1.0]), maths::translation(p));
    let cam = camera([0.0, 0.0, 10.0], [0.0; 3]);
    // The overlap: on the line of sight through the middle of the two centres.
    let overlap = |s: &Scene| s.camera.project([0.0, 0.0, 1.0], TW, TH).unwrap();
    for (near_is_blue, insts) in [
        (true, vec![red([-0.5, 0.0, 0.0]), blue([0.5, 0.0, 2.0])]),
        (true, vec![blue([0.5, 0.0, 2.0]), red([-0.5, 0.0, 0.0])]),
        (false, vec![red([-0.5, 0.0, 2.0]), blue([0.5, 0.0, 0.0])]),
        (false, vec![blue([0.5, 0.0, 0.0]), red([-0.5, 0.0, 2.0])]),
    ] {
        let scene = Scene {
            camera: cam,
            ambient: [0.3; 3],
            instances: insts,
            ..Scene::default()
        };
        let img = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
        let p = at(&img, overlap(&scene));
        let (red_v, blue_v) = (p[0] as i32, p[2] as i32);
        assert!(red_v > 20 && blue_v > 20, "both tints show in the overlap: {p:?}");
        if near_is_blue {
            assert!(blue_v > red_v + 10, "blue is nearer: {p:?}");
        } else {
            assert!(red_v > blue_v + 10, "red is nearer: {p:?}");
        }
    }
}

#[test]
fn same_scene_same_pixels_on_one_device() {
    let (gpu, mut r) = setup();
    let mut scene = floor_scene(&mut r, &gpu);
    let ball = r.meshes.add(&gpu, geom::icosphere(1.0, 3));
    scene.instances.push(Instance::new(
        ball,
        Material::toon(),
        maths::translation([0.0, 2.0, 0.0]),
    ));
    scene.instances.push(Instance::new(
        ball,
        Material::glass([1.0, 0.5, 0.2]),
        maths::translation([2.0, 1.5, 1.0]),
    ));
    scene.spot = Some(Spot {
        pos: [0.0, 12.0, 0.0],
        target: [0.0; 3],
        colour: [4.0; 3],
        angle_deg: 30.0,
        shadow: true,
    });
    scene.lights.push(Light {
        pos: [2.0, 1.0, 2.0],
        colour: [0.0, 2.0, 1.0],
        range: 5.0,
    });
    let a = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let b = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    assert!(a.data == b.data, "two renders of one scene differ");
}
