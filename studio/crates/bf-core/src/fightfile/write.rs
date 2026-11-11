//! The canonical fight-file writer: a fixed order of keys and sections, so the same fight file is the same bytes.

use std::fmt::Write as _;

use super::*;

impl FightFile {
    /// The canonical text: always the same bytes for the same fight file, and [`FightFile::parse`] reads it back to
    /// an equal value (floats bit for bit). A `Value::Null` in `pair_rules` cannot be written in TOML and is left out
    /// (`validate` refuses it).
    pub fn to_toml(&self) -> String {
        let mut o = String::new();
        // Always the newest format: the text holds format 2's settings whatever the value says.
        let _ = writeln!(o, "format = {FIGHT_FORMAT}");
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
        let _ = writeln!(o, "chords_tail = {}", w_strs(&m.chords_tail));
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
        o.push_str("\n[music.mix]\n");
        for (k, &(gain, send)) in &m.mix {
            let _ = writeln!(o, "{} = [{}, {}]", w_key(k), w_f64(gain), w_f64(send));
        }
        o.push_str("\n[music.theme_options]\n");
        for (k, v) in &m.theme_options {
            let v = match v {
                OptionValue::Bool(b) => b.to_string(),
                OptionValue::Int(i) => i.to_string(),
                OptionValue::Float(x) => w_f64(*x),
                OptionValue::Str(s) => w_str(s),
            };
            let _ = writeln!(o, "{} = {v}", w_key(k));
        }
        let p = &self.picture;
        o.push_str("\n[picture]\n");
        let _ = writeln!(o, "shard_seed = {}", p.shard_seed);
        if let Some(fx) = p.fx_seed {
            let _ = writeln!(o, "fx_seed = {fx}");
        }
        if let Some(balls) = &p.title_balls {
            let balls: Vec<String> = balls.iter().map(|b| format!("[{}]", b.map(w_f64).join(", "))).collect();
            let _ = writeln!(o, "title_balls = [{}]", balls.join(", "));
        }
        if let Some((word, turn)) = &p.middle {
            let turn = match turn {
                Middle::Divider => w_str(MIDDLE_DIVIDER),
                Middle::Degrees(d) => w_f64(*d),
            };
            let _ = writeln!(o, "middle = [{}, {turn}]", w_str(word));
        }
        o.push_str("\n[picture.title_lines]\n");
        for (k, (a, b)) in &p.title_lines {
            let _ = writeln!(o, "{} = [{}, {}]", w_key(k), w_str(a), w_str(b));
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
}

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
