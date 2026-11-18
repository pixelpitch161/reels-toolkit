//! What is drawn on the finished frame, after the cut to bytes: the count-in (`big_text`, `countdown`), the HP
//! numbers in the balls, the fight HUD with its ability list (`_hold`, `_abil_build`, `ability_rows`, `status`)
//! and the captions, as `kit/picture.py`'s `render` draws them.

use bf_core::event::Event;
use bf_core::py::py_max;
use bf_core::roster::FighterCfg;
use serde_json::Value as Json;

use super::state::BallState;
use super::{PictureCtx, PictureError, owner};
use crate::config::{H, SAFE, W, ss};
use crate::font::{self, ADV};
use crate::img::Rgb8;
use crate::look::DrawnBall;
use crate::np;
use crate::np::index::resample_index;
use crate::ui::{self, AbilityRow, CaptionStyle, DrawLog, Fighter, HudOptions, Kind, round_i32};

/// `SLOTS`: the HUD's ability list: the slot and its tag.
pub const SLOTS: [(&str, &str); 3] = [("unlock1", "LV2"), ("unlock2", "LV3"), ("low", "LOW")];
/// `SWEEP_T`: the seconds the glow takes to sweep across a name as it unlocks.
pub const SWEEP_T: f64 = 0.45;

/// The HUD's ability list of one picture: when each slot unlocked, when it was used, how long a use stays lit.
#[derive(Clone, Debug, PartialEq)]
pub struct Abilities {
    /// The names, per fighter and slot.
    pub names: [[String; 3]; 2],
    /// `ABIL_UNLOCK`: per fighter and slot, the unlock time (`low`: its first use).
    pub unlock: [[Option<f64>; 3]; 2],
    /// `ABIL_USES`: per fighter and slot, the times it was used.
    pub uses: [[Vec<f64>; 3]; 2],
    /// `ABIL_HOLD`: per fighter and slot, how long a use stays lit.
    pub hold: [[f64; 3]; 2],
}

/// The roster entry of an ability slot.
fn ability<'a>(cfg: &'a FighterCfg, slot: &str) -> Result<&'a Json, PictureError> {
    cfg.raw
        .get("abilities")
        .and_then(|a| a.get(slot))
        .filter(|a| a.is_object())
        .ok_or_else(|| PictureError::Roster(format!("{} has no ability {slot} in the roster", cfg.name)))
}

/// `_hold(k, slot)`: how long a use stays lit: while a shield, a haste, a dance or a held note lasts, a slam
/// until it lands, and so on; else a flash of 0.08 s.
fn hold(cfg: &FighterCfg, slot: &str, beat: f64, bar: f64) -> Result<f64, PictureError> {
    let ab = ability(cfg, slot)?;
    let kind = ab.get("kind").and_then(Json::as_str).unwrap_or("");
    let has = |key: &str| ab.get(key).is_some();
    let num = |key: &str| -> Result<f64, PictureError> {
        ab.get(key).and_then(Json::as_f64).ok_or_else(|| {
            PictureError::Roster(format!(
                "{}'s ability {slot} ({kind}) has no number {key} in the roster",
                cfg.name
            ))
        })
    };
    // ab['dur_beats'] * T.BEAT if 'dur_beats' in ab else ab['dur']
    let dur = || {
        if has("dur_beats") {
            Ok(num("dur_beats")? * beat)
        } else {
            num("dur")
        }
    };
    Ok(match kind {
        "shield" | "haste" | "dance" | "endless" | "fork" => dur()?,
        "slam" => num("tele")? * beat,
        "rhapsody" => num("n")? * num("step16")? * beat / 4.0,
        "snip" => num("open16")? * beat / 4.0,
        "stun" => num("stun")?,
        "sustain" | "vibrato" | "twin" | "lights_out" => num("dur_beats")? * beat,
        // max(0.08, ab.get('reload_beats', 0) * T.BEAT)
        "runoff" => {
            let reload = if has("reload_beats") { num("reload_beats")? } else { 0.0 };
            py_max(0.08, reload * beat)
        }
        "tapping" | "prelude" | "stampede" => num("beats")? * beat,
        "smash" => num("rebuild_beats")? * beat,
        "barrage" => num("flight_beats")? * beat + num("beats")? * beat,
        "copy" => bar,
        "mountain" => {
            // sum(ab['seq16']) * T.BEAT / 4
            let seq = ab.get("seq16").and_then(Json::as_array).ok_or_else(|| {
                PictureError::Roster(format!("{}'s ability {slot} (mountain) has no list seq16", cfg.name))
            })?;
            let mut sum = 0.0;
            for v in seq {
                sum += v.as_f64().ok_or_else(|| {
                    PictureError::Roster(format!(
                        "{}'s ability {slot}: seq16 holds something that is no number",
                        cfg.name
                    ))
                })?;
            }
            sum * beat / 4.0
        }
        "hook" => num("open16")? * beat / 4.0 + 0.3,
        "swan" => (num("rise_beats")? + num("glide_beats")? + num("dive_beats")?) * beat,
        "flick" => {
            // ab.get('wind_beats', 1) * T.BEAT
            let wind = if has("wind_beats") { num("wind_beats")? } else { 1.0 };
            wind * beat
        }
        _ => 0.08,
    })
}

/// `_abil_build()` and `ABIL_HOLD`: the HUD's ability list from the run's events up to the K.O.
pub(super) fn abilities(
    cfg: &[FighterCfg; 2],
    ev: &[Event],
    t_ko: f64,
    beat: f64,
    bar: f64,
) -> Result<Abilities, PictureError> {
    let mut names: [[String; 3]; 2] = Default::default();
    let mut holds = [[0.0; 3]; 2];
    for k in 0..2 {
        for (s, (slot, _)) in SLOTS.iter().enumerate() {
            let ab = ability(&cfg[k], slot)?;
            names[k][s] = ab
                .get("name")
                .and_then(Json::as_str)
                .ok_or_else(|| {
                    PictureError::Roster(format!("{}'s ability {slot} has no name in the roster", cfg[k].name))
                })?
                .to_string();
            holds[k][s] = hold(&cfg[k], slot, beat, bar)?;
        }
    }
    let mut unlock: [[Option<f64>; 3]; 2] = Default::default();
    let mut uses: [[Vec<f64>; 3]; 2] = Default::default();
    for e in ev {
        if e.t > t_ko + 1e-6 {
            break;
        }
        let Some(k) = owner(e) else { continue };
        if e.kind == "level" {
            // unl[e[2]]['unlock1' if e[3] == 2 else 'unlock2'] = e[0]
            let s = if matches!(e.fields.get(1), Some(bf_core::event::Value::Int(2))) {
                0
            } else {
                1
            };
            unlock[k][s] = Some(e.t);
        } else if e.kind == "ability" {
            for s in 0..3 {
                if names[k][s] == e.s(3) {
                    uses[k][s].push(e.t);
                    if s == 2 && unlock[k][2].is_none() {
                        unlock[k][2] = Some(e.t);
                    }
                }
            }
        }
    }
    Ok(Abilities {
        names,
        unlock,
        uses,
        hold: holds,
    })
}

/// `STATUS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'ability']`.
pub(super) fn status_list(ev: &[Event]) -> Vec<(f64, usize, String)> {
    ev.iter()
        .filter(|e| e.kind == "ability")
        .filter_map(|e| Some((e.t, owner(e)?, e.s(3).to_string())))
        .collect()
}

/// `big_text(img, text, cx, cy, size_px, col, alpha, outline=(16, 10, 26))`: the count-in's big text, centred
/// on `(cx, cy)`: a soft drop shadow, the outline, the letters.
#[allow(clippy::too_many_arguments)] // the Python signature
pub fn big_text(
    img: &mut Rgb8,
    log: &mut DrawLog,
    text: &str,
    cx: f64,
    cy: f64,
    size_px: f64,
    col: [u8; 3],
    alpha: f64,
) {
    let outline = [16u8, 10, 26];
    let base = font::text_mask(text, 4);
    let (h0, w0) = (base.h, base.w);
    let sc = size_px / h0 as f64;
    let side = |n: usize| round_i32(n as f64 * sc).max(1) as usize;
    let (hh, ww) = (side(h0), side(w0));
    // m = base[ys][:, xs]
    let (ys, xs) = (resample_index(hh, h0), resample_index(ww, w0));
    let mut m = crate::img::Img::<u8>::new(ww, hh, 1);
    for (y, &sy) in ys.iter().enumerate() {
        for (x, &sx) in xs.iter().enumerate() {
            m.data[y * ww + x] = base.data[sy * w0 + sx];
        }
    }
    let o = font::dilate(&m, 2);
    let (x0, y0) = (round_i32(cx - ww as f64 / 2.0), round_i32(cy - hh as f64 / 2.0));
    let (ww_i, hh_i) = (ww as i32, hh as i32);
    for (mm, c, ox, oy) in [(&o, [0u8, 0, 0], 2, 3), (&o, outline, 0, 0), (&m, col, 0, 0)] {
        let (hm, wm) = (mm.h as i32, mm.w as i32);
        let oxx = x0.saturating_sub((wm - ww_i).div_euclid(2)).saturating_add(ox);
        let oyy = y0.saturating_sub((hm - hh_i).div_euclid(2)).saturating_add(oy);
        let (xa, ya) = (oxx.max(0), oyy.max(0));
        let (xb, yb) = (
            oxx.saturating_add(wm).min(W as i32),
            oyy.saturating_add(hm).min(H as i32),
        );
        if xb <= xa || yb <= ya {
            continue;
        }
        let a = alpha * if ox != 0 { 0.5 } else { 1.0 };
        let keep = (1.0 - a) as f32;
        for y in ya..yb {
            for x in xa..xb {
                if mm.data[((y - oyy) * wm + (x - oxx)) as usize] == 0 {
                    continue;
                }
                for (p, &cc) in img.px_mut(x as usize, y as usize).iter_mut().zip(&c) {
                    // reg[sub] = reg[sub] * (1 - a) + c * a, in a float32 copy; then .astype(np.uint8)
                    let v = (f64::from(f32::from(*p) * keep) + f64::from(cc) * a) as f32;
                    *p = np::f32_to_u8(v);
                }
            }
        }
    }
    log.note(
        x0,
        y0,
        x0.saturating_add(ww_i),
        y0.saturating_add(hh_i),
        Kind::Caption,
        text,
    );
}

impl PictureCtx {
    /// `countdown(img, t, cs)`: 3, 2, 1, FIGHT! in the ring, one count a beat, centred on `cs`.
    pub(super) fn countdown(&self, img: &mut Rgb8, log: &mut DrawLog, t: f64, cs: (f64, f64)) {
        let count = &self.show.grid.count;
        for (j, &(tc, txt)) in count.iter().enumerate() {
            let tn = count.get(j + 1).map_or(tc + 0.85, |c| c.0);
            let fight = txt == "FIGHT!";
            let size = if fight { 44.0 } else { 78.0 };
            let col = if fight { [255, 214, 96] } else { [255, 244, 220] };
            if tc <= t && t < tn {
                let u = t - tc;
                // punch = 1 + 0.25 * max(0.0, 1 - u / 0.12) ** 2
                let p = py_max(0.0, 1.0 - u / 0.12);
                let punch = 1.0 + 0.25 * (p * p);
                let out = ss(tn - 0.1, tn, t);
                big_text(
                    img,
                    log,
                    txt,
                    cs.0,
                    cs.1,
                    size * punch * (1.0 - 0.35 * out),
                    col,
                    1.0 - out,
                );
            }
        }
    }

    /// `status(k, t)`: the ability fighter `k` used in the last 1.6 s and how strongly its name shows.
    pub fn status(&self, k: usize, t: f64) -> (String, f64) {
        match self
            .status
            .iter()
            .rev()
            .find(|(te, who, _)| *who == k && *te <= t && t < *te + 1.6)
        {
            Some((te, _, nm)) => (nm.clone(), 1.0 - ss(te + 1.3, te + 1.6, t)),
            None => (String::new(), 0.0),
        }
    }

    /// `ability_rows(k, t)`: the HUD's rows for fighter `k`: grey until unlocked (a glow sweeps across the name
    /// as it unlocks), white-hot on its pill whenever it is used. Frozen through the hit-stop; back to grey
    /// when HP refills on the glide home.
    pub fn ability_rows(&self, k: usize, t: f64) -> Vec<AbilityRow> {
        let g = &self.show.grid;
        let tc = if t < g.t_glide { self.fx_clock(t) } else { -1.0 };
        let ab = &self.abil;
        (0..3)
            .map(|s| {
                let tu = ab.unlock[k][s];
                let on = tu.is_some_and(|tu| g.t_fight <= tc && tc >= tu);
                let (mut glow, mut sweep) = (0.0, None);
                if let (true, Some(tu)) = (on, tu) {
                    if tc < tu + SWEEP_T {
                        sweep = Some((tc - tu) / SWEEP_T);
                    }
                    // past = [te for te in ABIL_USES[k][slot] if te <= tc]; past[-1]
                    if let Some(&te) = ab.uses[k][s].iter().rev().find(|&&te| te <= tc) {
                        let hold = ab.hold[k][s];
                        glow = 1.0 - ss(te + hold, te + hold + 0.5, tc);
                    }
                }
                AbilityRow {
                    tag: SLOTS[s].1.to_string(),
                    name: ab.names[k][s].clone(),
                    unlocked: on,
                    glow,
                    sweep,
                }
            })
            .collect()
    }

    /// `under_something(k, t)`: is fighter `k` covered right now (no HP number on it)?
    pub fn under_something(&self, k: usize, t: f64) -> bool {
        self.looks.iter().any(|l| l.hides_hp(self, k, t))
    }

    /// The HP numbers in the balls, the fight HUD with the ability list, and the six caption calls, in
    /// `render`'s order. `card_closed`: the title card covers the frame, so the numbers and the HUD are left out.
    pub(super) fn draw_ui(
        &self,
        img: &mut Rgb8,
        log: &mut DrawLog,
        t: f64,
        fs: &[BallState; 2],
        drawn: &[DrawnBall],
        card_closed: bool,
    ) {
        let g = &self.show.grid;
        if !card_closed {
            for d in drawn {
                let st = &fs[d.k];
                // hidden while FIGHT! covers the balls
                if st.scale < 0.95 || (g.t_fight - 0.05 <= t && t < g.t_fight + 0.85) {
                    continue;
                }
                if self.under_something(d.k, t) {
                    continue;
                }
                let hp = self.hp_at(d.k, t);
                // txt = str(int(math.ceil(hp - 1e-6)))
                let txt = format!("{}", np::f64_to_i64((hp - 1e-6).ceil()));
                let w_ = txt.chars().count() as i32 * ADV - 1;
                let x = round_i32(d.s.0 - f64::from(w_) / 2.0);
                let y = round_i32(d.s.1 + d.rp - 10.0);
                let right = if y.saturating_add(7) > 290 { 300 } else { SAFE.2 };
                if SAFE.0 <= x && x + w_ <= right && SAFE.1 <= y && y + 7 <= SAFE.3 {
                    // img[y - 1:y + 8, x - 2:x + w_ + 2] = (that * 0.35 + np.array(INK) * 0.65).astype(np.uint8)
                    for yy in (y - 1)..(y + 8) {
                        for xx in (x - 2)..(x + w_ + 2) {
                            for (p, &ink) in img.px_mut(xx as usize, yy as usize).iter_mut().zip(&ui::INK) {
                                *p = np::f64_to_u8(f64::from(*p) * 0.35 + f64::from(ink) * 0.65);
                            }
                        }
                    }
                    ui::label(img, log, &txt, x, y, [255, 244, 224], 1, Some(font::OUTLINE));
                }
            }
        }
        let hud: [Fighter; 2] = std::array::from_fn(|k| {
            let hp_lag = self.hp_at(k, py_max(0.0, t - 0.45));
            let (status, status_k) = self.status(k, t);
            let mut fl = 0.0;
            for i in self.events_between(t - 0.12, t, &["hit"]) {
                let e = &self.rec.events[i];
                let how = e.s(7);
                let quiet = owner(e).is_some_and(|a| self.looks[a].quiet_hits().contains(&how));
                if e.i(3) == k as i64 && how != "key" && !quiet {
                    fl = py_max(fl, 1.0 - (t - e.t) / 0.12);
                }
            }
            let hp = self.hp_at(k, t);
            Fighter {
                name: self.names[k].clone(),
                color: self.col[k],
                hp,
                hp_max: 100.0,
                trail: Some(py_max(hp, hp_lag)),
                level: if g.t_fight <= t {
                    fs[k].level.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
                } else {
                    1
                },
                max_level: 3,
                status,
                status_k,
                flash: fl,
                ko: k == self.lose && self.t_ko <= t && t < g.t_glide,
                win: k == self.win && self.t_ko + 1.3 <= t && t < g.t_glide,
                t,
            }
        });
        if !card_closed {
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
        // tuple(int(v) for v in COL[k])
        let ink = |c: [f64; 3]| -> [u8; 3] { [c[0] as u8, c[1] as u8, c[2] as u8] };
        let tint = |c: [f64; 3]| ink(c.map(|v| np::clip(v * 0.8 + 60.0, 0.0, 255.0)));
        let gold = [255, 214, 96];
        // CAP_TITLE is empty: the VS title card replaces the top caption
        let style = CaptionStyle {
            hi: ink(self.col[0]),
            alt: ink(self.col[1]),
            ..CaptionStyle::default()
        };
        ui::draw_captions(img, log, t, &[], &style);
        for k in 0..2 {
            let style = CaptionStyle {
                base: tint(self.col[k]),
                puff: Some(ink(self.col[k])),
                ..CaptionStyle::default()
            };
            ui::draw_captions(img, log, t, &self.show.caps_ability[k], &style);
        }
        let hi_gold = CaptionStyle {
            hi: gold,
            ..CaptionStyle::default()
        };
        ui::draw_captions(img, log, t, &self.show.cap_ko, &hi_gold);
        let wins = CaptionStyle {
            base: tint(self.col[self.win]),
            ..CaptionStyle::default()
        };
        ui::draw_captions(img, log, t, &self.show.cap_wins, &wins);
        ui::draw_captions(img, log, t, &self.show.cap_comment, &hi_gold);
    }
}
