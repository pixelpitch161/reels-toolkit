//! Three-component vectors as numpy computes them, for the 3D modules (`camera`, `geometry`, `lighting`,
//! `shadows`).
//!
//! Each function is one numpy expression with numpy's operation order. `np.dot`, `@` and `np.linalg.norm` of a 1-D
//! array go through OpenBLAS, whose kernels are fused (`fma(c, c, fma(b, b, a * a))`): [`dot_blas`],
//! [`dot_blas_mv`] and [`norm_blas`] are those kernels to the bit. [`dot`] and [`norm`] are the plain sums, so where
//! they stand for a BLAS product they can differ from Python's in the last bit; everything else here is exact.

use bf_core::dmath;

/// A point or a vector: `[x, y, z]`.
pub type V3 = [f64; 3];

/// `a + b`.
pub(crate) fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a - b`.
pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `a * k` for a scalar `k`.
pub(crate) fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// `a / k` for a scalar `k`.
pub(crate) fn div(a: V3, k: f64) -> V3 {
    [a[0] / k, a[1] / k, a[2] / k]
}

/// `-a`.
pub(crate) fn neg(a: V3) -> V3 {
    [-a[0], -a[1], -a[2]]
}

/// `np.dot(a, b)` (also `a @ b`), as the plain sum `a0 b0 + a1 b1 + a2 b2`. numpy's is fused: see the module
/// documentation.
pub(crate) fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// One element of a numpy matrix product with three terms as OpenBLAS's matrix-matrix kernel computes it on the
/// reference PC: the first product rounded, the other two added with one rounding each, `fma(a2, b2, fma(a1, b1,
/// a0 * b0))`. numpy takes this kernel for `X @ M.T` with two rows or more, for `X @ M` with any number of rows (one
/// included), and for `np.dot(a, b)` / `np.linalg.norm(a)` of 3-vectors (measured there on random values: every
/// element equal, ruling M3-R30).
pub(crate) fn dot_blas(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[1], b[1], a[0] * b[0]))
}

/// One element of a numpy matrix-vector product as OpenBLAS's matrix-vector kernel computes it on the reference PC:
/// the second product is the rounded one, `fma(a2, b2, fma(a0, b0, a1 * b1))`. numpy takes this kernel for `M @ v`
/// and for `x @ M.T` when `x` is one row (`(3,)` or `(1, 3)`, measured as for [`dot_blas`]).
pub(crate) fn dot_blas_mv(a: V3, b: V3) -> f64 {
    dmath::fma(a[2], b[2], dmath::fma(a[0], b[0], a[1] * b[1]))
}

/// `np.linalg.norm(a)` of one 3-vector: the square root of the fused [`dot_blas`]`(a, a)` (equal on 200,000 of
/// 200,000 random vectors on the reference PC; the plain [`norm`] on about 89 %).
pub(crate) fn norm_blas(a: V3) -> f64 {
    dot_blas(a, a).sqrt()
}

/// The length of `a`: the square root of [`dot`]`(a, a)`, a plain sum. It stands for two numpy expressions:
///
/// - `np.linalg.norm(a)` of one vector, which goes through BLAS and is fused there: this can differ from it in
///   the last bit;
/// - `np.linalg.norm(a, axis=1)` for one row of an array, and `np.sqrt((a * a).sum(1))`: numpy's own loop
///   (square, add along the row in order, root), with no BLAS. For these it is exact.
pub(crate) fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// `np.cross(a, b)`: numpy multiplies and subtracts in separate steps, `(a1 b2 - a2 b1, a2 b0 - a0 b2,
/// a0 b1 - a1 b0)`.
pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `Q.mean(0)` for the rows of a `(k, 3)` array: the rows added in order, then divided by their number.
pub(crate) fn mean(rows: &[V3]) -> V3 {
    let mut s = [0.0; 3];
    for r in rows {
        s = add(s, *r);
    }
    div(s, rows.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cross_product_is_right_handed() {
        assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(cross([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]);
        assert_eq!(cross([2.0, 3.0, 4.0], [5.0, 6.0, 7.0]), [-3.0, 6.0, -3.0]);
    }

    #[test]
    fn sums_and_norms() {
        assert_eq!(dot([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), 32.0);
        assert_eq!(norm([3.0, 0.0, 4.0]), 5.0);
        assert_eq!(norm([0.0, -2.0, 0.0]), 2.0);
        assert_eq!(mean(&[[1.0, 2.0, 3.0], [3.0, 4.0, 5.0]]), [2.0, 3.0, 4.0]);
        assert_eq!(sub(add([1.0, 2.0, 3.0], [1.0, 1.0, 1.0]), [2.0, 3.0, 4.0]), [0.0; 3]);
        assert_eq!(neg(scale(div([2.0, 4.0, 6.0], 2.0), 3.0)), [-3.0, -6.0, -9.0]);
    }
}
