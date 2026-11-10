//! The fight file (Task 7): the whole recipe of one video, as a small TOML file (design spec section 4, "The fight
//! file"). It mirrors an episode config of the Python kit field for field (`projects/ballfight/kit/episode.py` lists
//! what each one means): `FIGHTERS` is `fighters`, `DMG_SIDE` `handicap`, `RULES` `pair_rules`, `BPM` `tempo_bpm`,
//! `MOOD` `music.theme`, `CHORDS[:2]` `music.chords_intro`, `LOOP` `music.chord_loop`, `LINES`, `SOLO`, `TITLE_HITS`
//! and `ROLES` the `music` tables, `SHARD_SEED` and `FX_SEED` `[picture]`, `CAPTION` and `TAGS` `[captions]`, and
//! `SERIES` `series_tag`.
//!
//! [`FightFile::parse`] reads the text and checks every value; [`FightFile::to_toml`] writes the canonical text: a
//! fixed order of keys and sections, map entries sorted by name, and every float in Rust's shortest text that reads
//! back to the same bits, so the same fight file always gives the same bytes and `parse(to_toml(f)) == f`.
//!
//! Errors are one line, `field: what is wrong`, and nothing here panics on any input text.
//!
//! The format number: a file without `format` is refused ("missing"); `format = 0` is refused (formats start at 1); a
//! format above [`FIGHT_FORMAT`] is refused with "this app reads fight files up to format 1" before anything else in
//! the file is looked at, since a newer file may hold settings this app does not know.
//!
//! This file holds the types and the checks; `read.rs` is the TOML reader, `write.rs` the canonical writer, `key.rs`
//! the search key, `tests.rs` the tests.

use std::collections::BTreeMap;
use std::fmt;

use crate::event::Value;
use crate::grid::Grid;
use crate::harmony;

mod key;
mod read;
mod write;

#[cfg(test)]
mod tests;

/// The newest fight-file format this app reads and writes.
pub const FIGHT_FORMAT: u32 = 1;

/// The two retired fighters: never offered, never ported, never accepted in a fight file.
pub const RETIRED: [&str; 2] = ["GUITAR", "SAX"];

/// The arenas this app has (`ring` in version one).
pub const ARENAS: [&str; 1] = ["ring"];

/// The arrangements this app has (`classic` in Milestone 1).
pub const ARRANGEMENTS: [&str; 1] = ["classic"];

/// How many fighters a fight has.
pub const FIGHTERS: usize = 2;

/// The bars before the chord loop: the title card (bar 0) and the count-in (bar 1); the loop starts on FIGHT!.
pub const INTRO_BARS: usize = 2;

/// The bars of the chord loop.
pub const LOOP_BARS: usize = 8;

/// The tempos a fight file may ask for, in beats a minute (both ends included).
pub const TEMPO_RANGE: (f64, f64) = (30.0, 300.0);

/// The shatter seed when the file gives none (the kit's `SHARD_SEED` default).
pub const SHARD_SEED: u64 = 271;

/// How a rest is written in a `music.lines` list (Python's `None`).
pub const REST: &str = "rest";

/// The largest seed a TOML file can hold (TOML integers are signed 64-bit).
const SEED_MAX: u64 = i64::MAX as u64;

/// One fight: everything that makes its video.
#[derive(Clone, Debug, PartialEq)]
pub struct FightFile {
    /// The fight-file format, 1 to [`FIGHT_FORMAT`].
    pub format: u32,
    /// The version of the app that wrote the file.
    pub app_version: String,
    /// The fighters, left corner (screen left) first.
    pub fighters: Vec<String>,
    /// The arena (`ring`).
    pub arena: String,
    /// Each fighter's damage handicap, in fighter order (the kit's `DMG_SIDE`; default 1.0 each, which is what no
    /// handicap does).
    pub handicap: Vec<f64>,
    /// The damage scale both share (default 1.0).
    pub dmg_scale: f64,
    /// Opt-in engine rules for this pair, such as `swat_cobras = true` (default none).
    pub pair_rules: BTreeMap<String, Value>,
    /// The base tempo; the grid snaps it so a whole number of bars fills 61 s.
    pub tempo_bpm: f64,
    /// The chosen fight (`None` until one is picked).
    pub seed: Option<u64>,
    /// The music settings.
    pub music: Music,
    /// The picture's own seeds.
    pub picture: PictureSeeds,
    /// The post texts.
    pub captions: Captions,
    /// An old episode's "BALLFIGHT #N" tag (new fights leave it out).
    pub series_tag: Option<String>,
}

/// The music of a fight.
#[derive(Clone, Debug, PartialEq)]
pub struct Music {
    /// The theme (the kit's mood), such as `big_top`.
    pub theme: String,
    /// The arrangement (`classic` in Milestone 1; the default).
    pub arrangement: String,
    /// The key's note name, such as `E`.
    pub key: String,
    /// The scale's name (default `minor`, as in the kit).
    pub scale: String,
    /// The chords of bars 0 and 1.
    pub chords_intro: Vec<String>,
    /// The 8-bar chord loop from FIGHT! on.
    pub chord_loop: Vec<String>,
    /// The composer's seed.
    pub music_seed: u64,
    /// Who plays what (`bass`, `melody`, ...), by fighter.
    pub roles: BTreeMap<String, String>,
    /// Hand-written lines over the loop (8 notes a bar = 8ths, 4 = beats), by fighter; `None` is a rest.
    pub lines: BTreeMap<String, Vec<Option<i32>>>,
    /// Victory solos as `(beat, midi note, beats)`, by fighter.
    pub solos: BTreeMap<String, Vec<(f64, i32, f64)>>,
    /// The notes each fighter plays as its name slams in.
    pub title_hits: BTreeMap<String, Vec<i32>>,
}

/// The picture's seeds.
#[derive(Clone, Debug, PartialEq)]
pub struct PictureSeeds {
    /// The shatter seed (default 271).
    pub shard_seed: u64,
    /// The particle seed; `None` takes the kit's default, see [`FightFile::fx_seed`].
    pub fx_seed: Option<u64>,
}

/// The post texts.
#[derive(Clone, Debug, PartialEq)]
pub struct Captions {
    /// The post caption.
    pub caption: String,
    /// Instagram's tags.
    pub instagram: String,
    /// TikTok's tags.
    pub tiktok: String,
    /// YouTube Shorts' title, description and tags (three texts, or none).
    pub shorts: Vec<String>,
}

/// What the content registry says about a fighter name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameStatus {
    Known,
    Unknown,
    Retired,
}

/// A fight file this app cannot use: the field, and what is wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FightFileError {
    /// The field, as a path such as `music.chord_loop[2]` (or `line N` for text that is not TOML).
    pub field: String,
    /// What is wrong, in one line.
    pub message: String,
}

impl fmt::Display for FightFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for FightFileError {}

fn err(field: impl Into<String>, message: impl Into<String>) -> FightFileError {
    FightFileError {
        field: field.into(),
        message: message.into(),
    }
}

impl FightFile {
    /// Every check [`FightFile::parse`] makes, plus the fighter names against the registry: `names` says whether a
    /// name is in the roster, unknown, or retired. For a fight file built in code as well as one read from text.
    pub fn validate(&self, names: &dyn Fn(&str) -> NameStatus) -> Result<(), FightFileError> {
        self.check(Some(names))
    }

    /// The bar and beat grid at this fight's tempo.
    pub fn grid(&self) -> Grid {
        Grid::new(self.tempo_bpm)
    }

    /// The particle seed: `picture.fx_seed`, or the kit's default, the sum of the fighter names' character codes
    /// modulo 1000 (`sum(map(ord, a + b)) % 1000`).
    pub fn fx_seed(&self) -> u64 {
        self.picture.fx_seed.unwrap_or_else(|| {
            self.fighters
                .iter()
                .flat_map(|n| n.chars())
                .map(|c| u64::from(u32::from(c)))
                .sum::<u64>()
                % 1000
        })
    }

    /// The checks of `parse` and `validate`, in a fixed order (the first problem found is reported).
    fn check(&self, names: Option<&dyn Fn(&str) -> NameStatus>) -> Result<(), FightFileError> {
        if self.format == 0 {
            return Err(err("format", "0 is not a fight file format (they start at 1)"));
        }
        if self.format > FIGHT_FORMAT {
            return Err(err(
                "format",
                format!(
                    "this app reads fight files up to format {FIGHT_FORMAT} (this file is format {})",
                    self.format
                ),
            ));
        }
        if self.fighters.len() != FIGHTERS {
            return Err(err(
                "fighters",
                format!("{FIGHTERS} fighters needed, found {}", self.fighters.len()),
            ));
        }
        for (i, name) in self.fighters.iter().enumerate() {
            let field = format!("fighters[{i}]");
            let status = names.map(|f| f(name));
            if RETIRED.contains(&name.as_str()) || status == Some(NameStatus::Retired) {
                return Err(err(field, format!("{name:?} is retired")));
            }
            if let Some(j) = self.fighters[..i].iter().position(|m| m == name) {
                return Err(err(field, format!("{name:?} is already fighters[{j}]")));
            }
            if status == Some(NameStatus::Unknown) {
                return Err(err(field, format!("{name:?} is not in the roster")));
            }
        }
        if !ARENAS.contains(&self.arena.as_str()) {
            return Err(err(
                "arena",
                format!("{:?} is not an arena (this app has: {})", self.arena, ARENAS.join(", ")),
            ));
        }
        if self.handicap.len() != self.fighters.len() {
            return Err(err(
                "handicap",
                format!(
                    "{} numbers needed (one per fighter), found {}",
                    self.fighters.len(),
                    self.handicap.len()
                ),
            ));
        }
        for (i, &h) in self.handicap.iter().enumerate() {
            above_zero(h, &format!("handicap[{i}]"))?;
        }
        above_zero(self.dmg_scale, "dmg_scale")?;
        for (k, v) in &self.pair_rules {
            if has_null(v) {
                return Err(err(path("pair_rules", k), "has no value (TOML cannot write one)"));
            }
        }
        let (lo, hi) = TEMPO_RANGE;
        if !(lo..=hi).contains(&self.tempo_bpm) {
            return Err(err(
                "tempo_bpm",
                format!("must be from {lo} to {hi}, found {:?}", self.tempo_bpm),
            ));
        }
        let seeds = [
            ("seed", self.seed),
            ("music.music_seed", Some(self.music.music_seed)),
            ("picture.shard_seed", Some(self.picture.shard_seed)),
            ("picture.fx_seed", self.picture.fx_seed),
        ];
        for (field, seed) in seeds {
            if let Some(s) = seed.filter(|&s| s > SEED_MAX) {
                return Err(err(field, format!("must be at most {SEED_MAX}, found {s}")));
            }
        }
        self.music.check(&self.fighters)?;
        let n = self.captions.shorts.len();
        if n != 0 && n != 3 {
            return Err(err(
                "captions.shorts",
                format!("3 texts needed (title, description, tags), found {n}"),
            ));
        }
        Ok(())
    }
}

impl Music {
    fn check(&self, fighters: &[String]) -> Result<(), FightFileError> {
        if !ARRANGEMENTS.contains(&self.arrangement.as_str()) {
            return Err(err(
                "music.arrangement",
                format!(
                    "{:?} is not an arrangement (this app has: {})",
                    self.arrangement,
                    ARRANGEMENTS.join(", ")
                ),
            ));
        }
        harmony::pitch_class(&self.key).map_err(|e| err("music.key", e.to_string()))?;
        harmony::scale(&self.scale).map_err(|e| err("music.scale", e.to_string()))?;
        let lists = [
            (
                "music.chords_intro",
                &self.chords_intro,
                INTRO_BARS,
                " (the title card and the count-in)",
            ),
            ("music.chord_loop", &self.chord_loop, LOOP_BARS, ""),
        ];
        for (field, chords, want, why) in lists {
            if chords.len() != want {
                return Err(err(field, format!("{want} chords needed{why}, found {}", chords.len())));
            }
            for (i, c) in chords.iter().enumerate() {
                harmony::parse_chord(c).map_err(|e| err(format!("{field}[{i}]"), e.to_string()))?;
            }
        }
        let fighting = |table: &str, name: &str| {
            if fighters.iter().any(|f| f == name) {
                Ok(())
            } else {
                Err(err(path(table, name), format!("{name} is not one of the fighters")))
            }
        };
        for name in self.roles.keys() {
            fighting("music.roles", name)?;
        }
        for (name, line) in &self.lines {
            fighting("music.lines", name)?;
            let field = path("music.lines", name);
            if line.len() < LOOP_BARS {
                return Err(err(
                    field,
                    format!("at least {LOOP_BARS} notes needed (one a bar), found {}", line.len()),
                ));
            }
            for (i, n) in line.iter().enumerate() {
                if let Some(n) = n {
                    check_midi(*n, &format!("{field}[{i}]"))?;
                }
            }
        }
        for (name, solo) in &self.solos {
            fighting("music.solos", name)?;
            let field = path("music.solos", name);
            for (i, &(beat, note, beats)) in solo.iter().enumerate() {
                let f = format!("{field}[{i}]");
                if !beat.is_finite() || beat < 0.0 {
                    return Err(err(f, format!("the beat must be 0 or more, found {beat:?}")));
                }
                check_midi(note, &f)?;
                if !beats.is_finite() || beats <= 0.0 {
                    return Err(err(f, format!("the length must be above 0, found {beats:?}")));
                }
            }
        }
        for (name, hits) in &self.title_hits {
            fighting("music.title_hits", name)?;
            for (i, &n) in hits.iter().enumerate() {
                check_midi(n, &format!("{}[{i}]", path("music.title_hits", name)))?;
            }
        }
        Ok(())
    }
}

fn path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

fn check_midi(n: i32, field: &str) -> Result<(), FightFileError> {
    if (0..=127).contains(&n) {
        Ok(())
    } else {
        Err(err(field, format!("{n} is not a MIDI note (0 to 127)")))
    }
}

fn above_zero(x: f64, field: &str) -> Result<(), FightFileError> {
    if x.is_nan() || x <= 0.0 {
        return Err(err(field, format!("must be above 0, found {x:?}")));
    }
    if !x.is_finite() {
        return Err(err(field, format!("must be a finite number, found {x:?}")));
    }
    Ok(())
}

fn has_null(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::List(l) => l.iter().any(has_null),
        _ => false,
    }
}
