//! `reelkit.lens` (`picture_kit/lens.npz`): the kernels and the circle of confusion are exact; the blur goes
//! through the FFT and is measured against the picture's largest value.

use super::*;
use bf_picture::lens;

pub(super) const CALLS: &Calls = &[
    ("reelkit.lens.oct_kernel(r)", 14),
    ("reelkit.lens.coc_map(depth, np.float64(zf), K, sharp)", 8),
    ("reelkit.lens.dof(img, coc)", 12),
];

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

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
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
            );
            exact_img_f64(&got, arr("out"))
        }
        "reelkit.lens.dof(img, coc)" => {
            let (img, coc) = (img_f32(arr("in"))?, img_f64(arr("coc"))?);
            if !args["block"].is_null() {
                // The full frame: the stored arrays blown up, and two samplings of the result, every block-th
                // pixel from the top left corner (`out[::block, ::block]`) and from the middle of the first block.
                let block = count(args, "block")?;
                let out =
                    lens::dof(&repeated(&img, block), &repeated(&coc, block)).ok_or("None: nothing out of focus")?;
                let sampled = |from: usize| -> Vec<f64> {
                    let mut sub = Vec::new();
                    for y in (from..out.h).step_by(block) {
                        for x in (from..out.w).step_by(block) {
                            sub.extend_from_slice(out.px(x, y));
                        }
                    }
                    sub
                };
                let same_size = |role: &str| match arr(role).shape == [out.h / block, out.w / block, 3] {
                    true => Ok(()),
                    false => Err(format!("{role} has shape {:?}", arr(role).shape)),
                };
                same_size("out_sub")?;
                same_size("out_mid")?;
                return worst_of(&[
                    close_array(&sampled(0), arr("out_sub").f64()),
                    close_array(&sampled(block / 2), arr("out_mid").f64()),
                ]);
            }
            match (lens::dof(&img, &coc), arr("out").dtype()) {
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
fn vectors() {
    check_calls("lens", CALLS, run);
}

#[test]
fn constants() {
    let v = Vectors::open("picture_kit/lens.npz");
    let levels: Vec<u64> = v.index.raw["LEVELS"]
        .as_array()
        .expect("LEVELS")
        .iter()
        .map(|x| hex_arg(x).to_bits())
        .collect();
    assert_eq!(levels, lens::LEVELS.map(f64::to_bits));
}
