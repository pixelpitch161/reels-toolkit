//! `ballfight render <fight.toml> --out <folder> [--seed N | --rec F] [--frames A:B]`: render the video through
//! the render job (`bf_export::run_job`) into a new folder inside `--out`, and inherit its rules: the folder is
//! `<a>_vs_<b>` (or `<a>_vs_<b>_seed<N>` when that is taken; both taken is refused before any work), nothing is
//! ever replaced, and a failure or Ctrl+C leaves nothing behind. `--frames A:B` renders a clip of frames A to B-1
//! (a folder `<a>_vs_<b>_frames<A>-<B>` holding only the mp4; the whole-video checks are left out).

use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bf_export::{CheckResult, JobEvent, JobOpts, run_job};
use bf_picture::config::NF;
use serde_json::json;

use crate::cmd::check::{check_json, check_line};
use crate::{Ctx, Done, Fail, load_fight, need_ko, note, say, say_json, the_fight};

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
    /// Only frames A to B-1 (a clip for a quick look), for example 0:90.
    #[arg(long, value_name = "A:B", value_parser = clip)]
    frames: Option<Range<usize>>,
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

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let mut ff = load_fight(&a.fight)?;
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
    let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
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

    let opts = JobOpts {
        out_root: a.out.clone(),
        workers: ctx.workers,
        frames: a.frames.clone(),
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
    let folder = run_job(&ff, Arc::new(rec), &opts, &on, cancel).map_err(|e| Fail(e.to_string()))?;
    if cancel.load(Ordering::SeqCst) {
        note("render: Ctrl+C came after the video was saved");
    }
    let checks = checks.into_inner().unwrap_or_else(|e| e.into_inner());
    if ctx.json {
        say_json(&json!({
            "folder": folder.display().to_string(),
            "seed": ff.seed,
            "checks": checks.iter().map(check_json).collect::<Vec<_>>(),
        }));
    } else {
        say(&format!("saved {}", folder.display()));
    }
    Ok(())
}
