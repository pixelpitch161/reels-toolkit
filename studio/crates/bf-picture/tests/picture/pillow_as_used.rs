//! The renderer's whole Pillow calls, at full size: every case of `studio/reference/pillow/as_used.npz` (captured
//! from `kit.picture.render` by `tools/studio_export/pillow_vectors.py`) is made again through the public `pil`
//! API and compared byte for byte, every intermediate picture included:
//!
//! - `shadow_mask`: the canvas shadow mask of `reelkit.shadows.MaskGrid.raster`: 63 to 70 polygons at 255 on a
//!   360 x 360 `L` image, then `GaussianBlur(0.7)`;
//! - `bloom`: `reelkit.post.bloom`: the 360 x 640 picture resized to 90 x 160 (BILINEAR), blurred by 3.0, resized
//!   back;
//! - `beam`: the spot beam's hull, one polygon on a 360 x 640 `L` image;
//! - `starburst`: `reelkit.fx.starburst`: polygons with fill and outline on a small `RGBA` patch;
//! - `crack`: the title card's crack: width-2 lines on a 360 x 640 `L` image.
//!
//! The test prints `as_used: 23/23` (run with `--nocapture` to see it when it passes) and lists, for a case that
//! fails, its id, kind and frame, which picture differs, how many pixels and the box around them. A case whose
//! `call`, arguments or operations it does not know is a failure, never a skip.

use std::collections::BTreeSet;

use bf_picture::img::Img;
use bf_picture::pil;
use bf_picture::pil::blur::gaussian_blur;
use bf_picture::pil::resize::{Resample, resize};
use bf_testdata::{Case, NpArray, Vectors, diff_u8, hex_arg};
use serde_json::Value;

const CRACK: &str =
    "d = ImageDraw.Draw(Image.new('L', size, 0)); each op in order: d.line(xy, fill=255, width=2)  [title.draw_crack]";
const SHADOW: &str = "d = ImageDraw.Draw(Image.new('L', size, 0)); d.polygon(xy, fill=255) for each polygon in order; \
                      then .filter(ImageFilter.GaussianBlur(blur))  [reelkit.shadows.MaskGrid.raster]";
const BLOOM: &str = "small = Image.fromarray(a).resize(small_size, Image.BILINEAR); blur = \
                     small.filter(ImageFilter.GaussianBlur(radius)); out = blur.resize(size, Image.BILINEAR)  \
                     [reelkit.post.bloom]";
const BEAM: &str =
    "ImageDraw.Draw(Image.new('L', size, 0)).polygon(xy, fill=255)  [the spot beam's hull, kit.picture.render]";
const STARBURST: &str = "d = ImageDraw.Draw(Image.new('RGBA', size, (0, 0, 0, 0))); each op in order: \
                         d.polygon(xy, fill=..., outline=...)  [reelkit.fx.starburst]";

/// The keys of a JSON object must be exactly `want`: an unknown key is a part of the call the test would drop.
fn keys_are(v: &Value, want: &[&str], what: &str) -> Result<(), String> {
    let have: BTreeSet<&str> = v
        .as_object()
        .ok_or_else(|| format!("{what} is not an object"))?
        .keys()
        .map(String::as_str)
        .collect();
    let want: BTreeSet<&str> = want.iter().copied().collect();
    if have == want {
        Ok(())
    } else {
        Err(format!("{what} has the keys {have:?}, this test knows {want:?}"))
    }
}

/// The same, where the keys in `optional` may be absent.
fn keys_within(v: &Value, required: &[&str], optional: &[&str], what: &str) -> Result<(), String> {
    let obj = v.as_object().ok_or_else(|| format!("{what} is not an object"))?;
    if let Some(k) = obj
        .keys()
        .find(|k| !required.contains(&k.as_str()) && !optional.contains(&k.as_str()))
    {
        return Err(format!("{what} has a key this test does not handle: {k:?}"));
    }
    match required.iter().find(|k| !obj.contains_key(**k)) {
        Some(k) => Err(format!("{what} has no {k:?}")),
        None => Ok(()),
    }
}

fn size_of(v: &Value, what: &str) -> Result<(usize, usize), String> {
    match v.as_array().map(Vec::as_slice) {
        Some([w, h]) => match (w.as_u64(), h.as_u64()) {
            (Some(w), Some(h)) => Ok((w as usize, h as usize)),
            _ => Err(format!("{what} is not [width, height]")),
        },
        _ => Err(format!("{what} is not [width, height]")),
    }
}

/// A list of `[x, y]` pairs of float.hex() strings.
fn points(v: &Value, what: &str) -> Result<Vec<(f64, f64)>, String> {
    v.as_array()
        .ok_or_else(|| format!("{what} is not a list"))?
        .iter()
        .map(|p| match p.as_array().map(Vec::as_slice) {
            Some([x @ Value::String(_), y @ Value::String(_)]) => Ok((hex_arg(x), hex_arg(y))),
            _ => Err(format!(
                "{what} has an item that is not a pair of float.hex() strings: {p}"
            )),
        })
        .collect()
}

/// An ink of `n` channels: one int for one channel, else a list of `n` ints, each 0 to 255.
fn ink(v: &Value, n: usize, what: &str) -> Result<Vec<u8>, String> {
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

fn expect_mode(args: &Value, want: &str) -> Result<(), String> {
    match args["mode"].as_str() {
        Some(m) if m == want => Ok(()),
        other => Err(format!("mode {other:?}, the call draws on {want}")),
    }
}

/// What differs between a picture and Pillow's array `role`: `Ok(())` when nothing does.
fn compare(v: &Vectors, case: &Case, role: &str, got: &Img<u8>) -> Result<(), String> {
    let want: &NpArray = v.array(case, role);
    let shape: Vec<usize> = if got.c == 1 {
        vec![got.h, got.w]
    } else {
        vec![got.h, got.w, got.c]
    };
    if want.shape != shape {
        return Err(format!(
            "{role:?}: the picture is {shape:?}, Pillow's array {:?}",
            want.shape
        ));
    }
    let d = diff_u8(&got.data, want.u8(), got.w, got.h, got.c);
    if d.differing == 0 {
        Ok(())
    } else {
        Err(format!("{role:?}: {d}"))
    }
}

/// A numpy `uint8` array `(h, w, 3)` as an RGB image of the size the case names.
fn rgb_image(a: &NpArray, (w, h): (usize, usize), role: &str) -> Result<Img<u8>, String> {
    if a.shape != [h, w, 3] {
        return Err(format!(
            "array {role:?} has shape {:?}, the case says {w} x {h} RGB",
            a.shape
        ));
    }
    Ok(Img::from_vec(w, h, 3, a.u8().to_vec()))
}

/// Makes one case again. `Ok(Ok(()))`: every picture equals Pillow's. `Ok(Err(text))`: made, but a picture
/// differs. `Err(text)`: a case this test cannot make.
fn run_case(v: &Vectors, case: &Case) -> Result<Result<(), String>, String> {
    let args = &case.args;
    let kind = args["kind"].as_str().ok_or("kind is not a string")?;
    let size = size_of(&args["size"], "size")?;
    let (w, h) = size;
    match (case.call.as_str(), kind) {
        (CRACK, "crack") => {
            keys_are(args, &["frame", "kind", "mode", "ops", "size"], "args")?;
            expect_mode(args, "L")?;
            let mut im: Img<u8> = Img::new(w, h, 1);
            for op in args["ops"].as_array().ok_or("ops is not a list")? {
                keys_are(op, &["fill", "method", "width", "xy"], "an op")?;
                if op["method"] != "line" {
                    return Err(format!("an op this test does not know: {}", op["method"]));
                }
                let width = op["width"].as_u64().ok_or("width is not an int")?;
                let width = u32::try_from(width).map_err(|e| e.to_string())?;
                pil::line(&mut im, &points(&op["xy"], "xy")?, &ink(&op["fill"], 1, "fill")?, width);
            }
            Ok(compare(v, case, "out", &im))
        }
        (SHADOW, "shadow_mask") => {
            keys_are(
                args,
                &["blur", "fill", "frame", "kind", "mode", "polygons", "size"],
                "args",
            )?;
            expect_mode(args, "L")?;
            let fill = ink(&args["fill"], 1, "fill")?;
            let radius = match &args["blur"] {
                r @ Value::String(_) => hex_arg(r),
                other => return Err(format!("blur {other} is not a float.hex() string")),
            };
            let mut mask: Img<u8> = Img::new(w, h, 1);
            for p in args["polygons"].as_array().ok_or("polygons is not a list")? {
                pil::polygon(&mut mask, &points(p, "a polygon")?, &fill);
            }
            if let Err(diff) = compare(v, case, "mask", &mask) {
                return Ok(Err(diff));
            }
            Ok(compare(v, case, "out", &gaussian_blur(&mask, radius)))
        }
        (BLOOM, "bloom") => {
            keys_are(args, &["frame", "kind", "mode", "radius", "size", "small_size"], "args")?;
            expect_mode(args, "RGB")?;
            let (sw, sh) = size_of(&args["small_size"], "small_size")?;
            let radius = match &args["radius"] {
                r @ Value::String(_) => hex_arg(r),
                other => return Err(format!("radius {other} is not a float.hex() string")),
            };
            let source = rgb_image(v.array(case, "in"), size, "in")?;
            let small = resize(&source, sw, sh, Resample::Bilinear);
            if let Err(diff) = compare(v, case, "small", &small) {
                return Ok(Err(diff));
            }
            let blurred = gaussian_blur(&small, radius);
            if let Err(diff) = compare(v, case, "blur", &blurred) {
                return Ok(Err(diff));
            }
            Ok(compare(v, case, "out", &resize(&blurred, w, h, Resample::Bilinear)))
        }
        (BEAM, "beam") => {
            keys_are(args, &["fill", "frame", "kind", "mode", "size", "xy"], "args")?;
            expect_mode(args, "L")?;
            let mut im: Img<u8> = Img::new(w, h, 1);
            pil::polygon(&mut im, &points(&args["xy"], "xy")?, &ink(&args["fill"], 1, "fill")?);
            Ok(compare(v, case, "out", &im))
        }
        (STARBURST, "starburst") => {
            keys_are(args, &["background", "frame", "kind", "mode", "ops", "size"], "args")?;
            expect_mode(args, "RGBA")?;
            let mut im: Img<u8> = Img::filled(w, h, &ink(&args["background"], 4, "background")?);
            for op in args["ops"].as_array().ok_or("ops is not a list")? {
                keys_within(op, &["method", "xy"], &["fill", "outline"], "an op")?;
                if op["method"] != "polygon" {
                    return Err(format!("an op this test does not know: {}", op["method"]));
                }
                let pts = points(&op["xy"], "xy")?;
                let fill = op.get("fill").map(|f| ink(f, 4, "fill")).transpose()?;
                let outline = op.get("outline").map(|o| ink(o, 4, "outline")).transpose()?;
                match (fill, outline) {
                    (Some(f), None) => pil::polygon(&mut im, &pts, &f),
                    (f, Some(o)) => pil::polygon_outline(&mut im, &pts, f.as_deref(), &o),
                    (None, None) => return Err("a polygon op with neither fill nor outline".into()),
                }
            }
            Ok(compare(v, case, "out", &im))
        }
        (call, kind) => Err(format!("a call this test does not know (kind {kind:?}): {call}")),
    }
}

#[test]
fn as_used() {
    let v = Vectors::open("pillow/as_used.npz");
    let total = v.index.cases.len();
    assert!(total > 0, "as_used: no cases");
    let mut failures = Vec::new();
    let mut kinds = BTreeSet::new();
    for case in &v.index.cases {
        let label = format!("{} ({}, frame {})", case.id, case.args["kind"], case.args["frame"]);
        match run_case(&v, case) {
            Ok(Ok(())) => {
                kinds.insert(case.args["kind"].as_str().unwrap_or("?").to_string());
            }
            Ok(Err(diff)) => failures.push(format!("  {label}: {diff}")),
            Err(why) => failures.push(format!("  {label}: NOT REPRODUCED: {why}")),
        }
    }
    let line = format!("as_used: {}/{total}", total - failures.len());
    println!("{line}");
    assert!(
        failures.is_empty(),
        "{line}\n{} failing case(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    // All five kinds of call were there to be made.
    let want: BTreeSet<String> = ["beam", "bloom", "crack", "shadow_mask", "starburst"]
        .map(String::from)
        .into();
    assert_eq!(kinds, want);
}

/// The test's own guard: an unknown call, kind, argument or operation fails its case.
#[test]
fn unknown_calls_and_arguments_are_failures() {
    let v = Vectors::open("pillow/as_used.npz");
    let of_kind = |kind: &str| {
        v.index
            .cases
            .iter()
            .find(|c| c.args["kind"] == kind)
            .unwrap_or_else(|| panic!("no {kind} case"))
            .clone()
    };
    let mut case = of_kind("beam");
    case.call = "ImageDraw.Draw(Image.new('L', size, 0)).chord(xy, 0, 90, fill=255)".into();
    assert!(run_case(&v, &case).unwrap_err().contains("chord"));
    let mut case = of_kind("beam");
    case.args["kind"] = Value::from("bloom");
    assert!(run_case(&v, &case).unwrap_err().contains("does not know"));
    let mut case = of_kind("beam");
    case.args["width"] = Value::from(3);
    assert!(run_case(&v, &case).unwrap_err().contains("width"));
    let mut case = of_kind("beam");
    case.args["mode"] = Value::from("RGB");
    assert!(run_case(&v, &case).unwrap_err().contains("RGB"));
    let mut case = of_kind("crack");
    case.args["ops"][0]["method"] = Value::from("arc");
    assert!(run_case(&v, &case).unwrap_err().contains("arc"));
    let mut case = of_kind("crack");
    case.args["ops"][0]["joint"] = Value::from("curve");
    assert!(run_case(&v, &case).unwrap_err().contains("joint"));
    let mut case = of_kind("starburst");
    case.args["ops"][0]["width"] = Value::from(2);
    assert!(run_case(&v, &case).unwrap_err().contains("width"));
    let mut case = of_kind("bloom");
    case.args["reducing_gap"] = Value::from(2);
    assert!(run_case(&v, &case).unwrap_err().contains("reducing_gap"));
    // A picture that differs is reported with its role, not passed.
    let mut case = of_kind("shadow_mask");
    case.args["polygons"].as_array_mut().unwrap().truncate(3);
    assert!(run_case(&v, &case).unwrap().unwrap_err().contains("\"mask\""));
}
