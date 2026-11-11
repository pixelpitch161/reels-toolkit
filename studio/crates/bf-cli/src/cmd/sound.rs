//! `ballfight sound <fight.toml> --out audio.wav [--stems DIR] [--seed N | --rec F]`: build the soundtrack and save
//! the master as a 16-bit stereo .wav (48 kHz, 2,928,640 frames, as the video holds it); `--stems` also saves each
//! bus as `<bus>.wav` in that folder (made if missing; it must be empty).

use std::path::{Path, PathBuf};

use bf_sound::wav::wav16_bytes;
use serde_json::json;

use crate::{Ctx, Done, Fail, load_fight, need_ko, new_file, note, say, say_json, the_fight, write_new};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The .wav to write (a new file).
    #[arg(long, value_name = "FILE")]
    out: PathBuf,
    /// Also save each bus as a .wav in this folder (made if missing; it must be empty).
    #[arg(long, value_name = "DIR")]
    stems: Option<PathBuf>,
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
    if let Some(dir) = &a.stems {
        empty_folder(dir)?;
    }
    let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
    need_ko(&rec)?;
    note("sound: building the soundtrack");
    let st = bf_content::build_soundtrack(&rec, &ff).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
    write_new(&a.out, &wav16_bytes(&st.master))?;
    let mut stems = Vec::new();
    if let Some(dir) = &a.stems {
        std::fs::create_dir_all(dir)
            .map_err(|e| Fail(format!("the stems folder {} cannot be made ({e}).", dir.display())))?;
        for (bus, x) in &st.stems {
            let path = dir.join(format!("{bus}.wav"));
            write_new(&path, &wav16_bytes(x))?;
            stems.push(path.display().to_string());
        }
    }
    if ctx.json {
        say_json(&json!({
            "seed": rec.seed,
            "frames": st.master.len(),
            "out": a.out.display().to_string(),
            "stems": stems,
        }));
    } else {
        say(&format!("soundtrack of seed {}: saved {}", rec.seed, a.out.display()));
        for s in &stems {
            say(&format!("saved {s}"));
        }
    }
    Ok(())
}

/// Refuses a stems folder that is a file or holds anything.
fn empty_folder(dir: &Path) -> Done {
    match std::fs::symlink_metadata(dir) {
        Err(_) => Ok(()),
        Ok(m) if !m.is_dir() => Err(Fail(format!(
            "the stems folder {} is not a folder (a file has that name); choose another.",
            dir.display()
        ))),
        Ok(_) => {
            let full = std::fs::read_dir(dir)
                .map_err(|e| Fail(format!("the stems folder {} cannot be read ({e}).", dir.display())))?
                .next()
                .is_some();
            if full {
                return Err(Fail(format!(
                    "the stems folder {} is not empty, and ballfight never replaces a file; choose an empty or a \
                     new folder.",
                    dir.display()
                )));
            }
            Ok(())
        }
    }
}
