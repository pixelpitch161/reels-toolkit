//! `render_sprite_pitched` against #31's own `styles.render_sprite(name, r, yaw, roll, pitch)` (the model tipped
//! toward the camera by `Rz @ Rx @ Ry`; SYNTH and TURNTABLES draw their sprites this way, plan m4 Task 10), and at
//! pitch 0 against `render_sprite`, which every existing sprite still is.
//!
//! The SHA-256 of each RGBA sprite was taken with the reference interpreter (`PYTHONHASHSEED=0`, PYTHONPATH the
//! clone) running #31's styles.py from its scratch copy:
//!
//! ```text
//! from tools.studio_export.older import origin as O
//! O._episode_env(O.scratch('ballfight_turntables_vs_synth')); import styles as ST
//! hashlib.sha256(np.ascontiguousarray(ST.render_sprite(name, r, yaw, roll, pitch)).tobytes()).hexdigest()
//! ```
//!
//! and, for the pitch-0 case, `np.array_equal(ST.render_sprite(..., 0.0), sprites.render_sprite(...))` was True.

use bf_picture::sprite::{Mesh, render_sprite, render_sprite_pitched};
use bf_testdata::repo_root;
use sha2::{Digest, Sha256};

/// (model, radius, yaw, roll, pitch, the SHA-256 of #31's sprite)
const CASES: [(&str, u32, f64, f64, f64, &str); 6] = [
    (
        "TROMBONE",
        20,
        10.0,
        0.0,
        28.0,
        "11b4f385e227ca6d3649d5bb66ec946e5ef8985cb22572fc4dbfd4fb323f8770",
    ),
    (
        "TROMBONE",
        30,
        -14.0,
        6.0,
        45.0,
        "e82ca09379326aee0024ae33dddb0978aea16091ef84c50662eed3d7877b06e1",
    ),
    (
        "FLUTE",
        16,
        34.0,
        0.0,
        28.0,
        "3c1f54d8ac1e5355e1c91af2330916009e2789772c89996042c31283bc277a8d",
    ),
    (
        "FLUTE",
        12,
        0.0,
        -8.0,
        90.0,
        "71b2c2004ed271667d234a9ee9d9f4174f77b9a99938a5d87e725ef05d0d3aa7",
    ),
    (
        "TUBA",
        24,
        22.0,
        0.0,
        45.0,
        "b7d4206dcad8dd65002f42a0bfa51bb856ea0cd3b7e23812dd7133abc9b54c70",
    ),
    (
        "TROMBONE",
        18,
        6.0,
        0.0,
        0.0,
        "105987775cd5f81816872bd6f9ada238b721226e1d9d238d18666082e38e9aa6",
    ),
];

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

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn a_pitched_sprite_is_31s() {
    for (name, r, yaw, roll, pitch, want) in CASES {
        let img = render_sprite_pitched(&mesh(name), r, yaw, roll, pitch);
        assert_eq!(
            (img.w, img.h, img.c),
            (2 * r as usize + 7, 2 * r as usize + 7, 4),
            "{name} {r}"
        );
        assert_eq!(sha(&img.data), want, "{name} r {r} yaw {yaw} roll {roll} pitch {pitch}");
    }
}

#[test]
fn at_pitch_zero_it_is_the_kits_sprite() {
    for (name, r, yaw, roll) in [
        ("TROMBONE", 18, 6.0, 0.0),
        ("TUBA", 25, -30.0, 12.0),
        ("FLUTE", 30, 64.0, -5.0),
    ] {
        let m = mesh(name);
        assert_eq!(
            render_sprite_pitched(&m, r, yaw, roll, 0.0),
            render_sprite(&m, r, yaw, roll),
            "{name}"
        );
    }
}
