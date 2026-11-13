//! The reference set's size budgets (plan milestone 4, section 4), the same numbers as `tools/studio_export`'s
//! `budget` command (`m3refs.py`). KB and MB are 1,000 and 1,000,000 bytes, as `m1-measurements.md` section 10
//! counts. The files counted are those git tracks under `studio/reference` or would track (untracked and not
//! ignored), so a folder an export has just written is measured before it is committed.

use std::collections::BTreeMap;
use std::process::Command;

use crate::paths::repo_root;

pub const KB: u64 = 1000;
pub const MB: u64 = 1000 * 1000;
/// `fighters/<slug>/`, with no recording (every one is in the pinned cache).
pub const FIGHTER_BUDGET: u64 = 220 * KB;
/// The fighter folders merged before the milestone 4 budget that are over [`FIGHTER_BUDGET`] (EUPHONIUM, 227 KB).
pub const FIGHTER_BUDGET_LARGE: u64 = 300 * KB;
pub const LARGE_FIGHTERS: [&str; 1] = ["euphonium"];
/// `themes/<name>/`, with no recording.
pub const THEME_BUDGET: u64 = 120 * KB;
/// `episodes/<pair>/`, and the two pair gates that carry their own kit recording (Tasks 45 and 46).
pub const EPISODE_BUDGET: u64 = 100 * KB;
pub const EPISODE_BUDGET_LARGE: u64 = 260 * KB;
pub const LARGE_EPISODES: [&str; 2] = ["trombone_vs_tuba", "bassclarinet_vs_euphonium"];
/// `older/` (milestone 4's shared vectors and the origin gate's self-proof; the top-level manifest).
pub const OLDER: &str = "older";
pub const OLDER_BUDGET: u64 = 400 * KB;
/// Gate O's `origin.json` inside a fighter or theme folder.
pub const ORIGIN: &str = "origin.json";
pub const ORIGIN_BUDGET: u64 = 20 * KB;
/// Everything under `studio/reference`.
pub const TOTAL_BUDGET: u64 = 147 * MB;
/// Past this the budget warns (it still fails only past [`TOTAL_BUDGET`]): reserve C1 is then due.
pub const TRIPWIRE: u64 = 145_500_000;
/// The folders below `studio/reference` still allowed a committed `.bfrec`: milestone 1's fights. Every other
/// recording is the pinned cache file; a committed one is a row of its own with a limit of 0 bytes.
pub const COMMITTED_RECORDINGS: [&str; 1] = ["fights"];

/// One row of the report: a milestone 3 or 4 folder (`fighters/flute`, `older`), an `origin.json`, a committed
/// recording outside [`COMMITTED_RECORDINGS`] (limit 0), or the whole set (`studio/reference`, last).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub folder: String,
    pub bytes: u64,
    pub limit: u64,
}

/// The budget rows for `files`: `(path below studio/reference/, bytes)` pairs. One row per milestone 3 or 4 folder
/// that has files, per `origin.json` in a fighter or theme folder and per committed recording outside
/// [`COMMITTED_RECORDINGS`], sorted by name, then the whole set.
pub fn budget_rows(files: &[(String, u64)]) -> Vec<Budget> {
    let mut per: BTreeMap<String, u64> = BTreeMap::new();
    let mut rows: BTreeMap<String, Budget> = BTreeMap::new();
    let mut total = 0;
    for (rel, size) in files {
        total += size;
        let parts: Vec<&str> = rel.split('/').collect();
        if parts.len() >= 3 && crate::manifest::AREAS.contains(&parts[0]) {
            *per.entry(format!("{}/{}", parts[0], parts[1])).or_default() += size;
        } else if parts.len() >= 2 && parts[0] == OLDER {
            *per.entry(OLDER.to_string()).or_default() += size;
        }
        let mut own = |limit| {
            rows.insert(
                rel.clone(),
                Budget {
                    folder: rel.clone(),
                    bytes: *size,
                    limit,
                },
            );
        };
        if parts.len() == 3 && ["fighters", "themes"].contains(&parts[0]) && parts[2] == ORIGIN {
            own(ORIGIN_BUDGET);
        }
        if rel.ends_with(".bfrec") && !COMMITTED_RECORDINGS.contains(&parts[0]) {
            own(0);
        }
    }
    for (folder, bytes) in per {
        let (area, name) = folder.split_once('/').unwrap_or((folder.as_str(), ""));
        let limit = match area {
            "fighters" if LARGE_FIGHTERS.contains(&name) => FIGHTER_BUDGET_LARGE,
            "fighters" => FIGHTER_BUDGET,
            "themes" => THEME_BUDGET,
            "episodes" if LARGE_EPISODES.contains(&name) => EPISODE_BUDGET_LARGE,
            "episodes" => EPISODE_BUDGET,
            _ => OLDER_BUDGET,
        };
        rows.insert(folder.clone(), Budget { folder, bytes, limit });
    }
    let mut out: Vec<Budget> = rows.into_values().collect();
    out.push(Budget {
        folder: "studio/reference".into(),
        bytes: total,
        limit: TOTAL_BUDGET,
    });
    out
}

/// The warning the budget prints when the whole set (`total` bytes) is past [`TRIPWIRE`], else `None`; the same
/// text as `m3refs.tripwire_warning`.
pub fn tripwire_warning(total: u64) -> Option<String> {
    (total > TRIPWIRE).then(|| {
        format!(
            "warning: the reference set is {:.2} MB, past the 145.5 MB tripwire (plan m4 Section 4)",
            total as f64 / MB as f64
        )
    })
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
            f("fighters/flute/look.json", 210_000),
            f("fighters/flute/fight.json", 10_000),
            f("fighters/tuba/look.json", 221_000),
            f("fighters/euphonium/look.json", 290_000),
            f("fighters/viola/origin.json", 20_001),
            f("themes/heist/theme.json", 120_000),
            f("themes/surf_rock/origin.json", 20_000),
            f("themes/horror_trap/run.bfrec", 1),
            f("episodes/trombone_vs_tuba/checks.json", 1),
            f("episodes/oboe_vs_bassoon/checks.json", 1),
            f("episodes/oboe_vs_bassoon/origin.json", 30_000),
            f("older/vectors.json", 400_000),
            f("fights/flute_vs_trombone/ep392.bfrec", 1_000),
        ]);
        let by: BTreeMap<&str, &Budget> = rows.iter().map(|b| (b.folder.as_str(), b)).collect();
        let lim = |k: &str| (by[k].bytes, by[k].limit);
        assert_eq!(lim("fighters/flute"), (220_000, 220_000));
        assert!(
            by["fighters/tuba"].bytes > by["fighters/tuba"].limit,
            "a 221 KB fighter is over"
        );
        assert_eq!(by["fighters/euphonium"].limit, 300_000);
        assert_eq!(lim("fighters/viola/origin.json"), (20_001, 20_000));
        assert_eq!(lim("themes/surf_rock/origin.json"), (20_000, 20_000));
        assert!(!by.contains_key("episodes/oboe_vs_bassoon/origin.json"));
        assert_eq!(lim("themes/heist"), (120_000, 120_000));
        assert_eq!(
            lim("themes/horror_trap/run.bfrec"),
            (1, 0),
            "a committed recording is refused"
        );
        assert!(
            !by.contains_key("fights/flute_vs_trombone/ep392.bfrec"),
            "milestone 1's fights may"
        );
        assert_eq!(by["episodes/trombone_vs_tuba"].limit, 260_000);
        assert_eq!(by["episodes/oboe_vs_bassoon"].limit, 100_000);
        assert_eq!(lim("older"), (400_000, 400_000));
        let total = rows.last().unwrap();
        assert_eq!(
            (total.folder.as_str(), total.bytes, total.limit),
            ("studio/reference", 1_322_004, 147_000_000)
        );
        let names: Vec<&str> = rows[..rows.len() - 1].iter().map(|b| b.folder.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert_eq!(rows.len(), 14, "{rows:?}");
    }
}
