//! `reelkit/safezone.py`: are captions, labels and HUD panels inside the part of the frame that TikTok,
//! Instagram Reels and YouTube Shorts leave visible?
//!
//! The safe zone is internal x 20 to 340, y 96 to 500, less the apps' button column (x from 300 on, below
//! y 290): `bf_picture::config::{SAFE, SAFE_BUTTONS, safe_overshoot}`. The check reads the boxes the picture
//! filed while it drew a frame every 0.25 s (its [`DrawLog`], the kit's `reelkit.ui.DRAWN`) and reports every box
//! that sticks out. A label or panel partly off the frame is sliding in or out and is skipped; a caption never
//! is. A caption that covers a label or a panel is counted and named, and does not fail the check (the kit
//! prints it and still passes).

use bf_core::py::round_half_even;
use bf_picture::config::{H, SAFE, SAFE_BUTTONS, W, safe_overshoot};
use bf_picture::ui::{DrawLog, Kind};

use super::CheckResult;

/// `kit.checks.safezone`'s default step: a frame every quarter of a second.
pub const STEP: f64 = 0.25;

const NAME: &str = "safezone";
const TITLE: &str = "Safe zone";

/// The times the check looks at and the frame of each: `np.arange(0.0, nf / fps, 0.25)` with
/// `int(round(t * fps))` (Python's rounding: halves go to the even frame). 244 frames for a 61 s video.
pub fn frames(nf: usize, fps: u32) -> Vec<(f64, usize)> {
    if fps == 0 {
        return Vec::new();
    }
    let stop = nf as f64 / f64::from(fps);
    // np.arange: ceil((stop - start) / step) values, start + i * step
    let n = (stop / STEP).ceil() as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 * STEP;
            (t, round_half_even(t * f64::from(fps)) as usize)
        })
        .collect()
}

/// Something that sticks out of the safe zone: one entry of `check()`'s `outside`, by kind and text.
#[derive(Clone, Debug, PartialEq)]
pub struct Outside {
    pub kind: Kind,
    /// The caption's or label's text, the panel's name.
    pub info: String,
    /// In how many of the frames looked at it sticks out.
    pub n: usize,
    /// By how many pixels at most.
    pub worst: i32,
    /// The first and the last time it sticks out, and the time it does so the most.
    pub t_first: f64,
    pub t_last: f64,
    pub t_worst: f64,
    /// Its box `(x0, y0, x1, y1)` at `t_worst`.
    pub at: (i32, i32, i32, i32),
}

/// A caption that covers a label or a panel: one entry of `check()`'s `overlaps`.
#[derive(Clone, Debug, PartialEq)]
pub struct Overlap {
    /// The caption line.
    pub caption: String,
    /// What it covers.
    pub kind: Kind,
    pub info: String,
    /// In how many frames, and when first.
    pub n: usize,
    pub t_first: f64,
}

/// What `check()` returns: `(outside, overlaps, tracked)`, each list in the order the kit's dict fills.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Report {
    pub outside: Vec<Outside>,
    pub overlaps: Vec<Overlap>,
    /// How many boxes were looked at.
    pub tracked: usize,
}

/// `check(mod, times)` on the logs of frames already drawn: `logs` holds each time looked at and what the
/// picture filed while it drew that time's frame.
pub fn check(logs: &[(f64, DrawLog)]) -> Report {
    let mut report = Report::default();
    for (t, log) in logs {
        let t = *t;
        report.tracked += log.boxes.len();
        for b in &log.boxes {
            if b.kind != Kind::Caption && (b.x0 < 0 || b.y0 < 0 || b.x1 > W as i32 || b.y1 > H as i32) {
                continue; // a panel or label partly off the frame: sliding in or out
            }
            let over = safe_overshoot(b.x0, b.y0, b.x1, b.y1);
            if over > 0 {
                let i = match report.outside.iter().position(|o| o.kind == b.kind && o.info == b.info) {
                    Some(i) => i,
                    None => {
                        report.outside.push(Outside {
                            kind: b.kind,
                            info: b.info.clone(),
                            n: 0,
                            worst: 0,
                            t_first: t,
                            t_last: t,
                            t_worst: t,
                            at: (b.x0, b.y0, b.x1, b.y1),
                        });
                        report.outside.len() - 1
                    }
                };
                let d = &mut report.outside[i];
                d.n += 1;
                d.t_last = t;
                if over > d.worst {
                    (d.worst, d.at, d.t_worst) = (over, (b.x0, b.y0, b.x1, b.y1), t);
                }
            }
        }
        for c in log.boxes.iter().filter(|b| b.kind == Kind::Caption) {
            for o in log.boxes.iter().filter(|b| b.kind != Kind::Caption) {
                if c.x0 < o.x1 && o.x0 < c.x1 && c.y0 < o.y1 && o.y0 < c.y1 {
                    let found = report
                        .overlaps
                        .iter_mut()
                        .find(|v| v.caption == c.info && v.kind == o.kind && v.info == o.info);
                    match found {
                        Some(v) => v.n += 1,
                        None => report.overlaps.push(Overlap {
                            caption: c.info.clone(),
                            kind: o.kind,
                            info: o.info.clone(),
                            n: 1,
                            t_first: t,
                        }),
                    }
                }
            }
        }
    }
    report
}

/// A caption's, label's or panel's text for a sentence: at most 24 characters, on one line.
fn shown(info: &str) -> String {
    info.chars()
        .take(24)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// The safe-zone check on the draw logs of the frames looked at (`(time, log)`, see [`frames`]): passes when
/// nothing sticks out of the safe zone.
///
/// It fails too when there is nothing to check: no frames, or frames in which the picture filed no box at all
/// (the kit prints a warning there and passes; every video of this series draws its HUD through the logged
/// calls, so nothing filed means nothing was checked).
///
/// Numbers: `frames_checked`, `tracked` (boxes looked at), `outside` (things that stick out), `overlaps`
/// (captions covering a label or panel) and, when something sticks out, `worst_px`.
pub fn safezone(logs: &[(f64, DrawLog)]) -> CheckResult {
    let fail = |text: &str| CheckResult::failed(NAME, format!("{TITLE}: {text}"));
    if logs.is_empty() {
        return fail("no frames were given to look at, so nothing was checked; draw the frames again.");
    }
    let report = check(logs);
    let mut numbers = vec![
        ("frames_checked", logs.len() as f64),
        ("tracked", report.tracked as f64),
        ("outside", report.outside.len() as f64),
        ("overlaps", report.overlaps.len() as f64),
    ];
    if report.tracked == 0 {
        let summary = format!(
            "{TITLE}: the picture drew no caption, label or panel in the {} frames looked at, so there was \
             nothing to check; this is a fault in the app, not in your fight.",
            logs.len()
        );
        return CheckResult::new(NAME, false, summary, &numbers);
    }
    // the kit lists the worst first (sorted by -worst, the first found of equals first)
    let mut worst: Option<&Outside> = None;
    for o in &report.outside {
        if worst.is_none_or(|w| o.worst > w.worst) {
            worst = Some(o);
        }
    }
    let summary = match worst {
        None => {
            let covered = match report.overlaps.first() {
                None => String::new(),
                Some(v) => format!(
                    ", though the caption \"{}\" covers the {} \"{}\" from {:.1} s{}",
                    shown(&v.caption),
                    v.kind.as_str(),
                    shown(&v.info),
                    v.t_first,
                    match report.overlaps.len() {
                        1 => String::new(),
                        n => format!(" (and {} more such overlaps)", n - 1),
                    }
                ),
            };
            format!(
                "{TITLE}: all {} captions, labels and panels drawn in the {} frames looked at are inside the safe \
                 area{covered}.",
                report.tracked,
                logs.len()
            )
        }
        Some(o) => {
            numbers.push(("worst_px", f64::from(o.worst)));
            let (x0, y0, x1, y1) = o.at;
            format!(
                "{TITLE}: the {} \"{}\" sticks {} px out of the safe area at {:.1} s (it spans x {x0} to {x1}, \
                 y {y0} to {y1}; outside in {} of the frames looked at){}; keep it inside x {} to {} and y {} to \
                 {}, clear of the apps' buttons (x from {} on, below y {}).",
                o.kind.as_str(),
                shown(&o.info),
                o.worst,
                o.t_worst,
                o.n,
                match report.outside.len() {
                    1 => String::new(),
                    n => format!(", and {} more things stick out too", n - 1),
                },
                SAFE.0,
                SAFE.2,
                SAFE.1,
                SAFE.3,
                SAFE_BUTTONS.0,
                SAFE_BUTTONS.1
            )
        }
    };
    CheckResult::new(NAME, worst.is_none(), summary, &numbers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(boxes: &[(i32, i32, i32, i32, Kind, &str)]) -> DrawLog {
        let mut log = DrawLog::new();
        for &(x0, y0, x1, y1, kind, info) in boxes {
            log.note(x0, y0, x1, y1, kind, info);
        }
        log
    }

    #[test]
    fn the_frames_are_the_kits() {
        let f = frames(1830, 30);
        assert_eq!(f.len(), 244);
        // int(round(t * 30)): 7.5 -> 8, 22.5 -> 22, 37.5 -> 38
        assert_eq!(
            &f[..6],
            &[(0.0, 0), (0.25, 8), (0.5, 15), (0.75, 22), (1.0, 30), (1.25, 38)]
        );
        assert_eq!(f[243], (60.75, 1822));
        assert!(frames(0, 30).is_empty() && frames(10, 0).is_empty());
        // a video that ends between two steps: np.arange(0, 0.3, 0.25) has two values
        assert_eq!(frames(9, 30).len(), 2);
    }

    #[test]
    fn inside_passes_and_outside_is_named() {
        let inside = log(&[
            (100, 100, 200, 120, Kind::Caption, "FLUTE"),
            (30, 400, 290, 480, Kind::Panel, "fight hud"),
        ]);
        let r = safezone(&[(0.0, inside.clone()), (0.25, inside.clone())]);
        assert!(r.pass, "{}", r.summary);
        assert_eq!((r.numbers["tracked"], r.numbers["frames_checked"]), (4.0, 2.0));
        // into the button column: 10 px in x, 30 px in y: 10 over
        let column = log(&[(250, 300, 310, 320, Kind::Label, "LV 2")]);
        let r = safezone(&[(0.0, inside.clone()), (0.5, column.clone()), (0.75, column)]);
        assert!(!r.pass);
        assert_eq!((r.numbers["worst_px"], r.numbers["outside"]), (10.0, 1.0));
        assert!(
            r.summary.contains("the label \"LV 2\" sticks 10 px out"),
            "{}",
            r.summary
        );
        assert!(
            r.summary.contains("at 0.5 s") && r.summary.contains("in 2 of the frames"),
            "{}",
            r.summary
        );
    }

    #[test]
    fn sliding_in_is_skipped_but_never_for_a_caption() {
        let sliding = log(&[
            (-40, 100, 60, 120, Kind::Label, "slides"),
            (300, 600, 380, 630, Kind::Panel, "p"),
        ]);
        assert!(safezone(&[(0.0, sliding)]).pass);
        let caption = log(&[(-40, 100, 60, 120, Kind::Caption, "TOO LONG A LINE")]);
        let r = safezone(&[(0.0, caption)]);
        assert!(!r.pass && r.numbers["worst_px"] == 60.0, "{}", r.summary);
    }

    #[test]
    fn a_covered_label_is_counted_and_passes() {
        let both = log(&[
            (100, 100, 200, 120, Kind::Caption, "K.O."),
            (150, 110, 220, 130, Kind::Label, "HP"),
        ]);
        let r = safezone(&[(1.0, both.clone()), (1.25, both)]);
        assert!(r.pass && r.numbers["overlaps"] == 1.0, "{}", r.summary);
        assert!(
            r.summary.contains("covers the label \"HP\" from 1.0 s"),
            "{}",
            r.summary
        );
        let report = check(&[(
            1.0,
            log(&[(0, 0, 5, 5, Kind::Caption, "a"), (0, 0, 5, 5, Kind::Panel, "")]),
        )]);
        // both boxes are outside (the top-left corner), each under its own kind and text
        assert_eq!((report.overlaps[0].n, report.outside.len(), report.tracked), (1, 2, 2));
    }

    #[test]
    fn nothing_to_check_fails_in_words() {
        let r = safezone(&[]);
        assert!(!r.pass && r.summary.contains("no frames"), "{}", r.summary);
        let r = safezone(&[(0.0, DrawLog::new())]);
        assert!(
            !r.pass && r.summary.contains("no caption, label or panel"),
            "{}",
            r.summary
        );
    }
}
