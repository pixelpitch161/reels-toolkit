//! The sound upgrade's stage 3 (S1): the reworked voices against recordings of the real instruments (FluidR3's
//! fingerprints, `tests/reference/fluidr3.json`, made on the owner's PC by the `references` example).
//!
//! Saxes first (bari, alto, soprano, tenor), then the low instruments (tuba, sousaphone, euphonium, trombone,
//! octobass, contrabass): each, on every note of its register, is nearer to its own real instrument than to the real
//! brass it could be taken for, and within [`AT_MOST`] of its own (the fingerprint distance,
//! [`bf_harness::timbre::distance`]).

use std::collections::BTreeMap;

use bf_core::rng::Generator;
use bf_harness::timbre::{self, Fingerprint};
use bf_sound::voice::pp;
use bf_sound::voice::real::midi_of;
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
                // FluidR3 has no sample for some notes (the tuba above 72, the contrabass above 57): silence
                .filter(|f| f.harmonics.iter().any(|&h| h > -200.0))
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

/// Every note of `fighter`'s register that its real instrument `own` has a reference for, as the Composed arrangement
/// plays it ([`Voice::note_real`](bf_sound::voice::Voice::note_real)), is nearer to `own` than to each of `rivals`
/// and within [`AT_MOST`] of it.
fn reworked_nearer_its_own(fighter: &str, own: &str, rivals: &[&str]) {
    let v = bf_content::fighters::voice(fighter).expect("a voice");
    assert!(v.reworked(), "{fighter} is not reworked");
    nearer_at_48k(fighter, v.register(), own, rivals, |m| {
        v.note_real(&mut bf_sound::voice::PpNoise::start(), f64::from(m), 0.8, 1.0, 0)
    });
}

/// Every note from `lo` to `hi` that `own` has a reference for, held a second at 48 kHz by `note` (from its MIDI
/// note), is nearer to `own` than to each of `rivals` (where they have the note), and within [`AT_MOST`] of `own`.
fn nearer_at_48k(fighter: &str, (lo, hi): (i32, i32), own: &str, rivals: &[&str], note: impl Fn(i32) -> Vec<f64>) {
    let refs = references();
    let mut bad = Vec::new();
    let mut seen = 0;
    for m in lo..=hi {
        let Some(real) = refs[own].get(&m) else { continue };
        seen += 1;
        let x = note(m);
        let b = timbre::fingerprint(&x, m, 1.0, true);
        let d_own = timbre::distance(&b.harmonics, &real.harmonics);
        for r in rivals {
            let Some(rr) = refs[*r].get(&m) else { continue };
            let d = timbre::distance(&b.harmonics, &rr.harmonics);
            if d <= d_own {
                bad.push(format!("{m}: {d_own:.1} to the real {own}, {d:.1} to the real {r}"));
            }
        }
        if d_own > AT_MOST {
            bad.push(format!("{m}: {d_own:.1} to the real {own}, over {AT_MOST}"));
        }
    }
    assert!(seen >= 10, "{fighter}: only {seen} notes have a reference");
    assert!(bad.is_empty(), "{fighter}:\n{}", bad.join("\n"));
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

#[test]
fn the_alto_sax_is_an_alto_sax_not_brass() {
    use bf_content::fighters::alto_sax::voice::{REGISTER, TONE, tone_real};
    nearer_its_own(
        "ALTO SAX",
        REGISTER,
        "alto sax",
        &["trumpet", "french horn", "trombone"],
        |f| tone_real(&mut Generator::new(23), midi_of(f), 1.0, 0.8, &TONE, 0),
    );
}

#[test]
fn the_soprano_sax_is_a_soprano_sax_not_a_trumpet() {
    use bf_content::fighters::soprano_sax::voice::{REGISTER, TONE, tone_real};
    nearer_its_own(
        "SOPRANO SAX",
        REGISTER,
        "soprano sax",
        &["trumpet", "muted trumpet", "french horn"],
        |f| tone_real(&mut Generator::new(23), midi_of(f), 1.0, 0.8, &TONE, 0),
    );
}

#[test]
fn the_tenor_sax_is_a_tenor_sax_not_brass() {
    use bf_content::fighters::tenor_sax::voice::{REGISTER, VOICE_HONK, tenor_tone_real};
    nearer_its_own(
        "TENOR SAX",
        REGISTER,
        "tenor sax",
        &["trumpet", "french horn", "trombone"],
        |f| tenor_tone_real(f, 1.0, 0.8, VOICE_HONK, 0.0, 0),
    );
}

#[test]
fn the_tuba_family_is_tubas_not_trombones_or_saxes() {
    for fighter in ["TUBA", "SOUSAPHONE", "EUPHONIUM"] {
        reworked_nearer_its_own(fighter, "tuba", &["trombone", "bari sax", "bassoon", "french horn"]);
    }
}

#[test]
fn the_trombone_is_a_trombone_not_a_tuba_or_a_sax() {
    reworked_nearer_its_own(
        "TROMBONE",
        "trombone",
        &["tuba", "bari sax", "french horn", "tenor sax"],
    );
}

#[test]
fn the_big_strings_are_contrabasses_not_cellos_or_bass_guitars() {
    for fighter in ["OCTOBASS", "CONTRABASS"] {
        reworked_nearer_its_own(fighter, "contrabass", &["cello", "finger bass", "tuba"]);
    }
}

#[test]
fn the_flutes_and_the_oboe_are_their_own_not_each_other() {
    reworked_nearer_its_own("PICCOLO", "piccolo", &["flute", "oboe", "clarinet"]);
    reworked_nearer_its_own("FLUTE", "flute", &["piccolo", "oboe", "clarinet", "soprano sax"]);
    reworked_nearer_its_own("OBOE", "oboe", &["soprano sax", "clarinet", "trumpet", "flute"]);
}

#[test]
fn the_clarinets_are_clarinets_not_bassoons_or_horns() {
    reworked_nearer_its_own("CLARINET", "clarinet", &["oboe", "french horn", "bassoon", "flute"]);
    for fighter in ["BASS CLARINET", "CONTRABASS CLARINET"] {
        reworked_nearer_its_own(fighter, "clarinet", &["bassoon", "tuba", "french horn"]);
    }
}

#[test]
fn the_bagpipe_chanter_is_a_bagpipe_not_a_trumpet() {
    use bf_content::fighters::bagpipes::{sound::chanter_real, voice::REGISTER};
    nearer_at_48k(
        "BAGPIPES",
        REGISTER,
        "bagpipe",
        &["trumpet", "muted trumpet", "oboe"],
        |m| chanter_real(f64::from(m), 1.0, 0.8, false, 0),
    );
}

#[test]
fn the_trumpets_are_trumpets_not_horns_or_trombones() {
    // the cornet and the flugelhorn are aimed mellower than the trumpet (FluidR3 has neither), and stay nearer it
    reworked_nearer_its_own(
        "TRUMPET",
        "trumpet",
        &["trombone", "french horn", "alto sax", "soprano sax"],
    );
    for fighter in ["CORNET", "FLUGELHORN"] {
        reworked_nearer_its_own(fighter, "trumpet", &["french horn", "trombone", "alto sax"]);
    }
}

#[test]
fn the_horns_are_horns_not_trumpets_or_trombones() {
    reworked_nearer_its_own("HORN", "french horn", &["trombone", "trumpet", "alto sax", "bassoon"]);
    reworked_nearer_its_own("MELLOPHONE", "french horn", &["trumpet", "trombone", "alto sax"]);
}

#[test]
fn the_bowed_strings_are_their_own_not_each_other() {
    reworked_nearer_its_own("VIOLIN", "violin", &["viola", "cello", "french horn", "trumpet"]);
    reworked_nearer_its_own("VIOLA", "viola", &["violin", "cello", "french horn", "english horn"]);
    reworked_nearer_its_own("CELLO", "cello", &["viola", "contrabass", "french horn", "bassoon"]);
}

#[test]
fn the_drums_repeats_differ_and_keep_their_length() {
    for fighter in ["TIMPANI", "BASS DRUM"] {
        let v = bf_content::fighters::voice(fighter).expect("a voice");
        assert!(v.reworked(), "{fighter}");
        let (lo, hi) = v.register();
        let m = f64::from((lo + hi) / 2);
        let note = |variant| v.note_real(&mut bf_sound::voice::PpNoise::start(), m, 0.8, 0.5, variant);
        let (a, b) = (note(0), note(1));
        assert_eq!(
            a,
            v.note(&mut bf_sound::voice::PpNoise::start(), m, 0.8, 0.5),
            "{fighter}"
        );
        assert_eq!(a.len(), b.len(), "{fighter}");
        assert_ne!(a, b, "{fighter}");
        assert!(b.iter().all(|x| x.is_finite()), "{fighter}");
    }
}
