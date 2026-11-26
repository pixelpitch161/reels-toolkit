//! The melody styles other than Signature (sound upgrade, C11): each writes a whole phrase as a new tune, for a
//! melody, an answering line or a bass, from the fight's scale and chords inside the fighter's register. Seeded
//! picks (which figure, how high) make the two fighters and two fights differ.
//!
//! - **Climbing**: a short rising figure that starts two scale steps higher each bar; the last bar falls back to a
//!   chord tone. Bass: a walk up by step from the root.
//! - **Riff**: three or four close notes, the same bar on the changing chords, then a changed ending. Bass: root,
//!   octave and fifth figures.
//! - **Runs**: up the scale and back on every 8th, turning at the edges of the register; the last bar runs into
//!   its chord tone. Bass: the scale up one bar, down the next.
//! - **Broken chords**: up and down the notes of the bar's chord. Bass: root, third, fifth.
//! - **Anchor note**: one note keeps returning on the off 8ths while a second voice moves by step against it.
//!   Bass: the root, with its octave at the end of the bar.
//!
//! The figures are steps of the scale (or of the chord's tones) counted from a home tone; a step past the edge of
//! the register comes back in an octave lower or higher, so a figure keeps its shape in a narrow register.

use bf_core::rng::Generator;
use bf_sound::music::Style;

use super::{PER_BAR, Pitch, nearest, pick};
use crate::personality::Role;

/// Tones in rising order (a scale or a chord in the register), read by step.
struct Ladder {
    tones: Vec<i32>,
    /// The steps of an octave: 7 for a scale of seven, the number of chord tones in one for a chord.
    octave: i32,
}

impl Ladder {
    fn new(tones: Vec<i32>) -> Ladder {
        let lowest = tones[0];
        let octave = tones.iter().filter(|&&t| t < lowest + 12).count().max(1) as i32;
        Ladder { tones, octave }
    }

    fn top(&self) -> i32 {
        self.tones.len() as i32 - 1
    }

    /// The step nearest to the note `m` (the lower on a tie).
    fn idx(&self, m: i32) -> i32 {
        (0..self.tones.len())
            .min_by_key(|&i| ((self.tones[i] - m).abs(), self.tones[i]))
            .unwrap_or(0) as i32
    }

    /// The tone at step `i`; a step past either end comes back in by octaves.
    fn at(&self, mut i: i32) -> i32 {
        let top = self.top();
        while i > top {
            i -= self.octave;
        }
        while i < 0 {
            i += self.octave;
        }
        self.tones[i.clamp(0, top) as usize]
    }

    /// Where a figure that reaches from `low` to `high` steps about its home may start so that it stays on the
    /// ladder: `start` itself where it fits, else moved by octaves, else as near as the ends allow. A figure wider
    /// than the ladder keeps its start (its notes come back in by octaves).
    fn fit(&self, mut start: i32, (low, high): (i32, i32)) -> i32 {
        let (floor, ceiling) = (-low, self.top() - high);
        if floor > ceiling {
            return start;
        }
        while start > ceiling && start - self.octave >= floor {
            start -= self.octave;
        }
        while start < floor && start + self.octave <= ceiling {
            start += self.octave;
        }
        start.clamp(floor, ceiling)
    }
}

/// The lowest and highest step of a figure.
fn span(fig: &[i32]) -> (i32, i32) {
    (
        fig.iter().copied().min().unwrap_or(0),
        fig.iter().copied().max().unwrap_or(0),
    )
}

/// A step that walks a ladder and turns at its ends.
struct Walker {
    at: i32,
    dir: i32,
    top: i32,
}

impl Walker {
    /// One step on: the step it lands on. On a ladder of one tone it stays.
    fn step(&mut self) -> i32 {
        if self.top == 0 {
            return 0;
        }
        if !(0..=self.top).contains(&(self.at + self.dir)) {
            self.dir = -self.dir;
        }
        self.at += self.dir;
        self.at
    }
}

/// The figures of a melody's bar, as steps from the bar's home: three to pick from for each style that has them.
const CLIMB: [[i32; PER_BAR]; 3] = [
    [0, 1, 2, 1, 2, 3, 4, 3],
    [0, 2, 1, 3, 2, 4, 3, 5],
    [0, 1, 2, 3, 2, 3, 4, 5],
];
const RIFF: [[i32; PER_BAR]; 3] = [
    [0, 0, 2, 0, 3, 0, 2, -1],
    [0, 0, 0, 2, 3, 0, 2, 0],
    [0, 2, 0, 0, -1, 0, 2, 0],
];
const RIFF_END: [i32; PER_BAR] = [0, 0, 2, 3, 4, 4, 4, 4];
const BROKEN: [[i32; PER_BAR]; 3] = [
    [0, 1, 2, 3, 2, 1, 2, 1],
    [0, 2, 1, 3, 2, 1, 0, 1],
    [3, 2, 1, 0, 1, 2, 3, 2],
];
const BROKEN_END: [i32; PER_BAR] = [3, 2, 3, 2, 1, 2, 1, 0];
const ANCHOR: [i32; PER_BAR / 2] = [2, 3, 4, 3];

/// One phrase of a melody or an answering line in `style`, written into `out` from loop bar `lb` on. The
/// answering line plays the second half of each bar, in the gap the melody leaves.
pub(super) fn phrase(
    style: Style,
    role: Role,
    pitch: &Pitch,
    rng: &mut Generator,
    lb: usize,
    phrase_len: usize,
    out: &mut [Option<i32>],
) {
    let chords = &pitch.h.loop_;
    let mid = (pitch.lo + pitch.hi) / 2;
    let var = pick(rng, 3);
    let lift = pick(rng, 3) as i32 - 1;
    // the note the last bar ended on (Runs go on from it)
    let mut carry: Option<i32> = None;
    let mut run_dir = if var == 0 { -1 } else { 1 };
    for b in 0..phrase_len {
        let bar = lb + b;
        let chord = &chords[bar];
        let scale = Ladder::new(pitch.scale_tones(chord));
        let tones = pitch.chord_tones(chord);
        let cadence = b + 1 == phrase_len;
        let home = scale.idx(nearest(&tones, mid + 2 * lift));
        let mut notes = [0i32; PER_BAR];
        match style {
            Style::Climbing => {
                let start = scale.fit(home - 4 + 2 * b as i32, span(&CLIMB[var]));
                // the fall back: down the scale from a seventh above home
                let fall = scale.fit(home, (0, 7));
                for (j, n) in notes.iter_mut().enumerate() {
                    *n = if cadence {
                        scale.at(fall + 7 - j as i32)
                    } else {
                        scale.at(start + CLIMB[var][j])
                    };
                }
                if cadence {
                    // to rest on a chord tone that is not the note before it
                    let before = notes[PER_BAR - 2];
                    let rest: Vec<i32> = tones.iter().copied().filter(|&t| t != before).collect();
                    notes[PER_BAR - 1] = if rest.is_empty() {
                        before
                    } else {
                        nearest(&rest, before - 1)
                    };
                }
            }
            Style::Riff => {
                let fig = if cadence { &RIFF_END } else { &RIFF[var] };
                // one home for the riff and its ending, so the ending grows out of the riff
                let reach = (span(&RIFF[var]).0.min(0), span(&RIFF[var]).1.max(span(&RIFF_END).1));
                // not on the note the fighter's own motif starts on: a riff there can come out as the signature
                // tune (the electric guitar's chug is one), and a style is a new tune
                let free: Vec<i32> = tones.iter().copied().filter(|t| !pitch.own.contains(t)).collect();
                let from = if free.is_empty() { &tones } else { &free };
                let root = scale.fit(scale.idx(nearest(from, mid - 3 + 2 * lift)), reach);
                for (j, n) in notes.iter_mut().enumerate() {
                    *n = scale.at(root + fig[j]);
                }
            }
            Style::Runs => {
                if cadence {
                    // into the chord tone nearest the middle, from the side the run came from
                    let goal = scale.idx(nearest(&tones, mid));
                    let from_above = carry.is_some_and(|c| scale.idx(c) >= goal);
                    let side = if from_above { 1 } else { -1 };
                    for (j, n) in notes.iter_mut().enumerate() {
                        let left = (PER_BAR - 1 - j) as i32;
                        let i = goal + side * left;
                        *n = scale.tones[(if (0..=scale.top()).contains(&i) {
                            i
                        } else {
                            goal - side * left
                        })
                        .clamp(0, scale.top()) as usize];
                    }
                } else {
                    let mut w = Walker {
                        at: carry.map_or((home - 3).clamp(0, scale.top()), |c| scale.idx(c)),
                        dir: run_dir,
                        top: scale.top(),
                    };
                    for (j, n) in notes.iter_mut().enumerate() {
                        // every other bar turns in its middle: up and back
                        if b % 2 == 1 && j == PER_BAR / 2 {
                            w.dir = -w.dir;
                        }
                        // a phrase's first note is where the run stands; every later one a step on
                        *n = scale.tones[(if carry.is_none() && j == 0 { w.at } else { w.step() }) as usize];
                    }
                    run_dir = w.dir;
                }
            }
            Style::BrokenChords => {
                let chord_ladder = Ladder::new(tones.clone());
                let low = chord_ladder.idx(mid - 5);
                let fig = if cadence { &BROKEN_END } else { &BROKEN[var] };
                for (j, n) in notes.iter_mut().enumerate() {
                    *n = chord_ladder.at(low + fig[j] + i32::from(b == 2));
                }
            }
            Style::Anchor | Style::Signature => {
                // the anchor above with the voice moving down under it, or below with the voice climbing over it
                let above = var == 0;
                let anchor = nearest(&tones, if above { mid + 5 } else { mid - 5 });
                let (ai, dir) = (scale.idx(anchor), if above { -1 } else { 1 });
                for (j, n) in notes.iter_mut().enumerate() {
                    *n = if j % 2 == 1 || (cadence && j + 2 >= PER_BAR) {
                        anchor
                    } else {
                        scale.at(ai + dir * (ANCHOR[j / 2] + b as i32))
                    };
                }
            }
        }
        for (j, &m) in notes.iter().enumerate() {
            if role == Role::Answer && j < PER_BAR / 2 {
                continue;
            }
            out[bar * PER_BAR + j] = Some(m);
        }
        carry = Some(notes[PER_BAR - 1]);
    }
}

/// One phrase of a bass in `style`: four notes a bar (the beats), counted in scale steps from the chord's lowest
/// root.
pub(super) fn bass_phrase(
    style: Style,
    pitch: &Pitch,
    rng: &mut Generator,
    lb: usize,
    phrase_len: usize,
    out: &mut [Option<i32>],
) {
    let chords = &pitch.h.loop_;
    let var = pick(rng, 3);
    for b in 0..phrase_len {
        let bar = lb + b;
        let chord = &chords[bar];
        let scale = Ladder::new(pitch.scale_tones(chord));
        let root = pitch.low_root(chord);
        let ri = scale.idx(root);
        let fig: [i32; 4] = match style {
            Style::Climbing if b + 1 == phrase_len => [0, 4, 2, 1],
            Style::Climbing => [0, 1, 2, 3],
            Style::Riff => [[0, 0, 7, 4], [0, 7, 0, 4], [0, 0, 4, 3]][var],
            Style::Runs if b % 2 == 0 => [0, 1, 2, 3],
            Style::Runs => [4, 3, 2, 1],
            Style::BrokenChords => [[0, 2, 4, 2], [0, 4, 2, 4], [0, 2, 4, 7]][var],
            Style::Anchor | Style::Signature => [0, 0, 0, 7],
        };
        for (q, &off) in fig.iter().enumerate() {
            out[bar * PER_BAR + 2 * q] = Some(if off == 0 { root } else { scale.at(ri + off) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two octaves of C major from middle C.
    fn c_major() -> Ladder {
        Ladder::new(vec![60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84])
    }

    #[test]
    fn a_ladder_reads_by_step_and_comes_back_in_by_octaves() {
        let l = c_major();
        assert_eq!((l.octave, l.top()), (7, 14));
        // between two tones: the lower
        assert_eq!(l.idx(66), 3);
        assert_eq!(l.at(14), 84);
        assert_eq!(l.at(15), 74);
        assert_eq!(l.at(-1), 71);
        // the tones of a chord: three to the octave
        let chord = Ladder::new(vec![60, 64, 67, 72, 76, 79]);
        assert_eq!(chord.octave, 3);
        assert_eq!(chord.at(6), 72);
    }

    #[test]
    fn a_figure_is_fitted_onto_the_ladder() {
        let l = c_major();
        // it fits: it stays
        assert_eq!(l.fit(5, (0, 5)), 5);
        // too high: an octave down
        assert_eq!(l.fit(12, (0, 5)), 5);
        // too low: an octave up
        assert_eq!(l.fit(-3, (0, 2)), 4);
        assert_eq!(l.fit(2, (-3, 0)), 9);
        // a narrow ladder: as near as the ends allow, and a figure wider than the ladder keeps its start
        let narrow = Ladder::new(vec![60, 62, 64, 65]);
        assert_eq!(narrow.fit(3, (0, 2)), 1);
        assert_eq!(narrow.fit(3, (0, 5)), 3);
        assert_eq!(span(&[0, 2, 1, 3]), (0, 3));
    }

    #[test]
    fn a_walker_turns_at_the_ends() {
        let mut w = Walker { at: 1, dir: 1, top: 2 };
        assert_eq!([w.step(), w.step(), w.step(), w.step()], [2, 1, 0, 1]);
        let mut still = Walker { at: 0, dir: 1, top: 0 };
        assert_eq!(still.step(), 0);
    }
}
