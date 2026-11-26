//! The lines and solos of the timeline: `default_line`, `default_solo` and the plain half of `line_at` of
//! `projects/ballfight/kit/episode.py` (`_helpers`), as methods of [`Harmony`].
//!
//! A fighter plays the line the fight file writes for it over the chord loop (`LINES`: 8 notes a bar are 8ths,
//! 4 are beats) and the solo it writes (`SOLO`); for a fighter it writes none for, the kit sketches one from the
//! chords' tones in the fighter's register. Python looks the register up in the roster by name; here the caller
//! passes it (`FighterCfg::register`), since the roster is not this crate's.
//!
//! `line_at` also reads the windows a fighter registered with `music.line_override` (#53): that half is the
//! sound's (`bf_sound::MusicCtx::line_at`), which calls [`Harmony::line_note`] for the rest.

use crate::harmony::{Harmony, HarmonyError};
use crate::py;

/// `default_line`'s default `per_bar`: one note per 8th.
pub const DEFAULT_LINE_PER_BAR: usize = 8;

impl Harmony {
    /// `default_line(name, per_bar=8)` for a fighter whose register is `(lo, hi)`: a line for a fighter the
    /// config wrote none for: each loop bar's chord tones up and back down in the upper two thirds of its
    /// register, `per_bar` notes a bar (a sketch to replace with a real line).
    ///
    /// # Errors
    /// Where Python raises: a chord of the loop that is not one of the episode's chords (a loop longer than the
    /// bars that play it), or a register that holds no tone of a chord.
    pub fn default_line(&self, register: (i32, i32), per_bar: usize) -> Result<Vec<i32>, HarmonyError> {
        let (lo, hi) = register;
        let from = lo + py::ifloor_div(i64::from(hi) - i64::from(lo), 3) as i32;
        let mut out = Vec::with_capacity(per_bar * self.loop_.len());
        for ch in &self.loop_ {
            // chord_tones(lo + (hi - lo) // 3, hi, ch)[:4] or chord_tones(lo, hi, ch)[:4]
            let mut tones = self.chord_tones(from, hi, ch)?;
            if tones.is_empty() {
                tones = self.chord_tones(lo, hi, ch)?;
            }
            tones.truncate(4);
            if tones.is_empty() {
                return Err(HarmonyError::NoTones {
                    chord: ch.clone(),
                    lo,
                    hi,
                });
            }
            // shape = tones + tones[-2:0:-1]: up, then back down without the top and the bottom.
            let mut shape = tones.clone();
            shape.extend(tones.iter().skip(1).rev().skip(1));
            out.extend((0..per_bar).map(|i| shape[i % shape.len()]));
        }
        Ok(out)
    }

    /// `default_solo(name)` for a fighter whose register is `(lo, hi)`: a 2-bar victory motif on the first bar's
    /// chord in the upper three quarters of the register (a sketch to replace), as `(beat, midi note, beats)`.
    ///
    /// # Errors
    /// Where Python raises: the register holds no tone of that chord.
    pub fn default_solo(&self, register: (i32, i32)) -> Result<Vec<(f64, i32, f64)>, HarmonyError> {
        let (lo, hi) = register;
        // CHORDS[0]: a Harmony always has a chord for bar 0 when its grid has a bar.
        let ch = self
            .chords
            .first()
            .ok_or(HarmonyError::ChordCount { got: 0, want: 1 })?;
        let from = lo + py::ifloor_div(i64::from(hi) - i64::from(lo), 4) as i32;
        let tones = self.chord_tones(from, hi, ch)?;
        if tones.is_empty() {
            return Err(HarmonyError::NoTones {
                chord: ch.clone(),
                lo: from,
                hi,
            });
        }
        // up = tones[:4] if len(tones) >= 4 else (tones * 4)[:4]
        let up: Vec<i32> = tones.iter().cycle().take(4).copied().collect();
        Ok(vec![
            (0.0, up[0], 0.5),
            (0.5, up[1], 0.5),
            (1.0, up[2], 0.5),
            (1.5, up[3], 0.5),
            (2.0, up[2], 0.5),
            (2.5, up[1], 0.5),
            (3.0, up[3], 1.0),
            (4.0, up[2], 0.5),
            (4.5, up[1], 0.5),
            (5.0, up[0], 1.0),
            (6.0, up[0], 2.0),
        ])
    }

    /// The plain half of `line_at(name, t)`: the note of `line` at `t`. A line runs over the chord loop with
    /// `len(line) // len(LOOP)` notes a bar (8 are 8ths, 4 are beats), snapped to the nearest slot:
    /// `line[loop_index(t, per_bar)]`.
    ///
    /// # Errors
    /// Where Python raises: the line has fewer notes than the loop has bars.
    pub fn line_note<T: Copy>(&self, line: &[T], t: f64) -> Result<T, HarmonyError> {
        let per_bar = line.len() / self.loop_.len();
        if per_bar == 0 {
            return Err(HarmonyError::ShortLine {
                notes: line.len(),
                bars: self.loop_.len(),
            });
        }
        Ok(line[self.loop_index(t, per_bar)])
    }

    /// `roll_rows(name)` of `kit/story.py` (41 to 75): a roll fighter's rows, one per 8th of the loop. Its line
    /// (`line`: the fight file's, 8 notes a bar, or 4 = beats with an 8th's rest after each; a rest is `None`;
    /// without one the kit's sketch line for the fighter's `register`) with the harmony a denser row adds, in order:
    /// the melody, its octave below, then the chord's tones under it (the nearest first, each at least a minor third
    /// from every note already in the row), at most 4 notes, all inside the roll's range `reg`
    /// (`weapon.reg`, else the register). A rest is an empty row. The engine fires the first holes of each row.
    ///
    /// # Errors
    /// Where Python raises: a line that is not 8 or 4 notes a bar ([`RollRowsError::PerBar`], Python's `assert`), a
    /// loop chord that is not one of the episode's chords, a register that holds no tone of a chord (the sketch
    /// line).
    pub fn roll_rows(
        &self,
        line: Option<&[Option<i32>]>,
        reg: (i32, i32),
        register: (i32, i32),
    ) -> Result<Vec<Vec<i32>>, RollRowsError> {
        let (lo, hi) = reg;
        // line = list(T.LINES[name]) if name in T.LINES else T.default_line(name)
        let mut line: Vec<Option<i32>> = match line {
            Some(line) => line.to_vec(),
            None => self
                .default_line(register, DEFAULT_LINE_PER_BAR)?
                .into_iter()
                .map(Some)
                .collect(),
        };
        let bars = self.loop_.len();
        let mut per_bar = line.len() / bars;
        if per_bar == 4 {
            // line = [x for m in line for x in (m, None)]
            line = line.into_iter().flat_map(|m| [m, None]).collect();
            per_bar = 8;
        }
        if per_bar != 8 {
            return Err(RollRowsError::PerBar {
                notes: line.len(),
                bars,
            });
        }
        let notes = line.len();
        let mut out = Vec::with_capacity(notes);
        for (i, m) in line.into_iter().enumerate() {
            let Some(mut m) = m else {
                out.push(Vec::new());
                continue;
            };
            while m > hi {
                m -= 12;
            }
            while m < lo {
                m += 12;
            }
            // T.LOOP[i // 8]: past the loop's end (spare notes of a line) Python raises IndexError
            let ch = self.loop_.get(i / 8).ok_or(RollRowsError::PerBar { notes, bars })?;
            let mut row = vec![m];
            if m - 12 >= lo {
                row.push(m - 12);
            }
            // sorted((c for c in chord_tones(lo, m - 3, ch) if c not in row and c != m - 12), reverse=True)
            let tones = self.chord_tones(lo, m - 3, ch)?;
            for c in tones.into_iter().rev().filter(|&c| c != m - 12) {
                if row.len() >= 4 {
                    break;
                }
                if row.iter().all(|r| (c - r).abs() >= 3) {
                    row.push(c);
                }
            }
            out.push(row);
        }
        Ok(out)
    }
}

/// Why a roll fighter's rows cannot be made from its line ([`Harmony::roll_rows`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RollRowsError {
    /// The line is not 8 (or 4) notes a bar over the loop: the roll fires on the 8th grid.
    PerBar {
        /// How many notes the line has.
        notes: usize,
        /// How many chords the loop has.
        bars: usize,
    },
    /// The harmony refused: an unknown chord, or a register with no tone of a chord.
    Harmony(HarmonyError),
}

impl From<HarmonyError> for RollRowsError {
    fn from(e: HarmonyError) -> RollRowsError {
        RollRowsError::Harmony(e)
    }
}

impl std::fmt::Display for RollRowsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RollRowsError::PerBar { notes, bars } => write!(
                f,
                "{notes} notes over the {bars}-chord loop: a roll's line has 8 notes a bar (or 4), so {} or {}",
                8 * bars,
                4 * bars
            ),
            RollRowsError::Harmony(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RollRowsError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;
    use crate::harmony::{chords_for, scale};

    fn v<const N: usize>(names: [&str; N]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn loop8() -> Vec<String> {
        v(["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"])
    }

    /// flute_vs_trombone's harmony.
    fn episode() -> Harmony {
        let g = Grid::new(132.0);
        let chords = chords_for(&v(["Em", "B7"]), &loop8(), &[], g.n_bars);
        Harmony::new(&g, "E", scale("harmonic minor").unwrap(), chords, Some(loop8()), 2).unwrap()
    }

    /// The roster's registers of the two fighters (`fighters.FIGHTERS[name]['register']`).
    const FLUTE: (i32, i32) = (72, 93);
    const TROMBONE: (i32, i32) = (40, 65);

    /// The episode's FLUTE line (its fight file's, 8 notes a bar).
    #[rustfmt::skip]
    const FLUTE_LINE: [i32; 64] = [
        76, 79, 83, 79, 76, 75, 76, 71,   81, 84, 83, 81, 79, 76, 77, 76,
        75, 78, 81, 78, 83, 81, 78, 75,   76, 71, 76, 79, 83, 84, 83, 79,
        84, 83, 79, 76, 72, 76, 79, 84,   81, 76, 72, 76, 81, 84, 88, 84,
        83, 87, 86, 87, 90, 87, 83, 81,   78, 75, 71, 75, 78, 81, 83, 75,
    ];

    /// As the kit printed `T.default_line(name)` and `T.default_line(name, 4)` for the episode.
    #[test]
    fn default_lines_match_python() {
        let h = episode();
        #[rustfmt::skip]
        let flute = [
            79, 83, 88, 91, 88, 83, 79, 83,   81, 84, 88, 93, 88, 84, 81, 84,
            81, 83, 87, 90, 87, 83, 81, 83,   79, 83, 88, 91, 88, 83, 79, 83,
            79, 84, 88, 91, 88, 84, 79, 84,   81, 84, 88, 93, 88, 84, 81, 84,
            81, 83, 87, 90, 87, 83, 81, 83,   81, 83, 87, 90, 87, 83, 81, 83,
        ];
        #[rustfmt::skip]
        let trombone = [
            52, 55, 59, 64, 59, 55, 52, 55,   48, 52, 57, 60, 57, 52, 48, 52,
            51, 54, 57, 59, 57, 54, 51, 54,   52, 55, 59, 64, 59, 55, 52, 55,
            48, 52, 55, 60, 55, 52, 48, 52,   48, 52, 57, 60, 57, 52, 48, 52,
            51, 54, 57, 59, 57, 54, 51, 54,   51, 54, 57, 59, 57, 54, 51, 54,
        ];
        assert_eq!(h.default_line(FLUTE, DEFAULT_LINE_PER_BAR).unwrap(), flute);
        assert_eq!(h.default_line(TROMBONE, DEFAULT_LINE_PER_BAR).unwrap(), trombone);
        #[rustfmt::skip]
        let flute4 = [
            79, 83, 88, 91,   81, 84, 88, 93,   81, 83, 87, 90,   79, 83, 88, 91,
            79, 84, 88, 91,   81, 84, 88, 93,   81, 83, 87, 90,   81, 83, 87, 90,
        ];
        #[rustfmt::skip]
        let trombone4 = [
            52, 55, 59, 64,   48, 52, 57, 60,   51, 54, 57, 59,   52, 55, 59, 64,
            48, 52, 55, 60,   48, 52, 57, 60,   51, 54, 57, 59,   51, 54, 57, 59,
        ];
        assert_eq!(h.default_line(FLUTE, 4).unwrap(), flute4);
        assert_eq!(h.default_line(TROMBONE, 4).unwrap(), trombone4);
        assert!(h.default_line(FLUTE, 0).unwrap().is_empty());
    }

    #[test]
    fn a_default_line_turns_round_whatever_tones_the_register_holds() {
        let h = episode();
        // Three tones: up and back through the middle one. Em in 64..=71 is 64, 67, 71; the upper two thirds
        // (66 on) hold only 67 and 71, so the shape is those two.
        assert_eq!(
            h.default_line((64, 71), 8).unwrap()[..8],
            [67, 71, 67, 71, 67, 71, 67, 71]
        );
        // One tone: it repeats.
        assert_eq!(h.default_line((64, 66), 4).unwrap()[..4], [64, 64, 64, 64]);
        // The upper two thirds hold no tone of Am (67, 68): the whole register is taken (64).
        assert_eq!(h.default_line((64, 68), 2).unwrap()[2..4], [64, 64]);
        // No tone of a chord at all: Python divides by zero.
        assert_eq!(
            h.default_line((65, 66), 8),
            Err(HarmonyError::NoTones {
                chord: "Em".into(),
                lo: 65,
                hi: 66
            })
        );
        assert_eq!(
            HarmonyError::NoTones {
                chord: "Em".into(),
                lo: 65,
                hi: 66
            }
            .to_string(),
            "no note from 65 to 66 is a tone of Em"
        );
    }

    /// As the kit printed `T.default_solo(name)` for the episode.
    #[test]
    fn default_solos_match_python() {
        let h = episode();
        let solo = |up: [i32; 4]| {
            vec![
                (0.0, up[0], 0.5),
                (0.5, up[1], 0.5),
                (1.0, up[2], 0.5),
                (1.5, up[3], 0.5),
                (2.0, up[2], 0.5),
                (2.5, up[1], 0.5),
                (3.0, up[3], 1.0),
                (4.0, up[2], 0.5),
                (4.5, up[1], 0.5),
                (5.0, up[0], 1.0),
                (6.0, up[0], 2.0),
            ]
        };
        assert_eq!(h.default_solo(FLUTE).unwrap(), solo([79, 83, 88, 91]));
        assert_eq!(h.default_solo(TROMBONE).unwrap(), solo([47, 52, 55, 59]));
        // Fewer than four tones repeat: (tones * 4)[:4].
        assert_eq!(h.default_solo((60, 71)).unwrap(), solo([64, 67, 71, 64]));
        assert_eq!(h.default_solo((60, 64)).unwrap(), solo([64, 64, 64, 64]));
        assert_eq!(
            h.default_solo((60, 63)),
            Err(HarmonyError::NoTones {
                chord: "Em".into(),
                lo: 60,
                hi: 63
            })
        );
    }

    /// As the kit printed `T.line_at(name, t)` for the episode: the FLUTE's own line, the TROMBONE's default
    /// line, and a line of beats (`[40 + (i * 5) % 13 for i in range(32)]`).
    #[test]
    fn line_notes_match_python() {
        let h = episode();
        let ts = [
            0.0, 0.3, 1.7, 3.588, 3.6, 3.82, 4.05, 12.34, 17.94, 25.0, 31.4, 46.65, 50.02, 57.4, 60.99,
        ];
        let flute = [83, 87, 78, 76, 76, 79, 83, 84, 76, 79, 78, 76, 76, 83, 76];
        let trombone = [51, 54, 51, 52, 52, 55, 59, 52, 52, 55, 57, 52, 52, 51, 52];
        let beats = [43, 48, 50, 40, 40, 45, 45, 49, 40, 42, 47, 40, 41, 43, 40];
        let default = h.default_line(TROMBONE, DEFAULT_LINE_PER_BAR).unwrap();
        let line4: Vec<i32> = (0..32).map(|i| 40 + (i * 5) % 13).collect();
        for (i, t) in ts.into_iter().enumerate() {
            assert_eq!(h.line_note(&FLUTE_LINE, t), Ok(flute[i]), "FLUTE at {t}");
            assert_eq!(h.line_note(&default, t), Ok(trombone[i]), "TROMBONE at {t}");
            assert_eq!(h.line_note(&line4, t), Ok(beats[i]), "beats at {t}");
        }
        // A line may hold rests, or anything else that can be copied.
        let rests: Vec<Option<i32>> = (0..8).map(|i| (i != 1).then_some(60 + i)).collect();
        assert_eq!(h.line_note(&rests, 2.0 * 1.794 + 0.1), Ok(Some(60)));
        assert_eq!(h.line_note(&rests, 3.0 * 1.795), Ok(None));
        // Fewer notes than bars: Python divides by zero.
        assert_eq!(
            h.line_note(&FLUTE_LINE[..7], 4.0),
            Err(HarmonyError::ShortLine { notes: 7, bars: 8 })
        );
        assert_eq!(
            h.line_note::<i32>(&[], 4.0),
            Err(HarmonyError::ShortLine { notes: 0, bars: 8 })
        );
    }
}
