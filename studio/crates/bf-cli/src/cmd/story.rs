//! `ballfight story <fight.toml> [--seed N | --rec F]`: the story of one fight and whether it fits the series.

use std::path::PathBuf;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_story::{BeatSheet, Story, Verdict, score, story};
use serde_json::{Value, json};

use crate::{Ctx, Done, Fail, fighters, load_fight, num, say, say_json, secs, stopped, the_fight};

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
    let rec = the_fight(&ff, &a.fight, a.rec.as_deref(), a.seed, false)?;
    stopped(ctx, "the story")?;
    let (st, verdict) = judge(&ff, &rec)?;
    if ctx.json {
        say_json(&story_json(&st, &verdict, &ff.fighters));
    } else {
        for line in story_lines(&st, &verdict, &ff.fighters) {
            say(&line);
        }
    }
    Ok(())
}

/// The story of `rec` under the series' beat sheet for the fight file's two fighters, and its verdict.
pub fn judge(ff: &FightFile, rec: &Recording) -> Result<(Story, Verdict), Fail> {
    let cfg = fighters(ff)?;
    let sheet = BeatSheet::series(cfg);
    let st = story(rec, &ff.grid(), &sheet, [&cfg[0].low.name, &cfg[1].low.name]);
    let verdict = score(&st, &sheet);
    Ok((st, verdict))
}

/// The name of fighter `k`.
fn name(names: &[String], k: usize) -> &str {
    names.get(k).map_or("?", String::as_str)
}

/// A story and its verdict as a JSON object.
pub fn story_json(st: &Story, v: &Verdict, names: &[String]) -> Value {
    let firsts: Vec<Value> = st
        .firsts
        .iter()
        .map(|(t, who, ability)| json!({"t": num(*t), "fighter": *who, "fighter_name": name(names, *who), "ability": ability}))
        .collect();
    json!({
        "seed": st.seed,
        "fighters": names,
        "winner": st.winner,
        "winner_name": st.winner.map(|w| name(names, w)),
        "t_ko": st.t_ko.map(num),
        "lead_changes": st.lead_changes,
        "lead_change_times": st.lc_t.iter().map(|t| num(*t)).collect::<Vec<_>>(),
        "winner_hp": num(st.winner_hp),
        "max_gap": num(st.max_gap),
        "firsts": firsts,
        "lows": st.lows,
        "fits": v.fits,
        "score": num(v.score),
        "misses": v.misses,
    })
}

/// A story and its verdict in lines of text.
pub fn story_lines(st: &Story, v: &Verdict, names: &[String]) -> Vec<String> {
    let mut out = vec![format!(
        "{} against {}, seed {}",
        name(names, 0),
        name(names, 1),
        st.seed
    )];
    out.push(match (st.winner, st.t_ko) {
        (Some(w), Some(t)) => format!(
            "{} wins by K.O. at {} with {:.1} HP left",
            name(names, w),
            secs(t),
            st.winner_hp
        ),
        _ => "no K.O.: the fight runs out of time".to_string(),
    });
    let times: Vec<String> = st.lc_t.iter().map(|t| secs(*t)).collect();
    out.push(if times.is_empty() {
        format!("lead changes: {}", st.lead_changes)
    } else {
        format!("lead changes: {} (at {})", st.lead_changes, times.join(", "))
    });
    out.push(format!("longest time without a hit: {}", secs(st.max_gap)));
    let firsts: Vec<String> = st
        .firsts
        .iter()
        .map(|(t, who, ability)| format!("{} {} {ability}", secs(*t), name(names, *who)))
        .collect();
    out.push(format!(
        "first abilities: {}",
        if firsts.is_empty() {
            "none".to_string()
        } else {
            firsts.join("; ")
        }
    ));
    out.push(if v.fits {
        format!("fits the series: yes, story score {:.2}", v.score)
    } else {
        format!(
            "fits the series: no ({}), story score {:.2}",
            v.misses.join("; "),
            v.score
        )
    });
    out
}
