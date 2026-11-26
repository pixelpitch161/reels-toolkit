//! The episode helpers that more than one older fighter's look calls (milestone 4, Task 4), each from the episode
//! named beside it. The older looks live in their episodes' `styles.py` (`projects/<episode>/`), and an episode's
//! `styles.py` serves both of its fighters, so a helper there is shared when two older looks reach it: from #30
//! (`ballfight_bagpipes_vs_accordion`, BAGPIPES and ACCORDION), #28 (`ballfight_drums_vs_violin`, DRUMS and VIOLIN,
//! whose look #42 imports), #31 (`ballfight_turntables_vs_synth`, TURNTABLES and SYNTH), #35
//! (`ballfight_organ_vs_harp`, HARP), #33 (`ballfight_ukulele_vs_acoustic`, UKULELE) and #39
//! (`ballfight_bassoon_vs_barisax`, BARI SAX).
//!
//! Several are the same code as a kit helper already ported (milestone 3's [`super::reeds`]) and are that one here,
//! under the episode's name: the masks and the blends. The new ones are `annulus_polys`, `dart_quads` and
//! `draw_dart`. Each is proven against the episodes' own functions on the arguments the episodes give them
//! (`tests/older_helpers.rs`, the `older_vectors` export's `studio/reference/older/vectors/helpers.json`).
//!
//! Not here, because no two older looks share them: `_mask` (BARI SAX alone, [`super::reeds::mask`]), the episodes'
//! renderer functions (`solo_env`, `sim_i`, `layer_blend`, `screen`, ...: the kit renderer's, already in
//! `bf_picture::ring`), and the shatter parts (each look draws its own).

use bf_core::dmath;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::lighting::{Lights, shade};
use bf_picture::raster::Frame;
use bf_picture::{np, scene};

use super::reeds::{self, Num, Px};

/// A screen polygon.
pub type Poly = Vec<(f64, f64)>;

/// `mask_of(polys)` of #30 and #28 (`styles.py` 40 and 91): every list filled, whatever its length (Pillow's
/// polygon). The kit's HORN and TUBA keep the same code: [`reeds::mask_all`].
///
/// # Panics
/// Where Pillow raises: a list of fewer than two points.
pub fn mask_of_all(polys: &[Poly]) -> Img<f32> {
    reeds::mask_all(polys)
}

/// `mask_of(polys)` of #31 and #35 (`styles.py` 32 and 212): the lists of three points or more filled.
pub fn mask_of_polys(polys: &[Poly]) -> Img<f32> {
    reeds::mask(polys, &[], 1)
}

/// `mask_of(polys, width=None)` of #33 (`styles.py` 22): with a width every list drawn as a line of it, without it
/// the lists of three points or more filled. The kit's ACOUSTIC (the same episode's) and TRUMPET keep the same code:
/// [`reeds::mask_of`].
pub fn mask_of_lines(polys: &[Poly], width: Option<u32>) -> Img<f32> {
    reeds::mask_of(polys, width)
}

/// `blend(arr, m, col, k)` of #30 (`styles.py` 48): `arr += (col - arr) * (m * k)[..., None]`, in place, for any
/// `k`. The kit's TRUMPET keeps the same code: [`reeds::blend_into`].
pub fn blend_into<T: Px, M: Px>(img: &mut Img<T>, mk: &Img<M>, col: [f64; 3], alpha: Num) {
    reeds::blend_into(img, mk, col, alpha);
}

/// `blend(img, mk, col, alpha)` of #39 (`styles.py` 36): nothing when `alpha <= 0`, else [`blend_into`]. The kit's
/// `_reeds.blend`: [`reeds::blend`].
pub fn blend_early<T: Px, M: Px>(img: &mut Img<T>, mk: &Img<M>, col: [f64; 3], alpha: Num) {
    reeds::blend(img, mk, col, alpha);
}

/// `blend(arr, m, col, k)` of #33 (`styles.py` 33): [`blend_into`] with the colour clipped to 0..255 first. The kit's
/// ACOUSTIC keeps the same code.
pub fn blend_clip<T: Px, M: Px>(img: &mut Img<T>, mk: &Img<M>, col: [f64; 3], alpha: Num) {
    reeds::blend_into(img, mk, col.map(|c| np::clip(c, 0.0, 255.0)), alpha);
}

/// `np.linspace(start, stop, num)` (`num` at least 2): `k * step + start`, the last exactly `stop`; a zero step
/// is `k / div * delta + start`, as numpy computes it.
fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    let div = (num - 1) as f64;
    let delta = stop - start;
    let step = delta / div;
    let mut y: Vec<f64> = (0..num)
        .map(|k| {
            if step == 0.0 {
                k as f64 / div * delta + start
            } else {
                k as f64 * step + start
            }
        })
        .collect();
    y[num - 1] = stop;
    y
}

/// `annulus_polys(cam, cx, cz, r_in, r_out, y, n=36, a0=0.0, a1=2 * math.pi)` of #30 (`styles.py` 31): the screen
/// quads of a flat ring on the canvas at height `y` (or of an arc `a0..a1`), one quad per segment, each
/// `[outer j, outer j + 1, inner j + 1, inner j]`. #28's (`styles.py` 82, DRUMS's drumhead) is the same with the
/// defaults `a0 = 0`, `a1 = 2 pi`. The Python defaults: `n = 36`, `a0 = 0.0`, `a1 = 2 pi`.
///
/// # Panics
/// For `n` of 0 (numpy's `linspace` of one point would make no quad either; no episode asks for it).
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn annulus_polys(
    cam: &Cam,
    cx: f64,
    cz: f64,
    r_in: f64,
    r_out: f64,
    y: f64,
    n: usize,
    a0: f64,
    a1: f64,
) -> Vec<[(f64, f64); 4]> {
    assert!(n > 0, "annulus_polys: no segments");
    let a = linspace(a0, a1, n + 1);
    let ring = |r: f64| -> Vec<V3> {
        a.iter()
            .map(|&u| scene::to_world(cx + r * dmath::cos(u), cz + r * dmath::sin(u), y))
            .collect()
    };
    let (so, si) = (cam.wp(&ring(r_out)), cam.wp(&ring(r_in)));
    (0..n).map(|j| [so[j], so[j + 1], si[j + 1], si[j]]).collect()
}

/// `dart_quads(x, z, ang, y=0.48, L_=0.34, Wd=0.07)` of #28, #35 and #33 (`styles.py` 134, 195, 136: the same
/// code): a dart's head (a kite) and its body (a thin quad) as world points, the dart at ring `(x, z)` pointing
/// along `ang`, at height `y`. The Python defaults: `y = 0.48`, `l = 0.34`, `wd = 0.07`.
pub fn dart_quads(x: f64, z: f64, ang: f64, y: f64, l: f64, wd: f64) -> ([V3; 4], [V3; 4]) {
    let d = [dmath::cos(ang), dmath::sin(ang)];
    let n = [-d[1], d[0]];
    let c = [x, z];
    // c + d * L_ * k + n * w, numpy's order: (d * L_) * k, then the sums left to right
    let at = |k: f64, w: f64| -> V3 {
        let q: [f64; 2] = std::array::from_fn(|i| c[i] + d[i] * l * k + n[i] * w);
        scene::to_world(q[0], q[1], y)
    };
    let at_minus = |k: f64, w: f64| -> V3 {
        // c + d * L_ * k - n * w
        let q: [f64; 2] = std::array::from_fn(|i| c[i] + d[i] * l * k - n[i] * w);
        scene::to_world(q[0], q[1], y)
    };
    let back = |w: f64, plus: bool| -> V3 {
        // c - d * L_ * 0.5 (+ or -) n * 0.012
        let q: [f64; 2] = std::array::from_fn(|i| {
            let b = c[i] - d[i] * l * 0.5;
            if plus { b + n[i] * w } else { b - n[i] * w }
        });
        scene::to_world(q[0], q[1], y)
    };
    let head = [
        scene::to_world(c[0] + d[0] * l * 0.5, c[1] + d[1] * l * 0.5, y),
        at_minus(0.15, wd),
        scene::to_world(c[0] + d[0] * l * 0.05, c[1] + d[1] * l * 0.05, y),
        at(0.15, wd),
    ];
    let body = [
        at_minus(0.1, 0.012),
        back(0.012, false),
        back(0.012, true),
        at(0.1, 0.012),
    ];
    (head, body)
}

/// Whose `draw_dart` it is: the episodes keep the same shape with their own colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dart {
    /// #28's (`styles.py` 143): VIOLIN's PIZZICATO darts (#42 imports it). Body `col * 0.5 + 120`, head
    /// `col * 0.45 + DART_COL * 0.55` lit, plus 20.
    Violin,
    /// #35's (`styles.py` 204): HARP's darts. Body `col * 0.6 + 90`, head `col * 0.5 + PEARL * 0.5` lit, plus 25,
    /// clipped.
    Harp,
}

/// #28's `DART_COL`.
pub const DART_COL: [f64; 3] = [214.0, 232.0, 255.0];
/// #35's `PEARL`.
pub const PEARL_35: [f64; 3] = [255.0, 246.0, 222.0];

/// `draw_dart(fr, cam, L, shade, x, z, ang, col)`: a dart in flight drawn into the frame as two lit polygons, its
/// body then its head (with depth), in the colours of `style`.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn draw_dart(fr: &mut Frame, cam: &Cam, l: &Lights, x: f64, z: f64, ang: f64, col: V3, style: Dart) {
    let (head, body) = dart_quads(x, z, ang, 0.48, 0.34, 0.07);
    let (body_k, body_add, mix, tone, lift) = match style {
        Dart::Violin => (0.5, 120.0, 0.45, DART_COL.map(|c| c * 0.55), 20.0),
        Dart::Harp => (0.6, 90.0, 0.5, PEARL_35.map(|c| c * 0.5), 25.0),
    };
    fr.poly(
        cam,
        &body,
        col.map(|c| np::clip(c * body_k + body_add, 0.0, 255.0)),
        true,
    );
    // head.mean(0): the rows added in order, then divided by 4
    let mut ctr = [0.0; 3];
    for q in &head {
        for i in 0..3 {
            ctr[i] += q[i];
        }
    }
    let ctr = ctr.map(|v| v / 4.0);
    let base: V3 = std::array::from_fn(|i| col[i] * mix + tone[i]);
    let c = shade(l, &[[0.0, 1.0, 0.0]], &[ctr], &[base], None, 0.4, 0.0, 5.0)[0];
    let lit = match style {
        // c + 20: poly clips it to 0..255 itself
        Dart::Violin => c.map(|v| v + lift),
        Dart::Harp => c.map(|v| np::clip(v + lift, 0.0, 255.0)),
    };
    fr.poly(cam, &head, lit, true);
}
