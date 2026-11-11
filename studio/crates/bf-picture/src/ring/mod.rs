//! The ring renderer: the port of `projects/ballfight/kit/picture.py`. One [`Picture`] is one fight's video: the
//! run, the show's times, the two fighters' looks, every effect built once from the run's events, the sprite
//! cache and the VS title card. A frame is a pure function of its number, so frames render in any order and on
//! any number of threads.
//!
//! - [`PictureCtx`] is what the Python module holds at import (`RUN`, `EV`, `T_KO`, `COL`, `CFG`, the effects,
//!   `fighter()`, `sim_i()`, ...): the looks' hooks get it in place of the Python module `P`.
//! - [`state`]: the run's arrays, `fighter(k, t)`, the camera and the lights.
//! - [`arena`]: stands, crowd, phones, flashbulbs and LED boards.
//! - [`canvas`]: the two per-pixel passes (arena floor, canvas), the skirt, posts, rope lines and the canvas's
//!   shadow mask.
//! - [`ball`]: the glass balls and their sprites.
//! - [`ropes`]: the rope's shape at a time.
//! - [`shards`]: the K.O. shatter.
//! - [`effects`]: `_fx_build`, the camera shake, sparks and shimmers.
//! - [`post_fx`]: fringe, ability moments and bursts after the grade.
//! - [`hud`]: the count-in, HP numbers, the fight HUD's rows and the captions.
//! - [`frame`]: `render(f)`, stage by stage.

pub mod arena;
pub mod ball;
pub mod canvas;
pub mod effects;
pub mod frame;
pub mod hud;
pub mod post_fx;
pub mod ropes;
pub mod shards;
pub mod state;

#[cfg(test)]
mod tests;

use std::fmt;
use std::sync::Arc;

use bf_core::event::{Event, Value};
use bf_core::fightfile::FightFile;
use bf_core::harmony::{self, Harmony};
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_core::show::Show;

use self::frame::Rendered;
use crate::camera::{Cam, Key, V3};
use crate::config::{H, NF, W};
use crate::fx::Shake;
use crate::img::{Img, Rgb8, RgbF};
use crate::look::{Fx, Look};
use crate::pil;
use crate::scene::{self, Scene};
use crate::shadows::MaskGrid;
use crate::sprite::{Mesh, SpriteCache};
use crate::title::layout::Layout;
use crate::title::{CardLook, TitleCard, TitleError};
use crate::ui::DrawLog;

pub use self::frame::{FrameReport, Stage, StageBuffers, StagePic};

/// Why a [`Picture`] cannot be made: in plain words.
#[derive(Clone, Debug, PartialEq)]
pub enum PictureError {
    /// The recording, the fight file and the show do not belong together, or the recording lacks an array the
    /// renderer reads.
    Recording(String),
    /// An event of the recording lacks an element the renderer reads.
    Event(String),
    /// A fighter's roster entry lacks something the renderer reads.
    Roster(String),
    /// A fighter's model has no faces.
    Mesh(String),
    /// The fight file's music does not give the winner a solo.
    Music(String),
    /// The VS title card cannot be built (an unported panel pattern, a middle word that does not fit, a ball too
    /// big for its close-up).
    Title(TitleError),
}

impl fmt::Display for PictureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PictureError::Recording(s)
            | PictureError::Event(s)
            | PictureError::Roster(s)
            | PictureError::Mesh(s)
            | PictureError::Music(s) => f.write_str(s),
            PictureError::Title(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PictureError {}

/// What `kit/picture.py` holds at import for one episode: the run and everything derived from it once. The
/// looks' hooks read it where the Python hooks read the module `P`.
pub struct PictureCtx {
    /// The recording (`RUN`'s source; `EV` is `rec.events`).
    pub rec: Arc<Recording>,
    /// `RUN`: the per-frame arrays.
    pub run: state::Run,
    /// The show: `T` (the grid), `T_SOLO`, the caption windows.
    pub show: Show,
    /// `NAMES`.
    pub names: [String; 2],
    /// `CFG`: the two roster entries.
    pub cfg: [FighterCfg; 2],
    /// `CFG[k]['view']`: `(yaw centre, yaw swing, roll)` in degrees.
    pub view: [(f64, f64, f64); 2],
    /// `COL`: the fighters' colours as floats.
    pub col: [V3; 2],
    /// `DARK`.
    pub dark: [V3; 2],
    /// The ring with the fighters' colours on it (`S`).
    pub scene: Scene,
    /// `T_KO`.
    pub t_ko: f64,
    /// `WIN`.
    pub win: usize,
    /// `LOSE`.
    pub lose: usize,
    /// `T_HS = T_KO + HITSTOP`.
    pub t_hs: f64,
    /// `T_SOLO`.
    pub t_solo: f64,
    /// `SOLO_T`: the start of each note of the winner's solo.
    pub solo_t: Vec<f64>,
    /// `T.SOLO[NAMES[k]]`: each fighter's solo as `(beat, midi note, beats)`.
    pub solos: [Vec<(f64, i32, f64)>; 2],
    /// `REST[k] = weapon.get('rest', 0.0)`.
    pub rest: [f64; 2],
    /// `OMEGA[k] = weapon.get('spin', 0.0)`.
    pub omega: [f64; 2],
    /// `BASE_LEN[k] = weapon.get('base', 0.0)`.
    pub base_len: [f64; 2],
    /// `FRETS0[k] = weapon.get('frets0', 0)`.
    pub frets0: [i64; 2],
    /// `TH0[k]`.
    pub th0: [f64; 2],
    /// `SD0[k]`.
    pub sd0: [f64; 2],
    /// `CORNER0`.
    pub corner0: [[f64; 2]; 2],
    /// `P_KO`: where each ball was at the K.O.
    pub p_ko: [[f64; 2]; 2],
    /// `GLIDE_ARC`: `(normal, a)` of each fighter's bow on the glide home.
    pub glide_arc: [([f64; 2], f64); 2],
    /// `LIGHT_RGB`: the corner lights' colours (largest channel 1).
    pub light_rgb: [V3; 2],
    /// `CROWD_TINT`.
    pub crowd_tint: [V3; 2],
    /// `FL_COL`: the attack lights' colours (`COL[k] * flare_scale`).
    pub fl_col: [V3; 2],
    /// `T.SHARD_SEED`.
    pub shard_seed: u64,
    /// `T.FX_SEED`.
    pub fx_seed: u64,
    /// `EV_T`.
    pub ev_t: Vec<f64>,
    /// `KEYS`: the camera's keys.
    pub keys: Vec<Key>,
    /// `SHK`.
    pub shk: Shake,
    /// `ROPE_EV`.
    pub rope_ev: Vec<ropes::RopeEv>,
    /// `WAVE_EV`.
    pub wave_ev: Vec<(f64, f64)>,
    /// `PARTS`.
    pub parts: Vec<shards::Part>,
    /// `TRAJ`.
    pub traj: Vec<Vec<[f64; 4]>>,
    /// `STATUS`.
    pub status: Vec<(f64, usize, String)>,
    /// `ABIL_UNLOCK`, `ABIL_USES`, `ABIL_HOLD`.
    pub abil: hud::Abilities,
    /// `FLASH_T`.
    pub flash_t: Vec<f64>,
    /// `FL`, `BU`, `PA`, `MK`, `MO`. Empty while the effects are being built: the build hooks get the effects as
    /// their own argument.
    pub fx: Fx,
    /// `LOOKS`. Empty during [`Look::setup`].
    pub looks: Vec<Box<dyn Look>>,
    /// The fighters' models.
    pub meshes: [Mesh; 2],
    /// The ring's sprite cache (`_SPR`).
    pub sprites: SpriteCache,
    /// `STAND_N` and the other static draw lists.
    pub lists: arena::DrawLists,
    /// `STATIC_CASTERS`.
    pub static_casters: Vec<[V3; 4]>,
    /// `CANVAS_GRID`.
    pub canvas_grid: MaskGrid,
    /// The VS title card's `WORDS` and `BALLS` (`title.py`, imported before the looks are made): the fight
    /// file's opt-ins applied. A look that reads `TT.BALLS` in its `setup` (TUBA's panel rings) reads it here.
    pub title: Layout,
}

/// One fight's video: the frames of `kit.picture.render`.
pub struct Picture {
    p: PictureCtx,
    card: TitleCard,
}

/// `owner(e)`: the side an event belongs to (its attacker, user or owner), or `None`.
pub fn owner(e: &Event) -> Option<usize> {
    e.who().filter(|&k| k < 2)
}

/// `e[2]` of an event the renderer has checked to have a fighter there (a `hit`, an `ability`): Python's
/// `LOOKS[e[2]]`. [`Picture::new`] refuses a recording where it is not 0 or 1, so the fallback to 0 is never
/// taken; a debug build says so if it ever is.
pub(crate) fn checked_owner(e: &Event) -> usize {
    let k = owner(e);
    debug_assert!(
        k.is_some(),
        "a {} event without a fighter 0 or 1 passed the check",
        e.kind
    );
    k.unwrap_or(0)
}

/// `BAYER_FULL[y, x]` for any column and row (the table is a 4 x 4 tile).
pub(crate) fn bayer(x: usize, y: usize) -> f64 {
    crate::config::BAYER[y % 4][x % 4]
}

/// `screen(cam, P)`: world points on the screen.
pub fn screen(cam: &Cam, p: &[V3]) -> Vec<(f64, f64)> {
    cam.wp(p)
}

/// `ring_circle(cx, cz, r, y=0.02, n=40)`: a circle on the canvas as `n` world points. The Python defaults are
/// `y=0.02` and `n=40`.
pub fn ring_circle(cx: f64, cz: f64, r: f64, y: f64, n: usize) -> Vec<V3> {
    canvas::circle_angles(n)
        .iter()
        .map(|&a| scene::to_world(cx + r * bf_core::dmath::cos(a), cz + r * bf_core::dmath::sin(a), y))
        .collect()
}

/// The mask of `layer_blend` and `add_light`: the polygons of three points or more, filled into an `L` image.
fn poly_mask(polys: &[Vec<(f64, f64)>]) -> Img<u8> {
    let mut m: Img<u8> = Img::new(W, H, 1);
    for pts in polys {
        if pts.len() >= 3 {
            pil::polygon(&mut m, pts, &[255]);
        }
    }
    m
}

/// `layer_blend(arr, polys, col, alpha, depth=None, z=None)`: alpha-blends screen polygons of one colour onto
/// the `float32` picture; with `depth_z = Some((depth, z))` only where the depth buffer is behind `z - 0.05`.
///
/// The arithmetic is numpy's for the call every look makes: `alpha` a Python float (the mask times it stays
/// `float32`) and `z` a Python float. A `z` that is a numpy `float64` scalar is the same comparison here (the
/// `float32` depth is widened and compared with `z - 0.05` in `f64`, which is what numpy does with a `float64`
/// scalar); a Python float `z` makes numpy compare in `float32` instead, which can differ only for a depth within
/// one `float32` step of `z - 0.05`. An `alpha` that is a numpy `float64` scalar would make the mask `float64`
/// in numpy; this function has no such path.
pub fn layer_blend(
    arr: &mut Img<f32>,
    polys: &[Vec<(f64, f64)>],
    col: V3,
    alpha: f64,
    depth_z: Option<(&Img<f32>, f64)>,
) {
    if polys.is_empty() {
        return;
    }
    let m = poly_mask(polys);
    // mk = np.asarray(m, np.float32) / 255.0 * alpha: float32 all the way
    let alpha = alpha as f32;
    for (i, &b) in m.data.iter().enumerate() {
        if b == 0 {
            continue; // arr + (col - arr) * 0 is arr
        }
        let mut mk = f32::from(b) / 255.0f32 * alpha;
        if let Some((depth, z)) = depth_z {
            // mk = mk * (depth > z - 0.05)
            let behind = f64::from(depth.data[i]) > z - 0.05;
            mk *= if behind { 1.0 } else { 0.0 };
        }
        // arr += (col - arr) * mk: computed in float64, stored as float32
        for (v, &c) in arr.data[i * 3..i * 3 + 3].iter_mut().zip(&col) {
            let a = f64::from(*v);
            *v = (a + (c - a) * f64::from(mk)) as f32;
        }
    }
}

/// `add_light(arr, polys, col, k)`: adds `col * k` to the `float32` picture inside the screen polygons.
pub fn add_light(arr: &mut Img<f32>, polys: &[Vec<(f64, f64)>], col: V3, k: f64) {
    let m = poly_mask(polys);
    for (i, &b) in m.data.iter().enumerate() {
        if b == 0 {
            continue;
        }
        // arr += mk[..., None] * col * k, with mk = uint8 / float32(255)
        let mk = f64::from(f32::from(b) / 255.0f32);
        for (v, &c) in arr.data[i * 3..i * 3 + 3].iter_mut().zip(&col) {
            *v = (f64::from(*v) + mk * c * k) as f32;
        }
    }
}

/// A number of a roster entry's weapon, `CFG[k]['weapon'].get(key, 0.0)`.
fn weapon(cfg: &FighterCfg, key: &str) -> f64 {
    cfg.raw
        .get("weapon")
        .and_then(|w| w.get(key))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0)
}

/// That every event has the elements the renderer reads from its kind, so no frame can fail on one.
fn check_events(ev: &[Event]) -> Result<(), PictureError> {
    let bad = |i: usize, e: &Event, what: &str| {
        Err(PictureError::Event(format!(
            "event {i} ({}, at {:.3} s) {what}",
            e.kind, e.t
        )))
    };
    let num = |e: &Event, i: usize| matches!(e.fields.get(i - 2), Some(Value::Float(_) | Value::Int(_)));
    let int = |e: &Event, i: usize| matches!(e.fields.get(i - 2), Some(Value::Int(_)));
    let text = |e: &Event, i: usize| matches!(e.fields.get(i - 2), Some(Value::Str(_)));
    for (i, e) in ev.iter().enumerate() {
        let who = owner(e).is_some();
        let problem = match e.kind.as_str() {
            "hit" if !(who && int(e, 3) && num(e, 5) && num(e, 6) && text(e, 7)) => {
                Some("is not [t, 'hit', attacker 0 or 1, victim, damage, x, y, what hit]")
            }
            "ability" if !(who && text(e, 3) && text(e, 4)) => {
                Some("is not [t, 'ability', fighter 0 or 1, name, kind]")
            }
            "level" if !(who && int(e, 3)) => Some("is not [t, 'level', fighter 0 or 1, level]"),
            "rope"
                if !(int(e, 3) && int(e, 4) && num(e, 5) && num(e, 6) && num(e, 7))
                    || !matches!((e.i(3), e.i(4)), (0 | 1, 1 | -1)) =>
            {
                Some("is not [t, 'rope', ball, axis 0 or 1, side 1 or -1, x, y, speed]")
            }
            "parry" | "block" | "wall" | "cleared" | "fade" if !(num(e, 4) && num(e, 5)) => {
                Some("has no position in elements 4 and 5")
            }
            _ => None,
        };
        if let Some(what) = problem {
            return bad(i, e, what);
        }
    }
    if ev.windows(2).any(|w| w[1].t < w[0].t) {
        return Err(PictureError::Event(
            "the recording's events are not in time order".into(),
        ));
    }
    Ok(())
}

impl Picture {
    /// The picture of a fight: `rec` the recording, `ff` its fight file, `show` the show made of the two
    /// ([`Show::new`]), `cfg` the fighters' roster entries, `meshes` their models and `looks` their looks, each
    /// left corner first. Everything the Python module builds at import is built here.
    ///
    /// # Errors
    /// In plain words: a recording, fight file and show that do not belong together; a recording without the
    /// arrays the renderer reads, or with an event that lacks an element it reads; a roster entry without what
    /// it reads; a model without faces; a title card with an unported panel pattern.
    pub fn new(
        rec: Arc<Recording>,
        ff: &FightFile,
        show: Show,
        cfg: [&FighterCfg; 2],
        meshes: [Mesh; 2],
        mut looks: [Box<dyn Look>; 2],
    ) -> Result<Picture, PictureError> {
        let recording = |s: String| PictureError::Recording(s);
        if rec.names[..] != show.names[..] || ff.fighters[..] != show.names[..] {
            return Err(recording(format!(
                "the show is of {:?}, the recording of {:?} and the fight file of {:?}",
                show.names, rec.names, ff.fighters
            )));
        }
        if rec.t_ko.map(f64::to_bits) != Some(show.t_ko.to_bits()) || rec.winner != Some(show.win) {
            return Err(recording(
                "the show was not made from this recording (its K.O. differs)".into(),
            ));
        }
        if show.grid != ff.grid() {
            return Err(recording(
                "the show was not made from this fight file (its tempo differs)".into(),
            ));
        }
        for k in 0..2 {
            if cfg[k].name != show.names[k] {
                return Err(PictureError::Roster(format!(
                    "the roster entry given for fighter {k} is {}'s, not {}'s",
                    cfg[k].name, show.names[k]
                )));
            }
            if meshes[k].faces.is_empty() {
                return Err(PictureError::Mesh(format!("{}'s model has no faces", show.names[k])));
            }
        }
        if rec.corner.len() != 2 {
            return Err(recording("the recording does not have two corners".into()));
        }
        check_events(&rec.events)?;
        let run = state::Run::new(rec.clone()).map_err(recording)?;
        let cfg = [cfg[0].clone(), cfg[1].clone()];
        let mut view = [(0.0, 0.0, 0.0); 2];
        for k in 0..2 {
            let v: Vec<f64> = cfg[k]
                .raw
                .get("view")
                .and_then(serde_json::Value::as_array)
                .map(|a| a.iter().filter_map(serde_json::Value::as_f64).collect())
                .unwrap_or_default();
            if v.len() != 3 {
                return Err(PictureError::Roster(format!(
                    "{} has no view (yaw, swing, roll) in the roster",
                    cfg[k].name
                )));
            }
            view[k] = (v[0], v[1], v[2]);
        }
        let g = &show.grid;
        let col = [cfg[0].color.map(f64::from), cfg[1].color.map(f64::from)];
        let dark = [cfg[0].dark.map(f64::from), cfg[1].dark.map(f64::from)];
        let (win, lose, t_ko, t_hs, t_solo) = (show.win, show.lose, show.t_ko, show.t_hs, show.t_solo);
        // T.SOLO: the fight file's solos, or the kit's sketch
        let music = |e: harmony::HarmonyError| PictureError::Music(format!("the fight file's music: {e}"));
        let mut solos: [Vec<(f64, i32, f64)>; 2] = Default::default();
        for k in 0..2 {
            solos[k] = match ff.music.solos.get(&show.names[k]) {
                Some(s) => s.clone(),
                None => {
                    let chords = ff.chords(g.n_bars);
                    let scale = harmony::scale(&ff.music.scale).map_err(music)?;
                    Harmony::new(g, &ff.music.key, scale, chords, Some(ff.music.chord_loop.clone()), 2)
                        .and_then(|h| h.default_solo(cfg[k].register))
                        .map_err(music)?
                }
            };
        }
        let solo_t = show.solo_times(&solos[win]);
        let last = run.last();
        let two = |f: &dyn Fn(usize) -> f64| [f(0), f(1)];
        let sd0 = two(&|k| {
            if run.has("SPIN") {
                run.f("SPIN", 0, k).unwrap_or(1.0)
            } else {
                1.0
            }
        });
        let th0 = two(&|k| run.f("TH", 0, k).unwrap_or(0.0));
        let corner0 = [rec.corner[0], rec.corner[1]];
        let p_ko = [run.p(last, 0), run.p(last, 1)];
        // LIGHT_RGB = COL[k] / COL[k].max(); CROWD_TINT = 0.78 + 0.42 * LIGHT_RGB[k]
        let light_rgb = col.map(|c| {
            let top = crate::np::maximum(crate::np::maximum(c[0], c[1]), c[2]);
            c.map(|v| v / top)
        });
        let crowd_tint = light_rgb.map(|c| c.map(|v| 0.78 + 0.42 * v));
        let scene = Scene::new(col);
        let fx_seed = ff.fx_seed();
        // `import title as TT` comes before the looks: WORDS (with MIDDLE fitted) and BALLS
        let title = Layout::new(&show.names[0], &show.names[1], &ff.picture).map_err(PictureError::Title)?;
        let mut p = PictureCtx {
            run,
            names: show.names.clone(),
            view,
            col,
            dark,
            lists: arena::DrawLists::new(&scene),
            scene,
            t_ko,
            win,
            lose,
            t_hs,
            t_solo,
            solo_t,
            solos,
            rest: two(&|k| weapon(&cfg[k], "rest")),
            omega: two(&|k| weapon(&cfg[k], "spin")),
            base_len: two(&|k| weapon(&cfg[k], "base")),
            frets0: [weapon(&cfg[0], "frets0") as i64, weapon(&cfg[1], "frets0") as i64],
            th0,
            sd0,
            corner0,
            p_ko,
            glide_arc: state::glide_arcs(p_ko, corner0, win),
            light_rgb,
            crowd_tint,
            fl_col: col,
            shard_seed: ff.picture.shard_seed,
            fx_seed,
            ev_t: rec.events.iter().map(|e| e.t).collect(),
            keys: state::camera_keys(t_ko),
            shk: Shake::new(2.5, 0.09, 0.45),
            rope_ev: ropes::rope_events(&rec.events),
            wave_ev: Vec::new(),
            parts: Vec::new(),
            traj: Vec::new(),
            status: hud::status_list(&rec.events),
            abil: hud::abilities(&cfg, &rec.events, t_ko, g.beat, g.bar)?,
            flash_t: Vec::new(),
            fx: Fx::new(fx_seed),
            looks: Vec::new(),
            meshes,
            sprites: SpriteCache::new(),
            static_casters: canvas::static_casters(),
            canvas_grid: canvas::canvas_grid(),
            title,
            cfg,
            show,
            rec,
        };
        // the fighters' looks: LOOKS = [look(NAMES[k], P, k) for k in (0, 1)], then each one's setup()
        for (k, look) in looks.iter_mut().enumerate() {
            look.setup(&p, k);
        }
        let looks = Vec::from(looks);
        // FL_COL = COL[k] * LOOKS[k].flare_scale: near-white fighters scaled down
        p.fl_col = [0, 1].map(|k| p.col[k].map(|c| c * looks[k].flare_scale()));
        p.shk = effects::shake(&p, &looks);
        p.wave_ev = ropes::wave_events(&p, &looks);
        (p.parts, p.traj) = shards::shards(p.shard_seed, p.p_ko[p.lose], &looks[p.lose].shards());
        p.flash_t = arena::flash_times(&p.rec.events, &looks, p.t_ko);
        p.looks = looks;
        // every effect, once, in time order
        let mut fx = std::mem::replace(&mut p.fx, Fx::new(fx_seed));
        effects::fx_build(&p, &mut fx);
        p.fx = fx;
        let card = TitleCard::with_layout(
            &p.show,
            p.col,
            p.dark,
            [0, 1].map(|k| CardLook {
                look: &*p.looks[k],
                mesh: &p.meshes[k],
                yaw: p.view[k].0,
                roll: p.view[k].2,
            }),
            ff.series_tag.as_deref(),
            p.title.clone(),
        )
        .map_err(PictureError::Title)?;
        Ok(Picture { p, card })
    }

    /// What the looks' hooks read: the run and everything built from it.
    pub fn ctx(&self) -> &PictureCtx {
        &self.p
    }

    /// The VS title card.
    pub fn card(&self) -> &TitleCard {
        &self.card
    }

    /// `T.NF`: the number of frames of the video, 1,830.
    pub fn nf(&self) -> usize {
        NF
    }

    /// Frame `f`, 360 x 640: a pure function of `f`. A number at or past [`Picture::nf`] gives the last frame.
    ///
    /// The frame's report is dropped here (a debug build stops on a hull error). A render job, which must not
    /// write a frame that is not the kit's, calls [`Picture::frame_reported`] and reads the report.
    pub fn frame(&self, f: usize) -> Rgb8 {
        self.frame_logged(f).0
    }

    /// Frame `f` with its draw log: every caption, label and panel box drawn, for the safe-zone check. The
    /// report is dropped, as in [`Picture::frame`].
    pub fn frame_logged(&self, f: usize) -> (Rgb8, DrawLog) {
        let (img, log, report) = self.frame_reported(f);
        debug_assert_eq!(
            report.hull_errors, 0,
            "frame {f}: a hull scipy would refuse was left out"
        );
        (img, log)
    }

    /// Frame `f` with its draw log and its report (the hulls not drawn in scipy's order).
    ///
    /// `hull_fallbacks` counts every hull of the frame that fell back for a Qhull merge: the beam's and those the
    /// looks ask for in their hooks ([`crate::hull::convex_hull`] counts them on the drawing thread), so a merge
    /// in a look's hull is counted too, never silently drawn another way.
    pub fn frame_reported(&self, f: usize) -> (Rgb8, DrawLog, FrameReport) {
        let merges = crate::hull::merges();
        match self.render(f.min(self.nf() - 1), None) {
            Rendered::Frame(img, log, mut report) => {
                // the render counted the beam's merge; this thread's count since holds the beam's and the looks'
                report.hull_fallbacks = (crate::hull::merges() - merges) as usize;
                (img, log, report)
            }
            Rendered::Stage(_) => unreachable!("render without a stop returns the frame"),
        }
    }

    /// The buffers of frame `f` after `stage`, in the dtype the renderer holds them in there.
    pub fn stage_buffers(&self, f: usize, stage: Stage) -> StageBuffers {
        match self.render(f.min(self.nf() - 1), Some(stage)) {
            Rendered::Stage(s) => s,
            Rendered::Frame(..) => unreachable!("render stops at every stage"),
        }
    }

    /// The colour buffer of frame `f` after `stage`, as `float64` (for [`Stage::Coc`]: the map, one channel).
    pub fn stage(&self, f: usize, stage: Stage) -> RgbF {
        self.stage_buffers(f, stage).rgb.to_f64()
    }
}

/// `upscale3(img)`: the frame three times as large, nearest neighbour: 1080 x 1920.
pub fn upscale3(img: &Rgb8) -> Rgb8 {
    let up = crate::config::UP;
    let mut out: Img<u8> = Img::new(img.w * up, img.h * up, img.c);
    let c = img.c;
    for y in 0..img.h {
        let src = img.row(y);
        let mut line = Vec::with_capacity(img.w * up * c);
        for px in src.chunks_exact(c) {
            for _ in 0..up {
                line.extend_from_slice(px);
            }
        }
        for dy in 0..up {
            out.row_mut(y * up + dy).copy_from_slice(&line);
        }
    }
    out
}
