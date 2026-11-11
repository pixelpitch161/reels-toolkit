//! `reelkit/loopcheck.py`: does the video loop cleanly at the file boundary?
//!
//! The rule: the loop point is the file boundary. The last frame flows straight into frame 0, and the moment the
//! video starts over lands on frame 0, never seconds before the end.
//!
//! Picture ([`loop_picture`], on the last 5 s and the first second; `mad` is the mean absolute RGB difference of
//! two frames):
//! - **seam**: the change from the last frame to frame 0 is at most 2.5 times a normal frame-to-frame change;
//! - **restart**: no one-off jump above 6 times normal in the last 5 s (jumps that recur at a similar size are
//!   beat pulses, not a reset);
//! - **repeat**: no frame of the last 5 s (its final second left out) already looks like frame 0.
//!
//! Sound ([`loop_audio`]): the level of the last 20 ms is within 6 dB of the first 20 ms, and the jump from the
//! last sample to the first is no click.

use bf_picture::Rgb8;

use super::{CheckResult, SRF, level_db, median, mono, unusable};
use bf_core::py::py_max;

/// `TAIL_S`: the seconds before the end the picture check looks at.
pub const TAIL_S: f64 = 5.0;
/// `HEAD_S`: the seconds after the start it looks at.
pub const HEAD_S: f64 = 1.0;
/// The frames a second (`reelkit.config.FPS`).
const FPS: usize = bf_picture::config::FPS as usize;

const PICTURE: &str = "loop_picture";
const PICTURE_TITLE: &str = "Loop point (picture)";
const AUDIO: &str = "loop_audio";
const AUDIO_TITLE: &str = "Loop point (sound)";

/// `mad(a, b)`: the mean absolute difference of two pictures' samples. The sum is a whole number, so the mean is
/// numpy's to the bit. NaN for pictures of different sizes or without samples.
pub fn mad(a: &Rgb8, b: &Rgb8) -> f64 {
    if (a.w, a.h, a.c) != (b.w, b.h, b.c) || a.data.is_empty() {
        return f64::NAN;
    }
    let sum: u64 = a
        .data
        .iter()
        .zip(&b.data)
        .map(|(&x, &y)| u64::from(x.abs_diff(y)))
        .sum();
    sum as f64 / a.data.len() as f64
}

/// `steps` of `check_picture`: the [`mad`] of each frame of `tail` to the next, then of each frame of `head` to
/// the next.
pub fn steps(tail: &[Rgb8], head: &[Rgb8]) -> Vec<f64> {
    let pairs = |frames: &[Rgb8]| frames.windows(2).map(|w| mad(&w[0], &w[1])).collect::<Vec<f64>>();
    let mut out = pairs(tail);
    out.extend(pairs(head));
    out
}

/// The picture half of the loop check: `check_picture(tail, head, ..)`. `tail` holds the last frames of the
/// video in order (the kit gives the last 5 s) and `head` its first (the first second), all of one size.
///
/// Numbers: `seam`, `normal` (a normal step: the median of the steps, at least 0.5), `ratio` (seam / normal),
/// `big_jumps` (steps above 4 times normal, in `tail` and `head`), `nearest_to_frame_0` (the smallest `mad` to
/// frame 0 among the tail's frames before its last second) and `fails`.
pub fn loop_picture(tail: &[Rgb8], head: &[Rgb8]) -> CheckResult {
    let fail = |text: &str| CheckResult::failed(PICTURE, format!("{PICTURE_TITLE}: {text}"));
    let (Some(last), Some(first)) = (tail.last(), head.first()) else {
        return fail(
            "the last or the first frames of the video are missing, so the loop could not be checked; draw the frames again.",
        );
    };
    if tail.len() < 2 {
        return fail(
            "fewer than two frames of the video's end were given, so there is no normal frame-to-frame change to compare the loop with; draw the frames again.",
        );
    }
    let size = (first.w, first.h, first.c);
    if first.data.is_empty() || tail.iter().chain(head).any(|f| (f.w, f.h, f.c) != size) {
        return fail(
            "the frames given are empty or not all of one size, so they cannot be compared; draw the frames again.",
        );
    }
    let steps = steps(tail, head);
    // normal = max(float(np.median(steps)), 0.5); seam = mad(tail[-1], head[0])
    let normal = py_max(median(&steps), 0.5);
    let seam = mad(last, first);
    let r = seam / normal;
    let mut fails = 0usize;
    let mut faults = Vec::new();
    // how long before the end of the video tail frame `j` is shown (the tail ends where the video ends)
    let before_end = |j: usize| (tail.len() - j) as f64 / FPS as f64;
    // ok = r <= 2.5 (a ratio that is not a number is not ok)
    if r > 2.5 || r.is_nan() {
        fails += 1;
        faults.push(format!(
            "the picture changes {r:.1} times as much from the last frame to the first as between two normal \
             frames where at most 2.5 times is allowed, so make the last frame flow into the first"
        ));
    }
    // big = [(i, s / normal) for i, s in enumerate(steps) if s / normal > 4]; jumps: those of the tail
    let big: Vec<(usize, f64)> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (i, s / normal))
        .filter(|&(_, q)| q > 4.0)
        .collect();
    let ntail = tail.len() - 1;
    // a reset is a one-off; jumps that recur at a similar size (beat pulses) are rhythm
    let recurring = |q: f64| big.iter().filter(|&&(_, q2)| q / 1.6 <= q2 && q2 <= q * 1.6).count() >= 3;
    for &(i, q) in big.iter().filter(|&&(i, _)| i < ntail) {
        if recurring(q) || q <= 6.0 {
            continue;
        }
        fails += 1;
        faults.push(format!(
            "the picture jumps {q:.0} times as much as normal {:.2} s before the end, which looks like the video \
             starting over early, so move that cut or reset onto the first frame",
            before_end(i + 1)
        ));
    }
    // d0 = [mad(f, head[0]) for f in tail[:-FPS]]: the final second is left out (a held end pose is fine)
    let d0: Vec<f64> = tail[..tail.len().saturating_sub(FPS)]
        .iter()
        .map(|f| mad(f, first))
        .collect();
    let mut nearest = f64::NAN;
    if !d0.is_empty() {
        // np.argmin: the first of the smallest
        let mut i = 0;
        for (j, v) in d0.iter().enumerate() {
            if *v < d0[i] {
                i = j;
            }
        }
        nearest = d0[i];
        if r > 1.5 && d0[i] < 0.5 * seam && d0[i] < 3.0 {
            fails += 1;
            faults.push(format!(
                "the frame {:.2} s before the end already looks like the first frame, so the opening plays \
                 before the video ends; make the end run up to the first frame instead",
                before_end(i)
            ));
        }
    }
    let pass = fails == 0;
    let summary = if pass {
        format!(
            "{PICTURE_TITLE}: the last frame flows into the first (the change is {r:.1} times a normal \
             frame-to-frame change, at most 2.5 allowed) and nothing in the last {:.0} s looks like the video \
             starting over early.",
            tail.len() as f64 / FPS as f64
        )
    } else {
        format!("{PICTURE_TITLE}: {}.", faults.join("; and "))
    };
    CheckResult::new(
        PICTURE,
        pass,
        summary,
        &[
            ("seam", seam),
            ("normal", normal),
            ("ratio", r),
            ("big_jumps", big.len() as f64),
            ("nearest_to_frame_0", nearest),
            ("fails", fails as f64),
        ],
    )
}

/// The sound half of the loop check: `check_audio` on a master as its 16-bit file holds it (see the module
/// notes of [`super`]).
///
/// Numbers: `last_20ms_db`, `first_20ms_db`, `step_db` (their difference), `jump` (from the last sample to the
/// first), `typical_step` (the median sample step of the first second) and `fails`.
pub fn loop_audio(master: &[[f64; 2]]) -> CheckResult {
    if let Some(r) = unusable(AUDIO, AUDIO_TITLE, master) {
        return r;
    }
    let x = mono(master);
    // n = int(0.02 * sr); a, b = x[-n:], x[:n]
    let n = ((0.02 * SRF) as usize).min(x.len());
    let (a, b) = (&x[x.len() - n..], &x[..n]);
    let (la, lb) = (level_db(a, 1e-9), level_db(b, 1e-9));
    let step = (la - lb).abs();
    // typical = np.median(np.abs(np.diff(x[:sr]))); jump = abs(x[-1] - x[0])
    let second = &x[..x.len().min(SRF as usize)];
    let diffs: Vec<f64> = second.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let typical = median(&diffs);
    let jump = (x[x.len() - 1] - x[0]).abs();
    // click = jump > max(0.08, 12 * typical)
    let click = jump > py_max(0.08, 12.0 * typical);
    let uneven = step > 6.0;
    let fails = usize::from(uneven) + usize::from(click);
    let mut faults = Vec::new();
    if uneven {
        faults.push(format!(
            "the last 20 ms are at {la:.1} dBFS and the first 20 ms at {lb:.1} dBFS, {step:.1} dB apart where at \
             most 6 dB is allowed, so the loudness would step when the video loops; bring the end and the start \
             to the same level"
        ));
    }
    if click {
        faults.push(format!(
            "the sound jumps by {jump:.3} from the last sample to the first, which is heard as a click when the \
             video loops; let the sound of the end run on into the start"
        ));
    }
    let summary = if fails == 0 {
        format!(
            "{AUDIO_TITLE}: the sound runs through the loop point without a step or a click (the last and the \
             first 20 ms are {step:.1} dB apart, at most 6 allowed, and the samples meet within {jump:.3})."
        )
    } else {
        format!("{AUDIO_TITLE}: {}.", faults.join("; and "))
    };
    CheckResult::new(
        AUDIO,
        fails == 0,
        summary,
        &[
            ("last_20ms_db", la),
            ("first_20ms_db", lb),
            ("step_db", step),
            ("typical_step", typical),
            ("jump", jump),
            ("fails", fails as f64),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(v: u8) -> Rgb8 {
        Rgb8::filled(8, 8, &[v, v, v])
    }

    /// 150 frames that drift by one level a frame, then the head.
    fn drift() -> (Vec<Rgb8>, Vec<Rgb8>) {
        ((50..200).map(grey).collect(), (200..230).map(grey).collect())
    }

    #[test]
    fn mad_is_the_mean_absolute_difference() {
        assert_eq!(mad(&grey(10), &grey(13)), 3.0);
        assert_eq!(mad(&grey(13), &grey(10)), 3.0);
        assert!(mad(&grey(1), &Rgb8::filled(2, 2, &[1, 1, 1])).is_nan());
    }

    #[test]
    fn a_smooth_loop_passes() {
        let (tail, head) = drift();
        let r = loop_picture(&tail, &head);
        assert!(r.pass, "{}", r.summary);
        assert_eq!(
            (r.numbers["seam"], r.numbers["normal"], r.numbers["ratio"]),
            (1.0, 1.0, 1.0)
        );
        assert_eq!(r.numbers["nearest_to_frame_0"], 200.0 - 169.0);
    }

    #[test]
    fn each_fault_is_named() {
        // a hard cut at the seam
        let (tail, mut head) = drift();
        head[0] = grey(0);
        let r = loop_picture(&tail, &head);
        assert!(
            !r.pass && r.summary.contains("from the last frame to the first"),
            "{}",
            r.summary
        );
        // a one-off jump 1 s before the end: frame 120 of 150 is the first after it
        let (mut tail, head) = drift();
        for f in &mut tail[120..] {
            *f = grey(240);
        }
        let r = loop_picture(&tail, &head);
        assert!(!r.pass && r.summary.contains("1.00 s before the end"), "{}", r.summary);
        // the opening shown 3 s before the end while the seam itself is a cut
        let mut tail: Vec<Rgb8> = (0..150).map(|_| grey(100)).collect();
        let head: Vec<Rgb8> = (0..30).map(|_| grey(10)).collect();
        tail[60] = grey(10);
        let r = loop_picture(&tail, &head);
        assert!(
            !r.pass && r.summary.contains("3.00 s before the end already looks like"),
            "{}",
            r.summary
        );
        // the seam, the jump into that frame and out of it, and the repeat
        assert_eq!(r.numbers["fails"], 4.0);
    }

    #[test]
    fn pulses_that_recur_are_not_a_reset() {
        // a bright frame every 10 frames: big jumps, but they recur
        let tail: Vec<Rgb8> = (0..150).map(|i| grey(if i % 10 == 5 { 100 } else { 50 })).collect();
        let head: Vec<Rgb8> = (0..30).map(|_| grey(50)).collect();
        let r = loop_picture(&tail, &head);
        assert!(r.pass, "{}", r.summary);
        assert_eq!(r.numbers["big_jumps"], 30.0);
    }

    #[test]
    fn unusual_pictures_fail_in_words() {
        let (tail, head) = drift();
        for (t, h, word) in [
            (&tail[..0], &head[..], "missing"),
            (&tail[..], &head[..0], "missing"),
            (&tail[..1], &head[..], "fewer than two"),
            (&tail[..], &[Rgb8::filled(2, 2, &[0, 0, 0])][..], "one size"),
        ] {
            let r = loop_picture(t, h);
            assert!(!r.pass && r.summary.contains(word), "{}", r.summary);
        }
        // a short tail (under a second) has no frame to compare with frame 0: no panic
        assert!(loop_picture(&tail[140..], &head).pass);
    }

    #[test]
    fn sound_across_the_seam() {
        let wave = |n: usize| -> Vec<[f64; 2]> {
            (0..n)
                .map(|i| {
                    // 100 Hz: a whole number of cycles in every second
                    let v = 0.3 * bf_core::dmath::sin(std::f64::consts::TAU * 100.0 * i as f64 / SRF);
                    [v, v]
                })
                .collect()
        };
        let x = wave(96_000);
        let r = loop_audio(&x);
        assert!(r.pass, "{}", r.summary);
        // a quiet ending
        let mut y = x.clone();
        for f in &mut y[90_000..] {
            *f = [f[0] * 0.25, f[1] * 0.25];
        }
        let r = loop_audio(&y);
        assert!(!r.pass && r.summary.contains("12.0 dB apart"), "{}", r.summary);
        // a click
        let mut y = x.clone();
        y[95_999] = [0.5, 0.5];
        let r = loop_audio(&y);
        assert!(!r.pass && r.summary.contains("click"), "{}", r.summary);
        // silence loops cleanly; nothing and not-a-number fail in words; one sample does not panic
        assert!(loop_audio(&[[0.0, 0.0]; 4800]).pass);
        assert!(!loop_audio(&[]).pass);
        assert!(!loop_audio(&[[f64::NAN, 0.0]; 4800]).pass);
        assert!(loop_audio(&[[0.1, 0.1]]).pass);
    }
}
