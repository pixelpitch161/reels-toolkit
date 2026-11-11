//! `reelkit.lighting` (`picture_kit/lighting.npz`): the stored directions go through a norm; `shade` snaps to
//! steps, so its colours are compared exactly.

use super::*;
use bf_picture::lighting;

pub(super) const CALLS: &Calls = &[
    (
        "Lights(amb, sky) with key(name, d, rgb), fill(d, rgb), point(pos, rgb, radius)",
        6,
    ),
    (
        "reelkit.lighting.shade(L, nrm, ctr, base, kvis, rimk, trans, emit, step)",
        32,
    ),
];

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

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
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
            // Python's kvis and trans travel together here. A case with trans and no kvis would be one where
            // Python raises; the exporter writes none.
            let trans = float_arg(args, "trans")?;
            let kvis = match &kvis {
                Some(vis) => Some(lighting::KeyVis { vis, trans }),
                None if trans == 0.0 => None,
                None => return Err(format!("trans {trans} without kvis: Python raises here")),
            };
            let got = lighting::shade(
                &l,
                &nrm,
                &ctr,
                &base,
                kvis,
                float_arg(args, "rimk")?,
                float_arg(args, "emit")?,
                num(&args["step"], "step")?,
            );
            exact(&flat3(&got), arr("out").f64())
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn vectors() {
    check_calls("lighting", CALLS, run);
}
