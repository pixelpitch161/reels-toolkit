//! The plain data the interface shares: what a fight is made of (the draft), what the engine reports (the roster,
//! the shortlist, the balance numbers, a take, the queue, the library) and the app's few settings.
//!
//! Nothing here holds an egui type or does any work; the screens draw from these values and the seam
//! (`crate::seam`) carries them between the interface and the engine.

pub mod balance;
pub mod draft;
pub mod library;
pub mod queue;
pub mod roster;
pub mod shortlist;
pub mod story;
pub mod take;

use serde::{Deserialize, Serialize};

/// A generation of the fight. It rises by one whenever something that feeds the fight changes; every command about
/// the fight carries it and every answer echoes it, so an answer for an older generation is recognised as late.
pub type Gen = u64;
/// The number of one asked-for take (one seed's picture and sound); an answer for another take is late.
pub type TakeId = u64;
/// The number of one video in the render queue.
pub type JobId = u64;

/// Frames per second of the video.
pub const FPS: u32 = 30;
/// Frames in a video: 61 s at 30 fps.
pub const NF: u32 = 1_830;
/// Sound samples per second.
pub const SR: u32 = 48_000;
/// Sound samples in a soundtrack (61.013 s).
pub const NA: u64 = 2_928_640;
/// Sound samples per video frame.
pub const SAMPLES_PER_FRAME: u64 = 1_600;

/// The five steps of making a video, left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    Matchup,
    Balance,
    Seeds,
    Music,
    Render,
}

/// What the workspace shows: one of the steps, the Library or Settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum View {
    Step(Step),
    Library,
    Settings,
}

/// The platform a caption is written for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Instagram,
    TikTok,
    Shorts,
}

/// A line the app tells the owner (a file it could not read, a queue that was interrupted).
#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub text: String,
}

/// The app's settings, kept between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Where videos are saved.
    pub output_folder: std::path::PathBuf,
    /// Threads the engine may use for searching, balancing and rendering.
    pub workers: usize,
    /// How videos are encoded (Settings' "Video encoding"); a change applies to the next video that starts.
    #[serde(default)]
    pub encode: EncodeChoice,
}

/// How a video is encoded (plan M6, decision D4): the series' own setting, or one that makes the video sooner and
/// the file about a quarter larger. Only the encoder's preset differs; every frame and sample is the same. The
/// engine maps it to `bf_export::EncodeSpeed` (`engine::jobs::encode_speed`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EncodeChoice {
    /// "Best quality": the series' own setting, the default.
    #[default]
    Best,
    /// "Faster".
    Faster,
}

impl EncodeChoice {
    /// Both, in the order Settings shows them.
    pub const ALL: [EncodeChoice; 2] = [EncodeChoice::Best, EncodeChoice::Faster];

    /// The name the settings file keeps it by: "best" or "faster" (the names `bf_export::EncodeSpeed` uses too).
    pub fn name(self) -> &'static str {
        match self {
            EncodeChoice::Best => "best",
            EncodeChoice::Faster => "faster",
        }
    }

    /// The choice a settings file names; `None` for any other text.
    pub fn from_name(name: &str) -> Option<EncodeChoice> {
        EncodeChoice::ALL.into_iter().find(|c| c.name() == name)
    }

    /// What Settings calls it: "Best quality" or "Faster".
    pub fn label(self) -> &'static str {
        match self {
            EncodeChoice::Best => "Best quality",
            EncodeChoice::Faster => "Faster",
        }
    }
}
