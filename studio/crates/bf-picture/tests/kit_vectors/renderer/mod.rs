//! The renderer kit against Python: `camera`, `geometry`, `raster`, `lighting`, `shadows`, `lens`, `post`, the FFT
//! convolution under the lens blur and the convex hull, on the cases of `studio/reference/picture_kit`. One module
//! per reference file; this one holds what they share: the runner, the comparisons and the readers of a case.
//!
//! The bars: a `uint8` or `float32` picture, a mask, an index list and every result that is a chain of plain
//! IEEE operations is compared exactly (floats bit for bit). A `float64` result that goes through a dot product, a
//! norm, a sine or an FFT is compared value by value ([`close_rows`]): numpy's dot products are fused on the
//! reference PC and its maths library is not `bf_core::dmath`, so the last bit can differ. A value passes if
//!
//! ```text
//! |got - want| <= max(1e-9 * |want|, 1e-12 * S)
//! ```
//!
//! where `S` is the largest magnitude in Python's row (the three coordinates of a point, the two of a screen
//! position) or, for the FFT results and the lens blur, in Python's whole array. The first term is the bar of
//! 1e-9 relative. The second is a floor for a value that cancels to nothing next to its neighbours (a
//! coordinate that is zero by geometry is `3e-16` in Python and `0` here): such a value has no size of its own,
//! and the floor lets it differ by a millionth of a millionth of the point or picture it belongs to, no more.
//! Where Python has an infinity or a NaN, the same must be here, and the rest of that row must then be equal to
//! the bit (it has no finite size to measure against).
//!
//! Each file's test prints a line per Python call (`camera: Cam(**cam).wp(P): 18/18, worst 8.4e-16`: the cases
//! that passed and the largest `|got - want| / S` met) and fails listing the cases that do not match. It also
//! fails if the file does not hold exactly the calls and the number of cases this test was written for
//! (`CALLS` in each module, 600 cases in all), so a regenerated or cut-off file cannot pass by holding less. A
//! case whose `call` the test does not know is a failure; a panic in the function under test fails that case
//! only. With the environment variable `KIT_VECTORS_CASES` set, every case prints its own line.

use super::{Check, float_arg, int_arg, same, same_f32, same_f64, same_i64, same_image};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_testdata::{Case, NpArray, Vectors, hex_arg};
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

mod camera;
mod fftconv;
mod geometry;
mod hull;
mod lens;
mod lighting;
mod post;
mod raster;
mod shadows;

/// What comparing one case found: the largest difference from Python (0 for an exact result), or why it fails.
type Found = Result<f64, String>;

/// The calls a reference file must hold, each with its number of cases.
type Calls = [(&'static str, usize)];

/// The bar for float results that are not exact by construction, relative to Python's value.
const REL: f64 = 1e-9;

/// The floor under [`REL`], relative to the largest magnitude in the value's row or array.
const FLOOR: f64 = 1e-12;

/// How the calls found in a file differ from the ones expected: nothing if they are the same calls with the same
/// number of cases.
fn count_problems(expected: &Calls, found: &BTreeMap<&str, usize>) -> Vec<String> {
    let mut problems = Vec::new();
    for &(call, want) in expected {
        match found.get(call) {
            Some(&n) if n == want => {}
            Some(&n) => problems.push(format!("  {call}: {n} cases, expected {want}")),
            None => problems.push(format!("  {call}: no cases, expected {want}")),
        }
    }
    for (call, n) in found {
        if !expected.iter().any(|(c, _)| c == call) {
            problems.push(format!("  {call}: {n} cases of a call that is not expected"));
        }
    }
    problems
}

/// Runs every case of `picture_kit/<name>.npz` through `run`, prints a line per call and panics unless the file
/// holds exactly the `expected` calls and every case matches.
fn check_calls(name: &str, expected: &Calls, run: impl Fn(&Vectors, &Case) -> Found) {
    let v = Vectors::open(&format!("picture_kit/{name}.npz"));
    let total = v.index.cases.len();
    let each = std::env::var_os("KIT_VECTORS_CASES").is_some();
    let mut order: Vec<&str> = Vec::new();
    let mut per_call: BTreeMap<&str, (usize, usize, f64)> = BTreeMap::new();
    let mut failures = Vec::new();
    for case in &v.index.cases {
        if !per_call.contains_key(case.call.as_str()) {
            order.push(&case.call);
        }
        let row = per_call.entry(&case.call).or_insert((0, 0, 0.0));
        row.1 += 1;
        let found = catch_unwind(AssertUnwindSafe(|| run(&v, case))).unwrap_or_else(|panic| {
            let text = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("a panic without a message");
            Err(format!("panicked: {text}"))
        });
        if each {
            match &found {
                Ok(worst) => println!("{name} {} {}: {worst:.1e}", case.id, case.call),
                Err(why) => println!("{name} {} {}: FAILS: {why}", case.id, case.call),
            }
        }
        match found {
            Ok(worst) => {
                row.0 += 1;
                row.2 = row.2.max(worst);
            }
            Err(why) => failures.push(format!("  {} {} {}: {why}", case.id, case.call, brief(&case.args))),
        }
    }
    for call in order {
        let (passed, of, worst) = per_call[call];
        println!("{name}: {call}: {passed}/{of}, worst {worst:.1e}");
    }
    let line = format!("{name}: {}/{total}", total - failures.len());
    println!("{line}");
    let counts: BTreeMap<&str, usize> = per_call.iter().map(|(call, row)| (*call, row.1)).collect();
    let miscounted = count_problems(expected, &counts);
    assert!(
        miscounted.is_empty(),
        "{name}: the file does not hold the cases this test expects ({} in all):\n{}",
        expected.iter().map(|c| c.1).sum::<usize>(),
        miscounted.join("\n")
    );
    assert!(
        failures.is_empty(),
        "{line}\n{} failing case(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// A case's arguments for a failure line, cut short.
fn brief(args: &Value) -> String {
    let text = args.to_string();
    if text.len() > 160 {
        format!("{}...", &text[..160])
    } else {
        text
    }
}

// ---------------------------------------------------------------- comparisons
fn exact(got: &[f64], want: &[f64]) -> Found {
    same_f64(got, want).map(|()| 0.0)
}

fn exact_f32(got: &[f32], want: &[f32]) -> Found {
    same_f32(got, want).map(|()| 0.0)
}

/// `got` against Python's `want`, both rows of `n` values. A value passes if it is within [`REL`] of Python's
/// value or within [`FLOOR`] of the largest magnitude `S` in Python's row, whichever is wider. A row in which
/// Python has an infinity or a NaN has no such size: it must be equal to the bit (any NaN for a NaN). Returns the
/// largest `|got - want| / S` met.
fn close_rows(got: &[f64], want: &[f64], n: usize) -> Found {
    if got.len() != want.len() || n == 0 || !want.len().is_multiple_of(n) {
        return Err(format!("{} values, Python has {} (rows of {n})", got.len(), want.len()));
    }
    let mut worst = 0.0f64;
    for (k, (g, w)) in got.chunks(n).zip(want.chunks(n)).enumerate() {
        if w.iter().any(|x| !x.is_finite()) {
            for j in 0..n {
                if g[j].to_bits() != w[j].to_bits() && !(g[j].is_nan() && w[j].is_nan()) {
                    return Err(format!(
                        "row {k}, value {j}: got {:e}, Python {:e}, in a row where Python has a value that is not \
                         finite, which must match exactly",
                        g[j], w[j]
                    ));
                }
            }
            continue;
        }
        let size = w.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        for j in 0..n {
            let off = (g[j] - w[j]).abs();
            let allowed = (REL * w[j].abs()).max(FLOOR * size);
            // A `got` that is not finite gives an `off` that is infinite or not a number: both fail.
            if off.is_nan() || off > allowed {
                return Err(format!(
                    "row {k}, value {j}: got {:e}, Python {:e}, off by {off:.1e} where {allowed:.1e} is allowed \
                     (the row's size is {size:e})",
                    g[j], w[j]
                ));
            }
            if off > 0.0 {
                worst = worst.max(off / size);
            }
        }
    }
    Ok(worst)
}

/// One number at a time: within [`REL`] of Python's value, relative to it.
fn close(got: &[f64], want: &[f64]) -> Found {
    close_rows(got, want, 1)
}

/// A whole array as one row: every value within [`REL`] of Python's or within [`FLOOR`] of the largest magnitude
/// in Python's array (for FFT results, whose small values are rounding noise around zero).
fn close_array(got: &[f64], want: &[f64]) -> Found {
    close_rows(got, want, want.len().max(1))
}

fn worst_of(parts: &[Found]) -> Found {
    let mut worst = 0.0f64;
    for p in parts {
        worst = worst.max(p.clone()?);
    }
    Ok(worst)
}

// ---------------------------------------------------------------- reading cases
fn num(v: &Value, what: &str) -> Result<f64, String> {
    match v {
        Value::String(_) | Value::Number(_) => Ok(hex_arg(v)),
        other => Err(format!("{what} is {other}, not a number")),
    }
}

fn opt_num(v: &Value, what: &str) -> Result<Option<f64>, String> {
    match v {
        Value::Null => Ok(None),
        v => num(v, what).map(Some),
    }
}

fn vec3(v: &Value, what: &str) -> Result<V3, String> {
    let a = v
        .as_array()
        .filter(|a| a.len() == 3)
        .ok_or_else(|| format!("{what} is {v}, not three numbers"))?;
    Ok([num(&a[0], what)?, num(&a[1], what)?, num(&a[2], what)?])
}

fn pair(v: &Value, what: &str) -> Result<(f64, f64), String> {
    let a = v
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or_else(|| format!("{what} is {v}, not two numbers"))?;
    Ok((num(&a[0], what)?, num(&a[1], what)?))
}

fn count(args: &Value, key: &str) -> Result<usize, String> {
    usize::try_from(int_arg(args, key)?).map_err(|e| format!("args.{key}: {e}"))
}

/// A float64 array of shape `(k, 3)` as points.
fn rows3(a: &NpArray) -> Result<Vec<V3>, String> {
    match a.shape.as_slice() {
        [_, 3] => Ok(a.f64().as_chunks::<3>().0.to_vec()),
        other => Err(format!("an array of shape {other:?}, not (k, 3)")),
    }
}

/// A float64 array of shape `(k, 2)` as screen points.
fn rows2(a: &NpArray) -> Result<Vec<(f64, f64)>, String> {
    match a.shape.as_slice() {
        [_, 2] => Ok(a.f64().as_chunks::<2>().0.iter().map(|p| (p[0], p[1])).collect()),
        other => Err(format!("an array of shape {other:?}, not (k, 2)")),
    }
}

/// A float64 array of shape `(3,)`.
fn one3(a: &NpArray) -> Result<V3, String> {
    match (a.shape.as_slice(), a.f64()) {
        ([3], d) => Ok([d[0], d[1], d[2]]),
        (other, _) => Err(format!("an array of shape {other:?}, not (3,)")),
    }
}

fn flat3(rows: &[V3]) -> Vec<f64> {
    rows.concat()
}

fn flat2(rows: &[(f64, f64)]) -> Vec<f64> {
    rows.iter().flat_map(|&(x, y)| [x, y]).collect()
}

/// Rows stored one list after another, cut where `counts` says.
fn split<T: Clone>(rows: &[T], counts: &[i64]) -> Result<Vec<Vec<T>>, String> {
    let mut out = Vec::with_capacity(counts.len());
    let mut at = 0usize;
    for &c in counts {
        let c = usize::try_from(c).map_err(|e| e.to_string())?;
        let part = rows
            .get(at..at + c)
            .ok_or("the counts ask for more rows than there are")?;
        out.push(part.to_vec());
        at += c;
    }
    if at == rows.len() {
        Ok(out)
    } else {
        Err(format!("the counts cover {at} of {} rows", rows.len()))
    }
}

/// A float array `(h, w)` or `(h, w, c)` as an image.
fn img_f64(a: &NpArray) -> Result<Img<f64>, String> {
    match a.shape.as_slice() {
        [h, w] => Ok(Img::from_vec(*w, *h, 1, a.f64().to_vec())),
        [h, w, c] => Ok(Img::from_vec(*w, *h, *c, a.f64().to_vec())),
        other => Err(format!("an image of shape {other:?}")),
    }
}

fn img_f32(a: &NpArray) -> Result<Img<f32>, String> {
    match a.shape.as_slice() {
        [h, w] => Ok(Img::from_vec(*w, *h, 1, a.f32().to_vec())),
        [h, w, c] => Ok(Img::from_vec(*w, *h, *c, a.f32().to_vec())),
        other => Err(format!("an image of shape {other:?}")),
    }
}

/// The shape numpy would give the image: `(h, w)` for one channel when Python's array has two axes.
fn same_shape<T>(got: &Img<T>, want: &NpArray) -> Check {
    let shape = if want.shape.len() == 2 {
        vec![got.h, got.w]
    } else {
        vec![got.h, got.w, got.c]
    };
    if want.shape == shape && (want.shape.len() == 3 || got.c == 1) {
        Ok(())
    } else {
        Err(format!(
            "an image of {} x {} x {}, Python's array is {:?}",
            got.w, got.h, got.c, want.shape
        ))
    }
}

fn exact_img_f64(got: &Img<f64>, want: &NpArray) -> Found {
    same_shape(got, want)?;
    exact(&got.data, want.f64())
}

fn exact_img_f32(got: &Img<f32>, want: &NpArray) -> Found {
    same_shape(got, want)?;
    exact_f32(&got.data, want.f32())
}

/// A numpy bool array `(h, w)` as a mask image (0 or 1).
fn mask_u8(a: &NpArray) -> Result<Img<u8>, String> {
    match a.shape.as_slice() {
        [h, w] => Ok(Img::from_vec(
            *w,
            *h,
            1,
            a.bool().iter().map(|&b| u8::from(b)).collect(),
        )),
        other => Err(format!("a mask of shape {other:?}")),
    }
}

/// The camera a case names in `args.cam` (`Cam(**cam)`).
fn cam_of(args: &Value) -> Result<Cam, String> {
    let c = &args["cam"];
    let f = |key: &str| num(&c[key], key);
    Ok(Cam::new(
        vec3(&c["target"], "target")?,
        f("dist")?,
        f("yaw")?,
        f("pitch")?,
        f("fov")?,
        opt_num(&c["sy"], "sy")?,
        f("roll")?,
        f("near")?,
    ))
}

// ---------------------------------------------------------------- the test's own guards
/// The nine files, each with the calls it must hold and the function that runs a case.
type Group = (&'static str, &'static Calls, fn(&Vectors, &Case) -> Found);

const GROUPS: [Group; 9] = [
    ("camera", camera::CALLS, camera::run),
    ("geometry", geometry::CALLS, geometry::run),
    ("raster", raster::CALLS, raster::run),
    ("lighting", lighting::CALLS, lighting::run),
    ("shadows", shadows::CALLS, shadows::run),
    ("lens", lens::CALLS, lens::run),
    ("fftconv", fftconv::CALLS, fftconv::run),
    ("post", post::CALLS, post::run),
    ("hull", hull::CALLS, hull::run),
];

/// An unknown call fails its case, in every group.
#[test]
fn unknown_calls_are_failures() {
    for (name, _, run) in GROUPS {
        let v = Vectors::open(&format!("picture_kit/{name}.npz"));
        let mut case = v.index.cases[0].clone();
        case.call = "np.sqrt(x)".into();
        assert!(run(&v, &case).unwrap_err().contains("np.sqrt"), "{name}");
    }
}

/// The renderer kit is 600 cases, and a file that holds other calls or other numbers of them is noticed.
#[test]
fn the_expected_cases_are_counted() {
    let total: usize = GROUPS.iter().flat_map(|g| g.1).map(|c| c.1).sum();
    assert_eq!(total, 600);
    for (name, calls, _) in GROUPS {
        let mut names: Vec<&str> = calls.iter().map(|c| c.0).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), calls.len(), "{name}: a call is listed twice");
    }
    let expected: &Calls = &[("f(x)", 3), ("g(x)", 2)];
    let found = |pairs: &[(&'static str, usize)]| pairs.iter().copied().collect::<BTreeMap<_, _>>();
    assert!(count_problems(expected, &found(&[("g(x)", 2), ("f(x)", 3)])).is_empty());
    // a call that lost a case, one that is gone, one that grew, one that is new
    assert_eq!(count_problems(expected, &found(&[("f(x)", 2), ("g(x)", 2)])).len(), 1);
    assert_eq!(count_problems(expected, &found(&[("f(x)", 3)])).len(), 1);
    assert_eq!(count_problems(expected, &found(&[("f(x)", 4), ("g(x)", 2)])).len(), 1);
    assert_eq!(
        count_problems(expected, &found(&[("f(x)", 3), ("g(x)", 2), ("h(x)", 1)])).len(),
        1
    );
    assert_eq!(count_problems(expected, &found(&[])).len(), 2);
}

/// The comparison's own guard: what counts as close, and what does not.
#[test]
fn closeness_is_measured_value_by_value_with_a_floor() {
    assert_eq!(close_rows(&[1.0, 2.0], &[1.0, 2.0], 2), Ok(0.0));
    assert_eq!(close(&[-0.0], &[0.0]), Ok(0.0));
    // 1e-9 of the value itself
    assert!(close(&[1.0 + 1e-12], &[1.0]).unwrap() < 2e-12);
    assert!(close(&[1.0 + 3e-9], &[1.0]).is_err());
    assert!(close(&[3e-16], &[-1e-16]).is_err());
    // and so for a value of real size next to a larger one: the row does not widen its bar beyond the floor
    assert!(close_rows(&[1.0 + 5e-10, 1000.0], &[1.0, 1000.0], 2).is_ok());
    assert!(close_rows(&[1.0 + 2e-9, 1000.0], &[1.0, 1000.0], 2).is_err());
    // a component that cancelled to nothing next to a large one: the floor, 1e-12 of the row
    assert!(close_rows(&[3e-16, 10.0], &[-1e-16, 10.0], 2).unwrap() < 1e-16);
    assert!(close_rows(&[9e-12, 10.0], &[0.0, 10.0], 2).is_ok());
    assert!(close_rows(&[2e-11, 10.0], &[0.0, 10.0], 2).is_err());
    assert!(close_rows(&[5e-9, 10.0], &[0.0, 10.0], 2).is_err());
    assert!(close_array(&[1e-12, 255.0], &[0.0, 255.0]).unwrap() < 1e-14);
    assert!(close_array(&[1e-9, 255.0], &[0.0, 255.0]).is_err());
    // a row of zeros has no size: only zeros match it
    assert_eq!(close_rows(&[0.0, -0.0], &[0.0, 0.0], 2), Ok(0.0));
    assert!(close_rows(&[1e-300, 0.0], &[0.0, 0.0], 2).is_err());
    // what is not finite must be matched: the same infinity, a NaN for a NaN, and the rest of the row exactly
    assert!(close(&[f64::NAN], &[1.0]).is_err());
    assert!(close(&[f64::INFINITY], &[1.0]).is_err());
    assert!(close(&[1.0], &[f64::NAN]).is_err());
    assert_eq!(close(&[f64::NAN], &[f64::NAN]), Ok(0.0));
    assert_eq!(close(&[f64::INFINITY], &[f64::INFINITY]), Ok(0.0));
    assert!(close(&[f64::NEG_INFINITY], &[f64::INFINITY]).is_err());
    assert!(close(&[f64::MAX], &[f64::INFINITY]).is_err());
    assert_eq!(close_rows(&[f64::INFINITY, 1.0], &[f64::INFINITY, 1.0], 2), Ok(0.0));
    assert!(close_rows(&[f64::INFINITY, 1.0 + 1e-15], &[f64::INFINITY, 1.0], 2).is_err());
    assert!(close_rows(&[f64::NAN, 1.0, 5.0], &[f64::NAN, 1.0, 5.0 + 1e-15], 3).is_err());
    assert!(close(&[1.0, 2.0], &[1.0]).is_err());
    assert!(exact(&[0.0], &[-0.0]).is_err());
}
