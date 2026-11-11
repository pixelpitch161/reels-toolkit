//! `reelkit/lighting.py`: the lights of a scene and flat shading.
//!
//! [`Lights`] holds an ambient colour, extra light for faces that look up (`sky`), key lights (direction and
//! colour; the ones shadows apply to), fills (direction and colour, never shadowed), point lights (position,
//! colour, radius) and the `view` vector (unit, from the scene toward the camera, set from `cam.view()` each
//! frame; it drives the rim term). [`shade`] turns unit normals, face centres and base colours into the colours
//! to draw, snapped to steps of 5.
//!
//! How close to Python (`studio/reference/picture_kit/lighting.npz`): `shade` gives the same colours, to the
//! bit, on every reference case, though it is not exact by construction: its dot products are fused in numpy and
//! plain here, and its highlight roll-off takes an exponential (numpy's against `bf_core::dmath`), so the light
//! before the snap can differ in its last bit, which the snap to a step of 5 absorbs unless a channel sits within
//! that bit of the middle between two steps.
//!
//! Not ported (the BALLFIGHT renderer does not call them): `fog`, `sky_gradient`.

use crate::np;
use crate::v3::{self, V3};
use bf_core::{dmath, py};

/// `Lights`.
#[derive(Clone, Debug, PartialEq)]
pub struct Lights {
    /// Ambient light, an RGB multiplier.
    pub amb: V3,
    /// Extra light on faces that look up.
    pub sky: V3,
    /// Key lights: `(name, unit direction toward the light, rgb)`.
    pub keys: Vec<(String, V3, V3)>,
    /// Fill lights: `(unit direction toward the light, rgb)`.
    pub fills: Vec<(V3, V3)>,
    /// Point lights: `(position, rgb, radius)`; the falloff is `1 / (1 + (d / radius)^2)`.
    pub points: Vec<(V3, V3, f64)>,
    /// The unit vector from the scene toward the camera. Python's default is `(0, 0, 1)`.
    pub view: V3,
}

impl Lights {
    /// `Lights(amb, sky)` with no lights yet. The Python defaults are `(0.3, 0.3, 0.4)` and `(0.1, 0.1, 0.15)`.
    pub fn new(amb: V3, sky: V3) -> Lights {
        Lights {
            amb,
            sky,
            keys: Vec::new(),
            fills: Vec::new(),
            points: Vec::new(),
            view: [0.0, 0.0, 1.0],
        }
    }

    /// `key(name, d, rgb)`: a key light from direction `d` (any length; stored as a unit vector).
    pub fn key(&mut self, name: &str, d: V3, rgb: V3) {
        self.keys.push((name.to_string(), v3::div(d, v3::norm(d)), rgb));
    }

    /// `fill(d, rgb)`: a fill light from direction `d` (any length; stored as a unit vector).
    pub fn fill(&mut self, d: V3, rgb: V3) {
        self.fills.push((v3::div(d, v3::norm(d)), rgb));
    }

    /// `point(pos, rgb, radius)`: a point light.
    pub fn point(&mut self, pos: V3, rgb: V3, radius: f64) {
        self.points.push((pos, rgb, radius));
    }
}

/// `np.clip(x, 0, None)`.
fn positive(x: f64) -> f64 {
    np::maximum(x, 0.0)
}

/// The shadow terms of [`shade`]: Python's `kvis` and `trans` as one value, because `trans` reads `kvis` (with
/// `trans` and no `kvis` the Python raises a `TypeError`; here that cannot be written).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyVis<'a> {
    /// `kvis`: per face and key light, 1 where the light reaches the face and 0 in shadow, row-major
    /// (`vis[face * keys + key]`).
    pub vis: &'a [f64],
    /// `trans`: light shining through thin faces toward the camera. 0.0, Python's default, is none.
    pub trans: f64,
}

/// `shade(L, nrm, ctr, base, kvis=None, rimk=0.55, trans=0.0, emit=0.0, step=5)`: the colours of `nrm.len()`
/// faces.
///
/// - `nrm`: unit normals; `ctr`: face centres (used by the point lights).
/// - `base`: the albedo, 0 to 255: one colour for all faces, or one per face.
/// - `kvis`: the shadow terms of the key lights and, with them, Python's `trans` ([`KeyVis`]); `None` is
///   `kvis=None, trans=0.0`: every key light reaches every face.
/// - `rimk`: the strength of the silhouette rim each key adds; `emit`: self-illumination; `step`: the colours
///   are rounded to multiples of it (ties to the even multiple), then held to 0..=255.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn shade(
    l: &Lights,
    nrm: &[V3],
    ctr: &[V3],
    base: &[V3],
    kvis: Option<KeyVis<'_>>,
    rimk: f64,
    emit: f64,
    step: f64,
) -> Vec<V3> {
    let n = nrm.len();
    let nk = l.keys.len();
    assert_eq!(ctr.len(), n, "shade: {n} normals and {} centres", ctr.len());
    assert!(
        base.len() == 1 || base.len() == n,
        "shade: {} base colours for {n} faces",
        base.len()
    );
    if let Some(k) = kvis {
        assert_eq!(k.vis.len(), n * nk, "shade: kvis must hold {n} x {nk} values");
    }
    let trans = kvis.map_or(0.0, |k| k.trans);
    let vw = l.view;
    // Per key light: the direction of its rim (the part of d across the view), and how much it faces the camera
    // from behind.
    let rims: Vec<(V3, f64)> = l
        .keys
        .iter()
        .map(|(_, d, _)| {
            let kp = v3::sub(*d, v3::scale(vw, v3::dot(*d, vw)));
            let kp = v3::div(kp, v3::norm(kp) + 1e-9);
            // max(0.0, -float(vw @ d))
            (kp, py::py_max(0.0, -v3::dot(vw, *d)))
        })
        .collect();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (nr, c) = (nrm[i], ctr[i]);
        let up = positive(nr[1]);
        let mut light: V3 = std::array::from_fn(|ch| l.amb[ch] + up * l.sky[ch] + emit);
        let facing = v3::dot(nr, vw).abs();
        for (ki, (_, d, col)) in l.keys.iter().enumerate() {
            let vis = kvis.map_or(1.0, |k| k.vis[i * nk + ki]);
            let direct = positive(v3::dot(nr, *d)) * vis;
            for ch in 0..3 {
                light[ch] += direct * col[ch];
            }
            let (kp, back) = rims[ki];
            let away = 1.0 - facing;
            let rim = rimk * (away * away) * positive(v3::dot(nr, kp)) * vis;
            for ch in 0..3 {
                light[ch] += rim * col[ch];
            }
            if trans != 0.0 {
                let through = trans * vis * back;
                for ch in 0..3 {
                    // ... * c[None] * np.ones((N, 1))
                    light[ch] += through * col[ch] * 1.0;
                }
            }
        }
        for (d, col) in &l.fills {
            let direct = positive(v3::dot(nr, *d));
            for ch in 0..3 {
                light[ch] += direct * col[ch];
            }
        }
        for (lp, lc, radius) in &l.points {
            let v = v3::sub(*lp, c);
            // np.linalg.norm(v, axis=1): numpy's own loop, so exact here
            let dist = v3::norm(v) + 1e-6;
            // (nrm * v).sum(1): the three products added in order
            let toward = (nr[0] * v[0] + nr[1] * v[1] + nr[2] * v[2]) / dist;
            let lam = 0.3 + 0.7 * positive(toward);
            let reach = dist / radius;
            let amount = lam / (1.0 + reach * reach);
            for ch in 0..3 {
                light[ch] += amount * lc[ch];
            }
        }
        // The soft highlight roll-off: above 0.95 the brightest channel is bent toward 1.55, the others with it.
        let m = np::maximum(np::maximum(light[0], light[1]), light[2]);
        if m > 0.95 {
            let bent = 0.95 + 0.6 * (1.0 - dmath::exp(-(m - 0.95) / 0.6));
            for v in &mut light {
                *v = *v * bent / np::maximum(m, 1e-6);
            }
        }
        let b = base[if base.len() == 1 { 0 } else { i }];
        let mut col = [0.0; 3];
        for ch in 0..3 {
            col[ch] = np::clip(py::round_half_even(b[ch] * light[ch] / step) * step, 0.0, 255.0);
        }
        out.push(col);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white() -> [V3; 1] {
        [[200.0, 200.0, 200.0]]
    }

    #[test]
    fn directions_are_stored_as_unit_vectors() {
        let mut l = Lights::new([0.3, 0.3, 0.4], [0.1, 0.1, 0.15]);
        assert_eq!(l.view, [0.0, 0.0, 1.0]);
        l.key("sun", [0.0, 3.0, 4.0], [1.0, 1.0, 1.0]);
        l.fill([2.0, 0.0, 0.0], [0.5, 0.5, 0.5]);
        l.point([1.0, 2.0, 3.0], [1.0, 0.0, 0.0], 2.5);
        assert_eq!(l.keys[0], ("sun".to_string(), [0.0, 0.6, 0.8], [1.0, 1.0, 1.0]));
        assert_eq!(l.fills[0].0, [1.0, 0.0, 0.0]);
        assert_eq!(l.points[0], ([1.0, 2.0, 3.0], [1.0, 0.0, 0.0], 2.5));
    }

    #[test]
    fn ambient_alone_scales_the_base_and_snaps_to_fives() {
        let l = Lights::new([0.5, 0.25, 0.1], [0.0; 3]);
        let got = shade(&l, &[[1.0, 0.0, 0.0]], &[[0.0; 3]], &white(), None, 0.55, 0.0, 5.0);
        assert_eq!(got, vec![[100.0, 50.0, 20.0]]);
        // 0.33 * 200 = 66 snaps to 65; a step of 1 keeps it.
        let l = Lights::new([0.33; 3], [0.0; 3]);
        let face = ([[1.0, 0.0, 0.0]], [[0.0; 3]]);
        assert_eq!(
            shade(&l, &face.0, &face.1, &white(), None, 0.55, 0.0, 5.0)[0],
            [65.0; 3]
        );
        assert_eq!(
            shade(&l, &face.0, &face.1, &white(), None, 0.55, 0.0, 1.0)[0],
            [66.0; 3]
        );
        // Self-illumination adds to every channel; upward faces take the sky.
        assert_eq!(
            shade(&l, &face.0, &face.1, &white(), None, 0.55, 0.5, 1.0)[0],
            [166.0; 3]
        );
        let l = Lights::new([0.0; 3], [0.5, 0.25, 0.0]);
        let up = [[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
        let got = shade(&l, &up, &[[0.0; 3]; 2], &white(), None, 0.0, 0.0, 5.0);
        assert_eq!(got, vec![[100.0, 50.0, 0.0], [0.0, 0.0, 0.0]]);
    }

    #[test]
    fn a_key_light_lights_what_faces_it_unless_shadowed() {
        let mut l = Lights::new([0.0; 3], [0.0; 3]);
        l.view = [0.0, 0.0, 1.0];
        l.key("key", [0.0, 0.0, 1.0], [0.5, 0.5, 0.5]);
        let faces = [[0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
        let centres = [[0.0; 3]; 2];
        let lit = shade(&l, &faces, &centres, &white(), None, 0.0, 0.0, 5.0);
        assert_eq!(lit, vec![[100.0; 3], [0.0; 3]]);
        let shadow = KeyVis {
            vis: &[0.0, 1.0],
            trans: 0.0,
        };
        let shadowed = shade(&l, &faces, &centres, &white(), Some(shadow), 0.0, 0.0, 5.0);
        assert_eq!(shadowed, vec![[0.0; 3], [0.0; 3]]);
        // One base colour per face.
        let two = [[200.0; 3], [100.0; 3]];
        l.fill([0.0, 0.0, -1.0], [1.0; 3]);
        assert_eq!(
            shade(&l, &faces, &centres, &two, None, 0.0, 0.0, 5.0),
            vec![[100.0; 3], [100.0; 3]]
        );
    }

    #[test]
    fn bright_light_rolls_off_below_white() {
        let l = Lights::new([3.0; 3], [0.0; 3]);
        let got = shade(&l, &[[1.0, 0.0, 0.0]], &[[0.0; 3]], &[[100.0; 3]], None, 0.0, 0.0, 5.0)[0];
        // 3.0 becomes 0.95 + 0.6 (1 - exp(-2.05 / 0.6)) = 1.53: 153 snaps to 155.
        assert_eq!(got, [155.0; 3]);
        let l = Lights::new([0.9; 3], [0.0; 3]);
        let got = shade(&l, &[[1.0, 0.0, 0.0]], &[[0.0; 3]], &[[300.0; 3]], None, 0.0, 0.0, 5.0)[0];
        assert_eq!(got, [255.0; 3]);
    }

    #[test]
    fn a_point_light_falls_off_with_distance() {
        let mut l = Lights::new([0.0; 3], [0.0; 3]);
        l.point([0.0, 0.0, 2.0], [1.0, 1.0, 1.0], 2.0);
        let n = [[0.0, 0.0, 1.0]];
        let near = shade(&l, &n, &[[0.0, 0.0, 1.0]], &white(), None, 0.0, 0.0, 1.0)[0][0];
        let far = shade(&l, &n, &[[0.0, 0.0, -2.0]], &white(), None, 0.0, 0.0, 1.0)[0][0];
        // At distance 1: 1 / (1 + 0.25) = 0.8 of 200; at 4: 1 / 5.
        assert_eq!((near, far), (160.0, 40.0));
    }

    #[test]
    fn light_comes_through_a_thin_face_where_the_key_reaches_it() {
        // A key light behind two faces that look at the camera: nothing direct. With `trans` its light comes
        // through the face it reaches (0.4 of 200), and not through the one in shadow.
        let mut l = Lights::new([0.0; 3], [0.0; 3]);
        l.view = [0.0, 0.0, 1.0];
        l.key("back", [0.0, 0.0, -2.0], [1.0; 3]);
        let faces = [[0.0, 0.0, 1.0]; 2];
        let centres = [[0.0; 3]; 2];
        let reached = [1.0, 0.0];
        let with = |trans: f64| {
            let kvis = KeyVis { vis: &reached, trans };
            shade(&l, &faces, &centres, &white(), Some(kvis), 0.0, 0.0, 5.0)
        };
        assert_eq!(with(0.0), vec![[0.0; 3], [0.0; 3]]);
        assert_eq!(with(0.4), vec![[80.0; 3], [0.0; 3]]);
    }

    #[test]
    #[should_panic(expected = "kvis must hold 1 x 1 values")]
    fn shadow_terms_of_the_wrong_size_are_refused() {
        let mut l = Lights::new([0.0; 3], [0.0; 3]);
        l.key("key", [0.0, 0.0, 1.0], [1.0; 3]);
        let kvis = KeyVis {
            vis: &[1.0, 1.0],
            trans: 0.0,
        };
        shade(&l, &[[0.0, 0.0, 1.0]], &[[0.0; 3]], &white(), Some(kvis), 0.0, 0.0, 5.0);
    }
}
