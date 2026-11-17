use super::*;
use crate::testkit::{arm, step_to_t, swing};

fn tri_area(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f64 {
    let x = cross(sub(b, a), sub(c, a));
    0.5 * f64::from((x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt())
}

#[test]
fn smear_covers_the_swing_arc() {
    // A 90 degree swing of a 1-unit arm on a 0.6 ball, over 30 steps (0.25 s).
    let (r, len) = (0.6, 1.0);
    let rec = swing(30, std::f64::consts::FRAC_PI_2, 0);
    let rig = arm(len);
    let t_end = step_to_t(30);
    let pts = trail_points(&rec, &rig, r, 0, 0, t_end, 0.5, &step_to_t);
    assert_eq!(pts.len(), 31);
    let def = TrailDef {
        style: TrailStyle::Smear,
        length_s: 0.5,
        ..TrailDef::default()
    };
    let mut v = Vec::new();
    build_strip(&def, &pts, [0.0, 10.0, 10.0], 1.0, 1, t_end, &mut v);
    let area: f64 = v
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| tri_area(t[0].pos, t[1].pos, t[2].pos))
        .sum();
    // A root bone pivots on the ball's surface (E1's rig), so the swept arc is a quarter disc of the bone's length.
    let _ = r;
    let arc = std::f64::consts::FRAC_PI_2 / 2.0 * len * len;
    assert!((area - arc).abs() < 0.1 * arc, "smear area {area:.3}, arc {arc:.3}");
}

#[test]
fn trail_follows_the_rig() {
    let rec = swing(30, 1.2, 10);
    let rig = arm(1.0);
    let mut segs = Vec::new();
    for s in [5u32, 17, 30, 35] {
        // Between two steps: the end point is the interpolated pose.
        let t_end = step_to_t(s) + 0.5 * crate::testkit::DT;
        let pts = trail_points(&rec, &rig, 0.6, 0, 0, t_end, 0.1, &step_to_t);
        let (base, tip, age) = *pts.last().unwrap();
        assert_eq!(age, 0.0);
        let a = &rec.steps[s as usize];
        let b = &rec.steps[s as usize + 1];
        let yaw = 0.5 * (a.joints[0][0].yaw + b.joints[0][0].yaw);
        let pose = Pose {
            joints: vec![Joint { yaw, ..Joint::REST }],
        };
        solve(&rig, &pose, [0.0, 0.0, 0.6], 0.6, 0.0, &mut segs);
        for i in 0..3 {
            assert!((tip[i] - segs[0].b[i]).abs() < 1e-9 && (base[i] - segs[0].a[i]).abs() < 1e-9);
        }
        // History: 12 steps (0.1 s at 120 Hz), oldest first.
        assert!(pts.len() >= 12.min(s as usize + 1), "{} points at step {s}", pts.len());
        assert!(pts.windows(2).all(|w| w[0].2 >= w[1].2));
    }
}

#[test]
fn trail_fades_at_tail() {
    let rec = swing(30, 1.5, 0);
    let rig = arm(1.0);
    let t_end = step_to_t(30);
    for style in [TrailStyle::Ribbon, TrailStyle::Smear, TrailStyle::Lightning] {
        let def = TrailDef {
            style,
            length_s: 0.2,
            ..TrailDef::default()
        };
        let pts = trail_points(&rec, &rig, 0.6, 0, 0, t_end, def.length_s, &step_to_t);
        let mut v = Vec::new();
        build_strip(&def, &pts, [0.0, 10.0, 10.0], 1.0, 1, t_end, &mut v);
        assert!(!v.is_empty());
        let head = v.iter().map(|x| x.along).fold(f32::INFINITY, f32::min);
        let tail = v.iter().map(|x| x.along).fold(0.0, f32::max);
        assert_eq!(head, 0.0, "{style:?}");
        // The oldest sample is the window's length back: the shader's fade is (1 − smooth(along))², 0 there.
        assert!(tail > 0.99, "{style:?}: tail at {tail}");
        // And nothing is drawn with no envelope.
        let mut none = Vec::new();
        build_strip(&def, &pts, [0.0, 10.0, 10.0], 0.0, 1, t_end, &mut none);
        assert!(none.is_empty());
    }
}

#[test]
fn smear_draws() {
    use bf2_render::{FxPass, Post, Renderer, Scene};
    struct P<'a>(&'a mut Trails, TrailSource<'a>, Vec<&'a FxRecord>, f64);
    impl FxPass for P<'_> {
        fn draw(&mut self, ctx: &mut FxCtx<'_>) {
            self.0.draw(ctx, &self.1, &self.2, self.3);
        }
    }
    let gpu = Gpu::headless().unwrap();
    let mut r = Renderer::new(&gpu);
    let mut trails = Trails::new(&gpu);
    let rec = swing(30, 2.0, 0);
    let rig = arm(1.2);
    let fx = FxRecord {
        t0: step_to_t(0),
        life: 1.0,
        exit: crate::record::Exit::Fade,
        env: crate::record::Env::new(0.03, 0.0, 0.3),
        seed: 3,
        anchor: crate::record::Anchor::Bone { k: 0, bone: 0 },
        kind: FxKind::Trail(TrailDef {
            length_s: 0.25,
            ..TrailDef::default()
        }),
    };
    let src = TrailSource {
        rec: &rec,
        rigs: [&rig, &rig],
        r_ball: [0.6, 0.6],
        step_to_t: &step_to_t,
    };
    let t = step_to_t(30);
    let mut pass = P(&mut trails, src, vec![&fx], t);
    let img = r.render_sized_with(&gpu, &Scene::default(), &Post::default(), 270, 480, Some(&mut pass));
    let lit = img.data.as_chunks::<3>().0.iter().filter(|p| p[0] > 60).count();
    assert!(lit > 500, "the smear lights {lit} pixels");
}
