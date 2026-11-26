//! `reelkit/font.py`: the chunky 5 x 7 pixel font drawn by hand, and the helpers that stamp outlined text into a
//! frame.
//!
//! A mask is an `Img<u8>` of one channel holding 0 or 1 (numpy's `bool` array); any non-zero value counts as set.
//! `studio/reference/picture_kit/font.npz` checks every glyph, `text_mask`, `dilate` and `stamp` against Python.
//!
//! Not ported: `rich_line` (the typewriter captions). The fight draws its captions in the `impact` style, which
//! builds its own masks from [`text_mask`].

use crate::img::{Img, Rgb8};
use crate::np;

/// `ADV`: the advance from one character to the next: a 5 px glyph and a 1 px gap.
pub const ADV: i32 = 6;

/// `stamp`'s default outline colour, `(10, 20, 26)`.
pub const OUTLINE: [u8; 3] = [10, 20, 26];

/// A glyph's width and height in font pixels.
const GW: usize = 5;
const GH: usize = 7;

/// `_G`: every glyph, in the table's order: seven rows of five, `#` set and `.` clear, rows separated by `|`.
const GLYPHS: [(char, &str); 58] = [
    ('A', ".###.|#...#|#...#|#####|#...#|#...#|#...#"),
    ('B', "####.|#...#|#...#|####.|#...#|#...#|####."),
    ('C', ".###.|#...#|#....|#....|#....|#...#|.###."),
    ('D', "####.|#...#|#...#|#...#|#...#|#...#|####."),
    ('E', "#####|#....|#....|####.|#....|#....|#####"),
    ('F', "#####|#....|#....|####.|#....|#....|#...."),
    ('G', ".###.|#...#|#....|#.###|#...#|#...#|.####"),
    ('H', "#...#|#...#|#...#|#####|#...#|#...#|#...#"),
    ('I', "#####|..#..|..#..|..#..|..#..|..#..|#####"),
    ('J', "..###|...#.|...#.|...#.|#..#.|#..#.|.##.."),
    ('K', "#...#|#..#.|#.#..|##...|#.#..|#..#.|#...#"),
    ('L', "#....|#....|#....|#....|#....|#....|#####"),
    ('M', "#...#|##.##|#.#.#|#.#.#|#...#|#...#|#...#"),
    ('N', "#...#|##..#|#.#.#|#..##|#...#|#...#|#...#"),
    ('O', ".###.|#...#|#...#|#...#|#...#|#...#|.###."),
    ('P', "####.|#...#|#...#|####.|#....|#....|#...."),
    ('Q', ".###.|#...#|#...#|#...#|#.#.#|#..#.|.##.#"),
    ('R', "####.|#...#|#...#|####.|#.#..|#..#.|#...#"),
    ('S', ".####|#....|#....|.###.|....#|....#|####."),
    ('T', "#####|..#..|..#..|..#..|..#..|..#..|..#.."),
    ('U', "#...#|#...#|#...#|#...#|#...#|#...#|.###."),
    ('V', "#...#|#...#|#...#|#...#|#...#|.#.#.|..#.."),
    ('W', "#...#|#...#|#...#|#.#.#|#.#.#|#.#.#|.#.#."),
    ('X', "#...#|#...#|.#.#.|..#..|.#.#.|#...#|#...#"),
    ('Y', "#...#|#...#|.#.#.|..#..|..#..|..#..|..#.."),
    ('Z', "#####|....#|...#.|..#..|.#...|#....|#####"),
    ('0', ".###.|#...#|#..##|#.#.#|##..#|#...#|.###."),
    ('1', "..#..|.##..|..#..|..#..|..#..|..#..|.###."),
    ('2', ".###.|#...#|....#|...#.|..#..|.#...|#####"),
    ('3', "####.|....#|....#|.###.|....#|....#|####."),
    ('4', "...#.|..##.|.#.#.|#..#.|#####|...#.|...#."),
    ('5', "#####|#....|####.|....#|....#|#...#|.###."),
    ('6', ".###.|#....|#....|####.|#...#|#...#|.###."),
    ('7', "#####|....#|...#.|..#..|.#...|.#...|.#..."),
    ('8', ".###.|#...#|#...#|.###.|#...#|#...#|.###."),
    ('9', ".###.|#...#|#...#|.####|....#|....#|.###."),
    ('.', ".....|.....|.....|.....|.....|.##..|.##.."),
    (',', ".....|.....|.....|.....|.##..|..#..|.#..."),
    ('!', "..#..|..#..|..#..|..#..|..#..|.....|..#.."),
    ('?', ".###.|#...#|....#|...#.|..#..|.....|..#.."),
    (':', ".....|.##..|.##..|.....|.##..|.##..|....."),
    ('/', "....#|....#|...#.|..#..|.#...|#....|#...."),
    ('%', "##..#|##..#|...#.|..#..|.#...|#..##|#..##"),
    ('&', ".##..|#..#.|#.#..|.#...|#.#.#|#..#.|.##.#"),
    ('+', ".....|..#..|..#..|#####|..#..|..#..|....."),
    ('#', ".#.#.|#####|.#.#.|.#.#.|#####|.#.#.|....."),
    ('(', "...#.|..#..|.#...|.#...|.#...|..#..|...#."),
    (')', ".#...|..#..|...#.|...#.|...#.|..#..|.#..."),
    ('=', ".....|.....|#####|.....|#####|.....|....."),
    ('\u{b0}', ".##..|#..#.|#..#.|.##..|.....|.....|....."),
    ('\u{d7}', ".....|#...#|.#.#.|..#..|.#.#.|#...#|....."),
    ('\u{f7}', "..#..|.....|#####|.....|..#..|.....|....."),
    ('>', ".#...|..#..|...#.|....#|...#.|..#..|.#..."),
    ('<', "...#.|..#..|.#...|#....|.#...|..#..|...#."),
    ('"', ".#.#.|.#.#.|.....|.....|.....|.....|....."),
    ('-', ".....|.....|.....|#####|.....|.....|....."),
    ('\'', "..#..|..#..|.#...|.....|.....|.....|....."),
    (' ', ".....|.....|.....|.....|.....|.....|....."),
];

/// The rows of the glyph for exactly `ch` (no case folding), if the font has one.
fn rows_of(ch: char) -> Option<&'static str> {
    GLYPHS.iter().find(|(c, _)| *c == ch).map(|(_, rows)| *rows)
}

/// Whether the font pixel at column `x`, row `y` of a glyph's rows is set.
fn glyph_bit(rows: &str, x: usize, y: usize) -> bool {
    rows.as_bytes()[y * (GW + 1) + x] == b'#'
}

/// The characters the font has, in the table's order (`''.join(_G)`).
pub fn glyph_chars() -> String {
    GLYPHS.iter().map(|(c, _)| *c).collect()
}

/// `GLYPHS[ch]`: the 7 rows of 5 pixels of the glyph for exactly `ch`, or `None` if the font has no such
/// character. (Lower case is folded by [`text_mask`], not here.)
pub fn glyph(ch: char) -> Option<[[bool; 5]; 7]> {
    let rows = rows_of(ch)?;
    let mut out = [[false; GW]; GH];
    for (y, row) in out.iter_mut().enumerate() {
        for (x, bit) in row.iter_mut().enumerate() {
            *bit = glyph_bit(rows, x, y);
        }
    }
    Some(out)
}

/// The glyph a character is drawn with: `GLYPHS.get(ch.upper(), GLYPHS[' '])`. Python's `upper()` may give
/// several characters (the German sharp s, U+00DF, becomes `SS`); those, like every character the font does not
/// have, are a space.
fn drawn_rows(ch: char) -> &'static str {
    let mut upper = ch.to_uppercase();
    let found = match (upper.next(), upper.next()) {
        (Some(u), None) => rows_of(u),
        _ => None,
    };
    found.unwrap_or_else(|| rows_of(' ').expect("the font has a space"))
}

/// `text_mask(s, scale)`: the text as a mask, 7 font pixels high and `len(s) * ADV - 1` wide (1 for an empty
/// string), every font pixel a `scale x scale` block when `scale > 1`. A character counts as one Python character
/// (a Unicode code point).
pub fn text_mask(s: &str, scale: u32) -> Img<u8> {
    let n = s.chars().count();
    let w = (n * ADV as usize).saturating_sub(1).max(1);
    let k = scale.max(1) as usize;
    let mut m: Img<u8> = Img::new(w * k, GH * k, 1);
    for (i, ch) in s.chars().enumerate() {
        let rows = drawn_rows(ch);
        for gy in 0..GH {
            for gx in 0..GW {
                if glyph_bit(rows, gx, gy) {
                    let (x0, y0) = ((i * ADV as usize + gx) * k, gy * k);
                    for y in y0..y0 + k {
                        m.row_mut(y)[x0..x0 + k].fill(1);
                    }
                }
            }
        }
    }
    m
}

/// `dilate(m, r)`: the mask grown by `r` pixels in every direction (a square brush), on a canvas `2 r` wider and
/// higher: the pixel `(x, y)` of `m` lands on `(x + r, y + r)`. The Python default is `r = 1`.
pub fn dilate(m: &Img<u8>, r: u32) -> Img<u8> {
    assert_eq!(
        m.c, 1,
        "dilate wants a one-channel mask, this image has {} channels",
        m.c
    );
    let r = r as usize;
    let mut out: Img<u8> = Img::new(m.w + 2 * r, m.h + 2 * r, 1);
    for y in 0..m.h {
        for (x, &v) in m.row(y).iter().enumerate() {
            if v != 0 {
                // out[r + dy + y, r + dx + x] for dy, dx in -r..=r
                for oy in y..=y + 2 * r {
                    out.row_mut(oy)[x..=x + 2 * r].fill(1);
                }
            }
        }
    }
    out
}

/// One layer of [`stamp`]: the set pixels of `m`, placed with its top left at `(mx, my)`, take `col` (mixed into
/// the frame by `alpha` when `alpha < 0.999`). Pixels outside the frame are dropped.
fn stamp_layer(frame: &mut Rgb8, m: &Img<u8>, mx: i32, my: i32, col: [u8; 3], alpha: f64) {
    let (fw, fh) = (frame.w as i64, frame.h as i64);
    let (mx, my) = (i64::from(mx), i64::from(my));
    let (x0, y0) = (mx.max(0), my.max(0));
    let (x1, y1) = (fw.min(mx + m.w as i64), fh.min(my + m.h as i64));
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    for y in y0..y1 {
        let mrow = m.row((y - my) as usize);
        for x in x0..x1 {
            if mrow[(x - mx) as usize] == 0 {
                continue;
            }
            let px = frame.px_mut(x as usize, y as usize);
            if alpha >= 0.999 {
                px.copy_from_slice(&col);
            } else {
                // (region[sub] * (1 - alpha) + np.array(col) * alpha).astype(np.uint8)
                for (p, &c) in px.iter_mut().zip(&col) {
                    *p = np::f64_to_u8(f64::from(*p) * (1.0 - alpha) + f64::from(c) * alpha);
                }
            }
        }
    }
}

/// `stamp(frame, mask, x, y, color, outline=(10, 20, 26), shadow=True, alpha=1.0)`: draws the mask with its top
/// left at `(x, y)` in `color`, over a 1 px outline (the mask dilated by 1) and, with `shadow`, the same outline
/// once more two rows lower as a drop shadow. `outline` `None` draws the text alone (`shadow` then does nothing).
/// With `alpha` below 0.999 every layer is mixed into the frame instead of replacing it. The Python defaults are
/// `Some(`[`OUTLINE`]`)`, `true` and `1.0`.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn stamp(
    frame: &mut Rgb8,
    mask: &Img<u8>,
    x: i32,
    y: i32,
    color: [u8; 3],
    outline: Option<[u8; 3]>,
    shadow: bool,
    alpha: f64,
) {
    assert_eq!(
        frame.c, 3,
        "stamp wants an RGB frame, this one has {} channels",
        frame.c
    );
    assert_eq!(
        mask.c, 1,
        "stamp wants a one-channel mask, this one has {} channels",
        mask.c
    );
    if let Some(outline) = outline {
        let o = dilate(mask, 1);
        if shadow {
            stamp_layer(frame, &o, x - 1, y + 1, outline, alpha);
        }
        stamp_layer(frame, &o, x - 1, y - 1, outline, alpha);
    }
    stamp_layer(frame, mask, x, y, color, alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(m: &Img<u8>) -> Vec<String> {
        (0..m.h)
            .map(|y| m.row(y).iter().map(|&v| if v != 0 { '#' } else { '.' }).collect())
            .collect()
    }

    #[test]
    fn the_table_is_well_formed() {
        for (ch, glyph_rows) in GLYPHS {
            let parts: Vec<&str> = glyph_rows.split('|').collect();
            assert_eq!(parts.len(), GH, "{ch:?}");
            assert!(
                parts
                    .iter()
                    .all(|p| p.len() == GW && p.bytes().all(|b| b == b'#' || b == b'.')),
                "{ch:?}"
            );
        }
        let chars = glyph_chars();
        assert_eq!(chars.chars().count(), 58);
        let mut sorted: Vec<char> = chars.chars().collect();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 58, "a character appears twice");
        assert!(glyph('a').is_none() && glyph('A').is_some());
    }

    #[test]
    fn text_is_upper_cased_and_spaced() {
        let m = text_mask("hi", 1);
        assert_eq!((m.w, m.h, m.c), (11, 7, 1));
        assert_eq!(
            rows(&m),
            [
                "#...#.#####",
                "#...#...#..",
                "#...#...#..",
                "#####...#..",
                "#...#...#..",
                "#...#...#..",
                "#...#.#####"
            ]
        );
        assert_eq!(m, text_mask("HI", 1));
        assert_eq!(m, text_mask("HI", 0));
        let big = text_mask("hi", 3);
        assert_eq!((big.w, big.h), (33, 21));
        assert_eq!(big.px(14, 2), &[1]);
        assert_eq!(big.px(15, 2), &[0]);
    }

    #[test]
    fn unknown_characters_are_spaces() {
        let blank = text_mask("   ", 1);
        assert!(blank.data.iter().all(|&v| v == 0));
        assert_eq!(text_mask("@\u{df}\u{1f600}", 1), blank);
        // Dotless i and long s have the upper cases I and S.
        assert_eq!(text_mask("\u{131}\u{17f}", 1), text_mask("IS", 1));
        let empty = text_mask("", 2);
        assert_eq!((empty.w, empty.h), (2, 14));
    }

    #[test]
    fn dilation_grows_a_square() {
        let dot = Img::from_vec(1, 1, 1, vec![1u8]);
        assert_eq!(rows(&dilate(&dot, 1)), ["###", "###", "###"]);
        assert_eq!(dilate(&dot, 0), dot);
        let two = Img::from_vec(3, 1, 1, vec![1u8, 0, 0]);
        assert_eq!(rows(&dilate(&two, 1)), ["###..", "###..", "###.."]);
    }

    #[test]
    fn stamp_layers_shadow_outline_text() {
        let dot = Img::from_vec(1, 1, 1, vec![1u8]);
        let mut frame: Rgb8 = Img::new(5, 7, 3);
        stamp(&mut frame, &dot, 2, 2, [200, 0, 0], Some([0, 9, 0]), true, 1.0);
        let col = |y: usize| -> Vec<u8> { (0..3).map(|k| frame.px(2, y)[k]).collect() };
        // Outline rows 1..3, shadow rows 3..5 (two lower), text at row 2.
        assert_eq!(col(0), [0, 0, 0]);
        assert_eq!(col(1), [0, 9, 0]);
        assert_eq!(col(2), [200, 0, 0]);
        assert_eq!(col(3), [0, 9, 0]);
        assert_eq!(col(5), [0, 9, 0]);
        assert_eq!(col(6), [0, 0, 0]);
        let mut plain: Rgb8 = Img::new(5, 7, 3);
        stamp(&mut plain, &dot, 2, 2, [200, 0, 0], None, true, 1.0);
        assert_eq!(plain.data.iter().filter(|&&v| v != 0).count(), 1);
    }

    #[test]
    fn stamp_mixes_below_full_alpha_and_clips() {
        let dot = Img::from_vec(1, 1, 1, vec![1u8]);
        let mut frame: Rgb8 = Img::filled(2, 2, &[100, 100, 100]);
        stamp(&mut frame, &dot, 0, 0, [200, 0, 51], None, false, 0.5);
        assert_eq!(frame.px(0, 0), &[150, 50, 75]);
        stamp(&mut frame, &dot, -1, 0, [1, 2, 3], None, false, 1.0);
        stamp(&mut frame, &dot, 2, 5, [1, 2, 3], None, false, 1.0);
        assert_eq!(frame.px(1, 1), &[100, 100, 100]);
        // 0.999 and above replaces.
        stamp(&mut frame, &dot, 1, 1, [7, 8, 9], None, false, 0.999);
        assert_eq!(frame.px(1, 1), &[7, 8, 9]);
    }
}
