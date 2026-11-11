//! Names the commit the app is built from: `BF_COMMIT` (7 hex digits) and `BF_COMMIT_DATE` (the commit's own date,
//! `2026-11-12`, never the clock's), read through `git`. Without git, or outside a repository, both are "unknown"
//! and the build goes on without a word: this script never fails and prints nothing but cargo's own lines.
//!
//! Only `git` is started, through `bf_pack::run` (no console window), with fixed arguments. Cargo re-runs this
//! script only when one of the files named below changes, so an ordinary build is not a rebuild:
//! - `build.rs` itself (naming any file turns off cargo's default of re-running on every change in the crate);
//! - the git folder's `HEAD` (a checkout or a switch of branch), found with `git rev-parse --git-path HEAD`, which
//!   also works in lane clones and worktrees;
//! - the branch's own ref file when it is a loose file (a commit, a merge, a reset);
//! - otherwise, when the branch is packed, `packed-refs`, `logs/HEAD` and `reftable/`, whichever exist (a commit on a
//!   packed branch writes a new ref file there, and the log of `HEAD` grows).
//!
//! Only files that exist are named: cargo re-runs on every build for a named file that is missing.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const UNKNOWN: &str = "unknown";

/// Runs git with fixed arguments in this crate's folder (cargo's working folder for a build script) and gives its
/// first output line, or `None` when git is missing, fails or says nothing.
fn git(args: &[&str]) -> Option<String> {
    let never = AtomicBool::new(false);
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    let ran = bf_pack::run::run(Path::new("git"), &args, Duration::from_secs(20), &never).ok()?;
    if !ran.status.success() {
        return None;
    }
    let text = String::from_utf8(ran.stdout).ok()?;
    let line = text.lines().next()?.trim().to_string();
    (!line.is_empty()).then_some(line)
}

fn is_commit(s: &str) -> bool {
    s.len() == 7 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// A path git printed, which may be relative to this crate's folder.
fn from_git(manifest: &Path, printed: &str) -> PathBuf {
    let p = PathBuf::from(printed);
    if p.is_absolute() { p } else { manifest.join(p) }
}

/// The files whose change means another commit is checked out (see the top of this file).
fn watched(manifest: &Path) -> Vec<PathBuf> {
    let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]).map(|p| from_git(manifest, &p)) else {
        return Vec::new();
    };
    if !head.is_file() {
        return Vec::new();
    }
    let mut files = vec![head.clone()];
    let common = git(&["rev-parse", "--git-common-dir"]).map(|p| from_git(manifest, &p));
    let named = std::fs::read_to_string(&head)
        .ok()
        .and_then(|t| t.strip_prefix("ref:").map(|r| r.trim().to_string()));
    let loose = match (&common, &named) {
        (Some(common), Some(name))
            if name.starts_with("refs/") && name.split('/').all(|part| !part.is_empty() && part != "..") =>
        {
            Some(common.join(name))
        }
        _ => None,
    };
    match loose {
        Some(file) if file.is_file() => files.push(file),
        // A packed branch, a detached HEAD or another ref storage: what changes when a commit is made.
        _ => {
            let git_dir = head.parent().map(Path::to_path_buf);
            let candidates = [
                common.as_ref().map(|c| c.join("packed-refs")),
                git_dir.map(|g| g.join("logs").join("HEAD")),
                common.as_ref().map(|c| c.join("reftable")),
            ];
            files.extend(candidates.into_iter().flatten().filter(|p| p.exists()));
        }
    }
    files
}

fn main() {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR").map_or_else(|| PathBuf::from("."), PathBuf::from);

    let commit = git(&["rev-parse", "--short=7", "HEAD"]).filter(|c| is_commit(c));
    let date = commit
        .as_ref()
        .and_then(|_| git(&["log", "-1", "--format=%cs"]))
        .filter(|d| is_date(d));

    println!("cargo::rerun-if-changed=build.rs");
    if commit.is_some() {
        for file in watched(&manifest) {
            let text = file.to_string_lossy();
            // A path cargo could not read back on one line is left out rather than misread.
            if !text.contains(['\n', '\r']) {
                println!("cargo::rerun-if-changed={text}");
            }
        }
    }
    println!("cargo::rustc-env=BF_COMMIT={}", commit.as_deref().unwrap_or(UNKNOWN));
    println!("cargo::rustc-env=BF_COMMIT_DATE={}", date.as_deref().unwrap_or(UNKNOWN));
}
