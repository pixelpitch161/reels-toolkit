//! The renderer kit against Python: `camera`, `geometry`, `raster`, `lighting`, `shadows`, `lens`, `post`, the FFT
//! convolution under the lens blur and the convex hull, on the cases of `studio/reference/picture_kit`.
//!
//! The bars: a `uint8` or `float32` picture, a mask, an index list and every result that is a chain of plain
//! IEEE operations is compared exactly (floats bit for bit). A `float64` result that goes through a dot product, a
//! norm, a sine or an FFT is compared to 1e-9 relative: numpy's dot products are fused on the reference PC and its
//! maths library is not `bf_core::dmath`, so the last bit can differ. "Relative" is measured against the largest
//! value of the row the number belongs to (the three coordinates of a point, the two of a screen position), and
//! for the FFT results against the largest value of the array, because a component that cancels to zero has no
//! size of its own.
//!
//! One test per file. Each prints a line per Python call (`camera: Cam(**cam).wp(P): 18/18, worst 2.3e-16`: the
//! cases that passed and the largest difference met) and fails listing the cases that do not match. A case whose
//! `call` the test does not know is a failure; a panic in the function under test fails that case only.

use super::{Check, float_arg, int_arg, same, same_f32, same_f64, same_i64, same_image};
use bf_picture::camera::{self, Cam, Key, V3};
use bf_picture::img::Img;
use bf_picture::{fft, geometry, hull, lens, lighting, post, raster, shadows};
use bf_testdata::{Case, NpArray, Vectors, hex_arg};
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// What comparing one case found: the largest difference from Python (0 for an exact result), or why it fails.
type Found = Result<f64, String>;

/// The bar for float results that are not exact by construction.
const TOL: f64 = 1e-9;

/// Runs every case of `picture_kit/<name>.npz` through `run`, prints a line per call and panics with the failing
/// cases unless all match.
fn check_calls(name: &str, run: impl Fn(&Vectors, &Case) -> Found) {
    let v = Vectors::open(&format!("picture_kit/{name}.npz"));
    let total = v.index.cases.len();
    assert!(total > 0, "{name}: no cases");
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

/// `got` against Python's `want`, both rows of `n` values: every value within [`TOL`] of Python's, measured
/// against the largest magnitude in Python's row. Returns the largest such ratio.
fn close_rows(got: &[f64], want: &[f64], n: usize) -> Found {
    if got.len() != want.len() || n == 0 || !want.len().is_multiple_of(n) {
        return Err(format!("{} values, Python has {} (rows of {n})", got.len(), want.len()));
    }
    let mut worst = 0.0f64;
    for (k, (g, w)) in got.chunks(n).zip(want.chunks(n)).enumerate() {
        let scale = w.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        for j in 0..n {
            if g[j].to_bits() == w[j].to_bits() || g[j] == w[j] {
                continue;
            }
            let ratio = (g[j] - w[j]).abs() / scale;
            // A ratio that is not a number (a NaN on one side) fails too.
            if ratio.is_nan() || ratio > TOL {
                return Err(format!(
                    "row {k}, value {j}: got {:e}, Python {:e}, off by {ratio:.1e} of the row's size {scale:e}",
                    g[j], w[j]
                ));
            }
            worst = worst.max(ratio);
        }
    }
    Ok(worst)
}

/// One number at a time: within [`TOL`] of Python's value, relative to it.
fn close(got: &[f64], want: &[f64]) -> Found {
    close_rows(got, want, 1)
}

/// A whole array at once: every value within [`TOL`] of Python's, measured against the largest magnitude in
/// Python's array (for FFT results, whose small values are rounding noise around zero).
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

// ---------------------------------------------------------------- reelkit.camera
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

/// The fields of a camera against the arrays `pos`, `M`, `target` and `scalars` of a case.
fn cam_fields(cam: &Cam, v: &Vectors, case: &Case) -> Found {
    let f = |role: &str| v.array(case, role).f64();
    let s = f("scalars");
    worst_of(&[
        close_rows(&cam.pos, f("pos"), 3),
        close_rows(&cam.m.concat(), f("M"), 3),
        exact(&cam.target, f("target")),
        close(&[cam.f], &s[..1]),
        exact(&[cam.cx, cam.cy, cam.near], &s[1..]),
    ])
}

fn camera_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let f = |role: &str| v.array(case, role).f64();
    match case.call.as_str() {
        "Cam(**cam)" => {
            let cam = cam_of(args)?;
            worst_of(&[cam_fields(&cam, v, case), close_rows(&cam.view(), f("view"), 3)])
        }
        "Cam(**cam).to_cam(P)" => close_rows(&flat3(&cam_of(args)?.to_cam(&rows3(arr("P"))?)), f("out"), 3),
        "Cam(**cam).wp(P)" => close_rows(&flat2(&cam_of(args)?.wp(&rows3(arr("P"))?)), f("out"), 2),
        "Cam(**cam).depth(P)" => close(&cam_of(args)?.depth(&rows3(arr("P"))?), f("out")),
        "Cam(**cam).proj(Pc)" => close_rows(&flat2(&cam_of(args)?.proj(&rows3(arr("Pc"))?)), f("out"), 2),
        "Cam(**cam).pixel_rays(xs, ys)" => close_rows(&flat3(&cam_of(args)?.pixel_rays(f("xs"), f("ys"))), f("out"), 3),
        "Cam(**cam).ground_hit(xs, ys, y0)" => {
            let (x, z, t) = cam_of(args)?.ground_hit(f("xs"), f("ys"), float_arg(args, "y0")?);
            let zip = |a: &[f64], b: &[f64]| -> Vec<f64> { a.iter().zip(b).flat_map(|(&p, &q)| [p, q]).collect() };
            if x.len() != z.len() || x.len() != f("X").len() {
                return Err(format!(
                    "{} and {} values, Python has {}",
                    x.len(),
                    z.len(),
                    f("X").len()
                ));
            }
            worst_of(&[close_rows(&zip(&x, &z), &zip(f("X"), f("Z")), 2), close(&t, f("t"))])
        }
        "reelkit.camera.clip_near(Pc, near)" => {
            let got = camera::clip_near(&rows3(arr("Pc"))?, float_arg(args, "near")?);
            match (got, arr("none").bool()) {
                (None, [true]) => Ok(0.0),
                (Some(pts), [false]) => exact(&flat3(&pts), f("out")),
                (got, none) => Err(format!(
                    "{}, Python's none flag is {none:?}",
                    got.map_or("None".to_string(), |p| format!("{} points", p.len()))
                )),
            }
        }
        "reelkit.camera.interp_keys(keys, t)" => {
            let (kt, target, rest) = (f("key_t"), rows3(arr("key_target"))?, f("key_rest"));
            let keys: Vec<Key> = (0..kt.len())
                .map(|k| {
                    let r = &rest[5 * k..5 * k + 5];
                    Key {
                        t: kt[k],
                        target: target[k],
                        dist: r[0],
                        yaw: r[1],
                        pitch: r[2],
                        fov: r[3],
                        sy: r[4],
                    }
                })
                .collect();
            cam_fields(&camera::interp_keys(&keys, float_arg(args, "t")?, None), v, case)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn camera() {
    check_calls("camera", camera_case);
}

#[test]
fn camera_constants() {
    let v = Vectors::open("picture_kit/camera.npz");
    assert_eq!(hex_arg(&v.index.raw["NEAR"]).to_bits(), camera::NEAR.to_bits());
}

// ---------------------------------------------------------------- reelkit.geometry
/// A list of faces against the arrays `Q`, `counts` and `n` of a case; the corners exactly where `exact_q`.
fn faces_are(faces: &[geometry::Face], v: &Vectors, case: &Case, exact_q: bool) -> Found {
    let counts: Vec<i64> = faces.iter().map(|(q, _)| q.len() as i64).collect();
    same_i64(&counts, v.array(case, "counts").i64())?;
    let q: Vec<f64> = faces.iter().flat_map(|(q, _)| flat3(q)).collect();
    let n: Vec<f64> = faces.iter().flat_map(|(_, n)| *n).collect();
    let want_q = v.array(case, "Q").f64();
    worst_of(&[
        if exact_q {
            exact(&q, want_q)
        } else {
            close_rows(&q, want_q, 3)
        },
        close_rows(&n, v.array(case, "n").f64(), 3),
    ])
}

fn geometry_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let a = |key: &str| num(&args[key], key);
    match case.call.as_str() {
        "reelkit.geometry.face(pts, center, up)" => {
            let center = match &args["center"] {
                Value::Null => None,
                c => Some(vec3(c, "center")?),
            };
            let up = args["up"].as_bool().ok_or("up is not a bool")?;
            let (q, n) = geometry::face(&rows3(arr("pts"))?, center, up);
            worst_of(&[exact(&flat3(&q), arr("Q").f64()), close_rows(&n, arr("n").f64(), 3)])
        }
        "reelkit.geometry.box(cx, cy, cz, hx, hy, hz, rot)" => {
            let faces = geometry::box_(a("cx")?, a("cy")?, a("cz")?, a("hx")?, a("hy")?, a("hz")?, a("rot")?);
            faces_are(&faces, v, case, a("rot")? == 0.0)
        }
        "reelkit.geometry.prism(cx, cz, r, y0, y1, n, r1, rot, tilt)" => {
            let faces = geometry::prism(
                a("cx")?,
                a("cz")?,
                a("r")?,
                a("y0")?,
                a("y1")?,
                count(args, "n")?,
                opt_num(&args["r1"], "r1")?,
                a("rot")?,
                pair(&args["tilt"], "tilt")?,
            );
            faces_are(&faces, v, case, false)
        }
        "reelkit.geometry.lathe(cx, cz, prof, n, rot, cy)" => {
            let prof = rows2(arr("prof"))?;
            let faces = geometry::lathe(a("cx")?, a("cz")?, &prof, count(args, "n")?, a("rot")?, a("cy")?);
            faces_are(&faces, v, case, false)
        }
        "reelkit.geometry.catenary(a, b, sag, n)" => {
            let got = geometry::catenary(
                vec3(&args["a"], "a")?,
                vec3(&args["b"], "b")?,
                a("sag")?,
                count(args, "n")?,
            );
            exact(&flat3(&got), arr("out").f64())
        }
        "reelkit.geometry.thin_quad(a, b, w)" => {
            let got = geometry::thin_quad(vec3(&args["a"], "a")?, vec3(&args["b"], "b")?, a("w")?);
            close_rows(&flat3(&got), arr("out").f64(), 3)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn geometry() {
    check_calls("geometry", geometry_case);
}

// ---------------------------------------------------------------- reelkit.raster
/// What a frame holds against the arrays `rgb` and `dep` of a case.
fn frame_is(fr: &raster::Frame, v: &Vectors, case: &Case) -> Check {
    same_image(&fr.pil, v.array(case, "rgb")).map_err(|e| format!("the picture: {e}"))?;
    let dep = fr.depth();
    same_shape(&dep, v.array(case, "dep"))?;
    same_f32(&dep.data, v.array(case, "dep").f32()).map_err(|e| format!("the depth buffer: {e}"))
}

fn new_frame(args: &Value) -> Result<raster::Frame, String> {
    Ok(raster::Frame::new(vec3(&args["bg"], "bg")?))
}

fn optional(flags: &[bool], values: &[f64], k: usize) -> Option<f64> {
    flags[k].then_some(values[k])
}

fn glows_of(a: &NpArray) -> Result<Vec<raster::Glow>, String> {
    match a.shape.as_slice() {
        [_, 7] => Ok(a
            .f64()
            .as_chunks::<7>()
            .0
            .iter()
            .map(|g| raster::Glow {
                x: g[0],
                y: g[1],
                r: g[2],
                col: [g[3], g[4], g[5]],
                inten: g[6],
            })
            .collect()),
        other => Err(format!("glows of shape {other:?}, not (n, 7)")),
    }
}

fn raster_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let f = |role: &str| v.array(case, role).f64();
    match case.call.as_str() {
        "Frame(bg)" => frame_is(&new_frame(args)?, v, case).map(|()| 0.0),
        "Frame(bg_array)" => frame_is(&raster::Frame::from_array(&img_f64(arr("bg"))?), v, case).map(|()| 0.0),
        "fr.poly(Cam(**cam), P, col, depth)" => {
            let cam = cam_of(args)?;
            let mut fr = new_frame(args)?;
            let polys = split(&rows3(arr("P"))?, arr("counts").i64())?;
            let (cols, flags) = (rows3(arr("col"))?, arr("depth").bool());
            let (mut ret, mut ret_counts) = (Vec::new(), Vec::new());
            for (k, p) in polys.iter().enumerate() {
                let pts = fr.poly(&cam, p, cols[k], flags[k]).unwrap_or_default();
                ret_counts.push(pts.len() as i64);
                ret.extend(flat2(&pts));
            }
            same_i64(&ret_counts, arr("ret_counts").i64()).map_err(|e| format!("the returned points: {e}"))?;
            let worst = close_rows(&ret, f("ret"), 2)?;
            frame_is(&fr, v, case)?;
            Ok(worst)
        }
        "fr.line(Cam(**cam), pts, col, width)" => {
            let cam = cam_of(args)?;
            let mut fr = new_frame(args)?;
            let lines = split(&rows3(arr("P"))?, arr("counts").i64())?;
            let (cols, widths) = (rows3(arr("col"))?, arr("width").i64());
            for (k, p) in lines.iter().enumerate() {
                fr.line(&cam, p, cols[k], u32::try_from(widths[k]).map_err(|e| e.to_string())?);
            }
            frame_is(&fr, v, case).map(|()| 0.0)
        }
        "fr.screen_poly(pts, col, z)" => {
            let mut fr = new_frame(args)?;
            let polys = split(&rows2(arr("pts"))?, arr("counts").i64())?;
            let (cols, has_z, z) = (rows3(arr("col"))?, arr("has_z").bool(), f("z"));
            for (k, p) in polys.iter().enumerate() {
                fr.screen_poly(p, cols[k], optional(has_z, z, k));
            }
            frame_is(&fr, v, case).map(|()| 0.0)
        }
        "fr.disc(center, r, col, n, rot, z)" => {
            let mut fr = new_frame(args)?;
            let (centers, cols) = (rows2(arr("center"))?, rows3(arr("col"))?);
            let (r, n, rot, has_z, z) = (f("r"), arr("n").i64(), f("rot"), arr("has_z").bool(), f("z"));
            for k in 0..centers.len() {
                let corners = usize::try_from(n[k]).map_err(|e| e.to_string())?;
                fr.disc(centers[k], r[k], cols[k], corners, rot[k], optional(has_z, z, k));
            }
            frame_is(&fr, v, case).map(|()| 0.0)
        }
        "fr.set_array(arr, depth); fr.array()" => {
            let mut fr = new_frame(args)?;
            let depth = match case.arrays.contains_key("depth") {
                true => Some(img_f32(arr("depth"))?),
                false => None,
            };
            match arr("arr").dtype() {
                "<f4" => fr.set_array(&img_f32(arr("arr"))?, depth.as_ref()),
                "<f8" => fr.set_array(&img_f64(arr("arr"))?, depth.as_ref()),
                other => return Err(format!("arr is {other}")),
            }
            exact_img_f32(&fr.array(), arr("array"))?;
            frame_is(&fr, v, case).map(|()| 0.0)
        }
        "reelkit.raster.GLOW[r]" => exact_img_f64(raster::glow_kernel(count(args, "r")?), arr("out")),
        "reelkit.raster.apply_glows(img, glows)" => {
            let glows = glows_of(arr("glows"))?;
            match arr("in").dtype() {
                "<f4" => {
                    let mut img = img_f32(arr("in"))?;
                    raster::apply_glows(&mut img, &glows);
                    exact_img_f32(&img, arr("out"))
                }
                "<f8" => {
                    let mut img = img_f64(arr("in"))?;
                    raster::apply_glows(&mut img, &glows);
                    exact_img_f64(&img, arr("out"))
                }
                other => Err(format!("img is {other}")),
            }
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn raster() {
    check_calls("raster", raster_case);
}

#[test]
fn raster_constants() {
    let v = Vectors::open("picture_kit/raster.npz");
    assert_eq!(v.index.raw["W"], bf_picture::config::W);
    assert_eq!(v.index.raw["H"], bf_picture::config::H);
    assert_eq!(
        hex_arg(&v.index.raw["SKY_DEPTH"]).to_bits(),
        raster::SKY_DEPTH.to_bits()
    );
}

// ---------------------------------------------------------------- reelkit.lighting
/// The lights of a case, set up through `Lights::key`, `fill` and `point` as the Python did.
fn lights_of(v: &Vectors, case: &Case) -> Result<lighting::Lights, String> {
    let arr = |role: &str| v.array(case, role);
    let mut l = lighting::Lights::new(one3(arr("amb"))?, one3(arr("sky"))?);
    l.view = one3(arr("view"))?;
    let (key_d, key_rgb) = (rows3(arr("key_d"))?, rows3(arr("key_rgb"))?);
    for (k, (d, rgb)) in key_d.iter().zip(&key_rgb).enumerate() {
        l.key(&format!("key{k}"), *d, *rgb);
    }
    let (fill_d, fill_rgb) = (rows3(arr("fill_d"))?, rows3(arr("fill_rgb"))?);
    for (d, rgb) in fill_d.iter().zip(&fill_rgb) {
        l.fill(*d, *rgb);
    }
    let (pos, rgb, radius) = (
        rows3(arr("point_pos"))?,
        rows3(arr("point_rgb"))?,
        arr("point_radius").f64(),
    );
    for k in 0..pos.len() {
        l.point(pos[k], rgb[k], radius[k]);
    }
    Ok(l)
}

fn lighting_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    match case.call.as_str() {
        "Lights(amb, sky) with key(name, d, rgb), fill(d, rgb), point(pos, rgb, radius)" => {
            let l = lights_of(v, case)?;
            let keys: Vec<f64> = l.keys.iter().flat_map(|k| k.1).collect();
            let fills: Vec<f64> = l.fills.iter().flat_map(|k| k.0).collect();
            let rgb: Vec<f64> = l.keys.iter().flat_map(|k| k.2).collect();
            let points: Vec<f64> = l.points.iter().flat_map(|p| [p.0, p.1].concat()).collect();
            let want_points: Vec<f64> = rows3(arr("point_pos"))?
                .iter()
                .zip(&rows3(arr("point_rgb"))?)
                .flat_map(|(p, c)| [*p, *c].concat())
                .collect();
            let radius: Vec<f64> = l.points.iter().map(|p| p.2).collect();
            worst_of(&[
                close_rows(&keys, arr("keys_out").f64(), 3),
                close_rows(&fills, arr("fills_out").f64(), 3),
                exact(&rgb, arr("key_rgb").f64()),
                exact(&points, &want_points),
                exact(&radius, arr("point_radius").f64()),
                exact(&[l.amb, l.sky].concat(), &[arr("amb").f64(), arr("sky").f64()].concat()),
            ])
        }
        "reelkit.lighting.shade(L, nrm, ctr, base, kvis, rimk, trans, emit, step)" => {
            let l = lights_of(v, case)?;
            let (nrm, ctr) = (rows3(arr("nrm"))?, rows3(arr("ctr"))?);
            let base = match arr("base").shape.as_slice() {
                [3] => vec![one3(arr("base"))?],
                _ => rows3(arr("base"))?,
            };
            // Shadow terms come as float32 (shadows.Occluders.vis) or float64; widening float32 is exact.
            let kvis: Option<Vec<f64>> = case.arrays.get("kvis").map(|_| match arr("kvis").dtype() {
                "<f4" => arr("kvis").f32().iter().map(|&x| f64::from(x)).collect(),
                _ => arr("kvis").f64().to_vec(),
            });
            let got = lighting::shade(
                &l,
                &nrm,
                &ctr,
                &base,
                kvis.as_deref(),
                float_arg(args, "rimk")?,
                float_arg(args, "trans")?,
                float_arg(args, "emit")?,
                num(&args["step"], "step")?,
            );
            exact(&flat3(&got), arr("out").f64())
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn lighting() {
    check_calls("lighting", lighting_case);
}

// ---------------------------------------------------------------- reelkit.shadows
fn grid_of(args: &Value) -> Result<shadows::MaskGrid, String> {
    let e = args["ext"]
        .as_array()
        .filter(|a| a.len() == 5)
        .ok_or("ext is not five numbers")?;
    let mut ext = [0.0; 5];
    for (o, x) in ext.iter_mut().zip(e) {
        *o = num(x, "ext")?;
    }
    Ok(shadows::MaskGrid::new(ext))
}

fn shadows_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    match case.call.as_str() {
        "reelkit.shadows.MaskGrid(ext)" => {
            let g = grid_of(args)?;
            same_i64(&[g.w as i64, g.h as i64], arr("out").i64()).map(|()| 0.0)
        }
        "reelkit.shadows.MaskGrid(ext).raster(polys_xz, blur)" => {
            let polys = split(&rows2(arr("pts"))?, arr("counts").i64())?;
            exact_img_f32(&grid_of(args)?.raster(&polys, float_arg(args, "blur")?), arr("out"))
        }
        "reelkit.shadows.MaskGrid(ext).sample(mask, X, Z)" => {
            let got = grid_of(args)?.sample(&img_f32(arr("mask"))?, arr("X").f64(), arr("Z").f64());
            exact_f32(&got, arr("out").f32())
        }
        "reelkit.shadows.proj_point(Q, lp, max_scale)" => {
            let got = shadows::proj_point(
                &rows3(arr("Q"))?,
                vec3(&args["lp"], "lp")?,
                float_arg(args, "max_scale")?,
            );
            exact(&flat2(&got), arr("out").f64())
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn shadows() {
    check_calls("shadows", shadows_case);
}

// ---------------------------------------------------------------- reelkit.lens
/// Every value repeated `block` x `block` times: `np.repeat(np.repeat(a, block, 0), block, 1)`.
fn repeated<T: Copy>(a: &Img<T>, block: usize) -> Img<T> {
    let mut data = Vec::with_capacity(a.data.len() * block * block);
    for y in 0..a.h * block {
        for x in 0..a.w * block {
            data.extend_from_slice(a.px(x / block, y / block));
        }
    }
    Img::from_vec(a.w * block, a.h * block, a.c, data)
}

fn lens_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    match case.call.as_str() {
        "reelkit.lens.oct_kernel(r)" => exact_img_f64(&lens::oct_kernel(float_arg(args, "r")?), arr("out")),
        "reelkit.lens.coc_map(depth, np.float64(zf), K, sharp)" => {
            let got = lens::coc_map(
                &img_f32(arr("depth"))?,
                float_arg(args, "zf")?,
                float_arg(args, "K")?,
                float_arg(args, "sharp")?,
                &lens::LEVELS,
            );
            exact_img_f64(&got, arr("out"))
        }
        "reelkit.lens.dof(img, coc)" => {
            let (img, coc) = (img_f32(arr("in"))?, img_f64(arr("coc"))?);
            if !args["block"].is_null() {
                // The full frame: the stored arrays blown up, and every block-th pixel of the result.
                let block = count(args, "block")?;
                let out = lens::dof(&repeated(&img, block), &repeated(&coc, block), &lens::LEVELS)
                    .ok_or("None: nothing out of focus")?;
                let mut sub = Vec::new();
                for y in (0..out.h).step_by(block) {
                    for x in (0..out.w).step_by(block) {
                        sub.extend_from_slice(out.px(x, y));
                    }
                }
                return close_array(&sub, arr("out_sub").f64());
            }
            match (lens::dof(&img, &coc, &lens::LEVELS), arr("out").dtype()) {
                // Nothing out of focus: Python hands the float32 picture back as it was.
                (None, "<f4") => exact_f32(&img.data, arr("out").f32()),
                (Some(out), "<f8") => {
                    same_shape(&out, arr("out"))?;
                    close_array(&out.data, arr("out").f64())
                }
                (got, dtype) => Err(format!(
                    "{}, Python's result is {dtype}",
                    if got.is_some() { "a blurred picture" } else { "None" }
                )),
            }
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn lens() {
    check_calls("lens", lens_case);
}

#[test]
fn lens_constants() {
    let v = Vectors::open("picture_kit/lens.npz");
    let levels: Vec<u64> = v.index.raw["LEVELS"]
        .as_array()
        .expect("LEVELS")
        .iter()
        .map(|x| hex_arg(x).to_bits())
        .collect();
    assert_eq!(levels, lens::LEVELS.map(f64::to_bits));
}

// ---------------------------------------------------------------- the FFT convolution
fn shape2(a: &NpArray) -> Result<(usize, usize), String> {
    match a.shape.as_slice() {
        [h, w] | [h, w, 2] => Ok((*h, *w)),
        other => Err(format!("an array of shape {other:?}")),
    }
}

fn s_arg(args: &Value) -> Result<(usize, usize), String> {
    let s = args["s"]
        .as_array()
        .filter(|a| a.len() == 2)
        .ok_or("s is not two ints")?;
    let n = |x: &Value| x.as_u64().map(|u| u as usize).ok_or("s is not two ints");
    Ok((n(&s[0])?, n(&s[1])?))
}

fn fftconv_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    match case.call.as_str() {
        "scipy.fft.rfftn(x, s)" => {
            let (x, s) = (arr("x"), s_arg(args)?);
            if arr("out").shape != [s.0, s.1 / 2 + 1, 2] {
                return Err(format!("Python's result has shape {:?}", arr("out").shape));
            }
            match x.dtype() {
                // complex64: the float32 transform is repeated operation for operation
                "<f4" => {
                    let got: Vec<f64> = fft::rfftn_f32(x.f32(), shape2(x)?, s)
                        .iter()
                        .flat_map(|c| [f64::from(c[0]), f64::from(c[1])])
                        .collect();
                    let want: Vec<f64> = arr("out").f32().iter().map(|&x| f64::from(x)).collect();
                    close_array(&got, &want)
                }
                "<f8" => close_array(&fft::rfftn_f64(x.f64(), shape2(x)?, s).concat(), arr("out").f64()),
                other => Err(format!("x is {other}")),
            }
        }
        "scipy.fft.irfftn(X, s)" => {
            let s = s_arg(args)?;
            let x: Vec<fft::Cpx<f64>> = arr("X").f64().as_chunks::<2>().0.to_vec();
            if arr("X").shape != [s.0, s.1 / 2 + 1, 2] || arr("out").shape != [s.0, s.1] {
                return Err(format!("shapes {:?} and {:?}", arr("X").shape, arr("out").shape));
            }
            close_array(&fft::irfftn_f64(&x, s), arr("out").f64())
        }
        "scipy.signal.fftconvolve(a, reelkit.lens.kernel(r), mode='same')" => {
            let kern = lens::oct_kernel(float_arg(args, "r")?);
            let got = fft::fftconvolve_same(&img_f32(arr("a"))?, &kern);
            same_shape(&got, arr("out"))?;
            close_array(&got.data, arr("out").f64())
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn fftconv() {
    check_calls("fftconv", fftconv_case);
}

// ---------------------------------------------------------------- reelkit.post
fn post_case(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let a = |key: &str| num(&args[key], key);
    match case.call.as_str() {
        "reelkit.post.bloom(img, thr, k, exclude)" => {
            let exclude = match case.arrays.contains_key("exclude") {
                true => Some(mask_u8(arr("exclude"))?),
                false => None,
            };
            let got = post::bloom(&img_f64(arr("img"))?, a("thr")?, a("k")?, exclude.as_ref());
            exact_img_f64(&got, arr("out"))
        }
        "reelkit.post.grade(img, contrast, sat, black, gamma)" => {
            let gamma = a("gamma")?;
            let got = post::grade(&img_f64(arr("img"))?, a("contrast")?, a("sat")?, a("black")?, gamma);
            same_shape(&got, arr("out"))?;
            // A power is the one step that is not plain arithmetic.
            if gamma == 1.0 {
                exact(&got.data, arr("out").f64())
            } else {
                close(&got.data, arr("out").f64())
            }
        }
        "reelkit.post.vignette(strength, start, cap, cy)" => exact_img_f64(
            &post::vignette(a("strength")?, a("start")?, a("cap")?, a("cy")?),
            arr("out"),
        ),
        "reelkit.post.top_scrim(amount, top, span)" => {
            exact_img_f64(&post::top_scrim(a("amount")?, a("top")?, a("span")?), arr("out"))
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn post() {
    check_calls("post", post_case);
}

// ---------------------------------------------------------------- scipy.spatial.ConvexHull
fn hull_case(v: &Vectors, case: &Case) -> Found {
    match case.call.as_str() {
        "scipy.spatial.ConvexHull(pts).vertices" => {
            let got: Vec<i32> = hull::convex_hull(&rows2(v.array(case, "pts"))?)
                .iter()
                .map(|&k| k as i32)
                .collect();
            same(&got, v.array(case, "out").i32(), |a, b| a == b).map(|()| 0.0)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn hull() {
    check_calls("hull", hull_case);
}

/// The test's own guard: an unknown call fails its case, in every group.
#[test]
fn unknown_calls_are_failures() {
    type Run = fn(&Vectors, &Case) -> Found;
    let groups: [(&str, Run); 9] = [
        ("camera", camera_case),
        ("geometry", geometry_case),
        ("raster", raster_case),
        ("lighting", lighting_case),
        ("shadows", shadows_case),
        ("lens", lens_case),
        ("fftconv", fftconv_case),
        ("post", post_case),
        ("hull", hull_case),
    ];
    for (name, run) in groups {
        let v = Vectors::open(&format!("picture_kit/{name}.npz"));
        let mut case = v.index.cases[0].clone();
        case.call = "np.sqrt(x)".into();
        assert!(run(&v, &case).unwrap_err().contains("np.sqrt"), "{name}");
    }
}

/// The comparison's own guard: what counts as close, and what does not.
#[test]
fn closeness_is_measured_against_the_row() {
    assert_eq!(close_rows(&[1.0, 2.0], &[1.0, 2.0], 2), Ok(0.0));
    assert_eq!(close(&[-0.0], &[0.0]), Ok(0.0));
    assert!(close(&[1.0 + 1e-12], &[1.0]).unwrap() < 2e-12);
    assert!(close(&[1.0 + 3e-9], &[1.0]).is_err());
    // a component that cancelled to nothing next to a large one: small against the point, not against itself
    assert!(close_rows(&[3e-16, 10.0], &[-1e-16, 10.0], 2).unwrap() < 1e-15);
    assert!(close(&[3e-16], &[-1e-16]).is_err());
    assert!(close_rows(&[1e-7, 10.0], &[0.0, 10.0], 2).is_err());
    assert!(close(&[f64::NAN], &[1.0]).is_err());
    assert!(close(&[1.0], &[f64::NAN]).is_err());
    assert_eq!(close(&[f64::NAN], &[f64::NAN]), Ok(0.0));
    assert!(close(&[1.0, 2.0], &[1.0]).is_err());
    assert!(close_array(&[1e-12, 255.0], &[0.0, 255.0]).unwrap() < 1e-14);
    assert!(close_array(&[1e-6, 255.0], &[0.0, 255.0]).is_err());
    assert!(exact(&[0.0], &[-0.0]).is_err());
}
