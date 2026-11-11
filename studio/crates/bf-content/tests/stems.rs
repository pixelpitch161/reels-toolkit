//! The whole soundtrack of the example episode against the Python kit's: `bf_content::build_soundtrack` on the
//! imported recording (FLUTE against TROMBONE, seed 392) and the files in
//! `studio/reference/sound/flute_vs_trombone` (`python -m tools.studio_export stems`).
//!
//! - `stems.npz`: every bus after the master, float32 windows of 2.0 s (and the last 0.25 s before the loop
//!   point). The bar is -120 dBFS on every window, or -80 dBFS for a bus listed in
//!   `studio/reference/sound/exceptions.toml` with a reason. The references are float32, so the result is rounded
//!   to float32 before it is compared (the rounding itself sits near -150 dBFS).
//! - `rms.json`: the RMS of every 100 ms block of every stem and of the master, within 0.01 dB; a block the kit
//!   has below -90 dBFS must be below it here too and is not compared further.
//! - `audio.wav`: the mastered file as the kit wrote it (16 bit): every sample within 3 LSB, the RMS of the
//!   difference below -90 dBFS, and the first and last 4 ms (the loop seam) as loud as the kit's.
//! - `note_calls.json`: the order of the 174 first-time notes (every FLUTE note moves the shared noise stream on).
//! - `premaster.npz`: every bus as the score leaves it, before the master (the place to look first when a stem
//!   is off).
//!
//! Then what a fight file can do wrong (an unknown theme, a chord the theme has no voicing for, a fighter whose
//! part is not here, a fight with no K.O.), a panic inside the sound code, and the same fight file at two other
//! tempos.
//!
//! The tests print the worst difference per bus; run with `-- --nocapture` to see them.

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Instant;

use bf_content::{ContentError, SoundBuild, build_soundtrack, theme_names, validate};
use bf_core::event::Value as EvValue;
use bf_core::fightfile::FightFile;
use bf_core::recording::{Array, Recording};
use bf_core::rng::Generator;
use bf_core::show::{Show, ShowError};
use bf_sound::master::Soundtrack;
use bf_sound::music::NoteCall;
use bf_sound::voice::PpNoise;
use bf_sound::wav::sample_i16;
use bf_sound::{NA, SR};
use bf_testdata::{Vectors, hex_arg, reference, repo_root};
use serde_json::Value;

/// Sample differences of a stem window: below this many dBFS.
const DB: f64 = -120.0;
/// The same for a bus listed in `exceptions.toml`.
const DB_EXCEPTION: f64 = -80.0;
/// A 100 ms block's RMS against the kit's: within this many dB.
const RMS_DB: f64 = 0.01;
/// Blocks the kit has below this many dBFS are only checked to be below it.
const RMS_FLOOR: f64 = -90.0;
/// 100 ms.
const BLOCK: usize = SR / 10;
/// The buses of the episode, in the order its score first touches them.
const BUSES: [&str; 7] = ["drums", "calliope", "fx", "flute", "trombone", "glock", "solo"];
/// The windows of `stems.npz` and `premaster.npz`: eight of 2.0 s and one of 0.25 s.
const WINDOWS: usize = 9;

const DIR: &str = "sound/flute_vs_trombone";

// ---------------------------------------------------------------- the episode, built once

struct Episode {
    rec: Recording,
    ff: FightFile,
}

static EPISODE: LazyLock<Episode> = LazyLock::new(|| {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    Episode { rec, ff }
});

/// The soundtrack of the episode, as `build_soundtrack` makes it.
static BUILT: LazyLock<Soundtrack> = LazyLock::new(|| {
    let started = Instant::now();
    let st = build_soundtrack(&EPISODE.rec, &EPISODE.ff).expect("the episode's soundtrack builds");
    println!(
        "build_soundtrack (test profile): {:.2} s",
        started.elapsed().as_secs_f64()
    );
    st
});

/// What the score alone leaves: the windows of every bus before the master, each bus's 100 ms profile, the
/// first-time notes in order and the noise stream where it ended.
struct Scored {
    bus_order: Vec<String>,
    len: usize,
    windows: BTreeMap<(usize, String), Vec<[f64; 2]>>,
    rms: BTreeMap<String, Vec<f64>>,
    notes: Vec<NoteCall>,
    noise: PpNoise,
}

static SCORED: LazyLock<Scored> = LazyLock::new(|| {
    let ep = &*EPISODE;
    let show = Show::new(&ep.rec, &ep.ff).expect("the episode has a K.O.");
    let build = SoundBuild::new(&show, &ep.rec, &ep.ff).expect("the build starts");
    let started = Instant::now();
    let mix = build.score().expect("the score builds");
    println!("score (test profile): {:.2} s", started.elapsed().as_secs_f64());
    let pre = Vectors::open(&format!("{DIR}/premaster.npz"));
    let bus_order: Vec<String> = mix.bus_names().iter().map(|b| b.to_string()).collect();
    let mut windows = BTreeMap::new();
    let mut rms = BTreeMap::new();
    let mut len = 0;
    for bus in &bus_order {
        let x = mix.bus(bus).expect("a bus the mix lists");
        len = x.len();
        for (c, case) in pre.index.cases.iter().enumerate() {
            let (i0, n) = span(&case.args);
            windows.insert((c, bus.clone()), x[i0..i0 + n].to_vec());
        }
        rms.insert(bus.clone(), block_db(x));
    }
    Scored {
        bus_order,
        len,
        windows,
        rms,
        notes: build.ctx.first_notes().expect("the note log"),
        noise: build.ctx.noise().expect("the noise stream"),
    }
});

// ---------------------------------------------------------------- reading the files

fn json(rel: &str) -> Value {
    let path = reference(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `(i0, n)` of a window case.
fn span(args: &Value) -> (usize, usize) {
    let whole = |key: &str| {
        args[key]
            .as_u64()
            .unwrap_or_else(|| panic!("the case has no whole number {key:?}: {args}")) as usize
    };
    (whole("i0"), whole("n"))
}

/// A float32 `(n, 2)` reference window as float64 frames.
fn window(v: &Vectors, case: &bf_testdata::Case, bus: &str, n: usize) -> Vec<[f64; 2]> {
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
fn window_db(got: &[[f64; 2]], want: &[[f64; 2]]) -> (f64, f64) {
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

fn db(x: f64) -> f64 {
    if x == 0.0 {
        f64::NEG_INFINITY
    } else {
        20.0 * bf_core::dmath::log10(x)
    }
}

fn show_db(v: f64) -> String {
    if v == f64::NEG_INFINITY {
        "-inf dBFS (the same float32)".to_string()
    } else {
        format!("{v:.1} dBFS")
    }
}

/// `rms.json`'s formula: `20 * log10(sqrt(mean(x ** 2)) + 1e-12)` over both channels of each 100 ms block; the
/// last block is what is left.
fn block_db(x: &[[f64; 2]]) -> Vec<f64> {
    x.chunks(BLOCK)
        .map(|block| {
            let flat = block.as_flattened();
            let mean = flat.iter().map(|v| v * v).sum::<f64>() / flat.len() as f64;
            20.0 * bf_core::dmath::log10(mean.sqrt() + 1e-12)
        })
        .collect()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap_or_else(|| panic!("{v} is not a list"))
        .iter()
        .map(|x| x.as_f64().unwrap_or_else(|| panic!("{x} is not a number")))
        .collect()
}

/// The buses `exceptions.toml` lists for `episode`, each with its reason.
fn exceptions(episode: &str) -> BTreeMap<String, String> {
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
fn profile(what: &str, got: &[f64], want: &[f64], failures: &mut Vec<String>) -> (f64, usize) {
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
fn kit_wav() -> Vec<[i16; 2]> {
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

fn rms_i16(x: &[[i16; 2]]) -> f64 {
    let flat = x.as_flattened();
    let sum: f64 = flat.iter().map(|&v| f64::from(v) * f64::from(v)).sum();
    db((sum / flat.len() as f64).sqrt() / 32767.0)
}

// ---------------------------------------------------------------- the stems and the master

#[test]
fn stem_windows() {
    let st = &*BUILT;
    let v = Vectors::open(&format!("{DIR}/stems.npz"));
    assert_eq!(v.index.cases.len(), WINDOWS, "the windows of stems.npz");
    let buses: Vec<&str> = v.index.raw["buses"]
        .as_array()
        .expect("the buses")
        .iter()
        .map(|b| b.as_str().expect("a bus name"))
        .collect();
    assert_eq!(buses, BUSES);
    assert_eq!(
        st.stems.keys().map(String::as_str).collect::<Vec<_>>(),
        {
            let mut sorted = BUSES.to_vec();
            sorted.sort_unstable();
            sorted
        },
        "the stems of the soundtrack"
    );
    let excepted = exceptions("flute_vs_trombone");
    for bus in excepted.keys() {
        assert!(
            BUSES.contains(&bus.as_str()),
            "exceptions.toml lists {bus:?}, which is no bus"
        );
    }
    let silent: Vec<(String, String)> = v.index.raw["silent"]
        .as_array()
        .expect("the silent windows")
        .iter()
        .map(|p| (p[0].as_str().unwrap().to_string(), p[1].as_str().unwrap().to_string()))
        .collect();

    let mut failures = Vec::new();
    let mut worst: BTreeMap<&str, (f64, f64, String)> = BTreeMap::new();
    let mut heard: BTreeMap<&str, usize> = BTreeMap::new();
    for case in &v.index.cases {
        let (i0, n) = span(&case.args);
        for bus in BUSES {
            let want = window(&v, case, bus, n);
            let got = &st.stems[bus][i0..i0 + n];
            let quiet = want.as_flattened().iter().all(|x| *x == 0.0);
            assert_eq!(
                quiet,
                silent.contains(&(case.id.clone(), bus.to_string())),
                "{} {bus}: the sidecar's list of silent windows",
                case.id
            );
            if !quiet {
                *heard.entry(bus).or_default() += 1;
            }
            let (rounded, raw) = window_db(got, &want);
            let bar = if excepted.contains_key(bus) { DB_EXCEPTION } else { DB };
            if rounded.is_nan() || rounded >= bar {
                let first = got
                    .as_flattened()
                    .iter()
                    .zip(want.as_flattened())
                    .position(|(g, w)| (f64::from(*g as f32) - w).abs() >= 1e-6)
                    .map_or("none above -120 dBFS".to_string(), |i| {
                        format!("sample {} of the bus", i0 + i / 2)
                    });
                failures.push(format!(
                    "{} ({}) {bus}: {}, the bar is {bar} dBFS (first: {first})",
                    case.id,
                    case.args["t"],
                    show_db(rounded)
                ));
            }
            let entry = worst
                .entry(bus)
                .or_insert((f64::NEG_INFINITY, f64::NEG_INFINITY, String::new()));
            if rounded > entry.0 || entry.2.is_empty() {
                (entry.0, entry.2) = (rounded, case.id.clone());
            }
            entry.1 = entry.1.max(raw);
        }
    }
    println!("stems.npz: {} windows x {} buses", v.index.cases.len(), BUSES.len());
    for bus in BUSES {
        let (rounded, raw, id) = &worst[bus];
        println!(
            "    {bus}: worst {} ({id}); against the float32 reference without rounding {raw:.1} dBFS; heard in \
             {} window(s){}",
            show_db(*rounded),
            heard.get(bus).copied().unwrap_or(0),
            excepted
                .get(bus)
                .map_or(String::new(), |why| format!("; EXCEPTION: {why}"))
        );
    }
    // Every bus sounds in at least one window: a silent reference would prove nothing.
    for bus in BUSES {
        assert!(
            heard.get(bus).copied().unwrap_or(0) > 0,
            "{bus} is silent in every window"
        );
    }
    assert!(
        failures.is_empty(),
        "{} window(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // An exception that is not needed any more must go.
    for (bus, why) in &excepted {
        assert!(
            worst[bus.as_str()].0 >= DB,
            "{bus} is listed in exceptions.toml ({why}) but is within {DB} dBFS: remove the entry"
        );
    }
}

#[test]
fn stem_rms_profile() {
    let st = &*BUILT;
    let rms = json(&format!("{DIR}/rms.json"));
    assert_eq!(rms["block"], BLOCK);
    assert_eq!(
        (rms["stems"]["length"].as_u64(), rms["stems"]["blocks"].as_u64()),
        (Some(NA as u64), Some(611))
    );
    let db_of = &rms["stems"]["db"];
    assert_eq!(db_of.as_object().expect("the profiles").len(), BUSES.len() + 1);
    let mut failures = Vec::new();
    println!("rms.json: 611 blocks of 100 ms for each stem and the master");
    for bus in BUSES {
        let (worst, compared) = profile(bus, &block_db(&st.stems[bus]), &floats(&db_of[bus]), &mut failures);
        println!("    {bus}: worst {worst:.2e} dB over {compared} block(s) above {RMS_FLOOR} dBFS");
        assert!(compared > 0, "{bus}: no block above {RMS_FLOOR} dBFS");
    }
    let (worst, compared) = profile(
        "master",
        &block_db(&st.master),
        &floats(&db_of["master"]),
        &mut failures,
    );
    println!("    master: worst {worst:.2e} dB over {compared} block(s)");
    assert_eq!(compared, 611, "the master is never below {RMS_FLOOR} dBFS");
    assert!(
        failures.is_empty(),
        "{} block(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn length_is_whole_aac_frames() {
    let st = &*BUILT;
    assert_eq!(st.master.len(), 2_928_640);
    assert_eq!(st.master.len(), NA);
    assert_eq!(NA % 1024, 0);
    assert_eq!(NA / 1024, 2860);
    for (bus, stem) in &st.stems {
        assert_eq!(stem.len(), NA, "{bus}");
    }
}

#[test]
fn master_matches() {
    let st = &*BUILT;
    let wav = kit_wav();
    assert_eq!(wav.len(), NA, "audio.wav's length");
    assert_eq!(st.master.len(), wav.len());
    let (mut largest, mut differing, mut sum) = (0i32, 0usize, 0.0f64);
    let mut first = None;
    for (i, (ours, kit)) in st.master.iter().zip(&wav).enumerate() {
        for c in 0..2 {
            let d = (i32::from(sample_i16(ours[c])) - i32::from(kit[c])).abs();
            if d > 0 {
                differing += 1;
                first.get_or_insert((i, c));
            }
            largest = largest.max(d);
            sum += f64::from(d) * f64::from(d);
        }
    }
    let rms = db((sum / (2 * NA) as f64).sqrt() / 32767.0);
    println!(
        "audio.wav: largest difference {largest} LSB; {differing} of {} samples differ (first: {first:?}); RMS of \
         the difference {}",
        2 * NA,
        if rms == f64::NEG_INFINITY {
            "-inf dBFS (none)".to_string()
        } else {
            format!("{rms:.1} dBFS")
        }
    );
    assert!(largest <= 3, "a sample differs by {largest} LSB");
    assert!(rms < -90.0, "the RMS of the difference is {rms:.1} dBFS");
    // The float master against the kit's float master, block by block, is in `stem_rms_profile`.
}

#[test]
fn loops_cleanly() {
    let st = &*BUILT;
    let wav = kit_wav();
    // 4 ms at each end of the loop.
    let n = SR * 4 / 1000;
    let ours: Vec<[i16; 2]> = st.master.iter().map(|f| [sample_i16(f[0]), sample_i16(f[1])]).collect();
    for (what, range) in [("the first 4 ms", 0..n), ("the last 4 ms", NA - n..NA)] {
        let (got, want) = (&ours[range.clone()], &wav[range]);
        // The same samples as the kit's file (a sample may sit on a rounding edge: 1 LSB).
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            for c in 0..2 {
                assert!(
                    (i32::from(g[c]) - i32::from(w[c])).abs() <= 1,
                    "{what}, sample {i}, channel {c}: {} against the kit's {}",
                    g[c],
                    w[c]
                );
            }
        }
        // No fade beyond what the kit's file has: as loud as the kit there, and not quiet.
        let (level, kit) = (rms_i16(got), rms_i16(want));
        println!("{what}: {level:.3} dBFS, the kit's file {kit:.3} dBFS");
        assert!(
            (level - kit).abs() < RMS_DB,
            "{what}: {level:.3} dBFS, the kit has {kit:.3}"
        );
        assert!(level > -30.0, "{what} is at {level:.1} dBFS: faded out");
    }
    // What the score leaves past the loop point is folded onto the start: it is in no bus after sample NA, and
    // the kit's `tail` window has sound in it (so the fold has something to do).
    let pre = Vectors::open(&format!("{DIR}/premaster.npz"));
    let tail = pre.index.cases.last().expect("the tail window");
    assert_eq!(span(&tail.args), (NA, SR / 4));
    let folded: f64 = BUSES
        .iter()
        .flat_map(|bus| window(&pre, tail, bus, SR / 4))
        .map(|f| f[0].abs() + f[1].abs())
        .sum();
    assert!(folded > 1.0, "nothing lies past the loop point in the kit's mix");
}

// ---------------------------------------------------------------- the score

#[test]
fn note_order_matches() {
    let scored = &*SCORED;
    let calls = json(&format!("{DIR}/note_calls.json"));
    let want = calls["calls"].as_array().expect("the calls");
    assert_eq!((want.len(), calls["first_time"].as_u64()), (174, Some(174)));
    let name = |c: &NoteCall| format!("{} {} dur {} cap {} vel {}", c.name, c.midi, c.dur, c.cap, c.vel);
    for (i, w) in want.iter().enumerate() {
        let Some(g) = scored.notes.get(i) else {
            panic!(
                "the score made {} first-time notes, the kit {} (the kit's next: {w})",
                scored.notes.len(),
                want.len()
            );
        };
        let same = g.name == w["name"].as_str().expect("a name")
            && Some(g.midi) == w["midi"].as_i64()
            && g.dur.to_bits() == hex_arg(&w["hex"]["dur"]).to_bits()
            && g.cap.to_bits() == hex_arg(&w["hex"]["cap"]).to_bits()
            && g.vel.to_bits() == hex_arg(&w["hex"]["vel"]).to_bits();
        assert!(
            same,
            "first-time note {i}: the score made {}, the kit made {} {} dur {} cap {} vel {}{}",
            name(g),
            w["name"],
            w["midi"],
            w["dur"],
            w["cap"],
            w["vel"],
            if i > 0 {
                format!(" (the note before: {})", name(&scored.notes[i - 1]))
            } else {
                String::new()
            }
        );
    }
    assert_eq!(scored.notes.len(), want.len(), "first-time notes after the kit's last");
    // The noise stream ends where the kit's did: every FLUTE note drew what the kit's drew.
    let end: u128 = calls["end"]
        .as_str()
        .expect("the end state")
        .parse()
        .expect("a 128-bit number");
    let (mut ours, mut kit) = (scored.noise.0.clone(), Generator::from_state(end, PpNoise::START_INC));
    assert!(
        (0..4).all(|_| ours.random().to_bits() == kit.random().to_bits()),
        "the noise stream does not end where the kit's does"
    );
    println!("note_calls.json: {} first-time notes in the kit's order", want.len());
}

#[test]
fn premaster_windows() {
    let scored = &*SCORED;
    let v = Vectors::open(&format!("{DIR}/premaster.npz"));
    assert_eq!(v.index.cases.len(), WINDOWS);
    assert_eq!(
        scored.bus_order, BUSES,
        "the order the score first touches the buses in"
    );
    assert_eq!(
        Some(scored.len as u64),
        v.index.raw["length"].as_u64(),
        "the length of a bus"
    );
    let mut failures = Vec::new();
    println!("premaster.npz: {} windows x {} buses", v.index.cases.len(), BUSES.len());
    for bus in BUSES {
        let (mut worst, mut at) = (f64::NEG_INFINITY, String::new());
        for (c, case) in v.index.cases.iter().enumerate() {
            let (_, n) = span(&case.args);
            let (rounded, _) = window_db(&scored.windows[&(c, bus.to_string())], &window(&v, case, bus, n));
            if rounded.is_nan() || rounded >= DB {
                failures.push(format!("{} {bus}: {}", case.id, show_db(rounded)));
            }
            if rounded > worst || at.is_empty() {
                (worst, at) = (rounded, case.id.clone());
            }
        }
        println!("    {bus}: worst {} ({at})", show_db(worst));
    }
    let rms = json(&format!("{DIR}/rms.json"));
    for bus in BUSES {
        let want = floats(&rms["premaster"]["db"][bus]);
        assert_eq!(want.len(), 710);
        let (worst, compared) = profile(&format!("premaster {bus}"), &scored.rms[bus], &want, &mut failures);
        println!("    {bus}: 100 ms blocks, worst {worst:.2e} dB over {compared}");
    }
    assert!(
        failures.is_empty(),
        "{} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------- what a fight file can do wrong

#[test]
fn the_registry_lists_the_themes() {
    assert_eq!(theme_names(), ["big_top"]);
    assert_eq!(validate(&EPISODE.ff), Ok(()));
}

#[test]
fn unknown_theme_is_an_error() {
    let mut ff = EPISODE.ff.clone();
    ff.music.theme = "polka".into();
    let want = r#"music.theme: "polka" is not a theme"#;
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
}

#[test]
fn unvoiced_chord_is_an_error() {
    let mut ff = EPISODE.ff.clone();
    ff.music.chord_loop[4] = "F#m".into();
    let want = r#"music.chord_loop[4]: big_top has no voicing for "F#m""#;
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
    // The intro's chords are checked too, and before the loop's.
    ff.music.chords_intro[1] = "G7".into();
    assert_eq!(
        validate(&ff).unwrap_err().to_string(),
        r#"music.chords_intro[1]: big_top has no voicing for "G7""#
    );
    // What the fight file's own checks refuse comes first, in their words.
    ff.music.chord_loop.pop();
    assert_eq!(
        validate(&ff).unwrap_err().to_string(),
        "music.chord_loop: 8 chords needed, found 7"
    );
}

#[test]
fn a_fighter_without_a_part_is_an_error() {
    // PIANO is in the roster, so the fight file is valid, but its part in the score is not ported.
    let mut ff = EPISODE.ff.clone();
    ff.fighters[0] = "PIANO".into();
    ff.music.lines.clear();
    ff.music.solos.clear();
    ff.music.title_hits.clear();
    ff.music.roles.clear();
    let mut rec = EPISODE.rec.clone();
    rec.names[0] = "PIANO".into();
    // The app can refuse the file before it renders anything: `validate` says what the build would.
    let want = "fighters[0]: PIANO has no part in the score yet";
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&rec, &ff).unwrap_err().to_string(), want);
    // The other corner too.
    let mut ff = EPISODE.ff.clone();
    ff.fighters[1] = "PIANO".into();
    ff.music.lines.clear();
    ff.music.solos.clear();
    ff.music.title_hits.clear();
    ff.music.roles.clear();
    assert_eq!(
        validate(&ff).unwrap_err().to_string(),
        "fighters[1]: PIANO has no part in the score yet"
    );
}

#[test]
fn unknown_arrangement_is_an_error() {
    let mut ff = EPISODE.ff.clone();
    ff.music.arrangement = "jazz".into();
    // `FightFile::validate` knows the arrangements already, in its words.
    let want = r#"music.arrangement: "jazz" is not an arrangement (this app has: classic)"#;
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
}

#[test]
fn empty_title_hits_are_an_error() {
    // Every part plays its title hits on the VS card (the first and the last note): an empty list cannot be
    // played (Python raises an IndexError), whoever wins.
    for name in ["FLUTE", "TROMBONE"] {
        let mut ff = EPISODE.ff.clone();
        ff.music.title_hits.insert(name.into(), Vec::new());
        let want = format!("music.title_hits.{name}: the title hits are an empty list");
        assert_eq!(validate(&ff).unwrap_err().to_string(), want);
        assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
    }
}

#[test]
fn a_fight_without_a_ko_is_an_error() {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/no_ko.bfrec")).expect("the recording reads");
    let err = build_soundtrack(&rec, &EPISODE.ff).unwrap_err();
    assert!(matches!(err, ContentError::Show(_)), "{err:?}");
    assert_eq!(err.to_string(), "the fight has no K.O. (no winner and K.O. time)");
    // Another fight's recording.
    let mut ff = EPISODE.ff.clone();
    ff.fighters.reverse();
    let err = build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string();
    assert!(err.starts_with("the recording's fighters"), "{err}");
}

#[test]
fn a_panic_in_the_sound_code_is_an_error() {
    // A recording whose HP is above 100: the note thinning cannot take it (Python raises there, the port
    // panics), and the build must hand that back as an error, not take its caller down.
    let mut rec = EPISODE.rec.clone();
    let Some(Array::F64 { data, .. }) = rec.arrays.get_mut("HP") else {
        panic!("the recording has no HP");
    };
    data.fill(150.0);
    let err = build_soundtrack(&rec, &EPISODE.ff).unwrap_err();
    assert!(matches!(err, ContentError::Sound(_)), "{err:?}");
    let text = err.to_string();
    assert!(
        text.starts_with("the soundtrack could not be built: ") && text.contains("hp01 is 1.5, above 1"),
        "{text}"
    );
    // A recording that lacks what a part reads is an error too.
    let mut rec = EPISODE.rec.clone();
    rec.arrays.remove("HA");
    let text = build_soundtrack(&rec, &EPISODE.ff).unwrap_err().to_string();
    assert!(text.contains("HA"), "{text}");
}

// ---------------------------------------------------------------- other tempos

/// `rec` relabelled as fought on the grid of `ff`: its `rules.bpm`, `rules.beat`, `rules.t_fight` and `f0` are the
/// grid's, everything else (the run itself) is as it was.
fn on_grid(rec: &Recording, ff: &FightFile) -> Recording {
    let grid = ff.grid();
    let mut rec = rec.clone();
    for (key, v) in [("bpm", grid.bpm), ("beat", grid.beat), ("t_fight", grid.t_fight)] {
        rec.rules.insert(key.to_string(), EvValue::Float(v));
    }
    rec.f0 = bf_core::py::round_half_even(grid.t_fight * f64::from(rec.fps)) as usize;
    rec
}

/// What the kit made of the same run at another tempo: the bars, each stem's energy (the sum of its squared
/// samples, `float.hex()`), the master's, and the master's level in six 100 ms blocks.
struct OtherTempo {
    bpm: f64,
    bars: usize,
    energy: [(&'static str, &'static str); 7],
    master: &'static str,
    blocks: [(usize, f64); 6],
}

/// The fight file at `tempo_bpm = 120` and `150` (30 and 38 bars instead of 34): the bed and both parts follow
/// the grid, the soundtrack is still whole AAC frames, and it is the kit's.
///
/// The recording is the episode's own, fought at 132: a fight belongs to the tempo it was fought at (the engine
/// strikes on the beat), so a real video at another tempo needs a fight simulated at that tempo, and
/// `build_soundtrack` refuses the episode's recording under these fight files (`Show::new` compares the grids).
/// The kit has no such check: when an episode's `BPM` is changed after its run was made it plays the same run on
/// the new grid (it caches a run by its seed alone). The numbers are from exactly that, a copy of the example's
/// config with `BPM` replaced and one chord per bar of the new grid (the intro, then the loop over and over, as
/// `bf_core::harmony::chords_for` lays them out), through `kit.music.score()` and `master()`. Beyond these
/// numbers the kit's 16-bit master was compared once, by hand: the same samples at both tempos.
///
/// To compare with that, the parts are built through [`SoundBuild`] directly, from the episode's run relabelled
/// as fought on the new grid ([`on_grid`]): the score reads the run and the fight file's grid, never the
/// recording's own `rules` or `f0`, so this is the kit's stale run on the new grid, sample for sample.
#[test]
fn other_tempos_build() {
    #[rustfmt::skip]
    let cases = [
        OtherTempo {
            bpm: 120.0,
            bars: 30,
            energy: [
                ("drums", "0x1.ab892805681c8p+14"), ("calliope", "0x1.9721312b4ccf2p+13"),
                ("fx", "0x1.a209ec45f1e42p+12"), ("flute", "0x1.22d4facae6ebcp+15"),
                ("trombone", "0x1.81c9598f3270ap+16"), ("glock", "0x1.0896157e7479cp+5"),
                ("solo", "0x1.3f5e9d0f500f0p+13"),
            ],
            master: "0x1.421f3a1b2a7e6p+18",
            blocks: [
                (0, -10.268382592897261), (37, -11.381821917778698), (250, -11.985698527178094),
                (500, -9.95513838632431), (520, -12.373479082973988), (610, -11.433604111717937),
            ],
        },
        OtherTempo {
            bpm: 150.0,
            bars: 38,
            energy: [
                ("drums", "0x1.14e1452766800p+15"), ("calliope", "0x1.039ef06c140ccp+14"),
                ("fx", "0x1.b04f4f0b10994p+12"), ("flute", "0x1.89c3a73cce303p+15"),
                ("trombone", "0x1.77d0d515f4c7ep+16"), ("glock", "0x1.668d7d5125d2ep+5"),
                ("solo", "0x1.a9d36497fdaecp+13"),
            ],
            master: "0x1.6c9c189608643p+18",
            blocks: [
                (0, -9.91336100998854), (37, -11.77677961097145), (250, -12.612456059931102),
                (500, -9.16530826187168), (520, -12.02299463316066), (610, -11.23015564395333),
            ],
        },
    ];
    let energy = |x: &[[f64; 2]]| x.as_flattened().iter().map(|v| v * v).sum::<f64>();
    for case in cases {
        let mut ff = EPISODE.ff.clone();
        ff.tempo_bpm = case.bpm;
        assert_eq!(ff.grid().n_bars, case.bars, "tempo {}", case.bpm);
        assert_eq!(validate(&ff), Ok(()));
        let refused = build_soundtrack(&EPISODE.rec, &ff).unwrap_err();
        assert!(
            matches!(&refused, ContentError::Show(ShowError::Tempo(_))),
            "tempo {}: {refused:?}",
            case.bpm
        );
        let rec = on_grid(&EPISODE.rec, &ff);
        let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("tempo {}: {e}", case.bpm));
        let build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("tempo {}: {e}", case.bpm));
        let mix = build.score().unwrap_or_else(|e| panic!("tempo {}: {e}", case.bpm));
        let st = build.master(&mix);
        assert_eq!(st.master.len(), NA, "tempo {}", case.bpm);
        assert_eq!(st.stems.len(), BUSES.len(), "tempo {}", case.bpm);
        let mut worst = 0.0f64;
        for (bus, want) in case.energy {
            let stem = &st.stems[bus];
            assert_eq!(stem.len(), NA, "tempo {}: {bus}", case.bpm);
            let (got, want) = (energy(stem), bf_testdata::parse_hex_f64(want));
            let rel = ((got - want) / want).abs();
            assert!(
                rel < 1e-9,
                "tempo {}: {bus}'s energy is {got}, the kit's {want}",
                case.bpm
            );
            worst = worst.max(rel);
        }
        let (got, want) = (energy(&st.master), bf_testdata::parse_hex_f64(case.master));
        let rel = ((got - want) / want).abs();
        assert!(
            rel < 1e-9,
            "tempo {}: the master's energy is {got}, the kit's {want}",
            case.bpm
        );
        let levels = block_db(&st.master);
        assert_eq!(levels.len(), 611);
        for (b, want) in case.blocks {
            assert!(
                (levels[b] - want).abs() < 1e-6,
                "tempo {}: block {b} of the master is at {} dB, the kit's at {want}",
                case.bpm,
                levels[b]
            );
        }
        println!(
            "tempo {}: {} bars, {} samples; the stems' energies within {:.1e} of the kit's, the master's {rel:.1e}",
            case.bpm,
            case.bars,
            NA,
            worst.max(rel)
        );
    }
}
