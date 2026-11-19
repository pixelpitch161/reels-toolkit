//! The FFT convolution under the lens blur (`picture_kit/fftconv.npz`): scipy's `rfftn`, `irfftn` and
//! `fftconvolve`, each measured against the largest value of Python's array.

use super::*;
use bf_picture::{fft, lens};

pub(super) const CALLS: &Calls = &[
    ("scipy.fft.rfftn(x, s)", 39),
    ("scipy.fft.irfftn(X, s)", 15),
    ("scipy.signal.fftconvolve(a, reelkit.lens.kernel(r), mode='same')", 8),
];

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

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
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
fn vectors() {
    check_calls("fftconv", CALLS, run);
}
