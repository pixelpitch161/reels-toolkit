//! `ballfight render <fight.toml> --out <folder> [--seed N | --rec F] [--frames A:B] [--encode best|faster]`: render
//! the video through the render job (`bf_export::run_job`) into a new folder inside `--out`, and inherit its rules:
//! the folder is `<a>_vs_<b>` (or `<a>_vs_<b>_seed<N>` when that is taken; both taken is refused before any work),
//! nothing is ever replaced, and a failure or Ctrl+C leaves nothing behind. `--frames A:B` renders a clip of frames
//! A to B-1 (a folder `<a>_vs_<b>_frames<A>-<B>` holding only the mp4; the whole-video checks are left out).
//! `--encode faster` encodes with the Faster setting (a faster x264 preset, nothing else changed); its folder also
//! holds `encoding.txt`, saying so. The default, `best`, is the series' own encode, byte for byte.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_export::{CheckResult, EncodeSpeed, ExportError, JobEvent, JobOpts, run_job};
use bf_picture::config::NF;
use serde_json::json;

use crate::cmd::check::{check_json, check_line};
use crate::{Ctx, Done, Fail, encode_speed, load_fight, need_ko, note, say, say_json, seed_of, the_fight};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The output folder; the video's own folder is made inside it.
    #[arg(long, value_name = "FOLDER")]
    out: PathBuf,
    /// The seed to fight (default: the fight file's); the fight file saved beside the video names it.
    #[arg(long, value_name = "N", conflicts_with = "rec")]
    seed: Option<u64>,
    /// Render this recording instead of fighting the seed.
    #[arg(long, value_name = "FILE")]
    rec: Option<PathBuf>,
    /// The arrangement to play: classic or composed (default: the fight file's); the saved fight file names it.
    #[arg(long, value_name = "NAME", value_parser = crate::arrangement)]
    arrangement: Option<String>,
    /// The composer's seed (default: the fight file's music seed); the saved fight file names it.
    #[arg(long, value_name = "N")]
    music_seed: Option<u64>,
    /// Only frames A to B-1 (a clip for a quick look), for example 0:90.
    #[arg(long, value_name = "A:B", value_parser = clip)]
    frames: Option<Range<usize>>,
    /// The encoder setting: best (the series' own, the default) or faster (a quicker encode, a larger file).
    #[arg(long, value_name = "SETTING", default_value = "best", value_parser = encode_speed)]
    pub encode: EncodeSpeed,
}

/// `A:B` as a stretch of the video's frames.
fn clip(text: &str) -> Result<Range<usize>, String> {
    let bad = || format!("give A:B, the first frame and the frame after the last, with A below B and B at most {NF}");
    let (a, b) = text.split_once(':').ok_or_else(bad)?;
    let (a, b): (usize, usize) = (
        a.trim().parse().map_err(|_| bad())?,
        b.trim().parse().map_err(|_| bad())?,
    );
    if a < b && b <= NF {
        Ok(a..b)
    } else {
        Err(format!("{text} is not a stretch of the video's {NF} frames: {}", bad()))
    }
}

/// The job's failure as the command says it.
fn fail(e: ExportError) -> Fail {
    match e {
        ExportError::Cancelled(why) => {
            let mut why = why;
            if let Some(c) = why.get(..1) {
                why = c.to_lowercase() + &why[1..];
            }
            Fail(format!("the render was stopped (Ctrl+C): {why}"))
        }
        e => Fail(e.to_string()),
    }
}

/// Fights and renders `ff`: an Engine 2 file through `bf_engine::render` (fought from its seed), a current-engine
/// file exactly as before (the recording or the seed, the K.O. it needs, then `bf_export::run_job`). Sets the
/// fight file's seed to the one fought.
fn route(
    ff: &mut FightFile,
    a: &Args,
    opts: &JobOpts,
    on: &(dyn Fn(JobEvent) + Sync),
    cancel: &AtomicBool,
) -> Result<PathBuf, Fail> {
    if ff.engine == Engine::Two {
        if a.rec.is_some() {
            return Err(Fail(
                "--rec reads current-engine recordings; an Engine 2 fight is rendered from its seed.".into(),
            ));
        }
        let seed = seed_of(ff, &a.fight, a.seed)?;
        ff.seed = Some(seed);
        return bf_engine::render(ff, seed, opts, on, cancel).map_err(|e| match e {
            bf_engine::EngineError::Export(e) => fail(e),
            e => Fail(format!("{}: seed {seed}: {e}.", a.fight.display())),
        });
    }
    let rec = the_fight(ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
    match ff.seed {
        Some(s) if s != rec.seed => {
            return Err(Fail(format!(
                "the recording is the fight of seed {}, but the fight file says seed {s}, so they do not belong \
                 together.",
                rec.seed
            )));
        }
        _ => ff.seed = Some(rec.seed),
    }
    need_ko(&rec)?;

    run_job(ff, Arc::new(rec), opts, on, cancel).map_err(fail)
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let mut ff = load_fight(&a.fight)?;
    crate::music_flags(&mut ff, &a.fight, a.arrangement.as_ref(), a.music_seed)?;
    if std::fs::metadata(&a.out).is_ok_and(|m| !m.is_dir()) {
        return Err(Fail(format!(
            "the output folder {} is not a folder (a file has that name), so nothing was made; choose another.",
            a.out.display()
        )));
    }
    if let Some(seed) = a.seed {
        ff.seed = Some(seed);
        bf_content::validate(&ff).map_err(|e| Fail(format!("--seed {seed}: {e}.")))?;
    }
    let opts = JobOpts {
        out_root: a.out.clone(),
        workers: ctx.workers,
        frames: a.frames.clone(),
        speed: a.encode,
    };
    let checks: Mutex<Vec<CheckResult>> = Mutex::new(Vec::new());
    let last = Mutex::new(Instant::now());
    let on = |ev: JobEvent| match ev {
        JobEvent::Stage(s) => note(&format!("render: {s}")),
        JobEvent::Progress { done, total, seconds } => {
            let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
            if done == total || last.elapsed() >= Duration::from_secs(2) {
                *last = Instant::now();
                note(&format!("render: frame {done} of {total} ({seconds:.1} s)"));
            }
        }
        JobEvent::Check(r) => {
            note(&format!("render: {}", check_line(&r)));
            checks.lock().unwrap_or_else(|e| e.into_inner()).push(r);
        }
        JobEvent::Done(_) => {}
    };
    let cancel: &AtomicBool = ctx.cancel;
    let folder = route(&mut ff, a, &opts, &on, cancel)?;
    if cancel.load(Ordering::SeqCst) {
        note("render: Ctrl+C came after the video was saved");
    }
    let checks = checks.into_inner().unwrap_or_else(|e| e.into_inner());
    if ctx.json {
        say_json(&json!({
            "folder": folder.display().to_string(),
            "seed": ff.seed,
            "encode": a.encode.name(),
            "checks": checks.iter().map(check_json).collect::<Vec<_>>(),
        }));
    } else if a.encode == EncodeSpeed::Best {
        say(&format!("saved {}", folder.display()));
    } else {
        say(&format!(
            "saved {} (encoded with the {} setting, x264 preset {})",
            folder.display(),
            a.encode.name(),
            a.encode.preset()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_routes_engine2_files() {
        let fight = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fights/engine2/test_dummy.toml"
        ));
        let mut ff = load_fight(&fight).expect("the dummy's file loads");
        let out = std::env::temp_dir().join(format!("ballfight render route {}", std::process::id()));
        let a = Args {
            fight: fight.clone(),
            out: out.clone(),
            seed: None,
            rec: None,
            arrangement: None,
            music_seed: None,
            frames: Some(600..602),
            encode: EncodeSpeed::Faster,
        };
        let opts = JobOpts {
            out_root: out.clone(),
            workers: 1,
            frames: a.frames.clone(),
            speed: a.encode,
        };
        let cancel = AtomicBool::new(false);
        let first = Mutex::new(None);
        // The first event names the job's first stage; then the job is stopped.
        let on = |e: JobEvent| {
            let mut f = first.lock().unwrap();
            if f.is_none() {
                *f = Some(e);
            }
            cancel.store(true, Ordering::SeqCst);
        };
        let r = route(&mut ff, &a, &opts, &on, &cancel);
        assert!(r.is_err(), "stopped after its first event");
        assert_eq!(
            first.into_inner().unwrap(),
            Some(JobEvent::Stage(bf_export::stage::SOUNDTRACK)),
            "an Engine 2 file reaches run_job2"
        );
        assert_eq!(ff.seed, Some(576));
        let _ = std::fs::remove_dir_all(&out);
    }
}
