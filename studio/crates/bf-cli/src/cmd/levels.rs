//! `ballfight levels <fight.toml>... [--write FILE]`: the calibration run of the sound upgrade (M1). Each fight
//! file is a reference fight: its left fighter against the target dummy over the same Composed band
//! (`fights/dev/bold_<fighter>.toml`). The fighter's level and make-up are measured
//! ([`bf_content::measure_level`]) and printed as rows of the level table; `--write` puts them into the table's
//! source (`crates/bf-content/src/levels_table.rs`), which is then committed.
//!
//! Run it again after a voice changes. A fighter's kind (low, piercing) decides how it is treated, and the
//! treatment changes what it is made of (a piercing fighter's line is kept out of its top octave, so it then
//! measures less piercing). So a kind the table already gives a fighter is kept, with the share that gave it,
//! unless `--fresh` is given (for a voice rebuilt to meet the target by itself); after the first write of a new
//! kind a second run settles the gain.

use std::path::PathBuf;

use bf_content::levels_table::{self, Saved};
use serde_json::json;

use crate::{Ctx, Done, Fail, load_fight, need_ko, note, say, say_json, stopped, the_fight};

#[derive(clap::Args)]
pub struct Args {
    /// The reference fights, one a fighter (its left fighter is measured).
    #[arg(required = true)]
    fights: Vec<PathBuf>,
    /// Write the rows into this source file's level table (the rows of fighters not measured are kept).
    #[arg(long, value_name = "FILE")]
    write: Option<PathBuf>,
    /// Take each fighter's kind from this run alone, not from the table.
    #[arg(long)]
    fresh: bool,
}

pub fn run(a: &Args, ctx: &Ctx) -> Done {
    let mut rows: Vec<(String, Saved)> = Vec::new();
    for path in &a.fights {
        let ff = load_fight(path)?;
        let rec = the_fight(&ff, path, None, None, true)?;
        need_ko(&rec)?;
        stopped(ctx, "the levels")?;
        note(&format!("levels: measuring {}", path.display()));
        let (name, mut saved) =
            bf_content::measure_level(&rec, &ff).map_err(|e| Fail(format!("{}: {e}.", path.display())))?;
        if let (Some(old), false) = (levels_table::of(&name), a.fresh) {
            if old.low() && !saved.low() {
                saved.rumble = old.rumble;
            }
            if old.piercing() && !saved.piercing() {
                saved.piercing = old.piercing;
            }
        }
        if rows.iter().any(|(n, _)| *n == name) {
            return Err(Fail(format!("{}: {name} is measured twice.", path.display())));
        }
        rows.push((name, saved));
    }
    // kept fighters first as the table has them, then in roster order
    let mut table: Vec<(String, Saved)> = levels_table::LEVELS
        .iter()
        .map(|&(n, gain, rumble, piercing)| (n.to_string(), Saved { gain, rumble, piercing }))
        .filter(|(n, _)| rows.iter().all(|(m, _)| m != n))
        .collect();
    table.extend(rows.iter().cloned());
    let place = |name: &str| bf_content::roster().iter().position(|f| f.name == name);
    table.sort_by_key(|(n, _)| place(n));
    if let Some(file) = &a.write {
        let text = std::fs::read_to_string(file)
            .map_err(|e| Fail(format!("the table's source {} cannot be read ({e}).", file.display())))?;
        let missing = || Fail(format!("{} does not hold the level table.", file.display()));
        // the last one: the source also holds the opening line as a constant
        let open = text.rfind(levels_table::TABLE_OPENS).ok_or_else(missing)? + levels_table::TABLE_OPENS.len();
        let close = open + text[open..].find(levels_table::TABLE_CLOSES).ok_or_else(missing)?;
        let body: String = table.iter().map(|(n, s)| levels_table::row(n, *s) + "\n").collect();
        let new = format!("{}\n{}{}", &text[..open], body, &text[close..]);
        std::fs::write(file, new).map_err(|e| Fail(format!("{} cannot be written ({e}).", file.display())))?;
    }
    if ctx.json {
        let all: Vec<_> = rows
            .iter()
            .map(|(n, s)| json!({"fighter": n, "gain": s.gain, "rumble": s.rumble, "piercing": s.piercing}))
            .collect();
        say_json(&json!({"levels": all, "written": a.write.as_ref().map(|f| f.display().to_string())}));
    } else {
        for (n, s) in &rows {
            say(&levels_table::row(n, *s));
        }
        if let Some(file) = &a.write {
            say(&format!("wrote {} rows into {}", table.len(), file.display()));
        }
    }
    Ok(())
}
