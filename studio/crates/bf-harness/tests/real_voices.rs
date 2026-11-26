//! The sound upgrade's stage 3 (S1): the reworked voices against recordings of the real instruments (FluidR3's
//! fingerprints, `tests/reference/fluidr3.json`, made on the owner's PC by the `references` example).
//!
//! Saxes first: each sax, on every note of its register, is nearer to its own real instrument than to the real
//! brass it could be taken for, and within [`AT_MOST`] of its own (the fingerprint distance,
//! [`bf_harness::timbre::distance`]).

use std::collections::BTreeMap;

use bf_harness::timbre::{self, Fingerprint};
use bf_sound::voice::pp;
use serde_json::Value;

/// The farthest a reworked voice's note may be from its real instrument's, in dB (S1).
const AT_MOST: f64 = 6.0;

/// Each reference's fingerprints, by MIDI note.
fn references() -> BTreeMap<String, BTreeMap<i32, Fingerprint>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reference/fluidr3.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("the fingerprints")).expect("JSON");
    v["references"]
        .as_object()
        .expect("the references")
        .iter()
        .map(|(name, r)| {
            let notes = r["notes"]
                .as_array()
                .expect("its notes")
                .iter()
                .filter_map(Fingerprint::from_json)
                .map(|f| (f.midi, f))
                .collect();
            (name.clone(), notes)
        })
        .collect()
}

/// A built note's fingerprint: `tone` (at 44.1 kHz) for one note of 1 s.
fn built(midi: i32, tone: impl Fn(f64) -> Vec<f64>) -> Fingerprint {
    let x = pp::to_48k(&tone(pp::midi_hz(f64::from(midi))));
    timbre::fingerprint(&x, midi, 1.0, true)
}

/// Every note from `lo` to `hi` of `tone` is nearer to reference `own` than to each of `rivals`, and within
/// [`AT_MOST`] of `own`.
fn nearer_its_own(name: &str, (lo, hi): (i32, i32), own: &str, rivals: &[&str], tone: impl Fn(f64) -> Vec<f64>) {
    let refs = references();
    let mut bad = Vec::new();
    for m in lo..=hi {
        let b = built(m, &tone);
        let d_own = timbre::distance(&b.harmonics, &refs[own][&m].harmonics);
        for r in rivals {
            let d = timbre::distance(&b.harmonics, &refs[*r][&m].harmonics);
            if d <= d_own {
                bad.push(format!("{m}: {d_own:.1} to the real {own}, {d:.1} to the real {r}"));
            }
        }
        if d_own > AT_MOST {
            bad.push(format!("{m}: {d_own:.1} to the real {own}, over {AT_MOST}"));
        }
    }
    assert!(bad.is_empty(), "{name}:\n{}", bad.join("\n"));
}

#[test]
fn the_bari_sax_is_a_bari_sax_not_a_trombone() {
    use bf_content::fighters::bari_sax::voice::{REGISTER, bari_tone_real};
    nearer_its_own("BARI SAX", REGISTER, "bari sax", &["trombone", "tuba"], |f| {
        bari_tone_real(f, 1.0, 0.8, 0.0, 0)
    });
}
