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
//! What a fight file or a recording can do wrong is in `tests/errors.rs`, the same fight file at two other tempos
//! in `tests/tempos.rs`, the readers of the kit's files in `tests/common/mod.rs`.
//!
//! The tests print the worst difference per bus; run with `-- --nocapture` to see them.

mod common;

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::Instant;

use bf_content::{SoundBuild, build_soundtrack};
use bf_core::rng::Generator;
use bf_core::show::Show;
use bf_sound::master::Soundtrack;
use bf_sound::music::NoteCall;
use bf_sound::voice::PpNoise;
use bf_sound::wav::sample_i16;
use bf_sound::{NA, SR};
use bf_testdata::{Vectors, hex_arg};

use common::{
    BLOCK, BUSES, DIR, EPISODE, RMS_DB, RMS_FLOOR, block_db, db, exceptions, floats, json, kit_wav, profile, rms_i16,
    show_db, span, window, window_db,
};

/// Sample differences of a stem window: below this many dBFS.
const DB: f64 = -120.0;
/// The same for a bus listed in `exceptions.toml`.
const DB_EXCEPTION: f64 = -80.0;
/// The windows of `stems.npz` and `premaster.npz`: eight of 2.0 s and one of 0.25 s.
const WINDOWS: usize = 9;

// ---------------------------------------------------------------- the episode, built once

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
