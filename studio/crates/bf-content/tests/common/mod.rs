//! What the soundtrack tests share: the example episode (the imported recording of FLUTE against TROMBONE,
//! seed 392, and its fight file) and the readers of the kit's files in `studio/reference/sound/flute_vs_trombone`
//! (`python -m tools.studio_export stems`).
//!
//! Each test file uses part of this, so the parts it does not use are allowed to be unused.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::LazyLock;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_sound::SR;
use bf_testdata::{Vectors, reference, repo_root};
use serde_json::Value;

/// A 100 ms block's RMS against the kit's: within this many dB.
pub const RMS_DB: f64 = 0.01;
/// Blocks the kit has below this many dBFS are only checked to be below it.
pub const RMS_FLOOR: f64 = -90.0;
/// 100 ms.
pub const BLOCK: usize = SR / 10;
/// The buses of the episode, in the order its score first touches them.
pub const BUSES: [&str; 7] = ["drums", "calliope", "fx", "flute", "trombone", "glock", "solo"];

/// The folder of the episode's sound references, under `studio/reference`.
pub const DIR: &str = "sound/flute_vs_trombone";

// ---------------------------------------------------------------- the episode

/// The example episode: its recording and its fight file.
pub struct Episode {
    pub rec: Recording,
    pub ff: FightFile,
}

/// The example episode, read from `studio/reference/fights/flute_vs_trombone/ep392.bfrec` and
/// `studio/fights/flute_vs_trombone.toml`.
pub fn episode() -> Episode {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    Episode { rec, ff }
}

/// The example episode, read once per test binary.
pub static EPISODE: LazyLock<Episode> = LazyLock::new(episode);

// ---------------------------------------------------------------- reading the files

pub fn json(rel: &str) -> Value {
    let path = reference(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `(i0, n)` of a window case.
pub fn span(args: &Value) -> (usize, usize) {
    let whole = |key: &str| {
        args[key]
            .as_u64()
            .unwrap_or_else(|| panic!("the case has no whole number {key:?}: {args}")) as usize
    };
    (whole("i0"), whole("n"))
}

/// A float32 `(n, 2)` reference window as float64 frames.
pub fn window(v: &Vectors, case: &bf_testdata::Case, bus: &str, n: usize) -> Vec<[f64; 2]> {
    let a = v.array(case, bus);
    assert_eq!(a.shape, [n, 2], "{} {bus}: shape", case.id);
    a.f32()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|f| [f64::from(f[0]), f64::from(f[1])])
        .collect()
}

/// The largest sample difference in dBFS between `got`, rounded to float32 as the reference was, and `want`;
/// negative infinity when every sample rounds to the reference's float32. And the same without the rounding.
pub fn window_db(got: &[[f64; 2]], want: &[[f64; 2]]) -> (f64, f64) {
    assert_eq!(got.len(), want.len(), "window lengths");
    let (mut rounded, mut raw) = (0.0f64, 0.0f64);
    for (g, w) in got.as_flattened().iter().zip(want.as_flattened()) {
        // A NaN is kept, so that it never passes.
        let d = (f64::from(*g as f32) - w).abs();
        if d > rounded || d.is_nan() {
            rounded = d;
        }
        let d = (g - w).abs();
        if d > raw || d.is_nan() {
            raw = d;
        }
    }
    (db(rounded), db(raw))
}

pub fn db(x: f64) -> f64 {
    if x == 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * bf_core::dmath::log10(x)
    }
}

pub fn show_db(v: f64) -> String {
    if v == f64::NEG_INFINITY {
        "-inf dBFS (the same float32)".to_string()
    } else {
        format!("{v:.1} dBFS")
    }
}

/// `rms.json`'s formula: `20 * log10(sqrt(mean(x ** 2)) + 1e-12)` over both channels of each 100 ms block; the
/// last block is what is left.
pub fn block_db(x: &[[f64; 2]]) -> Vec<f64> {
    x.chunks(BLOCK)
        .map(|block| {
            let flat = block.as_flattened();
            let mean = flat.iter().map(|v| v * v).sum::<f64>() / flat.len() as f64;
            20.0 * bf_core::dmath::log10(mean.sqrt() + 1e-12)
        })
        .collect()
}

pub fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap_or_else(|| panic!("{v} is not a list"))
        .iter()
        .map(|x| x.as_f64().unwrap_or_else(|| panic!("{x} is not a number")))
        .collect()
}

/// The buses `exceptions.toml` lists for `episode`, each with its reason.
pub fn exceptions(episode: &str) -> BTreeMap<String, String> {
    let path = reference("sound/exceptions.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: toml::Table = text.parse().unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let list = doc
        .get("exceptions")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{}: no list `exceptions`", path.display()));
    let mut out = BTreeMap::new();
    for entry in list {
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(toml::Value::as_str)
                .unwrap_or_else(|| panic!("{}: an exception without a text {key:?}: {entry}", path.display()))
                .to_string()
        };
        let reason = text("reason");
        assert!(
            reason.trim().len() >= 20,
            "{}: the reason for {} says nothing: {reason:?}",
            path.display(),
            text("bus")
        );
        if text("episode") == episode {
            out.insert(text("bus"), reason);
        }
    }
    out
}

/// Compares the 100 ms profile `got` with the kit's `want`; returns the largest difference in dB over the
/// blocks compared, and how many blocks were compared.
pub fn profile(what: &str, got: &[f64], want: &[f64], failures: &mut Vec<String>) -> (f64, usize) {
    if got.len() != want.len() {
        failures.push(format!("{what}: {} blocks, the kit has {}", got.len(), want.len()));
        return (f64::NAN, 0);
    }
    let (mut worst, mut compared) = (0.0f64, 0);
    for (b, (g, w)) in got.iter().zip(want).enumerate() {
        if *w < RMS_FLOOR {
            // Quiet in the kit: it must be quiet here too (a hair of slack for a block right at the floor).
            if g.is_nan() || *g >= RMS_FLOOR + RMS_DB {
                failures.push(format!(
                    "{what} block {b}: {g:.3} dB, the kit has {w:.3} dB (below {RMS_FLOOR})"
                ));
            }
            continue;
        }
        compared += 1;
        let d = (g - w).abs();
        if d > worst || d.is_nan() {
            worst = d;
        }
        if d.is_nan() || d >= RMS_DB {
            failures.push(format!(
                "{what} block {b} ({:.1} s): {g:.4} dB, the kit has {w:.4} dB",
                b as f64 / 10.0
            ));
        }
    }
    (worst, compared)
}

/// `audio.wav` as the kit wrote it: 16 bit, 48 kHz, stereo, the canonical 44-byte header.
pub fn kit_wav() -> Vec<[i16; 2]> {
    let path = reference(&format!("{DIR}/audio.wav"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    assert!(bytes.len() > 44, "{}: too short for a wav", path.display());
    assert_eq!(
        (&bytes[0..4], &bytes[8..16], &bytes[36..40]),
        (&b"RIFF"[..], &b"WAVEfmt "[..], &b"data"[..]),
        "{}: not a canonical wav header",
        path.display()
    );
    // PCM, 2 channels, 48 kHz, 16 bit.
    assert_eq!((u16_at(20), u16_at(22), u32_at(24), u16_at(34)), (1, 2, SR as u32, 16));
    let data = &bytes[44..];
    assert_eq!(u32_at(40) as usize, data.len(), "the data chunk's length");
    data.as_chunks::<4>()
        .0
        .iter()
        .map(|f| [i16::from_le_bytes([f[0], f[1]]), i16::from_le_bytes([f[2], f[3]])])
        .collect()
}

pub fn rms_i16(x: &[[i16; 2]]) -> f64 {
    let flat = x.as_flattened();
    let sum: f64 = flat.iter().map(|&v| f64::from(v) * f64::from(v)).sum();
    db((sum / flat.len() as f64).sqrt() / 32767.0)
}
