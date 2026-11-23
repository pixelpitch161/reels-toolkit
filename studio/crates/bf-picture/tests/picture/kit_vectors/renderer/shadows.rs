//! `reelkit.shadows` (`picture_kit/shadows.npz`): masks, samples and projected points, all exact.

use super::*;
use bf_picture::shadows;

pub(super) const CALLS: &Calls = &[
    ("reelkit.shadows.MaskGrid(ext)", 6),
    ("reelkit.shadows.MaskGrid(ext).raster(polys_xz, blur)", 12),
    ("reelkit.shadows.MaskGrid(ext).sample(mask, X, Z)", 6),
    ("reelkit.shadows.proj_point(Q, lp, max_scale)", 10),
];

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

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
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
fn vectors() {
    check_calls("shadows", CALLS, run);
}
