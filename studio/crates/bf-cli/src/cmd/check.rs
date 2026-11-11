//! `ballfight check <fight.toml> [--seed N | --rec F]`: every check a video must pass before it is rendered
//! (focus, safe zone, levels, band, loudness, flashes, the loop's picture and sound), on the soundtrack and the
//! frames the render would make. Exit code 1 when one fails.

use std::path::PathBuf;
use std::sync::Arc;

use bf_export::{BandPlan, CheckResult, pre_render};
use bf_story::{BeatSheet, story};
use serde_json::{Value, json};

use crate::{Ctx, Done, Fail, load_fight, need_ko, note, num, say, say_json, the_fight};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// The seed to fight (default: the fight file's).
    #[arg(long, value_name = "N", conflicts_with = "rec")]
    seed: Option<u64>,
    /// Read this recording instead of fighting the seed.
    #[arg(long, value_name = "FILE")]
    rec: Option<PathBuf>,
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let ff = load_fight(&a.fight)?;
    let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, true)?;
    need_ko(&rec)?;
    let content = |e: bf_content::ContentError| Fail(format!("{}: {e}.", a.fight.display()));
    note("check: building the soundtrack");
    let st = bf_content::build_soundtrack(&rec, &ff).map_err(content)?;
    let rec = Arc::new(rec);
    let pic = bf_content::build_picture(Arc::clone(&rec), &ff).map_err(content)?;
    let cfg = &pic.ctx().cfg;
    let show = &pic.ctx().show;
    let sheet = BeatSheet::series([&cfg[0], &cfg[1]]);
    let st_story = story(&rec, &show.grid, &sheet, [&cfg[0].low.name, &cfg[1].low.name]);
    let plan = BandPlan::of(&ff).map_err(content)?;
    note(&format!("check: drawing and measuring on {} threads", ctx.workers));
    let results = pre_render(&pic, &st, show, &st_story, &plan, ctx.workers, ctx.cancel)
        .ok_or_else(|| Fail::new("the checks were stopped (Ctrl+C)."))?;
    let failed = results.iter().find(|r| !r.pass).cloned();
    if ctx.json {
        say_json(&json!({
            "seed": rec.seed,
            "pass": failed.is_none(),
            "checks": results.iter().map(check_json).collect::<Vec<_>>(),
        }));
    } else {
        for r in &results {
            say(&check_line(r));
        }
    }
    match failed {
        Some(r) => Err(Fail(format!("the {} check failed: {}", r.name, r.summary))),
        None => Ok(()),
    }
}

/// A check's result as JSON.
pub fn check_json(r: &CheckResult) -> Value {
    let numbers: serde_json::Map<String, Value> = r.numbers.iter().map(|(k, v)| (k.clone(), num(*v))).collect();
    json!({"name": r.name, "pass": r.pass, "summary": r.summary, "numbers": numbers})
}

/// A check's result as a line of text.
pub fn check_line(r: &CheckResult) -> String {
    format!("{} {}: {}", if r.pass { "pass" } else { "FAIL" }, r.name, r.summary)
}
