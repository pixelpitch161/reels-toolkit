//! The composer (milestone 5, Task 6, spec section 8): a fighter's line and victory solo written from its
//! musical personality, its role, the fight's harmony, the story signal and the music seed.
//!
//! The rules:
//! - **Chord tones on strong beats.** Beats 1 and 3 of every bar (every beat for a bass) land on a tone of that
//!   bar's chord.
//! - **A signature motif opens each phrase.** A phrase is 4 bars of the loop; the melody states one of its motifs
//!   at the phrase start and echoes it in the phrase's third bar, the answering line states it in the gap after
//!   the melody's first half bar, and a bass shapes its first bar after the motif's rhythm.
//! - **Inside the register.** Every note is folded into the fighter's register.
//! - **Shaped by role.** Bass: roots and fifths on the beat. Melody: motifs and the rhythm cells. Answer: replies
//!   in the second half of each bar, where the melody leaves a gap.
//! - **Roles follow the lead.** Bar by bar of the song, the leader fills out (passing notes between its notes),
//!   the trailing line thins as it falls behind, then roughens (only the strong beats, the octave thrown about).
//!   After the K.O. the winner plays full and the loser holds long notes.
//! - **No rests.** Most parts play a note on every 8th and refuse a rest, so a line breathes and thins by
//!   holding a note (the same note again), never by a rest. The parts' own HP thinning still drops 8ths on top.
//! - **A real victory solo**: the motif in the upper register, a run up the scale, the motif a third higher, and
//!   a held top note with the fighter's ornament on it.
//!
//! Only the music seed and the fighter's side seed it, so a re-roll changes the music and nothing else.
//!
//! The output is in the fight file's shapes: [`Composition::line`] is a `music.lines` entry over the chord loop
//! (8 notes a bar), [`Composition::song`] is the same line written out for every bar of the song with the lead's
//! shaping (for Task 7 to play bar by bar), and [`Composition::solo`] is a `music.solos` entry.

mod harmonise;
mod styles;

use bf_core::fightfile::SoloNote;
use bf_core::harmony::{Harmony, parse_chord};
use bf_core::rng::Generator;
use bf_sound::music::Style;
use bf_sound::music::signal::{Phase, StorySignal};

pub use self::harmonise::harmonise;
use crate::personality::{Ornament, Personality, Role, Step};

/// Notes a bar of a composed line: 8ths.
pub const PER_BAR: usize = 8;
/// Bars a phrase.
const PHRASE: usize = 4;

/// A fighter's composed music.
#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    /// The line over the chord loop, [`PER_BAR`] notes a bar (`None` is a rest): a `music.lines` entry.
    pub line: Vec<Option<i32>>,
    /// The line for every bar of the song, [`PER_BAR`] notes a bar, shaped by who leads.
    pub song: Vec<Option<i32>>,
    /// The victory solo, `(beat, note, beats)`: a `music.solos` entry.
    pub solo: Vec<(f64, SoloNote, f64)>,
}

impl Composition {
    /// The song line's notes of bar `bar`.
    pub fn song_bar(&self, bar: usize) -> &[Option<i32>] {
        &self.song[bar * PER_BAR..(bar + 1) * PER_BAR]
    }
}

/// How full a bar plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    /// Only the strong beats, the octave thrown about.
    Rough,
    /// Only the strong beats.
    Sparse,
    /// Only the beats.
    Thin,
    /// The line as written.
    Normal,
    /// Passing notes between the notes.
    Full,
}

/// The fighter's notes over the fight's harmony.
struct Pitch<'a> {
    h: &'a Harmony,
    lo: i32,
    hi: i32,
    /// Where scale degree 0 sits: the key's note nearest the personality's home.
    base: i32,
    /// The notes the fighter's own motifs start on (a Riff is not built on one).
    own: Vec<i32>,
}

impl Pitch<'_> {
    fn chord_pcs(chord: &str) -> Vec<i32> {
        parse_chord(chord).map(|c| c.pcs()).unwrap_or_else(|_| vec![0, 4, 7])
    }

    /// The chord's tones in the register (never empty: a register of less than an octave falls back to its ends).
    fn chord_tones(&self, chord: &str) -> Vec<i32> {
        let pcs = Self::chord_pcs(chord);
        let tones: Vec<i32> = (self.lo..=self.hi)
            .filter(|m| pcs.contains(&m.rem_euclid(12)))
            .collect();
        if tones.is_empty() { vec![self.lo] } else { tones }
    }

    /// The key's scale over `chord` in the register (a chord tone the scale lacks replaces the step below it).
    fn scale_tones(&self, chord: &str) -> Vec<i32> {
        let mut pcs: Vec<i32> = self
            .h
            .scale
            .iter()
            .map(|s| (self.h.key_pc + s).rem_euclid(12))
            .collect();
        for c in Self::chord_pcs(chord) {
            let below = (c - 1).rem_euclid(12);
            if !pcs.contains(&c) && pcs.contains(&below) {
                pcs.iter_mut().filter(|p| **p == below).for_each(|p| *p = c);
            }
        }
        let tones: Vec<i32> = (self.lo..=self.hi)
            .filter(|m| pcs.contains(&m.rem_euclid(12)))
            .collect();
        if tones.is_empty() {
            self.chord_tones(chord)
        } else {
            tones
        }
    }

    /// The root and fifth of `chord` in the register.
    fn roots_and_fifths(&self, chord: &str) -> Vec<i32> {
        let pcs = Self::chord_pcs(chord);
        let rf = [pcs[0], pcs.get(2).copied().unwrap_or(pcs[0])];
        let tones: Vec<i32> = (self.lo..=self.hi).filter(|m| rf.contains(&m.rem_euclid(12))).collect();
        if tones.is_empty() {
            self.chord_tones(chord)
        } else {
            tones
        }
    }

    /// The lowest root of `chord` in the register.
    fn low_root(&self, chord: &str) -> i32 {
        let r = Self::chord_pcs(chord)[0];
        let m = self.lo + (r - self.lo).rem_euclid(12);
        if m <= self.hi { m } else { self.chord_tones(chord)[0] }
    }

    /// `m` folded into the register by octaves.
    fn fold(&self, mut m: i32) -> i32 {
        while m > self.hi {
            m -= 12;
        }
        while m < self.lo {
            m += 12;
        }
        if m > self.hi { self.hi } else { m }
    }

    /// The degree's note before folding.
    fn degree(&self, deg: i32) -> i32 {
        let n = self.h.scale.len().max(1) as i32;
        let (oct, i) = (deg.div_euclid(n), deg.rem_euclid(n) as usize);
        self.base + 12 * oct + self.h.scale.get(i).copied().unwrap_or(0)
    }

    /// A motif as `(start sixteenth, note, sixteenths)`: moved as a whole by octaves to fit the register best
    /// (`high`: the highest octave that fits, for the solo), then each note folded in.
    fn motif(&self, motif: &[Step], high: bool) -> Vec<(usize, i32, usize)> {
        let raw: Vec<i32> = motif.iter().map(|s| self.degree(i32::from(s.deg))).collect();
        let outside = |k: i32| {
            raw.iter()
                .filter(|&&m| !(self.lo..=self.hi).contains(&(m + 12 * k)))
                .count()
        };
        let mut best = 0;
        for k in [0, 1, -1, 2, -2, 3, -3] {
            let better = outside(k) < outside(best) || (high && outside(k) == outside(best) && k > best);
            if better {
                best = k;
            }
        }
        let mut at = 0;
        motif
            .iter()
            .zip(raw)
            .map(|(s, m)| {
                let len = usize::from(s.len.max(1));
                let note = (at, self.fold(m + 12 * best), len);
                at += len;
                note
            })
            .collect()
    }
}

/// The nearest of `tones` to `m` (the lower one on a tie).
fn nearest(tones: &[i32], m: i32) -> i32 {
    *tones
        .iter()
        .min_by_key(|&&t| ((t - m).abs(), t))
        .expect("tones are never empty")
}

/// The seed of one fighter's music.
fn seed_of(seed: u64, side: usize) -> u64 {
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(side as u64 * 7919 + 17)
        & 0xFFFF_FFFF_FFFF
}

/// The melody style a fight on `theme` gets when none is chosen (the Music panel's "Auto"): one of the styles
/// that suit the theme's family, picked with the fight's music seed, the same for both fighters. A new seed is a
/// new pick; the fight file then stores the style's name, so the fight keeps it.
pub fn pick_style(theme: &str, music_seed: u64) -> Style {
    let list = crate::themes::family::styles_of(theme);
    // a stream of its own, so the pick does not move with what the lines draw
    let mut rng = Generator::new(seed_of(music_seed, 2) ^ 0x0057_171E);
    list[pick(&mut rng, list.len())]
}

/// How a fighter's music is to be written (sound upgrade, 2026-11-25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Brief {
    /// The melody style of the fight (C11).
    pub style: Style,
    /// The highest note the music may use, for a piercing instrument that is kept out of the top octave of its
    /// register (M3); `None` for the whole register.
    pub ceiling: Option<i32>,
}

impl Brief {
    /// The Signature style over the whole register.
    pub fn signature() -> Brief {
        Brief {
            style: Style::Signature,
            ceiling: None,
        }
    }
}

/// `compose(personality, role, chord loop, key, scale, signal, music seed)`: fighter `side`'s line and victory
/// solo, written as `brief` asks. The chord loop, key and scale are the harmony's.
pub fn compose(
    p: &Personality,
    role: Role,
    side: usize,
    h: &Harmony,
    signal: &StorySignal,
    music_seed: u64,
    brief: Brief,
) -> Composition {
    let (lo, hi) = (i32::from(p.register.0), i32::from(p.register.1.max(p.register.0)));
    let hi = brief.ceiling.map_or(hi, |c| hi.min(c).max(lo));
    let home = i32::from(p.home);
    let base = home + (h.key_pc - home + 6).rem_euclid(12) - 6;
    let mut pitch = Pitch {
        h,
        lo,
        hi,
        base,
        own: Vec::new(),
    };
    pitch.own = p.motifs.iter().map(|m| pitch.motif(m, false)[0].1).collect();
    let mut rng = Generator::new(seed_of(music_seed, side));
    let line = loop_line(p, role, &pitch, &mut rng, brief.style);
    let song = song_line(role, side, &pitch, signal, &line, &mut rng, brief.style);
    let solo = victory_solo(p, &pitch, signal, &mut rng);
    Composition { line, song, solo }
}

/// A pick of one of `n` things.
fn pick(rng: &mut Generator, n: usize) -> usize {
    if n <= 1 { 0 } else { rng.integers(n as u64) as usize }
}

/// A favourite leap: the first most often.
fn leap(p: &Personality, rng: &mut Generator) -> i32 {
    let ivs = if p.intervals.is_empty() {
        &[2u8][..]
    } else {
        p.intervals
    };
    let r = rng.random();
    let i = if r < 0.5 || ivs.len() == 1 {
        0
    } else if r < 0.8 || ivs.len() == 2 {
        1
    } else {
        2 + pick(rng, ivs.len() - 2)
    };
    i32::from(ivs[i])
}

/// The 8ths a rhythm cell plays.
fn slots_of(cell: &[u8]) -> Vec<usize> {
    let mut s: Vec<usize> = cell
        .iter()
        .map(|&x| usize::from(x) / 2)
        .filter(|&x| x < PER_BAR)
        .collect();
    s.dedup();
    s
}

/// The strong 8ths of a role's bar, which take chord tones. A Signature bass keeps every beat on a root or a
/// fifth; in the other styles a bass walks (by step, through the chord, to its octave), so only its downbeat is
/// held to the chord.
fn strong(role: Role, style: Style) -> &'static [usize] {
    match role {
        Role::Bass if style == Style::Signature => &[0, 2, 4, 6],
        Role::Bass => &[0],
        _ => &[0, 4],
    }
}

/// The strong 8ths of `bar` snapped to `chord`'s tones (a bass's to its roots and fifths).
fn snap(role: Role, style: Style, pitch: &Pitch, chord: &str, bar: &mut [Option<i32>]) {
    let tones = match role {
        Role::Bass => pitch.roots_and_fifths(chord),
        _ => pitch.chord_tones(chord),
    };
    for &j in strong(role, style) {
        if let Some(m) = bar[j] {
            bar[j] = Some(nearest(&tones, m));
        }
    }
}

/// What a bar between a Signature phrase's motif and its echo is made of (sound upgrade, C10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Between {
    /// The motif's own shape turned upside down, from the chord tone nearest the note before.
    Answer,
    /// A run by step into the chord tone nearest the motif's first note.
    Cadence,
    /// A walk by step with one of the fighter's favourite leaps, turned back after it.
    Walk,
}

/// The `k` notes of a melody's or answering line's bar between the motif and its echo, written out of the motif
/// itself instead of a walk by leaps (C10): lively, but no longer scattered.
fn lively_bar(
    (p, pitch): (&Personality, &Pitch),
    rng: &mut Generator,
    chord: &str,
    k: usize,
    kind: Between,
    motif: &[(usize, i32, usize)],
    prev: i32,
) -> Vec<i32> {
    let scale = pitch.scale_tones(chord);
    let tones = pitch.chord_tones(chord);
    let top = scale.len() as i32 - 1;
    let idx = |m: i32| {
        (0..scale.len())
            .min_by_key(|&i| ((scale[i] - m).abs(), scale[i]))
            .unwrap_or(0) as i32
    };
    let at = |i: i32| scale[i.clamp(0, top) as usize];
    let mut notes = Vec::with_capacity(k);
    match kind {
        Between::Cadence => {
            let goal = idx(nearest(&tones, motif[0].1));
            let side = if idx(prev) >= goal { 1 } else { -1 };
            for i in 0..k {
                let left = ((k - 1 - i) as i32).min(4);
                let j = goal + side * left;
                notes.push(at(if (0..=top).contains(&j) { j } else { goal - side * left }));
            }
        }
        Between::Answer => {
            // the motif's moves in scale steps, each the other way
            let moves: Vec<i32> = motif
                .windows(2)
                .map(|w| (idx(w[1].1) - idx(w[0].1)).clamp(-3, 3))
                .collect();
            let mut cur = idx(nearest(&tones, prev));
            let mut flip = -1;
            for i in 0..k {
                notes.push(at(cur));
                let d = moves[i % moves.len()];
                if !(0..=top).contains(&(cur + flip * d)) {
                    flip = -flip;
                }
                cur = (cur + flip * d).clamp(0, top);
            }
        }
        Between::Walk => {
            let mid = (pitch.lo + pitch.hi) / 2;
            let mut dir = if prev > mid { -1 } else { 1 };
            let mut cur = idx(prev);
            let leap_at = 1 + pick(rng, k.max(2) - 1);
            for i in 0..k {
                if i == leap_at {
                    cur = idx(nearest(&scale, at(cur) + dir * leap(p, rng)));
                    dir = -dir;
                } else {
                    if !(0..=top).contains(&(cur + dir)) {
                        dir = -dir;
                    }
                    cur = (cur + dir).clamp(0, top);
                }
                notes.push(at(cur));
            }
        }
    }
    notes
}

/// In a moving style no two notes written one after the other are the same (the line wraps round): a note that
/// repeats the one before it moves to the nearest other tone, a chord tone on a strong 8th and a scale tone
/// elsewhere, on the side of the note after it. What a held note or a thinned bar still repeats is dealt with
/// where the notes are played (`MusicCtx::play_line`).
fn unrepeat(out: &mut [Option<i32>], written: &[bool], role: Role, style: Style, pitch: &Pitch) {
    let chords = &pitch.h.loop_;
    let slots: Vec<usize> = (0..out.len()).filter(|&i| written[i]).collect();
    let len = slots.len();
    if len < 3 {
        return;
    }
    // the nearest other tone for slot `n` that is neither of its neighbours' notes
    let other = |out: &[Option<i32>], n: usize| -> Option<i32> {
        let i = slots[n];
        let m = out[i]?;
        let (before, after) = (out[slots[(n + len - 1) % len]], out[slots[(n + 1) % len]]);
        let chord = &chords[i / PER_BAR];
        let tones = match role {
            _ if !strong(role, style).contains(&(i % PER_BAR)) => pitch.scale_tones(chord),
            Role::Bass => pitch.roots_and_fifths(chord),
            _ => pitch.chord_tones(chord),
        };
        let upward = after.is_some_and(|a| a > m);
        tones
            .iter()
            .copied()
            .filter(|&t| Some(t) != before && Some(t) != after)
            .min_by_key(|&t| ((t - m).abs(), u8::from((t > m) != upward), t))
    };
    for n in 0..len {
        let before = (n + len - 1) % len;
        if out[slots[n]].is_none() || out[slots[n]] != out[slots[before]] {
            continue;
        }
        // the note itself moves; where it has nowhere to go (a strong 8th with two chord tones in reach), the
        // note before it does
        if let Some(t) = other(out, n) {
            out[slots[n]] = Some(t);
        } else if let Some(t) = other(out, before) {
            out[slots[before]] = Some(t);
        }
    }
}

/// The line over the loop at its normal shape.
fn loop_line(p: &Personality, role: Role, pitch: &Pitch, rng: &mut Generator, style: Style) -> Vec<Option<i32>> {
    let chords = &pitch.h.loop_;
    let n = chords.len();
    let mut out = vec![None; n * PER_BAR];
    let cells: Vec<Vec<usize>> = if p.rhythms.is_empty() {
        vec![vec![0, 2, 4, 6]]
    } else {
        p.rhythms.iter().map(|c| slots_of(c)).collect()
    };
    let sparsest = (0..cells.len()).min_by_key(|&i| cells[i].len()).unwrap_or(0);
    let mut prev = (pitch.lo + pitch.hi) / 2;
    let mut lb = 0;
    while lb < n {
        let phrase_len = PHRASE.min(n - lb);
        // Every style but Signature writes the whole phrase as a new tune (C11).
        if style != Style::Signature {
            if role == Role::Bass {
                styles::bass_phrase(style, pitch, rng, lb, phrase_len, &mut out);
            } else {
                styles::phrase(style, role, pitch, rng, lb, phrase_len, &mut out);
            }
            lb += phrase_len;
            continue;
        }
        let motif = if p.motifs.is_empty() {
            vec![(0, pitch.chord_tones(&chords[lb])[0], 4)]
        } else {
            pitch.motif(p.motifs[pick(rng, p.motifs.len())], false)
        };
        // The motif's 8ths from the phrase start (the answering line from the half bar).
        let offset = if role == Role::Answer { PER_BAR / 2 } else { 0 };
        let mut taken = vec![false; phrase_len * PER_BAR];
        let place = |out: &mut Vec<Option<i32>>, taken: &mut Vec<bool>, at: usize, notes: &[(usize, i32, usize)]| {
            for &(s, m, _) in notes {
                let j = at + s / 2;
                if j >= phrase_len * PER_BAR || taken[j] {
                    continue;
                }
                if role == Role::Answer && j % PER_BAR < PER_BAR / 2 {
                    continue;
                }
                taken[j] = true;
                out[lb * PER_BAR + j] = Some(m);
            }
        };
        let motif_end = offset + motif.last().map_or(0, |&(s, _, l)| (s + l).div_ceil(2));
        place(&mut out, &mut taken, offset, &motif);
        // The echo in the phrase's third bar: the motif moved to start on that bar's nearest chord tone.
        if role != Role::Bass && phrase_len >= 3 && motif_end <= 2 * PER_BAR {
            let first = motif[0].1;
            let to = nearest(&pitch.chord_tones(&chords[lb + 2]), first + 2);
            // moved by the same interval, then each note kept in key (the nearest tone of the scale over the chord)
            let in_key = pitch.scale_tones(&chords[lb + 2]);
            let echo: Vec<(usize, i32, usize)> = motif
                .iter()
                .map(|&(s, m, l)| (s, nearest(&in_key, pitch.fold(m + to - first)), l))
                .collect();
            place(&mut out, &mut taken, 2 * PER_BAR + offset, &echo);
        }
        if let Some(&(_, m, _)) = motif.last() {
            prev = m;
        }
        // The other bars, on the fighter's rhythm cells. A bass walks its roots and fifths. A melody or an answering
        // line writes them out of the motif (C10): its shape upside down as the answer in the second bar, a run by
        // step into the cadence in the last, a walk by step with one favourite leap anywhere else.
        for b in 0..phrase_len {
            let bar = lb + b;
            let chord = &chords[bar];
            if taken[b * PER_BAR..(b + 1) * PER_BAR].iter().any(|&t| t) {
                continue;
            }
            let cadence = b + 1 == phrase_len;
            let slots: Vec<usize> = match role {
                Role::Bass => (0..PER_BAR).step_by(2).collect(),
                Role::Melody => cells[if cadence { sparsest } else { pick(rng, cells.len()) }].clone(),
                Role::Answer => {
                    let c = &cells[if cadence { sparsest } else { pick(rng, cells.len()) }];
                    let mut s: Vec<usize> = c.iter().map(|&x| PER_BAR / 2 + x / 2).collect();
                    s.dedup();
                    s
                }
            };
            if role != Role::Bass {
                let kind = if cadence {
                    Between::Cadence
                } else if b == 1 && motif.len() >= 2 {
                    Between::Answer
                } else {
                    Between::Walk
                };
                let notes = lively_bar((p, pitch), rng, chord, slots.len(), kind, &motif, prev);
                for (&j, &m) in slots.iter().zip(&notes) {
                    out[bar * PER_BAR + j] = Some(m);
                }
                if let Some(&m) = notes.last() {
                    prev = m;
                }
                continue;
            }
            for &j in &slots {
                let m = if j == 0 {
                    pitch.low_root(chord)
                } else {
                    let rf = pitch.roots_and_fifths(chord);
                    rf[pick(rng, rf.len().min(3))]
                };
                out[bar * PER_BAR + j] = Some(m);
                prev = m;
            }
        }
        lb += phrase_len;
    }
    // No rests: most parts play a note on every 8th, so a rest holds the note before it (the line wraps round).
    let first = out
        .iter()
        .rev()
        .flatten()
        .next()
        .copied()
        .unwrap_or_else(|| pitch.low_root(&chords[0]));
    let written: Vec<bool> = out.iter().map(Option::is_some).collect();
    hold(&mut out, first);
    for (bar, chord) in chords.iter().enumerate() {
        let slice = &mut out[bar * PER_BAR..(bar + 1) * PER_BAR];
        snap(role, style, pitch, chord, slice);
    }
    if style.moving() {
        unrepeat(&mut out, &written, role, style, pitch);
        for (n, w) in out.iter_mut().zip(&written) {
            if !w {
                *n = None;
            }
        }
        let first = out.iter().rev().flatten().next().copied().unwrap_or(first);
        hold(&mut out, first);
    }
    out
}

/// No rests: each rest holds the note before it (`first` for a line that opens on one). Most parts play a note on
/// every 8th and refuse a rest, so a line thins by holding its notes longer.
fn hold(bar: &mut [Option<i32>], first: i32) {
    let mut cur = bar[0].unwrap_or(first);
    for n in bar.iter_mut() {
        match n {
            Some(m) => cur = *m,
            None => *n = Some(cur),
        }
    }
}

/// How fighter `side` plays bar `b` of the song.
fn shape_of(side: usize, signal: &StorySignal, b: usize) -> Shape {
    let bar = &signal.bars[b];
    let winner = signal.bars.last().and_then(|b| b.leader);
    // how far this side is behind, in HP
    let behind = if side == 0 { -bar.gap } else { bar.gap };
    let trailing = || {
        if behind < 15.0 {
            Shape::Thin
        } else if behind < 35.0 {
            Shape::Sparse
        } else {
            Shape::Rough
        }
    };
    match bar.phase {
        Phase::Intro => Shape::Thin,
        Phase::Fight | Phase::FinalExchange => match bar.leader {
            Some(k) if k == side => Shape::Full,
            Some(_) => trailing(),
            None => Shape::Normal,
        },
        Phase::Ko | Phase::Victory | Phase::BuildBack => {
            if winner == Some(side) {
                Shape::Full
            } else {
                Shape::Rough
            }
        }
    }
}

/// The line written out over every bar of the song, shaped by who leads.
fn song_line(
    role: Role,
    side: usize,
    pitch: &Pitch,
    signal: &StorySignal,
    line: &[Option<i32>],
    rng: &mut Generator,
    style: Style,
) -> Vec<Option<i32>> {
    let h = pitch.h;
    let n = h.loop_.len().max(1);
    let n_bars = signal.bars.len();
    let mut out = Vec::with_capacity(n_bars * PER_BAR);
    for b in 0..n_bars {
        let lb = (b as i64 - h.loop_start as i64).rem_euclid(n as i64) as usize;
        let chord = h.chords.get(b).unwrap_or(&h.loop_[lb]).clone();
        let next = h.chords.get(b + 1).unwrap_or(&chord).clone();
        let mut bar: Vec<Option<i32>> = line[lb * PER_BAR..(lb + 1) * PER_BAR].to_vec();
        // Variation (C10): on every other pass of the loop a melody's answering bar (the second of each phrase) is
        // turned upside down about its first note, so two passes in a row are never the same.
        let pass = b.checked_sub(h.loop_start).map_or(0, |d| d / n);
        if role != Role::Bass && pass % 2 == 1 && lb % PHRASE == 1 {
            mirror(pitch, &chord, &mut bar);
        }
        let shape = shape_of(side, signal, b);
        if role == Role::Bass {
            shape_bass(pitch, &chord, &next, shape, &mut bar);
        } else {
            shape_upper(role, pitch, &chord, shape, &mut bar, rng);
        }
        let first = out
            .last()
            .copied()
            .flatten()
            .or_else(|| line.iter().rev().flatten().next().copied());
        hold(&mut bar, first.unwrap_or_else(|| pitch.low_root(&chord)));
        snap(role, style, pitch, &chord, &mut bar);
        // No bar twice in a row (C10). A thinned line can hold one note over two bars, which comes out as the same
        // bar again: its second half then moves to the nearest other chord tones. A riff is the same bar on purpose.
        if role != Role::Bass && style != Style::Riff && b > 0 && out[(b - 1) * PER_BAR..] == bar[..] {
            let tones = pitch.chord_tones(&chord);
            for m in bar[PER_BAR / 2..].iter_mut().flatten() {
                let other = tones
                    .iter()
                    .copied()
                    .filter(|t| t != m)
                    .min_by_key(|&t| ((t - *m).abs(), t));
                *m = other.unwrap_or(*m);
            }
        }
        out.extend(bar);
    }
    out
}

/// `bar` turned upside down about its first note, in steps of the scale over `chord` (a note whose mirror would
/// leave the register stays).
fn mirror(pitch: &Pitch, chord: &str, bar: &mut [Option<i32>]) {
    let scale = pitch.scale_tones(chord);
    let idx = |m: i32| {
        (0..scale.len())
            .min_by_key(|&i| ((scale[i] - m).abs(), scale[i]))
            .unwrap_or(0) as i32
    };
    let Some(first) = bar.iter().flatten().next().copied() else {
        return;
    };
    let axis = idx(first);
    for m in bar.iter_mut().flatten() {
        let j = 2 * axis - idx(*m);
        if (0..scale.len() as i32).contains(&j) {
            *m = scale[j as usize];
        }
    }
}

/// A melody's or answering line's bar at `shape`.
fn shape_upper(role: Role, pitch: &Pitch, chord: &str, shape: Shape, bar: &mut [Option<i32>], rng: &mut Generator) {
    match shape {
        Shape::Normal => {}
        Shape::Full => {
            // passing notes in place of held 8ths (the answering line keeps the melody's gap)
            let scale = pitch.scale_tones(chord);
            let from = if role == Role::Answer { PER_BAR / 2 } else { 0 };
            for j in from + 1..PER_BAR {
                if bar[j] != bar[j - 1] || rng.random() < 0.25 {
                    continue;
                }
                let Some(a) = bar[j - 1] else { continue };
                let target = bar[j + 1..].iter().flatten().next().copied().unwrap_or(a + 2);
                let pass = if target >= a {
                    scale.iter().copied().find(|&t| t > a)
                } else {
                    scale.iter().copied().rev().find(|&t| t < a)
                };
                bar[j] = Some(pass.unwrap_or(a));
            }
        }
        Shape::Thin => bar.iter_mut().skip(1).step_by(2).for_each(|n| *n = None),
        Shape::Sparse | Shape::Rough => {
            for (j, n) in bar.iter_mut().enumerate() {
                if j % (PER_BAR / 2) != 0 {
                    *n = None;
                }
            }
            if role == Role::Answer {
                bar[0] = None;
            }
            if shape == Shape::Rough {
                // the octave thrown about, and now and then a strong beat lost
                for j in [0, PER_BAR / 2] {
                    if let Some(m) = bar[j] {
                        let r = rng.random();
                        bar[j] = if r < 0.3 {
                            None
                        } else if r < 0.65 && m - 12 >= pitch.lo {
                            Some(m - 12)
                        } else if m + 12 <= pitch.hi {
                            Some(m + 12)
                        } else {
                            Some(m)
                        };
                    }
                }
            }
        }
    }
}

/// A bass's bar at `shape`: it never rests, so thinning holds notes longer.
fn shape_bass(pitch: &Pitch, chord: &str, next: &str, shape: Shape, bar: &mut [Option<i32>]) {
    let root = pitch.low_root(chord);
    match shape {
        Shape::Normal => {}
        Shape::Full => {
            // an octave jump on the and of 2, a chromatic step into the next bar's root
            let up = root + 12;
            if up <= pitch.hi {
                bar[3] = Some(up);
            }
            let to = pitch.low_root(next);
            let approach = if to > pitch.lo { to - 1 } else { to + 1 };
            if approach <= pitch.hi {
                bar[7] = Some(approach);
            }
        }
        Shape::Thin => {
            for j in 0..PER_BAR {
                let from = j / (PER_BAR / 2) * (PER_BAR / 2);
                bar[j] = bar[from];
            }
        }
        Shape::Sparse => bar.iter_mut().for_each(|n| *n = Some(root)),
        Shape::Rough => {
            let low = if root - 12 >= pitch.lo { root - 12 } else { root };
            for (j, n) in bar.iter_mut().enumerate() {
                *n = Some(if j < PER_BAR / 2 { root } else { low });
            }
        }
    }
}

/// The winner's victory solo, over the victory bars' chords.
fn victory_solo(
    p: &Personality,
    pitch: &Pitch,
    signal: &StorySignal,
    rng: &mut Generator,
) -> Vec<(f64, SoloNote, f64)> {
    let h = pitch.h;
    let victory: Vec<usize> = signal
        .bars
        .iter()
        .filter(|b| b.phase == Phase::Victory)
        .map(|b| b.bar)
        .collect();
    let n_bars = victory.len().clamp(2, 4);
    let chord_of = |i: usize| -> String {
        victory
            .get(i)
            .and_then(|&b| h.chords.get(b))
            .unwrap_or_else(|| &h.loop_[i % h.loop_.len()])
            .clone()
    };
    let motif = if p.motifs.is_empty() {
        vec![(0, pitch.chord_tones(&chord_of(0))[0], 4)]
    } else {
        pitch.motif(p.motifs[pick(rng, p.motifs.len())], true)
    };
    let mut notes: Vec<(f64, i32, f64)> = Vec::new();
    // bar 1: the motif, high, within the bar
    let put_motif = |notes: &mut Vec<(f64, i32, f64)>, bar: usize, shift: i32| {
        for &(s, m, l) in &motif {
            if s >= 16 {
                break;
            }
            let len = l.min(16 - s);
            notes.push((
                (bar * 4) as f64 + s as f64 / 4.0,
                pitch.fold(m + shift),
                len as f64 / 4.0,
            ));
        }
        // its last note rings to the bar's end
        if let Some(n) = notes.last_mut() {
            n.2 = (bar * 4 + 4) as f64 - n.0;
        }
    };
    put_motif(&mut notes, 0, 0);
    let fast = p.rhythms.iter().map(|c| c.len()).max().unwrap_or(8) >= 10;
    let step = if fast { 0.25 } else { 0.5 };
    // a run up the scale over `chord` from above `from`, from beat `at` to beat `until`; the beat it ends on
    let run = |notes: &mut Vec<(f64, i32, f64)>, from: i32, chord: &str, at: f64, until: f64| -> f64 {
        let scale = pitch.scale_tones(chord);
        let start = scale.iter().position(|&t| t > from).unwrap_or(scale.len() - 1);
        let mut beat = at;
        for &m in &scale[start..] {
            if beat >= until - 1e-9 {
                break;
            }
            notes.push((beat, m, step));
            beat += step;
        }
        beat
    };
    if n_bars >= 3 {
        // bar 2: a run up the scale to a high chord tone, held
        let from = notes.last().map_or(pitch.lo, |n| n.1) - 7;
        let beat = run(&mut notes, from, &chord_of(1), 4.0, 7.0);
        let tones = pitch.chord_tones(&chord_of(1));
        let reached = notes.last().map_or(pitch.lo, |n| n.1);
        let top = tones
            .iter()
            .copied()
            .find(|&t| t > reached)
            .unwrap_or(*tones.last().expect("tones"));
        notes.push((beat, top, 8.0 - beat));
    }
    if n_bars == 4 {
        // bar 3: the motif a third higher
        let up = nearest(&pitch.chord_tones(&chord_of(2)), motif[0].1 + 4) - motif[0].1;
        put_motif(&mut notes, 2, up);
    }
    // the last bar: a climb (a quick run after a 2-bar solo's motif, else two chord tones), then the top root
    // held with the ornament
    let lb = (n_bars - 1) as f64 * 4.0;
    let ch = chord_of(n_bars - 1);
    let tones = pitch.chord_tones(&ch);
    let root = Pitch::chord_pcs(&ch)[0];
    let roots: Vec<i32> = tones.iter().copied().filter(|m| m.rem_euclid(12) == root).collect();
    let goal = roots.last().copied().unwrap_or(*tones.last().expect("tones"));
    let below: Vec<i32> = tones.iter().copied().filter(|&t| t < goal).collect();
    if n_bars == 2 {
        let from = below.len().checked_sub(3).map_or(goal, |i| below[i]);
        notes.push((lb, from, step));
        run(&mut notes, from, &ch, lb + step, lb + 1.75);
    } else {
        let a = below.len().checked_sub(2).map_or(goal, |i| below[i]);
        let b = below.last().copied().unwrap_or(goal);
        notes.push((lb, a, 1.0));
        notes.push((lb + 1.0, b, 0.75));
    }
    let upper = pitch.scale_tones(&ch).into_iter().find(|&t| t > goal).unwrap_or(goal);
    let lower = pitch
        .scale_tones(&ch)
        .into_iter()
        .rev()
        .find(|&t| t < goal)
        .unwrap_or(goal);
    match p.ornament {
        Ornament::Trill => {
            for i in 0..4 {
                let m = if i % 2 == 0 { goal } else { upper };
                notes.push((lb + 2.0 + i as f64 * 0.25, m, 0.25));
            }
            notes.push((lb + 3.0, goal, 1.0));
        }
        Ornament::Grace => {
            notes.push((lb + 1.875, upper, 0.125));
            notes.push((lb + 2.0, goal, 2.0));
        }
        Ornament::Slide => {
            notes.push((lb + 1.75, lower, 0.25));
            notes.push((lb + 2.0, goal, 2.0));
        }
        Ornament::Stab => {
            notes.push((lb + 2.0, goal, 0.5));
            notes.push((lb + 3.0, goal, 1.0));
        }
        Ornament::None => notes.push((lb + 2.0, goal, 2.0)),
    }
    // overlaps cut, strong beats (1 and 3 of each bar) on chord tones, every note inside the register
    notes.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<(f64, i32, f64)> = Vec::with_capacity(notes.len());
    for (i, &(bt, m, d)) in notes.iter().enumerate() {
        if out.last().is_some_and(|l| (l.0 - bt).abs() < 1e-9) {
            continue;
        }
        let next = notes.get(i + 1).map_or(f64::INFINITY, |n| n.0);
        let d = d.min(next - bt).max(0.125);
        let bar = (bt / 4.0).floor() as usize;
        let on_strong = (bt % 2.0).abs() < 1e-9;
        let m = if on_strong {
            nearest(&pitch.chord_tones(&chord_of(bar)), m)
        } else {
            pitch.fold(m)
        };
        out.push((bt, m, d));
    }
    out.into_iter().map(|(b, m, d)| (b, SoloNote::Note(m), d)).collect()
}

#[cfg(test)]
mod tests {
    use bf_core::grid::Grid;
    use bf_core::harmony::{Harmony, chords_for, scale};
    use bf_sound::music::signal::{BarSignal, Phase, StorySignal};

    use super::*;

    /// Auto picks one of the theme's styles with the music seed: the same for the same seed, and over a few seeds
    /// every style of the list.
    #[test]
    fn auto_picks_from_the_themes_styles() {
        for theme in [
            "mambo",
            "hot_jazz",
            "stadium_rock",
            "starfighter",
            "dubstep",
            "klezmer",
            "big_top",
        ] {
            let list = crate::themes::family::styles_of(theme);
            let picks: Vec<Style> = (0..64).map(|seed| pick_style(theme, seed)).collect();
            for (seed, s) in picks.iter().enumerate() {
                assert!(list.contains(s), "{theme}, seed {seed}: {s:?}");
                assert_eq!(pick_style(theme, seed as u64), *s);
            }
            for s in list {
                assert!(picks.contains(s), "{theme}: {s:?} is never picked in 64 seeds");
            }
        }
    }
    use crate::personality::personality;

    const ALL: [&str; 39] = [
        "ACOUSTIC",
        "ALTO SAX",
        "BASS CLARINET",
        "BASS DRUM",
        "BASSOON",
        "CELLO",
        "CLARINET",
        "CONTRABASS",
        "ELECTRIC",
        "EUPHONIUM",
        "FLUTE",
        "HORN",
        "OBOE",
        "ORGAN",
        "PIANO",
        "SOUSAPHONE",
        "TENOR SAX",
        "TROMBONE",
        "TRUMPET",
        "TUBA",
        "ACCORDION",
        "BAGPIPES",
        "BARI SAX",
        "DRUMS",
        "HARP",
        "SYNTH",
        "TURNTABLES",
        "UKULELE",
        "VIOLA",
        "VIOLIN",
        "MARIMBA",
        "PICCOLO",
        "SOPRANO SAX",
        "MELLOPHONE",
        "TIMPANI",
        "CONTRABASSOON",
        "SNARE DRUM",
        "OCTOBASS",
        "BASS GUITAR",
    ];

    fn v(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    /// The episode's harmony: E harmonic minor over its 8-chord loop.
    fn harmony() -> Harmony {
        let grid = Grid::new(134.0);
        let lp = v(&["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]);
        let chords = chords_for(&v(&["Em", "B7"]), &lp, &[], grid.n_bars);
        Harmony::new(&grid, "E", scale("harmonic minor").unwrap(), chords, Some(lp), 2).unwrap()
    }

    /// A story where fighter 0 pulls ever further ahead and wins.
    fn signal(n_bars: usize) -> StorySignal {
        let bars = (0..n_bars)
            .map(|bar| {
                let phase = match bar {
                    0 | 1 => Phase::Intro,
                    b if b + 1 == n_bars => Phase::BuildBack,
                    b if b + 3 >= n_bars => Phase::Victory,
                    b if b + 4 == n_bars => Phase::Ko,
                    b if b + 6 >= n_bars => Phase::FinalExchange,
                    _ => Phase::Fight,
                };
                BarSignal {
                    bar,
                    t: bar as f64,
                    leader: (bar >= 4).then_some(0),
                    gap: 3.0 * bar as f64,
                    phase,
                }
            })
            .collect();
        StorySignal {
            bars,
            moments: Vec::new(),
        }
    }

    fn compose_for(name: &str, role: Role, side: usize, seed: u64) -> Composition {
        let h = harmony();
        let sig = signal(h.chords.len());
        compose(
            personality(name).unwrap(),
            role,
            side,
            &h,
            &sig,
            seed,
            Brief::signature(),
        )
    }

    fn solo_notes(c: &Composition) -> Vec<(f64, i32, f64)> {
        c.solo.iter().map(|(b, n, d)| (*b, n.single().unwrap(), *d)).collect()
    }

    #[test]
    fn the_same_inputs_give_the_same_notes_and_another_seed_others() {
        let a = compose_for("FLUTE", Role::Melody, 0, 1);
        assert_eq!(a, compose_for("FLUTE", Role::Melody, 0, 1));
        assert_ne!(a, compose_for("FLUTE", Role::Melody, 0, 2));
        assert_ne!(a.line, compose_for("FLUTE", Role::Melody, 1, 1).line);
        assert_eq!(a.line.len(), 8 * PER_BAR);
        assert_eq!(a.song.len(), harmony().chords.len() * PER_BAR);
        bf_core::fightfile::check_line("FLUTE", &a.line, 8).unwrap();
    }

    #[test]
    fn every_note_is_in_register_and_strong_beats_are_chord_tones() {
        let h = harmony();
        for name in ALL {
            let p = personality(name).unwrap();
            let (lo, hi) = (i32::from(p.register.0), i32::from(p.register.1));
            for &role in p.roles {
                for seed in [1, 7, 99] {
                    let c = compose_for(name, role, 1, seed);
                    let pitch = Pitch {
                        h: &h,
                        lo,
                        hi,
                        base: 0,
                        own: Vec::new(),
                    };
                    let notes = c.line.iter().chain(&c.song).flatten().copied();
                    let solo = solo_notes(&c);
                    for m in notes.chain(solo.iter().map(|n| n.1)) {
                        assert!((lo..=hi).contains(&m), "{name} {role:?}: {m} outside {lo}..={hi}");
                    }
                    for (lb, chord) in h.loop_.iter().enumerate() {
                        let tones = pitch.chord_tones(chord);
                        for &j in strong(role, Style::Signature) {
                            if let Some(m) = c.line[lb * PER_BAR + j] {
                                assert!(
                                    tones.contains(&m),
                                    "{name} {role:?} loop bar {lb} 8th {j}: {m} on {chord}"
                                );
                            }
                        }
                    }
                    for (b, chord) in h.chords.iter().enumerate() {
                        let tones = pitch.chord_tones(chord);
                        for &j in strong(role, Style::Signature) {
                            if let Some(m) = c.song_bar(b)[j] {
                                assert!(tones.contains(&m), "{name} {role:?} song bar {b}: {m} on {chord}");
                            }
                        }
                    }
                    for &(bt, m, _) in &solo {
                        if (bt % 2.0).abs() < 1e-9 {
                            let bar = (bt / 4.0) as usize;
                            let chord = &h.loop_[bar % h.loop_.len()];
                            let sig = signal(h.chords.len());
                            let victory: Vec<usize> = sig
                                .bars
                                .iter()
                                .filter(|b| b.phase == Phase::Victory)
                                .map(|b| b.bar)
                                .collect();
                            let chord = victory.get(bar).map_or(chord, |&b| &h.chords[b]);
                            assert!(pitch.chord_tones(chord).contains(&m), "{name} solo beat {bt}: {m}");
                        }
                    }
                    assert!(
                        c.line.iter().chain(&c.song).all(Option::is_some),
                        "{name} {role:?}: a rest"
                    );
                }
            }
        }
    }

    #[test]
    fn a_motif_opens_each_phrase() {
        let h = harmony();
        for name in ["FLUTE", "ACCORDION", "TRUMPET", "VIOLIN"] {
            let p = personality(name).unwrap();
            let (lo, hi) = (i32::from(p.register.0), i32::from(p.register.1));
            let home = i32::from(p.home);
            let pitch = Pitch {
                h: &h,
                lo,
                hi,
                base: home + (h.key_pc - home + 6).rem_euclid(12) - 6,
                own: Vec::new(),
            };
            let firsts: Vec<i32> = p
                .motifs
                .iter()
                .map(|m| nearest(&pitch.chord_tones(&h.loop_[0]), pitch.motif(m, false)[0].1))
                .collect();
            let firsts4: Vec<i32> = p
                .motifs
                .iter()
                .map(|m| nearest(&pitch.chord_tones(&h.loop_[4]), pitch.motif(m, false)[0].1))
                .collect();
            for seed in [1, 2, 3] {
                let c = compose_for(name, Role::Melody, 0, seed);
                assert!(
                    firsts.contains(&c.line[0].unwrap()),
                    "{name}: phrase 1 opens {:?}",
                    c.line[0]
                );
                assert!(firsts4.contains(&c.line[4 * PER_BAR].unwrap()), "{name}: phrase 2");
                // the solo opens with a motif too
                assert!(!c.solo.is_empty() && c.solo[0].0 == 0.0);
            }
        }
    }

    /// A listening sample: FLUTE (melody) and TROMBONE (bass) of the episode playing their composed lines and
    /// solos over the classic bed, written as a wav to `BF_COMPOSER_WAV`, with the master's levels printed.
    #[test]
    #[ignore = "a listening sample, run by hand"]
    fn listening_sample() {
        use bf_core::fightfile::FightFile;
        use bf_core::recording::Recording;
        use bf_core::show::Show;

        let rec = Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec")).unwrap();
        let mut ff = FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).unwrap();
        let rms = |x: &[[f64; 2]]| {
            let r = (x.iter().map(|s| s[0] * s[0] + s[1] * s[1]).sum::<f64>() / (2 * x.len()) as f64).sqrt();
            20.0 * bf_core::dmath::log10(r + 1e-12)
        };
        let classic = crate::build_soundtrack(&rec, &ff).unwrap();
        ff.music.lines.clear();
        ff.music.solos.clear();
        let show = Show::new(&rec, &ff).unwrap();
        let (lines, solos) = {
            let build = crate::SoundBuild::new(&show, &rec, &ff).unwrap();
            let sig = StorySignal::from(&build.ctx).unwrap();
            let mut lines = Vec::new();
            let mut solos = Vec::new();
            for (k, (name, role)) in [("FLUTE", Role::Melody), ("TROMBONE", Role::Bass)]
                .into_iter()
                .enumerate()
            {
                let c = compose(
                    personality(name).unwrap(),
                    role,
                    k,
                    &build.ctx.harmony,
                    &sig,
                    ff.music.music_seed,
                    Brief::signature(),
                );
                println!("{name} line {:?}\n{name} solo {:?}", c.line, solo_notes(&c));
                lines.push((name.to_string(), c.line));
                solos.push((name.to_string(), c.solo));
            }
            (lines, solos)
        };
        ff.music.lines.extend(lines);
        ff.music.solos.extend(solos);
        let st = crate::build_soundtrack(&rec, &ff).unwrap();
        let peak = st.master.iter().fold(0.0f64, |p, s| p.max(s[0].abs()).max(s[1].abs()));
        println!(
            "master: peak {peak:.3}, {:.1} dBFS (classic {:.1})",
            rms(&st.master),
            rms(&classic.master)
        );
        for (bus, x) in &st.stems {
            println!(
                "stem {bus}: {:.1} dBFS (classic {:.1})",
                rms(x),
                rms(&classic.stems[bus])
            );
        }
        if let Ok(path) = std::env::var("BF_COMPOSER_WAV") {
            let n = st.master.len() as u32;
            let mut w = Vec::with_capacity(44 + 4 * n as usize);
            let rate = bf_sound::SR as u32;
            for (tag, v) in [(b"RIFF", 36 + 4 * n), (b"WAVE", 0)] {
                w.extend_from_slice(tag);
                if tag == b"RIFF" {
                    w.extend_from_slice(&v.to_le_bytes());
                }
            }
            w.extend_from_slice(b"fmt ");
            w.extend_from_slice(&16u32.to_le_bytes());
            for v in [1u16, 2] {
                w.extend_from_slice(&v.to_le_bytes());
            }
            w.extend_from_slice(&rate.to_le_bytes());
            w.extend_from_slice(&(rate * 4).to_le_bytes());
            for v in [4u16, 16] {
                w.extend_from_slice(&v.to_le_bytes());
            }
            w.extend_from_slice(b"data");
            w.extend_from_slice(&(4 * n).to_le_bytes());
            for s in &st.master {
                for c in s {
                    w.extend_from_slice(&((c.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
                }
            }
            std::fs::write(path, w).unwrap();
        }
    }

    #[test]
    fn the_trailing_side_plays_fewer_notes_in_its_last_bars_than_the_leader() {
        let h = harmony();
        let sig = signal(h.chords.len());
        let ko = sig.bars.iter().position(|b| b.phase == Phase::Ko).unwrap();
        // the notes struck: an 8th that holds the note before it strikes none
        let count = |c: &Composition| {
            let s = &c.song[(ko - 4) * PER_BAR - 1..ko * PER_BAR];
            s.windows(2).filter(|w| w[0] != w[1]).count()
        };
        for name in ["FLUTE", "TRUMPET", "OBOE", "SYNTH"] {
            for seed in [1, 5] {
                let lead = compose_for(name, Role::Melody, 0, seed);
                let trail = compose_for(name, Role::Melody, 1, seed);
                assert!(
                    count(&trail) < count(&lead),
                    "{name}: {} against {}",
                    count(&trail),
                    count(&lead)
                );
            }
        }
    }
}
