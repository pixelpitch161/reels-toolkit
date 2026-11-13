//! The themes: one folder per theme (the kit's moods, `projects/ballfight/kit/moods/<name>.py`), each with its
//! instruments, its voicings and its classic arrangement (the bed that reproduces the kit's), and the registry
//! that lists them.
//!
//! **The general model (milestone 3).** Every kit mood has a folder here from the start; a theme port fills its
//! own folder (`ported: true`, its card, its options, its bed) and the register task adds it to [`PORTED_THEMES`],
//! the themes the app offers. A theme voices every chord the harmony parses (a table where the mood has one, the
//! chord's tones otherwise), so a fight file may ask any theme for any chord: there is no list of allowed chords
//! (milestone 1's ruling R33 is replaced by the voicing rule). What a theme can refuse is a fight whose options do
//! not fit its schema ([`Theme::options_of`]) and, in `validate`, a fight whose fighters call a bed hook it does
//! not have (ruling M3-R2).

use std::collections::BTreeMap;
use std::sync::Arc;

use bf_core::fightfile::{FightFile, OptionValue};
use bf_sound::traits::Bed;
use bf_sound::voice::Voice;

use crate::cards::ThemeCard;
use crate::error::ContentError;
use crate::fighters;
use crate::registry::roster;

pub mod baroque_storm;
pub mod big_top;
pub mod bossa_noir;
pub mod brass_trailer;
pub mod dark_disco;
pub mod dark_drumline;
pub mod dark_tango;
pub mod heist;
pub mod horror_trap;
pub mod hot_jazz;
pub mod mambo;
pub mod prog_duel;
pub mod second_line;
pub mod stadium_rock;

// the older episodes' themes (milestone 4)
pub mod balkan_fire;
pub mod brass_house;
pub mod dark_breakbeat;
pub mod drift_phonk;
pub mod gothic_horror;
pub mod industrial;
pub mod noir_jazz;
pub mod orchestral_drill;
pub mod sea_shanty;
pub mod ska_punk;
pub mod spaghetti_western;
pub mod villain_march;

/// The arrangement every theme has: the one that reproduces the kit's bed.
pub const CLASSIC: &str = "classic";

/// The themes the app offers, by name (grows only in the register tasks). A theme that is ported but not listed
/// here is tested, never offered. The big top comes first, since a new match-up starts on the first theme offered
/// (it is the starting pair's theme); the others follow in the kit's alphabetical order of moods. A theme is offered
/// for a pair only when its bed has every hook both parts play ([`themes_for`], ruling M3-R2).
pub const PORTED_THEMES: [&str; 14] = [
    "big_top",
    "baroque_storm",
    "bossa_noir",
    "brass_trailer",
    "dark_disco",
    "dark_drumline",
    "dark_tango",
    "heist",
    "horror_trap",
    "hot_jazz",
    "mambo",
    "prog_duel",
    "second_line",
    "stadium_rock",
];

/// A theme's options as one fight has them: every option of the theme's schema, by its name, with the fight
/// file's value or the option's default (a fighter option's default is the left corner's fighter, as the kit's
/// `PLUGGED` is `M.NAMES[0]`).
pub type ThemeOptions = BTreeMap<String, OptionValue>;

/// What kind of value a theme option holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionKind {
    /// True or false (`TRAILER.sequel`, `DRUMLINE.bass_drums`).
    Bool,
    /// One of the two fighters of the fight, by name (`PLUGGED`).
    Fighter,
}

/// One option of a theme (Table S4 of the milestone 3 plan): its name in a fight file (`music.theme_options`),
/// the kit's config name for it, its kind and its default.
#[derive(Debug)]
pub struct ThemeOption {
    /// The fight file's name, such as `sequel`.
    pub key: &'static str,
    /// The kit's, such as `TRAILER.sequel` (`TRAILER = dict(sequel=True)`) or `PLUGGED`.
    pub kit: &'static str,
    /// Its kind.
    pub kind: OptionKind,
    /// The kit's default when the config does not set it. For a [`OptionKind::Fighter`] option, `None`
    /// stands for the left corner's fighter.
    pub default: Option<OptionValue>,
}

/// A kit episode config a theme ships with (Table S4): what its music fixes, for the starters (Task 59).
#[derive(Debug, PartialEq)]
pub struct Preset {
    /// The kit config, as the kit's episode finder takes it (`examples/flute_vs_trombone`,
    /// `ballfight_horn_vs_trombone_rematch`).
    pub config: &'static str,
    /// Its fighters, left corner first.
    pub fighters: [&'static str; 2],
    /// Its `BPM`.
    pub bpm: f64,
    /// Its `KEY` and `SCALE`.
    pub key: &'static str,
    pub scale: &'static str,
    /// `CHORDS[:2]`, its `LOOP` and the bars after its last whole loop.
    pub intro: [&'static str; 2],
    pub chord_loop: &'static [&'static str],
    pub tail: &'static [&'static str],
}

/// A theme as the registry lists it.
#[derive(Debug)]
pub struct Theme {
    /// Its name in a fight file (`music.theme`): the kit's mood name.
    pub name: &'static str,
    /// Whether its classic bed is ported (a port sets it in its own folder; [`PORTED_THEMES`] is what is offered).
    pub ported: bool,
    /// What the interface says about it.
    pub card: &'static ThemeCard,
    /// Its options (the kit config names the mood reads besides the music: `TRAILER`, `DRUMLINE`, `PLUGGED`).
    pub options: &'static [ThemeOption],
    /// The Pixel Pitch voices its bed plays itself (`hot_jazz` and `second_line` play the TUBA's), by `pp` id, in
    /// the order the build hands them to the music context ([`Theme::bed_voices`] makes them).
    pub voices: &'static [&'static str],
    /// A voice of its own for a `pp` id no roster fighter plays (`mambo`'s alto voice, ported in its own folder),
    /// or `None` ([`no_own_voice`]: every voice comes from a roster fighter's).
    pub own_voice: fn(&str) -> Option<Arc<dyn Voice>>,
    /// Its classic arrangement, for the fight's options.
    pub classic: fn(&ThemeOptions) -> Result<Box<dyn Bed>, ContentError>,
}

/// A theme with no voice of its own.
pub fn no_own_voice(pp: &str) -> Option<Arc<dyn Voice>> {
    let _ = pp;
    None
}

/// The card of a theme whose port has not filled its own yet.
pub static NOT_PORTED_CARD: ThemeCard = ThemeCard {
    display: "",
    text: "",
    key: "",
    scale: "",
    chord_loops: &[],
    presets: &[],
    bars: (0, 0),
};

impl Theme {
    /// The voices its bed plays, in the order of its `voices`: each its own ([`Theme::own_voice`]), else the voice
    /// of the roster fighter whose `pp` it is, from the fighters' voice registry. The music context looks a name
    /// up among the fighters' voices first, so a voice that a fighter of the fight also plays is that fighter's
    /// (one note cache and one gain for the name, as in the kit).
    ///
    /// # Errors
    /// A voice that is not ported yet, in words (`music.theme: Hot jazz plays the TUBA's voice, which is not
    /// ported yet`; a voice of the theme's own is not named, since such ids may carry a retired name).
    pub fn bed_voices(&self) -> Result<Vec<Arc<dyn Voice>>, ContentError> {
        let mut out = Vec::new();
        for &pp in self.voices {
            let fighter = roster().iter().find(|f| f.pp == pp);
            let voice = (self.own_voice)(pp)
                .or_else(|| fighter.and_then(|f| fighters::voice(&f.name)))
                .filter(|v| v.name() == pp);
            match (voice, fighter) {
                (Some(v), _) => out.push(v),
                (None, Some(f)) => {
                    return Err(ContentError::field(
                        "music.theme",
                        format!(
                            "{} plays the {}'s voice, which is not ported yet",
                            self.display(),
                            f.name
                        ),
                    ));
                }
                (None, None) => {
                    return Err(ContentError::field(
                        "music.theme",
                        format!("{} plays a voice of its own that is not ported yet", self.display()),
                    ));
                }
            }
        }
        Ok(out)
    }

    /// Its name as shown (the card's), or its fight-file name while it has no card.
    pub fn display(&self) -> &'static str {
        if self.card.display.is_empty() {
            self.name
        } else {
            self.card.display
        }
    }

    /// The fight's options for this theme: the fight file's `music.theme_options` checked against the schema,
    /// with every option it leaves out at its default.
    ///
    /// # Errors
    /// An option the theme does not have, a value of the wrong kind, or a fighter option naming someone who is not
    /// in the fight: `music.theme_options.<key>: ...`.
    pub fn options_of(&self, ff: &FightFile) -> Result<ThemeOptions, ContentError> {
        for (key, value) in &ff.music.theme_options {
            let field = format!("music.theme_options.{key}");
            let Some(option) = self.options.iter().find(|o| o.key == key) else {
                let has: Vec<&str> = self.options.iter().map(|o| o.key).collect();
                let has = if has.is_empty() {
                    "it has none".to_string()
                } else {
                    format!("it has: {}", has.join(", "))
                };
                return Err(ContentError::field(
                    field,
                    format!("{} has no option {key:?} ({has})", self.name),
                ));
            };
            match (option.kind, value) {
                (OptionKind::Bool, OptionValue::Bool(_)) => {}
                (OptionKind::Bool, _) => return Err(ContentError::field(field, "must be true or false")),
                (OptionKind::Fighter, OptionValue::Str(name)) => {
                    if !ff.fighters.iter().any(|f| f == name) {
                        return Err(ContentError::field(
                            field,
                            format!("{name:?} is not one of the fighters"),
                        ));
                    }
                }
                (OptionKind::Fighter, _) => {
                    return Err(ContentError::field(field, "must be the name of one of the fighters"));
                }
            }
        }
        let mut out = ThemeOptions::new();
        for option in self.options {
            let value = match (ff.music.theme_options.get(option.key), option.kind) {
                (Some(v), _) => v.clone(),
                (None, _) => match &option.default {
                    Some(v) => v.clone(),
                    None => OptionValue::Str(ff.fighters.first().cloned().unwrap_or_default()),
                },
            };
            out.insert(option.key.to_string(), value);
        }
        Ok(out)
    }

    /// The theme's bed in the arrangement `arrangement` (`classic`) for the fight's `options`.
    ///
    /// # Errors
    /// The theme has no such arrangement (`music.arrangement`), or its bed refuses (a theme that is not ported).
    pub fn bed(&self, arrangement: &str, options: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
        if arrangement != CLASSIC {
            return Err(ContentError::field(
                "music.arrangement",
                format!("{} has no arrangement {arrangement:?}", self.name),
            ));
        }
        (self.classic)(options)
    }
}

/// What a theme's classic bed answers before its port: the theme is not in this app yet.
pub(crate) fn not_ported(name: &str) -> ContentError {
    ContentError::field("music.theme", format!("{name:?} is not ported yet"))
}

/// Every theme, in alphabetical order (the ported and the not yet ported): the kit's fourteen moods and the twelve
/// beds of the older episodes (milestone 4, plan m4 Table O2), each a stub until its port.
static THEMES: [&Theme; 26] = [
    &balkan_fire::THEME,
    &baroque_storm::THEME,
    &big_top::THEME,
    &bossa_noir::THEME,
    &brass_house::THEME,
    &brass_trailer::THEME,
    &dark_breakbeat::THEME,
    &dark_disco::THEME,
    &dark_drumline::THEME,
    &dark_tango::THEME,
    &drift_phonk::THEME,
    &gothic_horror::THEME,
    &heist::THEME,
    &horror_trap::THEME,
    &hot_jazz::THEME,
    &industrial::THEME,
    &mambo::THEME,
    &noir_jazz::THEME,
    &orchestral_drill::THEME,
    &prog_duel::THEME,
    &sea_shanty::THEME,
    &second_line::THEME,
    &ska_punk::THEME,
    &spaghetti_western::THEME,
    &stadium_rock::THEME,
    &villain_march::THEME,
];

/// The names of the themes this app offers ([`PORTED_THEMES`]: every kit theme, the big top first). Which of them
/// a pair may play is [`themes_for`].
pub fn theme_names() -> Vec<&'static str> {
    PORTED_THEMES.to_vec()
}

/// The theme `name` (offered or not), or `None` for a name that is none of the 26.
pub fn theme(name: &str) -> Option<&'static Theme> {
    THEMES.iter().copied().find(|t| t.name == name)
}

/// Every theme, offered or not (the kit's and the older episodes').
pub fn all_themes() -> &'static [&'static Theme] {
    &THEMES
}

/// The offered themes the pair `a` (the left corner) and `b` can play, in [`PORTED_THEMES`]' order: those whose
/// classic bed has every hook both parts call without asking, and every voice it plays. A theme left out is one the
/// kit itself would crash on for this pair (ruling M3-R2: no `boom` under `dark_disco` or `heist` for a fighter
/// whose part plays one, such as the TROMBONE's PEDAL TONE); [`crate::validate`] refuses it in the same terms. A
/// name that is no kit fighter, or one whose part is not here, can play none.
pub fn themes_for(a: &str, b: &str) -> Vec<&'static str> {
    PORTED_THEMES
        .iter()
        .copied()
        .filter(|name| theme(name).is_some_and(|t| t.plays([a, b])))
        .collect()
}

/// The fight the compatibility check builds the parts from: the episode, with the asked fighters put in.
fn base_fight() -> &'static FightFile {
    static BASE: std::sync::OnceLock<FightFile> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        FightFile::parse(include_str!("../../../../fights/flute_vs_trombone.toml")).expect("the episode parses")
    })
}

impl Theme {
    /// Whether this theme is ported, plays its voices, and has every hook the parts of `pair` (left corner
    /// first) call without asking, with its default options.
    fn plays(&self, pair: [&str; 2]) -> bool {
        if !self.ported || self.bed_voices().is_err() {
            return false;
        }
        let mut ff = base_fight().clone();
        ff.fighters = pair.map(str::to_string).to_vec();
        ff.music.theme = self.name.to_string();
        ff.music.theme_options.clear();
        let Ok(bed) = self.options_of(&ff).and_then(|o| self.bed(CLASSIC, &o)) else {
            return false;
        };
        pair.iter().enumerate().all(|(k, name)| {
            fighters::sound(name, k, &ff).is_some_and(|part| part.bed_calls().iter().all(|h| bed.has(*h)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kit's moods, `projects/ballfight/kit/moods/*.py`.
    const KIT_MOODS: [&str; 14] = [
        "baroque_storm",
        "big_top",
        "bossa_noir",
        "brass_trailer",
        "dark_disco",
        "dark_drumline",
        "dark_tango",
        "heist",
        "horror_trap",
        "hot_jazz",
        "mambo",
        "prog_duel",
        "second_line",
        "stadium_rock",
    ];

    fn episode() -> FightFile {
        FightFile::parse(include_str!("../../../../fights/flute_vs_trombone.toml")).expect("the episode")
    }

    /// The older episodes' beds (plan m4 Table O2).
    const OLDER_MOODS: [&str; 12] = [
        "balkan_fire",
        "brass_house",
        "dark_breakbeat",
        "drift_phonk",
        "gothic_horror",
        "industrial",
        "noir_jazz",
        "orchestral_drill",
        "sea_shanty",
        "ska_punk",
        "spaghetti_western",
        "villain_march",
    ];

    #[test]
    fn the_registry_has_every_kit_theme_and_offers_them_all() {
        // every kit theme is offered once, the big top (the starting pair's theme) first
        let offered = theme_names();
        assert_eq!(offered[0], "big_top");
        let mut sorted = offered.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, KIT_MOODS);
        // the registry lists the 26 in alphabetical order: the kit's and the older episodes'
        let names: Vec<&str> = all_themes().iter().map(|t| t.name).collect();
        let mut all: Vec<&str> = KIT_MOODS.iter().chain(&OLDER_MOODS).copied().collect();
        all.sort_unstable();
        assert_eq!(names, all);
        for name in PORTED_THEMES {
            assert!(
                theme(name).is_some_and(|t| t.ported),
                "{name} is offered but not ported"
            );
        }
        let t = theme("big_top").expect("the big top");
        assert_eq!((t.name, t.ported, t.display()), ("big_top", true, "Big top"));
        assert!(t.options.is_empty() && t.voices.is_empty());
        for name in ["polka", "", "Big_Top", "big top"] {
            assert!(theme(name).is_none(), "{name:?}");
        }
        // The arrangement that reproduces the kit's bed, and no other yet.
        let none = ThemeOptions::new();
        let bed = t.bed(CLASSIC, &none).expect("the classic arrangement");
        assert_eq!(bed.reverb(), (2.0, 0.55, 0.22));
        assert_eq!(
            t.bed("layered", &none).err().map(|e| e.to_string()),
            Some(r#"music.arrangement: big_top has no arrangement "layered""#.to_string())
        );
        assert!(t.bed("", &none).is_err());
    }

    /// The fighters whose part plays the bed's `boom` without asking (the HORN only when the bed has no
    /// `trailer_hit`): `dark_disco` and `heist` have neither, so the kit would crash on any pair with one of them.
    const PLAYS_A_BOOM: [&str; 5] = ["TROMBONE", "ORGAN", "TUBA", "EUPHONIUM", "HORN"];

    /// Ruling M3-R2: every ordered pair of kit fighters is offered every theme, except `dark_disco` and `heist` for a
    /// pair with a fighter that plays a boom; a name that is no kit fighter is offered none.
    #[test]
    fn a_pair_is_offered_every_theme_its_parts_can_play() {
        let names: Vec<&str> = roster().iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names.len(), 20);
        for &a in &names {
            for &b in &names {
                let want: Vec<&str> = PORTED_THEMES
                    .iter()
                    .copied()
                    .filter(|t| {
                        !(["dark_disco", "heist"].contains(t)
                            && (PLAYS_A_BOOM.contains(&a) || PLAYS_A_BOOM.contains(&b)))
                    })
                    .collect();
                assert_eq!(themes_for(a, b), want, "{a} vs {b}");
            }
        }
        assert!(themes_for("FLUTE", "KAZOO").is_empty());
        assert!(themes_for("KAZOO", "FLUTE").is_empty());
    }

    /// `validate` agrees with [`themes_for`]: the starting pair validates under every theme it is offered, and each
    /// theme left out is refused for the fighter's part, in words.
    #[test]
    fn validate_refuses_exactly_the_themes_a_pair_is_not_offered() {
        let offered = themes_for("FLUTE", "TROMBONE");
        assert_eq!(offered.len(), 12);
        for name in PORTED_THEMES {
            let mut ff = episode();
            ff.music.theme = name.to_string();
            let got = crate::validate(&ff);
            if offered.contains(&name) {
                assert_eq!(got, Ok(()), "{name}");
            } else {
                assert_eq!(
                    got.unwrap_err().to_string(),
                    format!(
                        "music.theme: {} has no boom (a big drum hit), which the TROMBONE's part plays; pick another \
                         theme",
                        theme(name).unwrap().display()
                    ),
                    "{name}"
                );
            }
        }
    }

    /// A kit theme whose port has not filled its folder, as the registry lists one: no card, no bed.
    static NOT_PORTED: Theme = Theme {
        name: "not_ported_yet",
        ported: false,
        card: &NOT_PORTED_CARD,
        options: &[],
        voices: &[],
        own_voice: no_own_voice,
        classic: |_| Err(not_ported("not_ported_yet")),
    };

    /// The older episodes' themes are listed and, while their stub is not ported, never offered.
    #[test]
    fn the_older_themes_are_listed_and_not_offered_until_ported() {
        for name in OLDER_MOODS {
            let t = theme(name).unwrap_or_else(|| panic!("{name} is not listed"));
            assert_eq!(PORTED_THEMES.contains(&name), t.ported, "{name}");
            if !t.ported {
                assert!(!theme_names().contains(&name), "{name}");
                assert!(themes_for("FLUTE", "TROMBONE").iter().all(|n| *n != name), "{name}");
            }
        }
    }

    /// The kit themes not ported yet (found at run time, so this never names one a port makes whole) have no bed,
    /// are not offered, and `validate` refuses them in words. Once every kit theme is ported, a stand-in theme that
    /// is not takes their place, so the check never passes on nothing.
    #[test]
    fn a_theme_not_ported_yet_has_no_bed() {
        let mut left: Vec<&Theme> = all_themes().iter().copied().filter(|t| !t.ported).collect();
        if left.is_empty() {
            eprintln!("every kit theme is ported: checking a stand-in theme that is not");
            left.push(&NOT_PORTED);
        }
        for t in left {
            assert!(!PORTED_THEMES.contains(&t.name), "{}", t.name);
            assert_eq!(t.display(), t.name);
            let why = format!("music.theme: {:?} is not ported yet", t.name);
            let e = t
                .bed(CLASSIC, &ThemeOptions::new())
                .err()
                .expect("no bed yet")
                .to_string();
            assert_eq!(e, why);
            assert!(!t.plays(["FLUTE", "TROMBONE"]), "{}", t.name);
            assert_eq!(
                crate::soundtrack::validate_with(&episode(), t).unwrap_err().to_string(),
                why
            );
        }
    }

    /// A schema with both kinds, as `brass_trailer` (`TRAILER.sequel`) and `stadium_rock` (`PLUGGED`) have them.
    static OPTIONS: [ThemeOption; 2] = [
        ThemeOption {
            key: "sequel",
            kit: "TRAILER.sequel",
            kind: OptionKind::Bool,
            default: Some(OptionValue::Bool(false)),
        },
        ThemeOption {
            key: "plugged",
            kit: "PLUGGED",
            kind: OptionKind::Fighter,
            default: None,
        },
    ];

    static WITH_OPTIONS: Theme = Theme {
        name: "test_theme",
        ported: true,
        card: &NOT_PORTED_CARD,
        options: &OPTIONS,
        voices: &[],
        own_voice: no_own_voice,
        classic: |_| Err(not_ported("test_theme")),
    };

    #[test]
    fn options_follow_the_schema() {
        let mut ff = episode();
        // nothing set: the defaults, a fighter option's being the left corner's fighter
        let got = WITH_OPTIONS.options_of(&ff).unwrap();
        assert_eq!(got["sequel"], OptionValue::Bool(false));
        assert_eq!(got["plugged"], OptionValue::Str("FLUTE".into()));
        ff.music.theme_options.insert("sequel".into(), OptionValue::Bool(true));
        ff.music
            .theme_options
            .insert("plugged".into(), OptionValue::Str("TROMBONE".into()));
        let got = WITH_OPTIONS.options_of(&ff).unwrap();
        assert_eq!(
            (got["sequel"].clone(), got["plugged"].clone()),
            (OptionValue::Bool(true), OptionValue::Str("TROMBONE".into()))
        );
        let err = |key: &str, v: OptionValue| {
            let mut f = episode();
            f.music.theme_options.insert(key.into(), v);
            WITH_OPTIONS.options_of(&f).unwrap_err().to_string()
        };
        assert_eq!(
            err("sequal", OptionValue::Bool(true)),
            r#"music.theme_options.sequal: test_theme has no option "sequal" (it has: sequel, plugged)"#
        );
        assert_eq!(
            err("sequel", OptionValue::Str("yes".into())),
            "music.theme_options.sequel: must be true or false"
        );
        assert_eq!(
            err("plugged", OptionValue::Str("PIANO".into())),
            r#"music.theme_options.plugged: "PIANO" is not one of the fighters"#
        );
        assert_eq!(
            err("plugged", OptionValue::Int(0)),
            "music.theme_options.plugged: must be the name of one of the fighters"
        );
        // the big top has no options at all
        let mut f = episode();
        f.music.theme_options.insert("sequel".into(), OptionValue::Bool(true));
        assert_eq!(
            theme("big_top").unwrap().options_of(&f).unwrap_err().to_string(),
            r#"music.theme_options.sequel: big_top has no option "sequel" (it has none)"#
        );
    }

    /// A stand-in voice of a theme's own, under an id no roster fighter plays.
    struct OwnVoice;

    impl Voice for OwnVoice {
        fn note(&self, _: &mut bf_sound::voice::PpNoise, _: f64, _: f64, dur: f64) -> Vec<f64> {
            vec![0.0; (dur * 48000.0) as usize]
        }
        fn register(&self) -> (i32, i32) {
            (60, 72)
        }
        fn name(&self) -> &str {
            "OWN VOICE"
        }
        fn can_hold(&self, dur: f64) -> bool {
            dur >= 0.0
        }
    }

    fn own(pp: &str) -> Option<Arc<dyn Voice>> {
        (pp == "OWN VOICE").then(|| Arc::new(OwnVoice) as Arc<dyn Voice>)
    }

    fn voiced(voices: &'static [&'static str]) -> Theme {
        Theme {
            name: "voiced",
            ported: true,
            card: &NOT_PORTED_CARD,
            options: &[],
            voices,
            own_voice: own,
            classic: |_| Err(not_ported("voiced")),
        }
    }

    /// A roster fighter whose voice is not ported yet (the first in the roster's order) as (its name, its `pp`
    /// id), read from the fighters' own folders at run time, so these tests never name a fighter a port will make
    /// whole. `None` once every fighter has its voice: the caller then says so and checks the refusal of an id the
    /// registry does not know.
    fn unported_voice() -> Option<(&'static str, &'static str)> {
        roster()
            .iter()
            .find(|f| fighters::voice(&f.name).is_none())
            .map(|f| (f.name.as_str(), f.pp.as_str()))
    }

    /// `voices` for a test theme, as the theme's `'static` list.
    fn listed(voices: Vec<&'static str>) -> &'static [&'static str] {
        Box::leak(voices.into_boxed_slice())
    }

    #[test]
    fn a_themes_voices_come_from_the_fighters_registry_or_its_own() {
        // none: the big top
        assert!(theme("big_top").unwrap().bed_voices().unwrap().is_empty());
        // a roster fighter's voice, through the fighters' voice registry, and one of its own, in the listed order
        let got = voiced(&["TROMBONE", "OWN VOICE", "FLUTE"]).bed_voices().unwrap();
        let names: Vec<&str> = got.iter().map(|v| v.name()).collect();
        assert_eq!(names, ["TROMBONE", "OWN VOICE", "FLUTE"]);
        // a fighter's voice that is not ported yet, in words
        if let Some((name, pp)) = unported_voice() {
            assert_eq!(
                voiced(listed(vec!["TROMBONE", pp]))
                    .bed_voices()
                    .err()
                    .unwrap()
                    .to_string(),
                format!("music.theme: voiced plays the {name}'s voice, which is not ported yet")
            );
        } else {
            eprintln!("every roster fighter's voice is ported: only an id nobody has is refused below");
        }
        // an id nobody has: not named (also after a voice the registry has)
        assert_eq!(
            voiced(&["NOBODY"]).bed_voices().err().unwrap().to_string(),
            "music.theme: voiced plays a voice of its own that is not ported yet"
        );
        assert_eq!(
            voiced(&["TROMBONE", "NOBODY"]).bed_voices().err().unwrap().to_string(),
            "music.theme: voiced plays a voice of its own that is not ported yet"
        );
        // the kit themes that play a fighter's voice list it (their ports add the rest)
        assert_eq!(theme("hot_jazz").unwrap().voices, ["TUBA"]);
        assert_eq!(theme("second_line").unwrap().voices, ["TUBA"]);
        assert_eq!(theme("stadium_rock").unwrap().voices, ["ACOUSTIC"]);
        assert_eq!(theme("mambo").unwrap().voices, ["CONTRABASS", "PIANO", mambo::ALTO]);
    }

    /// `validate` refuses a ported theme whose bed plays a voice that is not ported, before anything else of the
    /// theme's: a roster fighter's voice not ported yet, named; once every fighter's voice is, an id the registry
    /// does not know.
    #[test]
    fn validate_refuses_a_theme_voice_not_ported() {
        let (voices, why) = match unported_voice() {
            Some((name, pp)) => (
                listed(vec![pp]),
                format!("music.theme: unported_bed plays the {name}'s voice, which is not ported yet"),
            ),
            None => {
                eprintln!("every roster fighter's voice is ported: an id the registry does not know is refused");
                (
                    listed(vec!["NOBODY"]),
                    "music.theme: unported_bed plays a voice of its own that is not ported yet".to_string(),
                )
            }
        };
        let bed = Theme {
            name: "unported_bed",
            ported: true,
            card: &NOT_PORTED_CARD,
            options: &[],
            voices,
            own_voice: no_own_voice,
            classic: |_| Err(not_ported("unported_bed")),
        };
        assert_eq!(
            crate::soundtrack::validate_with(&episode(), &bed)
                .unwrap_err()
                .to_string(),
            why
        );
    }
}
