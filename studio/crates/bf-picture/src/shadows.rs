//! `reelkit/shadows.py`: planar shadow masks. Caster faces are projected from a point light onto the ground plane
//! `y = 0` ([`proj_point`]), rasterised into a mask in world x and z ([`MaskGrid::raster`]), and the mask is read
//! back per pixel of the ground ([`MaskGrid::sample`]).
//!
//! All three are exact against Python (`studio/reference/picture_kit/shadows.npz`): the mask is Pillow's polygon
//! fill and Gaussian blur ([`crate::pil`]) and the rest is plain arithmetic.
//!
//! Not ported (the BALLFIGHT renderer does not call them): `proj_dir`, `directional_mask`, `point_mask`,
//! `ground_point`, the `Occluders` ray test and `contact_ao`.

use crate::img::Img;
use crate::v3::V3;
use crate::{np, pil};

/// `MaskGrid`: a raster over world x and z. `ext` is `(x0, x1, z0, z1, units per pixel)`; the mask is `w` pixels
/// along x and `h` along z, `(int((x1 - x0) / res), int((z1 - z0) / res))`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskGrid {
    pub ext: [f64; 5],
    pub w: usize,
    pub h: usize,
}

impl MaskGrid {
    /// `MaskGrid(ext)`.
    pub fn new(ext: [f64; 5]) -> MaskGrid {
        let [x0, x1, z0, z1, res] = ext;
        // int() drops the fraction; a size below zero has no image in Python either.
        let size = |a: f64, b: f64| ((b - a) / res).trunc().max(0.0) as usize;
        MaskGrid {
            ext,
            w: size(x0, x1),
            h: size(z0, z1),
        }
    }

    /// `raster(polys_xz, blur=0.0)`: the polygons (corners as world `(x, z)`) filled into the mask, then blurred
    /// by `blur` pixels if that is not 0. The result is `float32`, 0 to 1, one value per pixel.
    pub fn raster<P: AsRef<[(f64, f64)]>>(&self, polys_xz: &[P], blur: f64) -> Img<f32> {
        let [x0, _, z0, _, res] = self.ext;
        let mut im: Img<u8> = Img::new(self.w, self.h, 1);
        for q in polys_xz {
            let pts: Vec<(f64, f64)> = q
                .as_ref()
                .iter()
                .map(|&(x, z)| ((x - x0) / res, (z - z0) / res))
                .collect();
            pil::polygon(&mut im, &pts, &[255]);
        }
        if blur != 0.0 {
            im = pil::blur::gaussian_blur(&im, blur);
        }
        Img {
            w: im.w,
            h: im.h,
            c: 1,
            data: im.data.iter().map(|&v| f32::from(v) / 255.0).collect(),
        }
    }

    /// `sample(mask, X, Z)`: the mask's value under each world position, 0 outside the grid. `x` and `z` run
    /// together.
    pub fn sample(&self, mask: &Img<f32>, x: &[f64], z: &[f64]) -> Vec<f32> {
        assert_eq!(x.len(), z.len(), "sample: X and Z differ in length");
        assert_eq!(mask.c, 1, "sample: the mask has one channel");
        let [x0, _, z0, _, res] = self.ext;
        x.iter()
            .zip(z)
            .map(|(&px, &pz)| {
                let i = np::f64_to_i64(((pz - z0) / res).floor());
                let j = np::f64_to_i64(((px - x0) / res).floor());
                if i >= 0 && (i as u64) < mask.h as u64 && j >= 0 && (j as u64) < mask.w as u64 {
                    mask.data[i as usize * mask.w + j as usize]
                } else {
                    0.0
                }
            })
            .collect()
    }
}

/// `proj_point(Q, lp, max_scale=0.93)`: the shadow of world points cast from a point light at `lp` onto `y = 0`,
/// as `(x, z)`. A point near or above the light (above `lp.y * max_scale`) is treated as being at that height, so
/// its shadow lands far away instead of nowhere.
pub fn proj_point(q: &[V3], lp: V3, max_scale: f64) -> Vec<(f64, f64)> {
    q.iter()
        .map(|p| {
            let y = np::minimum(p[1], lp[1] * max_scale);
            let sc = lp[1] / (lp[1] - y);
            (lp[0] + (p[0] - lp[0]) * sc, lp[2] + (p[2] - lp[2]) * sc)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_canvas_grid_is_360_square() {
        let g = MaskGrid::new([-5.4, 5.4, -5.4, 5.4, 0.03]);
        assert_eq!((g.w, g.h), (360, 360));
        let g = MaskGrid::new([0.0, 10.0, 0.0, 5.0, 0.3]);
        assert_eq!((g.w, g.h), (33, 16));
    }

    #[test]
    fn a_square_rasters_to_ones_and_samples_back() {
        let g = MaskGrid::new([0.0, 4.0, 0.0, 2.0, 0.5]);
        assert_eq!((g.w, g.h), (8, 4));
        // x 1..2 and z 0.5..1 in world units: pixels 2..=4 by 1..=2 (Pillow fills both ends).
        let mask = g.raster(&[vec![(1.0, 0.5), (2.0, 0.5), (2.0, 1.0), (1.0, 1.0)]], 0.0);
        assert_eq!((mask.w, mask.h, mask.c), (8, 4, 1));
        let rows: Vec<String> = (0..4)
            .map(|y| mask.row(y).iter().map(|&v| if v == 1.0 { '#' } else { '.' }).collect())
            .collect();
        assert_eq!(rows, ["........", "..###...", "..###...", "........"]);
        assert_eq!(
            g.sample(&mask, &[1.2, 0.2, 2.4, -0.1, 4.0, 1.2], &[0.7, 0.7, 1.4, 0.7, 0.7, 2.0]),
            [1.0, 0.0, 1.0, 0.0, 0.0, 0.0]
        );
        // A blur spreads it: the middle stays the brightest, the neighbours rise above 0.
        let soft = g.raster(&[vec![(1.0, 0.5), (2.0, 0.5), (2.0, 1.0), (1.0, 1.0)]], 0.7);
        assert!(soft.px(3, 1)[0] > soft.px(1, 1)[0] && soft.px(1, 1)[0] > 0.0);
        assert!(g.raster::<Vec<(f64, f64)>>(&[], 0.7).data.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn shadows_fall_away_from_the_light() {
        let lp = [0.0, 10.0, 0.0];
        // Halfway up, the shadow is twice as far out; on the ground it is the point itself.
        let got = proj_point(&[[1.0, 5.0, 2.0], [3.0, 0.0, -1.0]], lp, 0.93);
        assert_eq!(got, vec![(2.0, 4.0), (3.0, -1.0)]);
        // At and above the light the height is held at 0.93 of the light's.
        let high = proj_point(&[[1.0, 10.0, 0.0], [1.0, 50.0, 0.0], [1.0, 9.3, 0.0]], lp, 0.93);
        assert_eq!(high[0], high[1]);
        assert_eq!(high[0], high[2]);
        assert!((high[0].0 - 1.0 / 0.07).abs() < 1e-9);
    }
}
