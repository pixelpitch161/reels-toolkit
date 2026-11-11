//! The VS title card against Python: `bf_picture::title::TitleCard` on the cards of
//! `studio/reference/frames/flute_vs_trombone/title/` (written by the export tool's `title`).
//!
//! Each card there is `title.draw(noise, t)` over one fixed seeded-noise image; `draw` here must give the same
//! picture, every byte. `title.json` is the index: the test walks it, fails on a call it does not know and on a
//! folder that does not hold the 18 cards the export picks by rule (each word's slam, the hold, the doors opening
//! and closing). The index also carries `title.door_offset(t)` and `title.shake(t)` for each card, compared here
//! bit for bit. (Python takes their `exp`, `sin` and `pow` from the C runtime of the reference PC and the Studio
//! from `bf_core::dmath`, and the two do not agree on every argument; on these 18 frames they do. On other
//! frames of the card a few of these values differ from Python's by one to three units in the last place, with
//! no pixel changed: measured on every card frame of seven timelines, not part of the reference set.)
//!
//! Every other frame on which the card is drawn is pinned by the SHA-256 of its bytes (`title/hashes.npz`): all
//! 61 of the episode, and every card frame of three other timelines of the same two fighters (swapped, 96 BPM
//! without a tag, 200 BPM), 249 in all.
//!
//! What the card takes from the two fighters' looks (`bf_picture::title::Look`) is fed as the Python kit had it
//! for this episode. The TROMBONE's look has none of the three hooks: its panel pattern is the card's own (by
//! name) and its close-up is its model. The FLUTE's look has all three, and that look is not ported yet, so
//! [`FluteStandIn`] stands in for it: the close-up is the sprite Python drew (the reference set holds it), the
//! glass tint is the look's constant, and the panel pattern is the ten lines of `flute_keys`, written out below.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
use bf_core::py;
use bf_core::recording::Recording;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_core::show::Show;
use bf_picture::img::{Img, RgbF};
use bf_picture::np;
use bf_picture::sprite::Mesh;
use bf_picture::title::{self, CardLook, Look, TitleCard};
use bf_picture::ui::{DrawLog, Kind};
use bf_testdata::{Vectors, diff_u8, hex_arg, read_png, reference, repo_root, write_png_rgb};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FOLDER: &str = "frames/flute_vs_trombone/title";
const FPS: f64 = 30.0;

// ---------------------------------------------------------------- the episode and its looks
/// The TROMBONE's look as far as the card goes: every default.
struct Plain;

impl Look for Plain {}

/// A stand-in for the FLUTE's look (`kit/fighters/flute.py`, class `Look`) as far as the card goes.
struct FluteStandIn {
    /// `flute_sprite(44, 0.0, 32)` as Python drew it: the `title` case of `sprites/FLUTE.npz`.
    closeup: Img<u8>,
}

impl Look for FluteStandIn {
    /// `title_pattern = staticmethod(flute_keys)`: rows of key cups along the divider.
    fn title_pattern(&self, base: &mut RgbF) -> bool {
        let dd = title::layout::dd();
        let side = title::layout::side();
        for y in 0..base.h {
            for x in 0..base.w {
                // u = (TT.XX * TT.DD[0] + TT.YY * TT.DD[1]) / 22.0;  v = TT.SIDE / 30.0
                let u = (x as f64 * dd[0] + y as f64 * dd[1]) / 22.0;
                let v = side.px(x, y)[0] / 30.0;
                let (cu, cv) = (py::fmod_py(u, 1.0) - 0.5, py::fmod_py(v, 1.0) - 0.5);
                // d = np.hypot(cu * 22.0, cv * 30.0): numpy's hypot, not math.hypot
                let d = np::hypot(cu * 22.0, cv * 30.0);
                let (cup, hole) = (d < 6.5, d < 3.0);
                for (p, tint) in base.px_mut(x, y).iter_mut().zip([0.70, 0.72, 0.80]) {
                    // base_ = base_ * np.array([0.70, 0.72, 0.80])
                    let mut c = *p * tint;
                    // np.where(cup, base_ * 1.07 + 10, base_ * 0.95), then np.where(hole, base_ * 0.62, base_)
                    c = if cup { c * 1.07 + 10.0 } else { c * 0.95 };
                    *p = if hole { c * 0.62 } else { c };
                }
            }
        }
        true
    }

    /// `title_closeup(r)`: `flute_sprite(r, 0.0, view roll)`.
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        assert_eq!(r, 44, "the card asks for its close-ups at radius int(56 * 0.8)");
        Some(self.closeup.clone())
    }

    /// `glass = (150, 160, 196)`: the cool, darker glass.
    fn glass(&self) -> [f64; 3] {
        [150.0, 160.0, 196.0]
    }
}

fn json(path: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

/// The roster entry of `name`, read by path (`bf-picture` does not depend on `bf-content`).
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

/// The yaw and roll of the fighter's view, `CFG[k]['view']` = `(yc, sw, roll)`.
fn view(cfg: &FighterCfg) -> (f64, f64) {
    let v = cfg.raw["view"].as_array().expect("the roster entry has a view");
    assert_eq!(v.len(), 3, "a view is (yaw, swing, roll)");
    (v[0].as_f64().expect("the yaw"), v[2].as_f64().expect("the roll"))
}

/// The FLUTE's close-up as `title.big_sprite(0)` returned it: the one `title` case of `sprites/FLUTE.npz`.
fn flute_closeup() -> Img<u8> {
    let v = Vectors::open("sprites/FLUTE.npz");
    let cases: Vec<_> = v.index.cases.iter().filter(|c| c.args["group"] == "title").collect();
    assert_eq!(cases.len(), 1, "sprites/FLUTE.npz holds one title close-up");
    let case = cases[0];
    assert_eq!(case.call, "title.big_sprite(k)");
    assert_eq!(
        (&case.args["drawn_by"], &case.args["radius"]),
        (&Value::from("closeup"), &Value::from(44))
    );
    let a = v.array(case, "out");
    assert_eq!(a.shape, [93, 93, 4]);
    Img::from_vec(93, 93, 4, a.u8().to_vec())
}

/// The example episode's show and its card, built once for the whole test binary.
struct Episode {
    show: Show,
    card: TitleCard,
}

fn episode() -> &'static Episode {
    static EPISODE: OnceLock<Episode> = OnceLock::new();
    EPISODE.get_or_init(|| {
        let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
        let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let ff = FightFile::parse(&text).expect("the fight file parses");
        let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
        assert_eq!(show.names, ["FLUTE", "TROMBONE"]);
        let card = card_of(&show, ff.series_tag.as_deref());
        assert_eq!(ff.series_tag.as_deref(), Some("BALLFIGHT #15"));
        Episode { show, card }
    })
}

/// The card of `show` (its timeline and names) with the looks the Python kit has for the two fighters: the
/// FLUTE's stand-in and the TROMBONE's defaults, on either side.
fn card_of(show: &Show, series_tag: Option<&str>) -> TitleCard {
    let cfg = [fighter(&show.names[0]), fighter(&show.names[1])];
    let meshes = [mesh(&cfg[0].pp), mesh(&cfg[1].pp)];
    let flute = FluteStandIn {
        closeup: flute_closeup(),
    };
    let looks: [&dyn Look; 2] = [0, 1].map(|k| -> &dyn Look {
        match show.names[k].as_str() {
            "FLUTE" => &flute,
            "TROMBONE" => &Plain,
            other => panic!("no look for {other} in this test"),
        }
    });
    TitleCard::new(
        show,
        // COL[k] = np.array(c['color'], float), DARK[k] = np.array(c['dark'], float)
        [cfg[0].color.map(f64::from), cfg[1].color.map(f64::from)],
        [cfg[0].dark.map(f64::from), cfg[1].dark.map(f64::from)],
        [0, 1].map(|k| {
            let (yaw, roll) = view(&cfg[k]);
            CardLook {
                look: looks[k],
                mesh: &meshes[k],
                yaw,
                roll,
            }
        }),
        series_tag,
    )
}

/// The seeded-noise image every card is drawn over.
fn noise() -> Img<u8> {
    let (w, h, c, data) = read_png(&reference(&format!("{FOLDER}/noise.png")));
    assert_eq!((w, h, c), (360, 640, 3), "noise.png");
    Img::from_vec(w, h, c, data)
}

fn index() -> Value {
    let index = json(&reference(&format!("{FOLDER}/title.json")));
    assert_eq!(index["tool"], "title");
    assert_eq!(index["episode"], "flute_vs_trombone");
    assert_eq!(index["size"], serde_json::json!([360, 640]));
    assert_eq!((&index["mode"], &index["fps"]), (&Value::from("RGB"), &Value::from(30)));
    index
}

/// The frame and time of a case; `t` must be `frame / 30` to the bit.
fn when(case: &Value) -> (usize, f64) {
    let frame = case["args"]["frame"].as_u64().expect("args.frame") as usize;
    let t = hex_arg(&case["args"]["t"]);
    assert_eq!(t.to_bits(), (frame as f64 / FPS).to_bits(), "frame {frame}");
    (frame, t)
}

// ---------------------------------------------------------------- the cards
/// How the floats of the index compare with the port's: how many, and how many equal to the bit.
#[derive(Default)]
struct Floats {
    total: usize,
    exact: usize,
}

impl Floats {
    /// Counts one value; one that is not Python's to the bit is a failure, reported with the distance.
    fn add(&mut self, got: f64, want: &Value, what: &str, failures: &mut Vec<String>) {
        let want = hex_arg(want);
        self.total += 1;
        if got.to_bits() == want.to_bits() {
            self.exact += 1;
        } else {
            let d = (got - want).abs();
            failures.push(format!("  {what}: got {got:e}, Python {want:e}: {d:e} apart"));
        }
    }
}

#[test]
fn every_card_matches_python() {
    let ep = episode();
    let index = index();
    let cases = index["cases"].as_array().expect("title.json has cases");
    assert_eq!(cases.len(), 18, "the title folder holds 18 cards");
    let noise = noise();
    let mut failures = Vec::new();
    let (mut passed, mut rules) = (0, BTreeSet::new());
    let (mut offsets, mut shakes) = (Floats::default(), Floats::default());
    for case in cases {
        let id = case["id"].as_str().expect("id");
        let file = case["file"].as_str().expect("file");
        if case["call"] != "title.draw(noise, t)" {
            failures.push(format!("  {id}: a call this test does not know: {}", case["call"]));
            continue;
        }
        let (frame, t) = when(case);
        assert_eq!(file, format!("f{frame:04}.png"), "{id}");
        rules.insert(case["rule"].as_str().expect("rule").to_string());
        let (w, h, c, want) = read_png(&reference(&format!("{FOLDER}/{file}")));
        assert_eq!((w, h, c), (360, 640, 3), "{file}");
        let mut img = noise.clone();
        ep.card.draw(&mut img, t);
        if img.data == want {
            passed += 1;
        } else {
            let out = repo_root().join("studio/target/title_frames").join(file);
            write_png_rgb(&out, w, h, &img.data);
            failures.push(format!(
                "  {id} {file} ({}): {}; the card as drawn is in {}",
                case["reason"].as_str().unwrap_or(""),
                diff_u8(&img.data, &want, w, h, c),
                out.display()
            ));
        }
        // title.door_offset(t) and title.shake(t)
        let expect = &case["expect"];
        offsets.add(
            ep.card.door_offset(t),
            &expect["door_offset"],
            &format!("{id} door_offset"),
            &mut failures,
        );
        let (sx, sy) = ep.card.shake(t);
        shakes.add(sx, &expect["shake"][0], &format!("{id} shake x"), &mut failures);
        shakes.add(sy, &expect["shake"][1], &format!("{id} shake y"), &mut failures);
    }
    println!("title.draw: {passed}/{}", cases.len());
    println!("door_offset: {}/{} bit for bit", offsets.exact, offsets.total);
    println!("shake: {}/{} bit for bit", shakes.exact, shakes.total);
    assert!(
        failures.is_empty(),
        "title.draw: {passed}/{}\n{} failure(s):\n{}",
        cases.len(),
        failures.len(),
        failures.join("\n")
    );
    // The cards the export picks by rule are all there: three per word, the hold, four of each door movement.
    let mut want: BTreeSet<String> = ["hold".to_string()].into();
    for j in 0..3 {
        want.extend([0, 2, 5].map(|d| format!("slam_{j}_{d}")));
    }
    for n in 0..4 {
        want.extend([format!("opening_{n}"), format!("closing_{n}")]);
    }
    assert_eq!(rules, want);
}

#[test]
fn the_noise_under_the_cards_is_not_flat() {
    // The cards prove nothing about the arena under the card if the image they are drawn over is flat.
    let noise = noise();
    let mut seen = [false; 256];
    for &v in &noise.data {
        seen[usize::from(v)] = true;
    }
    assert!(seen.iter().all(|&s| s), "noise.png holds every byte value");
}

// ---------------------------------------------------------------- every card frame, by its hash
const HASHES: &str = "frames/flute_vs_trombone/title/hashes.npz";

/// One timeline of `hashes.npz`: its name, the card frames Python listed and their SHA-256 digests.
struct Pinned {
    timeline: String,
    frames: Vec<usize>,
    digests: Vec<[u8; 32]>,
}

/// The timeline a case of `hashes.npz` describes, built here: the episode's show with the case's fighters and
/// tempo, checked against the case's timeline values to the bit, and the card with the case's tag.
fn pinned_card(case: &bf_testdata::Case) -> (Show, TitleCard) {
    let args = &case.args;
    let names: Vec<String> = args["fighters"]
        .as_array()
        .expect("args.fighters")
        .iter()
        .map(|v| v.as_str().expect("a name").to_string())
        .collect();
    let bpm = args["bpm"].as_f64().expect("args.bpm");
    let mut show = episode().show.clone();
    show.grid = Grid::new(bpm);
    show.names = [names[0].clone(), names[1].clone()];
    let g = &show.grid;
    let tv = &args["timeline_values"];
    let same = |got: f64, want: &Value, what: &str| {
        assert_eq!(got.to_bits(), hex_arg(want).to_bits(), "{}: {what}", case.id);
    };
    same(g.bpm, &tv["bpm"], "bpm");
    same(g.beat, &tv["beat"], "beat");
    same(g.bar, &tv["bar"], "bar");
    same(g.dur, &tv["dur"], "dur");
    same(g.t_close, &tv["t_close"], "t_close");
    same(g.t_doors.0, &tv["t_doors"][0], "t_doors[0]");
    same(g.t_doors.1, &tv["t_doors"][1], "t_doors[1]");
    for j in 0..3 {
        same(g.title[j], &tv["title"][j], "title");
    }
    assert_eq!(Value::from(g.nf), tv["nf"], "{}: nf", case.id);
    let card = card_of(&show, args["series"].as_str());
    (show, card)
}

#[test]
fn every_card_frame_matches_its_hash() {
    let v = Vectors::open(HASHES);
    assert_eq!(v.index.tool, "title");
    assert_eq!(v.index.raw["noise"]["file"], "noise.png");
    let noise = noise();
    let cases = &v.index.cases;
    let names: Vec<&str> = cases
        .iter()
        .map(|c| c.args["timeline"].as_str().expect("args.timeline"))
        .collect();
    assert_eq!(names, ["episode", "swapped", "bpm96", "bpm200"]);
    let pinned: Vec<(Pinned, &bf_testdata::Case)> = cases
        .iter()
        .map(|case| {
            assert_eq!(
                case.call, "hashlib.sha256(title.draw(noise, frame / 30).tobytes()).digest()",
                "{}: a call this test does not know",
                case.id
            );
            let frames = v.array(case, "frames");
            let sha = v.array(case, "sha256");
            let n = frames.len();
            assert_eq!(
                (frames.shape.as_slice(), sha.shape.as_slice()),
                (&[n][..], &[n, 32][..]),
                "{}",
                case.id
            );
            assert_eq!(Value::from(n), case.args["cards"], "{}", case.id);
            let p = Pinned {
                timeline: case.args["timeline"].as_str().expect("timeline").to_string(),
                frames: frames
                    .i32()
                    .iter()
                    .map(|&f| usize::try_from(f).expect("a frame"))
                    .collect(),
                digests: sha.u8().chunks(32).map(|d| d.try_into().expect("32 bytes")).collect(),
            };
            (p, case)
        })
        .collect();
    // The episode's digests are those of the 18 PNG cards, where both have a frame.
    let ep = &pinned[0].0;
    let mut checked = 0;
    for case in index()["cases"].as_array().expect("cases") {
        let (frame, _) = when(case);
        let file = case["file"].as_str().expect("file");
        let (_, _, _, png) = read_png(&reference(&format!("{FOLDER}/{file}")));
        let i = ep
            .frames
            .iter()
            .position(|&f| f == frame)
            .expect("a PNG card is a card frame");
        assert_eq!(Sha256::digest(&png)[..], ep.digests[i][..], "{file}");
        checked += 1;
    }
    assert_eq!(checked, 18);
    // Each timeline drawn here, one thread each.
    let results: Vec<(String, usize, Vec<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = pinned
            .iter()
            .map(|(p, case)| {
                let noise = &noise;
                s.spawn(move || {
                    let (show, card) = pinned_card(case);
                    let mut failures = Vec::new();
                    // the frames on which the card draws are the ones Python listed
                    let drawn: Vec<usize> = (0..show.grid.nf)
                        .filter(|&f| card.door_offset(f as f64 / FPS) < title::OPEN_PX - 1.0)
                        .collect();
                    if drawn != p.frames {
                        failures.push(format!("  {}: the card frames differ from Python's", p.timeline));
                    }
                    let mut passed = 0;
                    for (&frame, want) in p.frames.iter().zip(&p.digests) {
                        let mut img = noise.clone();
                        card.draw(&mut img, frame as f64 / FPS);
                        if Sha256::digest(&img.data)[..] == want[..] {
                            passed += 1;
                        } else {
                            let file = format!("{}_f{frame:04}.png", p.timeline);
                            let out = repo_root().join("studio/target/title_frames").join(&file);
                            write_png_rgb(&out, img.w, img.h, &img.data);
                            failures.push(format!(
                                "  {} frame {frame}: another picture, in {}",
                                p.timeline,
                                out.display()
                            ));
                        }
                    }
                    (p.timeline.clone(), passed, failures)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a timeline's thread"))
            .collect()
    });
    let mut failures = Vec::new();
    let mut counts = Vec::new();
    for ((name, passed, fails), (p, _)) in results.into_iter().zip(&pinned) {
        println!("title hashes, {name}: {passed}/{}", p.frames.len());
        counts.push(p.frames.len());
        failures.extend(fails);
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
    // every frame of the card at 132 BPM (both ways round), at 96 and at 200 BPM
    assert_eq!(counts, [61, 61, 84, 43]);
}

// ---------------------------------------------------------------- the hold and the cover
#[test]
fn the_doors_are_shut_at_the_hold_and_the_cover_frame_is_inside_it() {
    let ep = episode();
    let card = &ep.card;
    let index = index();
    let hold = index["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["rule"] == "hold")
        .expect("the hold is one of the cards");
    let (hold_frame, t_hold) = when(hold);
    assert_eq!(card.door_offset(t_hold), 0.0);
    assert_eq!(card.shake(t_hold), (0.0, 0.0));

    let cover = card.cover_frame();
    let t = cover as f64 / FPS;
    // Inside the hold: every word has landed and settled (0.26 s after the last one slams in), the doors are
    // shut and nothing shakes.
    let grid = &ep.show.grid;
    assert!(t >= grid.title[2] + 0.26 && t < grid.t_doors.0, "frame {cover}");
    assert_eq!(card.door_offset(t), 0.0);
    assert_eq!(card.shake(t), (0.0, 0.0));
    // It is the last such frame: on the next one the doors have begun to open.
    assert!(card.door_offset((cover + 1) as f64 / FPS) > 0.0);
    assert!(hold_frame <= cover);
    // The golden frames name the same frame: `doors_last_closed`, "the last frame with the title card shut".
    let frames = json(&reference("frames/flute_vs_trombone/frames.json"));
    let last_closed = frames["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .find(|f| {
            f["rules"]
                .as_array()
                .is_some_and(|r| r.iter().any(|x| x == "doors_last_closed"))
        })
        .expect("frames.json has doors_last_closed");
    assert_eq!(last_closed["frame"], cover);
    assert_eq!(cover, 43);
}

#[test]
fn nothing_is_drawn_once_the_doors_are_gone() {
    let card = &episode().card;
    let noise = noise();
    // From the end of the first bar to the last 0.3 s there is no card: Python's door_offset is None there.
    for frame in [53, 54, 600, 1820, 1821] {
        let t = frame as f64 / FPS;
        assert!(card.door_offset(t) >= title::OPEN_PX - 1.0, "frame {frame}");
        let mut img = noise.clone();
        let mut log = DrawLog::new();
        card.draw_logged(&mut img, &mut log, t);
        assert!(img == noise && log.boxes.is_empty(), "frame {frame}");
    }
    // The last frame of the doors opening and the first of them closing: the card is still, or already, there.
    for frame in [52, 1822] {
        let mut img = noise.clone();
        card.draw(&mut img, frame as f64 / FPS);
        assert!(img != noise, "frame {frame}");
    }
}

// ---------------------------------------------------------------- the draw log
/// The boxes the card files for the safe-zone check, against the ones the kit's own check recorded
/// (`checks/flute_vs_trombone.json`, `safezone.boxes`: every box `reelkit.ui` drew on the frames it looked at).
/// While the card is shut nothing else files a box, so the frame's whole list is the card's.
#[test]
fn settled_words_are_noted_as_captions() {
    let card = &episode().card;
    let checks = json(&reference("checks/flute_vs_trombone.json"));
    let rows = checks["checks"]["safezone"]["boxes"]
        .as_array()
        .expect("safezone.boxes");
    let noise = noise();
    let mut seen = Vec::new();
    for row in rows {
        let frame = row["frame"].as_u64().expect("frame") as usize;
        let t = frame as f64 / FPS;
        if card.door_offset(t) != 0.0 {
            continue;
        }
        let mut img = noise.clone();
        let mut log = DrawLog::new();
        card.draw_logged(&mut img, &mut log, t);
        let got: Vec<Value> = log
            .boxes
            .iter()
            .map(|b| {
                assert_eq!(b.kind, Kind::Caption);
                serde_json::json!([b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info])
            })
            .collect();
        assert_eq!(Value::from(got), row["boxes"], "frame {frame}");
        seen.push((frame, log.boxes.len()));
    }
    // The frames the check looked at while the card was shut: frame 0 (name A still slamming in: nothing has
    // settled), then one, two and all three words.
    assert_eq!(seen, [(0, 0), (8, 1), (15, 1), (22, 2), (30, 2), (38, 3)]);
    // `draw` is `draw_logged` without the log.
    let (mut a, mut b) = (noise.clone(), noise.clone());
    card.draw(&mut a, 38.0 / FPS);
    card.draw_logged(&mut b, &mut DrawLog::new(), 38.0 / FPS);
    assert!(a == b);
}
