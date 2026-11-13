//! One frame: `render(f)` of `kit/picture.py`, every stage and every hook call in the order the Python file
//! makes it, and the buffer after each stage for finding where a frame first differs.
//!
//! The pipeline: arena floor (per pixel), stands and crowd, ring skirt, canvas (per pixel), back ropes and posts,
//! the balls and the fighters' items (painter), front ropes and posts, the fighters' effects over the ring,
//! shards, sparks, particles, lens, shimmers, glows, spot beam, bloom, grade, scrim, the fighters' crisp
//! overlays, post fx, count-in, HP numbers, fight HUD, captions, the VS title card.

use std::any::Any;
use std::sync::OnceLock;

use bf_core::dmath;

use super::Picture;
use super::canvas::circle_angles;
use crate::camera::V3;
use crate::config::{FPS, H, W, ss};
use crate::hull::{self, Unusual};
use crate::img::{Img, Rgb8};
use crate::lens;
use crate::look::{DrawnBall, FrameCtx, Payload, Pic};
use crate::pil;
use crate::post;
use crate::raster::{self, Frame, Glow};
use crate::scene::{self, BACK, BLUE, CORNERS, FRONT, R_BALL as R, RED, SPOT, SUBJECT};
use crate::ui::DrawLog;
use crate::v3;

/// A stage of the pipeline, in order: the names of `tools/studio_export/stages.py`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// The arena floor, lit per pixel and dithered: the float buffers handed to the frame.
    Floor,
    /// After the stands, the crowd and the LED boards.
    Arena,
    /// After the ring platform's skirt.
    Skirt,
    /// The canvas, lit per pixel and dithered, with the looks' canvas overlays: the float buffers.
    Canvas,
    /// After the painter pass: back post and ropes, the balls, the looks' items, the side posts, front ropes and
    /// post, and the late balls (`ball_late`).
    Painter,
    /// After both looks' over-ring effects and the K.O. shards.
    OverRing,
    /// After the hit sparks.
    Sparks,
    /// After the particles: the float buffers handed to the frame.
    Particles,
    /// The circle-of-confusion radius per pixel.
    Coc,
    /// After the depth of field.
    Lens,
    /// After the looks' heat shimmers.
    Shimmer,
    /// After the glow sprites.
    Glows,
    /// After the spot's beam and the can at the top of the frame, clipped to 0..255.
    Beam,
    /// After the bloom.
    Bloom,
    /// After the grade.
    Grade,
    /// After the vignette and the top scrim, clipped to 0..255.
    Scrim,
    /// After both looks' crisp overlays and the post effects.
    PostFx,
    /// The picture cut to bytes, with the looks' `after_post` and the count-in drawn.
    Countdown,
    /// After the HP numbers, the fight HUD and the captions.
    Hud,
    /// After the VS title card: the frame.
    Title,
}

impl Stage {
    /// Every stage, in pipeline order.
    pub const ALL: [Stage; 20] = [
        Stage::Floor,
        Stage::Arena,
        Stage::Skirt,
        Stage::Canvas,
        Stage::Painter,
        Stage::OverRing,
        Stage::Sparks,
        Stage::Particles,
        Stage::Coc,
        Stage::Lens,
        Stage::Shimmer,
        Stage::Glows,
        Stage::Beam,
        Stage::Bloom,
        Stage::Grade,
        Stage::Scrim,
        Stage::PostFx,
        Stage::Countdown,
        Stage::Hud,
        Stage::Title,
    ];

    /// The stage's name in the export tool's dumps.
    pub fn name(self) -> &'static str {
        match self {
            Stage::Floor => "floor",
            Stage::Arena => "arena",
            Stage::Skirt => "skirt",
            Stage::Canvas => "canvas",
            Stage::Painter => "painter",
            Stage::OverRing => "over_ring",
            Stage::Sparks => "sparks",
            Stage::Particles => "particles",
            Stage::Coc => "coc",
            Stage::Lens => "lens",
            Stage::Shimmer => "shimmer",
            Stage::Glows => "glows",
            Stage::Beam => "beam",
            Stage::Bloom => "bloom",
            Stage::Grade => "grade",
            Stage::Scrim => "scrim",
            Stage::PostFx => "post_fx",
            Stage::Countdown => "countdown",
            Stage::Hud => "hud",
            Stage::Title => "title",
        }
    }
}

/// A stage's colour buffer in the dtype the renderer holds it in at that point.
#[derive(Clone, Debug, PartialEq)]
pub enum StagePic {
    U8(Img<u8>),
    F32(Img<f32>),
    F64(Img<f64>),
}

impl StagePic {
    /// The buffer as `float64` (bytes and `float32` samples widen exactly).
    pub fn to_f64(&self) -> Img<f64> {
        match self {
            StagePic::U8(a) => Img::from_vec(a.w, a.h, a.c, a.data.iter().map(|&v| f64::from(v)).collect()),
            StagePic::F32(a) => Img::from_vec(a.w, a.h, a.c, a.data.iter().map(|&v| f64::from(v)).collect()),
            StagePic::F64(a) => a.clone(),
        }
    }
}

impl From<&Pic> for StagePic {
    fn from(p: &Pic) -> StagePic {
        match p {
            Pic::F32(a) => StagePic::F32(a.clone()),
            Pic::F64(a) => StagePic::F64(a.clone()),
        }
    }
}

/// The buffers after one stage: the colour buffer (for [`Stage::Coc`], the map, one channel) and the depth
/// buffer where the stage has one.
#[derive(Clone, Debug, PartialEq)]
pub struct StageBuffers {
    pub stage: Stage,
    pub rgb: StagePic,
    pub depth: Option<Img<f32>>,
}

/// What a frame reports besides its picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameReport {
    /// Hulls drawn in the plain scan's order because Qhull's order is not known for them
    /// ([`Unusual::Merge`]). Expected: none.
    pub hull_fallbacks: usize,
    /// The spot beam's hull when scipy would refuse it (flat, or too few points), where the Python frame fails:
    /// the beam is not drawn, and a frame with one is not the kit's frame. Expected: none. It counts the beam
    /// only: a look's hull that comes back flat or with too few points is skipped by the look, as the kit's
    /// `try/except` round its `ConvexHull` does (ORGAN, ALTO SAX), and is no error of the frame.
    pub hull_errors: usize,
}

/// What [`Picture::render`] returns.
pub(super) enum Rendered {
    Frame(Rgb8, DrawLog, FrameReport),
    Stage(StageBuffers),
}

/// A painter item: a side post, a ball, or a look's item.
enum Item {
    Post(usize),
    Ball(usize),
    Look(usize, Payload),
}

/// `tuple(col)` as the colour of a glow.
fn glow(fr: &mut Frame, s: (f64, f64), r: f64, col: V3, inten: f64) {
    fr.glow(s.0, s.1, r, col, inten);
}

impl Picture {
    /// `render(f)`: the frame, or the buffers after `stop`.
    pub(super) fn render(&self, f: usize, stop: Option<Stage>) -> Rendered {
        let p = &self.p;
        let g = &p.show.grid;
        macro_rules! stage {
            ($stage:expr, $rgb:expr, $depth:expr) => {
                if stop == Some($stage) {
                    return Rendered::Stage(StageBuffers {
                        stage: $stage,
                        rgb: $rgb,
                        depth: $depth,
                    });
                }
            };
        }
        let mut report = FrameReport::default();
        let t = f as f64 / f64::from(FPS);
        let cam = p.camera(t);
        let l = p.lights(t, &cam);
        let fs = [p.fighter(0, t), p.fighter(1, t)];
        let i = p.sim_i(t);
        let in_fight = g.t_fight <= t && t < p.t_hs;
        let spot = SPOT;
        let aim = p.spot_aim(t);
        let to_aim = v3::sub(aim, spot);
        let axis = v3::div(to_aim, v3::norm(to_aim));
        let wk = p.win_k(t);
        let mut fr = Frame::new([10.0, 8.0, 18.0]);
        let fc = FrameCtx {
            t,
            f,
            cam: &cam,
            l: &l,
            fs: &fs,
            i,
            in_fight,
            spot,
            memo: [OnceLock::<Box<dyn Any + Send + Sync>>::new(), OnceLock::new()],
        };
        // ---------------- arena floor: per pixel (spill under the ring, corner lights)
        let (mut arr, mut depth) = (fr.array(), fr.depth());
        p.floor(&cam, &l, &mut arr, &mut depth);
        stage!(Stage::Floor, StagePic::F32(arr), Some(depth));
        fr.set_array(&arr, Some(&depth));
        p.arena(&mut fr, &cam, &l, t);
        stage!(Stage::Arena, StagePic::U8(fr.pil), Some(fr.dep));
        // ---------------- ring skirt (front sides only face the camera)
        p.skirt(&mut fr, &cam, &l);
        stage!(Stage::Skirt, StagePic::U8(fr.pil), Some(fr.dep));
        // ---------------- canvas: per pixel
        let (mut arr, mut depth) = (fr.array(), fr.depth());
        p.canvas(&fc, &fs, axis, &mut arr, &mut depth);
        // lines on the canvas
        for look in &p.looks {
            look.canvas_overlay(p, &fc, &mut arr);
        }
        stage!(Stage::Canvas, StagePic::F32(arr), Some(depth));
        fr.set_array(&arr, Some(&depth));
        // ---------------- back ropes + posts
        p.draw_post(&mut fr, &cam, &l, BACK);
        p.draw_rope_side(&mut fr, &cam, &l, t, 2);
        p.draw_rope_side(&mut fr, &cam, &l, t, 3);
        // ---------------- balls, the fighters' items, posts (painter by depth)
        let mut items: Vec<(f64, Item)> = Vec::new();
        // a late ball and its look's items_ball are drawn after the front ropes (opt-in `ball_late`)
        let mut late_items: Vec<(f64, Item)> = Vec::new();
        for corner in [RED, BLUE] {
            let w = scene::to_world(CORNERS[corner][0], CORNERS[corner][1], 0.7);
            items.push((cam.depth(&[w])[0], Item::Post(corner)));
        }
        for (k, st) in fs.iter().enumerate() {
            if !st.vis || st.scale < 0.05 {
                continue;
            }
            // pw = S.to_world(p[0], p[1], st.get('paint_h', R + st['lift']))
            let pw = scene::to_world(st.p[0], st.p[1], st.paint_h.unwrap_or(R + st.lift));
            let dz = cam.depth(&[pw])[0] - st.air_bias;
            let late = p.looks.iter().any(|l| l.ball_late(p, k, &fc, st));
            let dest = if late { &mut late_items } else { &mut items };
            dest.push((dz, Item::Ball(k)));
            for (dz, pay) in p.looks[k].items_ball(p, &fc, st) {
                dest.push((dz, Item::Look(k, pay)));
            }
        }
        for k in 0..2 {
            for (dz, pay) in p.looks[k].items_free(p, &fc) {
                items.push((dz, Item::Look(k, pay)));
            }
        }
        // items.sort(key=lambda it: -it[0]): a stable sort, far to near
        let by_depth = |a: &(f64, Item), b: &(f64, Item)| (-a.0 + 0.0).total_cmp(&(-b.0 + 0.0));
        items.sort_by(by_depth);
        let mut drawn: Vec<DrawnBall> = Vec::new();
        let paint = |fr: &mut Frame, items: &[(f64, Item)], drawn: &mut Vec<DrawnBall>| {
            for (_, item) in items {
                match item {
                    Item::Post(corner) => p.draw_post(fr, &cam, &l, *corner),
                    Item::Ball(k) => {
                        let (mut arr, mut depth) = (fr.array(), fr.depth());
                        let (s, rp) = p.draw_ball(&mut arr, &mut depth, &cam, *k, &fs[*k], t);
                        drawn.push(DrawnBall { k: *k, s, rp });
                        fr.set_array(&arr, Some(&depth));
                    }
                    Item::Look(k, pay) => p.looks[*k].draw_item(p, &fc, fr, pay),
                }
            }
        };
        paint(&mut fr, &items, &mut drawn);
        // ---------------- front ropes + posts
        p.draw_rope_side(&mut fr, &cam, &l, t, 0);
        p.draw_rope_side(&mut fr, &cam, &l, t, 1);
        p.draw_post(&mut fr, &cam, &l, FRONT);
        late_items.sort_by(by_depth);
        paint(&mut fr, &late_items, &mut drawn);
        let (mut arr, mut depth) = (fr.array(), fr.depth());
        stage!(Stage::Painter, StagePic::F32(arr), Some(depth));
        // ---------------- the fighters' effects over the ring, shards, sparks
        for look in &p.looks {
            look.over_ring(p, &fc, &mut fr, &mut arr, &mut depth);
        }
        p.draw_shards(&mut arr, &cam, t);
        stage!(Stage::OverRing, StagePic::F32(arr), None);
        p.sparks(&mut arr, &cam, t);
        stage!(Stage::Sparks, StagePic::F32(arr), None);
        let tc = p.fx_clock(t);
        // attack lights colour the particles too: light = lambda P: 1.0 + 0.6 * FL.tint(P, tc_)
        let light = |pts: &[V3]| -> Vec<V3> {
            p.fx.fl
                .tint(pts, tc, 1.0)
                .iter()
                .map(|c| c.map(|v| 1.0 + 0.6 * v))
                .collect()
        };
        if let Some(new) = p.fx.pa.draw(&arr, &cam, tc, Some(&light)) {
            arr = new;
        }
        // the countdown's spot in the ring
        let cs = cam.wp(&[[0.0, 0.3, 0.0]])[0];
        stage!(Stage::Particles, StagePic::F32(arr), Some(depth));
        fr.set_array(&arr, Some(&depth));
        // ---------------- lens (focus locked on the ring), glows, the spot beam, bloom, grade
        let (img32, depth) = (fr.array(), fr.depth());
        let zf = cam.depth(&[SUBJECT])[0];
        let coc = lens::coc_map(&depth, zf, 26.0, 0.17);
        stage!(Stage::Coc, StagePic::F64(coc), None);
        let mut img = match lens::dof(&img32, &coc) {
            Some(blurred) => Pic::F64(blurred),
            None => Pic::F32(img32),
        };
        stage!(Stage::Lens, StagePic::from(&img), None);
        p.shimmers(&mut img, &cam, t);
        stage!(Stage::Shimmer, StagePic::from(&img), None);
        let pulse = 0.35 + 0.15 * g.beat_pulse(t, 0.2, 0.12);
        for (lp, col) in [(p.scene.red_light, p.col[0]), (p.scene.blue_light, p.col[1])] {
            glow(&mut fr, cam.wp(&[lp])[0], 16.0, col, pulse);
        }
        for d in &drawn {
            let st = &fs[d.k];
            if st.glow > 0.05 {
                glow(
                    &mut fr,
                    d.s,
                    d.rp * 2.2,
                    p.col[d.k].map(|c| c * 0.8 + 50.0),
                    0.45 * st.glow,
                );
            }
        }
        for look in &p.looks {
            look.glows(p, &fc, &mut fr, &drawn);
        }
        for ei in p.events_between(t - 0.15, t, &["hit"]) {
            let e = &p.rec.events[ei];
            let att = super::checked_owner(e);
            if let Some((r, a)) = p.looks[att].hit_glow(p, e) {
                let s = cam.wp(&[scene::to_world(e.f(5), e.f(6), R)])[0];
                let col = p.fl_col[att].map(|c| c * 0.7 + 70.0);
                glow(&mut fr, s, r, col, a * (1.0 - (t - e.t) / 0.15));
            }
        }
        let pk = p.p_ko[p.lose];
        if p.t_ko <= t && t < p.t_hs + 0.25 {
            let s = cam.wp(&[scene::to_world(pk[0], pk[1], R)])[0];
            let k = 0.9 * (1.0 - ss(p.t_ko, p.t_hs + 0.25, t));
            glow(&mut fr, s, 46.0, [255.0, 240.0, 220.0], k);
        }
        match &mut img {
            Pic::F32(a) => raster::apply_glows(a, &fr.glows),
            Pic::F64(a) => raster::apply_glows(a, &fr.glows),
        }
        stage!(Stage::Glows, StagePic::from(&img), None);
        // the spot's beam: a soft wedge of light from the can down to the pool
        let s0 = cam.wp(&[spot])[0];
        let pool_r = spot[1] * dmath::tan(dmath::acos(scene::spot_cos().0)) * 0.92;
        let circle: Vec<V3> = circle_angles(32)
            .iter()
            .map(|&a| {
                [
                    aim[0] + pool_r * dmath::cos(a),
                    aim[1] + 0.0,
                    aim[2] + pool_r * dmath::sin(a),
                ]
            })
            .collect();
        let mut pts = vec![s0];
        pts.extend(cam.wp(&circle));
        let mut mask: Img<u8> = Img::new(W, H, 1);
        // hull = pts[ConvexHull(pts).vertices]: scipy raises for a flat hull, and the Python frame fails
        let hull = hull::convex_hull(&pts);
        match hull.fallback {
            Some(Unusual::Flat) | Some(Unusual::NotEnoughPoints) => report.hull_errors += 1,
            other => {
                if other.is_some() {
                    report.hull_fallbacks += 1;
                }
                let poly: Vec<(f64, f64)> = hull.vertices.iter().map(|&v| pts[v]).collect();
                if poly.len() >= 2 {
                    pil::polygon(&mut mask, &poly, &[255]);
                }
            }
        }
        let beam_k = 0.10 + 0.04 * wk;
        let tint = [255.0, 246.0, 230.0];
        let mut img64 = img.to_f64();
        drop(img);
        for y in 0..H {
            // fall = np.clip(1 - (YY_ - s0[1]) / 520.0, 0.25, 1.0)
            let fall = crate::np::clip(1.0 - (y as f64 - s0.1) / 520.0, 0.25, 1.0);
            for x in 0..W {
                // mk = np.asarray(m, np.float32) / 255.0
                let mk = f64::from(f32::from(mask.data[y * W + x]) / 255.0f32);
                // beam = np.floor((mk * fall * (0.10 + 0.04 * wk)) * 10 + BAYER_FULL * 0.9) / 10
                let beam = ((mk * fall * beam_k) * 10.0 + super::bayer(x, y) * 0.9).floor() / 10.0;
                let px = img64.px_mut(x, y);
                for c in 0..3 {
                    // img = img + beam[..., None] * np.array([255, 246, 230])[None, None] * 0.55
                    px[c] += beam * tint[c] * 0.55;
                }
            }
        }
        // the can + truss at the top of frame
        if -30.0 < s0.1 && s0.1 < 60.0 {
            let can = |r: f64, col: V3, inten: f64| Glow {
                x: s0.0,
                y: s0.1,
                r,
                col,
                inten,
            };
            raster::apply_glows(
                &mut img64,
                &[
                    can(30.0, [255.0, 244.0, 220.0], 0.9),
                    can(10.0, [255.0, 255.0, 255.0], 1.0),
                ],
            );
        }
        // img = grade(bloom(np.clip(img, 0, 255), thr=225, k=0.22), contrast=0.18, sat=1.18)
        for v in &mut img64.data {
            *v = crate::np::clip(*v, 0.0, 255.0);
        }
        stage!(Stage::Beam, StagePic::F64(img64), None);
        let img64 = post::bloom(&img64, 225.0, 0.22, None);
        stage!(Stage::Bloom, StagePic::F64(img64), None);
        let mut img64 = post::grade(&img64, 0.18, 1.18, 0.0, 1.0);
        stage!(Stage::Grade, StagePic::F64(img64), None);
        // img = np.clip(img * vignette(strength=0.45) * top_scrim(0.45, 100, 90), 0, 255)
        let (vig, scrim) = (
            post::vignette(0.45, 0.55, 0.5, 0.47),
            post::top_scrim(0.45, 100.0, 90.0),
        );
        for (i, c) in img64.data.iter_mut().enumerate() {
            *c = crate::np::clip(*c * vig.data[i / 3] * scrim.data[i / 3], 0.0, 255.0);
        }
        stage!(Stage::Scrim, StagePic::F64(img64), None);
        let mut img = Pic::F64(img64);
        // crisp status overlays (after the grade)
        for look in &p.looks {
            look.after_grade(p, &mut img, &fc, &drawn);
        }
        p.post_fx(&mut img, &cam, t, &fs, &drawn);
        stage!(Stage::PostFx, StagePic::from(&img), None);
        let mut out = img.to_bytes();
        drop(img);
        let mut log = DrawLog::new();
        // the looks' overlays on the bytes (opt-in `after_post`)
        for look in &p.looks {
            look.after_post(p, &mut out, &mut log, &fc, &drawn);
        }
        // ---------------- UI: countdown, HP numbers in the balls, fight HUD, captions
        p.countdown(&mut out, &mut log, t, cs);
        stage!(Stage::Countdown, StagePic::U8(out), None);
        // the title card covers the HUD: skip it
        let card_closed = self.card.door_offset(t) == 0.0;
        p.draw_ui(&mut out, &mut log, t, &fs, &drawn, card_closed);
        stage!(Stage::Hud, StagePic::U8(out), None);
        // the VS title card, over everything
        self.card.draw_logged(&mut out, &mut log, t);
        stage!(Stage::Title, StagePic::U8(out), None);
        Rendered::Frame(out, log, report)
    }

    /// The subject points of frame `f` with the circle of confusion of each, in pixels: the port of the focus
    /// check's inner loop (`kit/checks.py` `focus`): the ring's corners, both balls (bottom and top), each
    /// look's `focus_points`, the projectiles in flight and the shards. A frame is in focus when every value is 0.
    pub fn focus_points(&self, f: usize) -> Vec<(f64, String)> {
        let p = &self.p;
        let f = f.min(self.nf().saturating_sub(1));
        let t = f as f64 / f64::from(FPS);
        let cam = p.camera(t);
        let zf = cam.depth(&[SUBJECT])[0];
        let mut pts: Vec<(V3, String)> = Vec::new();
        // corners = [S.to_world(sx, sz, y) for sx in (-AP, AP) for sz in (-AP, AP) for y in (0.0, POST_H)]
        for sx in [-scene::AP, scene::AP] {
            for sz in [-scene::AP, scene::AP] {
                for y in [0.0, scene::POST_H] {
                    pts.push((scene::to_world(sx, sz, y), "ring".to_string()));
                }
            }
        }
        let fs = [p.fighter(0, t), p.fighter(1, t)];
        for (k, st) in fs.iter().enumerate() {
            for dy in [0.0, 2.0 * R] {
                pts.push((scene::to_world(st.p[0], st.p[1], dy), p.names[k].clone()));
            }
            pts.extend(p.looks[k].focus_points(p, t, st));
        }
        // the projectiles in flight: for row in RUN['K'][sim_i(t)] that is not NaN
        if p.show.grid.t_fight <= t && t < p.t_hs {
            for row in p.run.projectiles(p.sim_i(t)) {
                pts.push((scene::to_world(row[0], row[1], 0.45), "key".to_string()));
            }
        }
        for part in p.parts_at(t) {
            pts.push((scene::to_world(part.p[0], part.p[2], part.p[1]), "shard".to_string()));
        }
        let top = lens::LEVELS[lens::LEVELS.len() - 1];
        pts.into_iter()
            .map(|(w, name)| {
                // coc_map on float64 depths: np.clip(K * np.maximum(0.0, np.abs(1 - zf / np.maximum(d, 1e-3)) - sharp), 0, top)
                let d = crate::np::maximum(cam.depth(&[w])[0], 1e-3);
                let off = crate::np::maximum(0.0, (1.0 - zf / d).abs() - 0.17);
                (crate::np::clip(26.0 * off, 0.0, top), name)
            })
            .collect()
    }
}
