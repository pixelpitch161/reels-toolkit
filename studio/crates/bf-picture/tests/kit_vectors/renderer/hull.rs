//! `scipy.spatial.ConvexHull(pts).vertices` (`picture_kit/hull.npz`): the corners in scipy's order, exactly.

use super::*;
use bf_picture::hull;

pub(super) const CALLS: &Calls = &[("scipy.spatial.ConvexHull(pts).vertices", 93)];

pub(super) fn run(v: &Vectors, case: &Case) -> Found {
    match case.call.as_str() {
        "scipy.spatial.ConvexHull(pts).vertices" => {
            let pts = rows2(v.array(case, "pts"))?;
            // Qhull's own order, not the plain hull `convex_hull` falls back to: a reference case must be one
            // the port follows all the way.
            let order =
                hull::qhull_order(&pts).map_err(|why| format!("the port does not follow Qhull here: {why:?}"))?;
            let whole = hull::convex_hull(&pts);
            if whole.fallback.is_some() || whole.vertices != order {
                return Err(format!("convex_hull has {whole:?}, qhull_order {order:?}"));
            }
            let got: Vec<i32> = order.iter().map(|&k| k as i32).collect();
            same(&got, v.array(case, "out").i32(), |a, b| a == b).map(|()| 0.0)
        }
        other => Err(format!("a call this test does not know: {other}")),
    }
}

#[test]
fn vectors() {
    check_calls("hull", CALLS, run);
}
