//! `reelkit/levels.py`: loud and busy from frame 0, the same loudness all the way, never quiet.
//!
//! The loudness (RMS, dBFS) and a rough hit rate of every 2 s window, then four targets against the ending (the
//! mean of the last 10 windows, the level the series is mixed to):
//!
//! - **opening**: the first 5 windows are at most 1.5 dB below the ending;
//! - **dips**: at most one window is more than 3 dB below the ending (one deliberate dip, such as the hit-stop
//!   of a K.O.);
//! - **spread**: the loudest window is at most 5 dB above the quietest, the one allowed dip left out;
//! - **build**: the second half is at most 1.5 dB louder than the first (more means the start is too quiet).
//!
//! A silent soundtrack is refused before any of this (the kit would pass it: every window equally quiet).

use bf_core::dmath;
use bf_core::py::round_half_even;
use bf_sound::dsp;

use super::{CheckResult, SRF, mean_square, mono, unusable};

/// `WIN`: the window, 2 s.
pub const WIN: f64 = 2.0;
/// The opening may be this many dB below the ending.
pub const OPENING_WITHIN: f64 = 1.5;
/// A window more than this many dB below the ending is a dip.
pub const DIP_BELOW: f64 = 3.0;
/// The dips allowed.
pub const DIPS_ALLOWED: usize = 1;
/// The loudest window may be this many dB above the quietest.
pub const SPREAD_MAX: f64 = 5.0;
/// The second half may be this many dB louder than the first.
pub const BUILD_MAX: f64 = 1.5;

const NAME: &str = "levels";
const TITLE: &str = "Levels";

/// One row of `profile()`: `(w * WIN, rms, hits / WIN)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    /// Where the window starts, in seconds.
    pub t: f64,
    /// Its RMS in dBFS (-120 for a window past the end of the sound).
    pub dbfs: f64,
    /// The rough number of hits a second in it.
    pub hits_per_s: f64,
}

/// `profile(sr, x, dur)` at 48 kHz: the loudness and the rough hit rate of each of the `int(dur / WIN)` windows
/// of the mono signal `x`. (Hits: peaks of the 10 ms level that stand 6 dB above the lowest of the five levels
/// before, at least four apart and above -45 dBFS.)
///
/// `x` must hold numbers only (a NaN panics in the peak search, as [`levels`] checks before it calls this).
pub fn profile(x: &[f64], dur: f64) -> Vec<Row> {
    // hop = int(sr * 0.01); fr = sqrt(array([(x[i:i + hop] ** 2).mean() for i in range(0, len(x) - hop, hop)]) + 1e-12)
    let hop = (SRF * 0.01) as usize;
    let mut db = Vec::new();
    let mut i = 0;
    while i + hop < x.len() {
        let fr = (mean_square(&x[i..i + hop]) + 1e-12).sqrt();
        db.push(20.0 * dmath::log10(fr));
        i += hop;
    }
    // prev = [db[max(0, i - 5):i].min() if i else db[0]]
    let rise: Vec<f64> = (0..db.len())
        .map(|i| {
            let prev = if i == 0 {
                db[0]
            } else {
                db[i.saturating_sub(5)..i].iter().copied().fold(f64::INFINITY, f64::min)
            };
            db[i] - prev
        })
        .collect();
    // pk, _ = find_peaks(db - prev, height=6.0, distance=4); pk = pk[db[pk] > -45]
    let peaks: Vec<usize> = dsp::find_peaks(&rise, 6.0, 4)
        .into_iter()
        .filter(|&p| db[p] > -45.0)
        .collect();
    let windows = if dur.is_finite() && dur > 0.0 {
        (dur / WIN) as usize
    } else {
        0
    };
    (0..windows)
        .map(|w| {
            let (t0, t1) = (w as f64 * WIN, (w + 1) as f64 * WIN);
            // a, b = int(w * WIN * sr), int((w + 1) * WIN * sr); seg = x[a:b]
            let a = ((t0 * SRF) as usize).min(x.len());
            let b = ((t1 * SRF) as usize).min(x.len());
            let seg = &x[a..b];
            let dbfs = if seg.is_empty() {
                -120.0
            } else {
                20.0 * dmath::log10(mean_square(seg).sqrt() + 1e-12)
            };
            // hits = ((pk * 0.01 >= w * WIN) & (pk * 0.01 < (w + 1) * WIN)).sum()
            let hits = peaks
                .iter()
                .filter(|&&p| {
                    let t = p as f64 * 0.01;
                    t >= t0 && t < t1
                })
                .count();
            Row {
                t: t0,
                dbfs,
                hits_per_s: hits as f64 / WIN,
            }
        })
        .collect()
}

/// numpy's `mean` of a slice; NaN for none.
fn mean(x: &[f64]) -> f64 {
    dsp::mean(x)
}

/// The levels check on a master as its 16-bit file holds it (see the module notes of [`super`]): `main()` of
/// `reelkit/levels.py`. The profile runs over `round(seconds / 2) * 2` seconds, so a 61.013 s soundtrack has 31
/// windows, the last one holding the 1.013 s that are left.
///
/// Numbers: `ending`, `opening`, `opening_minus_ending`, `opening_hits_per_s`, `build`, `spread` (dB) and `dips`
/// (how many windows are more than 3 dB below the ending).
pub fn levels(master: &[[f64; 2]]) -> CheckResult {
    if let Some(r) = unusable(NAME, TITLE, master) {
        return r;
    }
    if master.iter().all(|f| f[0] == 0.0 && f[1] == 0.0) {
        return CheckResult::failed(
            NAME,
            format!(
                "{TITLE}: the soundtrack is silent from start to end, so there is no loudness to measure; check \
                 that the fight file's music plays."
            ),
        );
    }
    let x = mono(master);
    // rows = profile(sr, x, dur=round(len(x) / sr / WIN) * WIN)
    let rows = profile(&x, round_half_even(x.len() as f64 / SRF / WIN) * WIN);
    if rows.len() < 2 {
        return CheckResult::failed(
            NAME,
            format!(
                "{TITLE}: the soundtrack is only {:.1} s long, too short to compare its opening with its ending; \
                 a video's soundtrack is 61 s.",
                x.len() as f64 / SRF
            ),
        );
    }
    let r: Vec<f64> = rows.iter().map(|row| row.dbfs).collect();
    let h: Vec<f64> = rows.iter().map(|row| row.hits_per_s).collect();
    let n = r.len();
    let half = n / 2;
    // ref = r[-10:].mean(); opening, build = r[:5].mean(), r[half:].mean() - r[:half].mean()
    let ending = mean(&r[n.saturating_sub(10)..]);
    let opening = mean(&r[..n.min(5)]);
    let build = mean(&r[half..]) - mean(&r[..half]);
    // dips = [(t, v) for t, v, _ in rows if v < ref - 3.0]
    let dips: Vec<&Row> = rows.iter().filter(|row| row.dbfs < ending - DIP_BELOW).collect();
    let ok_open = opening >= ending - OPENING_WITHIN;
    let ok_dips = dips.len() <= DIPS_ALLOWED;
    // rest = np.delete(r, np.argmin(r)) if len(dips) == 1 else r; spread = np.ptp(rest)
    let mut rest = r.clone();
    if dips.len() == 1 {
        // np.argmin: the first of the smallest
        let mut lowest = 0;
        for (i, v) in rest.iter().enumerate() {
            if *v < rest[lowest] {
                lowest = i;
            }
        }
        rest.remove(lowest);
    }
    let top = rest.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let bottom = rest.iter().copied().fold(f64::INFINITY, f64::min);
    let spread = top - bottom;
    let ok_spread = spread <= SPREAD_MAX;
    let ok_build = build <= BUILD_MAX;
    let pass = ok_open && ok_dips && ok_spread && ok_build;
    let under = opening - ending;

    let summary = if pass {
        format!(
            "{TITLE}: the loudness is even from the first frame: the opening (first 10 s) is {under:+.1} dB \
             against the ending, the 2 s windows spread {spread:.1} dB, {} more than 3 dB below the ending and \
             the second half is {build:+.1} dB against the first.",
            match dips.len() {
                0 => "no window is".to_string(),
                _ => format!("one window (at {:.0} s, the one dip allowed) is", dips[0].t),
            }
        )
    } else {
        let mut faults = Vec::new();
        if !ok_open {
            faults.push(format!(
                "the opening (first 10 s) is {:.1} dB quieter than the ending where at most {OPENING_WITHIN} dB is \
                 allowed, so raise the level of the title card and the count-in",
                -under
            ));
        }
        if !ok_dips {
            let at: Vec<String> = dips
                .iter()
                .take(6)
                .map(|row| format!("{:.0} s ({:+.1} dB)", row.t, row.dbfs - ending))
                .collect();
            faults.push(format!(
                "{} windows of 2 s are more than {DIP_BELOW:.0} dB below the ending where one dip is allowed (at \
                 {}{}), so fill those stretches",
                dips.len(),
                at.join(", "),
                if dips.len() > at.len() { " and later" } else { "" }
            ));
        }
        if !ok_spread {
            faults.push(format!(
                "the loudest 2 s window is {spread:.1} dB above the quietest where at most {SPREAD_MAX:.0} dB is \
                 allowed, so even out the loud and the quiet stretches"
            ));
        }
        if !ok_build {
            faults.push(format!(
                "the second half is {build:+.1} dB louder than the first where at most +{BUILD_MAX} dB is allowed, \
                 so lift the start and build with the arrangement instead of the level"
            ));
        }
        format!("{TITLE}: the loudness is not even: {}.", faults.join("; and "))
    };
    CheckResult::new(
        NAME,
        pass,
        summary,
        &[
            ("ending", ending),
            ("opening", opening),
            ("opening_minus_ending", under),
            ("opening_hits_per_s", mean(&h[..n.min(5)])),
            ("build", build),
            ("spread", spread),
            ("dips", dips.len() as f64),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A steady tone at `amp`, `seconds` long.
    fn tone(seconds: f64, amp: f64) -> Vec<[f64; 2]> {
        (0..(seconds * SRF) as usize)
            .map(|i| {
                let v = amp * dmath::sin(i as f64 * 0.05);
                [v, v]
            })
            .collect()
    }

    #[test]
    fn a_steady_tone_passes() {
        let r = levels(&tone(61.0, 0.3));
        assert!(r.pass, "{}", r.summary);
        assert!(r.numbers["spread"] < 0.01 && r.numbers["opening_minus_ending"].abs() < 0.01);
        assert_eq!(r.numbers["dips"], 0.0);
    }

    #[test]
    fn unusual_input_fails_in_words() {
        for (x, word) in [
            (vec![], "empty"),
            (vec![[0.0, 0.0]; 96_000], "silent"),
            (tone(2.5, 0.3), "too short"),
            (vec![[f64::INFINITY, 0.0]; 10], "not a number"),
        ] {
            let r = levels(&x);
            assert!(!r.pass && r.summary.contains(word), "{}", r.summary);
            assert!(r.summary.starts_with("Levels: ") && r.summary.ends_with('.'));
        }
        // a single sample, a single window: no panic
        assert!(!levels(&[[0.1, 0.1]]).pass);
        assert!(profile(&[], 62.0).iter().all(|row| row.dbfs == -120.0));
        assert!(profile(&[0.5; 100], f64::NAN).is_empty());
    }

    #[test]
    fn each_target_is_named() {
        // a build: the second half 6 dB up
        let mut x = tone(30.0, 0.1);
        x.extend(tone(31.0, 0.2));
        let r = levels(&x);
        assert!(!r.pass && r.summary.contains("second half"), "{}", r.summary);
        assert!(r.summary.contains("opening"), "{}", r.summary);
        // an uneven middle: three windows 8 dB down are three dips
        let mut x = tone(61.0, 0.3);
        for f in &mut x[20 * 48_000..26 * 48_000] {
            *f = [f[0] * 0.4, f[1] * 0.4];
        }
        let r = levels(&x);
        assert!(!r.pass && r.summary.contains("3 windows"), "{}", r.summary);
        assert!(r.summary.contains("loudest"), "{}", r.summary);
        assert_eq!(r.numbers["dips"], 3.0);
        // one such window is the dip the rule allows, and it does not count toward the spread
        let mut x = tone(61.0, 0.3);
        for f in &mut x[20 * 48_000..22 * 48_000] {
            *f = [f[0] * 0.4, f[1] * 0.4];
        }
        let r = levels(&x);
        assert!(r.pass && r.summary.contains("at 20 s"), "{}", r.summary);
        assert!(r.numbers["spread"] < 0.01);
    }
}
