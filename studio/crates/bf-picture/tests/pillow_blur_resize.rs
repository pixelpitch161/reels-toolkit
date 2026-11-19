//! Parity with Pillow 12.3.0: `Image.filter(ImageFilter.GaussianBlur(radius))` and `Image.resize(size, filter)`.
//!
//! Every case of `studio/reference/pillow/{gaussian_blur,resize_nearest,resize_bilinear,resize_bicubic}.npz` must
//! come out byte for byte. Each test prints one line for its file (`resize_bilinear: 200/200`) and, for a failing
//! case, the case id, the call, the count of differing pixels, the largest difference and the box around them. A
//! case whose `call` or image mode the test does not handle is a failure, never a skip.

use std::panic::{AssertUnwindSafe, catch_unwind};

use bf_picture::img::Img;
use bf_picture::pil::blur::gaussian_blur;
use bf_picture::pil::resize::{Resample, resize};
use bf_testdata::{Case, Vectors, diff_u8, hex_arg};

/// The one call every case of `gaussian_blur.npz` makes.
const BLUR_CALL: &str = "Image.fromarray(a).filter(ImageFilter.GaussianBlur(radius))";

/// A string argument of the case.
fn text_arg<'a>(case: &'a Case, key: &str) -> Result<&'a str, String> {
    case.args[key]
        .as_str()
        .ok_or_else(|| format!("argument {key:?} is {}, not a string", case.args[key]))
}

/// A `[width, height]` argument of the case.
fn size_arg(case: &Case, key: &str) -> Result<(usize, usize), String> {
    let side = |i: usize| case.args[key][i].as_u64().and_then(|n| usize::try_from(n).ok());
    match (case.args[key].as_array().map(Vec::len), side(0), side(1)) {
        (Some(2), Some(w), Some(h)) => Ok((w, h)),
        _ => Err(format!("argument {key:?} is {}, not [width, height]", case.args[key])),
    }
}

/// The channel count of an image mode, for the modes in `handled`.
fn channels(mode: &str, handled: &[&str]) -> Result<usize, String> {
    let c = match mode {
        "L" => 1,
        "RGB" => 3,
        "RGBA" => 4,
        _ => 0,
    };
    if c == 0 || !handled.contains(&mode) {
        return Err(format!(
            "image mode {mode:?} is not handled (only {})",
            handled.join(", ")
        ));
    }
    Ok(c)
}

/// The case's array for `role` as a `w` x `h` image of `c` channels: `(height, width)` for one channel,
/// `(height, width, channels)` otherwise, `uint8`.
fn image(v: &Vectors, case: &Case, role: &str, (w, h): (usize, usize), c: usize) -> Result<Img<u8>, String> {
    let a = v.array(case, role);
    let shape = if c == 1 { vec![h, w] } else { vec![h, w, c] };
    if a.dtype() != "|u1" || a.shape != shape {
        return Err(format!(
            "array {role:?} is {} {:?}, expected |u1 {shape:?}",
            a.dtype(),
            a.shape
        ));
    }
    Ok(Img {
        w,
        h,
        c,
        data: a.u8().to_vec(),
    })
}

/// `Ok` when `got` is `want` byte for byte, else the pixel count, the largest difference and the box.
fn compare(got: &Img<u8>, want: &Img<u8>) -> Result<(), String> {
    if (got.w, got.h, got.c) != (want.w, want.h, want.c) || got.data.len() != want.data.len() {
        return Err(format!(
            "the result is {} x {} x {} ({} bytes), expected {} x {} x {} ({} bytes)",
            got.w,
            got.h,
            got.c,
            got.data.len(),
            want.w,
            want.h,
            want.c,
            want.data.len()
        ));
    }
    let d = diff_u8(&got.data, &want.data, want.w, want.h, want.c);
    if d.differing == 0 { Ok(()) } else { Err(d.to_string()) }
}

/// Runs every case of `pillow/<name>.npz` through `run`, prints `<name>: passed/total` and panics with one line
/// per failing case. A panic inside `run` fails that case only.
fn check_file(name: &str, run: impl Fn(&Vectors, &Case) -> Result<(), String>) {
    let v = Vectors::open(&format!("pillow/{name}.npz"));
    let total = v.index.cases.len();
    assert!(total > 0, "{name}: the file has no cases");
    let mut failures = Vec::new();
    for case in &v.index.cases {
        let outcome = catch_unwind(AssertUnwindSafe(|| run(&v, case))).unwrap_or_else(|e| {
            let text = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "a panic without a message".into());
            Err(format!("panicked: {text}"))
        });
        if let Err(why) = outcome {
            failures.push(format!("  {} {} {}: {why}", case.id, case.call, case.args));
        }
    }
    println!("{name}: {}/{total}", total - failures.len());
    assert!(
        failures.is_empty(),
        "{name}: {}/{total}, {} failing case(s):\n{}",
        total - failures.len(),
        failures.len(),
        failures.join("\n")
    );
}

/// One case of `gaussian_blur.npz`.
fn blur_case(v: &Vectors, case: &Case) -> Result<(), String> {
    if case.call != BLUR_CALL {
        return Err("this call is not handled".into());
    }
    let c = channels(text_arg(case, "mode")?, &["L", "RGB"])?;
    let size = size_arg(case, "size")?;
    let radius = hex_arg(&case.args["radius"]);
    let src = image(v, case, "in", size, c)?;
    let want = image(v, case, "out", size, c)?;
    compare(&gaussian_blur(&src, radius), &want)
}

/// One case of a `resize_*.npz` file whose filter is `how` (`filter` in Pillow's spelling).
fn resize_case(v: &Vectors, case: &Case, how: Resample, filter: &str) -> Result<(), String> {
    if case.call != format!("Image.fromarray(a).resize(size_out, Image.{filter})")
        || text_arg(case, "resample")? != filter
    {
        return Err("this call is not handled".into());
    }
    let c = channels(text_arg(case, "mode")?, &["L", "RGB", "RGBA"])?;
    let size_in = size_arg(case, "size_in")?;
    let size_out = size_arg(case, "size_out")?;
    let src = image(v, case, "in", size_in, c)?;
    let want = image(v, case, "out", size_out, c)?;
    compare(&resize(&src, size_out.0, size_out.1, how), &want)
}

#[test]
fn gaussian_blur_matches_pillow() {
    check_file("gaussian_blur", blur_case);
}

#[test]
fn resize_nearest_matches_pillow() {
    check_file("resize_nearest", |v, case| {
        resize_case(v, case, Resample::Nearest, "NEAREST")
    });
}

#[test]
fn resize_bilinear_matches_pillow() {
    check_file("resize_bilinear", |v, case| {
        resize_case(v, case, Resample::Bilinear, "BILINEAR")
    });
}

#[test]
fn resize_bicubic_matches_pillow() {
    check_file("resize_bicubic", |v, case| {
        resize_case(v, case, Resample::Bicubic, "BICUBIC")
    });
}

/// The picture the hand-made cases below use: `w * h * c` bytes from a small generator, the same in Python
/// (`s = (s * 1103515245 + 12345) & 0x7fffffff` from 12345, each byte `(s >> 16) & 0xff`, filled in C order).
fn seeded(w: usize, h: usize, c: usize) -> Img<u8> {
    let mut s: u64 = 12345;
    let data = (0..w * h * c)
        .map(|_| {
            s = (s * 1_103_515_245 + 12_345) & 0x7fff_ffff;
            ((s >> 16) & 0xff) as u8
        })
        .collect();
    Img { w, h, c, data }
}

#[test]
fn a_zero_radius_blur_is_a_copy() {
    // `GaussianBlur.filter` returns `image.copy()` for a radius of 0.
    let src = seeded(7, 5, 3);
    let out = gaussian_blur(&src, 0.0);
    assert_eq!((out.w, out.h, out.c), (7, 5, 3));
    assert_eq!(out.data, src.data);
}

#[test]
fn a_resize_to_the_same_size_is_a_copy() {
    // `Image.resize` returns `self.copy()` when the size does not change, for every filter.
    for c in [1, 3, 4] {
        let src = seeded(6, 9, c);
        for how in [Resample::Nearest, Resample::Bilinear, Resample::Bicubic] {
            let out = resize(&src, 6, 9, how);
            assert_eq!((out.w, out.h, out.c), (6, 9, c), "{how:?}");
            assert_eq!(out.data, src.data, "{c} channel(s), {how:?}");
        }
    }
}

/// `Image.resize` shrinks an image more than 100 times taller than wide in two calls, the height first and then
/// the width (the other order from the ordinary path), which rounds differently. Expected bytes are
/// `np.array(Image.fromarray(a).resize((3, 10), filter))` from Pillow 12.3.0 for `a = seeded(2, 203, 1)`; for
/// BILINEAR and BICUBIC they differ from the one-call result in 1 and 10 bytes.
#[test]
fn a_very_tall_image_is_resized_height_first() {
    let src = seeded(2, 203, 1);
    let cases: [(Resample, [u8; 30]); 3] = [
        (
            Resample::Nearest,
            [
                255, 25, 25, 60, 249, 249, 244, 121, 121, 210, 252, 252, 215, 25, 25, 188, 214, 214, 104, 229, 229, 41,
                230, 230, 55, 154, 154, 203, 87, 87,
            ],
        ),
        (
            Resample::Bilinear,
            [
                125, 112, 98, 113, 126, 139, 141, 140, 139, 134, 144, 154, 143, 126, 108, 124, 136, 147, 146, 144, 142,
                121, 126, 131, 125, 125, 124, 130, 133, 136,
            ],
        ),
        (
            Resample::Bicubic,
            [
                126, 111, 95, 111, 126, 141, 141, 140, 139, 133, 145, 157, 147, 125, 102, 119, 135, 150, 149, 147, 144,
                119, 125, 131, 126, 124, 122, 128, 133, 138,
            ],
        ),
    ];
    for (how, want) in cases {
        let out = resize(&src, 3, 10, how);
        assert_eq!((out.w, out.h, out.c), (3, 10, 1), "{how:?}");
        assert_eq!(out.data, want, "{how:?}");
    }
}

/// The two-call path needs a height above 100 times the width: 2 x 200 still takes the ordinary path. Expected
/// bytes from Pillow 12.3.0 for `seeded(2, 200, 1)` and BICUBIC; the two-call path would differ in 10 of them.
#[test]
fn an_image_exactly_100_times_taller_takes_the_ordinary_path() {
    let out = resize(&seeded(2, 200, 1), 3, 10, Resample::Bicubic);
    let want: [u8; 30] = [
        124, 111, 96, 110, 126, 140, 141, 140, 138, 133, 146, 158, 147, 126, 105, 120, 133, 144, 148, 148, 147, 122,
        127, 131, 125, 124, 123, 134, 132, 130,
    ];
    assert_eq!((out.w, out.h, out.c), (3, 10, 1));
    assert_eq!(out.data, want);
}
