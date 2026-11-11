//! The FLUTE's look against the Python kit: `bf_content::fighters::flute::FluteLook` is the `Look` half of
//! `projects/ballfight/kit/fighters/flute.py`.
//!
//! Both fighters are in every frame of the example episode, so no golden frame shows one look alone. The export
//! `look_flute` (`tools/studio_export/look_flute.py`) therefore draws the episode in Python with the FLUTE's real
//! look and the kit's base-class look for the other fighter, and `studio/reference/picture_looks/flute_look.json`
//! holds what it found, for three fights ([`FIGHTS`]): the episode itself (seed 392, the FLUTE loses: 124
//! frames), the same episode fought with seed 3 (the FLUTE wins, so its two victory baskets show: 65 frames) and
//! the episode with the corners swapped, seed 1 (the FLUTE is side 1, on the right, and wins: 65 frames). For each
//! frame (a spread over the whole video and the frames where each layer of the look is on screen) it holds the
//! SHA-256 of the finished frame, of the look hooks called and of the draw log; for each fight the hook calls
//! made while the renderer is built and the look's own state at a few times; and the look's title-card hooks.
//! The tests here draw the same frames with [`FluteLook`] and [`DefaultLook`] and compare every one.
//!
//! The reference set already held the look's sprites as Python drew them (`sprites/FLUTE.npz`: 503 in-ball
//! sprites and the VS card's close-up) and every frame of the VS card with the FLUTE on it
//! (`frames/flute_vs_trombone/title/hashes.npz`, four timelines); `bf-picture`'s own tests read those through a
//! stand-in for this look. Here the real look is held to them.
//!
//! One test reads local files that are not reference files: `python -m tools.studio_export look_flute_stages
//! [episode | flute_wins | flute_right] <frame>...` writes the buffers of those frames after each stage of the Python renderer
//! (drawn the same way) into `studio/reference/.cache/stages_look_flute/<fight>/`, and
//! `every_stage_matches_python` compares every stage of every frame dumped there. It prints that it was skipped
//! when the folders hold none. It is the tool for finding where a frame first differs.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use bf_content::fighters::flute::{FluteLook, flute_keys, flute_sprite};
use bf_core::event::Event;
use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
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
use bf_picture::title::{CardLook, TitleCard};
use bf_testdata::{
    NpArray, NpData, Npz, Vectors, hex_arg, parse_hex_f64, read_png, reference, reference_dir, repo_root,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------- the reference file
fn python() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| {
        let path = reference("picture_looks/flute_look.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let doc: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(doc["tool"], "look_flute");
        assert_eq!(doc["look"], "FLUTE");
        assert_eq!(doc["other_look"], "kit/fighters/base.py Look");
        let names: Vec<&String> = doc["fights"].as_object().expect("fights").keys().collect();
        let mut want = FIGHTS.map(|f| f.0);
        want.sort_unstable();
        assert_eq!(names, want);
        doc
    })
}

/// The fights of the file: `(name, the recording under studio/reference/fights/flute_vs_trombone, the winner,
/// whether the corners are swapped)`. All are the example episode's fight file: the second is fought with
/// another seed, the third with another seed and the TROMBONE in the left corner.
const FIGHTS: [(&str, &str, &str, bool); 3] = [
    ("episode", "ep392", "TROMBONE", false),
    ("flute_wins", "seed3", "FLUTE", false),
    ("flute_right", "scenarios/swapped_s1", "FLUTE", true),
];

/// The part of the file about one fight.
fn fight_doc(fight: &str) -> &'static Value {
    let doc = &python()["fights"][fight];
    assert!(doc.is_object(), "the file has no fight {fight:?}");
    doc
}

/// A float Python wrote as `float.hex()`.
fn h(v: &Value) -> f64 {
    parse_hex_f64(v.as_str().unwrap_or_else(|| panic!("{v} is not a hex float")))
}

fn hv(v: &Value) -> Vec<f64> {
    list(v).iter().map(h).collect()
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

/// One row of the file's `frames`.
struct Row<'a> {
    frame: usize,
    sha256: &'a str,
    hooks: (usize, &'a str),
    drawn: (usize, &'a str),
    tags: &'a str,
}

fn rows(fight: &str) -> Vec<Row<'static>> {
    let doc = python();
    assert_eq!(text(&doc["frame_columns"][0]), "frame");
    assert_eq!(list(&doc["frame_columns"]).len(), 8);
    let rows: Vec<Row> = list(&fight_doc(fight)["frames"])
        .iter()
        .map(|r| Row {
            frame: index(&r[0]),
            sha256: text(&r[1]),
            hooks: (index(&r[2]), text(&r[3])),
            drawn: (index(&r[4]), text(&r[5])),
            tags: text(&r[6]),
        })
        .collect();
    let need = if fight == "episode" { 64 } else { 32 };
    assert!(
        rows.len() >= need,
        "{fight}: the file holds {} frames, {need} are asked for",
        rows.len()
    );
    rows
}

// ---------------------------------------------------------------- the episode
fn fighter(name: &str) -> FighterCfg {
    bf_content::roster()
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the roster"))
        .clone()
}

fn mesh(pp: &str) -> Mesh {
    let path = repo_root().join(format!("studio/crates/bf-content/data/models/{pp}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Mesh::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn fight_file() -> FightFile {
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    FightFile::parse(&text).expect("the fight file parses")
}

/// The looks of this test: the real look for the FLUTE, the base-class look for anyone else.
fn one_real_look(name: &str) -> Box<dyn Look> {
    if name == "FLUTE" {
        Box::new(FluteLook::new())
    } else {
        Box::new(DefaultLook)
    }
}

/// The picture of a fight of [`FIGHTS`] with the looks `make` gives: the example episode's fight file and the
/// fight's recording, which must be the one the export drew (its content hash is in the file).
fn build(fight: &str, make: &dyn Fn(usize, &str) -> Box<dyn Look>) -> Picture {
    let &(_, file, winner, swapped) = FIGHTS.iter().find(|f| f.0 == fight).expect("a fight of FIGHTS");
    let rel = format!("fights/flute_vs_trombone/{file}.bfrec");
    let rec = Recording::read(&reference(&rel)).expect("the recording reads");
    let note = &fight_doc(fight)["recording"];
    assert_eq!(text(&note["file"]), format!("studio/reference/{rel}"));
    assert_eq!(
        rec.hash(),
        text(&note["hash"]),
        "{rel} is not the fight the export drew"
    );
    let mut ff = fight_file();
    if swapped {
        ff.fighters.swap(0, 1);
    }
    let show = Show::new(&rec, &ff).expect("the fight has a K.O.");
    let names: Vec<&str> = list(&fight_doc(fight)["fighters"]).iter().map(text).collect();
    assert_eq!(show.names[..], names[..], "{fight}");
    assert_eq!(names[index(&fight_doc(fight)["k"])], "FLUTE");
    assert_eq!(show.names[show.win], winner, "{fight}");
    assert_eq!(text(&fight_doc(fight)["winner"]), winner);
    let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
    let meshes = [mesh(&cfg[0].pp), mesh(&cfg[1].pp)];
    let looks = [make(0, &show.names[0]), make(1, &show.names[1])];
    Picture::new(Arc::new(rec), &ff, show, [&cfg[0], &cfg[1]], meshes, looks).expect("the picture builds")
}

/// A fight of [`FIGHTS`] with the FLUTE's real look and the default look for the TROMBONE, built once.
fn picture(fight: &str) -> &'static Picture {
    static PICTURES: [OnceLock<Picture>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = FIGHTS.iter().position(|f| f.0 == fight).expect("a fight of FIGHTS");
    PICTURES[i].get_or_init(|| build(fight, &|_, name| one_real_look(name)))
}

/// Runs `job` on every row, on eight threads, and returns what it gave for the rows it failed.
fn on_every_row<T: Send>(rows: &[Row<'_>], job: impl Fn(&Row<'_>) -> Option<T> + Sync) -> Vec<T> {
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        let jobs: Vec<_> = (0..8)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    while let Some(row) = rows.get(next.fetch_add(1, Ordering::Relaxed)) {
                        out.extend(job(row));
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

// ---------------------------------------------------------------- the frames
/// Every frame of the file is the bytes the Python kit drew with the FLUTE's real look: the title card, the
/// count-in, baskets landing and taken back, cobras rising, striking and biting, a swat, FLUTTER TONGUE, SNAKE
/// DANCE, PICCOLO's ring and its glass cracks, the K.O., the silver shards, the reassembly and the glide home.
#[test]
fn the_frames_are_pythons_with_the_flutes_look() {
    for (fight, ..) in FIGHTS {
        frames_are_pythons(fight);
    }
    // each layer of the look is in the frames compared: every one in the episode but the victory baskets (the
    // FLUTE loses it), and those in the fights it wins, from either corner
    let covered = |fight: &str, tag: &str| fight_doc(fight)["covered"][tag].as_u64().expect("a count");
    for tag in [
        "FLUTE:basket",
        "FLUTE:cobra",
        "FLUTE:strike",
        "FLUTE:swatted",
        "FLUTE:dance",
        "FLUTE:haste",
        "FLUTE:wave",
        "FLUTE:shards",
    ] {
        assert!(covered("episode", tag) >= 7, "{tag}");
    }
    for fight in ["flute_wins", "flute_right"] {
        assert!(covered(fight, "FLUTE:solo_baskets") >= 16, "{fight}");
        assert!(
            covered(fight, "FLUTE:strike") >= 2 && covered(fight, "FLUTE:swatted") >= 2,
            "{fight}"
        );
    }
    assert_eq!(
        (index(&fight_doc("episode")["k"]), index(&fight_doc("flute_right")["k"])),
        (0, 1)
    );
}

fn frames_are_pythons(fight: &str) {
    let pic = picture(fight);
    let rows = rows(fight);
    let mut wrong = on_every_row(&rows, |row| {
        let (img, _, report) = pic.frame_reported(row.frame);
        assert_eq!(
            (report.hull_fallbacks, report.hull_errors),
            (0, 0),
            "frame {}: the beam's hull",
            row.frame
        );
        (hex(&Sha256::digest(&img.data)) != row.sha256).then(|| format!("  frame {} ({})", row.frame, row.tags))
    });
    wrong.sort();
    println!(
        "frames ({fight}): {} of {} equal Python's",
        rows.len() - wrong.len(),
        rows.len()
    );
    // the file's count of the frames that show each layer is the count of the rows read here
    for (tag, n) in fight_doc(fight)["covered"].as_object().expect("covered") {
        let here = rows.iter().filter(|r| r.tags.split(' ').any(|t| t == tag)).count();
        assert_eq!(here as u64, n.as_u64().expect("a count"), "{fight}: {tag}");
    }
    assert!(
        wrong.is_empty(),
        "{fight}: {} of {} frames are not Python's bytes:\n{}",
        wrong.len(),
        rows.len(),
        wrong.join("\n")
    );
}

/// The draw log of every frame is `reelkit.ui.DRAWN` after Python's `render`.
#[test]
fn the_draw_log_is_pythons() {
    for (fight, ..) in FIGHTS {
        draw_log_is_pythons(fight);
    }
}

fn draw_log_is_pythons(fight: &str) {
    let pic = picture(fight);
    let rows = rows(fight);
    let wrong = on_every_row(&rows, |row| {
        let (_, log) = pic.frame_logged(row.frame);
        let got: Vec<String> = log
            .boxes
            .iter()
            .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
            .collect();
        ((got.len(), sha(&got).as_str()) != row.drawn).then_some(row.frame)
    });
    println!(
        "draw logs ({fight}): {} of {} equal Python's",
        rows.len() - wrong.len(),
        rows.len()
    );
    assert!(
        wrong.is_empty(),
        "{fight}: frames whose draw log is not Python's: {wrong:?}"
    );
}

// ---------------------------------------------------------------- the order of the hooks
type Log = Arc<Mutex<Vec<String>>>;

/// A look that notes every hook call as `<its side>.<hook>` (with the ball and the phase where a hook is called
/// per ball) and then makes it on the look it wraps: the `_spied` classes of the export.
struct Spy {
    k: usize,
    log: Log,
    inner: Box<dyn Look>,
}

impl Spy {
    fn note(&self, what: &str) {
        self.log.lock().expect("the log").push(format!("{}.{what}", self.k));
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

/// With the real look every hook is still called where the Python calls it: while the picture is built (the
/// look's `fx_setup`, `fx_event` and `ko_fx` among them) and in every frame of the file, the same calls in the
/// same order: as many `draw_item`, `canvas_wave`, `shimmer` and `draw_shard` calls as Python made.
#[test]
fn the_hooks_are_called_in_pythons_order() {
    // (each fight has its own picture and its own log: the three run side by side)
    std::thread::scope(|s| {
        let jobs: Vec<_> = FIGHTS
            .iter()
            .map(|&(fight, ..)| s.spawn(move || hooks_are_called_in_pythons_order(fight)))
            .collect();
        for job in jobs {
            job.join().expect("a fight's hooks differ");
        }
    });
}

fn hooks_are_called_in_pythons_order(fight: &str) {
    let log: Log = Arc::default();
    let pic = build(fight, &|k, name| {
        Box::new(Spy {
            k,
            log: log.clone(),
            inner: one_real_look(name),
        })
    });
    let take = || std::mem::take(&mut *log.lock().expect("the log"));
    // (the kit asks rope() twice for an event that shakes the ropes: the export leaves it out of the build's log)
    let build_log: Vec<String> = take().into_iter().filter(|c| !c.contains(".rope")).collect();
    let doc = fight_doc(fight);
    let head: Vec<&str> = list(&doc["hooks_build_head"]).iter().map(text).collect();
    assert_eq!(
        build_log[..head.len()],
        head[..],
        "{fight}: the first hook calls of the build"
    );
    assert_eq!(
        (build_log.len(), sha(&build_log).as_str()),
        (index(&doc["hooks_build"][0]), text(&doc["hooks_build"][1])),
        "{fight}: the hook calls while the picture is built"
    );
    // one frame's calls in full, for a readable failure
    let full = &doc["hooks_frame"];
    pic.frame(index(&full["frame"]));
    let want: Vec<&str> = list(&full["calls"]).iter().map(text).collect();
    assert_eq!(take(), want, "{fight}: the hook calls of frame {}", full["frame"]);
    let rows = rows(fight);
    for row in &rows {
        pic.frame(row.frame);
        let got = take();
        assert_eq!(
            (got.len(), sha(&got).as_str()),
            row.hooks,
            "{fight}: the hook calls of frame {}: {got:?}",
            row.frame
        );
    }
    println!(
        "hooks ({fight}): {} frames call the hooks in Python's order",
        rows.len()
    );
}

// ---------------------------------------------------------------- the look's own state
/// Counts the values compared and how many were Python's to the bit; a value that is not must be within `1e-12`
/// of its size (a sine, an arctangent or a power: `bf_core::dmath` against the C runtime).
#[derive(Default)]
struct Tally {
    n: usize,
    exact: usize,
    failures: Vec<String>,
}

impl Tally {
    fn near(&mut self, got: f64, want: f64, what: &str) {
        self.n += 1;
        if got.to_bits() == want.to_bits() {
            self.exact += 1;
            return;
        }
        // (a value that is not a number on one side only is a failure too)
        let apart = (got - want).abs();
        if apart.is_nan() || apart > 1e-12 * want.abs().max(1.0) {
            self.failures.push(format!("  {what}: got {got:e}, Python {want:e}"));
        }
    }

    fn same(&mut self, got: f64, want: f64, what: &str) {
        self.n += 1;
        if got.to_bits() == want.to_bits() {
            self.exact += 1;
        } else {
            self.failures.push(format!(
                "  {what}: got {got:e}, Python {want:e} (must be equal to the bit)"
            ));
        }
    }

    fn near_all(&mut self, got: &[f64], want: &[f64], what: &str) {
        assert_eq!(got.len(), want.len(), "{what}: the lengths differ");
        for (i, (&g, &w)) in got.iter().zip(want).enumerate() {
            self.near(g, w, &format!("{what}[{i}]"));
        }
    }
}

/// What the look works out from the run in `setup` (its baskets, strikes, dances and swats) and its cobras at a
/// spread of times (in the fight, in a strike, after a swat, in SNAKE DANCE, at the K.O. and while the baskets
/// fade; and, in the fights it wins, its two victory baskets as they pop in, stand and sink away) are Python's.
#[test]
fn the_looks_state_is_pythons() {
    let mut seen = [0; 4];
    for (fight, ..) in FIGHTS {
        for (total, n) in seen.iter_mut().zip(state_is_pythons(fight)) {
            *total += n;
        }
    }
    let [striking, swatted, away, victory] = seen;
    assert!(striking >= 3 && swatted >= 3 && away > 0, "{seen:?}");
    assert!(victory >= 16, "{victory} victory baskets compared");
}

/// Compares one fight's state; returns how many of the cobras compared were striking, were swatted, were away
/// from their baskets and were victory baskets (ids 100 and 101).
fn state_is_pythons(fight: &str) -> [usize; 4] {
    let pic = picture(fight);
    let p = pic.ctx();
    let k = index(&fight_doc(fight)["k"]);
    let mut look = FluteLook::new();
    look.setup(p, k);
    let state = &fight_doc(fight)["state"];
    let mut t = Tally::default();
    let mut victory = 0;
    // BASKETS, in the order they were dropped
    let baskets = list(&state["baskets"]);
    assert_eq!(look.baskets().len(), baskets.len(), "the number of baskets");
    for ((bid, b), want) in look.baskets().iter().zip(baskets) {
        assert_eq!(*bid, want[0].as_i64().expect("a basket id"), "the baskets' order");
        t.same(b.t_drop, h(&want[1]), "t_drop");
        t.same(b.home[0], h(&want[2][0]), "home x");
        t.same(b.home[1], h(&want[2][1]), "home z");
        t.same(b.t_gone, h(&want[3]), "t_gone");
    }
    // STRIKES: (t0, t_hit, t_end, aim, len) of each basket's first and last strike
    let strikes = list(&state["strikes"]);
    assert_eq!(
        look.strikes().values().map(Vec::len).sum::<usize>(),
        index(&state["n_strikes"]),
        "the number of strikes"
    );
    assert!(strikes.len() >= 3);
    for want in strikes {
        let bid = want[0].as_i64().expect("a basket id");
        let got = &look.strikes()[&bid];
        assert_eq!(got.len(), index(&want[1]), "the strikes of basket {bid}");
        for (q, w) in [(got[0], &want[2]), (got[got.len() - 1], &want[3])] {
            for (g, w) in [q.0, q.1, q.2, q.3, q.4].into_iter().zip(hv(w)) {
                t.same(g, w, "a strike");
            }
        }
    }
    let dances: Vec<f64> = look.dances().iter().flat_map(|d| [d.0, d.1]).collect();
    let want: Vec<f64> = list(&state["dances"]).iter().flat_map(hv).collect();
    assert_eq!(dances.len(), want.len(), "the SNAKE DANCEs");
    for (g, w) in dances.into_iter().zip(want) {
        t.same(g, w, "a dance");
    }
    t.same(look.swat_down(), h(&state["swat_down"]), "SWAT_DOWN");
    let swats = list(&state["swats"]);
    assert_eq!(look.swats().len(), swats.len(), "the baskets swatted");
    for want in swats {
        let got = &look.swats()[&want[0].as_i64().expect("a basket id")];
        let want = hv(&want[1]);
        assert_eq!(got.len(), want.len());
        for (g, w) in got.iter().zip(want) {
            t.same(*g, w, "a swat");
        }
    }
    // the cobras
    let rows = list(&state["cobras"]);
    assert!(rows.len() >= 6);
    let (mut striking, mut swatted, mut away) = (0, 0, 0);
    for row in rows {
        let time = h(&row[0]);
        let what = format!("t = {time}");
        t.near(look.dancing_k(time), h(&row[1]), &format!("{what}: dancing_k"));
        let fs = [p.fighter(0, time), p.fighter(1, time)];
        let got = look.cobras_at(p, time, &fs);
        let want = list(&row[2]);
        assert_eq!(got.len(), want.len(), "{what}: the number of cobras");
        for (c, w) in got.iter().zip(want) {
            assert_eq!(c.bid, w[0].as_i64().expect("a basket id"), "{what}: the cobras' order");
            let what = format!("{what}, basket {}", c.bid);
            t.near_all(&c.home, &hv(&w[1]), &format!("{what}: home"));
            t.near(c.bk, h(&w[2]), &format!("{what}: bk"));
            t.near_all(&c.cp, &hv(&w[3]), &format!("{what}: cp"));
            t.near(c.t_drop, h(&w[4]), &format!("{what}: t_drop"));
            assert_eq!(c.strike.is_some(), !w[5].is_null(), "{what}: striking");
            if let Some(q) = c.strike {
                striking += 1;
                t.near_all(&[q.0, q.1, q.2, q.3, q.4], &hv(&w[5]), &format!("{what}: strike"));
            }
            t.near(c.aim, h(&w[6]), &format!("{what}: aim"));
            let tk = if time < p.t_ko { time } else { p.t_ko };
            t.near(look.swat_level(c.bid, tk), h(&w[7]), &format!("{what}: swat_level"));
            let lean = look.swat_lean(c.bid, tk);
            assert_eq!(lean.is_some(), !w[8].is_null(), "{what}: swat_lean");
            if let Some((since, strength)) = lean {
                swatted += 1;
                t.near_all(&[since, strength], &hv(&w[8]), &format!("{what}: swat_lean"));
            }
            if c.cp != c.home {
                away += 1;
            }
            if c.bid >= 100 {
                victory += 1;
            }
        }
    }
    println!("state ({fight}): {} values, {} equal to the bit", t.n, t.exact);
    assert!(t.failures.is_empty(), "{fight}:\n{}", t.failures.join("\n"));
    [striking, swatted, away, victory]
}

// ---------------------------------------------------------------- the focus check's points
/// `focus_points` gives the kit's subject points for the look (`kit/checks.py` `focus`: each basket, each raised
/// hood and a strike's head at its reach) and every one of them is in focus (a circle of confusion of 0), on
/// every 10th frame of the three fights.
#[test]
fn the_looks_subject_points_stay_in_focus() {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (fight, ..) in FIGHTS {
        let pic = picture(fight);
        let p = pic.ctx();
        let k = index(&fight_doc(fight)["k"]);
        let mut look = FluteLook::new();
        look.setup(p, k);
        for f in (0..pic.nf()).step_by(10) {
            let t = f as f64 / 30.0;
            let fs = [p.fighter(0, t), p.fighter(1, t)];
            let cobras = look.cobras_at(p, t, &fs);
            let striking = cobras.iter().filter(|c| c.strike.is_some()).count();
            let own = look.focus_points(p, t, &fs[k]);
            assert_eq!(own.len(), 2 * cobras.len() + striking, "{fight}, frame {f}");
            for (_, label) in &own {
                *seen.entry(label.clone()).or_default() += 1;
            }
            // the frame's whole list holds them, and nothing on it is out of focus
            let all = pic.focus_points(f);
            for label in ["basket", "cobra", "strike"] {
                let n = own.iter().filter(|q| q.1 == label).count();
                assert_eq!(
                    all.iter().filter(|q| q.1 == label).count(),
                    n,
                    "{fight}, frame {f}: {label}"
                );
            }
            let worst = all
                .into_iter()
                .fold((0.0, String::new()), |m, q| if q.0 > m.0 { q } else { m });
            assert_eq!(worst.0, 0.0, "{fight}, frame {f}: {} is out of focus", worst.1);
        }
    }
    println!("focus: {seen:?}, all in focus");
    assert!(
        seen["basket"] > 100 && seen["cobra"] == seen["basket"] && seen["strike"] > 5,
        "{seen:?}"
    );
}

// ---------------------------------------------------------------- the sprites
/// `sprite(r, yaw, roll)` is `flute_sprite`: every in-ball sprite of the FLUTE in the reference set
/// (`kit.picture.sprite` at four radii, 60 yaws and two rolls, and the ones the episode's frames ask for).
#[test]
fn the_sprites_are_pythons() {
    let v = Vectors::open("sprites/FLUTE.npz");
    assert_eq!(v.index.raw["drawn_by"], "look");
    let look = FluteLook::new();
    let (mut n, mut wrong) = (0, Vec::new());
    for case in &v.index.cases {
        if case.call != "kit.picture.sprite(k, radius, yaw, roll)" {
            continue;
        }
        let a = &case.args;
        let (r, yaw, roll) = (
            a["radius"].as_u64().expect("radius") as u32,
            a["yaw"].as_f64().expect("yaw"),
            a["roll"].as_f64().expect("roll"),
        );
        let want = v.array(case, "out");
        let got = look.sprite(r, yaw, roll).expect("the look draws its own sprite");
        n += 1;
        if want.shape != [got.h, got.w, 4] || got.data != want.u8() {
            wrong.push(format!("{} (radius {r}, yaw {yaw}, roll {roll})", case.id));
        }
    }
    println!("sprites: {} of {n} equal Python's", n - wrong.len());
    assert_eq!(n, 503, "sprites/FLUTE.npz holds 480 + 23 in-ball sprites");
    assert!(wrong.is_empty(), "sprites that are not Python's: {wrong:?}");
}

// ---------------------------------------------------------------- the title card's hooks
/// The FLUTE's close-up as `title.big_sprite(0)` returned it: the one `title` case of `sprites/FLUTE.npz`.
fn python_closeup() -> Img<u8> {
    let v = Vectors::open("sprites/FLUTE.npz");
    let cases: Vec<_> = v.index.cases.iter().filter(|c| c.args["group"] == "title").collect();
    assert_eq!(cases.len(), 1, "sprites/FLUTE.npz holds one title close-up");
    assert_eq!(cases[0].args["drawn_by"], "closeup");
    let a = v.array(cases[0], "out");
    assert_eq!(a.shape, [93, 93, 4]);
    Img::from_vec(93, 93, 4, a.u8().to_vec())
}

/// The look's constants and its three title-card hooks give what Python's give: the glass, the close-up sprite
/// (against the export's hash and against the sprite in the reference set) and the panel pattern on a fixed
/// base picture, every `float64` of it.
#[test]
fn the_title_hooks_are_pythons() {
    let title = &python()["title"];
    let look = FluteLook::new();
    let glass: Vec<f64> = list(&title["glass"])
        .iter()
        .map(|v| v.as_f64().expect("a level"))
        .collect();
    assert_eq!(look.glass()[..], glass[..]);
    assert_eq!(look.glass(), [150.0, 160.0, 196.0]);
    assert_eq!(look.flare_scale().to_bits(), h(&title["flare_scale"]).to_bits());
    let shards: Vec<&str> = list(&title["shards"]).iter().map(text).collect();
    assert_eq!(look.shards(), shards);
    // title_closeup(44) = flute_sprite(44, 0.0, the view's roll)
    let close = look.title_closeup(44).expect("the look has its own close-up");
    assert_eq!(index(&title["closeup"]["radius"]), 44);
    assert_eq!((close.w, close.h, close.c), (93, 93, 4));
    assert_eq!(close, python_closeup(), "the close-up against sprites/FLUTE.npz");
    assert_eq!(hex(&Sha256::digest(&close.data)), text(&title["closeup"]["sha256"]));
    assert_eq!(close, flute_sprite(44, 0.0, 32.0));
    // title_pattern on base[y, x, c] = 40.0 + x * 0.5 + y * 0.25 + 20.0 * c
    let pattern = &title["pattern"];
    assert!(text(&pattern["base"]).starts_with("base[y, x, c] = 40.0 + x * 0.5 + y * 0.25 + 20.0 * c"));
    let mut base: RgbF = Img::new(360, 640, 3);
    for y in 0..640 {
        for x in 0..360 {
            for (c, v) in base.px_mut(x, y).iter_mut().enumerate() {
                *v = 40.0 + x as f64 * 0.5 + y as f64 * 0.25 + 20.0 * c as f64;
            }
        }
    }
    let mut by_hook = base.clone();
    look.title_pattern(&mut by_hook);
    flute_keys(&mut base);
    assert_eq!(by_hook, base, "title_pattern is flute_keys");
    for s in list(&pattern["samples"]) {
        let (x, y) = (index(&s[0]), index(&s[1]));
        let want = hv(&s[2]);
        for (c, (&g, w)) in base.px(x, y).iter().zip(want).enumerate() {
            assert_eq!(
                g.to_bits(),
                w.to_bits(),
                "the pattern at ({x}, {y}), channel {c}: {g:e}, Python {w:e}"
            );
        }
    }
    let bytes: Vec<u8> = base.data.iter().flat_map(|v| v.to_le_bytes()).collect();
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        text(&pattern["sha256"]),
        "the whole pattern"
    );
}

/// The card of `show` with the real look for the FLUTE and the defaults for the TROMBONE, built without a
/// picture (the look's `setup` is never called: its title hooks need nothing from a fight).
fn card_of(show: &Show, series_tag: Option<&str>) -> TitleCard {
    let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
    let meshes = [mesh(&cfg[0].pp), mesh(&cfg[1].pp)];
    let looks = [one_real_look(&show.names[0]), one_real_look(&show.names[1])];
    let view = |c: &FighterCfg| {
        let v = c.raw["view"].as_array().expect("the roster entry has a view");
        (v[0].as_f64().expect("the yaw"), v[2].as_f64().expect("the roll"))
    };
    TitleCard::new(
        show,
        [cfg[0].color.map(f64::from), cfg[1].color.map(f64::from)],
        [cfg[0].dark.map(f64::from), cfg[1].dark.map(f64::from)],
        [0, 1].map(|k| {
            let (yaw, roll) = view(&cfg[k]);
            CardLook {
                look: &*looks[k],
                mesh: &meshes[k],
                yaw,
                roll,
            }
        }),
        series_tag,
    )
    .expect("both fighters' panel patterns are ported")
}

/// Every frame of the VS card is Python's with the real look (`title/hashes.npz`: the episode, the fighters in
/// swapped corners, 96 BPM without a tag and 200 BPM; 249 cards), drawn over the reference set's noise image:
/// the data `bf-picture`'s stand-in for this look was checked against. The episode's cards are drawn twice: by a
/// card built from the look alone, and by the card of the picture (the look after `setup`).
#[test]
fn every_card_frame_is_pythons_with_the_real_look() {
    let v = Vectors::open("frames/flute_vs_trombone/title/hashes.npz");
    let (w, hgt, c, data) = read_png(&reference("frames/flute_vs_trombone/title/noise.png"));
    assert_eq!((w, hgt, c), (360, 640, 3));
    let noise: Img<u8> = Img::from_vec(w, hgt, c, data);
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let ff = fight_file();
    let episode = Show::new(&rec, &ff).expect("the episode has a K.O.");
    let (mut cards, mut wrong) = (0, Vec::new());
    let mut names = Vec::new();
    for case in &v.index.cases {
        assert_eq!(
            case.call,
            "hashlib.sha256(title.draw(noise, frame / 30).tobytes()).digest()"
        );
        let a = &case.args;
        let timeline = a["timeline"].as_str().expect("args.timeline");
        names.push(timeline.to_string());
        let mut show = episode.clone();
        show.grid = Grid::new(a["bpm"].as_f64().expect("args.bpm"));
        let fighters: Vec<&str> = list(&a["fighters"]).iter().map(text).collect();
        show.names = [fighters[0].to_string(), fighters[1].to_string()];
        assert_eq!(
            show.grid.beat.to_bits(),
            hex_arg(&a["timeline_values"]["beat"]).to_bits(),
            "{timeline}"
        );
        let alone = card_of(&show, a["series"].as_str());
        let mut drawers: Vec<(&str, &TitleCard)> = vec![("the look alone", &alone)];
        if timeline == "episode" {
            assert_eq!(ff.series_tag.as_deref(), a["series"].as_str());
            drawers.push(("the picture's card", picture("episode").card()));
        }
        let frames = v.array(case, "frames").i32();
        let digests = v.array(case, "sha256").u8();
        assert_eq!(digests.len(), frames.len() * 32);
        for (what, card) in &drawers {
            for (&f, want) in frames.iter().zip(digests.chunks(32)) {
                let mut img = noise.clone();
                card.draw(&mut img, f64::from(f) / 30.0);
                cards += 1;
                if Sha256::digest(&img.data)[..] != *want {
                    wrong.push(format!("{timeline}, frame {f} ({what})"));
                }
            }
        }
    }
    assert_eq!(names, ["episode", "swapped", "bpm96", "bpm200"]);
    println!("title cards: {} of {cards} equal Python's", cards - wrong.len());
    assert!(cards >= 249 + 61);
    assert!(
        wrong.is_empty(),
        "{} cards are not Python's bytes; the first: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(12)]
    );
}

// ---------------------------------------------------------------- the stages (local dumps)
/// How one buffer compares with Python's: `(samples that differ, the largest difference, the first)`.
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
            if d.first.is_none() {
                d.first = Some((i / c % w, i / c / w, i % c, gf, pf));
            }
            let (x, y) = (i / c % w, i / c / w);
            d.bbox = Some(match d.bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    d
}

#[derive(Debug, Default)]
struct Diff {
    differing: usize,
    worst: f64,
    /// `(x, y, channel, got, want)`.
    first: Option<(usize, usize, usize, f64, f64)>,
    /// `(x0, y0, x1, y1)`, inclusive.
    bbox: Option<(usize, usize, usize, usize)>,
}

fn diff_rgb(got: &StagePic, want: &NpArray) -> Diff {
    let (w, hgt, c, dtype) = match got {
        StagePic::U8(a) => (a.w, a.h, a.c, "|u1"),
        StagePic::F32(a) => (a.w, a.h, a.c, "<f4"),
        StagePic::F64(a) => (a.w, a.h, a.c, "<f8"),
    };
    let shape: Vec<usize> = if c == 1 { vec![hgt, w] } else { vec![hgt, w, c] };
    assert_eq!(want.shape, shape, "the shapes differ");
    assert_eq!(want.dtype(), dtype, "Python holds this buffer as {}", want.dtype());
    match (got, &want.data) {
        (StagePic::U8(a), NpData::U8(p)) => diff_of(&a.data, p, |x, y| x == y, w, c),
        (StagePic::F32(a), NpData::F32(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), w, c),
        (StagePic::F64(a), NpData::F64(p)) => diff_of(&a.data, p, |x, y| x.to_bits() == y.to_bits(), w, c),
        _ => unreachable!("the dtypes were compared"),
    }
}

/// Every stage of every frame dumped by `python -m tools.studio_export look_flute_stages <frame>...` (local
/// files, see the top of this file): the drawing stages and the depth buffers are Python's to the bit, and so is
/// the frame from the cut to bytes on; the float stages between the lens and the cut to bytes pass through the
/// FFT and are held to `1e-9` of a level (as in `bf-picture`'s `ring_stages.rs`).
#[test]
fn every_stage_matches_python() {
    let dir = reference_dir().join(".cache/stages_look_flute");
    let mut dumps: BTreeMap<(&str, usize), std::path::PathBuf> = BTreeMap::new();
    for (fight, ..) in FIGHTS {
        if let Ok(entries) = std::fs::read_dir(dir.join(fight)) {
            for path in entries.flatten().map(|e| e.path()) {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if let Some(f) = name.strip_prefix('f').and_then(|n| n.strip_suffix(".npz")) {
                    dumps.insert((fight, f.parse().expect("f<frame>.npz")), path.clone());
                }
            }
        }
    }
    if dumps.is_empty() {
        eprintln!(
            "skipped: no stage dumps in {} (see the top of tests/flute_look.rs)",
            dir.display()
        );
        return;
    }
    let mut failures = Vec::new();
    for (&(fight, f), path) in &dumps {
        let pic = picture(fight);
        let z = Npz::open(path);
        for stage in Stage::ALL {
            let got = pic.stage_buffers(f, stage);
            let name = stage.name();
            let key = if stage == Stage::Coc {
                format!("{name}__map")
            } else {
                format!("{name}__rgb")
            };
            let d = diff_rgb(&got.rgb, z.get(&key));
            let float = (Stage::Coc..=Stage::PostFx).contains(&stage);
            if if float { d.worst > 1e-9 } else { d.differing != 0 } {
                failures.push(format!(
                    "{fight}, frame {f} {name}: {} samples differ, worst {:e}, box {:?}, first {:?}",
                    d.differing, d.worst, d.bbox, d.first
                ));
            }
            let depth_key = format!("{name}__depth");
            if z.has(&depth_key) {
                let depth = got.depth.as_ref().expect("the stage has a depth buffer");
                let d = diff_of(
                    &depth.data,
                    z.get(&depth_key).f32(),
                    |x, y| x.to_bits() == y.to_bits(),
                    depth.w,
                    1,
                );
                if d.differing != 0 {
                    failures.push(format!(
                        "{fight}, frame {f} {name} depth: {} samples differ, worst {:e}, box {:?}, first {:?}",
                        d.differing, d.worst, d.bbox, d.first
                    ));
                }
            }
        }
    }
    println!(
        "stages: frames {:?}, {} buffers differ",
        dumps.keys().collect::<Vec<_>>(),
        failures.len()
    );
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
