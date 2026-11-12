//! The instrument voice of Template F (`fighters/<slug>/voice.json`): `pp_bridge.voice` at 4 pitches across its
//! register x 2 velocities x 3 lengths, each note from the shared noise state the one before left. Milestone 1's
//! `voices.rs` bar: every sample within -120 dBFS of the kit's (1e-6), here as the note's length (exactly), its
//! peak and the RMS of each 10 ms block (within 1e-6), and the noise stream where the kit's stood after it; the
//! whole notes against the cache.

use bf_core::rng::Generator;
use bf_sound::voice::{PpNoise, Voice};
use bf_testdata::Npz;
use serde_json::Value;

use crate::refs::{fighter_folder, hexf, index, json, list, num, report, text};

/// A sample, a peak or a block RMS: within this of the kit's (-120 dBFS).
pub const BAR: f64 = 1e-6;

fn big(v: &Value) -> u128 {
    text(v)
        .parse()
        .unwrap_or_else(|_| panic!("{v} is not a 128-bit number"))
}

fn stands_at(g: &Generator, state: u128) -> bool {
    let (mut a, mut b) = (g.clone(), Generator::from_state(state, PpNoise::START_INC));
    (0..4).all(|_| a.random().to_bits() == b.random().to_bits())
}

fn voice_of(name: &str) -> std::sync::Arc<dyn Voice> {
    let entry = bf_content::fighters::entry(name).unwrap_or_else(|| panic!("{name} has no fighter folder"));
    (entry.voice)().unwrap_or_else(|| panic!("{name}'s voice() is None: port its voice"))
}

fn doc(name: &str) -> Value {
    let d = json(&fighter_folder(name), "voice.json");
    assert_eq!(text(&d["fighter"]), name);
    d
}

/// Each note of `voice.json` from its stored noise state: its length, peak, 10 ms block RMS and the noise state
/// after it are the kit's. Always runs.
pub fn voice(name: &str) {
    let folder = fighter_folder(name);
    let d = doc(name);
    let v = voice_of(name);
    let (lo, hi) = v.register();
    let reg: Vec<u64> = list(&d["register"]).iter().map(|x| x.as_u64().unwrap()).collect();
    assert_eq!(
        reg,
        [u64::from(lo as u32), u64::from(hi as u32)],
        "{name}: the register of pp_bridge.entrant"
    );
    assert_eq!(
        PpNoise::START_STATE,
        big(&d["start"]["state"]),
        "{name}: the noise stream's start"
    );
    let block = index(&d["block"]);
    let notes = list(&d["notes"]);
    assert_eq!(
        notes.len(),
        4 * list(&d["velocities"]).len() * list(&d["durations"]).len(),
        "{folder}: the notes"
    );
    let (mut failures, mut same_sha) = (Vec::new(), 0);
    let mut noise = PpNoise::start();
    for n in notes {
        let id = text(&n["id"]).to_string();
        assert!(
            stands_at(&noise.0, big(&n["noise_before"])),
            "{name} {id}: the noise is not where the kit's stood"
        );
        let (midi, vel, dur) = (n["midi"].as_i64().unwrap() as f64, hexf(&n["vel"]), hexf(&n["dur"]));
        let x = v.note(&mut noise, midi, vel, dur);
        let what = format!("{id} ({midi} vel {vel} dur {dur})");
        if x.len() != index(&n["samples"]) {
            failures.push((what, format!("{} samples, the kit {}", x.len(), index(&n["samples"]))));
            continue;
        }
        let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let mut worst = (peak - hexf(&n["peak"])).abs();
        for (b, want) in x.chunks(block).zip(list(&n["rms"])) {
            let rms = (b.iter().map(|v| v * v).sum::<f64>() / b.len() as f64).sqrt();
            worst = worst.max((rms - num(want)).abs());
        }
        if worst.is_nan() || worst > BAR {
            failures.push((what.clone(), format!("{worst:e} from the kit's (bar {BAR})")));
        }
        if !stands_at(&noise.0, big(&n["noise_after"])) {
            failures.push((what.clone(), "drew another number of normals than the kit".into()));
        }
        let bytes: Vec<u8> = x.iter().flat_map(|v| v.to_le_bytes()).collect();
        same_sha += usize::from(crate::refs::sha(&bytes) == text(&n["sha256"]));
    }
    println!(
        "{name}: {same_sha} of {} notes are the kit's float64 bits to the last",
        notes.len()
    );
    report(&folder, "voice", "notes", failures, notes.len());
}

/// Every note sample for sample against the cache's float64 notes, within -120 dBFS. Ignored: reads the cache.
pub fn voice_notes(name: &str) {
    let folder = fighter_folder(name);
    let d = doc(name);
    let v = voice_of(name);
    let npz = Npz::open(&bf_testdata::cache_file(text(&d["cache"])));
    let mut noise = PpNoise::start();
    let (mut failures, mut worst_all) = (Vec::new(), 0.0f64);
    for n in list(&d["notes"]) {
        let id = text(&n["id"]);
        let x = v.note(
            &mut noise,
            n["midi"].as_i64().unwrap() as f64,
            hexf(&n["vel"]),
            hexf(&n["dur"]),
        );
        let want = npz.get(id).f64();
        let worst = if x.len() == want.len() {
            x.iter().zip(want).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max)
        } else {
            f64::INFINITY
        };
        worst_all = worst_all.max(worst);
        if worst.is_nan() || worst > BAR {
            failures.push((id.to_string(), format!("{worst:e} (bar {BAR})")));
        }
    }
    println!("{name}: the largest sample difference {worst_all:e}");
    report(&folder, "voice", "whole notes", failures, list(&d["notes"]).len());
}
