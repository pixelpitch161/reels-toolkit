//! The DSP layer against numpy and scipy: every case of every vector file under `studio/reference/dsp`.
//!
//! One test per file. Each walks every case of the sidecar, fails on a case whose `call` it does not know, and
//! prints one summary line (`butter: 311/311, worst 1.2e-15 rel (c042)`) and then the worst case of each function.
//! The bars are the plan's: second-order sections within 1e-12 relative; the filters, the resampler, `interp`,
//! `percentile`, `convolve` and every generator below -120 dBFS of difference (`20 log10(max |a - b|) < -120`);
//! `fftconvolve` and `reverb` below -100 dBFS. Results that are plain IEEE arithmetic in the library's order must
//! have the same bits: the filters on scipy's own coefficients, `upfirdn` on scipy's own filter, running sums,
//! numpy's pairwise sums, `linspace`, `interp`, `percentile` and the 16-bit conversion. `find_peaks` must give
//! scipy's peaks; where two equal peaks are too close, which one scipy keeps hangs on numpy's sort, and the test
//! then asks that some order of those equal peaks gives scipy's peaks (see `find_peaks_case`). `ufuncs` measures
//! how far `bf_core::dmath` is from numpy's array loops and from Python's `math` on the reference PC, in units in
//! the last place.
//!
//! Run with `-- --nocapture` to see the lines.

use std::collections::{BTreeMap, BTreeSet};

use bf_core::dmath;
use bf_core::rng::Generator;
use bf_sound::dsp::{self, FilterKind, Sos};
use bf_sound::synth;
use bf_sound::wav::sample_i16;
use bf_testdata::{Case, NpArray, Vectors, db_max_diff, hex_arg, max_rel_diff};
use serde_json::Value;

/// Second-order-section (and `b`, `a`) coefficients: the largest relative difference allowed.
const COEFF_REL: f64 = 1e-12;
/// Sample differences: below this many dBFS.
const DB: f64 = -120.0;
/// `fftconvolve` and `reverb`: below this many dBFS.
const DB_FFT: f64 = -100.0;
/// One `dmath` function against numpy's or Python's: at most this many units in the last place. (Measured on
/// the reference PC: 2 for `tanh`, 1 for every other function, 0 for the square root.)
const MAX_ULP: f64 = 2.0;
/// The square of such a function: squaring doubles a relative error, and a unit in the last place of the square
/// can be half the size, relative to the value, of one of the function. (Measured: 3.)
const MAX_ULP_SQUARED: f64 = 4.0;

/// How a measured difference is judged and printed.
#[derive(Clone, Copy, PartialEq)]
enum Unit {
    /// A relative difference; passes at or under the bar.
    Rel,
    /// `20 log10(max |a - b|)`; passes strictly under the bar.
    Db,
    /// Units in the last place; passes at or under the bar.
    Ulp,
}

/// The cases of one vector file: how many passed, the worst difference, and the worst of each function.
struct Tally {
    file: &'static str,
    unit: Unit,
    cases: BTreeSet<String>,
    failed_cases: BTreeSet<String>,
    failures: Vec<String>,
    worst: Option<(f64, String)>,
    by_function: BTreeMap<String, (f64, String, usize)>,
}

impl Tally {
    fn new(file: &'static str, unit: Unit) -> Tally {
        Tally {
            file,
            unit,
            cases: BTreeSet::new(),
            failed_cases: BTreeSet::new(),
            failures: Vec::new(),
            worst: None,
            by_function: BTreeMap::new(),
        }
    }

    fn show(&self, v: f64) -> String {
        match self.unit {
            Unit::Rel => format!("{v:.1e} rel"),
            Unit::Db if v == f64::NEG_INFINITY => "-inf dBFS (the same bits)".to_string(),
            Unit::Db => format!("{v:.1} dBFS"),
            Unit::Ulp => format!("{v} ulp"),
        }
    }

    /// Marks the case as seen (a case with no check still counts).
    fn case(&mut self, id: &str) {
        self.cases.insert(id.to_string());
    }

    /// One measured difference of case `id`, for the function `function`, against `bar`.
    fn check(&mut self, id: &str, function: &str, value: f64, bar: f64) {
        self.case(id);
        let pass = match self.unit {
            Unit::Db => value < bar,
            Unit::Rel | Unit::Ulp => value <= bar,
        };
        if !pass {
            self.failed_cases.insert(id.to_string());
            self.failures.push(format!(
                "{id} {function}: {}, the bar is {}",
                self.show(value),
                self.show(bar)
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

    /// A difference in dB of two sample arrays; a length mismatch is a failure, not a panic.
    fn db(&mut self, id: &str, function: &str, got: &[f64], want: &[f64], bar: f64) {
        if got.len() != want.len() {
            self.fail(
                id,
                function,
                format!("{} values, the reference has {}", got.len(), want.len()),
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
        self.case(id);
        let d = db_max_diff(got, want);
        if d != f64::NEG_INFINITY {
            self.failed_cases.insert(id.to_string());
            self.failures.push(format!(
                "{id} {function}: {d:.1} dBFS of difference, the same bits are expected"
            ));
        }
        if self.unit == Unit::Db {
            self.check(id, function, d, f64::INFINITY);
        }
    }

    /// A check with nothing to measure that passed (kept as a NaN among the per-function worsts).
    fn pass(&mut self, id: &str, function: &str) {
        self.case(id);
        let entry = self
            .by_function
            .entry(function.to_string())
            .or_insert((f64::NAN, id.to_string(), 0));
        entry.2 += 1;
    }

    fn fail(&mut self, id: &str, function: &str, why: String) {
        self.case(id);
        self.failed_cases.insert(id.to_string());
        self.failures.push(format!("{id} {function}: {why}"));
    }

    /// Prints the summary line and the per-function lines, then fails the test if any case failed.
    fn finish(self) {
        let total = self.cases.len();
        let passed = total - self.failed_cases.len();
        let worst = match &self.worst {
            Some((v, id)) => format!("worst {} ({id})", self.show(*v)),
            None => "nothing measured".to_string(),
        };
        println!("{}: {passed}/{total}, {worst}", self.file);
        for (function, (v, id, n)) in &self.by_function {
            if v.is_nan() {
                println!("    {function}: {n} check(s), passed ({id})");
            } else {
                println!("    {function}: {n} check(s), worst {} ({id})", self.show(*v));
            }
        }
        assert!(
            self.failures.is_empty(),
            "{}: {} of {total} case(s) failed:\n{}",
            self.file,
            self.failed_cases.len(),
            self.failures.join("\n")
        );
    }
}

// ---------------------------------------------------------------- reading cases

/// The `(n, 2)` array as stereo frames.
fn stereo(a: &NpArray) -> Vec<[f64; 2]> {
    assert!(
        a.shape.len() == 2 && a.shape[1] == 2,
        "shape {:?} is not (n, 2)",
        a.shape
    );
    a.f64().as_chunks::<2>().0.to_vec()
}

/// The `(sections, 6)` array as second-order sections.
fn sections(a: &NpArray) -> Sos {
    assert!(
        a.shape.len() == 2 && a.shape[1] == 6,
        "shape {:?} is not (n, 6)",
        a.shape
    );
    a.f64().as_chunks::<6>().0.to_vec()
}

fn hex_list(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap_or_else(|| panic!("{v} is not a list"))
        .iter()
        .map(hex_arg)
        .collect()
}

/// A design as `butter.json` and `filters.json` give it: `(kind, cutoffs, order, fs, output)`. `fs` is 2 where
/// Python passed none (the cutoffs are then fractions of the Nyquist frequency).
fn design(d: &Value) -> (FilterKind, Vec<f64>, usize, f64, String) {
    let kind = match d["btype"].as_str().expect("btype") {
        "lp" | "low" => FilterKind::Lp,
        "hp" | "high" => FilterKind::Hp,
        "bp" | "band" => FilterKind::Bp,
        other => panic!("unknown btype {other:?}"),
    };
    let fs = if d["fs"].is_null() { 2.0 } else { hex_arg(&d["fs"]) };
    let order = d["N"].as_u64().expect("N") as usize;
    (
        kind,
        hex_list(&d["Wn"]),
        order,
        fs,
        d["output"].as_str().expect("output").to_string(),
    )
}

/// The extra top-level fields the exporter gives a case (`what`, `generator`, ...), by case index.
fn extra<'a>(v: &'a Vectors, i: usize, key: &str) -> &'a Value {
    &v.index.raw["cases"][i][key]
}

// ---------------------------------------------------------------- butter

#[test]
fn butter() {
    let v = Vectors::open("dsp/butter.npz");
    let mut t = Tally::new("butter", Unit::Rel);
    for case in &v.index.cases {
        let id = case.id.as_str();
        if !case.call.starts_with("scipy.signal.butter(") {
            t.fail(id, &case.call, "a call this test does not know".into());
            continue;
        }
        let (kind, wn, order, fs, output) = design(&case.args);
        match output.as_str() {
            "sos" => {
                let want = v.array(case, "sos");
                let got = dsp::butter_sos(kind, &wn, order, fs);
                if want.shape != [got.len(), 6] {
                    t.fail(
                        id,
                        "butter sos",
                        format!("{} sections, the reference has shape {:?}", got.len(), want.shape),
                    );
                    continue;
                }
                let name = format!("butter {:?} order {order} sos", kind);
                t.check(id, &name, max_rel_diff(got.as_flattened(), want.f64()), COEFF_REL);
            }
            "ba" => {
                let (b, a) = dsp::butter_ba(kind, &wn, order, fs);
                let (want_b, want_a) = (v.array(case, "b").f64(), v.array(case, "a").f64());
                if b.len() != want_b.len() || a.len() != want_a.len() {
                    t.fail(id, "butter ba", "b or a has the wrong length".into());
                    continue;
                }
                let name = format!("butter {:?} order {order} ba", kind);
                t.check(id, &name, max_rel_diff(&b, want_b), COEFF_REL);
                t.check(id, &name, max_rel_diff(&a, want_a), COEFF_REL);
            }
            other => t.fail(id, "butter", format!("unknown output {other:?}")),
        }
    }
    t.finish();
}

// ---------------------------------------------------------------- sosfilt, lfilter

#[test]
fn filters() {
    let v = Vectors::open("dsp/filters.npz");
    let mut t = Tally::new("filters", Unit::Db);

    // The fixed inputs are what the sidecar says they are: seeded noise, drawn as numpy draws it.
    let mut noise = vec![0.0; 4800];
    Generator::new(4800).fill_normal(0.0, 1.0, &mut noise);
    assert_eq!(
        db_max_diff(&noise, v.npz.get("noise").f64()),
        f64::NEG_INFINITY,
        "noise"
    );
    let mut noise2 = vec![0.0; 9600];
    Generator::new(4801).fill_normal(0.0, 1.0, &mut noise2);
    assert_eq!(
        db_max_diff(&noise2, v.npz.get("noise2").f64()),
        f64::NEG_INFINITY,
        "noise2"
    );

    for case in &v.index.cases {
        let id = case.id.as_str();
        let want = v.array(case, "out");
        let x = v.array(case, "x");
        match case.call.as_str() {
            "scipy.signal.sosfilt(sos, noise, axis=0)" => {
                let sos = sections(v.array(case, "sos"));
                // On scipy's own coefficients the filter is the same arithmetic in the same order.
                t.same(id, "sosfilt (scipy's sos)", &dsp::sosfilt(&sos, x.f64()), want.f64());
                // And through the Studio's own design, as the generators use it.
                let (kind, wn, order, fs, _) = design(&case.args["design"]);
                let mine = dsp::butter_sos(kind, &wn, order, fs);
                assert_eq!(mine.len(), sos.len(), "{id}: sections");
                assert!(
                    max_rel_diff(mine.as_flattened(), sos.as_flattened()) <= COEFF_REL,
                    "{id}: design"
                );
                t.db(
                    id,
                    "sosfilt (own design)",
                    &dsp::sosfilt(&mine, x.f64()),
                    want.f64(),
                    DB,
                );
            }
            "scipy.signal.sosfilt(sos, noise2, axis=0)" => {
                let sos = sections(v.array(case, "sos"));
                let got = dsp::sosfilt_stereo(&sos, &stereo(x));
                t.same(id, "sosfilt stereo (scipy's sos)", got.as_flattened(), want.f64());
                let (kind, wn, order, fs, _) = design(&case.args["design"]);
                let got = dsp::sosfilt_stereo(&dsp::butter_sos(kind, &wn, order, fs), &stereo(x));
                t.db(id, "sosfilt stereo (own design)", got.as_flattened(), want.f64(), DB);
            }
            "scipy.signal.lfilter(b, a, noise)" => {
                let (b, a) = (v.array(case, "b").f64(), v.array(case, "a").f64());
                t.same(id, "lfilter (scipy's b, a)", &dsp::lfilter(b, a, x.f64()), want.f64());
                let (kind, wn, order, fs, _) = design(&case.args["design"]);
                let (mb, ma) = dsp::butter_ba(kind, &wn, order, fs);
                assert!(
                    max_rel_diff(&mb, b) <= COEFF_REL && max_rel_diff(&ma, a) <= COEFF_REL,
                    "{id}: design"
                );
                t.db(
                    id,
                    "lfilter (own design)",
                    &dsp::lfilter(&mb, &ma, x.f64()),
                    want.f64(),
                    DB,
                );
            }
            "scipy.signal.lfilter([0.002], [1, -0.998], noise ** 2)" => {
                let (b, a) = (v.array(case, "b").f64(), v.array(case, "a").f64());
                assert_eq!((b, a), (&[0.002][..], &[1.0, -0.998][..]), "{id}: b, a");
                let squared: Vec<f64> = noise.iter().map(|s| s * s).collect();
                assert_eq!(db_max_diff(&squared, x.f64()), f64::NEG_INFINITY, "{id}: noise ** 2");
                t.same(
                    id,
                    "lfilter (envelope follower)",
                    &dsp::lfilter(b, a, x.f64()),
                    want.f64(),
                );
            }
            "scipy.signal.lfilter(b1, a1, noise2, axis=0)" => {
                let (b, a) = (v.array(case, "b").f64(), v.array(case, "a").f64());
                let got = dsp::lfilter_stereo(b, a, &stereo(x));
                t.same(id, "lfilter stereo (K-weighting)", got.as_flattened(), want.f64());
            }
            "scipy.signal.lfilter(b2, a2, lfilter(b1, a1, noise2, axis=0), axis=0)" => {
                let (b1, a1) = (hex_list(&case.args["b1"]), hex_list(&case.args["a1"]));
                let (b2, a2) = (v.array(case, "b").f64(), v.array(case, "a").f64());
                assert_eq!(
                    (&hex_list(&case.args["b2"])[..], &hex_list(&case.args["a2"])[..]),
                    (b2, a2)
                );
                let first = dsp::lfilter_stereo(&b1, &a1, &stereo(x));
                let got = dsp::lfilter_stereo(b2, a2, &first);
                t.same(id, "lfilter stereo (K-weighting)", got.as_flattened(), want.f64());
            }
            other => t.fail(id, other, "a call this test does not know".into()),
        }
    }
    t.finish();
}

// ---------------------------------------------------------------- resample_poly

#[test]
fn resample() {
    let v = Vectors::open("dsp/resample.npz");
    let mut t = Tally::new("resample", Unit::Db);
    for case in &v.index.cases {
        let id = case.id.as_str();
        if case.call != "scipy.signal.resample_poly(x, 160, 147)" {
            t.fail(id, &case.call, "a call this test does not know".into());
            continue;
        }
        let args = &case.args;
        let (up, down) = (
            args["up"].as_u64().unwrap() as usize,
            args["down"].as_u64().unwrap() as usize,
        );
        let x = v.array(case, "x").f64();
        assert_eq!(x.len() as u64, args["n_in"].as_u64().unwrap(), "{id}: n_in");

        // The low-pass scipy designs inside, and the filter as it hands it to upfirdn.
        let taps = dsp::firwin_kaiser(2 * 10 * up.max(down) + 1, 1.0 / up.max(down) as f64, 5.0);
        t.db(id, "firwin (kaiser 5.0)", &taps, v.array(case, "firwin").f64(), DB);
        let n_pre_pad = args["n_pre_pad"].as_u64().unwrap() as usize;
        let mut h = vec![0.0; n_pre_pad];
        h.extend(taps.iter().map(|c| c * up as f64));
        t.db(id, "firwin * up, padded", &h, v.array(case, "h").f64(), DB);

        // upfirdn on scipy's own filter is the same arithmetic in the same order.
        let full = dsp::upfirdn(v.array(case, "h").f64(), x, up, down);
        assert_eq!(
            full.len() as u64,
            args["upfirdn_len"].as_u64().unwrap(),
            "{id}: upfirdn_len"
        );
        let n_pre_remove = args["n_pre_remove"].as_u64().unwrap() as usize;
        let n_out = args["n_out"].as_u64().unwrap() as usize;
        let want = v.array(case, "out").f64();
        t.same(
            id,
            "upfirdn (scipy's filter)",
            &full[n_pre_remove..n_pre_remove + n_out],
            want,
        );

        t.db(id, "resample_poly", &dsp::resample_poly(x, up, down), want, DB);
    }
    t.finish();
}

// ---------------------------------------------------------------- fftconvolve

#[test]
fn fftconvolve() {
    let v = Vectors::open("dsp/fftconvolve.npz");
    let mut t = Tally::new("fftconvolve", Unit::Db);
    for case in &v.index.cases {
        let id = case.id.as_str();
        if case.call != "scipy.signal.fftconvolve(a, b)" {
            t.fail(id, &case.call, "a call this test does not know".into());
            continue;
        }
        let (a, b) = (v.array(case, "a").f64(), v.array(case, "b").f64());
        t.db(
            id,
            "fftconvolve",
            &dsp::fftconvolve(a, b),
            v.array(case, "out").f64(),
            DB_FFT,
        );
        // The operands the other way round give the same convolution.
        t.db(
            id,
            "fftconvolve (b, a)",
            &dsp::fftconvolve(b, a),
            v.array(case, "out").f64(),
            DB_FFT,
        );
    }
    t.finish();
}

// ---------------------------------------------------------------- numpy

#[test]
fn numpy() {
    let v = Vectors::open("dsp/numpy.npz");
    let mut t = Tally::new("numpy", Unit::Db);
    for case in &v.index.cases {
        let id = case.id.as_str();
        let want = v.array(case, "out").f64();
        let get = |role: &str| v.array(case, role).f64();
        // A scalar result is also in the sidecar, as float.hex().
        let scalar = |t: &mut Tally, function: &str, got: f64| {
            let result = hex_arg(extra_by_id(&v, case, "result"));
            assert_eq!(want, [result], "{id}: `result` and `out`");
            t.same(id, function, &[got], want);
        };
        match case.call.as_str() {
            "np.interp(pos, np.arange(n), x, right=0.0)" | "np.interp(pos, np.arange(n), x)" => {
                let n = case.args["n"].as_u64().unwrap() as usize;
                let xp: Vec<f64> = (0..n).map(|i| i as f64).collect();
                let got = match case.args.get("right") {
                    Some(right) => dsp::interp_lr(get("pos"), &xp, get("x"), None, Some(hex_arg(right))),
                    None => dsp::interp(get("pos"), &xp, get("x")),
                };
                t.same(id, "interp", &got, want);
            }
            "np.interp(q, xp, fp)" => {
                t.same(id, "interp", &dsp::interp(get("q"), get("xp"), get("fp")), want);
            }
            "np.percentile(x, 99.97)" => {
                scalar(
                    &mut t,
                    "percentile",
                    dsp::percentile(get("x"), hex_arg(&case.args["q"])),
                );
            }
            "np.convolve(f ** 2, np.ones(w) / w, 'same')" => {
                let squared: Vec<f64> = get("f").iter().map(|s| s * s).collect();
                t.db(id, "convolve same", &dsp::convolve_same(&squared, get("k")), want, DB);
            }
            "np.cumsum(f)" => t.same(id, "cumsum", &dsp::cumsum(get("f")), want),
            "np.linspace(a, b, n)" => {
                let (a, b) = (hex_arg(&case.args["a"]), hex_arg(&case.args["b"]));
                let n = case.args["n"].as_u64().unwrap() as usize;
                t.same(id, "linspace", &dsp::linspace(a, b, n), want);
            }
            // A 1-D array, or a stereo one read flat: numpy adds either pairwise.
            "(x ** 2).mean()" => {
                let squared: Vec<f64> = get("x").iter().map(|s| s * s).collect();
                scalar(&mut t, "mean (pairwise)", dsp::mean(&squared));
            }
            "(x ** 2).mean(1)" => {
                let got: Vec<f64> = stereo(v.array(case, "x"))
                    .iter()
                    .map(|[l, r]| (l * l + r * r) / 2.0)
                    .collect();
                t.same(id, "mean of each row", &got, want);
            }
            "(x ** 2).sum(0)" => {
                let squared: Vec<[f64; 2]> = stereo(v.array(case, "x")).iter().map(|[l, r]| [l * l, r * r]).collect();
                t.same(id, "sum of each column", &dsp::sum_axis0(&squared), want);
            }
            "(x ** 2).mean(0).sum()" => {
                let squared: Vec<[f64; 2]> = stereo(v.array(case, "x")).iter().map(|[l, r]| [l * l, r * r]).collect();
                let n = squared.len() as f64;
                let means = dsp::sum_axis0(&squared).map(|s| s / n);
                scalar(&mut t, "mean of each column, added", dsp::sum(&means));
            }
            "np.abs(x).max()" => {
                let got = get("x").iter().fold(f64::NEG_INFINITY, |m, s| m.max(s.abs()));
                scalar(&mut t, "peak", got);
            }
            "(np.clip(x, -1, 1) * 32767).astype(np.int16)" => {
                let got: Vec<f64> = get("x").iter().map(|&s| f64::from(sample_i16(s))).collect();
                t.same(id, "16-bit conversion", &got, want);
            }
            "scipy.signal.find_peaks(x, height=6.0, distance=4)" => {
                let (height, distance) = (hex_arg(&case.args["height"]), case.args["distance"].as_u64().unwrap());
                find_peaks_case(&mut t, id, get("x"), height, distance as usize, want, get("heights"));
            }
            other => t.fail(id, other, "a call this test does not know".into()),
        }
    }
    t.finish();
}

/// One `find_peaks` case: `want` holds scipy's peak indices (as floats) and `heights` the signal at each.
///
/// scipy thins peaks that are closer than `distance` from the highest down, in the order of numpy's default
/// `argsort` of their heights. That sort does not keep equal values in order (on the reference PC it is a
/// vectorised sorting network), so of two equal peaks closer than `distance` scipy keeps whichever the sort
/// happened to put last. Nothing else in the result depends on the sort. So:
/// - no two equal peaks closer than `distance`: the peaks must be scipy's, exactly;
/// - otherwise: some order of those equal peaks must give scipy's peaks exactly (every other step is then
///   proven the same), and the line printed says where the Studio's own rule (the later of two equal peaks
///   stays) differs from what numpy's sort did on the reference PC.
fn find_peaks_case(t: &mut Tally, id: &str, x: &[f64], height: f64, distance: usize, want: &[f64], heights: &[f64]) {
    let want: Vec<usize> = want.iter().map(|&v| v as usize).collect();
    let at_want: Vec<f64> = want.iter().map(|&i| x[i]).collect();
    assert_eq!(heights, at_want, "{id}: the stored heights are x at the stored peaks");
    let got = dsp::find_peaks(x, height, distance);

    // The peaks tall enough, and the groups of equal ones chained together by gaps under `distance`.
    let tall: Vec<usize> = dsp::local_maxima(x).into_iter().filter(|&i| height <= x[i]).collect();
    let mut group: Vec<usize> = (0..tall.len()).collect();
    for a in 0..tall.len() {
        for b in a + 1..tall.len() {
            if tall[b] - tall[a] >= distance {
                break;
            }
            if x[tall[a]] == x[tall[b]] {
                let (from, to) = (group[b], group[a]);
                for g in &mut group {
                    if *g == from {
                        *g = to;
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, &g) in group.iter().enumerate() {
        groups.entry(g).or_default().push(i);
    }
    let tied: Vec<Vec<usize>> = groups.into_values().filter(|members| members.len() > 1).collect();

    if tied.is_empty() {
        let (got, want): (Vec<f64>, Vec<f64>) = (
            got.iter().map(|&i| i as f64).collect(),
            want.iter().map(|&i| i as f64).collect(),
        );
        t.same(id, "find_peaks", &got, &want);
        return;
    }

    // The stable order of the heights, then every order of each tied group within its own places.
    let mut order: Vec<usize> = (0..tall.len()).collect();
    order.sort_by(|&a, &b| x[tall[a]].partial_cmp(&x[tall[b]]).unwrap());
    let orders: usize = tied.iter().map(|g| (1..=g.len()).product::<usize>()).product();
    assert!(
        orders <= 1_000_000,
        "{id}: {orders} orders of the equal peaks are too many to try"
    );
    let peaks_for = |order: &[usize]| -> Vec<usize> {
        let keep = dsp::select_by_peak_distance(&tall, order, distance);
        tall.iter().zip(keep).filter(|&(_, k)| k).map(|(&p, _)| p).collect()
    };
    assert_eq!(peaks_for(&order), got, "{id}: find_peaks is the stable order");
    fn search(
        tied: &[Vec<usize>],
        order: &mut [usize],
        peaks_for: &dyn Fn(&[usize]) -> Vec<usize>,
        want: &[usize],
    ) -> bool {
        let Some((members, rest)) = tied.split_first() else {
            return peaks_for(order) == want;
        };
        let places: Vec<usize> = (0..order.len()).filter(|&s| members.contains(&order[s])).collect();
        let mut arrangement = members.clone();
        permute(&mut arrangement, 0, &mut |arranged| {
            for (&place, &member) in places.iter().zip(arranged) {
                order[place] = member;
            }
            search(rest, order, peaks_for, want)
        })
    }
    /// Every arrangement of `v[k..]` in turn, until `visit` returns true.
    fn permute(v: &mut [usize], k: usize, visit: &mut dyn FnMut(&[usize]) -> bool) -> bool {
        if k == v.len() {
            return visit(v);
        }
        for i in k..v.len() {
            v.swap(k, i);
            if permute(v, k + 1, visit) {
                return true;
            }
            v.swap(k, i);
        }
        false
    }
    let found = search(&tied, &mut order, &peaks_for, &want);
    let only_here: Vec<usize> = got.iter().copied().filter(|p| !want.contains(p)).collect();
    let only_scipy: Vec<usize> = want.iter().copied().filter(|p| !got.contains(p)).collect();
    println!(
        "    {id} find_peaks: {} group(s) of equal peaks closer than {distance} ({orders} orders); the Studio keeps \
         {} peaks, scipy on the reference PC {}; kept only here {only_here:?}, only by scipy {only_scipy:?}",
        tied.len(),
        got.len(),
        want.len()
    );
    let function = "find_peaks (equal peaks too close: some order of them)";
    if found {
        t.pass(id, function);
    } else {
        t.fail(id, function, "no order of the equal peaks gives scipy's peaks".into());
    }
}

/// A top-level field of a case that `Case` does not carry (`result`, `what`, ...).
fn extra_by_id<'a>(v: &'a Vectors, case: &Case, key: &str) -> &'a Value {
    let i = v
        .index
        .cases
        .iter()
        .position(|c| c.id == case.id)
        .expect("the case is in the sidecar");
    extra(v, i, key)
}

// ---------------------------------------------------------------- dmath against numpy and Python's math

/// How many representable numbers apart two floats are (0 for the same bits, and for `-0.0` against `0.0`).
fn ulps(a: f64, b: f64) -> f64 {
    let ordinal = |x: f64| {
        let bits = (x.to_bits() & 0x7fff_ffff_ffff_ffff) as i64;
        if x.is_sign_negative() { -bits } else { bits }
    };
    assert!(a.is_finite() && b.is_finite(), "{a} against {b}");
    (ordinal(a) - ordinal(b)).unsigned_abs() as f64
}

#[test]
fn ufuncs() {
    let v = Vectors::open("dsp/ufuncs.npz");
    let mut t = Tally::new("ufuncs", Unit::Ulp);
    println!("ufuncs: dmath against numpy 2.5.3's array loops and Python 3.13's math on the reference PC");
    for (i, case) in v.index.cases.iter().enumerate() {
        let id = case.id.as_str();
        let bar = if case.call.ends_with(" ** 2") && case.call != "x ** 2" {
            MAX_ULP_SQUARED
        } else {
            MAX_ULP
        };
        let f: fn(f64) -> f64 = match case.call.as_str() {
            "np.sin(x)" | "python: math.sin(x)" => dmath::sin,
            "np.cos(x)" | "python: math.cos(x)" => dmath::cos,
            "np.exp(x)" | "python: math.exp(x)" => dmath::exp,
            "np.tanh(x)" => dmath::tanh,
            "np.log10(x)" => dmath::log10,
            // numpy takes a square root for the power 0.5 and multiplies for the power 2.
            "np.sqrt(x)" | "x ** 0.5" => f64::sqrt,
            "x ** 2" => |x| x * x,
            "x ** 1.5" => |x| dmath::pow(x, 1.5),
            "x ** 0.62" => |x| dmath::pow(x, 0.62),
            "x ** 1.3" => |x| dmath::pow(x, 1.3),
            "2 ** x" => |x| dmath::pow(2.0, x),
            "20 ** x" => |x| dmath::pow(20.0, x),
            "np.sin(np.pi * 0.5 * x) ** 2" => |x| {
                let s = dmath::sin(dmath::PI * 0.5 * x);
                s * s
            },
            "python: x ** 0.8" => |x| dmath::pow(x, 0.8),
            "python: x ** 1.6" => |x| dmath::pow(x, 1.6),
            "python: 10 ** (x / 20)" => |x| dmath::pow(10.0, x / 20.0),
            "python: 2 ** ((x - 69) / 12)" => |x| dmath::pow(2.0, (x - 69.0) / 12.0),
            other => {
                t.fail(id, other, "a call this test does not know".into());
                continue;
            }
        };
        let (x, want) = (v.array(case, "x").f64(), v.array(case, "out").f64());
        assert_eq!(x.len(), want.len(), "{id}");
        let (mut worst, mut differing) = (0.0f64, 0usize);
        for (&x, &w) in x.iter().zip(want) {
            let d = ulps(f(x), w);
            worst = worst.max(d);
            differing += usize::from(d > 0.0);
        }
        println!(
            "    {id} {:<30} max {worst} ulp, {differing} of {} values differ ({})",
            case.call,
            x.len(),
            extra(&v, i, "what").as_str().unwrap_or("")
        );
        t.check(id, &case.call, worst, bar);
    }
    t.finish();
}

// ---------------------------------------------------------------- the generators

/// The generator a case calls: `reelkit.audio.kick(punch=1.0, ...)` gives `kick`.
fn generator_name(call: &str) -> Option<&str> {
    call.strip_prefix("reelkit.audio.")?
        .split_once('(')
        .map(|(name, _)| name)
}

#[test]
fn generators() {
    let v = Vectors::open("dsp/generators.npz");
    let mut t = Tally::new("generators", Unit::Db);
    for (i, case) in v.index.cases.iter().enumerate() {
        let id = case.id.as_str();
        let a = &case.args;
        let f = |key: &str| hex_arg(&a[key]);
        let u = |key: &str| {
            a[key]
                .as_u64()
                .unwrap_or_else(|| panic!("{id}: {key} is not a whole number"))
        };
        let n = |key: &str| u(key) as usize;
        let b = |key: &str| a[key].as_bool().unwrap_or_else(|| panic!("{id}: {key} is not a bool"));
        let Some(name) = generator_name(&case.call) else {
            t.fail(id, &case.call, "a call this test does not know".into());
            continue;
        };
        assert_eq!(extra(&v, i, "generator").as_str(), Some(name), "{id}: generator");
        let want = v.array(case, "out");
        let mono: Vec<f64> = match name {
            "kick" => synth::kick(f("punch"), f("dec"), f("f0"), f("f1")),
            "snare" => synth::snare(f("dec")),
            "clap" => synth::clap(),
            "hat" => synth::hat(f("dec"), b("open_")),
            "crash" => synth::crash(f("dur")),
            "whoosh" => synth::whoosh(f("dur"), f("lo"), f("hi"), b("rise"), u("seed")),
            "thud" => synth::thud(f("dur"), f("f0"), f("f1"), u("seed")),
            "pluck" => synth::pluck(f("m"), f("dur")),
            "pad" => synth::pad(&hex_list(&a["ms"]), f("dur"), f("cut"), f("att")),
            "bass" => synth::bass(f("m"), f("dur")),
            "chime" => synth::chime(f("m"), f("dur")),
            "saw" => synth::saw(f("f"), n("n"), f("det"), n("voices")),
            "square" => synth::square(f("f"), n("n"), f("duty")),
            "tri" => synth::tri(f("f"), n("n")),
            "adsr" => synth::adsr(n("n"), f("a"), f("d"), f("s"), f("r")),
            "expenv" => synth::expenv(n("n"), f("decay"), f("attack")),
            "reverb" => {
                let x = stereo(v.array(case, "x"));
                let got = synth::reverb(&x, f("dur"), f("decay"), u("seed"));
                assert_eq!(want.shape, [x.len(), 2], "{id}: shape");
                t.db(id, name, got.as_flattened(), want.f64(), DB_FFT);
                continue;
            }
            "tape_stop" => {
                let x = stereo(v.array(case, "x"));
                let got = synth::tape_stop(&x, f("t0"), f("length"));
                assert_eq!(want.shape, [x.len(), 2], "{id}: shape");
                t.db(id, name, got.as_flattened(), want.f64(), DB);
                continue;
            }
            other => {
                t.fail(id, other, "a generator this test does not know".into());
                continue;
            }
        };
        assert_eq!(want.shape.len(), 1, "{id}: a mono generator");
        t.db(id, name, &mono, want.f64(), DB);
    }
    t.finish();
}

/// A default argument value, as the sidecar's `signatures` gives it.
#[derive(Debug, PartialEq)]
enum Dflt {
    F(f64),
    U(u64),
    B(bool),
}

/// Rust has no default arguments: every generator takes all of Python's arguments, in Python's order, and the
/// default of each optional one is a constant. They are the signatures' defaults, bit for bit.
#[test]
fn generator_defaults_are_pythons() {
    use Dflt::{B, F, U};
    let ours: BTreeMap<(&str, &str), Dflt> = [
        (("adsr", "a"), F(synth::ADSR_A)),
        (("adsr", "d"), F(synth::ADSR_D)),
        (("adsr", "s"), F(synth::ADSR_S)),
        (("adsr", "r"), F(synth::ADSR_R)),
        (("chime", "dur"), F(synth::CHIME_DUR)),
        (("crash", "dur"), F(synth::CRASH_DUR)),
        (("expenv", "attack"), F(synth::EXPENV_ATTACK)),
        (("hat", "dec"), F(synth::HAT_DEC)),
        (("hat", "open_"), B(synth::HAT_OPEN)),
        (("kick", "punch"), F(synth::KICK_PUNCH)),
        (("kick", "dec"), F(synth::KICK_DEC)),
        (("kick", "f0"), F(synth::KICK_F0)),
        (("kick", "f1"), F(synth::KICK_F1)),
        (("pad", "cut"), F(synth::PAD_CUT)),
        (("pad", "att"), F(synth::PAD_ATT)),
        (("pluck", "dur"), F(synth::PLUCK_DUR)),
        (("reverb", "dur"), F(synth::REVERB_DUR)),
        (("reverb", "decay"), F(synth::REVERB_DECAY)),
        (("reverb", "seed"), U(synth::REVERB_SEED)),
        (("saw", "det"), F(synth::SAW_DET)),
        (("saw", "voices"), U(synth::SAW_VOICES as u64)),
        (("snare", "dec"), F(synth::SNARE_DEC)),
        (("square", "duty"), F(synth::SQUARE_DUTY)),
        (("tape_stop", "length"), F(synth::TAPE_STOP_LENGTH)),
        (("thud", "dur"), F(synth::THUD_DUR)),
        (("thud", "f0"), F(synth::THUD_F0)),
        (("thud", "f1"), F(synth::THUD_F1)),
        (("thud", "seed"), U(synth::THUD_SEED)),
        (("whoosh", "lo"), F(synth::WHOOSH_LO)),
        (("whoosh", "hi"), F(synth::WHOOSH_HI)),
        (("whoosh", "rise"), B(synth::WHOOSH_RISE)),
        (("whoosh", "seed"), U(synth::WHOOSH_SEED)),
    ]
    .into_iter()
    .collect();
    let v = Vectors::open("dsp/generators.npz");
    let signatures = v.index.raw["signatures"].as_object().expect("signatures");
    let mut seen = BTreeSet::new();
    for (generator, params) in signatures {
        for p in params.as_array().expect("a parameter list") {
            let name = p["name"].as_str().expect("name");
            let default = &p["default"];
            if p["required"].as_bool().expect("required") {
                assert!(default.is_null(), "{generator}.{name}");
                continue;
            }
            let key = (generator.as_str(), name);
            let mine = ours
                .get(&key)
                .unwrap_or_else(|| panic!("{generator}.{name} has a default in Python and no constant here"));
            let same = match mine {
                F(x) => hex_arg(default).to_bits() == x.to_bits(),
                U(x) => default.as_u64() == Some(*x),
                B(x) => default.as_bool() == Some(*x),
            };
            assert!(
                same,
                "{generator}.{name}: Python's default is {default}, the constant is {mine:?}"
            );
            seen.insert((generator.clone(), name.to_string()));
        }
    }
    assert_eq!(
        seen.len(),
        ours.len(),
        "a constant has no default in Python's signatures"
    );
}
