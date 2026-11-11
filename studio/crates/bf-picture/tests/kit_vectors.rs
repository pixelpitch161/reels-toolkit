//! The kit's own helpers against Python: `reelkit.config`, `reelkit.font` and the numpy semantics of
//! `bf_picture::np`, on the cases of `studio/reference/picture_kit` (written by
//! `tools/studio_export/kit_vectors.py`).
//!
//! Every result is compared exactly: `uint8`, `bool` and integer results value for value, floats bit for bit
//! (`-0.0` is not `0.0`, a NaN must be the same NaN). One test per file; each prints its result line
//! (`config: 12/12`) and lists the cases that fail. A case whose `call` the test does not know is a failure.
//!
//! The renderer kit's files (`camera`, `geometry`, `raster`, `lighting`, `shadows`, `lens`, `fftconv`, `post`,
//! `hull`) are checked in the module [`renderer`], which says what "equal" means for each result.

use bf_picture::img::Img;
use bf_picture::{config, font, np};
use bf_testdata::{Case, NpArray, Vectors, diff_u8, hex_arg};
use serde_json::Value;

#[path = "kit_vectors/renderer.rs"]
mod renderer;

type Check = Result<(), String>;

/// Two slices, element for element under `eq`; the error names the first difference and counts them all.
fn same<T: Copy + std::fmt::Debug>(got: &[T], want: &[T], eq: impl Fn(T, T) -> bool) -> Check {
    if got.len() != want.len() {
        return Err(format!("{} values, Python has {}", got.len(), want.len()));
    }
    let bad: Vec<usize> = (0..got.len()).filter(|&i| !eq(got[i], want[i])).collect();
    match bad.first() {
        None => Ok(()),
        Some(&i) => Err(format!(
            "{} of {} differ, first at [{i}]: got {:?}, Python {:?}",
            bad.len(),
            got.len(),
            got[i],
            want[i]
        )),
    }
}

fn same_f64(got: &[f64], want: &[f64]) -> Check {
    same(got, want, |a, b| a.to_bits() == b.to_bits())
}

fn same_f32(got: &[f32], want: &[f32]) -> Check {
    same(got, want, |a, b| a.to_bits() == b.to_bits())
}

fn same_i64(got: &[i64], want: &[i64]) -> Check {
    same(got, want, |a, b| a == b)
}

/// A mask the Rust side holds as 0 / 1 bytes against a numpy bool array of shape `(h, w)`.
fn same_mask(got: &Img<u8>, want: &NpArray) -> Check {
    if got.c != 1 || want.shape != [got.h, got.w] {
        return Err(format!(
            "a {} x {} mask of {} channel(s), Python's is {:?} (height, width)",
            got.w, got.h, got.c, want.shape
        ));
    }
    let want: Vec<u8> = want.bool().iter().map(|&b| u8::from(b)).collect();
    same(&got.data, &want, |a, b| a == b)
}

/// A numpy bool array `(h, w)` as a mask image.
fn mask_of(a: &NpArray) -> Result<Img<u8>, String> {
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

/// A numpy `uint8` array `(h, w)` or `(h, w, c)` as an image.
fn image_of(a: &NpArray) -> Result<Img<u8>, String> {
    match a.shape.as_slice() {
        [h, w] => Ok(Img::from_vec(*w, *h, 1, a.u8().to_vec())),
        [h, w, c] => Ok(Img::from_vec(*w, *h, *c, a.u8().to_vec())),
        other => Err(format!("an image of shape {other:?}")),
    }
}

fn same_image(got: &Img<u8>, want: &NpArray) -> Check {
    let shape: Vec<usize> = if want.shape.len() == 2 {
        vec![got.h, got.w]
    } else {
        vec![got.h, got.w, got.c]
    };
    if want.shape != shape {
        return Err(format!("an image of shape {shape:?}, Python's is {:?}", want.shape));
    }
    let d = diff_u8(&got.data, want.u8(), got.w, got.h, got.c);
    if d.differing == 0 { Ok(()) } else { Err(d.to_string()) }
}

fn int_arg(args: &Value, key: &str) -> Result<i64, String> {
    args[key].as_i64().ok_or_else(|| format!("args.{key} is not an int"))
}

fn float_arg(args: &Value, key: &str) -> Result<f64, String> {
    match &args[key] {
        v @ Value::String(_) => Ok(hex_arg(v)),
        other => Err(format!("args.{key} is {other}, not a float.hex() string")),
    }
}

fn rgb_arg(v: &Value, what: &str) -> Result<[u8; 3], String> {
    let bad = || format!("{what} {v} is not three ints from 0 to 255");
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(bad)?;
    let mut out = [0u8; 3];
    for (o, x) in out.iter_mut().zip(a) {
        *o = x.as_u64().filter(|&u| u <= 255).ok_or_else(bad)? as u8;
    }
    Ok(out)
}

fn i32_of(x: i64) -> i32 {
    i32::try_from(x).expect("a test input fits i32")
}

/// Runs every case of `picture_kit/<name>.npz` through `run`; panics with the failing cases unless all match.
fn check_file(name: &str, run: impl Fn(&Vectors, &Case) -> Check) {
    let v = Vectors::open(&format!("picture_kit/{name}.npz"));
    let total = v.index.cases.len();
    assert!(total > 0, "{name}: no cases");
    let failures: Vec<String> = v
        .index
        .cases
        .iter()
        .filter_map(|case| {
            run(&v, case)
                .err()
                .map(|why| format!("  {} {} {}: {why}", case.id, case.call, case.args))
        })
        .collect();
    let line = format!("{name}: {}/{total}", total - failures.len());
    println!("{line}");
    assert!(
        failures.is_empty(),
        "{line}\n{} failing case(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------- reelkit.config
fn config_case(v: &Vectors, case: &Case) -> Check {
    let f = |role: &str| v.array(case, role).f64();
    let i = |role: &str| v.array(case, role).i64();
    let table_shape = [config::H, config::W];
    match case.call.as_str() {
        "reelkit.config.BAYER" => {
            let out = v.array(case, "out");
            if out.shape != [4, 4] {
                return Err(format!("shape {:?}", out.shape));
            }
            same_f64(&config::BAYER.concat(), out.f64())
        }
        "reelkit.config.BAYER_FULL" => {
            let (out, t) = (v.array(case, "out"), config::bayer_full());
            if out.shape != table_shape || [t.h, t.w, t.c] != [config::H, config::W, 1] {
                return Err(format!("shape {:?} against {} x {} x {}", out.shape, t.w, t.h, t.c));
            }
            same_f64(&t.data, out.f64())
        }
        "reelkit.config.XX" | "reelkit.config.YY" => {
            let out = v.array(case, "out");
            let t = if case.call.ends_with("XX") {
                config::xx()
            } else {
                config::yy()
            };
            if out.shape != table_shape || [t.h, t.w, t.c] != [config::H, config::W, 1] {
                return Err(format!("shape {:?} against {} x {} x {}", out.shape, t.w, t.h, t.c));
            }
            same_f32(&t.data, out.f32())
        }
        "reelkit.config.ss(a, b, x)" => {
            let (a, b, x) = (f("a"), f("b"), f("x"));
            let got: Vec<f64> = (0..x.len()).map(|k| config::ss(a[k], b[k], x[k])).collect();
            same_f64(&got, f("out"))
        }
        "reelkit.config.ease(x)" => {
            let got: Vec<f64> = f("x").iter().map(|&x| config::ease(x)).collect();
            same_f64(&got, f("out"))
        }
        "reelkit.config.lerp(a, b, u)" => {
            let (a, b, u) = (f("a"), f("b"), f("u"));
            let got: Vec<f64> = (0..u.len()).map(|k| config::lerp(a[k], b[k], u[k])).collect();
            same_f64(&got, f("out"))
        }
        "reelkit.config.mixc(a, b, u)" => {
            let (a, b, u) = (f("a"), f("b"), f("u"));
            let rgb = |s: &[f64], k: usize| [s[3 * k], s[3 * k + 1], s[3 * k + 2]];
            let got: Vec<f64> = (0..u.len())
                .flat_map(|k| config::mixc(rgb(a, k), rgb(b, k), u[k]))
                .collect();
            same_f64(&got, f("out"))
        }
        "reelkit.config.whole_period(period, dur)" => {
            let (p, d) = (f("period"), f("dur"));
            let got: Vec<f64> = (0..p.len()).map(|k| config::whole_period(p[k], d[k])).collect();
            same_f64(&got, f("out"))
        }
        "reelkit.config.loop_bpm(bpm, dur, beats_per_bar)" => {
            let (bpm, dur, bpb) = (f("bpm"), f("dur"), i("beats_per_bar"));
            let got: Vec<f64> = (0..bpm.len())
                .map(|k| config::loop_bpm(bpm[k], dur[k], u32::try_from(bpb[k]).expect("beats per bar")))
                .collect();
            same_f64(&got, f("out"))
        }
        "reelkit.config.safe_x1(y0, y1)" => {
            let (y0, y1) = (i("y0"), i("y1"));
            let got: Vec<i64> = (0..y0.len())
                .map(|k| i64::from(config::safe_x1(i32_of(y0[k]), i32_of(y1[k]))))
                .collect();
            same_i64(&got, i("out"))
        }
        "reelkit.config.safe_overshoot(x0, y0, x1, y1)" => {
            let (x0, y0, x1, y1) = (i("x0"), i("y0"), i("x1"), i("y1"));
            let got: Vec<i64> = (0..x0.len())
                .map(|k| {
                    i64::from(config::safe_overshoot(
                        i32_of(x0[k]),
                        i32_of(y0[k]),
                        i32_of(x1[k]),
                        i32_of(y1[k]),
                    ))
                })
                .collect();
            same_i64(&got, i("out"))
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn config() {
    check_file("config", config_case);
}

#[test]
fn config_constants() {
    let v = Vectors::open("picture_kit/config.npz");
    let c = &v.index.raw["constants"];
    let four = |t: (i32, i32, i32, i32)| serde_json::json!([t.0, t.1, t.2, t.3]);
    assert_eq!(c["W"], config::W);
    assert_eq!(c["H"], config::H);
    assert_eq!(c["UP"], config::UP);
    assert_eq!(c["FPS"], config::FPS);
    assert_eq!(hex_arg(&c["DUR"]).to_bits(), config::DUR.to_bits());
    assert_eq!(c["NF"], config::NF);
    assert_eq!(c["SAFE"], four(config::SAFE));
    assert_eq!(c["SAFE_BUTTONS"], four(config::SAFE_BUTTONS));
    assert_eq!(c["SAFE_CX"], config::SAFE_CX);
    assert_eq!(
        c.as_object().map(|o| o.len()),
        Some(9),
        "a constant this test does not check"
    );
}

// ---------------------------------------------------------------- reelkit.font
/// The keywords of a `stamp` call, from its `call` text, checked against the keys of `args`.
fn stamp_keywords(case: &Case) -> Result<Vec<&str>, String> {
    let bad = || format!("a call this test does not know: {}", case.call);
    let rest = case
        .call
        .strip_prefix("reelkit.font.stamp(frame, mask, x, y, color")
        .and_then(|r| r.strip_suffix(')'))
        .ok_or_else(bad)?;
    let mut kws = Vec::new();
    for part in rest.split(", ").filter(|p| !p.is_empty()) {
        let (k, val) = part.split_once('=').ok_or_else(bad)?;
        if k != val || !["outline", "shadow", "alpha"].contains(&k) {
            return Err(bad());
        }
        kws.push(k);
    }
    let obj = case.args.as_object().ok_or("args is not an object")?;
    let passed: Vec<&str> = ["outline", "shadow", "alpha"]
        .into_iter()
        .filter(|k| obj.contains_key(*k))
        .collect();
    if kws != passed || obj.len() != 3 + passed.len() {
        return Err(format!(
            "the call names {kws:?}, args hold {:?}",
            obj.keys().collect::<Vec<_>>()
        ));
    }
    Ok(kws)
}

fn font_case(v: &Vectors, case: &Case) -> Check {
    let args = &case.args;
    match case.call.as_str() {
        "np.stack([reelkit.font.GLYPHS[ch] for ch in chars])" => {
            let chars = args["chars"].as_str().ok_or("chars is not a string")?;
            if font::glyph_chars() != chars {
                return Err(format!("the table has the glyphs {:?}", font::glyph_chars()));
            }
            let out = v.array(case, "out");
            if out.shape != [chars.chars().count(), 7, 5] {
                return Err(format!("shape {:?}", out.shape));
            }
            for (k, ch) in chars.chars().enumerate() {
                let g = font::glyph(ch).ok_or_else(|| format!("no glyph {ch:?}"))?;
                let got: Vec<bool> = g.iter().flatten().copied().collect();
                same(&got, &out.bool()[k * 35..(k + 1) * 35], |a, b| a == b)
                    .map_err(|e| format!("glyph {ch:?}: {e}"))?;
            }
            Ok(())
        }
        "reelkit.font.text_mask(s, scale)" => {
            let s = args["s"].as_str().ok_or("s is not a string")?;
            let scale = u32::try_from(int_arg(args, "scale")?).map_err(|e| e.to_string())?;
            same_mask(&font::text_mask(s, scale), v.array(case, "out"))
        }
        "reelkit.font.dilate(m, r)" => {
            let r = u32::try_from(int_arg(args, "r")?).map_err(|e| e.to_string())?;
            same_mask(&font::dilate(&mask_of(v.array(case, "m"))?, r), v.array(case, "out"))
        }
        call if call.starts_with("reelkit.font.stamp(") => {
            let kws = stamp_keywords(case)?;
            // The defaults of `def stamp(frame, mask, x, y, color, outline=(10, 20, 26), shadow=True, alpha=1.0)`.
            let outline = if kws.contains(&"outline") {
                match &args["outline"] {
                    Value::Null => None,
                    o => Some(rgb_arg(o, "outline")?),
                }
            } else {
                Some(font::OUTLINE)
            };
            let shadow = if kws.contains(&"shadow") {
                args["shadow"].as_bool().ok_or("shadow is not a bool")?
            } else {
                true
            };
            let alpha = if kws.contains(&"alpha") {
                float_arg(args, "alpha")?
            } else {
                1.0
            };
            let mut frame = image_of(v.array(case, "in"))?;
            font::stamp(
                &mut frame,
                &mask_of(v.array(case, "mask"))?,
                i32_of(int_arg(args, "x")?),
                i32_of(int_arg(args, "y")?),
                rgb_arg(&args["color"], "color")?,
                outline,
                shadow,
                alpha,
            );
            same_image(&frame, v.array(case, "out"))
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn font() {
    check_file("font", font_case);
}

#[test]
fn font_constants() {
    let v = Vectors::open("picture_kit/font.npz");
    assert_eq!(v.index.raw["ADV"], font::ADV);
    assert_eq!(v.index.raw["glyphs"], font::glyph_chars());
}

// ---------------------------------------------------------------- numpy semantics
fn np_case(v: &Vectors, case: &Case) -> Check {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let f = |role: &str| v.array(case, role).f64();
    match case.call.as_str() {
        "np.clip(x, lo, hi)" => {
            let (lo, hi) = (float_arg(args, "lo")?, float_arg(args, "hi")?);
            let got: Vec<f64> = f("x").iter().map(|&x| np::clip(x, lo, hi)).collect();
            same_f64(&got, f("out"))
        }
        "np.clip(x32, lo, hi)" => {
            // numpy converts the Python scalars to the array's float32.
            let (lo, hi) = (float_arg(args, "lo")? as f32, float_arg(args, "hi")? as f32);
            let got: Vec<f32> = arr("x32").f32().iter().map(|&x| np::clip_f32(x, lo, hi)).collect();
            same_f32(&got, arr("out").f32())
        }
        "np.clip(x, lo, None)" => {
            let lo = float_arg(args, "lo")?;
            let got: Vec<f64> = f("x").iter().map(|&x| np::maximum(x, lo)).collect();
            same_f64(&got, f("out"))
        }
        "np.clip(x, None, hi)" => {
            let hi = float_arg(args, "hi")?;
            let got: Vec<f64> = f("x").iter().map(|&x| np::minimum(x, hi)).collect();
            same_f64(&got, f("out"))
        }
        "np.maximum(a, b)" | "np.minimum(a, b)" => {
            let (a, b) = (f("a"), f("b"));
            let op = if case.call.contains("maximum") {
                np::maximum
            } else {
                np::minimum
            };
            let got: Vec<f64> = (0..a.len()).map(|k| op(a[k], b[k])).collect();
            same_f64(&got, f("out"))
        }
        "x.astype(np.uint8)" => {
            let got: Vec<u8> = f("x").iter().map(|&x| np::f64_to_u8(x)).collect();
            same(&got, arr("out").u8(), |a, b| a == b)
        }
        "x32.astype(np.uint8)" => {
            let got: Vec<u8> = arr("x32").f32().iter().map(|&x| np::f32_to_u8(x)).collect();
            same(&got, arr("out").u8(), |a, b| a == b)
        }
        "x.astype(np.int32)" => {
            let got: Vec<i32> = f("x").iter().map(|&x| np::f64_to_i32(x)).collect();
            same(&got, arr("out").i32(), |a, b| a == b)
        }
        "x.astype(np.int64)" => {
            let got: Vec<i64> = f("x").iter().map(|&x| np::f64_to_i64(x)).collect();
            same_i64(&got, arr("out").i64())
        }
        "np.clip(x, 0, 255).astype(np.uint8)" => {
            let got: Vec<u8> = f("x").iter().map(|&x| np::clip_u8(x)).collect();
            same(&got, arr("out").u8(), |a, b| a == b)
        }
        "np.clip(x32, 0, 255).astype(np.uint8)" => {
            let got: Vec<u8> = arr("x32").f32().iter().map(|&x| np::clip_u8_f32(x)).collect();
            same(&got, arr("out").u8(), |a, b| a == b)
        }
        "np.floor(v * levels + bayer * gain) / levels" => {
            let (levels, gain) = (int_arg(args, "levels")? as f64, float_arg(args, "gain")?);
            let (val, bayer) = (f("v"), f("bayer"));
            let got: Vec<f64> = (0..val.len())
                .map(|k| np::floor_dither(val[k], levels, bayer[k], gain))
                .collect();
            same_f64(&got, f("out"))
        }
        "np.floor(x / step + bayer) * step" => {
            let step = int_arg(args, "step")? as f64;
            let (x, bayer) = (f("x"), f("bayer"));
            let got: Vec<f64> = (0..x.len())
                .map(|k| np::floor_dither_step(x[k], step, bayer[k]))
                .collect();
            same_f64(&got, f("out"))
        }
        "np.floor(x / step + bayer * gain) * step" => {
            let (step, gain) = (int_arg(args, "step")? as f64, float_arg(args, "gain")?);
            let (x, bayer) = (f("x"), f("bayer"));
            let got: Vec<f64> = (0..x.len())
                .map(|k| np::floor_dither_step(x[k], step, bayer[k] * gain))
                .collect();
            same_f64(&got, f("out"))
        }
        "np.roll(a, shift, axis)" => {
            let axis = usize::try_from(int_arg(args, "axis")?).map_err(|e| e.to_string())?;
            let got = np::roll(&image_of(arr("a"))?, int_arg(args, "shift")?, axis);
            same_image(&got, arr("out"))
        }
        "np.roll(a, shift)" => {
            let got = np::roll_1d(arr("a").u8(), int_arg(args, "shift")?);
            same(&got, arr("out").u8(), |a, b| a == b)
        }
        "np.nonzero(m)" => {
            let got = np::nonzero(&mask_of(arr("m"))?);
            let ys: Vec<i64> = got.iter().map(|&(y, _)| y as i64).collect();
            let xs: Vec<i64> = got.iter().map(|&(_, x)| x as i64).collect();
            same_i64(&ys, arr("ys").i64())?;
            same_i64(&xs, arr("xs").i64())
        }
        "np.nonzero(m)[0]" => {
            let got: Vec<i64> = np::nonzero_1d(arr("m").bool()).iter().map(|&k| k as i64).collect();
            same_i64(&got, arr("out").i64())
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn numpy() {
    check_file("np", np_case);
}

/// The test's own guard: an unknown call fails its case.
#[test]
fn unknown_calls_are_failures() {
    let v = Vectors::open("picture_kit/np.npz");
    let mut case = v.index.cases[0].clone();
    case.call = "np.sqrt(x)".into();
    assert!(np_case(&v, &case).unwrap_err().contains("np.sqrt"));
    assert!(config_case(&v, &case).unwrap_err().contains("np.sqrt"));
    assert!(font_case(&v, &case).unwrap_err().contains("np.sqrt"));
    let f = Vectors::open("picture_kit/font.npz");
    let stamp = f
        .index
        .cases
        .iter()
        .find(|c| c.call.starts_with("reelkit.font.stamp("))
        .expect("a stamp case");
    let mut case = stamp.clone();
    case.args["blur"] = Value::from(1);
    assert!(font_case(&f, &case).unwrap_err().contains("blur"));
}
