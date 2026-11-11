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

use bf_core::fightfile::is_extra_line;
use bf_sound::master::BUS;

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
/// that its theme exists and is ported, that every voice its bed plays is ported, that its options fit the theme's schema, that each fighter has its roster
/// entry, voice and part in the score here, that the theme's bed has every hook a fighter's part calls (ruling
/// M3-R2: the kit itself would crash on that pair under that theme), that every `music.lines` key is a fighter or
/// an extra line one of them declares, that every `music.mix` bus is one the fight has (the theme's, a fighter's,
/// or a shared one), and that no title-hits list is empty (every part plays the first and the last of its title
/// hits on the VS card). Every chord the harmony parses has a voicing (the voicing rule replaces milestone 1's
/// list of allowed chords). [`build_soundtrack`] calls this first; what it can still refuse after this depends on
/// the recording.
///
/// # Errors
/// The first problem found, as `field: what is wrong` (`music.theme: "polka" is not a theme`,
/// `music.theme: Heist has no boom (a big drum hit), which the TROMBONE's part plays; pick another theme`,
/// `fighters[0]: PIANO has no part in the score yet`, `music.title_hits.FLUTE: the title hits are an empty list`).
pub fn validate(ff: &FightFile) -> Result<(), ContentError> {
    ff.validate(&name_status)?;
    validate_with(ff, theme_of(ff)?)
}

/// [`validate`]'s checks after the fight file's own, under `theme`.
pub(crate) fn validate_with(ff: &FightFile, theme: &Theme) -> Result<(), ContentError> {
    if !theme.ported {
        return Err(themes::not_ported(theme.name));
    }
    theme.bed_voices()?;
    let options = theme.options_of(ff)?;
    let bed = theme.bed(&ff.music.arrangement, &options)?;
    let mut players = Vec::new();
    for (k, name) in ff.fighters.iter().enumerate() {
        players.push(player(name, k, ff)?);
    }
    for (name, p) in ff.fighters.iter().zip(&players) {
        if let Some(&hook) = p.part.bed_calls().iter().find(|h| !bed.has(**h)) {
            return Err(ContentError::field(
                "music.theme",
                format!(
                    "{} has no {} ({}), which the {name}'s part plays; pick another theme",
                    theme.display(),
                    hook.name(),
                    hook.what()
                ),
            ));
        }
    }
    for key in ff.music.lines.keys() {
        let declared =
            ff.fighters.iter().zip(&players).any(|(name, p)| {
                key == name || (is_extra_line(key, name) && p.part.extra_lines().contains(&key.as_str()))
            });
        if !declared {
            let owner = ff.fighters.iter().find(|name| is_extra_line(key, name));
            let why = match owner {
                Some(name) => format!("the {name}'s part reads no line {key:?}"),
                None => format!("{key} is not one of the fighters"),
            };
            return Err(ContentError::field(format!("music.lines.{key}"), why));
        }
    }
    if !ff.music.mix.is_empty() {
        let mut buses: Vec<String> = bed.buses().iter().map(|(b, _)| b.to_string()).collect();
        buses.extend(BUS.iter().map(|(b, _)| b.to_string()));
        buses.extend(ff.fighters.iter().map(|name| name.to_lowercase()));
        buses.sort();
        buses.dedup();
        for bus in ff.music.mix.keys() {
            if !buses.contains(bus) {
                return Err(ContentError::field(
                    format!("music.mix.{bus}"),
                    format!("no bus {bus:?} in this fight (it has: {})", buses.join(", ")),
                ));
            }
        }
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

/// The bed of the fight file's arrangement of `theme` for its options, or the error that the theme has no such
/// arrangement or options that fit.
fn theme_bed(theme: &Theme, ff: &FightFile) -> Result<Box<dyn Bed>, ContentError> {
    theme.bed(&ff.music.arrangement, &theme.options_of(ff)?)
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
    /// is not ported, or a voice the theme's bed plays that is not; or the music context cannot be made (see
    /// [`MusicCtx::new`]).
    pub fn new(show: &'a Show, rec: &'a Recording, ff: &'a FightFile) -> Result<SoundBuild<'a>, ContentError> {
        let theme = theme_of(ff)?;
        let bed = theme_bed(theme, ff)?;
        let [a, b] = [player(&show.names[0], 0, ff)?, player(&show.names[1], 1, ff)?];
        let ctx = MusicCtx::new(show, rec, ff, [a.cfg, b.cfg], [a.voice, b.voice], theme.bed_voices()?)?;
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

    /// The bus levels of the master: the two parts' buses, the shared ones, the bed's, then the fight file's
    /// `music.mix` (the kit's `MIX`).
    pub fn levels(&self) -> Levels {
        let mix: Vec<(&str, (f64, f64))> = self
            .ctx
            .ff
            .music
            .mix
            .iter()
            .map(|(bus, level)| (bus.as_str(), *level))
            .collect();
        Levels::of(
            &self.ctx,
            [self.sounds[0].as_ref(), self.sounds[1].as_ref()],
            self.bed.as_ref(),
            &mix,
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

#[cfg(test)]
mod tests {
    use bf_sound::music::MusicCtx;
    use bf_sound::traits::{BedHook, Played, no_hook};

    use super::*;
    use crate::ThemeCard;
    use crate::themes::{Theme, ThemeOptions};

    fn episode() -> FightFile {
        FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the episode")
    }

    /// A bed with no boom, as `dark_disco` and `heist` are: its sections play nothing.
    struct NoBoom;

    impl Bed for NoBoom {
        fn hooks(&self) -> &'static [BedHook] {
            &[BedHook::Hit]
        }
        fn buses(&self) -> &[(&'static str, (f64, f64))] {
            &[("drums", (1.3, 0.1)), ("pad", (0.7, 0.35))]
        }
        fn reverb(&self) -> (f64, f64, f64) {
            (2.0, 0.5, 0.2)
        }
        fn comp(&self) -> (f64, f64) {
            (0.15, 0.62)
        }
        fn roar_gain(&self) -> f64 {
            0.15
        }
        fn doors_gain(&self) -> f64 {
            0.4
        }
        fn landing(&self) -> (f64, f64) {
            (0.07, 0.05)
        }
        fn pop(&self) -> (f64, f64, f64) {
            (95.0, 100.0, 0.1)
        }
        fn boom(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: f64) -> Played {
            no_hook(BedHook::Boom)
        }
        fn hit(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: &str, _: f64, _: f64) -> Played {
            Ok(())
        }
        fn clash(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn title(&self, _: &MusicCtx, _: &mut Mix, _: [f64; 3]) -> Played {
            Ok(())
        }
        fn vs(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn title_rest(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn count_in(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn fight_start(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn fight(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
            Ok(())
        }
        fn ko(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn stinger(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn after_ko(&self, _: &MusicCtx, _: &mut Mix, _: usize, _: f64, _: f64) -> Played {
            Ok(())
        }
        fn victory(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
            Ok(())
        }
        fn last_bars(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
    }

    fn no_boom(_: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
        Ok(Box::new(NoBoom))
    }

    static NO_BOOM_CARD: ThemeCard = ThemeCard {
        display: "Heist",
        text: "",
        key: "D",
        scale: "minor",
        chord_loops: &[],
        presets: &[],
        bars: (24, 42),
    };

    /// A ported theme whose bed has no boom (a stand-in for `heist`, which is not ported yet).
    static NO_BOOM: Theme = Theme {
        name: "heist",
        ported: true,
        card: &NO_BOOM_CARD,
        options: &[],
        voices: &[],
        own_voice: crate::themes::no_own_voice,
        classic: no_boom,
    };

    /// Review Focus 1 (ruling M3-R2): the TROMBONE's PEDAL TONE calls `bed.boom` without asking, so the kit would
    /// crash on this fight mid-soundtrack. It is refused in words, before any recording is read.
    #[test]
    fn a_theme_without_a_hook_a_fighter_plays_is_refused() {
        let mut ff = episode();
        ff.music.theme = "heist".into();
        assert_eq!(
            validate_with(&ff, &NO_BOOM).unwrap_err().to_string(),
            "music.theme: Heist has no boom (a big drum hit), which the TROMBONE's part plays; pick another theme"
        );
        // the same theme under two fighters that never call it is fine (FLUTE calls no hook)
        assert!(crate::fighters::sound("FLUTE", 0, &ff).unwrap().bed_calls().is_empty());
        // the real heist is not ported yet: that is said first
        assert_eq!(
            validate(&ff).unwrap_err().to_string(),
            r#"music.theme: "heist" is not ported yet"#
        );
        // the big top has a boom
        assert_eq!(validate(&episode()), Ok(()));
    }

    #[test]
    fn lines_and_mix_name_what_the_fight_has() {
        let mut ff = episode();
        // an extra line nobody declares
        let line = ff.music.lines["FLUTE"].clone();
        ff.music.lines.insert("FLUTE THEME".into(), line);
        assert_eq!(
            validate(&ff).unwrap_err().to_string(),
            r#"music.lines.FLUTE THEME: the FLUTE's part reads no line "FLUTE THEME""#
        );
        // a mix bus of the bed, of a fighter, or a shared one
        let mut ff = episode();
        for bus in ["calliope", "drums", "glock", "trombone", "flute", "solo", "fx"] {
            ff.music.mix.insert(bus.into(), (1.0, 0.1));
        }
        assert_eq!(validate(&ff), Ok(()));
        ff.music.mix.insert("pad".into(), (1.0, 0.1));
        assert_eq!(
            validate(&ff).unwrap_err().to_string(),
            r#"music.mix.pad: no bus "pad" in this fight (it has: calliope, drums, flute, fx, glock, solo, trombone)"#
        );
        // an option the big top does not have
        let mut ff = episode();
        ff.music
            .theme_options
            .insert("sequel".into(), bf_core::fightfile::OptionValue::Bool(true));
        assert_eq!(
            validate(&ff).unwrap_err().to_string(),
            r#"music.theme_options.sequel: big_top has no option "sequel" (it has none)"#
        );
    }

    /// The master takes the fight file's `MIX` as the kit's `levels.update(T.MIX)`: a bus already listed keeps its
    /// place and takes the new numbers.
    #[test]
    fn the_mix_reaches_the_master() {
        let rec = bf_core::recording::Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec"))
            .expect("the recording");
        let mut ff = episode();
        let show = Show::new(&rec, &ff).expect("a show");
        let plain = SoundBuild::new(&show, &rec, &ff).unwrap().levels();
        ff.music.mix.insert("trombone".into(), (2.3, 0.2));
        let build = SoundBuild::new(&show, &rec, &ff).unwrap();
        let mixed = build.levels();
        assert_eq!(mixed.get("trombone"), Some((2.3, 0.2)));
        let order = |l: &Levels| l.iter().map(|(b, _)| b.to_string()).collect::<Vec<_>>();
        assert_eq!(order(&mixed), order(&plain));
        for (bus, level) in plain.iter().filter(|(b, _)| *b != "trombone") {
            assert_eq!(mixed.get(bus), Some(level), "{bus}");
        }
    }
}
