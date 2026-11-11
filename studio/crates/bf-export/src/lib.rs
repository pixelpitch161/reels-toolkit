//! The checks, the ffmpeg encode, the render job and the output folders.
//!
//! | Module | Holds |
//! | --- | --- |
//! | [`checks`] | every check a video passes ([`CheckResult`], [`pre_render`], [`pre_render_keeping`]) |
//! | [`encode`] | finding ffmpeg ([`find_ffmpeg`]) and the loop-clean encode ([`Encoder`]) |
//! | [`probe`] | the checks of the finished file ([`verify_mp4`]) |
//! | [`output`] | the output folder: its name ([`folder_name`]), the temporary folder, the files beside the video |
//! | [`job`] | the whole render job ([`run_job`]) |

use std::fmt;

use bf_content::ContentError;

pub mod checks;
pub mod encode;
pub mod job;
pub mod output;
pub mod probe;

pub use self::checks::{BandPlan, CheckResult, Kept, pre_render, pre_render_keeping};
#[cfg(feature = "test-support")]
pub use self::encode::ffmpeg_at;
pub use self::encode::{
    APP_FFMPEG_DIR, Encoder, Ffmpeg, FfmpegSource, REINSTALL, find_ffmpeg, find_with as find_ffmpeg_with,
    is_about_the_apps_own,
};
#[cfg(feature = "test-support")]
pub use self::job::run_job_using;
pub use self::job::{JobEvent, JobOpts, run_job, stage};
pub use self::output::{clip_folder_name, folder_name};
pub use self::probe::{verify_clip, verify_mp4};

/// Why a video was not made. Its text (`Display`) is one plain sentence that says what went wrong and what to
/// do; nothing was saved.
///
/// What the app offers next follows the variant: [`ExportError::Ffmpeg`] install ffmpeg, [`ExportError::Folder`]
/// choose another folder, [`ExportError::Check`] the failed check (its `name` says which), the others nothing
/// but trying again. The kind never changes when the clean-up after it has trouble: a temporary folder that could
/// not be removed is named in an added clause of the same sentence ([`ExportError::with_clause`]).
#[derive(Clone, Debug, PartialEq)]
pub enum ExportError {
    /// ffmpeg (or ffprobe) is missing, cannot be started, is not a program file, or lacks the libx264 or aac
    /// encoder.
    Ffmpeg(String),
    /// ffmpeg was found but failed while it encoded or decoded the video: the sentence ends with its last words.
    Encode(String),
    /// The output folder is missing and cannot be made, is a file, cannot be written to, or both names for the
    /// video are taken; or a file of the video could not be written there or moved into place.
    Folder(String),
    /// The fight file or the recording cannot be turned into a picture or a soundtrack (the content's own words).
    Content(String),
    /// A check failed (the first that did); its `summary` is the sentence.
    Check(CheckResult),
    /// A frame could not be drawn as it must be (a panic, or a shape the drawing code refused); names the frame.
    Frame(String),
    /// The frames asked for are not a stretch of the video.
    Clip(String),
    /// The job was cancelled.
    Cancelled(String),
}

impl ExportError {
    /// The error of a cancelled job.
    pub fn cancelled() -> ExportError {
        ExportError::Cancelled("The render was cancelled, so nothing was saved.".to_string())
    }

    /// The same error with `clause` added to the end of its sentence ("..., and <clause>.").
    pub fn with_clause(self, clause: &str) -> ExportError {
        let add = |s: String| {
            format!(
                "{}, and {}.",
                s.trim_end().trim_end_matches('.'),
                clause.trim_end_matches('.')
            )
        };
        match self {
            ExportError::Ffmpeg(s) => ExportError::Ffmpeg(add(s)),
            ExportError::Encode(s) => ExportError::Encode(add(s)),
            ExportError::Folder(s) => ExportError::Folder(add(s)),
            ExportError::Content(s) => ExportError::Content(add(s)),
            ExportError::Frame(s) => ExportError::Frame(add(s)),
            ExportError::Clip(s) => ExportError::Clip(add(s)),
            ExportError::Cancelled(s) => ExportError::Cancelled(add(s)),
            ExportError::Check(mut r) => {
                r.summary = add(r.summary);
                ExportError::Check(r)
            }
        }
    }
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Ffmpeg(s)
            | ExportError::Encode(s)
            | ExportError::Folder(s)
            | ExportError::Content(s)
            | ExportError::Frame(s)
            | ExportError::Clip(s)
            | ExportError::Cancelled(s) => f.write_str(s),
            ExportError::Check(c) => f.write_str(&c.summary),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<ContentError> for ExportError {
    fn from(e: ContentError) -> ExportError {
        let text = e.to_string();
        ExportError::Content(format!(
            "This fight cannot be rendered: {}.",
            text.trim_end_matches('.')
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clause_keeps_the_kind() {
        let e =
            ExportError::cancelled().with_clause("the temporary folder X could not be removed, so delete it yourself");
        assert!(matches!(e, ExportError::Cancelled(_)));
        assert_eq!(
            e.to_string(),
            "The render was cancelled, so nothing was saved, and the temporary folder X could not be removed, so \
             delete it yourself."
        );
        let r = CheckResult {
            name: "levels",
            pass: false,
            summary: "Levels: too quiet.".into(),
            numbers: Default::default(),
        };
        let e = ExportError::Check(r).with_clause("Y is left");
        assert!(matches!(&e, ExportError::Check(r) if r.name == "levels"));
        assert_eq!(e.to_string(), "Levels: too quiet, and Y is left.");
    }
}
