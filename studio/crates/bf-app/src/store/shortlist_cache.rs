//! The shortlist cache, `cache/shortlists/<16 hex digits>.json`: how far a seed search got for one search key, so a
//! search picks up where it stopped. Only seed numbers are kept; the engine re-runs them (fights are reproducible).
//!
//! Nothing in the cache was made by a person: a file that cannot be read, does not parse, or was written for another
//! version or key is simply a miss, and may be written over or pruned. It is never moved aside.

use std::ffi::OsString;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::write_atomic;

/// The cache file's format. A file with any other number is a miss.
pub const CACHE_FORMAT: u32 = 1;

/// The most cache files kept: saving one more deletes the oldest beyond this.
pub const CACHE_FILES: usize = 200;

/// How far a search got: the next seed to try, how many were checked, and the seeds found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedShortlist {
    pub next_seed: u64,
    pub checked: u64,
    pub fit_seeds: Vec<u64>,
    pub other_seeds: Vec<u64>,
}

/// The cache file as it is written and read.
#[derive(Serialize, Deserialize)]
struct CacheFile {
    format: u32,
    app_version: String,
    search_key: String,
    next_seed: u64,
    checked: u64,
    fit_seeds: Vec<u64>,
    other_seeds: Vec<u64>,
}

/// `<dir>/<first 16 hex digits of sha256(app_version + "\n" + search_key)>.json`.
pub fn cache_file(dir: &Path, app_version: &str, search_key: &str) -> PathBuf {
    let digest = Sha256::digest(format!("{app_version}\n{search_key}").as_bytes());
    let name: String = digest[..8].iter().map(|byte| format!("{byte:02x}")).collect();
    dir.join(format!("{name}.json"))
}

/// The largest cache file read: 16 MiB. A seed is written in at most 21 bytes (20 digits and a comma), so this holds
/// about 800,000 seeds, thousands of times a real shortlist (200 fits and the others checked beside them); a larger
/// file can only be damaged or made by hand, and is a miss.
const MAX_CACHE_FILE: u64 = 16 << 20;

/// The search's cached progress, or None when there is none to use: no file, a file that cannot be read, is over
/// `MAX_CACHE_FILE` or does not parse, or one whose format, app version or search key is not this one's. Never an
/// error, and never waits on a file another program holds open (that is a miss too).
pub fn load_shortlist(dir: &Path, app_version: &str, search_key: &str) -> Option<CachedShortlist> {
    let mut bytes = Vec::new();
    fs::File::open(cache_file(dir, app_version, search_key))
        .ok()?
        .take(MAX_CACHE_FILE + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_CACHE_FILE {
        return None;
    }
    let file: CacheFile = serde_json::from_slice(&bytes).ok()?;
    let matches = file.format == CACHE_FORMAT && file.app_version == app_version && file.search_key == search_key;
    matches.then_some(CachedShortlist {
        next_seed: file.next_seed,
        checked: file.checked,
        fit_seeds: file.fit_seeds,
        other_seeds: file.other_seeds,
    })
}

/// Saves the search's progress (creating the folder if it is missing, writing over whatever file was there), then
/// deletes the oldest cache files beyond `CACHE_FILES`. An error means the file was not saved; a file that could not
/// be pruned is left for the next save, and is not an error.
pub fn save_shortlist(dir: &Path, app_version: &str, search_key: &str, c: &CachedShortlist) -> std::io::Result<()> {
    let path = cache_file(dir, app_version, search_key);
    let file = CacheFile {
        format: CACHE_FORMAT,
        app_version: app_version.to_string(),
        search_key: search_key.to_string(),
        next_seed: c.next_seed,
        checked: c.checked,
        fit_seeds: c.fit_seeds.clone(),
        other_seeds: c.other_seeds.clone(),
    };
    write_atomic(&path, &serde_json::to_vec(&file)?)?;
    prune(dir, &path);
    Ok(())
}

/// Whether `name` is one the cache makes: 16 lower-case hex digits and `.json`.
fn is_cache_name(name: &OsString) -> bool {
    name.to_str()
        .and_then(|name| name.strip_suffix(".json"))
        .is_some_and(|stem| stem.len() == 16 && stem.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// Deletes the oldest cache files in `dir` beyond `CACHE_FILES`: oldest by modified time, the name breaking ties, so
/// the choice is the same on every run. Only plain files with the cache's own names are counted or deleted (never a
/// folder or a link, which is not followed), and `keep`, the file just written, is never deleted. If `dir` itself is
/// a link, nothing is deleted. Best effort: a file that cannot be deleted is left.
fn prune(dir: &Path, keep: &Path) {
    if !fs::symlink_metadata(dir).is_ok_and(|meta| meta.is_dir()) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, OsString)> = entries
        .filter_map(Result::ok)
        .filter(|entry| is_cache_name(&entry.file_name()))
        // A `DirEntry`'s file type does not follow links.
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| {
            // A file whose time cannot be read counts as the oldest.
            let modified = entry.metadata().and_then(|meta| meta.modified()).unwrap_or(UNIX_EPOCH);
            (modified, entry.file_name())
        })
        .collect();
    if files.len() <= CACHE_FILES {
        return;
    }
    let excess = files.len() - CACHE_FILES;
    files.retain(|(_, name)| Some(name.as_os_str()) != keep.file_name());
    files.sort();
    for (_, name) in files.into_iter().take(excess) {
        let _ = fs::remove_file(dir.join(name));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::{TempRoot, files_in};
    use super::*;

    const VERSION: &str = "0.1.0";
    const KEY: &str = "flute_vs_trombone h1";

    fn sample() -> CachedShortlist {
        CachedShortlist {
            next_seed: 18_434,
            checked: 18_433,
            fit_seeds: vec![392, 4_471, 18_433],
            other_seeds: vec![7, 1 << 40, u64::MAX],
        }
    }

    fn set_modified(path: &Path, secs: u64) {
        let file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
    }

    /// `n` as a cache file name: 16 hex digits and `.json`.
    fn hex_name(n: usize) -> String {
        format!("{n:016x}.json")
    }

    #[test]
    fn cache_round_trip() {
        let root = TempRoot::new("cache_round_trip");
        let dir = root.0.join("shortlists");
        assert_eq!(load_shortlist(&dir, VERSION, KEY), None);
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(sample()));
        let newer = CachedShortlist {
            next_seed: 20_000,
            ..sample()
        };
        save_shortlist(&dir, VERSION, KEY, &newer).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(newer));
        assert_eq!(files_in(&dir), vec!["c0235a4f5c3232d6.json"]);
    }

    #[test]
    fn another_version_is_a_miss() {
        let root = TempRoot::new("another_version_is_a_miss");
        let dir = root.0.join("shortlists");
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        assert_eq!(load_shortlist(&dir, "0.2.0", KEY), None);
        // Even a file sitting under the other version's name is a miss when its header names this version.
        fs::copy(cache_file(&dir, VERSION, KEY), cache_file(&dir, "0.2.0", KEY)).unwrap();
        assert_eq!(load_shortlist(&dir, "0.2.0", KEY), None);
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(sample()));
    }

    #[test]
    fn another_key_is_a_miss() {
        let root = TempRoot::new("another_key_is_a_miss");
        let dir = root.0.join("shortlists");
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, "tuba_vs_bass_drum h1"), None);
        fs::copy(
            cache_file(&dir, VERSION, KEY),
            cache_file(&dir, VERSION, "tuba_vs_bass_drum h1"),
        )
        .unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, "tuba_vs_bass_drum h1"), None);
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(sample()));
    }

    #[test]
    fn a_corrupt_cache_is_a_miss() {
        let root = TempRoot::new("a_corrupt_cache_is_a_miss");
        let dir = root.0.join("shortlists");
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        let path = cache_file(&dir, VERSION, KEY);
        let good = fs::read(&path).unwrap();
        let text = String::from_utf8(good.clone()).unwrap();
        let other_format = text.replace("\"format\":1", "\"format\":2");
        assert_ne!(other_format, text);
        let missing_field = text.replace("\"checked\"", "\"looked\"");
        assert_ne!(missing_field, text);
        let corrupt: [&[u8]; 6] = [
            b"",
            b"not json at all",
            &good[..good.len() / 2],
            &[0xff, 0xfe, 0x00, 0x7b],
            other_format.as_bytes(),
            missing_field.as_bytes(),
        ];
        for bytes in corrupt {
            fs::write(&path, bytes).unwrap();
            assert_eq!(load_shortlist(&dir, VERSION, KEY), None, "{bytes:?}");
        }
        // A folder where the file should be is a miss too.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, KEY), None);
        fs::remove_dir(&path).unwrap();
        // A corrupt file is written over by the next save and never kept aside.
        fs::write(&path, b"{\"format\":").unwrap();
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(sample()));
        assert_eq!(files_in(&dir), vec!["c0235a4f5c3232d6.json"]);
    }

    #[test]
    fn an_oversized_cache_is_a_miss() {
        let root = TempRoot::new("an_oversized_cache_is_a_miss");
        let dir = root.0.join("shortlists");
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        let path = cache_file(&dir, VERSION, KEY);
        let mut bytes = fs::read(&path).unwrap();
        // Trailing spaces keep it valid JSON: exactly the limit is read, one byte more is a miss.
        bytes.resize(MAX_CACHE_FILE as usize, b' ');
        fs::write(&path, &bytes).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(sample()));
        bytes.push(b' ');
        fs::write(&path, &bytes).unwrap();
        assert_eq!(load_shortlist(&dir, VERSION, KEY), None);
    }

    #[test]
    fn the_file_name_is_16_hex_digits() {
        let dir = Path::new(r"C:\data dir\cache\shortlists");
        let path = cache_file(dir, VERSION, KEY);
        assert_eq!(path.parent(), Some(dir));
        // sha256("0.1.0\nflute_vs_trombone h1") starts c0235a4f5c3232d6.
        assert_eq!(path.file_name().unwrap(), "c0235a4f5c3232d6.json");
        for (version, key) in [("0.1.0", ""), ("", ""), ("0.2.0", "Zoë vs Ångström")] {
            let name = cache_file(dir, version, key).file_name().unwrap().to_os_string();
            assert!(is_cache_name(&name), "{name:?}");
        }
        // The line break keeps the version and the key apart.
        assert_ne!(cache_file(dir, "0.1", ".0\nkey"), cache_file(dir, "0.1.0", "key"));
    }

    #[test]
    fn old_files_are_pruned() {
        let root = TempRoot::new("old_files_are_pruned");
        let dir = root.0.join("shortlists");
        fs::create_dir_all(&dir).unwrap();
        // CACHE_FILES + 2 files already there, each a second newer than the one before; the save makes CACHE_FILES + 3.
        for n in 0..CACHE_FILES + 2 {
            let path = dir.join(hex_name(n));
            fs::write(&path, b"{}").unwrap();
            set_modified(&path, 1_700_000_000 + n as u64);
        }
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        let left = files_in(&dir);
        assert_eq!(left.len(), CACHE_FILES);
        for n in 0..3 {
            assert!(!left.contains(&hex_name(n)), "{} was kept", hex_name(n));
        }
        assert!(left.contains(&hex_name(3)));
        assert!(left.contains(&"c0235a4f5c3232d6.json".to_string()));
        assert_eq!(load_shortlist(&dir, VERSION, KEY), Some(sample()));
    }

    #[test]
    fn the_file_just_written_is_never_pruned_and_ties_go_by_name() {
        let root = TempRoot::new("the_file_just_written_is_never_pruned_and_ties_go_by_name");
        let dir = root.0.join("shortlists");
        fs::create_dir_all(&dir).unwrap();
        // Every other file is dated in 2100, so the one just written is the oldest; they all share one time, so the
        // lowest names go.
        for n in (0..CACHE_FILES + 2).rev() {
            let path = dir.join(hex_name(n));
            fs::write(&path, b"{}").unwrap();
            set_modified(&path, 4_102_444_800);
        }
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        let left = files_in(&dir);
        assert_eq!(left.len(), CACHE_FILES);
        assert!(left.contains(&"c0235a4f5c3232d6.json".to_string()));
        for n in 0..3 {
            assert!(!left.contains(&hex_name(n)), "{} was kept", hex_name(n));
        }
        for n in 3..CACHE_FILES + 2 {
            assert!(left.contains(&hex_name(n)), "{} was pruned", hex_name(n));
        }
    }

    #[test]
    fn pruning_touches_only_the_caches_own_files() {
        let root = TempRoot::new("pruning_touches_only_the_caches_own_files");
        let dir = root.0.join("shortlists");
        fs::create_dir_all(&dir).unwrap();
        // Files that are not the cache's, older than everything else.
        let strangers = [
            "notes.txt",
            "ABCDEF0123456789.json",
            "0123456789abcde.json",
            "0123456789abcdef0.json",
            "0123456789abcdef.json.tmp",
            "0123456789abcdeg.json",
        ];
        for name in strangers {
            let path = dir.join(name);
            fs::write(&path, b"mine").unwrap();
            set_modified(&path, 1_000);
        }
        // A folder with a cache file's name, holding a file of its own.
        let folder = dir.join("0000000000000000.json");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("inside.txt"), b"mine").unwrap();
        for n in 1..CACHE_FILES + 3 {
            let path = dir.join(hex_name(n));
            fs::write(&path, b"{}").unwrap();
            set_modified(&path, 1_700_000_000 + n as u64);
        }
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        let left = files_in(&dir);
        for name in strangers {
            assert!(left.contains(&name.to_string()), "{name} was deleted");
            assert_eq!(fs::read(dir.join(name)).unwrap(), b"mine");
        }
        assert_eq!(fs::read(folder.join("inside.txt")).unwrap(), b"mine");
        for n in 1..4 {
            assert!(!left.contains(&hex_name(n)), "{} was kept", hex_name(n));
        }
        assert_eq!(left.len(), strangers.len() + 1 + CACHE_FILES);
    }

    #[cfg(windows)]
    #[test]
    fn pruning_never_follows_a_link_out_of_the_folder() {
        let root = TempRoot::new("pruning_never_follows_a_link_out_of_the_folder");
        let dir = root.0.join("shortlists");
        fs::create_dir_all(&dir).unwrap();
        let outside = root.0.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("0000000000000001.json"), b"mine").unwrap();
        set_modified(&outside.join("0000000000000001.json"), 1_000);
        // A junction (no admin rights needed) with a cache file's name, pointing out of the folder.
        let link = dir.join("0000000000000000.json");
        let made = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
        for n in 2..CACHE_FILES + 4 {
            let path = dir.join(hex_name(n));
            fs::write(&path, b"{}").unwrap();
            set_modified(&path, 1_700_000_000 + n as u64);
        }
        save_shortlist(&dir, VERSION, KEY, &sample()).unwrap();
        assert_eq!(fs::read(outside.join("0000000000000001.json")).unwrap(), b"mine");
        assert!(fs::symlink_metadata(&link).is_ok());
        assert_eq!(files_in(&dir).len(), 1 + CACHE_FILES);
        fs::remove_dir(&link).unwrap();
    }
}
