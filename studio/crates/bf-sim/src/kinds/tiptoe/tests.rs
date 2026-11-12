//! The tiptoe's own cases: `_behind`, the setup, and whole fights of BASSOON against FLUTE read for what the kind
//! promises (every hop lands as a stomp on the grid, the caps, the record, MOUNTAIN KING's sequence).

use super::*;
use crate::fight::{Setup, run};
use crate::testing;

/// BASSOON (left) against FLUTE at the reference handicap of its folder, at the episode's tempo.
fn bassoon_fight(seed: u64) -> bf_core::recording::Recording {
    let roster = testing::roster();
    let mut rules = testing::episode_rules();
    rules.dmg_side = Some([0.41, 0.6]);
    let setup = Setup {
        fighters: [testing::fighter(&roster, "BASSOON"), testing::fighter(&roster, "FLUTE")],
        rules,
        seed,
    };
    run(&setup, true).expect("the engine runs BASSOON")
}

#[test]
fn behind_is_against_the_enemys_travel_or_toward_the_hopper() {
    let roster = testing::roster();
    let setup = Setup {
        fighters: [testing::fighter(&roster, "BASSOON"), testing::fighter(&roster, "FLUTE")],
        rules: testing::episode_rules(),
        seed: 1,
    };
    let mut f = Fight::new(&setup, false).expect("the fight");
    f.balls[0].p = [1.0, 2.0];
    f.balls[1].p = [1.0, -1.0];
    f.balls[1].v = [3.0, 4.0];
    assert_eq!(behind(&f, 0), [-3.0 / 5.0, -4.0 / 5.0]);
    // an enemy standing still: from it to the hopper
    f.balls[1].v = [0.0, 0.0];
    assert_eq!(behind(&f, 0), [0.0, 1.0]);
    // both in one place
    f.balls[1].p = [1.0, 2.0];
    assert_eq!(behind(&f, 0), [1.0, 0.0]);
    // seen from the other ball, the enemy is the hopper
    f.balls[0].v = [0.0, -2.0];
    assert_eq!(behind(&f, 1), [0.0, 1.0]);
}

#[test]
fn setup_makes_a_hopper_with_its_starting_reach_and_radius() {
    let roster = testing::roster();
    let setup = Setup {
        fighters: [testing::fighter(&roster, "FLUTE"), testing::fighter(&roster, "BASSOON")],
        rules: testing::episode_rules(),
        seed: 1,
    };
    let mut f = Fight::new(&setup, true).expect("the fight");
    assert!(!is(&f, 0) && is(&f, 1));
    assert_eq!(record_keys(&f), &["HOP"]);
    super::setup(&mut f, 1);
    assert!(f.balls[1].hopper && !f.balls[1].air && !f.balls[0].hopper);
    assert_eq!((f.balls[1].blade_len, f.tiptoe.stomp_r[1]), (1.6, 0.2));
    assert_eq!(f.tiptoe.hop, [None, None]);
}

/// Every hop lands as a stomp of its kind at its landing time, an ordinary hop is two 16ths long (one under
/// STACCATO), a follow-up one, the reach and the radius grow only to their caps, and the record follows the hops.
#[test]
fn hops_land_as_stomps_on_the_grid() {
    for seed in [1, 2, 9] {
        let rec = bassoon_fight(seed);
        let rules = testing::episode_rules();
        let s16 = rules.beat / 4.0;
        let hops: Vec<_> = rec.events.iter().filter(|e| e.kind == "hop").collect();
        let stomps: Vec<_> = rec.events.iter().filter(|e| e.kind == "stomp").collect();
        assert!(hops.len() > 40, "seed {seed}: {} hops", hops.len());
        assert!(
            stomps.len() == hops.len() || stomps.len() + 1 == hops.len(),
            "seed {seed}"
        );
        for (h, s) in hops.iter().zip(&stomps) {
            assert_eq!((h.who(), s.who()), (Some(0), Some(0)));
            let (t_land, kind) = (h.f(7), h.i(8));
            assert!(
                (s.t - t_land).abs() < rules.dt + 2e-5,
                "seed {seed}: the hop of {} s lands at {} s",
                h.t,
                s.t
            );
            assert_eq!(s.i(8), kind);
            let air16 = (t_land - h.t) / s16;
            match kind {
                0 => assert!((air16 - 2.0).abs() < 1e-3 || (air16 - 1.0).abs() < 1e-3, "{air16}"),
                1 => assert!((air16 - 1.0).abs() < 1e-3, "{air16}"),
                2 => assert!((1.0 - 1e-3..=2.0 + 1e-3).contains(&air16), "{air16}"),
                other => panic!("a hop of kind {other}"),
            }
            // inside the ropes
            let lim = rules.a - rules.r;
            assert!(h.f(5).abs() <= lim && h.f(6).abs() <= lim);
            // the stomp radius stays between its start and its cap, and a backstab is a hit
            assert!((0.2..=0.5).contains(&s.f(7)));
            assert!(s.i(6) == 0 || s.i(5) == 1);
        }
        // a `stomp` hit carries the backstab flag of its stomp event
        for e in rec.events.iter().filter(|e| e.kind == "hit" && e.s(7) == "stomp") {
            let at = stomps.iter().find(|s| s.t == e.t).expect("its stomp");
            assert_eq!((at.i(5), at.i(6)), (1, e.i(8)));
        }
        // the reach in `BL` only grows, from 1.6 to at most 3.0
        let bl = testing::f64s(&rec, "BL");
        let mine: Vec<f64> = bl.iter().step_by(2).copied().collect();
        assert_eq!(mine[0], 1.6);
        assert!(mine.windows(2).all(|w| w[1] >= w[0]) && mine.iter().all(|&b| b <= 3.0));
        // HOP: -1 on the ground, else a progress in 0..1 between its two times; FLUTE's row is always -1
        let hop = testing::f64s(&rec, "HOP");
        assert_eq!(hop.len(), mine.len() * 6);
        let mut in_air = 0;
        for row in hop.chunks(6) {
            assert_eq!(&row[3..], &[-1.0; 3]);
            if row[0] == -1.0 {
                assert_eq!(&row[..3], &[-1.0; 3]);
            } else {
                in_air += 1;
                assert!((0.0..=1.0).contains(&row[0]) && row[1] < row[2], "{row:?}");
            }
        }
        assert!(in_air > 100, "seed {seed}: {in_air} frames in the air");
    }
}

/// MOUNTAIN KING: `mountain` says 20 hops; the hops of kind 2 follow no closer than the sequence's spacing (3, 3,
/// 3, 2, ... 16ths), no ordinary hop starts between them, and `mountain_end` comes only after all 20.
#[test]
fn the_mountain_king_dances_its_sequence() {
    let rules = testing::episode_rules();
    let s16 = rules.beat / 4.0;
    let seq = [3, 3, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    let mut danced = 0;
    for seed in [1, 2, 3, 9, 500] {
        let rec = bassoon_fight(seed);
        let Some(start) = rec.events.iter().find(|e| e.kind == "mountain") else {
            continue;
        };
        assert_eq!(start.i(3), 20);
        let hops: Vec<_> = rec.events.iter().filter(|e| e.kind == "hop" && e.i(8) == 2).collect();
        let ended = rec.events.iter().any(|e| e.kind == "mountain_end");
        assert!(
            hops.len() <= 20 && (hops.len() == 20 || !ended),
            "seed {seed}: {} hops",
            hops.len()
        );
        for (i, pair) in hops.windows(2).enumerate() {
            let gap = (pair[1].t - pair[0].t) / s16;
            // a launch falls in the step its 16th comes; a stun or a hop still in the air can only delay it
            assert!(
                gap > f64::from(seq[i]) - 2.0 * rules.dt / s16,
                "seed {seed}: hop {i} after {gap} 16ths"
            );
        }
        if let (Some(first), Some(last)) = (hops.first(), hops.last()) {
            let between = rec
                .events
                .iter()
                .any(|e| e.kind == "hop" && e.i(8) != 2 && first.t < e.t && e.t < last.t);
            assert!(!between, "seed {seed}: an ordinary hop during the dance");
        }
        danced += 1;
    }
    assert!(danced >= 3, "MOUNTAIN KING fired in {danced} of 5 fights");
}
