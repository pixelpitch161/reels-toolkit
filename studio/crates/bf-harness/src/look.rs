//! The picture half of Template F: `fighters/<slug>/look.json` against the fighter's `Look`.
//!
//! The kit drew the `left` fight (and, cached, the `right` one) with the fighter's real look and the kit's base
//! `Look` for the other side; here the same fight is drawn with the fighter's look (`fighters::entry(name).look`,
//! never the registry) and [`DefaultLook`]. Every frame is compared by its bytes' SHA-256, the look hooks called in
//! it (both looks spied as `flute_look.rs` does), its draw log and its hull calls (vertices exactly, every input
//! point within 1e-12 px); a hull fallback stops the task (ruling M3-R4).

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::refs::reference;
use bf_core::event::Event;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_core::show::Show;
use bf_picture::camera::{Cam, V3};
use bf_picture::hull::{self, HullCall};
use bf_picture::img::{Img, RgbF};
use bf_picture::lighting::Lights;
use bf_picture::look::{BallSprite, CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{Picture, PictureCtx};
use bf_picture::sprite::{Mesh, render_sprite};
use bf_picture::title::layout::Layout;
use bf_picture::title::{CardLook, TitleCard};
use bf_picture::{Rgb8, title};
use bf_testdata::read_png;
use serde_json::Value;

use crate::fight;
use crate::refs::{
    cache_json, cached_recording, committed_recording, fighter_folder, hexf, index, json, list, num, report, sha,
    sha_lines, text,
};

// ---------------------------------------------------------------- the looks
/// The fighter's look from its own folder (`fighters::entry(name).look`), or a failure saying it is not ported.
pub fn real_look(name: &str) -> Box<dyn Look> {
    let entry = bf_content::fighters::entry(name).unwrap_or_else(|| panic!("{name} has no fighter folder"));
    (entry.look)().unwrap_or_else(|| {
        panic!("{name}'s look() is None: port its look (bf-content/src/fighters/<slug>/look.rs and mod.rs)")
    })
}

/// The fighter's model from its own folder (`MESH`), or a failure.
pub fn mesh_of(name: &str) -> Mesh {
    let entry = bf_content::fighters::entry(name).unwrap_or_else(|| panic!("{name} has no fighter folder"));
    let json = entry
        .mesh
        .unwrap_or_else(|| panic!("{name}'s MESH is None: export its model (data/models/<NAME>.json) and set MESH"));
    Mesh::from_json(json).unwrap_or_else(|e| panic!("{name}'s model: {e}"))
}

thread_local! {
    /// The hook calls of the frame being drawn on this thread (and of the build, on the building thread).
    static HOOKS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn take_hooks() -> Vec<String> {
    HOOKS.with(|h| std::mem::take(&mut *h.borrow_mut()))
}

/// A look that notes every hook of the export's list as `<its side>.<hook>` (with the ball and the phase where a
/// hook is called per ball) and makes the call on the look it wraps: the export's `_spied` classes
/// (`look_flute.HOOKS`). Every other method is passed through.
pub struct Spy {
    k: usize,
    inner: Box<dyn Look>,
}

impl Spy {
    pub fn new(inner: Box<dyn Look>) -> Spy {
        Spy { k: 0, inner }
    }

    fn note(&self, what: &str) {
        let line = format!("{}.{what}", self.k);
        HOOKS.with(|h| h.borrow_mut().push(line));
    }
}

impl Look for Spy {
    fn glass(&self) -> [f64; 3] {
        self.inner.glass()
    }
    fn flare_scale(&self) -> f64 {
        self.inner.flare_scale()
    }
    fn shards(&self) -> Vec<String> {
        self.inner.shards()
    }
    fn spark_big(&self) -> &[&str] {
        self.inner.spark_big()
    }
    fn spark_flash(&self) -> &[&str] {
        self.inner.spark_flash()
    }
    fn flash_hits(&self) -> &[&str] {
        self.inner.flash_hits()
    }
    fn fringe_hits(&self) -> &[&str] {
        self.inner.fringe_hits()
    }
    fn quiet_hits(&self) -> &[&str] {
        self.inner.quiet_hits()
    }
    fn teleports(&self) -> bool {
        self.inner.teleports()
    }
    fn speed_lift(&self) -> bool {
        self.inner.speed_lift()
    }
    fn rope_kinds(&self) -> &[(&str, f64)] {
        self.inner.rope_kinds()
    }
    fn sprite_k(&self) -> f64 {
        self.inner.sprite_k()
    }
    fn block_burst(&self) -> [f64; 3] {
        self.inner.block_burst()
    }
    fn block_spark(&self) -> Spark {
        self.inner.block_spark()
    }
    fn title_pattern(&self, base: &mut RgbF) {
        self.inner.title_pattern(base);
    }
    fn title_pattern_with(&self, base: &mut RgbF, card: &Layout) {
        self.inner.title_pattern_with(base, card);
    }
    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.k = k;
        self.note("setup");
        self.inner.setup(p, k);
    }
    fn setup_as(&mut self, p: &PictureCtx, k: usize, cfg: &FighterCfg) {
        self.k = k;
        self.inner.setup_as(p, k, cfg);
    }
    fn cfg_summary(&self) -> Option<String> {
        self.inner.cfg_summary()
    }
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        self.inner.title_closeup(r)
    }
    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        self.inner.sprite(r, yaw, roll)
    }
    fn sprite_rolls(&self, kk: usize, roll: f64) -> Vec<f64> {
        self.inner.sprite_rolls(kk, roll)
    }
    fn extra_sprites(&self) -> Vec<(usize, f64, f64, f64)> {
        self.inner.extra_sprites()
    }
    fn ball(&self, p: &PictureCtx, kk: usize, st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        self.note(&format!("ball({kk})"));
        self.inner.ball(p, kk, st, t, glass, roll)
    }
    fn ball_sprite<'a>(
        &'a self,
        p: &'a PictureCtx,
        kk: usize,
        st: &BallState,
        t: f64,
        rp: f64,
        yaw: f64,
    ) -> Option<BallSprite<'a>> {
        self.note(&format!("ball_sprite({kk})"));
        self.inner.ball_sprite(p, kk, st, t, rp, yaw)
    }
    fn state(&self, p: &PictureCtx, kk: usize, st: &mut BallState, t: f64, phase: Phase, gl: f64) {
        let name = match phase {
            Phase::Rev => "rev",
            Phase::Fight => "fight",
            Phase::Tail => "tail",
        };
        self.note(&format!("state({kk},{name})"));
        self.inner.state(p, kk, st, t, phase, gl);
    }
    fn hides_hp(&self, p: &PictureCtx, kk: usize, t: f64) -> bool {
        self.note(&format!("hides_hp({kk})"));
        self.inner.hides_hp(p, kk, t)
    }
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        self.note("shake");
        self.inner.shake(p, e)
    }
    fn rope(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        self.note("rope");
        self.inner.rope(p, e)
    }
    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        self.note("spark");
        self.inner.spark(p, e)
    }
    fn hit_glow(&self, p: &PictureCtx, e: &Event) -> Option<(f64, f64)> {
        self.note("hit_glow");
        self.inner.hit_glow(p, e)
    }
    fn fx_setup(&self, p: &PictureCtx, fx: &mut Fx) {
        self.note("fx_setup");
        self.inner.fx_setup(p, fx);
    }
    fn fx_event(&self, p: &PictureCtx, fx: &mut Fx, e: &Event) {
        self.note("fx_event");
        self.inner.fx_event(p, fx, e);
    }
    fn fx_after(&self, p: &PictureCtx, fx: &mut Fx) {
        self.note("fx_after");
        self.inner.fx_after(p, fx);
    }
    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        self.note("ko_fx");
        self.inner.ko_fx(p, fx, pos);
    }
    fn fx_end(&self, p: &PictureCtx, fx: &mut Fx) {
        self.note("fx_end");
        self.inner.fx_end(p, fx);
    }
    fn light(&self, p: &PictureCtx, l: &mut Lights, t: f64) {
        self.note("light");
        self.inner.light(p, l, t);
    }
    fn shadows(&self, p: &PictureCtx, st: &BallState, spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.note("shadows");
        self.inner.shadows(p, st, spot)
    }
    fn shadows_free(&self, p: &PictureCtx, f: &FrameCtx<'_>, spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.note("shadows_free");
        self.inner.shadows_free(p, f, spot)
    }
    fn canvas_first(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        self.inner.canvas_first(p, ic, c);
    }
    fn canvas(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        self.note("canvas");
        self.inner.canvas(p, ic, c);
    }
    fn canvas_wave(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>, w: &Wave) {
        self.note("canvas_wave");
        self.inner.canvas_wave(p, ic, c, w);
    }
    fn canvas_after(&self, p: &PictureCtx, ic: &mut [V3], c: &CanvasCtx<'_>) {
        self.note("canvas_after");
        self.inner.canvas_after(p, ic, c);
    }
    fn canvas_overlay(&self, p: &PictureCtx, f: &FrameCtx<'_>, arr: &mut Img<f32>) {
        self.note("canvas_overlay");
        self.inner.canvas_overlay(p, f, arr);
    }
    fn items_ball(&self, p: &PictureCtx, f: &FrameCtx<'_>, st: &BallState) -> Vec<(f64, Payload)> {
        self.note("items_ball");
        self.inner.items_ball(p, f, st)
    }
    fn items_free(&self, p: &PictureCtx, f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        self.note("items_free");
        self.inner.items_free(p, f)
    }
    fn draw_item(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, payload: &Payload) {
        self.note("draw_item");
        self.inner.draw_item(p, f, fr, payload);
    }
    fn over_ring(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, arr: &mut Img<f32>, depth: &mut Img<f32>) {
        self.note("over_ring");
        self.inner.over_ring(p, f, fr, arr, depth);
    }
    #[allow(clippy::too_many_arguments)]
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
        self.note("draw_shard");
        self.inner.draw_shard(p, arr, cam, kind, pos, rot, size, curl);
    }
    fn shimmer(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64, w: &Wave) {
        self.note("shimmer");
        self.inner.shimmer(p, img, cam, t, w);
    }
    fn shimmer_after(&self, p: &PictureCtx, img: &mut Pic, cam: &Cam, t: f64) {
        self.note("shimmer_after");
        self.inner.shimmer_after(p, img, cam, t);
    }
    fn glows(&self, p: &PictureCtx, f: &FrameCtx<'_>, fr: &mut Frame, drawn: &[DrawnBall]) {
        self.note("glows");
        self.inner.glows(p, f, fr, drawn);
    }
    fn fringe(&self, p: &PictureCtx, e: &Event) -> bool {
        self.note("fringe");
        self.inner.fringe(p, e)
    }
    fn fringe_size(&self, p: &PictureCtx, e: &Event) -> (f64, f64) {
        self.note("fringe_size");
        self.inner.fringe_size(p, e)
    }
    fn after_grade(&self, p: &PictureCtx, img: &mut Pic, f: &FrameCtx<'_>, drawn: &[DrawnBall]) {
        self.note("after_grade");
        self.inner.after_grade(p, img, f, drawn);
    }
    fn focus_points(&self, p: &PictureCtx, t: f64, st: &BallState) -> Vec<(V3, String)> {
        self.inner.focus_points(p, t, st)
    }
}

// ---------------------------------------------------------------- the fights
/// The picture of `rec` under `ff` with the fighter's real look and the default look for the other side, both
/// spied when `spy`.
pub fn picture(name: &str, rec: Recording, ff: &FightFile, spy: bool) -> Picture {
    let mut ff = ff.clone();
    ff.seed = Some(rec.seed);
    let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("{name}: the show of the fight: {e}"));
    let names = show.names.clone();
    let cfg = [fight::fighter(&names[0]), fight::fighter(&names[1])];
    let meshes = [mesh_of(&names[0]), mesh_of(&names[1])];
    let make = |n: &str| -> Box<dyn Look> {
        let look: Box<dyn Look> = if n == name {
            real_look(name)
        } else {
            Box::new(DefaultLook)
        };
        if spy { Box::new(Spy::new(look)) } else { look }
    };
    let looks = [make(&names[0]), make(&names[1])];
    Picture::new(Arc::new(rec), &ff, show, cfg, meshes, looks).unwrap_or_else(|e| panic!("{name}: the picture: {e}"))
}

fn workers() -> usize {
    std::thread::available_parallelism().map_or(3, |n| n.get()).min(3)
}

/// Runs `job` on every item on [`workers`] threads; returns what it gave, in no order.
pub fn on_threads<I: Sync, T: Send>(items: &[I], job: impl Fn(&I) -> Option<T> + Sync) -> Vec<T> {
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        let jobs: Vec<_> = (0..workers())
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    while let Some(item) = items.get(next.fetch_add(1, Ordering::Relaxed)) {
                        out.extend(job(item));
                    }
                    out
                })
            })
            .collect();
        jobs.into_iter()
            .flat_map(|j| j.join().expect("a frame panicked"))
            .collect()
    })
}

/// The hull calls of a frame against the kit's compact log (`[points, vertices, sum of x, sum of y]`): the same
/// count, the same points count and vertices in scipy's order, the sums within 1e-12 px a point.
pub fn hull_differs(calls: &[HullCall], want: &Value) -> Option<String> {
    let want = list(want);
    if calls.len() != want.len() {
        return Some(format!("{} hull calls, the kit {}", calls.len(), want.len()));
    }
    for (j, ((pts, h), w)) in calls.iter().zip(want).enumerate() {
        let n = index(&w[0]);
        let vertices: Vec<u64> = h.vertices.iter().map(|&v| v as u64).collect();
        let kit: Vec<u64> = list(&w[1]).iter().map(|v| v.as_u64().expect("a vertex")).collect();
        if pts.len() != n || vertices != kit {
            return Some(format!(
                "hull call {j}: {} points, vertices {vertices:?}; the kit {n} points, {kit:?}",
                pts.len()
            ));
        }
        let (sx, sy) = pts.iter().fold((0.0, 0.0), |(a, b), &(x, y)| (a + x, b + y));
        let bar = 1e-12 * n as f64 + 1e-12;
        if (sx - hexf(&w[2])).abs() > bar || (sy - hexf(&w[3])).abs() > bar {
            return Some(format!("hull call {j}: its points are not the kit's (sums {sx}, {sy})"));
        }
    }
    None
}

/// Every frame of `rows` (look.json's frame rows) drawn by `pic` (spied) against the kit: frame bytes, hooks, draw
/// log, hull calls. A hull fallback stops the task.
fn frames_against(name: &str, folder: &str, pic: &Picture, rows: &[Value], what: &str) {
    assert!(!rows.is_empty(), "{folder}: no frames in {what}");
    let fallbacks = AtomicUsize::new(0);
    let failures = on_threads(rows, |row| {
        let f = index(&row[0]);
        take_hooks();
        let ((img, log, rep), calls) = hull::logged(|| pic.frame_reported(f));
        let hooks = take_hooks();
        if rep.hull_fallbacks > 0 || calls.iter().any(|c| c.1.fallback.is_some()) {
            fallbacks.fetch_add(1, Ordering::Relaxed);
        }
        let boxes: Vec<String> = log
            .boxes
            .iter()
            .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
            .collect();
        let mut wrong = Vec::new();
        if sha(&img.data) != text(&row[1]) {
            wrong.push("its bytes".to_string());
        }
        if (hooks.len(), sha_lines(&hooks)) != (index(&row[2]), text(&row[3]).to_string()) {
            wrong.push(format!(
                "the hook calls ({}: {:?})",
                hooks.len(),
                &hooks[..hooks.len().min(12)]
            ));
        }
        if (boxes.len(), sha_lines(&boxes)) != (index(&row[4]), text(&row[5]).to_string()) {
            wrong.push("the draw log".to_string());
        }
        if let Some(h) = hull_differs(&calls, &row[6]) {
            wrong.push(h);
        }
        (!wrong.is_empty()).then(|| {
            (
                format!("frame {f}"),
                format!("{} ({})", wrong.join("; "), text(&row[7])),
            )
        })
    });
    assert_eq!(
        fallbacks.load(Ordering::Relaxed),
        0,
        "{name} ({what}): a hull fell back to the plain scan (a Qhull merge). Ruling M3-R4: this STOPS the task; report \
         it to the controller, who opens the task that ports Qhull's 2-D merge"
    );
    report(folder, "look", &format!("frames of {what}"), failures, rows.len());
}

/// The hook calls made while the picture is built are the kit's.
fn build_hooks_against(name: &str, build: &[String], doc: &Value) {
    let build: Vec<String> = build.iter().filter(|c| !c.contains(".rope")).cloned().collect();
    let head: Vec<&str> = list(&doc["hooks_build_head"]).iter().map(text).collect();
    assert_eq!(
        build[..head.len().min(build.len())],
        head[..head.len().min(build.len())],
        "{name}: the first hook calls of the build"
    );
    assert_eq!(
        (build.len(), sha_lines(&build)),
        (index(&doc["hooks_build"][0]), text(&doc["hooks_build"][1]).to_string()),
        "{name}: the hook calls while the picture is built"
    );
}

fn look_doc(name: &str) -> Value {
    let d = json(&fighter_folder(name), "look.json");
    assert_eq!(text(&d["fighter"]), name);
    d
}

/// The committed `left` fight's frames with the real look and the default look for the other side: every frame's
/// bytes, hook calls, draw log and hull calls are the kit's; the build's hook calls too. Always runs.
pub fn look_frames(name: &str) {
    let folder = fighter_folder(name);
    let doc = look_doc(name);
    let fd = fight::doc(name);
    let left = &doc["left"];
    let rec = committed_recording(text(&left["file"]), Some(text(&left["recording"]["hash"])));
    take_hooks();
    let pic = picture(name, rec, &fight::fight_file(&fd, false), true);
    build_hooks_against(name, &take_hooks(), left);
    let rows = list(&left["frames"]);
    let need = fd["spec"]["frames"]["left"].as_u64().unwrap_or(64) as usize;
    assert!(
        rows.len() >= need,
        "{folder}: {} frames, the spec asks for {need}",
        rows.len()
    );
    assert_eq!(index(&left["k"]), 0);
    frames_against(name, &folder, &pic, rows, "the left fight");
}

/// The same for the cached `right` fight (the fighter on side 1, losing: its shards). Ignored: reads the cache.
pub fn look_frames_cached(name: &str) {
    let folder = fighter_folder(name);
    let doc = look_doc(name);
    let fd = fight::doc(name);
    let right = cache_json(text(&doc["right"]["cache"]));
    let rec = cached_recording(text(&fd["right"]["cache"]));
    assert_eq!(rec.hash(), text(&right["recording"]["hash"]));
    take_hooks();
    let pic = picture(name, rec, &fight::fight_file(&fd, true), true);
    build_hooks_against(name, &take_hooks(), &right);
    assert_eq!(index(&right["k"]), 1);
    let rows = list(&right["frames"]);
    assert_eq!(rows.len(), index(&doc["right"]["frames"]));
    frames_against(name, &folder, &pic, rows, "the right fight");
}

/// Every hull call's input points of both fights (the cache's full log) within 1e-12 px of the kit's. Ignored.
pub fn hull_points(name: &str) {
    let fd = fight::doc(name);
    let doc = look_doc(name);
    for (which, cache, rec, swapped) in [
        (
            "left",
            text(&doc["hull_cache"]).to_string(),
            {
                let l = &doc["left"];
                committed_recording(text(&l["file"]), None)
            },
            false,
        ),
        (
            "right",
            text(&doc["right"]["hull_cache"]).to_string(),
            cached_recording(text(&fd["right"]["cache"])),
            true,
        ),
    ] {
        let kit = cache_json(&cache);
        let pic = picture(name, rec, &fight::fight_file(&fd, swapped), false);
        let (mut calls_n, mut worst) = (0, 0.0f64);
        for row in list(&kit["frames"]) {
            let f = index(&row["frame"]);
            let (_, calls) = hull::logged(|| pic.frame_reported(f));
            let want = list(&row["calls"]);
            assert_eq!(calls.len(), want.len(), "{name} {which} frame {f}: hull calls");
            for ((pts, h), w) in calls.iter().zip(want) {
                let kp = list(&w["points"]);
                assert_eq!(pts.len(), kp.len(), "{name} {which} frame {f}");
                for (&(x, y), k) in pts.iter().zip(kp) {
                    let d = (x - hexf(&k[0])).abs().max((y - hexf(&k[1])).abs());
                    assert!(
                        d <= 1e-12,
                        "{name} {which} frame {f}: a hull point {d:e} px from the kit's"
                    );
                    worst = worst.max(d);
                }
                let v: Vec<u64> = h.vertices.iter().map(|&v| v as u64).collect();
                assert_eq!(serde_json::json!(v), w["vertices"], "{name} {which} frame {f}");
                calls_n += 1;
            }
        }
        println!("{name} {which}: {calls_n} hull calls, every point within {worst:e} px of the kit's");
    }
}

// ---------------------------------------------------------------- the title card
/// Milestone 1's fixed backdrop of the card hashes.
pub fn noise() -> Rgb8 {
    let (w, h, c, data) = read_png(&reference("frames/flute_vs_trombone/title/noise.png"));
    Img::from_vec(w, h, c, data)
}

/// The card of `show` with the fighter's real look and the default look for the other side (no picture: the
/// card's hooks need nothing of a fight).
pub fn card_of(name: &str, show: &Show, series: Option<&str>) -> TitleCard {
    let cfg = [fight::fighter(&show.names[0]), fight::fighter(&show.names[1])];
    let meshes = [mesh_of(&show.names[0]), mesh_of(&show.names[1])];
    let looks: [Box<dyn Look>; 2] = [0, 1].map(|k| -> Box<dyn Look> {
        if show.names[k] == name {
            real_look(name)
        } else {
            Box::new(DefaultLook)
        }
    });
    let view = |c: &FighterCfg| {
        let v = c.raw["view"].as_array().expect("the roster entry has a view");
        (v[0].as_f64().expect("yaw"), v[2].as_f64().expect("roll"))
    };
    TitleCard::new(
        show,
        [cfg[0].color.map(f64::from), cfg[1].color.map(f64::from)],
        [cfg[0].dark.map(f64::from), cfg[1].dark.map(f64::from)],
        [0, 1].map(|k| {
            let (yaw, roll) = view(cfg[k]);
            CardLook {
                look: &*looks[k],
                mesh: &meshes[k],
                yaw,
                roll,
            }
        }),
        series,
    )
    .unwrap_or_else(|e| panic!("{name}: the card: {e}"))
}

/// The card's frames drawn on the noise backdrop: [frame, SHA-256] rows of the kit against `card`.
pub fn card_rows_against(card: &TitleCard, fps: f64, rows: &[Value], what: &str, failures: &mut Vec<(String, String)>) {
    let backdrop = noise();
    for r in rows {
        let f = index(&r[0]);
        let mut img = backdrop.clone();
        card.draw(&mut img, f as f64 / fps);
        if sha(&img.data) != text(&r[1]) {
            failures.push((format!("card {what} frame {f}"), "its bytes".into()));
        }
    }
}

/// The VS card with the fighter on either side at 16 times, its close-up, its panel pattern on the fixed base and
/// its constants (glass, flare scale, shatter parts) are the kit's. Always runs.
pub fn title(name: &str) {
    let folder = fighter_folder(name);
    let doc = look_doc(name);
    let fd = fight::doc(name);
    let t = &doc["title"];
    let look = real_look(name);
    let glass: Vec<f64> = list(&t["glass"]).iter().map(num).collect();
    assert_eq!(look.glass()[..], glass[..], "{name}: the glass");
    assert_eq!(
        look.flare_scale().to_bits(),
        hexf(&t["flare_scale"]).to_bits(),
        "{name}: the flare scale"
    );
    let shards: Vec<&str> = list(&t["shards"]).iter().map(text).collect();
    assert_eq!(look.shards(), shards, "{name}: the shatter parts");
    // the close-up
    let me = fight::fighter(name);
    let c = &t["closeup"];
    let r = index(&c["radius"]) as u32;
    let close = if text(&c["drawn_by"]) == "closeup" {
        look.title_closeup(r)
            .unwrap_or_else(|| panic!("{name}: the kit draws its own close-up; title_closeup is None"))
    } else {
        assert!(
            look.title_closeup(r).is_none(),
            "{name}: the kit draws the model's close-up, the look has its own"
        );
        let v = me.raw["view"].as_array().expect("view");
        render_sprite(&mesh_of(name), r, v[0].as_f64().unwrap(), v[2].as_f64().unwrap())
    };
    let mut failures = Vec::new();
    if (close.w, sha(&close.data)) != (index(&c["size"]), text(&c["sha256"]).to_string()) {
        failures.push(("closeup".to_string(), format!("{} px", close.w)));
    }
    // the pattern on base[y, x, c] = 40.0 + x * 0.5 + y * 0.25 + 20.0 * c
    let p = &t["pattern"];
    let mut base: RgbF = Img::new(360, 640, 3);
    for y in 0..640 {
        for x in 0..360 {
            for (ch, v) in base.px_mut(x, y).iter_mut().enumerate() {
                *v = 40.0 + x as f64 * 0.5 + y as f64 * 0.25 + 20.0 * ch as f64;
            }
        }
    }
    let layout = Layout::plain(
        fd["config"]["fighters"][0].as_str().unwrap_or(name),
        fd["config"]["fighters"][1].as_str().unwrap_or(name),
    );
    match title::patterns::builtin(name).unwrap_or_else(|e| panic!("{name}: {e}")) {
        Some(pattern) => pattern(&mut base, &layout),
        None => look.title_pattern_with(&mut base, &layout),
    }
    for s in list(&p["samples"]) {
        let (x, y) = (index(&s[0]), index(&s[1]));
        for (ch, (&g, w)) in base.px(x, y).iter().zip(list(&s[2])).enumerate() {
            if g.to_bits() != hexf(w).to_bits() {
                failures.push((
                    format!("pattern ({x}, {y}) channel {ch}"),
                    format!("{g:e}, the kit {:e}", hexf(w)),
                ));
            }
        }
    }
    let bytes: Vec<u8> = base.data.iter().flat_map(|v| v.to_le_bytes()).collect();
    if sha(&bytes) != text(&p["sha256"]) {
        failures.push(("pattern".to_string(), "the whole pattern".into()));
    }
    // the cards, with the fighter on side 0 and on side 1
    let left = &doc["left"];
    let rec = committed_recording(text(&left["file"]), None);
    let mut ff = fight::fight_file(&fd, false);
    ff.seed = Some(rec.seed);
    let show = Show::new(&rec, &ff).expect("the show");
    let fps = f64::from(show.grid.fps);
    let cards = &doc["cards"];
    card_rows_against(
        &card_of(name, &show, ff.series_tag.as_deref()),
        fps,
        list(&cards["left"]),
        "left",
        &mut failures,
    );
    let mut swapped = show.clone();
    swapped.names.reverse();
    card_rows_against(
        &card_of(name, &swapped, ff.series_tag.as_deref()),
        fps,
        list(&cards["right"]),
        "right",
        &mut failures,
    );
    let n = 2 + list(&cards["left"]).len() + list(&cards["right"]).len();
    assert!(
        list(&cards["left"]).len() >= 16 && list(&cards["right"]).len() >= 16,
        "{folder}: 16 card frames a side"
    );
    report(&folder, "title", "card frames and title hooks", failures, n);
}

// ---------------------------------------------------------------- the sprites
/// Every sprite of the kit's sprite cache (ruling M3-R18): the case list rebuilt from look.json's facts in the
/// export's order (`sprites.build`: picture, episode, title, model), each drawn by the look (or its model) and
/// hashed; the digest of the hashes is the kit's. Always runs (the cache's per-case hashes say which differs).
pub fn sprites(name: &str) {
    let folder = fighter_folder(name);
    let s = look_doc(name)["sprites"].clone();
    let look = real_look(name);
    let mesh = mesh_of(name);
    let radii: Vec<u32> = list(&s["radii"]).iter().map(|r| index(r) as u32).collect();
    let rolls: Vec<f64> = list(&s["rolls"]).iter().map(num).collect();
    let own = text(&s["drawn_by"]) == "look";
    let ball = |r: u32, yaw: f64, roll: f64| -> Img<u8> {
        look.sprite(r, yaw, roll)
            .unwrap_or_else(|| render_sprite(&mesh, r, yaw, roll))
    };
    let mut drawn: Vec<(String, Img<u8>)> = Vec::new();
    let mut have = std::collections::BTreeSet::new();
    for &roll in &rolls {
        for &r in &radii {
            for yaw in (0..360).step_by(6) {
                drawn.push((
                    format!("picture r {r} yaw {yaw} roll {roll}"),
                    ball(r, f64::from(yaw), roll),
                ));
                have.insert((r, yaw as i64, roll.to_bits()));
            }
        }
    }
    for key in list(&s["episode_keys"]) {
        let (r, yk, roll) = (index(&key[0]) as u32, key[1].as_i64().expect("a yaw"), num(&key[2]));
        if have.insert((r, yk, roll.to_bits())) {
            drawn.push((format!("episode r {r} yaw {yk} roll {roll}"), ball(r, yk as f64, roll)));
        }
    }
    let tc = &s["title"];
    let r = index(&tc["radius"]) as u32;
    let close = if text(&tc["drawn_by"]) == "closeup" {
        look.title_closeup(r).expect("the look's close-up")
    } else {
        render_sprite(&mesh, r, num(&tc["yaw"]), num(&tc["roll"]))
    };
    drawn.push(("title".into(), close));
    if own {
        for &roll in &rolls {
            for &r in &radii {
                for yaw in (0..360).step_by(6) {
                    drawn.push((
                        format!("model r {r} yaw {yaw} roll {roll}"),
                        render_sprite(&mesh, r, f64::from(yaw), roll),
                    ));
                }
            }
        }
    }
    let shas: Vec<String> = drawn.iter().map(|(_, img)| sha(&img.data)).collect();
    assert_eq!(
        drawn.len(),
        index(&s["count"]),
        "{folder}: the number of sprites (the case list)"
    );
    let digest = sha(shas.join("\n").as_bytes());
    if digest != text(&s["digest"]) {
        // which: the cache's sidecar has each case's own hash
        let side = cache_json(&text(&s["cache"]).replace(".npz", ".json"));
        let npz = bf_testdata::Npz::open(&bf_testdata::cache_file(text(&s["cache"])));
        let wrong: Vec<String> = list(&side["cases"])
            .iter()
            .zip(&drawn)
            .zip(&shas)
            .filter(|((case, _), got)| sha(npz.get(&format!("{}__out", text(&case["id"]))).u8()) != **got)
            .map(|((_, (what, _)), _)| what.clone())
            .take(12)
            .collect();
        panic!("{folder}: sprites that are not the kit's (the first): {wrong:?}");
    }
    println!("{name}: {} sprites equal the kit's", drawn.len());
}

// ---------------------------------------------------------------- setup_as
/// A look whose kit class reads `self.cfg` takes its roster numbers from `setup_as`: set up for its side with the
/// other fighter's numbers it differs (`cfg_summary`) from the look set up with its own, which equals `setup`.
/// (EUPHONIUM's copies draw a look with the copier's side and the copied fighter's numbers.)
pub fn setup_as(name: &str) {
    let doc = look_doc(name);
    let fd = fight::doc(name);
    let reads = doc["reads_cfg"].as_bool().expect("look.json reads_cfg");
    let rec = committed_recording(text(&doc["left"]["file"]), None);
    let pic = picture(name, rec, &fight::fight_file(&fd, false), false);
    let p = pic.ctx();
    let (mut own, mut as_own, mut other) = (real_look(name), real_look(name), real_look(name));
    own.setup(p, 0);
    as_own.setup_as(p, 0, &p.cfg[0]);
    other.setup_as(p, 0, &p.cfg[1]);
    assert_eq!(
        own.cfg_summary(),
        as_own.cfg_summary(),
        "{name}: setup(p, k) is not setup_as(p, k, &p.cfg[k])"
    );
    if !reads {
        println!("{name}: its kit look reads no roster numbers (self.cfg)");
        return;
    }
    let mine = own.cfg_summary().unwrap_or_else(|| {
        panic!(
            "{name}: the kit's look reads its roster numbers (self.cfg), so its Look must return them from \
             cfg_summary() (the fields setup_as takes from `cfg`)"
        )
    });
    let theirs = other.cfg_summary().expect("a summary");
    assert_ne!(
        mine, theirs,
        "{name}: set up with {}'s numbers the look takes the same numbers as with its own: it reads p.cfg[k] (or \
         nothing) instead of `cfg` in setup_as",
        p.cfg[1].name
    );
    println!(
        "{name}: setup_as takes its numbers from cfg: own {mine}; {}'s {theirs}",
        p.cfg[1].name
    );
}

// ---------------------------------------------------------------- stage dumps (local, for finding a difference)
#[derive(Debug, Default)]
struct Diff {
    differing: usize,
    worst: f64,
    first: Option<(usize, usize, usize, f64, f64)>,
}

fn diff_of<T: Copy + Into<f64>>(got: &[T], want: &[T], same: impl Fn(T, T) -> bool, w: usize, c: usize) -> Diff {
    assert_eq!(got.len(), want.len(), "the buffers differ in size");
    let mut d = Diff::default();
    for (i, (&g, &p)) in got.iter().zip(want).enumerate() {
        if !same(g, p) {
            d.differing += 1;
            let (gf, pf): (f64, f64) = (g.into(), p.into());
            let delta = (gf - pf).abs();
            d.worst = if delta.is_nan() {
                f64::INFINITY
            } else {
                d.worst.max(delta)
            };
            d.first.get_or_insert((i / c % w, i / c / w, i % c, gf, pf));
        }
    }
    d
}

fn diff_rgb(got: &bf_picture::ring::StagePic, want: &bf_testdata::NpArray) -> Diff {
    use bf_picture::ring::StagePic;
    use bf_testdata::NpData;
    let (w, c) = match got {
        StagePic::U8(a) => (a.w, a.c),
        StagePic::F32(a) => (a.w, a.c),
        StagePic::F64(a) => (a.w, a.c),
    };
    match (got, &want.data) {
        (StagePic::U8(a), NpData::U8(p)) => diff_of(&a.data, p, |x, y| x == y, w, c),
        (StagePic::F32(a), NpData::F32(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), w, c),
        (StagePic::F64(a), NpData::F64(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), w, c),
        _ => panic!("Python holds this buffer as {}", want.dtype()),
    }
}

/// Every stage of every frame `python -m tools.studio_export fighter_stages <slug> [left | right] <frame>...` dumped
/// (`studio/reference/.cache/stages_fighter/<slug>/<fight>/`): the drawing stages and depth buffers are the kit's
/// to the bit, the float stages from the lens to the post effects within 1e-9 of a level (`ring_stages.rs`'s
/// bars). The tool for finding the first stage of a frame that differs. Ignored; fails when nothing is dumped.
pub fn stages(name: &str) {
    use bf_picture::ring::Stage;
    let fd = fight::doc(name);
    let dir = bf_testdata::reference_dir()
        .join(".cache/stages_fighter")
        .join(crate::refs::slug(name));
    let mut failures = Vec::new();
    let mut frames = 0;
    for (fight, swapped) in [("left", false), ("right", true)] {
        let Ok(entries) = std::fs::read_dir(dir.join(fight)) else {
            continue;
        };
        let rec = if swapped {
            cached_recording(text(&fd["right"]["cache"]))
        } else {
            committed_recording(text(&fd["left"]["file"]), None)
        };
        let pic = picture(name, rec, &fight::fight_file(&fd, swapped), false);
        for path in entries.flatten().map(|e| e.path()) {
            let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let Some(f) = file
                .strip_prefix('f')
                .and_then(|n| n.strip_suffix(".npz"))
                .and_then(|n| n.parse::<usize>().ok())
            else {
                continue;
            };
            frames += 1;
            let z = bf_testdata::Npz::open(&path);
            for stage in Stage::ALL {
                let got = pic.stage_buffers(f, stage);
                let sname = stage.name();
                let key = if stage == Stage::Coc {
                    format!("{sname}__map")
                } else {
                    format!("{sname}__rgb")
                };
                let d = diff_rgb(&got.rgb, z.get(&key));
                let float = (Stage::Coc..=Stage::PostFx).contains(&stage);
                if if float { d.worst > 1e-9 } else { d.differing != 0 } {
                    failures.push(format!(
                        "{fight} frame {f} {sname}: {} differ, worst {:e}, first {:?}",
                        d.differing, d.worst, d.first
                    ));
                }
                let dk = format!("{sname}__depth");
                if z.has(&dk) {
                    let depth = got.depth.as_ref().expect("a depth buffer");
                    let d = diff_of(
                        &depth.data,
                        z.get(&dk).f32(),
                        |x, y| x.to_bits() == y.to_bits(),
                        depth.w,
                        1,
                    );
                    if d.differing != 0 {
                        failures.push(format!(
                            "{fight} frame {f} {sname} depth: {} differ, first {:?}",
                            d.differing, d.first
                        ));
                    }
                }
            }
        }
    }
    assert!(
        frames > 0,
        "no stage dumps in {}: python -m tools.studio_export fighter_stages {} [left | right] <frame>...",
        dir.display(),
        crate::refs::slug(name)
    );
    assert!(
        failures.is_empty(),
        "{name}: stages that are not the kit's:\n{}",
        failures.join("\n")
    );
    println!("{name}: every stage of {frames} dumped frames is the kit's");
}

// ---------------------------------------------------------------- every scenario draws
/// Every cached scenario fight of the folder (against each opponent its spec names, EUPHONIUM's copy sweep among
/// them, ruling M3-R3) drawn with both fighters' real looks (`build_picture`), every `step`-th frame and the K.O.:
/// no panic, no hull fallback (M3-R4), no hull scipy would refuse. Ignored: reads the cache.
pub fn scenarios_draw(name: &str, step: usize) {
    let fd = fight::doc(name);
    let mut fights = 0;
    for s in list(&fd["scenarios"]) {
        let fighters: Vec<String> = list(&s["fighters"]).iter().map(|f| text(f).to_string()).collect();
        for file in list(&s["cache"]) {
            let rec = cached_recording(text(file));
            let mut ff = fight::fight_file_of(&fd, &fighters);
            ff.seed = Some(rec.seed);
            let ko = rec.t_ko.map(|t| (t * 30.0) as usize);
            let pic = match bf_content::build_picture(Arc::new(rec), &ff) {
                Ok(p) => p,
                Err(e) if ko.is_none() => {
                    println!("{name}: {} has no K.O., no video: {e}", text(file));
                    continue;
                }
                Err(e) => panic!("{name}: {}: the picture: {e}", text(file)),
            };
            let frames: Vec<usize> = (0..pic.nf()).step_by(step).chain(ko).collect();
            let bad = on_threads(&frames, |&f| {
                let (_, _, rep) = pic.frame_reported(f);
                (rep.hull_fallbacks + rep.hull_errors > 0).then_some(f)
            });
            assert!(
                bad.is_empty(),
                "{name}: {}: hull fallbacks or refusals on frames {bad:?} (ruling M3-R4)",
                text(file)
            );
            fights += 1;
        }
    }
    assert!(fights > 0, "{name}: no scenario fight drew");
    println!("{name}: {fights} scenario fights draw every {step}th frame without a panic or a hull fallback");
}
