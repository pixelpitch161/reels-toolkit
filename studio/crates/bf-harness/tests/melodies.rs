//! The melody bars of the sound upgrade design (`docs/studio/2026-11-25-sound-upgrade-design.md`, C2, C10 and
//! C11), over generated pairings: every pitched fighter with two others, on two harmonies and three seeds, in
//! each of the six melody styles (`bf_harness::melody`). `cargo run --release -p bf-harness --example melodies`
//! prints the same numbers.

use bf_harness::melody::{SEEDS, survey};
use bf_sound::music::Style;

/// Today's lines (the composer as it was before the sound upgrade, measured on 2026-11-25 with this survey's
/// pairings, harmonies and story), for each seed of [`SEEDS`]: a melody's note changes a bar, and the part of its
/// bars that repeat the bar before.
const TODAY: [(f64, f64); 3] = [(4.8414, 0.0220), (4.8096, 0.0393), (4.7416, 0.0147)];

#[test]
fn the_melodies_hold_the_design_bars() {
    for (&seed, (changes, bar_repeats)) in SEEDS.iter().zip(TODAY) {
        let r = survey(&[seed]);
        assert_eq!(r.pairings, 76);
        assert_eq!(r.styles.len(), Style::ALL.len());
        for s in &r.styles {
            let style = s.style.key();
            // C2: no semitone, tritone or major seventh between the two lines, in any style
            assert_eq!(s.clashes, 0, "seed {seed}, {style}: harsh pairs between the two lines");
            assert_eq!(s.out_of_register, 0, "seed {seed}, {style}: notes outside a register");
            // C11: a moving style writes no note twice in a row
            if s.style.moving() {
                assert_eq!(s.repeats, 0, "seed {seed}, {style}: a note written twice in a row");
            }
            // C10 and C11: at most 30% of a melody's moves are bigger than a fourth (leaps are the nature of
            // Broken chords and Anchor note)
            if !matches!(s.style, Style::BrokenChords | Style::Anchor) {
                assert!(
                    s.worst_leaps <= 0.30,
                    "seed {seed}, {style}: {:.1}% of the moves of {} are bigger than a fourth",
                    100.0 * s.worst_leaps,
                    s.worst_leaper
                );
            }
        }
        // C10: the Signature melody is as lively as today's and repeats a bar no more often
        let signature = &r.styles[0];
        assert_eq!(signature.style, Style::Signature);
        assert!(
            signature.changes_per_bar >= 0.85 * changes,
            "seed {seed}: {:.2} note changes a bar, today {changes:.2}",
            signature.changes_per_bar
        );
        assert!(
            signature.bar_repeats <= bar_repeats,
            "seed {seed}: {:.4} of the bars repeat, today {bar_repeats:.4}",
            signature.bar_repeats
        );
        // C11: a style is a new tune
        assert!(
            r.melody_least_differ >= 0.5,
            "seed {seed}: two styles share more than half of a melody's notes ({})",
            r.melody_least
        );
        assert!(
            r.bass_least_differ >= 0.2,
            "seed {seed}: two styles share more than four fifths of a bass's notes ({})",
            r.bass_least
        );
    }
}
