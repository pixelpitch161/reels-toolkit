//! HORN's panel on the VS card: `title.py` `_horn_coils`, tubing coiled round the panel's big ball (it reads the
//! card's balls).
//!
//! Rings of tubing every 16 px round the big ball of the pixel's own panel: the inner 22 % of each ring is lit
//! (x 1.14), its outer 12 % shaded (x 0.86). Above the divider (`SIDE > 0`) the rings are the left fighter's ball's,
//! below it the right fighter's, so the pattern works on either side.

use bf_core::py;

use crate::config::W;
use crate::img::RgbF;
use crate::np;
use crate::title::layout::{self, Layout};

/// Whether [`pattern`] is the kit's `_horn_coils`.
pub const PORTED: bool = true;

/// `_horn_coils(base)`: the panel's `(H, W, 3)` `float64` gradient, changed in place; `card` is the card's layout
/// (where the Python reads `title.BALLS`).
pub fn pattern(base: &mut RgbF, card: &Layout) {
    let side = layout::side();
    for (i, (px, &s)) in base.data.as_chunks_mut::<3>().0.iter_mut().zip(&side.data).enumerate() {
        let (xx, yy) = ((i % W) as f64, (i / W) as f64);
        // b = np.where(SIDE > 0, 0, 1); cx, cy = BALLS[b]['cx'], BALLS[b]['cy']
        let ball = &card.balls[if s > 0.0 { 0 } else { 1 }];
        // q = (np.hypot(XX - cx, YY - cy) / 16.0) % 1
        let q = py::fmod_py(np::hypot(xx - ball.cx, yy - ball.cy) / 16.0, 1.0);
        let k = if q < 0.22 {
            1.14
        } else if q > 0.88 {
            0.86
        } else {
            1.0
        };
        for v in px.iter_mut() {
            *v *= k;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::H;
    use crate::img::Img;

    /// The rings are centred on each panel's own ball: lit just outside every 16 px from its centre, shaded just
    /// inside, plain between.
    #[test]
    fn the_coils_ring_each_panels_ball() {
        let card = Layout::plain("HORN", "FLUTE");
        let mut base: RgbF = Img::new(W, H, 3);
        base.data.fill(100.0);
        pattern(&mut base, &card);
        let side = layout::side();
        for (k, ball) in card.balls.iter().enumerate() {
            let (cx, cy) = (ball.cx as usize, ball.cy as usize);
            assert_eq!(side.px(cx, cy)[0] > 0.0, k == 0, "ball {k} lies on its own panel");
            // 32 px right of the centre: q = 0 (lit); 31 px: q = 0.9375 (shaded); 40 px: q = 0.5 (plain)
            assert_eq!(base.px(cx + 32, cy)[0], 100.0 * 1.14, "ball {k}");
            assert_eq!(base.px(cx + 31, cy)[0], 100.0 * 0.86, "ball {k}");
            assert_eq!(base.px(cx + 40, cy)[0], 100.0, "ball {k}");
        }
    }
}
