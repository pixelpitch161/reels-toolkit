//! Engine 2's render job: the soundtrack (`bf_content::build_soundtrack2`: the score on the shadow recording,
//! slowing down with the slow-mo), the UI layer (`bf2_render::Ui2`: count-in, HUD, captions and VS card,
//! composited over every GPU frame after the tone map), the render checks of a whole video
//! ([`crate::checks::engine2`]: safe zone, levels, band, loudness, loop picture and sound, focus, the K.O. window;
//! no flash check), the effect timeline (the events wired to `bf2-fx`'s presets plus each look's own records), every
//! frame drawn by `bf2-fx`'s frame path (motion-blur sub-frames, effects, the screen chain) on one GPU thread (the
//! GPU is the parallelism) and encoded at 1080 x 1920, the finished file's checks, then the folder moved into place
//! under the same names and rules as the current engine's job ([`crate::job`]).
//!
//! Stages, in order ([`JobEvent::Stage`]): [`stage::SOUNDTRACK`], [`stage::CHECKS`] (a whole video only),
//! [`stage::FRAMES`], [`stage::ENCODE`], [`stage::VERIFY`], [`stage::SAVE`]. Cancel is looked at between the stages
//! and before every frame; it ends ffmpeg and leaves nothing behind.
//!
//! The loop checks of the finished file are reported but do not stop the job (an Engine 2 fight does not yet end
//! exactly where it began).

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use bf_content::fighters2;
use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_picture::config::{FPS, NF};
use bf_picture::look::DefaultLook;
use bf_picture::title::layout::Layout;
use bf_picture::title::{CardLook, TitleCard};
use bf_sound::wav::wav16_bytes;
use bf_story::BeatSheet;
use bf2_direct::{ArenaTrack, CameraTrack, DirectCfg, TimeMap};
use bf2_fx::FxTimeline;
use bf2_fx::frame::{Direction, FxFrame};
use bf2_render::{Gpu, Look2, Renderer, RowDef, Stage, Ui2, rows_of};
use bf2_sim::Recording2;

use crate::checks::engine2::{focus2, ko_window, pre_render2};
use crate::checks::{BandPlan, CheckResult};
use crate::encode::{ENCODING_NOTE, Encoder, Ffmpeg};
use crate::job::{JobEvent, JobOpts, stage};
use crate::output::{TempFolder, caption_text, clip_folder_name, folder_stem, png_bytes, prepare_root, taken};
use crate::probe::verify_until;
use crate::{ExportError, output};

/// Engine 2's frames.
pub const W2: usize = bf2_render::W as usize;
pub const H2: usize = bf2_render::H as usize;

/// The checks of the finished file that are reported but do not stop an Engine 2 job yet.
const ADVISORY: [&str; 2] = ["mp4_loop_picture", "mp4_loop_audio"];
/// The finished file's checks not run on Engine 2 at all (no flash cap on Engine 2, Aedan 2026-11-17).
const NOT_RUN: [&str; 1] = ["mp4_flash"];

/// What the director made of a fight, once per job: the time map, the camera of every frame and the arena's
/// reactions.
pub struct Directed {
    pub map: TimeMap,
    pub camera: CameraTrack,
    pub arena: ArenaTrack,
}

impl Directed {
    /// As a frame reads it.
    pub fn direction(&self) -> Direction<'_> {
        Direction {
            map: &self.map,
            camera: Some(&self.camera),
            arena: Some(&self.arena),
        }
    }
}

/// Directs the fight `rec` of `ff` with the default numbers: the time map, then the camera and the arena (the
/// beat on the file's tempo grid).
pub fn direct(ff: &FightFile, rec: &Recording2) -> Directed {
    let cfg = DirectCfg::default();
    let map = TimeMap::from_recording(rec, &cfg);
    let camera = CameraTrack::build(rec, &map, &cfg);
    let arena = ArenaTrack::build(rec, &map, ff.grid().bpm, &cfg);
    Directed { map, camera, arena }
}

/// The VS card, the HUD rows and the fighters' colours (sRGB, 0 to 255) of [`ui_parts`].
pub type UiParts = (TitleCard, [Vec<RowDef>; 2], [[f64; 3]; 2]);

/// What the UI layer of an Engine 2 fight is drawn with besides its show: the VS card (each fighter's roster
/// colours and its close-up model, `fighters2::card_mesh`, drawn with the plain look), each fighter's HUD rows and
/// its colour (sRGB, 0 to 255).
///
/// # Errors
/// A fighter with no Engine 2 roster entry or card model, or a card that cannot be made, in one sentence.
pub fn ui_parts(
    ff: &FightFile,
    show: &bf_core::show::Show,
    defs: [&bf2_sim::FighterDef; 2],
    dt: f64,
) -> Result<UiParts, ExportError> {
    let no = |name: &str| {
        ExportError::Content(format!(
            "This fight cannot be rendered: {name} has no Engine 2 card yet."
        ))
    };
    let mut cfgs = Vec::new();
    let mut meshes = Vec::new();
    for name in &show.names {
        cfgs.push(fighters2::cfg(name).ok_or_else(|| no(name))?);
        meshes.push(fighters2::card_mesh(name).ok_or_else(|| no(name))?);
    }
    let colours = [cfgs[0].color.map(f64::from), cfgs[1].color.map(f64::from)];
    let darks = [cfgs[0].dark.map(f64::from), cfgs[1].dark.map(f64::from)];
    let card_error = |e: &dyn std::fmt::Display| {
        ExportError::Content(format!(
            "The VS card could not be made ({e}), so nothing was saved; this is a fault in the app, not in your fight."
        ))
    };
    let layout = Layout::new(&show.names[0], &show.names[1], &ff.picture).map_err(|e| card_error(&e))?;
    let look = |k: usize| CardLook {
        look: &DefaultLook,
        mesh: &meshes[k],
        yaw: 0.0,
        roll: 0.0,
    };
    let card = TitleCard::with_layout(
        show,
        colours,
        darks,
        [look(0), look(1)],
        ff.series_tag.as_deref(),
        layout,
    )
    .map_err(|e| card_error(&e))?;
    Ok((card, [rows_of(defs[0], dt), rows_of(defs[1], dt)], colours))
}

fn fx_error(what: &str, e: &bf2_fx::FxError) -> ExportError {
    ExportError::Content(format!(
        "The effects of {what} could not be made ({}), so nothing was saved; this is a fault in the app, not in your \
         fight.",
        e.0
    ))
}

/// A fighter's colour (sRGB bytes) in linear light.
pub fn linear(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| bf_core::dmath::powf(f32::from(v) / 255.0, 2.2))
}

/// Renders the Engine 2 fight `ff` from its recording `rec` into a new folder under `opts.out_root`, with the
/// ffmpeg given, and gives that folder. `opts.workers` is not used (one GPU thread draws); `opts.frames` makes a
/// clip, as in the current job.
///
/// # Errors
/// An [`ExportError`] in one plain sentence; nothing is left in the output folder.
pub fn run_job2(
    ffmpeg: &Ffmpeg,
    ff: &FightFile,
    rec: Arc<Recording2>,
    opts: &JobOpts,
    on: &(dyn Fn(JobEvent) + Sync),
    cancel: &AtomicBool,
) -> Result<PathBuf, ExportError> {
    let stop = || {
        if cancel.load(Ordering::SeqCst) {
            Err(ExportError::cancelled())
        } else {
            Ok(())
        }
    };
    stop()?;
    bf_content::validate(ff)?;
    if ff.engine != Engine::Two {
        return Err(ExportError::Content(
            "This fight is for the current engine, so Engine 2's job cannot render it; this is a fault in the app, \
             not in your fight."
                .into(),
        ));
    }
    let names = [ff.fighters[0].as_str(), ff.fighters[1].as_str()];
    let mut looks = Vec::new();
    let mut defs = Vec::new();
    let mut colours = [[0.0; 3]; 2];
    let mut timeline = FxTimeline::new();
    // Directed once for the whole job: the frames, the effects (in video time), the camera and the arena read it.
    let directed = direct(ff, &rec);
    let map = &directed.map;
    let step_to_t = |s: u32| map.step_video_t(s);
    for (k, name) in names.iter().enumerate() {
        let (Some(def), Some(look), Some(fx)) = (fighters2::def(name), fighters2::look(name), fighters2::look_fx(name))
        else {
            return Err(ExportError::Content(format!(
                "This fight cannot be rendered: {name} has no Engine 2 look yet."
            )));
        };
        colours[k] = linear(def.colour);
        fx.fx(k, &rec, &step_to_t, &mut timeline)
            .map_err(|e| fx_error(name, &e))?;
        looks.push(look);
        defs.push(def);
    }
    let defs: [Arc<bf2_sim::FighterDef>; 2] = defs.try_into().unwrap_or_else(|_| unreachable!("two fighters"));
    bf2_fx::wire::wire(&rec, [&defs[0], &defs[1]], colours, &step_to_t, &mut timeline)
        .map_err(|e| fx_error("the fight", &e))?;
    let looks: [Box<dyn Look2>; 2] = looks.try_into().unwrap_or_else(|_| unreachable!("two fighters"));
    let seed = ff.seed.unwrap_or(rec.seed);
    let frames = match &opts.frames {
        None => 0..NF,
        Some(r) if r.start < r.end && r.end <= NF => r.clone(),
        Some(r) => {
            return Err(ExportError::Clip(format!(
                "The frames {} to {} are not a stretch of the video's {NF} frames (0 to {}), so nothing was made; \
                 ask for frames inside it.",
                r.start,
                r.end,
                NF - 1
            )));
        }
    };
    let whole = opts.frames.is_none();
    let root = std::path::absolute(&opts.out_root).unwrap_or_else(|_| opts.out_root.clone());
    prepare_root(&root)?;
    let stem = folder_stem(names);
    let (target, first) = if whole {
        (output::folder_name(names, seed, &root), root.join(&stem))
    } else {
        (
            clip_folder_name(names, seed, &frames, &root),
            root.join(format!("{stem}_frames{}-{}", frames.start, frames.end)),
        )
    };
    if taken(&target) {
        return Err(ExportError::Folder(format!(
            "Both {} and {} are already taken in the output folder, so nothing was made; move or rename one of \
             them, or choose another folder.",
            first.display(),
            target.display()
        )));
    }
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(stem);
    stop()?;
    let mut tmp = TempFolder::create(&root, &name)?;
    let job = Job2 {
        ff,
        rec: &rec,
        directed: &directed,
        looks,
        defs,
        timeline,
        colours,
        frames,
        whole,
        ffmpeg,
        opts,
        name: &name,
        on,
        cancel,
    };
    let made = job.make(&mut tmp, &stop).and_then(|()| tmp.move_to(&target));
    match made {
        Ok(()) => {
            on(JobEvent::Done(target.clone()));
            Ok(target)
        }
        Err(e) => match tmp.discard() {
            Ok(()) => Err(e),
            Err(left) => Err(e.with_clause(&left)),
        },
    }
}

struct Job2<'a> {
    ff: &'a FightFile,
    rec: &'a Recording2,
    directed: &'a Directed,
    looks: [Box<dyn Look2>; 2],
    defs: [Arc<bf2_sim::FighterDef>; 2],
    timeline: FxTimeline,
    colours: [[f32; 3]; 2],
    frames: Range<usize>,
    whole: bool,
    ffmpeg: &'a Ffmpeg,
    opts: &'a JobOpts,
    name: &'a str,
    on: &'a (dyn Fn(JobEvent) + Sync),
    cancel: &'a AtomicBool,
}

/// Sends every result; fails with the first that did not pass and is not advisory.
fn report2(results: Vec<CheckResult>, on: &(dyn Fn(JobEvent) + Sync)) -> Result<(), ExportError> {
    let mut failed = None;
    for r in results {
        if NOT_RUN.contains(&r.name) {
            continue;
        }
        if !r.pass && failed.is_none() && !ADVISORY.contains(&r.name) {
            failed = Some(r.clone());
        }
        on(JobEvent::Check(r));
    }
    match failed {
        Some(r) => Err(ExportError::Check(r)),
        None => Ok(()),
    }
}

impl Job2<'_> {
    fn make(mut self, tmp: &mut TempFolder, stop: &dyn Fn() -> Result<(), ExportError>) -> Result<(), ExportError> {
        let (on, name) = (self.on, self.name);
        let defs = [&*self.defs[0], &*self.defs[1]];
        let (map, cams) = (&self.directed.map, &self.directed.camera);
        on(JobEvent::Stage(stage::SOUNDTRACK));
        let sound = bf_content::build_soundtrack2(self.rec, defs, map, self.ff)?;
        let (card, rows, ui_colours) = ui_parts(self.ff, &sound.show, defs, self.rec.dt)?;
        let ui = Ui2::new(&sound.show, &sound.shadow, self.rec, map, cams, rows, ui_colours, &card);
        let wav = tmp.write("audio.wav", &wav16_bytes(&sound.soundtrack.master))?;
        stop()?;

        let gpu = Gpu::headless().map_err(|e| {
            ExportError::Frame(format!(
                "{}, so nothing was saved.",
                e.to_string().trim_end_matches('.')
            ))
        })?;
        let mut r = Renderer::new(&gpu);
        let stage_set = Stage::new(&mut r, &gpu);
        for look in &mut self.looks {
            look.setup(&mut r, &gpu);
        }
        let mut fx = FxFrame::new(&gpu, &r);
        let (looks, timeline, colours, rec) = (&self.looks, &self.timeline, self.colours, self.rec);
        let dir = self.directed.direction();
        let size = (W2 as u32, H2 as u32);
        // every frame: the UI layer of that frame, then the GPU frame with it composited after the tone map
        let mut draw = |f: usize| {
            r.set_overlay(&gpu, Some(&ui.draw(f).0));
            fx.render(
                &mut r, &gpu, &stage_set, looks, defs, rec, &dir, timeline, colours, f, size,
            )
        };

        if self.whole {
            on(JobEvent::Stage(stage::CHECKS));
            let cfg = |k: usize| {
                fighters2::cfg(&self.ff.fighters[k]).ok_or_else(|| {
                    ExportError::Content(format!(
                        "This fight cannot be rendered: {} has no Engine 2 roster entry yet.",
                        self.ff.fighters[k]
                    ))
                })
            };
            let sheet = BeatSheet::series([cfg(0)?, cfg(1)?]);
            let plan = BandPlan::of(self.ff)?;
            let focus = focus2(rec, map, cams, &sound.show);
            let ko = ko_window(map.ko_video_t(), &sheet);
            let results = pre_render2(&ui, &mut draw, &sound, focus, ko, &plan, self.cancel)
                .ok_or_else(ExportError::cancelled)?;
            report2(results, on)?;
            stop()?;
        }

        let (part, mp4) = (format!("{name}.mp4.part"), format!("{name}.mp4"));
        let part_path = tmp.file(&part);
        let total = self.frames.len();
        on(JobEvent::Stage(stage::FRAMES));
        let mut enc = Encoder::start_sized(self.ffmpeg, &wav, &part_path, total, self.opts.speed, W2, H2, 1)?;
        let t0 = Instant::now();
        for (i, f) in self.frames.clone().enumerate() {
            if self.cancel.load(Ordering::SeqCst) {
                enc.abort();
                return Err(ExportError::cancelled());
            }
            let img = draw(f);
            if let Err(e) = enc.push(&img) {
                enc.abort();
                return Err(e);
            }
            on(JobEvent::Progress {
                done: i + 1,
                total,
                seconds: t0.elapsed().as_secs_f64(),
            });
        }
        stop()?;
        on(JobEvent::Stage(stage::ENCODE));
        enc.finish()?;
        stop()?;

        on(JobEvent::Stage(stage::VERIFY));
        let results =
            verify_until(self.ffmpeg, &part_path, total, self.whole, self.cancel).ok_or_else(ExportError::cancelled)?;
        report2(results, on)?;
        stop()?;

        on(JobEvent::Stage(stage::SAVE));
        tmp.rename(&part, &mp4)?;
        tmp.remove("audio.wav")?;
        if let Some(note) = self.opts.speed.note() {
            tmp.write(ENCODING_NOTE, note.as_bytes())?;
        }
        if self.whole {
            // The cover: the first frame of the K.O. freeze, in video time.
            let f = self
                .directed
                .map
                .ko_video_t()
                .filter(|_| self.rec.outcome.t_ko.is_some())
                .map_or(NF / 2, |t| ((t * f64::from(FPS)).ceil() as usize).min(NF - 1));
            let cover = draw(f);
            tmp.write("cover.png", &png_bytes(&cover))?;
            tmp.write("caption.txt", caption_text(self.ff).as_bytes())?;
            // the fight file last: a folder with it is a finished video
            tmp.write(&format!("{name}.toml"), self.ff.to_toml().as_bytes())?;
        }
        stop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy() -> (FightFile, Recording2) {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fights/engine2/test_dummy.toml");
        let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).expect("the dummy's file parses");
        let def = |k: usize| fighters2::def(&ff.fighters[k]).expect("an Engine 2 fighter");
        let mut rules = bf2_sim::Rules2::new(ff.grid().t_fight);
        rules.dmg_scale = ff.dmg_scale;
        let setup = bf2_sim::Setup2 {
            fighters: [def(0), def(1)],
            rules,
            seed: ff.seed.expect("the file names its seed"),
        };
        (ff, bf2_sim::run(&setup, true).expect("the dummy fights"))
    }

    #[test]
    fn engine2_job_uses_the_director() {
        let (ff, rec) = dummy();
        let d = direct(&ff, &rec);
        let dir = d.direction();
        // Frame 30 of the 60-frame test clip (frames 600 to 660, as `tests/job2.rs` renders).
        let t = 630.0 / f64::from(FPS);
        let cam = bf2_fx::frame::camera_at(&dir, t).expect("the director's camera");
        assert_ne!(cam, bf2_render::Camera::default_ring());
        let pose = d.camera.poses[630];
        assert!((f64::from(cam.fov_deg) - pose.fov_deg).abs() < 1e-4);
        assert_eq!(dir.arena_at(t), Some(d.arena.frames[630]));
        assert_eq!(
            d.map.ko_video_t(),
            TimeMap::from_recording(&rec, &DirectCfg::default()).ko_video_t()
        );
    }

    /// No flash check on Engine 2: none before the render (`pre_render2` has none) and the finished file's is not
    /// reported, even when it would fail.
    #[test]
    fn engine2_job_skips_flash() {
        let events = std::sync::Mutex::new(Vec::new());
        let on = |e: JobEvent| events.lock().unwrap().push(e);
        let results = vec![
            CheckResult::failed("mp4_flash", "flashes".into()),
            CheckResult::new("mp4_frames", true, "ok".into(), &[]),
        ];
        assert!(report2(results, &on).is_ok());
        let names: Vec<&str> = events
            .into_inner()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                JobEvent::Check(r) => Some(r.name),
                _ => None,
            })
            .collect();
        assert_eq!(names, ["mp4_frames"]);
        assert!(!names.iter().any(|n| n.contains("flash")));
    }
}
