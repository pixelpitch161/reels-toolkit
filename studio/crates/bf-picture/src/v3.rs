//! Three-component vectors as numpy computes them, for the 3D modules (`camera`, `geometry`, `lighting`,
//! `shadows`).
//!
//! Each function is one numpy expression with numpy's operation order. Two of them cannot be bit-identical to the
//! reference PC: `np.dot`, `@` and `np.linalg.norm` of a 1-D array go through OpenBLAS, whose kernels are fused
//! (`fma(c, c, fma(b, b, a * a))`), and the Studio does not fuse. [`dot`] and [`norm`] are the plain sums, so
//! they can differ from Python's in the last bit; everything else here is exact.

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

/// `np.linalg.norm(a)`: the square root of [`dot`]`(a, a)`.
pub(crate) fn norm(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// `np.linalg.norm(a, axis=1)` for one row, and `np.sqrt((a * a).sum(1))`: numpy squares, adds along the row in
/// order and takes the root, with no BLAS. The same sum as [`norm`], which is why this one is exact.
pub(crate) fn norm_rows(a: V3) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
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
        assert_eq!(norm_rows([0.0, -2.0, 0.0]), 2.0);
        assert_eq!(mean(&[[1.0, 2.0, 3.0], [3.0, 4.0, 5.0]]), [2.0, 3.0, 4.0]);
        assert_eq!(sub(add([1.0, 2.0, 3.0], [1.0, 1.0, 1.0]), [2.0, 3.0, 4.0]), [0.0; 3]);
        assert_eq!(neg(scale(div([2.0, 4.0, 6.0], 2.0), 3.0)), [-3.0, -6.0, -9.0]);
    }
}
