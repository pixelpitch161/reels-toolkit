//! The renderer: raster, camera, lighting, shadows, lens, post, effects, font, HUD, captions, ring and title card.
//!
//! The foundations are here: [`img`] (the image type), [`np`] (numpy's number semantics), [`config`]
//! (`reelkit/config.py`), [`font`] (`reelkit/font.py`), [`pil`] (Pillow 12.3.0's drawing primitives, blur and
//! resize, pixel for pixel) and [`sprite`] (the mesh sprites of the instruments). The documentation of [`pil`] is
//! the manual for the drawing calls.

pub mod config;
pub mod font;
pub mod img;
pub mod np;
pub mod pil;
pub mod sprite;
pub mod title;
pub mod ui;

pub use self::img::{Gray32, Img, Rgb8, RgbF};
