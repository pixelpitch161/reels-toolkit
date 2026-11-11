//! `label` and `panel` of `reelkit/ui.py`: a small outlined text, and a HUD box with a drop shadow.

use super::{DrawLog, Kind};
use crate::font;
use crate::img::Rgb8;
use crate::pil;

/// `panel`'s default `fill`, `(16, 20, 38)`.
pub const PANEL_FILL: [u8; 3] = [16, 20, 38];
/// `panel`'s default `edge`, `(62, 74, 112)`.
pub const PANEL_EDGE: [u8; 3] = [62, 74, 112];

/// `label(img, text, x, y, col, scale=2, outline=(10, 20, 26))`: the text in the pixel font with its top left at
/// `(x, y)`, over a 1 px outline and a drop shadow ([`font::stamp`]), noted as a label. Returns the text's width in
/// pixels (the outline is one pixel more on every side). The Python defaults are `scale` 2 and
/// `Some(`[`font::OUTLINE`]`)`; `outline` `None` draws the letters alone.
///
/// The noted box is the text's own, `(x, y, x + width, y + 7 * scale)`, wherever it lies: text partly or wholly
/// off the picture is cut by the picture's edge and still noted whole.
#[allow(clippy::too_many_arguments)] // the Python signature, and the log
pub fn label(
    img: &mut Rgb8,
    log: &mut DrawLog,
    text: &str,
    x: i32,
    y: i32,
    col: [u8; 3],
    scale: u32,
    outline: Option<[u8; 3]>,
) -> i32 {
    let m = font::text_mask(text, scale);
    font::stamp(img, &m, x, y, col, outline, true, 1.0);
    let (w, h) = (m.w as i32, m.h as i32);
    log.note(x, y, x + w, y + h, Kind::Label, text);
    w
}

/// `panel(draw, x0, y0, x1, y1, fill=(16, 20, 38), edge=(62, 74, 112))`: a HUD box on the pixels `x0..=x1`,
/// `y0..=y1` (both corners included, as Pillow's rectangle has them): a drop shadow two pixels down and right, the
/// fill, a 1 px edge and a lighter line under the top edge. Python draws through `ImageDraw.Draw(image)`; here the
/// picture is named. The Python defaults are [`PANEL_FILL`] and [`PANEL_EDGE`].
///
/// The noted box is `(x0, y0, x1 + 3, y1 + 3)`: the box with its shadow. Panics if `x1 < x0` or `y1 < y0`, as
/// Pillow raises.
#[allow(clippy::too_many_arguments)] // the Python signature, and the log
pub fn panel(img: &mut Rgb8, log: &mut DrawLog, x0: i32, y0: i32, x1: i32, y1: i32, fill: [u8; 3], edge: [u8; 3]) {
    let f = f64::from;
    // draw.rectangle([x0 + 2, y0 + 2, x1 + 2, y1 + 2], fill=(6, 8, 18))
    pil::rectangle(
        img,
        (f(x0 + 2), f(y0 + 2), f(x1 + 2), f(y1 + 2)),
        Some(&[6, 8, 18]),
        None,
    );
    // draw.rectangle([x0, y0, x1, y1], fill=fill); draw.rectangle([x0, y0, x1, y1], outline=edge)
    let bbox = (f(x0), f(y0), f(x1), f(y1));
    pil::rectangle(img, bbox, Some(&fill), None);
    pil::rectangle(img, bbox, None, Some(&edge));
    // draw.line([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=(40, 50, 84))
    pil::line(img, &[(f(x0 + 1), f(y0 + 1)), (f(x1 - 1), f(y0 + 1))], &[40, 50, 84], 1);
    log.note(x0, y0, x1 + 3, y1 + 3, Kind::Panel, "");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::img::Img;

    #[test]
    fn a_label_is_stamped_and_noted_whole() {
        let mut img: Rgb8 = Img::filled(40, 20, &[90, 90, 90]);
        let mut log = DrawLog::new();
        let w = label(&mut img, &mut log, "hi", 30, 4, [200, 10, 20], 2, Some(font::OUTLINE));
        // Two characters at scale 2: (2 * 6 - 1) * 2 wide, 14 high; it runs off the right edge and is noted whole.
        assert_eq!(w, 22);
        assert_eq!(log.boxes.len(), 1);
        let b = &log.boxes[0];
        assert_eq!(
            (b.x0, b.y0, b.x1, b.y1, b.kind, b.info.as_str()),
            (30, 4, 52, 18, Kind::Label, "hi")
        );
        assert_eq!(img.px(30, 4), &[200, 10, 20]); // the H's left stem
        assert_eq!(img.px(29, 4), &font::OUTLINE); // its outline
        assert_eq!(img.px(32, 6), &font::OUTLINE); // inside the H, between the stems: outline
        assert_eq!(img.px(10, 10), &[90, 90, 90]);
    }

    #[test]
    fn a_panel_has_shadow_fill_edge_and_a_lit_line() {
        let mut img: Rgb8 = Img::filled(20, 14, &[90, 90, 90]);
        let mut log = DrawLog::new();
        panel(&mut img, &mut log, 3, 2, 12, 8, PANEL_FILL, PANEL_EDGE);
        assert_eq!(img.px(3, 2), &PANEL_EDGE);
        assert_eq!(img.px(12, 8), &PANEL_EDGE);
        assert_eq!(img.px(4, 3), &[40, 50, 84]); // the lit line under the top edge
        assert_eq!(img.px(11, 3), &[40, 50, 84]);
        assert_eq!(img.px(5, 5), &PANEL_FILL);
        assert_eq!(img.px(14, 10), &[6, 8, 18]); // the shadow's corner
        assert_eq!(img.px(15, 10), &[90, 90, 90]);
        let b = &log.boxes[0];
        assert_eq!(
            (b.x0, b.y0, b.x1, b.y1, b.kind, b.info.as_str()),
            (3, 2, 15, 11, Kind::Panel, "")
        );
    }
}
