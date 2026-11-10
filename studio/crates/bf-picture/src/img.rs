//! The image buffer the whole renderer shares.

/// A `w` x `h` picture of `c` interleaved channels, rows top to bottom: channel `k` of the pixel at `(x, y)` is
/// `data[(y * w + x) * c + k]`.
pub struct Img<T> {
    pub w: usize,
    pub h: usize,
    pub c: usize,
    pub data: Vec<T>,
}

pub type Rgb8 = Img<u8>;
pub type RgbF = Img<f64>;
pub type Gray32 = Img<f32>;

impl<T: Clone + Default> Img<T> {
    /// A `w` x `h` picture of `c` channels with every sample `T::default()` (zero).
    pub fn new(w: usize, h: usize, c: usize) -> Self {
        Img {
            w,
            h,
            c,
            data: vec![T::default(); w * h * c],
        }
    }
}
