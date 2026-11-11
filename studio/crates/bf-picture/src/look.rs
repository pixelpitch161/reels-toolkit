//! The `Look` trait: how a fighter draws in the ring. The port of `Look` in `projects/ballfight/kit/fighters/base.py`.
//!
//! A fighter's look subclasses the Python base class and overrides the hooks it needs; every hook is optional and
//! the defaults draw nothing beyond the shared ball (glass and the model's sprite), sparks and shatter. The ring
//! renderer ([`crate::ring`]) calls the hooks in a fixed order, the order `kit/picture.py` makes them in, so a
//! look only ever adds its own layers. Here a look implements this trait: one method per hook, with the same
//! names and the same defaults, plus the base class's attributes as methods.
//!
//! Where a Python hook reads the renderer module `P` (`self.P`), the hook here takes `p: &PictureCtx`: the run,
//! the timeline, the colours, the built effects and the renderer's helpers (`fighter`, `sim_i`, `solo_env`, ...).
//! A Python look keeps `P`, its side `k` and its own precomputed state from [`Look::setup`]; here `setup` hands
//! the look its side and the context once, before any other hook, and the look keeps what it needs.
//!
//! The hooks that build effects (`fx_setup`, `fx_event`, `fx_after`, `ko_fx`, `fx_end`) also take `fx`, the
//! effects being built ([`Fx`]): `P.FL`, `P.BU`, `P.PA`, `P.MK` and `P.MO`. All looks share the one
//! [`Particles`](crate::fx::Particles) generator, so the order of their `emit` calls is part of the picture.

use std::any::Any;
use std::sync::{Arc, OnceLock};

use bf_core::event::Event;

use crate::camera::{Cam, V3};
use crate::fx::{Bursts, Emit, Flares, Marks, Moments, Particles};
use crate::img::{Img, RgbF};
use crate::lighting::Lights;
use crate::raster::Frame;
use crate::ring::PictureCtx;
use crate::ring::state::{BallState, Wave};
use crate::title::GLASS;

/// The phase `fighter(k, t)` is in when it calls [`Look::state`]: Python's `'rev'`, `'fight'` and `'tail'`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// In the corners, before FIGHT!.
    Rev,
    /// The fight, through the hit-stop.
    Fight,
    /// After the K.O. (`gl` is the glide home, 0 to 1).
    Tail,
}

/// The effects of one picture, `kit/picture.py`'s `FL`, `BU`, `PA`, `MK` and `MO`: built once from the run's
/// events (`_fx_build`), then only read.
#[derive(Clone, Debug)]
pub struct Fx {
    /// `FL = fx.Flares(max_n=6)`.
    pub fl: Flares,
    /// `BU = fx.Bursts()`.
    pub bu: Bursts,
    /// `PA = fx.Particles(seed=T.FX_SEED)`.
    pub pa: Particles,
    /// `MK = fx.Marks()`.
    pub mk: Marks,
    /// `MO = fx.Moments()`.
    pub mo: Moments,
}

impl Fx {
    /// The five empty effect objects with Python's defaults; `fx_seed` seeds the particles.
    pub fn new(fx_seed: u64) -> Fx {
        Fx {
            fl: Flares::new(6, 3.0),
            bu: Bursts::new(3, 7.0, 9.0, 2),
            pa: Particles::new(fx_seed, 600),
            mk: Marks::new(12.0, 40),
            mo: Moments::new(0.3, 0.55, 0.14, 0.4),
        }
    }
}

/// A float picture between the lens and the cut to bytes. numpy carries either dtype there and the arithmetic
/// follows it: the lens returns `float32` when nothing is out of focus and `float64` otherwise, and the particle
/// and speed-line draws return `float32`.
#[derive(Clone, Debug, PartialEq)]
pub enum Pic {
    F32(Img<f32>),
    F64(Img<f64>),
}

/// What the painter pass drew for a ball: Python's `drawn[k] = (s, rp)`, in the order the balls were drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawnBall {
    /// The fighter, 0 or 1.
    pub k: usize,
    /// `s`: the ball's centre on the screen.
    pub s: (f64, f64),
    /// `rp`: its radius in pixels.
    pub rp: f64,
}

/// The per-frame context handed to the looks: Python's `F = Ctx(t, f, cam, L, fs, i, in_fight, spot, fr)`. The
/// frame being drawn (`F.fr`) is handed to the hooks that draw as its own argument.
pub struct FrameCtx<'a> {
    /// The frame's time, `f / FPS`.
    pub t: f64,
    /// The frame's number.
    pub f: usize,
    /// The camera, with the shake's offset.
    pub cam: &'a Cam,
    /// `L`: the lights.
    pub l: &'a Lights,
    /// `fs`: both fighters' picture state.
    pub fs: &'a [BallState; 2],
    /// `i = sim_i(t)`: the recording frame.
    pub i: usize,
    /// `T.T_FIGHT <= t < T_HS`.
    pub in_fight: bool,
    /// `S.SPOT`: the spotlight's position.
    pub spot: V3,
    /// One slot per look for what it computes once a frame (the FLUTE's `cobs(F)` keeps its cobras on `F`). The
    /// frame never reads it.
    pub memo: [OnceLock<Box<dyn Any + Send + Sync>>; 2],
}

/// The canvas context: Python's `C = Ctx(t, i, in_fight, fs, sxp, szp, ys, xs)`, the canvas pixels of a frame.
pub struct CanvasCtx<'a> {
    pub t: f64,
    pub i: usize,
    pub in_fight: bool,
    pub fs: &'a [BallState; 2],
    /// The ring-frame x of each canvas pixel.
    pub sxp: &'a [f64],
    /// The ring-frame z of each canvas pixel.
    pub szp: &'a [f64],
    /// The screen row of each canvas pixel (`BAYER_FULL[C.ys, C.xs]` is `config::bayer_full().px(x, y)[0]`).
    pub ys: &'a [usize],
    /// The screen column of each canvas pixel.
    pub xs: &'a [usize],
}

/// What a painter item carries from [`Look::items_ball`] or [`Look::items_free`] to [`Look::draw_item`].
pub type Payload = Box<dyn Any + Send>;

/// `(colour, n, speed, life)` of a spark spray.
pub type Spark = (V3, usize, f64, f64);

/// The tint of [`BallSprite`]: `fn(lit) -> lit` on the lit sprite, a `float32` RGB picture of the sprite's part
/// on the frame. It changes the picture in place, so the result is `float32`: a Python tint that returns a
/// `float64` array (a product with a numpy array of colours, say) cannot be written as it is, and its port must
/// round each sample to `f32` last, which is what storing into the frame does to it in Python anyway (the
/// renderer only clips the result and stores it).
pub type Tint<'a> = Box<dyn Fn(&mut Img<f32>) + 'a>;

/// What [`Look::ball_sprite`] returns: the sprite to draw in the ball instead of the usual one, and its tint.
pub struct BallSprite<'a> {
    /// The RGBA sprite, square.
    pub sprite: Arc<Img<u8>>,
    /// `fn(lit) -> lit`, or `None`.
    pub tint: Option<Tint<'a>>,
}

/// Whether `how` is one of `kinds` (Python's `how in self.spark_big`).
fn has(kinds: &[&str], how: &str) -> bool {
    kinds.contains(&how)
}

/// `e[7]`, the `how` of a `hit` event, or `""` when the event has none (Python would raise).
fn how(e: &Event) -> &str {
    match e.fields.get(5) {
        Some(bf_core::event::Value::Str(s)) => s,
        _ => "",
    }
}

/// How a fighter draws in the ring: one method per hook of `kit/fighters/base.py` `Look`, with its defaults.
///
/// `Send + Sync`: frames are drawn from several threads at once, and every hook but [`Look::setup`] takes
/// `&self`.
#[allow(unused_variables)] // the defaults ignore most of their arguments
pub trait Look: Send + Sync {
    // ------------------------------------------------------------ the class attributes
    /// `glass`: its ball's glass (the ring and the VS card's close-up). The default is the cream [`GLASS`].
    fn glass(&self) -> [f64; 3] {
        GLASS
    }

    /// `flare_scale`: its attack lights are its colour times this (a near-white fighter: 0.85).
    fn flare_scale(&self) -> f64 {
        1.0
    }

    /// `shards`: its parts in the K.O. shatter (after the 24 glass shards), drawn by [`Look::draw_shard`].
    fn shards(&self) -> Vec<String> {
        Vec::new()
    }

    /// `spark_big`: its hit kinds with the big spark spray.
    fn spark_big(&self) -> &[&str] {
        &["slam", "wave", "shockwave", "cannon"]
    }

    /// `spark_flash`: its hit kinds with a 3 px white impact point.
    fn spark_flash(&self) -> &[&str] {
        &[]
    }

    /// `flash_hits`: its hit kinds that set off the crowd's flashbulbs.
    fn flash_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave"]
    }

    /// `fringe_hits`: its hit kinds that get chromatic fringing.
    fn fringe_hits(&self) -> &[&str] {
        &["slam", "wave", "shockwave"]
    }

    /// `quiet_hits`: its hit kinds that do not flash the victim's HP bar.
    fn quiet_hits(&self) -> &[&str] {
        &[]
    }

    /// `teleports`: its ball can jump, so no speed lines across a teleport.
    fn teleports(&self) -> bool {
        false
    }

    /// `speed_lift`: its speed lines are measured at its ball's lift.
    fn speed_lift(&self) -> bool {
        false
    }

    /// `rope_kinds`: its ability kinds that shake the ropes, with the strength.
    fn rope_kinds(&self) -> &[(&str, f64)] {
        &[("wave", 1.0), ("shockwave", 0.6)]
    }

    /// `sprite_k`: its instrument's size in the ball, times the ball's radius.
    fn sprite_k(&self) -> f64 {
        0.80
    }

    /// `block_burst`: the ink burst where a hit lands on its shield.
    fn block_burst(&self) -> [f64; 3] {
        [255.0, 226.0, 150.0]
    }

    /// `block_spark`: `(colour, n, speed, life)` of that block's spark spray.
    fn block_spark(&self) -> Spark {
        ([160.0, 220.0, 255.0], 12, 2.6, 0.3)
    }

    /// `title_pattern`: the fighter's panel pattern on the VS card. `base` is the panel's gradient, an
    /// `(H, W, 3)` `float64` picture not yet dithered; a look with a pattern changes it in place. The default
    /// (`title_pattern = None`) leaves it alone: the panel stays plain.
    ///
    /// A pattern reads the card's geometry from [`crate::title::layout`]: `side()` is `title.SIDE`, `dd()` is
    /// `title.DD`, and `title.XX`, `title.YY` are the pixel's column and row.
    ///
    /// As in Python (`PATTERNS.setdefault(name, look.title_pattern)`), a pattern the card itself has for the
    /// fighter's name ([`crate::title::panels::builtin_pattern`]) is used instead, and this hook is then not
    /// called.
    fn title_pattern(&self, base: &mut RgbF) {}

    // ------------------------------------------------------------ setup (once, from the run)
    /// `setup()`: precompute its state from the run's events. Called once, before every other hook, with the
    /// look's side `k` (Python's `Look(P, k)` and `setup()` in one). The other fighter's look is not in `p` yet
    /// (`p.looks` is empty), so `p.fighter` cannot be called here.
    fn setup(&mut self, p: &PictureCtx, k: usize) {}

    /// `title_closeup(r)`: the look's own close-up sprite for the VS card, RGBA and square, for a ball of radius
    /// `r` pixels; `None` (the default) for the fighter's model.
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        None
    }

    // ------------------------------------------------------------ the ball
    /// `sprite(r, yaw, roll)`: its own in-ball sprite (RGBA, square), or `None` for the model's.
    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        None
    }

    /// `sprite_rolls(kk, roll)`: extra sprite rolls ball `kk` needs. Python's sprite precompute reads it; the
    /// Studio draws sprites on demand, so nothing calls it.
    fn sprite_rolls(&self, kk: usize, roll: f64) -> Vec<f64> {
        Vec::new()
    }

    /// `ball(kk, st, t, glass, roll)`: changes ball `kk`'s glass tint and sprite roll at `t` (its debuffs on the
    /// enemy). Returns `(glass, roll)`.
    fn ball(&self, p: &PictureCtx, kk: usize, st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        (glass, roll)
    }

    /// `ball_sprite(kk, st, t, rp, yaw)`: its own ball's instrument at `t` instead of the usual one, or `None`.
    fn ball_sprite<'a>(
        &'a self,
        p: &'a PictureCtx,
        kk: usize,
        st: &BallState,
        t: f64,
        rp: f64,
        yaw: f64,
    ) -> Option<BallSprite<'a>> {
        None
    }

    /// `extra_sprites()`: `[(side, yaw centre, yaw swing, roll)]`, more views for Python's sprite precompute.
    /// Nothing calls it here (sprites are drawn on demand).
    fn extra_sprites(&self) -> Vec<(usize, f64, f64, f64)> {
        Vec::new()
    }

    /// `state(kk, st, t, phase, gl)`: adds to ball `kk`'s picture state (its own weapon fields, or its status
    /// effects on the enemy). `gl` is the glide home in the tail, 0 otherwise.
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, gl: f64) {}

    /// `hides_hp(kk, t)`: is ball `kk` hidden under something of its (no HP number drawn)?
    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        false
    }

    // ------------------------------------------------------------ events -> camera, ropes, crowd, sparks
    /// `shake(e)`: the camera shake for one of its events, or `None`.
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "slam" { Some(1.6) } else { None }
    }

    /// `rope(e)`: the rope wobble strength for one of its events, or `None`.
    fn rope(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        if e.kind == "ability" {
            // self.rope_kinds.get(e[4])
            let kind = match e.fields.get(2) {
                Some(bf_core::event::Value::Str(s)) => s.as_str(),
                _ => return None,
            };
            return self
                .rope_kinds()
                .iter()
                .find(|(name, _)| *name == kind)
                .map(|&(_, k)| k);
        }
        if e.kind == "slam" { Some(0.9) } else { None }
    }

    /// `spark(e)`: `(colour, n, speed, life)` of the spark spray for one of its hits.
    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        let (n, sp, life) = if has(self.spark_big(), how(e)) {
            (16, 3.2, 0.3)
        } else {
            (5, 1.8, 0.18)
        };
        // P.COL[e[2]] * 0.6 + 100
        let col = p.col[crate::ring::checked_owner(e)];
        (col.map(|c| c * 0.6 + 100.0), n, sp, life)
    }

    /// `hit_glow(e)`: `(radius px, alpha)` of the glow on one of its fresh hits, or `None`.
    fn hit_glow(&self, p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        None
    }

    // ------------------------------------------------------------ effects, built once in time order
    /// `fx_setup()`: effects from its own state (before the event pass).
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {}

    /// `fx_event(e)`: effects for one of its events, in event order.
    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {}

    /// `fx_after()`: effects after the event pass.
    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {}

    /// `ko_fx(pos)`: its K.O. spray (it lost): particles from where it shattered.
    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        // self.P.PA.emit(self.P.T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))
        fx.pa.emit(
            p.t_ko,
            pos,
            "sparks",
            &Emit {
                n: Some(30),
                spread: Some(180.0),
                speed: Some((0.8, 2.2)),
                ..Emit::default()
            },
        );
    }

    /// `fx_end()`: effects after the K.O.'s.
    fn fx_end(&self, p: &PictureCtx, fx: &mut Fx) {}

    /// `light(L, t)`: its own point lights in the scene's lights at `t`, after the attack flares.
    fn light(&self, p: &PictureCtx, l: &mut Lights, t: f64) {}

    // ------------------------------------------------------------ the canvas (per pixel)
    /// `shadows(st, spot)`: extra casters right after its ball (its weapon), as polygons in the canvas grid's
    /// frame (world x and z).
    fn shadows(&self, p: &PictureCtx, st: &BallState, spot: V3) -> Vec<Vec<(f64, f64)>> {
        Vec::new()
    }

    /// `shadows_free(F, spot)`: extra casters after both balls (things it left on the canvas).
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        Vec::new()
    }

    /// `canvas_first(Ic, C)`: light under both fighters' decals. Opt-in in Python (`hasattr`); the default here
    /// changes nothing, which is the same.
    fn canvas_first(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {}

    /// `canvas(Ic, C)`: canvas light decals; `ic` holds the light of each canvas pixel.
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {}

    /// `canvas_wave(Ic, C, w)`: the canvas band of one of its expanding rings (a row of `RUN['WAVES']`).
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {}

    /// `canvas_after(Ic, C)`: decals after the rings (a slam's darkening).
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {}

    /// `canvas_overlay(F, arr)`: lines drawn on the lit canvas before the back ropes. `arr` is the `float32`
    /// picture.
    fn canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {}

    // ------------------------------------------------------------ the painter pass, effects over the ring
    /// `items_ball(F, st)`: `[(depth, payload)]` drawn in the painter pass, queued right after its ball.
    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        Vec::new()
    }

    /// `items_free(F)`: `[(depth, payload)]` queued after both balls (its things on the canvas).
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        Vec::new()
    }

    /// `draw_item(F, payload)`: draws one of its painter items into `fr` (Python's `F.fr`).
    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {}

    /// `over_ring(F, arr, depth)`: its effects over the ring, after the front ropes. `arr` and `depth` are the
    /// `float32` buffers taken from `fr`; a look that draws through `fr` sets them into it first and takes them
    /// back after, as the Python does.
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, arr: &mut Img<f32>, depth: &mut Img<f32>) {}

    /// `draw_shard(arr, cam, kind, p, rot, size, curl)`: one of its shatter parts (a kind from
    /// [`Look::shards`]); `pos` is `(x, y, z)`: ring-frame x, height, ring-frame z.
    #[allow(clippy::too_many_arguments)] // the Python signature
    fn draw_shard(
        &self,
        p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        curl: f64,
    ) {
    }

    // ------------------------------------------------------------ lens, glows, post
    /// `shimmer(img, cam, t, w)`: heat shimmer for one of its rings (a row of `RUN['WAVES']`).
    fn shimmer(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {}

    /// `shimmer_after(img, cam, t)`.
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {}

    /// `glows(F, drawn)`: its own glows (`F.fr.glow`).
    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, drawn: &[DrawnBall]) {}

    /// `fringe(e)`: does this hit of its get chromatic fringing?
    fn fringe(&self, p: &PictureCtx, e: &Event) -> bool {
        has(self.fringe_hits(), how(e))
    }

    /// `fringe_size(e)`: `(radius px, shift px)` of the fringe on one of its fringed hits.
    fn fringe_size(&self, p: &PictureCtx, e: &Event) -> (f64, f64) {
        (26.0, 2.5)
    }

    /// `after_grade(img, F, drawn)`: crisp overlays after the grade.
    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {}

    // ------------------------------------------------------------ checks
    /// `focus_points(t, st)`: `[(world point, label)]` of its subject parts that must stay in focus at `t`.
    fn focus_points(&self, p: &PictureCtx, t: f64, st: &BallState) -> Vec<(V3, String)> {
        Vec::new()
    }
}

/// Every default, as the Python base class: the bare ball.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefaultLook;

impl Look for DefaultLook {}
