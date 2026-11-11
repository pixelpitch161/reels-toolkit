//! `bf-pack`: everything about files on disk outside the app's own data. The pinned ffmpeg and its checked download
//! (`ffmpeg`, `fetch`), the one way to start a program (`run`), checksums (`sha`); and, filled by later tasks, the
//! package folder, the install, the shortcuts, removal, `update.toml` and version numbers.
//!
//! Every module is declared here once, so later tasks fill their own files and never edit this one.

use std::fmt;

pub mod config;
pub mod fetch;
pub mod ffmpeg;
pub mod install;
pub mod package;
pub mod remove;
pub mod run;
pub mod sha;
pub mod version;

/// What can go wrong. Each one displays as one plain sentence that the owner can read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    /// A download failed or came down incomplete. The text says what and why.
    Download(String),
    /// A file is not the expected one.
    Checksum { what: String, want: String, got: String },
    /// The archive could not be read, or a file is missing from it.
    Archive(String),
    /// Reading or writing a file on this PC failed.
    Io(String),
    /// Asked for something this crate will not do (for example an address that holds a password).
    Refused(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::Download(s) | PackError::Archive(s) | PackError::Io(s) | PackError::Refused(s) => f.write_str(s),
            PackError::Checksum { what, want, got } => {
                write!(
                    f,
                    "{what} is not the expected file: its SHA-256 should be {want} but is {got}."
                )
            }
        }
    }
}

impl std::error::Error for PackError {}

/// A reading or writing failure as a plain sentence: "Could not {doing} {path}: {error}."
pub(crate) fn io_error(doing: &str, path: &std::path::Path, e: &std::io::Error) -> PackError {
    PackError::Io(format!("Could not {doing} {}: {e}.", path.display()))
}

/// Temp folders for tests: each test gets its own empty folder under the system temp folder, removed when the
/// guard is dropped. Nothing a test writes lands anywhere else.
#[cfg(test)]
pub(crate) mod testdir {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    pub(crate) struct TempDir(PathBuf);

    impl TempDir {
        pub(crate) fn new(name: &str) -> TempDir {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join("bf-pack-tests")
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
}
