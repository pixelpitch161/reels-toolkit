//! The play-time side of a composed line (sound upgrade, C11): [`MusicCtx::play_line`], which a part calls for
//! every ordinary note it sounds, beside [`MusicCtx::line_at`], which only reads the line.
//!
//! A line is a map from time to pitch, so a written line cannot stop a part from playing one pitch twice: a held
//! note sounds the same on every hit that lands on it, and several hits can land inside one written 8th. In a
//! **moving** style ([`Style::moving`]) the rule therefore lives here: a hit never plays the note of the hit before
//! it. Where the line still holds the note the last hit started from, the hit takes the next tone of the fight's
//! scale from the last played note, walking toward the line's next different note and turning back one step before
//! it; with no room between the two it alternates with the neighbour on the far side. A walked note that would
//! clash with the other fighter's line at that moment (a semitone, a tritone or a major seventh, in any octave)
//! takes the next step instead.
//!
//! The notes a fighter played are kept by their time, not by the order the part asked for them (a part may place
//! its bounces first and its swings after), so a note is checked against the one sounded before it *and* the one
//! after it in time. The record lives in the build's own state: two renders of one fight give the same notes.
//!
//! In a repeating style the line plays as written and the notes are only recorded ([`MusicCtx::played`]); under
//! the Classic arrangement ([`MusicCtx::polish`] is `None`) `play_line` is `line_at` and nothing is recorded.

use bf_core::py::round_half_even;

use super::error::MusicError;
use super::style::PartNote;
use super::{ComposedPart, MusicCtx};

/// A note a fighter sounded: its pitch, and the slot the line's note it came from began on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Hit {
    note: i32,
    run: usize,
}

/// The time of a hit as the key of the record: whole microseconds.
fn key_of(t: f64) -> i64 {
    round_half_even(t * 1e6) as i64
}

impl ComposedPart {
    /// The slot of the line `t` falls on (the nearest, as [`ComposedPart::note_at`] reads), if the line has any.
    fn slot_at(&self, t: f64, bar: f64) -> Option<usize> {
        if self.song.is_empty() || self.per_bar == 0 {
            return None;
        }
        let slot = round_half_even(t / (bar / self.per_bar as f64)).max(0.0) as usize;
        Some(slot.min(self.song.len() - 1))
    }

    /// Where the note sounding at `slot` began (the first slot of the run of that same note, rests held), and the
    /// line's next different note within the four bars after it.
    fn run_at(&self, slot: usize, note: i32) -> (usize, Option<i32>) {
        let same = |i: usize| self.song[i].is_none_or(|m| m == note);
        let mut start = slot;
        while start > 0 && same(start - 1) {
            start -= 1;
        }
        let target = self.song[slot + 1..]
            .iter()
            .take(4 * self.per_bar)
            .flatten()
            .find(|&&m| m != note)
            .copied();
        (start, target)
    }
}

/// Whether two notes clash in any octave: a semitone, a tritone or a major seventh apart.
pub fn clashes(a: i32, b: i32) -> bool {
    matches!((a - b).rem_euclid(12), 1 | 6 | 11)
}

impl MusicCtx<'_> {
    /// Whether this build plays the Composed arrangement's upgrade (a Classic build never does).
    pub fn composed_on(&self) -> bool {
        self.polish.is_some()
    }

    /// Whether the fight's melody style never plays a note twice in a row.
    pub fn moving(&self) -> bool {
        self.polish.as_ref().is_some_and(|p| p.style().moving())
    }

    /// How fighter `name`'s notes are treated under the Composed arrangement (`None` under Classic, and for a name
    /// that is no fighter of the fight).
    pub fn part_note(&self, name: &str) -> Option<PartNote> {
        self.polish.as_ref().and_then(|p| p.parts.get(name)).copied()
    }

    /// Whether fighter `name` sounds one note at a time here (see [`crate::mix::Mix::note`]). Never under Classic.
    pub fn one_voice(&self, name: &str) -> bool {
        self.part_note(name).is_some_and(|p| p.one_voice)
    }

    /// The level of an ordinary note `midi` of fighter `name`: under 1 for the top notes of a piercing instrument
    /// ([`super::style::soften`]), 1 for every other note and under Classic.
    pub fn soften(&self, name: &str, midi: i32) -> f64 {
        match self.part_note(name) {
            Some(p) if p.piercing => super::style::soften(midi),
            _ => 1.0,
        }
    }

    /// The next tone of the fight's scale from `m` in direction `dir` (1 up, -1 down).
    fn scale_step(&self, m: i32, dir: i32) -> i32 {
        let h = &self.harmony;
        let in_scale = |x: i32| {
            h.scale
                .iter()
                .any(|s| (h.key_pc + s).rem_euclid(12) == x.rem_euclid(12))
        };
        (1..=12).map(|d| m + dir * d).find(|&x| in_scale(x)).unwrap_or(m + dir)
    }

    /// The tone of the fight's scale a step from `m` toward `to` (the step below when `to` is not above `m`): the
    /// passing tone of a grace note in a moving style.
    pub fn passing_tone(&self, m: i32, to: i32) -> i32 {
        self.scale_step(m, if to > m { 1 } else { -1 })
    }

    /// The note fighter `name` sounds at `t`: a part calls this, once, for each ordinary note it really plays (after
    /// its own thinning), and [`MusicCtx::line_at`] for a note it only looks at. `None` is a rest.
    ///
    /// Under Classic this is `line_at`. Under Composed the note is recorded, and in a moving style it is never the
    /// note the fighter sounded just before or just after `t` (see the module notes). Asking twice for one instant
    /// gives one note.
    ///
    /// # Errors
    /// As [`MusicCtx::line_at`].
    pub fn play_line(&self, name: &str, t: f64) -> Result<Option<i32>, MusicError> {
        let line = self.line_at(name, t)?;
        let (Some(l), Some(part), Some(polish)) = (line, self.composed.get(name), self.polish.as_ref()) else {
            return Ok(line);
        };
        let Some(slot) = part.slot_at(t, self.show.grid.bar) else {
            return Ok(line);
        };
        let (run, target) = part.run_at(slot, l);
        let key = key_of(t);
        let mut st = self.lock()?;
        let log = st.played.entry(name.to_string()).or_default();
        if let Some(hit) = log.get(&key) {
            return Ok(Some(hit.note));
        }
        if !polish.style().moving() {
            log.insert(key, Hit { note: l, run });
            return Ok(Some(l));
        }
        let mut before = log.range(..key).rev().map(|(_, h)| *h);
        let (prev, prev2) = (before.next(), before.next());
        let next = log.range(key + 1..).next().map(|(_, h)| h.note);
        let holding = prev.is_some_and(|p| p.run == run);
        let taken = |m: i32| prev.is_some_and(|p| p.note == m) || next == Some(m);
        let note = if !holding && !taken(l) {
            l
        } else {
            let other = self
                .show
                .names
                .iter()
                .find(|n| n.as_str() != name)
                .and_then(|n| self.composed.get(n))
                .and_then(|p| p.note_at(t, self.show.grid.bar));
            let (lo_r, hi_r) = polish.parts.get(name).map_or((0, 127), |p| p.register);
            let toward = target.map_or(1, |tg| if tg > l { 1 } else { -1 });
            // the room of the walk: from the held note to one step before the next one
            let (mut lo, mut hi) = match target {
                Some(tg) if tg > l => (l, self.scale_step(tg, -1)),
                Some(tg) => (self.scale_step(tg, 1), l),
                None => (l, self.scale_step(self.scale_step(l, 1), 1)),
            };
            if hi <= lo {
                let far = self.scale_step(l, -toward);
                (lo, hi) = (far.min(l), far.max(l));
            }
            let from = match prev {
                Some(p) if holding => p.note,
                _ => l,
            };
            let dir = match (prev, prev2) {
                (Some(p), Some(p2)) if holding && p2.run == run && p2.note != p.note => (p.note - p2.note).signum(),
                _ => toward,
            };
            // in order of liking: on along the walk, back along it, the line's own note, then ever wider steps
            let mut wishes = vec![self.scale_step(from, dir), self.scale_step(from, -dir), l];
            let (mut up, mut down) = (from, from);
            for _ in 0..7 {
                up = self.scale_step(up, toward);
                down = self.scale_step(down, -toward);
                wishes.extend([up, down]);
            }
            let fits = |m: i32| !taken(m) && (lo_r..=hi_r).contains(&m);
            let clean = |m: i32| other.is_none_or(|o| !clashes(o, m));
            wishes
                .iter()
                .copied()
                .find(|&m| fits(m) && clean(m) && (lo..=hi).contains(&m))
                .or_else(|| wishes.iter().copied().find(|&m| fits(m) && clean(m)))
                .or_else(|| wishes.iter().copied().find(|&m| fits(m)))
                .or_else(|| wishes.iter().copied().find(|&m| !taken(m)))
                .unwrap_or(l)
        };
        log.insert(key, Hit { note, run });
        Ok(Some(note))
    }

    /// The note fighter `name` last sounded at or before `t` (the pitch a rapid-fire effect stays on), read without
    /// recording anything; the line's note where the fighter has not played yet, and under Classic.
    ///
    /// # Errors
    /// As [`MusicCtx::line_at`].
    pub fn hold_line(&self, name: &str, t: f64) -> Result<Option<i32>, MusicError> {
        if self.polish.is_some() && self.composed.contains_key(name) {
            let st = self.lock()?;
            let last = st.played.get(name).and_then(|log| log.range(..=key_of(t)).next_back());
            if let Some((_, hit)) = last {
                return Ok(Some(hit.note));
            }
        }
        self.line_at(name, t)
    }

    /// The ordinary notes fighter `name` has sounded so far, `(seconds, note)` in time order (empty under Classic).
    ///
    /// # Errors
    /// When a voice panicked earlier.
    pub fn played(&self, name: &str) -> Result<Vec<(f64, i32)>, MusicError> {
        let st = self.lock()?;
        Ok(st
            .played
            .get(name)
            .map(|log| log.iter().map(|(&k, h)| (k as f64 / 1e6, h.note)).collect())
            .unwrap_or_default())
    }
}
