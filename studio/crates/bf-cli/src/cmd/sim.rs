//! `ballfight sim <fight.toml> [--seed N] --out <file.bfrec>`: fight one seed and save its recording.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{Ctx, Done, load_fight, new_file, say, say_json, secs, seed_of, write_new_with};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The seed to fight (default: the fight file's).
    #[arg(long, value_name = "N")]
    seed: Option<u64>,
    /// The recording to write (a new file).
    #[arg(long, value_name = "FILE")]
    out: PathBuf,
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let ff = load_fight(&a.fight)?;
    let seed = seed_of(&ff, &a.fight, a.seed)?;
    new_file(&a.out)?;
    let rec = crate::fight(&ff, &a.fight, seed, true)?;
    write_new_with(&a.out, |tmp: &Path| rec.write(tmp).map_err(|e| e.to_string()))?;
    let winner = rec.winner.and_then(|w| rec.names.get(w)).cloned();
    if ctx.json {
        say_json(&json!({
            "seed": seed,
            "winner": rec.winner,
            "winner_name": winner,
            "t_ko": rec.t_ko,
            "events": rec.events.len(),
            "frames": rec.frames(),
            "hash": rec.hash(),
            "out": a.out.display().to_string(),
        }));
    } else {
        let outcome = match (&winner, rec.t_ko) {
            (Some(w), Some(t)) => format!("{w} wins by K.O. at {}", secs(t)),
            _ => "no K.O.".to_string(),
        };
        say(&format!(
            "seed {seed}: {outcome}, {} events, hash {}; saved {}",
            rec.events.len(),
            rec.hash(),
            a.out.display()
        ));
    }
    Ok(())
}
