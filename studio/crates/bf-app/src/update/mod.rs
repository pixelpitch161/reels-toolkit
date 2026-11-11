//! The update engine (plan M6, Task 12; decision D1, route A: the app builds the newest release on this PC).
//!
//! **What makes a new version acceptable** (D1, route A), and nothing else:
//! - it comes from the repository the install recorded (`update.toml`'s `remote`), fetched by Git into the app's
//!   own copy of the source (`git::GitSource`, which refuses a copy whose `origin` is anything else, and never uses a
//!   working clone of a person's);
//! - its tag has exactly the form `studio-v<version>` (`newer`, `git::parse_tags`), and the version follows the D1
//!   rule (a release after its candidates, a candidate only for a candidate);
//! - it is built from exactly the commit that tag names, in a detached checkout of that copy, with
//!   `cargo build --release --locked` (cargo checks every crate's checksum against `Cargo.lock`);
//! - the built program reports that version and that commit through `--version` (`stage::check_staged`);
//! - the staged package verifies file by file against its manifest, and its ffmpeg against the hashes pinned in that
//!   version's own `ffmpeg.toml` (`bf_pack::package::check_package`, `bf_pack::ffmpeg::place`).
//!
//! **Nothing running is ever destroyed.** A build is packaged into the program folder's `.next` and checked there;
//! only after a restart does the new version, from `.next`, swap itself in (`swap::finish`, by `bf_pack`'s
//! install swap: only files the manifest lists move, the marker, `.previous` kept for "Go back"). Every failure
//! before that leaves the running version as it is, and says so in a plain sentence.
//!
//! The functions and texts here are for the start-up (Task 9: `swap::finish`, `swap::at_start`) and the Settings
//! screen and toolbar (Task 13: `worker::Worker`, `UpdateView`, the texts below).

pub mod git;
pub mod stage;
pub mod swap;
pub mod worker;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use bf_pack::config::{CONFIG_NAME, UpdateConfig, read_config};
use bf_pack::package::{EXE_NAME, MANIFEST_NAME, read_manifest};
use bf_pack::version::Version;

/// An installed copy of the app: its program folder and its `update.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub dir: PathBuf,
    pub config: UpdateConfig,
}

/// The running copy, when it is an installed one (its exe is `ballfight-studio.exe` in a folder with our
/// `MANIFEST.txt` and a readable `update.toml`). `None` for a build in `target\` or anywhere else.
pub fn installed() -> Option<Installed> {
    installed_at(&std::env::current_exe().ok()?)
}

/// `installed`, for the exe at `exe`.
pub fn installed_at(exe: &Path) -> Option<Installed> {
    if !exe.file_name()?.to_string_lossy().eq_ignore_ascii_case(EXE_NAME) {
        return None;
    }
    let dir = exe.parent()?.to_path_buf();
    read_manifest(&dir.join(MANIFEST_NAME)).ok()?;
    let config = read_config(&dir.join(CONFIG_NAME)).ok()?;
    Some(Installed { dir, config })
}

/// A release: its tag, the version the tag names, and the commit it points at (full hex).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub version: Version,
    pub commit: String,
}

/// What a build made: the new exe, and the source tree it was built from (the folder that holds `studio\`), from
/// which the icon, the guide and the ffmpeg pin are packaged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Built {
    pub exe: PathBuf,
    pub tree: PathBuf,
}

/// Where new versions come from. Route A is `git::GitSource`; a route that downloads would be another one.
pub trait Source: Send {
    /// Looks for releases and gives the one to update to from `current` (by `newer`), or `None`.
    fn newest(&self, current: &Version) -> Result<Option<Release>, String>;
    /// Makes the new exe for `r`. `progress` gets a plain sentence for each stage; `cancel` stops it.
    fn build(&self, r: &Release, progress: &dyn Fn(String), cancel: &AtomicBool) -> Result<Built, String>;
}

/// Where the update stands, as Settings and the toolbar show it.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdateState {
    NotInstalled,
    Checking,
    UpToDate,
    Available {
        version: String,
    },
    Building {
        version: String,
        seconds: u64,
        doing: String,
    },
    Ready {
        version: String,
    },
    Failed {
        text: String,
    },
}

/// Everything the interface shows about updates.
#[derive(Clone, Debug, PartialEq)]
pub struct UpdateView {
    pub state: UpdateState,
    /// The version kept in `.previous`, for "Go back to version {w}".
    pub previous: Option<String>,
    /// The fault reports folder.
    pub reports: Option<PathBuf>,
    /// The guide's page, when the installed copy has one.
    pub guide: Option<PathBuf>,
    /// Set once the app should save and close now: after "Restart now" or "Go back" started the program that
    /// finishes the swap, or after the app was removed. The sentences (if any) are shown before it closes.
    pub close: Option<Vec<String>>,
    /// A one-off sentence about the last request that changed no state (the guide could not be opened, a removal
    /// was refused, going back did not start), shown under the Updates row until the next request.
    pub note: Option<String>,
}

/// The D1 tag rule: the newest release above `current`; a candidate (`-rc.N`) only when `current` is itself a
/// candidate of the same version or an older one.
pub fn newer(current: &Version, tags: &[Release]) -> Option<Release> {
    let numbers = |v: &Version| (v.major, v.minor, v.patch);
    tags.iter()
        .filter(|r| r.version > *current)
        .filter(|r| r.version.rc.is_none() || (current.rc.is_some() && numbers(current) <= numbers(&r.version)))
        .max_by(|a, b| a.version.cmp(&b.version))
        .cloned()
}

/// The running version, as `bf_pack::version` reads it.
pub fn running_version() -> Option<Version> {
    bf_pack::version::parse_version(crate::version::VERSION)
}

/// When Git or Rust cannot be found.
pub const NO_TOOLS: &str = "This PC can no longer build updates, because Git (or Rust) is not installed. Ask for it to \
     be set up again.";
/// Why "Restart now" and "Go back" wait while a video renders (Task 13 refuses them then).
pub const RENDERING: &str = "A video is rendering. Restart when it is done, or cancel it first.";
/// When another copy of the app is updating the same install.
pub const ANOTHER_COPY: &str = "Another copy of BALLFIGHT Studio is updating right now, so this one waits. Try again \
     when it is done.";

/// A failed look for a newer version.
pub fn check_failed(why: &str) -> String {
    format!(
        "BALLFIGHT Studio could not look for a newer version, so nothing changed. {}",
        sentence(why)
    )
}

/// A build that failed (or the steps before it).
pub fn build_failed(version: &str, running: &str, why: &str) -> String {
    format!(
        "Version {version} could not be built on this PC, so nothing changed: this is still version {running}. {}",
        sentence(why)
    )
}

/// A built version that did not pass its checks.
pub fn check_staged_failed(version: &str, running: &str, why: &str) -> String {
    format!(
        "Version {version} was built, but the new program did not pass its checks, so nothing changed: this is still \
         version {running}. {}",
        sentence(why)
    )
}

/// A built version that could not be packaged (its ffmpeg could not be had, the disk is full, ...).
pub fn stage_failed(version: &str, running: &str, why: &str) -> String {
    format!(
        "Version {version} was built, but it could not be made ready to start, so nothing changed: this is still \
         version {running}. {}",
        sentence(why)
    )
}

/// The update could not be put in place at the restart (the swap undid itself).
pub fn swap_failed(version: &str, running: &str) -> String {
    format!(
        "The update to version {version} could not be put in place, so this is still version {running}. Nothing was lost."
    )
}

/// "Restart now" or "Go back" could not start the new program.
pub fn restart_failed(why: &str) -> String {
    format!(
        "The new version could not be started, so nothing changed. {}",
        sentence(why)
    )
}

/// `text` as one whole sentence: trimmed, with a capital first letter and a full stop at the end.
pub(crate) fn sentence(text: &str) -> String {
    let t = text.trim();
    let mut s: String = match t.chars().next() {
        Some(c) => c.to_uppercase().chain(t.chars().skip(1)).collect(),
        None => return String::new(),
    };
    if !s.ends_with(['.', '!', '?']) {
        s.push('.');
    }
    s
}

/// Test helpers shared by the update tests: temp folders, and local git repositories made by the tests.
#[cfg(test)]
pub(crate) mod testkit {
    use std::ffi::OsStr;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    /// A temp folder of the test's own under `%TEMP%\bf-app-update-tests`, removed when dropped.
    pub(crate) struct TempDir(PathBuf);

    impl TempDir {
        pub(crate) fn new(name: &str) -> TempDir {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join("bf-app-update-tests")
                .join(format!("{name}-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a temp folder for the test");
            TempDir(path)
        }

        pub(crate) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Git on this PC, or `None` (the git tests then say so and pass).
    pub(crate) fn git() -> Option<PathBuf> {
        bf_pack::install::find_git()
    }

    /// Runs git in `dir` (with its own `.git`) and gives what it printed; panics when it fails.
    pub(crate) fn git_in(g: &Path, dir: &Path, args: &[&str]) -> String {
        let never = AtomicBool::new(false);
        let mut all: Vec<&OsStr> = vec![OsStr::new("-C"), dir.as_os_str()];
        all.extend(args.iter().map(OsStr::new));
        let env: [(&str, &OsStr); 6] = [
            ("GIT_TERMINAL_PROMPT", OsStr::new("0")),
            ("GIT_AUTHOR_NAME", OsStr::new("Test")),
            ("GIT_AUTHOR_EMAIL", OsStr::new("test@example.invalid")),
            ("GIT_COMMITTER_NAME", OsStr::new("Test")),
            ("GIT_COMMITTER_EMAIL", OsStr::new("test@example.invalid")),
            ("GIT_CEILING_DIRECTORIES", dir.parent().unwrap_or(dir).as_os_str()),
        ];
        let ran = bf_pack::run::run_with_env(g, &all, &env, Duration::from_secs(60), &never)
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
        assert!(ran.status.success(), "git {args:?}: {:?}", ran.tail);
        String::from_utf8_lossy(&ran.stdout).trim().to_string()
    }

    /// A new repository at `dir` with no commits, on branch `main`, with no signing and plain line ends.
    pub(crate) fn new_repo(g: &Path, dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        git_in(g, dir, &["init", "-q", "-b", "main"]);
        git_in(g, dir, &["config", "commit.gpgsign", "false"]);
        git_in(g, dir, &["config", "tag.gpgsign", "false"]);
        git_in(g, dir, &["config", "core.autocrlf", "false"]);
    }

    /// Commits everything in `dir` and gives the new commit (full hex).
    pub(crate) fn commit_all(g: &Path, dir: &Path, message: &str) -> String {
        git_in(g, dir, &["add", "-A"]);
        git_in(g, dir, &["commit", "-q", "-m", message]);
        git_in(g, dir, &["rev-parse", "HEAD"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_pack::version::parse_version;

    fn v(s: &str) -> Version {
        parse_version(s).unwrap()
    }

    fn rel(s: &str) -> Release {
        Release {
            tag: format!("studio-v{s}"),
            version: v(s),
            commit: "a".repeat(40),
        }
    }

    fn offered(current: &str, tags: &[&str]) -> Option<String> {
        let tags: Vec<Release> = tags.iter().map(|t| rel(t)).collect();
        newer(&v(current), &tags).map(|r| r.version.to_string())
    }

    #[test]
    fn the_tag_rule() {
        // From a release, a newer release is offered and a candidate is not.
        assert_eq!(offered("0.6.0", &["0.6.1"]), Some("0.6.1".into()));
        assert_eq!(offered("0.6.0", &["0.6.1-rc.1"]), None);
        assert_eq!(
            offered("0.6.0", &["0.6.1-rc.1", "0.6.1", "0.7.0-rc.1"]),
            Some("0.6.1".into())
        );
        // From a candidate, the next candidate and the release are offered; the newest wins.
        assert_eq!(offered("0.6.1-rc.1", &["0.6.1-rc.2"]), Some("0.6.1-rc.2".into()));
        assert_eq!(offered("0.6.1-rc.1", &["0.6.1"]), Some("0.6.1".into()));
        assert_eq!(offered("0.6.1-rc.1", &["0.6.1-rc.2", "0.6.1"]), Some("0.6.1".into()));
        // A candidate of a later version is offered to an older candidate.
        assert_eq!(offered("0.6.1-rc.1", &["0.7.0-rc.1"]), Some("0.7.0-rc.1".into()));
        // Never the same or an older one.
        assert_eq!(offered("0.6.1", &["0.6.1", "0.6.0", "0.6.1-rc.3"]), None);
        assert_eq!(offered("0.6.1-rc.2", &["0.6.1-rc.1", "0.6.1-rc.2"]), None);
    }

    #[test]
    fn no_newer_tag_is_up_to_date() {
        assert_eq!(offered("0.6.0", &[]), None);
        assert_eq!(offered("1.0.0", &["0.6.0", "0.9.9", "1.0.0-rc.4"]), None);
    }

    #[test]
    fn not_installed_says_so() {
        // This test program lives in cargo's target folder: not an installed copy.
        assert_eq!(installed(), None);
        let dir = testkit::TempDir::new("not-installed");
        let exe = dir.path().join(EXE_NAME);
        std::fs::write(&exe, b"pretend").unwrap();
        assert_eq!(installed_at(&exe), None, "no manifest, no update.toml");
    }

    #[test]
    fn an_installed_copy_is_found() {
        let dir = testkit::TempDir::new("installed");
        let exe = dir.path().join(EXE_NAME);
        std::fs::write(&exe, b"pretend").unwrap();
        let m = bf_pack::package::Manifest {
            version: "0.6.0".into(),
            files: vec![(EXE_NAME.into(), 7, bf_pack::sha::sha256_file(&exe).unwrap())],
        };
        std::fs::write(dir.path().join(MANIFEST_NAME), m.text()).unwrap();
        let config = UpdateConfig {
            source: dir.path().join("data").join("source"),
            remote: "https://github.com/someone/repo.git".into(),
            tag_prefix: "studio-v".into(),
            build_dir: dir.path().join("data").join("build"),
            downloads: dir.path().join("data").join("downloads"),
        };
        bf_pack::config::write_config(&dir.path().join(CONFIG_NAME), &config).unwrap();
        let found = installed_at(&exe).unwrap();
        assert_eq!(found.dir, dir.path());
        assert_eq!(found.config, config);
        // Another exe name in the same folder is not the app.
        assert_eq!(installed_at(&dir.path().join("other.exe")), None);
    }

    #[test]
    fn the_texts_are_whole_sentences() {
        let texts = [
            NO_TOOLS.to_string(),
            RENDERING.to_string(),
            ANOTHER_COPY.to_string(),
            check_failed("git said no"),
            build_failed("0.6.1", "0.6.0", "error: could not compile `bf-app`"),
            check_staged_failed("0.6.1", "0.6.0", "the program named 0.6.0"),
            stage_failed("0.6.1", "0.6.0", "the disk is full"),
            swap_failed("0.6.1", "0.6.0"),
            restart_failed("it could not be started"),
        ];
        for t in &texts {
            assert!(t.chars().next().is_some_and(char::is_uppercase), "{t}");
            assert!(t.ends_with('.'), "{t}");
            assert!(!t.contains("Err(") && !t.contains("Some("), "{t}");
        }
        assert_eq!(sentence("  done "), "Done.");
        assert_eq!(sentence("Already."), "Already.");
        assert_eq!(sentence(""), "");
    }
}
