//! Three-vectors: x and y are the ring floor (the current engine's axes), z is height above the canvas.

/// A point or a vector: `[x, y, z]`.
pub type V3 = [f64; 3];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The length, by `sqrt` (IEEE-exact, so the same bits everywhere).
pub fn len(a: V3) -> f64 {
    dot(a, a).sqrt()
}

/// The unit vector along `a`; the zero vector stays zero.
pub fn norm(a: V3) -> V3 {
    let l = len(a);
    if l > 0.0 { scale(a, 1.0 / l) } else { [0.0; 3] }
}

/// `a + (b - a)·t`.
pub fn lerp(a: V3, b: V3, t: f64) -> V3 {
    add(a, scale(sub(b, a), t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_of_zero_is_zero() {
        assert_eq!(norm([0.0; 3]), [0.0; 3]);
        let n = norm([3.0, 0.0, 4.0]);
        assert!((len(n) - 1.0).abs() < 1e-15);
        assert_eq!(
            add([1.0, 2.0, 3.0], sub([1.0, 1.0, 1.0], scale([1.0, 0.0, 0.0], 2.0))),
            [0.0, 3.0, 4.0]
        );
        assert_eq!(dot([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), 32.0);
        assert_eq!(lerp([0.0; 3], [2.0, 4.0, 6.0], 0.5), [1.0, 2.0, 3.0]);
    }
}
