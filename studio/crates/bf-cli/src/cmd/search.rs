//! `ballfight search <fight.toml> [--from 1] [--stop 200] [--max N] [--out shortlist.json]`: fight the seeds from
//! `--from` upward until `--stop` of them fit the series (or `--max` seeds are checked), and list the best.
//!
//! The list is the Studio's shortlist: every fit, then the best 200 of the others, each group by story score
//! downward, then by seed. `--out` saves it as JSON (the same value `--json` prints).

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use bf_sim::Setup;
use bf_story::{BeatSheet, Candidate, SearchEnd, SearchOpts, SearchProgress, search};
use serde_json::{Value, json};

use crate::cmd::story::story_json;
use crate::{Ctx, Done, Fail, fighters, json_text, load_fight, new_file, note, say, say_json, secs, write_new};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file (its own seed is not used).
    fight: PathBuf,
    /// The first seed.
    #[arg(long, value_name = "N", default_value_t = 1)]
    from: u64,
    /// Stop when this many seeds fit.
    #[arg(long, value_name = "N", default_value_t = 200)]
    stop: usize,
    /// Check at most this many seeds (default: no limit; Ctrl+C stops the search and shows what it found).
    #[arg(long, value_name = "N")]
    max: Option<u64>,
    /// Save the shortlist as JSON in this new file.
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,
}

/// How many candidates the text output lists.
const SHOWN: usize = 10;

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let ff = load_fight(&a.fight)?;
    if let Some(out) = &a.out {
        new_file(out)?;
    }
    let cfg = fighters(&ff)?;
    let sheet = BeatSheet::series(cfg);
    let base =
        bf_sim::setup_from(&ff, bf_content::roster(), 0).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
    let setup_for = |seed: u64| Setup { seed, ..base.clone() };
    let opts = SearchOpts {
        from: a.from,
        stop_fits: a.stop,
        max_seeds: a.max,
        workers: ctx.workers,
    };
    let last_note = Mutex::new(Instant::now());
    let progress = |p: SearchProgress| {
        let mut last = last_note.lock().unwrap_or_else(|e| e.into_inner());
        if last.elapsed() >= Duration::from_secs(1) {
            *last = Instant::now();
            note(&format!("search: {} seeds checked, {} fit", p.checked, p.fits));
        }
    };
    let t0 = Instant::now();
    let found = search(&setup_for, &sheet, &opts, &progress, ctx.cancel);
    let fits = found.candidates.iter().filter(|c| c.verdict.fits).count();
    let end = match &found.end {
        SearchEnd::Failed(why) => return Err(Fail(format!("{}: {why}.", a.fight.display()))),
        SearchEnd::Full => "full",
        SearchEnd::OutOfSeeds => "out_of_seeds",
        SearchEnd::Stopped => "stopped",
    };
    note(&format!(
        "search: {} seeds checked from seed {} in {:.1} s, {fits} fit",
        found.checked,
        a.from,
        t0.elapsed().as_secs_f64()
    ));
    let names = &ff.fighters;
    let list: Vec<Value> = found.candidates.iter().map(|c| candidate_json(c, names)).collect();
    let value = json!({
        "fight": a.fight.display().to_string(),
        "from": a.from,
        "checked": found.checked,
        "end": end,
        "fits": fits,
        "best": list.first().cloned().unwrap_or(Value::Null),
        "candidates": list,
    });
    if let Some(out) = &a.out {
        write_new(out, format!("{}\n", json_text(&value)).as_bytes())?;
    }
    if ctx.json {
        say_json(&value);
    } else {
        say("seed       fits  score  winner     K.O.      lead changes");
        for c in found.candidates.iter().take(SHOWN) {
            let st = &c.story;
            let winner = st.winner.and_then(|w| names.get(w)).map_or("-", String::as_str);
            say(&format!(
                "{:<10} {:<5} {:<6.2} {:<10} {:<9} {}",
                c.seed,
                if c.verdict.fits { "yes" } else { "no" },
                c.verdict.score,
                winner,
                st.t_ko.map_or("-".to_string(), secs),
                st.lead_changes
            ));
        }
        match found.candidates.first() {
            Some(best) if best.verdict.fits => say(&format!(
                "best: seed {} (fits, story score {:.2})",
                best.seed, best.verdict.score
            )),
            _ => say("best: none of the seeds checked fits the series"),
        }
        if let Some(out) = &a.out {
            say(&format!("saved {}", out.display()));
        }
    }
    if ctx.cancel.load(Ordering::SeqCst) || found.end == SearchEnd::Stopped {
        return Err(Fail(format!(
            "the search was stopped (Ctrl+C) after {} seeds; what it found is shown above.",
            found.checked
        )));
    }
    Ok(())
}

/// A candidate as a JSON object: its story and verdict.
fn candidate_json(c: &Candidate, names: &[String]) -> Value {
    story_json(&c.story, &c.verdict, names)
}
