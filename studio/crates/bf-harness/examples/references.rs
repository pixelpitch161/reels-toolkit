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
                .map(|f| (f.midi, f))
                .collect();
            (name.clone(), notes)
        })
        .collect()
}

/// A voice's note at 44.1 kHz, from its frequency, its length in seconds and which repeat of the note it is.
type Tone = Box<dyn Fn(f64, f64, u64) -> Vec<f64>>;

/// A reworked fighter: its register, its real instrument, its rivals, and its note today and reworked.
fn reworked(fighter: &str) -> ((i32, i32), &'static str, &'static [&'static str], Tone, Tone) {
    use bf_content::fighters::alto_sax::voice as alto;
    use bf_content::fighters::bari_sax::voice as bari;
    use bf_content::fighters::soprano_sax::voice as soprano;
    use bf_content::fighters::tenor_sax::voice as tenor;
    use bf_core::rng::Generator;
    use bf_sound::voice::pp::alto::alto_tone;
    use bf_sound::voice::real::midi_of;
    // the alto block's breath, from a fresh generator each note, as the alto part seeds it
    let rn = || Generator::new(23);
    match fighter {
        "BARI SAX" => (
            bari::REGISTER,
            "bari sax",
            &["trombone", "tuba"],
            Box::new(|f, dur, _| bari::bari_tone_scooped(f, dur, 0.8, 0.0, Some(bari::STEADY_SCOOP))),
            Box::new(|f, dur, repeat| bari::bari_tone_real(f, dur, 0.8, 0.0, repeat % 3)),
        ),
        "ALTO SAX" => (
            alto::REGISTER,
            "alto sax",
            &["trumpet", "french horn", "trombone"],
            Box::new(move |f, dur, _| alto_tone(&mut rn(), f, dur, 0.8, &alto::TONE)),
            Box::new(move |f, dur, repeat| alto::tone_real(&mut rn(), midi_of(f), dur, 0.8, &alto::TONE, repeat % 3)),
        ),
        "SOPRANO SAX" => (
            soprano::REGISTER,
            "soprano sax",
            &["trumpet", "muted trumpet", "french horn"],
            Box::new(move |f, dur, _| alto_tone(&mut rn(), f, dur, 0.8, &soprano::TONE)),
            Box::new(move |f, dur, repeat| {
                soprano::tone_real(&mut rn(), midi_of(f), dur, 0.8, &soprano::TONE, repeat % 3)
            }),
        ),
        "TENOR SAX" => (
            tenor::REGISTER,
            "tenor sax",
            &["trumpet", "french horn", "trombone"],
            Box::new(|f, dur, _| tenor::tenor_tone(f, dur, 0.8, tenor::VOICE_HONK, 0.0)),
            Box::new(|f, dur, repeat| tenor::tenor_tone_real(f, dur, 0.8, tenor::VOICE_HONK, 0.0, repeat % 3)),
        ),
        _ => panic!("{fighter} has no reworked voice yet"),
    }
}

fn compare(fighter: &str) {
    let refs = committed();
    let ((lo, hi), own, rivals, today, new) = reworked(fighter);
    let measure = |tone: &Tone, m: i32| {
        let x = bf_sound::voice::pp::to_48k(&tone(bf_sound::voice::pp::midi_hz(f64::from(m)), HELD, 0));
        timbre::fingerprint(&x, m, HELD, true)
    };
    println!(
        "note  today: {own} {}   reworked: {own} {}",
        rivals.join(" "),
        rivals.join(" ")
    );
    let (mut worst, mut wrong) = (0.0_f64, 0);
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
        let mut line = format!("{m:>4}");
        shape[2].push(of(&refs[own][&m]));
        for (i, tone) in [&today, &new].into_iter().enumerate() {
            let b = measure(tone, m);
            shape[i].push(of(&b));
            let d = |r: &str| timbre::distance(&b.harmonics, &refs[r][&m].harmonics);
            let d_own = d(own);
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
            let x = bf_sound::voice::pp::to_48k(&tone(bf_sound::voice::pp::midi_hz(f64::from(m)), dur, *repeat));
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
        _ => {
            eprintln!(
                "usage: references render <fluidsynth> <FluidR3_GM.sf2> <work dir> <out.json>\n       references built <out.json>\n       references profile <reference> <lo> <hi>\n       references compare <FIGHTER>\n       references phrase <FIGHTER> <dir>"
            );
            std::process::exit(2);
        }
    }
}
