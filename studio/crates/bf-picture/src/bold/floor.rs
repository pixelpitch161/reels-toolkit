//! Marks of light on the canvas for the bold effects. They are drawn in a look's canvas hooks
//! ([`crate::look::Look::canvas`], [`crate::look::Look::canvas_wave`]): `ic` is the light of each canvas pixel
//! and `c` says where each pixel is in the ring (`c.sxp`, `c.szp`: the ring frame, the ring is 6 units wide and
//! a ball's radius is 0.6). `tint` is the mark's colour as light ([`super::light`] of a colour) and `k` its
//! strength (about 0.5 for a quiet mark, 1 for a bold one). Every mark is cut into three dithered steps, as all
//! light on the canvas is.

use bf_core::dmath;

use super::px::bad;

use crate::camera::V3;
use crate::look::CanvasCtx;
use crate::ring::state::Wave;

/// The Bayer value of canvas pixel `j`.
fn bayer(c: &CanvasCtx<'_>, j: usize) -> f64 {
    crate::config::BAYER[c.ys[j] % 4][c.xs[j] % 4]
}

/// `v` (0 to 1) cut into three dithered steps at canvas pixel `j`.
fn steps(c: &CanvasCtx<'_>, j: usize, v: f64) -> f64 {
    (v.clamp(0.0, 1.0) * 3.0 + bayer(c, j) * 0.9).floor() / 3.0
}

/// Adds `v * tint` to a pixel's light.
fn lit(light: &mut V3, tint: V3, v: f64) {
    for ch in 0..3 {
        light[ch] += v * tint[ch];
    }
}

/// A ring of light of radius `r` round `(cx, cz)`, `half_w` wide on either side of the line.
#[allow(clippy::too_many_arguments)]
pub fn ring(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, half_w: f64, tint: V3, k: f64) {
    if bad(half_w) || k <= 0.0 {
        return;
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let d = dmath::hypot(c.sxp[j] - cx, c.szp[j] - cz);
        let band = 1.0 - (d - r).abs() / half_w;
        if band > 0.0 {
            lit(light, tint, steps(c, j, band) * k);
        }
    }
}

/// A filled disc of light of radius `r` round `(cx, cz)` with a soft rim.
pub fn disc(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, tint: V3, k: f64) {
    if bad(r) || k <= 0.0 {
        return;
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let d = dmath::hypot(c.sxp[j] - cx, c.szp[j] - cz);
        let v = (r - d) / (r * 0.35);
        if v > 0.0 {
            lit(light, tint, steps(c, j, v) * k);
        }
    }
}

/// A dark patch of radius `r` round `(cx, cz)`: the light there is multiplied down to `1 - k` in the middle.
pub fn shade(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, k: f64) {
    if bad(r) || k <= 0.0 {
        return;
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let d = dmath::hypot(c.sxp[j] - cx, c.szp[j] - cz);
        let v = (r - d) / (r * 0.5);
        if v > 0.0 {
            let m = 1.0 - k.min(1.0) * steps(c, j, v);
            for ch in light.iter_mut() {
                *ch *= m;
            }
        }
    }
}

/// How far the ring-frame point `(x, z)` is from the segment `a` to `b`, and how far along it (0 at `a`, 1 at
/// `b`) its nearest point lies.
fn to_segment(x: f64, z: f64, a: [f64; 2], b: [f64; 2]) -> (f64, f64) {
    let (vx, vz) = (b[0] - a[0], b[1] - a[1]);
    let len2 = vx * vx + vz * vz;
    let u = if len2 > 0.0 {
        (((x - a[0]) * vx + (z - a[1]) * vz) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (dmath::hypot(x - a[0] - u * vx, z - a[1] - u * vz), u)
}

/// A straight lane of light from `a` to `b`, `half_w` wide on either side: where a beam, a dash or a throw will
/// go.
pub fn lane(ic: &mut [V3], c: &CanvasCtx<'_>, a: [f64; 2], b: [f64; 2], half_w: f64, tint: V3, k: f64) {
    if bad(half_w) || k <= 0.0 {
        return;
    }
    for (j, light) in ic.iter_mut().enumerate() {
        let (d, _) = to_segment(c.sxp[j], c.szp[j], a, b);
        let v = 1.0 - d / half_w;
        if v > 0.0 {
            lit(light, tint, steps(c, j, v) * k);
        }
    }
}

/// A dashed lane: [`lane`] cut into dashes `dash` units long that run toward `b` as `phase` grows by 1 a dash.
#[allow(clippy::too_many_arguments)]
pub fn dashes(
    ic: &mut [V3],
    c: &CanvasCtx<'_>,
    a: [f64; 2],
    b: [f64; 2],
    half_w: f64,
    dash: f64,
    phase: f64,
    tint: V3,
    k: f64,
) {
    if bad(half_w) || bad(dash) || k <= 0.0 {
        return;
    }
    let len = dmath::hypot(b[0] - a[0], b[1] - a[1]);
    for (j, light) in ic.iter_mut().enumerate() {
        let (d, u) = to_segment(c.sxp[j], c.szp[j], a, b);
        let v = 1.0 - d / half_w;
        if v > 0.0 && ((u * len / dash - phase).rem_euclid(1.0)) < 0.55 {
            lit(light, tint, steps(c, j, v) * k);
        }
    }
}

/// S3, the wind-up warning on the floor: a landing mark at `(cx, cz)` for something that will hit a circle of
/// radius `r` there. `u` is how far the wind-up is (0 as the warning starts, 1 as the thing lands): an outer
/// ring closes in on the fixed inner ring, four ticks point at the middle, and the whole mark brightens.
pub fn target(ic: &mut [V3], c: &CanvasCtx<'_>, cx: f64, cz: f64, r: f64, u: f64, tint: V3) {
    if bad(r) || !(0.0..=1.0).contains(&u) {
        return;
    }
    let k = 0.45 + 0.55 * u;
    let outer = r * (2.2 - 1.2 * u);
    let w = (r * 0.12).max(0.05);
    for (j, light) in ic.iter_mut().enumerate() {
        let (dx, dz) = (c.sxp[j] - cx, c.szp[j] - cz);
        let d = dmath::hypot(dx, dz);
        if d > outer + w {
            continue;
        }
        let mut v = (1.0 - (d - r).abs() / w).max(1.0 - (d - outer).abs() / w);
        // four ticks across the inner ring
        if (d - r).abs() < r * 0.3 && dx.abs().min(dz.abs()) < w * 0.8 {
            v = v.max(0.8);
        }
        if v > 0.0 {
            lit(light, tint, steps(c, j, v) * k);
        }
    }
}

/// S5, the five kinds of ring. A fighter's expanding rings are drawn in its own family, so a brass blast, a
/// plucked string, a puff of air, a stomp and a burst of noise can be told apart at a glance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// Brass: three stacked gold bands.
    Brass,
    /// Strings: the ring wobbles like a plucked string.
    Strings,
    /// Air (woodwinds, bellows): a broken ring of soft puffs.
    Air,
    /// The floor (drums, stomps): a quake that lifts the canvas in square tiles.
    Quake,
    /// Noise (electric, synthetic, screeches): a jagged ring.
    Noise,
}

/// A deterministic 0 to 1 from two whole numbers.
fn hash01(a: i64, b: i64) -> f64 {
    let mut h = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (b as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h % 10_000) as f64 / 10_000.0
}

/// The band of one of the run's expanding rings `w` in its `family`, at time `t`: call it from
/// [`crate::look::Look::canvas_wave`]. The ring fades as it nears its largest radius. `tint` as everywhere here;
/// brass mixes its own gold into it.
pub fn wave(ic: &mut [V3], c: &CanvasCtx<'_>, family: Family, w: &Wave, t: f64, tint: V3, k: f64) {
    let rmax = if w.rmax > 0.0 { w.rmax } else { 1.0 };
    band(ic, c, family, w.cx, w.cz, w.r, 1.0 - w.r / rmax, t, tint, k);
}

/// A ring of `family` round `(cx, cz)` at radius `r`: [`wave`] for a ring that is not one of the run's. `fade`
/// is its strength from 1 (fresh) to 0 (gone).
#[allow(clippy::too_many_arguments)]
pub fn band(
    ic: &mut [V3],
    c: &CanvasCtx<'_>,
    family: Family,
    cx: f64,
    cz: f64,
    r: f64,
    fade: f64,
    t: f64,
    tint: V3,
    k: f64,
) {
    let fade = fade.clamp(0.0, 1.0);
    if bad(r) || fade <= 0.0 || k <= 0.0 {
        return;
    }
    let gold = [1.55, 1.2, 0.55];
    for (j, light) in ic.iter_mut().enumerate() {
        let (dx, dz) = (c.sxp[j] - cx, c.szp[j] - cz);
        let d = dmath::hypot(dx, dz);
        if (d - r).abs() > 0.9 {
            continue;
        }
        let ang = dmath::atan2(dz, dx);
        let (v, col) = match family {
            Family::Brass => {
                // three thin bands, the leading one brightest
                let mut v: f64 = 0.0;
                for (n, gain) in [(0.0, 1.0), (1.0, 0.7), (2.0, 0.45)] {
                    v = v.max((1.0 - (d - (r - 0.2 * n)).abs() / 0.07) * gain);
                }
                let mix: V3 = std::array::from_fn(|ch| tint[ch] * 0.45 + gold[ch] * 0.55);
                (v, mix)
            }
            Family::Strings => {
                let wob = 0.09 * dmath::sin(ang * 9.0 + t * 38.0) * (0.4 + 0.6 * fade);
                ((1.0 - (d - r - wob).abs() / 0.09), tint)
            }
            Family::Air => {
                // twelve soft puffs that drift round as the ring grows
                let lump = 0.5 + 0.5 * dmath::cos(ang * 12.0 + r * 2.0);
                (
                    (1.0 - (d - r).abs() / (0.14 + 0.16 * lump)) * (0.35 + 0.65 * lump) * 0.8,
                    tint,
                )
            }
            Family::Quake => {
                // square tiles: a tile is lit when its own middle is on the ring
                let cell = 0.34;
                let (tx, tz) = ((c.sxp[j] / cell).floor(), (c.szp[j] / cell).floor());
                let (mx, mz) = ((tx + 0.5) * cell - cx, (tz + 0.5) * cell - cz);
                let dm = dmath::hypot(mx, mz);
                let edge = ((c.sxp[j] / cell).rem_euclid(1.0) - 0.5)
                    .abs()
                    .max(((c.szp[j] / cell).rem_euclid(1.0) - 0.5).abs());
                let on = (dm - r).abs() < 0.24;
                (if on { if edge > 0.4 { 0.35 } else { 1.0 } } else { 0.0 }, tint)
            }
            Family::Noise => {
                let sector = (ang / dmath::TAU * 40.0).floor() as i64;
                let jag = (hash01(sector, (t * 15.0).floor() as i64) - 0.5) * 0.34;
                ((1.0 - (d - r - jag).abs() / 0.07), tint)
            }
        };
        if v > 0.0 {
            lit(light, col, steps(c, j, v) * fade * k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ring::state::BallState;

    /// A canvas of `n` x `n` pixels over the ring's middle 4 x 4 units.
    fn grid(n: usize) -> (Vec<f64>, Vec<f64>, Vec<usize>, Vec<usize>) {
        let (mut sx, mut sz, mut ys, mut xs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for y in 0..n {
            for x in 0..n {
                sx.push(-2.0 + 4.0 * (x as f64 + 0.5) / n as f64);
                sz.push(-2.0 + 4.0 * (y as f64 + 0.5) / n as f64);
                ys.push(y);
                xs.push(x);
            }
        }
        (sx, sz, ys, xs)
    }

    fn ball() -> BallState {
        BallState {
            p: [0.0, 0.0],
            vis: true,
            shield: 0.0,
            haste: 0.0,
            glow: 0.0,
            glow_np: false,
            scale: 1.0,
            th: 0.0,
            blen: 0.0,
            frets: 0,
            level: 1,
            shattered: false,
            om: 0.0,
            sd: 1.0,
            ext: 0.0,
            slow: 0.0,
            thr_k: 0.0,
            lift: 0.0,
            sink: 0.0,
            sx: None,
            sy: None,
            air_bias: 0.0,
            paint_h: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn every_family_lights_its_ring_and_nothing_far_from_it() {
        let (sx, sz, ys, xs) = grid(64);
        let fs = [ball(), ball()];
        let c = CanvasCtx {
            t: 1.0,
            i: 0,
            in_fight: true,
            fs: &fs,
            sxp: &sx,
            szp: &sz,
            ys: &ys,
            xs: &xs,
        };
        for family in [
            Family::Brass,
            Family::Strings,
            Family::Air,
            Family::Quake,
            Family::Noise,
        ] {
            let mut ic = vec![[0.0; 3]; sx.len()];
            band(&mut ic, &c, family, 0.0, 0.0, 1.0, 1.0, 1.0, [1.0; 3], 1.0);
            let lit = ic.iter().filter(|l| l[0] > 0.0).count();
            assert!(lit > 20, "{family:?} lit {lit} pixels");
            // the middle and the far corner stay dark
            assert_eq!(ic[32 * 64 + 32][0], 0.0, "{family:?}");
            assert_eq!(ic[0][0], 0.0, "{family:?}");
        }
    }

    #[test]
    fn the_target_brightens_as_it_closes() {
        let (sx, sz, ys, xs) = grid(64);
        let fs = [ball(), ball()];
        let c = CanvasCtx {
            t: 1.0,
            i: 0,
            in_fight: true,
            fs: &fs,
            sxp: &sx,
            szp: &sz,
            ys: &ys,
            xs: &xs,
        };
        let total = |u: f64| {
            let mut ic = vec![[0.0; 3]; sx.len()];
            target(&mut ic, &c, 0.0, 0.0, 0.6, u, [1.0; 3]);
            ic.iter().map(|l| l[0]).fold(0.0, f64::max)
        };
        assert!(total(1.0) > total(0.0));
        assert!(total(0.0) > 0.0);
    }
}
