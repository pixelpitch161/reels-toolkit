//! The fight-file reader: TOML text to a checked [`FightFile`], with one-line errors naming the field.

use super::*;

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
