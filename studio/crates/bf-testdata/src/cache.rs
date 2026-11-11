//! The pinned cache (milestone 3): bulky reference data the export tool regenerates on the reference PC under the
//! git-ignored `studio/reference/.cache/m3/<area>/<name>/`, each file pinned by SHA-256 in the folder's committed
//! `studio/reference/<area>/<name>/pins.json`. A cache-backed test gets its file through [`cache_file`], which
//! checks the pin first: a missing or changed file fails the test with the command that regenerates it, never a
//! silent skip, and never a re-pin (a changed regenerated file is investigated first; ruling M3-R1).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use sha2::{Digest, Sha256};

use crate::manifest::AREAS;
use crate::paths::reference_dir;

/// One line of a `pins.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    /// Below `studio/reference/.cache/m3/`, forward slashes (`fighters/flute/right.bfrec`).
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    /// The export that makes it (`fighter_flute`).
    pub export: String,
}

/// `<repo>/studio/reference/.cache/m3`.
pub fn cache_root() -> PathBuf {
    reference_dir().join(".cache").join("m3")
}

/// The command that regenerates a folder's cache files.
pub fn cache_command(folder: &str) -> String {
    format!("python -m tools.studio_export cache {folder}")
}

/// `<area>/<name>` of a cache path (`fighters/flute/right.bfrec` -> `fighters/flute`); panics on a path that is not
/// a file of a milestone 3 folder.
pub fn folder_of(rel: &str) -> String {
    let parts: Vec<&str> = rel.split('/').collect();
    assert!(
        parts.len() >= 3 && AREAS.contains(&parts[0]) && parts.iter().all(|p| !p.is_empty() && *p != ".."),
        "{rel:?} is not a cache file of a reference folder (<area>/<name>/<file>, area one of {AREAS:?})"
    );
    format!("{}/{}", parts[0], parts[1])
}

/// The pins of a folder (`<area>/<name>`), from `studio/reference/<folder>/pins.json`; empty when it has none.
pub fn pins(folder: &str) -> Vec<Pin> {
    read_pins(&reference_dir().join(folder).join("pins.json")).unwrap_or_default()
}

/// The export that makes a folder by its area (`fighters/flute` -> `fighter_flute`).
pub fn export_command(folder: &str) -> String {
    let (area, name) = folder.split_once('/').expect("<area>/<name>");
    let prefix = match area {
        "fighters" => "fighter_",
        "themes" => "theme_",
        _ => "episode_",
    };
    format!("python -m tools.studio_export {prefix}{name}")
}

fn read_pins(path: &Path) -> Option<Vec<Pin>> {
    let text = fs::read_to_string(path).ok()?;
    let doc: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: not JSON: {e}", path.display()));
    assert_eq!(doc["format"], 1, "{}: pins format", path.display());
    let rows = doc["pins"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no pins list", path.display()));
    rows.iter()
        .map(|p| {
            let field = |k: &str| {
                p[k].as_str()
                    .unwrap_or_else(|| panic!("{}: a pin without {k}: {p}", path.display()))
                    .to_string()
            };
            Pin {
                path: field("path"),
                sha256: field("sha256"),
                bytes: p["bytes"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{}: bytes of {p}", path.display())),
                export: field("export"),
            }
        })
        .collect::<Vec<_>>()
        .into()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The checked path and bytes of cache file `rel` under `root`, or the message a test fails with. `pins` is
/// `None` when the folder has no `pins.json` at all (never exported).
fn check(root: &Path, pins: Option<&[Pin]>, rel: &str) -> Result<(PathBuf, Vec<u8>), String> {
    let folder = folder_of(rel);
    let shown = format!("studio/reference/.cache/m3/{rel}");
    let Some(pins) = pins else {
        return Err(format!(
            "{shown}: studio/reference/{folder}/pins.json does not exist, the folder has never been exported;              run the export: {}",
            export_command(&folder)
        ));
    };
    let run = format!("run: {}", cache_command(&folder));
    let Some(pin) = pins.iter().find(|p| p.path == rel) else {
        return Err(format!(
            "{shown} is not pinned in studio/reference/{folder}/pins.json; {run}"
        ));
    };
    let path = root.join(rel);
    let Ok(bytes) = fs::read(&path) else {
        return Err(format!("{shown} is missing (a cache file, git-ignored); {run}"));
    };
    let sha = sha256_hex(&bytes);
    if bytes.len() as u64 != pin.bytes || sha != pin.sha256 {
        return Err(format!(
            "{shown} differs from its pin ({} bytes, sha256 {sha}; pinned {} bytes, {}); {run}              (if it still differs, find out why: a re-pin is the controller's decision)",
            bytes.len(),
            pin.bytes,
            pin.sha256
        ));
    }
    Ok((path, bytes))
}

/// The paths [`cache_file`] has already checked in this process: each is hashed once, not on every call.
static CHECKED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// `studio/reference/.cache/m3/<rel>`, after checking it against its pin in its folder's `pins.json` (once per path
/// and process). When it is absent, not pinned or changed, panics with "run: python -m tools.studio_export cache
/// <folder>"; when the folder has no `pins.json` at all, with the export command that makes it.
pub fn cache_file(rel: &str) -> PathBuf {
    if CHECKED.lock().unwrap_or_else(|e| e.into_inner()).contains(rel) {
        return cache_root().join(rel);
    }
    let folder = folder_of(rel);
    let pins = read_pins(&reference_dir().join(&folder).join("pins.json"));
    let (path, _) = check(&cache_root(), pins.as_deref(), rel).unwrap_or_else(|msg| panic!("{msg}"));
    CHECKED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(rel.to_string());
    path
}

/// The bytes of cache file `rel`, hashed against its pin as they are handed out (so a file that changes after an
/// earlier [`cache_file`] check is still caught). Panics as [`cache_file`] does.
pub fn cache_bytes(rel: &str) -> Vec<u8> {
    let folder = folder_of(rel);
    let pins = read_pins(&reference_dir().join(&folder).join("pins.json"));
    check(&cache_root(), pins.as_deref(), rel)
        .unwrap_or_else(|msg| panic!("{msg}"))
        .1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bf_testdata_cache_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("fighters").join("flute")).unwrap();
        dir
    }

    fn pin_of(bytes: &[u8]) -> Pin {
        Pin {
            path: "fighters/flute/right.bfrec".into(),
            sha256: sha256_hex(bytes),
            bytes: bytes.len() as u64,
            export: "fighter_flute".into(),
        }
    }

    #[test]
    fn a_pinned_file_is_handed_out() {
        let root = scratch("ok");
        fs::write(root.join("fighters/flute/right.bfrec"), b"abc").unwrap();
        let got = check(&root, Some(&[pin_of(b"abc")]), "fighters/flute/right.bfrec")
            .unwrap()
            .0;
        assert_eq!(got, root.join("fighters/flute/right.bfrec"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_missing_file_names_the_command() {
        let root = scratch("missing");
        let msg = check(&root, Some(&[pin_of(b"abc")]), "fighters/flute/right.bfrec").unwrap_err();
        assert!(
            msg.contains("studio/reference/.cache/m3/fighters/flute/right.bfrec is missing"),
            "{msg}"
        );
        assert!(
            msg.contains("run: python -m tools.studio_export cache fighters/flute"),
            "{msg}"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_changed_file_names_the_command_and_is_not_repinned() {
        let root = scratch("changed");
        fs::write(root.join("fighters/flute/right.bfrec"), b"abd").unwrap();
        let msg = check(&root, Some(&[pin_of(b"abc")]), "fighters/flute/right.bfrec").unwrap_err();
        assert!(msg.contains("differs from its pin"), "{msg}");
        assert!(
            msg.contains("run: python -m tools.studio_export cache fighters/flute"),
            "{msg}"
        );
        assert!(msg.contains("controller's decision"), "{msg}");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_unpinned_file_names_the_command() {
        let root = scratch("unpinned");
        fs::write(root.join("fighters/flute/other.json"), b"{}").unwrap();
        let msg = check(&root, Some(&[pin_of(b"abc")]), "fighters/flute/other.json").unwrap_err();
        assert!(
            msg.contains("is not pinned in studio/reference/fighters/flute/pins.json"),
            "{msg}"
        );
        assert!(
            msg.contains("run: python -m tools.studio_export cache fighters/flute"),
            "{msg}"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[should_panic(expected = "run the export: python -m tools.studio_export theme_no_such_theme")]
    fn a_folder_never_exported_names_the_export() {
        cache_file("themes/no_such_theme/stems.npz");
    }

    #[test]
    #[should_panic(expected = "run: python -m tools.studio_export cache fighters/flute")]
    fn cache_file_panics_with_the_cache_command() {
        cache_file("fighters/flute/no_such_file.json");
    }

    #[test]
    #[ignore = "reads the cache: run python -m tools.studio_export cache fighters/flute first"]
    fn a_pinned_file_is_checked_once_and_its_bytes_are_hashed() {
        let pins = pins("fighters/flute");
        let pin = pins
            .iter()
            .find(|p| p.path == "fighters/flute/right.bfrec")
            .expect("the flute's right pin");
        let path = cache_file(&pin.path);
        assert_eq!(
            cache_file(&pin.path),
            path,
            "the second call is answered from the checked set"
        );
        let bytes = cache_bytes(&pin.path);
        assert_eq!(bytes.len() as u64, pin.bytes);
    }

    #[test]
    #[should_panic(expected = "is not a cache file of a reference folder")]
    fn a_path_outside_the_areas_is_refused() {
        cache_file("fights/flute_vs_trombone/ep392.bfrec");
    }
}
