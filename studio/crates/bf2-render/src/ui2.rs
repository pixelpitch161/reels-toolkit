//! Engine 2's UI layer (milestone E5): the current engine's count-in, HP numbers, fight HUD, captions and VS card,
//! drawn by the same `bf-picture` code at 360 x 640 from the video-clock show and shadow recording, and matted for
//! the GPU composite ([`crate::overlay`]).
//!
//! The order is v1's (`ring/frame.rs`): the count-in, then (unless the card is shut) the HP numbers on the balls and
//! the fight HUD, then the captions, then the card over everything.

use bf_core::recording::{Array, Recording};
use bf_core::show::Show;
use bf_picture::config::{SAFE, ease, ss};
use bf_picture::font::{self, ADV};
use bf_picture::img::{Img, Rgb8};
use bf_picture::ring::hud::{SWEEP_T, countdown, draw_show_captions};
use bf_picture::title::TitleCard;
use bf_picture::ui::{self, AbilityRow, DrawLog, Fighter, HudOptions};
use bf2_direct::project::{ball_box, ball_centre, project};
use bf2_direct::{CameraTrack, TimeMap, to_render};
use bf2_sim::{FighterDef, Recording2, Tag};

use crate::overlay::{Overlay, matte, opaque};

/// The UI layer's size: the current engine's picture, 3x under the 1080 x 1920 video.
pub const UI_W: usize = 360;
pub const UI_H: usize = 640;
/// The video's scale over the UI layer.
const UP: f64 = 3.0;
/// Frames per second of the video.
const FPS: f64 = 30.0;

/// How long before the card's doors start to shut the picture starts dimming toward the card's own dim
/// ([`CARD_DIM`]), so the loop's run-up has no one-frame jump.
pub const PRE_CLOSE_DIM_S: f64 = 0.5;
/// How much the card dims the arena behind it (`TitleCard::draw_logged` keeps 0.55 of it).
const CARD_DIM: f64 = 0.45;

/// The share of the picture dimmed at `t` in the run-up to the card's close: 0 until [`PRE_CLOSE_DIM_S`] before
/// the doors start to move, rising evenly to [`CARD_DIM`] on the last frame before the card dims it itself.
fn pre_close_dim(g: &bf_core::grid::Grid, t: f64) -> f64 {
    let close = g.dur - g.t_close;
    let start = close - PRE_CLOSE_DIM_S;
    if t <= start || t > close + 0.5 / FPS {
        return 0.0;
    }
    CARD_DIM * ((t - start) / PRE_CLOSE_DIM_S).min(1.0)
}

/// Python's `int(round(x))` for a pixel coordinate: the nearest whole number, ties to even.
fn round_i32(x: f64) -> i32 {
    bf_core::py::round_half_even(x) as i32
}

/// One row of a fighter's HUD ability list: a move it unlocks by using it.
#[derive(Clone, Debug, PartialEq)]
pub struct RowDef {
    /// `"SP1"`, `"SP2"` or `"LOW"`.
    pub tag: &'static str,
    /// The move's name, as the shadow's `ability` events carry it.
    pub name: String,
    /// The move's index in the fighter's definition.
    pub mv: usize,
    /// How long a use stays lit: the whole move, `(windup + active + recovery)·dt`, in seconds.
    pub hold: f64,
}

/// The HUD rows of `def`: up to two `Special` moves in definition order (a `Low` move is not one of them), then
/// its `Low` move.
pub fn rows_of(def: &FighterDef, dt: f64) -> Vec<RowDef> {
    let row = |tag, (mv, m): (usize, &bf2_sim::MoveDef)| RowDef {
        tag,
        name: m.name.to_string(),
        mv,
        hold: f64::from(m.windup + m.active + m.recovery) * dt,
    };
    let moves = def.moves.iter().enumerate();
    let mut rows: Vec<RowDef> = moves
        .clone()
        .filter(|(_, m)| m.has(Tag::Special) && !m.has(Tag::Low))
        .take(2)
        .zip(["SP1", "SP2"])
        .map(|(m, tag)| row(tag, m))
        .collect();
    rows.extend(moves.filter(|(_, m)| m.has(Tag::Low)).take(1).map(|m| row("LOW", m)));
    rows
}

/// A row's uses: when it was first used (its unlock) and every use, in video seconds.
#[derive(Clone, Debug, Default)]
struct Uses {
    unlock: Option<f64>,
    all: Vec<f64>,
}

/// The UI layer of an Engine 2 fight, frame by frame.
pub struct Ui2<'a> {
    show: &'a Show,
    shadow: &'a Recording,
    rec: &'a Recording2,
    map: &'a TimeMap,
    cams: &'a CameraTrack,
    rows: [Vec<RowDef>; 2],
    uses: [Vec<Uses>; 2],
    colours: [[f64; 3]; 2],
    card: &'a TitleCard,
    /// `HP` rows of the shadow (frames from FIGHT!).
    hp: Vec<[f64; 2]>,
}

impl<'a> Ui2<'a> {
    /// The UI of the fight `rec` shown through `map` and `cams`, from the video-clock `show` and `shadow`
    /// (`bf_content::build_soundtrack2`'s), with each fighter's HUD `rows` and colours (sRGB, 0 to 255) and the VS
    /// `card`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        show: &'a Show,
        shadow: &'a Recording,
        rec: &'a Recording2,
        map: &'a TimeMap,
        cams: &'a CameraTrack,
        rows: [Vec<RowDef>; 2],
        colours: [[f64; 3]; 2],
        card: &'a TitleCard,
    ) -> Ui2<'a> {
        let uses = std::array::from_fn(|k| {
            rows[k]
                .iter()
                .map(|r| {
                    let mut u = Uses::default();
                    for e in shadow.events.iter().filter(|e| e.kind == "ability") {
                        if e.i(2) == k as i64 && e.s(3) == r.name {
                            u.unlock.get_or_insert(e.t);
                            u.all.push(e.t);
                        }
                    }
                    u
                })
                .collect()
        });
        let hp = match shadow.arrays.get("HP") {
            Some(Array::F64 { shape, data }) if shape.len() == 2 && shape[1] == 2 => data.as_chunks::<2>().0.to_vec(),
            _ => Vec::new(),
        };
        Ui2 {
            show,
            shadow,
            rec,
            map,
            cams,
            rows,
            uses,
            colours,
            card,
            hp,
        }
    }

    /// The shadow's HP frame at `t`: `round((t − T_FIGHT)·30)`, clamped to the frames there are.
    fn hp_frame(&self, k: usize, i: usize) -> f64 {
        self.hp
            .get(i.min(self.hp.len().saturating_sub(1)))
            .map_or(100.0, |p| p[k])
    }

    /// `hp_at(k, t)`, as the current engine's picture reads it: 100 before FIGHT!, the recorded HP to the K.O.,
    /// then the loser's 0 and the winner's last HP refilling to 100 on the glide home.
    pub fn hp_at(&self, k: usize, t: f64) -> f64 {
        let g = &self.show.grid;
        if t < g.t_fight {
            return 100.0;
        }
        if t < self.show.t_ko {
            let r = bf_core::py::round_half_even((t - g.t_fight) * FPS);
            return self.hp_frame(k, if r > 0.0 { r as usize } else { 0 });
        }
        let end = if k == self.show.win {
            self.hp_frame(k, usize::MAX)
        } else {
            0.0
        };
        end + (100.0 - end) * ease((t - g.t_glide) / (g.t_rev - g.t_glide))
    }

    /// Fighter `k`'s ability rows at `t`: grey until first used (a glow sweeps across as it unlocks), lit while a
    /// use lasts; all grey again from the glide home.
    fn ability_rows(&self, k: usize, t: f64) -> Vec<AbilityRow> {
        let g = &self.show.grid;
        let tc = if t < g.t_glide { t } else { -1.0 };
        self.rows[k]
            .iter()
            .zip(&self.uses[k])
            .map(|(r, u)| {
                let on = u.unlock.is_some_and(|tu| g.t_fight <= tc && tc >= tu);
                let (mut glow, mut sweep) = (0.0, None);
                if let (true, Some(tu)) = (on, u.unlock) {
                    if tc < tu + SWEEP_T {
                        sweep = Some((tc - tu) / SWEEP_T);
                    }
                    if let Some(&te) = u.all.iter().rev().find(|&&te| te <= tc) {
                        glow = 1.0 - ss(te + r.hold, te + r.hold + 0.5, tc);
                    }
                }
                AbilityRow {
                    tag: r.tag.to_string(),
                    name: r.name.clone(),
                    unlocked: on,
                    glow,
                    sweep,
                }
            })
            .collect()
    }

    /// The 1080 x 1920 point `p` on the 360 x 640 layer, with frame `f`'s shake.
    fn to_ui(&self, f: usize, p: (f64, f64)) -> (f64, f64) {
        let [dx, dy] = self.cams.shake.get(f).copied().unwrap_or([0.0; 2]);
        ((p.0 + dx) / UP, (p.1 + dy) / UP)
    }

    /// Everything of frame `f` onto `img` (black or white), noting every box in `log`.
    fn draw_on(&self, img: &mut Rgb8, log: &mut DrawLog, f: usize) {
        let t = f as f64 / FPS;
        let g = &self.show.grid;
        let pose = self.cams.poses.get(f).or(self.cams.poses.last());
        let (w, h) = (UP * UI_W as f64, UP * UI_H as f64);
        let cs = pose
            .and_then(|c| project(c, to_render([0.0; 3]), w, h))
            .map_or((UI_W as f64 / 2.0, UI_H as f64 / 2.0), |p| self.to_ui(f, p));
        countdown(img, log, g, t, cs);
        let card_closed = self.card.door_offset(t) == 0.0;
        if !card_closed {
            // the HP numbers on the balls, hidden while FIGHT! covers them
            let balls = bf2_direct::camera::balls_at(self.rec, self.map, t);
            for (k, b) in balls.iter().enumerate() {
                if g.t_fight - 0.05 <= t && t < g.t_fight + 0.85 {
                    continue;
                }
                let Some(bx) = pose.and_then(|c| ball_box(c, ball_centre(*b))) else {
                    continue;
                };
                let c = self.to_ui(f, ((bx[0] + bx[2]) / 2.0, (bx[1] + bx[3]) / 2.0));
                let rp = (bx[2] - bx[0]) / 2.0 / UP;
                let hp = self.hp_at(k, t);
                let txt = format!("{}", (hp - 1e-6).ceil() as i64);
                let w_ = txt.chars().count() as i32 * ADV - 1;
                let x = round_i32(c.0 - f64::from(w_) / 2.0);
                let y = round_i32(c.1 + rp - 10.0);
                let right = if y.saturating_add(7) > 290 { 300 } else { SAFE.2 };
                if SAFE.0 <= x && x + w_ <= right && SAFE.1 <= y && y + 7 <= SAFE.3 {
                    for yy in (y - 1)..(y + 8) {
                        for xx in (x - 2)..(x + w_ + 2) {
                            for (p, &ink) in img.px_mut(xx as usize, yy as usize).iter_mut().zip(&ui::INK) {
                                *p = (f64::from(*p) * 0.35 + f64::from(ink) * 0.65) as u8;
                            }
                        }
                    }
                    ui::label(img, log, &txt, x, y, [255, 244, 224], 1, Some(font::OUTLINE));
                }
            }
            let hud: [Fighter; 2] = std::array::from_fn(|k| {
                let hp = self.hp_at(k, t);
                let lag = self.hp_at(k, (t - 0.45).max(0.0));
                let mut flash: f64 = 0.0;
                for e in &self.shadow.events {
                    if e.kind == "hit" && t - 0.12 <= e.t && e.t <= t && e.i(3) == k as i64 {
                        flash = flash.max(1.0 - (t - e.t) / 0.12);
                    }
                }
                Fighter {
                    trail: Some(hp.max(lag)),
                    flash,
                    ko: k == self.show.lose && self.show.t_ko <= t && t < g.t_glide,
                    win: k == self.show.win && self.show.t_ko + 1.3 <= t && t < g.t_glide,
                    t,
                    ..Fighter::new(&self.show.names[k], self.colours[k], hp)
                }
            });
            let rows = [self.ability_rows(0, t), self.ability_rows(1, t)];
            ui::fight_hud(
                img,
                log,
                &hud[0],
                &hud[1],
                Some([&rows[0], &rows[1]]),
                &HudOptions::default(),
            );
        }
        draw_show_captions(img, log, t, self.show, self.colours);
        let dim = pre_close_dim(g, t);
        if dim > 0.0 {
            for p in &mut img.data {
                *p = (f64::from(*p) * (1.0 - dim)).round() as u8;
            }
        }
        self.card.draw_logged(img, log, t);
    }

    /// Frame `f`'s UI layer and its draw log (360-space, for the safe-zone check): drawn over black and over white
    /// and matted, or taken whole over black while the VS card is shut.
    pub fn draw(&self, f: usize) -> (Overlay, DrawLog) {
        let t = f as f64 / FPS;
        let mut black: Rgb8 = Img::new(UI_W, UI_H, 3);
        let mut log = DrawLog::new();
        self.draw_on(&mut black, &mut log, f);
        if self.card.door_offset(t) == 0.0 {
            return (opaque(&black), log);
        }
        let mut white: Rgb8 = Img::filled(UI_W, UI_H, &[255, 255, 255]);
        self.draw_on(&mut white, &mut DrawLog::new(), f);
        (matte(&black, &white), log)
    }
}

#[cfg(test)]
#[path = "ui2_tests.rs"]
mod tests;
