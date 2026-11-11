//! The renderer: raster, camera, lighting, shadows, lens, post, effects, font, HUD, captions, ring and title card.
//!
//! The foundations are here: [`img`] (the image type), [`np`] (numpy's number semantics), [`config`]
//! (`reelkit/config.py`), [`font`] (`reelkit/font.py`), [`pil`] (Pillow 12.3.0's drawing primitives, blur and
//! resize, pixel for pixel) and [`sprite`] (the mesh sprites of the instruments). The documentation of [`pil`] is
//! the manual for the drawing calls.
//!
//! On them stands the renderer kit, the 3D-to-2D machinery of `reelkit` a scene is drawn with: [`camera`] (the
//! look-at camera), [`geometry`] (low-poly builders), [`raster`] (the frame: colour and depth, and the glow
//! sprites), [`lighting`] (lights and flat shading), [`shadows`] (planar shadow masks), [`lens`] (depth of
//! field, on the FFT convolution of [`fft`]), [`post`] (bloom, grade, vignette, top scrim) and [`hull`] (the
//! convex hull in scipy's order). Each module's documentation says how close to Python it is and why.
//!
//! [`fx`] is the effects kit (`reelkit/fx.py`): attack lights, impact bursts, camera shake, heat shimmer,
//! chromatic fringing, particles, marks on the floor and the ability moments.

pub mod camera;
pub mod config;
pub mod fft;
pub mod font;
pub mod fx;
pub mod geometry;
pub mod hull;
pub mod img;
pub mod lens;
pub mod lighting;
pub mod np;
pub mod pil;
pub mod post;
pub mod raster;
pub mod shadows;
pub mod sprite;
pub mod title;
pub mod ui;
mod v3;

pub use self::img::{Gray32, Img, Rgb8, RgbF};
