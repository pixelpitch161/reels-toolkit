//! `ballfight hash <fight.toml> --seeds 1,2,3`: one line per seed, `left right seed winner t_ko n_events hash`, as
//! `studio/replay/hashes.txt` writes them (the winner as the fighter's index, `-` for none; the hash is the
//! recording's content hash over every event and every per-frame value). Two PCs or two builds that give the same
//! lines fight the same fights.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{Ctx, Done, load_fight, num, say, say_json, stopped};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file (its own seed is not used).
    fight: PathBuf,
    /// The seeds, separated by commas: 1,2,3.
    #[arg(long, value_name = "LIST", value_parser = seed_list)]
    seeds: Seeds,
}

#[derive(Clone)]
pub struct Seeds(Vec<u64>);

fn seed_list(text: &str) -> Result<Seeds, String> {
    let seeds = text
        .split(',')
        .map(|s| s.trim().parse::<u64>())
        .collect::<Result<Vec<u64>, _>>()
        .map_err(|_| "give whole numbers from 0 up, separated by commas, such as 1,2,3".to_string())?;
    Ok(Seeds(seeds))
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let ff = load_fight(&a.fight)?;
    let mut fights: Vec<Value> = Vec::new();
    for &seed in &a.seeds.0 {
        stopped(ctx, "the hashing")?;
        let rec = crate::fight(&ff, &a.fight, seed, true)?;
        let or_dash = |x: Option<String>| x.unwrap_or_else(|| "-".to_string());
        if ctx.json {
            fights.push(json!({
                "seed": seed,
                "winner": rec.winner,
                "t_ko": rec.t_ko.map(num),
                "events": rec.events.len(),
                "hash": rec.hash(),
            }));
        } else {
            say(&format!(
                "{} {} {seed} {} {} {} {}",
                ff.fighters[0],
                ff.fighters[1],
                or_dash(rec.winner.map(|w| w.to_string())),
                or_dash(rec.t_ko.map(|t| t.to_string())),
                rec.events.len(),
                rec.hash()
            ));
        }
    }
    if ctx.json {
        say_json(&json!({"fighters": ff.fighters, "fights": fights}));
    }
    Ok(())
}
