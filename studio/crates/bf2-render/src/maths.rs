//! The little linear algebra the renderer needs: 3-vectors and column-major 4 x 4 matrices in `f32`, laid out
//! as WGSL's `mat4x4<f32>` wants them (`m[c][r]`). Render space is right-handed with Y up; clip depth is 0..1.

pub type V3 = [f32; 3];
pub type M4 = [[f32; 4]; 4];

pub const IDENTITY: M4 = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
/// `a` scaled to unit length; the zero vector stays zero.
pub fn normalize(a: V3) -> V3 {
    let l = length(a);
    if l > 0.0 { scale(a, 1.0 / l) } else { a }
}

/// `a * b`: `b` applied first.
pub fn mul(a: &M4, b: &M4) -> M4 {
    let mut m = [[0.0; 4]; 4];
    for (c, col) in m.iter_mut().enumerate() {
        for (r, v) in col.iter_mut().enumerate() {
            *v = (0..4).map(|k| a[k][r] * b[c][k]).sum();
        }
    }
    m
}

/// `m * (p, 1)`, then the perspective divide.
pub fn transform_point(m: &M4, p: V3) -> [f32; 4] {
    let mut o = [0.0; 4];
    for (r, v) in o.iter_mut().enumerate() {
        *v = m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r];
    }
    o
}

pub fn translation(t: V3) -> M4 {
    let mut m = IDENTITY;
    m[3] = [t[0], t[1], t[2], 1.0];
    m
}

/// Translate, rotate about Y by `yaw` radians, then scale by `s` (applied first).
pub fn trs(t: V3, yaw: f32, s: V3) -> M4 {
    let (sn, cs) = yaw.sin_cos();
    [
        [cs * s[0], 0.0, -sn * s[0], 0.0],
        [0.0, s[1], 0.0, 0.0],
        [sn * s[2], 0.0, cs * s[2], 0.0],
        [t[0], t[1], t[2], 1.0],
    ]
}

/// A uniform scale about the origin, then a translation.
pub fn at_scaled(t: V3, s: f32) -> M4 {
    trs(t, 0.0, [s, s, s])
}

/// The view matrix of an eye at `eye` looking at `target`, with `up` rotated by `roll` radians about the view
/// direction.
pub fn look_at(eye: V3, target: V3, up: V3, roll: f32) -> M4 {
    let f = normalize(sub(target, eye));
    let mut s = normalize(cross(f, up));
    let mut u = cross(s, f);
    if roll != 0.0 {
        let (sn, cs) = roll.sin_cos();
        let s2 = add(scale(s, cs), scale(u, sn));
        u = sub(scale(u, cs), scale(s, sn));
        s = s2;
    }
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

/// A right-handed perspective projection to wgpu's clip space (depth 0 at `near`, 1 at `far`).
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> M4 {
    let f = 1.0 / (fov_y / 2.0).tan();
    let r = far / (near - far);
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, r, -1.0],
        [0.0, 0.0, r * near, 0.0],
    ]
}

/// The inverse transpose of `m`'s upper 3 x 3, as three padded columns: what normals are transformed by.
pub fn normal_matrix(m: &M4) -> [[f32; 4]; 3] {
    let a = |c: usize, r: usize| m[c][r];
    // Cofactors of the 3 x 3 (column c, row r); the inverse transpose is the cofactor matrix over the determinant.
    let c00 = a(1, 1) * a(2, 2) - a(2, 1) * a(1, 2);
    let c01 = a(2, 0) * a(1, 2) - a(1, 0) * a(2, 2);
    let c02 = a(1, 0) * a(2, 1) - a(2, 0) * a(1, 1);
    let c10 = a(2, 1) * a(0, 2) - a(0, 1) * a(2, 2);
    let c11 = a(0, 0) * a(2, 2) - a(2, 0) * a(0, 2);
    let c12 = a(2, 0) * a(0, 1) - a(0, 0) * a(2, 1);
    let c20 = a(0, 1) * a(1, 2) - a(1, 1) * a(0, 2);
    let c21 = a(1, 0) * a(0, 2) - a(0, 0) * a(1, 2);
    let c22 = a(0, 0) * a(1, 1) - a(1, 0) * a(0, 1);
    let det = a(0, 0) * c00 + a(1, 0) * c10 + a(2, 0) * c20;
    let k = if det.abs() > 1e-20 { 1.0 / det } else { 0.0 };
    // Row r of the inverse is the cofactors (r, *) over det; the transpose makes those the columns.
    [
        [c00 * k, c01 * k, c02 * k, 0.0],
        [c10 * k, c11 * k, c12 * k, 0.0],
        [c20 * k, c21 * k, c22 * k, 0.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_matrix_undoes_a_squash() {
        // Squashed in Y: a 45 degree slope's normal tilts toward Y.
        let m = trs([1.0, 2.0, 3.0], 0.0, [1.0, 0.5, 1.0]);
        let n = normal_matrix(&m);
        let v = [1.0f32, 1.0, 0.0];
        let t = [
            n[0][0] * v[0] + n[1][0] * v[1] + n[2][0] * v[2],
            n[0][1] * v[0] + n[1][1] * v[1] + n[2][1] * v[2],
            n[0][2] * v[0] + n[1][2] * v[1] + n[2][2] * v[2],
        ];
        assert!((t[0] - 1.0).abs() < 1e-6 && (t[1] - 2.0).abs() < 1e-6 && t[2].abs() < 1e-6);
    }

    #[test]
    fn look_at_puts_the_target_in_front() {
        let v = look_at([0.0, 5.0, 10.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.0);
        let p = transform_point(&v, [0.0, 0.0, 0.0]);
        assert!(p[0].abs() < 1e-5 && p[1].abs() < 1e-5 && p[2] < 0.0);
        let clip = transform_point(&mul(&perspective(1.0, 1.0, 0.5, 100.0), &v), [0.0, 0.0, 0.0]);
        let z = clip[2] / clip[3];
        assert!(z > 0.0 && z < 1.0, "{z}");
    }
}
