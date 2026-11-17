//! The render checks of an Engine 2 video (milestone E5): the current engine's safe zone, levels, band, loudness
//! and loop checks on Engine 2's UI layer, soundtrack and frames, plus Engine 2's own focus (the director's framing
//! rule) and K.O. window (in video time). There is no flash check on Engine 2 (Aedan, 2026-11-17).

use std::sync::atomic::{AtomicBool, Ordering};

use bf_content::Sound2;
use bf_core::show::Show;
use bf_picture::img::{Img, Rgb8};
use bf_story::BeatSheet;
use bf2_direct::project::{ball_box, ball_centre};
use bf2_direct::{CameraTrack, TimeMap};
use bf2_render::Ui2;
use bf2_sim::Recording2;

use super::{BandPlan, CheckResult, as_wav16, band, levels, loop_audio, loop_picture, loopcheck, loudness, safezone};

/// The frames per second of the video.
const FPS: f64 = 30.0;
/// The video's frames.
const NF: usize = 1830;
/// The video frame.
const FRAME: [f64; 4] = [0.0, 0.0, 1080.0, 1920.0];
/// Engine 2's framing safe zone for a ball's centre (E4's rule), in pixels of the 1080 x 1920 frame.
pub const SAFE2: [f64; 4] = [60.0, 288.0, 1020.0, 1500.0];
/// The share of frames on which each ball's centre must be inside [`SAFE2`].
pub const INSIDE_SAFE: f64 = 0.98;
/// Focus is checked to this long after the K.O.
pub const AFTER_KO: f64 = 2.5;

/// How far `(x, y)` lies outside `r`, in pixels (0 inside).
fn outside(x: f64, y: f64, r: [f64; 4]) -> f64 {
    (r[0] - x).max(r[1] - y).max(x - r[2]).max(y - r[3]).max(0.0)
}

/// `focus`: from FIGHT! to the K.O. + 2.5 s, each ball's projected circle (with the frame's shake) inside the
/// 1080 x 1920 frame on every frame, and its centre inside [`SAFE2`] on at least 98 % of frames. Numbers:
/// `inside_frame` and `inside_safe` (the worse ball's share of frames) and `worst_t` (the frame whose ball centre
/// lies furthest out of the safe zone).
pub fn focus2(rec: &Recording2, map: &TimeMap, cams: &CameraTrack, show: &Show) -> CheckResult {
    const NAME: &str = "focus";
    let f0 = (show.grid.t_fight * FPS).ceil() as usize;
    let f1 = (((show.t_ko + AFTER_KO) * FPS).floor() as usize).min(cams.poses.len().saturating_sub(1));
    if f1 < f0 {
        return CheckResult::failed(
            NAME,
            "Focus: the fight has no frames to look at; choose another seed.".into(),
        );
    }
    let n = (f1 - f0 + 1) as f64;
    let (mut in_frame, mut in_safe) = ([0usize; 2], [0usize; 2]);
    let (mut worst, mut worst_t) = (0.0, show.grid.t_fight);
    for f in f0..=f1 {
        let t = f as f64 / FPS;
        let [dx, dy] = cams.shake.get(f).copied().unwrap_or([0.0; 2]);
        let balls = bf2_direct::camera::balls_at(rec, map, t);
        for (k, b) in balls.iter().enumerate() {
            let Some(bx) = ball_box(&cams.poses[f], ball_centre(*b)) else {
                // behind the camera: neither in the picture nor in the safe zone
                if worst < f64::INFINITY {
                    (worst, worst_t) = (f64::INFINITY, t);
                }
                continue;
            };
            let bx = [bx[0] + dx, bx[1] + dy, bx[2] + dx, bx[3] + dy];
            if bf2_direct::project::inside(bx, FRAME) {
                in_frame[k] += 1;
            }
            let (cx, cy) = ((bx[0] + bx[2]) / 2.0, (bx[1] + bx[3]) / 2.0);
            let out = outside(cx, cy, SAFE2);
            if out == 0.0 {
                in_safe[k] += 1;
            } else if out > worst {
                (worst, worst_t) = (out, t);
            }
        }
    }
    let share = |c: [usize; 2]| c[0].min(c[1]) as f64 / n;
    let (frame_share, safe_share) = (share(in_frame), share(in_safe));
    let pass = frame_share >= 1.0 && safe_share >= INSIDE_SAFE;
    let summary = if pass {
        format!(
            "Focus: both balls stay in the picture on every frame from FIGHT! to just after the K.O., their centres \
             inside the safe zone on {:.1} % of frames.",
            100.0 * safe_share
        )
    } else {
        format!(
            "Focus: a ball leaves the picture or its safe zone (in the picture on {:.1} % of frames, centred in the \
             safe zone on {:.1} %, worst at {worst_t:.2} s); choose another seed.",
            100.0 * frame_share,
            100.0 * safe_share
        )
    };
    CheckResult::new(
        NAME,
        pass,
        summary,
        &[
            ("inside_frame", frame_share),
            ("inside_safe", safe_share),
            ("worst_t", worst_t),
        ],
    )
}

/// `ko_window`: the K.O. in video time (`TimeMap::ko_video_t`, after slow-mo) inside the beat sheet's window (49 to
/// 53 s).
pub fn ko_window(ko_video: Option<f64>, sheet: &BeatSheet) -> CheckResult {
    const NAME: &str = "ko_window";
    let (a, b) = sheet.t_ko;
    match ko_video {
        Some(t) if (a..=b).contains(&t) => CheckResult::new(
            NAME,
            true,
            format!("K.O. window: the K.O. comes at {t:.2} s, inside {a:.0} to {b:.0} s."),
            &[("t_ko", t)],
        ),
        Some(t) => CheckResult::new(
            NAME,
            false,
            format!("K.O. window: the K.O. comes at {t:.2} s, outside {a:.0} to {b:.0} s; choose another seed."),
            &[("t_ko", t)],
        ),
        None => CheckResult::new(
            NAME,
            false,
            "K.O. window: this fight has no K.O., so it has no video; choose another seed.".into(),
            &[],
        ),
    }
}

/// A 1080 x 1920 frame as the loop check reads the current engine's: 3 x 3 boxes averaged to 360 x 640.
fn third(img: &Rgb8) -> Rgb8 {
    let (w, h) = (img.w / 3, img.h / 3);
    let mut out: Rgb8 = Img::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                let mut s = 0u32;
                for dy in 0..3 {
                    for dx in 0..3 {
                        s += u32::from(img.px(3 * x + dx, 3 * y + dy)[c]);
                    }
                }
                out.px_mut(x, y)[c] = ((s + 4) / 9) as u8;
            }
        }
    }
    out
}

/// Every check an Engine 2 video passes before its render: `safezone` on the UI layer's draw log every 0.25 s,
/// `levels`, `band`, `loudness` and `loop_audio` on the 16-bit master, `loop_picture` on the last 5 s and the
/// first second of `frames` (composited 1080 x 1920 frames, read at 360 x 640), then `focus` and `ko` as given.
/// No flash check. `None` once `cancel` is set (looked at between the checks and before each frame).
pub fn pre_render2(
    ui: &Ui2,
    frames: &mut dyn FnMut(usize) -> Rgb8,
    s: &Sound2,
    focus: CheckResult,
    ko: CheckResult,
    plan: &BandPlan,
    cancel: &AtomicBool,
) -> Option<Vec<CheckResult>> {
    let cancelled = || cancel.load(Ordering::SeqCst);
    let mut out = Vec::new();
    let mut logs = Vec::new();
    for (t, f) in safezone::frames(NF, 30) {
        if cancelled() {
            return None;
        }
        logs.push((t, ui.draw(f).1));
    }
    out.push(safezone(&logs));
    let master = as_wav16(&s.soundtrack.master);
    let sounds = [
        (plan.sounds[0].0.as_str(), plan.sounds[0].1),
        (plan.sounds[1].0.as_str(), plan.sounds[1].1),
    ];
    let bed: Vec<&str> = plan.bed_buses.iter().map(String::as_str).collect();
    let sound: [&dyn Fn() -> CheckResult; 3] = [
        &|| levels(&master),
        &|| band(&s.soundtrack, &s.show, sounds, &bed),
        &|| loudness(&master, &s.show),
    ];
    for check in sound {
        if cancelled() {
            return None;
        }
        out.push(check());
    }
    let nt = (loopcheck::TAIL_S * FPS) as usize;
    let nh = (loopcheck::HEAD_S * FPS) as usize;
    let mut read = |range: std::ops::Range<usize>| -> Option<Vec<Rgb8>> {
        let mut v = Vec::with_capacity(range.len());
        for f in range {
            if cancelled() {
                return None;
            }
            v.push(third(&frames(f)));
        }
        Some(v)
    };
    let tail = read(NF - nt..NF)?;
    let head = read(0..nh)?;
    out.push(loop_picture(&tail, &head));
    out.push(loop_audio(&master));
    out.push(focus);
    out.push(ko);
    Some(out)
}

#[cfg(test)]
#[path = "engine2_tests.rs"]
mod tests;
