//! `bf_picture::ndimage` against scipy 1.18.1: `studio/reference/shared/ndimage.npz` (the `shared` export).
//!
//! - the kernel weights `_gaussian_kernel1d(sigma, 0, int(4 sigma + 0.5))` at twelve sigmas: bit for bit at every
//!   one (`dmath::exp` is numpy's `np.exp`: ruling M3-R38; before it sigma 2.4 was a table and sigma 6 was 2 of 49
//!   weights off);
//! - `gaussian_filter` on small pictures (one pixel, one row, one column, odd and even sizes, a kernel wider than
//!   the picture, so the reflection goes round), `float32` and `float64`, sigma 0.125 to 6: every value bit for
//!   bit;
//! - sigma 0, below `1e-15` and tiny (radius 0): a copy, bit for bit;
//! - a full frame (640 x 360, the size TRUMPET filters) at TRUMPET's three sigmas and 6, both dtypes: the SHA-256
//!   of the result.
//!
//! TRUMPET's own calls, on the masks its look drew, are checked in bf-content's `shared_helpers` (their masks are
//! made by `fighters::shared::reeds::mask_of`).

use bf_picture::img::Img;
use bf_picture::ndimage::{Sample, TRUNCATE, gaussian_filter, gaussian_kernel1d, radius};
use bf_testdata::{Vectors, hex_arg};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FILE: &str = "shared/ndimage.npz";

fn at(v: &Vectors, i: usize, role: &str) -> (usize, Vec<usize>) {
    let at = &v.index.raw["cases"][i]["at"][role];
    let a = at.as_array().unwrap();
    (
        a[0].as_u64().unwrap() as usize,
        a[1..].iter().map(|d| d.as_u64().unwrap() as usize).collect(),
    )
}

fn f64s<'a>(v: &'a Vectors, i: usize, role: &str) -> (&'a [f64], Vec<usize>) {
    let (off, shape) = at(v, i, role);
    let n: usize = shape.iter().product();
    (&v.array(&v.index.cases[i], role).f64()[off..off + n], shape)
}

fn f32s<'a>(v: &'a Vectors, i: usize, role: &str) -> (&'a [f32], Vec<usize>) {
    let (off, shape) = at(v, i, role);
    let n: usize = shape.iter().product();
    (&v.array(&v.index.cases[i], role).f32()[off..off + n], shape)
}

fn sigma(args: &Value) -> f64 {
    hex_arg(&args["sigma"])
}

trait Bits {
    fn bits(self) -> u64;
}

impl Bits for f32 {
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

impl Bits for f64 {
    fn bits(self) -> u64 {
        self.to_bits()
    }
}

/// The filtered picture against Python's: how many values differ in their bits.
fn check<T: Sample + Bits>(x: &[T], want: &[T], shape: &[usize], s: f64) -> usize {
    let img = Img::from_vec(shape[1], shape[0], 1, x.to_vec());
    let got = gaussian_filter(&img, s);
    got.data.iter().zip(want).filter(|(g, w)| g.bits() != w.bits()).count()
}

/// The sigmas whose computed kernel (`dmath::exp`) is not scipy's to the bit, measured; and the kernels by sigma.
fn kernels(v: &Vectors) -> (Vec<f64>, Vec<(f64, Vec<f64>)>) {
    let mut off = Vec::new();
    let mut all = Vec::new();
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call.starts_with("_gaussian_kernel1d") {
            let s = sigma(&c.args);
            let (want, _) = f64s(v, i, "weights");
            if gaussian_kernel1d(s, radius(s, TRUNCATE))
                .iter()
                .zip(want)
                .any(|(g, w)| g.to_bits() != w.to_bits())
            {
                off.push(s);
            }
            all.push((s, want.to_vec()));
        }
    }
    (off, all)
}

#[test]
fn the_kernels_are_scipys() {
    let v = Vectors::open(FILE);
    let (off, all) = kernels(&v);
    assert_eq!(all.len(), 12);
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call.starts_with("_gaussian_kernel1d") {
            let s = sigma(&c.args);
            assert_eq!(
                radius(s, TRUNCATE) as u64,
                c.args["radius"].as_u64().unwrap(),
                "{}: radius of {s}",
                c.id
            );
            let (want, _) = f64s(&v, i, "weights");
            let got = gaussian_kernel1d(s, radius(s, TRUNCATE));
            assert_eq!(got.len(), want.len());
            for (g, w) in got.iter().zip(want) {
                assert_eq!(g.to_bits(), w.to_bits(), "{}: sigma {s}: {g} against {w}", c.id);
            }
        }
    }
    // every kernel is computed and every one is scipy's to the bit: no sigma is a table, none is a bit off
    assert!(off.is_empty(), "sigmas whose kernel is not scipy's: {off:?}");
    println!("kernels: 12 sigmas, every weight scipy's bits");
}

#[test]
fn small_pictures_are_scipys() {
    let v = Vectors::open(FILE);
    let (mut n32, mut n64) = (0, 0);
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "scipy.ndimage.gaussian_filter(x, sigma)" {
            continue;
        }
        let s = sigma(&c.args);
        let dtype = c.args["dtype"].as_str().unwrap();
        let differ = match dtype {
            "float32" => {
                let ((x, shape), (want, _)) = (f32s(&v, i, "x"), f32s(&v, i, "out"));
                n32 += 1;
                check(x, want, &shape, s)
            }
            "float64" => {
                let ((x, shape), (want, _)) = (f64s(&v, i, "x"), f64s(&v, i, "out"));
                n64 += 1;
                check(x, want, &shape, s)
            }
            other => panic!("{}: dtype {other}", c.id),
        };
        // every kernel is scipy's (`the_kernels_are_scipys`), so every value is scipy's bits
        assert_eq!(differ, 0, "{}: sigma {s}, {dtype}: {differ} values differ", c.id);
    }
    println!("small pictures: {n32} float32 and {n64} float64 cases; all scipy's bits");
    assert!(n32 >= 30 && n64 >= 30, "{n32} {n64}");
}

fn synthetic_mask<T: Sample>(n: usize) -> Vec<T> {
    (0..n as u64)
        .map(|j| T::from_f64(((j * 104_729 + 7) % 256) as f64 / 255.0))
        .collect()
}

fn sha_of<T: Copy>(x: &[T], bytes: impl Fn(T) -> Vec<u8>) -> String {
    let b: Vec<u8> = x.iter().flat_map(|&v| bytes(v)).collect();
    Sha256::digest(&b).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn a_full_frame_is_scipys() {
    let v = Vectors::open(FILE);
    let mut n = 0;
    for c in &v.index.cases {
        if c.call != "scipy.ndimage.gaussian_filter(synthetic_mask, sigma)" {
            continue;
        }
        let s = sigma(&c.args);
        let (h, w) = (640, 360);
        assert_eq!(c.args["shape"], serde_json::json!([h, w]));
        let dtype = c.args["dtype"].as_str().unwrap();
        let got = match dtype {
            "float32" => {
                let out = gaussian_filter(&Img::from_vec(w, h, 1, synthetic_mask::<f32>(w * h)), s);
                sha_of(&out.data, |v| v.to_le_bytes().to_vec())
            }
            _ => {
                let out = gaussian_filter(&Img::from_vec(w, h, 1, synthetic_mask::<f64>(w * h)), s);
                sha_of(&out.data, |v| v.to_le_bytes().to_vec())
            }
        };
        assert_eq!(got, c.args["sha256"].as_str().unwrap(), "{}: sigma {s}, {dtype}", c.id);
        n += 1;
    }
    println!("full frames: {n}, every one scipy's bytes (float64 at sigma 6 too, since the kernel's exp is numpy's)");
    assert_eq!(n, 8);
}
