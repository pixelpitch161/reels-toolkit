//! The VS title card on bar 0: the port of `projects/ballfight/title.py`.
//!
//! The frame splits diagonally into two panels in the fighters' colours (the left fighter's at the top left, the
//! right fighter's at the bottom right), each with its fighter's ball big and close. The words slam in one per
//! beat: the left name, `VS` (it cracks the divider), the right name. On beat 4 `VS` pulses, then the panels
//! slide apart like doors along the cracked divider. In the last 0.3 s of the video the doors slide shut again
//! and the left name falls in as a ghost, so the slam on frame 0 is the loop's reset.
//!
//! ```no_run
//! # use bf_picture::title::{CardLook, Look, TitleCard};
//! # fn demo(show: &bf_core::show::Show, mesh: [&bf_picture::sprite::Mesh; 2], look: [&dyn Look; 2],
//! #         img: &mut bf_picture::Rgb8, t: f64) {
//! // colours and deep shades as floats (COL, DARK), each fighter's look, model and view (yaw and roll)
//! let looks = [0, 1].map(|k| CardLook { look: look[k], mesh: mesh[k], yaw: 205.0, roll: 30.0 });
//! let card = TitleCard::new(show, [[222.0, 232.0, 246.0], [120.0, 240.0, 184.0]],
//!                           [[120.0, 134.0, 162.0], [44.0, 150.0, 104.0]], looks, Some("BALLFIGHT #15"));
//! card.draw(img, t);                       // img = TT.draw(img, t): nothing while the doors are open
//! let shut = card.door_offset(t) == 0.0;   // the card covers the HUD: the renderer skips it
//! # }
//! ```
//!
//! # What is ported, and what is not
//!
//! Ported: the layout ([`layout`]), the panels with the TROMBONE's own pattern and the plain panel, the close-up
//! balls ([`panels`]), the words ([`words`]), the crack and the slam effects ([`fx`]), and here the card's
//! timing (`door_offset`, `slam_k`, `word_state`, `shake`), the series tag (`ui_tag`) and `draw`.
//!
//! Not ported, because the fight file has no setting for them: the three opt-ins an episode's timeline could
//! carry, `TITLE_LINES` (a long name on two lines), `TITLE_BALLS` (other places for the balls) and `MIDDLE`
//! (another word than `VS`, turned), with `rotate_mask`, `_fit_middle` and `_streaks_turned`. Without them the
//! Python card is exactly this one. Of the sixteen panel patterns in `PATTERNS` only the TROMBONE's is here: each
//! of the other fifteen comes with its fighter ([`panels::builtin_pattern`] is where it goes).
//!
//! # Numbers
//!
//! Everything is `f64` in Python's order of operations, with Python's `round` (ties to even) and `int`
//! (truncation). Python takes `exp`, `sin`, `cos` and `**` from the C runtime of the reference PC, the Studio from
//! `bf_core::dmath`, so a time-dependent value (the shake, the ball's bob and pulse, a glow, a door's offset
//! while closing) can differ from Python's in its last bit. That reaches a pixel only where the value decides
//! something within about 1e-13 of its threshold; on the 18 reference cards every byte is Python's
//! (`tests/title_frames.rs`).

pub mod fx;
pub mod layout;
pub mod panels;
pub mod words;

#[cfg(test)]
mod tests;

use bf_core::dmath;
use bf_core::grid::Grid;
use bf_core::py::{self, py_max, py_min};
use bf_core::show::Show;

use self::layout::{BALLS, Panel, Word};
use crate::config::{H, W};
use crate::font::{self, ADV};
use crate::img::{Img, Rgb8, RgbF};
use crate::np;
use crate::sprite::{self, Mesh};
use crate::ui::{DrawLog, Kind};

pub use self::layout::OPEN_PX;

/// `INK`: the card's dark ink, `np.array([16, 10, 26], float)`.
pub const INK: [f64; 3] = [16.0, 10.0, 26.0];
/// `GOLD`: the colour of `VS`, of the crack while it is hot and of the sparks.
pub const GOLD: [f64; 3] = [255.0, 226.0, 120.0];
/// The glass of a ball whose look has no tint of its own: `Look.glass`' default, the cream `(236, 232, 226)`.
pub const GLASS: [f64; 3] = [236.0, 232.0, 226.0];
/// The colour of the series tag, `np.array([255, 214, 96])`.
const TAG: [f64; 3] = [255.0, 214.0, 96.0];

/// What the card asks of a fighter's look: the three hooks of `kit/fighters/base.py` `Look` that `kit.picture`
/// hands to `title.py` (`PATTERNS`, `CLOSEUP` and `GLASS`), with the base class's defaults.
///
/// This is the minimal form of the look's trait: the ring renderer's task adds the other hooks to it.
pub trait Look {
    /// `title_pattern`: the fighter's panel pattern on the card. `base` is the panel's gradient, an `(H, W, 3)`
    /// `float64` picture not yet dithered; a look with a pattern changes it in place and returns `true`. The
    /// default (`title_pattern = None`) leaves it alone and returns `false`: the panel stays plain.
    ///
    /// A pattern reads the card's geometry from [`layout`]: [`layout::side`] is `title.SIDE`, [`layout::dd`] is
    /// `title.DD`, and `title.XX`, `title.YY` are the pixel's column and row.
    ///
    /// As in Python (`PATTERNS.setdefault(name, look.title_pattern)`), a pattern the card itself has for the
    /// fighter's name ([`panels::builtin_pattern`]) is used instead, and this hook is then not called.
    fn title_pattern(&self, base: &mut RgbF) -> bool {
        let _ = base;
        false
    }

    /// `title_closeup(r)`: the look's own close-up sprite for the card, RGBA and square, for a ball of radius `r`
    /// pixels; `None` (the default) for the fighter's Pixel Pitch model.
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        let _ = r;
        None
    }

    /// `glass`: the tint of the fighter's glass ball. The default is [`GLASS`].
    fn glass(&self) -> [f64; 3] {
        GLASS
    }
}

/// One fighter as the card takes it besides its colours: its look, and the model its close-up is drawn from
/// when the look has no close-up of its own (`sprites.render_sprite(CFG[k]['pp'], r, yc, roll)` with
/// `yc, sw, roll = CFG[k]['view']`).
#[derive(Clone, Copy)]
pub struct CardLook<'a> {
    pub look: &'a dyn Look,
    /// The fighter's Pixel Pitch model.
    pub mesh: &'a Mesh,
    /// The yaw of its view in degrees (`view[0]`), as it is: not rounded to the ring's six-degree steps.
    pub yaw: f64,
    /// The roll of its view in degrees (`view[2]`).
    pub roll: f64,
}

/// The VS title card of one episode: everything `title.py` builds once (the word layout, the two panels, the
/// close-up sprites, the crack), ready to draw at any time. Drawing never changes it, so frames may be drawn
/// from several threads at once.
#[derive(Clone, Debug)]
pub struct TitleCard {
    grid: Grid,
    /// `RD.COL`: the fighters' colours.
    colors: [[f64; 3]; 2],
    /// `GLASS.get(name, (236, 232, 226))` per fighter.
    glass: [[f64; 3]; 2],
    /// `T.SERIES`: the small tag at the top, or empty.
    series: String,
    /// `WORDS`, in the order of `T.TITLE`: the left name, `VS`, the right name.
    words: [Word; 3],
    /// Per side, the panel as every frame starts from it: `panels()[side]` with the divider's bright edge.
    panels: [RgbF; 2],
    /// Per side, the panel's opacity as every frame starts from it: 0.94 on its half, 1 on the bright edge.
    alpha: [Img<f64>; 2],
    /// `big_sprite(k)` per fighter.
    sprites: [Img<u8>; 2],
    /// `crack_pts()`.
    crack: Vec<Vec<(f64, f64)>>,
}

/// `_ease_in(u)`.
fn ease_in(u: f64) -> f64 {
    let u = py_min(1.0, py_max(0.0, u));
    u * u * u
}

/// `_ease_out(u)`: `1 - (1 - u) ** 3`. The cube is Python's `**`, the C runtime's `pow`, which is not the
/// product taken twice (they differ in the last bit for a quarter of all values); `dmath::pow` here.
fn ease_out(u: f64) -> f64 {
    let u = py_min(1.0, py_max(0.0, u));
    1.0 - dmath::pow(1.0 - u, 3.0)
}

/// `slam_k(t, t0)`: the size factor of a word that slams in at `t0`: 2.25 at `t0`, down to 1 in 0.16 s, a small
/// overshoot, settled (exactly 1) from 0.256 s; `None` before `t0`.
pub fn slam_k(t: f64, t0: f64) -> Option<f64> {
    let u = (t - t0) / 0.16;
    if u < 0.0 {
        return None;
    }
    if u < 1.0 {
        // 1 + 1.25 * (1 - u) ** 2: a Python float squared, a product here
        return Some(1.0 + 1.25 * ((1.0 - u) * (1.0 - u)));
    }
    if u < 1.6 {
        return Some(1.0 - 0.08 * dmath::sin(dmath::PI * (u - 1.0) / 0.6));
    }
    Some(1.0)
}

/// `dst * (1 - a) + src * a`: one sample laid over another at the opacity `a`, in the order numpy computes it.
/// The other common form, `dst + (src - dst) * a`, rounds differently and now and then gives the neighbouring
/// `float64`, which the cut to bytes can turn into another byte; every mix of the card that Python writes this
/// way goes through here.
pub(crate) fn over(dst: f64, src: f64, a: f64) -> f64 {
    dst * (1.0 - a) + src * a
}

/// Python's `int(round(x))` for a pixel count.
fn round(x: f64) -> i32 {
    np::f64_to_i32(py::round_half_even(x))
}

/// `door_offset(t)` on the timeline `g`, with [`OPEN_PX`] where Python returns `None`: see
/// [`TitleCard::door_offset`].
fn door_offset(g: &Grid, t: f64) -> f64 {
    if t >= g.dur - g.t_close {
        // the run-up: the doors slide shut, closed on the last frame
        let u = (t - (g.dur - g.t_close)) / (g.t_close - 1.0 / f64::from(g.fps));
        return OPEN_PX * (1.0 - ease_out(u));
    }
    if t < g.t_doors.0 {
        return 0.0;
    }
    if t < g.t_doors.1 {
        return OPEN_PX * ease_in((t - g.t_doors.0) / (g.t_doors.1 - g.t_doors.0));
    }
    OPEN_PX
}

/// `word_state(t, j)` on the timeline `g`: see [`TitleCard::word_state`].
fn word_state(g: &Grid, t: f64, j: usize) -> Option<(f64, f64)> {
    // the closing doors carry only the panels, then name A's ghost falls in (the frame-0 slam)
    if t >= g.dur - g.t_close && (j > 0 || t < g.dur - 0.12) {
        return None;
    }
    if j == 0 && t >= g.dur - 0.12 {
        let u = (t - (g.dur - 0.12)) / (0.12 - 1.0 / f64::from(g.fps));
        let u = py_min(1.0, u);
        // on the last frame: frame 0's size
        let k = if u >= 1.0 - 1e-6 { 2.25 } else { 2.25 + 0.6 * (1.0 - u) };
        return Some((k, u));
    }
    let k = slam_k(t, g.title[j])?;
    // only VS pulses in size on beat 4: the names already use the widest scale that fits the safe zone
    let pulse = if j == 1 && 3.0 * g.beat - 0.07 <= t && t < g.t_doors.0 + 0.05 {
        0.05 * g.beat_pulse(t, 0.12, 0.12)
    } else {
        0.0
    };
    Some((k + pulse, 1.0))
}

/// `shake(t)` on the timeline `g`: see [`TitleCard::shake`].
fn shake(g: &Grid, t: f64) -> (f64, f64) {
    let (mut dx, mut dy) = (0.0, 0.0);
    for (j, &t0) in g.title.iter().enumerate() {
        let u = t - t0 - 1.0 / f64::from(g.fps);
        if 0.0 < u && u < 0.35 {
            let a = (if j == 1 { 7.0 } else { 5.0 }) * dmath::exp(-u / 0.09);
            dx += a * dmath::sin(2.0 * dmath::PI * 23.0 * u);
            dy += a * 0.7 * dmath::sin(2.0 * dmath::PI * 29.0 * u + 1.0);
        }
    }
    (dx, dy)
}

/// The cover frame on the timeline `g`: see [`TitleCard::cover_frame`].
fn cover_frame(g: &Grid) -> usize {
    let fps = f64::from(g.fps);
    let still = |f: usize| {
        let t = f as f64 / fps;
        door_offset(g, t) == 0.0 && shake(g, t) == (0.0, 0.0)
    };
    let mut f = np::f64_to_i64((g.t_doors.0 * fps).ceil()).max(1) as usize - 1;
    while f > 0 && !still(f) {
        f -= 1;
    }
    f
}

/// One side of the card as [`TitleCard::new`] builds it: the panel and its opacity as every frame starts from
/// them, and the fighter's close-up sprite.
fn build_side(
    k: usize,
    name: &str,
    colour: [f64; 3],
    dark: [f64; 3],
    look: &CardLook<'_>,
) -> (RgbF, Img<f64>, Img<u8>) {
    // _panel_rgb(RD.COL[k], RD.DARK[k], side, RD.NAMES[k]): PATTERNS[name], if the card or the look has one
    let mut panel = panels::panel_rgb(colour, dark, |base| match panels::builtin_pattern(name) {
        Some(pattern) => pattern(base),
        None => {
            look.look.title_pattern(base);
        }
    });
    // What every frame does to a fresh copy of the panel before anything else, done once:
    //   alpha = ((SIDE > 0) if side == 0 else (SIDE <= 0)).astype(np.float64) * 0.94
    //   edge = np.abs(SIDE) < 2.2;  layer[edge] = np.clip(RD.COL[side] * 0.5 + 150, 0, 255);  alpha[edge] = 1.0
    let edge = colour.map(|c| np::clip(c * 0.5 + 150.0, 0.0, 255.0));
    let mut alpha: Img<f64> = Img::new(W, H, 1);
    for (i, &s) in layout::side().data.iter().enumerate() {
        let mine = if k == 0 { s > 0.0 } else { s <= 0.0 };
        alpha.data[i] = f64::from(u8::from(mine)) * 0.94;
        if s.abs() < 2.2 {
            panel.data[i * 3..i * 3 + 3].copy_from_slice(&edge);
            alpha.data[i] = 1.0;
        }
    }
    // big_sprite(k): the look's own close-up, else the model at its view's yaw and roll
    let r = np::f64_to_i32(f64::from(BALLS[k].r) * 0.8).max(0) as u32;
    let spr = look
        .look
        .title_closeup(r)
        .unwrap_or_else(|| sprite::render_sprite(look.mesh, r, look.yaw, look.roll));
    assert!(
        spr.c == 4 && spr.w == spr.h,
        "the close-up of {name} is {} x {} with {} channels: the card wants a square RGBA sprite",
        spr.w,
        spr.h,
        spr.c
    );
    (panel, alpha, spr)
}

/// A door laid over the picture, moved by `(dx, dy)` whole pixels:
/// `out[ys0 + dy:ys1 + dy, xs0 + dx:xs1 + dx] = dst * (1 - sa) + src * sa`. What the move pushes off the frame is
/// dropped.
fn lay_over(out: &mut RgbF, layer: &RgbF, alpha: &Img<f64>, dx: i32, dy: i32) {
    let (w, h) = (W as i32, H as i32);
    // xs0, xs1 = max(0, -dx), min(W, W - dx);  ys0, ys1 = max(0, -dy), min(H, H - dy)
    let (xs0, xs1) = ((-dx).max(0), w.min(w - dx));
    let (ys0, ys1) = ((-dy).max(0), h.min(h - dy));
    for y in ys0..ys1 {
        for x in xs0..xs1 {
            let sa = alpha.px(x as usize, y as usize)[0];
            let src = layer.px(x as usize, y as usize);
            for (d, &s) in out.px_mut((x + dx) as usize, (y + dy) as usize).iter_mut().zip(src) {
                *d = over(*d, s, sa);
            }
        }
    }
}

impl TitleCard {
    /// The card of a show: `show` gives the timeline (the beat, the bar, the times of the three words and of the
    /// doors) and the two names; `colors` and `darks` are the fighters' colours and deep shades as floats
    /// (`COL[k]`, `DARK[k]`); `looks` their looks and models; `series_tag` the small tag at the top of the card
    /// (`T.SERIES`; `None` or empty for none, as on every new episode).
    ///
    /// Everything Python builds on first use is built here: the layout, both panels, both close-up sprites and
    /// the crack.
    ///
    /// # Panics
    /// When a look's close-up sprite is not RGBA and square.
    pub fn new(
        show: &Show,
        colors: [[f64; 3]; 2],
        darks: [[f64; 3]; 2],
        looks: [CardLook<'_>; 2],
        series_tag: Option<&str>,
    ) -> TitleCard {
        let [(p0, a0, s0), (p1, a1, s1)] =
            [0, 1].map(|k| build_side(k, &show.names[k], colors[k], darks[k], &looks[k]));
        TitleCard {
            grid: show.grid.clone(),
            colors,
            glass: [looks[0].look.glass(), looks[1].look.glass()],
            series: series_tag.unwrap_or("").to_string(),
            words: layout::words(&show.names[0], &show.names[1]),
            panels: [p0, p1],
            alpha: [a0, a1],
            sprites: [s0, s1],
            crack: fx::crack_pts(),
        }
    }

    /// `WORDS`: where the three words rest, in the order they slam in.
    pub fn words(&self) -> &[Word; 3] {
        &self.words
    }

    /// `door_offset(t)`: how far the doors are open at `t`, in pixels along the divider's normal: 0 while the
    /// card is shut, up to [`OPEN_PX`] when the doors have left the frame. The doors open between the two times
    /// of `T_DOORS` and slide shut over the last `T_CLOSE` seconds (shut on the last frame).
    ///
    /// Between the two there is no card and Python returns `None`; this returns [`OPEN_PX`] there, the doors
    /// gone, which every reader takes the same way: [`TitleCard::draw`] draws nothing from `OPEN_PX - 1` up, and
    /// the renderer's "the card is shut" is `door_offset(t) == 0.0`.
    pub fn door_offset(&self, t: f64) -> f64 {
        door_offset(&self.grid, t)
    }

    /// `word_state(t, j)`: the size factor and the opacity of word `j` (0 the left name, 1 `VS`, 2 the right
    /// name) at `t`, or `None` while it is not drawn. `VS` swells a little on beat 4; the closing doors carry no
    /// words, but over the last 0.12 s the left name falls in as a ghost, on the last frame at frame 0's size.
    pub fn word_state(&self, t: f64, j: usize) -> Option<(f64, f64)> {
        word_state(&self.grid, t, j)
    }

    /// `shake(t)`: the card's own shake in pixels: a decaying jolt for 0.35 s after each slam (stronger for
    /// `VS`), from the frame after it; exactly `(0, 0)` on the slam frame itself and whenever no jolt runs.
    pub fn shake(&self, t: f64) -> (f64, f64) {
        shake(&self.grid, t)
    }

    /// The cover picture's frame: the last frame on which the card is shut (`door_offset` is 0) and does not
    /// shake, `ceil(T_DOORS[0] * FPS) - 1` stepped back while the card shakes. At the series' tempos that is the
    /// hold after the right name has landed. (At a tempo so fast that the doors open before the last jolt has
    /// died, from 199 BPM, every earlier frame but the first two shakes as well, and this is frame 1: the left
    /// name half-way through its slam.)
    pub fn cover_frame(&self) -> usize {
        cover_frame(&self.grid)
    }

    /// `img = TT.draw(img, t)`: the card over the finished frame, in place. Nothing is drawn while the doors are
    /// open. This is [`TitleCard::draw_logged`] without the draw log.
    pub fn draw(&self, img: &mut Rgb8, t: f64) {
        self.draw_logged(img, &mut DrawLog::new(), t);
    }

    /// [`TitleCard::draw`], filing in `log` what the Python files in `reelkit.ui.DRAWN` for the safe-zone check:
    /// one `Caption` box for each word that has settled on the shut card (the letters with their outline).
    ///
    /// The frame under the card is dimmed to 55 %, each door is laid over it (its panel at 94 %, the ball, the
    /// tag, the words and the crack opaque) at its place for `t`, shaken as one piece; over the shut card come
    /// the slam effects. The result is clipped to 0..255 and cut to bytes.
    pub fn draw_logged(&self, img: &mut Rgb8, log: &mut DrawLog, t: f64) {
        assert!(
            (img.w, img.h, img.c) == (W, H, 3),
            "the title card is drawn on a {W} x {H} RGB frame, not {} x {} with {} channels",
            img.w,
            img.h,
            img.c
        );
        let off = self.door_offset(t);
        if off >= OPEN_PX - 1.0 {
            return;
        }
        // out = img.astype(np.float64);  out *= 0.55: the arena dims behind the card
        let mut out: RgbF = Img::from_vec(W, H, 3, img.data.iter().map(|&v| f64::from(v) * 0.55).collect());
        let (sx, sy) = self.shake(t);
        let n = layout::normal();
        for side in 0..2 {
            let (layer, alpha) = self.door(side, t, off, log);
            // slide the door: a whole-pixel shift along its normal (and the shake)
            //   vec = N * off * (1 if side == 0 else -1) + np.array([sx, sy])
            let sgn = if side == 0 { 1.0 } else { -1.0 };
            let (dx, dy) = (round(n[0] * off * sgn + sx), round(n[1] * off * sgn + sy));
            lay_over(&mut out, &layer, &alpha, dx, dy);
        }
        if off == 0.0 && t < self.grid.dur - 1.0 {
            self.slam_effects(&mut out, t, sx, sy);
        }
        // out = np.clip(out, 0, 255);  out.astype(np.uint8)
        for (p, &v) in img.data.iter_mut().zip(&out.data) {
            *p = np::clip_u8(v);
        }
    }

    /// One door at `t`, before it is slid to its place: the panel of `side` with its ball, the tag (on the left
    /// door), its words and the crack, and the door's opacity. `off` is the doors' offset at `t`.
    fn door(&self, side: usize, t: f64, off: f64, log: &mut DrawLog) -> (RgbF, Img<f64>) {
        let g = &self.grid;
        let mut layer = self.panels[side].clone();
        let mut alpha = self.alpha[side].clone();
        let b = BALLS[side];
        // the ball flares as its fighter's name slams in, bobs once a bar and swells on the beat
        let ts = g.title[if side == 0 { 0 } else { 2 }];
        let flare = if ts <= t && t < g.dur - 1.0 {
            py_min(1.0, (t - ts) / 0.06) * dmath::exp(-py_max(0.0, t - ts - 0.06) / 0.2)
        } else {
            0.0
        };
        let gl = 0.25 + 0.5 * flare;
        let bob = 3.0 * dmath::sin(2.0 * dmath::PI * t / g.bar + side as f64 * dmath::PI);
        panels::draw_big_ball(
            &mut layer,
            &mut alpha,
            &self.sprites[side],
            self.glass[side],
            b.cx,
            f64::from(b.cy) + bob,
            f64::from(b.r) * (1.0 + 0.03 * g.beat_pulse(t, 0.15, 0.12)),
            gl,
            self.colors[side],
        );
        if side == 0 {
            self.tag(&mut layer, &mut alpha);
        }
        self.words_on(side, t, off, &mut layer, &mut alpha, log);
        if t < g.dur - 1.0 {
            fx::draw_crack(&mut layer, &mut alpha, g, t, &self.crack);
        }
        (layer, alpha)
    }

    /// The words of one door: each word of this panel that is up at `t`, stamped on a layer of its own and laid
    /// over the door. `VS` is on both doors, each keeping its half, so the word splits with them. A word that
    /// has settled on the shut card is filed in `log`, once.
    fn words_on(&self, side: usize, t: f64, off: f64, layer: &mut RgbF, alpha: &mut Img<f64>, log: &mut DrawLog) {
        let side_of = layout::side();
        for (j, word) in self.words.iter().enumerate() {
            if !word.panel.is_on(side) {
                continue;
            }
            let Some((k, a)) = self.word_state(t, j) else {
                continue;
            };
            let col = if j == 1 {
                GOLD
            } else {
                self.colors[j / 2].map(|c| np::clip(c * 0.85 + 50.0, 0.0, 255.0))
            };
            let settled = (k - 1.0).abs() < 0.06 && a >= 1.0 && off == 0.0;
            let (cx, cy) = (f64::from(word.cx), f64::from(word.cy));
            let stamped = words::stamp_word(W, H, &word.text, word.scale, k, cx, cy, col, a);
            let halved = word.panel == Panel::Both;
            // note=settled and (spec['side'] == side or side == 0)
            if settled && (!halved || side == 0) {
                let (x0, y0, x1, y1) = stamped.noted;
                log.note(x0, y0, x1, y1, Kind::Caption, &word.text);
            }
            let wl = &stamped.layer;
            for by in 0..wl.h {
                for bx in 0..wl.w {
                    let (x, y) = (wl.x + bx, wl.y + by);
                    let mut wa = wl.alpha[by * wl.w + bx];
                    if halved {
                        // wa = wa * ((SIDE > 0) if side == 0 else (SIDE <= 0)): keep only this side's half
                        let s = side_of.px(x, y)[0];
                        let mine = if side == 0 { s > 0.0 } else { s <= 0.0 };
                        wa *= f64::from(u8::from(mine));
                    }
                    if wa > 0.0 {
                        // layer[m] = layer[m] * (1 - wa) + wl[m] * wa;  alpha[m] = np.maximum(alpha[m], wa)
                        let src = &wl.rgb[(by * wl.w + bx) * 3..][..3];
                        for (p, &c) in layer.px_mut(x, y).iter_mut().zip(src) {
                            *p = over(*p, c, wa);
                        }
                        let ap = &mut alpha.px_mut(x, y)[0];
                        *ap = np::maximum(*ap, wa);
                    }
                }
            }
        }
    }

    /// The slam effects over the shut card, on the finished float picture: for each word that is up its light,
    /// its streaks and its dust, around the word as the card's shake `(sx, sy)` has moved it; then the sparks.
    fn slam_effects(&self, out: &mut RgbF, t: f64, sx: f64, sy: f64) {
        let g = &self.grid;
        for (j, word) in self.words.iter().enumerate() {
            if self.word_state(t, j).is_none() {
                continue;
            }
            let col = if j == 1 { GOLD } else { self.colors[j / 2] };
            // w_px = len(text) * ADV * scale;  h_px = 7 * scale
            let w_px = word.text.chars().count() as i32 * ADV * word.scale as i32;
            let h_px = 7 * word.scale as i32;
            let (cx, cy) = (f64::from(word.cx) + sx, f64::from(word.cy) + sy);
            fx::light(out, g, t, j, cx, cy, col);
            fx::streaks(out, g, t, j, cx, cy, w_px, h_px, col);
            fx::dust(out, g, t, j, cx, cy, w_px, h_px);
        }
        fx::sparks(out, g, t);
    }

    /// `ui_tag(layer, alpha)`: the small series tag at the top of the card, centred on column 180 at row 100:
    /// scale-1 text in pale gold on a 1 px ink outline, opaque. Without a tag nothing is drawn.
    ///
    /// A tag of 60 characters or more is wider than the frame; Python's slices then no longer match the mask and
    /// it raises. Here the part of such a tag that lies on the frame is drawn.
    fn tag(&self, layer: &mut RgbF, alpha: &mut Img<f64>) {
        if self.series.is_empty() {
            return;
        }
        let m = font::text_mask(&self.series, 1);
        let o = font::dilate(&m, 1);
        // x0, y0 = int(round(180 - w_ / 2)), 100
        let (x0, y0) = (round(180.0 - m.w as f64 / 2.0), 100);
        for (mask, c, rr) in [(&o, INK, 1), (&m, TAG, 0)] {
            for my in 0..mask.h {
                for (mx, &on) in mask.row(my).iter().enumerate() {
                    let (x, y) = (x0 - rr + mx as i32, y0 - rr + my as i32);
                    if on != 0 && (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
                        layer.px_mut(x as usize, y as usize).copy_from_slice(&c);
                        alpha.px_mut(x as usize, y as usize)[0] = 1.0;
                    }
                }
            }
        }
    }
}
