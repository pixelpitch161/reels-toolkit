//! Route A (plan M6, D1): new versions come from the app's own copy of the source, `<data>\source`, made by the
//! install with `git clone --no-checkout` of the owner's repository. A check fetches the release tags into that copy;
//! a build checks out exactly the tagged commit there (detached) and runs `cargo build --release --locked -p bf-app`
//! in its `studio` folder, with the build folder `<data>\build` as cargo's target folder, at below-normal priority.
//!
//! **Git is only ever pointed at the app's own copy.** Every git run names the copy's `.git` folder and work tree
//! explicitly (`--git-dir`, `--work-tree`), runs in the copy, has `GIT_CEILING_DIRECTORIES` set to the copy's parent,
//! and has every `GIT_` variable of the app's own environment taken away before its own are added. No hook and no
//! file-system monitor runs (`core.hooksPath` is an empty folder of the app's under the build folder,
//! `core.fsmonitor=false`), and git may reach the repository only by https or ssh, with TLS checked. Git signs in
//! by itself (its credential manager); the app never sees a credential, and `GIT_TERMINAL_PROMPT=0` keeps git from
//! waiting on a console that is not there. Only an https or ssh address is taken as where updates come from.
//!
//! **The copy is never a person's working clone.** Before any git command that changes it, the copy must have its
//! own `.git` folder (not a link, not a worktree's `.git` file), its `origin` in its own settings (`config --local
//! --no-includes`) must be exactly the `remote` that `update.toml` records, and it must carry the mark
//! `ballfightstudio.updatecopy = true` in its own settings (never one from the user's or the PC's settings). The mark
//! is set only on a copy that is still exactly what the install made: a `--no-checkout` clone whose work tree holds
//! nothing but `.git` and whose index is empty. A clone someone works in has files and an index, never gets the mark,
//! and is refused in words with nothing changed.
//!
//! **No git command destroys work.** A build checks out the tagged commit only when the copy is clean (`status
//! --porcelain --untracked-files=all --ignored` prints nothing) or still as the install made it. A marked copy with
//! anything else in it is never cleaned: it is renamed aside to `source.set-aside-N` (never deleted), a fresh
//! `--no-checkout` copy is made in its place, and the words say so. Nothing in a copy is ever deleted.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use bf_pack::config::{TAG_PREFIX, UpdateConfig, check_remote};
use bf_pack::package::{EXE_NAME, is_link};
use bf_pack::run::{Family, run_family};
use bf_pack::version::{Version, parse_version};

use super::{ANOTHER_COPY, Built, NO_TOOLS, Release, Source, newer, sentence};

/// Route A: the app's own copy of the source, git and cargo.
pub struct GitSource {
    config: UpdateConfig,
    git: PathBuf,
    cargo: PathBuf,
    /// Tests only (set by `for_tests`, which exists only in test builds): the repository may be a local folder.
    local: bool,
    /// Tests only: variables added to each git child after its `GIT_` variables were taken away.
    test_env: Vec<(String, OsString)>,
}

/// The longest a fetch may take (a sign-in window may be waiting for the owner).
const FETCH_LIMIT: Duration = Duration::from_secs(10 * 60);
/// The longest a fresh copy of the source may take.
const CLONE_LIMIT: Duration = Duration::from_secs(30 * 60);
/// The longest any other git command may take.
const GIT_LIMIT: Duration = Duration::from_secs(5 * 60);
/// The longest a build may take (a first build that also installs a new Rust toolchain is the slow case).
const BUILD_LIMIT: Duration = Duration::from_secs(3 * 60 * 60);

/// The git setting that marks the app's own copy of the source.
const MARK: &str = "ballfightstudio.updatecopy";

/// What the build takes away from cargo's environment as well: settings that would make it build something other
/// than what the tagged source says (another toolchain than its `rust-toolchain.toml`, other compiler flags or
/// profile values, a wrapper around the compiler). Every `GIT_` variable goes too.
const CARGO_ENV_REMOVED: [&str; 18] = [
    "RUSTUP_TOOLCHAIN",
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_BUILD_TARGET",
    "CARGO_BUILD_TARGET_DIR",
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "RUSTC_BOOTSTRAP",
    "CARGO_INCREMENTAL",
    "CARGO_PROFILE_RELEASE_DEBUG",
    "CARGO_PROFILE_RELEASE_LTO",
    "CARGO_PROFILE_RELEASE_OPT_LEVEL",
    "CARGO_PROFILE_RELEASE_CODEGEN_UNITS",
    "CARGO_PROFILE_RELEASE_STRIP",
    "CARGO_PROFILE_RELEASE_PANIC",
];

/// What `build` holds until the new version is staged: the update lock.
pub struct BuildHold(#[allow(dead_code)] File);

impl GitSource {
    /// Route A with the git and cargo of this PC, for an https or ssh repository only. Without git or cargo:
    /// `NO_TOOLS`.
    pub fn new(c: UpdateConfig) -> Result<GitSource, String> {
        check_remote(&c.remote).map_err(|e| e.to_string())?;
        let git = bf_pack::install::find_git().ok_or_else(|| NO_TOOLS.to_string())?;
        let cargo = find_cargo().ok_or_else(|| NO_TOOLS.to_string())?;
        Ok(GitSource {
            config: c,
            git,
            cargo,
            local: false,
            test_env: Vec::new(),
        })
    }

    /// Tests only: route A on a local repository the test made, with the given programs. It is compiled only into
    /// test builds, so no path of the app can reach it.
    #[cfg(test)]
    pub(crate) fn for_tests(config: UpdateConfig, git: PathBuf, cargo: PathBuf) -> GitSource {
        GitSource {
            config,
            git,
            cargo,
            local: true,
            test_env: Vec::new(),
        }
    }

    fn source(&self) -> &Path {
        &self.config.source
    }

    /// The empty folder of the app's that git is told holds its hooks, so that none runs.
    fn no_hooks(&self) -> Result<PathBuf, String> {
        let d = self.config.build_dir.join("no-hooks");
        std::fs::create_dir_all(&d).map_err(|e| format!("The folder {} could not be made: {e}.", d.display()))?;
        Ok(d)
    }

    /// Git's options for every run: no hooks, no file-system monitor, https and ssh only, TLS checked (and, in
    /// tests, local repositories).
    fn safe_options(&self) -> Result<Vec<OsString>, String> {
        let mut hooks = OsString::from("core.hooksPath=");
        hooks.push(self.no_hooks()?);
        let mut out: Vec<OsString> = vec!["-c".into(), hooks, "-c".into(), "core.fsmonitor=false".into()];
        out.extend(bf_pack::install::PROTOCOLS.iter().map(OsString::from));
        if self.local {
            out.extend(["-c".into(), "protocol.file.allow=always".into()]);
        }
        Ok(out)
    }

    /// Git with `args`, run in `dir` with every `GIT_` variable of the app's own environment taken away.
    fn run_git(&self, dir: &Path, args: &[&OsStr], limit: Duration, cancel: &AtomicBool) -> Result<String, String> {
        let ceiling = dir.parent().unwrap_or(dir).as_os_str();
        let mut env: Vec<(&str, &OsStr)> = vec![
            ("GIT_TERMINAL_PROMPT", OsStr::new("0")),
            ("GIT_CEILING_DIRECTORIES", ceiling),
        ];
        env.extend(self.test_env.iter().map(|(k, v)| (k.as_str(), v.as_os_str())));
        let how = Family {
            dir: Some(dir),
            env: &env,
            scrub_git: true,
            ..Family::default()
        };
        let ran = run_family(&self.git, args, &how, limit, cancel, &mut |_| {})?;
        if ran.status.success() {
            Ok(String::from_utf8_lossy(&ran.stdout).trim().to_string())
        } else {
            let said = ran.tail.last().cloned().unwrap_or_else(|| "nothing".to_string());
            Err(format!("Git said \"{}\"", said.trim().trim_end_matches('.')))
        }
    }

    /// Git with `args`, in the app's copy and pointed only at it (see the module notes). Gives what it printed,
    /// trimmed, or the last thing it said on failure.
    fn git(&self, args: &[&OsStr], limit: Duration, cancel: &AtomicBool) -> Result<String, String> {
        let src = self.source();
        let mut git_dir = OsString::from("--git-dir=");
        git_dir.push(src.join(".git"));
        let mut work_tree = OsString::from("--work-tree=");
        work_tree.push(src);
        let options = self.safe_options()?;
        let mut all: Vec<&OsStr> = vec![git_dir.as_os_str(), work_tree.as_os_str()];
        all.extend(options.iter().map(OsString::as_os_str));
        all.extend_from_slice(args);
        self.run_git(src, &all, limit, cancel)
    }

    /// A value from the copy's own settings file only: never the user's, the PC's, an included file's or one given
    /// in the environment.
    fn own_setting(&self, name: &str) -> Option<String> {
        let never = AtomicBool::new(false);
        let o = OsStr::new;
        self.git(
            &[o("config"), o("--local"), o("--no-includes"), o("--get"), o(name)],
            GIT_LIMIT,
            &never,
        )
        .ok()
    }

    /// Whether the copy is exactly as the install made it: nothing but `.git` in its folder, and an empty index.
    fn pristine(&self) -> bool {
        let never = AtomicBool::new(false);
        let only_git = std::fs::read_dir(self.source())
            .map(|entries| entries.into_iter().all(|e| e.is_ok_and(|e| e.file_name() == ".git")))
            .unwrap_or(false);
        only_git && self.git(&[OsStr::new("ls-files")], GIT_LIMIT, &never).as_deref() == Ok("")
    }

    /// Checks that the copy is the app's own (see the module notes), marking a fresh copy the first time.
    fn own_copy(&self) -> Result<(), String> {
        let never = AtomicBool::new(false);
        let src = self.source();
        let not_ours = |why: &str| {
            format!(
                "{} is not the app's own copy of the source ({why}), so it is not used for updates.",
                src.display()
            )
        };
        if self.config.tag_prefix != TAG_PREFIX {
            return Err(format!(
                "The update settings name release tags that start with something other than {TAG_PREFIX}, so they \
                 are not used."
            ));
        }
        if !self.local {
            check_remote(&self.config.remote).map_err(|e| e.to_string())?;
        }
        let dot_git = src.join(".git");
        if !src.is_absolute() || is_link(src) || is_link(&dot_git) || !dot_git.is_dir() {
            return Err(not_ours("it has no .git folder of its own"));
        }
        let origin = self
            .own_setting("remote.origin.url")
            .ok_or_else(|| not_ours("it names no repository it comes from"))?;
        if origin != self.config.remote {
            return Err(not_ours("it comes from another repository"));
        }
        if self.own_setting(MARK).as_deref() == Some("true") {
            return Ok(());
        }
        // Not marked yet: only a copy exactly as the install left it is taken (and marked).
        if !self.pristine() {
            return Err(not_ours("it holds files that someone may be working on"));
        }
        let o = OsStr::new;
        self.git(&[o("config"), o("--local"), o(MARK), o("true")], GIT_LIMIT, &never)
            .map(|_| ())
            .map_err(|e| not_ours(&e))
    }

    /// Holds the update lock (`<build folder>\update.lock`) while the copy is used, so two copies of the app never
    /// fetch, build or stage in it at once.
    fn lock(&self) -> Result<File, String> {
        let dir = &self.config.build_dir;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("The build folder {} could not be made: {e}.", dir.display()))?;
        let path = dir.join("update.lock");
        let f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| format!("{} could not be opened: {e}.", path.display()))?;
        match f.try_lock() {
            Ok(()) => Ok(f),
            Err(std::fs::TryLockError::WouldBlock) => Err(ANOTHER_COPY.to_string()),
            Err(std::fs::TryLockError::Error(e)) => Err(format!("{} could not be locked: {e}.", path.display())),
        }
    }

    /// The release tags in the copy.
    fn tags(&self) -> Result<Vec<Release>, String> {
        let never = AtomicBool::new(false);
        let o = OsStr::new;
        let pattern = format!("refs/tags/{TAG_PREFIX}*");
        let text = self.git(&[o("for-each-ref"), o(TAG_FORMAT), o(&pattern)], GIT_LIMIT, &never)?;
        Ok(parse_tags(&text, TAG_PREFIX))
    }

    /// A copy of ours that has something in it that a build did not put there is never cleaned: it is renamed to
    /// `source.set-aside-N` (the first free N), and a fresh `--no-checkout` copy of the repository takes its place.
    /// Gives the sentence that says so.
    fn set_aside_and_copy_again(&self, cancel: &AtomicBool) -> Result<String, String> {
        let src = self.source();
        let name = src.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let aside = (1..1000)
            .map(|n| src.with_file_name(format!("{name}.set-aside-{n}")))
            .find(|p| std::fs::symlink_metadata(p).is_err())
            .ok_or_else(|| "There is no free name to set the copy of the source aside.".to_string())?;
        std::fs::rename(src, &aside).map_err(|e| {
            format!(
                "The copy of the source has changes in it, and it could not be set aside as {}: {e}.",
                aside.display()
            )
        })?;
        let parent = src.parent().unwrap_or(src);
        let options = self.safe_options()?;
        let o = OsStr::new;
        let mut args: Vec<&OsStr> = options.iter().map(OsString::as_os_str).collect();
        args.extend([
            o("clone"),
            o("--no-checkout"),
            o("--quiet"),
            o("--"),
            o(&self.config.remote),
            src.as_os_str(),
        ]);
        self.run_git(parent, &args, CLONE_LIMIT, cancel).map_err(|e| {
            format!(
                "The copy of the source had changes in it, so it was set aside as {} (nothing in it was deleted), but \
                 a fresh copy could not be made. {}",
                aside.display(),
                sentence(&e)
            )
        })?;
        self.own_copy()?;
        Ok(format!(
            "The app's copy of the source had changes in it that BALLFIGHT Studio did not make, so it was set aside as \
             {} (nothing in it was deleted) and a fresh copy was made.",
            aside.display()
        ))
    }
}

/// The `for-each-ref` format `parse_tags` reads: the tag's object and its type, the object it peels to and that type.
const TAG_FORMAT: &str = "--format=%(objectname) %(objecttype) %(*objectname) %(*objecttype) %(refname)";

impl Source for GitSource {
    fn newest(&self, current: &Version) -> Result<Option<Release>, String> {
        let _lock = self.lock()?;
        self.own_copy()?;
        let never = AtomicBool::new(false);
        let o = OsStr::new;
        // Only the release tags, and never one that moved (no `+`, no `--force`).
        let refspec = format!("refs/tags/{TAG_PREFIX}*:refs/tags/{TAG_PREFIX}*");
        self.git(
            &[
                o("fetch"),
                o("--prune"),
                o("--no-tags"),
                o("--quiet"),
                o("origin"),
                o(&refspec),
            ],
            FETCH_LIMIT,
            &never,
        )
        .map_err(|e| format!("Git could not fetch the releases. {}", sentence(&e)))?;
        Ok(newer(current, &self.tags()?))
    }

    fn build(&self, r: &Release, progress: &dyn Fn(String), cancel: &AtomicBool) -> Result<Built, String> {
        let lock = self.lock()?;
        self.own_copy()?;
        let o = OsStr::new;
        let stopped = || "The build was stopped.".to_string();
        progress(format!("Getting the source of version {}.", r.version));
        // The tag must still name the commit that was checked.
        let peeled = format!("refs/tags/{}^{{commit}}", r.tag);
        let now = self
            .git(
                &[o("rev-parse"), o("--verify"), o("--quiet"), o(&peeled)],
                GIT_LIMIT,
                cancel,
            )
            .map_err(|_| {
                format!(
                    "The tag {} is no longer in the copy of the source. Look for updates again.",
                    r.tag
                )
            })?;
        if now != r.commit {
            return Err(format!(
                "The tag {} has moved since it was checked. Look for updates again.",
                r.tag
            ));
        }
        let status_args = [
            o("status"),
            o("--porcelain"),
            o("--untracked-files=all"),
            o("--ignored"),
        ];
        let mut notes = Vec::new();
        let clean = self.git(&status_args, GIT_LIMIT, cancel).as_deref() == Ok("");
        if !clean && !self.pristine() {
            notes.push(self.set_aside_and_copy_again(cancel)?);
            // The fresh copy needs the tag again (only the tags, as a look fetches them).
            let refspec = format!("refs/tags/{TAG_PREFIX}*:refs/tags/{TAG_PREFIX}*");
            self.git(
                &[o("fetch"), o("--no-tags"), o("--quiet"), o("origin"), o(&refspec)],
                FETCH_LIMIT,
                cancel,
            )?;
            let again = self.git(
                &[o("rev-parse"), o("--verify"), o("--quiet"), o(&peeled)],
                GIT_LIMIT,
                cancel,
            )?;
            if again != r.commit {
                return Err(format!(
                    "The tag {} has moved since it was checked. Look for updates again.",
                    r.tag
                ));
            }
        }
        // The copy is clean, or as the install made it (an empty index against a branch: every file looks deleted).
        // `--force` is what lets git fill that empty copy; on a clean copy it has nothing to discard.
        self.git(
            &[
                o("-c"),
                o("advice.detachedHead=false"),
                o("checkout"),
                o("--detach"),
                o("--force"),
                o("--quiet"),
                o(&r.commit),
            ],
            GIT_LIMIT,
            cancel,
        )?;
        let head = self.git(&[o("rev-parse"), o("HEAD")], GIT_LIMIT, cancel)?;
        let status = self.git(&status_args, GIT_LIMIT, cancel)?;
        if head != r.commit || !status.is_empty() {
            return Err(format!(
                "The copy of the source could not be made exactly version {}.",
                r.version
            ));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(stopped());
        }

        progress(format!("Building version {}.", r.version));
        let studio = self.source().join("studio");
        let ceiling = self.source().parent().unwrap_or(self.source()).as_os_str();
        let env = [
            ("CARGO_TARGET_DIR", self.config.build_dir.as_os_str()),
            ("CARGO_TERM_COLOR", OsStr::new("never")),
            ("GIT_TERMINAL_PROMPT", OsStr::new("0")),
            ("GIT_CEILING_DIRECTORIES", ceiling),
        ];
        let how = Family {
            dir: Some(&studio),
            env: &env,
            env_remove: &CARGO_ENV_REMOVED,
            scrub_git: true,
            below_normal: true,
        };
        let args = [o("build"), o("--release"), o("--locked"), o("-p"), o("bf-app")];
        let ran = run_family(&self.cargo, &args, &how, BUILD_LIMIT, cancel, &mut |line| {
            if let Some(part) = compiling(line) {
                progress(format!("Building {part}."));
            }
        });
        let ran = match ran {
            Ok(ran) => ran,
            Err(_) if cancel.load(Ordering::Relaxed) => return Err(stopped()),
            Err(e) => return Err(e),
        };
        if !ran.status.success() {
            return Err(format!("Cargo said \"{}\"", cargo_said(&ran.tail)));
        }
        let exe = self.config.build_dir.join("release").join(EXE_NAME);
        if !exe.is_file() {
            return Err(format!("The build ended without making {}.", exe.display()));
        }
        let mut built = Built::new(exe, self.source().to_path_buf());
        built.notes = notes;
        built.hold = Some(BuildHold(lock));
        Ok(built)
    }
}

/// Cargo of this PC: `%USERPROFILE%\.cargo\bin\cargo.exe` (rustup's), else the first `cargo.exe` on `PATH`.
pub fn find_cargo() -> Option<PathBuf> {
    let rustup = std::env::var_os("USERPROFILE")
        .map(|home| PathBuf::from(home).join(".cargo").join("bin").join("cargo.exe"))
        .filter(|c| c.is_file());
    rustup.or_else(|| {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|d| d.join("cargo.exe"))
            .find(|c| c.is_absolute() && c.is_file())
    })
}

/// The crate a line of cargo's output says it is compiling: "   Compiling eframe v0.37.0" gives "eframe".
fn compiling(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("Compiling ")?;
    rest.split_whitespace().next()
}

/// What cargo said last about an error (its last line starting with "error"), else its last line.
fn cargo_said(tail: &[String]) -> String {
    tail.iter()
        .rev()
        .find(|l| l.trim_start().starts_with("error"))
        .or(tail.last())
        .map(|l| l.trim().trim_end_matches('.').to_string())
        .unwrap_or_else(|| "nothing".to_string())
}

/// Whether `s` is a full object id: 40 (or, for a SHA-256 repository, 64) lower-case hex digits.
fn full_commit(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Reads `git for-each-ref` output in `TAG_FORMAT` (`<object> <type> <peeled object> <peeled type> <refname>`): one
/// release for each tag named exactly `refs/tags/<prefix><version>` with a version `parse_version` reads, whose
/// object is a commit (a plain tag) or peels to a commit (an annotated tag). Anything else (another prefix, a bad
/// version, a tag of a tree or a file, a line not in that form) is left out.
pub fn parse_tags(for_each_ref: &str, prefix: &str) -> Vec<Release> {
    let mut out: Vec<Release> = Vec::new();
    for line in for_each_ref.lines() {
        let parts: Vec<&str> = line.splitn(5, ' ').collect();
        let [object, kind, peeled, peeled_kind, refname] = parts[..] else {
            continue;
        };
        let commit = match (kind, peeled.is_empty()) {
            ("commit", true) if peeled_kind.is_empty() => object,
            ("tag", false) if peeled_kind == "commit" => peeled,
            _ => continue,
        };
        if !full_commit(object) || !full_commit(commit) {
            continue;
        }
        let Some(tag) = refname.strip_prefix("refs/tags/") else {
            continue;
        };
        let Some(version) = tag.strip_prefix(prefix).and_then(parse_version) else {
            continue;
        };
        if out.iter().all(|r| r.tag != tag) {
            out.push(Release {
                tag: tag.to_string(),
                version,
                commit: commit.to_string(),
            });
        }
    }
    out.sort_by(|a, b| a.version.cmp(&b.version));
    out
}

/// A tiny stand-in for the real project, made by the tests: a workspace in `studio\` whose `bf-app` builds a
/// `ballfight-studio.exe` that answers `--version` the way the app does, with its commit from a build script.
#[cfg(test)]
pub(crate) mod standin {
    use std::path::{Path, PathBuf};

    use super::super::testkit::{commit_all, git_in, new_repo};

    pub(crate) const FFMPEG: &[u8] = b"pretend ffmpeg";
    pub(crate) const FFPROBE: &[u8] = b"pretend ffprobe";
    pub(crate) const LICENCE: &[u8] = b"GNU GENERAL PUBLIC LICENSE";

    fn sha(bytes: &[u8]) -> String {
        bf_pack::sha::sha256_reader(bytes).unwrap()
    }

    /// How the stand-in of a version behaves.
    #[derive(Clone, Default)]
    pub(crate) struct Kind {
        /// Prints this version instead of its own.
        pub(crate) claims: Option<String>,
        /// Does not compile.
        pub(crate) broken: bool,
        /// Its build script writes its process id to this file, then waits two minutes.
        pub(crate) slow: Option<PathBuf>,
    }

    /// Writes the stand-in of `version` into `tree` (the folder that holds `studio\`).
    pub(crate) fn write(tree: &Path, version: &str, kind: &Kind) {
        let studio = tree.join("studio");
        let app = studio.join("crates").join("bf-app");
        std::fs::create_dir_all(app.join("src")).unwrap();
        std::fs::create_dir_all(app.join("assets").join("icon")).unwrap();
        std::fs::create_dir_all(studio.join("packaging").join("guide")).unwrap();
        std::fs::write(
            studio.join("Cargo.toml"),
            format!(
                "[workspace]\nresolver = \"2\"\nmembers = [\"crates/bf-app\"]\n\n[workspace.package]\nversion = \
                 \"{version}\"\nedition = \"2021\"\n\n[profile.release]\nopt-level = 0\ndebug = 0\n"
            ),
        )
        .unwrap();
        std::fs::write(
            app.join("Cargo.toml"),
            "[package]\nname = \"bf-app\"\nversion.workspace = true\nedition.workspace = true\nbuild = \
             \"build.rs\"\n\n[[bin]]\nname = \"ballfight-studio\"\npath = \"src/main.rs\"\n",
        )
        .unwrap();
        let slow = match &kind.slow {
            Some(pid_file) => format!(
                "    std::fs::write({:?}, std::process::id().to_string()).unwrap();\n    \
                 std::thread::sleep(std::time::Duration::from_secs(120));\n",
                pid_file.display().to_string()
            ),
            None => String::new(),
        };
        // The commit comes from the detached HEAD the build checks out (`.git\HEAD` holds the commit id), so the
        // stand-in starts no program of its own.
        std::fs::write(
            app.join("build.rs"),
            format!(
                "fn main() {{\n    println!(\"cargo::rerun-if-changed=build.rs\");\n    \
                 println!(\"cargo::rerun-if-changed=../../../.git/HEAD\");\n{slow}    let head = \
                 std::fs::read_to_string(\"../../../.git/HEAD\").unwrap_or_default();\n    let head = \
                 head.trim();\n    let commit = if head.len() >= 7 && head.bytes().all(|b| b.is_ascii_hexdigit()) \
                 {{ head[..7].to_string() }} else {{ \"unknown\".to_string() }};\n    \
                 println!(\"cargo::rustc-env=BF_COMMIT={{commit}}\");\n}}\n"
            ),
        )
        .unwrap();
        let shown = match &kind.claims {
            Some(v) => format!("{v:?}"),
            None => "env!(\"CARGO_PKG_VERSION\")".to_string(),
        };
        let body = if kind.broken {
            "    this_does_not_compile\n".to_string()
        } else {
            format!(
                "    if std::env::args().nth(1).as_deref() == Some(\"--version\") {{\n        println!(\"BALLFIGHT \
                 Studio {{}} ({{}}, 11 Nov 2026)\", {shown}, env!(\"BF_COMMIT\"));\n    }}\n"
            )
        };
        std::fs::write(app.join("src").join("main.rs"), format!("fn main() {{\n{body}}}\n")).unwrap();
        std::fs::write(app.join("assets").join("icon").join("ballfight.ico"), b"pretend icon").unwrap();
        std::fs::write(
            studio.join("packaging").join("guide").join("index.html"),
            b"<p>The guide.</p>",
        )
        .unwrap();
        let pin = format!(
            "version = \"9.0.2\"\nbuild = \"full_build-www.gyan.dev\"\n\
             url = \"https://example.org/ffmpeg-9.0.2-full_build.zip\"\n\
             archive_sha256 = \"{}\"\narchive_bytes = 100\nlicence = \"GPL-3.0-or-later\"\n\
             source_note = \"The source is at https://example.org/source.\"\n\n\
             [[files]]\nname = \"ffmpeg.exe\"\nin_archive = \"x/bin/ffmpeg.exe\"\nsha256 = \"{}\"\n\n\
             [[files]]\nname = \"ffprobe.exe\"\nin_archive = \"x/bin/ffprobe.exe\"\nsha256 = \"{}\"\n\n\
             [[files]]\nname = \"LICENSE.txt\"\nin_archive = \"x/LICENSE\"\nsha256 = \"{}\"\n",
            sha(b"pretend archive"),
            sha(FFMPEG),
            sha(FFPROBE),
            sha(LICENCE)
        );
        std::fs::write(studio.join("packaging").join("ffmpeg.toml"), pin).unwrap();
    }

    /// The checked ffmpeg files for the stand-in's pin, in `dir`.
    pub(crate) fn checked_ffmpeg(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("ffmpeg.exe"), FFMPEG).unwrap();
        std::fs::write(dir.join("ffprobe.exe"), FFPROBE).unwrap();
        std::fs::write(dir.join("LICENSE.txt"), LICENCE).unwrap();
        dir.to_path_buf()
    }

    /// Makes the stand-in's `Cargo.lock` (no network: it has no dependencies).
    fn lock_file(cargo: &Path, studio: &Path) {
        let never = std::sync::atomic::AtomicBool::new(false);
        let args = [
            std::ffi::OsStr::new("generate-lockfile"),
            std::ffi::OsStr::new("--offline"),
        ];
        let how = bf_pack::run::Family {
            dir: Some(studio),
            env_remove: &["RUSTUP_TOOLCHAIN", "CARGO_TARGET_DIR"],
            ..Default::default()
        };
        let ran = bf_pack::run::run_family(
            cargo,
            &args,
            &how,
            std::time::Duration::from_secs(120),
            &never,
            &mut |_| {},
        )
        .unwrap();
        assert!(ran.status.success(), "{:?}", ran.tail);
    }

    /// The repository the releases come from (a local one, made here): the stand-in at each `(version, kind)` in
    /// turn, each commit tagged `studio-v<version>`, annotated when `annotated` says so. Gives each commit.
    pub(crate) fn upstream(g: &Path, cargo: &Path, repo: &Path, versions: &[(&str, Kind, bool)]) -> Vec<String> {
        new_repo(g, repo);
        let mut commits = Vec::new();
        for (version, kind, annotated) in versions {
            write(repo, version, kind);
            lock_file(cargo, &repo.join("studio"));
            let c = commit_all(g, repo, &format!("version {version}"));
            let tag = format!("studio-v{version}");
            if *annotated {
                git_in(g, repo, &["tag", "-a", "-m", &tag, &tag]);
            } else {
                git_in(g, repo, &["tag", &tag]);
            }
            commits.push(c);
        }
        commits
    }

    /// The install's copy of `repo`: `git clone --no-checkout`, as `bf_pack::install::source_copy` makes it.
    pub(crate) fn source_copy(g: &Path, repo: &Path, source: &Path) {
        let parent = source.parent().unwrap();
        std::fs::create_dir_all(parent).unwrap();
        git_in(
            g,
            parent,
            &[
                "clone",
                "--no-checkout",
                "--quiet",
                "--",
                &repo.to_string_lossy(),
                &source.to_string_lossy(),
            ],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit::{TempDir, commit_all, git, git_in};
    use super::standin::{Kind, source_copy, upstream};
    use super::*;

    fn config_for(root: &Path, repo: &Path) -> UpdateConfig {
        UpdateConfig {
            source: root.join("data").join("source"),
            remote: repo.to_string_lossy().into_owned(),
            tag_prefix: TAG_PREFIX.to_string(),
            build_dir: root.join("data").join("build"),
            downloads: root.join("data").join("downloads"),
        }
    }

    fn tools() -> Option<(PathBuf, PathBuf)> {
        let (Some(g), Some(c)) = (git(), find_cargo()) else {
            eprintln!("git or cargo is not on this PC, so this test checks nothing");
            return None;
        };
        Some((g, c))
    }

    #[test]
    fn tags_parse() {
        let a = "1".repeat(40);
        let b = "2".repeat(40);
        let c = "3".repeat(40);
        let d = "4".repeat(40);
        let text = format!(
            "{a} commit   refs/tags/studio-v0.6.0\n\
             {b} tag {c} commit refs/tags/studio-v0.6.1\n\
             {d} commit   refs/tags/other-v0.9.0\n\
             {d} commit   refs/tags/studio-v0.7\n\
             {d} commit   refs/tags/studio-v0.7.0-beta\n\
             {d} commit   refs/tags/studio-vX\n\
             {d} tag {d} commit refs/tags/studio-v0.8.0-rc.2\n\
             {d} tree   refs/tags/studio-v0.8.1\n\
             {d} blob   refs/tags/studio-v0.8.2\n\
             {d} tag {d} tree refs/tags/studio-v0.8.3\n\
             zzzz commit   refs/tags/studio-v0.9.0\n\
             {d} commit   refs/heads/studio-v1.0.0\n\
             not a line\n"
        );
        let tags = parse_tags(&text, "studio-v");
        let read: Vec<(String, String, String)> = tags
            .iter()
            .map(|r| (r.tag.clone(), r.version.to_string(), r.commit.clone()))
            .collect();
        assert_eq!(
            read,
            [
                ("studio-v0.6.0".to_string(), "0.6.0".to_string(), a.clone()),
                ("studio-v0.6.1".to_string(), "0.6.1".to_string(), c.clone()),
                ("studio-v0.8.0-rc.2".to_string(), "0.8.0-rc.2".to_string(), d.clone()),
            ]
        );
    }

    #[test]
    fn only_https_or_ssh_repositories_outside_tests() {
        let dir = TempDir::new("git-https");
        let config = config_for(dir.path(), &dir.path().join("upstream"));
        let err = GitSource::new(config).err().unwrap();
        assert_eq!(err, bf_pack::config::REMOTE_NOT_HTTPS_OR_SSH);
    }

    #[test]
    fn a_mark_from_the_users_settings_does_not_count() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-global");
        let repo = dir.path().join("upstream");
        upstream(&g, &cargo, &repo, &[("0.6.1", Kind::default(), false)]);
        // A clone someone works in (checked out, never marked), from the remote update.toml names.
        let config = config_for(dir.path(), &repo);
        std::fs::create_dir_all(config.source.parent().unwrap()).unwrap();
        git_in(
            &g,
            config.source.parent().unwrap(),
            &[
                "clone",
                "--quiet",
                "--",
                &repo.to_string_lossy(),
                &config.source.to_string_lossy(),
            ],
        );
        // The user's own settings (given to git through GIT_CONFIG_GLOBAL, in the test's git children only) carry
        // the mark.
        let global = dir.path().join("global.gitconfig");
        std::fs::write(&global, "[ballfightstudio]\n\tupdatecopy = true\n").unwrap();
        let mut s = GitSource::for_tests(config.clone(), g.clone(), cargo);
        s.test_env = vec![("GIT_CONFIG_GLOBAL".into(), global.into_os_string())];
        let never = AtomicBool::new(false);
        let o = OsStr::new;
        // Git does read it there, so only the copy's own settings keep it out.
        assert_eq!(
            s.git(&[o("config"), o("--get"), o(MARK)], GIT_LIMIT, &never).unwrap(),
            "true"
        );
        let err = s.newest(&parse_version("0.6.0").unwrap()).unwrap_err();
        assert!(err.contains("someone may be working on"), "{err}");
        assert_eq!(
            git_in(&g, &config.source, &["rev-parse", "--abbrev-ref", "HEAD"]),
            "main"
        );
    }

    #[test]
    fn a_changed_copy_is_set_aside_whole_and_no_hook_runs() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-aside");
        let repo = dir.path().join("upstream");
        let commits = upstream(&g, &cargo, &repo, &[("0.6.1", Kind::default(), false)]);
        let config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        // A hook in the user's own settings would write this file if git ran it.
        let hooks = dir.path().join("user hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        let ran_hook = dir.path().join("hook-ran.txt");
        std::fs::write(
            hooks.join("post-checkout"),
            format!(
                "#!/bin/sh\necho ran > '{}'\n",
                ran_hook.to_string_lossy().replace('\\', "/")
            ),
        )
        .unwrap();
        let global = dir.path().join("global.gitconfig");
        std::fs::write(
            &global,
            format!("[core]\n\thooksPath = {}\n", hooks.to_string_lossy().replace('\\', "/")),
        )
        .unwrap();
        let mut s = GitSource::for_tests(config.clone(), g.clone(), cargo);
        s.test_env = vec![("GIT_CONFIG_GLOBAL".into(), global.into_os_string())];
        let never = AtomicBool::new(false);
        let r = s.newest(&parse_version("0.6.0").unwrap()).unwrap().unwrap();
        drop(s.build(&r, &|_| {}, &never).unwrap());
        assert!(!ran_hook.exists(), "no hook of the user's ran");
        // Someone changes the copy: a tracked file edited and a file of their own.
        let main = config.source.join("studio/crates/bf-app/src/main.rs");
        let mut edited = std::fs::read(&main).unwrap();
        edited.extend_from_slice(b"// my change\n");
        std::fs::write(&main, &edited).unwrap();
        std::fs::write(config.source.join("studio/my notes.txt"), b"my notes").unwrap();
        let built = s.build(&r, &|_| {}, &never).unwrap();
        let aside = config.source.with_file_name("source.set-aside-1");
        assert_eq!(built.notes.len(), 1, "{:?}", built.notes);
        assert!(
            built.notes[0].contains(&aside.display().to_string()),
            "{:?}",
            built.notes
        );
        // Every byte of the person's work is kept in the set-aside folder; nothing was cleaned.
        assert_eq!(
            std::fs::read(aside.join("studio/crates/bf-app/src/main.rs")).unwrap(),
            edited
        );
        assert_eq!(std::fs::read(aside.join("studio/my notes.txt")).unwrap(), b"my notes");
        // The fresh copy is exactly the tagged commit, and the build used it.
        assert_eq!(git_in(&g, &config.source, &["rev-parse", "HEAD"]), commits[0]);
        assert!(!config.source.join("studio/my notes.txt").exists());
        assert!(!ran_hook.exists());
    }

    #[test]
    fn cargo_lines_read_plainly() {
        assert_eq!(compiling("   Compiling eframe v0.37.0"), Some("eframe"));
        assert_eq!(compiling("    Finished `release` profile"), None);
        let tail = vec![
            "error[E0425]: cannot find value `x` in this scope".to_string(),
            "error: could not compile `bf-app` (bin \"ballfight-studio\") due to 1 previous error".to_string(),
            "warning: build failed, waiting for other jobs to finish...".to_string(),
        ];
        assert_eq!(
            cargo_said(&tail),
            "error: could not compile `bf-app` (bin \"ballfight-studio\") due to 1 previous error"
        );
        assert_eq!(cargo_said(&[]), "nothing");
    }

    #[test]
    fn the_newest_tag_comes_from_the_apps_copy() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-newest");
        let repo = dir.path().join("upstream");
        let commits = upstream(
            &g,
            &cargo,
            &repo,
            &[
                ("0.6.0", Kind::default(), false),
                ("0.6.1", Kind::default(), true),
                ("0.7.0-rc.1", Kind::default(), true),
            ],
        );
        // Tags that are not releases are never taken.
        git_in(&g, &repo, &["tag", "studio-v0.9"]);
        git_in(&g, &repo, &["tag", "other-v9.0.0"]);
        let config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        let s = GitSource::for_tests(config.clone(), g.clone(), cargo.clone());
        let v = |t: &str| parse_version(t).unwrap();
        let r = s.newest(&v("0.6.0")).unwrap().unwrap();
        assert_eq!(r.tag, "studio-v0.6.1");
        // An annotated tag's commit is the commit it names, not the tag object.
        assert_eq!(r.commit, commits[1]);
        assert_eq!(s.newest(&v("0.6.1")).unwrap(), None);
        assert_eq!(s.newest(&v("0.7.0-rc.1")).unwrap(), None);
        assert_eq!(s.newest(&v("0.6.1-rc.1")).unwrap().unwrap().tag, "studio-v0.7.0-rc.1");
        // A new release pushed later is found by the next look.
        super::standin::write(&repo, "0.6.2", &Kind::default());
        let c = commit_all(&g, &repo, "version 0.6.2");
        git_in(&g, &repo, &["tag", "studio-v0.6.2"]);
        let r = s.newest(&v("0.6.0")).unwrap().unwrap();
        assert_eq!((r.tag.as_str(), r.commit.as_str()), ("studio-v0.6.2", c.as_str()));
        // The copy is marked as the app's own, and its work tree is still untouched by a look.
        assert_eq!(git_in(&g, &config.source, &["config", "--get", MARK]), "true");
        let names: Vec<_> = std::fs::read_dir(&config.source)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, [".git"]);
    }

    #[test]
    fn a_copy_of_another_remote_is_refused() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-other");
        let repo = dir.path().join("upstream");
        upstream(&g, &cargo, &repo, &[("0.6.1", Kind::default(), false)]);
        let mut config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        // update.toml names another repository than the copy's origin.
        config.remote = dir.path().join("elsewhere").to_string_lossy().into_owned();
        let s = GitSource::for_tests(config.clone(), g.clone(), cargo.clone());
        let v = parse_version("0.6.0").unwrap();
        let err = s.newest(&v).unwrap_err();
        assert!(err.contains("comes from another repository"), "{err}");
        let r = Release {
            tag: "studio-v0.6.1".into(),
            version: parse_version("0.6.1").unwrap(),
            commit: "0".repeat(40),
        };
        let never = AtomicBool::new(false);
        assert!(s.build(&r, &|_| {}, &never).unwrap_err().contains("another repository"));
        // Nothing was checked out and nothing marked.
        let names: Vec<_> = std::fs::read_dir(&config.source)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, [".git"]);
        assert!(
            git_in(&g, &config.source, &["config", "--list", "--local"])
                .lines()
                .all(|l| !l.starts_with(MARK))
        );
        // A prefix other than studio-v is refused too.
        let mut odd = config_for(dir.path(), &repo);
        odd.tag_prefix = "v".into();
        let s = GitSource::for_tests(odd, g, cargo);
        assert!(s.newest(&v).unwrap_err().contains("release tags"));
    }

    #[test]
    fn a_working_clone_is_never_used() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-working");
        let repo = dir.path().join("upstream");
        upstream(&g, &cargo, &repo, &[("0.6.1", Kind::default(), false)]);
        // A clone someone works in: checked out, with a change of their own, origin as update.toml says.
        let config = config_for(dir.path(), &repo);
        std::fs::create_dir_all(config.source.parent().unwrap()).unwrap();
        git_in(
            &g,
            config.source.parent().unwrap(),
            &[
                "clone",
                "--quiet",
                "--",
                &repo.to_string_lossy(),
                &config.source.to_string_lossy(),
            ],
        );
        let mine = config.source.join("studio").join("my work.txt");
        std::fs::write(&mine, b"unsaved work").unwrap();
        let s = GitSource::for_tests(config.clone(), g.clone(), cargo);
        let err = s.newest(&parse_version("0.6.0").unwrap()).unwrap_err();
        assert!(err.contains("someone may be working on"), "{err}");
        let r = Release {
            tag: "studio-v0.6.1".into(),
            version: parse_version("0.6.1").unwrap(),
            commit: git_in(&g, &repo, &["rev-parse", "HEAD"]),
        };
        let never = AtomicBool::new(false);
        assert!(s.build(&r, &|_| {}, &never).is_err());
        assert_eq!(
            std::fs::read(&mine).unwrap(),
            b"unsaved work",
            "the person's file is untouched"
        );
        assert_eq!(
            git_in(&g, &config.source, &["rev-parse", "--abbrev-ref", "HEAD"]),
            "main"
        );
        // A folder whose .git is a file (a worktree pointing elsewhere) is not a copy of its own either.
        let wt = dir.path().join("wt").join("data").join("source");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join(".git"), format!("gitdir: {}", repo.join(".git").display())).unwrap();
        let mut c2 = config_for(&dir.path().join("wt"), &repo);
        c2.source = wt;
        let s = GitSource::for_tests(c2, g, find_cargo().unwrap());
        assert!(
            s.newest(&parse_version("0.6.0").unwrap())
                .unwrap_err()
                .contains("no .git folder")
        );
    }

    #[test]
    fn a_release_is_built_from_exactly_its_tag() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-build");
        let repo = dir.path().join("upstream");
        let commits = upstream(
            &g,
            &cargo,
            &repo,
            &[("0.6.0", Kind::default(), false), ("0.6.1", Kind::default(), true)],
        );
        // A later commit on the branch, not tagged: never built.
        std::fs::write(repo.join("studio").join("later.txt"), b"not released").unwrap();
        commit_all(&g, &repo, "not released");
        let config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        let s = GitSource::for_tests(config.clone(), g.clone(), cargo.clone());
        let r = s.newest(&parse_version("0.6.0").unwrap()).unwrap().unwrap();
        // Something left in the copy's work tree is cleared by git before the build.
        std::fs::create_dir_all(config.source.join("studio")).unwrap();
        std::fs::write(config.source.join("studio").join("leftover.txt"), b"left").unwrap();
        let said = std::sync::Mutex::new(Vec::new());
        let never = AtomicBool::new(false);
        let started = std::time::Instant::now();
        let built = s.build(&r, &|t| said.lock().unwrap().push(t), &never).unwrap();
        eprintln!("the stand-in built in {:?}", started.elapsed());
        assert_eq!(built.tree, config.source);
        assert_eq!(built.exe, config.build_dir.join("release").join(EXE_NAME));
        assert_eq!(git_in(&g, &config.source, &["rev-parse", "HEAD"]), commits[1]);
        assert!(!config.source.join("studio").join("later.txt").exists());
        assert!(!config.source.join("studio").join("leftover.txt").exists());
        let said = said.into_inner().unwrap();
        assert_eq!(
            said.first().map(String::as_str),
            Some("Getting the source of version 0.6.1.")
        );
        assert!(said.iter().any(|t| t == "Building version 0.6.1."), "{said:?}");
        assert!(said.iter().any(|t| t == "Building bf-app."), "{said:?}");
        // The built program names the tag's version and commit.
        let ran = bf_pack::run::run(&built.exe, &[OsStr::new("--version")], Duration::from_secs(10), &never).unwrap();
        let text = String::from_utf8_lossy(&ran.stdout).trim().to_string();
        assert_eq!(
            text,
            format!("BALLFIGHT Studio 0.6.1 ({}, 11 Nov 2026)", &commits[1][..7])
        );

        // The source stays locked until the new version is staged; then it is free again.
        drop(built);
        // A tag that moves after the look is refused, with nothing built.
        git_in(
            &g,
            &repo,
            &["tag", "-f", "-a", "-m", "moved", "studio-v0.6.1", &commits[0]],
        );
        git_in(&g, &config.source, &["fetch", "--quiet", "--force", "--tags", "origin"]);
        let err = s.build(&r, &|_| {}, &never).unwrap_err();
        assert!(err.contains("has moved"), "{err}");
    }

    #[test]
    fn a_build_that_fails_says_what_cargo_said() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-broken");
        let repo = dir.path().join("upstream");
        let broken = Kind {
            broken: true,
            ..Kind::default()
        };
        upstream(&g, &cargo, &repo, &[("0.6.1", broken, false)]);
        let config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        let s = GitSource::for_tests(config.clone(), g, cargo);
        let r = s.newest(&parse_version("0.6.0").unwrap()).unwrap().unwrap();
        let never = AtomicBool::new(false);
        let err = s.build(&r, &|_| {}, &never).unwrap_err();
        assert!(err.starts_with("Cargo said \"error"), "{err}");
        assert!(!config.build_dir.join("release").join(EXE_NAME).exists());
    }

    #[test]
    fn stopping_a_build_stops_cargo_and_what_it_started() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-stop");
        let repo = dir.path().join("upstream");
        let pid_file = dir.path().join("build-script.pid");
        let slow = Kind {
            slow: Some(pid_file.clone()),
            ..Kind::default()
        };
        upstream(&g, &cargo, &repo, &[("0.6.1", slow, false)]);
        let config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        let s = GitSource::for_tests(config, g, cargo);
        let r = s.newest(&parse_version("0.6.0").unwrap()).unwrap().unwrap();
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let watched = pid_file.clone();
        // Once the build script runs (it has written its id), the build is stopped.
        let stopper = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(120);
            while std::time::Instant::now() < deadline {
                if let Ok(text) = std::fs::read_to_string(&watched)
                    && let Ok(pid) = text.trim().parse::<u32>()
                {
                    flag.store(true, Ordering::Relaxed);
                    return Some((pid, std::time::Instant::now()));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            flag.store(true, Ordering::Relaxed);
            None
        });
        let err = s.build(&r, &|_| {}, &cancel).unwrap_err();
        let (pid, asked) = stopper.join().unwrap().expect("the build script started");
        assert_eq!(err, "The build was stopped.");
        assert!(asked.elapsed() < Duration::from_secs(2), "{:?}", asked.elapsed());
        // The build script, cargo's child's child, is gone too.
        assert!(
            bf_pack::run::wait_for_exit(pid, Duration::from_secs(2)),
            "the build script still runs"
        );
    }

    /// The size of everything under `dir`, in bytes.
    fn size_of(dir: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
        entries
            .filter_map(Result::ok)
            .map(|e| {
                let p = e.path();
                if p.is_dir() && !is_link(&p) {
                    size_of(&p)
                } else {
                    e.metadata().map(|m| m.len()).unwrap_or(0)
                }
            })
            .sum()
    }

    /// Route A for real, on this PC: a temp copy of this lane's own repository as the origin (a local path, no
    /// network for git), two local tags, the real `cargo build --release --locked -p bf-app` of the whole workspace
    /// into a temp build folder (cargo may fetch crates and rustup a toolchain if this PC lacks them), then staging
    /// and the swap with ffmpeg files of the real sizes. Prints the times and sizes for `m6-measurements.md`. All of
    /// it is under one temp folder that this test makes and removes. Several GB and minutes.
    #[test]
    #[ignore = "builds the whole workspace twice (minutes, several GB in a temp folder); run by name"]
    fn a_real_update_on_this_pc() {
        let Some((g, cargo)) = tools() else { return };
        // The lane's own folder: studio\crates\bf-app is three folders down.
        let lane = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .unwrap()
            .to_path_buf();
        assert!(
            lane.join(".git").exists() && lane.join("studio").is_dir(),
            "{}",
            lane.display()
        );
        let dir = TempDir::new("real-update");
        eprintln!("working in {}", dir.path().display());
        let origin = dir.path().join("origin.git");
        let t = std::time::Instant::now();
        git_in(
            &g,
            dir.path(),
            &[
                "clone",
                "--bare",
                "--quiet",
                "--",
                &lane.to_string_lossy(),
                &origin.to_string_lossy(),
            ],
        );
        git_in(&g, &origin, &["tag", "studio-v9.0.0", "HEAD"]);
        let config = config_for(dir.path(), &origin);
        source_copy(&g, &origin, &config.source);
        eprintln!("origin and the app's copy made in {:?}", t.elapsed());
        let s = GitSource::for_tests(config.clone(), g.clone(), cargo.clone());
        let never = AtomicBool::new(false);
        let v = |t: &str| parse_version(t).unwrap();

        let t = std::time::Instant::now();
        let r = s.newest(&v("0.1.0")).unwrap().unwrap();
        eprintln!("look (fetch and tags): {:?}", t.elapsed());
        let t = std::time::Instant::now();
        let built = s.build(&r, &|_| {}, &never).unwrap();
        let cold = t.elapsed();
        let exe_bytes = std::fs::metadata(&built.exe).unwrap().len();
        eprintln!(
            "cold build: {cold:?}; build folder {:.2} GB; exe {:.1} MB",
            size_of(&config.build_dir) as f64 / 1e9,
            exe_bytes as f64 / 1e6
        );
        let ran = bf_pack::run::run(&built.exe, &[OsStr::new("--version")], Duration::from_secs(10), &never);
        match ran {
            Ok(ran) => eprintln!("--version said {:?}", String::from_utf8_lossy(&ran.stdout).trim()),
            Err(e) => eprintln!("--version: {e}"),
        }

        // The source's lock goes with the first build.
        drop(built);
        // A one-line change in bf-app, released as the next tag.
        let work = dir.path().join("work");
        git_in(
            &g,
            dir.path(),
            &[
                "clone",
                "--quiet",
                "--",
                &origin.to_string_lossy(),
                &work.to_string_lossy(),
            ],
        );
        git_in(&g, &work, &["config", "commit.gpgsign", "false"]);
        let lib = work
            .join("studio")
            .join("crates")
            .join("bf-app")
            .join("src")
            .join("lib.rs");
        let mut text = std::fs::read_to_string(&lib).unwrap();
        text.push_str("// A one-line change for the warm build measurement.\n");
        std::fs::write(&lib, text).unwrap();
        commit_all(&g, &work, "a one-line change");
        git_in(&g, &work, &["tag", "studio-v9.0.1"]);
        git_in(
            &g,
            &work,
            &["push", "--quiet", "origin", "HEAD:refs/heads/measure", "studio-v9.0.1"],
        );
        let r = s.newest(&v("9.0.0")).unwrap().unwrap();
        assert_eq!(r.tag, "studio-v9.0.1");
        let t = std::time::Instant::now();
        let built = s.build(&r, &|_| {}, &never).unwrap();
        eprintln!(
            "warm build after a one-line change in bf-app: {:?}; build folder {:.2} GB",
            t.elapsed(),
            size_of(&config.build_dir) as f64 / 1e9
        );

        // Staging and the swap, with ffmpeg files of the real sizes (the exes are about 228 MB each).
        let tree = dir.path().join("stage-tree");
        standin::write(&tree, "9.0.1", &Kind::default());
        let ff = dir.path().join("big-ffmpeg");
        std::fs::create_dir_all(&ff).unwrap();
        let mut files = Vec::new();
        for (name, mb) in [("ffmpeg.exe", 228usize), ("ffprobe.exe", 228), ("LICENSE.txt", 1)] {
            let bytes: Vec<u8> = (0..mb * 1_000_000).map(|i| (i % 251) as u8).collect();
            std::fs::write(ff.join(name), &bytes).unwrap();
            files.push((name, bf_pack::sha::sha256_reader(&bytes[..]).unwrap()));
        }
        let pin_path = tree.join("studio").join("packaging").join("ffmpeg.toml");
        let mut pin = std::fs::read_to_string(&pin_path).unwrap();
        for (name, sha) in [
            ("ffmpeg.exe", super::standin::FFMPEG),
            ("ffprobe.exe", super::standin::FFPROBE),
            ("LICENSE.txt", super::standin::LICENCE),
        ] {
            let old = bf_pack::sha::sha256_reader(sha).unwrap();
            let new = &files.iter().find(|(n, _)| *n == name).unwrap().1;
            pin = pin.replace(&old, new);
        }
        std::fs::write(&pin_path, pin).unwrap();
        let installed_root = dir.path().join("install");
        let layout = bf_pack::install::Layout::under(&installed_root);
        // The installed 9.0.0: the same files, so the swap moves as many bytes as a real one.
        let pkg = dir.path().join("pkg-9.0.0");
        bf_pack::package::package(&built.exe, &tree, &ff, &pkg).unwrap();
        // The install's update.toml names an https address (a local one is refused there).
        let mut installed_config = config.clone();
        installed_config.remote = "https://example.invalid/someone/repo.git".into();
        bf_pack::install::install(&pkg, &layout, &installed_config).unwrap();
        let inst = super::super::Installed {
            dir: layout.programs.clone(),
            config: installed_config,
        };
        let t = std::time::Instant::now();
        let next = super::super::stage::stage_with(&inst, &built.exe, &tree, &|_| Ok(ff.clone())).unwrap();
        eprintln!("staging ({:.0} MB): {:?}", size_of(&next) as f64 / 1e6, t.elapsed());
        let t = std::time::Instant::now();
        bf_pack::package::check_package(&next).unwrap();
        eprintln!("checking the staged package: {:?}", t.elapsed());
        let t = std::time::Instant::now();
        super::super::swap::finish_with(&inst.dir, u32::MAX, None, Duration::from_secs(5), &|_| Ok(())).unwrap();
        eprintln!("the swap: {:?}", t.elapsed());
    }

    #[test]
    fn two_looks_never_use_the_copy_at_once() {
        let Some((g, cargo)) = tools() else { return };
        let dir = TempDir::new("git-lock");
        let repo = dir.path().join("upstream");
        upstream(&g, &cargo, &repo, &[("0.6.1", Kind::default(), false)]);
        let config = config_for(dir.path(), &repo);
        source_copy(&g, &repo, &config.source);
        let s = GitSource::for_tests(config.clone(), g, cargo);
        // Another copy of the app holds the lock.
        let held = s.lock().unwrap();
        let other = GitSource::for_tests(config.clone(), s.git.clone(), s.cargo.clone());
        assert_eq!(
            other.newest(&parse_version("0.6.0").unwrap()).unwrap_err(),
            ANOTHER_COPY
        );
        drop(held);
        assert!(other.newest(&parse_version("0.6.0").unwrap()).unwrap().is_some());
    }
}
