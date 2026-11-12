//! The glass balls: `sprite` and `draw_ball` of `kit/picture.py`.
//!
//! The picture here is the frame's `float32` array, and numpy's dtype rules decide the arithmetic: a `float32`
//! sample times a Python float is a `float32` product (the float is rounded to `float32` first), a `float64`
//! term widens the sum, and storing into the picture rounds back to `float32`.

use std::sync::Arc;

use bf_core::dmath;
use bf_core::py::py_max;

use super::{PictureCtx, bayer};
use crate::camera::Cam;
use crate::config::{H, W};
use crate::img::Img;
use crate::np;
use crate::scene::{self, R_BALL as R};
use crate::sprite::{self, SpriteKey};
use crate::ui::round_i32;

use super::state::BallState;

impl PictureCtx {
    /// `sprite(k, radius_px, yaw_deg, roll)`: fighter `k`'s instrument for a ball: its look's own sprite, or its
    /// model's, at the radius rounded and held to 12..30 and the yaw rounded to a multiple of 6 degrees. Drawn
    /// once and kept.
    ///
    /// The cache's key is what the sprite is drawn from: the fighter, the radius, the yaw step as it is and the
    /// roll to the bit. Python's key is `<model>_<r>_<yaw % 360>_<int(roll)>` while it draws with `float(yk)`
    /// and `float(roll)`: two rolls with the same whole part (30.2 and 30.9) share one sprite there, the first
    /// asked, and so do yaw steps a full turn apart. Here each is its own sprite, so a frame does not depend on
    /// which was drawn first. The kit's rolls are whole numbers (the view's, and a look's debuff), for which the
    /// two agree.
    pub fn sprite(&self, k: usize, radius_px: f64, yaw_deg: f64, roll: f64) -> Arc<Img<u8>> {
        let key = SpriteKey::for_ball(k, radius_px, yaw_deg, roll);
        self.sprites.get_or_draw(key, || {
            self.looks[k]
                .sprite(key.radius, key.yaw_deg(), key.roll_deg())
                .unwrap_or_else(|| sprite::render_sprite(&self.meshes[k], key.radius, key.yaw_deg(), key.roll_deg()))
        })
    }

    /// `draw_ball(arr, dep, cam, k, st, t)`: fighter `k`'s glass ball with its instrument inside, into the
    /// `float32` picture and the depth buffer. Returns `(s, rp)`: its centre on the screen and its radius in
    /// pixels.
    pub(super) fn draw_ball(
        &self,
        arr: &mut Img<f32>,
        dep: &mut Img<f32>,
        cam: &Cam,
        k: usize,
        st: &BallState,
        t: f64,
    ) -> ((f64, f64), f64) {
        let sink = st.sink;
        let sunk = sink != 0.0;
        // pw = S.to_world(p[0], p[1], (R * (1 - 0.75 * sink) if sink else R) + st['lift'])
        let h = if sunk { R * (1.0 - 0.75 * sink) } else { R };
        let pw = scene::to_world(st.p[0], st.p[1], h + st.lift);
        let mut pc = cam.to_cam(&[pw])[0];
        let mut s = cam.proj(&[pc])[0];
        let mut rp = R * cam.f / pc[2] * st.scale;
        if sunk {
            rp *= 1.0 - 0.35 * sink;
        }
        // a look may squash or stretch it, about the bottom of the ball
        let sq = st.sx.is_some() || st.sy.is_some();
        let (sx_, sy_) = (st.sx.unwrap_or(1.0), st.sy.unwrap_or(1.0));
        let ri = if sq {
            s = (s.0, s.1 + rp * (1.0 - sy_));
            round_i32(rp * py_max(sx_, sy_))
        } else {
            round_i32(rp)
        };
        let (x0, y0) = (
            round_i32(s.0).wrapping_sub(ri).wrapping_sub(2),
            round_i32(s.1).wrapping_sub(ri).wrapping_sub(2),
        );
        let n = ri.saturating_mul(2).saturating_add(5);
        let (xa, ya) = (x0.max(0), y0.max(0));
        let (xb, yb) = (x0.saturating_add(n).min(W as i32), y0.saturating_add(n).min(H as i32));
        if xb <= xa || yb <= ya {
            return (s, rp);
        }
        let (rx, ry) = (if sq { rp * sx_ } else { rp }, if sq { rp * sy_ } else { rp });
        let col = self.col[k];
        let glow = st.glow;
        let (_, _, mut roll) = self.view[k];
        // its own glass, then the debuffs: a soured glass, a seasick tilt
        let mut glass = self.looks[k].glass();
        for l in &self.looks {
            (glass, roll) = l.ball(self, k, st, t, glass, roll);
        }
        let tone: [f64; 3] = std::array::from_fn(|c| col[c] * 0.30 + glass[c] * 0.70);
        // (1 - 2.0 / rp) ** 2 and the like: squares are products
        let rim_in = (1.0 - 2.0 / rp) * (1.0 - 2.0 / rp);
        let edge_out = (1.0 + 1.3 / rp) * (1.0 + 1.3 / rp);
        let spec_r = (2.0 / rp) * (2.0 / rp) + 0.014;

        // The masks of the part of the patch that is on the frame, row by row.
        struct Cell {
            x: usize,
            y: usize,
            dx: f64,
            dy: f64,
            d2: f64,
            band: f64,
        }
        let mut cells = Vec::with_capacity(((xb - xa) * (yb - ya)) as usize);
        for y in ya..yb {
            for x in xa..xb {
                // dx = (xx + x0 + 0.5 - s[0]) / rp, with xx + x0 the screen column
                let dx = (f64::from(x) + 0.5 - s.0) / rx;
                let dy = (f64::from(y) + 0.5 - s.1) / ry;
                let d2 = dx * dx + dy * dy;
                let nz = np::clip(1.0 - d2, 0.0, 1.0).sqrt();
                // lit from the spot above (the top of the ball)
                let lam = np::clip(-0.25 * dx - 0.55 * dy + nz * 0.6, 0.0, 1.0);
                // BAYER_FULL[:n, :n]: the dither starts at the patch's own corner
                let b = bayer((x - x0) as usize, (y - y0) as usize);
                let band = (lam * 4.0 + b * 0.9).floor() / 4.0;
                cells.push(Cell {
                    x: x as usize,
                    y: y as usize,
                    dx,
                    dy,
                    d2,
                    band,
                });
            }
        }
        // the glass: reg[m] = reg[m] * 0.12 + inner[sl][m] * 0.88
        for c in cells.iter().filter(|c| c.d2 <= 1.0) {
            let px = arr.px_mut(c.x, c.y);
            for ch in 0..3 {
                let mut inner = tone[ch] * (0.70 + 0.36 * c.band);
                if sunk {
                    inner *= 1.0 - 0.72 * sink;
                }
                px[ch] = (f64::from(px[ch] * 0.12f32) + inner * 0.88) as f32;
            }
        }
        // the instrument
        let (yc, sw, _) = self.view[k];
        let yaw = yc + sw * dmath::sin(1.25 * st.p[0] + 0.45 * st.p[1]);
        let over = self.looks[k].ball_sprite(self, k, st, t, rp, yaw);
        let spr = match &over {
            Some(o) => o.sprite.clone(),
            None => self.sprite(k, rp * self.looks[k].sprite_k(), yaw, roll),
        };
        // Python reads the sprite's height for both sides and its fourth channel as the alpha: a sprite that is not
        // square RGBA is a look's bug (numpy would cut it or raise). A debug build stops; a release build draws
        // the ball without it.
        debug_assert!(
            spr.c == 4 && spr.w == spr.h,
            "fighter {k}'s sprite is {} x {} with {} channels, not square RGBA",
            spr.w,
            spr.h,
            spr.c
        );
        if spr.c == 4 && spr.w == spr.h {
            let sn = spr.h as i32;
            let (sx0, sy0) = (round_i32(s.0).wrapping_sub(sn / 2), round_i32(s.1).wrapping_sub(sn / 2));
            let (ax0, ay0) = (sx0.max(0), sy0.max(0));
            let (ax1, ay1) = (
                sx0.saturating_add(sn).min(W as i32),
                sy0.saturating_add(sn).min(H as i32),
            );
            if ax1 > ax0 && ay1 > ay0 {
                let (w, h) = ((ax1 - ax0) as usize, (ay1 - ay0) as usize);
                // lit = sub[..., :3].astype(np.float32) * (1.06 + 0.3 * glow)
                let tint = over.as_ref().and_then(|o| o.tint.as_ref());
                // a numpy-scalar glow makes the whole of `lit` float64, the sink's factor too (ruling M3-R37); a
                // tint is written for a float32 picture, so under one the float64 product is rounded to float32 first
                let wide = st.glow_np && tint.is_none();
                let mut lit: Img<f32> = Img::new(w, h, 3);
                for yy in 0..h {
                    for xx in 0..w {
                        let src = spr.px((ax0 - sx0) as usize + xx, (ay0 - sy0) as usize + yy);
                        let dst = lit.px_mut(xx, yy);
                        for ch in 0..3 {
                            dst[ch] = if wide {
                                lit_wide(src[ch], glow, sink)
                            } else if st.glow_np {
                                lit_wide(src[ch], glow, 0.0)
                            } else {
                                lit_narrow(src[ch], glow)
                            };
                        }
                    }
                }
                if let Some(tint) = tint {
                    tint(&mut lit);
                }
                if sunk && !wide {
                    let dark = (1.0 - 0.72 * sink) as f32;
                    for v in &mut lit.data {
                        *v *= dark;
                    }
                }
                for yy in 0..h {
                    for xx in 0..w {
                        if spr.px((ax0 - sx0) as usize + xx, (ay0 - sy0) as usize + yy)[3] > 0 {
                            let dst = arr.px_mut(ax0 as usize + xx, ay0 as usize + yy);
                            for (d, &v) in dst.iter_mut().zip(lit.px(xx, yy)) {
                                *d = np::clip_f32(v, 0.0, 255.0);
                            }
                        }
                    }
                }
            }
        }
        // the rim, the dark edge and the highlight
        let rc: [f64; 3] = std::array::from_fn(|ch| {
            let v = col[ch] * (0.9 + 0.5 * glow) + 50.0 * glow;
            np::clip(if sunk { v * (1.0 - 0.5 * sink) } else { v }, 0.0, 255.0)
        });
        for c in cells.iter().filter(|c| c.d2 <= 1.0 && c.d2 > rim_in) {
            let px = arr.px_mut(c.x, c.y);
            for ch in 0..3 {
                px[ch] = rc[ch] as f32;
            }
        }
        let ink = [22.0, 14.0, 30.0];
        for c in cells.iter().filter(|c| c.d2 > 1.0 && c.d2 <= edge_out) {
            let px = arr.px_mut(c.x, c.y);
            for ch in 0..3 {
                px[ch] = (f64::from(px[ch] * 0.2f32) + ink[ch] * 0.8) as f32;
            }
        }
        // an airborne ball is drawn over what it hops past
        if st.air_bias != 0.0 {
            pc[2] -= st.air_bias;
        }
        for c in &cells {
            let inside = c.d2 <= 1.0;
            if inside && (c.dx + 0.38) * (c.dx + 0.38) + (c.dy + 0.5) * (c.dy + 0.5) < spec_r {
                arr.px_mut(c.x, c.y).copy_from_slice(&[255.0, 252.0, 244.0]);
            }
            if inside || (c.d2 > 1.0 && c.d2 <= edge_out) {
                dep.px_mut(c.x, c.y)[0] = pc[2] as f32;
            }
        }
        (s, rp)
    }
}

/// One sample of `lit = sub[..., :3].astype(np.float32) * (1.06 + 0.3 * glow)` for a Python-float `glow`: numpy keeps
/// the array's `float32`, so the gain is rounded to `float32` and the product is a `float32` one.
fn lit_narrow(src: u8, glow: f64) -> f32 {
    f32::from(src) * (1.06 + 0.3 * glow) as f32
}

/// The same sample for a `glow` that is a numpy `float64` scalar (`BallState::glow_np`, ruling M3-R37): the product
/// is `float64`, so is `lit * (1 - 0.72 * sink)` when the ball is sunk (`sink` not 0), and the value is rounded to
/// `float32` only when it is stored in the frame (`rg[ms] = np.clip(lit[ms], 0, 255)`, clipped again there).
fn lit_wide(src: u8, glow: f64, sink: f64) -> f32 {
    let mut v = f64::from(src) * (1.06 + 0.3 * glow);
    if sink != 0.0 {
        v *= 1.0 - 0.72 * sink;
    }
    v as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    // numpy 2.5.3 on the reference PC: `s = np.arange(256, dtype=np.float32)`; wide is
    // `np.float32((s * (1.06 + 0.3 * np.float64(glow)))[i])`, narrow `(s * float(1.06 + 0.3 * glow))[i]`, as bits;
    // sunk is the wide product times `(1 - 0.72 * 0.4)`; the last is how many of the 256 bytes agree at glow 0.3.
    const WIDE_50: u32 = 0x42540000;
    const NARROW_50: u32 = 0x4253ffff;
    const WIDE_186: u32 = 0x434b0000;
    const NARROW_186: u32 = 0x434affff;
    const WIDE_186_SUNK: u32 = 0x43108937;
    const SAME_AT_03: usize = 256;

    /// The byte the frame holds for a lit sample (`np.clip(arr, 0, 255).astype(np.uint8)`).
    fn byte(v: f32) -> u8 {
        np::clip_f32(v, 0.0, 255.0) as u8
    }

    /// Ruling M3-R37: with a numpy glow of 0.0 (the frame a CLARINET jump lands on) the sprite byte 50 is 53 where the
    /// Python-float rule gives 52, and likewise 100, 150, 200; with a glow of 0.10465939151547463 the byte 186 is
    /// 203, not 202.
    #[test]
    fn a_numpy_glow_lights_the_sprite_in_float64() {
        for (src, narrow, wide) in [(50, 52, 53), (100, 105, 106), (150, 158, 159), (200, 211, 212)] {
            let got = (byte(lit_narrow(src, 0.0)), byte(lit_wide(src, 0.0, 0.0)));
            assert_eq!(got, (narrow, wide), "{src}");
        }
        let g = 0.10465939151547463;
        assert_eq!((byte(lit_narrow(186, g)), byte(lit_wide(186, g, 0.0))), (202, 203));
        // the float32 values themselves, as numpy stores them
        assert_eq!(lit_wide(50, 0.0, 0.0).to_bits(), WIDE_50);
        assert_eq!(lit_narrow(50, 0.0).to_bits(), NARROW_50);
        assert_eq!(lit_wide(186, g, 0.0).to_bits(), WIDE_186);
        assert_eq!(lit_narrow(186, g).to_bits(), NARROW_186);
        assert_eq!(lit_wide(186, g, 0.4).to_bits(), WIDE_186_SUNK);
        // the two rules differ only next to a whole number
        let same = (0..=255u8)
            .filter(|&s| byte(lit_narrow(s, 0.3)) == byte(lit_wide(s, 0.3, 0.0)))
            .count();
        assert_eq!(same, SAME_AT_03);
    }
}
