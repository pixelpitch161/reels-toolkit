//! `bf_picture::ndimage` against scipy 1.18.1: `studio/reference/shared/ndimage.npz` (the `shared` export).
//!
//! - the kernel weights `_gaussian_kernel1d(sigma, 0, int(4 sigma + 0.5))` at twelve sigmas: bit for bit at every
//!   sigma a kit look passes (TRUMPET: 2.4, 0.9, 0.5; 2.4 is held as the kit's constant kernel) and at all others
//!   but 6, where `dmath::exp` puts 2 of 49 weights one bit off (measured, and pinned: the test fails if the set of
//!   such sigmas changes);
//! - `gaussian_filter` on small pictures (one pixel, one row, one column, odd and even sizes, a kernel wider than
//!   the picture, so the reflection goes round), `float32` and `float64`, sigma 0.125 to 6: every value bit for
//!   bit, except the `float64` results at sigma 6, which are held to ruling R34's per-value measure (the kernel
//!   passes through `exp`; the `float32` ones are bit for bit there too);
//! - sigma 0, below `1e-15` and tiny (radius 0): a copy, bit for bit;
//! - a full frame (640 x 360, the size TRUMPET filters) at TRUMPET's three sigmas and 6, both dtypes: the SHA-256
//!   of the result.
//!
//! TRUMPET's own calls, on the masks its look drew, are checked in bf-content's `shared_helpers` (their masks are
//! made by `fighters::shared::reeds::mask_of`).

use bf_picture::img::Img;
use bf_picture::ndimage::{
    KIT_KERNELS, Sample, TRUNCATE, computed_kernel1d, gaussian_filter, gaussian_kernel1d, radius,
};
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

/// The filtered picture against Python's: how many values differ in their bits, and whether every value is within
/// ruling R34's measure (`|got - want| <= max(1e-9 |want|, 1e-12 S)`, S the largest magnitude of Python's result).
fn check<T: Sample + Bits>(x: &[T], want: &[T], shape: &[usize], s: f64) -> (usize, bool) {
    let img = Img::from_vec(shape[1], shape[0], 1, x.to_vec());
    let got = gaussian_filter(&img, s);
    let big = want.iter().fold(0.0f64, |m, w| m.max(w.to_f64().abs()));
    let differ = got.data.iter().zip(want).filter(|(g, w)| g.bits() != w.bits()).count();
    let near = got.data.iter().zip(want).all(|(g, w)| {
        let (g, w) = (g.to_f64(), w.to_f64());
        (g - w).abs() <= (1e-9 * w.abs()).max(1e-12 * big)
    });
    (differ, near)
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
            // where the computed kernel is not scipy's bits it is one bit off at most (dmath::exp against the C
            // library's)
            for (g, w) in got.iter().zip(want) {
                assert!(
                    (g.to_bits() as i64 - w.to_bits() as i64).abs() <= 1,
                    "{}: sigma {s}: {g} against {w}",
                    c.id
                );
            }
        }
    }
    // every sigma a kit look passes (TRUMPET: 2.4, 0.9, 0.5) is scipy's to the bit; of the others only 6 is not
    for s in [2.4, 0.9, 0.5] {
        assert!(!off.contains(&s), "the kit's sigma {s} is not scipy's kernel");
    }
    assert_eq!(off, [6.0], "the sigmas whose kernel is a bit off changed");
    // the pinned kernel is the vectors' and is what dmath would miss
    let (k_s, _, _) = KIT_KERNELS[0];
    let s = f64::from_bits(k_s);
    let want = &all.iter().find(|(x, _)| *x == s).unwrap().1;
    assert!(
        gaussian_kernel1d(s, 10)
            .iter()
            .zip(want)
            .all(|(g, w)| g.to_bits() == w.to_bits())
    );
    let computed = computed_kernel1d(s, 10);
    let missed = computed
        .iter()
        .zip(want)
        .filter(|(g, w)| g.to_bits() != w.to_bits())
        .count();
    println!(
        "kernels: 12 sigmas; sigma 6 computed {} weights a bit off; 2.4 held as the kit's ({missed} computed weights would be a bit off)",
        {
            let w6 = &all.iter().find(|(x, _)| *x == 6.0).unwrap().1;
            computed_kernel1d(6.0, 24)
                .iter()
                .zip(w6)
                .filter(|(g, w)| g.to_bits() != w.to_bits())
                .count()
        }
    );
    assert_eq!(missed, 4);
}

#[test]
fn small_pictures_are_scipys() {
    let v = Vectors::open(FILE);
    let (off, _) = kernels(&v);
    let (mut n32, mut n64, mut near_only) = (0, 0, 0);
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "scipy.ndimage.gaussian_filter(x, sigma)" {
            continue;
        }
        let s = sigma(&c.args);
        let dtype = c.args["dtype"].as_str().unwrap();
        let (differ, near) = match dtype {
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
        if dtype == "float64" && off.contains(&s) {
            // a kernel a bit off: float64 values within ruling R34's measure
            assert!(
                near,
                "{}: sigma {s}, float64: {differ} values differ beyond R34's measure",
                c.id
            );
            near_only += usize::from(differ > 0);
        } else {
            // the kernel is scipy's (or float32): every value scipy's bits
            assert_eq!(differ, 0, "{}: sigma {s}, {dtype}: {differ} values differ", c.id);
        }
    }
    println!(
        "small pictures: {n32} float32 and {n64} float64 cases; all scipy's bits but {near_only} float64 at sigma 6 (within R34)"
    );
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
    let (off, _) = kernels(&v);
    let (mut n, mut by_sha) = (0, 0);
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "scipy.ndimage.gaussian_filter(synthetic_mask, sigma)" {
            continue;
        }
        let s = sigma(&c.args);
        let (h, w) = (640, 360);
        assert_eq!(c.args["shape"], serde_json::json!([h, w]));
        let dtype = c.args["dtype"].as_str().unwrap();
        let (got, corner) = match dtype {
            "float32" => {
                let out = gaussian_filter(&Img::from_vec(w, h, 1, synthetic_mask::<f32>(w * h)), s);
                let corner: Vec<f64> = (0..9).map(|k| f64::from(out.data[(k / 3) * w + k % 3])).collect();
                (sha_of(&out.data, |v| v.to_le_bytes().to_vec()), corner)
            }
            _ => {
                let out = gaussian_filter(&Img::from_vec(w, h, 1, synthetic_mask::<f64>(w * h)), s);
                let corner: Vec<f64> = (0..9).map(|k| out.data[(k / 3) * w + k % 3]).collect();
                (sha_of(&out.data, |v| v.to_le_bytes().to_vec()), corner)
            }
        };
        if dtype == "float64" && off.contains(&s) {
            // the kernel is a bit off (sigma 6): the stored corner within ruling R34's measure
            let (o, _) = at(&v, i, "corner");
            let want = &v.array(c, "corner").f64()[o..o + 9];
            let big = want.iter().fold(0.0f64, |m, x| m.max(x.abs()));
            for (g, x) in corner.iter().zip(want) {
                assert!(
                    (g - x).abs() <= (1e-9 * x.abs()).max(1e-12 * big),
                    "{}: sigma {s}: {g} against {x}",
                    c.id
                );
            }
        } else {
            assert_eq!(got, c.args["sha256"].as_str().unwrap(), "{}: sigma {s}, {dtype}", c.id);
            by_sha += 1;
        }
        n += 1;
    }
    println!(
        "full frames: {n}, {by_sha} of them scipy's bytes; the other (float64 at sigma 6) is checked at 9 values only, the stored corner, within R34: enough, as no kit look passes sigma 6 (the small pictures check it whole)"
    );
    assert_eq!((n, by_sha), (8, 7));
}
