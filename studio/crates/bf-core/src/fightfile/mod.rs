//! The fight file (Task 7): the whole recipe of one video, as a small TOML file (design spec section 4, "The fight
//! file"). It mirrors an episode config of the Python kit field for field (`projects/ballfight/kit/episode.py` lists
//! what each one means): `FIGHTERS` is `fighters`, `DMG_SIDE` `handicap`, `RULES` `pair_rules`, `BPM` `tempo_bpm`,
//! `MOOD` `music.theme`, `CHORDS[:2]` `music.chords_intro`, `LOOP` `music.chord_loop`, `LINES`, `SOLO`, `TITLE_HITS`
//! and `ROLES` the `music` tables, `SHARD_SEED` and `FX_SEED` `[picture]`, `CAPTION` and `TAGS` `[captions]`, and
//! `SERIES` `series_tag`. Format 2 (milestone 3) adds what the kit's other episode configs use: a 4- or 8-chord
//! `LOOP`, the bars after the last whole loop (`CHORDS[2 + n * len(LOOP):]`, `music.chords_tail`), `MIX`
//! (`music.mix`), the theme's options (`TRAILER`, `DRUMLINE`, `PLUGGED`: `music.theme_options`, checked against the
//! theme's schema by the content registry) and the title card's opt-ins (`TITLE_LINES`, `TITLE_BALLS`, `MIDDLE`:
//! `[picture]`).
//!
//! [`FightFile::parse`] reads the text and checks every value; [`FightFile::to_toml`] writes the canonical text: a
//! fixed order of keys and sections, map entries sorted by name, and every float in Rust's shortest text that reads
//! back to the same bits, so the same fight file always gives the same bytes and `parse(to_toml(f)) == f`.
//!
//! Errors are one line, `field: what is wrong`, and nothing here panics on any input text.
//!
//! The format number: a file without `format` is refused ("missing"); `format = 0` is refused (formats start at 1); a
//! format above [`FIGHT_FORMAT`] is refused with "this app reads fight files up to format 2" before anything else in
//! the file is looked at, since a newer file may hold settings this app does not know. A format 1 file (milestones 1
//! and 2) reads with what format 1 could not say at its defaults (no tail, no mix overrides, no theme options, no
//! title opt-ins), and a setting that format 2 added is refused in it. Reading never writes: a format 1 file stays as
//! it is on disk. The fight it reads to is a format 2 fight ([`FightFile::format`] is [`FIGHT_FORMAT`]), which
//! [`FightFile::to_toml`] writes as format 2.
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

/// The newest fight-file format this app reads and writes (it reads 1 and 2, and writes 2).
pub const FIGHT_FORMAT: u32 = 2;

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

/// The lengths a chord loop may have, in bars (the kit's `LOOP`s are 4 or 8 chords).
pub const LOOP_LENGTHS: [usize; 2] = [4, 8];

/// The most chords the tail may hold (the kit's configs have 0 to 6 bars after their last whole loop).
pub const TAIL_MAX: usize = 8;

/// The tempos a fight file may ask for, in beats a minute (both ends included).
pub const TEMPO_RANGE: (f64, f64) = (30.0, 300.0);

/// The shatter seed when the file gives none (the kit's `SHARD_SEED` default).
pub const SHARD_SEED: u64 = 271;

/// How a rest is written in a `music.lines` list (Python's `None`).
pub const REST: &str = "rest";

/// How [`Middle::Divider`] is written (the kit's word for it).
pub const MIDDLE_DIVIDER: &str = "divider";

/// The largest seed a TOML file can hold (TOML integers are signed 64-bit).
const SEED_MAX: u64 = i64::MAX as u64;

/// One fight: everything that makes its video.
#[derive(Clone, Debug, PartialEq)]
pub struct FightFile {
    /// The fight-file format: [`FIGHT_FORMAT`] for a file read from text (an older file reads as this format's
    /// fight) and for every file this app makes; `validate` refuses a newer one.
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
    /// The picture's own seeds and the title card's opt-ins.
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
    /// The chord loop from FIGHT! on: 4 or 8 chords (the kit's `LOOP`).
    pub chord_loop: Vec<String>,
    /// The bars after the last whole loop, 0 to [`TAIL_MAX`] chords (format 2; a format 1 file has none). The song
    /// is the intro, the loop repeated (the last repeat may stop part-way), then the tail: see
    /// [`crate::harmony::chords_for`].
    pub chords_tail: Vec<String>,
    /// The composer's seed.
    pub music_seed: u64,
    /// Who plays what (`bass`, `melody`, ...), by fighter.
    pub roles: BTreeMap<String, String>,
    /// Hand-written lines over the loop (8 notes a bar = 8ths, 4 = beats), by fighter, or by an extra line a
    /// fighter declares (its name, a space and more, such as `HORN THEME`); `None` is a rest. A line has the same
    /// whole number of notes for every bar of the loop.
    pub lines: BTreeMap<String, Vec<Option<i32>>>,
    /// Victory solos as `(beat, midi note, beats)`, by fighter.
    pub solos: BTreeMap<String, Vec<(f64, i32, f64)>>,
    /// The notes each fighter plays as its name slams in.
    pub title_hits: BTreeMap<String, Vec<i32>>,
    /// The kit's `MIX` (format 2): a bus's `(gain, reverb send)` in the master instead of its own, by bus name.
    /// Which buses exist (the theme's, the fighters', the shared ones) is the content registry's to check. The kit
    /// applies its `MIX` in the config's order, but only buses that already have a level are named in it, and those
    /// keep their place: a sorted map gives the same master.
    pub mix: BTreeMap<String, (f64, f64)>,
    /// The theme's options by the theme's own names (format 2), such as `sequel = true` for the kit's
    /// `TRAILER = dict(sequel=True)`: each is true or false, a number or a text. The theme's schema is checked by
    /// the content registry.
    pub theme_options: BTreeMap<String, Value>,
}

/// The picture's seeds and the title card's opt-ins.
#[derive(Clone, Debug, PartialEq)]
pub struct PictureSeeds {
    /// The shatter seed (default 271).
    pub shard_seed: u64,
    /// The particle seed; `None` takes the kit's default, see [`FightFile::fx_seed`].
    pub fx_seed: Option<u64>,
    /// The kit's `TITLE_LINES` (format 2): a fighter's name on the VS card as two stacked lines, by fighter.
    pub title_lines: BTreeMap<String, (String, String)>,
    /// The kit's `TITLE_BALLS` (format 2): the VS card's two big balls as `[cx, cy, r]`, left corner first, moved
    /// to make room for a two-line name. `None` keeps the card's own.
    pub title_balls: Option<[[f64; 3]; 2]>,
    /// The kit's `MIDDLE` (format 2): another word instead of VS, and how it is turned. `None` is VS.
    pub middle: Option<(String, Middle)>,
}

/// How the title card's middle word is turned (the second value of the kit's `MIDDLE`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Middle {
    /// Along the card's diagonal divider (`'divider'`).
    Divider,
    /// Turned this many degrees.
    Degrees(f64),
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

    /// One chord per bar of `n_bars`: the intro, the loop repeated (the last repeat may stop part-way), then the
    /// tail ([`harmony::chords_for`] of this fight's chords).
    pub fn chords(&self, n_bars: usize) -> Vec<String> {
        let m = &self.music;
        harmony::chords_for(&m.chords_intro, &m.chord_loop, &m.chords_tail, n_bars)
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
        self.picture.check(&self.fighters)?;
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
        if self.chords_intro.len() != INTRO_BARS {
            return Err(err(
                "music.chords_intro",
                format!(
                    "{INTRO_BARS} chords needed (the title card and the count-in), found {}",
                    self.chords_intro.len()
                ),
            ));
        }
        if !LOOP_LENGTHS.contains(&self.chord_loop.len()) {
            let [a, b] = LOOP_LENGTHS;
            return Err(err(
                "music.chord_loop",
                format!("{a} or {b} chords needed, found {}", self.chord_loop.len()),
            ));
        }
        if self.chords_tail.len() > TAIL_MAX {
            return Err(err(
                "music.chords_tail",
                format!(
                    "at most {TAIL_MAX} chords (the bars after the last whole loop), found {}",
                    self.chords_tail.len()
                ),
            ));
        }
        let lists = [
            ("music.chords_intro", &self.chords_intro),
            ("music.chord_loop", &self.chord_loop),
            ("music.chords_tail", &self.chords_tail),
        ];
        for (field, chords) in lists {
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
        let bars = self.chord_loop.len();
        for (name, line) in &self.lines {
            let field = path("music.lines", name);
            if !fighters.iter().any(|f| f == name || is_extra_line(name, f)) {
                return Err(err(
                    field,
                    format!(
                        "{name} is not one of the fighters (an extra line is named after its fighter, such as \
                         \"HORN THEME\")"
                    ),
                ));
            }
            if line.len() < bars {
                return Err(err(
                    field,
                    format!("at least {bars} notes needed (one a bar), found {}", line.len()),
                ));
            }
            if !line.len().is_multiple_of(bars) {
                return Err(err(
                    field,
                    format!(
                        "{} notes do not fit the {bars}-chord loop (every bar needs the same number of notes)",
                        line.len()
                    ),
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
        for (bus, &(gain, send)) in &self.mix {
            let field = path("music.mix", bus);
            if bus.is_empty() {
                return Err(err(field, "a bus needs a name"));
            }
            for (what, x) in [("gain", gain), ("reverb send", send)] {
                if !x.is_finite() || x < 0.0 {
                    return Err(err(field, format!("the {what} must be 0 or more, found {x:?}")));
                }
            }
        }
        for (key, v) in &self.theme_options {
            let field = path("music.theme_options", key);
            match v {
                Value::Bool(_) | Value::Int(_) | Value::Str(_) => {}
                Value::Float(x) if x.is_finite() => {}
                Value::Float(x) => return Err(err(field, format!("must be a finite number, found {x:?}"))),
                Value::Null | Value::List(_) => {
                    return Err(err(field, "must be true or false, a number or a text"));
                }
            }
        }
        Ok(())
    }
}

/// Whether `line` names an extra line of the fighter `fighter`: its name, a space, and at least one more letter
/// (`HORN THEME` is the HORN's).
pub fn is_extra_line(line: &str, fighter: &str) -> bool {
    line.strip_prefix(fighter)
        .and_then(|rest| rest.strip_prefix(' '))
        .is_some_and(|word| !word.trim().is_empty())
}

impl PictureSeeds {
    fn check(&self, fighters: &[String]) -> Result<(), FightFileError> {
        for (name, (a, b)) in &self.title_lines {
            let field = path("picture.title_lines", name);
            if !fighters.iter().any(|f| f == name) {
                return Err(err(field, format!("{name} is not one of the fighters")));
            }
            if a.trim().is_empty() || b.trim().is_empty() {
                return Err(err(field, "both lines need a text"));
            }
        }
        if let Some(balls) = &self.title_balls {
            for (i, &[cx, cy, r]) in balls.iter().enumerate() {
                let field = format!("picture.title_balls[{i}]");
                if !(cx.is_finite() && cy.is_finite()) {
                    return Err(err(field, "the centre must be two finite numbers"));
                }
                if !r.is_finite() || r <= 0.0 {
                    return Err(err(field, format!("the radius must be above 0, found {r:?}")));
                }
            }
        }
        if let Some((word, turn)) = &self.middle {
            if word.trim().is_empty() {
                return Err(err("picture.middle", "the word needs a text"));
            }
            if let Middle::Degrees(d) = turn
                && !d.is_finite()
            {
                return Err(err(
                    "picture.middle",
                    format!("the angle must be a finite number, found {d:?}"),
                ));
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
