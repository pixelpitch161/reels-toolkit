//! `bf_content::fighters::shared` against the kit: the vectors of the `shared` export in `studio/reference/shared/`,
//! each case a call the kit made while a fighter that uses the helper drew a fight against FLUTE (its "fighter"), or
//! an edge case.
//!
//! - `reeds.npz`: every mask (`_mask` and its copies, ACOUSTIC's and TRUMPET's `mask_of`, HORN's and TUBA's,
//!   ORGAN's `mask_of` and `line_mask`) byte for byte by its SHA-256; `blend` and `add` on synthetic pictures and
//!   masks of the recorded dtypes, with the recorded colour and alpha (how the kit passed it), the result's bytes;
//!   `draw_woodwind_shard` on a synthetic `float32` frame, the frame's bytes; `beam_lane` on a grid of canvas
//!   pixels, every value; `ring_exit` and `tail_theta` to the bit; `ribbon` value by value (ruling R34: its points
//!   go through the camera, whose numpy products are fused on the reference PC: `|got - want| <=
//!   max(1e-9 |want|, 1e-12 S)`, S the point's largest coordinate).
//! - `padded.npz`: every `padded_sprite` and `_render` call, replayed on the TROMBONE's and the FLUTE's models,
//!   the RGBA bytes by their SHA-256.
//! - `ndimage.npz`: TRUMPET's `gaussian_filter` calls on the masks its beams drew (the mask rebuilt by `mask_of`),
//!   the result's bytes.
//! - `pp_blocks.npz`: `bassclarinet_lowpoly.slap` against its fingerprints (below -120 dBFS), the parts' hash
//!   `h01` to the bit.

use bf_content::fighters::shared::reeds::{Num, Px};
use bf_content::fighters::{self, shared::hash, shared::padded, shared::reeds, shared::slap};
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::ndimage::gaussian_filter;
use bf_picture::scene;
use bf_picture::sprite::Mesh;
use bf_sound::dsp;
use bf_testdata::{Case, Vectors, db_max_diff, hex_arg};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn sha_bytes(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|b| format!("{b:02x}")).collect()
}

fn sha_f32(x: &[f32]) -> String {
    sha_bytes(&x.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>())
}

fn sha_f64(x: &[f64]) -> String {
    sha_bytes(&x.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>())
}

fn raw(v: &Vectors, i: usize) -> &Value {
    &v.index.raw["cases"][i]
}

/// The offset and shape of `role` in case `i` (its slice of the shared member).
fn at(v: &Vectors, i: usize, role: &str) -> (usize, Vec<usize>) {
    let a = raw(v, i)["at"][role]
        .as_array()
        .unwrap_or_else(|| panic!("case {i} has no {role}"));
    (
        a[0].as_u64().unwrap() as usize,
        a[1..].iter().map(|d| d.as_u64().unwrap() as usize).collect(),
    )
}

fn f64s<'a>(v: &'a Vectors, i: usize, role: &str) -> &'a [f64] {
    let (off, shape) = at(v, i, role);
    &v.array(&v.index.cases[i], role).f64()[off..off + shape.iter().product::<usize>()]
}

fn i64s<'a>(v: &'a Vectors, i: usize, role: &str) -> &'a [i64] {
    let (off, shape) = at(v, i, role);
    &v.array(&v.index.cases[i], role).i64()[off..off + shape.iter().product::<usize>()]
}

fn u8s<'a>(v: &'a Vectors, i: usize, role: &str) -> &'a [u8] {
    let (off, shape) = at(v, i, role);
    &v.array(&v.index.cases[i], role).u8()[off..off + shape.iter().product::<usize>()]
}

fn arg(c: &Case, key: &str) -> f64 {
    hex_arg(&c.args[key])
}

fn v3(x: &[f64]) -> V3 {
    [x[0], x[1], x[2]]
}

/// A kit camera from its recorded numbers.
fn cam(v: &Vectors, i: usize) -> Cam {
    let (m, s) = (f64s(v, i, "M"), f64s(v, i, "scal"));
    Cam {
        pos: v3(f64s(v, i, "pos")),
        m: [v3(&m[0..3]), v3(&m[3..6]), v3(&m[6..9])],
        f: s[0],
        cx: s[1],
        cy: s[2],
        near: s[3],
        target: v3(f64s(v, i, "target")),
    }
}

/// A list of screen points.
type Path = Vec<(f64, f64)>;

/// A mask's polygons and lines from `pts` and `lens` (`[polygons, their lengths..., the lines' lengths...]`).
fn lists(v: &Vectors, i: usize) -> (Vec<Path>, Vec<Path>) {
    let (pts, lens) = (f64s(v, i, "pts"), i64s(v, i, "lens"));
    let np = lens[0] as usize;
    let mut k = 0;
    let mut take = |n: i64| -> Vec<(f64, f64)> {
        let out = (0..n as usize)
            .map(|j| (pts[2 * (k + j)], pts[2 * (k + j) + 1]))
            .collect();
        k += n as usize;
        out
    };
    let polys: Vec<_> = lens[1..=np].iter().map(|&n| take(n)).collect();
    let lines: Vec<_> = lens[np + 1..].iter().map(|&n| take(n)).collect();
    assert_eq!(2 * k, pts.len(), "case {i}: the lengths do not cover the points");
    (polys, lines)
}

fn width(c: &Case) -> Option<u32> {
    c.args["width"].as_u64().map(|w| w as u32)
}

/// The mask a case asks for, by its family.
fn mask_of_case(c: &Case, family: &str, polys: &[Vec<(f64, f64)>], lines: &[Vec<(f64, f64)>]) -> Img<f32> {
    match family {
        "reeds" => reeds::mask(polys, lines, width(c).unwrap()),
        "mask_of" => reeds::mask_of(polys, width(c)),
        "mask_all" => reeds::mask_all(polys),
        "mask_polys" => reeds::mask(polys, &[], 1),
        "line_mask" => reeds::mask(&[], lines, width(c).unwrap()),
        other => panic!("{}: a mask family this test does not know: {other}", c.id),
    }
}

const MASKS: [&str; 5] = ["reeds", "mask_of", "mask_all", "mask_polys", "line_mask"];

#[test]
fn every_mask_is_the_kits() {
    let v = Vectors::open("shared/reeds.npz");
    let mut n = std::collections::BTreeMap::new();
    for (i, c) in v.index.cases.iter().enumerate() {
        if !MASKS.contains(&c.call.as_str()) {
            continue;
        }
        let (polys, lines) = lists(&v, i);
        let m = mask_of_case(c, &c.call, &polys, &lines);
        let r = raw(&v, i);
        assert_eq!(
            sha_f32(&m.data),
            r["sha256"].as_str().unwrap(),
            "{} ({}, {}): the mask differs; {} set, Python's {}",
            c.id,
            c.call,
            r["fighter"],
            m.data.iter().filter(|&&x| x != 0.0).count(),
            r["nonzero"]
        );
        *n.entry(c.call.clone()).or_insert(0) += 1;
    }
    println!("masks: {n:?}");
    for family in MASKS {
        assert!(
            n.get(family).copied().unwrap_or(0) >= 3,
            "too few {family} cases: {n:?}"
        );
    }
}

/// The synthetic picture and mask of the export (`synthetic`, `synthetic_mask`), `h` x `w`.
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

fn run_blend<T: Px, M: Px>(c: &Case, col: [f64; 3], h: usize, w: usize) -> Img<T> {
    let mut img = Img::from_vec(w, h, 3, synthetic::<T>(h * w * 3));
    let mk = Img::from_vec(w, h, 1, synthetic_mask::<M>(h * w));
    let a = arg(c, "alpha");
    let alpha = match c.args["alpha_type"].as_str().unwrap() {
        "float" | "int" => Num::Py(a),
        "float64" => Num::F64(a),
        other => panic!("{}: an alpha of type {other}", c.id),
    };
    if c.call == "blend" {
        reeds::blend(&mut img, &mk, col, alpha);
    } else {
        reeds::add(&mut img, &mk, col, a);
    }
    img
}

#[test]
fn blend_and_add_are_the_kits_for_every_dtype() {
    let v = Vectors::open("shared/reeds.npz");
    let shape = &v.index.raw["synthetic_shape"];
    let (h, w) = (shape[0].as_u64().unwrap() as usize, shape[1].as_u64().unwrap() as usize);
    let mut kinds = std::collections::BTreeSet::new();
    let mut n = 0;
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "blend" && c.call != "add" {
            continue;
        }
        let col = f64s(&v, i, "col");
        let col = [col[0], col[1], col[2]];
        let got = match (c.args["img"].as_str().unwrap(), c.args["mk"].as_str().unwrap()) {
            ("float32", "float32") => sha_f32(&run_blend::<f32, f32>(c, col, h, w).data),
            ("float32", "float64") => sha_f32(&run_blend::<f32, f64>(c, col, h, w).data),
            ("float64", "float32") => sha_f64(&run_blend::<f64, f32>(c, col, h, w).data),
            ("float64", "float64") => sha_f64(&run_blend::<f64, f64>(c, col, h, w).data),
            other => panic!("{}: dtypes {other:?}", c.id),
        };
        let r = raw(&v, i);
        assert_eq!(
            got,
            r["sha256"].as_str().unwrap(),
            "{} ({}, {}, {:?})",
            c.id,
            c.call,
            r["fighter"],
            c.args
        );
        kinds.insert((
            c.call.clone(),
            c.args["img"].to_string(),
            c.args["mk"].to_string(),
            c.args["alpha_type"].to_string(),
        ));
        n += 1;
    }
    println!("blend and add: {n} cases, {} kinds of call", kinds.len());
    assert!(kinds.len() >= 12, "{kinds:?}");
}

#[test]
fn ribbons_are_the_kits() {
    let v = Vectors::open("shared/reeds.npz");
    let (mut n, mut worst) = (0, 0.0f64);
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "ribbon" {
            continue;
        }
        let p: Vec<V3> = f64s(&v, i, "P").chunks(3).map(v3).collect();
        let got = reeds::ribbon(&cam(&v, i), &p, f64s(&v, i, "hw"));
        let want = f64s(&v, i, "out");
        assert_eq!(2 * got.len(), want.len(), "{}: points", c.id);
        for (j, (g, w)) in got.iter().zip(want.chunks(2)).enumerate() {
            let s = w[0].abs().max(w[1].abs());
            for (a, b) in [(g.0, w[0]), (g.1, w[1])] {
                let d = (a - b).abs();
                assert!(
                    d <= (1e-9 * b.abs()).max(1e-12 * s),
                    "{}: point {j}: {a} against {b}",
                    c.id
                );
                worst = worst.max(d / s.max(f64::MIN_POSITIVE));
            }
        }
        n += 1;
    }
    println!("ribbons: {n}, worst {worst:.1e} of the point's size");
    assert!(n >= 6, "{n}");
}

#[test]
fn ring_exit_beam_lane_and_the_glide_home_are_the_kits_to_the_bit() {
    let v = Vectors::open("shared/reeds.npz");
    let (mut exits, mut lanes, mut tails) = (0, 0, 0);
    // the export's canvas grid: 48 x 48 ring positions across the ropes and past them
    let a = scene::A + 0.3;
    let u = dsp::linspace(-a, a, 48);
    let sx: Vec<f64> = (0..48 * 48).map(|i| u[i / 48]).collect();
    let sz: Vec<f64> = (0..48 * 48).map(|i| u[i % 48]).collect();
    let ys: Vec<usize> = (0..48 * 48).map(|i| (i / 48) % 640).collect();
    let xs: Vec<usize> = (0..48 * 48).map(|i| (i * 7) % 360).collect();
    for (i, c) in v.index.cases.iter().enumerate() {
        match c.call.as_str() {
            "ring_exit" => {
                assert_eq!(arg(c, "S_A"), scene::A);
                let (o, d) = (f64s(&v, i, "o"), f64s(&v, i, "d"));
                let lim = (!c.args["A"].is_null()).then(|| arg(c, "A"));
                let got = reeds::ring_exit([o[0], o[1]], [d[0], d[1]], lim);
                assert_eq!(
                    got.to_bits(),
                    arg(c, "out").to_bits(),
                    "{}: {got} against {}",
                    c.id,
                    arg(c, "out")
                );
                exits += 1;
            }
            "beam_lane" => {
                let o = f64s(&v, i, "o");
                let got = reeds::beam_lane(
                    &sx,
                    &sz,
                    [o[0], o[1]],
                    arg(c, "aim"),
                    arg(c, "length"),
                    arg(c, "width"),
                    &ys,
                    &xs,
                );
                let want = u8s(&v, i, "thirds");
                let differ = got
                    .iter()
                    .zip(want)
                    .filter(|(g, w)| g.to_bits() != (f64::from(**w) / 3.0).to_bits())
                    .count();
                assert_eq!(differ, 0, "{}: {differ} pixels of the lane differ", c.id);
                lanes += 1;
            }
            "tail_theta" => {
                let got = reeds::tail_theta_of(arg(c, "th_end"), arg(c, "th0"), arg(c, "gl"));
                assert_eq!(got.to_bits(), arg(c, "th").to_bits(), "{}", c.id);
                tails += 1;
            }
            _ => {}
        }
    }
    println!("ring_exit: {exits}, beam_lane: {lanes}, tail_theta: {tails}");
    assert!(exits >= 10 && lanes >= 6 && tails >= 6, "{exits} {lanes} {tails}");
}

#[test]
fn woodwind_shards_are_the_kits() {
    let v = Vectors::open("shared/reeds.npz");
    let mut kinds = std::collections::BTreeSet::new();
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "draw_woodwind_shard" {
            continue;
        }
        let mut arr = Img::from_vec(360, 640, 3, synthetic::<f32>(640 * 360 * 3));
        let kind = c.args["kind"].as_str().unwrap();
        reeds::draw_woodwind_shard(
            &mut arr,
            &cam(&v, i),
            kind,
            v3(f64s(&v, i, "p")),
            arg(c, "rot"),
            arg(c, "size"),
        );
        assert_eq!(
            sha_f32(&arr.data),
            raw(&v, i)["sha256"].as_str().unwrap(),
            "{}: a {kind} shard",
            c.id
        );
        kinds.insert(kind.to_string());
    }
    assert_eq!(kinds.len(), 4, "{kinds:?}");
}

fn mesh(name: &str) -> Mesh {
    Mesh::from_json(fighters::mesh_json(name).unwrap_or_else(|| panic!("{name} has no model"))).unwrap()
}

#[test]
fn padded_sprites_are_the_kits() {
    let v = Vectors::open("shared/padded.npz");
    let meshes = [("TROMBONE", mesh("TROMBONE")), ("FLUTE", mesh("FLUTE"))];
    let mut callers = std::collections::BTreeSet::new();
    for (i, c) in v.index.cases.iter().enumerate() {
        let r = c.args["r"].as_u64().unwrap() as u32;
        for (name, m) in &meshes {
            let s = padded::padded_sprite(m, r, arg(c, "yaw"), arg(c, "roll"), arg(c, "pad"));
            assert_eq!(s.w as i64, i64s(&v, i, "side")[0], "{}: side", c.id);
            assert_eq!(
                sha_bytes(&s.data),
                c.args["replay"][*name].as_str().unwrap(),
                "{} on {name}",
                c.id
            );
        }
        callers.insert(format!(
            "{} {}",
            raw(&v, i)["fighter"].as_str().unwrap(),
            c.args["caller"]
        ));
    }
    println!("padded sprites: {} cases, {callers:?}", v.index.cases.len());
    for who in ["CONTRABASS", "SOUSAPHONE", "BASS DRUM"] {
        assert!(
            callers.iter().any(|c| c.starts_with(who)),
            "no call of {who}'s: {callers:?}"
        );
    }
}

#[test]
fn trumpets_gaussian_filters_are_the_kits() {
    let v = Vectors::open("shared/ndimage.npz");
    let mut n = 0;
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "gaussian_filter" {
            continue;
        }
        let (polys, lines) = lists(&v, i);
        let family = c.args["mask"].as_str().unwrap();
        let m = mask_of_case(c, family, &polys, &lines);
        let r = raw(&v, i);
        assert_eq!(
            sha_f32(&m.data),
            r["input_sha256"].as_str().unwrap(),
            "{}: the mask",
            c.id
        );
        assert_eq!(
            c.args["extra"].as_u64(),
            Some(0),
            "{}: gaussian_filter with more arguments",
            c.id
        );
        let out = gaussian_filter(&m, arg(c, "sigma"));
        assert_eq!(
            sha_f32(&out.data),
            r["sha256"].as_str().unwrap(),
            "{}: sigma {}",
            c.id,
            arg(c, "sigma")
        );
        n += 1;
    }
    println!("TRUMPET's gaussian filters: {n}");
    assert!(n >= 3, "{n}");
}

#[test]
fn the_bass_clarinets_slap_is_the_kits() {
    let v = Vectors::open("shared/pp_blocks.npz");
    let (mut n, mut worst) = (0, f64::NEG_INFINITY);
    for (i, c) in v.index.cases.iter().enumerate() {
        if c.call != "slap" {
            continue;
        }
        let x = slap::slap(arg(c, "f"), arg(c, "v"), arg(c, "dur"));
        let r = raw(&v, i);
        let len = r["n"].as_u64().unwrap() as usize;
        assert_eq!(x.len(), len, "{}", c.id);
        let picks = f64s(&v, i, "picks");
        let k = picks.len();
        let got: Vec<f64> = (0..k).map(|j| x[(j * (len - 1)) / (k - 1)]).collect();
        let d = db_max_diff(&got, picks);
        let rms = f64s(&v, i, "rms");
        let blocks: Vec<f64> = x
            .chunks(882)
            .map(|b| dsp::mean(&b.iter().map(|s| s * s).collect::<Vec<_>>()).sqrt())
            .collect();
        let d = d.max(db_max_diff(&blocks, rms));
        assert!(d < -120.0, "{}: {d:.1} dBFS", c.id);
        worst = worst.max(d);
        n += 1;
    }
    println!("slap: {n} pops, worst {worst:.1} dBFS");
    assert!(n >= 12, "{n}");
}

#[test]
fn the_parts_hash_is_the_kits_to_the_bit() {
    let v = Vectors::open("shared/pp_blocks.npz");
    let rows = v.index.raw["h01"].as_array().unwrap();
    for row in rows {
        let xs: Vec<f64> = row["xs"].as_array().unwrap().iter().map(hex_arg).collect();
        let got = hash::h01(&xs);
        assert_eq!(got.to_bits(), hex_arg(&row["out"]).to_bits(), "h01({xs:?})");
    }
    assert!(rows.len() >= 10);
}
