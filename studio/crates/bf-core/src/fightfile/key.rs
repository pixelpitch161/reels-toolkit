//! The search key: what feeds the fight, as a string defined by its construction alone.

use std::fmt::Write as _;

use super::*;

impl FightFile {
    /// The search key: a string made only of what changes the fight, so the seed shortlist can be kept while it
    /// stays the same (spec section 4, "What feeds the fight"). Fighters in side order, arena, handicap, damage
    /// scale, pair rules and tempo always count; music counts only when `feeds_fight[i]` is true for some fighter
    /// `i` (a fighter whose notes are its weapon; a missing entry counts as false), and then the key, scale, intro
    /// chords, chord loop, chord tail, music seed and that fighter's hand-written line count (never the theme, its
    /// options, the mix, roles, solos or title hits: the kit's `story.roll_rows` reads `LINES`, `LOOP` and the chord
    /// tones, never `ROLES`). Seed, picture seeds, the title card's opt-ins, captions and series tag never count.
    /// The app version is not part of it either: whoever caches the shortlist across app versions adds that.
    ///
    /// The key is a conservative superset of what the fight reads: when music feeds the fight it holds the intro
    /// chords and the chord tail too, though `roll_rows` reads only `LINES`, `LOOP` and the loop's chord tones. Two
    /// fights with the same key always have the same fight; two fights whose keys differ only there have the same
    /// fight too, and only cost a search that a finer key would have saved.
    ///
    /// The string is defined by its construction alone (no hashing, no map iteration other than sorted maps, no
    /// pointers): texts are written as `<byte length>:<text>`, floats as the hex of their bits, so the same inputs
    /// give the same key on any PC and any run.
    ///
    /// Version 2 of the key (fight-file format 2) starts `fight-key 2` and, when music feeds the fight, writes
    /// `;chords_tail=[...]` after the chord loop; everything else is written as version 1 wrote it. So two fights
    /// have the same key exactly when they had before, and a shortlist cached under a version 1 key is looked for
    /// under another key once (it is searched again, never read under the wrong fight).
    pub fn search_key(&self, feeds_fight: &[bool]) -> String {
        let mut o = String::from("fight-key 2;fighters=");
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
        o.push_str(";chords_tail=");
        k_list(&mut o, &m.chords_tail, |o, s| k_str(o, s));
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
}

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
