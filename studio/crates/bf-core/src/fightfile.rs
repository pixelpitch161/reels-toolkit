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

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;

use crate::event::Value;
use crate::grid::Grid;
use crate::harmony;

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
    /// Reads a fight file and checks every value (everything but the fighter names against the roster, which is
    /// [`FightFile::validate`]'s job; the retired fighters are refused here too). Whole numbers are accepted where a
    /// decimal is expected; unknown settings are refused, naming the setting.
    pub fn parse(text: &str) -> Result<FightFile, FightFileError> {
        let top: toml::Table = text.parse().map_err(|e: toml::de::Error| not_toml(text, &e))?;
        let format = match top.get("format") {
            None => return Err(err("format", "missing (a fight file starts with format = 1)")),
            Some(v) => as_u64(v, "format")?,
        };
        if format == 0 {
            return Err(err("format", "0 is not a fight file format (they start at 1)"));
        }
        if format > u64::from(FIGHT_FORMAT) {
            return Err(err(
                "format",
                format!("this app reads fight files up to format {FIGHT_FORMAT} (this file is format {format})"),
            ));
        }
        check_keys(&top, "", &TOP_KEYS)?;
        let fighters = str_list(required(&top, "", "fighters")?, "fighters")?;
        let handicap = match top.get("handicap") {
            Some(v) => f64_list(v, "handicap")?,
            None => vec![1.0; fighters.len()],
        };
        let mut pair_rules = BTreeMap::new();
        if let Some(v) = top.get("pair_rules") {
            for (k, v) in as_table(v, "pair_rules")? {
                let field = path("pair_rules", k);
                pair_rules.insert(k.clone(), to_value(v, &field, true)?);
            }
        }
        let ff = FightFile {
            format: format as u32,
            app_version: as_str(required(&top, "", "app_version")?, "app_version")?.to_string(),
            fighters,
            arena: opt_str(&top, "", "arena")?.unwrap_or_else(|| ARENAS[0].to_string()),
            handicap,
            dmg_scale: opt_f64(&top, "", "dmg_scale")?.unwrap_or(1.0),
            pair_rules,
            tempo_bpm: as_f64(required(&top, "", "tempo_bpm")?, "tempo_bpm")?,
            seed: opt_u64(&top, "", "seed")?,
            music: Music::read(as_table(required(&top, "", "music")?, "music")?)?,
            picture: PictureSeeds::read(top.get("picture"))?,
            captions: Captions::read(top.get("captions"))?,
            series_tag: opt_str(&top, "", "series_tag")?,
        };
        ff.check(None)?;
        Ok(ff)
    }

    /// The canonical text: always the same bytes for the same fight file, and [`FightFile::parse`] reads it back to
    /// an equal value (floats bit for bit). A `Value::Null` in `pair_rules` cannot be written in TOML and is left out
    /// (`validate` refuses it).
    pub fn to_toml(&self) -> String {
        let mut o = String::new();
        let _ = writeln!(o, "format = {}", self.format);
        let _ = writeln!(o, "app_version = {}", w_str(&self.app_version));
        let _ = writeln!(o, "fighters = {}", w_strs(&self.fighters));
        let _ = writeln!(o, "arena = {}", w_str(&self.arena));
        let hc: Vec<String> = self.handicap.iter().map(|&h| w_f64(h)).collect();
        let _ = writeln!(o, "handicap = [{}]", hc.join(", "));
        let _ = writeln!(o, "dmg_scale = {}", w_f64(self.dmg_scale));
        let _ = writeln!(o, "tempo_bpm = {}", w_f64(self.tempo_bpm));
        if let Some(seed) = self.seed {
            let _ = writeln!(o, "seed = {seed}");
        }
        if let Some(tag) = &self.series_tag {
            let _ = writeln!(o, "series_tag = {}", w_str(tag));
        }
        o.push_str("\n[pair_rules]\n");
        for (k, v) in &self.pair_rules {
            if let Some(v) = w_value(v) {
                let _ = writeln!(o, "{} = {v}", w_key(k));
            }
        }
        let m = &self.music;
        o.push_str("\n[music]\n");
        let _ = writeln!(o, "theme = {}", w_str(&m.theme));
        let _ = writeln!(o, "arrangement = {}", w_str(&m.arrangement));
        let _ = writeln!(o, "key = {}", w_str(&m.key));
        let _ = writeln!(o, "scale = {}", w_str(&m.scale));
        let _ = writeln!(o, "chords_intro = {}", w_strs(&m.chords_intro));
        let _ = writeln!(o, "chord_loop = {}", w_strs(&m.chord_loop));
        let _ = writeln!(o, "music_seed = {}", m.music_seed);
        o.push_str("\n[music.roles]\n");
        for (k, v) in &m.roles {
            let _ = writeln!(o, "{} = {}", w_key(k), w_str(v));
        }
        o.push_str("\n[music.lines]\n");
        for (k, line) in &m.lines {
            let notes: Vec<String> = line
                .iter()
                .map(|n| match n {
                    Some(n) => n.to_string(),
                    None => w_str(REST),
                })
                .collect();
            w_rows(&mut o, k, &notes, 8);
        }
        o.push_str("\n[music.solos]\n");
        for (k, solo) in &m.solos {
            let notes: Vec<String> = solo
                .iter()
                .map(|&(b, n, d)| format!("[{}, {n}, {}]", w_f64(b), w_f64(d)))
                .collect();
            w_rows(&mut o, k, &notes, 4);
        }
        o.push_str("\n[music.title_hits]\n");
        for (k, hits) in &m.title_hits {
            let notes: Vec<String> = hits.iter().map(i32::to_string).collect();
            let _ = writeln!(o, "{} = [{}]", w_key(k), notes.join(", "));
        }
        o.push_str("\n[picture]\n");
        let _ = writeln!(o, "shard_seed = {}", self.picture.shard_seed);
        if let Some(fx) = self.picture.fx_seed {
            let _ = writeln!(o, "fx_seed = {fx}");
        }
        let c = &self.captions;
        o.push_str("\n[captions]\n");
        let _ = writeln!(o, "caption = {}", w_str(&c.caption));
        let _ = writeln!(o, "instagram = {}", w_str(&c.instagram));
        let _ = writeln!(o, "tiktok = {}", w_str(&c.tiktok));
        let shorts: Vec<String> = c.shorts.iter().map(|s| w_str(s)).collect();
        w_rows(&mut o, "shorts", &shorts, 1);
        o
    }

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

    /// The search key: a string made only of what changes the fight, so the seed shortlist can be kept while it
    /// stays the same (spec section 4, "What feeds the fight"). Fighters in side order, arena, handicap, damage
    /// scale, pair rules and tempo always count; music counts only when `feeds_fight[i]` is true for some fighter
    /// `i` (a fighter whose notes are its weapon; a missing entry counts as false), and then the key, scale, chords,
    /// chord loop, music seed and that fighter's hand-written line count (never the theme, roles, solos or title
    /// hits). Seed, picture seeds, captions and series tag never count. The app version is not part of it either:
    /// whoever caches the shortlist across app versions adds that.
    ///
    /// The string is defined by its construction alone (no hashing, no map iteration other than sorted maps, no
    /// pointers): texts are written as `<byte length>:<text>`, floats as the hex of their bits, so the same inputs
    /// give the same key on any PC and any run.
    pub fn search_key(&self, feeds_fight: &[bool]) -> String {
        let mut o = String::from("fight-key 1;fighters=");
        k_list(&mut o, &self.fighters, |o, s| k_str(o, s));
        o.push_str(";arena=");
        k_str(&mut o, &self.arena);
        o.push_str(";handicap=");
        k_list(&mut o, &self.handicap, |o, &x| k_f64(o, x));
        o.push_str(";dmg_scale=");
        k_f64(&mut o, self.dmg_scale);
        o.push_str(";tempo_bpm=");
        k_f64(&mut o, self.tempo_bpm);
        o.push_str(";pair_rules=");
        let rules: Vec<(&String, &Value)> = self.pair_rules.iter().collect();
        k_list(&mut o, &rules, |o, (k, v)| {
            k_str(o, k);
            o.push('=');
            k_value(o, v);
        });
        let fed: Vec<&String> = self
            .fighters
            .iter()
            .zip(feeds_fight)
            .filter(|(_, f)| **f)
            .map(|(n, _)| n)
            .collect();
        if fed.is_empty() {
            o.push_str(";music=none");
            return o;
        }
        let m = &self.music;
        o.push_str(";music=[key=");
        k_str(&mut o, &m.key);
        o.push_str(";scale=");
        k_str(&mut o, &m.scale);
        o.push_str(";chords_intro=");
        k_list(&mut o, &m.chords_intro, |o, s| k_str(o, s));
        o.push_str(";chord_loop=");
        k_list(&mut o, &m.chord_loop, |o, s| k_str(o, s));
        let _ = write!(o, ";music_seed={};lines=", m.music_seed);
        let lines: Vec<(&String, &Vec<Option<i32>>)> =
            fed.iter().filter_map(|n| m.lines.get(*n).map(|l| (*n, l))).collect();
        k_list(&mut o, &lines, |o, (n, line)| {
            k_str(o, n);
            o.push('=');
            k_list(o, line, |o, note| match note {
                Some(n) => {
                    let _ = write!(o, "{n}");
                }
                None => o.push('r'),
            });
        });
        o.push(']');
        o
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

/// The top-level keys, in the canonical order.
const TOP_KEYS: [&str; 13] = [
    "format",
    "app_version",
    "fighters",
    "arena",
    "handicap",
    "dmg_scale",
    "tempo_bpm",
    "seed",
    "series_tag",
    "pair_rules",
    "music",
    "picture",
    "captions",
];

/// The `[music]` keys, in the canonical order.
const MUSIC_KEYS: [&str; 11] = [
    "theme",
    "arrangement",
    "key",
    "scale",
    "chords_intro",
    "chord_loop",
    "music_seed",
    "roles",
    "lines",
    "solos",
    "title_hits",
];

impl Music {
    fn read(t: &toml::Table) -> Result<Music, FightFileError> {
        let p = "music";
        check_keys(t, p, &MUSIC_KEYS)?;
        let mut roles = BTreeMap::new();
        for (k, v) in opt_table(t, p, "roles")? {
            let field = path("music.roles", k);
            roles.insert(k.clone(), as_str(v, &field)?.to_string());
        }
        let mut lines = BTreeMap::new();
        for (k, v) in opt_table(t, p, "lines")? {
            let field = path("music.lines", k);
            let mut line = Vec::new();
            for (i, n) in as_list(v, &field)?.iter().enumerate() {
                let f = format!("{field}[{i}]");
                line.push(match n {
                    toml::Value::String(s) if s == REST => None,
                    toml::Value::Integer(m) => Some(midi(*m, &f)?),
                    other => {
                        return Err(err(
                            f,
                            format!("must be a MIDI note (0 to 127) or {REST:?}, found {}", kind(other)),
                        ));
                    }
                });
            }
            lines.insert(k.clone(), line);
        }
        let mut solos = BTreeMap::new();
        for (k, v) in opt_table(t, p, "solos")? {
            let field = path("music.solos", k);
            let mut solo = Vec::new();
            for (i, n) in as_list(v, &field)?.iter().enumerate() {
                let f = format!("{field}[{i}]");
                let shape = || err(f.clone(), "must be [beat, note, beats]");
                let triple = match n {
                    toml::Value::Array(a) if a.len() == 3 => a,
                    _ => return Err(shape()),
                };
                let beat = number(&triple[0]).ok_or_else(shape)?;
                let note = match &triple[1] {
                    toml::Value::Integer(m) => midi(*m, &f)?,
                    _ => return Err(shape()),
                };
                let beats = number(&triple[2]).ok_or_else(shape)?;
                solo.push((beat, note, beats));
            }
            solos.insert(k.clone(), solo);
        }
        let mut title_hits = BTreeMap::new();
        for (k, v) in opt_table(t, p, "title_hits")? {
            let field = path("music.title_hits", k);
            let mut hits = Vec::new();
            for (i, n) in as_list(v, &field)?.iter().enumerate() {
                let f = format!("{field}[{i}]");
                hits.push(match n {
                    toml::Value::Integer(m) => midi(*m, &f)?,
                    other => return Err(err(f, format!("must be a MIDI note (0 to 127), found {}", kind(other)))),
                });
            }
            title_hits.insert(k.clone(), hits);
        }
        Ok(Music {
            theme: as_str(required(t, p, "theme")?, "music.theme")?.to_string(),
            arrangement: opt_str(t, p, "arrangement")?.unwrap_or_else(|| ARRANGEMENTS[0].to_string()),
            key: as_str(required(t, p, "key")?, "music.key")?.to_string(),
            scale: opt_str(t, p, "scale")?.unwrap_or_else(|| "minor".to_string()),
            chords_intro: str_list(required(t, p, "chords_intro")?, "music.chords_intro")?,
            chord_loop: str_list(required(t, p, "chord_loop")?, "music.chord_loop")?,
            music_seed: as_u64(required(t, p, "music_seed")?, "music.music_seed")?,
            roles,
            lines,
            solos,
            title_hits,
        })
    }

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

impl PictureSeeds {
    fn read(v: Option<&toml::Value>) -> Result<PictureSeeds, FightFileError> {
        let empty = toml::Table::new();
        let t = match v {
            Some(v) => as_table(v, "picture")?,
            None => &empty,
        };
        check_keys(t, "picture", &["shard_seed", "fx_seed"])?;
        Ok(PictureSeeds {
            shard_seed: opt_u64(t, "picture", "shard_seed")?.unwrap_or(SHARD_SEED),
            fx_seed: opt_u64(t, "picture", "fx_seed")?,
        })
    }
}

impl Captions {
    fn read(v: Option<&toml::Value>) -> Result<Captions, FightFileError> {
        let p = "captions";
        let empty = toml::Table::new();
        let t = match v {
            Some(v) => as_table(v, p)?,
            None => &empty,
        };
        check_keys(t, p, &["caption", "instagram", "tiktok", "shorts"])?;
        Ok(Captions {
            caption: opt_str(t, p, "caption")?.unwrap_or_default(),
            instagram: opt_str(t, p, "instagram")?.unwrap_or_default(),
            tiktok: opt_str(t, p, "tiktok")?.unwrap_or_default(),
            shorts: match t.get("shorts") {
                Some(v) => str_list(v, "captions.shorts")?,
                None => Vec::new(),
            },
        })
    }
}

// ---------------------------------------------------------------- reading

/// One line for text the TOML reader refuses: the line it stopped on, and its message.
fn not_toml(text: &str, e: &toml::de::Error) -> FightFileError {
    let msg = e.message().split_whitespace().collect::<Vec<_>>().join(" ");
    match e.span() {
        Some(span) => {
            let upto = span.start.min(text.len());
            let line = 1 + text.as_bytes()[..upto].iter().filter(|&&b| b == b'\n').count();
            err(format!("line {line}"), format!("not TOML: {msg}"))
        }
        None => err("fight file", format!("not TOML: {msg}")),
    }
}

fn path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

/// A word for what a TOML value is, for messages.
fn kind(v: &toml::Value) -> &'static str {
    match v {
        toml::Value::String(_) => "a text",
        toml::Value::Integer(_) => "a whole number",
        toml::Value::Float(_) => "a decimal",
        toml::Value::Boolean(_) => "true or false",
        toml::Value::Datetime(_) => "a date",
        toml::Value::Array(_) => "a list",
        toml::Value::Table(_) => "a table",
    }
}

fn check_keys(t: &toml::Table, prefix: &str, allowed: &[&str]) -> Result<(), FightFileError> {
    match t.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(err(path(prefix, k), "unknown setting (check the spelling)")),
        None => Ok(()),
    }
}

fn required<'a>(t: &'a toml::Table, prefix: &str, key: &str) -> Result<&'a toml::Value, FightFileError> {
    t.get(key).ok_or_else(|| err(path(prefix, key), "missing"))
}

fn as_str<'a>(v: &'a toml::Value, field: &str) -> Result<&'a str, FightFileError> {
    match v {
        toml::Value::String(s) => Ok(s),
        other => Err(err(field, format!("must be a text, found {}", kind(other)))),
    }
}

/// A number: a whole number becomes the nearest `f64` (exact up to 2^53).
fn number(v: &toml::Value) -> Option<f64> {
    match v {
        toml::Value::Float(x) => Some(*x),
        toml::Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

fn as_f64(v: &toml::Value, field: &str) -> Result<f64, FightFileError> {
    number(v).ok_or_else(|| err(field, format!("must be a number, found {}", kind(v))))
}

fn as_u64(v: &toml::Value, field: &str) -> Result<u64, FightFileError> {
    match v {
        toml::Value::Integer(i) => {
            u64::try_from(*i).map_err(|_| err(field, format!("must be a whole number from 0 up, found {i}")))
        }
        other => Err(err(field, format!("must be a whole number, found {}", kind(other)))),
    }
}

fn as_list<'a>(v: &'a toml::Value, field: &str) -> Result<&'a [toml::Value], FightFileError> {
    match v {
        toml::Value::Array(a) => Ok(a.as_slice()),
        other => Err(err(field, format!("must be a list, found {}", kind(other)))),
    }
}

fn as_table<'a>(v: &'a toml::Value, field: &str) -> Result<&'a toml::Table, FightFileError> {
    match v {
        toml::Value::Table(t) => Ok(t),
        other => Err(err(field, format!("must be a table, found {}", kind(other)))),
    }
}

fn str_list(v: &toml::Value, field: &str) -> Result<Vec<String>, FightFileError> {
    as_list(v, field)?
        .iter()
        .enumerate()
        .map(|(i, s)| as_str(s, &format!("{field}[{i}]")).map(str::to_string))
        .collect()
}

fn f64_list(v: &toml::Value, field: &str) -> Result<Vec<f64>, FightFileError> {
    as_list(v, field)?
        .iter()
        .enumerate()
        .map(|(i, x)| as_f64(x, &format!("{field}[{i}]")))
        .collect()
}

fn opt_str(t: &toml::Table, prefix: &str, key: &str) -> Result<Option<String>, FightFileError> {
    t.get(key)
        .map(|v| as_str(v, &path(prefix, key)).map(str::to_string))
        .transpose()
}

fn opt_f64(t: &toml::Table, prefix: &str, key: &str) -> Result<Option<f64>, FightFileError> {
    t.get(key).map(|v| as_f64(v, &path(prefix, key))).transpose()
}

fn opt_u64(t: &toml::Table, prefix: &str, key: &str) -> Result<Option<u64>, FightFileError> {
    t.get(key).map(|v| as_u64(v, &path(prefix, key))).transpose()
}

/// A sub-table that may be left out (its entries, sorted by name).
fn opt_table<'a>(
    t: &'a toml::Table,
    prefix: &str,
    key: &str,
) -> Result<Vec<(&'a String, &'a toml::Value)>, FightFileError> {
    match t.get(key) {
        Some(v) => Ok(as_table(v, &path(prefix, key))?.iter().collect()),
        None => Ok(Vec::new()),
    }
}

/// A pair-rule value: true or false, a number, a text, or a list of those.
fn to_value(v: &toml::Value, field: &str, top: bool) -> Result<Value, FightFileError> {
    Ok(match v {
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Integer(i) => Value::Int(*i),
        toml::Value::Float(x) => Value::Float(*x),
        toml::Value::String(s) => Value::Str(s.clone()),
        toml::Value::Array(a) => Value::List(
            a.iter()
                .enumerate()
                .map(|(i, x)| to_value(x, &format!("{field}[{i}]"), false))
                .collect::<Result<_, _>>()?,
        ),
        other => {
            let what = if top {
                "true or false, a number, a text or a list"
            } else {
                "true or false, a number or a text"
            };
            return Err(err(field, format!("must be {what}, found {}", kind(other))));
        }
    })
}

fn midi(m: i64, field: &str) -> Result<i32, FightFileError> {
    let n = i32::try_from(m).map_err(|_| err(field, format!("{m} is not a MIDI note (0 to 127)")))?;
    check_midi(n, field)?;
    Ok(n)
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

// ---------------------------------------------------------------- writing

/// A TOML basic string.
fn w_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => o.push_str("\\r"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            c if c < ' ' || c == '\u{7f}' => {
                let _ = write!(o, "\\u{:04X}", u32::from(c));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn w_strs(list: &[String]) -> String {
    let items: Vec<String> = list.iter().map(|s| w_str(s)).collect();
    format!("[{}]", items.join(", "))
}

/// A key: bare when TOML allows it, quoted otherwise (`"ALTO SAX"`).
fn w_key(k: &str) -> String {
    let bare = !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { k.to_string() } else { w_str(k) }
}

/// A float in Rust's shortest text that reads back to the same bits (`2.0`, `0.4`, `1e-7`, `1e300`), or TOML's
/// `inf`, `-inf`, `nan`, `-nan` (a NaN's payload is not kept).
fn w_f64(x: f64) -> String {
    if x.is_nan() {
        if x.is_sign_negative() { "-nan" } else { "nan" }.to_string()
    } else if x.is_infinite() {
        if x > 0.0 { "inf" } else { "-inf" }.to_string()
    } else {
        format!("{x:?}")
    }
}

/// A pair-rule value (`None` for a null, which TOML cannot hold).
fn w_value(v: &Value) -> Option<String> {
    Some(match v {
        Value::Null => return None,
        Value::Bool(b) => b.to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(x) => w_f64(*x),
        Value::Str(s) => w_str(s),
        Value::List(l) => format!("[{}]", l.iter().filter_map(w_value).collect::<Vec<_>>().join(", ")),
    })
}

/// `key = [` then `per_row` items a row, each row ending in a comma, then `]`; `key = []` when there are none.
fn w_rows(o: &mut String, key: &str, items: &[String], per_row: usize) {
    if items.is_empty() {
        let _ = writeln!(o, "{} = []", w_key(key));
        return;
    }
    let _ = writeln!(o, "{} = [", w_key(key));
    for row in items.chunks(per_row) {
        let _ = writeln!(o, "    {},", row.join(", "));
    }
    o.push_str("]\n");
}

// ---------------------------------------------------------------- the search key

fn k_str(o: &mut String, s: &str) {
    let _ = write!(o, "{}:{s}", s.len());
}

fn k_f64(o: &mut String, x: f64) {
    let _ = write!(o, "{:016x}", x.to_bits());
}

fn k_list<T>(o: &mut String, items: &[T], mut each: impl FnMut(&mut String, &T)) {
    o.push('[');
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            o.push(',');
        }
        each(o, x);
    }
    o.push(']');
}

fn k_value(o: &mut String, v: &Value) {
    match v {
        Value::Null => o.push('n'),
        Value::Bool(b) => {
            let _ = write!(o, "b:{b}");
        }
        Value::Int(i) => {
            let _ = write!(o, "i:{i}");
        }
        Value::Float(x) => {
            o.push_str("f:");
            k_f64(o, *x);
        }
        Value::Str(s) => {
            o.push_str("s:");
            k_str(o, s);
        }
        Value::List(l) => {
            o.push_str("l:");
            k_list(o, l, k_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPISODE: &str = include_str!("../../../fights/flute_vs_trombone.toml");

    fn episode() -> FightFile {
        FightFile::parse(EPISODE).expect("the example fight file parses")
    }

    /// The example with `old` replaced by `new` (which must be there exactly once).
    fn with(old: &str, new: &str) -> String {
        assert_eq!(EPISODE.matches(old).count(), 1, "{old:?} is in the example once");
        EPISODE.replace(old, new)
    }

    /// The error `text` gives, as one line.
    fn parse_err(text: &str) -> String {
        let e = FightFile::parse(text)
            .expect_err("this fight file is rejected")
            .to_string();
        assert!(!e.contains('\n'), "one line: {e:?}");
        e
    }

    /// The roster as the content registry reports it, for `validate`.
    fn names(n: &str) -> NameStatus {
        match n {
            "FLUTE" | "TROMBONE" | "TUBA" => NameStatus::Known,
            "GUITAR" | "SAX" => NameStatus::Retired,
            _ => NameStatus::Unknown,
        }
    }

    fn validate_err(ff: &FightFile) -> String {
        let e = ff
            .validate(&names)
            .expect_err("this fight file is rejected")
            .to_string();
        assert!(!e.contains('\n'), "one line: {e:?}");
        e
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_the_episode() {
        let ff = episode();
        assert_eq!(ff.format, 1);
        assert_eq!(ff.app_version, "0.1.0");
        assert_eq!(ff.fighters, strings(&["FLUTE", "TROMBONE"]));
        assert_eq!(ff.arena, "ring");
        assert_eq!(ff.handicap.len(), 2);
        assert_eq!(ff.handicap[0].to_bits(), 2.0f64.to_bits());
        assert_eq!(ff.handicap[1].to_bits(), 0.40f64.to_bits());
        assert_eq!(ff.dmg_scale.to_bits(), 1.0f64.to_bits());
        assert_eq!(ff.pair_rules.len(), 1);
        assert_eq!(ff.pair_rules["swat_cobras"], Value::Bool(true));
        assert_eq!(ff.tempo_bpm.to_bits(), 132.0f64.to_bits());
        assert_eq!(ff.seed, Some(392));
        assert_eq!(ff.series_tag.as_deref(), Some("BALLFIGHT #15"));
        let m = &ff.music;
        assert_eq!(m.theme, "big_top");
        assert_eq!(m.arrangement, "classic");
        assert_eq!(m.key, "E");
        assert_eq!(m.scale, "harmonic minor");
        assert_eq!(m.chords_intro, strings(&["Em", "B7"]));
        assert_eq!(m.chord_loop, strings(&["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]));
        assert_eq!(m.music_seed, 1);
        assert_eq!(m.roles.len(), 2);
        assert_eq!(m.roles["TROMBONE"], "bass");
        assert_eq!(m.roles["FLUTE"], "melody");
        assert_eq!(m.lines.len(), 1);
        let line = &m.lines["FLUTE"];
        assert_eq!(line.len(), 64);
        assert_eq!(&line[..8], &[76, 79, 83, 79, 76, 75, 76, 71].map(Some));
        assert_eq!(&line[56..], &[78, 75, 71, 75, 78, 81, 83, 75].map(Some));
        assert_eq!(line.iter().map(|n| n.unwrap()).sum::<i32>(), 5087); // Python: sum(LINES["FLUTE"])
        let flute = &m.solos["FLUTE"];
        assert_eq!(flute.len(), 15);
        assert_eq!(flute[0], (0.0, 83, 0.5));
        assert_eq!(flute[14], (6.0, 76, 1.5));
        let trombone = &m.solos["TROMBONE"];
        assert_eq!(trombone.len(), 22);
        assert_eq!(trombone[9], (5.0, 60, 0.75));
        assert_eq!(trombone[21], (11.0, 45, 1.0));
        assert_eq!(m.title_hits["FLUTE"], vec![76, 79, 83, 84, 86, 87, 88]);
        assert_eq!(m.title_hits["TROMBONE"], vec![40, 52]);
        assert_eq!(ff.picture.shard_seed, 271);
        assert_eq!(ff.picture.fx_seed, Some(41));
        assert_eq!(ff.fx_seed(), 41);
        let c = &ff.captions;
        assert_eq!(
            c.caption,
            "The front row vs the back row 🪈🎺 Comment the next match-up 👇"
        );
        assert!(c.instagram.starts_with("#ballfight #flute") && c.instagram.ends_with("#satisfying #music"));
        assert_eq!(
            c.tiktok,
            "#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music"
        );
        assert_eq!(c.shorts.len(), 3);
        assert_eq!(c.shorts[0], "Flute vs Trombone: the front row fights back");
        assert_eq!(ff.grid().n_bars, 34);
        assert_eq!(ff.validate(&names), Ok(()));
    }

    #[test]
    fn to_toml_round_trips() {
        let ff = episode();
        let text = ff.to_toml();
        // the example is written in the canonical layout, so it comes back byte for byte
        assert_eq!(text, EPISODE);
        let back = FightFile::parse(&text).expect("to_toml output parses");
        assert_eq!(back, ff);
        assert_eq!(back.to_toml(), text);
    }

    #[test]
    fn floats_round_trip_bit_for_bit() {
        let mut ff = episode();
        let odd = [
            0.1 + 0.2,
            1.0 / 3.0,
            5e-324,
            2.2250738585072014e-308,
            1.7976931348623157e308,
            1e16,
            123456789012345680.0,
            1e-7,
            -0.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for (i, x) in odd.iter().enumerate() {
            ff.pair_rules.insert(format!("odd_{i:02}"), Value::Float(*x));
        }
        ff.pair_rules.insert(
            "list".into(),
            Value::List(vec![
                Value::Int(-7),
                Value::Str("a \"q\" \\ \t é".into()),
                Value::Bool(false),
            ]),
        );
        ff.handicap = vec![0.1 + 0.2, 1.0 / 3.0];
        ff.dmg_scale = 0.7000000000000001;
        ff.tempo_bpm = 133.7704918032787;
        ff.music.solos.get_mut("FLUTE").unwrap()[3] = (1e-7, 81, 2.0 / 3.0);
        let back = FightFile::parse(&ff.to_toml()).expect("parses");
        assert_eq!(back, ff);
        for (i, x) in odd.iter().enumerate() {
            match &back.pair_rules[&format!("odd_{i:02}")] {
                Value::Float(y) => assert_eq!(y.to_bits(), x.to_bits(), "{x:?}"),
                v => panic!("{v:?}"),
            }
        }
        assert_eq!(back.handicap[0].to_bits(), (0.1f64 + 0.2).to_bits());
        assert_eq!(back.to_toml(), ff.to_toml());
    }

    #[test]
    fn a_whole_number_is_accepted_where_a_decimal_is_expected() {
        let ff = FightFile::parse(&with("tempo_bpm = 132.0", "tempo_bpm = 132")).expect("parses");
        assert_eq!(ff.tempo_bpm.to_bits(), 132.0f64.to_bits());
        let ff = FightFile::parse(&with("handicap = [2.0, 0.4]", "handicap = [2, 0.4]")).expect("parses");
        assert_eq!(ff.handicap[0].to_bits(), 2.0f64.to_bits());
        let ff = FightFile::parse(&with("[0.0, 83, 0.5]", "[0, 83, 0.5]")).expect("parses");
        assert_eq!(ff.music.solos["FLUTE"][0], (0.0, 83, 0.5));
        // the canonical text writes it back as a decimal
        assert_eq!(ff.to_toml(), EPISODE);
    }

    #[test]
    fn optional_settings_take_the_series_defaults() {
        let text = [
            "format = 1",
            "app_version = \"0.1.0\"",
            "fighters = [\"FLUTE\", \"TROMBONE\"]",
            "tempo_bpm = 132",
            "[music]",
            "theme = \"big_top\"",
            "key = \"E\"",
            "chords_intro = [\"Em\", \"B7\"]",
            "chord_loop = [\"Em\", \"Am\", \"B7\", \"Em\", \"C\", \"Am\", \"B7\", \"B7\"]",
            "music_seed = 7",
        ]
        .join("\n");
        let ff = FightFile::parse(&text).expect("parses");
        assert_eq!(ff.arena, "ring");
        assert_eq!(ff.handicap, vec![1.0, 1.0]);
        assert_eq!(ff.dmg_scale, 1.0);
        assert!(ff.pair_rules.is_empty());
        assert_eq!(ff.seed, None);
        assert_eq!(ff.series_tag, None);
        assert_eq!(ff.music.arrangement, "classic");
        assert_eq!(ff.music.scale, "minor");
        assert!(ff.music.roles.is_empty() && ff.music.lines.is_empty());
        assert!(ff.music.solos.is_empty() && ff.music.title_hits.is_empty());
        assert_eq!(ff.picture.shard_seed, 271);
        assert_eq!(ff.picture.fx_seed, None);
        // Python: sum(map(ord, 'FLUTE' + 'TROMBONE')) % 1000
        assert_eq!(ff.fx_seed(), 998);
        assert_eq!(ff.captions.caption, "");
        assert!(ff.captions.shorts.is_empty());
        // None stays None through the canonical text
        assert_eq!(FightFile::parse(&ff.to_toml()), Ok(ff));
    }

    #[test]
    fn search_key_ignores_music() {
        let ff = episode();
        let key = ff.search_key(&[false, false]);
        assert!(!key.is_empty());
        let mut m = ff.clone();
        m.music.theme = "noir_jazz".into();
        m.music.key = "D".into();
        m.music.music_seed = 99;
        m.music.chord_loop[4] = "Am".into();
        m.music.roles.clear();
        m.seed = Some(7);
        m.captions.caption = "other".into();
        m.picture.fx_seed = None;
        m.series_tag = None;
        assert_eq!(
            m.search_key(&[false, false]),
            key,
            "music, seed, captions and picture do not feed the fight"
        );

        let changed = |f: &dyn Fn(&mut FightFile)| {
            let mut c = ff.clone();
            f(&mut c);
            c.search_key(&[false, false])
        };
        assert_ne!(changed(&|c| c.handicap[1] = 0.41), key);
        assert_ne!(changed(&|c| c.dmg_scale = 1.1), key);
        assert_ne!(changed(&|c| c.tempo_bpm = 120.0), key);
        assert_ne!(changed(&|c| c.fighters.reverse()), key, "sides");
        assert_ne!(changed(&|c| c.fighters[1] = "TUBA".into()), key);
        assert_ne!(changed(&|c| c.arena = "cage".into()), key);
        assert_ne!(
            changed(&|c| _ = c.pair_rules.insert("swat_cobras".into(), Value::Bool(false))),
            key
        );
        assert_ne!(changed(&|c| c.pair_rules.clear()), key);
        assert_ne!(changed(&|c| _ = c.pair_rules.insert("x".into(), Value::Int(1))), key);
        // an Int and a Float that are numerically equal are different rules
        let i = changed(&|c| _ = c.pair_rules.insert("n".into(), Value::Int(1)));
        let f = changed(&|c| _ = c.pair_rules.insert("n".into(), Value::Float(1.0)));
        assert_ne!(i, f);

        // a fighter whose notes are its weapon: the chord loop, key and music seed feed the fight
        let fed = ff.search_key(&[true, false]);
        assert_ne!(fed, key);
        let mut c = ff.clone();
        c.music.chord_loop[4] = "Am".into();
        assert_ne!(c.search_key(&[true, false]), fed);
        let mut c = ff.clone();
        c.music.key = "D".into();
        assert_ne!(c.search_key(&[true, false]), fed);
        let mut c = ff.clone();
        c.music.music_seed = 2;
        assert_ne!(c.search_key(&[true, false]), fed);
        let mut c = ff.clone();
        c.music.theme = "noir_jazz".into();
        assert_eq!(c.search_key(&[true, false]), fed, "the theme never feeds the fight");
    }

    #[test]
    fn search_key_is_a_fixed_string() {
        // the same inputs give these bytes on any PC and any run
        assert_eq!(
            episode().search_key(&[false, false]),
            "fight-key 1;fighters=[5:FLUTE,8:TROMBONE];arena=4:ring;handicap=[4000000000000000,3fd999999999999a];\
             dmg_scale=3ff0000000000000;tempo_bpm=4060800000000000;pair_rules=[11:swat_cobras=b:true];music=none"
        );
        let fed = episode().search_key(&[false, true]);
        assert!(fed.ends_with(
            "music=[key=1:E;scale=14:harmonic minor;chords_intro=[2:Em,2:B7];\
             chord_loop=[2:Em,2:Am,2:B7,2:Em,1:C,2:Am,2:B7,2:B7];music_seed=1;lines=[]]"
        ));
        // separators inside names cannot fake another key
        let mut a = episode();
        a.fighters = strings(&["A,1:B", "C"]);
        let mut b = episode();
        b.fighters = strings(&["A", "B,1:C"]);
        assert_ne!(a.search_key(&[false, false]), b.search_key(&[false, false]));
    }

    #[test]
    fn unknown_fighter() {
        let mut ff = episode();
        ff.fighters[1] = "KAZOO".into();
        assert_eq!(validate_err(&ff), r#"fighters[1]: "KAZOO" is not in the roster"#);
        // the file names KAZOO everywhere TROMBONE was (its music too), so only the roster can refuse it
        let text = EPISODE.replace("TROMBONE", "KAZOO");
        let parsed = FightFile::parse(&text).expect("the file itself is well formed");
        assert_eq!(validate_err(&parsed), r#"fighters[1]: "KAZOO" is not in the roster"#);
    }

    #[test]
    fn retired_fighter() {
        let text = with(
            r#"fighters = ["FLUTE", "TROMBONE"]"#,
            r#"fighters = ["SAX", "TROMBONE"]"#,
        );
        assert_eq!(parse_err(&text), r#"fighters[0]: "SAX" is retired"#);
        let text = with(
            r#"fighters = ["FLUTE", "TROMBONE"]"#,
            r#"fighters = ["FLUTE", "GUITAR"]"#,
        );
        assert_eq!(parse_err(&text), r#"fighters[1]: "GUITAR" is retired"#);
        let mut ff = episode();
        ff.fighters[0] = "SAX".into();
        assert_eq!(validate_err(&ff), r#"fighters[0]: "SAX" is retired"#);
        // the registry's word counts too
        let mut ff = episode();
        ff.fighters[1] = "OLD".into();
        let e = ff.validate(&|n| if n == "OLD" { NameStatus::Retired } else { names(n) });
        assert_eq!(e.unwrap_err().to_string(), r#"fighters[1]: "OLD" is retired"#);
    }

    #[test]
    fn the_same_fighter_twice() {
        let text = with(
            r#"fighters = ["FLUTE", "TROMBONE"]"#,
            r#"fighters = ["FLUTE", "FLUTE"]"#,
        );
        assert_eq!(parse_err(&text), r#"fighters[1]: "FLUTE" is already fighters[0]"#);
    }

    #[test]
    fn one_fighter_or_three() {
        let text = with(r#"fighters = ["FLUTE", "TROMBONE"]"#, r#"fighters = ["FLUTE"]"#);
        assert_eq!(parse_err(&text), "fighters: 2 fighters needed, found 1");
        let text = with(
            r#"fighters = ["FLUTE", "TROMBONE"]"#,
            r#"fighters = ["FLUTE", "TROMBONE", "TUBA"]"#,
        );
        assert_eq!(parse_err(&text), "fighters: 2 fighters needed, found 3");
    }

    #[test]
    fn format_newer_than_the_app() {
        let text = with("format = 1", "format = 2");
        assert_eq!(
            parse_err(&text),
            "format: this app reads fight files up to format 1 (this file is format 2)"
        );
        // a newer file is refused for its format, before any setting this app does not know
        let text = with("format = 1", "format = 7\nwings = true");
        assert_eq!(
            parse_err(&text),
            "format: this app reads fight files up to format 1 (this file is format 7)"
        );
    }

    #[test]
    fn format_zero_or_missing() {
        assert_eq!(
            parse_err(&with("format = 1", "format = 0")),
            "format: 0 is not a fight file format (they start at 1)"
        );
        assert_eq!(
            parse_err(&with("format = 1\n", "")),
            "format: missing (a fight file starts with format = 1)"
        );
        assert_eq!(
            parse_err(&with("format = 1", "format = -1")),
            "format: must be a whole number from 0 up, found -1"
        );
        assert_eq!(
            parse_err(&with("format = 1", "format = \"1\"")),
            "format: must be a whole number, found a text"
        );
    }

    #[test]
    fn a_seven_chord_loop() {
        let text = with(
            r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
            r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7"]"#,
        );
        assert_eq!(parse_err(&text), "music.chord_loop: 8 chords needed, found 7");
    }

    #[test]
    fn an_unparseable_chord() {
        let text = with(
            r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
            r#"chord_loop = ["Em", "Am", "H7", "Em", "C", "Am", "B7", "B7"]"#,
        );
        assert_eq!(
            parse_err(&text),
            r#"music.chord_loop[2]: "H7" is not a chord name (a root such as E, F# or Bb, then a quality)"#
        );
        let text = with(r#"chords_intro = ["Em", "B7"]"#, r#"chords_intro = ["Em", "B13"]"#);
        assert_eq!(
            parse_err(&text),
            r#"music.chords_intro[1]: "B13" is not a chord name (a root such as E, F# or Bb, then a quality)"#
        );
        let text = with(r#"key = "E""#, r#"key = "X""#);
        assert_eq!(
            parse_err(&text),
            r#"music.key: "X" is not a note name (A to G, then # or b)"#
        );
        let text = with(r#"scale = "harmonic minor""#, r#"scale = "harmonic""#);
        assert_eq!(parse_err(&text), r#"music.scale: "harmonic" is not a known scale"#);
    }

    #[test]
    fn an_empty_file() {
        assert_eq!(parse_err(""), "format: missing (a fight file starts with format = 1)");
        assert_eq!(
            parse_err("\n\n  \n"),
            "format: missing (a fight file starts with format = 1)"
        );
    }

    #[test]
    fn text_that_is_not_toml() {
        let e = parse_err("format = 1\nthis is not a fight file\n");
        assert!(e.starts_with("line 2: not TOML: "), "{e}");
        let e = parse_err("{\"format\": 1}");
        assert!(e.starts_with("line 1: not TOML: "), "{e}");
        let e = parse_err(&with("[captions]", "[captions"));
        assert!(e.contains(": not TOML: "), "{e}");
    }

    #[test]
    fn a_wrong_type() {
        assert_eq!(
            parse_err(&with("tempo_bpm = 132.0", "tempo_bpm = \"fast\"")),
            "tempo_bpm: must be a number, found a text"
        );
        assert_eq!(
            parse_err(&with("seed = 392", "seed = 392.5")),
            "seed: must be a whole number, found a decimal"
        );
        assert_eq!(
            parse_err(&with("fighters = [\"FLUTE\", \"TROMBONE\"]", "fighters = \"FLUTE\"")),
            "fighters: must be a list, found a text"
        );
        assert_eq!(
            parse_err(&with("swat_cobras = true", "swat_cobras = { on = true }")),
            "pair_rules.swat_cobras: must be true or false, a number, a text or a list, found a table"
        );
        assert_eq!(
            parse_err(&with("[0.0, 83, 0.5]", "[0.0, 83]")),
            "music.solos.FLUTE[0]: must be [beat, note, beats]"
        );
        assert_eq!(
            parse_err(&with(
                "76, 79, 83, 79, 76, 75, 76, 71,",
                "76, 79, 83, 79, 76, 75, 76, 7.1,"
            )),
            "music.lines.FLUTE[7]: must be a MIDI note (0 to 127) or \"rest\", found a decimal"
        );
    }

    #[test]
    fn a_rest_in_a_line() {
        let ff = FightFile::parse(&with(
            "76, 79, 83, 79, 76, 75, 76, 71,",
            "76, \"rest\", 83, 79, 76, 75, 76, 71,",
        ))
        .expect("parses");
        assert_eq!(ff.music.lines["FLUTE"][1], None);
        assert_eq!(FightFile::parse(&ff.to_toml()), Ok(ff));
    }

    #[test]
    fn a_negative_handicap() {
        let text = with("handicap = [2.0, 0.4]", "handicap = [2.0, -0.4]");
        assert_eq!(parse_err(&text), "handicap[1]: must be above 0, found -0.4");
        let text = with("handicap = [2.0, 0.4]", "handicap = [0, 0.4]");
        assert_eq!(parse_err(&text), "handicap[0]: must be above 0, found 0.0");
        let text = with("handicap = [2.0, 0.4]", "handicap = [nan, 0.4]");
        assert_eq!(parse_err(&text), "handicap[0]: must be above 0, found NaN");
    }

    #[test]
    fn a_handicap_of_the_wrong_length() {
        let text = with("handicap = [2.0, 0.4]", "handicap = [2.0, 0.4, 1.0]");
        assert_eq!(
            parse_err(&text),
            "handicap: 2 numbers needed (one per fighter), found 3"
        );
        let text = with("handicap = [2.0, 0.4]", "handicap = [2.0]");
        assert_eq!(
            parse_err(&text),
            "handicap: 2 numbers needed (one per fighter), found 1"
        );
    }

    #[test]
    fn unknown_settings_are_named() {
        assert_eq!(
            parse_err(&with("seed = 392", "seed = 392\ntempo = 120")),
            "tempo: unknown setting (check the spelling)"
        );
        assert_eq!(
            parse_err(&with("music_seed = 1", "music_seed = 1\nmusic_sead = 2")),
            "music.music_sead: unknown setting (check the spelling)"
        );
        assert_eq!(
            parse_err(&with("fx_seed = 41", "fx_seed = 41\nfx_sed = 41")),
            "picture.fx_sed: unknown setting (check the spelling)"
        );
        assert_eq!(
            parse_err(&with("[captions]", "[captions]\ntwitter = \"x\"")),
            "captions.twitter: unknown setting (check the spelling)"
        );
        assert_eq!(
            parse_err(&with("[picture]", "[pictures]")),
            "pictures: unknown setting (check the spelling)"
        );
    }

    #[test]
    fn music_for_someone_not_fighting() {
        let text = with("FLUTE = \"melody\"", "TUBA = \"melody\"");
        assert_eq!(parse_err(&text), "music.roles.TUBA: TUBA is not one of the fighters");
        let text = with("TROMBONE = [40, 52]", "TROMBONE = [40, 152]");
        assert_eq!(
            parse_err(&text),
            "music.title_hits.TROMBONE[1]: 152 is not a MIDI note (0 to 127)"
        );
    }

    #[test]
    fn other_values_out_of_range() {
        assert_eq!(
            parse_err(&with("tempo_bpm = 132.0", "tempo_bpm = 1e300")),
            "tempo_bpm: must be from 30 to 300, found 1e300"
        );
        assert_eq!(
            parse_err(&with("dmg_scale = 1.0", "dmg_scale = -1.0")),
            "dmg_scale: must be above 0, found -1.0"
        );
        assert_eq!(
            parse_err(&with("arena = \"ring\"", "arena = \"cage\"")),
            r#"arena: "cage" is not an arena (this app has: ring)"#
        );
        assert_eq!(
            parse_err(&with("arrangement = \"classic\"", "arrangement = \"remix\"")),
            r#"music.arrangement: "remix" is not an arrangement (this app has: classic)"#
        );
        assert_eq!(
            parse_err(&with(r#"chords_intro = ["Em", "B7"]"#, r#"chords_intro = ["Em"]"#)),
            "music.chords_intro: 2 chords needed (the title card and the count-in), found 1"
        );
        assert_eq!(
            parse_err(&with("seed = 392", "seed = -392")),
            "seed: must be a whole number from 0 up, found -392"
        );
        assert_eq!(parse_err(&with("theme = \"big_top\"\n", "")), "music.theme: missing");
        assert_eq!(
            parse_err(&with("[music]", "[musik]")),
            "musik: unknown setting (check the spelling)"
        );
    }

    #[test]
    fn no_input_text_panics() {
        // every prefix of the example, and of a few broken files, gives a result, never a panic
        let mut cut: Vec<usize> = EPISODE.char_indices().map(|(i, _)| i).collect();
        cut.push(EPISODE.len());
        for i in cut {
            let _ = FightFile::parse(&EPISODE[..i]);
        }
        for text in [
            "\u{0}",
            "format = 1\n[[music]]\n",
            "format = 9999999999999",
            "format = 1\nfighters = [[[]]]",
        ] {
            let _ = FightFile::parse(text);
        }
    }
}
