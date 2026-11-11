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
}
