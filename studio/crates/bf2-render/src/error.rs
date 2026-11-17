//! The renderer's one error: a plain sentence for the person running the render.

use std::fmt;

/// Something the renderer could not do, as a sentence that can be shown as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderError(pub String);

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RenderError {}
