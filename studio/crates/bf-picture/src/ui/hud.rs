//! `fight_hud` of `reelkit/ui.py`: two fighting-game HP bars at the bottom of the safe box. The left fighter's bar
//! is anchored at the left edge and the right fighter's at the right edge, so damage eats each bar from the
//! middle, with a pale trail that catches up. Over each bar: the name and the HP number; under it: the fighter's
//! ability rows (`_ability_rows`), or without them the level pips and a status line; between the bars: `VS`.
//!
//! Numbers: the bars and texts are whole pixels from Python's `round` (ties to even); every mix is `float64`
//! but two that numpy does in `float32` (the backing's `reg * 0.40` and the pill's body colour), kept as `f32`
//! here. The low-HP pulse takes a sine (`bf_core::dmath`, where Python uses the C runtime's); it only tints the
//! bar, and a last-bit difference could change a byte only where a tint lies within about 1e-13 of a whole
//! number. On the reference vectors the nearest is 4e-3, but for bars at `t` = 1/8 s, where the sine is exactly 1,
//! the pulse exactly 1 and two tints whole numbers; there a sine one bit off gives the same pulse
//! (`0.5 + 0.5 * sin` rounds back to 1), and so it does at the pulse's low points.

use super::label::label;
use super::{AMBER, DrawLog, INK, Kind, clip_rgb, fill, mix_over, py_index, py_max, py_min, py_slice, round_i32};
use crate::config::{SAFE, safe_x1};
use crate::font::{self, ADV};
use crate::img::Rgb8;
use crate::np;
use bf_core::dmath;

/// `ABIL_ROW`: the pixels from one ability row to the next (scale-1 text and its outline).
pub const ABIL_ROW: i32 = 9;
/// `ABIL_HOT`: an ability in use: white-hot text on a glowing pill in the fighter's colour.
pub const ABIL_HOT: [u8; 3] = [255, 250, 232];
/// `ABIL_GREY`: a locked ability: the colours of its tag and of its name.
pub const ABIL_GREY: [[u8; 3]; 2] = [[96, 94, 116], [118, 116, 138]];

/// The edge colour of the backing, `np.array([62, 74, 112], float)`.
const EDGE: [f64; 3] = [62.0, 74.0, 112.0];
/// The height of scale-1 text.
const ROW: i32 = 7;

/// One fighter as the HUD shows it: the `left` / `right` dict of `fight_hud`. [`Fighter::new`] gives every field
/// but the name, the colour and the HP its Python default.
#[derive(Clone, Debug, PartialEq)]
pub struct Fighter {
    /// `name`: drawn over the bar's outer end.
    pub name: String,
    /// `color`: the bar's and the texts' colour, as floats (the kit passes its float colour array).
    pub color: [f64; 3],
    /// `hp`: clamped to `0..=hp_max`.
    pub hp: f64,
    /// `hp_max` (100).
    pub hp_max: f64,
    /// `trail`: the HP the pale trail shows; `None` or less than `hp` is `hp`.
    pub trail: Option<f64>,
    /// `level` (1): the pips lit, when there are no ability rows.
    pub level: i32,
    /// `max_level` (3): the pips drawn.
    pub max_level: i32,
    /// `status` (empty): a short line beside the pips, when there are no ability rows.
    pub status: String,
    /// `status_k` (0): the status line's fade, 0 to 1; at 0.01 or less it is not drawn.
    pub status_k: f64,
    /// `flash` (0): 0 to 1, the bar whitened on a hit.
    pub flash: f64,
    /// `ko` (false): the name dims and `K.O.` stands for the number.
    pub ko: bool,
    /// `win` (false): a gold frame around the bar.
    pub win: bool,
    /// `t` (0): the time, for the low-HP pulse (two beats a second).
    pub t: f64,
}

impl Fighter {
    /// A fighter with Python's defaults: `hp_max` 100, no trail, level 1 of 3, no status, no flash, not K.O., no
    /// win, `t` 0.
    pub fn new(name: &str, color: [f64; 3], hp: f64) -> Self {
        Fighter {
            name: name.to_string(),
            color,
            hp,
            hp_max: 100.0,
            trail: None,
            level: 1,
            max_level: 3,
            status: String::new(),
            status_k: 0.0,
            flash: 0.0,
            ko: false,
            win: false,
            t: 0.0,
        }
    }
}

/// One row of a fighter's ability list (`LV2 SAD TROMBONE`): a row dict of `_ability_rows`.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityRow {
    /// `tag` (empty): `LV2`, `LV3`, `LOW`; drawn dimmer than the name.
    pub tag: String,
    /// `name`: the ability's name.
    pub name: String,
    /// `state != 'locked'`: an unlocked row is in the fighter's colour, a locked one grey (false is the default).
    pub unlocked: bool,
    /// `glow` (0): 0 to 1, in use: the name turns white-hot on a pill in the fighter's colour. Ignored while
    /// locked.
    pub glow: f64,
    /// `sweep` (`None`): 0 to 1, the progress of the glow that sweeps across the name as it unlocks. Ignored
    /// while locked, and outside `0 <= sweep < 1`.
    pub sweep: Option<f64>,
}

impl AbilityRow {
    /// A locked row without a tag, glow or sweep: Python's defaults.
    pub fn new(name: &str) -> Self {
        AbilityRow {
            tag: String::new(),
            name: name.to_string(),
            unlocked: false,
            glow: 0.0,
            sweep: None,
        }
    }
}

/// The keywords of `fight_hud` besides `abilities`. `Default` is Python's.
#[derive(Clone, Debug, PartialEq)]
pub struct HudOptions {
    /// `x0`: the left end of the left bar; `None`: 4 px inside the safe box.
    pub x0: Option<i32>,
    /// `x1`: the right end of the right bar; `None`: 4 px left of the safe limit for the HUD's rows (the button
    /// column's edge when the HUD reaches down into it).
    pub x1: Option<i32>,
    /// `y1`: the HUD's bottom; `None`: 4 px above the bottom of the safe box.
    pub y1: Option<i32>,
    /// `alpha` (1): below 0.999 the whole HUD is mixed into the picture by it.
    pub alpha: f64,
    /// `ink` ([`INK`]): the bar frames and the backing's tint.
    pub ink: [u8; 3],
    /// `bar_h` (6): the height of the bars.
    pub bar_h: i32,
    /// `center` (`VS`): the text between the bars; empty for none (the bars then nearly meet).
    pub center: String,
}

impl Default for HudOptions {
    fn default() -> Self {
        HudOptions {
            x0: None,
            x1: None,
            y1: None,
            alpha: 1.0,
            ink: INK,
            bar_h: 6,
            center: "VS".to_string(),
        }
    }
}

/// A box of pixels, `(x0, y0, x1, y1)`: the columns `x0..x1` of the rows `y0..y1`.
pub type PxBox = (i32, i32, i32, i32);

/// What `fight_hud` returns, so a caller can hang extras on the HUD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hud {
    /// `top`: the top row of the backing.
    pub top: i32,
    /// `bars`: `(xa, xb, y0, y1)` of the left and the right bar (ends exclusive).
    pub bars: [PxBox; 2],
    /// `rows`: each side's ability rows as `(x0, y0, x1, y1)` text boxes; `None` without `abilities`.
    pub rows: Option<[Vec<PxBox>; 2]>,
}

/// `len(s)` as Python counts it: characters (Unicode code points).
fn chars(s: &str) -> i32 {
    s.chars().count() as i32
}

/// `tuple(int(v) for v in c)`: the fraction dropped. (The colours here are within 0..=255.)
fn trunc_rgb(c: [f64; 3]) -> [u8; 3] {
    c.map(|v| v as u8)
}

/// The backing: the picture dimmed and tinted under the HUD, with a lighter edge on its first and last row.
fn backing(img: &mut Rgb8, (bx0, by0, bx1, by1): PxBox, ink: [u8; 3]) {
    // reg = img[by0:by1, bx0:bx1].astype(np.float32)
    // img[by0:by1, bx0:bx1] = (reg * 0.40 + np.array(ink, float) * 0.32).astype(np.uint8)
    // reg * 0.40 is a float32 product (numpy keeps the array's type for a Python scalar); the sum is float64.
    let dim = 0.40_f64 as f32;
    let tint = ink.map(|v| f64::from(v) * 0.32);
    let cols = py_slice(bx0, bx1, img.w);
    for y in py_slice(by0, by1, img.h) {
        for x in cols.clone() {
            for (p, t) in img.px_mut(x, y).iter_mut().zip(tint) {
                *p = np::f64_to_u8(f64::from(f32::from(*p) * dim) + t);
            }
        }
    }
    // img[by0, bx0:bx1] = (img[by0, bx0:bx1] * 0.4 + edge * 0.6).astype(np.uint8)
    // img[by1 - 1, bx0:bx1] = (img[by1 - 1, bx0:bx1] * 0.6 + edge * 0.4).astype(np.uint8)
    for (row, keep, add) in [(by0, 0.4, 0.6), (by1 - 1, 0.6, 0.4)] {
        let y = py_index(row, img.h);
        let edge = EDGE.map(|v| v * add);
        for x in cols.clone() {
            for (p, e) in img.px_mut(x, y).iter_mut().zip(edge) {
                *p = np::f64_to_u8(f64::from(*p) * keep + e);
            }
        }
    }
}

/// `img[y, xs.0:xs.1] = col`: part of one row.
fn fill_row(img: &mut Rgb8, y: i32, xs: (i32, i32), col: [u8; 3]) {
    let y = py_index(y, img.h);
    for x in py_slice(xs.0, xs.1, img.w) {
        img.px_mut(x, y).copy_from_slice(&col);
    }
}

/// `img[ys.0:ys.1, x] = col`: part of one column.
fn fill_col(img: &mut Rgb8, ys: (i32, i32), x: i32, col: [u8; 3]) {
    let x = py_index(x, img.w);
    for y in py_slice(ys.0, ys.1, img.h) {
        img.px_mut(x, y).copy_from_slice(&col);
    }
}

/// One fighter's bar on the columns `xa..xb`, rows `yb0..yb1`: the frame and the empty well, the pale trail,
/// the HP (red-orange and pulsing at a quarter of the maximum or less, whitened by a flash), the quarter ticks and
/// the winner's gold frame. Returns the HP clamped to `0..=hp_max`.
fn bar(img: &mut Rgb8, f: &Fighter, side: usize, (xa, xb, yb0, yb1): PxBox, ink: [u8; 3]) -> f64 {
    let hp_max = f.hp_max;
    assert!(
        hp_max != 0.0,
        "fight_hud: {}'s hp_max is 0 (Python raises ZeroDivisionError)",
        f.name
    );
    let hp = py_max(0.0, py_min(hp_max, f.hp));
    let trail = py_max(hp, f.trail.unwrap_or(hp));
    let w = xb - xa;
    // bar frame + empty well
    fill(img, (yb0 - 1, yb1 + 1), (xa - 1, xb + 1), ink);
    fill(img, (yb0, yb1), (xa, xb), [40, 30, 52]);
    let (fh, ft) = (f64::from(w) * hp / hp_max, f64::from(w) * trail / hp_max);
    let low = hp <= 0.25 * hp_max && !f.ko;
    let mut bc = f.color;
    if low {
        let pulse = 0.5 + 0.5 * dmath::sin(f.t * 2.0 * dmath::PI * 2.0);
        bc = [255.0, 96.0, 52.0].map(|v| v * (0.8 + 0.2 * pulse));
    }
    let fl = f.flash;
    let bc = bc.map(|v| np::clip(v * (1.0 - 0.6 * fl) + 255.0 * 0.6 * fl, 0.0, 255.0));
    // anchored at the outer edge: damage eats from the middle
    let (seg_t, seg_h) = if side == 0 {
        (
            (xa, xa.saturating_add(round_i32(ft))),
            (xa, xa.saturating_add(round_i32(fh))),
        )
    } else {
        (
            (xb.saturating_sub(round_i32(ft)), xb),
            (xb.saturating_sub(round_i32(fh)), xb),
        )
    };
    if seg_t.1 > seg_t.0 {
        fill(img, (yb0, yb1), seg_t, [255, 236, 200]);
    }
    if seg_h.1 > seg_h.0 {
        fill(img, (yb0, yb1), seg_h, clip_rgb(bc));
        fill(img, (yb0, yb0 + 1), seg_h, clip_rgb(bc.map(|v| v * 0.55 + 115.0)));
        fill(img, (yb1 - 1, yb1), seg_h, clip_rgb(bc.map(|v| v * 0.65)));
    }
    // quarter ticks
    for k in 1..4 {
        let xt = xa + round_i32(f64::from(w * k) / 4.0);
        fill_col(img, (yb1 - 2, yb1), xt, ink);
    }
    if f.win {
        fill_row(img, yb0 - 2, (xa - 2, xb + 2), AMBER);
        fill_row(img, yb1 + 1, (xa - 2, xb + 2), AMBER);
        fill_col(img, (yb0 - 2, yb1 + 2), xa - 2, AMBER);
        fill_col(img, (yb0 - 2, yb1 + 2), xb + 1, AMBER);
    }
    hp
}

/// One ability row laid out: where its tag and its name go and how lit it is.
struct RowPlan<'a> {
    y: i32,
    tag: &'a str,
    name: &'a str,
    on: bool,
    /// The glow, 0 unless unlocked.
    g: f64,
    /// The unlock sweep's progress, if one is running.
    sw: Option<f64>,
    /// The row's left end (the tag's, if it has one).
    x: i32,
    /// The name's left end.
    nx: i32,
    /// The name's width.
    nw: i32,
}

/// The column the unlock sweep is at: it runs across the name from the fighter's outer edge inwards.
fn sweep_x(r: &RowPlan, side: usize, sw: f64) -> f64 {
    // bx = (nx - 5 + sw * (nw + 10)) if side == 0 else (nx + nw + 4 - sw * (nw + 10))
    let run = sw * f64::from(r.nw + 10);
    if side == 0 {
        f64::from(r.nx - 5) + run
    } else {
        f64::from(r.nx + r.nw + 4) - run
    }
}

/// The pill under a name in use: a rounded rectangle 10 px tall in the fighter's colour with a lit top edge, a
/// shaded bottom edge and a soft halo, mixed in by the glow. Noted as a panel.
fn pill(img: &mut Rgb8, log: &mut DrawLog, r: &RowPlan, col: [f64; 3]) {
    let (px0, px1, py0, py1) = (r.nx - 3, r.nx + r.nw + 3, r.y - 1, r.y + 9);
    let (cx0, cx1) = ((px0 - 1).max(0), (px1 + 1).min(img.w as i32));
    let (cy0, cy1) = ((py0 - 1).max(0), (py1 + 1).min(img.h as i32));
    // fill = np.empty_like(reg); fill[:] = np.clip(col * 0.74 + 22, 0, 255): reg is float32, so the body colour
    // is rounded to float32; the two np.where below put float64 colours on the edge rows.
    let body = col.map(|v| f64::from(np::clip(v * 0.74 + 22.0, 0.0, 255.0) as f32));
    let lit = col.map(|v| np::clip(v * 0.9 + 60.0, 0.0, 255.0));
    let shade = col.map(|v| np::clip(v * 0.55 + 6.0, 0.0, 255.0));
    // distance outside a rounded rectangle (corner radius 2.5): 0 inside, growing outward
    let outside = |centre: f64, lo: i32, hi: i32| {
        np::maximum(
            np::maximum(f64::from(lo) + 2.5 - centre, centre - (f64::from(hi) - 2.5)),
            0.0,
        )
    };
    for yy in cy0..cy1 {
        let qy = outside(f64::from(yy) + 0.5, py0, py1);
        let band = yy - py0;
        let colour = if band >= py1 - py0 - 1 {
            shade
        } else if band <= 1 {
            lit
        } else {
            body
        };
        for xx in cx0..cx1 {
            let qx = outside(f64::from(xx) + 0.5, px0, px1);
            let dist = dmath::hypot(qx, qy) - 2.5;
            // a = np.where(dist <= -0.5, 0.92, np.where(dist <= 0.5, 0.92 * (0.5 - dist), 0.0))
            let core = if dist <= -0.5 {
                0.92
            } else if dist <= 0.5 {
                0.92 * (0.5 - dist)
            } else {
                0.0
            };
            // a = np.maximum(a, np.clip(0.24 * (1.2 - dist) / 1.2, 0, 0.24) * (dist > -0.5))   # soft halo
            let halo = np::clip(0.24 * (1.2 - dist) / 1.2, 0.0, 0.24) * if dist > -0.5 { 1.0 } else { 0.0 };
            let a = np::maximum(core, halo) * r.g;
            for (p, c) in img.px_mut(xx as usize, yy as usize).iter_mut().zip(colour) {
                *p = np::clip_u8(f64::from(*p) * (1.0 - a) + c * a);
            }
        }
    }
    log.note(cx0, cy0, cx1, cy1, Kind::Panel, &format!("ability {}", r.name));
}

/// The soft haze in the fighter's colour that runs with the unlock sweep: 15 columns around it, 11 rows.
fn haze(img: &mut Rgb8, r: &RowPlan, side: usize, sw: f64, col: [f64; 3]) {
    let bx = sweep_x(r, side, sw);
    // xs_ = np.arange(max(0, int(bx) - 7), min(W_, int(bx) + 8))
    let at = np::f64_to_i64(bx);
    let (lo, hi) = ((at - 7).max(0), (at + 8).min(img.w as i64));
    // ys_ = slice(max(0, y - 2), min(H_, y + 9))
    let rows = py_slice((r.y - 2).max(0), (r.y + 9).min(img.h as i32), img.h);
    for x in lo..hi {
        // k_ = np.clip(1 - np.abs(xs_ + 0.5 - bx) / 7.0, 0, 1) * 0.55 * (1 - 0.5 * sw)
        let k = np::clip(1.0 - (x as f64 + 0.5 - bx).abs() / 7.0, 0.0, 1.0) * 0.55 * (1.0 - 0.5 * sw);
        for y in rows.clone() {
            for (p, c) in img.px_mut(x as usize, y).iter_mut().zip(col) {
                *p = np::clip_u8(f64::from(*p) * (1.0 - k) + c * k);
            }
        }
    }
}

/// `_ability_rows(img, rows, side, xa, xb, yp, col)`: the ability list under one fighter's bar, one scale-1 row
/// per ability from row `yp` down, aligned to the fighter's outer edge (`xa` for the left fighter, `xb` for the
/// right). First the pills and hazes of every row, then every row's text, so no pill covers a neighbour's
/// letters. Returns each row's text box.
#[allow(clippy::too_many_arguments)] // the Python signature, and the log
fn ability_rows(
    img: &mut Rgb8,
    log: &mut DrawLog,
    rows: &[AbilityRow],
    side: usize,
    xa: i32,
    xb: i32,
    yp: i32,
    col: [f64; 3],
) -> Vec<PxBox> {
    let lay: Vec<RowPlan> = rows
        .iter()
        .enumerate()
        .map(|(j, a)| {
            let on = a.unlocked;
            let g = if on { py_min(1.0, py_max(0.0, a.glow)) } else { 0.0 };
            let sw = a.sweep.filter(|s| on && (0.0..1.0).contains(s));
            let tw = if a.tag.is_empty() { 0 } else { (chars(&a.tag) + 1) * ADV }; // the tag + one space
            let nw = chars(&a.name) * ADV - 1;
            let x = if side == 0 { xa } else { xb - (tw + nw) };
            RowPlan {
                y: yp + j as i32 * ABIL_ROW,
                tag: &a.tag,
                name: &a.name,
                on,
                g,
                sw,
                x,
                nx: x + tw,
                nw,
            }
        })
        .collect();
    // pass 1: the pills (in use) and the unlock haze, under every row's text
    for r in &lay {
        if r.g > 0.004 {
            pill(img, log, r, col);
        }
        if let Some(sw) = r.sw {
            haze(img, r, side, sw, col);
        }
    }
    // pass 2: the text
    let hot = ABIL_HOT.map(f64::from);
    let ink0 = font::OUTLINE.map(f64::from);
    let mut out = Vec::with_capacity(lay.len());
    for r in &lay {
        let g = r.g;
        let (tcol, ncol) = if r.on {
            let tcol = col.map(|v| np::clip(v * 0.42 + 84.0, 0.0, 255.0));
            let ncol = col.map(|v| np::clip(v * 0.85 + 38.0, 0.0, 255.0));
            // ncol = ncol * (1 - g) + HOT * g; tcol = tcol * (1 - 0.5 * g) + HOT * 0.5 * g
            (
                [0, 1, 2].map(|c| tcol[c] * (1.0 - 0.5 * g) + hot[c] * 0.5 * g),
                [0, 1, 2].map(|c| ncol[c] * (1.0 - g) + hot[c] * g),
            )
        } else {
            (ABIL_GREY[0].map(f64::from), ABIL_GREY[1].map(f64::from))
        };
        if !r.tag.is_empty() {
            label(img, log, r.tag, r.x, r.y, trunc_rgb(tcol), 1, Some(font::OUTLINE));
        }
        // lit: the letters' outline turns a deep shade of the fighter's colour, so the pill reads as one glow
        let ol = [0, 1, 2].map(|c| ink0[c] * (1.0 - g) + np::clip(col[c] * 0.38, 0.0, 255.0) * g);
        let m = font::text_mask(r.name, 1);
        font::stamp(img, &m, r.nx, r.y, trunc_rgb(ncol), Some(trunc_rgb(ol)), true, 1.0);
        log.note(r.nx, r.y, r.nx + m.w as i32, r.y + m.h as i32, Kind::Label, r.name);
        if let Some(sw) = r.sw {
            // the sweep's bright band over the letters themselves
            let bx = sweep_x(r, side, sw);
            for (yy, xx) in np::nonzero(&m) {
                let (x, y) = (r.nx + xx as i32, r.y + yy as i32);
                let d = (f64::from(x) + 0.5 - bx).abs();
                if d < 5.0 && (0..img.w as i32).contains(&x) && (0..img.h as i32).contains(&y) {
                    let kk = 1.0 - d / 5.0;
                    for (p, h) in img.px_mut(x as usize, y as usize).iter_mut().zip(hot) {
                        *p = np::clip_u8(f64::from(*p) * (1.0 - kk) + h * kk);
                    }
                }
            }
        }
        out.push((r.x, r.y, r.nx + r.nw, r.y + ROW));
    }
    out
}

/// The level pips (`LV` and one small box per level, lit up to the fighter's level) and, when it fits between
/// the pips and the middle, the status line, faded by `status_k`.
fn pips_and_status(
    img: &mut Rgb8,
    log: &mut DrawLog,
    f: &Fighter,
    side: usize,
    (xa, xb, yp): (i32, i32, i32),
    ink: [u8; 3],
) {
    let col = f.color;
    let (lv, mx) = (f.level, f.max_level);
    let pw = 2 * ADV + 1 + mx * 5;
    let px = if side == 0 { xa } else { xb - pw };
    label(img, log, "LV", px, yp, [176, 176, 200], 1, Some(font::OUTLINE));
    for k in 0..mx {
        let cx = px + 2 * ADV + 1 + k * 5;
        fill(img, (yp + 1, yp + 6), (cx, cx + 4), ink);
        let pip = if k < lv {
            clip_rgb(col.map(|v| v * 0.9 + 30.0))
        } else {
            [70, 64, 88]
        };
        fill(img, (yp + 2, yp + 5), (cx + 1, cx + 3), pip);
    }
    let (st, sk) = (f.status.as_str(), f.status_k);
    if !st.is_empty() && sk > 0.01 {
        let sw = chars(st) * ADV - 1;
        let sx = if side == 0 { px + pw + 4 } else { px - 4 - sw };
        if (side == 0 && sx + sw <= xb) || (side == 1 && sx >= xa) {
            let before = (sk < 0.999).then(|| img.clone());
            let tint = clip_rgb(col.map(|v| v * 0.6 + 110.0));
            label(img, log, st, sx, yp, tint, 1, Some(font::OUTLINE));
            if let Some(before) = before {
                // img[:] = (before * (1 - sk) + img * sk).astype(np.uint8)
                mix_over(img, &before, sk);
            }
        }
    }
}

/// `fight_hud(img, left, right, x0=None, x1=None, y1=None, alpha=1.0, ink=INK, bar_h=6, center='VS',
/// abilities=None)`: draws the two fighters' HUD into the picture and notes its parts (the backing and each bar
/// as panels, every text as a label, every lit pill as a panel).
///
/// With `abilities` (`[left rows, right rows]`, what the fight always passes) each fighter's ability list stands
/// under its bar in place of the level pips and the status line, and the HUD grows upward by [`ABIL_ROW`] pixels
/// for every row after the first; it still ends at `y1`. At a quarter of the maximum HP or less a bar turns
/// red-orange and pulses with `t`.
///
/// With the defaults and three ability rows a side the backing covers columns 20 to 299 and rows 448 to 498.
/// Panics where Python raises: a fighter whose `hp_max` is 0, and a HUD placed so that one of its single-row or
/// single-column strokes (the backing's edge rows, the bar's quarter ticks, the winner's frame) lies at or beyond
/// the picture's far edge; see the module documentation of [`super`] for what a HUD partly outside draws.
pub fn fight_hud(
    img: &mut Rgb8,
    log: &mut DrawLog,
    left: &Fighter,
    right: &Fighter,
    abilities: Option<[&[AbilityRow]; 2]>,
    opts: &HudOptions,
) -> Hud {
    assert_eq!(
        img.c, 3,
        "fight_hud wants an RGB picture, this one has {} channels",
        img.c
    );
    let (ink, bar_h, center) = (opts.ink, opts.bar_h, opts.center.as_str());
    let x0 = opts.x0.unwrap_or(SAFE.0 + 4);
    let y1 = opts.y1.unwrap_or(SAFE.3 - 4);
    let mut h = ROW + 3 + bar_h + 3 + ROW;
    if let Some(rows) = abilities {
        let n_rows = rows[0].len().max(rows[1].len()).max(1) as i32;
        h += (n_rows - 1) * ABIL_ROW;
    }
    let y0 = y1 - h;
    // stays left of the app's button column
    let x1 = opts.x1.unwrap_or_else(|| safe_x1(y0, y1) - 4);
    let under = (opts.alpha < 0.999).then(|| img.clone());
    let (bx0, by0, bx1, by1) = (x0 - 4, y0 - 4, x1 + 4, y1 + 3);
    backing(img, (bx0, by0, bx1, by1), ink);
    log.note(bx0, by0, bx1, by1, Kind::Panel, "fight hud");
    let cw = if center.is_empty() { 0 } else { chars(center) * ADV - 1 };
    let mid = f64::from(x0 + x1) / 2.0;
    let gap = if center.is_empty() { 6 } else { cw + 8 };
    let yb0 = y0 + ROW + 3;
    let yb1 = yb0 + bar_h;
    let mut bars = [(0, 0, 0, 0); 2];
    let mut rows_out = [Vec::new(), Vec::new()];
    for (side, f) in [left, right].into_iter().enumerate() {
        let (xa, xb) = if side == 0 {
            (x0, round_i32(mid - f64::from(gap) / 2.0))
        } else {
            (round_i32(mid + f64::from(gap) / 2.0), x1)
        };
        let hp = bar(img, f, side, (xa, xb, yb0, yb1), ink);
        log.note(
            xa - 1,
            yb0 - 1,
            xb + 1,
            yb1 + 1,
            Kind::Panel,
            &format!("hp bar {}", f.name),
        );
        // name + number above
        let col = f.color;
        let tcol = clip_rgb(col.map(|v| if f.ko { v * 0.35 + 60.0 } else { v * 0.85 + 38.0 }));
        // num = 'K.O.' if ko else str(int(math.ceil(hp - 1e-6)))
        let (num, num_col) = if f.ko {
            ("K.O.".to_string(), [255, 120, 96])
        } else {
            (np::f64_to_i64((hp - 1e-6).ceil()).to_string(), [255, 240, 214])
        };
        let nw = chars(&num) * ADV - 1;
        let outline = Some(font::OUTLINE);
        if side == 0 {
            label(img, log, &f.name, xa, y0, tcol, 1, outline);
            label(img, log, &num, xb - nw, y0, num_col, 1, outline);
        } else {
            label(img, log, &num, xa, y0, num_col, 1, outline);
            label(img, log, &f.name, xb - (chars(&f.name) * ADV - 1), y0, tcol, 1, outline);
        }
        let yp = yb1 + 3;
        match abilities {
            // the ability list replaces the level pips + status line
            Some(rows) => rows_out[side] = ability_rows(img, log, rows[side], side, xa, xb, yp, col),
            None => pips_and_status(img, log, f, side, (xa, xb, yp), ink),
        }
        bars[side] = (xa, xb, yb0, yb1);
    }
    if !center.is_empty() {
        let x = round_i32(mid - f64::from(cw) / 2.0);
        label(img, log, center, x, yb0 - 1, [232, 220, 180], 1, Some(font::OUTLINE));
    }
    if let Some(under) = under {
        // img[:] = (under * (1 - alpha) + img * alpha).astype(np.uint8)
        mix_over(img, &under, opts.alpha);
    }
    Hud {
        top: by0,
        bars,
        rows: abilities.map(|_| rows_out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::img::Img;

    fn grey() -> Rgb8 {
        Img::filled(360, 640, &[128, 128, 128])
    }

    fn pair() -> (Fighter, Fighter) {
        (
            Fighter::new("FLUTE", [222.0, 232.0, 246.0], 100.0),
            Fighter::new("TROMBONE", [120.0, 240.0, 184.0], 50.0),
        )
    }

    #[test]
    fn the_default_hud_sits_at_the_bottom_of_the_safe_box() {
        let (left, right) = pair();
        let (mut img, mut log) = (grey(), DrawLog::new());
        let hud = fight_hud(&mut img, &mut log, &left, &right, None, &HudOptions::default());
        // h = 7 + 3 + 6 + 3 + 7 = 26: rows 470..496, bars on rows 480..486; x 24..296 less the 19 px for VS.
        assert_eq!(hud.top, 466);
        assert_eq!(hud.bars, [(24, 150, 480, 486), (170, 296, 480, 486)]);
        assert_eq!(hud.rows, None);
        let noted: Vec<_> = log
            .boxes
            .iter()
            .map(|b| (b.x0, b.y0, b.x1, b.y1, b.kind, b.info.as_str()))
            .collect();
        assert_eq!(
            noted,
            [
                (20, 466, 300, 499, Kind::Panel, "fight hud"),
                (23, 479, 151, 487, Kind::Panel, "hp bar FLUTE"),
                (24, 470, 53, 477, Kind::Label, "FLUTE"),
                (133, 470, 150, 477, Kind::Label, "100"),
                (24, 489, 35, 496, Kind::Label, "LV"),
                (169, 479, 297, 487, Kind::Panel, "hp bar TROMBONE"),
                (170, 470, 181, 477, Kind::Label, "50"),
                (249, 470, 296, 477, Kind::Label, "TROMBONE"),
                (268, 489, 279, 496, Kind::Label, "LV"),
                (154, 479, 165, 486, Kind::Label, "VS"),
            ]
        );
        // The left bar is full: its body in the fighter's colour, a lit top row, a shaded bottom row.
        assert_eq!(img.px(30, 482), &[222, 232, 246]);
        assert_eq!(img.px(30, 480), &[237, 242, 250]); // 222 * 0.55 + 115 = 237.1, ...
        assert_eq!(img.px(30, 485), &[144, 150, 159]); // 222 * 0.65 = 144.3, ...
        // The right bar is half full, from the right: the well on its left half.
        assert_eq!(img.px(200, 482), &[40, 30, 52]);
        assert_eq!(img.px(280, 482), &[120, 240, 184]);
        // The backing: 128 * 0.4 + ink * 0.32 = 55.04, 56.32, 60.8; outside it nothing.
        assert_eq!(img.px(22, 468), &[55, 56, 60]);
        assert_eq!(img.px(19, 480), &[128, 128, 128]);
        assert_eq!(img.px(300, 480), &[128, 128, 128]);
    }

    #[test]
    fn ability_rows_grow_the_hud_upward() {
        let (left, right) = pair();
        let rows_l = [
            AbilityRow::new("FLUTTER TONGUE"),
            AbilityRow::new("SNAKE DANCE"),
            AbilityRow::new("PICCOLO"),
        ];
        let mut lit = AbilityRow::new("SAD TROMBONE");
        lit.tag = "LV2".into();
        lit.unlocked = true;
        lit.glow = 1.0;
        let rows_r = [lit];
        let (mut img, mut log) = (grey(), DrawLog::new());
        let hud = fight_hud(
            &mut img,
            &mut log,
            &left,
            &right,
            Some([&rows_l, &rows_r]),
            &HudOptions::default(),
        );
        // Three rows: 18 px taller, still ending at row 496.
        assert_eq!(hud.top, 448);
        assert_eq!(hud.bars, [(24, 150, 462, 468), (170, 296, 462, 468)]);
        let rows = hud.rows.expect("rows");
        assert_eq!(rows[0], [(24, 471, 107, 478), (24, 480, 89, 487), (24, 489, 65, 496)]);
        // LV2 SAD TROMBONE: 4 * 6 + 12 * 6 - 1 = 95 px, ending at the bar's right end.
        assert_eq!(rows[1], [(201, 471, 296, 478)]);
        // The lit row has a pill, noted as a panel one pixel larger all round than the pill itself.
        let pill = log
            .boxes
            .iter()
            .find(|b| b.info == "ability SAD TROMBONE")
            .expect("the pill");
        assert_eq!(
            (pill.x0, pill.y0, pill.x1, pill.y1, pill.kind),
            (221, 469, 300, 481, Kind::Panel)
        );
        // Locked rows are grey and have no pill; without a tag there is one label a row.
        assert_eq!(log.boxes.iter().filter(|b| b.info.starts_with("ability ")).count(), 1);
        assert_eq!(log.boxes.iter().filter(|b| b.info == "PICCOLO").count(), 1);
        assert_eq!(img.px(24, 489), &ABIL_GREY[1]); // the P's stem
    }

    #[test]
    fn hp_is_clamped_and_shown_rounded_up() {
        let number = |hp: f64, ko: bool| {
            let (mut left, right) = pair();
            left.hp = hp;
            left.ko = ko;
            let (mut img, mut log) = (grey(), DrawLog::new());
            fight_hud(&mut img, &mut log, &left, &right, None, &HudOptions::default());
            log.boxes[3].info.clone()
        };
        assert_eq!(number(100.0, false), "100");
        assert_eq!(number(250.0, false), "100");
        assert_eq!(number(-3.0, false), "0");
        assert_eq!(number(0.5, false), "1");
        assert_eq!(number(1e-7, false), "0");
        assert_eq!(number(41.0000001, false), "41");
        assert_eq!(number(41.00001, false), "42");
        assert_eq!(number(41.0, true), "K.O.");
    }

    #[test]
    fn a_faded_hud_mixes_the_whole_picture() {
        let (left, right) = pair();
        let opts = HudOptions {
            alpha: 0.5,
            ..HudOptions::default()
        };
        let (mut img, mut log) = (grey(), DrawLog::new());
        fight_hud(&mut img, &mut log, &left, &right, None, &opts);
        // Half of the full bar's 222 over 128: 175. Far from the HUD: 128 * 0.5 + 128 * 0.5.
        assert_eq!(img.px(30, 482), &[175, 180, 187]);
        assert_eq!(img.px(5, 5), &[128, 128, 128]);
    }

    #[test]
    #[should_panic(expected = "hp_max is 0")]
    fn a_fighter_without_hp_max_is_refused() {
        let (mut left, right) = pair();
        left.hp_max = 0.0;
        fight_hud(
            &mut grey(),
            &mut DrawLog::new(),
            &left,
            &right,
            None,
            &HudOptions::default(),
        );
    }
}
