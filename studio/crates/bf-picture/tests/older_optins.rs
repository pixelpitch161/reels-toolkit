//! The picture opt-ins of the older looks (plan m4, Task 4) against the kit's renderer patched at run time
//! (`tools/studio_export/older/picture_patches.py`): `studio/reference/older/vectors/optins.json`, export
//! `older_vectors`.
//!
//! - **Off.** A look that uses none of them draws what it drew before: the example episode with the default look for
//!   both fighters gives the frames and draw logs the patched kit gives with its base-class look (which the export
//!   checked against `picture_ring/unit.json`, the kit before the patches). Every milestone 1 and 3 gate (golden
//!   frames, each fighter's look frames, the title cards) is the rest of this proof: none of them changed.
//! - **On.** The example episode with the default look for FLUTE and [`StubLook`] for TROMBONE, a look that uses every
//!   opt-in with arithmetic written out from the Python stub (`older_vectors.py` `stub_look`): every frame's bytes
//!   and draw log are the patched kit's, and the rope events after the stub's are the kit's, value for value.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use bf_core::fightfile::FightFile;
use bf_core::py::fmod_py;
use bf_core::recording::Recording;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_core::show::Show;
use bf_picture::camera::V3;
use bf_picture::config::{H, W};
use bf_picture::img::Img;
use bf_picture::look::{CanvasCtx, DefaultLook, DrawnBall, FrameCtx, GlassPx, Look, Tint};
use bf_picture::ring::ropes::RopeEv;
use bf_picture::ring::state::BallState;
use bf_picture::ring::{Picture, PictureCtx};
use bf_picture::sprite::Mesh;
use bf_picture::ui::{DrawLog, Kind};
use bf_testdata::{reference, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The opt-ins, in the order `picture_patches.py` registers them.
const OPTINS: [&str; 10] = [
    "canvas_albedo",
    "canvas_paint",
    "ball_tint",
    "ball_glass",
    "ball_lit",
    "ball_after_sprite",
    "ball_late",
    "rope_press",
    "rope_events",
    "after_post",
];

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

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The example episode with the given looks.
fn build(looks: [Box<dyn Look>; 2]) -> Picture {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
    let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
    let meshes = [mesh(&cfg[0].pp), mesh(&cfg[1].pp)];
    Picture::new(Arc::new(rec), &ff, show, [&cfg[0], &cfg[1]], meshes, looks).expect("the picture builds")
}

fn doc() -> &'static Value {
    static DOC: OnceLock<Value> = OnceLock::new();
    DOC.get_or_init(|| {
        let path = reference("older/vectors/optins.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e} (python -m tools.studio_export older_vectors)",
                path.display()
            )
        });
        let d: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(d["tool"], "older_vectors");
        d
    })
}

/// The bytes' and the draw log's SHA-256 of frame `f`, as the export writes them: `[f, sha, boxes, boxes sha]`.
fn row(pic: &Picture, f: usize) -> (String, usize, String) {
    let (img, log) = pic.frame_logged(f);
    let lines: Vec<String> = log
        .boxes
        .iter()
        .map(|b| format!("{},{},{},{},{},{}", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
        .collect();
    (
        hex(&Sha256::digest(&img.data)),
        lines.len(),
        hex(&Sha256::digest(lines.join("\n").as_bytes())),
    )
}

/// Every row of `rows` against `pic`, on a few threads: the frames that differ.
fn wrong_rows(pic: &Picture, rows: &[Value]) -> Vec<String> {
    let rows: Vec<(usize, String, usize, String)> = rows
        .iter()
        .map(|r| {
            (
                r[0].as_u64().expect("a frame") as usize,
                r[1].as_str().expect("a hash").to_string(),
                r[2].as_u64().expect("a count") as usize,
                r[3].as_str().expect("a hash").to_string(),
            )
        })
        .collect();
    let chunks: Vec<&[(usize, String, usize, String)]> = rows.chunks(rows.len().div_ceil(4).max(1)).collect();
    std::thread::scope(|s| {
        let jobs: Vec<_> = chunks
            .iter()
            .map(|chunk| {
                s.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|(f, sha, n, boxes)| {
                            let got = row(pic, *f);
                            (got != (sha.clone(), *n, boxes.clone())).then(|| format!("frame {f}: {got:?}"))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        jobs.into_iter()
            .flat_map(|j| j.join().expect("a frame panicked"))
            .collect()
    })
}

/// How many times each opt-in of [`StubLook`] changed something (the kit's stub counts the same calls).
static CALLS: [AtomicUsize; 10] = [const { AtomicUsize::new(0) }; 10];

fn hit(name: &str) {
    let i = OPTINS.iter().position(|n| *n == name).expect("an opt-in");
    CALLS[i].fetch_add(1, Ordering::Relaxed);
}

/// `older_vectors.py` `stub_look`: a look that uses every opt-in on side `k` (its own ball, or the other's), its
/// arithmetic written out from the Python (numpy's dtypes: `float64` for the canvas, the glass and the colours,
/// `float32` for the lit sprite).
#[derive(Default)]
struct StubLook {
    k: usize,
}

impl Look for StubLook {
    fn setup(&mut self, _p: &PictureCtx, k: usize) {
        self.k = k;
    }

    fn canvas_albedo(&self, _p: &PictureCtx, alb: &mut [V3], c: &CanvasCtx<'_>) {
        // chk = (np.floor(C.sxp * 2.0) + np.floor(C.szp * 2.0)) % 2 == 0; out[chk] = out[chk] * 0.75
        for (j, a) in alb.iter_mut().enumerate() {
            if fmod_py((c.sxp[j] * 2.0).floor() + (c.szp[j] * 2.0).floor(), 2.0) == 0.0 {
                *a = a.map(|v| v * 0.75);
            }
        }
        hit("canvas_albedo");
    }

    fn canvas_paint(&self, _p: &PictureCtx, col: &mut [V3], ic: &[V3], c: &CanvasCtx<'_>) {
        let p = c.fs[self.k].p;
        let tone = [60.0, 20.0, 90.0];
        let mut any = false;
        for (j, cv) in col.iter_mut().enumerate() {
            let (dx, dz) = (c.sxp[j] - p[0], c.szp[j] - p[1]);
            if dx * dx + dz * dz < 1.0 {
                any = true;
                *cv = std::array::from_fn(|ch| cv[ch] * 0.5 + tone[ch] * ic[j][ch]);
            }
        }
        if any {
            hit("canvas_paint");
        }
    }

    fn ball_tint(&self, _p: &PictureCtx, kk: usize, _st: &BallState, _t: f64, col: V3) -> V3 {
        if kk == self.k {
            return col;
        }
        hit("ball_tint");
        let to = [100.0, 100.0, 120.0];
        std::array::from_fn(|ch| col[ch] * 0.5 + to[ch] * 0.5)
    }

    fn ball_glass(&self, _p: &PictureCtx, kk: usize, _st: &BallState, _t: f64, px: &GlassPx, inner: &mut V3) {
        if kk != self.k {
            return;
        }
        // (called pixel by pixel; the kit's stub counts its whole-patch calls)
        hit("ball_glass");
        if fmod_py((px.dx * 4.0 + px.dy * 2.0).floor(), 2.0) == 0.0 {
            let red = [200.0, 40.0, 40.0];
            *inner = std::array::from_fn(|ch| inner[ch] * 0.6 + red[ch] * 0.4);
        }
    }

    fn ball_lit<'a>(&'a self, _p: &'a PictureCtx, kk: usize, _st: &BallState, _t: f64) -> Option<Tint<'a>> {
        if kk == self.k {
            return None;
        }
        hit("ball_lit");
        // lit * 0.5 + 20: a float32 array times a Python float, plus a Python int, stays float32
        Some(Box::new(|lit: &mut Img<f32>| {
            for v in &mut lit.data {
                *v = *v * 0.5f32 + 20.0f32;
            }
        }))
    }

    fn ball_after_sprite(
        &self,
        _p: &PictureCtx,
        kk: usize,
        _st: &BallState,
        _t: f64,
        arr: &mut Img<f32>,
        s: (f64, f64),
        rp: f64,
    ) {
        if kk != self.k {
            return;
        }
        // y = int(s[1]); xa, xb = max(0, int(s[0] - rp)), min(W, int(s[0] + rp))
        let y = s.1 as i64;
        let (xa, xb) = (((s.0 - rp) as i64).max(0), ((s.0 + rp) as i64).min(W as i64));
        if 0 <= y && y < H as i64 && xb > xa {
            for x in xa..xb {
                arr.px_mut(x as usize, y as usize).copy_from_slice(&[255.0, 0.0, 255.0]);
            }
            hit("ball_after_sprite");
        }
    }

    fn ball_late(&self, _p: &PictureCtx, kk: usize, f: &FrameCtx<'_>, _st: &BallState) -> bool {
        let late = kk == self.k && f.f % 2 == 1;
        if late {
            hit("ball_late");
        }
        late
    }

    fn rope_press(&self, p: &PictureCtx, _s: usize, _y: f64, t: f64, pts: &[[f64; 2]], yy: &mut [f64]) {
        let c = p.fighter(self.k, t).p;
        let mut any = false;
        for (q, h) in pts.iter().zip(yy.iter_mut()) {
            let (dx, dz) = (q[0] - c[0], q[1] - c[1]);
            if dx * dx + dz * dz < 2.25 {
                any = true;
                *h = bf_picture::np::maximum(*h - 0.25, 0.05);
            }
        }
        if any {
            hit("rope_press");
        }
    }

    fn rope_events(&self, p: &PictureCtx) -> Vec<RopeEv> {
        // (e[0] + 0.4, SIDE_OF[(e[3], e[4])], e[5], e[6], 3.2) for the stub's own ball's rope events
        let side = |axis: i64, sign: i64| match (axis, sign) {
            (1, 1) => 0,
            (0, 1) => 1,
            (1, -1) => 2,
            (0, -1) => 3,
            other => panic!("no rope side {other:?}"),
        };
        let out: Vec<RopeEv> = p
            .rec
            .events
            .iter()
            .filter(|e| e.kind == "rope" && e.i(2) == self.k as i64)
            .map(|e| RopeEv {
                t: e.t + 0.4,
                side: side(e.i(3), e.i(4)),
                x: e.f(5),
                z: e.f(6),
                spd: 3.2,
            })
            .collect();
        CALLS[8].store(out.len(), Ordering::Relaxed);
        out
    }

    fn after_post(
        &self,
        _p: &PictureCtx,
        img: &mut Img<u8>,
        log: &mut DrawLog,
        _f: &FrameCtx<'_>,
        drawn: &[DrawnBall],
    ) {
        for d in drawn.iter().filter(|d| d.k == self.k) {
            let (x0, y0) = (d.s.0 as i64 - 4, d.s.1 as i64 - 4);
            if 0 <= x0 && x0 + 9 <= W as i64 && 0 <= y0 && y0 + 9 <= H as i64 {
                for y in y0..y0 + 9 {
                    for x in x0..x0 + 9 {
                        for v in img.px_mut(x as usize, y as usize) {
                            *v = 255 - *v;
                        }
                    }
                }
                log.note(
                    x0 as i32,
                    y0 as i32,
                    x0 as i32 + 9,
                    y0 as i32 + 9,
                    Kind::Label,
                    "stub after_post",
                );
                hit("after_post");
            }
        }
    }
}

fn stub_picture() -> &'static Picture {
    static PICTURE: OnceLock<Picture> = OnceLock::new();
    PICTURE.get_or_init(|| {
        let side = doc()["stub_side"].as_u64().expect("the stub's side") as usize;
        let mut looks: [Box<dyn Look>; 2] = [Box::new(DefaultLook), Box::new(DefaultLook)];
        looks[side] = Box::new(StubLook::default());
        build(looks)
    })
}

#[test]
fn the_opt_ins_are_the_kits() {
    let names: Vec<&str> = doc()["optins"]
        .as_array()
        .expect("the opt-ins")
        .iter()
        .map(|v| v.as_str().expect("a name"))
        .collect();
    assert_eq!(names, OPTINS);
}

/// Off: with no opt-in used, the picture is the kit's (the default look for both fighters, the frames of the opt-in
/// test).
#[test]
fn a_look_without_opt_ins_draws_as_before() {
    let pic = build([Box::new(DefaultLook), Box::new(DefaultLook)]);
    let rows = doc()["on"]["base_rows"].as_array().expect("the base rows");
    assert!(rows.len() >= 20, "{} frames", rows.len());
    let wrong = wrong_rows(&pic, rows);
    assert!(wrong.is_empty(), "frames that are not the kit's: {wrong:?}");
}

/// On: the stub's frames and draw logs are the patched kit's, every opt-in changed something, and the frames are not
/// the off frames.
#[test]
fn every_opt_in_draws_as_the_kits() {
    let pic = stub_picture();
    let on = &doc()["on"];
    let rows = on["rows"].as_array().expect("the rows");
    let wrong = wrong_rows(pic, rows);
    assert!(wrong.is_empty(), "frames that are not the kit's: {wrong:?}");
    let base = on["base_rows"].as_array().expect("the base rows");
    let changed = rows.iter().zip(base).filter(|(a, b)| a[1] != b[1]).count();
    assert!(
        changed * 4 >= rows.len() * 3,
        "the stub changed only {changed} of {} frames",
        rows.len()
    );
    for (i, name) in OPTINS.iter().enumerate() {
        assert!(
            CALLS[i].load(Ordering::Relaxed) > 0 && on["calls"][name].as_u64().unwrap_or(0) > 0,
            "opt-in {name} never changed anything"
        );
    }
}

/// The rope events after the stub's are the kit's `ROPE_EV`, value for value, in its order.
#[test]
fn the_rope_events_are_the_kits() {
    let pic = stub_picture();
    let want = doc()["on"]["rope_ev"].as_array().expect("the rope events");
    let got = &pic.ctx().rope_ev;
    assert_eq!(got.len(), want.len());
    assert_eq!(got.len() as u64, doc()["on"]["n_rope_ev"].as_u64().expect("a count"));
    let h = |v: &Value| -> f64 {
        let s = v.as_str().expect("a hex float");
        bf_testdata::hex_arg(&Value::String(s.to_string()))
    };
    for (j, (g, w)) in got.iter().zip(want).enumerate() {
        let w = (
            h(&w[0]),
            w[1].as_u64().expect("a side") as usize,
            h(&w[2]),
            h(&w[3]),
            h(&w[4]),
        );
        assert_eq!(
            (g.t.to_bits(), g.side, g.x.to_bits(), g.z.to_bits(), g.spd.to_bits()),
            (w.0.to_bits(), w.1, w.2.to_bits(), w.3.to_bits(), w.4.to_bits()),
            "rope event {j}"
        );
    }
}
