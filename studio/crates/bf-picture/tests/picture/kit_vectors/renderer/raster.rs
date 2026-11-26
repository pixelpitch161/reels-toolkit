//! `reelkit.raster` (`picture_kit/raster.npz`): every picture, depth buffer and glow table is exact; the screen
//! points `Frame.poly` returns are a projection.

use super::*;
use bf_picture::raster;

pub(super) const CALLS: &Calls = &[
    ("Frame(bg)", 6),
    ("Frame(bg_array)", 1),
    ("fr.poly(Cam(**cam), P, col, depth)", 10),
    ("fr.line(Cam(**cam), pts, col, width)", 5),
    ("fr.screen_poly(pts, col, z)", 3),
    ("fr.disc(center, r, col, n, rot, z)", 3),
    ("fr.set_array(arr, depth); fr.array()", 3),
    ("reelkit.raster.GLOW[r]", 60),
    ("reelkit.raster.apply_glows(img, glows)", 8),
];

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

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
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
fn vectors() {
    check_calls("raster", CALLS, run);
}

#[test]
fn constants() {
    let v = Vectors::open("picture_kit/raster.npz");
    assert_eq!(v.index.raw["W"], bf_picture::config::W);
    assert_eq!(v.index.raw["H"], bf_picture::config::H);
    assert_eq!(
        hex_arg(&v.index.raw["SKY_DEPTH"]).to_bits(),
        raster::SKY_DEPTH.to_bits()
    );
}
