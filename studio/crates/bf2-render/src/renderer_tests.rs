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
            priority: 0,
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
        priority: 0,
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
        priority: 0,
    });
    let a = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let b = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    assert!(a.data == b.data, "two renders of one scene differ");
}

#[test]
fn an_empty_fx_pass_changes_nothing() {
    let (gpu, mut r) = setup();
    let mut scene = floor_scene(&mut r, &gpu);
    let ball = r.meshes.add(&gpu, geom::icosphere(1.0, 3));
    scene.instances.push(Instance::new(
        ball,
        Material::glass([1.0, 0.5, 0.2]),
        maths::translation([0.0, 1.0, 0.0]),
    ));
    let plain = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let hooked = r.render_sized_with(&gpu, &scene, &Post::default(), TW, TH, Some(&mut crate::fx::NoFx));
    assert!(plain.data == hooked.data);
}

#[test]
fn instance_encodes_its_surface() {
    use crate::scene::Surface;
    use crate::texture::TexId;
    let mesh = crate::mesh::MeshId(0);
    let plain = InstanceG::new(&Instance::new(mesh, Material::toon(), maths::IDENTITY));
    assert_eq!((plain.kind[2], plain.kind[3]), (0, 0), "no maps");
    assert_eq!(plain.mat_c[3], -1.0, "no glass back fill");
    let mapped = Instance::new(mesh, Material::glass([1.0; 3]), maths::IDENTITY).with_surface(Surface {
        albedo: Some(TexId(2)),
        normal: Some(TexId(0)),
        uv_scale: 6.0,
        normal_strength: 0.5,
        back: Some(([0.8, 0.7, 0.6], 0.9)),
    });
    let g = InstanceG::new(&mapped);
    assert_eq!((g.kind[2], g.kind[3]), (3, 1));
    assert_eq!((g.mat_b[0], g.mat_b[1], g.mat_b[2]), (0.9, 6.0, 0.5));
    assert_eq!(g.mat_c, [0.8, 0.7, 0.6, 0.9]);
}

/// A lit floor: a flat normal map draws the same picture as none; a dimpled one breaks the light up.
#[test]
fn normal_map_breaks_up_the_light() {
    use crate::scene::Surface;
    use crate::texture::{TEX_SIZE, flat_normals, hammered_height, normals_from_height};
    let (gpu, mut r) = setup();
    let mut scene = floor_scene(&mut r, &gpu);
    scene.instances[0].material = Material::matte();
    scene.spot = Some(Spot {
        pos: [0.0, 12.0, 4.0],
        target: [0.0; 3],
        colour: [0.4; 3],
        angle_deg: 40.0,
        shadow: false,
    });
    let flat = r.textures.add_normal(&gpu, &flat_normals(TEX_SIZE)).unwrap();
    let bumps = normals_from_height(&hammered_height(TEX_SIZE, 8, 1), TEX_SIZE, 40.0);
    let dimpled = r.textures.add_normal(&gpu, &bumps).unwrap();
    let spread = |img: &Rgb8| {
        let c = [TW as usize / 2, TH as usize / 2];
        let l: Vec<f32> = (0..40)
            .flat_map(|y| (0..40).map(move |x| (x, y)))
            .map(|(x, y)| lum(img.px(c[0] - 20 + x, c[1] - 20 + y)))
            .collect();
        let mean = l.iter().sum::<f32>() / l.len() as f32;
        (l.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / l.len() as f32).sqrt()
    };
    let none = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let with = |r: &mut Renderer, scene: &mut Scene, id| {
        scene.instances[0] = scene.instances[0].with_surface(Surface {
            normal: Some(id),
            uv_scale: 16.0,
            ..Surface::default()
        });
        r.render_sized(&gpu, scene, &Post::default(), TW, TH)
    };
    let flat_img = with(&mut r, &mut scene, flat);
    // (8-bit 128 is a hair off zero, so a toon band edge may move by a pixel: count the pixels that change.)
    let changed = none
        .data
        .chunks(3)
        .zip(flat_img.data.chunks(3))
        .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| (i32::from(*x) - i32::from(*y)).abs() > 1))
        .count();
    assert!(
        changed * 200 < none.data.len() / 3,
        "a flat map changes {changed} pixels"
    );
    let bumpy = with(&mut r, &mut scene, dimpled);
    assert!(
        spread(&bumpy) > spread(&none) + 2.0,
        "{} {}",
        spread(&bumpy),
        spread(&none)
    );
}

/// Additive light: a patch adds its colour (times its vertices' weight) to what is behind it, unlit, with no
/// highlight and no cover; where its weight is 0 nothing changes.
#[test]
fn additive_adds_without_covering() {
    let (gpu, mut r) = setup();
    let mut scene = floor_scene(&mut r, &gpu);
    let mut patch = geom::plane(6.0, 6.0);
    for v in &mut patch.verts {
        // weight 1 on the +x edge, 0 on the -x edge
        v.rgb = [(v.pos[0] / 6.0 + 0.5).clamp(0.0, 1.0); 3];
    }
    let patch = r.meshes.add(&gpu, patch);
    scene.spot = Some(Spot {
        pos: [0.0, 12.0, 4.0],
        target: [0.0; 3],
        colour: [0.2; 3],
        angle_deg: 40.0,
        shadow: false,
    });
    let before = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    scene.instances.push(
        Instance::new(
            patch,
            Material::Additive { strength: 0.3 },
            maths::translation([0.0, 0.01, 0.0]),
        )
        .coloured([0.0, 0.0, 1.0]),
    );
    let after = r.render_sized(&gpu, &scene, &Post::default(), TW, TH);
    let px = |img: &Rgb8, p: [f32; 3]| at(img, scene.camera.project(p, TW, TH).unwrap());
    let (b, a) = (px(&before, [2.5, 0.0, 0.0]), px(&after, [2.5, 0.0, 0.0]));
    assert!(i32::from(a[2]) > i32::from(b[2]) + 20, "blue is added: {b:?} -> {a:?}");
    assert!(
        (i32::from(a[0]) - i32::from(b[0])).abs() <= 2,
        "red stays: {b:?} -> {a:?}"
    );
    let (b, a) = (px(&before, [-2.9, 0.0, 0.0]), px(&after, [-2.9, 0.0, 0.0]));
    assert!(
        b.iter().zip(a).all(|(x, y)| (i32::from(*x) - i32::from(y)).abs() <= 3),
        "{b:?} -> {a:?}"
    );
}

/// A lit scene with edges, a gradient of light, glass and a coloured point light, for the pixel pass's tests.
fn pixel_scene(r: &mut Renderer, gpu: &Gpu) -> Scene {
    let mut scene = floor_scene(r, gpu);
    let ball = r.meshes.add(gpu, geom::icosphere(1.0, 3));
    scene.instances.push(Instance::new(
        ball,
        Material::toon(),
        maths::translation([0.0, 1.0, 0.0]),
    ));
    scene.instances.push(Instance::new(
        ball,
        Material::glass([1.0, 0.5, 0.2]),
        maths::translation([2.0, 1.0, 1.0]),
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
        priority: 0,
    });
    scene
}

/// Whether every `n x n` block of `img` (from the top left) is one colour.
fn whole_blocks(img: &Rgb8, n: usize) -> Result<(), String> {
    for y in 0..img.h {
        for x in 0..img.w {
            let (bx, by) = (x / n * n, y / n * n);
            if img.px(x, y) != img.px(bx, by) {
                return Err(format!("({x}, {y}) differs from its block's corner ({bx}, {by})"));
            }
        }
    }
    Ok(())
}

#[test]
fn pixel_pass_draws_whole_art_pixels_on_the_value_steps() {
    let (gpu, mut r) = setup();
    let scene = pixel_scene(&mut r, &gpu);
    // 1080 wide: pixel 3 is 360 art pixels across, as the current engine draws
    let post = Post::ring();
    assert_eq!((post.pixel, post.step), (3, 5.0), "the default is the current engine's grid and steps");
    let img = r.render_sized(&gpu, &scene, &post, 1080, 960);
    whole_blocks(&img, 3).unwrap();
    let mut levels = std::collections::BTreeSet::new();
    for (i, &c) in img.data.iter().enumerate() {
        assert_eq!(c % 5, 0, "byte {i} is {c}, not on a step of 5");
        levels.insert(c);
    }
    assert!(levels.len() >= 12, "the picture is not flat: {levels:?}");
}

#[test]
fn pixel_two_is_a_finer_grid() {
    let (gpu, mut r) = setup();
    let scene = pixel_scene(&mut r, &gpu);
    let post = Post { pixel: 2, ..Post::ring() };
    let img = r.render_sized(&gpu, &scene, &post, 1080, 960);
    whole_blocks(&img, 2).unwrap();
    assert!(whole_blocks(&img, 6).is_err(), "pixel 2 is not as coarse as pixel 6");
    let three = r.render_sized(&gpu, &scene, &Post::ring(), 1080, 960);
    assert!(whole_blocks(&three, 2).is_err(), "pixel 3 is coarser than pixel 2");
}

#[test]
fn pixel_pass_keeps_the_art_grid_at_the_previews_size() {
    let (gpu, mut r) = setup();
    let scene = pixel_scene(&mut r, &gpu);
    // half the video's width: still 360 art pixels across, so 1.5 output pixels each
    let img = r.render_sized(&gpu, &scene, &Post::ring(), 540, 480);
    for y in 0..img.h {
        let mut runs = 1;
        for x in 1..img.w {
            if img.px(x, y) != img.px(x - 1, y) {
                runs += 1;
            }
        }
        assert!(runs <= 360, "row {y} has {runs} runs, more than the 360 art pixels");
    }
    // pixel 1 at the video's size, with no steps: the pass is off and the picture is as before
    let plain = Post { pixel: 1, step: 0.0, ..Post::ring() };
    let mut old = plain;
    old.pixel = 0;
    let a = r.render_sized(&gpu, &scene, &plain, 1080, 960);
    let b = r.render_sized(&gpu, &scene, &old, 1080, 960);
    assert!(a.data == b.data);
    assert!(whole_blocks(&a, 3).is_err(), "pixel 1 leaves the full resolution");
}
