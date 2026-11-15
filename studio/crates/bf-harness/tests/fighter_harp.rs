//! HARP, quick-ported (ruling M4-R16): its fights against a few fighters, ORGAN among them, run to the end through
//! every socket of its `trap` kind (no stub reached), its strings and their crossings happen, and a K.O. comes in a
//! normal share of seeds. No parity gates: the port is checked by eye against #35.

use bf_harness::fight::{fighter, traced};
use bf_sim::Setup;
use bf_sim::rules::Rules;

#[test]
fn harp_fights_to_the_end() {
    let sim = bf_harness::sim!();
    let mut kos = 0;
    let mut n = 0;
    for enemy in ["ORGAN", "FLUTE", "CONTRABASS", "BAGPIPES"] {
        let (mut strings, mut crossings) = (0, 0);
        for seed in 1..=3u64 {
            for swapped in [false, true] {
                let (h, e) = (fighter("HARP"), fighter(enemy));
                let fighters = if swapped { [e, h] } else { [h, e] };
                let setup = Setup {
                    fighters,
                    rules: Rules::new(120.0, 3.6),
                    seed,
                };
                let rec = traced(&sim, &setup, false, &format!("HARP v {enemy} seed {seed}"));
                let k = usize::from(swapped);
                strings += rec
                    .events
                    .iter()
                    .filter(|e| e.kind == "string" && e.who() == Some(k))
                    .count();
                crossings += rec
                    .events
                    .iter()
                    .filter(|e| e.kind == "hit" && e.who() == Some(k) && e.fields.len() > 5 && e.s(7) == "string")
                    .count();
                kos += usize::from(rec.t_ko.is_some());
                n += 1;
            }
        }
        assert!(strings > 0, "HARP laid no string against {enemy}");
        assert!(crossings > 0, "no enemy crossed a HARP string against {enemy}");
    }
    assert!(kos * 2 >= n, "a K.O. in only {kos} of {n} fights");
}
