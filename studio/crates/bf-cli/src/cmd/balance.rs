//! `ballfight balance <fight.toml> [--write]`: find the damage handicap that makes the pairing even (the left
//! fighter wins 40 to 60 % of the decided fights, median K.O. 49 to 52 s), on the balance seeds from `1 << 40`.
//!
//! Fights are never edited to make them even: only the handicap changes. `--write` stores it in the fight file and
//! changes nothing else there: the `handicap = [..]` value is replaced in the file's own text, so comments, the
//! order of the keys and every other line stay as they were. The new text is read back and must be the same fight
//! file with the new handicap before it is written (through a temporary file beside it, then a rename). A file
//! whose handicap is not written on one line of its own (or not at all) is refused before the balance runs: the
//! rest of the file is never rewritten.

use std::path::{Path, PathBuf};
use std::time::Instant;

use bf_core::fightfile::FightFile;
use bf_sim::Setup;
use bf_story::{BalanceProgress, BalanceRound, Halt, Stats, auto_balance};
use serde_json::{Value, json};

use crate::{Ctx, Done, Fail, fighters, load_fight_text, note, num, say, say_json, temp_beside, write_whole};

#[derive(clap::Args)]
pub struct Args {
    /// The fight file.
    fight: PathBuf,
    /// Store the handicap found in the fight file.
    #[arg(long)]
    write: bool,
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let (ff, text) = load_fight_text(&a.fight)?;
    if a.write && set_handicap(&text, [1.0, 1.0]).is_none() {
        return Err(one_line_handicap(&a.fight));
    }
    let cfg = fighters(&ff)?;
    let sheet = bf_story::BeatSheet::series(cfg);
    let base =
        bf_sim::setup_from(&ff, bf_content::roster(), 0).map_err(|e| Fail(format!("{}: {e}.", a.fight.display())))?;
    let setup_for = |h: [f64; 2], seed: u64| {
        let mut s = Setup { seed, ..base.clone() };
        s.rules.dmg_side = Some(h);
        s
    };
    let start = [ff.handicap[0], ff.handicap[1]];
    let progress = |p: BalanceProgress| note(&format!("balance: step {} of {}", p.done, p.total));
    let t0 = Instant::now();
    let found = auto_balance(&setup_for, &sheet, start, ctx.workers, &progress, ctx.cancel).map_err(|h| match h {
        Halt::Cancelled => Fail::new("the balance was stopped (Ctrl+C), so nothing was written."),
        Halt::Failed(why) => Fail(format!("{}: {why}.", a.fight.display())),
    })?;
    note(&format!("balance: done in {:.1} s", t0.elapsed().as_secs_f64()));
    let h = found.handicap;
    let names = &ff.fighters;
    if ctx.json {
        let steps: Vec<Value> = found
            .steps
            .iter()
            .map(|s| {
                json!({
                    "round": s.round,
                    "what": match s.what { BalanceRound::Ratio => "ratio", BalanceRound::Scale => "scale" },
                    "handicap": [num(s.handicap[0]), num(s.handicap[1])],
                    "stats": stats_json(&s.stats),
                })
            })
            .collect();
        say_json(&json!({
            "fight": a.fight.display().to_string(),
            "was": [num(start[0]), num(start[1])],
            "handicap": [num(h[0]), num(h[1])],
            "in_band": found.in_band,
            "confirm": stats_json(&found.confirm),
            "steps": steps,
            "written": a.write,
        }));
    } else {
        let c = &found.confirm;
        say(&format!(
            "handicap: [{:?}, {:?}] (was [{:?}, {:?}])",
            h[0], h[1], start[0], start[1]
        ));
        say(&format!(
            "{} wins {:.1} % of {} decided fights of {}, median K.O. {:.2} s, no K.O. {:.1} %",
            names[0],
            100.0 * c.win_share,
            c.decided,
            c.n,
            c.ko_median,
            100.0 * c.no_ko_share
        ));
        say(&format!(
            "even (40 to 60 % wins, median K.O. 49 to 52 s): {}",
            if found.in_band { "yes" } else { "no" }
        ));
    }
    if a.write {
        write_handicap(&a.fight, &text, &ff, h)?;
        if !ctx.json {
            say(&format!("saved the handicap in {}", a.fight.display()));
        }
    }
    Ok(())
}

/// The confirmation figures as JSON.
fn stats_json(s: &Stats) -> Value {
    json!({
        "n": s.n,
        "decided": s.decided,
        "win_share": num(s.win_share),
        "ko_median": num(s.ko_median),
        "ko_p10": num(s.ko_p10),
        "ko_p90": num(s.ko_p90),
        "no_ko_share": num(s.no_ko_share),
        "lead_changes_mean": num(s.lead_changes_mean),
        "fit_rate": num(s.fit_rate),
        "ko_hist": s.ko_hist,
    })
}

/// Stores `h` as the handicap of the fight file at `path`, whose text was `text` when it was read.
fn write_handicap(path: &Path, text: &str, ff: &FightFile, h: [f64; 2]) -> Done {
    let now = std::fs::read_to_string(path)
        .map_err(|e| Fail(format!("the fight file {} cannot be read again ({e}).", path.display())))?;
    if now != text {
        return Err(Fail(format!(
            "the fight file {} was changed while the balance ran, so the handicap was not written; run it again.",
            path.display()
        )));
    }
    let mut want = ff.clone();
    want.handicap = h.to_vec();
    let new_text = set_handicap(text, h)
        .filter(|t| FightFile::parse(t).is_ok_and(|f| f == want))
        .ok_or_else(|| one_line_handicap(path))?;
    let tmp = temp_beside(path);
    let wrote =
        write_whole(&tmp, new_text.as_bytes()).and_then(|()| std::fs::rename(&tmp, path).map_err(|e| e.to_string()));
    if let Err(why) = wrote {
        let _ = std::fs::remove_file(&tmp);
        return Err(Fail(format!(
            "the handicap could not be written into {} ({why}); the file is as it was.",
            path.display()
        )));
    }
    Ok(())
}

/// The refusal of `--write` for a fight file whose handicap is not one line of its own.
fn one_line_handicap(path: &Path) -> Fail {
    Fail(format!(
        "the fight file {} has no line `handicap = [a, b]` of its own before its first table, so --write cannot store \n         the handicap without rewriting the rest of the file; write the handicap on one line (for example \n         handicap = [1.0, 1.0]) and run it again.",
        path.display()
    ))
}

/// `text` with the value of its top-level `handicap = [..]` line replaced by `h`, or `None` when the file has no
/// such line before its first table.
fn set_handicap(text: &str, h: [f64; 2]) -> Option<String> {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            return None; // the first table: top-level keys end here
        }
        if let Some(rest) = trimmed.strip_prefix("handicap") {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix('=') {
                let open = line.len() - value.len() + (value.len() - value.trim_start().len());
                if !line[open..].starts_with('[') {
                    return None;
                }
                let close = open + line[open..].find(']')?;
                let mut out = String::with_capacity(text.len() + 16);
                out.push_str(&text[..at + open]);
                out.push_str(&format!("[{:?}, {:?}]", h[0], h[1]));
                out.push_str(&text[at + close + 1..]);
                return Some(out);
            }
        }
        at += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::set_handicap;

    #[test]
    fn only_the_value_changes() {
        let text = "# notes\nformat = 1\n  handicap   =  [2.0, 0.4]  # tuned\nseed = 1\n[music]\nhandicap = [9.0]\n";
        assert_eq!(
            set_handicap(text, [1.94, 0.39]).as_deref(),
            Some("# notes\nformat = 1\n  handicap   =  [1.94, 0.39]  # tuned\nseed = 1\n[music]\nhandicap = [9.0]\n")
        );
        assert_eq!(
            set_handicap("format = 1\r\nhandicap = [1.0, 1.0]\r\n", [2.0, 3.0]).as_deref(),
            Some("format = 1\r\nhandicap = [2.0, 3.0]\r\n")
        );
        assert_eq!(
            set_handicap("format = 1\n[music]\nhandicap = [1.0, 1.0]\n", [2.0, 3.0]),
            None
        );
        assert_eq!(set_handicap("handicap = [\n 1.0,\n 1.0,\n]\n", [2.0, 3.0]), None);
        assert_eq!(set_handicap("handicaps = 3\n", [2.0, 3.0]), None);
    }
}
