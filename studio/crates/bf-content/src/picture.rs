//! From a fight to its picture: the kit's `kit.picture` module for one episode, with the content of this app.
//!
//! [`build_picture`] is what the app's preview and the render job call: it checks the fight file against what
//! this app has ([`validate`]), derives the show from the recording, gathers the two fighters' roster entries,
//! models and looks ([`look_for`]) and builds the [`Picture`]. Everything a fight file or a recording can do
//! wrong comes back as a [`ContentError`]; so does a panic while the picture is built (the looks' `setup` and
//! the effects build run there), which is caught here as [`crate::build_soundtrack`] catches the sound code's.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_core::show::Show;
use bf_picture::ring::{Picture, PictureError};
use bf_picture::sprite::Mesh;

use crate::error::ContentError;
use crate::fighters::look_for;
use crate::registry::roster;
use crate::soundtrack::validate;

/// The model file of the instrument `pp` (a roster entry's `pp`), as the export tool's `models` wrote it, for
/// the fighters whose look is ported.
fn model_json(pp: &str) -> Option<&'static str> {
    match pp {
        "FLUTE" => Some(include_str!("../data/models/FLUTE.json")),
        "TROMBONE" => Some(include_str!("../data/models/TROMBONE.json")),
        _ => None,
    }
}

/// The roster entry and the model of the fighter `name` on side `k`, or the error that this app has none.
fn fighter(name: &str, k: usize) -> Result<(&'static FighterCfg, Mesh), ContentError> {
    let missing = || ContentError::field(format!("fighters[{k}]"), format!("{name} has no look yet"));
    let cfg = roster().iter().find(|f| f.name == name).ok_or_else(missing)?;
    let json = model_json(&cfg.pp).ok_or_else(missing)?;
    let mesh = Mesh::from_json(json)
        .map_err(|e| ContentError::Picture(PictureError::Mesh(format!("{name}'s model cannot be read: {e}"))))?;
    Ok((cfg, mesh))
}

/// What a caught panic said.
fn panic_text(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(text) => (*text).to_string(),
            Err(_) => "the picture code stopped without a message".to_string(),
        },
    }
}

/// The picture of the fight `rec` under the fight file `ff`: every frame of its video, drawn on demand
/// ([`Picture::frame`], [`Picture::frame_reported`]). The picture is `Send + Sync` and a frame is a pure function
/// of its number, so the preview and the render job draw frames in any order on any number of threads and get
/// the same bytes on every PC.
///
/// # Errors
/// - [`ContentError::FightFile`]: what [`validate`] refuses, or a fighter whose look is not in this app.
/// - [`ContentError::Show`]: the fight has no K.O., or the recording is not this fight file's fight.
/// - [`ContentError::Picture`]: the recording lacks something the picture reads, or holds values the picture
///   code cannot use (a panic while the picture is built is caught and reported here too).
pub fn build_picture(rec: Arc<Recording>, ff: &FightFile) -> Result<Picture, ContentError> {
    validate(ff)?;
    let show = Show::new(&rec, ff)?;
    let [(cfg0, mesh0), (cfg1, mesh1)] = [fighter(&show.names[0], 0)?, fighter(&show.names[1], 1)?];
    let looks = [look_for(&show.names[0]), look_for(&show.names[1])];
    let built = catch_unwind(AssertUnwindSafe(|| {
        Picture::new(rec, ff, show, [cfg0, cfg1], [mesh0, mesh1], looks)
    }));
    match built {
        Ok(result) => result.map_err(ContentError::Picture),
        Err(payload) => Err(ContentError::Picture(PictureError::Recording(format!(
            "the picture could not be built: {}",
            panic_text(payload)
        )))),
    }
}
