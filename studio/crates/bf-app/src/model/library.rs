//! The Library (videos already made) and the state of the output folder.

/// One video folder found under the output folder.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryEntry {
    pub folder: std::path::PathBuf,
    /// "Flute vs Trombone".
    pub title: String,
    /// The two fighters' colours, left then right.
    pub colors: [[u8; 3]; 2],
    pub seed: Option<u64>,
    /// When the video was made, in seconds since 1970 (UTC).
    pub modified: u64,
    /// The fight file in the folder, if there is one: what reopening it loads.
    pub fight_path: Option<std::path::PathBuf>,
    /// Why this video cannot be reopened, in words ("Made by a newer version of the app.").
    pub problem: Option<String>,
}

/// What the app knows about the output folder (or a video's folder in it).
#[derive(Clone, Debug, PartialEq)]
pub enum FolderState {
    /// Not checked yet.
    Unknown,
    /// It can be written; `taken` when the asked-for name is already in use.
    Ok {
        taken: bool,
    },
    Missing,
    /// It cannot be written, with the reason in words.
    NotWritable(String),
}
