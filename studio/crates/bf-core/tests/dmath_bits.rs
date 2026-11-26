//! `bf_core::dmath` gives the same bits on every PC.
//!
//! For each `dmath` function this test feeds 4,096 fixed inputs (edge values first: zeros of both signs, subnormals,
//! huge values, infinities, NaN and multiples of pi / 2 with their neighbours; then draws from a SplitMix64 written
//! here) and compares every output's `to_bits()` with `studio/reference/dmath/bits.json`. A NaN output is stored as
//! `"nan"` and matches any NaN: the sign and payload of a NaN are not part of what the engine may depend on, and
//! they are where CPUs legitimately differ.
//!
//! The file was blessed on the Windows reference PC (`BF_BLESS=1 cargo test -p bf-core --test dmath_bits` writes it
//! instead of checking it). `dmath` is plain arithmetic in Rust (the reference PC's C runtime functions written out,
//! and `libm` with `force-soft-floats`): no platform maths library, and the one CPU instruction it may use, the
//! fused multiply-add, is exact, so the Linux CI job must reproduce exactly the same bits from the same file.

use bf_core::dmath;
use serde_json::Value;
use std::fmt::Write as _;

/// Inputs per function.
const N: usize = 4096;

/// The reference file, relative to this crate.
const BITS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../reference/dmath/bits.json");

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1), 53 bits.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in [lo, hi).
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// An integer in lo..=hi.
    fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo + 1) as u64)) as i64
    }
}

/// FNV-1a over bytes: seeds per function name and fingerprints of the inputs.
fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    let mut h = 0xCBF2_9CE4_8422_2325u64;
    for b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The next representable value `steps` ulps away (towards +inf for positive steps), without any maths library.
fn ulps(x: f64, steps: i64) -> f64 {
    let mut y = x;
    for _ in 0..steps.unsigned_abs() {
        let up = steps > 0;
        y = if y == 0.0 {
            if up { f64::from_bits(1) } else { -f64::from_bits(1) }
        } else if (y > 0.0) == up {
            f64::from_bits(y.to_bits() + 1)
        } else {
            f64::from_bits(y.to_bits() - 1)
        };
    }
    y
}

fn edges() -> Vec<f64> {
    let pi = std::f64::consts::PI;
    let mut v = vec![
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        1e-300,
        -1e-300,
        1e-10,
        -1e-10,
        1e-5,
        0.5,
        -0.5,
        1.0,
        -1.0,
        ulps(1.0, -1),
        ulps(1.0, 1),
        ulps(-1.0, 1),
        2.0,
        -2.0,
        10.0,
        100.0,
        -100.0,
        700.0,
        709.782712893384,
        710.0,
        -708.0,
        -745.1332191019411,
        -746.0,
        1e10,
        1e22,
        4503599627370496.0,
        9007199254740992.0,
        1e300,
        -1e300,
        f64::MAX,
        -f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        f64::EPSILON,
        1e6 * pi,
    ];
    for k in -16..=16 {
        let x = f64::from(k) * (pi / 2.0);
        v.extend([x, ulps(x, 1), ulps(x, -1)]);
    }
    v
}

/// The 4,096 inputs of a one-argument f64 function.
fn inputs(seed: u64) -> Vec<f64> {
    let mut r = SplitMix64(seed);
    let mut v = edges();
    let pi = std::f64::consts::PI;
    while v.len() < N {
        let x = match v.len() % 6 {
            0 => f64::from_bits(r.next()),
            1 => r.range(-1.0, 1.0),
            2 => r.range(-10.0, 10.0),
            3 => r.range(-1000.0, 1000.0),
            4 => ulps(r.int(-1000, 1000) as f64 * (pi / 2.0), r.int(-3, 3)),
            _ => r.range(-1e-3, 1e-3) * f64::from_bits(((1023 - r.int(0, 60)) as u64) << 52),
        };
        v.push(x);
    }
    v
}

/// The 4,096 input pairs of a two-argument f64 function: every pair of a small edge set, then seeded pairs.
fn pairs(seed: u64) -> Vec<(f64, f64)> {
    let small = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        3.0,
        -3.0,
        f64::from_bits(1),
        1e300,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let mut v: Vec<(f64, f64)> = small.iter().flat_map(|&a| small.iter().map(move |&b| (a, b))).collect();
    let xs = inputs(seed);
    let ys = inputs(seed ^ 0x5555_5555_5555_5555);
    let mut r = SplitMix64(seed.rotate_left(17));
    let mut k = 0;
    while v.len() < N {
        let (x, y) = (xs[k % N], ys[(k * 7 + 3) % N]);
        let pair = match k % 4 {
            0 => (x, y),
            1 => (r.range(0.0, 10.0), r.range(-10.0, 10.0)),
            2 => (r.range(-10.0, 10.0), r.int(-20, 20) as f64),
            _ => (y, x),
        };
        v.push(pair);
        k += 1;
    }
    v
}

fn inputs_f32(seed: u64) -> Vec<f32> {
    let mut r = SplitMix64(seed ^ 0xF32);
    let mut v: Vec<f32> = inputs(seed).into_iter().map(|x| x as f32).collect();
    for (k, x) in v.iter_mut().enumerate().skip(edges().len()) {
        if k % 5 == 0 {
            *x = f32::from_bits(r.next() as u32);
        }
    }
    v
}

fn pairs_f32(seed: u64) -> Vec<(f32, f32)> {
    pairs(seed).into_iter().map(|(a, b)| (a as f32, b as f32)).collect()
}

/// One function's run: the inputs' fingerprint, a printable form of each input, and each output as text.
struct Run {
    fingerprint: u64,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

fn out64(y: f64) -> String {
    if y.is_nan() {
        "nan".into()
    } else {
        format!("{:016x}", y.to_bits())
    }
}

fn out32(y: f32) -> String {
    if y.is_nan() {
        "nan".into()
    } else {
        format!("{:08x}", y.to_bits())
    }
}

fn run1(seed: u64, f: fn(f64) -> f64) -> Run {
    let xs = inputs(seed);
    Run {
        fingerprint: fnv1a(xs.iter().flat_map(|x| x.to_bits().to_le_bytes())),
        inputs: xs.iter().map(|x| format!("{x:e}")).collect(),
        outputs: xs.iter().map(|&x| out64(f(x))).collect(),
    }
}

fn run2(seed: u64, f: fn(f64, f64) -> f64) -> Run {
    let ps = pairs(seed);
    Run {
        fingerprint: fnv1a(
            ps.iter()
                .flat_map(|(a, b)| [a.to_bits(), b.to_bits()])
                .flat_map(u64::to_le_bytes),
        ),
        inputs: ps.iter().map(|(a, b)| format!("{a:e}, {b:e}")).collect(),
        outputs: ps.iter().map(|&(a, b)| out64(f(a, b))).collect(),
    }
}

fn run1f(seed: u64, f: fn(f32) -> f32) -> Run {
    let xs = inputs_f32(seed);
    Run {
        fingerprint: fnv1a(xs.iter().flat_map(|x| x.to_bits().to_le_bytes())),
        inputs: xs.iter().map(|x| format!("{x:e}")).collect(),
        outputs: xs.iter().map(|&x| out32(f(x))).collect(),
    }
}

fn run2f(seed: u64, f: fn(f32, f32) -> f32) -> Run {
    let ps = pairs_f32(seed);
    Run {
        fingerprint: fnv1a(
            ps.iter()
                .flat_map(|(a, b)| [a.to_bits(), b.to_bits()])
                .flat_map(u32::to_le_bytes),
        ),
        inputs: ps.iter().map(|(a, b)| format!("{a:e}, {b:e}")).collect(),
        outputs: ps.iter().map(|&(a, b)| out32(f(a, b))).collect(),
    }
}

/// Every `dmath` function, in file order.
fn runs() -> Vec<(&'static str, Run)> {
    enum Func {
        F64(fn(f64) -> f64),
        F64x2(fn(f64, f64) -> f64),
        F32(fn(f32) -> f32),
        F32x2(fn(f32, f32) -> f32),
    }
    let table: [(&str, Func); 21] = [
        ("sin", Func::F64(dmath::sin)),
        ("cos", Func::F64(dmath::cos)),
        ("tan", Func::F64(dmath::tan)),
        ("asin", Func::F64(dmath::asin)),
        ("acos", Func::F64(dmath::acos)),
        ("atan", Func::F64(dmath::atan)),
        ("atan2", Func::F64x2(dmath::atan2)),
        ("hypot", Func::F64x2(dmath::hypot)),
        ("exp", Func::F64(dmath::exp)),
        ("ln", Func::F64(dmath::ln)),
        ("log10", Func::F64(dmath::log10)),
        ("log2", Func::F64(dmath::log2)),
        ("pow", Func::F64x2(dmath::pow)),
        ("tanh", Func::F64(dmath::tanh)),
        ("np_tanh", Func::F64(dmath::np_tanh)),
        ("sinh", Func::F64(dmath::sinh)),
        ("fmod", Func::F64x2(dmath::fmod)),
        ("sinf", Func::F32(dmath::sinf)),
        ("cosf", Func::F32(dmath::cosf)),
        ("expf", Func::F32(dmath::expf)),
        ("powf", Func::F32x2(dmath::powf)),
    ];
    table
        .into_iter()
        .map(|(name, f)| {
            let seed = fnv1a(name.bytes());
            let run = match f {
                Func::F64(f) => run1(seed, f),
                Func::F64x2(f) => run2(seed, f),
                Func::F32(f) => run1f(seed, f),
                Func::F32x2(f) => run2f(seed, f),
            };
            assert_eq!(run.outputs.len(), N, "{name}");
            (name, run)
        })
        .collect()
}

fn render(runs: &[(&str, Run)]) -> String {
    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(
        " \"about\": \"bf_core::dmath output bits for fixed inputs (crates/bf-core/tests/dmath_bits.rs makes the \
         inputs and checks every output against this file; BF_BLESS=1 rewrites it). An output is the hex of to_bits(), \
         or nan for any NaN. inputs_fnv1a fingerprints the inputs, so a change to the input generator is caught.\",\n",
    );
    s.push_str(" \"format\": 1,\n");
    let _ = writeln!(s, " \"inputs_per_function\": {N},");
    s.push_str(" \"functions\": {\n");
    for (k, (name, run)) in runs.iter().enumerate() {
        let _ = writeln!(
            s,
            "  \"{name}\": {{\"inputs_fnv1a\": \"{:016x}\", \"bits\": [",
            run.fingerprint
        );
        for (j, chunk) in run.outputs.chunks(8).enumerate() {
            let line: Vec<String> = chunk.iter().map(|o| format!("\"{o}\"")).collect();
            let last = j + 1 == run.outputs.len().div_ceil(8);
            let _ = writeln!(s, "   {}{}", line.join(", "), if last { "" } else { "," });
        }
        let _ = writeln!(s, "  ]}}{}", if k + 1 == runs.len() { "" } else { "," });
    }
    s.push_str(" }\n}\n");
    s
}

#[test]
fn bits_are_stable() {
    let runs = runs();
    if std::env::var_os("BF_BLESS").is_some_and(|v| v == "1") {
        let path = std::path::Path::new(BITS);
        std::fs::create_dir_all(path.parent().expect("bits.json has a folder")).expect("create reference/dmath");
        std::fs::write(path, render(&runs)).expect("write bits.json");
        eprintln!("blessed {BITS}");
        return;
    }
    let text = std::fs::read_to_string(BITS)
        .unwrap_or_else(|e| panic!("cannot read {BITS}: {e} (bless it with BF_BLESS=1 on the reference PC)"));
    let doc: Value = serde_json::from_str(&text).expect("bits.json is JSON");
    assert_eq!(doc["format"], 1, "bits.json format");
    let funcs = doc["functions"].as_object().expect("bits.json has functions");
    let names: Vec<&str> = runs.iter().map(|(n, _)| *n).collect();
    let in_file: Vec<&str> = funcs.keys().map(String::as_str).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(in_file, sorted, "bits.json lists other functions than dmath has");
    let mut problems = Vec::new();
    for (name, run) in &runs {
        let entry = &funcs[*name];
        let fp = entry["inputs_fnv1a"].as_str().expect("inputs_fnv1a");
        assert_eq!(
            fp,
            format!("{:016x}", run.fingerprint),
            "{name}: the inputs differ from the blessed ones (the generator changed; re-bless deliberately)"
        );
        let want = entry["bits"].as_array().expect("bits");
        assert_eq!(want.len(), N, "{name}: wrong number of outputs in bits.json");
        for (k, (w, got)) in want.iter().zip(&run.outputs).enumerate() {
            let w = w.as_str().expect("an output is a string");
            if w != got {
                problems.push(format!(
                    "{name}({}): got {got}, bits.json has {w} (input {k})",
                    run.inputs[k]
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} dmath outputs differ from bits.json (first 30):\n{}",
        problems.len(),
        problems.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}
