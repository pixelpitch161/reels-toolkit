//! VIOLIN's panel on the VS card: `title.py` `_violin` (#42), scrolls, f-holes and flamed maple.
//!
//! Pixel by pixel; the glyphs are `title.py`'s `_GLYPHS` (`scroll`, `fhole`) stamped by `_glyph_mask` on the spots
//! of `_spots_along`.

use bf_core::dmath;

use crate::config::{H, W};
use crate::img::RgbF;
use crate::title::layout::{self, DA, Layout};

/// Whether [`pattern`] is the kit's `_violin`.
pub const PORTED: bool = true;

/// `_GLYPHS['scroll']`.
const SCROLL: [&str; 12] = [
    "..######..",
    ".#......#.",
    "#..####..#",
    "#.#....#.#",
    "#.#.##.#.#",
    "#.#.#..#.#",
    "#.#..##..#",
    "#..#....#.",
    ".#..####..",
    "..#.......",
    "..#.......",
    "..#.......",
];

/// `_GLYPHS['fhole']`.
const FHOLE: [&str; 15] = [
    "....##.", "...#..#", "....##.", "....#..", "....#..", "...#...", "...#...", ".####..", "..#....", "..#....",
    ".#.....", ".#.....", "##.....", "#..#...", ".##....",
];

/// `_spots_along(off, step, phase)`: screen spots on both sides of the divider, `|SIDE| = off`, every `step` px
/// along it.
fn spots_along(off: f64, step: f64, phase: f64) -> Vec<(f64, f64)> {
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
                out.push((p[0], p[1]));
            }
        }
    }
    out
}

/// `_glyph_mask(name, spots, scale)`: a full-frame mask with the glyph stamped (upright) centred on each spot.
fn glyph_mask(rows: &[&str], spots: &[(f64, f64)], scale: usize) -> Vec<bool> {
    let (gh, gw) = (rows.len() * scale, rows[0].len() * scale);
    let on = |gy: usize, gx: usize| rows[gy / scale].as_bytes()[gx / scale] == b'#';
    let mut m = vec![false; W * H];
    for &(cx, cy) in spots {
        let x0 = (cx - gw as f64 / 2.0).round_ties_even() as i64;
        let y0 = (cy - gh as f64 / 2.0).round_ties_even() as i64;
        for gy in 0..gh {
            for gx in 0..gw {
                let (x, y) = (x0 + gx as i64, y0 + gy as i64);
                if (0..W as i64).contains(&x) && (0..H as i64).contains(&y) && on(gy, gx) {
                    m[y as usize * W + x as usize] = true;
                }
            }
        }
    }
    m
}

/// `_violin(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place: flamed maple stripes, a row of
/// pale scrolls near the divider and f-holes further out, on both sides. `card` is not read.
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let _ = card;
    let w = base.w;
    let scr = glyph_mask(&SCROLL, &spots_along(52.0, 44.0, 0.3), 2);
    let fh = glyph_mask(&FHOLE, &spots_along(118.0, 70.0, 0.0), 3);
    let pale = [246.0, 236.0, 255.0];
    for (i, px) in base.data.as_chunks_mut::<3>().0.iter_mut().enumerate() {
        let (xx, yy) = ((i % w) as f64, (i / w) as f64);
        let g = dmath::sin((xx * 0.94 - yy * 0.34) / 3.2 + 1.2 * dmath::sin(xx / 29.0 - yy / 61.0));
        let k = 0.94 + 0.08 * f64::from(u8::from(g > 0.55)) - 0.05 * f64::from(u8::from(g < -0.7));
        for (ch, v) in px.iter_mut().enumerate() {
            *v *= k;
            if scr.get(i) == Some(&true) {
                *v = *v * 0.45 + pale[ch] * 0.55;
            }
            if fh.get(i) == Some(&true) {
                *v *= 0.42;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolls_and_f_holes_are_stamped() {
        let scr = glyph_mask(&SCROLL, &spots_along(52.0, 44.0, 0.3), 2);
        let fh = glyph_mask(&FHOLE, &spots_along(118.0, 70.0, 0.0), 3);
        assert!(scr.iter().filter(|&&b| b).count() > 500);
        assert!(fh.iter().filter(|&&b| b).count() > 500);
    }
}
