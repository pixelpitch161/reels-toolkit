//! `kit/fighters/contrabass.py` `padded_sprite` (which the SOUSAPHONE imports) and `kit/fighters/bass_drum.py`
//! `_render`: the model drawn as `sprites.render_sprite` draws it at radius `r`, but cropped round a square of
//! half-side `int(round(r * pad))`, because those instruments reach past the ball's crop. The two are one function
//! of the model; the BASS DRUM passes its own variant model (the drum without its mallet), the others their
//! roster model. The drawing itself is `bf_picture::sprite::render_sprite_padded`.
//!
//! Not here, because one fighter uses each: CONTRABASS's `jag_ring` (the quake ring) and ELECTRIC's (a different
//! function of the same name).

use bf_core::py;
use bf_picture::img::Img;
use bf_picture::sprite::{Mesh, render_sprite_padded};

/// `contrabass.PAD`, `padded_sprite`'s default `pad`: the contrabass model's crop margin (its x1.2 size). The
/// SOUSAPHONE passes its own (1.22), the BASS DRUM its own.
pub const PAD: f64 = 1.25;

/// `padded_sprite(name, r, yaw, roll, pad)` (and `_render(geo, r, yaw, roll, pad)`) for the model `mesh`: RGBA,
/// side `2 * int(round(r * pad)) + 7`.
///
/// # Panics
/// If the crop does not fit the sprite renderer's 270 x 480 canvas (`int(round(r * pad))` above 131). Python does
/// not raise there: `img[:n, :n]` quietly returns a picture narrower than `n` (cut at the canvas's 270 columns), not
/// the square sprite the ring expects; this refuses it instead (no kit look asks for one).
pub fn padded_sprite(mesh: &Mesh, r: u32, yaw: f64, roll: f64, pad: f64) -> Img<u8> {
    let half = py::round_half_even(f64::from(r) * pad);
    assert!(
        (0.0..=f64::from(u32::MAX)).contains(&half),
        "padded_sprite: a crop of {half} pixels"
    );
    render_sprite_padded(mesh, r, yaw, roll, half as u32)
}
