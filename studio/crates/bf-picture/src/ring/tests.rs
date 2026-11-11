//! Unit tests of the ring renderer and the scene against numbers the Python kit printed for the example episode
//! drawn with its base-class look for both fighters: `tests_python.json` beside this file is the output of
//! `tools/studio_export/unit_checks/ring_unit_vectors.py`.
//!
//! A value made of plain arithmetic must be Python's to the bit. A value that passes through a sine, an
//! exponential or a fused dot product may differ in its last bits (`bf_core::dmath` against the C runtime and
//! numpy): those are held to `1e-12` of their size, and each test prints how many were equal to the bit.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use bf_core::event::Event;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_core::show::Show;
use bf_testdata::{parse_hex_f64, reference, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::state::{BallState, Wave};
use super::{Picture, PictureCtx, PictureError};
use crate::camera::{Cam, V3};
use crate::img::Img;
use crate::lighting::Lights;
use crate::look::{BallSprite, CanvasCtx, DefaultLook, DrawnBall, FrameCtx, Fx, Look, Payload, Phase, Pic, Spark};
use crate::raster::Frame;
use crate::scene::{self, Scene};
use crate::sprite::Mesh;

fn python() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| serde_json::from_str(include_str!("tests_python.json")).expect("tests_python.json is JSON"))
}

/// A float Python wrote as `float.hex()`.
fn h(v: &Value) -> f64 {
    parse_hex_f64(v.as_str().unwrap_or_else(|| panic!("{v} is not a hex float")))
}

/// A list of floats.
fn hv(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap_or_else(|| panic!("{v} is not a list"))
        .iter()
        .map(h)
        .collect()
}

fn v3(v: &Value) -> V3 {
    let a = hv(v);
    [a[0], a[1], a[2]]
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

fn fighter(name: &str) -> FighterCfg {
    let path = repo_root().join("studio/crates/bf-content/data/roster.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let roster = parse_roster(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    roster
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the roster"))
}

fn mesh(pp: &str) -> Mesh {
    let path = repo_root().join(format!("studio/crates/bf-content/data/models/{pp}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Mesh::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// What a picture of the example episode is built from.
struct Parts {
    rec: Arc<Recording>,
    ff: FightFile,
    show: Show,
    cfg: [FighterCfg; 2],
}

fn parts() -> Parts {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
    let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
    Parts {
        rec: Arc::new(rec),
        ff,
        show,
        cfg,
    }
}

fn build(q: Parts, looks: [Box<dyn Look>; 2]) -> Result<Picture, PictureError> {
    let meshes = [mesh(&q.cfg[0].pp), mesh(&q.cfg[1].pp)];
    Picture::new(q.rec, &q.ff, q.show, [&q.cfg[0], &q.cfg[1]], meshes, looks)
}

fn default_looks() -> [Box<dyn Look>; 2] {
    [Box::new(DefaultLook), Box::new(DefaultLook)]
}

/// The example episode with the default look for both fighters.
fn picture() -> &'static Picture {
    static PICTURE: OnceLock<Picture> = OnceLock::new();
    PICTURE.get_or_init(|| build(parts(), default_looks()).expect("the picture builds"))
}

/// Counts the values compared and how many were Python's to the bit.
#[derive(Default)]
struct Tally {
    n: usize,
    exact: usize,
    worst: f64,
}

impl Tally {
    /// `got` is Python's `want` to the bit.
    #[track_caller]
    fn same(&mut self, got: f64, want: f64, what: &str) {
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{what}: got {got:?}, Python has {want:?}"
        );
        self.n += 1;
        self.exact += 1;
    }

    /// `got` is within `1e-12` of the size of Python's `want` (or of `size`, for a component of a vector).
    #[track_caller]
    fn near_in(&mut self, got: f64, want: f64, size: f64, what: &str) {
        self.n += 1;
        if got.to_bits() == want.to_bits() {
            self.exact += 1;
            return;
        }
        let d = (got - want).abs();
        assert!(
            d <= 1e-12 * size.abs().max(want.abs()).max(1e-3),
            "{what}: got {got:?}, Python has {want:?} (off by {d:e})"
        );
        self.worst = self.worst.max(d);
    }

    #[track_caller]
    fn near(&mut self, got: f64, want: f64, what: &str) {
        self.near_in(got, want, want, what);
    }

    #[track_caller]
    fn near_all(&mut self, got: &[f64], want: &[f64], what: &str) {
        assert_eq!(
            got.len(),
            want.len(),
            "{what}: {} values, Python has {}",
            got.len(),
            want.len()
        );
        let size = want.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        for (j, (&g, &w)) in got.iter().zip(want).enumerate() {
            self.near_in(g, w, size, &format!("{what}[{j}]"));
        }
    }

    #[track_caller]
    fn same_all(&mut self, got: &[f64], want: &[f64], what: &str) {
        assert_eq!(
            got.len(),
            want.len(),
            "{what}: {} values, Python has {}",
            got.len(),
            want.len()
        );
        for (j, (&g, &w)) in got.iter().zip(want).enumerate() {
            self.same(g, w, &format!("{what}[{j}]"));
        }
    }

    fn report(&self, name: &str) {
        println!(
            "{name}: {} values, {} equal to the bit, worst difference {:e}",
            self.n, self.exact, self.worst
        );
    }
}

fn rows(v: &Value) -> Vec<V3> {
    list(v).iter().map(v3).collect()
}

fn flat(pts: &[V3]) -> Vec<f64> {
    pts.iter().flatten().copied().collect()
}

#[test]
fn the_scene_is_pythons() {
    let s = &python()["scene"];
    let p = picture().ctx();
    let sc: &Scene = &p.scene;
    let mut t = Tally::default();
    assert_eq!(sc.crowd.len(), index(&s["n_crowd"]));
    assert_eq!(sc.stands.len(), index(&s["n_stands"]));
    assert_eq!(sc.barrier.len(), index(&s["n_barrier"]));
    let (cos0, cos1) = scene::spot_cos();
    let want = hv(&python()["spot_cos"]);
    t.near(cos0, want[0], "SPOT_COS[0]");
    t.near(cos1, want[1], "SPOT_COS[1]");
    for row in list(&s["crowd"]) {
        let q = &sc.crowd[index(&row[0])];
        t.near_all(&q.p, &hv(&row[1]), "CROWD p");
        assert_eq!(q.ci, index(&row[2]), "CROWD colour index");
        t.same(q.ph, h(&row[3]), "CROWD phase");
        assert_eq!(q.row, index(&row[4]), "CROWD row");
        t.same(q.ht, h(&row[5]), "CROWD height");
    }
    for row in list(&s["stands"]) {
        let q = &sc.stands[index(&row[0])];
        t.near_all(&flat(&q.q), &flat(&rows(&row[1])), "STANDS quad");
        t.near_all(&q.n, &hv(&row[2]), "STANDS normal");
        t.same_all(&q.col, &hv(&row[3]), "STANDS colour");
        t.same(q.am, h(&row[4]), "STANDS angle");
    }
    for row in list(&s["barrier"]) {
        let q = &sc.barrier[index(&row[0])];
        t.near_all(&flat(&q.q), &flat(&rows(&row[1])), "BARRIER quad");
        t.near_all(&q.n, &hv(&row[2]), "BARRIER normal");
        t.same(q.am, h(&row[3]), "BARRIER angle");
    }
    for row in list(&s["skirt"]) {
        let q = &sc.skirt_faces[index(&row[0])];
        t.same_all(&flat(&q.q), &flat(&rows(&row[1])), "SKIRT_FACES quad");
        t.near_all(&q.n, &hv(&row[2]), "SKIRT_FACES normal");
        t.same_all(&q.col, &hv(&row[3]), "SKIRT_FACES colour");
    }
    let counts: Vec<usize> = list(&s["n_posts"]).iter().map(index).collect();
    assert_eq!(sc.posts.iter().map(Vec::len).collect::<Vec<_>>(), counts);
    for row in list(&s["posts"]) {
        let q = &sc.posts[index(&row[0])][index(&row[1])];
        t.near_all(&flat(&q.q), &flat(&rows(&row[2])), "POSTS quad");
        t.near_all(&q.n, &hv(&row[3]), "POSTS normal");
        t.same_all(&q.col, &hv(&row[4]), "POSTS colour");
    }
    for (c, row) in list(&s["corner_col"]).iter().enumerate() {
        t.same_all(&sc.corner_col[c], &hv(row), "CORNER_COL");
    }
    t.same_all(&sc.red_light, &hv(&s["red_light"]), "RED_LIGHT");
    t.same_all(&sc.blue_light, &hv(&s["blue_light"]), "BLUE_LIGHT");
    for row in list(&s["stand_c"]) {
        t.near_all(&p.lists.stand_c[index(&row[0])], &hv(&row[1]), "STAND_C");
    }
    for row in list(&s["bar_c"]) {
        t.near_all(&p.lists.bar_c[index(&row[0])], &hv(&row[1]), "BAR_C");
    }
    let casters: Vec<f64> = p.static_casters.iter().flatten().flatten().copied().collect();
    let want: Vec<f64> = list(&s["static_casters"]).iter().flat_map(|q| flat(&rows(q))).collect();
    t.same_all(&casters, &want, "STATIC_CASTERS");
    for row in list(&python()["albedo"]) {
        let got = scene::canvas_albedo(h(&row[0]), h(&row[1]));
        t.same_all(&got, &hv(&row[2]), "canvas_albedo");
    }
    t.report("scene");
}

#[test]
fn the_derivations_are_pythons() {
    let d = &python()["derived"];
    let p = picture().ctx();
    let mut t = Tally::default();
    assert_eq!(p.run.nsim, index(&d["nsim"]));
    assert_eq!(p.win, index(&d["win"]));
    t.same(p.t_ko, h(&d["t_ko"]), "T_KO");
    t.same(p.t_hs, h(&d["t_hs"]), "T_HS");
    t.same(p.t_solo, h(&d["t_solo"]), "T_SOLO");
    t.same_all(&p.solo_t, &hv(&d["solo_t"]), "SOLO_T");
    t.same_all(&p.rest, &hv(&d["rest"]), "REST");
    t.same_all(&p.omega, &hv(&d["omega"]), "OMEGA");
    t.same_all(&p.th0, &hv(&d["th0"]), "TH0");
    t.same_all(&p.sd0, &hv(&d["sd0"]), "SD0");
    t.same_all(&p.base_len, &hv(&d["base_len"]), "BASE_LEN");
    let frets: Vec<i64> = list(&d["frets0"])
        .iter()
        .map(|v| v.as_i64().expect("a whole number"))
        .collect();
    assert_eq!(p.frets0[..], frets[..]);
    for k in 0..2 {
        t.same_all(&p.p_ko[k], &hv(&d["p_ko"][k]), "P_KO");
        t.near_all(&p.glide_arc[k].0, &hv(&d["glide_arc"][k][0]), "GLIDE_ARC normal");
        t.same(p.glide_arc[k].1, h(&d["glide_arc"][k][1]), "GLIDE_ARC a");
        t.same_all(&p.light_rgb[k], &hv(&d["light_rgb"][k]), "LIGHT_RGB");
        t.same_all(&p.crowd_tint[k], &hv(&d["crowd_tint"][k]), "CROWD_TINT");
        t.same_all(&p.fl_col[k], &hv(&d["fl_col"][k]), "FL_COL");
    }
    assert_eq!(p.shard_seed, d["shard_seed"].as_u64().expect("a seed"));
    assert_eq!(p.fx_seed, d["fx_seed"].as_u64().expect("a seed"));
    assert_eq!(p.rope_ev.len(), index(&d["n_rope_ev"]));
    for row in list(&d["rope_ev"]) {
        let r = &p.rope_ev[index(&row[0])];
        assert_eq!(r.side, index(&row[2]), "ROPE_EV side");
        t.same_all(
            &[r.t, r.x, r.z, r.spd],
            &[h(&row[1]), h(&row[3]), h(&row[4]), h(&row[5])],
            "ROPE_EV",
        );
    }
    let waves: Vec<f64> = p.wave_ev.iter().flat_map(|&(a, b)| [a, b]).collect();
    let want: Vec<f64> = list(&d["wave_ev"]).iter().flat_map(hv).collect();
    t.same_all(&waves, &want, "WAVE_EV");
    let nf = p.flash_t.len();
    assert_eq!(nf, index(&d["n_flash"]));
    let picked: Vec<f64> = [0, 1, 2, 20, nf - 17, nf - 1].iter().map(|&j| p.flash_t[j]).collect();
    t.same_all(&picked, &hv(&d["flash_t"]), "FLASH_T");
    assert_eq!(p.status.len(), index(&d["n_status"]));
    for k in 0..2 {
        for s in 0..3 {
            let want = &d["abil_unlock"][k][s];
            match p.abil.unlock[k][s] {
                Some(tu) => t.same(tu, h(want), "ABIL_UNLOCK"),
                None => assert!(want.is_null(), "ABIL_UNLOCK[{k}][{s}]: Python has {want}"),
            }
            t.same_all(&p.abil.uses[k][s], &hv(&d["abil_uses"][k][s]), "ABIL_USES");
            t.same(p.abil.hold[k][s], h(&d["abil_hold"][k][s]), "ABIL_HOLD");
        }
    }
    for row in list(&d["shake"]) {
        let (dx, dy) = p.shk.offset(h(&row[0]));
        t.near_all(&[dx, dy], &hv(&row[1]), "SHK.offset");
    }
    assert_eq!(p.shk.len(), index(&d["n_shake"]), "SHK");
    assert_eq!(p.fx.fl.len(), index(&d["n_flares"]), "FL");
    assert_eq!(p.fx.bu.len(), index(&d["n_bursts"]), "BU");
    assert_eq!(p.fx.mk.len(), index(&d["n_marks"]), "MK");
    assert_eq!(p.fx.mo.len(), index(&d["n_moments"]), "MO");
    assert_eq!(p.fx.pa.particles().len(), index(&d["n_particles"]), "PA");
    // the first particle of the K.O. spray as emitted (the particles are kept sorted by time: all share T_KO)
    let first = &p.fx.pa.particles()[0];
    let want = list(&d["ko_particle"]);
    t.near_all(&first.p, &hv(&want[0]), "the K.O. spray's first particle, position");
    t.near_all(&first.v, &hv(&want[1]), "the K.O. spray's first particle, velocity");
    t.same(first.life, h(&want[2]), "the K.O. spray's first particle, life");
    t.report("derivations");
}

#[test]
fn fighter_and_the_clocks_are_pythons() {
    let p = picture().ctx();
    let mut t = Tally::default();
    for row in list(&python()["fighter"]) {
        let time = h(&row[0]);
        for k in 0..2 {
            let (st, w) = (p.fighter(k, time), &row[1 + k]);
            let what = |f: &str| format!("fighter({k}, {time}).{f}");
            t.near_all(&st.p, &hv(&w["p"]), &what("p"));
            assert_eq!(i64::from(st.vis), w["vis"].as_i64().expect("vis"), "{}", what("vis"));
            assert_eq!(
                i64::from(st.shattered),
                w["shattered"].as_i64().expect("shattered"),
                "{}",
                what("shattered")
            );
            assert_eq!(st.frets, w["frets"].as_i64().expect("frets"), "{}", what("frets"));
            assert_eq!(st.level, w["level"].as_i64().expect("level"), "{}", what("level"));
            for (name, got) in [
                ("shield", st.shield),
                ("haste", st.haste),
                ("glow", st.glow),
                ("scale", st.scale),
                ("th", st.th),
                ("blen", st.blen),
                ("om", st.om),
                ("sd", st.sd),
                ("ext", st.ext),
                ("slow", st.slow),
                ("thr_k", st.thr_k),
                ("lift", st.lift),
            ] {
                t.near(got, h(&w[name]), &what(name));
            }
        }
    }
    for row in list(&python()["clock"]) {
        let time = h(&row[0]);
        assert_eq!(p.sim_i(time), index(&row[1]), "sim_i({time})");
        t.same(p.fx_clock(time), h(&row[2]), "fx_clock");
        t.near(p.hp_at(0, time), h(&row[3]), "hp_at(0)");
        t.near(p.hp_at(1, time), h(&row[4]), "hp_at(1)");
        t.near(p.win_k(time), h(&row[5]), "win_k");
        t.near(p.solo_env(time), h(&row[6]), "solo_env");
        t.near_all(&p.spot_aim(time), &hv(&row[7]), "spot_aim");
        t.same(p.part_time(time), h(&row[8]), "part_time");
    }
    t.report("fighter and clocks");
}

#[test]
fn the_camera_is_pythons() {
    let p = picture().ctx();
    let mut t = Tally::default();
    for row in list(&python()["camera"]) {
        let cam = p.camera(h(&row[0]));
        t.near_all(&cam.pos, &hv(&row[1]), "cam.pos");
        t.near_all(&flat(&cam.m), &flat(&rows(&row[2])), "cam.M");
        t.near(cam.f, h(&row[3]), "cam.f");
        t.near(cam.cx, h(&row[4]), "cam.cx");
        t.near(cam.cy, h(&row[5]), "cam.cy");
    }
    t.report("camera");
}

#[test]
fn the_ropes_are_pythons() {
    let p = picture().ctx();
    let mut t = Tally::default();
    for row in list(&python()["ropes"]) {
        let (s, y, time, n) = (index(&row[0]), h(&row[1]), h(&row[2]), index(&row[3]));
        let got = p.rope_points(s, y, time, n);
        t.near_all(
            &flat(&got),
            &flat(&rows(&row[4])),
            &format!("rope_points({s}, {y}, {time}, {n})"),
        );
    }
    t.report("ropes");
}

#[test]
fn the_shards_are_pythons() {
    let q = &python()["parts"];
    let p = picture().ctx();
    let mut t = Tally::default();
    assert_eq!(p.parts.len(), index(&q["n"]));
    assert_eq!(p.traj[0].len(), index(&q["n_steps"]));
    for row in list(&q["parts"]) {
        let part = &p.parts[index(&row[0])];
        assert_eq!(part.kind, text(&row[1]));
        t.same_all(&part.p, &hv(&row[2]), "PARTS p");
        t.near_all(&part.v, &hv(&row[3]), "PARTS v");
        t.same_all(
            &[part.rot, part.w, part.size, part.curl],
            &[h(&row[4]), h(&row[5]), h(&row[6]), h(&row[7])],
            "PARTS rot, w, size, curl",
        );
    }
    for row in list(&q["traj"]) {
        let (j, i) = (index(&row[0]), index(&row[1]));
        t.near_all(&p.traj[j][i], &hv(&row[2]), &format!("TRAJ[{j}, {i}]"));
    }
    for row in list(&q["at"]) {
        let time = h(&row[0]);
        let got = p.parts_at(time);
        let want = list(&row[1]);
        assert_eq!(got.is_empty(), want.is_empty(), "parts_at({time})");
        for w in want {
            let part = &got[index(&w[0])];
            t.near_all(&part.p, &hv(&w[1]), &format!("parts_at({time}) p"));
            t.near(part.rot, h(&w[2]), "parts_at rot");
            t.near(part.size, h(&w[3]), "parts_at size");
        }
    }
    t.report("shards");
}

#[test]
fn the_huds_rows_and_status_are_pythons() {
    let p = picture().ctx();
    let mut t = Tally::default();
    for row in list(&python()["rows"]) {
        let time = h(&row[0]);
        for k in 0..2 {
            let got = p.ability_rows(k, time);
            let want = list(&row[1][k]);
            assert_eq!(got.len(), want.len());
            for (r, w) in got.iter().zip(want) {
                let what = format!("ability_rows({k}, {time}) {}", r.name);
                assert_eq!((r.tag.as_str(), r.name.as_str()), (text(&w[0]), text(&w[1])));
                assert_eq!(r.unlocked, w[2] == "unlocked", "{what}");
                t.near(r.glow, h(&w[3]), &what);
                match r.sweep {
                    Some(s) => t.near(s, h(&w[4]), &what),
                    None => assert!(w[4].is_null(), "{what}: Python has a sweep"),
                }
            }
            let (name, sk) = p.status(k, time);
            assert_eq!(name, text(&row[2][k][0]), "status({k}, {time})");
            t.near(sk, h(&row[2][k][1]), "status_k");
        }
    }
    for row in list(&python()["led"]) {
        let got = p.led_colour(h(&row[0]), h(&row[1]), h(&row[2]));
        t.near_all(&got, &hv(&row[3]), "led_colour");
    }
    t.report("HUD rows, status and LED boards");
}

// ---------------------------------------------------------------- the order of the hooks
type Log = Arc<Mutex<Vec<String>>>;

/// The default look, noting every hook call as `<its side>.<hook>` (with the ball and the phase where a hook is
/// called per ball), as the `Spy` look of the Python script does.
struct Spy {
    k: usize,
    log: Log,
}

impl Spy {
    fn note(&self, what: &str) {
        self.log.lock().expect("the log").push(format!("{}.{what}", self.k));
    }
}

impl Look for Spy {
    fn setup(&mut self, _p: &PictureCtx, k: usize) {
        self.k = k;
        self.note("setup");
    }
    fn ball(&self, p: &PictureCtx, kk: usize, st: &BallState, t: f64, glass: [f64; 3], roll: f64) -> ([f64; 3], f64) {
        self.note(&format!("ball({kk})"));
        DefaultLook.ball(p, kk, st, t, glass, roll)
    }
    fn ball_sprite<'a>(
        &'a self,
        _p: &'a PictureCtx,
        kk: usize,
        _st: &BallState,
        _t: f64,
        _rp: f64,
        _yaw: f64,
    ) -> Option<BallSprite<'a>> {
        self.note(&format!("ball_sprite({kk})"));
        None
    }
    fn state(&self, _p: &PictureCtx, kk: usize, _st: &mut BallState, _t: f64, phase: Phase, _gl: f64) {
        let phase = match phase {
            Phase::Rev => "rev",
            Phase::Fight => "fight",
            Phase::Tail => "tail",
        };
        self.note(&format!("state({kk},{phase})"));
    }
    fn hides_hp(&self, _p: &PictureCtx, kk: usize, _t: f64) -> bool {
        self.note(&format!("hides_hp({kk})"));
        false
    }
    fn shake(&self, p: &PictureCtx, e: &Event) -> Option<f64> {
        self.note("shake");
        DefaultLook.shake(p, e)
    }
    fn spark(&self, p: &PictureCtx, e: &Event) -> Spark {
        self.note("spark");
        DefaultLook.spark(p, e)
    }
    fn hit_glow(&self, _p: &PictureCtx, _e: &Event) -> Option<(f64, f64)> {
        self.note("hit_glow");
        None
    }
    fn fx_setup(&self, _p: &PictureCtx, _fx: &mut Fx) {
        self.note("fx_setup");
    }
    fn fx_event(&self, _p: &PictureCtx, _fx: &mut Fx, _e: &Event) {
        self.note("fx_event");
    }
    fn fx_after(&self, _p: &PictureCtx, _fx: &mut Fx) {
        self.note("fx_after");
    }
    fn ko_fx(&self, p: &PictureCtx, fx: &mut Fx, pos: V3) {
        self.note("ko_fx");
        DefaultLook.ko_fx(p, fx, pos);
    }
    fn fx_end(&self, _p: &PictureCtx, _fx: &mut Fx) {
        self.note("fx_end");
    }
    fn light(&self, _p: &PictureCtx, _l: &mut Lights, _t: f64) {
        self.note("light");
    }
    fn shadows(&self, _p: &PictureCtx, _st: &BallState, _spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.note("shadows");
        Vec::new()
    }
    fn shadows_free(&self, _p: &PictureCtx, _f: &FrameCtx<'_>, _spot: V3) -> Vec<Vec<(f64, f64)>> {
        self.note("shadows_free");
        Vec::new()
    }
    fn canvas(&self, _p: &PictureCtx, _ic: &mut [V3], _c: &CanvasCtx<'_>) {
        self.note("canvas");
    }
    fn canvas_wave(&self, _p: &PictureCtx, _ic: &mut [V3], _c: &CanvasCtx<'_>, _w: &Wave) {
        self.note("canvas_wave");
    }
    fn canvas_after(&self, _p: &PictureCtx, _ic: &mut [V3], _c: &CanvasCtx<'_>) {
        self.note("canvas_after");
    }
    fn canvas_overlay(&self, _p: &PictureCtx, _f: &FrameCtx<'_>, _arr: &mut Img<f32>) {
        self.note("canvas_overlay");
    }
    fn items_ball(&self, _p: &PictureCtx, _f: &FrameCtx<'_>, _st: &BallState) -> Vec<(f64, Payload)> {
        self.note("items_ball");
        Vec::new()
    }
    fn items_free(&self, _p: &PictureCtx, _f: &FrameCtx<'_>) -> Vec<(f64, Payload)> {
        self.note("items_free");
        Vec::new()
    }
    fn draw_item(&self, _p: &PictureCtx, _f: &FrameCtx<'_>, _fr: &mut Frame, _payload: &Payload) {
        self.note("draw_item");
    }
    fn over_ring(
        &self,
        _p: &PictureCtx,
        _f: &FrameCtx<'_>,
        _fr: &mut Frame,
        _arr: &mut Img<f32>,
        _depth: &mut Img<f32>,
    ) {
        self.note("over_ring");
    }
    fn draw_shard(
        &self,
        _p: &PictureCtx,
        _arr: &mut Img<f32>,
        _cam: &Cam,
        _kind: &str,
        _pos: V3,
        _rot: f64,
        _size: f64,
        _curl: f64,
    ) {
        self.note("draw_shard");
    }
    fn shimmer(&self, _p: &PictureCtx, _img: &mut Pic, _cam: &Cam, _t: f64, _w: &Wave) {
        self.note("shimmer");
    }
    fn shimmer_after(&self, _p: &PictureCtx, _img: &mut Pic, _cam: &Cam, _t: f64) {
        self.note("shimmer_after");
    }
    fn glows(&self, _p: &PictureCtx, _f: &FrameCtx<'_>, _fr: &mut Frame, _drawn: &[DrawnBall]) {
        self.note("glows");
    }
    fn fringe(&self, p: &PictureCtx, e: &Event) -> bool {
        self.note("fringe");
        DefaultLook.fringe(p, e)
    }
    fn fringe_size(&self, p: &PictureCtx, e: &Event) -> (f64, f64) {
        self.note("fringe_size");
        DefaultLook.fringe_size(p, e)
    }
    fn after_grade(&self, _p: &PictureCtx, _img: &mut Pic, _f: &FrameCtx<'_>, _drawn: &[DrawnBall]) {
        self.note("after_grade");
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha(lines: &[String]) -> String {
    hex(&Sha256::digest(lines.join("\n").as_bytes()))
}

/// Every hook is called where the Python calls it: while the picture is built (the camera shake, then the
/// effects in `_fx_build`'s order) and in each of 64 frames spread over every phase of the video, the same
/// calls in the same order for the same ball and phase.
#[test]
fn the_hooks_are_called_in_pythons_order() {
    let log: Log = Arc::default();
    let spy = |k| -> Box<dyn Look> { Box::new(Spy { k, log: log.clone() }) };
    let pic = build(parts(), [spy(0), spy(1)]).expect("the picture builds");
    let take = || std::mem::take(&mut *log.lock().expect("the log"));
    let build_log = take();
    let doc = python();
    let head: Vec<&str> = list(&doc["hooks_build_head"]).iter().map(text).collect();
    assert_eq!(build_log[..head.len()], head[..], "the first hook calls of the build");
    assert_eq!(
        (build_log.len(), sha(&build_log).as_str()),
        (index(&doc["hooks_build"][0]), text(&doc["hooks_build"][1])),
        "the hook calls while the picture is built"
    );
    // one frame's calls in full, for a readable failure
    pic.frame(600);
    let want: Vec<&str> = list(&doc["hooks_600"]).iter().map(text).collect();
    assert_eq!(take(), want, "the hook calls of frame 600");
    let frames = list(&doc["hooks"]);
    assert!(frames.len() >= 60);
    for row in frames {
        let f = index(&row[0]);
        pic.frame(f);
        let got = take();
        assert_eq!(
            (got.len(), sha(&got).as_str()),
            (index(&row[1]), text(&row[2])),
            "the hook calls of frame {f}: {got:?}"
        );
    }
}

/// 64 finished frames, spread over the title card, the count-in, the fight (hits, waves), the K.O., the
/// hit-stop, the shatter, the solo, the reassembly and the glide home, are the bytes the Python kit drew with
/// its base-class look for both fighters.
#[test]
fn the_frames_are_pythons_with_default_looks() {
    let pic = picture();
    let rows = list(&python()["frames"]);
    let next = AtomicUsize::new(0);
    let wrong: Vec<usize> = std::thread::scope(|s| {
        let jobs: Vec<_> = (0..8)
            .map(|_| {
                s.spawn(|| {
                    let mut wrong = Vec::new();
                    while let Some(row) = rows.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let f = index(&row[0]);
                        if hex(&Sha256::digest(&pic.frame(f).data)) != text(&row[1]) {
                            wrong.push(f);
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
    println!("frames: {} of {} equal Python's", rows.len() - wrong.len(), rows.len());
    assert!(wrong.is_empty(), "frames that are not Python's bytes: {wrong:?}");
}

/// The draw log of a frame is `reelkit.ui.DRAWN` after Python's `render`: the same boxes, kinds and texts in the
/// same order, in 64 frames (five of them compared line by line).
#[test]
fn the_draw_log_is_pythons() {
    let pic = picture();
    let doc = python();
    let lines = |f: usize| -> Vec<String> {
        let (_, log) = pic.frame_logged(f);
        log.boxes
            .iter()
            .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
            .collect()
    };
    let full = doc["drawn_full"].as_object().expect("the frames written out");
    assert_eq!(full.len(), 5);
    for (f, want) in full {
        let want: Vec<&str> = list(want).iter().map(text).collect();
        assert_eq!(
            lines(f.parse().expect("a frame number")),
            want,
            "the draw log of frame {f}"
        );
    }
    for row in list(&doc["drawn"]) {
        let f = index(&row[0]);
        let got = lines(f);
        assert_eq!(
            (got.len(), sha(&got).as_str()),
            (index(&row[1]), text(&row[2])),
            "the draw log of frame {f}: {got:?}"
        );
    }
}

/// The focus check's points and circles of confusion are Python's (`kit/checks.py` `focus`), and all zero.
#[test]
fn the_focus_points_are_pythons() {
    let pic = picture();
    let mut t = Tally::default();
    for row in list(&python()["focus"]) {
        let f = index(&row[0]);
        let got = pic.focus_points(f);
        let names: Vec<&str> = list(&row[1]).iter().map(text).collect();
        assert_eq!(got.iter().map(|q| q.1.as_str()).collect::<Vec<_>>(), names, "frame {f}");
        let coc: Vec<f64> = got.iter().map(|q| q.0).collect();
        t.near_all(&coc, &hv(&row[2]), &format!("the circles of confusion of frame {f}"));
        assert!(
            coc.iter().all(|&c| c == 0.0),
            "frame {f}: a subject point is out of focus: {got:?}"
        );
    }
    t.report("focus");
    // every 30th frame: the ring, the balls and the shards stay in focus
    for f in (0..pic.nf()).step_by(30) {
        let worst = pic
            .focus_points(f)
            .into_iter()
            .fold((0.0, String::new()), |m, q| if q.0 > m.0 { q } else { m });
        assert_eq!(worst.0, 0.0, "frame {f}: {} is out of focus", worst.1);
    }
}

/// Bad input is an error in words, not a panic.
#[test]
fn a_picture_that_cannot_be_built_says_why() {
    let message = |r: Result<Picture, PictureError>| match r {
        Ok(_) => panic!("the picture was built"),
        Err(e) => e.to_string(),
    };
    // the roster entries the wrong way round
    let mut q = parts();
    q.cfg.swap(0, 1);
    assert_eq!(
        message(build(q, default_looks())),
        "the roster entry given for fighter 0 is TROMBONE's, not FLUTE's"
    );
    // a recording without the rings
    let mut q = parts();
    let mut rec = (*q.rec).clone();
    rec.arrays.remove("WAVES");
    q.rec = Arc::new(rec);
    assert_eq!(
        message(build(q, default_looks())),
        "the recording has no WAVES list with one entry a frame"
    );
    // a hit without what hit
    let mut q = parts();
    let mut rec = (*q.rec).clone();
    let at = rec
        .events
        .iter()
        .position(|e| e.kind == "hit")
        .expect("the fight has a hit");
    rec.events[at].fields.truncate(4);
    let when = rec.events[at].t;
    q.rec = Arc::new(rec);
    assert_eq!(
        message(build(q, default_looks())),
        format!("event {at} (hit, at {when:.3} s) is not [t, 'hit', attacker 0 or 1, victim, damage, x, y, what hit]")
    );
    // a show of another fight
    let mut q = parts();
    q.show.t_ko += 1.0;
    assert_eq!(
        message(build(q, default_looks())),
        "the show was not made from this recording (its K.O. differs)"
    );
    // a model without faces
    let q = parts();
    let meshes = [mesh(&q.cfg[0].pp), Mesh { faces: Vec::new() }];
    let r = Picture::new(q.rec, &q.ff, q.show, [&q.cfg[0], &q.cfg[1]], meshes, default_looks());
    assert_eq!(message(r), "TROMBONE's model has no faces");
}

/// `upscale3` is nearest neighbour: every pixel becomes a 3 x 3 block.
#[test]
fn upscale3_repeats_every_pixel() {
    let small: Img<u8> = Img::from_vec(2, 2, 3, (0..12).collect());
    let big = super::upscale3(&small);
    assert_eq!((big.w, big.h, big.c), (6, 6, 3));
    for y in 0..6 {
        for x in 0..6 {
            assert_eq!(big.px(x, y), small.px(x / 3, y / 3), "({x}, {y})");
        }
    }
}
