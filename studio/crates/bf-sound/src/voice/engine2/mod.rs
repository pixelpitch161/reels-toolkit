//! Voice Engine 2: each fighter's instrument built from how it works (the Composed arrangement only; design:
//! `docs/studio/2026-11-26-voice-engine-2-design.md`, with its revision "built engines").
//!
//! One engine per family of instruments, each playing every technique its instruments have:
//!
//! - [`strings`] (with its soundboard, [`board`]): struck strings, the piano.
//! - [`pipes`]: pipes, the organ.
//!
//! Stage 1 rebuilt notes from recordings instead (the recipe engine below); no fighter plays a recipe now, but the
//! engine stays for a family that wants a measured part:
//!
//! Each recorded note of a free (CC0) recording set is measured once, by the `recipes` tool in `bf-harness`, into a
//! recipe: how loud each overtone is at each moment, how far off pitch it sits, and how loud the noise between the
//! overtones is in 24 bands ([`recipe`]). A note is rebuilt from the recipes of the recorded notes nearest it in
//! pitch and strength. What a recording can't hold, the instrument's working parts give: a piano's dampers and pedal,
//! an organ's wind and its stops coming in.
//!
//! - [`recipe`]: the recipe format.
//! - [`player`]: a note rebuilt from a recipe.
//! - [`play`]: how a note is played (accents, slurs, swells, the pedal...), for the parts and the composition system.
//! - [`piano`], [`organ`]: the instruments' working parts.
//! - [`space`]: where a voice sits in the stereo picture, and its own room.
//! - [`eq`]: gentle tone controls for a body or a polish.
//!
//! A fighter's voice has an [`Instrument`] ([`crate::Voice::engine2`]); [`crate::MusicCtx`] plays it under the
//! Composed arrangement in place of the voice's own notes. The Classic arrangement never reaches any of this.

use std::sync::OnceLock;

pub mod board;
pub mod eq;
pub mod organ;
pub mod piano;
pub mod pipes;
pub mod play;
pub mod player;
pub mod recipe;
pub mod space;
pub mod strings;

use self::organ::OrganParts;
use self::piano::PianoParts;
use self::pipes::PipeParts;
use self::play::Play;
use self::recipe::RecipeSet;
use self::space::Space;
use self::strings::StringsParts;
use crate::SRF;

/// Which engine an instrument plays on, with its settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Parts {
    /// The recipe piano of stage 1.
    Piano(PianoParts),
    /// The recipe organ of stage 1.
    Organ(OrganParts),
    /// The struck-string engine.
    Strings(StringsParts),
    /// The pipe engine.
    Pipes(PipeParts),
}

impl Parts {
    /// Whether the engine plays from a recipe (else it is built, and has none).
    pub fn has_recipe(&self) -> bool {
        matches!(self, Parts::Piano(_) | Parts::Organ(_))
    }
}

/// An instrument of Voice Engine 2: its engine and settings, its place in the mix, and for a recipe instrument its
/// recipe (compiled in).
#[derive(Debug)]
pub struct Instrument {
    /// The voice's name (`PIANO`).
    pub name: &'static str,
    /// Its recipe file's bytes.
    pub recipes: &'static [u8],
    pub parts: Parts,
    pub space: Space,
    set: OnceLock<RecipeSet>,
}

/// The longest a note with no length asked for rings, seconds.
const LONGEST: f64 = 12.0;

impl Instrument {
    /// An instrument; its recipe is read the first time a note asks for it.
    pub const fn new(name: &'static str, recipes: &'static [u8], parts: Parts, space: Space) -> Instrument {
        Instrument {
            name,
            recipes,
            parts,
            space,
            set: OnceLock::new(),
        }
    }

    /// A built instrument: no recipe.
    pub const fn built(name: &'static str, parts: Parts, space: Space) -> Instrument {
        Instrument::new(name, &[], parts, space)
    }

    /// Its recipe, read once.
    ///
    /// # Panics
    /// If the compiled-in recipe file can't be read (a test reads every instrument's, so a build never ships one).
    pub fn set(&self) -> &RecipeSet {
        self.set
            .get_or_init(|| RecipeSet::parse(self.recipes).unwrap_or_else(|e| panic!("{}'s recipe: {e}", self.name)))
    }

    /// How long the note `play` rings with no length asked for: to its stop's fall of 70 dB, or, for a note that
    /// isn't stopped, until it has died away, at most [`LONGEST`] seconds.
    fn natural(&self, play: &Play) -> f64 {
        let dur = play.dur.max(0.0);
        match self.parts {
            Parts::Strings(p) => return strings::rings_for(&p, play, LONGEST),
            Parts::Pipes(p) => return pipes::rings_for(&p, play, LONGEST),
            _ => {}
        }
        let set = self.set();
        let tau = set
            .nearest(play.midi, 1.0)
            .iter()
            .map(|(i, w)| w * set.notes[i].release)
            .sum::<f64>()
            .max(0.03);
        match self.parts {
            Parts::Piano(p) if piano::held(play) || play.midi >= f64::from(p.undamped_from) => (dur + 6.0).min(LONGEST),
            _ => (dur + 8.1 * tau).min(LONGEST),
        }
    }
}

/// The note `play` on instrument `i`: `cap` seconds long (or as long as it rings, for a `cap` of 0 or less), as
/// repeat `variant`. Mono, at 48 kHz.
pub fn play(i: &Instrument, play: &Play, cap: f64, variant: u64) -> Vec<f64> {
    let secs = if cap > 0.0 { cap } else { i.natural(play) };
    let len = (secs * SRF) as usize;
    match &i.parts {
        Parts::Piano(p) => piano::note(i.set(), p, play, len, variant),
        Parts::Organ(p) => organ::note(i.set(), p, play, len, variant),
        Parts::Strings(p) => strings::note(p, play, len, variant, SRF),
        Parts::Pipes(p) => pipes::note(p, play, len, variant, SRF),
    }
}
