//! The reference set's size budgets (plan milestone 3, section 4), the same numbers as `tools/studio_export`'s
//! `budget` command (`m3refs.py`). KB and MB are 1,000 and 1,000,000 bytes, as `m1-measurements.md` section 10
//! counts. The files counted are those git tracks under `studio/reference` or would track (untracked and not
//! ignored), so a folder an export has just written is measured before it is committed.

use std::collections::BTreeMap;
use std::process::Command;

use crate::paths::repo_root;

pub const KB: u64 = 1000;
pub const MB: u64 = 1000 * 1000;
/// `fighters/<slug>/`.
pub const FIGHTER_BUDGET: u64 = 300 * KB;
/// `themes/<name>/`: this plus [`THEME_PER_RECORDING`] for every `.bfrec` in it.
pub const THEME_BUDGET: u64 = 120 * KB;
pub const THEME_PER_RECORDING: u64 = 160 * KB;
/// `episodes/<pair>/`, and the two pair gates that carry their own kit recording (Tasks 45 and 46).
pub const EPISODE_BUDGET: u64 = 100 * KB;
pub const EPISODE_BUDGET_LARGE: u64 = 260 * KB;
pub const LARGE_EPISODES: [&str; 2] = ["trombone_vs_tuba", "bassclarinet_vs_euphonium"];
/// Everything under `studio/reference`.
pub const TOTAL_BUDGET: u64 = 147 * MB;

/// One row of the report: a milestone 3 folder (`fighters/flute`), or the whole set (`studio/reference`, last).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub folder: String,
    pub bytes: u64,
    pub limit: u64,
}

/// The budget rows for `files`: `(path below studio/reference/, bytes)` pairs. One row per milestone 3 folder that
/// has files, in path order, then the whole set.
pub fn budget_rows(files: &[(String, u64)]) -> Vec<Budget> {
    let mut per: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut total = 0;
    for (rel, size) in files {
        total += size;
        let parts: Vec<&str> = rel.split('/').collect();
        if parts.len() >= 3 && crate::manifest::AREAS.contains(&parts[0]) {
            let row = per.entry(format!("{}/{}", parts[0], parts[1])).or_default();
            row.0 += size;
            row.1 += u64::from(rel.ends_with(".bfrec"));
        }
    }
    let mut out: Vec<Budget> = per
        .into_iter()
        .map(|(folder, (bytes, recordings))| {
            let (area, name) = folder.split_once('/').expect("<area>/<name>");
            let limit = match area {
                "fighters" => FIGHTER_BUDGET,
                "themes" => THEME_BUDGET + THEME_PER_RECORDING * recordings,
                _ if LARGE_EPISODES.contains(&name) => EPISODE_BUDGET_LARGE,
                _ => EPISODE_BUDGET,
            };
            Budget { folder, bytes, limit }
        })
        .collect();
    out.push(Budget {
        folder: "studio/reference".into(),
        bytes: total,
        limit: TOTAL_BUDGET,
    });
    out
}

/// `(path below studio/reference/, bytes)` of every file git tracks there or would track, in path order; a
/// tracked file missing from the working tree is left out.
pub fn reference_files() -> Vec<(String, u64)> {
    let root = repo_root();
    let out = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            "studio/reference",
        ])
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("git ls-files in {}: {e} (the budget needs git on PATH)", root.display()));
    assert!(
        out.status.success(),
        "git ls-files: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut files: BTreeMap<String, u64> = BTreeMap::new();
    for rel in String::from_utf8(out.stdout).expect("git paths are UTF-8").split('\0') {
        let Some(below) = rel.strip_prefix("studio/reference/") else {
            continue;
        };
        if let Ok(meta) = std::fs::metadata(root.join(rel))
            && meta.is_file()
        {
            files.insert(below.to_string(), meta.len());
        }
    }
    files.into_iter().collect()
}

/// Section 4's budgets over the reference set as it is on disk ([`reference_files`]).
pub fn budget_report() -> Vec<Budget> {
    budget_rows(&reference_files())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(rel: &str, bytes: u64) -> (String, u64) {
        (rel.to_string(), bytes)
    }

    #[test]
    fn the_numbers_of_section_4() {
        let rows = budget_rows(&[
            f("fighters/flute/left.bfrec", 290_000),
            f("fighters/flute/fight.json", 10_000),
            f("fighters/tuba/left.bfrec", 301_000),
            f("themes/heist/a.bfrec", 150_000),
            f("themes/heist/b.bfrec", 150_000),
            f("episodes/trombone_vs_tuba/run.bfrec", 1),
            f("episodes/oboe_vs_bassoon/checks.json", 1),
            f("fights/flute_vs_trombone/ep392.bfrec", 1_000),
        ]);
        let by: BTreeMap<&str, &Budget> = rows.iter().map(|b| (b.folder.as_str(), b)).collect();
        assert_eq!(
            (by["fighters/flute"].bytes, by["fighters/flute"].limit),
            (300_000, 300_000)
        );
        assert!(
            by["fighters/tuba"].bytes > by["fighters/tuba"].limit,
            "a 301 KB fighter is over"
        );
        assert_eq!(by["themes/heist"].limit, 440_000);
        assert_eq!(by["episodes/trombone_vs_tuba"].limit, 260_000);
        assert_eq!(by["episodes/oboe_vs_bassoon"].limit, 100_000);
        let total = rows.last().unwrap();
        assert_eq!(
            (total.folder.as_str(), total.bytes, total.limit),
            ("studio/reference", 902_002, 147_000_000)
        );
        assert_eq!(rows.len(), 6, "{rows:?}");
    }
}
