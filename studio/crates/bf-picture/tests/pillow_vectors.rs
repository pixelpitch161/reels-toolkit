//! Pillow parity for the drawing primitives: every case of the eleven drawing files under
//! `studio/reference/pillow` (written by `tools/studio_export/pillow_vectors.py` with Pillow 12.3.0) is drawn again
//! with `bf_picture::pil` and compared with what Pillow drew, byte for byte; a mode `F` image float32 bit for bit.
//!
//! One test per file. Each prints its result line (`polygon: 208/208`; run with `--nocapture` to see them when all
//! pass) and, for every case that fails, the case id, its kind, mode and size, the call, how many pixels differ and
//! the box around them. A case whose `call`, image mode or arguments the test does not know how to reproduce is a
//! failure too, never a skip.

use bf_picture::img::Img;
use bf_picture::pil;
use bf_testdata::{Case, NpArray, Vectors, diff_u8, hex_arg};
use serde_json::Value;

/// Every key a case's `args` may have. Any other key is a part of the call this test would silently drop.
const KNOWN_ARGS: [&str; 10] = [
    "kind",
    "mode",
    "size",
    "xy",
    "xy_form",
    "background",
    "fill",
    "outline",
    "width",
    "draw_mode",
];

/// The keywords a call may pass, in the order the exporter writes them.
const KEYWORDS: [&str; 3] = ["fill", "outline", "width"];

/// A `call` string taken apart: `ImageDraw.Draw(im[, 'RGBA']).<method>(xy[, k=k]...)`.
struct Call {
    blend: bool,
    method: String,
    keywords: Vec<String>,
}

fn parse_call(call: &str) -> Result<Call, String> {
    let bad = || format!("a call this test does not know: {call}");
    let rest = call.strip_prefix("ImageDraw.Draw(im").ok_or_else(bad)?;
    let (blend, rest) = if let Some(r) = rest.strip_prefix(", 'RGBA').") {
        (true, r)
    } else if let Some(r) = rest.strip_prefix(").") {
        (false, r)
    } else {
        return Err(bad());
    };
    let (method, rest) = rest.split_once("(xy").ok_or_else(bad)?;
    let rest = rest.strip_suffix(')').ok_or_else(bad)?;
    let mut keywords = Vec::new();
    for part in rest.split(", ").filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=').ok_or_else(bad)?;
        if k != v || !KEYWORDS.contains(&k) {
            return Err(bad());
        }
        keywords.push(k.to_string());
    }
    Ok(Call {
        blend,
        method: method.to_string(),
        keywords,
    })
}

/// An 8-bit ink or background: one int for a one-channel image, else a list of `n` ints, each 0 to 255.
fn u8_samples(v: &Value, n: usize, what: &str) -> Result<Vec<u8>, String> {
    let one = |x: &Value| {
        x.as_u64()
            .filter(|&u| u <= 255)
            .map(|u| u as u8)
            .ok_or_else(|| format!("{what} {v} is not made of ints from 0 to 255"))
    };
    match v {
        Value::Array(items) if items.len() == n => items.iter().map(one).collect(),
        Value::Number(_) if n == 1 => Ok(vec![one(v)?]),
        _ => Err(format!("{what} {v} does not fit an image of {n} channel(s)")),
    }
}

/// A mode `F` ink or background: a Python float (given as its hex string), stored as float32 the way Pillow's
/// `getink` does (`FLOAT32 ftmp = f`).
fn f32_sample(v: &Value, what: &str) -> Result<Vec<f32>, String> {
    match v {
        Value::String(_) => Ok(vec![hex_arg(v) as f32]),
        _ => Err(format!("{what} {v} is not a float.hex() string")),
    }
}

/// `xy` as points: a list of `[x, y]` pairs, or one flat list `x0, y0, x1, y1, ...`.
fn points(args: &Value) -> Result<Vec<(f64, f64)>, String> {
    let xy = args["xy"].as_array().ok_or("xy is not a list")?;
    match args["xy_form"].as_str() {
        Some("pairs") => xy
            .iter()
            .map(|p| match p.as_array().map(Vec::as_slice) {
                Some([x, y]) => Ok((hex_arg(x), hex_arg(y))),
                _ => Err(format!("xy has an item that is not a pair: {p}")),
            })
            .collect(),
        Some("flat") if xy.len() % 2 == 0 => Ok(xy.chunks(2).map(|p| (hex_arg(&p[0]), hex_arg(&p[1]))).collect()),
        other => Err(format!("xy_form {other:?} with {} numbers", xy.len())),
    }
}

/// The two corners of an ellipse's or rectangle's box.
fn bbox(pts: &[(f64, f64)]) -> Result<(f64, f64, f64, f64), String> {
    match pts {
        [(x0, y0), (x1, y1)] => Ok((*x0, *y0, *x1, *y1)),
        _ => Err(format!("a box needs two points, xy has {}", pts.len())),
    }
}

/// `ImageDraw.Draw(im).<method>(xy, ...)` on an 8-bit or float32 image.
fn draw_plain<T: pil::Sample>(
    img: &mut Img<T>,
    method: &str,
    pts: &[(f64, f64)],
    fill: Option<&[T]>,
    outline: Option<&[T]>,
    width: Option<u32>,
) -> Result<(), String> {
    match (method, fill, outline, width) {
        ("polygon", Some(f), None, None) => pil::polygon(img, pts, f),
        ("polygon", f, Some(o), None) => pil::polygon_outline(img, pts, f, o),
        // ImageDraw.line's width defaults to 1.
        ("line", Some(f), None, w) => pil::line(img, pts, f, w.unwrap_or(1)),
        // ImageDraw.ellipse's and rectangle's outline width defaults to 1.
        ("ellipse", f, o, None) if f.is_some() || o.is_some() => pil::ellipse(img, bbox(pts)?, f, o, 1),
        ("rectangle", f, o, None) if f.is_some() || o.is_some() => pil::rectangle(img, bbox(pts)?, f, o),
        ("point", Some(f), None, None) => pil::point(img, pts, f),
        _ => {
            return Err(format!(
                "no way to make this call: {method} with fill {}, outline {}, width {width:?}",
                fill.is_some(),
                outline.is_some()
            ));
        }
    }
    Ok(())
}

/// `ImageDraw.Draw(rgb, 'RGBA').<method>(xy, ...)`: the ink is blended into an RGB image by its alpha.
fn draw_blend(
    img: &mut Img<u8>,
    method: &str,
    pts: &[(f64, f64)],
    fill: Option<[u8; 4]>,
    outline: Option<[u8; 4]>,
    width: Option<u32>,
) -> Result<(), String> {
    match (method, fill, outline, width) {
        ("polygon", Some(f), None, None) => pil::blend::polygon(img, pts, f),
        ("polygon", f, Some(o), None) => pil::blend::polygon_outline(img, pts, f, o),
        ("line", Some(f), None, w) => pil::blend::line(img, pts, f, w.unwrap_or(1)),
        ("ellipse", f, o, None) if f.is_some() || o.is_some() => pil::blend::ellipse(img, bbox(pts)?, f, o, 1),
        ("rectangle", f, o, None) if f.is_some() || o.is_some() => pil::blend::rectangle(img, bbox(pts)?, f, o),
        ("point", Some(f), None, None) => pil::blend::point(img, pts, f),
        _ => {
            return Err(format!(
                "no way to make this blended call: {method} with fill {}, outline {}, width {width:?}",
                fill.is_some(),
                outline.is_some()
            ));
        }
    }
    Ok(())
}

/// What differs between a drawn image and Pillow's: `None` when nothing does.
fn compare_u8(got: &Img<u8>, want: &NpArray) -> Option<String> {
    let d = diff_u8(&got.data, want.u8(), got.w, got.h, got.c);
    (d.differing > 0).then(|| d.to_string())
}

/// The same for float32 planes, compared bit for bit.
fn compare_f32(got: &Img<f32>, want: &NpArray) -> Option<String> {
    let want = want.f32();
    assert_eq!(got.data.len(), want.len());
    let mut n = 0usize;
    let mut bx: Option<(usize, usize, usize, usize)> = None;
    for (i, (a, b)) in got.data.iter().zip(want).enumerate() {
        if a.to_bits() != b.to_bits() {
            let (x, y) = (i % got.w, i / got.w);
            n += 1;
            bx = Some(match bx {
                None => (x, y, x + 1, y + 1),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)),
            });
        }
    }
    bx.map(|(x0, y0, x1, y1)| format!("{n} px differ (float32 bits), box x {x0}..{x1} y {y0}..{y1}"))
}

/// The array of `role`, checked against the shape the image must have.
fn array_of<'a>(v: &'a Vectors, case: &Case, role: &str, shape: &[usize]) -> Result<&'a NpArray, String> {
    let a = v.array(case, role);
    if a.shape != shape {
        return Err(format!(
            "array {role:?} has shape {:?}, the image is {shape:?}",
            a.shape
        ));
    }
    Ok(a)
}

/// Draws one case again. `Ok(None)`: identical to Pillow. `Ok(Some(text))`: drawn, but pixels differ. `Err`: a
/// case this test cannot reproduce.
fn run_case(v: &Vectors, case: &Case) -> Result<Option<String>, String> {
    let args = &case.args;
    let obj = args.as_object().ok_or("args is not an object")?;
    if let Some(k) = obj.keys().find(|k| !KNOWN_ARGS.contains(&k.as_str())) {
        return Err(format!("an argument this test does not handle: {k:?}"));
    }
    let call = parse_call(&case.call)?;
    let passed: Vec<&str> = KEYWORDS.iter().copied().filter(|k| obj.contains_key(*k)).collect();
    if call.keywords != passed {
        return Err(format!(
            "the call names the keywords {:?} but args hold {passed:?}",
            call.keywords
        ));
    }
    match (call.blend, args.get("draw_mode").and_then(Value::as_str)) {
        (false, None) | (true, Some("RGBA")) => {}
        (_, dm) => return Err(format!("draw_mode {dm:?} does not go with the call")),
    }
    let mode = args["mode"].as_str().ok_or("mode is not a string")?;
    let (w, h) = match args["size"].as_array().map(Vec::as_slice) {
        Some([w, h]) => (
            w.as_u64().ok_or("size is not two ints")? as usize,
            h.as_u64().ok_or("size is not two ints")? as usize,
        ),
        _ => return Err("size is not [width, height]".into()),
    };
    let pts = points(args)?;
    let width = match args.get("width") {
        None => None,
        Some(wv) => Some(
            wv.as_u64()
                .and_then(|u| u32::try_from(u).ok())
                .ok_or_else(|| format!("width {wv} is not a small non-negative int"))?,
        ),
    };
    let channels = match mode {
        "L" | "F" => 1,
        "RGB" => 3,
        "RGBA" => 4,
        other => return Err(format!("an image mode this test does not handle: {other:?}")),
    };
    let shape: Vec<usize> = if channels == 1 {
        vec![h, w]
    } else {
        vec![h, w, channels]
    };

    if call.blend {
        if mode != "RGB" {
            return Err(format!("a blended draw on a {mode} image"));
        }
        let ink = |key: &str| -> Result<Option<[u8; 4]>, String> {
            match args.get(key) {
                None => Ok(None),
                Some(x) => {
                    let s = u8_samples(x, 4, key)?;
                    Ok(Some([s[0], s[1], s[2], s[3]]))
                }
            }
        };
        let before = array_of(v, case, "in", &shape)?;
        let mut img = Img::from_vec(w, h, 3, before.u8().to_vec());
        draw_blend(&mut img, &call.method, &pts, ink("fill")?, ink("outline")?, width)?;
        return Ok(compare_u8(&img, array_of(v, case, "out", &shape)?));
    }

    let background = args.get("background").ok_or("no background")?;
    if mode == "F" {
        let ink = |key: &str| args.get(key).map(|x| f32_sample(x, key)).transpose();
        let mut img = Img::filled(w, h, &f32_sample(background, "background")?);
        let (fill, outline) = (ink("fill")?, ink("outline")?);
        draw_plain(&mut img, &call.method, &pts, fill.as_deref(), outline.as_deref(), width)?;
        Ok(compare_f32(&img, array_of(v, case, "out", &shape)?))
    } else {
        let ink = |key: &str| args.get(key).map(|x| u8_samples(x, channels, key)).transpose();
        let mut img = Img::filled(w, h, &u8_samples(background, channels, "background")?);
        let (fill, outline) = (ink("fill")?, ink("outline")?);
        draw_plain(&mut img, &call.method, &pts, fill.as_deref(), outline.as_deref(), width)?;
        Ok(compare_u8(&img, array_of(v, case, "out", &shape)?))
    }
}

/// Runs every case of `pillow/<name>.npz`; panics with the list of failing cases unless all of them match.
fn check_file(name: &str) {
    let v = Vectors::open(&format!("pillow/{name}.npz"));
    let total = v.index.cases.len();
    assert!(total > 0, "{name}: no cases");
    let mut failures = Vec::new();
    for case in &v.index.cases {
        let label = format!(
            "{} ({}, {} {}) {}",
            case.id,
            case.args["kind"].as_str().unwrap_or("?"),
            case.args["mode"].as_str().unwrap_or("?"),
            case.args["size"],
            case.call
        );
        match run_case(&v, case) {
            Ok(None) => {}
            Ok(Some(diff)) => failures.push(format!("  {label}: {diff}")),
            Err(why) => failures.push(format!("  {label}: NOT REPRODUCED: {why}")),
        }
    }
    let line = format!("{name}: {}/{total}", total - failures.len());
    println!("{line}");
    assert!(
        failures.is_empty(),
        "{line}\n{} failing case(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn polygon() {
    check_file("polygon");
}

#[test]
fn polygon_outline() {
    check_file("polygon_outline");
}

#[test]
fn line() {
    check_file("line");
}

#[test]
fn ellipse() {
    check_file("ellipse");
}

#[test]
fn rectangle() {
    check_file("rectangle");
}

#[test]
fn point() {
    check_file("point");
}

#[test]
fn blend_polygon() {
    check_file("blend_polygon");
}

#[test]
fn blend_line() {
    check_file("blend_line");
}

#[test]
fn blend_ellipse() {
    check_file("blend_ellipse");
}

#[test]
fn blend_rectangle() {
    check_file("blend_rectangle");
}

#[test]
fn blend_point() {
    check_file("blend_point");
}

/// The test's own guard: a call or an argument it does not know is reported, not passed over.
#[test]
fn unknown_calls_and_arguments_are_failures() {
    assert!(parse_call("ImageDraw.Draw(im).polygon(xy, fill=fill)").is_ok());
    assert!(parse_call("ImageDraw.Draw(im, 'RGBA').line(xy, fill=fill, width=width)").is_ok());
    for call in [
        "ImageDraw.Draw(im).polygon(xy, fill=fill, joint=joint)",
        "ImageDraw.Draw(im, 'L').polygon(xy, fill=fill)",
        "Image.fromarray(a).filter(ImageFilter.GaussianBlur(radius))",
        "ImageDraw.Draw(im).polygon(xy, fill=fill",
    ] {
        assert!(parse_call(call).is_err(), "{call}");
    }
    let v = Vectors::open("pillow/polygon.npz");
    let mut case = v.index.cases[0].clone();
    case.args["joint"] = Value::from("curve");
    assert!(run_case(&v, &case).unwrap_err().contains("joint"));
    let mut case = v.index.cases[0].clone();
    case.args["mode"] = Value::from("P");
    assert!(run_case(&v, &case).unwrap_err().contains("\"P\""));
    let mut case = v.index.cases[0].clone();
    case.call = "ImageDraw.Draw(im).arc(xy, fill=fill)".into();
    assert!(run_case(&v, &case).unwrap_err().contains("arc"));
    let mut case = v.index.cases[0].clone();
    case.args["width"] = Value::from(3);
    assert!(run_case(&v, &case).unwrap_err().contains("keywords"));
}
