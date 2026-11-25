//! A theme's own band patterns and band balance (sound upgrade, stage 2: C4 and M6), for its Composed arrangement.
//!
//! Without a [`Groove`] every layer plays the shared [`pattern`] of its kind. A groove gives some of a theme's
//! layers their own rhythm in the theme's style (a [`Part`] of [`Line`]s), a swing where the style swings, small
//! seeded differences in timing and strength so the band does not sound stamped, a level for each of its buses
//! (M6: a band that loses little on a phone) and an octave move for a layer that needs one.
//!
//! A line is written as a step string, one character a sixteenth, 16 for a one-bar pattern or 32 for a two-bar
//! one (the second half plays in odd bars):
//!
//! | char | meaning |
//! |---|---|
//! | `X` | an accent (velocity 1.0) |
//! | `x` | a plain note (0.8) |
//! | `o` | a soft note (0.6) |
//! | `g` | a ghost note (0.35) |
//! | `_` | holds the note before one more sixteenth |
//! | `.` | a rest |
//!
//! A fill line is 4, 8 or 16 steps long and replaces the last beat, the last two beats or the bar.
//!
//! The groove is only ever handed to a Composed bed ([`LayeredBed::with_groove`]): Classic keeps the shared
//! patterns, sample for sample.
//!
//! [`pattern`]: crate::music::layers::pattern
//! [`LayeredBed::with_groove`]: crate::music::layers::LayeredBed::with_groove

use crate::music::layers::{Hit, LayerKind, MAX_DENSITY, pattern};

/// What a line plays: a drum, or a tone of the bar's chord as the layer voices it in its range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// A drum number (`KICK`, `SNARE`, ... of [`crate::music::layers`]); for drum and percussion layers.
    Drum(i32),
    /// The chord's root (the lowest one in the layer's range; for a bass, the first of its tones).
    Root,
    /// The chord's third (the nearest above the root), or the root when it has none.
    Third,
    /// The chord's fifth, or the root.
    Fifth,
    /// The chord's seventh or sixth, or the root's octave.
    Seventh,
    /// The root an octave up (or down, when the range has no room above), or the root.
    Octave,
    /// The highest chord tone in the range.
    Top,
    /// The `n`-th chord tone in the range, lowest first, counted round.
    Tone(u8),
    /// Every chord tone (the lowest four), struck together.
    Chord,
    /// The chord without its lowest tone, struck together.
    Upper,
}

impl Play {
    fn drum(self) -> bool {
        matches!(self, Play::Drum(_))
    }
}

/// One line of a part: what it plays and when, at densities `from` to `to`, in a bar with probability `p`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub from: u8,
    pub to: u8,
    pub play: Play,
    pub steps: &'static str,
    pub p: f64,
}

/// A line at every density from `from` up.
pub const fn line(from: u8, play: Play, steps: &'static str) -> Line {
    Line {
        from,
        to: MAX_DENSITY,
        play,
        steps,
        p: 1.0,
    }
}

/// A line at density `d` only.
pub const fn only(d: u8, play: Play, steps: &'static str) -> Line {
    Line {
        from: d,
        to: d,
        play,
        steps,
        p: 1.0,
    }
}

/// A line from density 1 up to `d`.
pub const fn upto(d: u8, play: Play, steps: &'static str) -> Line {
    Line {
        from: 1,
        to: d,
        play,
        steps,
        p: 1.0,
    }
}

impl Line {
    /// The line, played in a bar only with probability `p` (drawn from the music seed, bar by bar).
    pub const fn maybe(self, p: f64) -> Line {
        Line { p, ..self }
    }
}

/// A layer's own rhythm: its lines, and the fill it plays in a bar with a lead change (empty: the shared fill of a
/// drum or percussion layer, none for the others).
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub layer: &'static str,
    pub lines: &'static [Line],
    pub fill: &'static [Line],
}

/// A theme's own band: see the module notes.
#[derive(Clone, Copy, Debug)]
pub struct Groove {
    /// Where the off-beat eighth falls in its beat: 0.5 is straight, 0.67 a triplet swing.
    pub swing: f64,
    /// Swing the sixteenths (in each eighth) in place of the eighths.
    pub swing16: bool,
    /// How far a note may move from its sixteenth, in ms either way (seeded per note).
    pub push_ms: f64,
    /// How much a note's strength may move, as a part of it either way (seeded per note).
    pub vel_spread: f64,
    /// The layers with their own rhythm; the others play the shared pattern.
    pub parts: &'static [Part],
    /// M6: each named bus's notes in the layered bed louder or softer by this many dB.
    pub mix: &'static [(&'static str, f64)],
    /// A layer's range moved by this many semitones.
    pub lift: &'static [(&'static str, i32)],
    /// M6: the band's buses whose lows a phone loses (a deep bass, a sub drum) and that get the overtones of M2
    /// (`polish::lows::treat`) in the Composed master.
    pub lows: &'static [&'static str],
}

/// A straight groove with the small human differences and nothing else: start a theme's groove from it.
pub const STRAIGHT: Groove = Groove {
    swing: 0.5,
    swing16: false,
    push_ms: 4.0,
    vel_spread: 0.08,
    parts: &[],
    mix: &[],
    lift: &[],
    lows: &[],
};

/// The most a groove may move one bus against another, in dB (M6).
pub const MAX_SPREAD: f64 = 12.0;

/// The velocity of a step character, or `None` for a rest or a hold.
fn vel(c: u8) -> Option<f64> {
    match c {
        b'X' => Some(1.0),
        b'x' => Some(0.8),
        b'o' => Some(0.6),
        b'g' => Some(0.35),
        _ => None,
    }
}

/// The first of `tones` whose interval above `root` is one of `ivs` (mod 12), the nearest above the root first.
fn by_interval(tones: &[i32], root: i32, ivs: &[i32]) -> Option<i32> {
    let mut hits: Vec<i32> = tones
        .iter()
        .copied()
        .filter(|&m| ivs.contains(&(m - root).rem_euclid(12)))
        .collect();
    hits.sort_by_key(|&m| if m > root { m - root } else { 1000 + root - m });
    hits.first().copied()
}

/// The notes `play` strikes over `tones` (lowest first; a bass's root first).
pub fn notes_of(play: Play, tones: &[i32]) -> Vec<i32> {
    let Some(&root) = tones.first() else {
        return Vec::new();
    };
    let mut sorted = tones.to_vec();
    sorted.sort_unstable();
    let one = |m: Option<i32>| vec![m.unwrap_or(root)];
    match play {
        Play::Drum(n) => vec![n],
        Play::Root => vec![root],
        Play::Third => one(by_interval(tones, root, &[3, 4])),
        Play::Fifth => one(by_interval(tones, root, &[6, 7, 8])),
        Play::Seventh => {
            let octave = tones.iter().copied().find(|&m| m == root + 12 || m == root - 12);
            one(by_interval(tones, root, &[9, 10, 11]).or(octave))
        }
        Play::Octave => one(tones
            .iter()
            .copied()
            .find(|&m| m == root + 12)
            .or_else(|| tones.iter().copied().find(|&m| m == root - 12))),
        Play::Top => vec![*sorted.last().unwrap_or(&root)],
        Play::Tone(n) => vec![sorted[n as usize % sorted.len()]],
        Play::Chord => tones.iter().copied().take(4).collect(),
        Play::Upper => tones.iter().copied().take(4).skip(1).collect(),
    }
}

/// The step string of `steps` for bar `bar`: the whole of a 16-step line, the half of a 32-step one.
fn half(steps: &str, bar: usize) -> &[u8] {
    let b = steps.as_bytes();
    if b.len() == 32 {
        &b[(bar % 2) * 16..(bar % 2) * 16 + 16]
    } else {
        b
    }
}

/// The hits of the step string `steps` playing `play` over `tones`, from sixteenth `at`.
fn hits_of(steps: &[u8], play: Play, tones: &[i32], at: f64, out: &mut Vec<Hit>) {
    for (i, &c) in steps.iter().enumerate() {
        let Some(v) = vel(c) else { continue };
        let len = 1 + steps[i + 1..].iter().take_while(|&&c| c == b'_').count();
        for midi in notes_of(play, tones) {
            out.push(Hit {
                s16: at + i as f64,
                midi,
                len: len as f64,
                vel: v,
            });
        }
    }
}

impl Part {
    /// The bar's hits at density `d` (1 to 3) over the chord tones `tones` (empty for drums), with the fill when
    /// `fill`. `r(salt)` draws the bar's variation.
    pub fn hits(
        &self,
        kind: LayerKind,
        d: u8,
        tones: &[i32],
        fill: bool,
        bar: usize,
        r: &dyn Fn(u64) -> f64,
    ) -> Vec<Hit> {
        let mut out = Vec::new();
        let d = d.min(MAX_DENSITY);
        if d == 0 {
            return out;
        }
        for (n, l) in self.lines.iter().enumerate() {
            if l.from <= d && d <= l.to && (l.p >= 1.0 || r(100 + n as u64) < l.p) {
                hits_of(half(l.steps, bar), l.play, tones, 0.0, &mut out);
            }
        }
        if fill {
            if self.fill.is_empty() {
                if matches!(kind, LayerKind::Drums | LayerKind::Perc) {
                    out.retain(|h| h.s16 < 12.0);
                    out.extend(pattern(kind, d, tones, true, r).into_iter().filter(|h| h.s16 >= 12.0));
                }
            } else {
                let n = self.fill.iter().map(|l| l.steps.len()).max().unwrap_or(0).min(16);
                let at = (16 - n) as f64;
                out.retain(|h| h.s16 < at);
                for l in self.fill.iter().filter(|l| l.from <= d && d <= l.to) {
                    hits_of(l.steps.as_bytes(), l.play, tones, at, &mut out);
                }
            }
        }
        out
    }
}

impl Groove {
    /// The part of layer `layer`, if it has its own.
    pub fn part(&self, layer: &str) -> Option<&Part> {
        self.parts.iter().find(|p| p.layer == layer)
    }

    /// The gain of the notes on bus `bus` (M6), as a factor.
    pub fn bus_gain(&self, bus: &str) -> f64 {
        self.mix
            .iter()
            .find(|(b, _)| *b == bus)
            .map_or(1.0, |(_, db)| 10f64.powf(db / 20.0))
    }

    /// The move of layer `layer`'s range, in semitones.
    pub fn lift_of(&self, layer: &str) -> i32 {
        self.lift.iter().find(|(l, _)| *l == layer).map_or(0, |(_, s)| *s)
    }

    /// Where a note written on sixteenth `s16` of its bar sounds, in sixteenths, with the swing.
    pub fn swung(&self, s16: f64) -> f64 {
        let r = self.swing.clamp(0.5, 0.75);
        if r == 0.5 {
            return s16;
        }
        // the unit that swings (a beat of four sixteenths, or an eighth of two) and the place in it
        let unit = if self.swing16 { 2.0 } else { 4.0 };
        let start = (s16 / unit).floor() * unit;
        let p = (s16 - start) / unit;
        let q = if p < 0.5 {
            p * 2.0 * r
        } else {
            r + (p - 0.5) * 2.0 * (1.0 - r)
        };
        start + q * unit
    }

    /// Whether the groove is well written for a bed whose layers are `layers` (`(name, kind)`): every part names
    /// one of them, drum lines are on drum and percussion layers and tone lines on the others, every step string
    /// is 16 or 32 steps (a fill 4, 8 or 16) of the step characters, every density is 1 to 3, every probability
    /// is in `(0, 1]`, and the bus levels are at most [`MAX_SPREAD`] apart.
    ///
    /// # Errors
    /// What is wrong, first found first.
    pub fn check(&self, layers: &[(&str, LayerKind)]) -> Result<(), String> {
        for part in self.parts {
            let Some(&(_, kind)) = layers.iter().find(|(n, _)| *n == part.layer) else {
                return Err(format!("a part for `{}`, which the bed lacks", part.layer));
            };
            let drums = matches!(kind, LayerKind::Drums | LayerKind::Perc);
            for (l, is_fill) in part
                .lines
                .iter()
                .map(|l| (l, false))
                .chain(part.fill.iter().map(|l| (l, true)))
            {
                let at = format!("`{}` line {:?}", part.layer, l.steps);
                if l.play.drum() != drums {
                    return Err(format!(
                        "{at}: a {} line on a {kind:?} layer",
                        if drums { "tone" } else { "drum" }
                    ));
                }
                let lens: &[usize] = if is_fill { &[4, 8, 16] } else { &[16, 32] };
                if !lens.contains(&l.steps.len()) {
                    return Err(format!("{at}: {} steps", l.steps.len()));
                }
                if let Some(c) = l.steps.bytes().find(|c| !b"Xxog_.".contains(c)) {
                    return Err(format!("{at}: the step {:?}", c as char));
                }
                if !(1 <= l.from && l.from <= l.to && l.to <= MAX_DENSITY) {
                    return Err(format!("{at}: densities {} to {}", l.from, l.to));
                }
                if !(l.p > 0.0 && l.p <= 1.0) {
                    return Err(format!("{at}: probability {}", l.p));
                }
            }
        }
        for (name, _) in self.lift {
            if !layers.iter().any(|(n, _)| n == name) {
                return Err(format!("a lift for `{name}`, which the bed lacks"));
            }
        }
        let dbs = self.mix.iter().map(|(_, db)| *db).chain([0.0]);
        let (lo, hi) = dbs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| (lo.min(d), hi.max(d)));
        if hi - lo > MAX_SPREAD + 1e-9 {
            return Err(format!("the bus levels are {:.1} dB apart, over {MAX_SPREAD}", hi - lo));
        }
        if !(0.5..=0.75).contains(&self.swing) || self.push_ms < 0.0 || self.push_ms > 15.0 {
            return Err(format!("swing {} and push {} ms", self.swing, self.push_ms));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::layers::{HAT, KICK, SNARE};

    const PART: Part = Part {
        layer: "drums",
        lines: &[
            line(1, Play::Drum(KICK), "X.....x...x....."),
            line(2, Play::Drum(HAT), "x.o.x.o.x.o.x.o_"),
            only(3, Play::Drum(SNARE), "....X.......X..g").maybe(0.5),
        ],
        fill: &[line(1, Play::Drum(SNARE), "xxxX")],
    };

    #[test]
    fn a_line_plays_its_steps_at_its_densities() {
        let r = |_: u64| 0.0;
        let at = |d, fill| {
            PART.hits(LayerKind::Drums, d, &[], fill, 0, &r)
                .iter()
                .map(|h| (h.s16 as i32, h.midi, h.vel, h.len as i32))
                .collect::<Vec<_>>()
        };
        assert_eq!(at(1, false), [(0, KICK, 1.0, 1), (6, KICK, 0.8, 1), (10, KICK, 0.8, 1)]);
        let two = at(2, false);
        assert_eq!(two.len(), 3 + 8);
        // the last hat holds one more sixteenth
        assert!(two.contains(&(14, HAT, 0.6, 2)));
        assert_eq!(at(3, false).len(), 3 + 8 + 3);
        // the fill takes the last beat
        let fill = at(1, true);
        assert_eq!(
            &fill[3..],
            [
                (12, SNARE, 0.8, 1),
                (13, SNARE, 0.8, 1),
                (14, SNARE, 0.8, 1),
                (15, SNARE, 1.0, 1)
            ]
        );
        // a line drawn out of the bar does not play
        let never = |_: u64| 0.99;
        assert_eq!(PART.hits(LayerKind::Drums, 3, &[], false, 0, &never).len(), 3 + 8);
    }

    #[test]
    fn the_tones_are_the_chords() {
        // C7 in a bass's range, root first: C2 E2 G2 Bb2 C3
        let bass = [36, 40, 43, 46, 48];
        assert_eq!(notes_of(Play::Root, &bass), [36]);
        assert_eq!(notes_of(Play::Third, &bass), [40]);
        assert_eq!(notes_of(Play::Fifth, &bass), [43]);
        assert_eq!(notes_of(Play::Seventh, &bass), [46]);
        assert_eq!(notes_of(Play::Octave, &bass), [48]);
        assert_eq!(notes_of(Play::Top, &bass), [48]);
        assert_eq!(notes_of(Play::Chord, &bass), [36, 40, 43, 46]);
        assert_eq!(notes_of(Play::Upper, &bass), [40, 43, 46]);
        // a triad has no seventh: the octave; a range with no root above: the fifth below is no octave
        assert_eq!(notes_of(Play::Seventh, &[48, 52, 55, 60]), [60]);
        assert_eq!(notes_of(Play::Octave, &[48, 43]), [48]);
        assert!(notes_of(Play::Root, &[]).is_empty());
    }

    const TWO_BAR: Part = Part {
        layer: "bass",
        lines: &[line(1, Play::Root, "x...............x.......x.......")],
        fill: &[],
    };
    const TONE_ON_DRUMS: Part = Part {
        layer: "drums",
        lines: &[line(1, Play::Root, "x...............")],
        fill: &[],
    };
    const SHORT: Part = Part {
        layer: "bass",
        lines: &[line(1, Play::Root, "x...")],
        fill: &[],
    };
    const MISSING: Part = Part {
        layer: "horns",
        lines: &[],
        fill: &[],
    };

    #[test]
    fn a_two_bar_line_alternates_and_the_swing_moves_the_off_beats() {
        let r = |_: u64| 0.0;
        let s = |bar| {
            TWO_BAR
                .hits(LayerKind::Bass, 1, &[40], false, bar, &r)
                .iter()
                .map(|h| h.s16 as i32)
                .collect::<Vec<_>>()
        };
        assert_eq!((s(0), s(1)), (vec![0], vec![0, 8]));
        let swing = Groove {
            swing: 2.0 / 3.0,
            ..STRAIGHT
        };
        assert!((swing.swung(2.0) - 8.0 / 3.0).abs() < 1e-9);
        assert_eq!((swing.swung(0.0), swing.swung(4.0)), (0.0, 4.0));
        assert_eq!(STRAIGHT.swung(2.0), 2.0);
        let sixteens = Groove {
            swing16: true,
            swing: 0.6,
            ..STRAIGHT
        };
        assert!((sixteens.swung(1.0) - 1.2).abs() < 1e-9 && sixteens.swung(2.0) == 2.0);
    }

    #[test]
    fn a_badly_written_groove_is_refused() {
        let layers = [("drums", LayerKind::Drums), ("bass", LayerKind::Bass)];
        let ok = Groove {
            parts: &[PART],
            mix: &[("bass", -6.0), ("drums", 5.0)],
            ..STRAIGHT
        };
        assert_eq!(ok.check(&layers), Ok(()));
        let wide = Groove {
            mix: &[("bass", -7.0), ("drums", 6.0)],
            ..ok
        };
        assert!(wide.check(&layers).is_err());
        for part in [TONE_ON_DRUMS, SHORT, MISSING] {
            let g = Groove {
                parts: std::slice::from_ref(Box::leak(Box::new(part))),
                ..STRAIGHT
            };
            assert!(g.check(&layers).is_err(), "{part:?}");
        }
    }
}
