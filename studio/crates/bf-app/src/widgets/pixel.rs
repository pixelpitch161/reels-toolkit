//! Pixel text (interface reference, sections 4 and 5.1): a fighter's name, or "VS", in the series' 5 x 7 pixel
//! letters, painted from its `PixelMask` one square per lit pixel. It is drawn crisp: a whole-number scale, and every
//! square's edges on whole physical pixels, so no pixel is blended at any zoom.

use egui::accesskit::Role;
use egui::{Color32, Mesh, Rect, Response, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::model::roster::PixelMask;

/// The largest scale, at most `max_scale`, at which a mask `mask_w` pixels wide fits in `avail_w` points; at least 1
/// (a name that does not fit even at 1 is drawn at 1). A `max_scale` of 0 counts as 1; an `avail_w` that is not a
/// number fits nothing but scale 1.
pub fn fit_scale(mask_w: usize, max_scale: u32, avail_w: f32) -> u32 {
    (1..=max_scale.max(1))
        .rev()
        .find(|&scale| (mask_w as f64) * f64::from(scale) <= f64::from(avail_w))
        .unwrap_or(1)
}

/// `mask` painted in `color` at `scale` points per mask pixel, named `name` for the accessibility tree (an image).
///
/// It takes exactly `mask.w * scale` by `mask.h * scale` points and paints one square per lit pixel, each snapped to
/// whole physical pixels. A mask whose buffer does not hold `w * h` values is drawn from what it has: a missing value
/// is dark and values past `w * h` are ignored; an empty mask (or scale 0) takes no space and paints nothing.
pub fn pixel_text(ui: &mut Ui, mask: &PixelMask, scale: u32, color: Color32, name: &str) -> Response {
    let s = scale as f32;
    let size = vec2(mask.w as f32 * s, mask.h as f32 * s);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Image, true, name));
    if scale == 0 || !ui.is_rect_visible(rect) {
        return response;
    }
    let ppp = ui.ctx().pixels_per_point();
    let snap = |v: f32| (v * ppp).round() / ppp;
    let visible = ui.clip_rect();
    let mut mesh = Mesh::default();
    for (i, _) in mask
        .on
        .iter()
        .enumerate()
        .take(mask.w.saturating_mul(mask.h))
        .filter(|(_, on)| **on)
    {
        let (x, y) = ((i % mask.w) as f32, (i / mask.w) as f32);
        let square = Rect::from_min_max(
            pos2(snap(rect.min.x + x * s), snap(rect.min.y + y * s)),
            pos2(snap(rect.min.x + (x + 1.0) * s), snap(rect.min.y + (y + 1.0) * s)),
        );
        if square.intersects(visible) {
            mesh.add_colored_rect(square, color);
        }
    }
    ui.painter().add(mesh);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_largest_scale_that_fits() {
        assert_eq!(fit_scale(29, 4, 172.0), 4, "FLUTE is 29 wide");
        assert_eq!(fit_scale(47, 4, 172.0), 3, "TROMBONE is 47 wide");
        assert_eq!(fit_scale(47, 4, 20.0), 1);
    }

    #[test]
    fn fit_scale_is_at_least_1_whatever_it_is_given() {
        assert_eq!(fit_scale(29, 0, 172.0), 1, "a max scale of 0");
        assert_eq!(fit_scale(29, 4, f32::NAN), 1);
        assert_eq!(fit_scale(29, 4, -5.0), 1);
        assert_eq!(fit_scale(0, 4, 0.0), 4, "an empty mask fits at any scale");
        assert_eq!(fit_scale(usize::MAX, 4, f32::INFINITY), 4);
        assert_eq!(fit_scale(29, 4, 116.0), 4, "exactly the width fits");
    }
}
