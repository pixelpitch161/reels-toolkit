//! The effects kit against Python: `bf_picture::fx` (the port of `reelkit/fx.py`) on the cases of
//! `studio/reference/picture_kit/fx_*.npz` (written by `tools/studio_export/kit_fx_vectors.py`).
//!
//! The bars: every picture is compared exactly, sample for sample (the `uint8` pictures the drawing calls give
//! byte for byte, the float pictures `starburst` and `apply_dim` give bit for bit), and so is every result that
//! is a chain of plain IEEE operations (index lists, flags, envelopes made of products and sums). A `float64`
//! result that goes through a sine, an exponential, a power or a dot product is compared value by value with the
//! measure of the renderer kit's test (`tests/kit_vectors/renderer/mod.rs`): a value passes if
//!
//! ```text
//! |got - want| <= max(1e-9 * |want|, 1e-12 * S)
//! ```
//!
//! where `S` is the largest magnitude in Python's row (the coordinates of a point, the channels of a colour).
//! Where Python has an infinity or a NaN the same must be here, and the rest of its row equal to the bit.
//!
//! One test per file. Each prints a line per Python call (the cases that passed and the largest
//! `|got - want| / S` met) and fails listing the cases that do not match; it also fails if the file does not
//! hold exactly the calls and the number of cases this test was written for ([`GROUPS`]). A case whose `call`
//! the test does not know is a failure, and so is a script argument it does not know. A panic in the function
//! under test fails that case only. With the environment variable `KIT_VECTORS_CASES` set, every case prints
//! its own line.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};

use bf_picture::camera::{Cam, V3};
use bf_picture::config::BAYER;
use bf_picture::fx::marks::LiveMark;
use bf_picture::fx::particles::{Particle, Shape};
use bf_picture::fx::{self, Bursts, Emit, Flares, FlashBudget, Mark, MarkKind, Marks, Moments, Particles, Shake};
use bf_picture::img::Img;
use bf_picture::lighting::Lights;
use bf_picture::np;
use bf_testdata::{Case, NpArray, Vectors, hex_arg};
use serde_json::Value;

/// What comparing one case found: the largest difference from Python (0 for an exact result), or why it fails.
type Found = Result<f64, String>;

/// The calls a reference file must hold, each with its number of cases.
type Calls = [(&'static str, usize)];

/// The bar for float results that are not exact by construction, relative to Python's value.
const REL: f64 = 1e-9;

/// The floor under [`REL`], relative to the largest magnitude in the value's row.
const FLOOR: f64 = 1e-12;

// ---------------------------------------------------------------- the files
/// `picture_kit/<name>.npz` with its sidecar, read once for the whole test binary.
fn vectors(name: &str) -> &'static Vectors {
    static OPEN: OnceLock<Mutex<BTreeMap<String, &'static Vectors>>> = OnceLock::new();
    let mut open = OPEN
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    open.entry(name.to_string())
        .or_insert_with(|| Box::leak(Box::new(Vectors::open(&format!("picture_kit/{name}.npz")))))
}

/// The case's `expect` object, which sits beside `args` in the sidecar.
fn expect_of<'a>(v: &'a Vectors, case: &Case) -> Result<&'a Value, String> {
    v.index.raw["cases"]
        .as_array()
        .and_then(|cases| cases.iter().find(|c| c["id"] == case.id.as_str()))
        .map(|c| &c["expect"])
        .filter(|e| e.is_object())
        .ok_or_else(|| "the case has no expect object".to_string())
}

/// The script a case names in `args.script`: an object's `add` or `emit` calls, from the sidecar's `scripts`.
fn script_of<'a>(v: &'a Vectors, args: &Value) -> Result<&'a Value, String> {
    let name = args["script"].as_str().ok_or("args.script is not a name")?;
    Some(&v.index.raw["scripts"][name])
        .filter(|s| s.is_object())
        .ok_or_else(|| format!("the sidecar has no script {name:?}"))
}

fn how_the_calls_differ(expected: &Calls, found: &BTreeMap<&str, usize>) -> Vec<String> {
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
fn check_calls(name: &str, expected: &Calls, run: fn(&Vectors, &Case) -> Found) {
    let v = vectors(name);
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
        let found = catch_unwind(AssertUnwindSafe(|| run(v, case))).unwrap_or_else(|panic| {
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
    let miscounted = how_the_calls_differ(expected, &counts);
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

/// A case's arguments for a failure line, without the camera and cut short.
fn brief(args: &Value) -> String {
    let mut args = args.clone();
    if let Some(map) = args.as_object_mut() {
        map.remove("cam");
    }
    let text = args.to_string();
    if text.len() > 200 {
        format!("{}...", &text[..200])
    } else {
        text
    }
}

// ---------------------------------------------------------------- comparisons
fn exact<T: Copy + PartialEq + std::fmt::Debug>(got: &[T], want: &[T], what: &str) -> Found {
    if got.len() != want.len() {
        return Err(format!("{what}: {} values, Python has {}", got.len(), want.len()));
    }
    match (0..got.len()).find(|&i| got[i] != want[i]) {
        None => Ok(0.0),
        Some(i) => Err(format!("{what}[{i}]: got {:?}, Python {:?}", got[i], want[i])),
    }
}

fn exact_f64(got: &[f64], want: &[f64], what: &str) -> Found {
    if got.len() != want.len() {
        return Err(format!("{what}: {} values, Python has {}", got.len(), want.len()));
    }
    match (0..got.len()).find(|&i| got[i].to_bits() != want[i].to_bits()) {
        None => Ok(0.0),
        Some(i) => Err(format!(
            "{what}[{i}]: got {:e}, Python {:e} (off by {:.1e})",
            got[i],
            want[i],
            (got[i] - want[i]).abs()
        )),
    }
}

/// `got` against Python's `want`, both rows of `n` values: the measure of the module documentation. Returns the
/// largest `|got - want| / S` met.
fn close_rows(got: &[f64], want: &[f64], n: usize, what: &str) -> Found {
    if got.len() != want.len() || n == 0 || !want.len().is_multiple_of(n) {
        return Err(format!(
            "{what}: {} values, Python has {} (rows of {n})",
            got.len(),
            want.len()
        ));
    }
    let mut worst = 0.0f64;
    for (k, (g, w)) in got.chunks(n).zip(want.chunks(n)).enumerate() {
        if w.iter().any(|x| !x.is_finite()) {
            for j in 0..n {
                if g[j].to_bits() != w[j].to_bits() && !(g[j].is_nan() && w[j].is_nan()) {
                    return Err(format!(
                        "{what}: row {k}, value {j}: got {:e}, Python {:e}, in a row where Python has a value that \
                         is not finite, which must match exactly",
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
            if off.is_nan() || off > allowed {
                return Err(format!(
                    "{what}: row {k}, value {j}: got {:e}, Python {:e}, off by {off:.1e} where {allowed:.1e} is \
                     allowed (the row's size is {size:e})",
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

/// One number at a time: within [`REL`] of Python's value.
fn close(got: &[f64], want: &[f64], what: &str) -> Found {
    close_rows(got, want, 1, what)
}

fn worst_of(parts: &[Found]) -> Found {
    let mut worst = 0.0f64;
    for p in parts {
        worst = worst.max(p.clone()?);
    }
    Ok(worst)
}

// ---------------------------------------------------------------- reading arguments
fn num(v: &Value, what: &str) -> Result<f64, String> {
    match v {
        Value::String(_) | Value::Number(_) => Ok(hex_arg(v)),
        other => Err(format!("{what} is {other}, not a number")),
    }
}

/// A float that must be written as `float.hex()`.
fn hexf(args: &Value, key: &str) -> Result<f64, String> {
    match &args[key] {
        v @ Value::String(_) => Ok(hex_arg(v)),
        other => Err(format!("args.{key} is {other}, not a float.hex() string")),
    }
}

fn int(v: &Value, what: &str) -> Result<i64, String> {
    v.as_i64().ok_or_else(|| format!("{what} is {v}, not an int"))
}

fn size(v: &Value, what: &str) -> Result<usize, String> {
    v.as_u64()
        .map(|x| x as usize)
        .ok_or_else(|| format!("{what} is {v}, not a count"))
}

fn flag(v: &Value, what: &str) -> Result<bool, String> {
    v.as_bool().ok_or_else(|| format!("{what} is {v}, not a bool"))
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

/// A float64 array of shape `(k, 3)` as rows.
fn rows3(a: &NpArray) -> Result<Vec<V3>, String> {
    match a.shape.as_slice() {
        [_, 3] => Ok(a.f64().chunks(3).map(|p| [p[0], p[1], p[2]]).collect()),
        other => Err(format!("an array of shape {other:?}, not (k, 3)")),
    }
}

fn flat3(rows: &[V3]) -> Vec<f64> {
    rows.concat()
}

/// `args.cam`: a camera by its fields.
fn cam_of(args: &Value) -> Result<Cam, String> {
    let c = &args["cam"];
    let m = c["M"]
        .as_array()
        .filter(|m| m.len() == 3)
        .ok_or("cam.M is not three rows")?;
    Ok(Cam {
        pos: vec3(&c["pos"], "cam.pos")?,
        m: [vec3(&m[0], "cam.M")?, vec3(&m[1], "cam.M")?, vec3(&m[2], "cam.M")?],
        f: num(&c["f"], "cam.f")?,
        cx: num(&c["cx"], "cam.cx")?,
        cy: num(&c["cy"], "cam.cy")?,
        near: num(&c["near"], "cam.near")?,
        target: vec3(&c["target"], "cam.target")?,
    })
}

// ---------------------------------------------------------------- scripts
/// The calls of a script, each checked to hold only the `known` argument names.
fn calls_of<'a>(script: &'a Value, known: &[&str]) -> Result<Vec<&'a serde_json::Map<String, Value>>, String> {
    let calls = script["calls"].as_array().ok_or("the script has no calls")?;
    let mut out = Vec::with_capacity(calls.len());
    for c in calls {
        let c = c.as_object().ok_or("a call of the script is not an object")?;
        if let Some(k) = c.keys().find(|k| !known.contains(&k.as_str())) {
            return Err(format!(
                "a call of the script has an argument this test does not know: {k}"
            ));
        }
        out.push(c);
    }
    Ok(out)
}

/// An argument of a script's call, or Python's default where the call leaves it out.
fn or(c: &serde_json::Map<String, Value>, key: &str, default: f64) -> Result<f64, String> {
    c.get(key).map_or(Ok(default), |v| num(v, key))
}

fn need<'a>(c: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a Value, String> {
    c.get(key).ok_or_else(|| format!("a call of the script has no {key}"))
}

fn flares_of(script: &Value) -> Result<Flares, String> {
    let mut fl = Flares::new(size(&script["max_n"], "max_n")?, num(&script["span"], "span")?);
    for c in calls_of(
        script,
        &["t0", "pos", "rgb", "radius", "peak", "attack", "decay", "hold"],
    )? {
        fl.add(
            num(need(c, "t0")?, "t0")?,
            vec3(need(c, "pos")?, "pos")?,
            vec3(need(c, "rgb")?, "rgb")?,
            or(c, "radius", 2.6)?,
            or(c, "peak", 1.0)?,
            or(c, "attack", 0.03)?,
            or(c, "decay", 0.25)?,
            or(c, "hold", 0.0)?,
        );
    }
    Ok(fl)
}

fn bursts_of(script: &Value) -> Result<Bursts, String> {
    let mut bu = Bursts::new(
        size(&script["max_per_s"], "max_per_s")?,
        num(&script["base_px"], "base_px")?,
        num(&script["per_strength"], "per_strength")?,
        size(&script["frames"], "frames")? as u32,
    );
    for c in calls_of(script, &["t0", "pos", "rgb", "strength", "seed", "bright"])? {
        let seed = match c.get("seed") {
            None | Some(Value::Null) => None,
            Some(v) => Some(v.as_u64().ok_or("a burst's seed is not a whole number")?),
        };
        bu.add(
            num(need(c, "t0")?, "t0")?,
            vec3(need(c, "pos")?, "pos")?,
            vec3(need(c, "rgb")?, "rgb")?,
            or(c, "strength", 1.0)?,
            seed,
            c.get("bright").map_or(Ok(true), |v| flag(v, "bright"))?,
        );
    }
    Ok(bu)
}

fn shake_of(script: &Value, args: &Value) -> Result<Shake, String> {
    let mut sh = Shake::new(hexf(args, "cap")?, hexf(args, "decay")?, hexf(args, "span")?);
    for c in calls_of(script, &["t0", "px"])? {
        sh.add(num(need(c, "t0")?, "t0")?, num(need(c, "px")?, "px")?);
    }
    Ok(sh)
}

fn particles_of(script: &Value) -> Result<Particles, String> {
    let seed = script["seed"]
        .as_u64()
        .ok_or("the script's seed is not a whole number")?;
    let mut pa = Particles::new(seed, size(&script["max_alive"], "max_alive")?);
    let known = [
        "t0", "pos", "kind", "n", "dir", "vel", "col", "scale", "speed", "spread", "life",
    ];
    for c in calls_of(script, &known)? {
        let mut opts = Emit::default();
        if let Some(v) = c.get("n").filter(|v| !v.is_null()) {
            opts.n = Some(int(v, "n")?);
        }
        if let Some(v) = c.get("dir") {
            opts.dir = vec3(v, "dir")?;
        }
        if let Some(v) = c.get("vel") {
            opts.vel = vec3(v, "vel")?;
        }
        if let Some(v) = c.get("col").filter(|v| !v.is_null()) {
            // one colour (np.ndim(col) == 1) or a list of them
            let list = v.as_array().ok_or("col is not a list")?;
            opts.col = Some(if list.first().is_some_and(Value::is_array) {
                list.iter().map(|q| vec3(q, "col")).collect::<Result<_, _>>()?
            } else {
                vec![vec3(v, "col")?]
            });
        }
        opts.scale = or(c, "scale", 1.0)?;
        if let Some(v) = c.get("speed").filter(|v| !v.is_null()) {
            opts.speed = Some(pair(v, "speed")?);
        }
        if let Some(v) = c.get("spread").filter(|v| !v.is_null()) {
            opts.spread = Some(num(v, "spread")?);
        }
        if let Some(v) = c.get("life").filter(|v| !v.is_null()) {
            opts.life = Some(pair(v, "life")?);
        }
        let kind = need(c, "kind")?.as_str().ok_or("kind is not a name")?;
        pa.emit(num(need(c, "t0")?, "t0")?, vec3(need(c, "pos")?, "pos")?, kind, &opts);
    }
    Ok(pa)
}

fn marks_of(script: &Value) -> Result<Marks, String> {
    let mut mk = Marks::new(num(&script["span"], "span")?, size(&script["max_live"], "max_live")?);
    let known = [
        "t0", "kind", "u", "v", "size", "angle", "col", "life", "n", "seed", "strength",
    ];
    for c in calls_of(script, &known)? {
        let name = need(c, "kind")?.as_str().ok_or("kind is not a name")?;
        let kind = MarkKind::from_name(name).ok_or_else(|| format!("no kind of mark named {name:?}"))?;
        let mut m = Mark::new(kind, num(need(c, "u")?, "u")?, num(need(c, "v")?, "v")?);
        m.size = or(c, "size", m.size)?;
        m.angle = or(c, "angle", m.angle)?;
        if let Some(v) = c.get("col") {
            m.col = vec3(v, "col")?;
        }
        m.life = or(c, "life", m.life)?;
        if let Some(v) = c.get("n") {
            m.n = int(v, "n")?;
        }
        if let Some(v) = c.get("seed") {
            m.seed = v.as_u64().ok_or("a mark's seed is not a whole number")?;
        }
        m.strength = or(c, "strength", m.strength)?;
        mk.add(num(need(c, "t0")?, "t0")?, m);
    }
    Ok(mk)
}

fn moments_of(script: &Value) -> Result<Moments, String> {
    let f = |key: &str| num(&script[key], key);
    let mut mo = Moments::new(f("max_dim")?, f("hold")?, f("t_in")?, f("t_out")?);
    for c in calls_of(script, &["t0", "key", "rgb", "k"])? {
        mo.add(
            num(need(c, "t0")?, "t0")?,
            int(need(c, "key")?, "key")?,
            vec3(need(c, "rgb")?, "rgb")?,
            or(c, "k", 1.0)?,
        );
    }
    Ok(mo)
}

// ---------------------------------------------------------------- pictures
/// A float picture of either dtype.
enum Pic {
    F32(Img<f32>),
    F64(Img<f64>),
}

/// Runs the same generic call on the picture of either dtype.
macro_rules! on {
    ($pic:expr, $img:ident => $body:expr) => {
        match $pic {
            Pic::F32($img) => $body,
            Pic::F64($img) => $body,
        }
    };
}

impl Pic {
    fn dtype(&self) -> &'static str {
        match self {
            Pic::F32(_) => "float32",
            Pic::F64(_) => "float64",
        }
    }

    fn dims(&self) -> (usize, usize) {
        on!(self, img => (img.w, img.h))
    }

    /// The samples as `f64` (exact for both dtypes).
    fn values(&self) -> Vec<f64> {
        match self {
            Pic::F32(img) => img.data.iter().map(|&v| f64::from(v)).collect(),
            Pic::F64(img) => img.data.clone(),
        }
    }

    /// `np.clip(img, 0, 255).astype(np.uint8)`, as `f64`.
    fn bytes(&self) -> Vec<f64> {
        match self {
            Pic::F32(img) => img.data.iter().map(|&v| f64::from(np::clip_u8_f32(v))).collect(),
            Pic::F64(img) => img.data.iter().map(|&v| f64::from(np::clip_u8(v))).collect(),
        }
    }
}

/// The picture `args.canvas` stands for (`canvas()` of the export module).
fn canvas_of(args: &Value) -> Result<Pic, String> {
    let spec = args["canvas"].as_object().ok_or("args.canvas is not an object")?;
    if let Some(k) = spec
        .keys()
        .find(|k| !["w", "h", "flat", "mul", "add", "dtype", "gain", "bias"].contains(&k.as_str()))
    {
        return Err(format!("args.canvas has a key this test does not know: {k}"));
    }
    let (w, h) = (size(&spec["w"], "canvas.w")?, size(&spec["h"], "canvas.h")?);
    let three = |key: &str| -> Result<[usize; 3], String> {
        let a = spec[key]
            .as_array()
            .filter(|a| a.len() == 3)
            .ok_or_else(|| format!("canvas.{key} is not three ints"))?;
        Ok([size(&a[0], key)?, size(&a[1], key)?, size(&a[2], key)?])
    };
    let mut bytes = Vec::with_capacity(w * h * 3);
    if spec.contains_key("mul") {
        let (mul, add) = (three("mul")?, three("add")?);
        for i in 0..w * h {
            for c in 0..3 {
                bytes.push(((mul[c] * i + add[c]) % 256) as f64);
            }
        }
    } else {
        let flat = three("flat")?;
        for _ in 0..w * h {
            bytes.extend(flat.map(|v| v as f64));
        }
    }
    if spec.contains_key("gain") {
        let (gain, bias) = (num(&spec["gain"], "canvas.gain")?, num(&spec["bias"], "canvas.bias")?);
        for v in &mut bytes {
            *v = *v * gain + bias;
        }
    }
    match spec["dtype"].as_str() {
        Some("float64") => Ok(Pic::F64(Img::from_vec(w, h, 3, bytes))),
        Some("float32") => Ok(Pic::F32(Img::from_vec(
            w,
            h,
            3,
            bytes.iter().map(|&v| v as f32).collect(),
        ))),
        other => Err(format!("canvas.dtype is {other:?}")),
    }
}

/// The array `out` as `f64`, whatever it was stored as.
fn stored(a: &NpArray) -> Result<Vec<f64>, String> {
    match a.dtype() {
        "|u1" => Ok(a.u8().iter().map(|&v| f64::from(v)).collect()),
        "<f4" => Ok(a.f32().iter().map(|&v| f64::from(v)).collect()),
        "<f8" => Ok(a.f64().to_vec()),
        other => Err(format!("out is stored as {other}")),
    }
}

/// A picture call's result against the case's `expect` and `out`. `canvas` is the picture as made; `got` what
/// the call gave: `(the result, its dtype, whether it is a new picture)`.
fn same_picture(v: &Vectors, case: &Case, canvas: &Pic, got: (&[f64], &str, bool)) -> Found {
    let expect = expect_of(v, case)?;
    let (values, dtype, new) = got;
    let returned = if new { "new" } else { "img" };
    if expect["returned"] != returned {
        return Err(format!("returned {returned}, Python {}", expect["returned"]));
    }
    if expect["dtype"] != dtype {
        return Err(format!("the result is {dtype}, Python's {}", expect["dtype"]));
    }
    let base = match expect["baseline"].as_str() {
        Some("canvas") => canvas.values(),
        Some("bytes") => canvas.bytes(),
        other => return Err(format!("expect.baseline is {other:?}")),
    };
    let (w, h) = canvas.dims();
    let mut want = base;
    if !expect["box"].is_null() {
        let b = expect["box"]
            .as_array()
            .filter(|b| b.len() == 4)
            .ok_or("expect.box is not a box")?;
        let (x0, y0, x1, y1) = (
            size(&b[0], "box")?,
            size(&b[1], "box")?,
            size(&b[2], "box")?,
            size(&b[3], "box")?,
        );
        let out = v.array(case, "out");
        if out.shape != [y1 - y0, x1 - x0, 3] || x1 > w || y1 > h {
            return Err(format!("out is {:?}, the box {b:?}", out.shape));
        }
        let crop = stored(out)?;
        for y in y0..y1 {
            let (to, from) = ((y * w + x0) * 3, (y - y0) * (x1 - x0) * 3);
            want[to..to + (x1 - x0) * 3].copy_from_slice(&crop[from..from + (x1 - x0) * 3]);
        }
    } else if case.arrays.contains_key("out") {
        return Err("the case has an out array and no box".into());
    }
    if values.len() != want.len() {
        return Err(format!("{} samples, Python has {}", values.len(), want.len()));
    }
    let bad: Vec<usize> = (0..want.len())
        .filter(|&i| values[i].to_bits() != want[i].to_bits())
        .collect();
    let Some(&first) = bad.first() else {
        return Ok(0.0);
    };
    let at = |i: usize| ((i / 3) % w, (i / 3) / w);
    let (xa, xb) = bad.iter().fold((w, 0), |(a, b), &i| (a.min(at(i).0), b.max(at(i).0)));
    let (ya, yb) = bad.iter().fold((h, 0), |(a, b), &i| (a.min(at(i).1), b.max(at(i).1)));
    let largest = bad.iter().map(|&i| (values[i] - want[i]).abs()).fold(0.0, f64::max);
    Err(format!(
        "{} samples differ in x {xa}..={xb}, y {ya}..={yb} (largest difference {largest:e}); the first at {:?} \
         channel {}: got {:?}, Python {:?}",
        bad.len(),
        at(first),
        first % 3,
        values[first],
        want[first]
    ))
}

/// An in-place call's result: the picture as it is now.
fn same_in_place(v: &Vectors, case: &Case, canvas: &Pic, now: &Pic) -> Found {
    same_picture(v, case, canvas, (&now.values(), now.dtype(), false))
}

/// A drawing call's result: a new `float32` picture, or nothing drawn.
fn same_drawn(v: &Vectors, case: &Case, canvas: &Pic, got: Option<Img<f32>>) -> Found {
    match got {
        Some(img) => {
            let values: Vec<f64> = img.data.iter().map(|&x| f64::from(x)).collect();
            same_picture(v, case, canvas, (&values, "float32", true))
        }
        None => {
            same_picture(v, case, canvas, (&canvas.values(), canvas.dtype(), false)).and_then(|_| {
                match expect_of(v, case)?["box"].is_null() {
                    true => Ok(0.0),
                    false => Err("nothing was drawn, Python drew".into()),
                }
            })
        }
    }
}

// ---------------------------------------------------------------- fx_timing
const TIMING: &Calls = &[
    ("env_ad(u, attack, decay, hold)", 4),
    ("FlashBudget(max_per_s, window).assign(times, priority)", 11),
    ("Shake(cap, decay, span).offset(t)", 5),
    ("Flares(max_n, span).at(t)", 4),
    ("Flares(max_n, span).tint(P, t, gain)", 25),
    ("Flares(max_n, span).apply(L, t)", 12),
    ("Moments(max_dim, hold, t_in, t_out).active(t)", 4),
    ("Moments(max_dim, hold, t_in, t_out).dim_k(t)", 4),
    ("Moments(max_dim, hold, t_in, t_out).env(t, t0)", 1),
    ("Marks(span, max_live).live(t)", 4),
];

/// A list of lights against the arrays `p`, `c` and `r` of a case.
fn same_lights(v: &Vectors, case: &Case, got: &[(V3, V3, f64)]) -> Found {
    let p: Vec<V3> = got.iter().map(|q| q.0).collect();
    let c: Vec<V3> = got.iter().map(|q| q.1).collect();
    let r: Vec<f64> = got.iter().map(|q| q.2).collect();
    worst_of(&[
        exact_f64(&flat3(&p), v.array(case, "p").f64(), "p"),
        close_rows(&flat3(&c), v.array(case, "c").f64(), 3, "c"),
        exact_f64(&r, v.array(case, "r").f64(), "r"),
    ])
}

fn timing_case(v: &Vectors, case: &Case) -> Found {
    let a = &case.args;
    let f = |role: &str| v.array(case, role).f64();
    let counts = |got: &[usize]| {
        let got: Vec<i64> = got.iter().map(|&n| n as i64).collect();
        exact(&got, v.array(case, "count").i64(), "count")
    };
    match case.call.as_str() {
        "env_ad(u, attack, decay, hold)" => {
            let u = f("u");
            let per_row = |key: &str| -> Result<Vec<f64>, String> {
                if case.arrays.contains_key(key) {
                    Ok(f(key).to_vec())
                } else {
                    Ok(vec![hexf(a, key)?; u.len()])
                }
            };
            let (attack, decay, hold) = (per_row("attack")?, per_row("decay")?, per_row("hold")?);
            let got: Vec<f64> = (0..u.len())
                .map(|i| fx::env_ad(u[i], attack[i], decay[i], hold[i]))
                .collect();
            close(&got, f("out"), "out")
        }
        "FlashBudget(max_per_s, window).assign(times, priority)" => {
            let budget = FlashBudget::new(size(&a["max_per_s"], "max_per_s")?, hexf(a, "window")?);
            let priority = flag(&a["priority"], "priority")?.then(|| f("priority"));
            exact(&budget.assign(f("times"), priority), v.array(case, "out").bool(), "out")
        }
        "Shake(cap, decay, span).offset(t)" => {
            let sh = shake_of(script_of(v, a)?, a)?;
            let got: Vec<f64> = f("t").iter().flat_map(|&t| <[f64; 2]>::from(sh.offset(t))).collect();
            close(&got, f("out"), "out")
        }
        "Flares(max_n, span).at(t)" => {
            let fl = flares_of(script_of(v, a)?)?;
            let got: Vec<Vec<(V3, V3, f64)>> = f("t").iter().map(|&t| fl.at(t)).collect();
            let n: Vec<usize> = got.iter().map(Vec::len).collect();
            worst_of(&[counts(&n), same_lights(v, case, &got.concat())])
        }
        "Flares(max_n, span).tint(P, t, gain)" => {
            let fl = flares_of(script_of(v, a)?)?;
            let got = fl.tint(&rows3(v.array(case, "P"))?, hexf(a, "t")?, hexf(a, "gain")?);
            close_rows(&flat3(&got), f("out"), 3, "out")
        }
        "Flares(max_n, span).apply(L, t)" => {
            let fl = flares_of(script_of(v, a)?)?;
            let mut l = Lights::new([0.36, 0.33, 0.44], [0.10, 0.08, 0.14]);
            fl.apply(&mut l, hexf(a, "t")?);
            same_lights(v, case, &l.points)
        }
        "Moments(max_dim, hold, t_in, t_out).active(t)" => {
            let mo = moments_of(script_of(v, a)?)?;
            let got: Vec<Vec<fx::moments::Active<'_>>> = f("t").iter().map(|&t| mo.active(t)).collect();
            let n: Vec<usize> = got.iter().map(Vec::len).collect();
            let all = got.concat();
            let which: Vec<i64> = all.iter().map(|q| q.which as i64).collect();
            worst_of(&[
                counts(&n),
                exact(&which, v.array(case, "which").i64(), "which"),
                exact_f64(&all.iter().map(|q| q.e).collect::<Vec<_>>(), f("e"), "e"),
                exact_f64(&all.iter().map(|q| q.u).collect::<Vec<_>>(), f("u"), "u"),
            ])
        }
        "Moments(max_dim, hold, t_in, t_out).dim_k(t)" => {
            let mo = moments_of(script_of(v, a)?)?;
            exact_f64(
                &f("t").iter().map(|&t| mo.dim_k(t)).collect::<Vec<_>>(),
                f("out"),
                "out",
            )
        }
        "Moments(max_dim, hold, t_in, t_out).env(t, t0)" => {
            let mo = moments_of(script_of(v, a)?)?;
            let got: Vec<f64> = f("t").iter().zip(f("t0")).map(|(&t, &t0)| mo.env(t, t0)).collect();
            exact_f64(&got, f("out"), "out")
        }
        "Marks(span, max_live).live(t)" => {
            let mk = marks_of(script_of(v, a)?)?;
            let got: Vec<Vec<LiveMark<'_>>> = f("t").iter().map(|&t| mk.live(t)).collect();
            let n: Vec<usize> = got.iter().map(Vec::len).collect();
            let all = got.concat();
            let which: Vec<i64> = all.iter().map(|q| q.which as i64).collect();
            worst_of(&[
                counts(&n),
                exact(&which, v.array(case, "which").i64(), "which"),
                exact_f64(&all.iter().map(|q| q.k).collect::<Vec<_>>(), f("k"), "k"),
                exact_f64(&all.iter().map(|q| q.age).collect::<Vec<_>>(), f("age"), "age"),
            ])
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn timing_matches_python() {
    check_calls("fx_timing", TIMING, timing_case);
}

// ---------------------------------------------------------------- fx_bursts
const BURSTS: &Calls = &[
    ("starburst(img, x, y, r, rgb, k, spikes, seed, hollow, bright)", 73),
    ("Bursts(max_per_s, base_px, per_strength, frames).bright_times()", 4),
    ("Bursts(max_per_s, base_px, per_strength, frames).draw(img, cam, t)", 25),
];

fn bursts_case(v: &Vectors, case: &Case) -> Found {
    let a = &case.args;
    match case.call.as_str() {
        "starburst(img, x, y, r, rgb, k, spikes, seed, hollow, bright)" => {
            let canvas = canvas_of(a)?;
            let mut pic = canvas_of(a)?;
            let (x, y, r, k) = (hexf(a, "x")?, hexf(a, "y")?, hexf(a, "r")?, hexf(a, "k")?);
            let rgb = vec3(&a["rgb"], "rgb")?;
            let spikes = size(&a["spikes"], "spikes")?;
            let seed = a["seed"].as_u64().ok_or("seed is not a whole number")?;
            let (hollow, bright) = (flag(&a["hollow"], "hollow")?, flag(&a["bright"], "bright")?);
            on!(&mut pic, img => fx::starburst(img, x, y, r, rgb, k, spikes, seed, hollow, bright));
            same_in_place(v, case, &canvas, &pic)
        }
        "Bursts(max_per_s, base_px, per_strength, frames).bright_times()" => {
            let bu = bursts_of(script_of(v, a)?)?;
            exact_f64(&bu.bright_times(), v.array(case, "out").f64(), "out")
        }
        "Bursts(max_per_s, base_px, per_strength, frames).draw(img, cam, t)" => {
            let bu = bursts_of(script_of(v, a)?)?;
            let canvas = canvas_of(a)?;
            let mut pic = canvas_of(a)?;
            let (cam, t) = (cam_of(a)?, hexf(a, "t")?);
            on!(&mut pic, img => bu.draw(img, &cam, t));
            same_in_place(v, case, &canvas, &pic)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn bursts_match_python() {
    check_calls("fx_bursts", BURSTS, bursts_case);
}

// ---------------------------------------------------------------- fx_warp
const WARP: &Calls = &[
    ("shimmer_world(img, cam, c, r, width, amp, t, k, seed)", 34),
    ("ellipse_of(cam, c, r)", 24),
    ("shimmer(img, cx, cy, ax, ay, width, amp, t, k, seed)", 14),
    ("fringe(img, cx, cy, r, px, k)", 30),
];

fn warp_case(v: &Vectors, case: &Case) -> Found {
    let a = &case.args;
    let h = |key: &str| hexf(a, key);
    let seed = || int(&a["seed"], "seed").map(|s| s as f64);
    match case.call.as_str() {
        "shimmer_world(img, cam, c, r, width, amp, t, k, seed)" => {
            let canvas = canvas_of(a)?;
            let mut pic = canvas_of(a)?;
            let (cam, c) = (cam_of(a)?, vec3(&a["c"], "c")?);
            let (r, width, amp, t, k, seed) = (h("r")?, h("width")?, h("amp")?, h("t")?, h("k")?, seed()?);
            on!(&mut pic, img => fx::shimmer_world(img, &cam, c, r, width, amp, t, k, seed));
            same_in_place(v, case, &canvas, &pic)
        }
        "ellipse_of(cam, c, r)" => {
            let (cx, cy, ax, ay) = fx::ellipse_of(&cam_of(a)?, vec3(&a["c"], "c")?, h("r")?);
            close_rows(&[cx, cy, ax, ay], v.array(case, "out").f64(), 4, "out")
        }
        "shimmer(img, cx, cy, ax, ay, width, amp, t, k, seed)" => {
            let canvas = canvas_of(a)?;
            let mut pic = canvas_of(a)?;
            let (cx, cy, ax, ay) = (h("cx")?, h("cy")?, h("ax")?, h("ay")?);
            let (width, amp, t, k, seed) = (h("width")?, h("amp")?, h("t")?, h("k")?, seed()?);
            on!(&mut pic, img => fx::shimmer(img, cx, cy, ax, ay, width, amp, t, k, seed));
            same_in_place(v, case, &canvas, &pic)
        }
        "fringe(img, cx, cy, r, px, k)" => {
            let canvas = canvas_of(a)?;
            let mut pic = canvas_of(a)?;
            let (cx, cy, r, px, k) = (h("cx")?, h("cy")?, h("r")?, h("px")?, h("k")?);
            on!(&mut pic, img => fx::fringe(img, cx, cy, r, px, k));
            same_in_place(v, case, &canvas, &pic)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn warps_match_python() {
    check_calls("fx_warp", WARP, warp_case);
}

// ---------------------------------------------------------------- fx_particles
const PARTICLES: &Calls = &[
    ("Particles(seed, max_alive) after the script", 5),
    ("Particles(seed, max_alive).alive(t)", 5),
    ("Particles._pos(A, idx, t - t0)", 16),
    ("Particles(seed, max_alive).draw(img, cam, t, light)", 38),
];

/// The particles against the columns of `_arrays()`.
fn same_columns(v: &Vectors, case: &Case, got: &[Particle]) -> Found {
    let f = |role: &str| v.array(case, role).f64();
    let col = |pick: fn(&Particle) -> f64| got.iter().map(pick).collect::<Vec<f64>>();
    let rows = |pick: fn(&Particle) -> V3| got.iter().flat_map(pick).collect::<Vec<f64>>();
    let flags = |pick: fn(&Particle) -> bool| got.iter().map(pick).collect::<Vec<bool>>();
    let shape: Vec<i64> = got.iter().map(|q| q.shape.index() as i64).collect();
    let tn: Vec<i64> = got.iter().map(|q| i64::from(q.tn)).collect();
    worst_of(&[
        exact_f64(&col(|q| q.t0), f("t0"), "t0"),
        close_rows(&rows(|q| q.p), f("p"), 3, "p"),
        close_rows(&rows(|q| q.v), f("v"), 3, "v"),
        exact_f64(&col(|q| q.life), f("life"), "life"),
        exact_f64(&col(|q| q.size), f("size"), "size"),
        exact_f64(&col(|q| q.grow), f("grow"), "grow"),
        exact_f64(&rows(|q| q.col), f("col"), "col"),
        exact_f64(&col(|q| q.g), f("g"), "g"),
        exact_f64(&col(|q| q.drag), f("drag"), "drag"),
        exact_f64(&col(|q| q.wa), f("wa"), "wa"),
        exact_f64(&col(|q| q.wf), f("wf"), "wf"),
        exact_f64(&col(|q| q.ph), f("ph"), "ph"),
        exact_f64(&col(|q| q.tw), f("tw"), "tw"),
        exact(&shape, v.array(case, "shape").i64(), "shape"),
        exact_f64(&col(|q| q.alpha), f("alpha"), "alpha"),
        exact(&flags(|q| q.ground), v.array(case, "ground").bool(), "ground"),
        exact_f64(&col(|q| q.spin), f("spin"), "spin"),
        exact(&flags(|q| q.glow), v.array(case, "glow").bool(), "glow"),
        exact(&tn, v.array(case, "tn").i64(), "tn"),
        exact_f64(&col(|q| q.tdt), f("tdt"), "tdt"),
    ])
}

fn particles_case(v: &Vectors, case: &Case) -> Found {
    let a = &case.args;
    let pa = particles_of(script_of(v, a)?)?;
    match case.call.as_str() {
        "Particles(seed, max_alive) after the script" => {
            if v.index.raw["shapes"] != serde_json::json!(["blob", "dot", "shard", "square", "streak", "ring"]) {
                return Err("the sidecar's shapes are not in the order of Shape".into());
            }
            debug_assert_eq!(Shape::Blob.index(), 0);
            same_columns(v, case, pa.particles())
        }
        "Particles(seed, max_alive).alive(t)" => {
            let got: Vec<Vec<usize>> = v.array(case, "t").f64().iter().map(|&t| pa.alive(t)).collect();
            let n: Vec<i64> = got.iter().map(|g| g.len() as i64).collect();
            let idx: Vec<i64> = got.concat().iter().map(|&i| i as i64).collect();
            worst_of(&[
                exact(&n, v.array(case, "count").i64(), "count"),
                exact(&idx, v.array(case, "idx").i64(), "idx"),
            ])
        }
        "Particles._pos(A, idx, t - t0)" => {
            let t = hexf(a, "t")?;
            let all = pa.particles();
            let got: Vec<f64> = pa.alive(t).iter().flat_map(|&i| all[i].pos(t - all[i].t0)).collect();
            close_rows(&got, v.array(case, "out").f64(), 3, "out")
        }
        "Particles(seed, max_alive).draw(img, cam, t, light)" => {
            let canvas = canvas_of(a)?;
            let (cam, t) = (cam_of(a)?, hexf(a, "t")?);
            let lit;
            let light: Option<fx::particles::Light<'_>> = if a["light"].is_null() {
                None
            } else {
                // lambda P: 1.0 + gain * FL.tint(P, t)
                let l = &a["light"];
                let fl = flares_of(script_of(v, l)?)?;
                let (gain, tl) = (hexf(l, "gain")?, hexf(l, "t")?);
                lit = move |p: &[V3]| -> Vec<V3> {
                    fl.tint(p, tl, 1.0).iter().map(|q| q.map(|x| 1.0 + gain * x)).collect()
                };
                Some(&lit)
            };
            let got = on!(&canvas, img => pa.draw(img, &cam, t, light));
            same_drawn(v, case, &canvas, got)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn particles_match_python() {
    check_calls("fx_particles", PARTICLES, particles_case);
}

// ---------------------------------------------------------------- fx_marks
const MARKS: &Calls = &[("Marks(span, max_live).apply(alb, U, V, t, dither)", 48)];

fn marks_case(v: &Vectors, case: &Case) -> Found {
    let a = &case.args;
    if case.call != "Marks(span, max_live).apply(alb, U, V, t, dither)" {
        return Err(format!("a call this test does not know: {}", case.call));
    }
    if v.index.raw["kinds"] != serde_json::json!(MarkKind::NAMES) {
        return Err("the sidecar's kinds are not MarkKind's".into());
    }
    let mk = marks_of(script_of(v, a)?)?;
    let g = &a["grid"];
    let (nu, nv) = (size(&g["nu"], "grid.nu")?, size(&g["nv"], "grid.nv")?);
    let (u0, du) = (num(&g["u0"], "grid.u0")?, num(&g["du"], "grid.du")?);
    let (v0, dv) = (num(&g["v0"], "grid.v0")?, num(&g["dv"], "grid.dv")?);
    let n = nu * nv;
    let u: Vec<f64> = (0..n).map(|i| u0 + (i % nu) as f64 * du).collect();
    let vv: Vec<f64> = (0..n).map(|i| v0 + (i / nu) as f64 * dv).collect();
    let weave = |key: &str| -> Result<[usize; 3], String> {
        let w = a["alb"][key]
            .as_array()
            .filter(|w| w.len() == 3)
            .ok_or("args.alb is not a weave")?;
        Ok([size(&w[0], key)?, size(&w[1], key)?, size(&w[2], key)?])
    };
    let (mul, add) = (weave("mul")?, weave("add")?);
    let before: Vec<V3> = (0..n)
        .map(|i| [0, 1, 2].map(|c| ((mul[c] * i + add[c]) % 256) as f64))
        .collect();
    let dither: Option<Vec<f64>> =
        flag(&a["dither"], "dither")?.then(|| (0..n).map(|i| BAYER[(i / nu) % 4][(i % nu) % 4]).collect());
    let mut alb = before.clone();
    let applied = mk.apply(&mut alb, &u, &vv, hexf(a, "t")?, dither.as_deref());
    let expect = expect_of(v, case)?;
    let returned = if applied { "new" } else { "alb" };
    if expect["returned"] != returned {
        return Err(format!("returned {returned}, Python {}", expect["returned"]));
    }
    let mut want = before;
    let rows = rows3(v.array(case, "out"))?;
    let changed = v.array(case, "changed").i64();
    if rows.len() != changed.len() {
        return Err(format!("{} changed points and {} rows", changed.len(), rows.len()));
    }
    for (&i, row) in changed.iter().zip(rows) {
        want[i as usize] = row;
    }
    close_rows(&flat3(&alb), &flat3(&want), 3, "alb")
}

#[test]
fn marks_match_python() {
    check_calls("fx_marks", MARKS, marks_case);
}

// ---------------------------------------------------------------- fx_moments
const MOMENTS: &Calls = &[
    ("Moments(max_dim, hold, t_in, t_out).apply_dim(img, t, keep)", 15),
    (
        "Moments(max_dim, hold, t_in, t_out).speed_lines(img, t, key, x, y, r, direction, n, length, seed)",
        19,
    ),
];

fn moments_case(v: &Vectors, case: &Case) -> Found {
    let a = &case.args;
    let mo = moments_of(script_of(v, a)?)?;
    let t = hexf(a, "t")?;
    match case.call.as_str() {
        "Moments(max_dim, hold, t_in, t_out).apply_dim(img, t, keep)" => {
            let Pic::F64(mut img) = canvas_of(a)? else {
                return Err("apply_dim on a picture that is not float64".into());
            };
            let before = img.clone();
            let keep: Vec<(f64, f64, f64)> = a["keep"]
                .as_array()
                .ok_or("args.keep is not a list")?
                .iter()
                .map(|q| vec3(q, "keep").map(|[x, y, r]| (x, y, r)))
                .collect::<Result<_, _>>()?;
            let expect = expect_of(v, case)?;
            exact_f64(&[mo.dim_k(t)], &[num(&expect["k"], "expect.k")?], "dim_k")?;
            let dimmed = mo.apply_dim(&mut img, t, &keep);
            let returned = if dimmed { "new" } else { "img" };
            if expect["returned"] != returned || expect["dtype"] != "float64" {
                return Err(format!(
                    "returned {returned}, Python {} of {}",
                    expect["returned"], expect["dtype"]
                ));
            }
            if !dimmed {
                return exact_f64(&img.data, &before.data, "the picture");
            }
            let step = size(&expect["step"], "expect.step")?;
            let got: Vec<f64> = img.data.chunks(3).step_by(step).flatten().copied().collect();
            exact_f64(&got, v.array(case, "out").f64(), "out")
        }
        "Moments(max_dim, hold, t_in, t_out).speed_lines(img, t, key, x, y, r, direction, n, length, seed)" => {
            let canvas = canvas_of(a)?;
            let (key, n) = (int(&a["key"], "key")?, size(&a["n"], "n")?);
            let (x, y, r) = (hexf(a, "x")?, hexf(a, "y")?, hexf(a, "r")?);
            let (direction, length) = (pair(&a["direction"], "direction")?, pair(&a["length"], "length")?);
            let seed = a["seed"].as_u64().ok_or("seed is not a whole number")?;
            let got = on!(&canvas, img => mo.speed_lines(img, t, key, x, y, r, direction, n, length, seed));
            same_drawn(v, case, &canvas, got)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn moments_match_python() {
    check_calls("fx_moments", MOMENTS, moments_case);
}

// ---------------------------------------------------------------- the test's own guards
/// The six files, each with the calls it must hold and the function that runs a case.
type Group = (&'static str, &'static Calls, fn(&Vectors, &Case) -> Found);

const GROUPS: [Group; 6] = [
    ("fx_timing", TIMING, timing_case),
    ("fx_bursts", BURSTS, bursts_case),
    ("fx_warp", WARP, warp_case),
    ("fx_particles", PARTICLES, particles_case),
    ("fx_marks", MARKS, marks_case),
    ("fx_moments", MOMENTS, moments_case),
];

/// An unknown call fails its case, in every group.
#[test]
fn unknown_calls_are_failures() {
    for (name, _, run) in GROUPS {
        let v = vectors(name);
        let mut case = v.index.cases[0].clone();
        case.call = "np.sqrt(x)".into();
        if case.args["script"].is_null() {
            // the groups that build their object first need a script to get as far as the call
            case.args["script"] = v.index.raw["scripts"]
                .as_object()
                .and_then(|s| s.keys().next())
                .map_or(Value::Null, |k| Value::String(k.clone()));
        }
        assert!(run(v, &case).unwrap_err().contains("np.sqrt"), "{name}");
    }
}

/// The effects kit is 424 cases, and a file that holds other calls or other numbers of them is noticed.
#[test]
fn the_expected_cases_are_counted() {
    let total: usize = GROUPS.iter().flat_map(|g| g.1).map(|c| c.1).sum();
    assert_eq!(total, 424);
    for (name, calls, _) in GROUPS {
        let mut names: Vec<&str> = calls.iter().map(|c| c.0).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), calls.len(), "{name}: a call is listed twice");
    }
    let expected: &Calls = &[("f(x)", 3), ("g(x)", 2)];
    let found = |pairs: &[(&'static str, usize)]| pairs.iter().copied().collect::<BTreeMap<_, _>>();
    assert!(how_the_calls_differ(expected, &found(&[("g(x)", 2), ("f(x)", 3)])).is_empty());
    assert_eq!(
        how_the_calls_differ(expected, &found(&[("f(x)", 2), ("g(x)", 2)])).len(),
        1
    );
    assert_eq!(how_the_calls_differ(expected, &found(&[("f(x)", 3)])).len(), 1);
    assert_eq!(
        how_the_calls_differ(expected, &found(&[("f(x)", 3), ("g(x)", 2), ("h(x)", 1)])).len(),
        1
    );
}

/// The real calls are there: every file with cases from `kit.picture` has them, and the scripts of the example
/// episode are the sizes the kit builds (187 emits, 135 flares, 133 bursts, 68 marks, 5 moments, 135 jolts).
#[test]
fn the_vectors_cover_the_real_calls() {
    for (name, _, _) in GROUPS {
        let v = vectors(name);
        let mut real: BTreeMap<&str, usize> = BTreeMap::new();
        for c in &v.index.cases {
            if c.args["source"] == "kit.picture" {
                *real.entry(c.call.as_str()).or_default() += 1;
            }
        }
        assert!(real.values().sum::<usize>() >= 8, "{name}: {real:?}");
    }
    let calls = |file: &str, script: &str| {
        vectors(file).index.raw["scripts"][script]["calls"]
            .as_array()
            .map(Vec::len)
    };
    assert_eq!(calls("fx_particles", "particles kit.picture"), Some(187));
    assert_eq!(calls("fx_timing", "flares kit.picture"), Some(135));
    assert_eq!(calls("fx_bursts", "bursts kit.picture"), Some(133));
    assert_eq!(calls("fx_marks", "marks kit.picture"), Some(68));
    assert_eq!(calls("fx_moments", "moments kit.picture"), Some(5));
    assert_eq!(calls("fx_timing", "shake kit.picture"), Some(135));
    let pa = particles_of(&vectors("fx_particles").index.raw["scripts"]["particles kit.picture"]).unwrap();
    assert_eq!(pa.particles().len(), 1276);
}

/// The comparison's own guard: what counts as close, and what does not.
#[test]
fn closeness_is_measured_value_by_value_with_a_floor() {
    assert_eq!(close_rows(&[1.0, 2.0], &[1.0, 2.0], 2, ""), Ok(0.0));
    assert!(close(&[1.0 + 1e-12], &[1.0], "").unwrap() < 2e-12);
    assert!(close(&[1.0 + 3e-9], &[1.0], "").is_err());
    assert!(close(&[3e-16], &[-1e-16], "").is_err());
    assert!(close_rows(&[1.0 + 2e-9, 1000.0], &[1.0, 1000.0], 2, "").is_err());
    assert!(close_rows(&[9e-12, 10.0], &[0.0, 10.0], 2, "").is_ok());
    assert!(close_rows(&[2e-11, 10.0], &[0.0, 10.0], 2, "").is_err());
    assert!(close_rows(&[1e-300, 0.0], &[0.0, 0.0], 2, "").is_err());
    assert!(close(&[f64::NAN], &[1.0], "").is_err());
    assert!(close(&[1.0], &[f64::NAN], "").is_err());
    assert_eq!(close(&[f64::NAN], &[f64::NAN], ""), Ok(0.0));
    assert!(close_rows(&[f64::NAN, 1.0 + 1e-15], &[f64::NAN, 1.0], 2, "").is_err());
    assert!(close(&[1.0, 2.0], &[1.0], "").is_err());
    assert!(exact_f64(&[0.0], &[-0.0], "").is_err());
    assert!(exact_f64(&[f64::NAN], &[f64::NAN], "").is_ok());
    assert!(exact(&[1u8, 2], &[1, 3], "").is_err());
}
