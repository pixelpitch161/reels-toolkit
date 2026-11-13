//! VIOLA's three picture opt-ins (plan m4, Task 11: `crowd_laugh`, `flash_hit`, `spark_events`) against the kit's
//! renderer patched at run time (`tools/studio_export/older/picture_patches.py`, its VIOLA section):
//! `studio/reference/older/vectors/optins_viola.json`, export `older_vectors_viola`.
//!
//! - **Off.** The default look draws what it drew before: the example episode with the default look for both fighters
//!   gives the patched kit's frames and draw logs with its base-class look (the export checked, with all thirteen
//!   patches installed, that the base-class looks still draw `picture_ring/unit.json` and that three kit episodes draw
//!   the same frames patched and unpatched). Every milestone 1 and 3 frame gate is the rest of this proof.
//! - **On.** The example episode with [`StubLook`] on the TROMBONE's side, a look that uses the three with arithmetic
//!   written out from the Python stub (`older_vectors_viola.py` `stub_look`): every frame's bytes and draw log are the
//!   patched kit's, and the flashbulb list is as long.

use std::sync::{Arc, OnceLock};

use bf_core::event::Event;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_core::show::Show;
use bf_picture::look::{DefaultLook, Look};
use bf_picture::ring::{Picture, PictureCtx};
use bf_picture::sprite::Mesh;
use bf_testdata::{reference, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

const OPTINS: [&str; 3] = ["crowd_laugh", "flash_hit", "spark_events"];

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
        let path = reference("older/vectors/optins_viola.json");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "cannot read {}: {e} (python -m tools.studio_export older_vectors_viola)",
                path.display()
            )
        });
        let d: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(d["tool"], "older_vectors_viola");
        d
    })
}

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

/// The Python stub, written out: a laugh rising from `laugh[0]` over 0.5 s and falling from `laugh[1]` over 0.5 s,
/// every one of its hits flashing, the extra spark kinds.
struct StubLook {
    laugh: (f64, f64),
    sparks: Vec<&'static str>,
}

impl Look for StubLook {
    fn crowd_laugh(&self, _p: &PictureCtx, t: f64) -> Option<f64> {
        let (a, b) = self.laugh;
        if t < a || t >= b + 0.5 {
            return Some(0.0);
        }
        Some(if t < b { bf_core::py::py_min(1.0, (t - a) / 0.5) } else { 1.0 - (t - b) / 0.5 })
    }

    fn flash_hit(&self, _e: &Event) -> Option<bool> {
        Some(true)
    }

    fn spark_events(&self) -> &[&str] {
        &self.sparks
    }
}

fn stub_picture() -> &'static Picture {
    static PICTURE: OnceLock<Picture> = OnceLock::new();
    PICTURE.get_or_init(|| {
        let d = doc();
        let side = d["stub_side"].as_u64().expect("the stub's side") as usize;
        let laugh = d["laugh"].as_array().expect("the laugh");
        let kinds: Vec<&'static str> = d["spark_kinds"]
            .as_array()
            .expect("the spark kinds")
            .iter()
            .map(|v| &*Box::leak(v.as_str().expect("a kind").to_string().into_boxed_str()))
            .collect();
        let stub = StubLook {
            laugh: (laugh[0].as_f64().expect("a time"), laugh[1].as_f64().expect("a time")),
            sparks: kinds,
        };
        let mut looks: [Box<dyn Look>; 2] = [Box::new(DefaultLook), Box::new(DefaultLook)];
        looks[side] = Box::new(stub);
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

/// Off: with no opt-in used, the picture is the kit's (the default look for both fighters, the on-test's frames).
#[test]
fn a_look_without_viola_opt_ins_draws_as_before() {
    let pic = build([Box::new(DefaultLook), Box::new(DefaultLook)]);
    let rows = doc()["on"]["base_rows"].as_array().expect("the base rows");
    assert!(rows.len() >= 15, "{} frames", rows.len());
    let wrong = wrong_rows(&pic, rows);
    assert!(wrong.is_empty(), "frames that are not the kit's: {wrong:?}");
    assert_eq!(pic.ctx().flash_t.len() as u64, doc()["on"]["base_n_flash"].as_u64().expect("a count"));
}

/// On: the stub's frames and draw logs are the patched kit's, and they are not the off frames.
#[test]
fn the_viola_opt_ins_draw_as_the_kits() {
    let pic = stub_picture();
    let on = &doc()["on"];
    let rows = on["rows"].as_array().expect("the rows");
    let wrong = wrong_rows(pic, rows);
    assert!(wrong.is_empty(), "frames that are not the kit's: {wrong:?}");
    let changed = on["changed"].as_array().expect("the changed frames").len();
    assert!(changed * 2 >= rows.len(), "the stub changed only {changed} of {} frames", rows.len());
    assert_eq!(pic.ctx().flash_t.len() as u64, on["n_flash"].as_u64().expect("a count"));
    assert!(on["n_flash"].as_u64() > on["base_n_flash"].as_u64(), "flash_hit added no flashbulbs");
}
