//! Staging (plan M6, Task 12): the built exe, with the tagged tree's icon and guide and the ffmpeg of that version's
//! own pin, is packaged into the program folder's `.next` by `bf_pack::package::package`, then checked there: the
//! package file by file against its manifest, and the new exe by what it says to `--version`. Nothing in the
//! program folder outside `.next` is touched; a check that fails removes `.next` again (only its own files).
//!
//! A `.next` left by an earlier stage is cleared first, only by its own records: a finished one by its manifest, one
//! stopped part-way (a power cut) by the list `package` writes before anything else (`clear_unfinished`). Anything
//! in it that is not ours stops the stage in words, and stays.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use bf_pack::PackError;
use bf_pack::ffmpeg::{FfmpegPin, parse_pin};
use bf_pack::package::{
    EXE_NAME, MANIFEST_NAME, check_kept_package, check_package, clear_unfinished, package, read_manifest,
};

use super::{Installed, Release};

/// The folder an update is staged in, inside the program folder.
pub const NEXT: &str = ".next";

/// How long the staged exe may take to say its version.
const VERSION_LIMIT: Duration = Duration::from_secs(10);

/// Stages `exe`, built from `tree` (the folder that holds `studio\`), into `<inst.dir>\.next`, with the ffmpeg that
/// `tree`'s pin names from the install's downloads folder (downloaded only when that pin is not there yet, with
/// Windows' own curl, and checked against the pin's hashes). Gives the `.next` folder.
pub fn stage(inst: &Installed, exe: &Path, tree: &Path) -> Result<PathBuf, String> {
    let downloads = inst.config.downloads.clone();
    stage_with(inst, exe, tree, &|pin| {
        bf_pack::ffmpeg::ensure(pin, &downloads, &bf_pack::fetch::Curl)
    })
}

/// `stage`, with where the checked ffmpeg files come from given (tests).
pub fn stage_with(
    inst: &Installed,
    exe: &Path,
    tree: &Path,
    ffmpeg: &dyn Fn(&FfmpegPin) -> Result<PathBuf, PackError>,
) -> Result<PathBuf, String> {
    if bf_pack::install::swap_unfinished(&inst.dir) {
        return Err(
            "An earlier update is still being put in place. Start BALLFIGHT Studio again to finish it.".to_string(),
        );
    }
    clear_next(&inst.dir)?;
    let pin_path = tree.join("studio").join("packaging").join("ffmpeg.toml");
    let pin_text =
        std::fs::read_to_string(&pin_path).map_err(|e| format!("{} could not be read: {e}.", pin_path.display()))?;
    let pin = parse_pin(&pin_text).map_err(|e| e.to_string())?;
    let verified = ffmpeg(&pin).map_err(|e| e.to_string())?;
    let next = inst.dir.join(NEXT);
    package(exe, tree, &verified, &next).map_err(|e| e.to_string())?;
    Ok(next)
}

/// Clears `<dir>\.next` by its own records (see the module notes). Nothing there is fine.
pub fn clear_next(dir: &Path) -> Result<(), String> {
    let next = dir.join(NEXT);
    if std::fs::symlink_metadata(&next).is_err() {
        return Ok(());
    }
    match clear_unfinished(&next) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(e) => return Err(e.to_string()),
    }
    let stayed = bf_pack::remove::remove_by_manifest(&next).map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(&next).is_ok() {
        let names: Vec<String> = stayed.iter().map(|(p, _)| p.display().to_string()).collect();
        return Err(format!(
            "{} could not be cleared, because these stayed: {}.",
            next.display(),
            names.join(", ")
        ));
    }
    Ok(())
}

/// The version staged in `<dir>\.next`, by its manifest (not checked).
pub fn staged_version(dir: &Path) -> Option<String> {
    read_manifest(&dir.join(NEXT).join(MANIFEST_NAME))
        .ok()
        .map(|m| m.version)
}

/// Checks a staged update: every file against its manifest, the manifest's version, and the staged exe, which must
/// print `BALLFIGHT Studio <version> (<first 7 of the commit>, ...)` for `--version` within 10 s. On any failure the
/// stage is removed (by its own records) and the words say why.
pub fn check_staged(next: &Path, want: &Release) -> Result<(), String> {
    let commit = want.commit.get(..7).unwrap_or("");
    let checked = check_named(next, &want.version.to_string(), Some(commit), false);
    if checked.is_err()
        && next.file_name() == Some(OsStr::new(NEXT))
        && let Some(dir) = next.parent()
    {
        let _ = clear_next(dir);
    }
    checked
}

/// Checks a whole package in `dir` of `version` whose exe says so for `--version`, naming `commit` (its first 7 hex
/// digits) when given. `kept`: a version kept from before, which may also hold its copy of `update.toml`. Nothing is
/// removed.
pub fn check_named(dir: &Path, version: &str, commit: Option<&str>, kept: bool) -> Result<(), String> {
    let m = if kept {
        check_kept_package(dir)
    } else {
        check_package(dir)
    }
    .map_err(|e| e.to_string())?;
    if m.version != version {
        return Err(format!("Its files say version {}, not {version}.", m.version));
    }
    let want = match commit {
        Some(c) => format!("BALLFIGHT Studio {version} ({c}, "),
        None => format!("BALLFIGHT Studio {version} ("),
    };
    let never = AtomicBool::new(false);
    let exe = dir.join(EXE_NAME);
    let ran = bf_pack::run::run(&exe, &[OsStr::new("--version")], VERSION_LIMIT, &never)
        .map_err(|e| format!("The new program did not say its version: {}", e.trim_end_matches('.')))?;
    let text = String::from_utf8_lossy(&ran.stdout).trim().to_string();
    let first: String = text.lines().next().unwrap_or("").chars().take(120).collect();
    let named = commit.is_none_or(|c| c.len() == 7) && first.starts_with(&want) && first.ends_with(')');
    if !ran.status.success() || !named {
        let expected = match commit {
            Some(c) => format!("version {version} from commit {c}"),
            None => format!("version {version}"),
        };
        return Err(if first.is_empty() {
            format!("The new program did not say which version it is, where it should say {expected}.")
        } else {
            format!("The new program says \"{first}\", where it should say {expected}.")
        });
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::git::standin;
    use super::super::testkit::TempDir;
    use super::*;
    use bf_pack::config::UpdateConfig;
    use bf_pack::install::Layout;
    use bf_pack::version::parse_version;

    /// Version `version` of the stand-in installed under `root\pc` (its exe is a copy of `exe`, or plain bytes).
    pub(crate) fn installed(root: &Path, version: &str, exe: Option<&Path>) -> Installed {
        let tree = root.join(format!("installed-tree-{version}"));
        standin::write(&tree, version, &standin::Kind::default());
        let built = root.join(format!("installed-exe-{version}")).join(EXE_NAME);
        std::fs::create_dir_all(built.parent().unwrap()).unwrap();
        match exe {
            Some(from) => {
                std::fs::copy(from, &built).unwrap();
            }
            None => std::fs::write(&built, format!("pretend exe {version}")).unwrap(),
        }
        let checked = standin::checked_ffmpeg(&root.join("checked-ffmpeg"));
        let pkg = root.join(format!("installed-package-{version}"));
        package(&built, &tree, &checked, &pkg).unwrap();
        let layout = Layout::under(&root.join("pc"));
        let config = UpdateConfig {
            source: root.join("pc").join("data").join("source"),
            remote: "https://example.invalid/someone/repo.git".into(),
            tag_prefix: "studio-v".into(),
            build_dir: root.join("pc").join("data").join("build"),
            downloads: root.join("pc").join("data").join("downloads"),
        };
        bf_pack::install::install(&pkg, &layout, &config).unwrap();
        Installed {
            dir: layout.programs,
            config,
        }
    }

    /// Every file under `dir` with its bytes, for before-and-after comparisons.
    pub(crate) fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &Path, base: &str, out: &mut Vec<(String, Vec<u8>)>) {
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for e in entries {
                let e = e.unwrap();
                let rel = format!("{base}/{}", e.file_name().to_string_lossy());
                if e.file_type().unwrap().is_dir() {
                    out.push((format!("{rel}/"), Vec::new()));
                    walk(&e.path(), &rel, out);
                } else {
                    out.push((rel, std::fs::read(e.path()).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, "", &mut out);
        out.sort();
        out
    }

    fn ffmpeg_from(dir: PathBuf) -> impl Fn(&FfmpegPin) -> Result<PathBuf, PackError> {
        move |_| Ok(dir.clone())
    }

    fn release(version: &str, commit: &str) -> Release {
        Release {
            tag: format!("studio-v{version}"),
            version: parse_version(version).unwrap(),
            commit: commit.to_string(),
        }
    }

    fn ping() -> PathBuf {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32")
            .join("PING.EXE")
    }

    #[test]
    fn a_staged_exe_must_name_the_tag() {
        let dir = TempDir::new("stage-name");
        let inst = installed(dir.path(), "0.6.0", None);
        let before = snapshot(&inst.dir);
        let tree = dir.path().join("tree-0.6.1");
        standin::write(&tree, "0.6.1", &standin::Kind::default());
        // A program that does not answer as the app does (Windows' ping) is staged and refused.
        let checked = standin::checked_ffmpeg(&dir.path().join("ff"));
        let next = stage_with(&inst, &ping(), &tree, &ffmpeg_from(checked)).unwrap();
        assert_eq!(next, inst.dir.join(NEXT));
        check_package(&next).unwrap();
        assert_eq!(staged_version(&inst.dir).as_deref(), Some("0.6.1"));
        let err = check_staged(&next, &release("0.6.1", &"a".repeat(40))).unwrap_err();
        assert!(
            err.contains("where it should say version 0.6.1 from commit aaaaaaa"),
            "{err}"
        );
        assert!(!next.exists(), "a refused stage is removed");
        assert_eq!(snapshot(&inst.dir), before, "the installed version is untouched");
    }

    #[test]
    fn a_stage_of_another_version_is_refused() {
        let dir = TempDir::new("stage-version");
        let inst = installed(dir.path(), "0.6.0", None);
        let tree = dir.path().join("tree-0.6.2");
        standin::write(&tree, "0.6.2", &standin::Kind::default());
        let checked = standin::checked_ffmpeg(&dir.path().join("ff"));
        let next = stage_with(&inst, &ping(), &tree, &ffmpeg_from(checked)).unwrap();
        let err = check_staged(&next, &release("0.6.1", &"b".repeat(40))).unwrap_err();
        assert_eq!(err, "Its files say version 0.6.2, not 0.6.1.");
        assert!(!next.exists());
    }

    #[test]
    fn an_earlier_stage_is_cleared_only_by_its_own_records() {
        let dir = TempDir::new("stage-earlier");
        let inst = installed(dir.path(), "0.6.0", None);
        let checked = standin::checked_ffmpeg(&dir.path().join("ff"));
        let tree = dir.path().join("tree-0.6.1");
        standin::write(&tree, "0.6.1", &standin::Kind::default());
        let fake_exe = dir.path().join("fake").join(EXE_NAME);
        std::fs::create_dir_all(fake_exe.parent().unwrap()).unwrap();
        std::fs::write(&fake_exe, b"pretend exe 0.6.1").unwrap();
        // A finished stage of an earlier attempt is replaced.
        stage_with(&inst, &fake_exe, &tree, &ffmpeg_from(checked.clone())).unwrap();
        std::fs::write(&fake_exe, b"pretend exe 0.6.1, built again").unwrap();
        let next = stage_with(&inst, &fake_exe, &tree, &ffmpeg_from(checked.clone())).unwrap();
        assert_eq!(
            std::fs::read(next.join(EXE_NAME)).unwrap(),
            b"pretend exe 0.6.1, built again"
        );
        // A stage stopped part-way (its list, and the exe copied) is cleared by its list.
        clear_next(&inst.dir).unwrap();
        std::fs::create_dir_all(&next).unwrap();
        std::fs::write(
            next.join("PACKING.txt"),
            "BALLFIGHT Studio package being made 1\r\nF ballfight-studio.exe\r\nF MANIFEST.txt\r\n",
        )
        .unwrap();
        std::fs::write(next.join(EXE_NAME), b"half an exe").unwrap();
        stage_with(&inst, &fake_exe, &tree, &ffmpeg_from(checked.clone())).unwrap();
        check_package(&next).unwrap();
        // A file of a person's in it stops the stage, and stays.
        std::fs::write(next.join("notes.txt"), b"mine").unwrap();
        let err = stage_with(&inst, &fake_exe, &tree, &ffmpeg_from(checked)).unwrap_err();
        assert!(err.contains("could not be cleared"), "{err}");
        assert_eq!(std::fs::read(next.join("notes.txt")).unwrap(), b"mine");
    }

    #[test]
    fn no_ffmpeg_no_stage() {
        let dir = TempDir::new("stage-ffmpeg");
        let inst = installed(dir.path(), "0.6.0", None);
        let before = snapshot(&inst.dir);
        let tree = dir.path().join("tree-0.6.1");
        standin::write(&tree, "0.6.1", &standin::Kind::default());
        let refuse = |_: &FfmpegPin| -> Result<PathBuf, PackError> {
            Err(PackError::Download(
                "The ffmpeg 9.0.2 archive could not be downloaded from https://example.org: no network.".into(),
            ))
        };
        let err = stage_with(&inst, &ping(), &tree, &refuse).unwrap_err();
        assert!(err.contains("could not be downloaded"), "{err}");
        assert_eq!(snapshot(&inst.dir), before);
    }
}
