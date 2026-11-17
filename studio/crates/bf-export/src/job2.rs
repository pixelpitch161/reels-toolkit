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
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_picture::config::{FPS, NF};
use bf_picture::img::Rgb8;
use bf_picture::look::DefaultLook;
use bf_picture::title::layout::Layout;
use bf_picture::title::{CardLook, TitleCard};
use bf_sound::wav::wav16_bytes;
use bf_story::BeatSheet;
use bf2_direct::{ArenaTrack, CameraTrack, DirectCfg, TimeMap};
use bf2_fx::FxTimeline;
use bf2_fx::frame::{Direction, FxFrame};
use bf2_render::{Gpu, Look2, Overlay, Renderer, RowDef, Stage, Ui2, rows_of};
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
    let scene = Scene2::new(ff, rec)?;
    let seed = ff.seed.unwrap_or(scene.rec.seed);
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
        scene,
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
    scene: Scene2,
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

/// Everything of an Engine 2 fight a frame is drawn from that needs no GPU: the fighters' definitions and looks,
/// the effect timeline (the events wired to `bf2-fx`'s presets plus each look's own records, in video time), the
/// fighters' colours, what the director made of the fight, and the UI layer's parts (the video-clock show and
/// shadow, `bf_content::video_show2`'s, the VS card, the HUD rows and colours, [`ui_parts`]'s). Made once per fight.
pub struct Scene2 {
    pub rec: Arc<Recording2>,
    pub directed: Directed,
    /// The show on the video clock: its captions and K.O. are where the picture shows them.
    pub show: Show,
    shadow: Recording,
    looks: [Box<dyn Look2>; 2],
    defs: [Arc<bf2_sim::FighterDef>; 2],
    timeline: FxTimeline,
    colours: [[f32; 3]; 2],
    card: TitleCard,
    rows: [Vec<RowDef>; 2],
    ui_colours: [[f64; 3]; 2],
}

impl Scene2 {
    /// The scene of the fight `rec` of `ff`.
    ///
    /// # Errors
    /// A fighter with no Engine 2 look, effects that cannot be made, a fight with no K.O., or a fighter with no
    /// Engine 2 card, in one sentence.
    pub fn new(ff: &FightFile, rec: Arc<Recording2>) -> Result<Scene2, ExportError> {
        let names = [ff.fighters[0].as_str(), ff.fighters[1].as_str()];
        let mut looks = Vec::new();
        let mut defs = Vec::new();
        let mut colours = [[0.0; 3]; 2];
        let mut timeline = FxTimeline::new();
        // Directed once for the whole fight: the frames, the effects (in video time), the camera and the arena
        // read it.
        let directed = direct(ff, &rec);
        let map = &directed.map;
        let step_to_t = |s: u32| map.step_video_t(s);
        for (k, name) in names.iter().enumerate() {
            let (Some(def), Some(look), Some(fx)) =
                (fighters2::def(name), fighters2::look(name), fighters2::look_fx(name))
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
        let dref = [&*defs[0], &*defs[1]];
        let (show, shadow) = bf_content::video_show2(&rec, dref, map, ff)?;
        let (card, rows, ui_colours) = ui_parts(ff, &show, dref, rec.dt)?;
        Ok(Scene2 {
            rec,
            directed,
            show,
            shadow,
            looks,
            defs,
            timeline,
            colours,
            card,
            rows,
            ui_colours,
        })
    }

    /// The VS card.
    pub fn card(&self) -> &TitleCard {
        &self.card
    }

    /// The fight's UI layer (count-in, HUD, captions, VS card), frame by frame.
    pub fn ui(&self) -> Ui2<'_> {
        Ui2::new(
            &self.show,
            &self.shadow,
            &self.rec,
            &self.directed.map,
            &self.directed.camera,
            self.rows.clone(),
            self.ui_colours,
            &self.card,
        )
    }
}

/// The UI layer `ov` (360 x 640) for a `w x h` frame: as it is when the frame is a whole multiple of it (the GPU
/// pass scales it up exactly, nearest), otherwise resampled to the frame's size, nearest (texel `x·ov.w / w`, the
/// same texel the GPU pass reads on a whole multiple), so the preview's smaller frames carry it too.
fn fit(ov: Overlay, (w, h): (u32, u32)) -> Overlay {
    let s = w / ov.w.max(1);
    if s >= 1 && w == s * ov.w && h == s * ov.h {
        return ov;
    }
    let (ow, oh) = (u64::from(ov.w), u64::from(ov.h));
    let mut rgba = Vec::with_capacity((w * h) as usize);
    for y in 0..u64::from(h) {
        let row = (y * oh / u64::from(h)) * ow;
        for x in 0..u64::from(w) {
            rgba.push(ov.rgba[(row + x * ow / u64::from(w)) as usize]);
        }
    }
    Overlay { w, h, rgba }
}

/// The frames of an Engine 2 fight, drawn on demand in any order at any size by `bf2-fx`'s frame path (motion-blur
/// sub-frames, effects, the screen chain) with the fight's UI layer composited after the tone map, exactly as the
/// render job draws them: the job draws through it at 1080 x 1920, the app's preview at a smaller size on the
/// window's own device (HUD, captions, count-in and VS card included).
pub struct Frames2 {
    gpu: Gpu,
    r: Renderer,
    stage: Stage,
    fx: FxFrame,
    scene: Scene2,
    size: (u32, u32),
}

impl Frames2 {
    /// The frames of the fight `rec` of `ff` at `w x h`, drawn on `gpu`'s device (shared, not copied).
    ///
    /// # Errors
    /// As [`Scene2::new`].
    pub fn new(gpu: &Gpu, ff: &FightFile, rec: Arc<Recording2>, w: u32, h: u32) -> Result<Frames2, ExportError> {
        Ok(Frames2::of_scene(gpu, Scene2::new(ff, rec)?, w, h))
    }

    /// The frames of `scene` at `w x h` on `gpu`'s device.
    pub fn of_scene(gpu: &Gpu, mut scene: Scene2, w: u32, h: u32) -> Frames2 {
        let gpu = Gpu::from_device(gpu.device.clone(), gpu.queue.clone(), gpu.adapter_name.clone());
        let mut r = Renderer::new(&gpu);
        let stage = Stage::new(&mut r, &gpu);
        for look in &mut scene.looks {
            look.setup(&mut r, &gpu);
        }
        let fx = FxFrame::new(&gpu, &r);
        Frames2 {
            gpu,
            r,
            stage,
            fx,
            scene,
            size: (w, h),
        }
    }

    /// How many frames the video has.
    pub fn nf(&self) -> usize {
        NF
    }

    /// The scene the frames are drawn from.
    pub fn scene(&self) -> &Scene2 {
        &self.scene
    }

    /// Draws frame `f` (at `f / 30` s), its UI layer included.
    pub fn frame(&mut self, f: usize) -> Rgb8 {
        self.with_ui(|_, draw| draw(f))
    }

    /// Calls `body` with the fight's UI layer and the frame drawer ([`Frames2::frame`]) together, as the checks
    /// before the render read both (`checks::engine2::pre_render2`).
    pub fn with_ui<R>(&mut self, body: impl FnOnce(&Ui2<'_>, &mut dyn FnMut(usize) -> Rgb8) -> R) -> R {
        let Frames2 {
            gpu,
            r,
            stage,
            fx,
            scene: sc,
            size,
        } = self;
        let ui = sc.ui();
        let defs = [&*sc.defs[0], &*sc.defs[1]];
        let dir = sc.directed.direction();
        // every frame: the UI layer of that frame, then the GPU frame with it composited after the tone map
        let mut draw = |f: usize| {
            r.set_overlay(gpu, Some(&fit(ui.draw(f).0, *size)));
            fx.render(
                r,
                gpu,
                stage,
                &sc.looks,
                defs,
                &sc.rec,
                &dir,
                &sc.timeline,
                sc.colours,
                f,
                *size,
            )
        };
        body(&ui, &mut draw)
    }
}

/// The beat sheet of the fight `ff`'s two Engine 2 fighters (its K.O. window).
fn sheet_of(ff: &FightFile) -> Result<BeatSheet, ExportError> {
    let cfg = |k: usize| {
        fighters2::cfg(&ff.fighters[k]).ok_or_else(|| {
            ExportError::Content(format!(
                "This fight cannot be rendered: {} has no Engine 2 roster entry yet.",
                ff.fighters[k]
            ))
        })
    };
    Ok(BeatSheet::series([cfg(0)?, cfg(1)?]))
}

/// The K.O. window check of `scene`, as the job runs it: [`ko_window`] on the K.O. in video time (after slow-mo,
/// `map.ko_video_t`) and the fighters' beat sheet.
///
/// # Errors
/// A fighter with no Engine 2 roster entry, in one sentence.
pub fn ko_check(ff: &FightFile, scene: &Scene2) -> Result<CheckResult, ExportError> {
    let ko = scene.rec.outcome.t_ko.and(scene.directed.map.ko_video_t());
    Ok(ko_window(ko, &sheet_of(ff)?))
}

impl Job2<'_> {
    fn make(self, tmp: &mut TempFolder, stop: &dyn Fn() -> Result<(), ExportError>) -> Result<(), ExportError> {
        let (on, name) = (self.on, self.name);
        on(JobEvent::Stage(stage::SOUNDTRACK));
        let sound = {
            let sc = &self.scene;
            let defs = [&*sc.defs[0], &*sc.defs[1]];
            bf_content::build_soundtrack2(&sc.rec, defs, &sc.directed.map, self.ff)?
        };
        let wav = tmp.write("audio.wav", &wav16_bytes(&sound.soundtrack.master))?;
        stop()?;

        let gpu = Gpu::headless().map_err(|e| {
            ExportError::Frame(format!(
                "{}, so nothing was saved.",
                e.to_string().trim_end_matches('.')
            ))
        })?;
        // The cover: the first frame of the K.O. freeze, in video time.
        let cover_frame = self
            .scene
            .directed
            .map
            .ko_video_t()
            .filter(|_| self.scene.rec.outcome.t_ko.is_some())
            .map_or(NF / 2, |t| ((t * f64::from(FPS)).ceil() as usize).min(NF - 1));
        let ko = if self.whole {
            Some(ko_check(self.ff, &self.scene)?)
        } else {
            None
        };
        let mut frames2 = Frames2::of_scene(&gpu, self.scene, W2 as u32, H2 as u32);

        if let Some(ko) = ko {
            on(JobEvent::Stage(stage::CHECKS));
            let plan = BandPlan::of(self.ff)?;
            let sc = frames2.scene();
            let focus = focus2(&sc.rec, &sc.directed.map, &sc.directed.camera, &sc.show);
            let results = frames2
                .with_ui(|ui, draw| pre_render2(ui, draw, &sound, focus, ko, &plan, self.cancel))
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
            let img = frames2.frame(f);
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
            let cover = frames2.frame(cover_frame);
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

    #[test]
    fn frames2_draws_any_size_in_any_order() {
        let Ok(gpu) = Gpu::headless() else {
            eprintln!("no GPU here: skipped");
            return;
        };
        let (ff, rec) = dummy();
        let mut frames = Frames2::new(&gpu, &ff, Arc::new(rec), 108, 192).expect("the dummy's frames");
        assert_eq!(frames.nf(), NF);
        let late = frames.frame(900);
        let early = frames.frame(30);
        assert_eq!((late.w, late.h), (108, 192));
        assert_eq!(late.data.len(), 108 * 192 * 3);
        assert_ne!(late.data, early.data, "two moments of the fight differ");
        // the same frame again is the same picture: nothing of one frame leaks into the next
        assert!(frames.frame(900).data == late.data);
    }

    /// The UI layer is on every frame at any size: the VS card fills the first frames whole, over the fight.
    #[test]
    fn frames2_draw_the_ui_layer() {
        let Ok(gpu) = Gpu::headless() else {
            eprintln!("no GPU here: skipped");
            return;
        };
        let (ff, rec) = dummy();
        let mut frames = Frames2::new(&gpu, &ff, Arc::new(rec), 108, 192).expect("the dummy's frames");
        let f = frames.scene().card.cover_frame();
        let shown = frames.frame(f);
        let ui = frames.with_ui(|ui, _| ui.draw(f).0);
        assert!(ui.rgba.iter().all(|p| p[3] == 255), "the VS card is opaque");
        let want = fit(ui, (108, 192));
        let worst = shown
            .data
            .as_chunks::<3>()
            .0
            .iter()
            .zip(&want.rgba)
            .map(|(a, b)| {
                (0..3)
                    .map(|c| (i32::from(a[c]) - i32::from(b[c])).abs())
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        assert!(worst <= 1, "the frame is the VS card ({worst})");
    }

    #[test]
    fn fit_is_exact_on_whole_multiples_and_nearest_otherwise() {
        let mut ov = Overlay::empty(2, 2);
        ov.rgba = vec![[1, 1, 1, 1], [2, 2, 2, 2], [3, 3, 3, 3], [4, 4, 4, 4]];
        assert_eq!(
            fit(ov.clone(), (6, 6)),
            ov,
            "a whole multiple is the GPU pass's own scale"
        );
        let f = fit(ov.clone(), (3, 3));
        assert_eq!((f.w, f.h), (3, 3));
        let col = |p: &[u8; 4]| p[0];
        assert_eq!(f.rgba.iter().map(col).collect::<Vec<_>>(), [1, 1, 2, 1, 1, 2, 3, 3, 4]);
    }

    #[test]
    fn ko_window_is_in_video_time() {
        let (ff, rec) = dummy();
        let scene = Scene2::new(&ff, Arc::new(rec)).expect("the dummy's scene");
        let r = ko_check(&ff, &scene).expect("the dummy's beat sheet");
        let t = scene.directed.map.ko_video_t().expect("the dummy's seed K.O.s");
        let (a, b) = sheet_of(&ff).expect("the dummy's beat sheet").t_ko;
        // the K.O. after slow-mo, not the fight clock's
        assert!(t >= scene.rec.outcome.t_ko.unwrap());
        assert!((r.numbers["t_ko"] - t).abs() < 1e-12);
        assert!(r.summary.contains(&format!("{t:.2}")), "{}", r.summary);
        assert_eq!(r.pass, (a..=b).contains(&t), "{}", r.summary);
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
