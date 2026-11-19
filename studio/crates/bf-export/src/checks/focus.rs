//! `kit/checks.py` `focus()`: the subject is sharp on every frame.
//!
//! For every frame of the video the camera's depth of field is asked about the subject's points: the ring's
//! eight corners, both balls (floor and top), each look's own points, what flies through the ring and the
//! shards. The circle of confusion of each (`reelkit.lens.coc_map(depth, focus depth, 26.0, 0.17)`, in pixels)
//! must be 0. Only the camera maths runs ([`Picture::focus_points`]); no frame is drawn.

use std::panic::{AssertUnwindSafe, catch_unwind};

use bf_picture::config::FPS;
use bf_picture::ring::Picture;

use super::{CheckResult, panic_text};

const NAME: &str = "focus";
const TITLE: &str = "Focus";

/// The focus check: passes when the largest circle of confusion over all frames is 0.
///
/// Numbers: `worst_coc` (pixels), `frames` and `points` (how many were looked at) and, when the check fails,
/// `worst_t` (the time of the worst point).
pub fn focus(pic: &Picture) -> CheckResult {
    let nf = pic.nf();
    if nf == 0 {
        return CheckResult::failed(
            NAME,
            format!("{TITLE}: the video has no frames, so nothing was checked; build the picture again."),
        );
    }
    // worst = (0.0, None, None): the largest circle so far, its time and what it is on
    let mut worst: (f64, Option<(f64, String)>) = (0.0, None);
    let mut points = 0usize;
    for f in 0..nf {
        let t = f as f64 / f64::from(FPS);
        let pts = match catch_unwind(AssertUnwindSafe(|| pic.focus_points(f))) {
            Ok(pts) => pts,
            Err(payload) => {
                return CheckResult::failed(
                    NAME,
                    format!(
                        "{TITLE}: the camera of frame {f} (at {t:.2} s) could not be worked out ({}), so this \
                         check did not run; this is a fault in the app, not in your fight.",
                        panic_text(payload).replace('\n', " ").trim_end_matches('.')
                    ),
                );
            }
        };
        points += pts.len();
        if let Some((_, name)) = pts.iter().find(|(c, _)| !c.is_finite()) {
            return CheckResult::new(
                NAME,
                false,
                format!(
                    "{TITLE}: the sharpness of the {name} at {t:.2} s is not a number, so the frame cannot be \
                     trusted; this is a fault in the app, not in your fight."
                ),
                &[("frames", nf as f64), ("worst_t", t)],
            );
        }
        // i = int(np.argmax(c)); if c[i] > worst[0]: worst = (c[i], t, names[i])
        let mut top: Option<&(f64, String)> = None;
        for p in &pts {
            if top.is_none_or(|q| p.0 > q.0) {
                top = Some(p);
            }
        }
        if let Some((c, name)) = top
            && *c > worst.0
        {
            worst = (*c, Some((t, name.clone())));
        }
    }
    let mut r = CheckResult::new(
        NAME,
        worst.0 == 0.0,
        String::new(),
        &[("worst_coc", worst.0), ("frames", nf as f64), ("points", points as f64)],
    );
    r.summary = match &worst.1 {
        None => format!(
            "{TITLE}: the ring, both fighters and everything they throw are sharp on all {nf} frames (the largest \
             blur is 0 px)."
        ),
        Some((t, name)) => {
            r.put("worst_t", *t);
            format!(
                "{TITLE}: the {name} is blurred by {:.3} px at {t:.2} s where the subject must be sharp (0 px) on \
                 every frame; this comes from the camera or the fighter's look, not from your fight.",
                worst.0
            )
        }
    };
    r
}
