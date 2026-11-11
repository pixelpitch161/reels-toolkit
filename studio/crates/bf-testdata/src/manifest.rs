//! The reference manifest, sharded by folder (milestone 3): every `studio/reference/fighters/<slug>/`,
//! `themes/<name>/` and `episodes/<pair>/` folder has its own `manifest.json` listing the files under it; every other
//! reference file is in the top-level `studio/reference/manifest.json`. `tools/studio_export/manifest.py` writes
//! them; this module reads them, so a Rust test can see the same layout `verify` checks.

use std::fs;
use std::path::PathBuf;

use crate::paths::{reference_dir, repo_root};

/// The sharded areas: one manifest per folder below each.
pub const AREAS: [&str; 3] = ["fighters", "themes", "episodes"];

/// One manifest line (the fields a Rust test needs; the version fields are left out).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestEntry {
    /// Repo-relative, forward slashes.
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    /// The module that wrote the file (or `rust-bless`).
    pub tool: String,
}

/// `<area>/<name>` when the repo-relative path lies inside a sharded folder (with something below `<name>`).
pub fn area_folder(rel: &str) -> Option<String> {
    let rest = rel.strip_prefix("studio/reference/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    (parts.len() >= 3 && AREAS.contains(&parts[0]) && !parts[1].is_empty() && parts[1] != ".cache")
        .then(|| format!("{}/{}", parts[0], parts[1]))
}

/// The repo-relative path of the manifest that lists the file at the repo-relative `rel`.
pub fn shard_of(rel: &str) -> String {
    match area_folder(rel) {
        Some(folder) => format!("studio/reference/{folder}/manifest.json"),
        None => "studio/reference/manifest.json".to_string(),
    }
}

/// Every manifest that exists, repo-relative: the top-level one first, then the shards in path order.
pub fn manifest_shards() -> Vec<String> {
    let root = reference_dir();
    let mut out = Vec::new();
    if root.join("manifest.json").is_file() {
        out.push("studio/reference/manifest.json".to_string());
    }
    for area in AREAS {
        let Ok(dir) = fs::read_dir(root.join(area)) else {
            continue;
        };
        let mut names: Vec<String> = dir
            .filter_map(|e| e.ok())
            .filter(|e| e.path().join("manifest.json").is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        out.extend(
            names
                .into_iter()
                .map(|n| format!("studio/reference/{area}/{n}/manifest.json")),
        );
    }
    out
}

fn read_shard(rel: &str) -> Vec<ManifestEntry> {
    let path: PathBuf = repo_root().join(rel);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: not JSON: {e}", path.display()));
    assert_eq!(doc["format"], 1, "{}: manifest format", path.display());
    let entries = doc["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no entries", path.display()));
    entries
        .iter()
        .map(|e| {
            let field = |k: &str| {
                e[k].as_str()
                    .unwrap_or_else(|| panic!("{}: an entry without {k}: {e}", path.display()))
                    .to_string()
            };
            ManifestEntry {
                path: field("path"),
                sha256: field("sha256"),
                bytes: e["bytes"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{}: bytes of {e}", path.display())),
                tool: field("tool"),
            }
        })
        .collect()
}

/// `(shard, entry)` for every entry of every manifest, shards in `manifest_shards` order.
pub fn manifest_entries() -> Vec<(String, ManifestEntry)> {
    manifest_shards()
        .into_iter()
        .flat_map(|s| read_shard(&s).into_iter().map(move |e| (s.clone(), e)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shards_by_folder() {
        assert_eq!(
            shard_of("studio/reference/fighters/flute/fight.json"),
            "studio/reference/fighters/flute/manifest.json"
        );
        assert_eq!(
            shard_of("studio/reference/episodes/a_vs_b/sub/x.json"),
            "studio/reference/episodes/a_vs_b/manifest.json"
        );
        assert_eq!(
            shard_of("studio/reference/fighters/x.json"),
            "studio/reference/manifest.json"
        );
        assert_eq!(
            shard_of("studio/reference/fights/flute_vs_trombone/ep392.bfrec"),
            "studio/reference/manifest.json"
        );
        assert_eq!(
            shard_of("studio/crates/bf-content/data/roster.json"),
            "studio/reference/manifest.json"
        );
    }

    #[test]
    fn the_top_level_manifest_reads() {
        let shards = manifest_shards();
        assert_eq!(shards[0], "studio/reference/manifest.json");
        let entries = manifest_entries();
        assert!(
            entries
                .iter()
                .any(|(_, e)| e.path == "studio/reference/fights/flute_vs_trombone/ep392.bfrec")
        );
    }
}
