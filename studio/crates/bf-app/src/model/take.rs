use std::sync::Arc;

/// One picture of the video, `w` x `h` pixels of RGB: 360 x 640 unless said.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgb: Arc<[u8]>,
}
