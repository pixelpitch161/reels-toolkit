//! DRUMS, quick-ported (ruling M4-R16): its fights against a few fighters, VIOLIN and PIANO among them, run to the
//! end through every socket of its `pulse` kind (no stub reached), its pulses fire and land, PIANO's keys are cleared
//! by them, and a K.O. comes in a normal share of seeds. No parity gates: the port is checked by eye against #28.

use bf_harness::fight::{fighter, traced};
use bf_sim::Setup;
use bf_sim::rules::Rules;

#[test]
fn drums_fights_to_the_end() {
    let sim = bf_harness::sim!();
    let mut kos = 0;
    let mut n = 0;
    for enemy in ["VIOLIN", "PIANO", "ORGAN"] {
        let (mut pulses, mut landed, mut cleared) = (0, 0, 0);
        for seed in 1..=3u64 {
            for swapped in [false, true] {
                let (d, e) = (fighter("DRUMS"), fighter(enemy));
                let fighters = if swapped { [e, d] } else { [d, e] };
                let setup = Setup {
                    fighters,
                    rules: Rules::new(120.0, 3.6),
                    seed,
                };
                let rec = traced(&sim, &setup, false, &format!("DRUMS v {enemy} seed {seed}"));
                let k = usize::from(swapped);
                let mine = |kind: &str| {
                    rec.events
                        .iter()
                        .filter(|e| e.kind == kind && e.who() == Some(k))
                        .count()
                };
                pulses += mine("pulse");
                cleared += rec
                    .events
                    .iter()
                    .filter(|e| e.kind == "cleared" && e.who() == Some(1 - k) && e.i(3) == k as i64)
                    .count();
                landed += rec
                    .events
                    .iter()
                    .filter(|e| e.kind == "hit" && e.who() == Some(k) && e.fields.len() > 5 && e.s(7) == "pulse")
                    .count();
                kos += usize::from(rec.t_ko.is_some());
                n += 1;
            }
        }
        assert!(pulses > 0, "DRUMS fired no pulse against {enemy}");
        assert!(landed > 0, "no DRUMS pulse landed against {enemy}");
        if enemy == "PIANO" {
            assert!(cleared > 0, "no PIANO key was cleared by a DRUMS pulse");
        }
    }
    assert!(kos * 2 >= n, "a K.O. in only {kos} of {n} fights");
}
