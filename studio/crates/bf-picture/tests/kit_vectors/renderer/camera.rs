//! `reelkit.camera` (`picture_kit/camera.npz`): the camera's fields and every projection are float results (a
//! dot product or a tangent is in each), `clip_near` is exact.

use super::*;
use bf_picture::camera::{self, Key};

pub(super) const CALLS: &Calls = &[
    ("Cam(**cam)", 18),
    ("Cam(**cam).to_cam(P)", 18),
    ("Cam(**cam).wp(P)", 18),
    ("Cam(**cam).depth(P)", 18),
    ("Cam(**cam).proj(Pc)", 18),
    ("Cam(**cam).pixel_rays(xs, ys)", 9),
    ("Cam(**cam).ground_hit(xs, ys, y0)", 18),
    ("reelkit.camera.clip_near(Pc, near)", 20),
    ("reelkit.camera.interp_keys(keys, t)", 21),
];

/// The fields of a camera against the arrays `pos`, `M`, `target` and `scalars` of a case.
fn cam_fields(cam: &Cam, v: &Vectors, case: &Case) -> Found {
    let f = |role: &str| v.array(case, role).f64();
    let s = f("scalars");
    worst_of(&[
        close_rows(&cam.pos, f("pos"), 3),
        close_rows(&cam.m.concat(), f("M"), 3),
        exact(&cam.target, f("target")),
        close(&[cam.f], &s[..1]),
        exact(&[cam.cx, cam.cy, cam.near], &s[1..]),
    ])
}

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let f = |role: &str| v.array(case, role).f64();
    match case.call.as_str() {
        "Cam(**cam)" => {
            let cam = cam_of(args)?;
            worst_of(&[cam_fields(&cam, v, case), close_rows(&cam.view(), f("view"), 3)])
        }
        "Cam(**cam).to_cam(P)" => close_rows(&flat3(&cam_of(args)?.to_cam(&rows3(arr("P"))?)), f("out"), 3),
        "Cam(**cam).wp(P)" => close_rows(&flat2(&cam_of(args)?.wp(&rows3(arr("P"))?)), f("out"), 2),
        "Cam(**cam).depth(P)" => close(&cam_of(args)?.depth(&rows3(arr("P"))?), f("out")),
        "Cam(**cam).proj(Pc)" => close_rows(&flat2(&cam_of(args)?.proj(&rows3(arr("Pc"))?)), f("out"), 2),
        "Cam(**cam).pixel_rays(xs, ys)" => close_rows(&flat3(&cam_of(args)?.pixel_rays(f("xs"), f("ys"))), f("out"), 3),
        "Cam(**cam).ground_hit(xs, ys, y0)" => {
            let (x, z, t) = cam_of(args)?.ground_hit(f("xs"), f("ys"), float_arg(args, "y0")?);
            let zip = |a: &[f64], b: &[f64]| -> Vec<f64> { a.iter().zip(b).flat_map(|(&p, &q)| [p, q]).collect() };
            if x.len() != z.len() || x.len() != f("X").len() {
                return Err(format!(
                    "{} and {} values, Python has {}",
                    x.len(),
                    z.len(),
                    f("X").len()
                ));
            }
            worst_of(&[close_rows(&zip(&x, &z), &zip(f("X"), f("Z")), 2), close(&t, f("t"))])
        }
        "reelkit.camera.clip_near(Pc, near)" => {
            let got = camera::clip_near(&rows3(arr("Pc"))?, float_arg(args, "near")?);
            match (got, arr("none").bool()) {
                (None, [true]) => Ok(0.0),
                (Some(pts), [false]) => exact(&flat3(&pts), f("out")),
                (got, none) => Err(format!(
                    "{}, Python's none flag is {none:?}",
                    got.map_or("None".to_string(), |p| format!("{} points", p.len()))
                )),
            }
        }
        "reelkit.camera.interp_keys(keys, t)" => {
            let (kt, target, rest) = (f("key_t"), rows3(arr("key_target"))?, f("key_rest"));
            let keys: Vec<Key> = (0..kt.len())
                .map(|k| {
                    let r = &rest[5 * k..5 * k + 5];
                    Key {
                        t: kt[k],
                        target: target[k],
                        dist: r[0],
                        yaw: r[1],
                        pitch: r[2],
                        fov: r[3],
                        sy: r[4],
                    }
                })
                .collect();
            cam_fields(&camera::interp_keys(&keys, float_arg(args, "t")?, None), v, case)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn vectors() {
    check_calls("camera", CALLS, run);
}

#[test]
fn constants() {
    let v = Vectors::open("picture_kit/camera.npz");
    assert_eq!(hex_arg(&v.index.raw["NEAR"]).to_bits(), camera::NEAR.to_bits());
}
