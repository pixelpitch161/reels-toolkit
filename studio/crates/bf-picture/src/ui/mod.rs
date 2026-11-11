//! `reelkit/ui.py`: the on-screen text of a fight video, drawn straight into the finished `uint8` frame: the
//! impact captions with their sand puff ([`captions`]), the two fighters' HP bars and ability rows ([`hud`]),
//! small labels and panels ([`label`]), and the [`DrawLog`] every one of them files its box in.
//!
//! Ported: `draw_captions` in the `impact` style with `_draw_impact`, `_line_ink`, `_resample`, `_stamp_rgb`,
//! `_sand_puff` and `_hash01`; `impact_landings`; `caption_scale`; `label`; `panel`; `fight_hud` with
//! `_ability_rows`; `_note` and `reset_drawn`. Not ported, because the fight never calls them: the `type` caption
//! style (and `font.rich_line` with it), `slide_offset`, `share_bar`, `race_lanes` and `roster_grid`.
//!
//! Every picture these functions draw equals Python's byte for byte on the cases of
//! `studio/reference/picture_kit/ui_*.npz` (`tests/kit_ui_vectors.rs`).
//!
//! # The draw log
//!
//! Python keeps one module-level list, `reelkit.ui.DRAWN`, that every draw call appends to and the safe-zone
//! check empties before a frame and reads after it. Here the list is a value, [`DrawLog`], that the caller makes
//! for a frame and hands to each draw call, so frames drawn at the same time on several threads never share one.
//!
//! ```
//! use bf_picture::img::Img;
//! use bf_picture::ui::{self, DrawLog, Kind};
//!
//! let mut frame: Img<u8> = Img::filled(360, 640, &[20, 20, 30]);
//! let mut log = DrawLog::new();
//! // label(img, 'K.O.', 150, 300, (255, 244, 224), 1)
//! let width = ui::label(&mut frame, &mut log, "K.O.", 150, 300, [255, 244, 224], 1, Some([10, 20, 26]));
//! assert_eq!(width, 23);
//! assert_eq!(log.boxes.len(), 1);
//! assert_eq!((log.boxes[0].x0, log.boxes[0].y1, log.boxes[0].kind), (150, 307, Kind::Label));
//! ```
//!
//! # Indices
//!
//! The HUD paints with numpy slices such as `img[y0:y1, x0:x1] = colour`, and a slice or an index that leaves the
//! picture does not fail in Python: a slice is cut to the picture, and a negative bound or index counts from the
//! far edge. The port does the same ([`py_slice`], [`py_index`]), so a HUD placed partly outside the picture is
//! drawn as Python draws it, odd as that is; only an index at or beyond the far edge, where numpy raises, panics.

pub mod captions;
pub mod hud;
pub mod label;

pub use self::captions::{Caption, CaptionStyle, Landing, caption_scale, draw_captions, impact_landings};
pub use self::hud::{AbilityRow, Fighter, Hud, HudOptions, fight_hud};
pub use self::label::{label, panel};

use crate::img::Rgb8;
use crate::np;
use std::ops::Range;

/// `CREAM`: the captions' base colour.
pub const CREAM: [u8; 3] = [252, 240, 214];
/// `AMBER`: the captions' highlight (`*word*`), and the gold of the winner's frame.
pub const AMBER: [u8; 3] = [255, 198, 92];
/// `SKYB`: the captions' second highlight (`~word~`).
pub const SKYB: [u8; 3] = [140, 200, 255];
/// `MINT`.
pub const MINT: [u8; 3] = [124, 246, 200];
/// `CORAL`.
pub const CORAL: [u8; 3] = [255, 118, 96];
/// `PINK`.
pub const PINK: [u8; 3] = [255, 120, 200];
/// `CYAN`.
pub const CYAN: [u8; 3] = [110, 240, 255];
/// `INK`: the dark ink of the HUD's frames and backing.
pub const INK: [u8; 3] = [12, 16, 30];

/// What a noted box is: the `kind` string of `reelkit.ui._note`. The safe-zone check wants captions inside the
/// safe box always, lets labels and panels that are partly off the frame pass as sliding in or out, and reports a
/// caption that overlaps a label or a panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// `'caption'`: a caption line (also the count-in and the title card's text).
    Caption,
    /// `'label'`: a small text.
    Label,
    /// `'panel'`: a box, a bar, a pill.
    Panel,
}

impl Kind {
    /// The Python string: `caption`, `label` or `panel`.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Caption => "caption",
            Kind::Label => "label",
            Kind::Panel => "panel",
        }
    }
}

/// One entry of `reelkit.ui.DRAWN`: `(x0, y0, x1, y1, kind, info)`, the box of something drawn (`x1` and `y1`
/// are one past its last pixel) and a word about it (a caption's or label's text, a panel's name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Noted {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub kind: Kind,
    pub info: String,
}

/// `reelkit.ui.DRAWN`: every caption line, label and panel drawn since the log was made or cleared, in drawing
/// order. The boxes are what was asked for, not what fitted: a label partly off the frame keeps its whole box.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DrawLog {
    pub boxes: Vec<Noted>,
}

impl DrawLog {
    /// An empty log.
    pub fn new() -> Self {
        DrawLog::default()
    }

    /// `reset_drawn()`: forgets every box.
    pub fn clear(&mut self) {
        self.boxes.clear();
    }

    /// `_note(x0, y0, x1, y1, kind, info)`: files a box. Python takes `int()` of each coordinate; a caller with
    /// float coordinates truncates them toward zero (`as i32`) the same way.
    pub fn note(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, kind: Kind, info: &str) {
        self.boxes.push(Noted {
            x0,
            y0,
            x1,
            y1,
            kind,
            info: info.to_string(),
        });
    }
}

/// Python's `min(a, b)` on floats: `a` unless `b` is smaller.
pub(crate) fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// Python's `max(a, b)` on floats: `a` unless `b` is greater.
pub(crate) fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// Python's `int(round(x))` where the result is a pixel coordinate: the nearest whole number, ties to even.
pub(crate) fn round_i32(x: f64) -> i32 {
    np::f64_to_i64(bf_core::py::round_half_even(x)).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// The indices Python's `a[lo:hi]` selects on an axis of `n` elements: a negative bound counts from the end, and
/// both bounds are then cut to the axis. Empty when the bounds cross.
pub(crate) fn py_slice(lo: i32, hi: i32, n: usize) -> Range<usize> {
    let n = n as i64;
    let bound = |b: i32| {
        let b = i64::from(b);
        (if b < 0 { b + n } else { b }).clamp(0, n) as usize
    };
    let (lo, hi) = (bound(lo), bound(hi));
    lo..hi.max(lo)
}

/// The element Python's `a[i]` reads on an axis of `n` elements: a negative index counts from the end. Panics
/// when the index is outside the axis, where numpy raises `IndexError`.
pub(crate) fn py_index(i: i32, n: usize) -> usize {
    let wrapped = if i < 0 { i64::from(i) + n as i64 } else { i64::from(i) };
    assert!(
        (0..n as i64).contains(&wrapped),
        "index {i} is out of bounds for an axis of {n} pixels"
    );
    wrapped as usize
}

/// `img[ys.0:ys.1, xs.0:xs.1] = col`: a filled rectangle, as numpy's slice assignment paints it.
pub(crate) fn fill(img: &mut Rgb8, ys: (i32, i32), xs: (i32, i32), col: [u8; 3]) {
    let cols = py_slice(xs.0, xs.1, img.w);
    for y in py_slice(ys.0, ys.1, img.h) {
        for x in cols.clone() {
            img.px_mut(x, y).copy_from_slice(&col);
        }
    }
}

/// `np.clip(c, 0, 255).astype(np.uint8)` for a colour.
pub(crate) fn clip_rgb(c: [f64; 3]) -> [u8; 3] {
    c.map(np::clip_u8)
}

/// `img[:] = (before * (1 - k) + img * k).astype(np.uint8)`: the whole picture mixed with an earlier copy of
/// itself, every pixel (also the ones that did not change: `v * (1 - k) + v * k` is not always `v`).
pub(crate) fn mix_over(img: &mut Rgb8, before: &Rgb8, k: f64) {
    let back = 1.0 - k;
    for (p, &b) in img.data.iter_mut().zip(&before.data) {
        *p = np::f64_to_u8(f64::from(b) * back + f64::from(*p) * k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::img::Img;

    #[test]
    fn the_log_keeps_boxes_in_order_and_clears() {
        let mut log = DrawLog::new();
        log.note(1, 2, 3, 4, Kind::Panel, "fight hud");
        log.note(-5, 0, 9, 7, Kind::Caption, "K.O.");
        assert_eq!(log.boxes.len(), 2);
        assert_eq!(
            log.boxes[1],
            Noted {
                x0: -5,
                y0: 0,
                x1: 9,
                y1: 7,
                kind: Kind::Caption,
                info: "K.O.".into()
            }
        );
        assert_eq!(
            [Kind::Caption.as_str(), Kind::Label.as_str(), Kind::Panel.as_str()],
            ["caption", "label", "panel"]
        );
        log.clear();
        assert!(log.boxes.is_empty());
    }

    #[test]
    fn slices_count_from_the_end_and_are_cut() {
        assert_eq!(py_slice(2, 5, 10), 2..5);
        assert_eq!(py_slice(-3, 20, 10), 7..10);
        assert_eq!(py_slice(-30, 4, 10), 0..4);
        assert_eq!(py_slice(4, -2, 10), 4..8);
        assert!(py_slice(6, 3, 10).is_empty());
        assert!(py_slice(12, 15, 10).is_empty());
        assert!(py_slice(-2, 3, 10).is_empty());
        assert_eq!((py_index(3, 10), py_index(-1, 10), py_index(-10, 10)), (3, 9, 0));
    }

    #[test]
    #[should_panic(expected = "index 10 is out of bounds")]
    fn an_index_past_the_axis_panics() {
        py_index(10, 10);
    }

    #[test]
    fn python_min_max_and_round() {
        assert_eq!((py_min(1.0, 2.0), py_min(2.0, 1.0), py_max(1.0, 2.0)), (1.0, 1.0, 2.0));
        // A NaN second argument never wins, a NaN first argument stays.
        assert_eq!(py_max(0.0, f64::NAN), 0.0);
        assert!(py_min(f64::NAN, 1.0).is_nan());
        assert!(py_max(0.0, -0.0).is_sign_positive());
        assert_eq!(
            (round_i32(0.5), round_i32(1.5), round_i32(2.5), round_i32(-0.5)),
            (0, 2, 2, 0)
        );
        assert_eq!(round_i32(-1.5), -2);
    }

    #[test]
    fn fill_and_mix() {
        let mut img: Rgb8 = Img::filled(4, 3, &[100, 100, 100]);
        fill(&mut img, (1, 9), (-2, 4), [1, 2, 3]);
        assert_eq!(img.px(1, 1), &[100, 100, 100]);
        assert_eq!(img.px(2, 1), &[1, 2, 3]);
        assert_eq!(img.px(3, 2), &[1, 2, 3]);
        assert_eq!(img.px(3, 0), &[100, 100, 100]);
        let before: Rgb8 = Img::filled(4, 3, &[200, 200, 200]);
        mix_over(&mut img, &before, 0.25);
        // 200 * 0.75 + 100 * 0.25 = 175; 200 * 0.75 + 1 * 0.25 = 150.25
        assert_eq!(img.px(0, 0), &[175, 175, 175]);
        assert_eq!(img.px(2, 1), &[150, 150, 150]);
        assert_eq!(clip_rgb([-3.0, 255.9, 300.0]), [0, 255, 255]);
    }
}
