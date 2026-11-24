//! The bold effects (2026-11-24): the upgraded attack visuals the owner picked from the round 2 review. A fight
//! file opts in with `picture.effects = "bold"` ([`bf_core::fightfile::EFFECTS`]); without it nothing here runs
//! and every frame is the one it was, so the series' videos and every reference picture are untouched.
//!
//! The upgrades are look-only: nothing here changes a fight, its damage or its timing. They read the finished
//! recording, which is why a wind-up can be shown before the event it warns of ([`until`]).
//!
//! - [`px`]: crisp pixel shapes with the dark outline, on any of the renderer's pictures.
//! - [`floor`]: marks of light on the canvas (landing marks, lanes, the five kinds of ring).
//! - [`shared`]: the upgrades every fighter gets from the ring itself (hit sizes, the speed-up look, the low-HP
//!   spotlight, shield blocks, rope sparks, the level-up pop, status marks, the side banners).
//! - here: what a fighter's look calls for its own moves: readable weapons ([`thick_line`], [`outlined_poly`],
//!   [`blob`]), the held pose on contact ([`hold_time`]), and the event clock ([`since`], [`until`]).
//!
//! The shared upgrades by their codes on the review page: S1 [`hold_time`]; S2 [`thick_line`], [`outlined_poly`],
//! [`blob`]; S3 [`until`] with [`floor::target`], [`floor::lane`] and [`px::glint`]; S4, S6, S7, S9, S10, S11,
//! S12 and the banners (N4) in [`shared`]; S5 [`floor::wave`]; S8 [`crate::fx::Marks::cap_lives`].

pub mod floor;
pub mod px;
pub mod shared;

use bf_core::event::Event;

use crate::camera::{Cam, V3};
use crate::raster::Frame;
use crate::ring::PictureCtx;
use crate::scene;

use self::px::bad;
pub use self::shared::{HitSize, Shared, Status, StatusKind};

/// The dark outline of every bold shape (the ring's own ink).
pub const INK: V3 = [22.0, 14.0, 30.0];
/// The white of glints, flares and hit stars.
pub const WHITE: V3 = [255.0, 252.0, 244.0];
/// The warm gold of brass rings, level-ups and stun stars.
pub const GOLD: V3 = [255.0, 214.0, 96.0];

/// `col` made lighter: the "light edge" of a weapon and the colour of a mark that must read on a dark floor.
pub fn pale(col: V3) -> V3 {
    col.map(|c| (c * 0.55 + 125.0).min(255.0))
}

/// `col` made darker: the body of a banner, the shaded side of a weapon.
pub fn deep(col: V3) -> V3 {
    col.map(|c| c * 0.42)
}

/// `col` as canvas light: what the [`floor`] marks take as their tint (about 1 a channel for a bright colour).
pub fn light(col: V3) -> V3 {
    col.map(|c| c / 255.0 * 1.3 + 0.3)
}

/// 0 to 1, fast at first and settling: for things that pop out.
pub fn ease_out(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    1.0 - (1.0 - u) * (1.0 - u)
}

/// 0 to 1, slow at first: for things that wind up.
pub fn ease_in(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u
}

/// The ring-frame point `(x, z)` at height `y`, on the screen.
pub fn at(cam: &Cam, x: f64, z: f64, y: f64) -> (f64, f64) {
    cam.wp(&[scene::to_world(x, z, y)])[0]
}

/// How many screen pixels one world unit is at the ring-frame point `(x, z)`, height `y` (a ball's radius is
/// [`scene::R_BALL`] units, about 20 pixels in the middle of the ring).
pub fn px_per_unit(cam: &Cam, x: f64, z: f64, y: f64) -> f64 {
    let pc = cam.to_cam(&[scene::to_world(x, z, y)])[0];
    if pc[2] > cam.near { cam.f / pc[2] } else { 0.0 }
}

/// A world polyline as a readable weapon: the dark outline (`width + 2`), the body in `col` and, from 3 pixels
/// up, a light edge along its middle. `width` is the body's width in pixels; anything below 3 is drawn as 3.
pub fn thick_line(fr: &mut Frame, cam: &Cam, pts: &[V3], col: V3, width: u32) {
    let w = width.max(3);
    fr.line(cam, pts, INK, w + 2);
    fr.line(cam, pts, col, w);
    fr.line(cam, pts, pale(col), 1);
}

/// A world polygon filled with `col` inside a dark outline.
pub fn outlined_poly(fr: &mut Frame, cam: &Cam, pts: &[V3], col: V3) {
    if pts.len() < 3 {
        return;
    }
    fr.poly(cam, pts, col, true);
    let mut closed = pts.to_vec();
    closed.push(pts[0]);
    fr.line(cam, &closed, INK, 1);
}

/// A round thing in the air (a projectile, a bead, a knob) at the world point `w`, `r` world units across its
/// radius and never under 2.5 pixels: the dark outline, the body and a small highlight.
pub fn blob(fr: &mut Frame, cam: &Cam, w: V3, r: f64, col: V3) {
    let pc = cam.to_cam(&[w])[0];
    if pc[2] <= cam.near {
        return;
    }
    let s = cam.proj(&[pc])[0];
    let rp = (r * cam.f / pc[2]).max(2.5);
    fr.disc(s, rp + 1.2, INK, 14, 0.0, Some(pc[2]));
    fr.disc(s, rp, col, 14, 0.0, Some(pc[2] - 0.01));
    if rp >= 3.0 {
        fr.disc(
            (s.0 - rp * 0.35, s.1 - rp * 0.35),
            rp * 0.3,
            pale(col),
            8,
            0.0,
            Some(pc[2] - 0.02),
        );
    }
}

/// The event of one of `kinds` by `who` that is most lately at or before `t`, no more than `window` seconds
/// ago: its age in seconds and the event. For a `hit`, give the hit's `how` (its element 7) in `kinds` as
/// `"hit:how"`, or `"hit"` for any hit.
pub fn since<'a>(p: &'a PictureCtx, who: usize, kinds: &[&str], t: f64, window: f64) -> Option<(f64, &'a Event)> {
    let i0 = p.ev_t.partition_point(|&te| te < t - window);
    let i1 = p.ev_t.partition_point(|&te| te <= t);
    p.rec.events[i0..i1.max(i0)]
        .iter()
        .rev()
        .find(|e| e.who() == Some(who) && is_kind(e, kinds))
        .map(|e| (t - e.t, e))
}

/// The next event of one of `kinds` by `who` after `t`, no more than `window` seconds ahead: how far the
/// wind-up is (0 when the event is `window` away, 1 as it happens) and the event. The recording is finished
/// before any frame is drawn, so a warning can be shown before the move it warns of. Kinds as in [`since`].
pub fn until<'a>(p: &'a PictureCtx, who: usize, kinds: &[&str], t: f64, window: f64) -> Option<(f64, &'a Event)> {
    if bad(window) {
        return None;
    }
    let i0 = p.ev_t.partition_point(|&te| te <= t);
    let i1 = p.ev_t.partition_point(|&te| te <= t + window);
    p.rec.events[i0..i1.max(i0)]
        .iter()
        .find(|e| e.who() == Some(who) && is_kind(e, kinds))
        .map(|e| (1.0 - (e.t - t) / window, e))
}

/// Whether the event is one of `kinds` (see [`since`]).
fn is_kind(e: &Event, kinds: &[&str]) -> bool {
    kinds.iter().any(|k| match k.strip_prefix("hit:") {
        Some(how) => e.kind == "hit" && matches!(e.fields.get(5), Some(bf_core::event::Value::Str(s)) if s == how),
        None => e.kind == *k,
    })
}

/// S1, "stops on contact": the time a weapon's pose should be read at. For three frames after `who` lands a hit
/// of one of `hows` (a hit's element 7; empty: any hit) the pose is held where it touched: this returns the
/// hit's time. Otherwise it returns `t`. Only the picture holds; the fight carries on.
pub fn hold_time(p: &PictureCtx, who: usize, hows: &[&str], t: f64) -> f64 {
    const HOLD: f64 = 3.0 / 30.0;
    let i0 = p.ev_t.partition_point(|&te| te < t - HOLD);
    let i1 = p.ev_t.partition_point(|&te| te <= t);
    p.rec.events[i0..i1.max(i0)]
        .iter()
        .rev()
        .find(|e| {
            e.kind == "hit"
                && e.who() == Some(who)
                && (hows.is_empty()
                    || matches!(e.fields.get(5), Some(bf_core::event::Value::Str(s)) if hows.contains(&s.as_str())))
        })
        .map_or(t, |e| e.t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_helpers_stay_in_range() {
        let c = [255.0, 10.0, 128.0];
        assert!(pale(c).iter().all(|&v| (0.0..=255.0).contains(&v)));
        assert!(deep(c).iter().zip(&c).all(|(d, c)| d <= c));
        assert!(light(c)[0] > light(c)[1]);
    }

    #[test]
    fn the_eases_run_from_0_to_1() {
        assert_eq!((ease_out(-1.0), ease_out(2.0)), (0.0, 1.0));
        assert_eq!((ease_in(0.0), ease_in(1.0)), (0.0, 1.0));
        assert!(ease_out(0.5) > 0.5 && ease_in(0.5) < 0.5);
    }
}
