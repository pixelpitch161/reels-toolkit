//! The VS title card's opt-ins against Python: `studio/reference/picture_title_optins/title_optins.json`, written by
//! the export tool's `title_optins` (`tools/studio_export/title_optins.py`).
//!
//! Each case is a kit config's card with its `TITLE_LINES`, `TITLE_BALLS` or `MIDDLE` (the fight file's
//! `picture.title_lines`, `title_balls`, `middle`), drawn by Python as `title.draw(noise, t)` over the title vectors'
//! noise on every frame the card is drawn; each frame is pinned by the SHA-256 of its bytes and of its draw log
//! (`reelkit.ui.DRAWN`). The cards are the configs' own (names, colours, tempo, tag, opt-ins) with two stand-ins
//! for the fighters not ported yet, the same on both sides: their panel patterns are taken out (plain panels) and
//! their close-ups are [`standin_closeup`]. TROMBONE keeps its own pattern and its model's close-up.
//!
//! Configs: `examples/bassclarinet_vs_euphonium` (two lines on the left, moved balls; drawn a second time by
//! Python with the balls as floats, the same bytes), `ballfight_bassclarinet_vs_bassoon` (two lines, the card's
//! balls), `examples/barisax_vs_bassclarinet` (two lines on the right, moved balls),
//! `ballfight_horn_vs_trombone_rematch` (REMATCH along the divider), and two variants of the rematch: a middle word
//! at -12.5 degrees and one at 0 degrees (Python's falsy angle). A middle word too long to fit is the kit's error.

use std::collections::BTreeMap;

use bf_core::fightfile::{Middle, PictureSeeds};
use bf_core::grid::Grid;
use bf_core::show::Show;
use bf_picture::img::Img;
use bf_picture::look::Look;
use bf_picture::sprite::Mesh;
use bf_picture::title::layout::{Layout, Panel};
use bf_picture::title::{self, CardLook, TitleCard, TitleError, patterns};
use bf_picture::ui::DrawLog;
use bf_testdata::{hex_arg, read_png, reference, repo_root, write_png_rgb};
use serde_json::Value;
use sha2::{Digest, Sha256};

const FILE: &str = "picture_title_optins/title_optins.json";
const FPS: f64 = 30.0;

fn index() -> Value {
    let text = std::fs::read_to_string(reference(FILE)).expect("title_optins.json: run the export title_optins");
    serde_json::from_str(&text).expect("title_optins.json is JSON")
}

/// The seeded-noise image every card is drawn over.
fn noise() -> Img<u8> {
    let (w, h, c, data) = read_png(&reference("frames/flute_vs_trombone/title/noise.png"));
    assert_eq!((w, h, c), (360, 640, 3), "noise.png");
    Img::from_vec(w, h, c, data)
}

/// `standin_closeup(k)(r)` of the export: an RGBA disc of radius about 0.95 r, striped by integer arithmetic.
fn standin_closeup(k: i64, r: u32) -> Img<u8> {
    let n = 2 * r as usize + 7;
    let c = (n / 2) as i64;
    let mut img: Img<u8> = Img::new(n, n, 4);
    for y in 0..n as i64 {
        for x in 0..n as i64 {
            // inside = ((xx - c) ** 2 + (yy - c) ** 2) * 10 <= 9 * r * r
            if ((x - c) * (x - c) + (y - c) * (y - c)) * 10 <= 9 * i64::from(r) * i64::from(r) {
                let px = img.px_mut(x as usize, y as usize);
                px[0] = ((x * 7 + 40 * k) % 256) as u8;
                px[1] = ((y * 5 + 90) % 256) as u8;
                px[2] = (((x + y) * 3 + 20 * k) % 256) as u8;
                px[3] = 255;
            }
        }
    }
    img
}

/// The stand-in look of a fighter not ported yet: no pattern, [`standin_closeup`], the base class's glass.
struct StandIn(i64);

impl Look for StandIn {
    fn title_closeup(&self, r: u32) -> Option<Img<u8>> {
        Some(standin_closeup(self.0, r))
    }
}

/// A ported fighter's look as far as the card goes here: TROMBONE's has none of the three hooks.
struct Plain;

impl Look for Plain {}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("a list")
        .iter()
        .map(|s| s.as_str().expect("a text").to_string())
        .collect()
}

fn f3(v: &Value) -> [f64; 3] {
    let a: Vec<f64> = v.as_array().expect("three numbers").iter().map(hex_arg).collect();
    [a[0], a[1], a[2]]
}

/// The fight file's `picture` settings of a case: what the kit config sets, as a fight file holds it (the balls as
/// `f64`).
fn seeds(case: &Value) -> PictureSeeds {
    let mut title_lines = BTreeMap::new();
    for (name, lines) in case["title_lines"].as_object().expect("title_lines") {
        let l = strs(lines);
        title_lines.insert(name.clone(), (l[0].clone(), l[1].clone()));
    }
    let title_balls = match &case["title_balls"] {
        Value::Null => None,
        b => Some([f3(&b[0]), f3(&b[1])]),
    };
    let middle = match &case["middle"] {
        Value::Null => None,
        m => {
            let word = m[0].as_str().expect("the word").to_string();
            let turn = match &m[1] {
                Value::String(s) if s == "divider" => Middle::Divider,
                n => Middle::Degrees(n.as_f64().expect("an angle")),
            };
            Some((word, turn))
        }
    };
    PictureSeeds {
        shard_seed: 271,
        fx_seed: None,
        title_lines,
        title_balls,
        middle,
    }
}

/// The case's show: the grid of its tempo, checked against the timeline Python drew, and its names.
fn show(case: &Value) -> Show {
    let names = strs(&case["fighters"]);
    let grid = Grid::new(case["bpm"].as_f64().expect("bpm"));
    let tv = &case["timeline_values"];
    let id = case["case"].as_str().expect("case");
    let same = |got: f64, want: &Value, what: &str| assert_eq!(got.to_bits(), hex_arg(want).to_bits(), "{id}: {what}");
    same(grid.beat, &tv["beat"], "beat");
    same(grid.bar, &tv["bar"], "bar");
    same(grid.dur, &tv["dur"], "dur");
    same(grid.t_close, &tv["t_close"], "t_close");
    same(grid.t_doors.0, &tv["t_doors"][0], "t_doors[0]");
    same(grid.t_doors.1, &tv["t_doors"][1], "t_doors[1]");
    for j in 0..3 {
        same(grid.title[j], &tv["title"][j], "title");
    }
    Show {
        grid,
        names: [names[0].clone(), names[1].clone()],
        win: 0,
        lose: 1,
        t_ko: 50.0,
        t_hs: 50.35,
        t_solo: 52.0,
        caps_ability: [Vec::new(), Vec::new()],
        cap_ko: Vec::new(),
        cap_wins: Vec::new(),
        cap_comment: Vec::new(),
        captions: Vec::new(),
    }
}

/// The layout against Python's `WORDS` and `BALLS`.
fn check_layout(case: &Value, layout: &Layout) {
    let id = case["case"].as_str().expect("case");
    for (j, role) in ["A", "VS", "B"].iter().enumerate() {
        let want = &case["words"][role];
        let w = &layout.words[j];
        let at = format!("{id}: {role}");
        assert_eq!(Value::from(w.scale), want["scale"], "{at}: scale");
        assert_eq!(w.cx.to_bits(), hex_arg(&want["cx"]).to_bits(), "{at}: cx");
        assert_eq!(w.cy.to_bits(), hex_arg(&want["cy"]).to_bits(), "{at}: cy");
        let side = match w.panel {
            Panel::Left => 0,
            Panel::Right => 1,
            Panel::Both => -1,
        };
        assert_eq!(Value::from(side), want["side"], "{at}: side");
        assert_eq!(w.w.map(Value::from).unwrap_or(Value::Null), want["w"], "{at}: w");
        assert_eq!(w.h.map(Value::from).unwrap_or(Value::Null), want["h"], "{at}: h");
        match &w.lines {
            Some(lines) => {
                assert_eq!(
                    strs(&want["lines"]),
                    [lines[0].0.clone(), lines[1].0.clone()],
                    "{at}: lines"
                );
                for (n, (_, dy)) in lines.iter().enumerate() {
                    assert_eq!(dy.to_bits(), hex_arg(&want["dy"][n]).to_bits(), "{at}: dy");
                }
            }
            None => assert!(want["lines"].is_null(), "{at}: lines"),
        }
        match &w.middle {
            Some((word, ang)) => {
                assert_eq!(want["word"], Value::from(word.as_str()), "{at}: word");
                assert_eq!(ang.to_bits(), hex_arg(&want["ang"]).to_bits(), "{at}: ang");
            }
            None => assert!(want["word"].is_null(), "{at}: word"),
        }
    }
    for k in 0..2 {
        let b = layout.balls[k];
        let want = &case["balls"][k];
        for (n, got) in [b.cx, b.cy, b.r].into_iter().enumerate() {
            assert_eq!(got.to_bits(), hex_arg(&want[n]).to_bits(), "{id}: BALLS[{k}]");
        }
    }
}

/// `reelkit.ui.DRAWN` as the export hashes it: a line `x0,y0,x1,y1,kind,info` a box.
fn drawn_text(log: &DrawLog) -> String {
    log.boxes
        .iter()
        .map(|b| format!("{},{},{},{},{},{}\n", b.x0, b.y0, b.x1, b.y1, b.kind.as_str(), b.info))
        .collect()
}

fn hex(d: &[u8]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// A case's card drawn on every frame Python drew it; returns how many frames matched in picture and draw log.
fn draw_case(case: &Value, trombone: &Mesh, noise: &Img<u8>) -> (usize, usize, Vec<String>) {
    let id = case["case"].as_str().expect("case");
    let show = show(case);
    let seeds = seeds(case);
    let layout = Layout::new(&show.names[0], &show.names[1], &seeds).expect("the kit's layout fits");
    check_layout(case, &layout);
    let standins = strs(&case["standins"]);
    let empty = Mesh { faces: Vec::new() };
    let looks: [Box<dyn Look>; 2] = [0, 1].map(|k| -> Box<dyn Look> {
        if standins.contains(&show.names[k]) {
            Box::new(StandIn(k as i64))
        } else {
            Box::new(Plain)
        }
    });
    let views: Vec<[f64; 3]> = (0..2).map(|k| f3(&case["views"][k])).collect();
    let card_looks = [0, 1].map(|k| CardLook {
        look: &*looks[k],
        mesh: if show.names[k] == "TROMBONE" { trombone } else { &empty },
        yaw: views[k][0],
        roll: views[k][2],
    });
    // PATTERNS: the stood-in fighters' are taken out, the others are the card's own
    let builtin = [0, 1].map(|k| {
        if standins.contains(&show.names[k]) {
            None
        } else {
            patterns::builtin(&show.names[k]).expect("ported")
        }
    });
    let col = [f3(&case["colours"][0]), f3(&case["colours"][1])];
    let dark = [f3(&case["darks"][0]), f3(&case["darks"][1])];
    let card = TitleCard::from_parts(&show, col, dark, card_looks, case["series"].as_str(), layout, builtin)
        .expect("the card");
    let frames: Vec<usize> = case["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .map(|f| f.as_u64().expect("a frame") as usize)
        .collect();
    let drawn: Vec<usize> = (0..show.grid.nf)
        .filter(|&f| card.door_offset(f as f64 / FPS) < title::OPEN_PX - 1.0)
        .collect();
    let mut failures = Vec::new();
    if drawn != frames {
        failures.push(format!("  {id}: the card frames differ from Python's"));
    }
    let (mut pictures, mut logs) = (0, 0);
    for (n, &frame) in frames.iter().enumerate() {
        let mut img = noise.clone();
        let mut log = DrawLog::new();
        card.draw_logged(&mut img, &mut log, frame as f64 / FPS);
        if hex(&Sha256::digest(&img.data)) == case["sha256"][n] {
            pictures += 1;
        } else {
            let out = repo_root().join(format!("studio/target/title_optins/{id}_f{frame:04}.png"));
            write_png_rgb(&out, img.w, img.h, &img.data);
            failures.push(format!("  {id} frame {frame}: another picture, in {}", out.display()));
        }
        let text = drawn_text(&log);
        let want = &case["drawn"][n];
        if Value::from(log.boxes.len()) == want[0] && hex(&Sha256::digest(text.as_bytes())) == want[1] {
            logs += 1;
        } else {
            failures.push(format!("  {id} frame {frame}: another draw log:\n{text}"));
        }
        if frame as u64 == case["cover"]["frame"].as_u64().expect("cover") {
            let want: Vec<String> = case["cover"]["drawn"]
                .as_array()
                .expect("cover.drawn")
                .iter()
                .map(|b| {
                    let b = b.as_array().expect("a box");
                    format!(
                        "{},{},{},{},{},{}\n",
                        b[0],
                        b[1],
                        b[2],
                        b[3],
                        b[4].as_str().unwrap(),
                        b[5].as_str().unwrap()
                    )
                })
                .collect();
            assert_eq!(text, want.concat(), "{id}: the cover frame's boxes");
        }
    }
    (pictures, logs, failures)
}

#[test]
fn the_opt_in_cards_are_pythons() {
    let doc = index();
    assert_eq!(doc["tool"], "title_optins");
    let cases = doc["cases"].as_array().expect("cases");
    let names: Vec<&str> = cases.iter().map(|c| c["case"].as_str().expect("case")).collect();
    assert_eq!(
        names,
        [
            "bassclarinet_vs_euphonium",
            "bassclarinet_vs_euphonium_floats",
            "bassclarinet_vs_bassoon",
            "barisax_vs_bassclarinet",
            "horn_vs_trombone_rematch",
            "rematch_degrees",
            "rematch_upright"
        ]
    );
    let text = std::fs::read_to_string(repo_root().join("studio/crates/bf-content/data/models/TROMBONE.json"))
        .expect("TROMBONE's model");
    let trombone = Mesh::from_json(&text).expect("TROMBONE's model reads");
    let noise = noise();
    let results: Vec<(String, usize, usize, usize, Vec<String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = cases
            .iter()
            .map(|case| {
                let (trombone, noise) = (&trombone, &noise);
                s.spawn(move || {
                    let (p, l, f) = draw_case(case, trombone, noise);
                    let n = case["frames"].as_array().map_or(0, Vec::len);
                    (case["case"].as_str().unwrap().to_string(), n, p, l, f)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a case's thread"))
            .collect()
    });
    let mut failures = Vec::new();
    let mut total = 0;
    for (name, n, p, l, f) in results {
        println!("title opt-ins, {name}: pictures {p}/{n}, draw logs {l}/{n}");
        assert!(n >= 16, "{name}: {n} frames");
        total += n;
        failures.extend(f);
    }
    assert!(
        failures.is_empty(),
        "frames not equal to Python's:\n{}",
        failures.join("\n")
    );
    assert_eq!(total, 454, "the card frames of the seven cases");
}

#[test]
fn the_balls_read_as_floats_are_the_kits_ints() {
    // The kit's TITLE_BALLS are Python ints and a fight file's are floats; Python drew the bass clarinet example
    // both ways, and the two are the same bytes on every frame. The Studio reads the floats and draws the same.
    let doc = index();
    let cases = doc["cases"].as_array().expect("cases");
    let (ints, floats) = (&cases[0], &cases[1]);
    assert_eq!(
        ints["title_balls_python"],
        serde_json::json!([["int", "int", "int"], ["int", "int", "int"]])
    );
    assert_eq!(
        floats["title_balls_python"],
        serde_json::json!([["float", "float", "float"], ["float", "float", "float"]])
    );
    assert_eq!(ints["sha256"], floats["sha256"]);
    assert_eq!(ints["drawn"], floats["drawn"]);
    assert_eq!(
        seeds(ints).title_balls,
        Some([[74.0, 250.0, 54.0], [282.0, 396.0, 56.0]])
    );
}

#[test]
fn a_middle_word_that_cannot_fit_is_the_kits_error() {
    let doc = index();
    let err = &doc["middle_error"];
    let names = strs(&err["fighters"]);
    let word = err["middle"][0].as_str().expect("the word");
    assert_eq!(err["middle"][1], "divider");
    let seeds = PictureSeeds {
        shard_seed: 271,
        fx_seed: None,
        title_lines: BTreeMap::new(),
        title_balls: None,
        middle: Some((word.to_string(), Middle::Divider)),
    };
    let got = Layout::new(&names[0], &names[1], &seeds).expect_err("it does not fit");
    assert_eq!(got, TitleError::MiddleDoesNotFit(word.to_string()));
    assert_eq!(got.to_string(), err["error"].as_str().expect("the kit's message"));
    // a word the font draws no letter of cannot fit either (Python fails on its empty mask)
    let blank = PictureSeeds {
        middle: Some(("~~~".into(), Middle::Degrees(10.0))),
        ..seeds
    };
    assert!(matches!(
        Layout::new("A", "B", &blank),
        Err(TitleError::MiddleDoesNotFit(_))
    ));
}

#[test]
fn without_opt_ins_the_layout_is_the_plain_cards() {
    let none = PictureSeeds {
        shard_seed: 271,
        fx_seed: None,
        title_lines: BTreeMap::new(),
        title_balls: None,
        middle: None,
    };
    for (a, b) in [
        ("FLUTE", "TROMBONE"),
        ("TROMBONE", "FLUTE"),
        ("BASS CLARINET", "EUPHONIUM"),
    ] {
        assert_eq!(Layout::new(a, b, &none).expect("plain"), Layout::plain(a, b));
    }
    // a two-line name for a fighter not on the card changes nothing
    let mut other = none.clone();
    other.title_lines.insert("TUBA".into(), ("TU".into(), "BA".into()));
    assert_eq!(
        Layout::new("FLUTE", "TROMBONE", &other).expect("plain"),
        Layout::plain("FLUTE", "TROMBONE")
    );
}

#[test]
fn a_ball_too_big_for_its_close_up_is_refused() {
    let show = show(&index()["cases"][4]);
    let mut seeds = seeds(&index()["cases"][4]);
    seeds.middle = None;
    seeds.title_balls = Some([[78.0, 222.0, 56.0], [282.0, 396.0, 400.0]]);
    let layout = Layout::new(&show.names[0], &show.names[1], &seeds).expect("a layout");
    let mesh = Mesh { faces: Vec::new() };
    let looks = [0, 1].map(|_| CardLook {
        look: &Plain,
        mesh: &mesh,
        yaw: 0.0,
        roll: 0.0,
    });
    let got = TitleCard::from_parts(&show, [[1.0; 3]; 2], [[1.0; 3]; 2], looks, None, layout, [None, None]);
    assert_eq!(got.err(), Some(TitleError::CloseupTooBig("TROMBONE".into(), 320)));
}

#[test]
fn the_divider_angle_is_pythons() {
    // Python: math.degrees(math.atan2(470.0 - 170.0, 360.0)).hex() and math.atan2(300.0, 360.0).hex()
    let python = bf_testdata::parse_hex_f64("0x1.3e71cf41bf0e5p+5");
    assert_eq!(title::layout::DIVIDER_DEG.to_bits(), python.to_bits());
    assert_eq!(title::layout::divider_deg().to_bits(), python.to_bits());
    // the computed value is one unit in the last place off, from dmath's atan2
    let atan = bf_core::dmath::atan2(300.0, 360.0);
    assert_eq!(
        atan.to_bits(),
        bf_testdata::parse_hex_f64("0x1.63b4bc3f16a8bp-1").to_bits()
    );
    assert_eq!(
        (atan * (180.0 / bf_core::dmath::PI))
            .to_bits()
            .abs_diff(python.to_bits()),
        1,
        "dmath's angle is the next double"
    );
    // the angle the rematch's card was fitted at is this one
    let doc = index();
    let rematch = &doc["cases"][4]["words"]["VS"]["ang"];
    assert_eq!(hex_arg(rematch).to_bits(), python.to_bits());
}
