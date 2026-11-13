//! `bf_content::fighters::shared::older` against the episodes' own helpers: `studio/reference/older/vectors/
//! helpers.json` (export `older_vectors`), each case a call an episode's renderer made while it drew one frame a
//! second in its scratch copy (its `where`), or an edge case computed with the episode's function.
//!
//! - `mask_of`, its three families (`all`: #30, #28; `polys`: #31, #35; `lines`: #33): the mask's bytes.
//! - `blend`, its three families (`into`: #30; `early`: #39; `clip`: #33), on the synthetic pictures and masks of
//!   the shared export (`shared_vectors.synthetic`, `synthetic_mask`) in the recorded dtypes, with the recorded
//!   colour and alpha (how the episode passed it): the result's bytes.
//! - `dart_quads` (#28 = #35 = #33): every point to the bit.
//! - `annulus_polys` (#30's; #28's with the defaults): every point value by value under ruling R34 (its points go
//!   through the camera, whose numpy products are fused on the reference PC): `|got - want| <= max(1e-9 |want|,
//!   1e-12 S)`, S the point's largest coordinate.
//! - `draw_dart` (#28's VIOLIN darts, #35's HARP darts): each dart drawn on a fresh frame of the example episode at
//!   the file's `dart_t` (its camera and lights there, the default look for both fighters): the frame's bytes and
//!   depth buffer.

use std::sync::Arc;

use bf_content::fighters::shared::older::{self, Dart};
use bf_content::fighters::shared::reeds::{Num, Px};
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::{DefaultLook, Look};
use bf_picture::raster::Frame;
use bf_picture::ring::Picture;
use bf_testdata::{hex_arg, reference, repo_root};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn doc() -> Value {
    let path = reference("older/vectors/helpers.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (python -m tools.studio_export older_vectors)",
            path.display()
        )
    });
    let d: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(d["tool"], "older_vectors");
    d
}

fn cases<'a>(d: &'a Value, fun: &str) -> Vec<&'a Value> {
    let out: Vec<&Value> = d["cases"]
        .as_array()
        .expect("the cases")
        .iter()
        .filter(|c| c["fn"] == fun)
        .collect();
    assert!(out.len() >= 4, "{fun}: only {} cases", out.len());
    // every helper is proven on calls an episode really made, besides the edge cases
    assert!(out.iter().any(|c| c["where"] != "edge"), "{fun}: no recorded call");
    out
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn h(v: &Value) -> f64 {
    hex_arg(v)
}

fn hs(v: &Value) -> Vec<f64> {
    v.as_array().expect("a list").iter().map(h).collect()
}

fn v3(v: &Value) -> V3 {
    let x = hs(v);
    [x[0], x[1], x[2]]
}

fn sha_px<T: Px>(data: &[T]) -> String {
    // numpy's bytes: little-endian samples of the array's dtype
    let mut raw = Vec::with_capacity(data.len() * 8);
    for &v in data {
        if std::mem::size_of::<T>() == 4 {
            raw.extend_from_slice(&(v.get() as f32).to_le_bytes());
        } else {
            raw.extend_from_slice(&v.get().to_le_bytes());
        }
    }
    hex(&raw)
}

fn polys(c: &Value) -> Vec<older::Poly> {
    c["polys"]
        .as_array()
        .expect("the lists")
        .iter()
        .map(|p| {
            p.as_array()
                .expect("a list of points")
                .iter()
                .map(|q| (h(&q[0]), h(&q[1])))
                .collect()
        })
        .collect()
}

#[test]
fn the_masks_are_the_episodes() {
    let d = doc();
    let mut n = 0;
    for c in cases(&d, "mask_of") {
        let lists = polys(c);
        let width = c["width"].as_u64().map(|w| w as u32);
        let m = match c["family"].as_str().expect("a family") {
            "all" => older::mask_of_all(&lists),
            "polys" => older::mask_of_polys(&lists),
            "lines" => older::mask_of_lines(&lists, width),
            other => panic!("no mask family {other}"),
        };
        assert_eq!((m.w, m.h, m.c), (360, 640, 1));
        assert_eq!(
            sha_px(&m.data),
            c["sha256"].as_str().unwrap(),
            "{}: {}",
            c["where"],
            c["family"]
        );
        n += 1;
    }
    println!("masks: {n}");
}

fn synthetic<T: Px>(n: usize) -> Vec<T> {
    (0..n as u64)
        .map(|i| T::set(((i * 7919 + 13) % 25601) as f64 / 100.0))
        .collect()
}

fn synthetic_mask<T: Px>(n: usize) -> Vec<T> {
    (0..n as u64)
        .map(|j| T::set(((j * 104_729 + 7) % 256) as f64 / 255.0))
        .collect()
}

fn run_blend<T: Px, M: Px>(c: &Value) -> String {
    let (w, hgt) = (360, 640);
    let mut img = Img::from_vec(w, hgt, 3, synthetic::<T>(w * hgt * 3));
    let mk = Img::from_vec(w, hgt, 1, synthetic_mask::<M>(w * hgt));
    let col = v3(&c["col"]);
    let a = h(&c["alpha"][1]);
    let alpha = match c["alpha"][0].as_str().expect("how the alpha came") {
        "py" => Num::Py(a),
        "f64" => Num::F64(a),
        other => panic!("an alpha of kind {other} is not covered"),
    };
    match c["family"].as_str().expect("a family") {
        "into" => older::blend_into(&mut img, &mk, col, alpha),
        "early" => older::blend_early(&mut img, &mk, col, alpha),
        "clip" => older::blend_clip(&mut img, &mk, col, alpha),
        other => panic!("no blend family {other}"),
    }
    sha_px(&img.data)
}

#[test]
fn the_blends_are_the_episodes() {
    let d = doc();
    let mut n = 0;
    for c in cases(&d, "blend") {
        assert_eq!(c["col_shape"], serde_json::json!([3]));
        assert_eq!(c["out"], c["img"], "the picture keeps its dtype");
        let got = match (c["img"].as_str().unwrap(), c["mask"].as_str().unwrap()) {
            ("<f4", "<f4") => run_blend::<f32, f32>(c),
            ("<f4", "<f8") => run_blend::<f32, f64>(c),
            ("<f8", "<f4") => run_blend::<f64, f32>(c),
            ("<f8", "<f8") => run_blend::<f64, f64>(c),
            other => panic!("dtypes {other:?}"),
        };
        assert_eq!(got, c["sha256"].as_str().unwrap(), "{}: {}", c["where"], c["family"]);
        n += 1;
    }
    println!("blends: {n}");
}

#[test]
fn the_darts_are_the_episodes() {
    let d = doc();
    for c in cases(&d, "dart_quads") {
        let a = &c["args"];
        let (head, body) = older::dart_quads(
            h(&a["x"]),
            h(&a["z"]),
            h(&a["ang"]),
            h(&a["y"]),
            h(&a["L_"]),
            h(&a["Wd"]),
        );
        for (got, want) in [(head, &c["head"]), (body, &c["body"])] {
            let want: Vec<V3> = want.as_array().expect("points").iter().map(v3).collect();
            let bits = |p: &[V3]| p.iter().flat_map(|q| q.map(f64::to_bits)).collect::<Vec<_>>();
            assert_eq!(bits(&got), bits(&want), "{}: {got:?} against {want:?}", c["where"]);
        }
    }
}

fn cam(v: &Value) -> Cam {
    let m: Vec<V3> = v["M"].as_array().expect("the rows").iter().map(v3).collect();
    Cam {
        pos: v3(&v["pos"]),
        m: [m[0], m[1], m[2]],
        f: h(&v["f"]),
        cx: h(&v["cx"]),
        cy: h(&v["cy"]),
        near: h(&v["near"]),
        target: v3(&v["target"]),
    }
}

#[test]
fn the_rings_are_the_episodes() {
    let d = doc();
    let (mut n, mut worst) = (0, 0.0f64);
    for c in cases(&d, "annulus_polys") {
        let a = &c["args"];
        let got = older::annulus_polys(
            &cam(&c["cam"]),
            h(&a["cx"]),
            h(&a["cz"]),
            h(&a["r_in"]),
            h(&a["r_out"]),
            h(&a["y"]),
            a["n"].as_u64().expect("n") as usize,
            h(&a["a0"]),
            h(&a["a1"]),
        );
        let want = c["out"].as_array().expect("the quads");
        assert_eq!(got.len(), want.len(), "{}", c["where"]);
        for (q, w) in got.iter().zip(want) {
            for (g, w) in q.iter().zip(w.as_array().expect("a quad")) {
                let w = hs(w);
                let s = w[0].abs().max(w[1].abs());
                for (x, y) in [(g.0, w[0]), (g.1, w[1])] {
                    let e = (x - y).abs();
                    assert!(e <= (1e-9 * y.abs()).max(1e-12 * s), "{}: {x} against {y}", c["where"]);
                    worst = worst.max(e / s.max(f64::MIN_POSITIVE));
                }
            }
        }
        n += 1;
    }
    println!("rings: {n}, worst {worst:.1e} of the point's size");
}

/// The example episode with the default look for both fighters (what the export drew the darts on).
fn example() -> Picture {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
    let cfg: Vec<_> = (0..2)
        .map(|k| {
            bf_content::roster()
                .iter()
                .find(|f| f.name == show.names[k])
                .expect("in the roster")
                .clone()
        })
        .collect();
    let meshes = [0, 1].map(|k| bf_content::mesh(&show.names[k]).expect("a model"));
    let looks: [Box<dyn Look>; 2] = [Box::new(DefaultLook), Box::new(DefaultLook)];
    Picture::new(Arc::new(rec), &ff, show, [&cfg[0], &cfg[1]], meshes, looks).expect("the picture builds")
}

#[test]
fn the_dart_drawings_are_the_episodes() {
    let d = doc();
    let pic = example();
    let t = h(&d["dart_t"]);
    let c0 = pic.ctx().camera(t);
    let want_cam = cam(&d["dart_cam"]);
    assert_eq!(
        format!("{:?}", (c0.pos, c0.m, c0.f, c0.cx, c0.cy)),
        format!("{:?}", (want_cam.pos, want_cam.m, want_cam.f, want_cam.cx, want_cam.cy)),
        "the example's camera at {t}"
    );
    let l = pic.ctx().lights(t, &c0);
    let mut n = 0;
    for c in cases(&d, "draw_dart") {
        let style = match c["family"].as_str().expect("whose") {
            "violin" => Dart::Violin,
            "harp" => Dart::Harp,
            other => panic!("no dart {other}"),
        };
        let a = &c["args"];
        let mut fr = Frame::new([10.0, 8.0, 18.0]);
        older::draw_dart(
            &mut fr,
            &c0,
            &l,
            h(&a["x"]),
            h(&a["z"]),
            h(&a["ang"]),
            v3(&c["col"]),
            style,
        );
        assert_eq!(
            hex(&fr.pil.data),
            c["pil"].as_str().unwrap(),
            "{}: {style:?} the picture",
            c["where"]
        );
        let dep: Vec<u8> = fr.dep.data.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(
            hex(&dep),
            c["depth"].as_str().unwrap(),
            "{}: {style:?} the depth",
            c["where"]
        );
        n += 1;
    }
    println!("darts drawn: {n}");
}
