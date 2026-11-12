//! Look, voice and part helpers that two or more kit fighters share, ported once so that no fighter's port adds to a
//! shared file (milestone 3, Task 4). Each is proven against the kit's own function on the arguments the fighters
//! give it (`tests/shared_helpers.rs`, vectors of the `shared` export in `studio/reference/shared/`).
//!
//! - [`reeds`]: `kit/fighters/_reeds.py` whole (the screen masks, `blend`, `add`, `ribbon`, `ring_exit`,
//!   `beam_lane`, `draw_woodwind_shard`, `tail_theta`, the colours), which nine looks import, and the same-code
//!   copies other looks keep: `_mask` and `blend` (BASS CLARINET, BASSOON, EUPHONIUM), `add` (BASSOON), `mask_of`
//!   (ACOUSTIC, TRUMPET), `mask_of` (HORN, TUBA), `mask_of` and `line_mask` (ORGAN).
//! - [`padded`]: CONTRABASS's `padded_sprite` (the SOUSAPHONE's too) and the BASS DRUM's `_render`, which are
//!   the same sprite with a wider crop.
//! - [`hash`]: the parts' deterministic hash, the same code under four names (ACOUSTIC `h01_33`, HORN `_h01_34`,
//!   ORGAN `h01_35`, TRUMPET `h01_29`).
//! - [`slap`]: `bassclarinet_lowpoly.slap`, the slap-tongue pop the BASS CLARINET's part plays and the EUPHONIUM's
//!   plays for a copied SLAP TONGUE.
//!
//! The Pixel Pitch voice blocks several voices share are in `bf_sound::voice::pp` (`brass`, `pluck`, `sax`), and
//! `scipy.ndimage.gaussian_filter` (TRUMPET) in `bf_picture::ndimage`.

pub mod hash;
pub mod padded;
pub mod reeds;
pub mod slap;
