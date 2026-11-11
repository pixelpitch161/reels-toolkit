//! Where things are on the closed card: the diagonal divider, the two close-up balls and the three words.
//!
//! The port of `title.py`'s module-level geometry (`DA`, `DB`, `DD`, `N`, `SIDE`, `OPEN_PX`, `BALLS`) and of
//! `_fit_scale` and `_layout`. A fighter's panel pattern reads the geometry the way the Python patterns read
//! `title.XX`, `title.YY`, `title.SIDE` and `title.DD`: `XX` and `YY` are a pixel's column and row as `f64`,
//! [`side`] and [`dd`] are here.

use std::sync::OnceLock;

use bf_core::py::{self, py_min};

use crate::config::{H, W};
use crate::font;
use crate::img::Img;

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

/// One entry of `BALLS`: a fighter's ball on the card, big and close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ball {
    pub cx: i32,
    pub cy: i32,
    pub r: i32,
}

/// `BALLS`: the left fighter's ball, then the right fighter's.
pub const BALLS: [Ball; 2] = [
    Ball { cx: 78, cy: 222, r: 56 },
    Ball {
        cx: 282,
        cy: 396,
        r: 56,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    /// The pixel font's scale.
    pub scale: u32,
    pub cx: i32,
    pub cy: i32,
    pub panel: Panel,
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

/// `_layout()`: the three words in the order they slam in: the left fighter's name centred at column 180 near the
/// top (below the series tag), `VS` on the divider, the right fighter's name at the bottom, left of the button
/// column (moved left to a multiple of 10 if it does not fit centred). Each name is as big as fits the safe box.
pub fn words(name_a: &str, name_b: &str) -> [Word; 3] {
    let sa = fit_scale(name_a, 320);
    let (ha, ra) = (7 * sa as i32, outline(sa));
    // int(max(134, round(109 + ha / 2 + ra)))
    let cya = (py::round_half_even(109.0 + f64::from(ha) / 2.0 + f64::from(ra)) as i32).max(134);

    let sb = fit_scale(name_b, 280);
    let wb = font::text_mask(name_b, sb).w as f64;
    let (hb, rb) = (f64::from(7 * sb as i32), f64::from(outline(sb)));
    // 180 if 180 + wb / 2 + rb <= 300 else int((300 - wb / 2 - rb) // 10 * 10)
    let cxb = if 180.0 + wb / 2.0 + rb <= 300.0 {
        180
    } else {
        (py::floor_div(300.0 - wb / 2.0 - rb, 10.0) * 10.0) as i32
    };
    // int(min(468, 500 - hb / 2 - rb - 3))
    let cyb = py_min(468.0, 500.0 - hb / 2.0 - rb - 3.0) as i32;
    [
        Word {
            text: name_a.to_string(),
            scale: sa,
            cx: 180,
            cy: cya,
            panel: Panel::Left,
        },
        Word {
            text: "VS".to_string(),
            scale: VS_SCALE,
            cx: 180,
            cy: 300,
            panel: Panel::Both,
        },
        Word {
            text: name_b.to_string(),
            scale: sb,
            cx: cxb,
            cy: cyb,
            panel: Panel::Right,
        },
    ]
}
