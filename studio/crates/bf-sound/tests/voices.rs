//! The fighters' voices, the music helpers and the master against the kit: every case under
//! `studio/reference/sound/voices` and the episode's note and master files in
//! `studio/reference/sound/flute_vs_trombone`.
//!
//! - `FLUTE.npz`, `TROMBONE.npz`: `pp_bridge.voice` at 4 pitches x 2 velocities x 3 lengths, each started from the
//!   noise generator state stored with it. The references are float32 (their rounding sits near -150 dBFS); the
//!   bar is -120 dBFS on the float64 result. The two `raw44k` cases of each file (float64, before resampling) are
//!   compared too, and the generator must stand where Python left it after every call.
//! - `helpers.npz`: `inst_gain`, `note`, `fit`, `gliss`, `bend`, `vibrato`, `wah`, `colour`, `clank`, `hiss`,
//!   `roar`, `thunder`, `riser` in the order Python ran them, from the start state with an empty note cache:
//!   below -120 dBFS; `fit` is plain IEEE arithmetic and must give the same bits.
//! - `scalars.json`: `h01`, `Thinner`, `h01_37`, `thin_37`, `q16`: the same bits and the same decisions.
//! - `first_notes.npz`: the episode's first three notes (float64) replayed from the start state: the first note of
//!   an instrument measures its gain AFTER its own voice call.
//! - `note_calls.json`: all 174 first-time notes of the episode's score, in order: lengths, gains and the
//!   generator state after each; then every one again from the cache, which draws nothing.
//! - `master_case.npz`: a three-bus mix through `master`: below -110 dBFS in every window, and every 100 ms block
//!   of the whole 61 s within 1e-6 dB of Python's.
//!
//! Each test prints a summary line and the worst case of each function, how many float64 SHA-256 values it
//! matched exactly (a nice-to-know: `dmath` differs from numpy's maths in the last bit here and there), and the
//! timings. Run with `-- --nocapture` to see them.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

use bf_content::fighters;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::rng::Generator;
use bf_core::show::Show;
use bf_sound::dsp;
use bf_sound::master::{BUS, Levels, MASTER_CEILING, master};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{self, ColourArgs, MusicCtx, MusicError, Thinner};
use bf_sound::traits::{Bed, Sound};
use bf_sound::voice::PpNoise;
use bf_sound::{NA, SR};
use bf_testdata::{Vectors, db_max_diff, hex_arg, parse_hex_f64, reference, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Sample differences: below this many dBFS.
const DB: f64 = -120.0;
/// The master on the synthetic mix: below this many dBFS.
const DB_MASTER: f64 = -110.0;
/// A level (a gain, a master scale) against Python's: the largest relative difference allowed.
const LEVEL_REL: f64 = 1e-9;

// ---------------------------------------------------------------- the tally

/// The cases of one vector file: how many passed, the worst difference, and the worst of each function.
struct Tally {
    file: &'static str,
    cases: BTreeSet<String>,
    failed_cases: BTreeSet<String>,
    failures: Vec<String>,
    worst: Option<(f64, String)>,
    by_function: BTreeMap<String, (f64, String, usize)>,
}

fn show_db(v: f64) -> String {
    if v == f64::NEG_INFINITY {
        "-inf dBFS (the same bits)".to_string()
    } else {
        format!("{v:.1} dBFS")
    }
}

impl Tally {
    fn new(file: &'static str) -> Tally {
        Tally {
            file,
            cases: BTreeSet::new(),
            failed_cases: BTreeSet::new(),
            failures: Vec::new(),
            worst: None,
            by_function: BTreeMap::new(),
        }
    }

    /// One difference in dB of case `id`, for `function`; it passes strictly under `bar`.
    fn check(&mut self, id: &str, function: &str, value: f64, bar: f64) {
        self.cases.insert(id.to_string());
        if value.is_nan() || value >= bar {
            self.failed_cases.insert(id.to_string());
            self.failures.push(format!(
                "{id} {function}: {}, the bar is {}",
                show_db(value),
                show_db(bar)
            ));
        }
        if self.worst.as_ref().is_none_or(|(w, _)| value > *w) {
            self.worst = Some((value, id.to_string()));
        }
        let entry = self
            .by_function
            .entry(function.to_string())
            .or_insert((value, id.to_string(), 0));
        if value > entry.0 {
            (entry.0, entry.1) = (value, id.to_string());
        }
        entry.2 += 1;
    }

    /// The difference in dB of two sample arrays; a length mismatch is a failure, not a panic.
    fn db(&mut self, id: &str, function: &str, got: &[f64], want: &[f64], bar: f64) {
        if got.len() != want.len() {
            self.fail(
                id,
                function,
                format!("{} samples, the reference has {}", got.len(), want.len()),
            );
            return;
        }
        self.check(id, function, db_max_diff(got, want), bar);
    }

    /// The same bits (`-0.0` and `0.0` count as equal, as in `db_max_diff`).
    fn same(&mut self, id: &str, function: &str, got: &[f64], want: &[f64]) {
        if got.len() != want.len() {
            self.fail(
                id,
                function,
                format!("{} values, the reference has {}", got.len(), want.len()),
            );
            return;
        }
        let d = db_max_diff(got, want);
        if d != f64::NEG_INFINITY {
            self.fail(
                id,
                function,
                format!("{d:.1} dBFS of difference, the same bits are expected"),
            );
            return;
        }
        self.check(id, function, d, f64::INFINITY);
    }

    fn fail(&mut self, id: &str, function: &str, why: String) {
        self.cases.insert(id.to_string());
        self.failed_cases.insert(id.to_string());
        self.failures.push(format!("{id} {function}: {why}"));
    }

    /// Prints the summary and the per-function lines, then fails the test if the file had no case, if any case
    /// failed, or if one of the `required` functions had no check at all.
    fn finish(self, required: &[&str]) {
        let total = self.cases.len();
        let passed = total - self.failed_cases.len();
        let worst = match &self.worst {
            Some((v, id)) => format!("worst {} ({id})", show_db(*v)),
            None => "nothing measured".to_string(),
        };
        println!("{}: {passed}/{total}, {worst}", self.file);
        for (function, (v, id, n)) in &self.by_function {
            println!("    {function}: {n} check(s), worst {} ({id})", show_db(*v));
        }
        assert!(total > 0, "{}: no cases", self.file);
        assert!(
            self.failures.is_empty(),
            "{}: {} of {total} case(s) failed:\n{}",
            self.file,
            self.failed_cases.len(),
            self.failures.join("\n")
        );
        let missing: Vec<&str> = required
            .iter()
            .copied()
            .filter(|function| !self.by_function.contains_key(*function))
            .collect();
        assert!(missing.is_empty(), "{}: no case for {}", self.file, missing.join(", "));
    }
}

#[test]
#[should_panic(expected = "empty: no cases")]
fn a_file_without_cases_fails() {
    Tally::new("empty").finish(&[]);
}

#[test]
#[should_panic(expected = "partial: no case for b")]
fn a_function_without_a_case_fails() {
    let mut t = Tally::new("partial");
    t.check("c000", "a", -200.0, DB);
    t.finish(&["a", "b"]);
}

#[test]
#[should_panic(expected = "c000 a: -119.0 dBFS, the bar is -120.0 dBFS")]
fn a_difference_at_the_bar_fails() {
    let mut t = Tally::new("loud");
    t.check("c000", "a", -119.0, DB);
    t.finish(&[]);
}

/// The vector files under `studio/reference/sound/voices`: each has a test here that walks all of its cases.
const VOICE_FILES: [&str; 3] = ["FLUTE", "TROMBONE", "helpers"];

#[test]
fn the_folder_holds_only_the_files_tested() {
    let dir = bf_testdata::reference_dir().join("sound").join("voices");
    let mut found: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|x| x == "npz"))
        .map(|path| path.file_stem().expect("a file name").to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(found, VOICE_FILES, "the .npz files under {}", dir.display());
}

// ---------------------------------------------------------------- reading the files

fn json(rel: &str) -> Value {
    let path = reference(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A generator state or increment: a decimal string.
fn big(v: &Value) -> u128 {
    v.as_str()
        .unwrap_or_else(|| panic!("{v} is not a decimal string"))
        .parse()
        .unwrap_or_else(|e| panic!("{v} is not a 128-bit number: {e}"))
}

/// `{"state": ..., "inc": ...}` as a noise stream standing there.
fn noise_at(v: &Value) -> PpNoise {
    PpNoise::from_state(big(&v["state"]), big(&v["inc"]))
}

/// Whether the generator stands at `state` (on the shared stream's increment): the next draws are the same.
fn stands_at(g: &Generator, state: u128) -> bool {
    let (mut a, mut b) = (g.clone(), Generator::from_state(state, PpNoise::START_INC));
    (0..4).all(|_| a.random().to_bits() == b.random().to_bits())
}

fn hex_pairs(v: &Value) -> Vec<(f64, f64)> {
    v.as_array()
        .unwrap_or_else(|| panic!("{v} is not a list"))
        .iter()
        .map(|p| (hex_arg(&p[0]), hex_arg(&p[1])))
        .collect()
}

fn whole(v: &Value) -> u64 {
    v.as_u64().unwrap_or_else(|| panic!("{v} is not a whole number"))
}

/// The SHA-256 of the samples as little-endian float64, as the exporter takes it.
fn sha(x: &[f64]) -> String {
    let bytes: Vec<u8> = x.iter().flat_map(|v| v.to_le_bytes()).collect();
    Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn rel(got: f64, want: f64) -> f64 {
    ((got - want) / want).abs()
}

/// The `(n, 2)` array as stereo frames.
fn stereo(a: &bf_testdata::NpArray) -> Vec<[f64; 2]> {
    assert!(
        a.shape.len() == 2 && a.shape[1] == 2,
        "shape {:?} is not (n, 2)",
        a.shape
    );
    a.f64().as_chunks::<2>().0.to_vec()
}

// ---------------------------------------------------------------- the episode

/// The example episode: its recording (seed 392), its fight file and the show derived from them.
struct Episode {
    rec: Recording,
    ff: FightFile,
    show: Show,
}

fn episode() -> Episode {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
    Episode { rec, ff, show }
}

/// The music context of the episode, at the start of the noise stream with nothing cached.
fn context(ep: &Episode) -> MusicCtx<'_> {
    let cfg = |k: usize| {
        bf_content::roster()
            .iter()
            .find(|f| f.name == ep.show.names[k])
            .expect("the fighter is in the roster")
    };
    let voice = |k: usize| fighters::voice(&ep.show.names[k]).expect("the fighter has a voice");
    MusicCtx::new(&ep.show, &ep.rec, &ep.ff, [cfg(0), cfg(1)], [voice(0), voice(1)]).expect("the context builds")
}

// ---------------------------------------------------------------- the noise stream

#[test]
fn the_noise_stream_starts_where_python_starts() {
    let start = json("sound/pp_noise_start.json");
    assert_eq!(PpNoise::START_STATE, big(&start["state"]));
    assert_eq!(PpNoise::START_INC, big(&start["inc"]));
    assert_eq!(start["seed"], PpNoise::SEED);
    assert_eq!(
        (whole(&start["has_uint32"]), whole(&start["draws_before_start"])),
        (0, 0)
    );
    // It is a fresh `np.random.default_rng(77)`, and `nz(n)` is `_rng.normal(0, 1, n)`.
    let (mut a, mut b) = (PpNoise::start(), Generator::new(PpNoise::SEED));
    let mut want = vec![0.0; 1000];
    b.fill_normal(0.0, 1.0, &mut want);
    assert_eq!(a.nz(1000), want);
    assert!(a.nz(0).is_empty());
}

// ---------------------------------------------------------------- the voices

/// What the float32 references and the SHA-256 values say beyond the bar.
#[derive(Default)]
struct Fine {
    cases: usize,
    sha_same: usize,
    f32_same_cases: usize,
    f32_off_samples: usize,
    samples: usize,
}

impl Fine {
    /// One note against its float32 reference and the SHA-256 of Python's float64 samples.
    fn note(&mut self, got: &[f64], want32: &[f32], sha256: &Value) {
        self.cases += 1;
        if sha256.as_str() == Some(sha(got).as_str()) {
            self.sha_same += 1;
        }
        if got.len() == want32.len() {
            // Rounded to float32 as numpy rounds it (to nearest, ties to even), is it the reference's float32?
            let off = got.iter().zip(want32).filter(|&(&g, &w)| g as f32 != w).count();
            self.f32_off_samples += off;
            self.samples += got.len();
            if off == 0 {
                self.f32_same_cases += 1;
            }
        }
    }

    fn print(&self, file: &str) {
        println!(
            "    {file}: float64 SHA-256 equal in {} of {} case(s); the float32 form equal in {} of {} case(s) \
             ({} of {} samples round to another float32)",
            self.sha_same, self.cases, self.f32_same_cases, self.cases, self.f32_off_samples, self.samples
        );
    }
}

/// The note at 44,100 Hz, as the instrument's own synth makes it.
fn raw_44k(name: &str, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
    match name {
        "FLUTE" => fighters::flute::voice::synth(noise, midi, vel, dur),
        "TROMBONE" => fighters::trombone::voice::synth(midi, vel, dur),
        other => panic!("no 44,100 Hz synth known for {other}"),
    }
}

fn voice_file(name: &'static str) {
    let v = Vectors::open(&format!("sound/voices/{name}.npz"));
    let top = &v.index.raw;
    let voice = fighters::voice(name).unwrap_or_else(|| panic!("{name} has no voice"));
    assert_eq!(top["voice"], name);
    assert_eq!(
        (top["sample_rate"].as_u64(), top["voice_sample_rate"].as_u64()),
        (Some(48000), Some(44100))
    );
    assert_eq!(top["scene"], "modern");
    let (lo, hi) = voice.register();
    assert_eq!(
        top["register"],
        serde_json::json!([lo, hi]),
        "{name}: the register of pp_bridge.entrant"
    );

    let mut t = Tally::new(name);
    let mut fine = Fine::default();
    let mut seen = BTreeSet::new();
    let mut second = None;
    for (i, case) in v.index.cases.iter().enumerate() {
        let id = case.id.as_str();
        let extra = &top["cases"][i];
        if !case.call.starts_with("pp_bridge.voice(") || case.args["name"] != name {
            t.fail(id, &case.call, "a call this test does not know".into());
            continue;
        }
        let midi = case.args["midi"].as_i64().expect("midi") as f64;
        let (vel, dur) = (hex_arg(&case.args["vel"]), hex_arg(&case.args["dur"]));
        seen.insert((midi as i64, vel.to_bits(), dur.to_bits()));
        let want32 = v.array(case, "out").f32();
        let want: Vec<f64> = want32.iter().map(|&x| f64::from(x)).collect();
        assert_eq!(whole(&extra["samples"]) as usize, want.len(), "{id}: samples");

        let mut noise = noise_at(&extra["noise"]);
        let started = Instant::now();
        let got = voice.note(&mut noise, midi, vel, dur);
        let took = started.elapsed();
        if dur == 1.0 && second.is_none() {
            second = Some(took);
        }
        t.db(id, "voice (float32 reference)", &got, &want, DB);
        fine.note(&got, want32, &extra["sha256"]);
        // The call drew what Python's drew: the generator stands where Python's stood after it.
        let drawn: u64 = extra["draws"].as_array().expect("draws").iter().map(whole).sum();
        let after = big(&extra["noise_after"]);
        assert!(
            stands_at(&noise.0, after),
            "{id}: the generator is not where Python left it"
        );
        assert_eq!(drawn == 0, after == big(&extra["noise"]["state"]), "{id}: draws");

        if case.arrays.contains_key("raw44k") {
            let mut noise = noise_at(&extra["noise"]);
            let raw = raw_44k(name, &mut noise, midi, vel, dur);
            t.db(
                id,
                "synth at 44,100 Hz (float64)",
                &raw,
                v.array(case, "raw44k").f64(),
                DB,
            );
            assert_eq!(got.len(), (raw.len() * 160).div_ceil(147), "{id}: resampled length");
            // On Python's own 44,100 Hz samples the way to 48,000 Hz is the resampler alone.
            let up = bf_sound::voice::pp::to_48k(v.array(case, "raw44k").f64());
            t.db(id, "to_48k of Python's 44,100 Hz note", &up, &want, DB);
        }
    }
    t.finish(&[
        "voice (float32 reference)",
        "synth at 44,100 Hz (float64)",
        "to_48k of Python's 44,100 Hz note",
    ]);
    fine.print(name);
    assert_eq!(seen.len(), 24, "{name}: 4 pitches x 2 velocities x 3 lengths");
    println!(
        "    {name}: one note of 1.0 s took {:.1} ms",
        second.expect("a case of 1.0 s").as_secs_f64() * 1e3
    );
}

#[test]
fn flute_voice() {
    voice_file("FLUTE");
}

#[test]
fn trombone_voice() {
    voice_file("TROMBONE");
}

#[test]
fn a_flute_note_depends_on_where_the_noise_stands() {
    // The control of the replay: the same call from another state is another note.
    let voice = fighters::voice("FLUTE").expect("FLUTE has a voice");
    let a = voice.note(&mut PpNoise::start(), 76.0, 0.8, 0.2);
    let mut later = PpNoise::start();
    later.nz(1);
    let b = voice.note(&mut later, 76.0, 0.8, 0.2);
    assert!(db_max_diff(&a, &b) > -60.0, "{}", db_max_diff(&a, &b));
    // The trombone draws nothing: its note does not depend on the stream, and leaves it alone.
    let voice = fighters::voice("TROMBONE").expect("TROMBONE has a voice");
    let mut noise = PpNoise::start();
    let a = voice.note(&mut noise, 52.0, 0.8, 0.2);
    assert!(stands_at(&noise.0, PpNoise::START_STATE));
    assert_eq!(a, voice.note(&mut later, 52.0, 0.8, 0.2));
}

// ---------------------------------------------------------------- the helpers

/// The `ColourArgs` of a `colour` case.
fn colour_args(a: &Value) -> ColourArgs {
    ColourArgs {
        cut0: hex_arg(&a["cut0"]),
        cut_span: hex_arg(&a["cut_span"]),
        drive: hex_arg(&a["drive"]),
        low: a["low"].as_bool().expect("low"),
        low_cut: (hex_arg(&a["low_cut"][0]), hex_arg(&a["low_cut"][1])),
    }
}

const HELPERS: [&str; 13] = [
    "inst_gain",
    "note",
    "fit",
    "gliss",
    "bend",
    "vibrato",
    "wah",
    "colour",
    "clank",
    "hiss",
    "roar",
    "thunder",
    "riser",
];

#[test]
fn helpers() {
    let ep = episode();
    let ctx = context(&ep);
    let v = Vectors::open("sound/voices/helpers.npz");
    let top = &v.index.raw;
    assert_eq!(top["start"]["state"], PpNoise::START_STATE.to_string());
    // The tempo the cases were made at is the fight file's.
    let grid = &ep.show.grid;
    for (name, mine) in [("BEAT", grid.beat), ("BAR", grid.bar), ("S16", grid.s16)] {
        assert_eq!(
            parse_hex_f64(top["tempo"][name].as_str().expect("a tempo")),
            mine,
            "{name}"
        );
    }

    let mut t = Tally::new("helpers");
    for (i, case) in v.index.cases.iter().enumerate() {
        let id = case.id.as_str();
        let extra = &top["cases"][i];
        let helper = extra["helper"].as_str().expect("helper");
        let a = &case.args;
        // A case that drew noise names the state before it: the stream must stand there.
        let before = ctx.noise().unwrap();
        if let Some(noise) = extra.get("noise") {
            assert!(
                stands_at(&before.0, big(&noise["state"])),
                "{id}: the stream before the call"
            );
        }
        let failed = |e: MusicError| -> String { format!("refused: {e}") };
        let x = || v.array(case, "x").f64();
        let want = v.array(case, "out").f64();
        let out: Result<Vec<f64>, MusicError> = match helper {
            "inst_gain" => {
                let name = a["name"].as_str().expect("name");
                ctx.inst_gain(name).map(|g| {
                    let python = parse_hex_f64(extra["result"].as_str().expect("result"));
                    assert!(rel(g, python) <= LEVEL_REL, "{id}: gain {g} against {python}");
                    vec![g]
                })
            }
            "note" => {
                let name = a["name"].as_str().expect("name");
                let midi = a["midi"].as_i64().expect("midi") as f64;
                ctx.note(name, midi, hex_arg(&a["dur"]), hex_arg(&a["cap"]), hex_arg(&a["vel"]))
                    .map(|x| {
                        let python = parse_hex_f64(extra["gain"].as_str().expect("gain"));
                        let g = ctx.inst_gain(name).expect("the gain is measured by now");
                        assert!(rel(g, python) <= LEVEL_REL, "{id}: gain {g} against {python}");
                        x.to_vec()
                    })
            }
            "fit" => music::fit(x(), hex_arg(&a["cap"]), hex_arg(&a["fade"])),
            "gliss" => music::gliss(x(), hex_arg(&a["semis"]), hex_arg(&a["dur"])),
            "bend" => music::bend(x(), hex_arg(&a["semis"]), hex_arg(&a["t0"]), hex_arg(&a["t1"])),
            "vibrato" => music::vibrato(x(), hex_arg(&a["depth"]), hex_arg(&a["rate"]), hex_arg(&a["delay"])),
            "wah" => music::wah(x(), &hex_pairs(&a["opens"])),
            "colour" => {
                let k = whole(&a["k"]) as usize;
                let (tm, drive_k, args) = (hex_arg(&a["t"]), hex_arg(&a["drive_k"]), colour_args(a));
                // The two HP values Python read: the recording gives the same, bit for bit.
                let (own, other) = (hex_arg(&extra["hp"]["own"]), hex_arg(&extra["hp"]["other"]));
                assert_eq!(
                    (ctx.hp_at(k, tm).to_bits(), ctx.hp_at(1 - k, tm).to_bits()),
                    (own.to_bits(), other.to_bits()),
                    "{id}: hp_at"
                );
                ctx.colour(x(), k, tm, drive_k, &args).map(|(y, g, lead)| {
                    let gl = v.array(case, "gl").f64();
                    assert!(rel(g, gl[0]) <= LEVEL_REL, "{id}: gain {g} against {}", gl[0]);
                    assert_eq!(lead.to_bits(), gl[1].to_bits(), "{id}: lead");
                    // And from the two HP values alone, without the recording.
                    let again = music::colour_hp(x(), own, other, drive_k, &args).expect("colour_hp");
                    assert_eq!(again, (y.clone(), g, lead), "{id}: colour_hp");
                    y
                })
            }
            "clank" => music::clank(
                a["m"].as_i64().expect("m") as f64,
                whole(&a["seed"]),
                hex_arg(&a["dur"]),
                &hex_pairs(&a["decays"]),
            ),
            "hiss" => music::hiss(
                hex_arg(&a["dur"]),
                whole(&a["seed"]),
                hex_arg(&a["lo"]),
                hex_arg(&a["hi"]),
                hex_arg(&a["dec"]),
            ),
            "roar" => music::roar(hex_arg(&a["dur"]), whole(&a["seed"])),
            "thunder" => music::thunder(
                hex_arg(&a["dur"]),
                whole(&a["seed"]),
                a["crack"].as_bool().expect("crack"),
            ),
            "riser" => music::riser(hex_arg(&a["dur"]), hex_arg(&a["over"]), whole(&a["seed"])),
            other => {
                t.fail(id, other, "a helper this test does not know".into());
                continue;
            }
        };
        match out {
            Err(e) => t.fail(id, helper, failed(e)),
            Ok(got) if helper == "fit" => t.same(id, helper, &got, want),
            Ok(got) => t.db(id, helper, &got, want, DB),
        }
        // The stream after the call: where Python's stood, or where it was.
        let after = ctx.noise().unwrap();
        match extra.get("noise_after") {
            Some(state) => assert!(stands_at(&after.0, big(state)), "{id}: the stream after the call"),
            None => {
                let (mut a, mut b) = (before.0.clone(), after.0.clone());
                assert_eq!(a.random().to_bits(), b.random().to_bits(), "{id}: the stream moved");
            }
        }
    }
    t.finish(&HELPERS);
}

#[test]
fn defaults_are_pythons() {
    // The exporter passed each of these helpers its defaults in one case and stored them with it.
    let v = Vectors::open("sound/voices/helpers.npz");
    let args = |id: &str| -> &Value {
        &v.index
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("no case {id}"))
            .args
    };
    let same = |name: &str, mine: f64, python: &Value| {
        assert_eq!(
            mine.to_bits(),
            hex_arg(python).to_bits(),
            "{name}: Python's default is {python}"
        );
    };
    let note = args("c002");
    same("note.dur", music::NOTE_DUR, &note["dur"]);
    same("note.cap", music::NOTE_CAP, &note["cap"]);
    same("note.vel", music::NOTE_VEL, &note["vel"]);
    same("fit.fade", music::FIT_FADE, &args("c014")["fade"]);
    let vibrato = args("c026");
    same("vibrato.depth", music::VIBRATO_DEPTH, &vibrato["depth"]);
    same("vibrato.rate", music::VIBRATO_RATE, &vibrato["rate"]);
    same("vibrato.delay", music::VIBRATO_DELAY, &vibrato["delay"]);
    let colour = args("c030");
    assert_eq!(ColourArgs::default(), colour_args(colour));
    assert_eq!(music::COLOUR_DRIVE_K, 1.0);
    let clank = args("c036");
    assert_eq!(whole(&clank["seed"]), music::CLANK_SEED);
    same("clank.dur", music::CLANK_DUR, &clank["dur"]);
    assert_eq!(hex_pairs(&clank["decays"]), music::CLANK_DECAYS);
    let hiss = args("c039");
    same("hiss.dur", music::HISS_DUR, &hiss["dur"]);
    assert_eq!(whole(&hiss["seed"]), music::HISS_SEED);
    same("hiss.lo", music::HISS_LO, &hiss["lo"]);
    same("hiss.hi", music::HISS_HI, &hiss["hi"]);
    same("hiss.dec", music::HISS_DEC, &hiss["dec"]);
    assert_eq!(whole(&args("c042")["seed"]), music::ROAR_SEED);
    let thunder = args("c043");
    same("thunder.dur", music::THUNDER_DUR, &thunder["dur"]);
    assert_eq!(whole(&thunder["seed"]), music::THUNDER_SEED);
    assert_eq!(thunder["crack"], music::THUNDER_CRACK);
    let riser = args("c046");
    same("riser.over", music::RISER_OVER, &riser["over"]);
    assert_eq!(whole(&riser["seed"]), music::RISER_SEED);
    // The master's ceiling and the shared buses, from the master case's sidecar.
    let m = &Vectors::open("sound/flute_vs_trombone/master_case.npz").index.raw["master"];
    same("master.ceiling", MASTER_CEILING, &m["ceiling"]);
    for (bus, (gain, send)) in BUS {
        same(bus, gain, &m["levels"][bus]["gain"]);
        same(bus, send, &m["levels"][bus]["send"]);
    }
}

// ---------------------------------------------------------------- the number helpers

#[test]
fn scalars() {
    let doc = json("sound/voices/scalars.json");
    let ep = episode();
    let ctx = context(&ep);
    let floats = |v: &Value| -> Vec<f64> { v.as_array().expect("a list").iter().map(hex_arg).collect() };
    let bits = |x: &[f64]| -> Vec<u64> { x.iter().map(|v| v.to_bits()).collect() };
    let flags = |v: &Value| -> Vec<bool> {
        v.as_array()
            .expect("a list")
            .iter()
            .map(|b| b.as_bool().expect("a bool"))
            .collect()
    };
    let mut seen = BTreeSet::new();
    for case in doc["cases"].as_array().expect("cases") {
        let (id, call, a) = (&case["id"], case["call"].as_str().expect("call"), &case["args"]);
        let want = &case["result"];
        assert!(want.as_array().is_some_and(|w| !w.is_empty()), "{id}: no results");
        seen.insert(call);
        match call {
            "[kit.music.h01(slot, seed) for slot in slots]" => {
                let seed = hex_arg(&a["seed"]);
                let got: Vec<f64> = floats(&a["slots"]).iter().map(|&s| music::h01(&[s, seed])).collect();
                assert_eq!(bits(&got), bits(&floats(want)), "{id}");
            }
            "[kit.music.h01(x) for x in xs]" => {
                let got: Vec<f64> = floats(&a["xs"]).iter().map(|&x| music::h01(&[x])).collect();
                assert_eq!(bits(&got), bits(&floats(want)), "{id}");
            }
            "th = kit.music.Thinner(seed); [th.keep(slot, hp) for slot, hp in zip(slots, hps)]" => {
                let mut th = Thinner::new(hex_arg(&a["seed"]));
                let got: Vec<bool> = floats(&a["slots"])
                    .iter()
                    .zip(floats(&a["hps"]))
                    .map(|(&slot, hp)| th.keep(slot, hp))
                    .collect();
                let want = flags(want);
                assert_eq!(got, want, "{id}");
                // Something is dropped, and never two in a row.
                assert!(
                    want.contains(&false) && !want.windows(2).any(|w| w == [false, false]),
                    "{id}"
                );
            }
            "[kit.music.h01_37(slot, key) for slot in slots]" => {
                let key = hex_arg(&a["key"]);
                let got: Vec<f64> = floats(&a["slots"]).iter().map(|&s| music::h01_37(&[s, key])).collect();
                assert_eq!(bits(&got), bits(&floats(want)), "{id}");
            }
            "prev = False; for slot, hp: prev = kit.music.thin_37(slot, key, hp, prev)" => {
                let key = hex_arg(&a["key"]);
                let mut prev = false;
                let got: Vec<bool> = floats(&a["slots"])
                    .iter()
                    .zip(floats(&a["hps"]))
                    .map(|(&slot, hp)| {
                        prev = music::thin_37(slot, key, hp, prev);
                        prev
                    })
                    .collect();
                let want = flags(want);
                assert_eq!(got, want, "{id}");
                assert!(want.contains(&true), "{id}: nothing is dropped");
            }
            "[kit.music.q16(t) for t in ts]" => {
                let s16 = hex_arg(&a["S16"]);
                assert_eq!(s16.to_bits(), ep.show.grid.s16.to_bits(), "{id}: S16");
                let ts = floats(&a["ts"]);
                let got: Vec<f64> = ts.iter().map(|&t| music::q16(t, s16)).collect();
                assert_eq!(bits(&got), bits(&floats(want)), "{id}");
                let got: Vec<f64> = ts.iter().map(|&t| ctx.q16(t)).collect();
                assert_eq!(bits(&got), bits(&floats(want)), "{id}: through the context");
            }
            other => panic!("{id}: a call this test does not know: {other}"),
        }
    }
    assert_eq!(
        seen.len(),
        6,
        "h01 (two forms), Thinner, h01_37, thin_37, q16: {seen:?}"
    );
    println!(
        "scalars: {} case(s), every value the same bits",
        doc["cases"].as_array().expect("cases").len()
    );
}

// ---------------------------------------------------------------- the episode's notes

#[test]
fn first_notes() {
    let ep = episode();
    let ctx = context(&ep);
    let v = Vectors::open("sound/flute_vs_trombone/first_notes.npz");
    let top = &v.index.raw;
    assert_eq!(top["start"]["state"], PpNoise::START_STATE.to_string());
    let mut t = Tally::new("first_notes");
    let mut shas = (0, 0);
    let calls = json("sound/flute_vs_trombone/note_calls.json");
    assert_eq!(v.index.cases.len(), 3, "the first three notes of the score");
    for (i, case) in v.index.cases.iter().enumerate() {
        let id = case.id.as_str();
        let extra = &top["cases"][i];
        let a = &case.args;
        let name = a["name"].as_str().expect("name");
        let midi = a["midi"].as_i64().expect("midi") as f64;
        assert!(
            stands_at(&ctx.noise().unwrap().0, big(&extra["noise"]["state"])),
            "{id}: before"
        );
        let out = ctx
            .note(name, midi, hex_arg(&a["dur"]), hex_arg(&a["cap"]), hex_arg(&a["vel"]))
            .expect("the note");
        t.db(id, "note (float64)", &out, v.array(case, "out").f64(), DB);
        assert!(
            stands_at(&ctx.noise().unwrap().0, big(&extra["noise_after"])),
            "{id}: after"
        );
        let gain = parse_hex_f64(extra["gain"].as_str().expect("gain"));
        let mine = ctx.inst_gain(name).expect("the gain");
        assert!(rel(mine, gain) <= LEVEL_REL, "{id}: gain {mine} against {gain}");
        // The voice calls behind it, each from the state recorded with it: the note, then (first note only) the
        // level-matching note of inst_gain.
        let voice = fighters::voice(name).expect("a voice");
        let logged = &calls["calls"][i]["voice_calls"];
        let behind = extra["voice_calls"].as_array().expect("voice_calls").len();
        assert_eq!(
            behind,
            if i == 0 { 2 } else { 1 },
            "{id}: the note's voice call, and the gain note after the first"
        );
        for (j, call) in extra["voice_calls"].as_array().expect("voice_calls").iter().enumerate() {
            let role = if call["why"] == "note" { "voice" } else { "gain_voice" };
            let log = &logged[j];
            let mut noise = noise_at(&call["noise"]);
            let x = voice.note(
                &mut noise,
                log["midi"].as_i64().expect("midi") as f64,
                hex_arg(&log["hex"]["vel"]),
                hex_arg(&log["hex"]["dur"]),
            );
            t.db(id, &format!("{role} (float64)"), &x, v.array(case, role).f64(), DB);
            shas.1 += 1;
            if log["sha256"].as_str() == Some(sha(&x).as_str()) {
                shas.0 += 1;
            }
        }
    }
    t.finish(&["note (float64)", "voice (float64)", "gain_voice (float64)"]);
    println!(
        "    first_notes: float64 SHA-256 equal in {} of {} voice call(s)",
        shas.0, shas.1
    );

    // The trap: measuring the gain BEFORE the first note moves the noise under it, and the note comes out
    // different. (Python evaluates fit(voice(...)) before inst_gain(name).)
    let wrong = context(&ep);
    wrong.inst_gain("FLUTE").expect("the gain");
    let first = &v.index.cases[0];
    let a = &first.args;
    let out = wrong
        .note(
            "FLUTE",
            a["midi"].as_i64().expect("midi") as f64,
            hex_arg(&a["dur"]),
            hex_arg(&a["cap"]),
            hex_arg(&a["vel"]),
        )
        .expect("the note");
    let d = db_max_diff(&out, v.array(first, "out").f64());
    assert!(
        d > -60.0,
        "the gain note first gives the same note ({d:.1} dBFS): the order is not tested"
    );
}

#[test]
fn every_first_time_note_of_the_episode() {
    let ep = episode();
    let ctx = context(&ep);
    let doc = json("sound/flute_vs_trombone/note_calls.json");
    let calls = doc["calls"].as_array().expect("calls");
    // The episode's score makes 174 first-time notes, 95 of the FLUTE and 79 of the TROMBONE: an empty or
    // cut-short log must not pass for want of notes to replay.
    assert_eq!(calls.len(), 174, "the first-time notes in note_calls.json");
    let of = |name: &str| calls.iter().filter(|c| c["name"] == name).count();
    assert_eq!((of("FLUTE"), of("TROMBONE")), (95, 79));
    assert_eq!(calls.len() as u64, whole(&doc["first_time"]));
    assert_eq!(doc["start"], PpNoise::START_STATE.to_string());
    assert_eq!(doc["generator"]["inc"], PpNoise::START_INC.to_string());
    let args = |c: &Value| -> (String, f64, f64, f64, f64) {
        (
            c["name"].as_str().expect("name").to_string(),
            c["midi"].as_i64().expect("midi") as f64,
            hex_arg(&c["hex"]["dur"]),
            hex_arg(&c["hex"]["cap"]),
            hex_arg(&c["hex"]["vel"]),
        )
    };
    let started = Instant::now();
    let mut notes = Vec::new();
    let (mut sha_same, mut gain_worst) = (0, 0.0f64);
    for c in calls {
        let i = &c["i"];
        let (name, midi, dur, cap, vel) = args(c);
        assert!(
            stands_at(&ctx.noise().unwrap().0, big(&c["noise_before"])),
            "note {i}: before"
        );
        let x = ctx.note(&name, midi, dur, cap, vel).expect("the note");
        assert_eq!(x.len() as u64, whole(&c["samples"]), "note {i}: samples");
        assert!(x.iter().all(|v| v.is_finite()), "note {i}");
        assert!(
            stands_at(&ctx.noise().unwrap().0, big(&c["noise_after"])),
            "note {i}: it drew another number of normals"
        );
        let gain = parse_hex_f64(c["gain"].as_str().expect("gain"));
        gain_worst = gain_worst.max(rel(ctx.inst_gain(&name).expect("the gain"), gain));
        if c["sha256"].as_str() == Some(sha(&x).as_str()) {
            sha_same += 1;
        }
        notes.push(x);
    }
    let took = started.elapsed();
    assert!(
        stands_at(&ctx.noise().unwrap().0, big(&doc["end"])),
        "the end of the score"
    );
    assert!(gain_worst <= LEVEL_REL, "a gain is {gain_worst:e} from Python's");
    // The log of first-time notes is the file's, in order.
    let log = ctx.first_notes().unwrap();
    assert_eq!(log.len(), calls.len());
    for (mine, c) in log.iter().zip(calls) {
        let (name, midi, dur, cap, vel) = args(c);
        assert_eq!(
            (mine.name.as_str(), mine.midi as f64, mine.dur, mine.cap, mine.vel),
            (name.as_str(), midi, dur, cap, vel),
            "note {}",
            c["i"]
        );
    }
    // Asked for again, every note comes from the cache: the same samples, nothing drawn, nothing logged.
    let before = ctx.noise().unwrap();
    for (c, first) in calls.iter().zip(&notes) {
        let (name, midi, dur, cap, vel) = args(c);
        let again = ctx.note(&name, midi, dur, cap, vel).expect("the note");
        assert!(Arc::ptr_eq(&again, first), "note {}: synthesised twice", c["i"]);
    }
    let (mut a, mut b) = (before.0.clone(), ctx.noise().unwrap().0.clone());
    assert_eq!(a.random().to_bits(), b.random().to_bits(), "a cached note drew noise");
    assert_eq!(ctx.first_notes().unwrap().len(), calls.len());
    println!(
        "note_calls: {} first-time notes replayed in order in {:.2} s; every length and every generator state \
         as Python's; worst gain {gain_worst:.1e} rel; float64 SHA-256 equal for {sha_same} of {} notes",
        calls.len(),
        took.as_secs_f64(),
        calls.len()
    );
}

// ---------------------------------------------------------------- the master

/// A bed with the big top's levels and clash (`kit/moods/big_top.py`) and the reverb and compressor of the master
/// case; its sections play nothing.
struct CaseBed {
    reverb: (f64, f64, f64),
    comp: (f64, f64),
}

type Played = Result<(), MusicError>;

impl Bed for CaseBed {
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("drums", (1.0, 0.15)), ("calliope", (1.2, 0.45)), ("glock", (1.0, 0.5))]
    }
    fn reverb(&self) -> (f64, f64, f64) {
        self.reverb
    }
    fn comp(&self) -> (f64, f64) {
        self.comp
    }
    fn roar_gain(&self) -> f64 {
        0.15
    }
    fn doors_gain(&self) -> f64 {
        0.40
    }
    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }
    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 100.0, 0.10)
    }
    fn boom(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: f64) -> Played {
        Ok(())
    }
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        // mix.place('fx', self.M.clank(79, int(t * 3)), t, 0.04)
        let ring = music::clank(79.0, (t * 3.0) as u64, music::CLANK_DUR, &music::CLANK_DECAYS)?;
        mix.place("fx", &Sig::Mono(ring), t, 0.04, 0.0);
        Ok(())
    }
    fn title(&self, _: &MusicCtx, _: &mut Mix, _: [f64; 3]) -> Played {
        Ok(())
    }
    fn vs(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
        Ok(())
    }
    fn title_rest(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn count_in(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn fight_start(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn fight(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
        Ok(())
    }
    fn ko(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn stinger(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
        Ok(())
    }
    fn after_ko(&self, _: &MusicCtx, _: &mut Mix, _: usize, _: f64, _: f64) -> Played {
        Ok(())
    }
    fn victory(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
        Ok(())
    }
    fn last_bars(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
}

/// A fighter's part with the bus level its kit module sets, and no notes.
struct CaseSound {
    k: usize,
    bus_gain: f64,
    send: f64,
}

impl Sound for CaseSound {
    fn k(&self) -> usize {
        self.k
    }
    fn bus_gain(&self) -> f64 {
        self.bus_gain
    }
    fn send(&self) -> f64 {
        self.send
    }
}

/// `20 * log10(sqrt(mean(x ** 2)) + 1e-12)` of every 100 ms block, over both channels (the exporter's `block_db`).
fn block_db(x: &[[f64; 2]]) -> Vec<f64> {
    x.chunks(SR / 10)
        .map(|block| {
            let squares: Vec<f64> = block.as_flattened().iter().map(|v| v * v).collect();
            20.0 * bf_core::dmath::log10(dsp::mean(&squares).sqrt() + 1e-12)
        })
        .collect()
}

#[test]
fn master_case() {
    let v = Vectors::open("sound/flute_vs_trombone/master_case.npz");
    let top = &v.index.raw;
    let m = &top["master"];
    assert_eq!(whole(&top["na"]) as usize, NA);
    assert_eq!(whole(&top["sample_rate"]) as usize, SR);
    let bed = CaseBed {
        reverb: (
            hex_arg(&m["reverb"]["dur"]),
            hex_arg(&m["reverb"]["decay"]),
            hex_arg(&m["reverb"]["wet"]),
        ),
        comp: (hex_arg(&m["comp"]["thr"]), hex_arg(&m["comp"]["ex"])),
    };
    assert_eq!(whole(&m["reverb"]["seed"]), bf_sound::synth::REVERB_SEED);

    // The levels as the kit's master() builds them for the episode: the two fighters' buses (the values their
    // kit modules set), the shared buses, the bed's. They must be Python's, in Python's order.
    let ep = episode();
    let ctx = context(&ep);
    let level = |bus: &str| (hex_arg(&m["levels"][bus]["gain"]), hex_arg(&m["levels"][bus]["send"]));
    let sounds = [0, 1].map(|k| {
        let (bus_gain, send) = level(&ep.show.names[k].to_lowercase());
        CaseSound { k, bus_gain, send }
    });
    let levels = Levels::of(&ctx, [&sounds[0], &sounds[1]], &bed, &[]);
    let order: Vec<&str> = levels.iter().map(|(bus, _)| bus).collect();
    let kit_order = json("sound/flute_vs_trombone/stems.json");
    let kit_order: Vec<&str> = kit_order["master"]["send_order"]
        .as_array()
        .expect("send_order")
        .iter()
        .map(|b| b.as_str().expect("a bus"))
        .collect();
    assert_eq!(order, kit_order, "the order the reverb send adds the buses in");
    for (bus, mine) in levels.iter() {
        assert_eq!(mine, level(bus), "{bus}");
    }
    assert_eq!(levels.iter().count(), m["levels"].as_object().expect("levels").len());

    // The mix, as the sidecar says it was made.
    let mut mix = Mix::new(hex_arg(&top["mix_dur"]));
    for p in top["placements"].as_array().expect("placements") {
        let signal = v.npz.get(p["signal"].as_str().expect("signal")).f64().to_vec();
        let (bus, at) = (p["bus"].as_str().expect("bus"), hex_arg(&p["t"]));
        mix.place(bus, &Sig::Mono(signal), at, hex_arg(&p["gain"]), hex_arg(&p["pan"]));
    }
    let bus_order: Vec<&str> = m["bus_order"]
        .as_array()
        .expect("bus_order")
        .iter()
        .map(|b| b.as_str().expect("a bus"))
        .collect();
    assert_eq!(mix.bus_names(), bus_order);
    let sent: Vec<&str> = order.iter().copied().filter(|b| bus_order.contains(b)).collect();
    assert_eq!(serde_json::json!(sent), m["send_order"]);

    let started = Instant::now();
    let st = master(&mix, &levels, &bed, hex_arg(&m["ceiling"]));
    let took = started.elapsed();
    assert_eq!(st.master.len(), NA);
    assert_eq!(st.stems.keys().map(String::as_str).collect::<Vec<_>>(), bus_order);
    assert!(st.stems.values().all(|s| s.len() == NA));

    let mut all_roles: Vec<&str> = st.stems.keys().map(String::as_str).chain(["master"]).collect();
    all_roles.sort_unstable();
    let mut windows = 0;
    let mut t = Tally::new("master_case");
    for case in &v.index.cases {
        let id = case.id.as_str();
        if case.call == "the placed signals" {
            continue;
        }
        if case.call != "kit.music.master(mix): (m, stems)[i0:i0 + n]" {
            t.fail(id, &case.call, "a call this test does not know".into());
            continue;
        }
        let (i0, n) = (whole(&case.args["i0"]) as usize, whole(&case.args["n"]) as usize);
        windows += 1;
        let roles: Vec<&str> = case.arrays.keys().map(String::as_str).collect();
        assert_eq!(roles, all_roles, "{id}: the master and every stem");
        for role in case.arrays.keys() {
            let mine = if role == "master" { &st.master } else { &st.stems[role] };
            let function = if role == "master" { "master" } else { "stem" };
            let want = stereo(v.array(case, role));
            t.db(
                id,
                function,
                mine[i0..i0 + n].as_flattened(),
                want.as_flattened(),
                DB_MASTER,
            );
        }
    }
    t.finish(&["master", "stem"]);
    assert_eq!(windows, 3, "the start, the middle and the seam");

    // Every 100 ms block of the whole length against Python's levels (blocks under -200 dB are silence).
    let mut worst = 0.0f64;
    let profiles = top["db"].as_object().expect("db");
    let mut profiled: Vec<&str> = profiles.keys().map(String::as_str).filter(|r| *r != "about").collect();
    profiled.sort_unstable();
    assert_eq!(profiled, all_roles, "the level profiles: the master and every stem");
    for (role, want) in profiles {
        if role == "about" {
            continue;
        }
        let mine = block_db(if role == "master" { &st.master } else { &st.stems[role] });
        let want: Vec<f64> = want
            .as_array()
            .expect("levels")
            .iter()
            .map(|d| d.as_f64().expect("dB"))
            .collect();
        assert_eq!(want.len(), NA.div_ceil(SR / 10), "{role}: a level for every 100 ms");
        assert_eq!(mine.len(), want.len(), "{role}: blocks");
        for (b, (got, want)) in mine.iter().zip(&want).enumerate() {
            if *want > -200.0 {
                worst = worst.max((got - want).abs());
                assert!(
                    (got - want).abs() < 1e-6,
                    "{role} block {b}: {got} dB, Python has {want} dB"
                );
            } else {
                assert!(*got < -200.0, "{role} block {b}: {got} dB, Python has silence");
            }
        }
    }
    println!(
        "    master_case: every 100 ms block within {worst:.1e} dB of Python's; the master took {:.2} s",
        took.as_secs_f64()
    );
}

// ---------------------------------------------------------------- the base classes on the episode

/// A fighter that overrides nothing: `kit/fighters/base.py`'s `Sound(M, k)` itself.
struct Base(usize);

impl Sound for Base {
    fn k(&self) -> usize {
        self.0
    }
}

#[test]
fn the_base_sound_plays_as_base_py_does() {
    // What `base.Sound(M, 1)` gave for the episode on the reference PC, from the start state with nothing cached
    // (floats as float.hex()): `x = s.voice(52, 0.2504, 0.84)` asked for ('TROMBONE', 52, 0.25, 0.45, 0.8), gave
    // 21,600 samples with the peak below; `s.colour(x, 30.0, 0.8)` and `s.colour(x, 30.0, low=True)`.
    let voice_peak = parse_hex_f64("0x1.3f9287c636972p-2");
    let (colour_peak, gain, lead) = (
        parse_hex_f64("0x1.5617f05ad0c00p-1"),
        parse_hex_f64("0x1.0924939fb16a3p+0"),
        parse_hex_f64("0x1.a027525460bb8p-5"),
    );
    let low_peak = parse_hex_f64("0x1.4091f174a81d8p-1");

    let ep = episode();
    let ctx = context(&ep);
    let s = Base(1);
    assert_eq!(
        (s.name(&ctx), s.bus(&ctx), s.role(&ctx), s.enemy()),
        ("TROMBONE", "trombone".to_string(), "bass", 0)
    );
    assert_eq!((Base(0).bus(&ctx), Base(0).role(&ctx)), ("flute".to_string(), "melody"));
    assert_eq!((s.bus_gain(), s.send(), s.band()), (1.5, 0.35, (200.0, 4000.0)));
    assert_eq!((s.detune(&ctx, 12.0), s.detune(&ctx, 1.0)), (Ok(0.0), Ok(0.0)));
    let peak = |x: &[f64]| x.iter().fold(0.0f64, |p, v| p.max(v.abs()));

    let x = s.voice(&ctx, 52.0, 0.2504, 0.84).expect("the note");
    assert_eq!(x.len(), 21600);
    let first = &ctx.first_notes().unwrap()[0];
    assert_eq!(
        (first.name.as_str(), first.midi, first.dur, first.cap, first.vel),
        ("TROMBONE", 52, 0.25, 0.45, 0.8)
    );
    assert!(rel(peak(&x), voice_peak) <= LEVEL_REL, "{}", peak(&x));

    let (y, g, l) = s.colour(&ctx, &x, 30.0, 0.8, &s.colour_args()).expect("the tone");
    assert!(rel(peak(&y), colour_peak) <= LEVEL_REL, "{}", peak(&y));
    assert!(rel(g, gain) <= LEVEL_REL, "{g}");
    assert_eq!(l.to_bits(), lead.to_bits());
    // `low=True` over its own arguments, at the default drive.
    let low = ColourArgs {
        low: true,
        ..s.colour_args()
    };
    let (y, g, _) = s
        .colour(&ctx, &x, 30.0, bf_sound::traits::SOUND_COLOUR_DRIVE_K, &low)
        .expect("the tone");
    assert!(rel(peak(&y), low_peak) <= LEVEL_REL, "{}", peak(&y));
    assert!(rel(g, gain) <= LEVEL_REL, "{g}");
}

#[test]
fn shared_events_of_the_episode() {
    // What `kit.music.shared_events(mix)` put on an empty `Mix(T.DUR + 4.0)` for the episode on the reference PC
    // (144 rope bounces and 9 clashes up to the K.O.; no block, no parry): only the bus `fx`; its first and last
    // frames that are not silent and how many are not; its largest sample (by `np.abs(fx).argmax()`, a flat
    // index) and the sums of its squares and of its samples, left and right (floats as float.hex()).
    let (first, last, sounding) = (214635, 2405195, 1196885);
    let (peak_at, peak) = (3598299, parse_hex_f64("0x1.ff7ff8cb852adp-4"));
    let squares = ["0x1.bc49f484d54cap+7", "0x1.f685756519306p+7"].map(parse_hex_f64);
    let sums = ["0x1.74b3fe4a00208p+8", "0x1.8f71cf9c9ed30p+8"].map(parse_hex_f64);

    let ep = episode();
    let ctx = context(&ep);
    let bed = CaseBed {
        reverb: (2.0, 0.55, 0.22),
        comp: (0.15, 0.62),
    };
    let mut mix = Mix::new(ep.show.grid.dur + 4.0);
    music::shared_events(&ctx, &bed, &mut mix).expect("the events");
    assert_eq!(mix.bus_names(), ["fx"]);
    // It makes no note: the noise stream has not moved.
    assert!(stands_at(&ctx.noise().unwrap().0, PpNoise::START_STATE));
    let fx = mix.bus("fx").expect("fx");
    let loud: Vec<usize> = (0..fx.len()).filter(|&i| fx[i] != [0.0, 0.0]).collect();
    assert_eq!((loud[0], loud[loud.len() - 1], loud.len()), (first, last, sounding));
    let flat = fx.as_flattened();
    let at = (0..flat.len()).fold(0, |best, i| if flat[i].abs() > flat[best].abs() { i } else { best });
    assert_eq!(at, peak_at);
    assert!(rel(flat[at].abs(), peak) <= LEVEL_REL, "{}", flat[at]);
    for c in 0..2 {
        let channel: Vec<f64> = fx.iter().map(|f| f[c]).collect();
        let square: Vec<f64> = channel.iter().map(|v| v * v).collect();
        assert!(rel(dsp::sum(&square), squares[c]) <= LEVEL_REL, "channel {c}: squares");
        assert!(rel(dsp::sum(&channel), sums[c]) <= LEVEL_REL, "channel {c}: sum");
    }
}

// ---------------------------------------------------------------- notes below zero

/// The samples of `x` at the given places against what the kit printed there (`float.hex()`), to 1e-12.
fn samples_are(x: &[f64], want: &[(usize, &str)], what: &str) {
    for &(i, hex) in want {
        let python = parse_hex_f64(hex);
        assert!(
            (x[i] - python).abs() < 1e-12,
            "{what}: sample {i} is {}, Python has {python}",
            x[i]
        );
    }
}

#[test]
fn a_note_below_zero_is_pythons() {
    // `kit.music.note(name, midi, dur, cap, 0.8)` on the reference PC for a held length and for a cap below
    // zero, each from the start state with nothing cached. Python raises for none of them except where it says
    // so: a held length a little below zero is the note's release alone, and a cap below zero cuts that much off
    // the end of the note (the slice `x[:int(cap * SR)]`) before the fade.
    let ep = episode();
    let peak = |x: &[f64]| x.iter().fold(0.0f64, |p, v| p.max(v.abs()));
    let refused = |r: Result<Arc<[f64]>, MusicError>| r.expect_err("Python raises here").to_string();

    // ---- the TROMBONE (no noise: one context does for all)
    let ctx = context(&ep);
    // dur -0.1: 0.04 s of release (1,764 samples at 44.1 kHz, 1,920 at 48 kHz), padded to the cap.
    let x = ctx.note("TROMBONE", 52.0, -0.1, 0.3, 0.8).expect("a release alone");
    assert_eq!(x.len(), 14400);
    samples_are(
        &x,
        &[
            (100, "0x1.c6f5013bd0986p-13"),
            (1000, "0x1.aa3c9f75af71bp-9"),
            (1900, "0x1.caaa768668bdfp-10"),
            (1919, "0x1.218fea43200ccp-13"),
        ],
        "TROMBONE, dur -0.1",
    );
    assert!(x[1919] != 0.0 && x[1920..].iter().all(|v| *v == 0.0));
    assert!(rel(peak(&x), parse_hex_f64("0x1.513f6e29f32cfp-7")) <= LEVEL_REL);
    // dur -0.14: a note of no samples at all, which the cap pads to silence.
    let silent = ctx.note("TROMBONE", 52.0, -0.14, 0.3, 0.8).expect("silence");
    assert!(silent.len() == 14400 && silent.iter().all(|v| *v == 0.0));
    // dur -0.2: a negative number of samples. Python raises; nothing is synthesised.
    let e = refused(ctx.note("TROMBONE", 52.0, -0.2, 0.3, 0.8));
    assert_eq!(
        e,
        "note: TROMBONE has no note for a held length (dur) of -0.2 s: it would end before it starts"
    );
    // cap -0.01: the 16,320 samples of the note less 480, the last 1,920 of what is left faded.
    let x = ctx.note("TROMBONE", 52.0, 0.2, -0.01, 0.8).expect("a note cut short");
    assert_eq!(x.len(), 15840);
    samples_are(
        &x,
        &[
            (5000, "0x1.2eff86a1567bfp-2"),
            (13920, "-0x1.1a48cb1cc885ap-6"),
            (15000, "0x1.4614fa2e44490p-9"),
            (15838, "0x1.f6e98971d9bfep-19"),
            (15839, "0x0.0p+0"),
        ],
        "TROMBONE, cap -0.01",
    );
    assert!(rel(peak(&x), parse_hex_f64("0x1.3f9287c636972p-2")) <= LEVEL_REL);
    // A cap that leaves 3 samples (int(cap * SR) is -16317), one sample (-16319: Python raises, there is no
    // room for the fade), none (-16320), and less than none.
    let x = ctx
        .note("TROMBONE", 52.0, 0.2, parse_hex_f64("-0x1.5c1e098ead65bp-2"), 0.8)
        .expect("three samples");
    assert_eq!(x.len(), 3);
    samples_are(
        &x,
        &[(1, "0x1.f253ed4fa969bp-21"), (2, "0x1.0870e8b263de0p-18")],
        "TROMBONE, three samples",
    );
    let e = refused(ctx.note("TROMBONE", 52.0, 0.2, parse_hex_f64("-0x1.5c237fa89e60fp-2"), 0.8));
    assert!(
        e.starts_with("note: 1 sample(s) with a fade of 0.04 s leave no room for a fade"),
        "{e}"
    );
    for cap in [parse_hex_f64("-0x1.5c28f5c28f5c3p-2"), -1.0] {
        assert!(
            ctx.note("TROMBONE", 52.0, 0.2, cap, 0.8)
                .expect("nothing left")
                .is_empty()
        );
    }
    let gain = parse_hex_f64("0x1.a1206dba7baf8p-1");
    assert!(rel(ctx.inst_gain("TROMBONE").expect("the gain"), gain) <= LEVEL_REL);

    // ---- the FLUTE (each from the start of the noise stream)
    // dur -0.1: 0.08 s of tone (3,528 normals), then the gain note.
    let ctx = context(&ep);
    let x = ctx.note("FLUTE", 76.0, -0.1, 0.3, 0.8).expect("a release alone");
    assert_eq!(x.len(), 14400);
    samples_are(
        &x,
        &[
            (0, "0x1.d0b443d0ccdd1p-12"),
            (100, "-0x1.61082e8071100p-8"),
            (1000, "-0x1.66517c454efddp-7"),
            (3000, "0x1.2b0f6e39938e3p-7"),
        ],
        "FLUTE, dur -0.1",
    );
    assert!(x[3848] != 0.0 && x[3849..].iter().all(|v| *v == 0.0));
    assert!(stands_at(
        &ctx.noise().unwrap().0,
        103275142608439468136010328423419220429
    ));
    let gain = parse_hex_f64("0x1.5eea83f2d92eap-1");
    assert!(rel(ctx.inst_gain("FLUTE").expect("the gain"), gain) <= LEVEL_REL);
    // dur -0.18: a tone of no samples (nothing drawn for it), silence; only the gain note draws.
    let ctx = context(&ep);
    let silent = ctx.note("FLUTE", 76.0, -0.18, 0.3, 0.8).expect("silence");
    assert!(silent.len() == 14400 && silent.iter().all(|v| *v == 0.0));
    assert!(stands_at(
        &ctx.noise().unwrap().0,
        298504411499926805183013488005417175337
    ));
    // dur -0.19: Python raises before it draws.
    let ctx = context(&ep);
    let e = refused(ctx.note("FLUTE", 76.0, -0.19, 0.3, 0.8));
    assert!(
        e.starts_with("note: FLUTE has no note for a held length (dur) of -0.19 s"),
        "{e}"
    );
    assert!(stands_at(&ctx.noise().unwrap().0, PpNoise::START_STATE));
    assert!(ctx.first_notes().unwrap().is_empty());
    // cap -0.9: the 56,810 samples of the note and its echoes less 43,200.
    let ctx = context(&ep);
    let x = ctx.note("FLUTE", 76.0, 0.2, -0.9, 0.8).expect("a note cut short");
    assert_eq!(x.len(), 13610);
    samples_are(
        &x,
        &[
            (0, "0x1.d0bf08706ff66p-12"),
            (3000, "0x1.0a768e66dbfedp-2"),
            (13609, "0x0.0p+0"),
        ],
        "FLUTE, cap -0.9",
    );
    let gain = parse_hex_f64("0x1.5ef2a594e7b91p-1");
    assert!(rel(ctx.inst_gain("FLUTE").expect("the gain"), gain) <= LEVEL_REL);
}
