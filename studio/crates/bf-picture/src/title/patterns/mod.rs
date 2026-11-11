//! The card's own panel patterns, `title.py`'s `PATTERNS`: one file per fighter.
//!
//! A pattern the card has for a fighter's name wins over the fighter's look's own (`kit.picture` registers a
//! look's pattern with `PATTERNS.setdefault`). Of the sixteen only the TROMBONE's (`_slide_tubes`, in
//! [`super::panels`]) was ported with milestone 1. Each of the other fifteen has its file here: a stub with
//! `PORTED = false` until its fighter's port fills it (eight are kit fighters', in milestone 3; seven belong to
//! older fighters, milestone 4). Until then a card for that fighter is an error, never a plain panel.
//!
//! Adding a pattern needs no edit here: the table below names every file once, and a port changes only its own
//! file (the body of `pattern` and its `PORTED`). A fighter whose look carries its own pattern (FLUTE, TUBA, ...)
//! is not in this table at all: its look's [`crate::look::Look::title_pattern_with`] draws it.

pub mod accordion;
pub mod acoustic;
pub mod bagpipes;
pub mod bari_sax;
pub mod bass_clarinet;
pub mod bassoon;
pub mod clarinet;
pub mod euphonium;
pub mod harp;
pub mod horn;
pub mod oboe;
pub mod organ;
pub mod ukulele;
pub mod viola;
pub mod violin;

use super::panels::{Pattern, TitleError, slide_tubes};

/// `PATTERNS`: each name, whether its pattern is ported, and the pattern, in `title.py`'s order.
const TABLE: [(&str, bool, Pattern); 16] = [
    ("BAGPIPES", bagpipes::PORTED, bagpipes::pattern),
    ("ACCORDION", accordion::PORTED, accordion::pattern),
    ("ORGAN", organ::PORTED, organ::pattern),
    ("HARP", harp::PORTED, harp::pattern),
    ("TROMBONE", true, slide_tubes),
    ("HORN", horn::PORTED, horn::pattern),
    ("UKULELE", ukulele::PORTED, ukulele::pattern),
    ("ACOUSTIC", acoustic::PORTED, acoustic::pattern),
    ("CLARINET", clarinet::PORTED, clarinet::pattern),
    ("OBOE", oboe::PORTED, oboe::pattern),
    ("BASS CLARINET", bass_clarinet::PORTED, bass_clarinet::pattern),
    ("EUPHONIUM", euphonium::PORTED, euphonium::pattern),
    ("BASSOON", bassoon::PORTED, bassoon::pattern),
    ("BARI SAX", bari_sax::PORTED, bari_sax::pattern),
    ("VIOLA", viola::PORTED, viola::pattern),
    ("VIOLIN", violin::PORTED, violin::pattern),
];

/// `PATTERNS.get(name)`: the card's own pattern for the fighter `name`, `None` when the card has none (the look's
/// pattern or the plain panel is drawn then).
///
/// # Errors
/// [`TitleError::PatternNotPorted`] when the card has a pattern for `name` that is not ported yet.
pub fn builtin(name: &str) -> Result<Option<Pattern>, TitleError> {
    match TABLE.iter().find(|row| row.0 == name) {
        None => Ok(None),
        Some(&(_, true, pattern)) => Ok(Some(pattern)),
        Some(_) => Err(TitleError::PatternNotPorted(name.to_string())),
    }
}

/// The names `PATTERNS` holds, in its order.
pub fn names() -> [&'static str; 16] {
    TABLE.map(|row| row.0)
}

/// The names whose pattern is not ported yet, in `PATTERNS`' order.
pub fn unported() -> Vec<&'static str> {
    TABLE.iter().filter(|row| !row.1).map(|row| row.0).collect()
}
