//! `ImageDraw.point`: `_draw_points` of `_imaging.c` over `ImagingDrawPoint`.

use super::{Plain, Sample, Surface, c_int};
use crate::img::Img;

/// `_draw_points`: one pixel per point, each coordinate truncated to a C `int`.
pub(crate) fn draw_points<S: Surface>(s: &mut S, pts: &[(f64, f64)]) {
    for &(x, y) in pts {
        s.point(c_int(x), c_int(y));
    }
}

/// `ImageDraw.Draw(im).point(pts, fill=fill)`: the pixel under each point (`draw.point((x, y), ...)` is a slice
/// of one point). Points outside the image are skipped.
pub fn point<T: Sample>(img: &mut Img<T>, pts: &[(f64, f64)], fill: &[T]) {
    draw_points(&mut Plain::new(img, fill), pts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_truncate_and_clip() {
        let mut im: Img<u8> = Img::new(3, 2, 1);
        point(
            &mut im,
            &[(-0.9, -0.2), (2.99, 1.5), (3.0, 0.0), (0.0, 2.0), (-1.0, 0.0)],
            &[7],
        );
        assert_eq!(im.data, [7, 0, 0, 0, 0, 7]);
    }
}
