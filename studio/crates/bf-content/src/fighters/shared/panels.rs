//! The VS-card panel patterns of the fighters made from scratch (Nov 16, 2026), one function each, drawn the way
//! the older fighters' panels are (`bf_picture::title::patterns`): `base` is the panel's gradient over the whole
//! card, changed in place, and `SIDE` ([`layout::side`]) tells the two halves apart (above zero is the left
//! fighter's, the top left). Each look's `title_pattern` calls its own function here. Judged by eye, not ported.

use bf_core::dmath;
use bf_picture::img::RgbF;
use bf_picture::title::layout;

/// Every pixel of `base` with its `(x, y)` and its distance from the divider (`SIDE`): `f` gives a multiplier
/// and a lift (added as `255 * lift`) for that pixel.
fn shade(base: &mut RgbF, f: impl Fn(f64, f64, f64) -> (f64, f64)) {
    let side = layout::side();
    for y in 0..base.h {
        for x in 0..base.w {
            let s = side.px(x, y)[0];
            let (mul, lift) = f(x as f64, y as f64, s);
            for v in base.px_mut(x, y).iter_mut() {
                *v = *v * mul + 255.0 * lift;
            }
        }
    }
}

/// The distance from the panel's own outer corner (the top left above the divider, the bottom right below it).
fn from_corner(x: f64, y: f64, s: f64, w: f64, h: f64) -> f64 {
    let (cx, cy) = if s > 0.0 { (0.0, 0.0) } else { (w, h) };
    dmath::hypot(x - cx, y - cy)
}

/// CORNET: rows of small wrapped coils (the cornet's tight loops of tubing), each ring lit on its inner edge.
pub fn cornet(base: &mut RgbF) {
    shade(base, |x, y, _| {
        let row = (y / 64.0).floor();
        let ox = if row as i64 % 2 == 0 { 0.0 } else { 32.0 };
        let (gx, gy) = (((x + ox) / 64.0).fract() - 0.5, (y / 64.0).fract() - 0.5);
        let r = dmath::hypot(gx * 1.3, gy) * 64.0;
        if r > 28.0 {
            return (0.94, 0.0);
        }
        let q = (r / 7.0).fract();
        if q < 0.25 {
            (1.0, 0.09)
        } else if q > 0.8 {
            (0.86, 0.0)
        } else {
            (1.0, 0.0)
        }
    });
}

/// MARIMBA: rosewood bars of stepped lengths side by side, with a resonator tube's round mouth under each.
pub fn marimba(base: &mut RgbF) {
    shade(base, |x, y, _| {
        let col = (x / 30.0).floor();
        let fx = (x / 30.0).fract();
        let len = 150.0 + 90.0 * (1.0 + dmath::sin(col * 0.7)) / 2.0;
        let fy = y % 300.0;
        let top = (300.0 - len) / 2.0;
        if fx < 0.14 {
            return (0.78, 0.0);
        }
        if fy > top && fy < top + len {
            // the bar: grain across it, a lit top edge and a mounting hole near each end
            let grain = 0.03 * dmath::sin(fy * 0.9 + col * 3.0);
            let edge = if fy - top < 5.0 { 0.08 } else { 0.0 };
            let hole = ((fx - 0.57) * 30.0).abs() < 2.5
                && ((fy - top - 18.0).abs() < 2.5 || (top + len - 18.0 - fy).abs() < 2.5);
            if hole { (0.6, 0.0) } else { (1.04 + grain, edge) }
        } else {
            let r = dmath::hypot(
                (fx - 0.57) * 30.0,
                (fy - if fy < top { top / 2.0 } else { 300.0 - top / 2.0 }),
            );
            if r < 9.0 {
                (0.66, 0.0)
            } else if r < 12.0 {
                (1.0, 0.07)
            } else {
                (0.88, 0.0)
            }
        }
    });
}

/// PICCOLO: bright, thin streaks of a high whistle leaning along the divider, with small tone holes in rows.
pub fn piccolo(base: &mut RgbF) {
    shade(base, |x, y, s| {
        let along = x * 0.77 + y * 0.64;
        let lane = (s / 26.0).fract();
        let streak = if lane < 0.08 { 0.07 } else { 0.0 };
        let (gx, gy) = ((along / 40.0).fract() - 0.5, lane - 0.55);
        let r = dmath::hypot(gx * 40.0, gy * 26.0);
        let hole = if r < 4.0 {
            (0.7, 0.0)
        } else if r < 6.5 {
            (1.0, 0.12)
        } else {
            (1.0, 0.0)
        };
        let glint = if ((along + s * 3.0) / 97.0).fract() < 0.015 {
            0.10
        } else {
            0.0
        };
        (hole.0, hole.1 + streak + glint)
    });
}

/// SOPRANO SAX: a straight body's rows of pearl-topped keys, slanting across the panel.
pub fn soprano_sax(base: &mut RgbF) {
    shade(base, |x, y, _| {
        let (u, v) = (x * 0.9 + y * 0.44, -x * 0.44 + y * 0.9);
        let lane = (v / 70.0).fract();
        let rod = if (lane - 0.5).abs() < 0.03 { 0.06 } else { 0.0 };
        let (gx, gy) = ((u / 46.0).fract() - 0.5, lane - 0.5);
        let r = dmath::hypot(gx * 46.0, gy * 70.0);
        if r < 6.0 {
            (1.0, 0.18)
        } else if r < 13.0 {
            (0.82, 0.0)
        } else if r < 15.0 {
            (1.0, 0.08)
        } else {
            (0.97, rod)
        }
    });
}

/// MELLOPHONE: the forward bell's flare, wide rings opening from the panel's outer corner.
pub fn mellophone(base: &mut RgbF) {
    let (w, h) = (base.w as f64, base.h as f64);
    shade(base, |x, y, s| {
        let r = from_corner(x, y, s, w, h);
        let q = (r.sqrt() / 2.2).fract();
        let fade = (1.0 - r / 900.0).max(0.25);
        if q < 0.12 {
            (1.0, 0.11 * fade)
        } else if q > 0.85 {
            (1.0 - 0.14 * fade, 0.0)
        } else {
            (1.0, 0.0)
        }
    });
}

/// TIMPANI: a hammered copper kettle, a honeycomb of dimples each lit at the top left and shaded at the bottom right.
pub fn timpani(base: &mut RgbF) {
    shade(base, |x, y, _| {
        let row = (y / 22.0).floor();
        let ox = if row as i64 % 2 == 0 { 0.0 } else { 13.0 };
        let (gx, gy) = (((x + ox) / 26.0).fract() - 0.5, (y / 22.0).fract() - 0.5);
        let r = dmath::hypot(gx * 26.0, gy * 22.0);
        if r > 10.0 {
            return (0.9, 0.0);
        }
        let light = -(gx + gy) * 1.4;
        (1.0 + 0.1 * light.min(0.0), 0.08 * light.max(0.0))
    });
}

/// CONTRABASSOON: thick plumbing pipes along the divider, shaded round, with a collar and bolts at each joint.
pub fn contrabassoon(base: &mut RgbF) {
    shade(base, |x, y, s| {
        let lane = (s.abs() / 52.0).fract();
        let along = x * 0.77 + y * 0.64;
        if lane > 0.62 {
            return (0.8, 0.0);
        }
        let t = lane / 0.62;
        let round = dmath::sin(t * dmath::PI);
        let joint = (along / 150.0).fract();
        if joint < 0.06 {
            let bolt = (t - 0.2).abs() < 0.08 || (t - 0.8).abs() < 0.08;
            return if bolt { (1.0, 0.16) } else { (0.9 + 0.12 * round, 0.05) };
        }
        let shine = if (t - 0.3).abs() < 0.05 { 0.07 } else { 0.0 };
        (0.86 + 0.16 * round, shine)
    });
}

/// SNARE DRUM: the snare wires stretched across the panel under a row of tension lugs along the divider.
pub fn snare_drum(base: &mut RgbF) {
    shade(base, |x, y, s| {
        let along = x * 0.77 + y * 0.64;
        if s.abs() > 12.0 && s.abs() < 40.0 {
            // the lugs: short bars with a lit edge, one every 60 px
            let f = (along / 60.0).fract();
            if f < 0.3 {
                let lit = if s.abs() < 16.0 { 0.12 } else { 0.03 };
                return (0.9, lit);
            }
        }
        let wire = (s / 7.0).fract();
        let crimp = 0.5 + 0.5 * dmath::sin(along / 9.0);
        if wire < 0.18 {
            (1.0, 0.05 + 0.04 * crimp)
        } else {
            (0.96, 0.0)
        }
    });
}

/// OCTOBASS: tall wood grain (long wavy vertical streaks of the body's spruce and maple), crossed every 150 px by a
/// row of brass lever bars, each a short horizontal bar with a lit top edge and a dark pivot dot at its end.
pub fn octobass(base: &mut RgbF) {
    shade(base, |x, y, _| {
        let band = y % 150.0;
        if (40.0..58.0).contains(&band) {
            // the levers: one bar every 46 px across, a gap between
            let fx = x % 46.0;
            if fx < 34.0 {
                let lit = if band < 43.0 { 0.16 } else { 0.06 };
                let pivot = dmath::hypot(fx - 30.0, band - 49.0) < 4.0;
                return if pivot { (0.55, 0.0) } else { (1.0, lit) };
            }
        }
        // the grain: wavy streaks that run down the panel
        let wave = x + 9.0 * dmath::sin(y / 70.0 + x / 40.0);
        let g = (wave / 11.0).fract();
        if g < 0.12 {
            (0.86, 0.0)
        } else if g < 0.2 {
            (1.0, 0.035)
        } else {
            (0.97, 0.0)
        }
    });
}

/// TARGET DUMMY (a developer fighter): rows of small bullseyes, each ring alternately lit and shaded.
pub fn target_dummy(base: &mut RgbF) {
    shade(base, |x, y, _| {
        let row = (y / 72.0).floor();
        let ox = if row as i64 % 2 == 0 { 0.0 } else { 36.0 };
        let (gx, gy) = (((x + ox) / 72.0).fract() - 0.5, (y / 72.0).fract() - 0.5);
        let r = dmath::hypot(gx, gy) * 72.0;
        if r > 30.0 {
            return (0.94, 0.0);
        }
        if ((r / 7.5).floor() as i64) % 2 == 0 {
            (1.0, 0.07)
        } else {
            (0.9, 0.0)
        }
    });
}
