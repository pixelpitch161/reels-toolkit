//! A fight frame with its effects: the timeline's live records drawn into every sub-frame of the shutter
//! ([`subframes`], [`shutter_times`]), camera shake and effect lights added to each sub-frame's scene, the
//! sub-frames accumulated by `Renderer::render_accum`, then the screen chain. This is E2's `render_frame` with an
//! effect timeline: it lives here because `bf2-render` cannot know `bf2-fx`'s records.

use bf_picture::img::Rgb8;
use bf2_direct::{ArenaFrame, ArenaTrack, CameraTrack, TimeMap};
use bf2_render::{Camera, FightFrame, FxCtx, FxPass, Gpu, Look2, Post, Renderer, Stage, frame_at_time, scene_at};
use bf2_sim::{FighterDef, Pose, Recording2, solve};

use crate::decal::Decals;
use crate::lights::lights_for;
use crate::meshfx::MeshFx;
use crate::particles::Particles;
use crate::record::{Anchor, FxError, FxRecord};
use crate::screen::{Screen, shake_camera, shutter_times, subframes};
use crate::timeline::FxTimeline;
use crate::trail::{TrailSource, Trails};

/// What a look adds to the timeline: its own records for fighter `k` of `rec` (smears, its special's beam, ...).
pub trait LookFx: Send + Sync {
    /// # Errors
    /// A record the timeline refuses.
    fn fx(
        &self,
        k: usize,
        rec: &Recording2,
        step_to_t: &dyn Fn(u32) -> f64,
        out: &mut FxTimeline,
    ) -> Result<(), FxError> {
        let _ = (k, rec, step_to_t, out);
        Ok(())
    }

    /// How the shared presets' records of this fighter's event `e` are turned down (`tone::Restraint::NONE`: as
    /// they are). The K.O. event is the winner's.
    fn tone(&self, e: &bf2_sim::EvKind) -> crate::tone::Restraint {
        let _ = e;
        crate::tone::Restraint::NONE
    }
}

/// Where `a` is in frame `f` (sim space).
pub fn resolve(a: &Anchor, f: &FightFrame<'_>, defs: [&FighterDef; 2]) -> [f64; 3] {
    match a {
        Anchor::World(p) => *p,
        Anchor::Body { k, offset } => {
            let k = (*k).min(1);
            let p = f.snap.pos[k];
            [p[0] + offset[0], p[1] + offset[1], p[2] + defs[k].r + offset[2]]
        }
        Anchor::Bone { k, bone } => {
            let k = (*k).min(1);
            let p = f.snap.pos[k];
            let r = defs[k].r;
            let centre = [p[0], p[1], p[2] + r];
            let mut segs = Vec::new();
            let joints = f.snap.joints[k].clone();
            if joints.len() == defs[k].rig.bones.len() {
                solve(&defs[k].rig, &Pose { joints }, centre, r, f.snap.facing[k], &mut segs);
            }
            segs.get(*bone).map_or(centre, |s| s.b)
        }
        // Screen effects place themselves; anything else anchored to the screen sits at the ring centre.
        Anchor::Screen => [0.0; 3],
    }
}

/// What the director decided for a fight, as a frame reads it: the time map, and the camera and arena tracks
/// when there are any (without a camera track the frame keeps the default ring camera and the effects' shake;
/// without an arena track the arena is quiet).
#[derive(Clone, Copy, Debug)]
pub struct Direction<'a> {
    pub map: &'a TimeMap,
    pub camera: Option<&'a CameraTrack>,
    pub arena: Option<&'a ArenaTrack>,
}

impl<'a> Direction<'a> {
    /// Only the time map: the default camera and a quiet arena.
    pub fn time_only(map: &'a TimeMap) -> Direction<'a> {
        Direction {
            map,
            camera: None,
            arena: None,
        }
    }

    /// The arena of the video frame nearest `t` (video seconds).
    pub fn arena_at(&self, t: f64) -> Option<ArenaFrame> {
        let frames = &self.arena?.frames;
        let last = frames.len().checked_sub(1)?;
        let f = (t * bf2_render::fight_frame::FPS).round().clamp(0.0, last as f64) as usize;
        Some(frames[f])
    }
}

/// The director's camera at video time `t` (any time, for sub-frames): its poses and shake interpolated between
/// the two video frames around `t`, the shake (pixels of the 1080 x 1920 picture) applied as a sideways slide.
/// `None` without a camera track.
pub fn camera_at(dir: &Direction<'_>, t: f64) -> Option<Camera> {
    let track = dir.camera?;
    let last = track.poses.len().checked_sub(1)?;
    let x = (t * bf2_render::fight_frame::FPS).clamp(0.0, last as f64);
    let i = (x.floor() as usize).min(last);
    let j = (i + 1).min(last);
    let a = x - i as f64;
    let (p, q) = (track.poses[i], track.poses[j]);
    let l = |u: f64, v: f64| u + (v - u) * a;
    let l3 = |u: [f64; 3], v: [f64; 3]| [l(u[0], v[0]), l(u[1], v[1]), l(u[2], v[2])];
    let pose = bf2_direct::CamPose {
        eye: l3(p.eye, q.eye),
        target: l3(p.target, q.target),
        roll: l(p.roll, q.roll),
        fov_deg: l(p.fov_deg, q.fov_deg),
    };
    let shake = |k: usize| track.shake.get(k).copied().unwrap_or([0.0; 2]);
    let (s, u) = (shake(i), shake(j));
    Some(Camera::from(pose).shifted_px(l(s[0], u[0]), l(s[1], u[1]), bf2_direct::project::H))
}

/// The effect passes, built once.
pub struct FxFrame {
    pub particles: Particles,
    pub trails: Trails,
    pub meshfx: MeshFx,
    pub decals: Decals,
    pub screen: Screen,
    /// The post settings every frame is drawn with: [`Post::ring`], the current engine's grade, value steps and
    /// pixel grid (its `pixel` is the art pixel's size, 3 by default).
    pub post: Post,
}

/// One sub-frame's records, anchors resolved.
struct Sub<'a> {
    t: f64,
    recs: Vec<(&'a FxRecord, [f64; 3])>,
    frame: FightFrame<'a>,
}

struct Pass<'a, 'b> {
    particles: &'b mut Particles,
    trails: &'b mut Trails,
    meshfx: &'b mut MeshFx,
    decals: &'b mut Decals,
    subs: &'b [Sub<'a>],
    src: &'b TrailSource<'a>,
    defs: [&'b FighterDef; 2],
}

impl FxPass for Pass<'_, '_> {
    fn draw(&mut self, ctx: &mut FxCtx<'_>) {
        let Some(s) = self.subs.get(ctx.sub) else { return };
        let plain: Vec<&FxRecord> = s.recs.iter().map(|(r, _)| *r).collect();
        let defs = self.defs;
        let frame = &s.frame;
        let res = |a: &Anchor| resolve(a, frame, defs);
        self.decals.draw(ctx, &s.recs, s.t);
        self.meshfx.draw(ctx, &s.recs, &res, s.t);
        self.trails.draw(ctx, self.src, &plain, s.t);
        self.particles.draw(ctx, &s.recs, s.t);
    }
}

impl FxFrame {
    pub fn new(gpu: &Gpu, r: &Renderer) -> FxFrame {
        FxFrame {
            particles: Particles::new(gpu, r),
            trails: Trails::new(gpu),
            meshfx: MeshFx::new(gpu),
            decals: Decals::new(gpu),
            screen: Screen::new(gpu),
            post: Post::ring(),
        }
    }

    /// Video frame `f` (at `f / 30` s) of the fight in `rec` as `dir` directs it (the time map, and the
    /// director's camera, shake and arena when it has them), with the effects of `timeline` (whose records are in
    /// video time, from `map.step_video_t`), at `w x h`. With the director's camera, the shake is the director's
    /// alone: the effects' own shake records are not added on top.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        r: &mut Renderer,
        gpu: &Gpu,
        stage: &Stage,
        looks: &[Box<dyn Look2>; 2],
        defs: [&FighterDef; 2],
        rec: &Recording2,
        dir: &Direction<'_>,
        timeline: &FxTimeline,
        colours: [[f32; 3]; 2],
        f: usize,
        size: (u32, u32),
    ) -> Rgb8 {
        let t = f as f64 / bf2_render::fight_frame::FPS;
        let map = dir.map;
        let step_to_t = |s: u32| map.step_video_t(s);
        let src = TrailSource {
            rec,
            rigs: [&defs[0].rig, &defs[1].rig],
            r_ball: [defs[0].r, defs[1].r],
            step_to_t: &step_to_t,
        };
        // the pixel look draws one moment, as the current engine does (no motion blur: its smears are effects);
        // sub-frames would average into soft in-between tones the hard art pixels cannot hold
        let n = if self.post.pixel > 1 {
            1
        } else {
            subframes(&src, t, timeline)
        };
        let times = shutter_times(t, n);
        let mut idx = Vec::new();
        let mut subs = Vec::with_capacity(times.len());
        let mut scenes = Vec::with_capacity(times.len());
        for &ts in &times {
            idx.clear();
            timeline.active(ts, &mut idx);
            let mut frame = frame_at_time(rec, map, ts);
            if let Some(a) = dir.arena_at(ts) {
                frame.arena = a;
            }
            let recs: Vec<(&FxRecord, [f64; 3])> = idx
                .iter()
                .map(|&i| {
                    let r = &timeline.records[i];
                    (r, resolve(&r.anchor, &frame, defs))
                })
                .collect();
            let mut scene = scene_at(stage, looks, &frame, colours);
            let plain: Vec<&FxRecord> = recs.iter().map(|(r, _)| *r).collect();
            scene.camera = match camera_at(dir, ts) {
                Some(c) => c,
                None => shake_camera(&scene.camera, &plain, ts),
            };
            lights_for(ts, &recs, &mut scene.lights);
            scenes.push(scene);
            subs.push(Sub { t: ts, recs, frame });
        }
        // The screen chain reads the frame's own time.
        idx.clear();
        timeline.active(t, &mut idx);
        let mid = frame_at_time(rec, map, t);
        let now: Vec<(&FxRecord, [f64; 3])> = idx
            .iter()
            .map(|&i| {
                let r = &timeline.records[i];
                (r, resolve(&r.anchor, &mid, defs))
            })
            .collect();
        let res = |a: &Anchor| resolve(a, &mid, defs);
        self.screen.set(&now, &res, t);
        let weights = vec![1.0; scenes.len()];
        let FxFrame {
            particles,
            trails,
            meshfx,
            decals,
            screen,
            post,
        } = self;
        let mut pass = Pass {
            particles,
            trails,
            meshfx,
            decals,
            subs: &subs,
            src: &src,
            defs,
        };
        r.render_accum_sized(gpu, &scenes, &weights, post, &mut pass, Some(screen), size.0, size.1)
    }
}
