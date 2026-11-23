//! `reelkit.post` (`picture_kit/post.npz`): all exact, except `grade` with a `gamma` other than 1, which takes a
//! power.

use super::*;
use bf_picture::post;

pub(super) const CALLS: &Calls = &[
    ("reelkit.post.bloom(img, thr, k, exclude)", 7),
    ("reelkit.post.grade(img, contrast, sat, black, gamma)", 8),
    ("reelkit.post.vignette(strength, start, cap, cy)", 6),
    ("reelkit.post.top_scrim(amount, top, span)", 5),
];

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
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
fn vectors() {
    check_calls("post", CALLS, run);
}
