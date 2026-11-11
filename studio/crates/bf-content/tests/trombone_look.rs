//! The TROMBONE's look, proven on its own against the Python kit.
//!
//! Both fighters are in every frame, so no frame of the episode can prove one look alone. The export
//! `python -m tools.studio_export look_trombone` drew two fights with the kit's renderer, the TROMBONE with its
//! own look (`kit/fighters/trombone.py`) and the FLUTE with the kit's base-class look, and wrote what it saw to
//! `studio/reference/picture_looks/trombone_frames.json`: `ep392`, the episode's own fight (the TROMBONE wins:
//! its solo on the slide), and `seed3` (it loses: its shards and its K.O. spray; PEDAL TONE's slam lands). Here
//! the same frames are drawn with [`TromboneLook`] and [`DefaultLook`] and must be the same bytes, with the same
//! hook calls in the same order and the same draw log.
//!
//! `every_stage_matches_python` compares stage by stage against local dumps
//! (`python -m tools.studio_export look_trombone_stages ep392 1319 ...`, git-ignored) and is skipped with a note
//! when there are none: it is the tool for finding where a frame first differs.

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use bf_content::fighters::trombone::TromboneLook;
use bf_core::event::Event;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_core::show::Show;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::{Img, RgbF};
use bf_picture::lighting::Lights;
use bf_picture::look::{BallSprite, CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use bf_picture::raster::Frame;
use bf_picture::ring::state::{BallState, Wave};
use bf_picture::ring::{Picture, PictureCtx, Stage, StagePic};
use bf_picture::sprite::Mesh;
use bf_testdata::{NpArray, NpData, Npz, parse_hex_f64, reference, reference_dir, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------- the reference file
fn python() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| {
        let path = reference("picture_looks/trombone_frames.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    })
}

fn h(v: &Value) -> f64 {
    parse_hex_f64(v.as_str().unwrap_or_else(|| panic!("{v} is not a hex float")))
}

fn list(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("{v} is not a list"))
}

fn index(v: &Value) -> usize {
    v.as_u64().unwrap_or_else(|| panic!("{v} is not a whole number")) as usize
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_else(|| panic!("{v} is not text"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha(lines: &[String]) -> String {
    hex(&Sha256::digest(lines.join("\n").as_bytes()))
}

// ---------------------------------------------------------------- the hook log
thread_local! {
    /// The hook calls made on this thread (a frame is drawn on the thread that asks for it).
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn take() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// A look with every hook call noted as `<its side>.<hook>` (with the ball and the phase where a hook is called
/// per ball), as the export's `_spy` notes them. Everything is passed on to the look inside.
struct Spy<L: Look> {
    k: usize,
    inner: L,
}

impl<L: Look> Spy<L> {
    fn note(&self, what: &str) {
        LOG.with(|l| l.borrow_mut().push(format!("{}.{what}", self.k)));
    }
}

impl<L: Look> Look for Spy<L> {
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
    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.k = k;
        self.note("setup");
        self.inner.setup(p, k);
    }
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        self.inner.title_closeup(r)
    }
    fn sprite(&self, r: u32, yaw: f64, roll: f64) -> Option<Img<u8>> {
        self.inner.sprite(r, yaw, roll)
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

// ---------------------------------------------------------------- the two fights
fn mesh(pp: &str) -> Mesh {
    let path = repo_root().join(format!("studio/crates/bf-content/data/models/{pp}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Mesh::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn cfg(name: &str) -> FighterCfg {
    bf_content::roster()
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the roster"))
        .clone()
}

/// One fight of the reference file, drawn here: the TROMBONE with [`TromboneLook`], the FLUTE with
/// [`DefaultLook`], every hook noted.
struct Fight {
    name: &'static str,
    /// The fight's entry in the reference file.
    doc: &'static Value,
    pic: Picture,
    /// The hook calls made while the picture was built (without `rope`: the kit asks twice an event).
    build: Vec<String>,
    /// The TROMBONE's side.
    k: usize,
}

fn build(name: &'static str) -> Fight {
    let doc = list(&python()["fights"])
        .iter()
        .find(|f| f["name"] == name)
        .unwrap_or_else(|| panic!("the reference file has no fight {name}"));
    let rec = Recording::read(&repo_root().join(text(&doc["recording"]["file"]))).expect("the recording reads");
    assert_eq!(
        rec.hash(),
        text(&doc["recording"]["hash"]),
        "{name}: not the recording Python drew"
    );
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let toml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&toml).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the fight has a K.O.");
    let cfgs = [cfg(&show.names[0]), cfg(&show.names[1])];
    let meshes = [mesh(&cfgs[0].pp), mesh(&cfgs[1].pp)];
    let look = |n: &str| -> Box<dyn Look> {
        match n {
            "TROMBONE" => Box::new(Spy {
                k: 0,
                inner: TromboneLook::default(),
            }),
            _ => Box::new(Spy {
                k: 0,
                inner: DefaultLook,
            }),
        }
    };
    let looks = [look(&show.names[0]), look(&show.names[1])];
    let k = index(&doc["side"]);
    assert_eq!(show.names[k], "TROMBONE");
    take();
    let pic = Picture::new(Arc::new(rec), &ff, show, [&cfgs[0], &cfgs[1]], meshes, looks).expect("the picture builds");
    let build = take().into_iter().filter(|c| !c.contains(".rope")).collect();
    Fight {
        name,
        doc,
        pic,
        build,
        k,
    }
}

fn fights() -> [&'static Fight; 2] {
    static EP392: OnceLock<Fight> = OnceLock::new();
    static SEED3: OnceLock<Fight> = OnceLock::new();
    [
        EP392.get_or_init(|| build("ep392")),
        SEED3.get_or_init(|| build("seed3")),
    ]
}

// ---------------------------------------------------------------- the tests
/// The class attributes of `kit/fighters/trombone.py` `Look`, and that it has no hook for the title card.
#[test]
fn the_class_attributes_are_pythons() {
    let look = TromboneLook::default();
    let mut want = vec!["slidebit"; 4];
    want.extend(["tube"; 4]);
    want.extend(["bellbit"; 6]);
    assert_eq!(look.shards(), want);
    assert_eq!(look.spark_flash(), ["blade", "slam"]);
    let base = DefaultLook;
    assert_eq!(look.spark_big(), base.spark_big());
    assert_eq!(look.flash_hits(), base.flash_hits());
    assert_eq!(look.fringe_hits(), base.fringe_hits());
    assert_eq!(look.quiet_hits(), base.quiet_hits());
    assert_eq!(look.rope_kinds(), base.rope_kinds());
    assert_eq!(
        (look.flare_scale(), look.sprite_k(), look.teleports(), look.speed_lift()),
        (1.0, 0.80, false, false)
    );
    assert_eq!(look.block_burst(), base.block_burst());
    assert_eq!(look.block_spark(), base.block_spark());
    // the title card: the base class's glass, no close-up of its own, no panel pattern
    let hooks = &python()["title_hooks"];
    let glass: Vec<f64> = list(&hooks["glass"])
        .iter()
        .map(|v| v.as_f64().expect("a number"))
        .collect();
    assert_eq!(look.glass()[..], glass[..]);
    assert!(hooks["title_closeup"].is_null() && hooks["title_pattern"].is_null());
    assert!(look.title_closeup(44).is_none());
    let mut panel: RgbF = Img::filled(8, 8, &[10.0, 20.0, 30.0]);
    let before = panel.clone();
    look.title_pattern(&mut panel);
    assert_eq!(panel, before);
}

/// `shake(e)`, `spark(e)` and `hit_glow(e)` for every event of the TROMBONE in both fights.
#[test]
fn the_event_hooks_are_pythons() {
    for fight in fights() {
        let p = fight.pic.ctx();
        let look = &p.looks[fight.k];
        let table = fight.doc["events"].as_object().expect("the events' table");
        let (mut n, mut seen) = (0, std::collections::BTreeSet::new());
        for e in &p.rec.events {
            if bf_picture::ring::owner(e) != Some(fight.k) {
                continue;
            }
            let key = if e.kind == "hit" {
                format!("hit:{}", e.s(7))
            } else {
                e.kind.clone()
            };
            let want = table
                .get(&key)
                .unwrap_or_else(|| panic!("{}: no {key} in the table", fight.name));
            let shake = (!want["shake"].is_null()).then(|| h(&want["shake"]));
            assert_eq!(
                look.shake(p, e).map(f64::to_bits),
                shake.map(f64::to_bits),
                "{}: shake of {key}",
                fight.name
            );
            if e.kind == "hit" {
                let s = list(&want["spark"]);
                let col: Vec<f64> = list(&s[0]).iter().map(h).collect();
                let got = look.spark(p, e);
                assert_eq!(
                    (
                        got.0.map(f64::to_bits).to_vec(),
                        got.1,
                        got.2.to_bits(),
                        got.3.to_bits()
                    ),
                    (
                        col.iter().map(|c| c.to_bits()).collect(),
                        index(&s[1]),
                        h(&s[2]).to_bits(),
                        h(&s[3]).to_bits()
                    ),
                    "{}: spark of {key}",
                    fight.name
                );
                let glow = (!want["hit_glow"].is_null()).then(|| (h(&want["hit_glow"][0]), h(&want["hit_glow"][1])));
                assert_eq!(look.hit_glow(p, e), glow, "{}: hit_glow of {key}", fight.name);
            }
            n += 1;
            seen.insert(key);
        }
        take();
        assert_eq!(seen.len(), table.len(), "{}: kinds of events", fight.name);
        println!("{}: {n} events of {} kinds", fight.name, seen.len());
    }
}

/// While the picture is built the hooks are called as Python calls them (the camera shake, then the effects in
/// `_fx_build`'s order, with the `fighter` calls the look makes for each landed lance), and the effects that
/// come out of it are as many: particles, flares, bursts, marks, moments, shakes, and the parts of the shatter.
#[test]
fn the_build_is_pythons() {
    for fight in fights() {
        let (doc, p) = (fight.doc, fight.pic.ctx());
        let head: Vec<&str> = list(&doc["build"]["head"]).iter().map(text).collect();
        assert_eq!(
            fight.build[..head.len()],
            head[..],
            "{}: the first hook calls",
            fight.name
        );
        assert_eq!(
            (fight.build.len(), sha(&fight.build).as_str()),
            (index(&doc["build"]["calls"]), text(&doc["build"]["sha256"])),
            "{}: the hook calls while the picture is built",
            fight.name
        );
        let b = &doc["built"];
        let got = [
            ("particles", p.fx.pa.particles().len()),
            ("flares", p.fx.fl.len()),
            ("bursts", p.fx.bu.len()),
            ("marks", p.fx.mk.len()),
            ("moments", p.fx.mo.len()),
            ("shakes", p.shk.len()),
        ];
        for (what, n) in got {
            assert_eq!(n, index(&b[what]), "{}: {what}", fight.name);
        }
        let parts: Vec<&str> = p.parts.iter().map(|q| q.kind.as_str()).collect();
        let want: Vec<&str> = list(&b["parts"]).iter().map(text).collect();
        assert_eq!(parts, want, "{}: the parts of the shatter", fight.name);
        println!(
            "{}: {} hook calls in the build, {} particles",
            fight.name,
            fight.build.len(),
            got[0].1
        );
    }
}

/// Every frame of the reference file: the bytes of the finished frame, the hook calls of the frame, the draw
/// log, and `hides_hp` for both balls.
#[test]
fn every_frame_is_pythons() {
    let mut total = 0;
    for fight in fights() {
        let (pic, p) = (&fight.pic, fight.pic.ctx());
        // a few frames' hook calls in full, for a readable failure
        for (f, want) in fight.doc["hooks_full"].as_object().expect("the frames written out") {
            take();
            pic.frame(f.parse().expect("a frame number"));
            let want: Vec<&str> = list(want).iter().map(text).collect();
            assert_eq!(take(), want, "{}: the hook calls of frame {f}", fight.name);
        }
        let rows = list(&fight.doc["frames"]);
        let next = AtomicUsize::new(0);
        let mut wrong: Vec<(usize, String)> = std::thread::scope(|s| {
            let jobs: Vec<_> = (0..8)
                .map(|_| {
                    s.spawn(|| {
                        let mut wrong = Vec::new();
                        while let Some(row) = rows.get(next.fetch_add(1, Ordering::Relaxed)) {
                            let f = index(&row[0]);
                            let t = f as f64 / 30.0;
                            take();
                            let (img, log) = pic.frame_logged(f);
                            let calls = take();
                            let boxes: Vec<String> = log
                                .boxes
                                .iter()
                                .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
                                .collect();
                            let hides = [0, 1].map(|kk| usize::from(p.looks[fight.k].hides_hp(p, kk, t)));
                            take();
                            let mut bad = Vec::new();
                            if hex(&Sha256::digest(&img.data)) != text(&row[1]) {
                                bad.push("the picture".to_string());
                            }
                            if (calls.len(), sha(&calls).as_str()) != (index(&row[2]), text(&row[3])) {
                                bad.push(format!("the hook calls ({} for {})", calls.len(), row[2]));
                            }
                            if (boxes.len(), sha(&boxes).as_str()) != (index(&row[4]), text(&row[5])) {
                                bad.push(format!("the draw log ({} for {})", boxes.len(), row[4]));
                            }
                            if hides != [index(&row[6]), index(&row[7])] {
                                bad.push(format!("hides_hp {hides:?}"));
                            }
                            if !bad.is_empty() {
                                wrong.push((f, format!("frame {f} ({}): {}", text(&row[8]), bad.join(", "))));
                            }
                        }
                        wrong
                    })
                })
                .collect();
            jobs.into_iter()
                .flat_map(|j| j.join().expect("a frame panicked"))
                .collect()
        });
        wrong.sort();
        println!(
            "{}: {} of {} frames are Python's (picture, hook calls, draw log, hides_hp)",
            fight.name,
            rows.len() - wrong.len(),
            rows.len()
        );
        let lines: Vec<&str> = wrong.iter().map(|w| w.1.as_str()).collect();
        assert!(
            wrong.is_empty(),
            "{}: {} of {} frames differ:\n{}",
            fight.name,
            wrong.len(),
            rows.len(),
            lines.join("\n")
        );
        total += rows.len();
    }
    assert!(total >= 64 && list(&fights()[0].doc["frames"]).len() >= 64);
}

/// `a` is `b` to within `1e-12` of its size (a value that passes through a sine or an exponential may differ
/// from Python's in its last bits).
fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-12 * b.abs().max(1.0)
}

/// The look's fields of the picture state (`thr_k`, `glow`, `ext`) and its `focus_points` on the frames the
/// reference file lists them for.
#[test]
fn the_state_and_the_focus_points_are_pythons() {
    for fight in fights() {
        let p = fight.pic.ctx();
        let (mut n, mut exact) = (0, 0);
        for row in list(&fight.doc["state"]) {
            let f = index(&row[0]);
            let st = p.fighter(fight.k, f as f64 / 30.0);
            for (what, got, want) in [
                ("thr_k", st.thr_k, h(&row[1])),
                ("glow", st.glow, h(&row[2])),
                ("ext", st.ext, h(&row[3])),
            ] {
                assert!(
                    near(got, want),
                    "{}: frame {f}: {what} is {got:?}, Python's {want:?}",
                    fight.name
                );
                n += 1;
                exact += usize::from(got.to_bits() == want.to_bits());
            }
        }
        for row in list(&fight.doc["focus"]) {
            let f = index(&row[0]);
            let t = f as f64 / 30.0;
            let got = p.looks[fight.k].focus_points(p, t, &p.fighter(fight.k, t));
            let want = list(&row[1]);
            assert_eq!(got.len(), want.len(), "{}: frame {f}: focus points", fight.name);
            for ((w, label), q) in got.iter().zip(want) {
                assert_eq!(label, text(&q[0]), "{}: frame {f}", fight.name);
                for c in 0..3 {
                    let py = h(&q[1][c]);
                    assert!(
                        near(w[c], py),
                        "{}: frame {f}: {label} is {w:?}, Python's {py:?} in {c}",
                        fight.name
                    );
                    n += 1;
                    exact += usize::from(w[c].to_bits() == py.to_bits());
                }
            }
        }
        take();
        assert!(n > 100, "{}: {n} values", fight.name);
        println!("{}: {n} state and focus values, {exact} equal to the bit", fight.name);
    }
}

// ---------------------------------------------------------------- stage by stage (local dumps)
/// `(samples that differ, the largest difference, the first: x, y, channel, got, want)` of two buffers.
fn diff_of<T: Copy + Into<f64>>(
    got: &[T],
    want: &[T],
    same: impl Fn(T, T) -> bool,
    w: usize,
    c: usize,
) -> (usize, f64, String) {
    assert_eq!(got.len(), want.len(), "the buffers differ in size");
    let (mut n, mut worst, mut first) = (0, 0.0f64, String::new());
    for (i, (&g, &p)) in got.iter().zip(want).enumerate() {
        if !same(g, p) {
            let (gf, pf): (f64, f64) = (g.into(), p.into());
            let d = (gf - pf).abs();
            worst = if d.is_nan() { f64::INFINITY } else { worst.max(d) };
            if n == 0 {
                first = format!("x {} y {} c {}: {gf:?} for {pf:?}", i / c % w, i / c / w, i % c);
            }
            n += 1;
        }
    }
    (n, worst, first)
}

fn diff_rgb(got: &StagePic, want: &NpArray) -> (usize, f64, String) {
    match (got, &want.data) {
        (StagePic::U8(a), NpData::U8(p)) => diff_of(&a.data, p, |x, y| x == y, a.w, a.c),
        (StagePic::F32(a), NpData::F32(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), a.w, a.c),
        (StagePic::F64(a), NpData::F64(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), a.w, a.c),
        _ => panic!("the stage's dtype is not Python's {}", want.dtype()),
    }
}

/// Every stage of every locally dumped frame against Python's buffers: the drawing stages to the bit, the float
/// stages between the lens and the cut to bytes to `1e-9` of a level (the lens's FFT). Skipped when there are no
/// dumps.
#[test]
fn every_stage_matches_python() {
    let (mut compared, mut float_worst) = (0, 0.0f64);
    for fight in fights() {
        let dir: PathBuf = reference_dir().join(".cache/stages_look_trombone").join(fight.name);
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "npz"))
            .collect();
        files.sort();
        let mut failures = Vec::new();
        for file in files {
            let stem = file.file_stem().and_then(|s| s.to_str()).expect("a file name");
            let f: usize = stem[1..].parse().expect("f<frame>.npz");
            let z = Npz::open(&file);
            for stage in Stage::ALL {
                let got = fight.pic.stage_buffers(f, stage);
                let name = stage.name();
                let key = if stage == Stage::Coc {
                    format!("{name}__map")
                } else {
                    format!("{name}__rgb")
                };
                let (n, worst, first) = diff_rgb(&got.rgb, z.get(&key));
                let float = (Stage::Coc..=Stage::PostFx).contains(&stage);
                if float {
                    float_worst = float_worst.max(worst);
                }
                if if float { worst > 1e-9 } else { n != 0 } {
                    failures.push(format!(
                        "{} frame {f} {name}: {n} samples differ, worst {worst:e}, first {first}",
                        fight.name
                    ));
                }
                let depth_key = format!("{name}__depth");
                if let (true, Some(depth)) = (z.has(&depth_key), &got.depth) {
                    let (n, worst, first) = diff_of(
                        &depth.data,
                        z.get(&depth_key).f32(),
                        |x, y| x.to_bits() == y.to_bits(),
                        depth.w,
                        1,
                    );
                    if n != 0 {
                        failures.push(format!(
                            "{} frame {f} {name} depth: {n} samples differ, worst {worst:e}, first {first}",
                            fight.name
                        ));
                    }
                }
            }
            compared += 1;
        }
        take();
        assert!(failures.is_empty(), "\n{}", failures.join("\n"));
    }
    if compared == 0 {
        eprintln!(
            "skipped: no stage dumps under studio/reference/.cache/stages_look_trombone \
             (python -m tools.studio_export look_trombone_stages <fight> <frame>...)"
        );
    } else {
        println!(
            "{compared} frames compared stage by stage; the float stages differ by at most {float_worst:e} of a level"
        );
    }
}
