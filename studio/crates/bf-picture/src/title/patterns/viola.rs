//! VIOLA's panel on the VS card: `title.py` `_viola`, alto clefs, f-holes and varnish grain.
//!
//! Pixel by pixel in the order numpy's whole-array expressions run. The transcendentals are the two sines of the
//! varnish grain (`bf_core::dmath::sin` where Python has numpy's), which only decide on which side of 0.6 and -0.7 a
//! pixel's grain value lies. The glyphs (`_GLYPHS['alto']` and `['fhole']`, #42) are stamped upright on spots both
//! sides of the divider (`_spots_along`), so the pattern is the same on either panel.

use bf_core::dmath;
use bf_core::py::round_half_even;

use crate::config::{H, W};
use crate::img::RgbF;
use crate::title::layout::{self, DA, Layout};

/// Whether [`pattern`] is the kit's `_viola`.
pub const PORTED: bool = true;

/// `_GLYPHS['alto']`: the alto clef.
const ALTO: [&str; 15] = [
    "##.#..####.",
    "##.#.##..##",
    "##.#.#....#",
    "##.#......#",
    "##.#.....##",
    "##.#..###..",
    "##.#.##....",
    "##.###.....",
    "##.#.##....",
    "##.#..###..",
    "##.#.....##",
    "##.#......#",
    "##.#.#....#",
    "##.#.##..##",
    "##.#..####.",
];

/// `_GLYPHS['fhole']`: an f-hole.
const FHOLE: [&str; 15] = [
    "....##.", "...#..#", "....##.", "....#..", "....#..", "...#...", "...#...", ".####..", "..#....", "..#....",
    ".#.....", ".#.....", "##.....", "#..#...", ".##....",
];

/// `_spots_along(off, step, phase)`: screen spots on both sides of the divider, `|SIDE| = off`, every `step` px along
/// it: `DA + DD * (step * (j + phase)) + N * off * sgn` for `sgn` in (1, -1) and `j` from -2 to 15, those within 20 px
/// of the frame.
fn spots_along(off: f64, step: f64, phase: f64) -> Vec<[f64; 2]> {
    let (dd, n) = (layout::dd(), layout::normal());
    let mut out = Vec::new();
    for sgn in [1.0, -1.0] {
        for j in -2..16 {
            let a = step * (f64::from(j) + phase);
            let p = [
                DA[0] + dd[0] * a + n[0] * off * sgn,
                DA[1] + dd[1] * a + n[1] * off * sgn,
            ];
            if -20.0 < p[0] && p[0] < W as f64 + 20.0 && -20.0 < p[1] && p[1] < H as f64 + 20.0 {
                out.push(p);
            }
        }
    }
    out
}

/// `_glyph_mask(name, spots, scale)`: a full-frame mask with the glyph stamped (upright, each cell `scale` px)
/// centred on each spot: its top left at `int(round(cx - gw / 2)), int(round(cy - gh / 2))`, cut to the frame.
fn glyph_mask(rows: &[&str], spots: &[[f64; 2]], scale: usize) -> Vec<bool> {
    let (gh, gw) = (rows.len() * scale, rows[0].len() * scale);
    let cell = |y: usize, x: usize| rows[y / scale].as_bytes()[x / scale] == b'#';
    let mut m = vec![false; W * H];
    for &[cx, cy] in spots {
        let x0 = round_half_even(cx - gw as f64 / 2.0) as i64;
        let y0 = round_half_even(cy - gh as f64 / 2.0) as i64;
        let (xa, ya) = (x0.max(0), y0.max(0));
        let (xb, yb) = ((x0 + gw as i64).min(W as i64), (y0 + gh as i64).min(H as i64));
        for y in ya..yb {
            for x in xa..xb {
                if cell((y - y0) as usize, (x - x0) as usize) {
                    m[y as usize * W + x as usize] = true;
                }
            }
        }
    }
    m
}

/// `_viola(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: the varnish's grain, alto clefs 52
/// px from the divider every 46 px, f-holes (at 3x) 118 px from it every 70 px, half a step on. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    let clef = glyph_mask(&ALTO, &spots_along(52.0, 46.0, 0.0), 2);
    let fh = glyph_mask(&FHOLE, &spots_along(118.0, 70.0, 0.5), 3);
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        // g = np.sin((XX * 0.35 + YY * 0.94) / 6.0 + 1.8 * np.sin(XX / 33.0 + YY / 81.0))
        let g = dmath::sin((xx * 0.35 + yy * 0.94) / 6.0 + 1.8 * dmath::sin(xx / 33.0 + yy / 81.0));
        // base = base * (0.95 + 0.06 * (g > 0.6) - 0.05 * (g < -0.7))
        let grain = 0.95 + 0.06 * f64::from(u8::from(g > 0.6)) - 0.05 * f64::from(u8::from(g < -0.7));
        for v in px.iter_mut() {
            *v *= grain;
        }
        if clef[i] {
            for (v, c) in px.iter_mut().zip([236.0, 255.0, 246.0]) {
                *v = *v * 0.45 + c * 0.55;
            }
        }
        if fh[i] {
            for v in px.iter_mut() {
                *v *= 0.42;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_glyphs_are_whole_and_the_spots_fall_on_both_sides() {
        assert!(ALTO.iter().all(|r| r.len() == 11) && FHOLE.iter().all(|r| r.len() == 7));
        let spots = spots_along(52.0, 46.0, 0.0);
        let side = |p: &[f64; 2]| {
            let n = layout::normal();
            (p[0] - DA[0]) * n[0] + (p[1] - DA[1]) * n[1]
        };
        assert!(spots.iter().any(|p| side(p) > 51.9) && spots.iter().any(|p| side(p) < -51.9));
        // 21 x 45 px centred on (10, 10): its top left at (round(-0.5), round(-12.5)) = (0, -12), ties to even,
        // so the frame's top row is the glyph's row 12 / 3 = 4 ("....#..")
        let m = glyph_mask(&FHOLE, &[[10.0, 10.0]], 3);
        assert!(m[12] && m[14] && !m[11] && !m[15] && !m[0]);
    }
}
