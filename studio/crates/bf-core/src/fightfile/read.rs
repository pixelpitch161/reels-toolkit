//! The fight-file reader: TOML text to a checked [`FightFile`], with one-line errors naming the field.

use super::*;

impl FightFile {
    /// Reads a fight file and checks every value (everything but the fighter names against the roster, which is
    /// [`FightFile::validate`]'s job; the retired fighters are refused here too). Whole numbers are accepted where a
    /// decimal is expected; unknown settings are refused, naming the setting.
    pub fn parse(text: &str) -> Result<FightFile, FightFileError> {
        let top: toml::Table = text.parse().map_err(|e: toml::de::Error| not_toml(text, &e))?;
        let format = match top.get("format") {
            None => {
                return Err(err(
                    "format",
                    format!("missing (a fight file starts with format = {FIGHT_FORMAT})"),
                ));
            }
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
        // 1 or 2 here.
        let v = Version(format as u32);
        check_keys(&top, "", &TOP_KEYS, v)?;
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
            // A format 1 file reads as the format 2 fight it is (and is written as format 2).
            format: FIGHT_FORMAT,
            app_version: as_str(required(&top, "", "app_version")?, "app_version")?.to_string(),
            fighters,
            arena: opt_str(&top, "", "arena")?.unwrap_or_else(|| ARENAS[0].to_string()),
            handicap,
            dmg_scale: opt_f64(&top, "", "dmg_scale")?.unwrap_or(1.0),
            pair_rules,
            tempo_bpm: as_f64(required(&top, "", "tempo_bpm")?, "tempo_bpm")?,
            seed: opt_u64(&top, "", "seed")?,
            music: Music::read(as_table(required(&top, "", "music")?, "music")?, v)?,
            picture: PictureSeeds::read(top.get("picture"), v)?,
            captions: Captions::read(top.get("captions"), v)?,
            series_tag: opt_str(&top, "", "series_tag")?,
        };
        ff.check(None)?;
        Ok(ff)
    }
}

/// The format of the file being read (1 or 2): which settings it may hold.
#[derive(Clone, Copy)]
struct Version(u32);

/// The top-level keys, in the canonical order, each with the format it came in.
const TOP_KEYS: [(&str, u32); 13] = [
    ("format", 1),
    ("app_version", 1),
    ("fighters", 1),
    ("arena", 1),
    ("handicap", 1),
    ("dmg_scale", 1),
    ("tempo_bpm", 1),
    ("seed", 1),
    ("series_tag", 1),
    ("pair_rules", 1),
    ("music", 1),
    ("picture", 1),
    ("captions", 1),
];

/// The `[music]` keys, in the canonical order, each with the format it came in.
const MUSIC_KEYS: [(&str, u32); 14] = [
    ("theme", 1),
    ("arrangement", 1),
    ("key", 1),
    ("scale", 1),
    ("chords_intro", 1),
    ("chord_loop", 1),
    ("chords_tail", 2),
    ("music_seed", 1),
    ("roles", 1),
    ("lines", 1),
    ("solos", 1),
    ("title_hits", 1),
    ("mix", 2),
    ("theme_options", 2),
];

/// The `[picture]` keys, in the canonical order, each with the format it came in.
const PICTURE_KEYS: [(&str, u32); 5] = [
    ("shard_seed", 1),
    ("fx_seed", 1),
    ("title_balls", 2),
    ("middle", 2),
    ("title_lines", 2),
];

/// The `[captions]` keys, each with the format it came in.
const CAPTION_KEYS: [(&str, u32); 4] = [("caption", 1), ("instagram", 1), ("tiktok", 1), ("shorts", 1)];

impl Music {
    fn read(t: &toml::Table, v: Version) -> Result<Music, FightFileError> {
        let p = "music";
        check_keys(t, p, &MUSIC_KEYS, v)?;
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
        let format = v.0;
        for (k, v) in opt_table(t, p, "solos")? {
            let field = path("music.solos", k);
            let mut solo = Vec::new();
            for (i, n) in as_list(v, &field)?.iter().enumerate() {
                let f = format!("{field}[{i}]");
                let shape = || {
                    err(
                        f.clone(),
                        "must be [beat, note, beats] or, for a chord, [beat, [note, ...], beats]",
                    )
                };
                let triple = match n {
                    toml::Value::Array(a) if a.len() == 3 => a,
                    _ => return Err(shape()),
                };
                let beat = number(&triple[0]).ok_or_else(shape)?;
                let note = match &triple[1] {
                    toml::Value::Integer(m) => SoloNote::Note(midi(*m, &f)?),
                    // a chord (format 2): the kit's tuple of notes
                    toml::Value::Array(ns) => {
                        if format < 2 {
                            return Err(err(
                                f,
                                "a chord in a solo is not a setting of format 1 fight files (it came in format 2: a \
                                 file with it says format = 2)",
                            ));
                        }
                        let mut chord = Vec::new();
                        for (j, m) in ns.iter().enumerate() {
                            match m {
                                toml::Value::Integer(m) => match i32::try_from(*m) {
                                    Ok(n) if (0..=127).contains(&n) => chord.push(n),
                                    _ => {
                                        return Err(err(
                                            f,
                                            format!("note {} of the chord: {m} is not a MIDI note (0 to 127)", j + 1),
                                        ));
                                    }
                                },
                                _ => return Err(shape()),
                            }
                        }
                        SoloNote::Chord(chord)
                    }
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
        let mut mix = BTreeMap::new();
        for (k, v) in opt_table(t, p, "mix")? {
            let field = path("music.mix", k);
            let shape = || err(field.clone(), "must be [gain, reverb send]");
            let pair = match v {
                toml::Value::Array(a) if a.len() == 2 => a,
                _ => return Err(shape()),
            };
            let gain = number(&pair[0]).ok_or_else(shape)?;
            let send = number(&pair[1]).ok_or_else(shape)?;
            mix.insert(k.clone(), (gain, send));
        }
        let mut theme_options = BTreeMap::new();
        for (k, v) in opt_table(t, p, "theme_options")? {
            let field = path("music.theme_options", k);
            let value = match v {
                toml::Value::Boolean(b) => OptionValue::Bool(*b),
                toml::Value::Integer(i) => OptionValue::Int(*i),
                toml::Value::Float(x) => OptionValue::Float(*x),
                toml::Value::String(s) => OptionValue::Str(s.clone()),
                other => {
                    return Err(err(
                        field,
                        format!("must be true or false, a number or a text, found {}", kind(other)),
                    ));
                }
            };
            theme_options.insert(k.clone(), value);
        }
        Ok(Music {
            theme: as_str(required(t, p, "theme")?, "music.theme")?.to_string(),
            arrangement: opt_str(t, p, "arrangement")?.unwrap_or_else(|| ARRANGEMENTS[0].to_string()),
            key: as_str(required(t, p, "key")?, "music.key")?.to_string(),
            scale: opt_str(t, p, "scale")?.unwrap_or_else(|| "minor".to_string()),
            chords_intro: str_list(required(t, p, "chords_intro")?, "music.chords_intro")?,
            chord_loop: str_list(required(t, p, "chord_loop")?, "music.chord_loop")?,
            chords_tail: match t.get("chords_tail") {
                Some(v) => str_list(v, "music.chords_tail")?,
                None => Vec::new(),
            },
            music_seed: as_u64(required(t, p, "music_seed")?, "music.music_seed")?,
            roles,
            lines,
            solos,
            title_hits,
            mix,
            theme_options,
        })
    }
}

impl PictureSeeds {
    fn read(v: Option<&toml::Value>, version: Version) -> Result<PictureSeeds, FightFileError> {
        let p = "picture";
        let empty = toml::Table::new();
        let t = match v {
            Some(v) => as_table(v, p)?,
            None => &empty,
        };
        check_keys(t, p, &PICTURE_KEYS, version)?;
        let mut title_lines = BTreeMap::new();
        for (k, v) in opt_table(t, p, "title_lines")? {
            let field = path("picture.title_lines", k);
            let lines = str_list(v, &field)?;
            match <[String; 2]>::try_from(lines) {
                Ok([a, b]) => title_lines.insert(k.clone(), (a, b)),
                Err(_) => return Err(err(field, "must be two texts (the first line and the second)")),
            };
        }
        let title_balls = match t.get("title_balls") {
            None => None,
            Some(v) => {
                let field = "picture.title_balls";
                let shape = || err(field, "must be two balls, each [cx, cy, r]");
                let balls = as_list(v, field)?;
                if balls.len() != 2 {
                    return Err(shape());
                }
                let mut out = [[0.0; 3]; 2];
                for (ball, b) in out.iter_mut().zip(balls) {
                    let nums = match b {
                        toml::Value::Array(a) if a.len() == 3 => a,
                        _ => return Err(shape()),
                    };
                    for (x, n) in ball.iter_mut().zip(nums) {
                        *x = number(n).ok_or_else(shape)?;
                    }
                }
                Some(out)
            }
        };
        let middle = match t.get("middle") {
            None => None,
            Some(v) => {
                let field = "picture.middle";
                let shape = || {
                    err(
                        field,
                        format!("must be [word, {MIDDLE_DIVIDER:?}] or [word, an angle in degrees]"),
                    )
                };
                let pair = match v {
                    toml::Value::Array(a) if a.len() == 2 => a,
                    _ => return Err(shape()),
                };
                let word = match &pair[0] {
                    toml::Value::String(s) => s.clone(),
                    _ => return Err(shape()),
                };
                let turn = match &pair[1] {
                    toml::Value::String(s) if s == MIDDLE_DIVIDER => Middle::Divider,
                    n => Middle::Degrees(number(n).ok_or_else(shape)?),
                };
                Some((word, turn))
            }
        };
        Ok(PictureSeeds {
            shard_seed: opt_u64(t, p, "shard_seed")?.unwrap_or(SHARD_SEED),
            fx_seed: opt_u64(t, p, "fx_seed")?,
            title_lines,
            title_balls,
            middle,
        })
    }
}

impl Captions {
    fn read(v: Option<&toml::Value>, version: Version) -> Result<Captions, FightFileError> {
        let p = "captions";
        let empty = toml::Table::new();
        let t = match v {
            Some(v) => as_table(v, p)?,
            None => &empty,
        };
        check_keys(t, p, &CAPTION_KEYS, version)?;
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

/// Every key of `t` is one of `allowed` and came in the file's format or before it.
fn check_keys(t: &toml::Table, prefix: &str, allowed: &[(&str, u32)], v: Version) -> Result<(), FightFileError> {
    for k in t.keys() {
        match allowed.iter().find(|(name, _)| name == k) {
            None => return Err(err(path(prefix, k), "unknown setting (check the spelling)")),
            Some(&(_, since)) if since > v.0 => {
                return Err(err(
                    path(prefix, k),
                    format!(
                        "not a setting of format {} fight files (it came in format {since}: a file with it says \
                         format = {since})",
                        v.0
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    Ok(())
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
