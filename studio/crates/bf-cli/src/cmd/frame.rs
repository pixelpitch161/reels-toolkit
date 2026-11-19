//! `ballfight frame <fight.toml> --frame N --out f.png [--up] [--seed N | --rec F]`: draw one frame of the video
//! (360 x 640, or 1080 x 1920 with `--up`) as a PNG.

use std::path::PathBuf;
use std::sync::Arc;

use bf_export::output::png_bytes;
use bf_picture::config::NF;
use bf_picture::ring::upscale3;
use serde_json::json;

use crate::{Ctx, Done, Fail, load_fight, need_ko, new_file, say, say_json, stopped, the_fight, write_new};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The frame, from 0 (the video has 1,830).
    #[arg(long, value_name = "N", value_parser = clap::value_parser!(u64).range(0..NF as u64))]
    frame: u64,
    /// The PNG to write (a new file).
    #[arg(long, value_name = "FILE")]
    out: PathBuf,
    /// Write it at the video's size, 1080 x 1920 (three times each way).
    #[arg(long)]
    up: bool,
    /// The seed to fight (default: the fight file's).
    #[arg(long, value_name = "N", conflicts_with = "rec")]
    seed: Option<u64>,
    /// Read this recording instead of fighting the seed.
    #[arg(long, value_name = "FILE")]
    rec: Option<PathBuf>,
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let ff = load_fight(&a.fight)?;
    new_file(&a.out)?;
    let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
    need_ko(&rec)?;
    stopped(ctx, "the frame")?;
    let seed = rec.seed;
    let pic =
        bf_content::build_picture(Arc::new(rec), &ff).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
    stopped(ctx, "the frame")?;
    let f = usize::try_from(a.frame).unwrap_or(0);
    let (img, _, report) = pic.frame_reported(f);
    if report.hull_errors != 0 {
        return Err(Fail(format!(
            "frame {f} cannot be drawn the way the reference does ({} of its shapes were refused), so it was not \
             written.",
            report.hull_errors
        )));
    }
    let img = if a.up { upscale3(&img) } else { img };
    write_new(&a.out, &png_bytes(&img))?;
    if ctx.json {
        say_json(&json!({
            "seed": seed,
            "frame": f,
            "width": img.w,
            "height": img.h,
            "out": a.out.display().to_string(),
        }));
    } else {
        say(&format!("frame {f} of seed {seed}: saved {}", a.out.display()));
    }
    Ok(())
}
