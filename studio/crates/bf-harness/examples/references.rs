//! The real-instrument references of the sound upgrade's stage 3 (S1), and the built voices' fingerprints beside them.
//!
//! ```text
//! cargo run --release -p bf-harness --example references -- render <fluidsynth> <FluidR3_GM.sf2> <work dir> <out.json>
//! cargo run --release -p bf-harness --example references -- built <out.json>
//! cargo run --release -p bf-harness --example references -- profile "<reference>" <lo> <hi>
//! cargo run --release -p bf-harness --example references -- compare "<FIGHTER>"
//! cargo run --release -p bf-harness --example references -- phrase "<FIGHTER>" <dir>
//! ```
//!
//! `render` plays every reference instrument (a General MIDI program of FluidR3, or a drum of its kit) through
//! fluidsynth, one note of 1 s at velocity 100 every 2.5 s across the instrument's range, and writes each note's
//! fingerprint ([`bf_harness::timbre`]). The recordings stay in the work folder: they are a development measure
//! only and are never committed (the fingerprints are). `built` writes the same fingerprints of each fighter's built
//! voice, one note of 1 s at velocity 0.8 at every note of its register. `profile` prints a reference's first eight
//! overtones from `lo` to `hi` as a `bf_sound::voice::real::Profile` (from the committed fingerprints). `compare`
//! prints, note by note, how far a reworked fighter's voice is from its real instrument and from the rivals it could be
//! taken for, today (its ported voice) and reworked (the fingerprint distance, as `tests/real_voices.rs` tests it).

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

/// The committed fingerprints.
const FLUIDR3: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reference/fluidr3.json");

fn profile(name: &str, lo: i32, hi: i32) {
    let text = std::fs::read_to_string(FLUIDR3).unwrap_or_else(|e| panic!("{FLUIDR3}: {e}"));
    let v: Value = serde_json::from_str(&text).expect("the fingerprints' JSON");
    let notes = v["references"][name]["notes"]
        .as_array()
        .unwrap_or_else(|| panic!("no reference {name:?}"));
    println!("pub const PROFILE: &Profile = &[");
    for n in notes {
        let Some(f) = timbre::Fingerprint::from_json(n) else {
            continue;
        };
        if f.midi < lo || f.midi > hi {
            continue;
        }
        let h: Vec<String> = f
            .harmonics
            .iter()
            .take(8)
            .map(|x| format!("{:.1}", x.clamp(timbre::FLOOR, 0.0)))
            .collect();
        println!("    ({}, [{}]),", f.midi, h.join(", "));
    }
    println!("];");
}

/// Each reference's fingerprints, by MIDI note, from the committed file.
fn committed() -> BTreeMap<String, BTreeMap<i32, timbre::Fingerprint>> {
    let text = std::fs::read_to_string(FLUIDR3).unwrap_or_else(|e| panic!("{FLUIDR3}: {e}"));
    let v: Value = serde_json::from_str(&text).expect("the fingerprints' JSON");
    v["references"]
        .as_object()
        .expect("the references")
        .iter()
        .map(|(name, r)| {
            let notes = r["notes"]
                .as_array()
                .expect("its notes")
                .iter()
                .filter_map(timbre::Fingerprint::from_json)
                // FluidR3 has no sample for some notes (the tuba above 72, the contrabass above 57): silence
                .filter(|f| f.harmonics.iter().any(|&h| h > -200.0))
                .map(|f| (f.midi, f))
                .collect();
            (name.clone(), notes)
        })
        .collect()
}

/// A voice's note at 44.1 kHz, from its frequency, its length in seconds and which repeat of the note it is.
/// A note at `f` Hz held `dur` s, the `repeat`th time, at 48 kHz.
type Tone = Box<dyn Fn(f64, f64, u64) -> Vec<f64>>;

/// A reworked fighter: its register, its real instrument, its rivals, and its note today and reworked.
fn reworked(fighter: &str) -> ((i32, i32), &'static str, &'static [&'static str], Tone, Tone) {
    use bf_content::fighters::alto_sax::voice as alto;
    use bf_content::fighters::bari_sax::voice as bari;
    use bf_content::fighters::soprano_sax::voice as soprano;
    use bf_content::fighters::tenor_sax::voice as tenor;
    use bf_core::rng::Generator;
    use bf_sound::voice::pp::alto::alto_tone;
    use bf_sound::voice::pp::to_48k;
    use bf_sound::voice::real::midi_of;
    // the alto block's breath, from a fresh generator each note, as the alto part seeds it
    let rn = || Generator::new(23);
    match fighter {
        "BARI SAX" => (
            bari::REGISTER,
            "bari sax",
            &["trombone", "tuba"],
            Box::new(|f, dur, _| to_48k(&bari::bari_tone_scooped(f, dur, 0.8, 0.0, Some(bari::STEADY_SCOOP)))),
            Box::new(|f, dur, repeat| to_48k(&bari::bari_tone_real(f, dur, 0.8, 0.0, repeat % 3))),
        ),
        "ALTO SAX" => (
            alto::REGISTER,
            "alto sax",
            &["trumpet", "french horn", "trombone"],
            Box::new(move |f, dur, _| to_48k(&alto_tone(&mut rn(), f, dur, 0.8, &alto::TONE))),
            Box::new(move |f, dur, repeat| {
                to_48k(&alto::tone_real(
                    &mut rn(),
                    midi_of(f),
                    dur,
                    0.8,
                    &alto::TONE,
                    repeat % 3,
                ))
            }),
        ),
        "SOPRANO SAX" => (
            soprano::REGISTER,
            "soprano sax",
            &["trumpet", "muted trumpet", "french horn"],
            Box::new(move |f, dur, _| to_48k(&alto_tone(&mut rn(), f, dur, 0.8, &soprano::TONE))),
            Box::new(move |f, dur, repeat| {
                to_48k(&soprano::tone_real(
                    &mut rn(),
                    midi_of(f),
                    dur,
                    0.8,
                    &soprano::TONE,
                    repeat % 3,
                ))
            }),
        ),
        "TENOR SAX" => (
            tenor::REGISTER,
            "tenor sax",
            &["trumpet", "french horn", "trombone"],
            Box::new(|f, dur, _| to_48k(&tenor::tenor_tone(f, dur, 0.8, tenor::VOICE_HONK, 0.0))),
            Box::new(|f, dur, repeat| to_48k(&tenor::tenor_tone_real(f, dur, 0.8, tenor::VOICE_HONK, 0.0, repeat % 3))),
        ),
        "BAGPIPES" => (
            bf_content::fighters::bagpipes::voice::REGISTER,
            "bagpipe",
            rivals(fighter),
            Box::new(|f, dur, _| bf_content::fighters::bagpipes::sound::chanter(midi_of(f).round(), dur, 0.8, false)),
            Box::new(|f, dur, repeat| {
                bf_content::fighters::bagpipes::sound::chanter_real(midi_of(f).round(), dur, 0.8, false, repeat % 3)
            }),
        ),
        _ => {
            // a voice reworked from the outside (Voice::rework): its own note, and its Composed note
            let v = bf_content::fighters::voice(fighter).unwrap_or_else(|| panic!("{fighter} has no voice"));
            assert!(v.reworked(), "{fighter} has no reworked voice yet");
            let own = MAP
                .iter()
                .find(|(f, _)| *f == fighter)
                .map(|(_, r)| *r)
                .expect("a reference");
            let midi = |f: f64| midi_of(f).round();
            let w = v.clone();
            (
                v.register(),
                own,
                rivals(fighter),
                Box::new(move |f, dur, _| w.note(&mut PpNoise::start(), midi(f), 0.8, dur)),
                Box::new(move |f, dur, repeat| v.note_real(&mut PpNoise::start(), midi(f), 0.8, dur, repeat % 3)),
            )
        }
    }
}

/// How much brighter (dB per harmonic, positive) or mellower (negative) a fighter is aimed than the real instrument
/// it is measured against, for one measured against a relative FluidR3 lacks: the cornet and flugelhorn against the
/// trumpet (a cornet is a little mellower, a flugelhorn more so) and the mellophone against the horn (a little
/// brighter), so the family stays three voices, not one. The bass guitar is aimed a little brighter than FluidR3's
/// finger bass, whose overtones nearly stop at the fourth harmonic: so its notes still speak on a phone.
fn tilt(fighter: &str) -> f64 {
    match fighter {
        "CORNET" => -0.5,
        "FLUGELHORN" => -1.0,
        "MELLOPHONE" => 0.5,
        "BASS GUITAR" => 1.0,
        "EUPHONIUM" => 1.5,
        _ => 0.0,
    }
}

/// The real levels `real` a fighter is aimed at: every harmonic tilted by [`tilt`] per harmonic, the strongest of the
/// first eight back at 0 dB.
fn shaded(fighter: &str, real: &[f64]) -> Vec<f64> {
    let t = tilt(fighter);
    if t == 0.0 {
        return real.to_vec();
    }
    let n = real.len().min(bf_sound::voice::real::SET);
    let mut h: Vec<f64> = real.iter().enumerate().map(|(k, v)| v + t * k as f64).collect();
    let top = h[..n].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    h.iter_mut().for_each(|v| *v -= top);
    h
}

/// The note a fighter's part plays at MIDI `m`, held, for the measures: its voice's, but the BAGPIPES' chanter alone
/// (the voice's note carries the drones, which the part plays apart).
fn own_note(fighter: &str, v: &dyn bf_sound::voice::Voice, m: i32) -> Vec<f64> {
    if fighter == "BAGPIPES" {
        return bf_content::fighters::bagpipes::sound::chanter(f64::from(m), HELD, 0.8, false);
    }
    v.note(&mut PpNoise::start(), f64::from(m), 0.8, HELD)
}

/// The gains, refined from none, that bring the overtones `measure` gives for them nearest `real` (or stop
/// improving), with their distance; `before` is the distance with none.
fn refine(
    measure: &dyn Fn(&[f64; bf_sound::voice::real::SET]) -> Vec<f64>,
    real: &[f64],
    before: f64,
) -> (f64, [f64; bf_sound::voice::real::SET]) {
    use bf_sound::voice::real::SET;
    let mut g = [0.0; SET];
    let (mut best, mut best_g) = (before, g);
    for _ in 0..8 {
        let h = measure(&g);
        for k in 0..SET {
            // levels under the floor count as the floor, as the distance has them: a harmonic both real and built
            // have too weak to hear is left alone (the eighth's gain turns everything above it, the breath between
            // the harmonics too)
            let gap = real[k].max(timbre::FLOOR) - h[k].max(timbre::FLOOR);
            g[k] = (g[k] + 0.85 * gap).clamp(-30.0, 24.0);
        }
        let d = timbre::distance(&measure(&g), real);
        if d < best {
            (best, best_g) = (d, g);
        }
    }
    (best, best_g)
}

/// A voice's overtone [`Correction`](bf_sound::voice::real::Correction) toward its real instrument, printed as the
/// table its voice file keeps: for each note of its register, gains for its first eight harmonics, refined until the
/// corrected note's overtones match the real ones (or stop improving).
fn correct(fighter: &str) {
    use bf_sound::voice::real::{self, SET};
    let refs = committed();
    let v = bf_content::fighters::voice(fighter).unwrap_or_else(|| panic!("{fighter} has no voice"));
    let own = MAP
        .iter()
        .find(|(f, _)| *f == fighter)
        .map(|(_, r)| *r)
        .expect("a reference");
    let (lo, hi) = v.register();
    // each note: its MIDI note, the added overtones, the correction, and its distance before and after
    type Row = (i32, [f64; SET], [f64; SET], f64, f64);
    let rows: Vec<Row> = std::thread::scope(|sc| {
        let hs: Vec<_> = (lo..=hi)
            .map(|m| {
                let v = &v;
                let refs = &refs;
                sc.spawn(move || {
                    let real = &shaded(fighter, &refs[own].get(&m)?.harmonics);
                    let x = own_note(fighter, &**v, m);
                    let f = bf_sound::voice::pp::midi_hz(f64::from(m));
                    // the overtones the voice barely has (15 dB or more under the real balance, beside its
                    // fundamental) are laid in at the real level; the peaks then take the rest
                    let was = timbre::fingerprint(&x, m, HELD, true).harmonics;
                    let mut add = [real::NONE; SET];
                    for k in 1..SET {
                        let want = real[k] - real[0];
                        if real[k] > timbre::FLOOR && was[k] - was[0] < want - 15.0 {
                            add[k] = want;
                        }
                    }
                    let before = timbre::distance(&was, real);
                    let attempt = |add: &[f64; SET]| {
                        let x = real::fill(&x, f, bf_sound::SRF, add);
                        let measure = |g: &[f64; SET]| {
                            let y = real::correct(&x, f, bf_sound::SRF, g);
                            timbre::fingerprint(&y, m, HELD, true).harmonics
                        };
                        let (d, g) = refine(&measure, real, before);
                        (d, g, measure(&g))
                    };
                    let (mut best, mut best_g, mut now) = attempt(&add);
                    if best > 5.0 {
                        // a note whose own fundamental is weak (a pickup's notch, say) has the overtones it lacks
                        // against its strongest laid in too, as loud beside its fundamental as the real balance
                        // has them beside the strongest
                        let mut more = add;
                        for k in 1..SET {
                            if more[k] == real::NONE && real[k] > timbre::FLOOR && was[k] < real[k] - 15.0 {
                                more[k] = real[k] - was[0];
                            }
                        }
                        if more != add {
                            let (d, g, n) = attempt(&more);
                            if d < best {
                                (best, best_g, now, add) = (d, g, n, more);
                            }
                        }
                    }
                    if std::env::var_os("BF_SHOW").is_some() {
                        let show = |h: &[f64]| h.iter().take(SET).map(|x| format!("{x:>6.1}")).collect::<String>();
                        eprintln!(
                            "{m} real {}\n{m} was  {}\n{m} now  {}\n{m} gain {}",
                            show(real),
                            show(&was),
                            show(&now),
                            show(&best_g)
                        );
                    }
                    Some((m, add, best_g, before, best))
                })
            })
            .collect();
        hs.into_iter().filter_map(|h| h.join().expect("a note")).collect()
    });
    let table = |name: &str, ty: &str, pick: &dyn Fn(&Row) -> [f64; SET]| {
        println!("pub const {name}: &{ty} = &[");
        for r in &rows {
            let g: Vec<String> = pick(r).iter().map(|x| format!("{x:.1}")).collect();
            println!("    ({}, [{}]),", r.0, g.join(", "));
        }
        println!("];");
    };
    if rows.iter().any(|r| r.1.iter().any(|&d| d > real::NONE)) {
        table("FILL", "Fill", &|r| r.1);
    }
    table("CORRECTION", "Correction", &|r| r.2);
    for (m, add, _, was, now) in &rows {
        let n = add.iter().filter(|&&d| d > real::NONE).count();
        eprintln!(
            "{m:>4} {was:>5.1} -> {now:>5.1}{}",
            if n > 0 { format!("  ({n} added)") } else { String::new() }
        );
    }
    let worst = rows.iter().fold(0.0_f64, |w, r| w.max(r.4));
    let was = rows.iter().fold(0.0_f64, |w, r| w.max(r.3));
    eprintln!("{fighter}: farthest from the real {own} {was:.1} today, {worst:.1} corrected");
}

/// How far a note's 9th to 12th harmonics sit under its 5th to 8th, in dB of their summed powers (each level floored
/// at [`timbre::FLOOR`]).
fn fall(h: &[f64]) -> f64 {
    let power = |s: &[f64]| s.iter().map(|&d| 10f64.powf(d.max(timbre::FLOOR) / 10.0)).sum::<f64>();
    10.0 * (power(&h[4..8]) / power(&h[8..12])).log10()
}

/// Prints a fighter's [`Tail`](bf_sound::voice::real::Tail) table: at each note of its register that its real
/// instrument has, the real (shaded) 9th to 12th harmonics in dB beside its 5th to 8th together.
fn tail(fighter: &str) {
    let refs = committed();
    let v = bf_content::fighters::voice(fighter).unwrap_or_else(|| panic!("{fighter} has no voice"));
    let own = MAP
        .iter()
        .find(|(f, _)| *f == fighter)
        .map(|(_, r)| *r)
        .expect("a reference");
    let (lo, hi) = v.register();
    let power = |db: f64| 10f64.powf(db.max(timbre::FLOOR) / 10.0);
    println!("pub const TAIL: &Tail = &[");
    for m in lo..=hi {
        let Some(r) = refs[own].get(&m) else { continue };
        let real = shaded(fighter, &r.harmonics);
        let base: f64 = real[4..8].iter().map(|&d| power(d)).sum();
        let row: Vec<String> = real[8..12]
            .iter()
            .map(|&d| format!("{:.1}", 10.0 * (power(d) / base).log10()))
            .collect();
        println!("    ({m}, [{}]),", row.join(", "));
    }
    println!("];");
}

/// The real instruments a reworked voice could be taken for, beside its own (S1's test).
fn rivals(fighter: &str) -> &'static [&'static str] {
    match fighter {
        "TUBA" | "SOUSAPHONE" | "EUPHONIUM" => &["trombone", "bari sax", "bassoon", "french horn"],
        "TROMBONE" => &["tuba", "bari sax", "french horn", "tenor sax"],
        "OCTOBASS" | "CONTRABASS" => &["cello", "finger bass", "tuba"],
        "TIMPANI" => &["marimba", "finger bass", "tuba"],
        "PICCOLO" => &["flute", "oboe", "clarinet"],
        "FLUTE" => &["piccolo", "oboe", "clarinet", "soprano sax"],
        "OBOE" => &["soprano sax", "clarinet", "trumpet", "flute"],
        "CLARINET" => &["oboe", "french horn", "bassoon", "flute"],
        "BASS CLARINET" | "CONTRABASS CLARINET" => &["bassoon", "tuba", "french horn"],
        "BAGPIPES" => &["trumpet", "muted trumpet", "oboe"],
        "TRUMPET" => &["trombone", "french horn", "alto sax", "soprano sax"],
        "CORNET" | "FLUGELHORN" => &["french horn", "trombone", "alto sax"],
        "HORN" => &["trombone", "trumpet", "alto sax", "bassoon"],
        "MELLOPHONE" => &["trumpet", "trombone", "alto sax"],
        "VIOLIN" => &["viola", "cello", "french horn", "trumpet"],
        "VIOLA" => &["violin", "cello", "french horn", "english horn"],
        "CELLO" => &["viola", "contrabass", "french horn", "bassoon"],
        "BASSOON" | "CONTRABASSOON" => &["bari sax", "clarinet", "tuba", "french horn"],
        "ACCORDION" => &["church organ", "drawbar organ", "oboe", "bagpipe"],
        "MARIMBA" => &["piano", "harp", "timpani"],
        _ => &[],
    }
}

fn compare(fighter: &str) {
    let refs = committed();
    let ((lo, hi), own, rivals, today, new) = reworked(fighter);
    let measure = |tone: &Tone, m: i32| {
        let x = tone(bf_sound::voice::pp::midi_hz(f64::from(m)), HELD, 0);
        timbre::fingerprint(&x, m, HELD, true)
    };
    println!(
        "note  today: {own} {}   reworked: {own} {}",
        rivals.join(" "),
        rivals.join(" ")
    );
    let (mut worst, mut wrong) = (0.0_f64, 0);
    // how much further the 9th to 12th sit under the 5th to 8th than the real one's: today's worst, reworked's worst
    let mut darker = [f64::NEG_INFINITY; 2];
    // attack, release and pitch travel: today's, reworked and the real instrument's, note by note
    let mut shape: [Vec<[f64; 3]>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let of = |f: &timbre::Fingerprint| {
        [
            f.attack_ms,
            f.release_ms.unwrap_or(f64::NAN),
            f.span.unwrap_or(f64::NAN),
        ]
    };
    for m in lo..=hi {
        if !refs[own].contains_key(&m) {
            continue;
        }
        let mut line = format!("{m:>4}");
        shape[2].push(of(&refs[own][&m]));
        for (i, tone) in [&today, &new].into_iter().enumerate() {
            let b = measure(tone, m);
            shape[i].push(of(&b));
            let d = |r: &str| {
                refs[r]
                    .get(&m)
                    .map_or(f64::NAN, |x| timbre::distance(&b.harmonics, &x.harmonics))
            };
            let d_own = d(own);
            darker[i] = darker[i].max(fall(&b.harmonics) - fall(&refs[own][&m].harmonics));
            if std::env::var_os("BF_SHOW").is_some() {
                let show = |h: &[f64]| h.iter().take(8).map(|x| format!("{x:>6.1}")).collect::<String>();
                eprintln!(
                    "{m} {i} built {}\n{m} {i} real  {}",
                    show(&b.harmonics),
                    show(&refs[own][&m].harmonics)
                );
            }
            let ds: Vec<f64> = rivals.iter().map(|r| d(r)).collect();
            if i == 1 {
                worst = worst.max(d_own);
                wrong += usize::from(ds.iter().any(|&x| x <= d_own));
            }
            line += &format!(
                "   {d_own:>5.1} {}",
                ds.iter().map(|x| format!("{x:>5.1}")).collect::<Vec<_>>().join(" ")
            );
        }
        println!("{line}");
    }
    println!("reworked: farthest from the real {own} {worst:.1}; notes nearer a rival {wrong}");
    println!(
        "above the eighth: the 9th to 12th at most {:.1} dB further under the 5th to 8th than the real {own}'s (today {:.1})",
        darker[1], darker[0]
    );
    for (name, rows) in ["today", "reworked", "real"].iter().zip(&shape) {
        let median = |k: usize| {
            let mut v: Vec<f64> = rows.iter().map(|r| r[k]).filter(|x| x.is_finite()).collect();
            v.sort_by(f64::total_cmp);
            v.get(v.len() / 2).copied().unwrap_or(f64::NAN)
        };
        println!(
            "{name:>9}: attack {:.0} ms, release {:.0} ms, pitch travel {:.2} semitones (medians)",
            median(0),
            median(1),
            median(2)
        );
    }
}

/// Today's voices of `fighters` against every reference: for each fighter, how far its notes are from its own
/// real instrument ([`MAP`]), on how many notes another real instrument is as near or nearer (and which), and its
/// attack, release and pitch travel against the real one's. The baseline before a family is reworked.
fn survey(fighters: &[&str]) {
    let refs = committed();
    let map: BTreeMap<&str, &str> = MAP.iter().copied().collect();
    for fighter in fighters {
        let v = bf_content::fighters::voice(fighter).unwrap_or_else(|| panic!("{fighter} has no voice"));
        let own = map[fighter];
        let (lo, hi) = v.register();
        let (mut best, mut worst, mut wrong) = (f64::INFINITY, 0.0_f64, 0);
        let mut nearer: BTreeMap<&str, usize> = BTreeMap::new();
        let mut shape: [Vec<[f64; 3]>; 2] = [Vec::new(), Vec::new()];
        let of = |f: &timbre::Fingerprint| {
            [
                f.attack_ms,
                f.release_ms.unwrap_or(f64::NAN),
                f.span.unwrap_or(f64::NAN),
            ]
        };
        for m in lo..=hi {
            let Some(real) = refs[own].get(&m) else { continue };
            let b = timbre::fingerprint(&own_note(fighter, &*v, m), m, HELD, true);
            shape[0].push(of(&b));
            shape[1].push(of(real));
            let d_own = timbre::distance(&b.harmonics, &real.harmonics);
            best = best.min(d_own);
            worst = worst.max(d_own);
            let mut any = false;
            for (name, notes) in &refs {
                if name == own || name == "drum kit" {
                    continue;
                }
                if let Some(r) = notes.get(&m)
                    && timbre::distance(&b.harmonics, &r.harmonics) <= d_own
                {
                    *nearer.entry(name.as_str()).or_insert(0) += 1;
                    any = true;
                }
            }
            wrong += usize::from(any);
        }
        let mut top: Vec<(&str, usize)> = nearer.into_iter().collect();
        top.sort_by_key(|a| std::cmp::Reverse(a.1));
        let top: Vec<String> = top.iter().take(4).map(|(n, c)| format!("{n} {c}")).collect();
        let median = |rows: &[[f64; 3]], k: usize| {
            let mut v: Vec<f64> = rows.iter().map(|r| r[k]).filter(|x| x.is_finite()).collect();
            v.sort_by(f64::total_cmp);
            v.get(v.len() / 2).copied().unwrap_or(f64::NAN)
        };
        println!(
            "{fighter} ({own}, {} notes): {best:.1} to {worst:.1} dB; another as near on {wrong} ({}); attack {:.0}/{:.0} ms, release {:.0}/{:.0} ms, travel {:.2}/{:.2} (today/real)",
            shape[0].len(),
            top.join(", "),
            median(&shape[0], 0),
            median(&shape[1], 0),
            median(&shape[0], 1),
            median(&shape[1], 1),
            median(&shape[0], 2),
            median(&shape[1], 2),
        );
    }
}

/// A short riff through a reworked fighter's voice alone, today's and reworked, as `<dir>/<fighter>_today.wav` and
/// `_reworked.wav` (48 kHz): the low notes of its register, a repeated note and a run, for the listening page.
fn phrase(fighter: &str, dir: &Path) {
    let ((lo, _), _, _, today, new) = reworked(fighter);
    // (semitones over the register's foot, beats); 0.5 s a beat
    const RIFF: [(i32, f64); 14] = [
        (0, 1.0),
        (0, 0.5),
        (0, 0.5),
        (7, 1.0),
        (5, 0.5),
        (3, 0.5),
        (0, 2.0),
        (12, 0.5),
        (10, 0.5),
        (7, 0.5),
        (5, 0.5),
        (3, 0.5),
        (5, 0.5),
        (0, 2.0),
    ];
    std::fs::create_dir_all(dir).ok();
    for (name, tone) in [("today", &today), ("reworked", &new)] {
        let mut out = vec![0.0; (SR * 13.0) as usize];
        let mut at = 0.25;
        let mut seen: BTreeMap<i32, u64> = BTreeMap::new();
        for (step, beats) in RIFF {
            let m = lo + 4 + step;
            let repeat = seen.entry(m).or_insert(0);
            let dur = beats * 0.5 * 0.92;
            let x = tone(bf_sound::voice::pp::midi_hz(f64::from(m)), dur, *repeat);
            *repeat += 1;
            let start = (at * SR) as usize;
            for (i, v) in x.iter().enumerate() {
                if let Some(o) = out.get_mut(start + i) {
                    *o += 0.5 * v;
                }
            }
            at += beats * 0.5;
        }
        let peak = out.iter().fold(0.0_f64, |p, v| p.max(v.abs())).max(1e-9);
        let stereo: Vec<[f64; 2]> = out.iter().map(|v| [0.8 * v / peak; 2]).collect();
        let path = dir.join(format!("{}_{name}.wav", fighter.to_lowercase().replace(' ', "_")));
        std::fs::write(&path, bf_harness::checks::wav_bytes(&stereo))
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        eprintln!("wrote {}", path.display());
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["render", fs, font, work, out] => render(fs, font, &PathBuf::from(work), &PathBuf::from(out)),
        ["built", out] => built(&PathBuf::from(out)),
        ["profile", name, lo, hi] => profile(name, lo.parse().expect("lo"), hi.parse().expect("hi")),
        ["compare", fighter] => compare(fighter),
        ["phrase", fighter, dir] => phrase(fighter, &PathBuf::from(dir)),
        ["correct", fighter] => correct(fighter),
        ["tail", fighter] => tail(fighter),
        ["survey", fighters @ ..] if !fighters.is_empty() => survey(fighters),
        _ => {
            eprintln!(
                "usage: references render <fluidsynth> <FluidR3_GM.sf2> <work dir> <out.json>\n       references built <out.json>\n       references profile <reference> <lo> <hi>\n       references compare <FIGHTER>\n       references phrase <FIGHTER> <dir>\n       references survey <FIGHTER>...\n       references correct <FIGHTER>\n       references tail <FIGHTER>"
            );
            std::process::exit(2);
        }
    }
}
