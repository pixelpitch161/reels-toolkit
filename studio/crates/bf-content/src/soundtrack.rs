//! From a fight to its soundtrack: `kit.music.main()` without the file writing.
//!
//! [`build_soundtrack`] is what the render job and the app call: it checks the fight file against what this app
//! has ([`validate`]), derives the show from the recording, gathers the theme's bed and the two fighters' voices
//! and parts ([`SoundBuild`]), runs the score and masters it. Everything a fight file or a recording can do
//! wrong comes back as a [`ContentError`]; so does a panic inside the sound code, which is caught here (the
//! synthesis panics where Python raises, and a caller must never be taken down by a soundtrack). Catching needs
//! unwinding: the workspace's profiles must not set `panic = "abort"`. The panic's own message still goes to
//! stderr through the panic hook before the error comes back.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_core::show::Show;
use bf_sound::master::{Levels, MASTER_CEILING, Soundtrack, master};
use bf_sound::mix::Mix;
use bf_sound::music::MusicCtx;
use bf_sound::score::score;
use bf_sound::traits::{Bed, Sound};
use bf_sound::voice::Voice;

use crate::error::ContentError;
use crate::fighters;
use crate::registry::{name_status, roster};
use crate::themes::{self, Theme};

/// The theme a fight file names, or the error that it is none.
fn theme_of(ff: &FightFile) -> Result<&'static Theme, ContentError> {
    themes::theme(&ff.music.theme)
        .ok_or_else(|| ContentError::field("music.theme", format!("{:?} is not a theme", ff.music.theme)))
}

/// What fighter `k` of a fight brings to its soundtrack: its roster entry, its instrument voice and its part.
struct Player {
    cfg: &'static FighterCfg,
    voice: Arc<dyn Voice>,
    part: Box<dyn Sound>,
}

/// The roster entry, voice and part of the fighter `name` on side `k` of the fight file `ff`, or the error that
/// one of them is not in this app. The voice must be the one the roster entry names (its `pp`): another would
/// play the part on the wrong instrument.
fn player(name: &str, k: usize, ff: &FightFile) -> Result<Player, ContentError> {
    let missing = || ContentError::field(format!("fighters[{k}]"), format!("{name} has no part in the score yet"));
    let cfg = roster().iter().find(|f| f.name == name).ok_or_else(missing)?;
    let voice = fighters::voice(name).ok_or_else(missing)?;
    let part = fighters::sound(name, k, ff).ok_or_else(missing)?;
    if voice.name() != cfg.pp {
        return Err(ContentError::field(
            format!("fighters[{k}]"),
            format!(
                "{name}'s voice is {:?}, but the roster says it plays {:?}",
                voice.name(),
                cfg.pp
            ),
        ));
    }
    Ok(Player { cfg, voice, part })
}

/// Whether this app can make the soundtrack of the fight `ff`, before any recording is read: everything
/// `FightFile::validate` checks (with the registry's fighter names, the themes' arrangements included), then
/// that its theme exists and has a voicing for every chord of the intro and of the loop, that each fighter has
/// its roster entry, voice and part in the score here, and that no title-hits list is empty (every part plays
/// the first and the last of its title hits on the VS card). [`build_soundtrack`] calls this first; what it can
/// still refuse after this depends on the recording.
///
/// # Errors
/// The first problem found, as `field: what is wrong` (`music.theme: "polka" is not a theme`,
/// `music.chord_loop[4]: big_top has no voicing for "F#m"`, `fighters[0]: PIANO has no part in the score yet`,
/// `music.title_hits.FLUTE: the title hits are an empty list`).
pub fn validate(ff: &FightFile) -> Result<(), ContentError> {
    ff.validate(&name_status)?;
    let theme = theme_of(ff)?;
    theme_bed(theme, ff)?;
    let lists = [
        ("music.chords_intro", &ff.music.chords_intro),
        ("music.chord_loop", &ff.music.chord_loop),
    ];
    for (field, chords) in lists {
        for (i, chord) in chords.iter().enumerate() {
            if !theme.voices(chord) {
                return Err(ContentError::field(
                    format!("{field}[{i}]"),
                    format!("{} has no voicing for {chord:?}", theme.name),
                ));
            }
        }
    }
    for (k, name) in ff.fighters.iter().enumerate() {
        player(name, k, ff)?;
    }
    for (name, hits) in &ff.music.title_hits {
        if hits.is_empty() {
            return Err(ContentError::field(
                format!("music.title_hits.{name}"),
                "the title hits are an empty list",
            ));
        }
    }
    Ok(())
}

/// The bed of the fight file's arrangement of `theme`, or the error that the theme has no such arrangement.
fn theme_bed(theme: &Theme, ff: &FightFile) -> Result<Box<dyn Bed>, ContentError> {
    theme.bed(&ff.music.arrangement).ok_or_else(|| {
        ContentError::field(
            "music.arrangement",
            format!("{} has no arrangement {:?}", theme.name, ff.music.arrangement),
        )
    })
}

/// What a soundtrack is built from: the music context of the fight (`kit.music`'s module state: the run, the
/// timeline, the two voices, the note cache and the noise stream), the theme's bed and the two fighters' parts.
///
/// [`build_soundtrack`] makes one and runs it; the parity tests and the timings use the steps one by one. A
/// build makes one soundtrack: the notes it has synthesised stay cached and the noise stream stays where the
/// score left it, so a second [`SoundBuild::score`] on the same build gives other FLUTE notes (as a second
/// `kit.music.score()` in one interpreter does).
pub struct SoundBuild<'a> {
    /// The music context.
    pub ctx: MusicCtx<'a>,
    /// The theme's bed, in the fight file's arrangement.
    pub bed: Box<dyn Bed>,
    /// The two fighters' parts, left corner first.
    pub sounds: [Box<dyn Sound>; 2],
}

impl<'a> SoundBuild<'a> {
    /// The build of the fight `rec` under the fight file `ff`, whose show is `show` (`Show::new(rec, ff)`).
    ///
    /// It does not run [`validate`] (the tests build fight files outside the theme's voicings), but refuses in
    /// the same words what it cannot build at all.
    ///
    /// # Errors
    /// The fight file names a theme or an arrangement this app does not have, or a fighter whose voice or part
    /// is not ported; or the music context cannot be made (see [`MusicCtx::new`]).
    pub fn new(show: &'a Show, rec: &'a Recording, ff: &'a FightFile) -> Result<SoundBuild<'a>, ContentError> {
        let bed = theme_bed(theme_of(ff)?, ff)?;
        let [a, b] = [player(&show.names[0], 0, ff)?, player(&show.names[1], 1, ff)?];
        let ctx = MusicCtx::new(show, rec, ff, [a.cfg, b.cfg], [a.voice, b.voice])?;
        Ok(SoundBuild {
            ctx,
            bed,
            sounds: [a.part, b.part],
        })
    }

    /// `kit.music.score()`: the whole soundtrack as a mix, before the master.
    ///
    /// # Errors
    /// What the bed or a part reports (see `bf_sound::music`). The synthesis may also panic where Python
    /// raises; [`build_soundtrack`] catches that.
    pub fn score(&self) -> Result<Mix, ContentError> {
        Ok(score(
            &self.ctx,
            self.bed.as_ref(),
            [self.sounds[0].as_ref(), self.sounds[1].as_ref()],
        )?)
    }

    /// The bus levels of the master: the two parts' buses, the shared ones, the bed's (the fight file has no
    /// overrides, the kit's `MIX`).
    pub fn levels(&self) -> Levels {
        Levels::of(
            &self.ctx,
            [self.sounds[0].as_ref(), self.sounds[1].as_ref()],
            self.bed.as_ref(),
            &[],
        )
    }

    /// `kit.music.master(mix)`: the mix mastered: the soundtrack and its stems.
    pub fn master(&self, mix: &Mix) -> Soundtrack {
        master(mix, &self.levels(), self.bed.as_ref(), MASTER_CEILING)
    }
}

/// What a caught panic said.
fn panic_text(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(text) => (*text).to_string(),
            Err(_) => "the sound code stopped without a message".to_string(),
        },
    }
}

/// The soundtrack of the fight `rec` under the fight file `ff`: the master and every bus after it, each
/// `bf_sound::NA` frames long (`kit.music.main()`'s `audio.wav` and stems, before they are written).
///
/// The same recording and fight file give the same samples on every PC and with any number of threads: the
/// notes are made one at a time, in the score's order.
///
/// # Errors
/// - [`ContentError::FightFile`]: what [`validate`] refuses (a fighter whose part is not ported among it).
/// - [`ContentError::Show`]: the fight has no K.O., or the recording is not this fight file's fight.
/// - [`ContentError::Sound`]: the recording lacks something the score reads, or holds values the sound code
///   cannot use (a panic inside it is caught and reported here too).
pub fn build_soundtrack(rec: &Recording, ff: &FightFile) -> Result<Soundtrack, ContentError> {
    validate(ff)?;
    let show = Show::new(rec, ff)?;
    let built = catch_unwind(AssertUnwindSafe(|| -> Result<Soundtrack, ContentError> {
        let build = SoundBuild::new(&show, rec, ff)?;
        let mix = build.score()?;
        Ok(build.master(&mix))
    }));
    match built {
        Ok(result) => result,
        Err(payload) => Err(ContentError::Sound(panic_text(payload))),
    }
}
