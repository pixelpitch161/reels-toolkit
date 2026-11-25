//! The melodies measured (sound upgrade, 2026-11-25: C10, C2 and C11): generated pairings of the pitched fighters
//! composed in every melody style, and the figures the design's "Done when" lines name. The test `melodies` holds
//! them to the bars; `cargo run --release -p bf-harness --example melodies` prints the same report.
//!
//! A pairing is two fighters with roles (a melody over a bass where the second fighter can play one, else a melody
//! and an answering line), composed over two harmonies with a story in which the left fighter pulls ahead and wins,
//! then put through the harmony pass as a render does it.

use bf_content::composed_parts::UNPITCHED;
use bf_content::composer::{Brief, Composition, PER_BAR, compose, harmonise};
use bf_content::{Role, personality, roster};
use bf_core::grid::Grid;
use bf_core::harmony::{Harmony, chords_for, scale};
use bf_sound::music::signal::{BarSignal, Phase, StorySignal};
use bf_sound::music::{Style, clashes};

/// The music seeds the test and the example survey.
pub const SEEDS: [u64; 3] = [1, 7, 99];

/// What one style measured over every pairing.
#[derive(Clone, Debug)]
pub struct StyleReport {
    pub style: Style,
    /// Slots where the two song lines are a semitone, a tritone or a major seventh apart (C2: none).
    pub clashes: usize,
    /// Notes outside the fighter's register.
    pub out_of_register: usize,
    /// Written notes of the loop lines that repeat the written note before them (a moving style: none).
    pub repeats: usize,
    /// Of every melody line, the largest part of its moves that is bigger than a fourth (Signature: of the moves
    /// inside the bars between the motif and its echo).
    pub worst_leaps: f64,
    /// The fighter and seed of [`StyleReport::worst_leaps`].
    pub worst_leaper: String,
    /// The mean number of note changes a bar in the melody song lines.
    pub changes_per_bar: f64,
    /// The mean part of a melody song line's bars that are the bar before them again.
    pub bar_repeats: f64,
}

/// The whole survey.
#[derive(Clone, Debug)]
pub struct Report {
    pub styles: Vec<StyleReport>,
    /// The least part of a melody fighter's written notes that differ between two styles of one pairing and seed.
    pub melody_least_differ: f64,
    pub melody_least: String,
    /// The same of a bass fighter's.
    pub bass_least_differ: f64,
    pub bass_least: String,
    /// How many pairings were composed (each in every style, seed and harmony).
    pub pairings: usize,
}

fn v(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

/// The two harmonies: the first episode's (E harmonic minor over eight chords) and a major one over four.
pub fn harmonies() -> Vec<Harmony> {
    let grid = Grid::new(134.0);
    let minor = v(&["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]);
    let major = v(&["C", "Am", "F", "G"]);
    vec![
        Harmony::new(
            &grid,
            "E",
            scale("harmonic minor").expect("the scale"),
            chords_for(&v(&["Em", "B7"]), &minor, &[], grid.n_bars),
            Some(minor),
            2,
        )
        .expect("the minor harmony"),
        Harmony::new(
            &grid,
            "C",
            scale("major").expect("the scale"),
            chords_for(&v(&["C", "G"]), &major, &[], grid.n_bars),
            Some(major),
            2,
        )
        .expect("the major harmony"),
    ]
}

/// A story of `n_bars` bars in which the left fighter pulls ever further ahead and wins.
pub fn story(n_bars: usize) -> StorySignal {
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

/// The pitched fighters that have a personality, in roster order.
pub fn pitched() -> Vec<&'static str> {
    roster()
        .iter()
        .map(|f| f.name.as_str())
        .filter(|n| !UNPITCHED.contains(n) && personality(n).is_some())
        .collect()
}

/// The pairings: every pitched fighter as the left one, with the fighters 1 and 7 places on in the roster.
pub fn pairings() -> Vec<[(&'static str, Role); 2]> {
    let names = pitched();
    let mut out = Vec::new();
    for (i, &a) in names.iter().enumerate() {
        for step in [1, 7] {
            let b = names[(i + step) % names.len()];
            let can = |n: &str, r: Role| personality(n).is_some_and(|p| p.roles.contains(&r));
            let first = |n: &str| {
                personality(n)
                    .and_then(|p| p.roles.first().copied())
                    .unwrap_or(Role::Melody)
            };
            let ra = if can(a, Role::Melody) { Role::Melody } else { first(a) };
            let rb = if can(b, Role::Bass) && ra != Role::Bass {
                Role::Bass
            } else if can(b, Role::Answer) {
                Role::Answer
            } else {
                first(b)
            };
            out.push([(a, ra), (b, rb)]);
        }
    }
    out
}

fn register(name: &str) -> (i32, i32) {
    let p = personality(name).expect("a personality");
    (i32::from(p.register.0), i32::from(p.register.1.max(p.register.0)))
}

/// A pairing composed in `style` with `seed` over `h`, harmony pass and all: the two compositions.
pub fn composed(pair: &[(&'static str, Role); 2], h: &Harmony, style: Style, seed: u64) -> [Composition; 2] {
    let sig = story(h.chords.len());
    let brief = Brief { style, ceiling: None };
    let mut c = [0, 1].map(|k| {
        compose(
            personality(pair[k].0).expect("a personality"),
            pair[k].1,
            k,
            h,
            &sig,
            seed,
            brief,
        )
    });
    let (r0, r1) = (pair[0].1, pair[1].1);
    let mid = |k: usize| register(pair[k].0).0 + register(pair[k].0).1;
    let giver = if r1 == Role::Bass && r0 != Role::Bass {
        0
    } else if r0 == Role::Bass && r1 != Role::Bass {
        1
    } else {
        usize::from(mid(0) > mid(1))
    };
    let scale_pcs: Vec<i32> = h.scale.iter().map(|s| (h.key_pc + s).rem_euclid(12)).collect();
    let other = c[1 - giver].song.clone();
    harmonise(&mut c[giver].song, &other, &scale_pcs, register(pair[giver].0));
    c
}

/// The 8ths of a bar a role writes in a style other than Signature (the rest hold the note before).
fn written(role: Role) -> Vec<usize> {
    match role {
        Role::Melody => (0..PER_BAR).collect(),
        Role::Answer => (PER_BAR / 2..PER_BAR).collect(),
        Role::Bass => (0..PER_BAR).step_by(2).collect(),
    }
}

/// The survey over `seeds`.
pub fn survey(seeds: &[u64]) -> Report {
    let hs = harmonies();
    let pairs = pairings();
    let mut styles: Vec<StyleReport> = Vec::new();
    let mut least = [(f64::INFINITY, String::new()), (f64::INFINITY, String::new())];
    for style in Style::ALL {
        let mut r = StyleReport {
            style,
            clashes: 0,
            out_of_register: 0,
            repeats: 0,
            worst_leaps: 0.0,
            worst_leaper: String::new(),
            changes_per_bar: 0.0,
            bar_repeats: 0.0,
        };
        let mut melodies = 0usize;
        for pair in &pairs {
            for (hi, h) in hs.iter().enumerate() {
                for &seed in seeds {
                    let c = composed(pair, h, style, seed);
                    r.clashes += c[0]
                        .song
                        .iter()
                        .zip(&c[1].song)
                        .filter(|(a, b)| matches!((a, b), (Some(a), Some(b)) if clashes(*a, *b)))
                        .count();
                    for k in 0..2 {
                        let (name, role) = pair[k];
                        let (lo, hi_note) = register(name);
                        let all = c[k].line.iter().chain(&c[k].song).flatten();
                        r.out_of_register += all.filter(|m| !(lo..=hi_note).contains(m)).count();
                        if style != Style::Signature {
                            // the written notes, one after the other round the loop
                            let slots = written(role);
                            let notes: Vec<i32> = c[k]
                                .line
                                .chunks(PER_BAR)
                                .flat_map(|bar| slots.iter().map(|&j| bar[j]))
                                .flatten()
                                .collect();
                            r.repeats += (0..notes.len())
                                .filter(|&i| notes[i] == notes[(i + notes.len() - 1) % notes.len()])
                                .count();
                        }
                        if role == Role::Bass {
                            continue;
                        }
                        melodies += 1;
                        // The moves of the line. Signature: those inside the bars the composer writes between the
                        // motif and its echo (the second and last bar of each phrase); a fighter's own motif may
                        // leap (a harp's sweep) and is not what C10 changed.
                        let from = if role == Role::Answer { PER_BAR / 2 } else { 0 };
                        let moves: Vec<i32> = c[k]
                            .line
                            .chunks(PER_BAR)
                            .enumerate()
                            .filter(|(b, _)| style != Style::Signature || b % 2 == 1)
                            .flat_map(|(_, bar)| bar[from..].windows(2))
                            .filter_map(|w| Some((w[1]? - w[0]?).abs()))
                            .filter(|&d| d > 0)
                            .collect();
                        let big = moves.iter().filter(|&&d| d > 5).count() as f64 / moves.len().max(1) as f64;
                        if big > r.worst_leaps {
                            r.worst_leaps = big;
                            r.worst_leaper = format!("{name} {role:?}, seed {seed}, harmony {hi}");
                        }
                        let song = &c[k].song[h.loop_start * PER_BAR..];
                        let bars: Vec<&[Option<i32>]> = song.chunks(PER_BAR).collect();
                        r.changes_per_bar +=
                            song.windows(2).filter(|w| w[0] != w[1]).count() as f64 / bars.len().max(1) as f64;
                        r.bar_repeats +=
                            bars.windows(2).filter(|w| w[0] == w[1]).count() as f64 / (bars.len().max(2) - 1) as f64;
                    }
                }
            }
        }
        r.changes_per_bar /= melodies.max(1) as f64;
        r.bar_repeats /= melodies.max(1) as f64;
        styles.push(r);
    }
    // how far two styles' written lines differ, for the same pairing, seed and harmony
    for pair in &pairs {
        for (hi, h) in hs.iter().enumerate() {
            for &seed in seeds {
                let lines: Vec<[Composition; 2]> = Style::ALL.iter().map(|&s| composed(pair, h, s, seed)).collect();
                for i in 0..lines.len() {
                    for j in i + 1..lines.len() {
                        for k in 0..2 {
                            // over the 8ths the role writes (an answering line holds through the first half bar)
                            let slots = written(pair[k].1);
                            let notes = |c: &Composition| -> Vec<Option<i32>> {
                                c.line
                                    .chunks(PER_BAR)
                                    .flat_map(|bar| slots.iter().map(|&j| bar[j]))
                                    .collect()
                            };
                            let (a, b) = (notes(&lines[i][k]), notes(&lines[j][k]));
                            let differ =
                                a.iter().zip(&b).filter(|(x, y)| x != y).count() as f64 / a.len().max(1) as f64;
                            let which = usize::from(pair[k].1 == Role::Bass);
                            if differ < least[which].0 {
                                least[which] = (
                                    differ,
                                    format!(
                                        "{} {:?}, {} against {}, seed {seed}, harmony {hi}",
                                        pair[k].0,
                                        pair[k].1,
                                        Style::ALL[i].key(),
                                        Style::ALL[j].key()
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    let [(melody_least_differ, melody_least), (bass_least_differ, bass_least)] = least;
    Report {
        styles,
        melody_least_differ,
        melody_least,
        bass_least_differ,
        bass_least,
        pairings: pairs.len(),
    }
}
