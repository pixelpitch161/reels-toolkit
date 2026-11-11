//! `reelkit/flashcheck.py` and `kit/checks.py` `flash()`: the photosensitivity check (a general-flash test in
//! the style of Harding and WCAG 2.3.1).
//!
//! A transition counts when the relative luminance changes by 0.1 or more (with the darker state below 0.8)
//! over a quarter of the frame or more, comparing each frame with the one to five frames before it. A flash is a
//! pair of opposing transitions; the rule is at most 3 flashes in any second.
//!
//! The luminance is `float32`, as `rel_lum` makes it in Python: the thresholds 0.1 and 0.8 are compared as
//! `float32` values, so [`rel_lum`] and [`analyse`] keep that type.

use bf_core::dmath;
use bf_core::py::{py_max, py_min};
use bf_core::show::Show;
use bf_picture::{Gray32, Rgb8};
use bf_story::Story;

use super::CheckResult;

/// The flashes allowed in any second.
pub const LIMIT: usize = 3;
/// The share of the frame that must change for a transition.
pub const AREA: f64 = 0.25;
/// The frames a second the series is drawn at (`reelkit.config.FPS`).
pub const FPS: usize = bf_picture::config::FPS as usize;

const NAME: &str = "flash";
const TITLE: &str = "Flashes";

/// `rel_lum(rgb)`: the relative luminance of every pixel of an RGB picture, `float32`:
/// `c = rgb / 255; c = where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)`, then
/// `0.2126 r + 0.7152 g + 0.0722 b`. A picture that is not RGB gives an empty plane.
pub fn rel_lum(rgb: &Rgb8) -> Gray32 {
    if rgb.c != 3 {
        return Gray32::from_vec(0, 0, 1, Vec::new());
    }
    let mut linear = [0.0f32; 256];
    for (v, out) in linear.iter_mut().enumerate() {
        let c = v as f32 / 255.0;
        *out = if c <= 0.04045 {
            c / 12.92
        } else {
            // numpy's float32 power is the correctly rounded one for every one of these 256 values (the
            // float64 power, rounded); the software `powf` is 1 ulp off for some
            dmath::pow(f64::from((c + 0.055) / 1.055), f64::from(2.4f32)) as f32
        };
    }
    let data = rgb
        .data
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| {
            let [r, g, b] = [linear[p[0] as usize], linear[p[1] as usize], linear[p[2] as usize]];
            0.2126f32 * r + 0.7152f32 * g + 0.0722f32 * b
        })
        .collect();
    Gray32::from_vec(rgb.w, rgb.h, 1, data)
}

/// What `analyse` returns: `(area, flashes, worst)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    /// Per frame, the largest share of the frame whose luminance rose (or fell) by 0.1 or more against one of
    /// the five frames before it. 0 for the first frame.
    pub area: Vec<f64>,
    /// The frame each flash starts on.
    pub flashes: Vec<usize>,
    /// The most flashes that start within any one second.
    pub worst: usize,
}

/// `analyse(Y, fps)`: the transitions and flashes of a run of luminance planes.
///
/// The planes must be of one size (a plane of another size than the first counts as no change).
pub fn analyse(lum: &[Gray32], fps: usize) -> Analysis {
    let n = lum.len();
    let mut area = vec![0.0f64; n];
    let mut sign = vec![0i32; n];
    for i in 1..n {
        let (mut best, mut sg) = (0.0f64, 0);
        for k in 1..=5 {
            if i < k {
                break;
            }
            let (now, then) = (&lum[i].data, &lum[i - k].data);
            if now.len() != then.len() || now.is_empty() {
                continue;
            }
            // d = Y[i] - Y[i - k]; dark = np.minimum(Y[i], Y[i - k]) < 0.8
            let (mut up, mut dn) = (0usize, 0usize);
            for (&a, &b) in now.iter().zip(then) {
                let d = a - b;
                let dark = a < 0.8f32 || b < 0.8f32;
                if d >= 0.1f32 && dark {
                    up += 1;
                }
                if d <= -0.1f32 && dark {
                    dn += 1;
                }
            }
            let (up, dn) = (up as f64 / now.len() as f64, dn as f64 / now.len() as f64);
            if up > best {
                (best, sg) = (up, 1);
            }
            if dn > best {
                (best, sg) = (dn, -1);
            }
        }
        (area[i], sign[i]) = (best, sg);
    }
    // trans = [(i, sign[i]) for i in range(n) if area[i] >= 0.25]; runs of one sign at most 5 frames apart merge
    let mut ev: Vec<(usize, i32)> = Vec::new();
    for i in (0..n).filter(|&i| area[i] >= AREA) {
        match ev.last_mut() {
            Some(last) if last.1 == sign[i] && i - last.0 <= 5 => *last = (i, sign[i]),
            _ => ev.push((i, sign[i])),
        }
    }
    let mut flashes = Vec::new();
    let mut k = 0;
    while k + 1 < ev.len() {
        if ev[k].1 != ev[k + 1].1 {
            flashes.push(ev[k].0);
            k += 2;
        } else {
            k += 1;
        }
    }
    // worst = max([sum(1 for b in flashes if a <= b < a + fps) for a in flashes] or [0])
    let worst = flashes
        .iter()
        .map(|&a| flashes.iter().filter(|&&b| a <= b && b < a + fps).count())
        .max()
        .unwrap_or(0);
    Analysis { area, flashes, worst }
}

/// The flash check on the luminance of a run of frames at 30 frames a second ([`rel_lum`] of each): passes when
/// no second holds more than 3 flashes. Times and frame numbers count from the first plane; see [`flash_from`].
pub fn flash(lum: &[Gray32]) -> CheckResult {
    flash_from(lum, 0)
}

/// [`flash`] for a stretch of a video that starts at frame `f0`: the times in the summary and the frame number
/// among the numbers are the video's.
///
/// Numbers: `general_flashes`, `worst_in_1s`, `largest_area` (the largest share of the frame changing by 0.1 or
/// more, 0 to 1) and `largest_area_frame` (the frame it is on).
pub fn flash_from(lum: &[Gray32], f0: usize) -> CheckResult {
    let fail = |text: &str| CheckResult::failed(NAME, format!("{TITLE}: {text}"));
    let Some(first) = lum.first() else {
        return fail("no frames were given to look at, so nothing was checked; draw the frames again.");
    };
    if first.data.is_empty() || lum.iter().any(|y| (y.w, y.h, y.c) != (first.w, first.h, 1)) {
        return fail(
            "the frames given are empty or not all of one size, so they cannot be compared; draw the frames again.",
        );
    }
    let a = analyse(lum, FPS);
    // i = int(np.argmax(area)): the first of the largest
    let mut i = 0;
    for (j, v) in a.area.iter().enumerate() {
        if *v > a.area[i] {
            i = j;
        }
    }
    let at = |frame: usize| (f0 + frame) as f64 / FPS as f64;
    let pass = a.worst <= LIMIT;
    let largest = format!(
        "the largest share of the picture changing brightness at once is {:.1} % (at {:.2} s; a flash needs {:.0} %)",
        100.0 * a.area[i],
        at(i),
        100.0 * AREA
    );
    let summary = if pass {
        format!(
            "{TITLE}: no second of the {} frames looked at holds more than {LIMIT} full-screen flashes (the worst \
             holds {}), and {largest}.",
            lum.len(),
            a.worst
        )
    } else {
        // the first second that holds too many
        let start = a
            .flashes
            .iter()
            .copied()
            .find(|&s| a.flashes.iter().filter(|&&b| s <= b && b < s + FPS).count() == a.worst)
            .unwrap_or(0);
        format!(
            "{TITLE}: the second from {:.2} s holds {} flashes over a quarter of the picture or more where at \
             most {LIMIT} are allowed, so soften or spread out the bright effects there.",
            at(start),
            a.worst
        )
    };
    CheckResult::new(
        NAME,
        pass,
        summary,
        &[
            ("general_flashes", a.flashes.len() as f64),
            ("worst_in_1s", a.worst as f64),
            ("largest_area", a.area[i]),
            ("largest_area_frame", (f0 + i) as f64),
        ],
    )
}

/// The stretch `kit/checks.py` `flash()` looks at by default, as frames `(f0, f1)` (`f1` is one past the last):
/// the busiest one, from 1 s before the first desperation move (or 8 s before the K.O. if that is earlier; never
/// before FIGHT!) to 1.5 s after the K.O. `low_names` are the two fighters' desperation moves
/// (`CFG[k]['abilities']['low']['name']`); the first uses come from the story.
pub fn flash_span(show: &Show, story: &Story, low_names: [&str; 2]) -> (usize, usize) {
    let t = &show.grid;
    // lows = [t for t, w, n in STORY['firsts'] if n in low names]; min(lows + [T_KO - 8])
    let mut first = None;
    for (at, _, name) in &story.firsts {
        if low_names.contains(&name.as_str()) {
            first = Some(first.map_or(*at, |m| py_min(m, *at)));
        }
    }
    let ko = show.t_ko - 8.0;
    let earliest = first.map_or(ko, |m| py_min(m, ko));
    // t0 = max(T.T_FIGHT, earliest - 1.0); t1 = min(T.DUR, T_KO + 1.5)
    let t0 = py_max(t.t_fight, earliest - 1.0);
    let t1 = py_min(t.dur, show.t_ko + 1.5);
    let frame = |x: f64| {
        let f = x * f64::from(t.fps);
        if f.is_nan() || f <= 0.0 { 0 } else { f as usize }
    };
    (frame(t0), frame(t1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plane(v: f32) -> Gray32 {
        Gray32::from_vec(4, 4, 1, vec![v; 16])
    }

    #[test]
    fn luminance_is_float32() {
        let px = |r, g, b| rel_lum(&Rgb8::filled(1, 1, &[r, g, b])).data[0];
        assert_eq!(px(0, 0, 0), 0.0);
        // 0.2126 + 0.7152 + 0.0722 in float32, added left to right
        assert_eq!(px(255, 255, 255), 0.2126f32 + 0.7152f32 + 0.0722f32);
        // the linear part: 10 / 255 = 0.0392 is below 0.04045
        assert_eq!(px(10, 0, 0), 0.2126f32 * (10.0f32 / 255.0 / 12.92));
        assert!((f64::from(px(128, 128, 128)) - 0.2158605).abs() < 1e-6);
        // not RGB
        assert!(rel_lum(&Rgb8::filled(2, 2, &[1, 2, 3, 4])).data.is_empty());
    }

    #[test]
    fn every_level_is_numpys_float32() {
        // reelkit.flashcheck.rel_lum on the reference PC (numpy 2.5.3) for every level v of 0..=255 as (v, 0, 0),
        // (0, v, 0), (0, 0, v) and (v, v, v): the float32 results' bit patterns, added up and xor-ed together.
        let (mut sum, mut xor, mut n) = (0u64, 0u64, 0);
        for v in 0..=255u8 {
            for px in [[v, 0, 0], [0, v, 0], [0, 0, v], [v, v, v]] {
                let bits = u64::from(rel_lum(&Rgb8::filled(1, 1, &px)).data[0].to_bits());
                (sum, xor, n) = (sum + bits, xor ^ bits, n + 1);
            }
        }
        assert_eq!((n, sum, xor), (1024, 1_047_474_085_882, 244_551_662));
    }

    #[test]
    fn a_transition_needs_a_tenth_and_a_dark_side() {
        // 0.1 as float32 on a float32 difference: 0.5 -> 0.6 is a difference of 0.100000024 (counts)
        let a = analyse(&[plane(0.5), plane(0.6)], 30);
        assert_eq!(a.area, [0.0, 1.0]);
        // both states at 0.8 or above: no transition
        let a = analyse(&[plane(0.85), plane(1.0)], 30);
        assert_eq!(a.area, [0.0, 0.0]);
        // a quarter of the frame is a transition, less is not
        let mut q = plane(0.0);
        q.data[..4].fill(1.0);
        let a = analyse(&[plane(0.0), q.clone(), plane(0.0), plane(0.0)], 30);
        assert_eq!((a.area[1], a.flashes.as_slice(), a.worst), (0.25, &[1usize][..], 1));
        q.data[3] = 0.0;
        assert!(analyse(&[plane(0.0), q, plane(0.0)], 30).flashes.is_empty());
    }

    #[test]
    fn unusual_input_fails_in_words() {
        let r = flash(&[]);
        assert!(!r.pass && r.summary.contains("no frames"), "{}", r.summary);
        let r = flash(&[plane(0.0), Gray32::from_vec(2, 2, 1, vec![0.0; 4])]);
        assert!(!r.pass && r.summary.contains("one size"), "{}", r.summary);
        let r = flash(&[Gray32::from_vec(0, 0, 1, vec![])]);
        assert!(!r.pass);
        // one frame, and frames that are not numbers: nothing changes, no panic
        assert!(flash(&[plane(0.3)]).pass);
        assert!(flash(&[plane(f32::NAN), plane(0.0), plane(f32::NAN)]).pass);
        // analyse on planes of other sizes: no change, no panic
        assert_eq!(
            analyse(&[plane(0.0), Gray32::from_vec(1, 1, 1, vec![1.0])], 30).area,
            [0.0, 0.0]
        );
    }

    #[test]
    fn times_count_from_the_stretch() {
        let (b, w) = (plane(0.0), plane(1.0));
        let lum: Vec<Gray32> = (0..30)
            .map(|f| {
                if [3, 9, 15, 21].contains(&f) {
                    w.clone()
                } else {
                    b.clone()
                }
            })
            .collect();
        let r = flash_from(&lum, 300);
        assert!(!r.pass && r.summary.contains("from 10.10 s"), "{}", r.summary);
        assert_eq!(r.numbers["largest_area_frame"], 303.0);
    }
}
