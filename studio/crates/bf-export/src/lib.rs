//! The checks, the ffmpeg encode, the render job and the output folders.
//!
//! | Module | Holds |
//! | --- | --- |
//! | [`checks`] | every check a video passes ([`CheckResult`], [`pre_render`]) |
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

pub use self::checks::{BandPlan, CheckResult, pre_render};
pub use self::encode::{Encoder, Ffmpeg, find_ffmpeg};
pub use self::job::{JobEvent, JobOpts, run_job, stage};
pub use self::output::folder_name;
pub use self::probe::{verify_clip, verify_mp4};

/// Why a video was not made. Its text (`Display`) is one plain sentence that says what went wrong and what to
/// do; nothing was saved.
///
/// What the app offers next follows the variant: [`ExportError::Ffmpeg`] install ffmpeg, [`ExportError::Folder`]
/// choose another folder, [`ExportError::Check`] the failed check (its `name` says which), the others nothing
/// but trying again.
#[derive(Clone, Debug, PartialEq)]
pub enum ExportError {
    /// ffmpeg (or ffprobe) is missing, cannot be started, or lacks the libx264 or aac encoder.
    Ffmpeg(String),
    /// ffmpeg was found but failed while it encoded or decoded the video: the sentence ends with its last words.
    Encode(String),
    /// The output folder is missing and cannot be made, is a file, cannot be written to, or both names for the
    /// video are taken; or a file of the video could not be written there.
    Folder(String),
    /// The fight file or the recording cannot be turned into a picture or a soundtrack.
    Content(ContentError),
    /// A check failed (the first that did); its `summary` is the sentence.
    Check(CheckResult),
    /// A frame could not be drawn as it must be (a panic, or a shape the drawing code refused); names the frame.
    Frame(String),
    /// The frames asked for are not a stretch of the video.
    Clip(String),
    /// The job was cancelled.
    Cancelled,
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::Ffmpeg(s)
            | ExportError::Encode(s)
            | ExportError::Folder(s)
            | ExportError::Frame(s)
            | ExportError::Clip(s) => f.write_str(s),
            ExportError::Content(e) => {
                let text = e.to_string();
                write!(f, "This fight cannot be rendered: {}.", text.trim_end_matches('.'))
            }
            ExportError::Check(c) => f.write_str(&c.summary),
            ExportError::Cancelled => f.write_str("The render was cancelled, so nothing was saved."),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<ContentError> for ExportError {
    fn from(e: ContentError) -> ExportError {
        ExportError::Content(e)
    }
}
