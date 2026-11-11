//! Why a music helper could not make its sound, and what one that can returns.

use std::fmt;

/// Why a music helper could not make its sound: the helper's name and the problem, in plain words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicError {
    helper: &'static str,
    pub(super) problem: String,
}

impl MusicError {
    /// An error of the helper `helper` (the kit's name for it, such as `note` or `wah`).
    pub fn new(helper: &'static str, problem: impl Into<String>) -> MusicError {
        MusicError {
            helper,
            problem: problem.into(),
        }
    }

    /// The helper that failed.
    pub fn helper(&self) -> &'static str {
        self.helper
    }

    /// What was wrong.
    pub fn problem(&self) -> &str {
        &self.problem
    }
}

impl fmt::Display for MusicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.helper, self.problem)
    }
}

impl std::error::Error for MusicError {}

/// What a helper that makes or shapes samples returns.
pub type Samples = Result<Vec<f64>, MusicError>;
