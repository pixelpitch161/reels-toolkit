//! `reelkit.geometry` (`picture_kit/geometry.npz`): corners are exact where no sine turns them; normals go
//! through a norm.

use super::*;
use bf_picture::geometry;

pub(super) const CALLS: &Calls = &[
    ("reelkit.geometry.face(pts, center, up)", 18),
    ("reelkit.geometry.box(cx, cy, cz, hx, hy, hz, rot)", 8),
    ("reelkit.geometry.prism(cx, cz, r, y0, y1, n, r1, rot, tilt)", 10),
    ("reelkit.geometry.lathe(cx, cz, prof, n, rot, cy)", 6),
    ("reelkit.geometry.catenary(a, b, sag, n)", 6),
    ("reelkit.geometry.thin_quad(a, b, w)", 8),
];

/// A list of faces against the arrays `Q`, `counts` and `n` of a case; the corners exactly where `exact_q`.
fn faces_are(faces: &[geometry::Face], v: &Vectors, case: &Case, exact_q: bool) -> Found {
    let counts: Vec<i64> = faces.iter().map(|(q, _)| q.len() as i64).collect();
    same_i64(&counts, v.array(case, "counts").i64())?;
    let q: Vec<f64> = faces.iter().flat_map(|(q, _)| flat3(q)).collect();
    let n: Vec<f64> = faces.iter().flat_map(|(_, n)| *n).collect();
    let want_q = v.array(case, "Q").f64();
    worst_of(&[
        if exact_q {
            exact(&q, want_q)
        } else {
            close_rows(&q, want_q, 3)
        },
        close_rows(&n, v.array(case, "n").f64(), 3),
    ])
}

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
    let args = &case.args;
    let arr = |role: &str| v.array(case, role);
    let a = |key: &str| num(&args[key], key);
    match case.call.as_str() {
        "reelkit.geometry.face(pts, center, up)" => {
            let center = match &args["center"] {
                Value::Null => None,
                c => Some(vec3(c, "center")?),
            };
            let up = args["up"].as_bool().ok_or("up is not a bool")?;
            let (q, n) = geometry::face(&rows3(arr("pts"))?, center, up);
            worst_of(&[exact(&flat3(&q), arr("Q").f64()), close_rows(&n, arr("n").f64(), 3)])
        }
        "reelkit.geometry.box(cx, cy, cz, hx, hy, hz, rot)" => {
            let faces = geometry::box_(a("cx")?, a("cy")?, a("cz")?, a("hx")?, a("hy")?, a("hz")?, a("rot")?);
            faces_are(&faces, v, case, a("rot")? == 0.0)
        }
        "reelkit.geometry.prism(cx, cz, r, y0, y1, n, r1, rot, tilt)" => {
            let faces = geometry::prism(
                a("cx")?,
                a("cz")?,
                a("r")?,
                a("y0")?,
                a("y1")?,
                count(args, "n")?,
                opt_num(&args["r1"], "r1")?,
                a("rot")?,
                pair(&args["tilt"], "tilt")?,
            );
            faces_are(&faces, v, case, false)
        }
        "reelkit.geometry.lathe(cx, cz, prof, n, rot, cy)" => {
            let prof = rows2(arr("prof"))?;
            let faces = geometry::lathe(a("cx")?, a("cz")?, &prof, count(args, "n")?, a("rot")?, a("cy")?);
            faces_are(&faces, v, case, false)
        }
        "reelkit.geometry.catenary(a, b, sag, n)" => {
            let got = geometry::catenary(
                vec3(&args["a"], "a")?,
                vec3(&args["b"], "b")?,
                a("sag")?,
                count(args, "n")?,
            );
            exact(&flat3(&got), arr("out").f64())
        }
        "reelkit.geometry.thin_quad(a, b, w)" => {
            let got = geometry::thin_quad(vec3(&args["a"], "a")?, vec3(&args["b"], "b")?, a("w")?);
            close_rows(&flat3(&got), arr("out").f64(), 3)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn vectors() {
    check_calls("geometry", CALLS, run);
}
