//! Where things are on the closed card: the diagonal divider, the two close-up balls and the three words.
//!
//! The port of `title.py`'s module-level geometry (`DA`, `DB`, `DD`, `N`, `SIDE`, `OPEN_PX`, `BALLS`), of
//! `_fit_scale` and `_layout`, and of the card's three opt-ins (`title.py` 18-26, 61-171): `TITLE_LINES` (a long
//! name stacked on two lines), `TITLE_BALLS` (the big balls moved to make room) and `MIDDLE` (another word than
//! `VS`, turned, fitted by `_fit_middle`). [`Layout`] is the result for one card: the three words and the two
//! balls, which [`Layout::new`] reads from the fight file's `picture` settings.
//!
//! A fighter's panel pattern reads the geometry the way the Python patterns read `title.XX`, `title.YY`,
//! `title.SIDE`, `title.DD` and `title.BALLS`: `XX` and `YY` are a pixel's column and row as `f64`, [`side`] and
//! [`dd`] are here, and the balls are [`Layout::balls`] (a pattern is handed the card's layout).

use std::sync::OnceLock;

use bf_core::dmath;
use bf_core::fightfile::{Middle, PictureSeeds};
use bf_core::py::{self, py_min};

use super::panels::TitleError;
use super::words;
use crate::config::{H, SAFE, SAFE_BUTTONS, W};
use crate::font;
use crate::img::Img;
use crate::np;

/// `DA`: the divider's lower end, on the left edge of the frame.
pub const DA: [f64; 2] = [0.0, 470.0];
/// `DB`: the divider's upper end, on the right edge.
pub const DB: [f64; 2] = [360.0, 170.0];
/// `OPEN_PX`: how far each door travels along the divider's normal to clear the frame.
pub const OPEN_PX: f64 = 560.0;
/// `MAX_NAME_SCALE`: the largest font scale of a name (`VS`, at 12, stays the largest word).
pub const MAX_NAME_SCALE: u32 = 7;
/// The font scale of `VS`.
pub const VS_SCALE: u32 = 12;

/// `DD = (DB - DA) / np.linalg.norm(DB - DA)`: the unit vector along the divider, from the bottom left to the top
/// right. The norm is the square root of a whole number (219,600), so there is one way to round it.
pub fn dd() -> [f64; 2] {
    let d = [DB[0] - DA[0], DB[1] - DA[1]];
    let norm = (d[0] * d[0] + d[1] * d[1]).sqrt();
    [d[0] / norm, d[1] / norm]
}

/// `N`: the divider's unit normal, pointing to the top left (the left fighter's side).
pub fn normal() -> [f64; 2] {
    let dd = dd();
    let n = [dd[1], -dd[0]];
    // if (np.array([0.0, 0.0]) - DA) @ N < 0: N = -N
    if (0.0 - DA[0]) * n[0] + (0.0 - DA[1]) * n[1] < 0.0 {
        [-n[0], -n[1]]
    } else {
        n
    }
}

/// `SIDE = (XX - DA[0]) * N[0] + (YY - DA[1]) * N[1]`: every pixel's distance from the divider in pixels, above
/// zero on the left fighter's half (the top left), one `float64` per pixel.
pub fn side() -> &'static Img<f64> {
    static TABLE: OnceLock<Img<f64>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let n = normal();
        let mut data = Vec::with_capacity(W * H);
        for y in 0..H {
            for x in 0..W {
                data.push((x as f64 - DA[0]) * n[0] + (y as f64 - DA[1]) * n[1]);
            }
        }
        Img::from_vec(W, H, 1, data)
    })
}

/// `DIVIDER_DEG = math.degrees(math.atan2(470.0 - 170.0, 360.0))`: the crack's own angle, about 39.8 degrees
/// anticlockwise, as Python computes it on the reference PC (`0x1.3e71cf41bf0e5p+5`).
///
/// This one value is kept as Python's bits rather than computed: the C runtime's `atan2(300, 360)` is
/// `0x1.63b4bc3f16a8ap-1` and `dmath::atan2` gives the next double up (`...8bp-1`, which is also the C runtime's
/// `atan(5 / 6)`), and `math.degrees` (a product with `180 / pi`) carries the difference into the angle's last bit.
/// The angle turns the middle word's letters, which a last bit can move by a pixel, and its arguments never
/// change, so the constant is exactly the kit's. The test `the_divider_angle_is_pythons` holds both values.
pub const DIVIDER_DEG: f64 = f64::from_bits(0x4043_e71c_f41b_f0e5);

/// [`DIVIDER_DEG`].
pub fn divider_deg() -> f64 {
    DIVIDER_DEG
}

/// `math.radians`: CPython multiplies by the constant `pi / 180`.
pub(crate) fn radians(deg: f64) -> f64 {
    deg * (dmath::PI / 180.0)
}

/// One entry of `BALLS`: a fighter's ball on the card, big and close. The card's own two are whole numbers; the
/// fight file's `title_balls` (the kit's `TITLE_BALLS`, whole numbers in every kit config) are read as `f64`,
/// and everything the card does with them is Python's arithmetic on the value, whole or not (see
/// [`Layout::new`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
}

/// `BALLS`: the left fighter's ball, then the right fighter's, when the fight file does not move them.
pub const BALLS: [Ball; 2] = [
    Ball {
        cx: 78.0,
        cy: 222.0,
        r: 56.0,
    },
    Ball {
        cx: 282.0,
        cy: 396.0,
        r: 56.0,
    },
];

/// The `side` of a word in `WORDS`: the panel it is drawn on. `VS` (-1 in Python) is on both, each panel keeping
/// its own half, so the word splits with the doors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    /// 0: the left fighter's panel, above the divider.
    Left,
    /// 1: the right fighter's panel, below it.
    Right,
    /// -1: both.
    Both,
}

impl Panel {
    /// `spec['side'] in (side, -1)`: whether the word is drawn on panel `side` (0 left, 1 right).
    pub fn is_on(self, side: usize) -> bool {
        match self {
            Panel::Left => side == 0,
            Panel::Right => side == 1,
            Panel::Both => true,
        }
    }
}

/// One entry of `WORDS`: a word of the card where it rests, centred on `(cx, cy)`.
///
/// `text` is the word's entry in `T.TITLE` (a fighter's name, or `VS`); what is stamped is `text` itself, or its
/// two `lines` (a name on two lines), or the `middle` word (in place of `VS`).
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    /// The pixel font's scale.
    pub scale: u32,
    pub cx: f64,
    pub cy: f64,
    pub panel: Panel,
    /// `spec['lines']` and `spec['dy']` (the kit's `TITLE_LINES`): each line and its offset from `cy`, stamped
    /// at `cy + dy * k` for the slam's size factor `k`.
    pub lines: Option<[(String, f64); 2]>,
    /// `spec['word']` and `spec['ang']` (the kit's `MIDDLE`): the word stamped in place of `VS` and its turn in
    /// degrees anticlockwise. An angle of 0 is Python's falsy angle: the word is stamped upright, and the
    /// effects and the sparks take the upright paths.
    pub middle: Option<(String, f64)>,
    /// `spec['w']`: the width the slam effects use, where the spec has one (two lines, the middle word);
    /// otherwise they use `len(text) * ADV * scale`.
    pub w: Option<i32>,
    /// `spec['h']`: the height the slam effects use, where the spec has one; otherwise `7 * scale`.
    pub h: Option<i32>,
}

impl Word {
    /// A word stamped as its own text.
    fn plain(text: &str, scale: u32, cx: f64, cy: f64, panel: Panel) -> Word {
        Word {
            text: text.to_string(),
            scale,
            cx,
            cy,
            panel,
            lines: None,
            middle: None,
            w: None,
            h: None,
        }
    }

    /// `spec.get('ang')` as Python tests it: the middle word's angle when it is not 0.
    pub fn turn(&self) -> Option<f64> {
        self.middle.as_ref().map(|m| m.1).filter(|&a| a != 0.0)
    }

    /// `w_px = spec.get('w', len(text) * ADV * spec['scale'])`: the width the slam effects use.
    pub fn effect_w(&self) -> i32 {
        self.w
            .unwrap_or(self.text.chars().count() as i32 * font::ADV * self.scale as i32)
    }

    /// `h_px = spec.get('h', 7 * spec['scale'])`: the height the slam effects use.
    pub fn effect_h(&self) -> i32 {
        self.h.unwrap_or(7 * self.scale as i32)
    }
}

/// `max(2, int(round(sc * 0.6)))`: the thickness of a word's outline at font scale `sc`.
pub fn outline(scale: u32) -> i32 {
    (py::round_half_even(f64::from(scale) * 0.6) as i32).max(2)
}

/// `_fit_scale(text, avail)`: the biggest scale up to [`MAX_NAME_SCALE`] at which the word and its outline are at
/// most `avail` pixels wide; 2 if none is.
pub fn fit_scale(text: &str, avail: i32) -> u32 {
    for sc in (2..=MAX_NAME_SCALE).rev() {
        let w = font::text_mask(text, sc).w as i32 + 2 * outline(sc);
        if w <= avail {
            return sc;
        }
    }
    2
}

/// `text_mask(text, scale).shape[1]`.
fn mask_w(text: &str, scale: u32) -> i32 {
    font::text_mask(text, scale).w as i32
}

/// `_layout()` without opt-ins: the three words in the order they slam in: the left fighter's name centred at
/// column 180 near the top (below the series tag), `VS` on the divider, the right fighter's name at the bottom,
/// left of the button column (moved left to a multiple of 10 if it does not fit centred). Each name is as big as
/// fits the safe box.
pub fn words(name_a: &str, name_b: &str) -> [Word; 3] {
    let none = std::collections::BTreeMap::new();
    let [a, b] = name_words(name_a, name_b, &none);
    [a, vs(), b]
}

/// `out['VS']` of `_layout()`: `VS` at scale 12 on the divider.
fn vs() -> Word {
    Word::plain("VS", VS_SCALE, 180.0, 300.0, Panel::Both)
}

/// The two names of `_layout()`, each on one line or, where `lines` has it (the kit's `TITLE_LINES`), on two.
fn name_words(name_a: &str, name_b: &str, lines: &std::collections::BTreeMap<String, (String, String)>) -> [Word; 2] {
    let a = match lines.get(name_a) {
        // #38: a long name on two lines, each at the scale that fits the longest
        Some((l1, l2)) => {
            let sa = fit_scale(l1, 320).min(fit_scale(l2, 320));
            let (ha, ra) = (7 * sa as i32, outline(sa));
            // gap = ha + 2 * ra - 2: the line pitch, the outlines just touching
            let gap = ha + 2 * ra - 2;
            // cy0 = int(max(134, round(109 + ha / 2 + ra)))
            let cy0 = (py::round_half_even(109.0 + f64::from(ha) / 2.0 + f64::from(ra)) as i32).max(134);
            let half = f64::from(gap) / 2.0;
            Word {
                lines: Some([(l1.clone(), -half), (l2.clone(), half)]),
                w: Some(mask_w(l1, sa).max(mask_w(l2, sa))),
                h: Some(ha + gap),
                ..Word::plain(name_a, sa, 180.0, f64::from(cy0) + half, Panel::Left)
            }
        }
        None => {
            let sa = fit_scale(name_a, 320);
            let (ha, ra) = (7 * sa as i32, outline(sa));
            // int(max(134, round(109 + ha / 2 + ra)))
            let cya = (py::round_half_even(109.0 + f64::from(ha) / 2.0 + f64::from(ra)) as i32).max(134);
            Word::plain(name_a, sa, 180.0, f64::from(cya), Panel::Left)
        }
    };
    // name B: 180 if 180 + wb / 2 + rb <= 300 else int((300 - wb / 2 - rb) // 10 * 10)
    let cx_of = |wb: f64, rb: f64| {
        if 180.0 + wb / 2.0 + rb <= 300.0 {
            180
        } else {
            (py::floor_div(300.0 - wb / 2.0 - rb, 10.0) * 10.0) as i32
        }
    };
    let b = match lines.get(name_b) {
        // #40: name B on two lines too; the lower line sits where a one-line name would
        Some((l1, l2)) => {
            let sb = fit_scale(l1, 280).min(fit_scale(l2, 280));
            let wb = mask_w(l1, sb).max(mask_w(l2, sb));
            let (hb, rb) = (7 * sb as i32, outline(sb));
            let gap = hb + 2 * rb - 2;
            let cxb = cx_of(f64::from(wb), f64::from(rb));
            // cyb = int(min(468, 500 - hb / 2 - rb - 3)) - gap / 2
            let half = f64::from(gap) / 2.0;
            let cyb = f64::from(py_min(468.0, 500.0 - f64::from(hb) / 2.0 - f64::from(rb) - 3.0) as i32) - half;
            Word {
                lines: Some([(l1.clone(), -half), (l2.clone(), half)]),
                w: Some(wb),
                h: Some(hb + gap),
                ..Word::plain(name_b, sb, f64::from(cxb), cyb, Panel::Right)
            }
        }
        None => {
            let sb = fit_scale(name_b, 280);
            let wb = f64::from(mask_w(name_b, sb));
            let (hb, rb) = (f64::from(7 * sb as i32), f64::from(outline(sb)));
            let cxb = cx_of(wb, rb);
            // int(min(468, 500 - hb / 2 - rb - 3))
            let cyb = py_min(468.0, 500.0 - hb / 2.0 - rb - 3.0) as i32;
            Word::plain(name_b, sb, f64::from(cxb), f64::from(cyb), Panel::Right)
        }
    };
    [a, b]
}

/// Python's `int(x)` for a pixel bound (truncation toward zero).
fn int(x: f64) -> i32 {
    np::f64_to_i32(x)
}

/// `_fit_middle(words)`: the middle word's place: centred on the crack's centre (180, 320), at the biggest scale
/// up to 12 whose turned letters, outline and drop shadow stay inside the safe zone, out of the button column and
/// clear of both names (each name's box grown by its outline and 4 px), slid along its own direction by up to
/// 40 px in steps of 4 if that lets it grow.
///
/// # Errors
/// [`TitleError::MiddleDoesNotFit`] when no scale fits (Python's `ValueError`), or when the word has no letter
/// the font draws (Python fails on the empty mask).
fn fit_middle(word: &str, turn: Middle, names: &[Word; 2]) -> Result<Word, TitleError> {
    let deg = match turn {
        Middle::Divider => divider_deg(),
        Middle::Degrees(d) => d,
    };
    let (cx, cy) = (180.0, 320.0);
    // keep: the allowed pixels
    let mut keep = vec![false; W * H];
    let (x0, y0, x1, y1) = SAFE;
    let fill = |keep: &mut [bool], ys: (i32, i32), xs: (i32, i32), on: bool| {
        // a numpy slice keep[y0:y1, x0:x1] of bounds that are never negative here
        let (ya, yb) = (ys.0.clamp(0, H as i32), ys.1.clamp(0, H as i32));
        let (xa, xb) = (xs.0.clamp(0, W as i32), xs.1.clamp(0, W as i32));
        for y in ya..yb {
            for x in xa..xb {
                keep[y as usize * W + x as usize] = on;
            }
        }
    };
    fill(&mut keep, (y0, y1), (x0, x1), true);
    fill(&mut keep, (SAFE_BUTTONS.1, H as i32), (SAFE_BUTTONS.0, W as i32), false);
    for sp in names {
        // the name's box, with its outline and a little air
        let wn = f64::from(sp.w.unwrap_or_else(|| mask_w(&sp.text, sp.scale)));
        let hn = f64::from(sp.h.unwrap_or(7 * sp.scale as i32));
        let rn = f64::from(outline(sp.scale) + 4);
        let ys = (int(sp.cy - hn / 2.0 - rn).max(0), int(sp.cy + hn / 2.0 + rn));
        let xs = (int(sp.cx - wn / 2.0 - rn).max(0), int(sp.cx + wn / 2.0 + rn));
        fill(&mut keep, ys, xs, false);
    }
    let a = radians(deg);
    // sorted(range(-40, 41, 4), key=abs): 0, -4, 4, -8, 8, ...
    let mut slides: Vec<i32> = (-40..=40).step_by(4).collect();
    slides.sort_by_key(|s| s.abs());
    let does_not_fit = || TitleError::MiddleDoesNotFit(word.to_string());
    for sc in (2..=12).rev() {
        let masks = words::word_masks_at(word, sc, 1.0, deg);
        let o = &masks.outline;
        let (hm, wm) = (o.h as f64, o.w as f64);
        let set: Vec<(i32, i32)> = (0..o.h)
            .flat_map(|y| {
                (0..o.w)
                    .filter(move |&x| o.data[y * o.w + x] != 0)
                    .map(move |x| (x as i32, y as i32))
            })
            .collect();
        if set.is_empty() {
            return Err(does_not_fit());
        }
        for &sl in &slides {
            let px = cx + f64::from(sl) * dmath::cos(a);
            let py = cy - f64::from(sl) * dmath::sin(a);
            let (bx, by) = (
                py::round_half_even(px - wm / 2.0) as i32,
                py::round_half_even(py - hm / 2.0) as i32,
            );
            let fits = [(0, 0), (3, 4)].iter().all(|&(ddx, ddy)| {
                set.iter().all(|&(xs, ys)| {
                    let (x, y) = (xs + bx + ddx, ys + by + ddy);
                    (0..W as i32).contains(&x) && (0..H as i32).contains(&y) && keep[y as usize * W + x as usize]
                })
            });
            if fits {
                return Ok(Word {
                    middle: Some((word.to_string(), deg)),
                    w: Some(mask_w(word, sc)),
                    h: Some(7 * sc as i32),
                    ..Word::plain("VS", sc, px, py, Panel::Both)
                });
            }
        }
    }
    Err(does_not_fit())
}

/// What the card puts where: `WORDS` and `BALLS` of `title.py` for one card.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// `WORDS`, in the order of `T.TITLE`: the left name, `VS` (or the middle word), the right name.
    pub words: [Word; 3],
    /// `BALLS`: the left fighter's big ball, then the right fighter's.
    pub balls: [Ball; 2],
}

impl Layout {
    /// The card without opt-ins: [`words`] and the card's own [`BALLS`].
    pub fn plain(name_a: &str, name_b: &str) -> Layout {
        Layout {
            words: words(name_a, name_b),
            balls: BALLS,
        }
    }

    /// The card of the fight file's `picture` settings: `title_lines` stack a fighter's name on two lines
    /// (`TITLE_LINES`), `title_balls` move the big balls (`TITLE_BALLS`), `middle` replaces `VS` with another
    /// word, turned (`MIDDLE`). Without them this is [`Layout::plain`], the card every existing fight draws.
    ///
    /// The kit's `TITLE_BALLS` are Python ints, the fight file's are `f64`. Everything the card does with a ball
    /// is Python's arithmetic on the value: `int(cx)` and `int(round(cx))` for the pixel bounds and the
    /// sprite's corner, `int(r * 0.8)` for the close-up's radius, float arithmetic for the rest (`cy + bob`,
    /// `r * (1 + pulse)`, the distances from the centre). For a whole number each of these gives the same value
    /// from the int as from its float, so a fight file's `74.0` draws the kit's `74`; the reference cards prove
    /// it (`tests/title_optins.rs`).
    ///
    /// # Errors
    /// [`TitleError::MiddleDoesNotFit`] when the middle word fits at no scale (the kit's `ValueError`).
    pub fn new(name_a: &str, name_b: &str, seeds: &PictureSeeds) -> Result<Layout, TitleError> {
        let [a, b] = name_words(name_a, name_b, &seeds.title_lines);
        let middle = match &seeds.middle {
            Some((word, turn)) => fit_middle(word, *turn, &[a.clone(), b.clone()])?,
            None => vs(),
        };
        let balls = match seeds.title_balls {
            Some(bs) => bs.map(|[cx, cy, r]| Ball { cx, cy, r }),
            None => BALLS,
        };
        Ok(Layout {
            words: [a, middle, b],
            balls,
        })
    }

    /// The radius of fighter `k`'s close-up sprite: `int(BALLS[k]['r'] * 0.8)`.
    pub fn closeup_r(&self, k: usize) -> i64 {
        np::f64_to_i64(self.balls[k].r * 0.8)
    }
}
