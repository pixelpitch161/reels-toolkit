//! The on-screen text layer against Python: `bf_picture::ui` (captions, the fight HUD, labels, panels and the draw
//! log) on the cases of `studio/reference/picture_kit/ui_*.npz` (written by
//! `tools/studio_export/kit_ui_vectors.py`).
//!
//! Every picture is compared byte for byte, every noted box and returned value exactly. One test per file; each
//! prints one result line per Python function (`fight_hud: 115/115`) and lists the cases that fail, with the
//! count of differing pixels, the largest difference and the box around them. A case whose `call` the test does
//! not know is a failure, and so is a keyword or an argument it does not know. A panic inside the function under
//! test fails that case only.
//!
//! The one comparison that is not exact is `_hash01`'s result: Python takes its sine from the C runtime of the
//! reference PC, the Studio from `bf_core::dmath`, and the two sines are allowed to differ in their last bit. The
//! number the sine is taken of is compared bit for bit; see [`HASH_TOLERANCE`].

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, OnceLock};

use bf_picture::img::Img;
use bf_picture::ui::captions::{self, Caption, CaptionStyle};
use bf_picture::ui::hud::{self, AbilityRow, Fighter, HudOptions};
use bf_picture::ui::{self, DrawLog, Kind};
use bf_testdata::{Case, NpArray, Vectors, diff_u8, hex_arg};
use serde_json::Value;

type Check = Result<(), String>;

/// How far `_hash01` may be from Python's value. `_hash01` is `v - floor(v)` with `v = sin(phase) * 43758.5453`.
/// Two sines one unit in the last place apart (at most 2^-53 = 1.11e-16 for a sine of 0.5 or more) put the two
/// products 4.9e-12 apart, and each product is rounded to a multiple of 7.3e-12 (the spacing of doubles at 43758)
/// or finer: 1.22e-11 in all. Anything beyond that is not a last-bit difference of the sine.
const HASH_TOLERANCE: f64 = 1.25e-11;

// ---------------------------------------------------------------- reading a case
/// `picture_kit/<name>.npz` with its sidecar, read once for the whole test binary: several tests look at the same
/// file, and one file unpacks to as much as 120 MB of pictures.
fn vectors(name: &str) -> &'static Vectors {
    static OPEN: OnceLock<Mutex<BTreeMap<String, &'static Vectors>>> = OnceLock::new();
    let mut open = OPEN
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    open.entry(name.to_string())
        .or_insert_with(|| Box::leak(Box::new(Vectors::open(&format!("picture_kit/{name}.npz")))))
}

fn obj<'a>(v: &'a Value, what: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    v.as_object().ok_or_else(|| format!("{what} is {v}, not an object"))
}

fn int_of(v: &Value, what: &str) -> Result<i32, String> {
    v.as_i64()
        .and_then(|x| i32::try_from(x).ok())
        .ok_or_else(|| format!("{what} is {v}, not an int"))
}

fn int_arg(args: &Value, key: &str) -> Result<i32, String> {
    int_of(&args[key], &format!("args.{key}"))
}

fn float_of(v: &Value, what: &str) -> Result<f64, String> {
    match v {
        Value::String(_) => Ok(hex_arg(v)),
        other => Err(format!("{what} is {other}, not a float.hex() string")),
    }
}

fn float_arg(args: &Value, key: &str) -> Result<f64, String> {
    float_of(&args[key], &format!("args.{key}"))
}

fn str_of<'a>(v: &'a Value, what: &str) -> Result<&'a str, String> {
    v.as_str().ok_or_else(|| format!("{what} is {v}, not a string"))
}

fn rgb_of(v: &Value, what: &str) -> Result<[u8; 3], String> {
    let bad = || format!("{what} {v} is not three ints from 0 to 255");
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(bad)?;
    let mut out = [0u8; 3];
    for (o, x) in out.iter_mut().zip(a) {
        *o = x.as_u64().filter(|&u| u <= 255).ok_or_else(bad)? as u8;
    }
    Ok(out)
}

/// The picture `args.canvas` stands for (`canvas()` of the export module): one colour, or a weave in which pixel
/// `i` (counted row by row) has the channels `(mul[c] * i + add[c]) % 256`, all over or in the rows `rows`.
fn canvas_of(args: &Value) -> Result<Img<u8>, String> {
    let spec = obj(&args["canvas"], "args.canvas")?;
    if let Some(k) = spec
        .keys()
        .find(|k| !["w", "h", "flat", "mul", "add", "rows"].contains(&k.as_str()))
    {
        return Err(format!("args.canvas has a key this test does not know: {k}"));
    }
    let size = |key: &str| {
        spec.get(key)
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .ok_or_else(|| format!("args.canvas.{key} is not a size"))
    };
    let (w, h) = (size("w")?, size("h")?);
    let Some(mul) = spec.get("mul") else {
        return Ok(Img::filled(w, h, &rgb_of(&spec["flat"], "canvas.flat")?));
    };
    let (mul, add) = (rgb_of(mul, "canvas.mul")?, rgb_of(&spec["add"], "canvas.add")?);
    let woven = |i: usize, c: usize| ((usize::from(mul[c]) * i + usize::from(add[c])) % 256) as u8;
    let (ya, yb, flat) = match spec.get("rows") {
        None => (0, h, [0; 3]),
        Some(rows) => {
            let r = rows
                .as_array()
                .filter(|r| r.len() == 2)
                .ok_or("canvas.rows is not a pair")?;
            let end = |v: &Value| {
                v.as_u64()
                    .map(|v| v as usize)
                    .ok_or("canvas.rows is not a pair of rows")
            };
            (end(&r[0])?, end(&r[1])?, rgb_of(&spec["flat"], "canvas.flat")?)
        }
    };
    let mut img = Img::filled(w, h, &flat);
    for y in ya..yb.min(h) {
        for x in 0..w {
            let i = y * w + x;
            img.px_mut(x, y)
                .copy_from_slice(&[woven(i, 0), woven(i, 1), woven(i, 2)]);
        }
    }
    Ok(img)
}

fn same_image(got: &Img<u8>, want: &NpArray) -> Check {
    if want.shape != [got.h, got.w, got.c] {
        return Err(format!(
            "a picture of shape {:?}, Python's is {:?}",
            [got.h, got.w, got.c],
            want.shape
        ));
    }
    let d = diff_u8(&got.data, want.u8(), got.w, got.h, got.c);
    if d.differing == 0 { Ok(()) } else { Err(d.to_string()) }
}

/// The case's `expect` object: what the Python call returned and noted. It sits beside `args` in the sidecar.
fn expect_of<'a>(v: &'a Vectors, case: &Case) -> Result<&'a Value, String> {
    v.index.raw["cases"]
        .as_array()
        .and_then(|cases| cases.iter().find(|c| c["id"] == case.id.as_str()))
        .map(|c| &c["expect"])
        .filter(|e| e.is_object())
        .ok_or_else(|| "the case has no expect object".to_string())
}

/// The draw log against `expect.noted`: rows of `[x0, y0, x1, y1, kind, info]`, in the order they were noted.
fn same_noted(log: &DrawLog, expect: &Value) -> Check {
    let want = expect["noted"].as_array().ok_or("expect.noted is not a list")?;
    let got: Vec<Value> = log
        .boxes
        .iter()
        .map(|b| serde_json::json!([b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info]))
        .collect();
    if &got == want {
        return Ok(());
    }
    let first = (0..got.len().max(want.len()))
        .find(|&i| got.get(i) != want.get(i))
        .unwrap_or(0);
    Err(format!(
        "noted {} boxes, Python {}; first difference at [{first}]: got {}, Python {}",
        got.len(),
        want.len(),
        got.get(first).unwrap_or(&Value::Null),
        want.get(first).unwrap_or(&Value::Null)
    ))
}

/// The keywords a call passed: the `name=name` items after `head` in the call text. Every one must be in
/// `known` and in `args`; every other key of `args` must be one of `positional` or `notes` (fields that describe
/// the case and are no argument).
fn keywords<'a>(
    case: &'a Case,
    head: &str,
    positional: &[&str],
    known: &[&str],
    notes: &[&str],
) -> Result<Vec<&'a str>, String> {
    let bad = || unknown(case);
    let rest = case
        .call
        .strip_prefix(head)
        .and_then(|r| r.strip_suffix(')'))
        .ok_or_else(bad)?;
    let mut kws = Vec::new();
    for part in rest.split(", ").filter(|p| !p.is_empty()) {
        let (k, val) = part.split_once('=').ok_or_else(bad)?;
        if k != val || !known.contains(&k) || kws.contains(&k) {
            return Err(bad());
        }
        kws.push(k);
    }
    let args = obj(&case.args, "args")?;
    for k in &kws {
        if !args.contains_key(*k) {
            return Err(format!("the call passes {k}, args do not hold it"));
        }
    }
    for k in args.keys() {
        let k = k.as_str();
        if !(kws.contains(&k) || positional.contains(&k) || notes.contains(&k)) {
            return Err(format!("args.{k}: an argument the call does not name"));
        }
    }
    for k in positional {
        if !args.contains_key(*k) {
            return Err(format!("args.{k} is missing"));
        }
    }
    Ok(kws)
}

/// What a case whose call the test has no code for fails with.
fn unknown(case: &Case) -> String {
    format!("a call this test does not know: {}", case.call)
}

/// The Python function a call names: the text between `reelkit.ui.` and the first `(`.
fn function_of(call: &str) -> &str {
    let name = call.strip_prefix("reelkit.ui.").unwrap_or(call);
    name.split('(').next().unwrap_or(name)
}

/// Runs every case of `picture_kit/<name>.npz` through `run`, prints one line per Python function and panics
/// with the failing cases unless all match. A panic inside `run` fails its case.
fn check_file(name: &str, run: impl Fn(&Vectors, &Case) -> Check) {
    let v = vectors(name);
    assert!(!v.index.cases.is_empty(), "{name}: no cases");
    let mut tally: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    for case in &v.index.cases {
        let result = catch_unwind(AssertUnwindSafe(|| run(v, case))).unwrap_or_else(|panic| {
            let text = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("a panic without a message");
            Err(format!("the port panicked: {text}"))
        });
        let entry = tally.entry(function_of(&case.call)).or_default();
        entry.1 += 1;
        match result {
            Ok(()) => entry.0 += 1,
            Err(why) => failures.push(format!(
                "  {} {}: {why}\n      args {}",
                case.id,
                case.call,
                brief(&case.args)
            )),
        }
    }
    let lines: Vec<String> = tally
        .iter()
        .map(|(function, (passed, total))| format!("{function}: {passed}/{total}"))
        .collect();
    println!("{}", lines.join("\n"));
    assert!(
        failures.is_empty(),
        "{name}: {}\n{} failing case(s):\n{}",
        lines.join(", "),
        failures.len(),
        failures.join("\n")
    );
}

/// The arguments of a failing case in one line, cut short (a HUD case holds two fighters and six ability rows).
fn brief(args: &Value) -> String {
    let text = args.to_string();
    match text.char_indices().nth(400) {
        Some((cut, _)) => format!("{} ...", &text[..cut]),
        None => text,
    }
}

// ---------------------------------------------------------------- label and panel
fn label_case(v: &Vectors, case: &Case) -> Check {
    let args = &case.args;
    let is_label = case.call.starts_with("reelkit.ui.label(");
    if !is_label && !case.call.starts_with("reelkit.ui.panel(") {
        return Err(unknown(case));
    }
    let expect = expect_of(v, case)?;
    let mut img = canvas_of(args)?;
    let mut log = DrawLog::new();
    if is_label {
        // def label(img, text, x, y, col, scale=2, outline=(10, 20, 26))
        let (head, positional): (&str, &[&str]) =
            if case.call.starts_with("reelkit.ui.label(img, text, x, y, col, scale") {
                (
                    "reelkit.ui.label(img, text, x, y, col, scale",
                    &["canvas", "text", "x", "y", "col", "scale"],
                )
            } else {
                (
                    "reelkit.ui.label(img, text, x, y, col",
                    &["canvas", "text", "x", "y", "col"],
                )
            };
        let kws = keywords(case, head, positional, &["outline"], &["frame", "source"])?;
        let scale = if positional.contains(&"scale") {
            u32::try_from(int_arg(args, "scale")?).map_err(|e| e.to_string())?
        } else {
            2
        };
        let outline = if kws.contains(&"outline") {
            match &args["outline"] {
                Value::Null => None,
                o => Some(rgb_of(o, "outline")?),
            }
        } else {
            Some(bf_picture::font::OUTLINE)
        };
        let width = ui::label(
            &mut img,
            &mut log,
            str_of(&args["text"], "args.text")?,
            int_arg(args, "x")?,
            int_arg(args, "y")?,
            rgb_of(&args["col"], "col")?,
            scale,
            outline,
        );
        if expect["returns"] != width {
            return Err(format!("returned {width}, Python {}", expect["returns"]));
        }
    } else {
        // def panel(draw, x0, y0, x1, y1, fill=(16, 20, 38), edge=(62, 74, 112))
        let kws = keywords(
            case,
            "reelkit.ui.panel(draw, x0, y0, x1, y1",
            &["canvas", "x0", "y0", "x1", "y1"],
            &["fill", "edge"],
            &[],
        )?;
        let colour = |key: &str, default: [u8; 3]| {
            if kws.contains(&key) {
                rgb_of(&args[key], key)
            } else {
                Ok(default)
            }
        };
        ui::panel(
            &mut img,
            &mut log,
            int_arg(args, "x0")?,
            int_arg(args, "y0")?,
            int_arg(args, "x1")?,
            int_arg(args, "y1")?,
            colour("fill", ui::label::PANEL_FILL)?,
            colour("edge", ui::label::PANEL_EDGE)?,
        );
    }
    same_noted(&log, expect)?;
    same_image(&img, v.array(case, "out"))
}

#[test]
fn label_and_panel() {
    check_file("ui_label", label_case);
}

// ---------------------------------------------------------------- _hash01 and the sand puff
/// One `_hash01` case: the phase bit for bit, the result within [`HASH_TOLERANCE`] of Python's. The difference is
/// taken as it is, not around the wrap from 1 to 0: a value whose floor flipped (0.99999 against 0.00001) is not
/// a last bit of the sine, and fails. Returns how many results are Python's bit for bit and the largest
/// difference.
fn hash_rows(v: &Vectors, case: &Case) -> Result<(usize, usize, f64), String> {
    let names: Vec<&str> = case
        .call
        .strip_prefix("reelkit.ui._hash01(")
        .and_then(|r| r.strip_suffix(')'))
        .ok_or_else(|| unknown(case))?
        .split(", ")
        .collect();
    if names.is_empty() || names.len() > 4 || names != ["a", "b", "c", "d"][..names.len()] {
        return Err(unknown(case));
    }
    if case.arrays.len() != names.len() + 2 {
        return Err(format!("the case holds {} arrays", case.arrays.len()));
    }
    let cols: Vec<&[i64]> = names.iter().map(|n| v.array(case, n).i64()).collect();
    let (phase, out) = (v.array(case, "phase").f64(), v.array(case, "out").f64());
    let (mut exact, mut worst) = (0, 0.0f64);
    for row in 0..out.len() {
        let xs: Vec<i64> = cols.iter().map(|c| c[row]).collect();
        let got_phase = captions::hash01_phase(&xs);
        if got_phase.to_bits() != phase[row].to_bits() {
            return Err(format!(
                "row {row} {xs:?}: the phase is {got_phase:e}, Python's {:e}",
                phase[row]
            ));
        }
        let got = captions::hash01(&xs);
        if !(0.0..=1.0).contains(&got) {
            return Err(format!("row {row} {xs:?}: {got} is outside 0..1"));
        }
        let d = (got - out[row]).abs();
        if d > HASH_TOLERANCE {
            return Err(format!(
                "row {row} {xs:?}: got {got:e}, Python {:e}: {d:e} apart, more than a last bit of the sine",
                out[row]
            ));
        }
        exact += usize::from(got.to_bits() == out[row].to_bits());
        worst = worst.max(d);
    }
    Ok((exact, out.len(), worst))
}

fn puff_case(v: &Vectors, case: &Case) -> Check {
    let args = &case.args;
    if case.call.starts_with("reelkit.ui._hash01(") {
        return hash_rows(v, case).map(|_| ());
    }
    keywords(
        case,
        "reelkit.ui._sand_puff(img, t, tl, x0, x1, ybot, col, key",
        &["canvas", "t", "tl", "x0", "x1", "ybot", "col", "key"],
        &[],
        &["line", "frame", "source"],
    )?;
    let col = match &args["col"] {
        Value::Null => None,
        c => Some(rgb_of(c, "col")?),
    };
    let mut img = canvas_of(args)?;
    captions::sand_puff(
        &mut img,
        float_arg(args, "t")?,
        float_arg(args, "tl")?,
        int_arg(args, "x0")?,
        int_arg(args, "x1")?,
        int_arg(args, "ybot")?,
        col,
        args["key"].as_i64().ok_or("args.key is not an int")?,
    );
    same_image(&img, v.array(case, "out"))
}

#[test]
fn hash_and_sand_puff() {
    check_file("ui_puff", puff_case);
    // How close the Studio's sine keeps `_hash01` to Python's, for the record.
    let v = vectors("ui_puff");
    let (mut exact, mut total, mut worst) = (0, 0, 0.0f64);
    for case in v
        .index
        .cases
        .iter()
        .filter(|c| c.call.starts_with("reelkit.ui._hash01("))
    {
        let (e, n, w) = hash_rows(v, case).expect("checked above");
        exact += e;
        total += n;
        worst = worst.max(w);
    }
    println!("_hash01 values: {exact}/{total} are Python's bit for bit, the largest difference is {worst:e}");
    assert!(total > 4000 && exact * 10 > total * 9, "{exact} of {total}");
    let k = v.index.raw["HASH_K"].as_array().expect("HASH_K in the sidecar");
    assert_eq!(k.len(), captions::HASH_K.len());
    for (got, want) in captions::HASH_K.iter().zip(k) {
        assert_eq!(got.to_bits(), hex_arg(want).to_bits(), "a multiplier of _hash01");
    }
    assert_eq!(hex_arg(&v.index.raw["PUFF_T"]).to_bits(), captions::PUFF_T.to_bits());
}

// ---------------------------------------------------------------- captions
fn caps_of(v: &Value) -> Result<Vec<Caption>, String> {
    let list = v.as_array().ok_or("args.caps is not a list")?;
    let mut caps = Vec::with_capacity(list.len());
    for c in list {
        let lines = c["lines"].as_array().ok_or("a caption's lines are not a list")?;
        let lines: Result<Vec<String>, String> = lines
            .iter()
            .map(|l| str_of(l, "a caption line").map(str::to_string))
            .collect();
        caps.push(Caption {
            t0: float_of(&c["t0"], "t0")?,
            t1: float_of(&c["t1"], "t1")?,
            lines: lines?,
        });
    }
    Ok(caps)
}

fn caption_case(v: &Vectors, case: &Case) -> Check {
    let args = &case.args;
    let known = ["caption_scale", "impact_landings", "draw_captions"];
    if !known.contains(&function_of(&case.call)) {
        return Err(unknown(case));
    }
    let expect = expect_of(v, case)?;
    if case.call == "reelkit.ui.caption_scale(lines)" {
        let lines = args["lines"].as_array().ok_or("args.lines is not a list")?;
        let lines: Result<Vec<&str>, String> = lines.iter().map(|l| str_of(l, "a line")).collect();
        let got = captions::caption_scale(&lines?);
        return if expect["returns"] == got {
            Ok(())
        } else {
            Err(format!("returned {got}, Python {}", expect["returns"]))
        };
    }
    if case.call.starts_with("reelkit.ui.impact_landings(") {
        // def impact_landings(caps, stagger=IMPACT_STAGGER, fall=IMPACT_FALL)
        let kws = keywords(
            case,
            "reelkit.ui.impact_landings(caps",
            &["caps"],
            &["stagger", "fall"],
            &[],
        )?;
        let number = |key: &str, default: f64| {
            if kws.contains(&key) {
                float_arg(args, key)
            } else {
                Ok(default)
            }
        };
        let got = captions::impact_landings(
            &caps_of(&args["caps"])?,
            number("stagger", captions::IMPACT_STAGGER)?,
            number("fall", captions::IMPACT_FALL)?,
        );
        let want = expect["returns"].as_array().ok_or("expect.returns is not a list")?;
        if got.len() != want.len() {
            return Err(format!("{} landings, Python {}", got.len(), want.len()));
        }
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            let t = float_of(&w[0], "a landing time")?;
            if g.t.to_bits() != t.to_bits() || w[1] != g.line || w[2] != g.chars {
                return Err(format!("landing {i}: got {g:?}, Python [{t}, {}, {}]", w[1], w[2]));
            }
        }
        return Ok(());
    }
    // def draw_captions(img, t, caps, y0=None, cx=None, base=CREAM, hi=AMBER, alt=SKYB, cps=55, style='type',
    //                   puff=(236, 204, 150))
    let kws = keywords(
        case,
        "reelkit.ui.draw_captions(img, t, caps, style='impact'",
        &["canvas", "t", "caps"],
        &["y0", "cx", "base", "hi", "alt", "puff"],
        &["frame", "source"],
    )?;
    let mut style = CaptionStyle::default();
    for k in kws {
        match k {
            "y0" => style.y0 = int_arg(args, "y0")?,
            "cx" => style.cx = int_arg(args, "cx")?,
            "base" => style.base = rgb_of(&args["base"], "base")?,
            "hi" => style.hi = rgb_of(&args["hi"], "hi")?,
            "alt" => style.alt = rgb_of(&args["alt"], "alt")?,
            _ => {
                style.puff = match &args["puff"] {
                    Value::Null => None,
                    c => Some(rgb_of(c, "puff")?),
                }
            }
        }
    }
    let mut img = canvas_of(args)?;
    let mut log = DrawLog::new();
    captions::draw_captions(
        &mut img,
        &mut log,
        float_arg(args, "t")?,
        &caps_of(&args["caps"])?,
        &style,
    );
    same_noted(&log, expect)?;
    same_image(&img, v.array(case, "out"))
}

#[test]
fn captions_and_landings() {
    check_file("ui_captions", caption_case);
}

#[test]
fn caption_constants() {
    let v = vectors("ui_captions");
    let c = &v.index.raw["constants"];
    let same_float = |key: &str, got: f64| assert_eq!(hex_arg(&c[key]).to_bits(), got.to_bits(), "{key}");
    assert_eq!(c["CAP_Y0"], captions::CAP_Y0);
    assert_eq!(c["CAP_BIG_MAX"], captions::CAP_BIG_MAX);
    assert_eq!(c["CAP_SMALL_MAX"], captions::CAP_SMALL_MAX);
    assert_eq!(c["IMPACT_DROP"].as_f64(), Some(captions::IMPACT_DROP));
    same_float("IMPACT_FALL", captions::IMPACT_FALL);
    same_float("IMPACT_STAGGER", captions::IMPACT_STAGGER);
    same_float("IMPACT_PUNCH", captions::IMPACT_PUNCH);
    same_float("IMPACT_PUNCH_T", captions::IMPACT_PUNCH_T);
    same_float("IMPACT_SHAKE", captions::IMPACT_SHAKE);
    same_float("IMPACT_SHAKE_T", captions::IMPACT_SHAKE_T);
    same_float("PUFF_T", captions::PUFF_T);
    let colours = [
        ("CREAM", ui::CREAM),
        ("AMBER", ui::AMBER),
        ("SKYB", ui::SKYB),
        ("MINT", ui::MINT),
        ("CORAL", ui::CORAL),
        ("PINK", ui::PINK),
        ("CYAN", ui::CYAN),
        ("INK", ui::INK),
    ];
    for (key, got) in colours {
        assert_eq!(c[key], serde_json::json!(got), "{key}");
    }
    assert_eq!(
        c.as_object().map(|o| o.len()),
        Some(19),
        "a constant this test does not check"
    );
    let style = CaptionStyle::default();
    assert_eq!((style.y0, style.cx), (captions::CAP_Y0, bf_picture::config::SAFE_CX));
    assert_eq!((style.base, style.hi, style.alt), (ui::CREAM, ui::AMBER, ui::SKYB));
    assert_eq!(style.puff, Some([236, 204, 150]));
}

// ---------------------------------------------------------------- the fight HUD
/// A fighter dict: `fight_hud` reads every key but `name`, `color` and `hp` with a default.
fn fighter_of(v: &Value, what: &str) -> Result<Fighter, String> {
    let d = obj(v, what)?;
    let color = d
        .get("color")
        .and_then(Value::as_array)
        .filter(|c| c.len() == 3)
        .ok_or_else(|| format!("{what}.color is not three floats"))?;
    let mut f = Fighter::new(
        str_of(&v["name"], "name")?,
        [
            float_of(&color[0], "color")?,
            float_of(&color[1], "color")?,
            float_of(&color[2], "color")?,
        ],
        float_of(&v["hp"], "hp")?,
    );
    for (k, val) in d {
        let key = format!("{what}.{k}");
        match k.as_str() {
            "name" | "color" | "hp" => {}
            "hp_max" => f.hp_max = float_of(val, &key)?,
            "trail" => {
                f.trail = match val {
                    Value::Null => None,
                    x => Some(float_of(x, &key)?),
                }
            }
            "level" => f.level = int_of(val, &key)?,
            "max_level" => f.max_level = int_of(val, &key)?,
            "status" => f.status = str_of(val, &key)?.to_string(),
            "status_k" => f.status_k = float_of(val, &key)?,
            "flash" => f.flash = float_of(val, &key)?,
            "ko" => f.ko = val.as_bool().ok_or_else(|| format!("{key} is not a bool"))?,
            "win" => f.win = val.as_bool().ok_or_else(|| format!("{key} is not a bool"))?,
            "t" => f.t = float_of(val, &key)?,
            _ => return Err(format!("{key}: a fighter key this test does not know")),
        }
    }
    Ok(f)
}

/// One side's ability rows: `_ability_rows` reads every key but `name` with a default.
fn rows_of(v: &Value) -> Result<Vec<AbilityRow>, String> {
    let list = v.as_array().ok_or("a side's ability rows are not a list")?;
    let mut rows = Vec::with_capacity(list.len());
    for r in list {
        let mut row = AbilityRow::new(str_of(&r["name"], "a row's name")?);
        for (k, val) in obj(r, "an ability row")? {
            match k.as_str() {
                "name" => {}
                "tag" => row.tag = str_of(val, "tag")?.to_string(),
                "state" => row.unlocked = str_of(val, "state")? != "locked",
                "glow" => row.glow = float_of(val, "glow")?,
                "sweep" => {
                    row.sweep = match val {
                        Value::Null => None,
                        x => Some(float_of(x, "sweep")?),
                    }
                }
                _ => return Err(format!("{k}: an ability row key this test does not know")),
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

fn boxes_json(boxes: &[(i32, i32, i32, i32)]) -> Value {
    Value::from(
        boxes
            .iter()
            .map(|b| serde_json::json!([b.0, b.1, b.2, b.3]))
            .collect::<Vec<_>>(),
    )
}

fn hud_case(v: &Vectors, case: &Case) -> Check {
    let args = &case.args;
    // def fight_hud(img, left, right, x0=None, x1=None, y1=None, alpha=1.0, ink=INK, bar_h=6, center='VS',
    //               abilities=None)
    let kws = keywords(
        case,
        "reelkit.ui.fight_hud(img, left, right",
        &["canvas", "left", "right"],
        &["x0", "x1", "y1", "alpha", "ink", "bar_h", "center", "abilities"],
        &["state", "frame", "why", "source"],
    )?;
    let expect = expect_of(v, case)?;
    let mut opts = HudOptions::default();
    let mut rows: Option<[Vec<AbilityRow>; 2]> = None;
    for k in kws {
        match k {
            "x0" => opts.x0 = Some(int_arg(args, "x0")?),
            "x1" => opts.x1 = Some(int_arg(args, "x1")?),
            "y1" => opts.y1 = Some(int_arg(args, "y1")?),
            "alpha" => opts.alpha = float_arg(args, "alpha")?,
            "ink" => opts.ink = rgb_of(&args["ink"], "ink")?,
            "bar_h" => opts.bar_h = int_arg(args, "bar_h")?,
            "center" => opts.center = str_of(&args["center"], "center")?.to_string(),
            _ => {
                rows = match &args["abilities"] {
                    Value::Null => None,
                    both => {
                        let both = both
                            .as_array()
                            .filter(|b| b.len() == 2)
                            .ok_or("abilities is not a pair")?;
                        Some([rows_of(&both[0])?, rows_of(&both[1])?])
                    }
                }
            }
        }
    }
    let (left, right) = (fighter_of(&args["left"], "left")?, fighter_of(&args["right"], "right")?);
    let mut img = canvas_of(args)?;
    let mut log = DrawLog::new();
    let got = hud::fight_hud(
        &mut img,
        &mut log,
        &left,
        &right,
        rows.as_ref().map(|r| [r[0].as_slice(), r[1].as_slice()]),
        &opts,
    );
    let mut returned = serde_json::json!({"top": got.top, "bars": boxes_json(&got.bars)});
    if let Some(rows) = &got.rows {
        returned["rows"] = serde_json::json!([boxes_json(&rows[0]), boxes_json(&rows[1])]);
    }
    if returned != expect["returns"] {
        return Err(format!("returned {returned}, Python {}", expect["returns"]));
    }
    same_noted(&log, expect)?;
    same_image(&img, v.array(case, "out"))
}

#[test]
fn fight_hud() {
    check_file("ui_hud", hud_case);
}

#[test]
fn hud_constants() {
    let v = vectors("ui_hud");
    let c = &v.index.raw["constants"];
    assert_eq!(c["ABIL_ROW"], hud::ABIL_ROW);
    assert_eq!(c["ABIL_HOT"], serde_json::json!(hud::ABIL_HOT));
    assert_eq!(c["ABIL_GREY"], serde_json::json!(hud::ABIL_GREY));
    assert_eq!(c["AMBER"], serde_json::json!(ui::AMBER));
    assert_eq!(c["INK"], serde_json::json!(ui::INK));
    assert_eq!(
        c.as_object().map(|o| o.len()),
        Some(5),
        "a constant this test does not check"
    );
    let opts = HudOptions::default();
    assert_eq!((opts.x0, opts.x1, opts.y1), (None, None, None));
    assert_eq!(
        (opts.alpha, opts.ink, opts.bar_h, opts.center.as_str()),
        (1.0, ui::INK, 6, "VS")
    );
    let f = Fighter::new("FLUTE", [1.0, 2.0, 3.0], 55.0);
    assert_eq!((f.hp_max, f.trail, f.level, f.max_level), (100.0, None, 1, 3));
    assert_eq!(
        (f.status.as_str(), f.status_k, f.flash, f.ko, f.win, f.t),
        ("", 0.0, 0.0, false, false, 0.0)
    );
    let row = AbilityRow::new("PICCOLO");
    assert_eq!(
        (row.tag.as_str(), row.unlocked, row.glow, row.sweep),
        ("", false, 0.0, None)
    );
}

// ---------------------------------------------------------------- the test's own guards
/// Whether the case's arguments were recorded from `kit.picture.render`.
fn is_real(case: &Case) -> bool {
    case.args["source"] == "kit.picture"
}

/// The captions of a `draw_captions` case, as JSON (none for any other case).
fn caps_json(case: &Case) -> &[Value] {
    case.args["caps"].as_array().map_or(&[], Vec::as_slice)
}

/// The cases the vectors must hold, so that a thinner export cannot pass unnoticed.
#[test]
fn the_vectors_cover_the_real_calls() {
    let count = |file: &str, pick: &dyn Fn(&Case) -> bool| vectors(file).index.cases.iter().filter(|c| pick(c)).count();
    // The HUD: the calls of the example episode (always with ability rows), the plan's states, and the form
    // without ability rows.
    let hud_real = count("ui_hud", &|c| is_real(c) && c.call.contains("abilities=abilities"));
    let hud_made = count("ui_hud", &|c| c.args["state"].is_string());
    let hud_plain = count("ui_hud", &|c| c.call == "reelkit.ui.fight_hud(img, left, right)");
    assert!(
        hud_real >= 40 && hud_made >= 18 && hud_plain >= 9,
        "{hud_real} real, {hud_made} hand-made, {hud_plain} without ability rows"
    );
    // The captions: each text of the example episode at six times of its life or more.
    for text in [
        "FLUTTER TONGUE",
        "SNAKE DANCE",
        "PICCOLO",
        "SAD TROMBONE",
        "POWER SLIDE",
        "*K.O.*",
        "WINS!",
        "COMMENT THE",
        "*NEXT MATCH-UP*",
    ] {
        let n = count("ui_captions", &|c| {
            let shown = |cap: &Value| cap["lines"].as_array().is_some_and(|l| l.iter().any(|x| x == text));
            is_real(c) && c.call.starts_with("reelkit.ui.draw_captions(") && caps_json(c).iter().any(shown)
        });
        assert!(n >= 6, "{text}: {n} real caption cases");
    }
    // A caption whose window never opens (the kit makes one for a late K.O.), alone in its call.
    let dead = count("ui_captions", &|c| {
        let never = |cap: &Value| hex_arg(&cap["t1"]) <= hex_arg(&cap["t0"]);
        c.call.starts_with("reelkit.ui.draw_captions(") && !caps_json(c).is_empty() && caps_json(c).iter().all(never)
    });
    assert!(dead >= 3, "{dead} cases of a caption whose window never opens");
    assert!(count("ui_puff", &is_real) >= 50);
    assert!(count("ui_label", &is_real) >= 8);
    assert!(count("ui_label", &|c| c.call.starts_with("reelkit.ui.panel(")) >= 20);
    // The exact number of cases per Python function over the four files, so that a dropped group of edge cases
    // cannot pass on the minimums above. An export that adds or removes cases changes these with it.
    let mut total: BTreeMap<&str, usize> = BTreeMap::new();
    for file in ["ui_label", "ui_puff", "ui_captions", "ui_hud"] {
        for case in &vectors(file).index.cases {
            *total.entry(function_of(&case.call)).or_default() += 1;
        }
    }
    let want = [
        ("_hash01", 5),
        ("_sand_puff", 110),
        ("caption_scale", 10),
        ("draw_captions", 194),
        ("fight_hud", 127),
        ("impact_landings", 6),
        ("label", 56),
        ("panel", 24),
    ];
    assert_eq!(total.into_iter().collect::<Vec<_>>(), want, "cases per function");
}

/// An unknown call, keyword or argument fails its case.
#[test]
fn unknown_calls_are_failures() {
    // A call no runner knows, on a case that holds nothing else (a `_hash01` case has no picture and no `expect`):
    // each runner must name the call, not stumble over what is missing.
    let puffv = vectors("ui_puff");
    let mut case = puffv.index.cases[0].clone();
    assert!(case.call.starts_with("reelkit.ui._hash01(") && case.args == serde_json::json!({}));
    case.call = "reelkit.ui.share_bar(img, shares, names, colors)".into();
    let runs: [fn(&Vectors, &Case) -> Check; 4] = [label_case, puff_case, caption_case, hud_case];
    for run in runs {
        let why = run(puffv, &case).unwrap_err();
        assert!(why.contains("does not know: reelkit.ui.share_bar"), "{why}");
    }
    let hudv = vectors("ui_hud");
    let mut case = hudv.index.cases[0].clone();
    case.args["left"]["mood"] = Value::from("calm");
    assert!(hud_case(hudv, &case).unwrap_err().contains("mood"));
    let mut case = hudv.index.cases[0].clone();
    case.args["blur"] = Value::from(1);
    assert!(hud_case(hudv, &case).unwrap_err().contains("blur"));
    let mut case = hudv.index.cases[0].clone();
    case.call = case
        .call
        .replace("abilities=abilities", "abilities=abilities, glow=glow");
    assert!(hud_case(hudv, &case).unwrap_err().contains("does not know"));
    let capv = vectors("ui_captions");
    let mut case = capv.index.cases[1].clone();
    case.args["cps"] = Value::from(55);
    assert!(caption_case(capv, &case).unwrap_err().contains("cps"));
    assert_eq!(function_of("reelkit.ui._sand_puff(img, t)"), "_sand_puff");
}

/// A changed picture or a changed box fails its case, with the pixel count, the largest difference and the box.
#[test]
fn a_wrong_picture_is_reported_with_its_box() {
    let v = vectors("ui_label");
    let case = &v.index.cases[0];
    let mut img = canvas_of(&case.args).expect("the canvas");
    let want = v.array(case, "out");
    assert!(same_image(&img, want).is_err(), "the label changes the picture");
    img.data.copy_from_slice(want.u8());
    assert_eq!(same_image(&img, want), Ok(()));
    img.px_mut(3, 2)[1] ^= 4;
    let why = same_image(&img, want).unwrap_err();
    assert!(
        why.contains("1 px differ") && why.contains("max 4 levels") && why.contains("box x 3..4 y 2..3"),
        "{why}"
    );
    let expect = expect_of(v, case).expect("expect");
    let mut log = DrawLog::new();
    assert!(same_noted(&log, expect).unwrap_err().contains("noted 0 boxes"));
    log.note(1, 2, 3, 4, Kind::Label, "x");
    assert!(
        same_noted(&log, expect)
            .unwrap_err()
            .contains("first difference at [0]")
    );
}
