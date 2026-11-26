//! The sprite renderer against the Python kit: every sprite in `reference/sprites/{FLUTE,TROMBONE}.npz` that the
//! kit drew from a mesh with `sprites.render_sprite` must come out the same, pixel for pixel, alpha included.
//!
//! Which cases are mesh renders is read from the sidecars (`tools/studio_export/sprites.py` wrote them):
//!
//! - group `model` is `sprites.render_sprite(name, radius, yaw, roll)` itself;
//! - groups `picture` and `episode` are `kit.picture.sprite`, which draws the mesh
//!   (`sprites.render_sprite(pp, r, float(yk), float(roll))`) only for a fighter whose look has no sprite of its
//!   own: the sidecar's `drawn_by` is `"model"` for those and `"look"` for the others;
//! - group `title` is `title.big_sprite(k)`, and the case's own `args.drawn_by` says how it was made: `"model"` is
//!   the mesh at the view's yaw and roll, `"closeup"` is the look's own close-up function, called with the radius
//!   alone (such a case has no yaw or roll).
//!
//! So TROMBONE's `picture`, `episode` and `title` cases and FLUTE's `model` cases are tested here. FLUTE's
//! `picture`, `episode` and `title` cases are `flute_sprite` of `kit/fighters/flute.py` (its look draws a pixel-art
//! flute, side `2 r + 5`): they belong to the FLUTE look, not to this renderer, and are counted as skipped.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bf_picture::img::Img;
use bf_picture::sprite::{Mesh, SpriteCache, SpriteKey, render_sprite};
use bf_testdata::{Case, Vectors, diff_u8, repo_root};

/// The model file of `name`, read by path (`bf-picture` does not depend on `bf-content`).
fn mesh(name: &str) -> Mesh {
    let path = repo_root()
        .join("studio")
        .join("crates")
        .join("bf-content")
        .join("data")
        .join("models")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Mesh::from_json(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn int_arg(case: &Case, key: &str) -> i64 {
    case.args[key]
        .as_i64()
        .unwrap_or_else(|| panic!("case {}: args.{key} is not an integer: {}", case.id, case.args[key]))
}

fn group(case: &Case) -> &str {
    case.args["group"]
        .as_str()
        .unwrap_or_else(|| panic!("case {} has no args.group", case.id))
}

/// Whether the kit drew this case from the mesh with `sprites.render_sprite`; `ball_drawn_by` is the sidecar's
/// top-level `drawn_by` (who draws the sprite inside the ball). Panics on a group, call or `drawn_by` this test
/// does not know, so a new kind of case cannot slip through untested.
fn drawn_from_mesh(ball_drawn_by: &str, case: &Case) -> bool {
    match (group(case), case.call.as_str()) {
        ("model", "sprites.render_sprite(name, radius, yaw, roll)") => true,
        ("picture" | "episode", "kit.picture.sprite(k, radius, yaw, roll)") => match ball_drawn_by {
            "model" => true,
            "look" => false,
            other => panic!("the sidecar's drawn_by is {other:?}"),
        },
        ("title", "title.big_sprite(k)") => match case.args["drawn_by"].as_str() {
            Some("model") => true,
            Some("closeup") => false,
            other => panic!("case {}: a title case with args.drawn_by {other:?}", case.id),
        },
        (g, call) => panic!("case {}: unknown group {g:?} with call {call:?}", case.id),
    }
}

/// A group of cases: its name, how many of its cases are mesh renders and how many are not.
type GroupCount = (&'static str, usize, usize);

/// The groups each fighter's file holds.
const EXPECTED: [(&str, &[GroupCount]); 2] = [
    (
        "FLUTE",
        &[
            ("picture", 0, 480),
            ("episode", 0, 23),
            ("title", 0, 1),
            ("model", 480, 0),
        ],
    ),
    ("TROMBONE", &[("picture", 480, 0), ("episode", 17, 0), ("title", 1, 0)]),
];

#[test]
fn every_mesh_sprite_matches_python() {
    let mut failures = Vec::new();
    let mut report = Vec::new();
    for (name, expected) in EXPECTED {
        let vectors = Vectors::open(&format!("sprites/{name}.npz"));
        let raw = &vectors.index.raw;
        assert_eq!(raw["tool"], "sprites");
        assert_eq!(raw["fighter"], name);
        let drawn_by = raw["drawn_by"].as_str().expect("the sidecar has drawn_by");
        let pp = raw["pp"].as_str().expect("the sidecar has pp");
        let mesh = mesh(pp);

        // (mesh renders, matching, not mesh renders) per group
        let mut counts: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for case in &vectors.index.cases {
            let radius = int_arg(case, "radius");
            let size = int_arg(case, "size") as usize;
            let want = vectors.array(case, "out");
            assert_eq!(want.shape, [size, size, 4], "case {}", case.id);
            let entry = counts.entry(group(case).to_string()).or_default();
            if !drawn_from_mesh(drawn_by, case) {
                // The look's own sprite is 2 r + 5 wide; a case of the mesh renderer's size must not be skipped.
                assert_eq!(
                    size as i64,
                    2 * radius + 5,
                    "case {} is skipped but is 2 r + 7 wide",
                    case.id
                );
                entry.2 += 1;
                continue;
            }
            let (yaw, roll) = (int_arg(case, "yaw"), int_arg(case, "roll"));
            assert_eq!(size as i64, 2 * radius + 7, "case {}", case.id);
            entry.0 += 1;
            seen.insert((radius, yaw, roll));
            let got = render_sprite(&mesh, radius as u32, yaw as f64, roll as f64);
            assert_eq!((got.w, got.h, got.c), (size, size, 4), "case {}", case.id);
            if got.data == want.u8() {
                entry.1 += 1;
            } else {
                failures.push(format!(
                    "{name} {} ({}, radius {radius}, yaw {yaw}, roll {roll}): {}",
                    case.id,
                    group(case),
                    diff_u8(&got.data, want.u8(), size, size, 4)
                ));
            }
        }

        // Nothing went missing: the groups and their sizes are the ones this test was written for.
        let got_counts: Vec<(&str, usize, usize)> = expected
            .iter()
            .map(|&(g, _, _)| {
                let (mesh_cases, _, other) = counts.get(g).copied().unwrap_or_default();
                (g, mesh_cases, other)
            })
            .collect();
        assert_eq!(
            got_counts, expected,
            "{name}: the cases per group (mesh renders, others)"
        );
        assert_eq!(counts.len(), expected.len(), "{name}: a group this test does not know");

        // The brief's grid: the four radii, 60 yaws and each roll of the sidecar.
        let rolls: Vec<i64> = raw["rolls"]
            .as_array()
            .expect("the sidecar has rolls")
            .iter()
            .map(|r| r.as_i64().expect("a roll is an integer"))
            .collect();
        assert!(rolls.len() >= 2 && rolls.contains(&0), "{name}: rolls {rolls:?}");
        for &roll in &rolls {
            for radius in [12, 16, 22, 30] {
                for yaw in (0..360).step_by(6) {
                    assert!(
                        seen.contains(&(radius, yaw, roll)),
                        "{name}: no mesh sprite at radius {radius}, yaw {yaw}, roll {roll}"
                    );
                }
            }
        }

        for (g, (mesh_cases, matching, other)) in &counts {
            if *mesh_cases > 0 {
                report.push(format!("{name} {g}: {matching}/{mesh_cases} mesh sprites match"));
            }
            if *other > 0 {
                report.push(format!(
                    "{name} {g}: {other} drawn by the fighter's look, not by this renderer (skipped)"
                ));
            }
        }
    }
    println!("{}", report.join("\n"));
    assert!(
        failures.is_empty(),
        "{} mesh sprites differ from Python:\n{}\n\n{}",
        failures.len(),
        failures.join("\n"),
        report.join("\n")
    );
}

/// The model files read back to the very bits Python wrote: an FNV-style hash over every float's bit pattern, the
/// group ids and the flags, computed from the JSON by the pinned Python (`json.load`, `struct.pack('<d', x)`).
#[test]
fn meshes_read_every_float_exactly() {
    for (name, faces, vertices, hash, first, last) in [
        (
            "FLUTE",
            112usize,
            516usize,
            0xc93b_748b_acf8_b504u64,
            "-0x1.ff3d65527a159p-1",
            "-0x1.262e33f673e05p-2",
        ),
        (
            "TROMBONE",
            573,
            2312,
            0x04f5_194b_4d24_4219,
            "-0x1.4a6e5cc38e731p-2",
            "-0x1.a713209200f1ep-2",
        ),
    ] {
        let mesh = mesh(name);
        assert_eq!(mesh.faces.len(), faces, "{name}");
        assert_eq!(mesh.faces.iter().map(|f| f.p.len()).sum::<usize>(), vertices, "{name}");
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let mut mix = |v: u64| h = h.wrapping_mul(0x0000_0100_0000_01b3) ^ v;
        for f in &mesh.faces {
            for x in f.p.iter().flatten().chain(&f.n).chain(&f.rgb).chain([&f.spec]) {
                mix(x.to_bits());
            }
            mix(f.gid as i64 as u64);
            mix(u64::from(f.two) | (u64::from(f.emit) << 1));
        }
        assert_eq!(h, hash, "{name}: a float, a group id or a flag was not read exactly");
        assert_eq!(
            mesh.faces[0].p[0][0].to_bits(),
            bf_testdata::parse_hex_f64(first).to_bits(),
            "{name}"
        );
        assert_eq!(
            mesh.faces[faces - 1].n[2].to_bits(),
            bf_testdata::parse_hex_f64(last).to_bits(),
            "{name}"
        );
    }
}

/// `SpriteKey::for_ball` rounds as `kit.picture.sprite` does: `int(np.clip(round(radius_px), 12, 30))` and
/// `int(round(yaw_deg / 6.0))`, both half to even. The expected values are what the pinned Python gave.
#[test]
fn sprite_key_rounds_as_the_kit_does() {
    let radii = [
        (11.2, 12),
        (12.5, 12),
        (13.5, 14),
        (14.5, 14),
        (17.49999, 17),
        (18.5, 18),
        (19.2, 19),
        (29.5, 30),
        (30.5, 30),
        (31.0, 30),
        (100.0, 30),
        (0.0, 12),
    ];
    for (radius_px, want) in radii {
        assert_eq!(
            SpriteKey::for_ball(0, radius_px, 0.0, 0.0).radius,
            want,
            "radius {radius_px}"
        );
    }
    let yaws = [
        (203.0, 34),
        (207.0, 34),
        (201.0, 34),
        (195.0, 32),
        (187.2, 31),
        (222.99, 37),
        (-3.0, 0),
        (3.0, 0),
        (-9.0, -2),
        (9.0, 2),
        (-17.99, -3),
        (356.9, 59),
        (357.0, 60),
        (359.9, 60),
        (720.5, 120),
        (0.0, 0),
    ];
    for (yaw_deg, want) in yaws {
        let key = SpriteKey::for_ball(1, 17.3, yaw_deg, 30.0);
        assert_eq!(key.yaw_step, want, "yaw {yaw_deg}");
        // the yaw the sprite is drawn at is float(yk), not wrapped: -12 stays -12 and 360 stays 360
        assert_eq!(key.yaw_deg(), f64::from(want * 6), "yaw {yaw_deg}");
        assert_eq!((key.fighter, key.radius, key.roll_deg()), (1, 17, 30.0));
    }
    assert_eq!(
        SpriteKey::for_ball(1, 17.3, 203.0, 30.0),
        SpriteKey::new(1, 17, 34, 30.0)
    );
    // a key is one picture: another fighter, radius, yaw step or roll is another key
    let key = SpriteKey::new(1, 17, 34, 30.0);
    for other in [
        SpriteKey::new(0, 17, 34, 30.0),
        SpriteKey::new(1, 18, 34, 30.0),
        SpriteKey::new(1, 17, -26, 30.0),
        SpriteKey::new(1, 17, 34, 30.5),
        SpriteKey::new(1, 17, 34, 0.0),
    ] {
        assert_ne!(key, other);
    }
}

/// A sprite from the cache is the sprite `render_sprite` draws for the key's radius, yaw and roll.
#[test]
fn sprite_cache_draws_from_the_key() {
    let mesh = mesh("TROMBONE");
    let cache = SpriteCache::new();
    assert!(cache.is_empty());
    for key in [
        SpriteKey::for_ball(1, 17.3, 203.0, 30.0),
        SpriteKey::new(1, 12, -1, 0.0),
        SpriteKey::new(1, 12, 59, 0.0),
        SpriteKey::new(1, 30, 37, 30.0),
    ] {
        let fresh = render_sprite(&mesh, key.radius, key.yaw_deg(), key.roll_deg());
        let cached = cache.mesh_sprite(key, &mesh);
        assert_eq!((cached.w, cached.h, cached.c), (fresh.w, fresh.h, 4));
        assert_eq!(cached.data, fresh.data, "{key:?}");
        // asked again: the very same sprite, not a second drawing
        assert!(Arc::ptr_eq(&cached, &cache.mesh_sprite(key, &mesh)));
        assert!(Arc::ptr_eq(
            &cached,
            &cache.get_or_draw(key, || unreachable!("drawn already"))
        ));
    }
    assert_eq!(cache.len(), 4);
    // the reference sprite for the first key (TROMBONE, radius 17, yaw 204, roll 30) is in the vectors
    let vectors = Vectors::open("sprites/TROMBONE.npz");
    let case = vectors
        .index
        .cases
        .iter()
        .find(|c| c.args["key"] == "TROMBONE_17_204_30")
        .expect("the frames ask for TROMBONE_17_204_30");
    let cached = cache.mesh_sprite(SpriteKey::for_ball(1, 17.3, 203.0, 30.0), &mesh);
    assert_eq!(cached.data, vectors.array(case, "out").u8());
}

/// Render threads share the cache: whichever thread asks first, every thread gets the same pixels for a key, and
/// each sprite is drawn once.
#[test]
fn sprite_cache_is_shared_by_threads() {
    fn shared<T: Send + Sync>() {}
    shared::<SpriteCache>();

    let mesh = mesh("FLUTE");
    let mut keys = Vec::new();
    for radius in [12, 13, 19] {
        for yaw_step in -3..=3 {
            for roll in [0.0, 32.0] {
                keys.push(SpriteKey::new(0, radius, yaw_step, roll));
            }
        }
    }
    let cache = SpriteCache::new();
    let drawn = AtomicUsize::new(0);
    let threads = 8;
    let per_thread: Vec<Vec<(usize, Arc<Img<u8>>)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (cache, mesh, keys, drawn) = (&cache, &mesh, &keys, &drawn);
                scope.spawn(move || {
                    // every thread walks the keys in its own order
                    let mut order: Vec<usize> = (0..keys.len()).collect();
                    order.rotate_left(t * 5 % keys.len());
                    if t % 2 == 1 {
                        order.reverse();
                    }
                    order
                        .into_iter()
                        .map(|i| {
                            let key = keys[i];
                            let sprite = cache.get_or_draw(key, || {
                                drawn.fetch_add(1, Ordering::Relaxed);
                                render_sprite(mesh, key.radius, key.yaw_deg(), key.roll_deg())
                            });
                            (i, sprite)
                        })
                        .collect()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a render thread panicked"))
            .collect()
    });
    assert_eq!(drawn.load(Ordering::Relaxed), keys.len(), "each sprite is drawn once");
    assert_eq!(cache.len(), keys.len());
    let mut checked = 0;
    for sprites in &per_thread {
        assert_eq!(sprites.len(), keys.len());
        for (i, sprite) in sprites {
            let key = keys[*i];
            let first = per_thread[0]
                .iter()
                .find(|(j, _)| j == i)
                .map(|(_, s)| s)
                .expect("thread 0 asked for every key");
            assert!(
                Arc::ptr_eq(sprite, first),
                "{key:?}: two threads hold different sprites"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, threads * keys.len());
    // and the shared sprite is what a single thread draws on its own
    for (i, sprite) in &per_thread[0] {
        let key = keys[*i];
        let alone = render_sprite(&mesh, key.radius, key.yaw_deg(), key.roll_deg());
        assert_eq!(sprite.data, alone.data, "{key:?}");
    }
}
