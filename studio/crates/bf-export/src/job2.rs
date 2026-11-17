//! Engine 2's render job: a silent soundtrack (E5 brings sound), the K.O. window check, every frame drawn by
//! `bf2-render` on one GPU thread (the GPU is the parallelism) and encoded at 1080 x 1920, the finished file's
//! checks, then the folder moved into place under the same names and rules as the current engine's job
//! ([`crate::job`]).
//!
//! Stages, in order ([`JobEvent::Stage`]): [`SILENCE`], [`stage::CHECKS`] (a whole video only), [`stage::FRAMES`],
//! [`stage::ENCODE`], [`stage::VERIFY`], [`stage::SAVE`]. Cancel is looked at between the stages and before every
//! frame; it ends ffmpeg and leaves nothing behind.
//!
//! The checks of the current engine's picture and sound (focus, safe zone, levels, band, loudness, the flash
//! limits) are not run: they are E5's. The loop checks of the finished file are reported but do not stop the job
//! until E4 gives Engine 2 its ending (an Engine 2 fight does not yet end where it began).

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use bf_content::fighters2;
use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_picture::config::{FPS, NF};
use bf_sound::NA;
use bf_sound::wav::wav16_bytes;
use bf2_render::{Gpu, Look2, Renderer, Stage, render_frame};
use bf2_sim::Recording2;

use crate::checks::CheckResult;
use crate::encode::{ENCODING_NOTE, Encoder, Ffmpeg};
use crate::job::{JobEvent, JobOpts, stage};
use crate::output::{TempFolder, caption_text, clip_folder_name, folder_stem, png_bytes, prepare_root, taken};
use crate::probe::verify_until;
use crate::{ExportError, output};

/// Engine 2's first stage: the silent soundtrack is written.
pub const SILENCE: &str = "silence";
/// The last moment a K.O. may come (video seconds): the video needs its last three seconds after it.
pub const KO_BY: f64 = 58.0;
/// Engine 2's frames.
pub const W2: usize = bf2_render::W as usize;
pub const H2: usize = bf2_render::H as usize;

/// The checks of the finished file that are reported but do not stop an Engine 2 job yet.
const ADVISORY: [&str; 2] = ["mp4_loop_picture", "mp4_loop_audio"];
/// The finished file's checks not run on Engine 2 at all (no flash cap on Engine 2, Aedan 2026-11-17).
const NOT_RUN: [&str; 1] = ["mp4_flash"];

/// The K.O. window: a K.O. after the fight starts and by [`KO_BY`].
pub fn ko_window(rec: &Recording2) -> CheckResult {
    const TITLE: &str = "K.O. window";
    match rec.outcome.t_ko {
        Some(t) if t >= rec.t_fight && t <= KO_BY => CheckResult::new(
            "ko_window",
            true,
            format!("{TITLE}: the K.O. comes at {t:.2} s, by {KO_BY:.0} s."),
            &[("t_ko", t)],
        ),
        Some(t) => CheckResult::new(
            "ko_window",
            false,
            format!(
                "{TITLE}: the K.O. comes at {t:.2} s, after {KO_BY:.0} s, so the video has no ending; choose another seed."
            ),
            &[("t_ko", t)],
        ),
        None => CheckResult::new(
            "ko_window",
            false,
            format!("{TITLE}: this fight has no K.O., so it has no video; choose another seed."),
            &[],
        ),
    }
}

/// A fighter's colour (sRGB bytes) in linear light.
pub fn linear(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| (f32::from(v) / 255.0).powf(2.2))
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
    let mut colours = [[0.0; 3]; 2];
    for (k, name) in names.iter().enumerate() {
        let (Some(def), Some(look)) = (fighters2::def(name), fighters2::look(name)) else {
            return Err(ExportError::Content(format!(
                "This fight cannot be rendered: {name} has no Engine 2 look yet."
            )));
        };
        colours[k] = linear(def.colour);
        looks.push(look);
    }
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
        looks,
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
    looks: [Box<dyn Look2>; 2],
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
        on(JobEvent::Stage(SILENCE));
        let wav = tmp.write("audio.wav", &wav16_bytes(&vec![[0.0; 2]; NA]))?;
        stop()?;
        if self.whole {
            on(JobEvent::Stage(stage::CHECKS));
            report2(vec![ko_window(self.rec)], on)?;
            stop()?;
        }

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
        let mut draw = |f: usize| render_frame(&mut r, &gpu, &stage_set, &self.looks, self.rec, f, self.colours);

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
            // The cover: the K.O. frame (E4's director picks a better one).
            let f = self
                .rec
                .outcome
                .t_ko
                .map_or(NF / 2, |t| ((t * f64::from(FPS)).round() as usize).min(NF - 1));
            let cover = draw(f);
            tmp.write("cover.png", &png_bytes(&cover))?;
            tmp.write("caption.txt", caption_text(self.ff).as_bytes())?;
            // the fight file last: a folder with it is a finished video
            tmp.write(&format!("{name}.toml"), self.ff.to_toml().as_bytes())?;
        }
        stop()
    }
}
