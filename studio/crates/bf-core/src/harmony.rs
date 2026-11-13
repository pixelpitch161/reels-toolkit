//! Keys, scales and chords (Task 5).
//!
//! A port of the harmony part of `projects/ballfight/kit/episode.py`: `NOTE_PC`, `QUALITY`, `SCALES`,
//! `pitch_class`, `parse_chord`, the harmony half of `build_timeline` (`KEY_PC`, `KEY_ROOT`, `SCALE`, the
//! `CHORDS` length check, `CHORD_PCS`, the default `LOOP`) and the helpers `chord_at`, `chord_tones`, `scale_notes`,
//! `root_above` and `loop_index` (`default_line`, `default_solo` and the plain half of `line_at` are in
//! [`crate::lines`]). Where Python raises on a bad name (a `KeyError` or an `IndexError`) this returns a
//! [`HarmonyError`] naming the text; nothing here panics on what a config holds.

use std::collections::BTreeMap;
use std::fmt;

use crate::grid::Grid;
use crate::py;

/// `NOTE_PC`: the pitch class of each note letter.
pub const NOTE_PC: [(char, i32); 7] = [('C', 0), ('D', 2), ('E', 4), ('F', 5), ('G', 7), ('A', 9), ('B', 11)];

/// `QUALITY`: a chord's intervals above its root, by the suffix after the root, in Python's order.
pub const QUALITY: [(&str, &[i32]); 13] = [
    ("", &[0, 4, 7]),
    ("m", &[0, 3, 7]),
    ("7", &[0, 4, 7, 10]),
    ("m7", &[0, 3, 7, 10]),
    ("maj7", &[0, 4, 7, 11]),
    ("dim", &[0, 3, 6]),
    ("dim7", &[0, 3, 6, 9]),
    ("m7b5", &[0, 3, 6, 10]),
    ("aug", &[0, 4, 8]),
    ("sus2", &[0, 2, 7]),
    ("sus4", &[0, 5, 7]),
    ("6", &[0, 4, 7, 9]),
    ("m6", &[0, 3, 7, 9]),
];

/// `SCALES`: each scale's steps above the key, by name, in Python's order.
pub const SCALES: [(&str, &[i32]); 8] = [
    ("major", &[0, 2, 4, 5, 7, 9, 11]),
    ("minor", &[0, 2, 3, 5, 7, 8, 10]),
    ("harmonic minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("melodic minor", &[0, 2, 3, 5, 7, 9, 11]),
    ("dorian", &[0, 2, 3, 5, 7, 9, 10]),
    ("phrygian", &[0, 1, 3, 5, 7, 8, 10]),
    ("mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
    ("lydian", &[0, 2, 4, 6, 7, 9, 11]),
];

/// A key, chord or scale name the kit would reject, or a progression that does not fit the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HarmonyError {
    /// A note name that does not start with a letter A to G (Python: `KeyError` or `IndexError`).
    BadNote(String),
    /// A chord name whose root or quality is unknown.
    BadChord(String),
    /// A scale name that is not in [`SCALES`].
    UnknownScale(String),
    /// A chord the episode's `CHORDS` does not hold (Python: `KeyError` on `CHORD_PCS`).
    UnknownChord(String),
    /// `CHORDS` must hold one chord per bar.
    ChordCount { got: usize, want: usize },
    /// The loop has no chords (Python would divide by zero in `loop_index`).
    EmptyLoop,
    /// No note from `lo` to `hi` is a tone of the chord, where a line or a solo is to be made of them (Python
    /// divides by zero in `default_line` and indexes an empty list in `default_solo`).
    NoTones { chord: String, lo: i32, hi: i32 },
    /// A line with fewer notes than the loop has bars (Python divides by zero in `line_at`: no note a bar).
    ShortLine { notes: usize, bars: usize },
}

impl fmt::Display for HarmonyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarmonyError::BadNote(s) => write!(f, "{s:?} is not a note name (A to G, then # or b)"),
            HarmonyError::BadChord(s) => write!(
                f,
                "{s:?} is not a chord name (a root such as E, F# or Bb, then a quality)"
            ),
            HarmonyError::UnknownScale(s) => write!(f, "{s:?} is not a known scale"),
            HarmonyError::UnknownChord(s) => write!(f, "{s:?} is not one of the episode's chords"),
            HarmonyError::ChordCount { got, want } => write!(f, "CHORDS has {got} bars, the episode has {want}"),
            HarmonyError::EmptyLoop => write!(f, "the chord loop is empty"),
            HarmonyError::NoTones { chord, lo, hi } => {
                write!(f, "no note from {lo} to {hi} is a tone of {chord}")
            }
            HarmonyError::ShortLine { notes, bars } => write!(
                f,
                "a line of {notes} note(s) is shorter than the loop's {bars} bars (it needs at least one a bar)"
            ),
        }
    }
}

impl std::error::Error for HarmonyError {}

/// A parsed chord: its root's pitch class and its quality's intervals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    /// The root's pitch class, 0 to 11.
    pub root: i32,
    /// The intervals above the root, from [`QUALITY`].
    pub intervals: &'static [i32],
}

impl Chord {
    /// The chord's pitch classes, root first (the second item of Python's `parse_chord`).
    pub fn pcs(&self) -> Vec<i32> {
        self.intervals.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }
}

/// `pitch_class(name)`: the letter's pitch class, plus one for each `#` and minus one for each `b` after it (any
/// other character adds nothing, as in Python), modulo 12.
pub fn pitch_class(name: &str) -> Result<i32, HarmonyError> {
    let mut chars = name.chars();
    let letter = chars.next().ok_or_else(|| HarmonyError::BadNote(name.to_string()))?;
    let mut pc = NOTE_PC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, pc)| *pc)
        .ok_or_else(|| HarmonyError::BadNote(name.to_string()))?;
    for ch in chars {
        pc += match ch {
            '#' => 1,
            'b' => -1,
            _ => 0,
        };
    }
    Ok(pc.rem_euclid(12))
}

/// `parse_chord(name)`: a root (a letter, then `#` or `b` if the second character is one) and a quality from
/// [`QUALITY`] for the rest. `'B7'` gives root 11 and intervals `(0, 4, 7, 10)`, whose pitch classes are
/// `(11, 3, 6, 9)`.
pub fn parse_chord(name: &str) -> Result<Chord, HarmonyError> {
    let bad = || HarmonyError::BadChord(name.to_string());
    let mut chars = name.char_indices();
    chars.next().ok_or_else(bad)?;
    // the byte offset where the quality starts: after one character, or two when the second is '#' or 'b'
    let split = match chars.next() {
        Some((i, c)) if c == '#' || c == 'b' => i + c.len_utf8(),
        Some((i, _)) => i,
        None => name.len(),
    };
    let root = pitch_class(&name[..split]).map_err(|_| bad())?;
    let intervals = QUALITY
        .iter()
        .find(|(q, _)| *q == &name[split..])
        .map(|(_, i)| *i)
        .ok_or_else(bad)?;
    Ok(Chord { root, intervals })
}

/// `SCALES[name]`.
pub fn scale(name: &str) -> Result<&'static [i32], HarmonyError> {
    SCALES
        .iter()
        .find(|(s, _)| *s == name)
        .map(|(_, steps)| *steps)
        .ok_or_else(|| HarmonyError::UnknownScale(name.to_string()))
}

/// One chord per bar for `n_bars` bars: the intro, then the loop repeated (the last repeat may stop part-way), then
/// the tail on the last bars, as the kit's configs write `CHORDS = [intro 2] + LOOP * n + tail` (the example's
/// `['Em', 'B7'] + LOOP * 4` for 34 bars, the rematch's `['Dm', 'A'] + LOOP * 3 + ['Dm', 'Bb', 'Gm', 'A', 'A']`
/// for 31), but for any bar count. A fight file of format 1 has no tail. When the bars run out first, the intro
/// comes first, then the tail (cut where the bars end), and the loop gets what is left. With no loop the intro
/// repeats in its place; with no intro and no loop only the tail is there (the list is then shorter than
/// `n_bars`, and empty when there are no chords at all).
pub fn chords_for(intro: &[String], loop_: &[String], tail: &[String], n_bars: usize) -> Vec<String> {
    let cycle = if loop_.is_empty() { intro } else { loop_ };
    let mut out: Vec<String> = intro.iter().take(n_bars).cloned().collect();
    let looped = (n_bars - out.len()).saturating_sub(tail.len());
    if !cycle.is_empty() {
        out.extend(cycle.iter().cycle().take(looped).cloned());
    }
    let room = n_bars - out.len();
    out.extend(tail.iter().take(room).cloned());
    out
}

/// The episode's harmony: its key, scale and chords, with the timeline's harmony helpers.
#[derive(Clone, Debug, PartialEq)]
pub struct Harmony {
    /// `KEY_PC`: the key's pitch class.
    pub key_pc: i32,
    /// `KEY_ROOT`: the key's root in the C2 octave, `36 + KEY_PC` (E is 40).
    pub key_root: i32,
    /// `SCALE`: the scale's steps above the key.
    pub scale: Vec<i32>,
    /// `CHORDS`: one chord per bar.
    pub chords: Vec<String>,
    /// `LOOP`: the chord cycle the lines follow from bar `loop_start`.
    pub loop_: Vec<String>,
    /// `LOOP_START`: the bar the loop starts on (2: FIGHT!).
    pub loop_start: usize,
    /// `BAR`, from the grid.
    bar: f64,
    /// `CHORD_PCS`: the pitch classes of every chord in `CHORDS`, by name.
    chord_pcs: BTreeMap<String, Vec<i32>>,
}

impl Harmony {
    /// The harmony half of `build_timeline`: `key` as a note name, `scale` as steps (see [`scale`] for a name),
    /// `chords` one per bar of `grid`, and `loop_` the chord cycle (`None`: `CHORDS[loop_start..loop_start + 8]`,
    /// Python's default). Every chord in `chords` and in the loop must parse.
    pub fn new(
        grid: &Grid,
        key: &str,
        scale: &[i32],
        chords: Vec<String>,
        loop_: Option<Vec<String>>,
        loop_start: usize,
    ) -> Result<Harmony, HarmonyError> {
        let key_pc = pitch_class(key)?;
        if chords.len() != grid.n_bars {
            return Err(HarmonyError::ChordCount {
                got: chords.len(),
                want: grid.n_bars,
            });
        }
        let mut chord_pcs = BTreeMap::new();
        for c in &chords {
            if !chord_pcs.contains_key(c) {
                chord_pcs.insert(c.clone(), parse_chord(c)?.pcs());
            }
        }
        let loop_ = match loop_ {
            Some(l) => l,
            None => {
                let start = loop_start.min(chords.len());
                let end = loop_start.saturating_add(8).min(chords.len());
                chords[start..end].to_vec()
            }
        };
        if loop_.is_empty() {
            return Err(HarmonyError::EmptyLoop);
        }
        for c in &loop_ {
            parse_chord(c)?;
        }
        Ok(Harmony {
            key_pc,
            key_root: 36 + key_pc,
            scale: scale.to_vec(),
            chords,
            loop_,
            loop_start,
            bar: grid.bar,
            chord_pcs,
        })
    }

    /// `CHORD_PCS[chord]`: the pitch classes of one of the episode's chords, root first.
    pub fn chord_pcs(&self, chord: &str) -> Result<&[i32], HarmonyError> {
        self.chord_pcs
            .get(chord)
            .map(Vec::as_slice)
            .ok_or_else(|| HarmonyError::UnknownChord(chord.to_string()))
    }

    /// `chord_at(t)`: the chord of the bar `t` falls in, `CHORDS[int(max(0.0, t) // BAR) % N_BARS]`.
    pub fn chord_at(&self, t: f64) -> &str {
        // max(0.0, t) keeps 0.0 unless t is greater (so NaN gives 0.0).
        let t = if t > 0.0 { t } else { 0.0 };
        let bar = py::floor_div(t, self.bar) as usize;
        &self.chords[bar % self.chords.len()]
    }

    /// `chord_tones(lo, hi, chord)`: every midi note from `lo` to `hi` (both included) in the chord.
    pub fn chord_tones(&self, lo: i32, hi: i32, chord: &str) -> Result<Vec<i32>, HarmonyError> {
        let pcs = self.chord_pcs(chord)?;
        Ok((lo..=hi).filter(|m| pcs.contains(&m.rem_euclid(12))).collect())
    }

    /// `scale_notes(lo, hi, chord=None)`: the key's scale from `lo` to `hi`; over a chord with a tone the scale
    /// lacks, the scale step a semitone below that tone is raised to it (D minor over A: C becomes C#).
    pub fn scale_notes(&self, lo: i32, hi: i32, chord: Option<&str>) -> Result<Vec<i32>, HarmonyError> {
        let mut pcs: Vec<i32> = self.scale.iter().map(|s| (self.key_pc + s).rem_euclid(12)).collect();
        if let Some(chord) = chord {
            for &c in self.chord_pcs(chord)? {
                let below = (c - 1).rem_euclid(12);
                if !pcs.contains(&c) && pcs.contains(&below) {
                    pcs = pcs.iter().map(|&p| if p == below { c } else { p }).collect();
                }
            }
        }
        Ok((lo..=hi).filter(|m| pcs.contains(&m.rem_euclid(12))).collect())
    }

    /// `root_above(chord, lo)`: the chord's root at or above midi `lo` (the lowest such note).
    pub fn root_above(&self, chord: &str, lo: i32) -> Result<i32, HarmonyError> {
        let r = parse_chord(chord)?.root;
        Ok(lo + (r - lo).rem_euclid(12))
    }

    /// `loop_index(t, per_bar)`: the slot inside the loop at `t`, with `per_bar` slots a bar, snapped to the nearest
    /// slot (round half to even), counted from bar `loop_start`. `per_bar` must be at least 1.
    pub fn loop_index(&self, t: f64, per_bar: usize) -> usize {
        assert!(per_bar >= 1, "loop_index needs at least one slot a bar");
        let n = (per_bar * self.loop_.len()) as i64;
        let slot = py::round_half_even(t / (self.bar / per_bar as f64)) as i64;
        py::imod_py(slot - (per_bar * self.loop_start) as i64, n) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v<const N: usize>(names: [&str; N]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn loop8() -> Vec<String> {
        v(["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"])
    }

    /// flute_vs_trombone's harmony: E harmonic minor, CHORDS = ['Em', 'B7'] + LOOP * 4, the LOOP given.
    fn flute_vs_trombone() -> Harmony {
        let g = Grid::new(132.0);
        let chords = chords_for(&v(["Em", "B7"]), &loop8(), &[], g.n_bars);
        Harmony::new(&g, "E", scale("harmonic minor").unwrap(), chords, Some(loop8()), 2).unwrap()
    }

    #[test]
    fn pitch_classes() {
        assert_eq!(pitch_class("E"), Ok(4));
        assert_eq!(pitch_class("Bb"), Ok(10));
        assert_eq!(pitch_class("Cb"), Ok(11));
        assert_eq!(pitch_class("B#"), Ok(0));
        assert_eq!(pitch_class("F#"), Ok(6));
        assert_eq!(pitch_class("Ebb"), Ok(2));
        // Python adds nothing for a character that is neither '#' nor 'b'.
        assert_eq!(pitch_class("Em"), Ok(4));
    }

    #[test]
    fn chords_parse_as_python() {
        let b7 = parse_chord("B7").unwrap();
        assert_eq!(b7.root, 11);
        assert_eq!(b7.intervals, &[0, 4, 7, 10]);
        assert_eq!(b7.pcs(), vec![11, 3, 6, 9]);
        let em = parse_chord("Em").unwrap();
        assert_eq!((em.root, em.pcs()), (4, vec![4, 7, 11]));
        let bb = parse_chord("Bb").unwrap();
        assert_eq!((bb.root, bb.pcs()), (10, vec![10, 2, 5]));
        let c = parse_chord("C#m7b5").unwrap();
        assert_eq!((c.root, c.pcs()), (1, vec![1, 4, 7, 11]));
        let e = parse_chord("Ebmaj7").unwrap();
        assert_eq!((e.root, e.pcs()), (3, vec![3, 7, 10, 2]));
    }

    #[test]
    fn bad_names_are_errors_naming_the_text() {
        assert_eq!(pitch_class(""), Err(HarmonyError::BadNote(String::new())));
        assert_eq!(pitch_class("H"), Err(HarmonyError::BadNote("H".into())));
        assert_eq!(parse_chord("Exm"), Err(HarmonyError::BadChord("Exm".into())));
        assert_eq!(parse_chord("Q7"), Err(HarmonyError::BadChord("Q7".into())));
        assert_eq!(parse_chord(""), Err(HarmonyError::BadChord(String::new())));
        assert_eq!(parse_chord("日"), Err(HarmonyError::BadChord("日".into())));
        assert_eq!(scale("blues"), Err(HarmonyError::UnknownScale("blues".into())));
        let e = parse_chord("Exm").unwrap_err().to_string();
        assert!(e.contains("Exm"), "{e}");
        let g = Grid::new(132.0);
        let short = Harmony::new(&g, "E", scale("minor").unwrap(), loop8(), None, 2);
        assert_eq!(short, Err(HarmonyError::ChordCount { got: 8, want: 34 }));
        let chords = chords_for(&v(["Em"]), &v(["Em", "Zz"]), &[], 34);
        assert_eq!(
            Harmony::new(&g, "E", scale("minor").unwrap(), chords, None, 2),
            Err(HarmonyError::BadChord("Zz".into()))
        );
        let chords = chords_for(&v(["Em"]), &loop8(), &[], 34);
        assert_eq!(
            Harmony::new(&g, "X", scale("minor").unwrap(), chords, None, 2),
            Err(HarmonyError::BadNote("X".into()))
        );
        let h = flute_vs_trombone();
        assert_eq!(h.chord_tones(40, 64, "F"), Err(HarmonyError::UnknownChord("F".into())));
        assert_eq!(h.root_above("??", 40), Err(HarmonyError::BadChord("??".into())));
    }

    #[test]
    fn scales_in_python_order() {
        assert_eq!(scale("major"), Ok(&[0, 2, 4, 5, 7, 9, 11][..]));
        assert_eq!(scale("harmonic minor"), Ok(&[0, 2, 3, 5, 7, 8, 11][..]));
        assert_eq!(scale("lydian"), Ok(&[0, 2, 4, 6, 7, 9, 11][..]));
        assert_eq!(SCALES.len(), 8);
        assert_eq!(QUALITY.len(), 13);
    }

    #[test]
    fn chords_for_any_bar_count() {
        assert_eq!(chords_for(&v(["Em", "B7"]), &loop8(), &[], 34).len(), 34);
        assert_eq!(chords_for(&v(["Em", "B7"]), &loop8(), &[], 30).len(), 30); // Review Focus 5: any bar count
        let mut want = v(["Em", "B7"]);
        for _ in 0..4 {
            want.extend(loop8());
        }
        assert_eq!(chords_for(&v(["Em", "B7"]), &loop8(), &[], 34), want);
        assert_eq!(chords_for(&v(["Em", "B7"]), &loop8(), &[], 1), v(["Em"]));
        assert_eq!(chords_for(&v(["Em", "B7"]), &loop8(), &[], 0), Vec::<String>::new());
        assert_eq!(chords_for(&[], &v(["A", "D"]), &[], 3), v(["A", "D", "A"]));
        // no loop: the intro cycles; nothing at all: an empty list (never a panic)
        assert_eq!(chords_for(&v(["A", "D"]), &[], &[], 3), v(["A", "D", "A"]));
        assert_eq!(chords_for(&[], &[], &[], 3), Vec::<String>::new());
        for bpm in 60..=200 {
            let g = Grid::new(f64::from(bpm));
            let chords = chords_for(&v(["Em", "B7"]), &loop8(), &[], g.n_bars);
            assert_eq!(chords.len(), g.n_bars, "bpm {bpm}");
            assert!(
                Harmony::new(&g, "E", scale("minor").unwrap(), chords, Some(loop8()), 2).is_ok(),
                "bpm {bpm}"
            );
        }
    }

    /// The kit's configs are `[intro 2] + LOOP * n + tail`; the rematch (31 bars, an 8-chord loop and 5 bars
    /// after the third repeat) and `examples/trombone_vs_horn` (31 bars, a 4-chord loop and one bar). The 17 kit
    /// configs themselves are checked against the export in `bf-content/tests/theme_big_top.rs`.
    #[test]
    fn chords_for_with_a_tail() {
        let rematch = v(["Dm", "Bb", "Gm", "A", "Dm", "Bb", "Eb", "A"]);
        let tail = v(["Dm", "Bb", "Gm", "A", "A"]);
        let mut want = v(["Dm", "A"]);
        for _ in 0..3 {
            want.extend(rematch.clone());
        }
        want.extend(tail.clone());
        assert_eq!(chords_for(&v(["Dm", "A"]), &rematch, &tail, 31), want);
        let four = v(["Dm", "Bb", "Gm", "A"]);
        let mut want = v(["Dm", "A"]);
        for _ in 0..7 {
            want.extend(four.clone());
        }
        want.push("A".into());
        assert_eq!(chords_for(&v(["Dm", "A"]), &four, &v(["A"]), 31), want);
        // At another bar count the loop's last repeat stops part-way and the tail still ends the song.
        let got = chords_for(&v(["Dm", "A"]), &rematch, &tail, 34);
        assert_eq!(got.len(), 34);
        assert_eq!(&got[29..], &tail[..]);
        assert_eq!(&got[26..29], &rematch[..3]);
        // Too few bars: the intro, then the tail cut where the bars end; the loop gets none.
        assert_eq!(
            chords_for(&v(["Dm", "A"]), &rematch, &tail, 5),
            v(["Dm", "A", "Dm", "Bb", "Gm"])
        );
        assert_eq!(
            chords_for(&v(["Dm", "A"]), &rematch, &tail, 7),
            v(["Dm", "A", "Dm", "Bb", "Gm", "A", "A"])
        );
        assert_eq!(chords_for(&v(["Dm", "A"]), &rematch, &tail, 8)[2..3], v(["Dm"])[..]);
        assert_eq!(chords_for(&v(["Dm", "A"]), &rematch, &tail, 1), v(["Dm"]));
        // Only a tail: it is all there is.
        assert_eq!(chords_for(&[], &[], &tail, 31), tail);
        // An empty tail is a format 1 fight's chords exactly.
        for n in 0..40 {
            assert_eq!(chords_for(&v(["Em", "B7"]), &loop8(), &[], n).len(), n, "{n} bars");
        }
    }

    #[test]
    fn harmony_of_flute_vs_trombone() {
        let h = flute_vs_trombone();
        assert_eq!(h.key_pc, 4);
        assert_eq!(h.key_root, 40);
        assert_eq!(h.scale, vec![0, 2, 3, 5, 7, 8, 11]);
        assert_eq!(h.loop_, loop8());
        assert_eq!(h.loop_start, 2);
        assert_eq!(h.chords.len(), 34);
    }

    /// As Python printed them for flute_vs_trombone: chord_at, loop_index(t, 8), loop_index(t, 4).
    #[test]
    fn chord_at_and_loop_index_match_python() {
        let h = flute_vs_trombone();
        let cases = [
            (0.0, "Em", 48, 24),
            (0.3, "Em", 49, 25),
            (1.7, "Em", 56, 28),
            (3.588, "B7", 0, 0),
            (12.34, "C", 39, 20),
            (60.99, "B7", 0, 0),
        ];
        for (t, chord, i8, i4) in cases {
            assert_eq!(h.chord_at(t), chord, "t {t}");
            assert_eq!(h.loop_index(t, 8), i8, "t {t}");
            assert_eq!(h.loop_index(t, 4), i4, "t {t}");
        }
        assert_eq!(h.chord_at(-3.0), "Em");
        assert_eq!(h.chord_at(f64::NAN), "Em");
        assert_eq!(h.chord_at(62.0), "Em"); // bar 34 wraps to bar 0
    }

    #[test]
    fn notes_match_python() {
        let h = flute_vs_trombone();
        assert_eq!(
            h.chord_tones(40, 64, "B7").unwrap(),
            vec![42, 45, 47, 51, 54, 57, 59, 63]
        );
        let e_harm = vec![60, 63, 64, 66, 67, 69, 71, 72];
        assert_eq!(h.scale_notes(60, 72, None).unwrap(), e_harm);
        assert_eq!(h.scale_notes(60, 72, Some("B7")).unwrap(), e_harm);
        assert_eq!(h.scale_notes(60, 72, Some("Am")).unwrap(), e_harm);
        assert_eq!(h.root_above("B7", 40).unwrap(), 47);
        assert_eq!(h.root_above("Em", 40).unwrap(), 40);
        assert_eq!(h.root_above("C", 41).unwrap(), 48);
        // D minor over A: C is raised to C#, the leading tone (Python gave [61, 62, 64, 65, 67, 69, 70]).
        let g = Grid::new(132.0);
        let d = Harmony::new(
            &g,
            "D",
            scale("minor").unwrap(),
            chords_for(&v(["Dm"]), &v(["A"]), &[], 34),
            None,
            2,
        )
        .unwrap();
        assert_eq!(
            d.scale_notes(60, 72, Some("A")).unwrap(),
            vec![61, 62, 64, 65, 67, 69, 70]
        );
        assert_eq!(d.loop_, v(["A"; 8]));
    }
}
