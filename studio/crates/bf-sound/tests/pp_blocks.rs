//! The Pixel Pitch voice blocks several kit voices share, against the kit: `studio/reference/shared/pp_blocks.npz`
//! (the `shared` export). Each case is a call the kit made while its users' voices played (`pp_bridge.voice` over
//! each user's register) or an edge case:
//!
//! - `brass` (TRUMPET, TUBA, HORN), `ks_pluck` (ACOUSTIC, ELECTRIC), `sax_tone` (ALTO SAX and the mambo bed's alto,
//!   from the state of `altosax60_lowpoly._RN` recorded before it; the generator must stand where Python left it
//!   after), `bp` (on the normals of a fresh generator);
//! - each note against its fingerprint: the length exactly; 64 samples and the RMS of every 882-sample (20 ms) block below
//!   -120 dBFS of difference (the voices' bar of milestone 1); a note that is NaN in Python is NaN here. The SHA-256
//!   of the float64 samples is counted (a nice-to-know: `dmath` and the C library differ in the last bit of some
//!   `sin`, `exp` and `pow` values); `ks_pluck` is plain arithmetic on the generator and must match it every time;
//! - `starts`: the series' three module generators are fresh generators of their seeds where a soundtrack starts.
//!
//! The parts' hash and `bassclarinet_lowpoly.slap` in the same file are checked in bf-content's `shared_helpers`.

use bf_core::rng::Generator;
use bf_sound::dsp;
use bf_sound::voice::pp::brass::{Brass, brass};
use bf_sound::voice::pp::pluck::ks_pluck;
use bf_sound::voice::pp::sax::{Sax, sax_tone};
use bf_sound::voice::pp::{self};
use bf_sound::voice::{ModuleNoise, PpNoise};
use bf_testdata::{Case, Vectors, db_max_diff, hex_arg};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FILE: &str = "shared/pp_blocks.npz";
/// Sample and block-RMS differences: below this many dBFS.
const DB: f64 = -120.0;
/// The fingerprint's RMS blocks, in samples.
const BLOCK: usize = 882;

fn sha(x: &[f64]) -> String {
    let bytes: Vec<u8> = x.iter().flat_map(|v| v.to_le_bytes()).collect();
    Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// The JSON of case `i` as the sidecar holds it (its `at`, `n`, `sha256`, ... beside the parsed fields).
fn raw(v: &Vectors, i: usize) -> &Value {
    &v.index.raw["cases"][i]
}

/// The packed array of `role` in case `i`: the slice of the shared member its `at` names.
fn packed<'a>(v: &'a Vectors, i: usize, role: &str) -> &'a [f64] {
    let case = &v.index.cases[i];
    let at = &raw(v, i)["at"][role];
    let off = at[0].as_u64().unwrap() as usize;
    let len: usize = at.as_array().unwrap()[1..]
        .iter()
        .map(|d| d.as_u64().unwrap() as usize)
        .product();
    &v.array(case, role).f64()[off..off + len]
}

fn big(v: &Value) -> u128 {
    v.as_str().unwrap().parse().unwrap()
}

fn generator(v: &Value) -> Generator {
    Generator::from_state(big(&v["state"]), big(&v["inc"]))
}

/// Whether two generators will draw the same next numbers.
fn same_place(a: &Generator, b: &Generator) -> bool {
    let (mut a, mut b) = (a.clone(), b.clone());
    (0..4).all(|_| a.random().to_bits() == b.random().to_bits())
}

fn arg(c: &Case, key: &str) -> f64 {
    hex_arg(&c.args[key])
}

/// The tally of one test: how many notes, the worst differences, how many SHA-256 matched.
#[derive(Default)]
struct Tally {
    notes: usize,
    worst_pick: f64,
    worst_rms: f64,
    same_bits: usize,
    failures: Vec<String>,
}

impl Tally {
    fn new() -> Tally {
        Tally {
            worst_pick: f64::NEG_INFINITY,
            worst_rms: f64::NEG_INFINITY,
            ..Tally::default()
        }
    }

    /// One note against its fingerprint.
    fn note(&mut self, v: &Vectors, i: usize, x: &[f64]) {
        let r = raw(v, i);
        let id = &v.index.cases[i].id;
        self.notes += 1;
        let n = r["n"].as_u64().unwrap() as usize;
        if x.len() != n {
            self.failures
                .push(format!("{id}: {} samples, Python's has {n}", x.len()));
            return;
        }
        if sha(x) == r["sha256"].as_str().unwrap() {
            self.same_bits += 1;
        }
        let nonfinite = r["nonfinite"].as_u64().unwrap() as usize;
        let ours = x.iter().filter(|v| !v.is_finite()).count();
        if nonfinite > 0 || ours > 0 {
            if ours != nonfinite || !x.iter().all(|v| v.is_nan()) {
                self.failures
                    .push(format!("{id}: {ours} samples not finite, Python's has {nonfinite}"));
            }
            return;
        }
        let want = packed(v, i, "picks");
        let k = want.len();
        assert!(k >= 2, "{id}: a fingerprint of {k} samples");
        let got: Vec<f64> = (0..k).map(|j| x[(j * (n - 1)) / (k - 1)]).collect();
        let d = db_max_diff(&got, want);
        self.worst_pick = self.worst_pick.max(d);
        if d.is_nan() || d >= DB {
            self.failures.push(format!("{id}: samples {d:.1} dBFS apart"));
        }
        let want = packed(v, i, "rms");
        let got: Vec<f64> = x
            .chunks(BLOCK)
            .map(|b| dsp::mean(&b.iter().map(|s| s * s).collect::<Vec<_>>()).sqrt())
            .collect();
        let d = if got.len() == want.len() {
            db_max_diff(&got, want)
        } else {
            f64::INFINITY
        };
        self.worst_rms = self.worst_rms.max(d);
        if d.is_nan() || d >= DB {
            self.failures.push(format!(
                "{id}: block RMS {d:.1} dBFS apart ({} blocks, Python's {})",
                got.len(),
                want.len()
            ));
        }
    }

    fn finish(self, what: &str, at_least: usize) {
        println!(
            "{what}: {} notes, worst sample {:.1} dBFS, worst block RMS {:.1} dBFS, {} of them Python's bits",
            self.notes, self.worst_pick, self.worst_rms, self.same_bits
        );
        assert!(
            self.notes >= at_least,
            "{what}: {} notes, {at_least} expected",
            self.notes
        );
        assert!(self.failures.is_empty(), "{what}:\n{}", self.failures.join("\n"));
    }
}

fn cases<'a>(v: &'a Vectors, call: &'a str) -> impl Iterator<Item = (usize, &'a Case)> + 'a {
    v.index.cases.iter().enumerate().filter(move |(_, c)| c.call == call)
}

/// Every user each block is recorded for (the edge cases besides).
fn users(v: &Vectors, call: &str) -> Vec<String> {
    let mut u: Vec<String> = cases(v, call)
        .map(|(i, _)| raw(v, i)["user"].as_str().unwrap().to_string())
        .collect();
    u.sort();
    u.dedup();
    u
}

#[test]
fn the_file_holds_every_block_and_user() {
    let v = Vectors::open(FILE);
    assert_eq!(users(&v, "brass"), ["HORN", "TRUMPET", "TUBA", "edge"]);
    assert_eq!(users(&v, "ks_pluck"), ["ACOUSTIC", "ELECTRIC", "edge"]);
    assert_eq!(users(&v, "sax_tone"), ["ALTO SAX", "edge", "mambo alto"]);
    assert_eq!(users(&v, "slap"), ["BASS CLARINET", "EUPHONIUM", "edge"]);
    assert_eq!(users(&v, "bp"), ["edge"]);
    let known = ["brass", "ks_pluck", "sax_tone", "slap", "bp"];
    for c in &v.index.cases {
        assert!(
            known.contains(&c.call.as_str()),
            "{}: a call this test does not know: {}",
            c.id,
            c.call
        );
    }
}

#[test]
fn brass_is_the_kits() {
    let v = Vectors::open(FILE);
    let mut t = Tally::new();
    for (i, c) in cases(&v, "brass") {
        let tone = Brass {
            bright: arg(c, "bright"),
            vib: arg(c, "vib"),
            scoop: arg(c, "scoop"),
            rough: arg(c, "rough"),
            tilt: arg(c, "tilt"),
        };
        t.note(&v, i, &brass(arg(c, "f"), arg(c, "dur"), arg(c, "vel"), &tone));
    }
    t.finish("brass", 20);
}

#[test]
fn ks_pluck_is_the_kits_to_the_bit() {
    let v = Vectors::open(FILE);
    let mut t = Tally::new();
    for (i, c) in cases(&v, "ks_pluck") {
        let x = ks_pluck(
            arg(c, "f"),
            arg(c, "dur"),
            arg(c, "bright"),
            arg(c, "decay"),
            c.args["seed"].as_u64().unwrap(),
        );
        t.note(&v, i, &x);
    }
    let (notes, same) = (t.notes, t.same_bits);
    t.finish("ks_pluck", 15);
    assert_eq!(
        same, notes,
        "ks_pluck is plain arithmetic: every note must be Python's bits"
    );
}

#[test]
fn sax_tone_is_the_kits_and_moves_its_generator_as_python() {
    let v = Vectors::open(FILE);
    let mut t = Tally::new();
    let mut moved = 0;
    for (i, c) in cases(&v, "sax_tone") {
        let get = |key: &str, default: f64| c.args.get(key).map_or(default, hex_arg);
        let d = Sax::default();
        let gl = match c.args.get("gl") {
            None | Some(Value::Null) => None,
            Some(g) => Some((hex_arg(&g[0]), hex_arg(&g[1]))),
        };
        let tone = Sax {
            bright: get("bright", d.bright),
            vib: get("vib", d.vib),
            vrate: get("vrate", d.vrate),
            odd: get("odd", d.odd),
            scoop: get("scoop", d.scoop),
            att: get("att", d.att),
            formant: get("formant", d.formant),
            fboost: get("fboost", d.fboost),
            breath: get("breath", d.breath),
            growl: get("growl", d.growl),
            slap: get("slap", d.slap),
            gl,
        };
        let r = raw(&v, i);
        let mut rn = generator(&r["rn_before"]);
        let x = sax_tone(&mut rn, arg(c, "f"), arg(c, "dur"), arg(c, "v"), &tone);
        t.note(&v, i, &x);
        let after = generator(&r["rn_after"]);
        assert!(
            same_place(&rn, &after),
            "{}: _RN does not stand where Python left it",
            c.id
        );
        if !same_place(&generator(&r["rn_before"]), &after) {
            moved += 1;
        }
    }
    assert!(moved > 10, "only {moved} cases drew from _RN");
    t.finish("sax_tone", 15);
}

#[test]
fn the_band_pass_is_the_series() {
    let v = Vectors::open(FILE);
    let mut t = Tally::new();
    for (i, c) in cases(&v, "bp") {
        // the input: default_rng(noise_seed).normal(0, 1, noise_n)
        let mut g = Generator::new(c.args["noise_seed"].as_u64().unwrap());
        let mut x = vec![0.0; c.args["noise_n"].as_u64().unwrap() as usize];
        g.fill_normal(0.0, 1.0, &mut x);
        t.note(&v, i, &pp::bp(&x, arg(c, "lo"), arg(c, "hi")));
    }
    t.finish("bp", 4);
}

#[test]
fn the_module_generators_start_where_python_starts() {
    let v = Vectors::open(FILE);
    let starts = &v.index.raw["starts"];
    let s = PpNoise::start();
    for (label, ours, seed) in [
        ("drums60_lowpoly._rng", &s.0, PpNoise::SEED),
        ("altosax60_lowpoly._RN", &s.1.sax, ModuleNoise::SAX_SEED),
        ("violin_lowpoly.rng", &s.1.violin, ModuleNoise::VIOLIN_SEED),
    ] {
        let want = &starts[label];
        assert_eq!(want["seed"].as_u64(), Some(seed), "{label}");
        assert!(
            same_place(ours, &generator(want)),
            "{label} does not start where Python's does"
        );
    }
    assert_eq!(big(&starts["drums60_lowpoly._rng"]["state"]), PpNoise::START_STATE);
}
