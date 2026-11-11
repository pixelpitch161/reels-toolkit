use std::sync::Arc;

/// A one-bit picture, such as a fighter's name in pixel letters. Row-major; `h` is 7 for one line of text.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelMask {
    pub w: usize,
    pub h: usize,
    pub on: Vec<bool>,
}

/// A fighter's portrait, `w` x `h` pixels of RGBA.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub rgba: Arc<[u8]>,
}
