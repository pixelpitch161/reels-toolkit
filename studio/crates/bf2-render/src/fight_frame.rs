//! From the recording to frames: which steps a video frame falls between, the state interpolated there, and the
//! whole frame drawn (stage, looks, projectiles).

use bf_picture::img::Rgb8;
use bf2_direct::TimeMap;
use bf2_sim::{Joint, Recording2, V3};

use crate::gpu::Gpu;
use crate::look2::Look2;
use crate::post::Post;
use crate::renderer::Renderer;
use crate::scene::Scene;
use crate::stage::Stage;

/// Video frames per second.
pub const FPS: f64 = 30.0;

/// The state drawn at one video time: step `step` moved `alpha` of the way toward step `step + 1`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepSnapLerp {
    /// Each ball's position (sim space; z = 0 is a ball resting on the canvas).
    pub pos: [V3; 2],
    /// Radians in the floor plane, turned the short way round.
    pub facing: [f64; 2],
    pub joints: [Vec<Joint>; 2],
    /// (id, centre) of each live projectile at `step`; one still alive at `step + 1` is interpolated.
    pub projs: Vec<(u32, V3)>,
    /// The HP at `step` (not interpolated: a bar drops when the hit lands).
    pub hp: [f64; 2],
}

/// One video frame of a fight.
#[derive(Clone, Debug)]
pub struct FightFrame<'a> {
    pub rec: &'a Recording2,
    /// Seconds of video.
    pub t_video: f64,
    /// Seconds of fight since the fight started, through the time map (negative before it): it runs slowly in
    /// slow-mo and stands still in the K.O. freeze.
    pub t_fight: f64,
    pub step: usize,
    /// 0 to 1, toward `step + 1`.
    pub alpha: f64,
    pub snap: StepSnapLerp,
}

/// Video frame `f` (at `f / 30` s), through the time map `map`.
pub fn frame_at<'a>(rec: &'a Recording2, map: &TimeMap, f: usize) -> FightFrame<'a> {
    frame_at_time(rec, map, f as f64 / FPS)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp3(a: V3, b: V3, t: f64) -> V3 {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

/// The angle `a` turned toward `b` the short way, by the share `t`.
fn lerp_angle(a: f64, b: f64, t: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let d = (b - a).rem_euclid(tau);
    let d = if d > std::f64::consts::PI { d - tau } else { d };
    a + d * t
}

/// The frame at `t_video` seconds of video (any time, for sub-frames): the fight time `map` shows there, so
/// sub-frames in slow-mo are closer together in fight time and a freeze shows one moment. Before the fight it
/// holds step 0, after the last step it holds the last, and while either ball is frozen by hitstop it holds the
/// step it is on.
pub fn frame_at_time<'a>(rec: &'a Recording2, map: &TimeMap, t_video: f64) -> FightFrame<'a> {
    let t_fight = map.fight_t(t_video);
    let n = rec.steps.len();
    let s = t_fight / rec.dt;
    let (step, mut alpha) = if n == 0 || s.is_nan() || s <= 0.0 {
        (0, 0.0)
    } else if s >= (n - 1) as f64 {
        (n - 1, 0.0)
    } else {
        let k = s.floor();
        (k as usize, s - k)
    };
    let Some(a) = rec.steps.get(step) else {
        return FightFrame {
            rec,
            t_video,
            t_fight,
            step,
            alpha: 0.0,
            snap: StepSnapLerp::default(),
        };
    };
    if a.hitstop.iter().any(|&h| h > 0) {
        alpha = 0.0;
    }
    let b = rec.steps.get(step + 1).filter(|_| alpha > 0.0).unwrap_or(a);
    let joints = |k: usize| -> Vec<Joint> {
        a.joints[k]
            .iter()
            .enumerate()
            .map(|(i, j)| b.joints[k].get(i).map_or(*j, |jb| j.lerp(*jb, alpha)))
            .collect()
    };
    let projs = a
        .projs
        .iter()
        .map(|&(id, p)| match b.projs.iter().find(|q| q.0 == id) {
            Some(&(_, q)) => (id, lerp3(p, q, alpha)),
            None => (id, p),
        })
        .collect();
    FightFrame {
        rec,
        t_video,
        t_fight,
        step,
        alpha,
        snap: StepSnapLerp {
            pos: [lerp3(a.pos[0], b.pos[0], alpha), lerp3(a.pos[1], b.pos[1], alpha)],
            facing: [
                lerp_angle(a.facing[0], b.facing[0], alpha),
                lerp_angle(a.facing[1], b.facing[1], alpha),
            ],
            joints: [joints(0), joints(1)],
            projs,
            hp: a.hp,
        },
    }
}

/// The scene of video frame `f`: the looks (weapons, instruments, their lights), the stage and balls, then a
/// default glowing sphere for each projectile no look drew.
pub fn scene_at(stage: &Stage, looks: &[Box<dyn Look2>; 2], f: &FightFrame<'_>, colours: [[f32; 3]; 2]) -> Scene {
    let mut scene = Scene::default();
    let mut drawn = Vec::new();
    for (k, look) in looks.iter().enumerate() {
        drawn.extend(look.draw(f, k, &mut scene));
    }
    stage.draw(f, colours, &mut scene);
    stage.draw_projectiles(f, &drawn, &mut scene);
    scene
}

/// Video frame `f` of the fight in `rec` (through the time map `map`), drawn at 1080 x 1920.
#[allow(clippy::too_many_arguments)]
pub fn render_frame(
    r: &mut Renderer,
    gpu: &Gpu,
    stage: &Stage,
    looks: &[Box<dyn Look2>; 2],
    rec: &Recording2,
    map: &TimeMap,
    f: usize,
    colours: [[f32; 3]; 2],
) -> Rgb8 {
    let frame = frame_at(rec, map, f);
    let scene = scene_at(stage, looks, &frame, colours);
    r.render(gpu, &scene, &Post::default())
}

#[cfg(test)]
#[path = "fight_frame_tests.rs"]
mod tests;
