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
//! [`ndimage`] is `scipy.ndimage.gaussian_filter` as a fighter's look calls it.
//!
//! [`fx`] is the effects kit (`reelkit/fx.py`): attack lights, impact bursts, camera shake, heat shimmer,
//! chromatic fringing, particles, marks on the floor and the ability moments.
//!
//! [`ring`] assembles the frame: the port of `kit/picture.py` on the static [`scene`] (`scene.py`). A
//! [`Picture`] is one fight's video, and [`Picture::frame`] a pure function of the frame's number. Each fighter
//! draws its own layers through a [`Look`] ([`look`]: the hooks of `kit/fighters/base.py`); the looks themselves
//! live in `bf-content`.

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
pub mod look;
pub mod ndimage;
pub mod np;
pub mod pil;
pub mod post;
pub mod raster;
pub mod ring;
pub mod scene;
pub mod shadows;
pub mod sprite;
pub mod title;
pub mod ui;
mod v3;

pub use self::img::{Gray32, Img, Rgb8, RgbF};
pub use self::look::{DefaultLook, Look};
pub use self::ring::{Picture, PictureError, Stage, upscale3};
