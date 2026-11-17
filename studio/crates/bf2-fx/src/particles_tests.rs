use bf_picture::img::Rgb8;
use bf2_render::{FxPass, Post, Scene};

use super::*;
use crate::record::{Anchor, Env};

const TW: u32 = 270;
const TH: u32 = 480;

fn burst(t0: f64, life: f32, seed: u32, def: BurstDef) -> FxRecord {
    FxRecord {
        t0,
        life,
        exit: Exit::Fade,
        env: Env::HIT,
        seed,
        anchor: Anchor::World([0.0, 0.0, 1.0]),
        kind: FxKind::Burst(def),
    }
}

struct Pass<'a> {
    p: &'a mut Particles,
    recs: Vec<(&'a FxRecord, [f64; 3])>,
    t: f64,
}

impl FxPass for Pass<'_> {
    fn draw(&mut self, ctx: &mut FxCtx<'_>) {
        self.p.draw(ctx, &self.recs, self.t);
    }
}

fn render(r: &mut Renderer, gpu: &Gpu, p: &mut Particles, recs: &[&FxRecord], t: f64) -> Rgb8 {
    let mut pass = Pass {
        p,
        recs: recs.iter().map(|&r| (r, [0.0, 0.0, 1.0])).collect(),
        t,
    };
    r.render_sized_with(gpu, &Scene::default(), &Post::default(), TW, TH, Some(&mut pass))
}

fn bright_share(img: &Rgb8) -> f64 {
    let n = img
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|p| (0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32) / 255.0 > 0.8)
        .count();
    n as f64 / (img.w * img.h) as f64
}

fn total_light(img: &Rgb8) -> f64 {
    img.data.iter().map(|&v| v as f64).sum()
}

fn sparks(count: u32) -> BurstDef {
    BurstDef {
        count,
        speed: (3.0, 14.0),
        size: (0.07, 0.03),
        colour: [14.0, 7.0, 2.5],
        colour_end: [3.0, 0.6, 0.1],
        stretch: 0.035,
        ..BurstDef::default()
    }
}

fn setup() -> (Gpu, Renderer, Particles) {
    let gpu = Gpu::headless().unwrap();
    let r = Renderer::new(&gpu);
    let p = Particles::new(&gpu, &r);
    (gpu, r, p)
}

#[test]
fn burst_is_dense_and_bright() {
    let (gpu, mut r, mut p) = setup();
    let rec = burst(10.0, 1.2, 77, sparks(20_000));
    let peak = [0.08, 0.15, 0.25, 0.4]
        .iter()
        .map(|dt| bright_share(&render(&mut r, &gpu, &mut p, &[&rec], 10.0 + dt)))
        .fold(0.0, f64::max);
    assert!(
        peak > 0.05,
        "only {:.2}% of the picture is bright at the peak",
        peak * 100.0
    );
}

#[test]
fn particles_are_stateless() {
    let (gpu, mut r, mut p) = setup();
    let a = burst(10.0, 1.5, 3, sparks(8000));
    let b = burst(
        10.2,
        2.0,
        4,
        BurstDef {
            shape: Shape::Smoke,
            colour: [0.2; 3],
            colour_end: [0.1; 3],
            size: (0.3, 0.9),
            ..sparks(500)
        },
    );
    let first = render(&mut r, &gpu, &mut p, &[&a, &b], 10.4);
    let _later = render(&mut r, &gpu, &mut p, &[&a, &b], 11.4);
    let again = render(&mut r, &gpu, &mut p, &[&a, &b], 10.4);
    assert!(first.data == again.data, "frame t drawn after t + 1 s differs");
    assert!(total_light(&first) > 0.0);
}

#[test]
fn floor_bounce_stays_above_floor() {
    for (bounce, seed) in [(0.5, 1u32), (0.0, 2), (0.9, 3)] {
        let def = BurstDef {
            count: 4000,
            speed: (2.0, 20.0),
            gravity: 25.0,
            drag: 1.0,
            bounce,
            emit_over: 0.2,
            ..BurstDef::default()
        };
        let mut seen_low = false;
        for i in 0..def.count {
            for k in 0..60 {
                let age = k as f32 / 30.0;
                if let Some(s) = particle_state(&def, seed, 2.0, [0.0, 0.8, 0.0], i, age) {
                    assert!(
                        s.pos[1] >= 0.0,
                        "particle {i} at {age} s is under the canvas: {:?}",
                        s.pos
                    );
                    seen_low |= s.pos[1] < 0.05;
                }
            }
        }
        assert!(seen_low || bounce == 0.0, "nothing reached the canvas");
    }
}

#[test]
fn over_budget_drops_oldest() {
    let (gpu, mut r, mut p) = setup();
    let old = burst(
        10.0,
        2.0,
        5,
        BurstDef {
            colour: [20.0, 0.0, 0.0],
            colour_end: [5.0, 0.0, 0.0],
            ..sparks(150_000)
        },
    );
    let new = burst(
        10.1,
        2.0,
        6,
        BurstDef {
            colour: [0.0, 20.0, 0.0],
            colour_end: [0.0, 5.0, 0.0],
            ..sparks(150_000)
        },
    );
    let img = render(&mut r, &gpu, &mut p, &[&old, &new], 10.3);
    assert_eq!(p.dropped(), 1);
    // Exactly the newest record's picture: the oldest is gone whole, the newest whole.
    let only_new = render(&mut r, &gpu, &mut p, &[&new], 10.3);
    assert!(img.data == only_new.data, "the frame is not the newest record alone");
    assert!(total_light(&img) > 0.0);
}

#[test]
fn exit_fades_particles() {
    let (gpu, mut r, mut p) = setup();
    for exit in [
        Exit::Fade,
        Exit::Ember,
        Exit::Shrink,
        Exit::Dissolve,
        Exit::Burst,
        Exit::FlyOff,
    ] {
        let rec = FxRecord {
            exit,
            ..burst(10.0, 1.0, 9, sparks(20_000))
        };
        let peak = [0.05, 0.1, 0.2]
            .iter()
            .map(|dt| total_light(&render(&mut r, &gpu, &mut p, &[&rec], 10.0 + dt)))
            .fold(0.0, f64::max);
        let last = total_light(&render(&mut r, &gpu, &mut p, &[&rec], 10.0 + 1.0 - 1.0 / 30.0));
        assert!(
            last < 0.02 * peak,
            "{exit:?}: the last frame keeps {:.1}% of the peak",
            100.0 * last / peak
        );
    }
}
