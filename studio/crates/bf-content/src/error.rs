//! Why the content cannot make a picture or a soundtrack of a fight.

use std::fmt;

use bf_core::fightfile::FightFileError;
use bf_core::show::ShowError;
use bf_sound::MusicError;

/// Why a fight cannot be turned into its picture or its soundtrack. Each message is one line; nothing that
/// returns this panics on what a fight file or a recording holds.
#[derive(Clone, Debug, PartialEq)]
pub enum ContentError {
    /// The fight file asks for something this app does not have or cannot use: the field and what is wrong with
    /// it, written `field: message` (an unknown theme, a chord the theme has no voicing for, a fighter whose
    /// part is not ported, and everything `FightFile::validate` refuses).
    FightFile(FightFileError),
    /// The recording gives no show: the fight has no K.O., or it is not this fight file's fight.
    Show(ShowError),
    /// The soundtrack could not be built from this recording and fight file: what the sound code reported, or
    /// the message of a panic inside it.
    Sound(String),
}

impl ContentError {
    /// A problem with one field of the fight file.
    pub(crate) fn field(field: impl Into<String>, message: impl Into<String>) -> ContentError {
        ContentError::FightFile(FightFileError {
            field: field.into(),
            message: message.into(),
        })
    }
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContentError::FightFile(e) => e.fmt(f),
            ContentError::Show(e) => e.fmt(f),
            ContentError::Sound(problem) => write!(f, "the soundtrack could not be built: {problem}"),
        }
    }
}

impl std::error::Error for ContentError {}

impl From<FightFileError> for ContentError {
    fn from(e: FightFileError) -> ContentError {
        ContentError::FightFile(e)
    }
}

impl From<ShowError> for ContentError {
    fn from(e: ShowError) -> ContentError {
        ContentError::Show(e)
    }
}

impl From<MusicError> for ContentError {
    fn from(e: MusicError) -> ContentError {
        ContentError::Sound(e.to_string())
    }
}
