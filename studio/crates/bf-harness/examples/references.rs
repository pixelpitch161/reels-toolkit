//! The real-instrument references of the sound upgrade's stage 3 (S1), and the built voices' fingerprints beside them.
//!
//! ```text
//! cargo run --release -p bf-harness --example references -- render <fluidsynth> <FluidR3_GM.sf2> <work dir> <out.json>
//! cargo run --release -p bf-harness --example references -- built <out.json>
//! ```
//!
//! `render` plays every reference instrument (a General MIDI program of FluidR3, or a drum of its kit) through
//! fluidsynth, one note of 1 s at velocity 100 every 2.5 s across the instrument's range, and writes each note's
//! fingerprint ([`bf_harness::timbre`]). The recordings stay in the work folder: they are a development measure
//! only and are never committed (the fingerprints are). `built` writes the same fingerprints of each fighter's built
//! voice, one note of 1 s at velocity 0.8 at every note of its register.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use bf_harness::timbre::{self, SR};
use bf_sound::voice::PpNoise;
use serde_json::{Value, json};

/// A reference instrument: its name, the General MIDI program (`None` for the drum kit, channel 10) and the notes
/// it plays.
struct Reference {
    name: &'static str,
    program: Option<u8>,
    notes: Vec<i32>,
}

/// Each fighter's reference (S1's list: the instrument itself where FluidR3 has it, else its nearest relative), as
/// (fighter, reference name). Turntables and synth have no real counterpart.
const MAP: [(&str, &str); 40] = [
    ("ACCORDION", "accordion"),
    ("ACOUSTIC", "steel guitar"),
    ("ALTO SAX", "alto sax"),
    ("BAGPIPES", "bagpipe"),
    ("BARI SAX", "bari sax"),
    ("BASS CLARINET", "clarinet"),
    ("BASS DRUM", "drum kit"),
    ("BASS GUITAR", "finger bass"),
    ("BASSOON", "bassoon"),
    ("CELLO", "cello"),
    ("CLARINET", "clarinet"),
    ("CONTRABASS", "contrabass"),
    ("CONTRABASS CLARINET", "clarinet"),
    ("CONTRABASSOON", "bassoon"),
    ("CORNET", "trumpet"),
    ("DRUMS", "drum kit"),
    ("ELECTRIC", "overdriven guitar"),
    ("EUPHONIUM", "tuba"),
    ("FLUGELHORN", "trumpet"),
    ("FLUTE", "flute"),
    ("HARP", "harp"),
    ("HORN", "french horn"),
    ("MARIMBA", "marimba"),
    ("MELLOPHONE", "french horn"),
    ("OBOE", "oboe"),
    ("OCTOBASS", "contrabass"),
    ("ORGAN", "church organ"),
    ("PIANO", "piano"),
    ("PICCOLO", "piccolo"),
    ("SNARE DRUM", "drum kit"),
    ("SOPRANO SAX", "soprano sax"),
    ("SOUSAPHONE", "tuba"),
    ("TENOR SAX", "tenor sax"),
    ("TIMPANI", "timpani"),
    ("TROMBONE", "trombone"),
    ("TRUMPET", "trumpet"),
    ("TUBA", "tuba"),
    ("UKULELE", "nylon guitar"),
    ("VIOLA", "viola"),
    ("VIOLIN", "violin"),
];

/// The reference instruments and their General MIDI programs (0-based); the families each sax and brass voice is
/// also compared against.
const PROGRAMS: [(&str, Option<u8>); 33] = [
    ("piano", Some(0)),
    ("marimba", Some(12)),
    ("drawbar organ", Some(16)),
    ("church organ", Some(19)),
    ("accordion", Some(21)),
    ("nylon guitar", Some(24)),
    ("steel guitar", Some(25)),
    ("clean guitar", Some(27)),
    ("overdriven guitar", Some(29)),
    ("finger bass", Some(33)),
    ("violin", Some(40)),
    ("viola", Some(41)),
    ("cello", Some(42)),
    ("contrabass", Some(43)),
    ("harp", Some(46)),
    ("timpani", Some(47)),
    ("trumpet", Some(56)),
    ("trombone", Some(57)),
    ("tuba", Some(58)),
    ("muted trumpet", Some(59)),
    ("french horn", Some(60)),
    ("brass section", Some(61)),
    ("soprano sax", Some(64)),
    ("alto sax", Some(65)),
    ("tenor sax", Some(66)),
    ("bari sax", Some(67)),
    ("oboe", Some(68)),
    ("english horn", Some(69)),
    ("bassoon", Some(70)),
    ("clarinet", Some(71)),
    ("piccolo", Some(72)),
    ("flute", Some(73)),
    ("bagpipe", Some(109)),
];

/// The drum kit's notes (channel 10): kicks, snares, toms, hats, crash.
const KIT: [i32; 10] = [35, 36, 38, 40, 41, 45, 48, 42, 46, 49];
/// Every reed and brass reference also covers the saxes' and the brass's whole range, so each sax can be measured
/// against the brass at its own notes (and the other way round).
const WIND_SPAN: (i32, i32) = (28, 88);
/// How far past its fighters' registers a reference plays.
const MARGIN: i32 = 7;
/// Seconds between notes, a note's length, and its velocity.
const STEP: f64 = 2.5;
const HELD: f64 = 1.0;
const VELOCITY: u8 = 100;

fn references() -> Vec<Reference> {
    let winds = [
        "trumpet",
        "trombone",
        "tuba",
        "muted trumpet",
        "french horn",
        "brass section",
        "soprano sax",
        "alto sax",
        "tenor sax",
        "bari sax",
        "clarinet",
        "bassoon",
        "oboe",
        "english horn",
    ];
    PROGRAMS
        .iter()
        .map(|&(name, program)| {
            let mut lo = i32::MAX;
            let mut hi = i32::MIN;
            for (fighter, r) in MAP {
                if r == name {
                    let reg = bf_content::fighters::voice(fighter)
                        .unwrap_or_else(|| panic!("{fighter} has no voice"))
                        .register();
                    lo = lo.min(reg.0 - MARGIN);
                    hi = hi.max(reg.1 + MARGIN);
                }
            }
            if winds.contains(&name) {
                lo = lo.min(WIND_SPAN.0);
                hi = hi.max(WIND_SPAN.1);
            }
            if lo > hi {
                (lo, hi) = (48, 72);
            }
            Reference {
                name,
                program,
                notes: (lo.max(21)..=hi.min(108)).collect(),
            }
        })
        .chain(std::iter::once(Reference {
            name: "drum kit",
            program: None,
            notes: KIT.to_vec(),
        }))
        .collect()
}

/// A standard MIDI file (format 0, 960 ticks a beat at 120 BPM) playing `notes` one every [`STEP`] seconds.
fn midi_file(program: Option<u8>, notes: &[i32]) -> Vec<u8> {
    let ch: u8 = if program.is_some() { 0 } else { 9 };
    let tick = |s: f64| (s * 1920.0).round() as u32;
    let mut ev: Vec<(u32, Vec<u8>)> = vec![(0, vec![0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20])];
    if let Some(p) = program {
        ev.push((0, vec![0xC0 | ch, p]));
    }
    for (i, &m) in notes.iter().enumerate() {
        let t = i as f64 * STEP;
        ev.push((tick(t), vec![0x90 | ch, m as u8, VELOCITY]));
        ev.push((tick(t + HELD), vec![0x80 | ch, m as u8, 0]));
    }
    ev.sort_by_key(|e| e.0);
    let end = tick(notes.len() as f64 * STEP + 1.0);
    let mut trk = Vec::new();
    let mut last = 0;
    for (t, bytes) in ev.into_iter().chain(std::iter::once((end, vec![0xFF, 0x2F, 0x00]))) {
        vlq(&mut trk, t - last);
        last = t;
        trk.extend(bytes);
    }
    let mut out = b"MThd".to_vec();
    out.extend(6u32.to_be_bytes());
    out.extend([0, 0, 0, 1]);
    out.extend(960u16.to_be_bytes());
    out.extend(b"MTrk");
    out.extend((trk.len() as u32).to_be_bytes());
    out.extend(trk);
    out
}

fn vlq(out: &mut Vec<u8>, mut v: u32) {
    let mut b = vec![(v & 0x7F) as u8];
    v >>= 7;
    while v > 0 {
        b.push(0x80 | (v & 0x7F) as u8);
        v >>= 7;
    }
    out.extend(b.iter().rev());
}

/// A WAV file as mono samples at [`SR`] (16-bit or 32-bit float PCM; channels averaged).
fn read_wav(path: &Path) -> Vec<f64> {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let u16le = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32le = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    assert!(
        &b[0..4] == b"RIFF" && &b[8..12] == b"WAVE",
        "{} is not a WAV file",
        path.display()
    );
    let (mut fmt, mut ch, mut bits, mut rate) = (0, 0, 0, 0);
    let mut i = 12;
    while i + 8 <= b.len() {
        let (id, len) = (&b[i..i + 4], u32le(i + 4) as usize);
        let body = i + 8;
        if id == b"fmt " {
            fmt = u16le(body);
            ch = u16le(body + 2) as usize;
            rate = u32le(body + 4);
            bits = u16le(body + 14);
            if fmt == 0xFFFE {
                fmt = u16le(body + 24);
            }
        } else if id == b"data" {
            assert_eq!(rate, SR as u32, "{} is not at 48 kHz", path.display());
            let data = &b[body..(body + len).min(b.len())];
            let frame = ch * usize::from(bits / 8);
            return data
                .chunks_exact(frame)
                .map(|f| {
                    let s: f64 = (0..ch)
                        .map(|c| match (fmt, bits) {
                            (1, 16) => f64::from(i16::from_le_bytes([f[2 * c], f[2 * c + 1]])) / 32768.0,
                            (3, 32) => {
                                f64::from(f32::from_le_bytes([f[4 * c], f[4 * c + 1], f[4 * c + 2], f[4 * c + 3]]))
                            }
                            _ => panic!("{}: format {fmt} at {bits} bits is not read", path.display()),
                        })
                        .sum();
                    s / ch as f64
                })
                .collect();
        }
        i = body + len + (len & 1);
    }
    panic!("{} has no data", path.display())
}

fn render(fluidsynth: &str, font: &str, work: &Path, out: &Path) {
    std::fs::create_dir_all(work).expect("the work folder");
    let mut table = BTreeMap::new();
    for r in references() {
        let stem = r.name.replace(' ', "_");
        let (mid, wav) = (work.join(format!("{stem}.mid")), work.join(format!("{stem}.wav")));
        std::fs::write(&mid, midi_file(r.program, &r.notes)).expect("the MIDI file");
        let status = Command::new(fluidsynth)
            .args(["-ni", "-q", "-F"])
            .arg(&wav)
            .args(["-r", "48000", "-g", "0.6", "-R", "0", "-C", "0", "-O", "s16"])
            .arg(font)
            .arg(&mid)
            .status()
            .unwrap_or_else(|e| panic!("fluidsynth: {e}"));
        assert!(status.success(), "fluidsynth failed on {}", r.name);
        let x = read_wav(&wav);
        let notes = fingerprints(&r.notes, r.program.is_some(), |i| {
            let a = ((i as f64 * STEP) * SR) as usize;
            let b = (((i + 1) as f64 * STEP) * SR) as usize;
            x[a.min(x.len())..b.min(x.len())].to_vec()
        });
        eprintln!("{}: {} notes", r.name, notes.len());
        table.insert(r.name.to_string(), json!({ "program": r.program, "notes": notes }));
    }
    write(
        out,
        json!({
            "what": "FluidR3_GM through fluidsynth: one note of 1 s at velocity 100, reverb and chorus off, 48 kHz (sound upgrade stage 3, S1)",
            "map": MAP.iter().map(|(f, r)| (f.to_string(), Value::from(*r))).collect::<serde_json::Map<_, _>>(),
            "references": table,
        }),
    );
}

fn built(out: &Path) {
    let mut table = BTreeMap::new();
    for fighter in bf_content::PORTED {
        let v = bf_content::fighters::voice(fighter).unwrap_or_else(|| panic!("{fighter} has no voice"));
        let (lo, hi) = v.register();
        let notes: Vec<i32> = (lo..=hi).collect();
        let pitched = !matches!(fighter, "BASS DRUM" | "DRUMS" | "SNARE DRUM" | "TURNTABLES");
        let fp = fingerprints(&notes, pitched, |i| {
            let mut noise = PpNoise::start();
            v.note(&mut noise, f64::from(notes[i]), 0.8, HELD)
        });
        eprintln!("{fighter}: {} notes", fp.len());
        table.insert(fighter.to_string(), json!({ "notes": fp }));
    }
    write(
        out,
        json!({ "what": "each fighter's built voice: one note of 1 s at velocity 0.8", "voices": table }),
    );
}

/// The fingerprints of `notes`, the `i`th note's audio from `note(i)`, measured on a few threads.
fn fingerprints(notes: &[i32], pitched: bool, note: impl Fn(usize) -> Vec<f64> + Sync) -> Vec<Value> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(8);
    let mut out: Vec<(usize, Value)> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                let note = &note;
                s.spawn(move || {
                    (t..notes.len())
                        .step_by(threads)
                        .map(|i| (i, timbre::fingerprint(&note(i), notes[i], HELD, pitched).to_json()))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter()
            .flat_map(|h| h.join().expect("a measuring thread"))
            .collect()
    });
    out.sort_by_key(|(i, _)| *i);
    out.into_iter().map(|(_, v)| v).collect()
}

fn write(path: &Path, v: Value) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let text = serde_json::to_string_pretty(&v).expect("JSON");
    std::fs::write(path, text + "\n").unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    eprintln!("wrote {}", path.display());
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["render", fs, font, work, out] => render(fs, font, &PathBuf::from(work), &PathBuf::from(out)),
        ["built", out] => built(&PathBuf::from(out)),
        _ => {
            eprintln!(
                "usage: references render <fluidsynth> <FluidR3_GM.sf2> <work dir> <out.json>\n       references built <out.json>"
            );
            std::process::exit(2);
        }
    }
}
