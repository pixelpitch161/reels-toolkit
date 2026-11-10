//! The image type every part of the renderer shares.

/// An image of `w` x `h` pixels with `c` channels each, rows top to bottom, channels interleaved: the value of
/// channel `ch` at (`x`, `y`) is `data[(y * w + x) * c + ch]`.
pub struct Img<T> {
    pub w: usize,
    pub h: usize,
    pub c: usize,
    pub data: Vec<T>,
}

/// 8-bit pixels (RGB, RGBA or one channel).
pub type Rgb8 = Img<u8>;
/// Float pixels (numpy `float64` buffers).
pub type RgbF = Img<f64>;
/// One float32 channel (numpy `float32` buffers).
pub type Gray32 = Img<f32>;

impl<T: Clone + Default> Img<T> {
    /// A `w` x `h` image of `c` channels, every value zero.
    pub fn new(w: usize, h: usize, c: usize) -> Img<T> {
        Img {
            w,
            h,
            c,
            data: vec![T::default(); w * h * c],
        }
    }
}
