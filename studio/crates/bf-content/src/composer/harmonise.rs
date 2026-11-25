//! The harmony pass (sound upgrade, C2): after both fighters' lines are written, the one that gives way moves a
//! note wherever the two would clash.

use bf_sound::music::clashes;

use super::PER_BAR;

/// Whether `m` under or over `other` makes a unison, a third, a fifth, a sixth or an octave (in any octave).
fn sweet(other: i32, m: i32) -> bool {
    matches!((other - m).rem_euclid(12), 0 | 3 | 4 | 7 | 8 | 9)
}

/// Moves the notes of `giver` that clash with `other`, slot by slot (both lines hold [`PER_BAR`] notes a bar from
/// the same bar). A pair clashes when the two notes are a semitone, a tritone or a major seventh apart in any
/// octave, or a whole tone apart on a strong 8th (the first and fifth of the bar). The giver's note then moves to
/// the nearest note of the scale (`scale_pcs`: its pitch classes), within four semitones and inside `register`,
/// that makes a unison, third, fifth, sixth or octave with the other; of two equally near, the one closer to the
/// giver's note before. Where the register holds no such note (a bass's chromatic step under a narrow instrument),
/// the nearest note of the scale that at least does not clash; where there is none either, the note stays. Returns
/// how many notes moved.
pub fn harmonise(giver: &mut [Option<i32>], other: &[Option<i32>], scale_pcs: &[i32], register: (i32, i32)) -> usize {
    let (lo, hi) = register;
    let mut moved = 0;
    let mut before: Option<i32> = None;
    for i in 0..giver.len().min(other.len()) {
        if let (Some(mine), Some(theirs)) = (giver[i], other[i]) {
            let strong = i % (PER_BAR / 2) == 0;
            let whole_tone = matches!((theirs - mine).rem_euclid(12), 2 | 10);
            if clashes(theirs, mine) || (strong && whole_tone) {
                let near = |ok: &dyn Fn(i32) -> bool| {
                    (mine - 4..=mine + 4)
                        .filter(|m| (lo..=hi).contains(m) && scale_pcs.contains(&m.rem_euclid(12)) && ok(*m))
                        .min_by_key(|&m| ((m - mine).abs(), before.map_or(0, |q| (m - q).abs()), m))
                };
                let best = near(&|m| sweet(theirs, m)).or_else(|| near(&|m| !clashes(theirs, m) && m != mine));
                if let Some(m) = best {
                    giver[i] = Some(m);
                    moved += 1;
                }
            }
        }
        if giver[i].is_some() {
            before = giver[i];
        }
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    const C_MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

    #[test]
    fn a_clash_moves_to_the_nearest_sweet_note_of_the_scale() {
        // against a held C (60)
        let other = vec![Some(60); 8];
        let mut giver = vec![
            Some(71),
            Some(64),
            Some(66),
            Some(65),
            Some(62),
            None,
            Some(59),
            Some(65),
        ];
        let moved = harmonise(&mut giver, &other, &C_MAJOR, (55, 80));
        // 71 (B, a major seventh) -> 72 (C, a step away; A is two away). 64 stays (a third). 66 (a tritone) -> 65
        // (F, a fifth under the C; G would be a fourth under it and E is further). 65 stays. 62 (D, a whole tone)
        // on the strong fifth 8th -> 64: C is as near, but E is closer to the F before. The rest stays a rest.
        // 59 (B) -> 60. The last 65 stays.
        assert_eq!(
            giver,
            [
                Some(72),
                Some(64),
                Some(65),
                Some(65),
                Some(64),
                None,
                Some(60),
                Some(65)
            ]
        );
        assert_eq!(moved, 4);
        // a whole tone on a weak 8th is let through
        let mut weak = vec![Some(64), Some(62)];
        assert_eq!(harmonise(&mut weak, &other, &C_MAJOR, (55, 80)), 0);
        for (g, o) in giver.iter().zip(&other) {
            if let (Some(g), Some(o)) = (g, o) {
                assert!(!clashes(*o, *g));
            }
        }
    }

    #[test]
    fn a_note_with_nowhere_to_go_stays() {
        // the register holds only the clashing note
        let mut giver = vec![Some(61)];
        assert_eq!(harmonise(&mut giver, &[Some(60)], &C_MAJOR, (61, 61)), 0);
        assert_eq!(giver, [Some(61)]);
        // no sweet note in reach (against a B flat the scale's sweet notes are G and D, both outside 69 to 73):
        // the nearest note of the scale that does not clash, here C (a whole tone over the B flat)
        let mut giver = vec![Some(69), Some(69)];
        assert_eq!(harmonise(&mut giver, &[Some(46), Some(46)], &C_MAJOR, (69, 73)), 2);
        assert_eq!(giver, [Some(72), Some(72)]);
        // lines of different lengths: the shorter one's slots are looked at; an empty one changes nothing
        let mut giver = vec![Some(61), Some(61)];
        assert_eq!(harmonise(&mut giver, &[Some(60)], &C_MAJOR, (40, 90)), 1);
        assert_eq!(giver, [Some(60), Some(61)]);
        assert_eq!(harmonise(&mut giver, &[], &C_MAJOR, (40, 90)), 0);
    }
}
