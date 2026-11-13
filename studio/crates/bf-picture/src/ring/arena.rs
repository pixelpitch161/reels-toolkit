//! The arena around the ring: `led_colour` and `arena` of `kit/picture.py`, with its static draw lists. Tiers,
//! people (phones, flashbulbs) and LED boards go down in one painter pass, far to near.

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::py::{self, py_max};

use super::PictureCtx;
use crate::camera::{Cam, V3};
use crate::config::{H, W};
use crate::lighting::{Lights, shade};
use crate::look::Look;
use crate::np;
use crate::pil;
use crate::raster::Frame;
use crate::scene::{CROWD_COLS, Scene};
use crate::v3;

/// `WP = 61.0 / 137`: the period of a phone's twinkle (a whole number of cycles in 61 s).
const WP: f64 = 61.0 / 137.0;

/// The static draw lists `kit/picture.py` derives from the scene: `STAND_N`, `STAND_C`, `STAND_A`, `BAR_N`,
/// `BAR_C` (the crowd's columns are read from the scene's people).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawLists {
    pub stand_n: Vec<V3>,
    pub stand_c: Vec<V3>,
    pub stand_a: Vec<V3>,
    pub bar_n: Vec<V3>,
    pub bar_c: Vec<V3>,
}

impl DrawLists {
    pub(super) fn new(scene: &Scene) -> DrawLists {
        DrawLists {
            stand_n: scene.stands.iter().map(|s| s.n).collect(),
            stand_c: scene.stands.iter().map(|s| v3::mean(&s.q)).collect(),
            stand_a: scene.stands.iter().map(|s| s.col).collect(),
            bar_n: scene.barrier.iter().map(|b| b.n).collect(),
            bar_c: scene.barrier.iter().map(|b| v3::mean(&b.q)).collect(),
        }
    }
}

/// `FLASH_T`: when the crowd's flashbulbs go off: every hit of a kind its attacker's look lists, every ability,
/// and sixteen at the K.O., sorted.
pub(super) fn flash_times(ev: &[Event], looks: &[Box<dyn Look>], t_ko: f64) -> Vec<f64> {
    let mut out: Vec<f64> = ev
        .iter()
        .filter(|e| {
            e.kind == "ability"
                || (e.kind == "hit" && super::owner(e).is_some_and(|k| looks[k].flash_hits().contains(&e.s(7))))
        })
        .map(|e| e.t)
        .collect();
    out.extend((0..16).map(|j| t_ko + 0.03 * f64::from(j)));
    out.sort_by(|a, b| a.total_cmp(b));
    out
}

/// `tuple(int(v) for v in col)`: a Pillow ink.
fn ink(col: V3) -> [u8; 3] {
    [col[0] as u8, col[1] as u8, col[2] as u8]
}

/// What the painter pass draws: `(depth, kind, index, colour)`.
enum Item {
    Stand(usize, V3),
    Person(usize),
    Board(usize),
}

impl PictureCtx {
    /// `led_colour(am, t, wk)`: the LED boards: each fighter's colour on its own side, a chase that steps on the
    /// beat, gold for the champion.
    pub fn led_colour(&self, am: f64, t: f64, wk: f64) -> V3 {
        let g = &self.show.grid;
        let beat = t / g.beat;
        let u = py::fmod_py(am / (2.0 * dmath::PI) * 16.0 + beat, 4.0);
        let side = if dmath::sin(am) < 0.0 { self.col[0] } else { self.col[1] };
        let base = if u < 2.0 { side } else { side.map(|c| c * 0.42 + 40.0) };
        let k = 0.55 + 0.45 * g.beat_pulse(t, 0.18, 0.12);
        let gold = [255.0, 200.0, 90.0];
        std::array::from_fn(|c| (base[c] * (1.0 - wk) + gold[c] * wk) * k)
    }

    /// `arena(fr, cam, L, t)`: tiers, people and LED boards in one painter pass (far to near).
    pub(super) fn arena(&self, fr: &mut Frame, cam: &Cam, l: &Lights, t: f64) {
        let g = &self.show.grid;
        let dl = &self.lists;
        let wk = self.win_k(t);
        let mut items: Vec<(f64, Item)> = Vec::new();
        // vis = np.einsum('ij,ij->i', STAND_N, cam.pos[None] - STAND_C) > 0
        let facing = |n: &[V3], c: &[V3]| -> Vec<usize> {
            (0..n.len())
                .filter(|&j| v3::dot(n[j], v3::sub(cam.pos, c[j])) > 0.0)
                .collect()
        };
        let idx = facing(&dl.stand_n, &dl.stand_c);
        let pick = |a: &[V3]| -> Vec<V3> { idx.iter().map(|&j| a[j]).collect() };
        let (nrm, ctr) = (pick(&dl.stand_n), pick(&dl.stand_c));
        let cols = shade(l, &nrm, &ctr, &pick(&dl.stand_a), None, 0.2, 0.0, 5.0);
        for ((&j, c), d) in idx.iter().zip(&cols).zip(cam.depth(&ctr)) {
            items.push((d, Item::Stand(j, c.map(|v| v * 0.55))));
        }
        let bidx = facing(&dl.bar_n, &dl.bar_c);
        let bctr: Vec<V3> = bidx.iter().map(|&j| dl.bar_c[j]).collect();
        for (&j, d) in bidx.iter().zip(cam.depth(&bctr)) {
            items.push((d, Item::Board(j)));
        }
        let crowd = &self.scene.crowd;
        // heads = CROWD_P + [0, 0.55 * CROWD_HT, 0]
        let heads: Vec<V3> = crowd
            .iter()
            .map(|q| [q.p[0] + 0.0, q.p[1] + 0.55 * q.ht, q.p[2] + 0.0])
            .collect();
        // attack lights reach the crowd
        let tint = self.fx.fl.tint(&heads, self.fx_clock(t), 1.0);
        let dh = cam.depth(&heads);
        let sp = cam.wp(&heads);
        for (j, &d) in dh.iter().enumerate() {
            items.push((d - 0.3, Item::Person(j)));
        }
        // items.sort(key=lambda it: -it[0]): a stable sort, far to near
        items.sort_by(|a, b| (-a.0 + 0.0).total_cmp(&(-b.0 + 0.0)));
        let beat = np::f64_to_i64(t / g.beat);
        // recent = FLASH_T[(FLASH_T <= t) & (FLASH_T > t - 0.1)]
        let recent: Vec<f64> = self
            .flash_t
            .iter()
            .copied()
            .filter(|&ft| ft <= t && ft > t - 0.1)
            .collect();
        let nflash = recent.len() as i64;
        let spill = 0.48 + 0.28 * wk;
        let ph_b = py::fmod_py(t, g.beat);
        let on_b = ph_b < 0.12 || ph_b > g.beat - 0.04;
        let beat_j = beat + i64::from(ph_b > g.beat - 0.04);
        for (_, item) in &items {
            match *item {
                Item::Stand(j, c) => {
                    fr.poly(cam, &self.scene.stands[j].q, c, true);
                }
                Item::Board(j) => {
                    let b = &self.scene.barrier[j];
                    let col = self.led_colour(b.am, t, wk);
                    fr.poly(cam, &b.q, col.map(|c| c * 0.55 + 18.0), true);
                    let top = cam.wp(&b.q[2..4]);
                    let edge = col.map(|c| np::clip(c * 0.9 + 60.0, 0.0, 255.0));
                    pil::line(&mut fr.pil, &[top[0], top[1]], &ink(edge), 1);
                }
                Item::Person(j) => {
                    let q = &crowd[j];
                    let (x, mut y) = sp[j];
                    let even = py::imod_py(beat_j + q.ci as i64, 2) == 0;
                    let hop = (even && on_b) || (wk > 0.3 && q.ph < wk && on_b);
                    if hop {
                        y -= 1.0;
                    }
                    let rh = 0.15 * cam.f / dh[j];
                    if !(-6.0 < x && x < W as f64 + 6.0 && -6.0 < y && y < H as f64 + 10.0) {
                        continue;
                    }
                    // (a person behind the camera has a radius below zero: Pillow refuses the ellipse and the
                    // Python frame fails. The ring's camera never turns its back on the crowd; skip.)
                    if rh < 0.0 {
                        continue;
                    }
                    let side = q.p[0];
                    let tone = if side < -1.5 {
                        self.crowd_tint[0]
                    } else if side > 1.5 {
                        self.crowd_tint[1]
                    } else {
                        [1.0, 0.92, 1.0]
                    };
                    // crowd_lit = 1.0 + 0.9 * FL.tint(heads, tc)
                    let body: V3 = std::array::from_fn(|c| {
                        let lit = 1.0 + 0.9 * tint[j][c];
                        np::clip(CROWD_COLS[q.ci][c] * tone[c] * spill * lit, 0.0, 255.0)
                    });
                    let pic = &mut fr.pil;
                    pil::ellipse(
                        pic,
                        (x - 1.9 * rh, y + 0.6 * rh, x + 1.9 * rh, y + 3.4 * rh),
                        Some(&ink(body.map(|v| v * 0.8))),
                        None,
                        1,
                    );
                    pil::ellipse(
                        pic,
                        (x - rh, y - rh, x + rh, y + rh),
                        Some(&ink(body.map(|v| v * 0.62))),
                        None,
                        1,
                    );
                    // rim light from the ring on the heads (they face it)
                    if rh >= 1.5 {
                        let rim = body.map(|v| np::clip(v * 1.5 + 30.0, 0.0, 255.0));
                        pil::line(
                            pic,
                            &[(x - rh * 0.6, y - rh * 0.8), (x + rh * 0.6, y - rh * 0.8)],
                            &ink(rim),
                            1,
                        );
                    }
                    let ph = q.ph;
                    // phones up (lots more for the champion)
                    if ph < 0.07 + 0.45 * wk {
                        let tw = 0.65 + 0.35 * dmath::sin(2.0 * dmath::PI * (t / WP + ph * 11.0));
                        let c2 = ink([255.0 * tw, 248.0 * tw, 226.0 * tw]);
                        let py_ = y - 2.2 * rh;
                        pil::rectangle(
                            pic,
                            (
                                x + rh * 0.3,
                                py_ - py_max(1.0, rh * 0.7),
                                x + rh * 0.3 + py_max(1.0, rh * 0.5),
                                py_,
                            ),
                            Some(&c2),
                            None,
                        );
                    }
                    if let Some(&last) = recent.last() {
                        let roll = np::f64_to_i64(ph * 997.0) + np::f64_to_i64(last * 100.0);
                        if py::imod_py(roll, 47) < 1 + nflash {
                            pil::ellipse(
                                pic,
                                (x - 1.6, y - 2.6 * rh - 1.6, x + 1.6, y - 2.6 * rh + 1.6),
                                Some(&[255, 255, 255]),
                                None,
                                1,
                            );
                        }
                    }
                }
            }
        }
    }
}
