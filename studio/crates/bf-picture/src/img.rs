//! The image type every picture stage shares: a width, a height, a channel count and the samples.
//!
//! The samples are row-major and interleaved, exactly as a C-order numpy array of shape `(h, w)` (one channel) or
//! `(h, w, c)` lays them out: the sample of channel `k` at column `x`, row `y` is `data[(y * w + x) * c + k]`.
//! The fields are public; the methods are conveniences that never hide that layout.
//!
//! The three aliases name the forms the renderer uses: [`Rgb8`] is what `np.asarray(pil_image)` gives (`uint8`),
//! [`RgbF`] a float64 working picture, [`Gray32`] a float32 plane such as the depth buffer (Pillow's mode `F`).

/// An image: `w` columns, `h` rows, `c` samples per pixel, row-major and interleaved.
#[derive(Clone, Debug, PartialEq)]
pub struct Img<T> {
    pub w: usize,
    pub h: usize,
    pub c: usize,
    pub data: Vec<T>,
}

/// `uint8` samples: an RGB picture (`c == 3`), an RGBA patch (`c == 4`) or an `L` mask (`c == 1`).
pub type Rgb8 = Img<u8>;
/// `float64` samples: a working picture between the stages.
pub type RgbF = Img<f64>;
/// `float32` samples: the depth buffer and other float32 planes.
pub type Gray32 = Img<f32>;

impl<T: Clone + Default> Img<T> {
    /// A `w x h` image of `c` channels with every sample `T::default()` (zero): `np.zeros((h, w, c), dtype)`.
    pub fn new(w: usize, h: usize, c: usize) -> Self {
        Img {
            w,
            h,
            c,
            data: vec![T::default(); w * h * c],
        }
    }
}

impl<T: Clone> Img<T> {
    /// A `w x h` image whose every pixel is `px` (`px.len()` channels): `Image.new(mode, (w, h), px)`.
    pub fn filled(w: usize, h: usize, px: &[T]) -> Self {
        let mut data = Vec::with_capacity(w * h * px.len());
        for _ in 0..w * h {
            data.extend_from_slice(px);
        }
        Img {
            w,
            h,
            c: px.len(),
            data,
        }
    }
}

impl<T> Img<T> {
    /// Wraps samples that are already laid out row-major and interleaved. Panics unless there are `w * h * c`.
    pub fn from_vec(w: usize, h: usize, c: usize, data: Vec<T>) -> Self {
        assert_eq!(
            data.len(),
            w * h * c,
            "{w} x {h} x {c} wants {} samples, got {}",
            w * h * c,
            data.len()
        );
        Img { w, h, c, data }
    }

    /// The index in `data` of the first channel of the pixel at column `x`, row `y`.
    pub fn at(&self, x: usize, y: usize) -> usize {
        debug_assert!(
            x < self.w && y < self.h,
            "({x}, {y}) is outside {} x {}",
            self.w,
            self.h
        );
        (y * self.w + x) * self.c
    }

    /// The `c` samples of the pixel at column `x`, row `y`.
    pub fn px(&self, x: usize, y: usize) -> &[T] {
        let i = self.at(x, y);
        &self.data[i..i + self.c]
    }

    /// The `c` samples of the pixel at column `x`, row `y`, to write.
    pub fn px_mut(&mut self, x: usize, y: usize) -> &mut [T] {
        let i = self.at(x, y);
        &mut self.data[i..i + self.c]
    }

    /// Row `y`: `w * c` samples.
    pub fn row(&self, y: usize) -> &[T] {
        let n = self.w * self.c;
        &self.data[y * n..(y + 1) * n]
    }

    /// Row `y`, to write.
    pub fn row_mut(&mut self, y: usize) -> &mut [T] {
        let n = self.w * self.c;
        &mut self.data[y * n..(y + 1) * n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_row_major_and_interleaved() {
        let mut im: Img<u8> = Img::new(3, 2, 3);
        assert_eq!((im.w, im.h, im.c, im.data.len()), (3, 2, 3, 18));
        assert!(im.data.iter().all(|&v| v == 0));
        im.px_mut(2, 1).copy_from_slice(&[7, 8, 9]);
        assert_eq!(im.at(2, 1), 15);
        assert_eq!(&im.data[15..18], &[7, 8, 9]);
        assert_eq!(im.px(2, 1), &[7, 8, 9]);
        assert_eq!(im.row(1), &[0, 0, 0, 0, 0, 0, 7, 8, 9]);
        im.row_mut(0)[0] = 1;
        assert_eq!(im.px(0, 0), &[1, 0, 0]);
    }

    #[test]
    fn filled_repeats_the_pixel() {
        let im = Img::filled(2, 2, &[1.5f32]);
        assert_eq!((im.c, im.data.as_slice()), (1, &[1.5f32; 4][..]));
        let im = Img::filled(2, 1, &[1u8, 2, 3, 4]);
        assert_eq!((im.c, im.data.as_slice()), (4, &[1u8, 2, 3, 4, 1, 2, 3, 4][..]));
    }

    #[test]
    fn from_vec_takes_the_layout_as_given() {
        let im = Img::from_vec(2, 1, 2, vec![1u8, 2, 3, 4]);
        assert_eq!(im.px(1, 0), &[3, 4]);
    }

    #[test]
    #[should_panic(expected = "wants 6 samples, got 5")]
    fn from_vec_checks_the_length() {
        Img::from_vec(3, 2, 1, vec![0u8; 5]);
    }
}
