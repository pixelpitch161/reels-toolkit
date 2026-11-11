//! The checks every video passes before it is saved: the ports of `projects/ballfight/kit/checks.py` and of the
//! `reelkit` modules it runs.
//!
//! | Check | Python | Passes when |
//! | --- | --- | --- |
//! | [`focus`] | `kit/checks.py focus()` | the subject's circle of confusion is 0 px on every frame |
//! | [`safezone`] | `reelkit/safezone.py check()` | every caption, label and panel is inside the safe zone |
//! | [`levels`] | `reelkit/levels.py` | the loudness is even from frame 0 (opening, dips, spread, build) |
//! | [`band`] | `kit/checks.py band()` | each fighter is louder than the bed in its own band while it plays |
//! | [`loudness`] | `kit/checks.py loudness()` | always (the kit measures it and sets no limit) |
//! | [`flash`] | `reelkit/flashcheck.py analyse()` | no second holds more than 3 general flashes |
//! | [`loop_picture`], [`loop_audio`] | `reelkit/loopcheck.py` | the loop point is the file boundary |
//!
//! Every check returns a [`CheckResult`] and never panics on its input: silence, no frames, a sample that is
//! not a number or a soundtrack too short to measure give a failed result whose `summary` says in one plain
//! sentence what is wrong and what to change.
//!
//! **The sound the checks read.** The kit runs its sound checks on the 16-bit files it wrote (`audio.wav` and
//! `check/stem_<bus>.wav`), each sample divided by 32,768. [`levels`], [`loudness`] and [`loop_audio`] therefore
//! take the master as that file holds it: pass [`as_wav16`] of the soundtrack's master (as [`pre_render`] does), or
//! the samples of a decoded file. [`band`] takes the [`Soundtrack`] and reads its stems that way itself.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bf_content::ContentError;
use bf_core::dmath;
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_picture::ring::{FrameReport, Picture};
use bf_picture::ui::DrawLog;
use bf_picture::{Gray32, Rgb8};
use bf_sound::dsp;
use bf_sound::master::Soundtrack;
use bf_sound::wav::sample_i16;
use bf_story::Story;

pub mod band;
pub mod flash;
pub mod focus;
pub mod levels;
pub mod loopcheck;
pub mod loudness;
pub mod safezone;

pub use self::band::band;
pub use self::flash::{flash, flash_span};
pub use self::focus::focus;
pub use self::levels::levels;
pub use self::loopcheck::{loop_audio, loop_picture};
pub use self::loudness::loudness;
pub use self::safezone::safezone;

/// The sample rate of every soundtrack, as a float.
pub(crate) const SRF: f64 = bf_sound::SR as f64;

/// What one check found.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckResult {
    /// The check: `focus`, `safezone`, `levels`, `band`, `loudness`, `flash`, `loop_picture` or `loop_audio`.
    pub name: &'static str,
    /// Whether the video may be saved as far as this check goes.
    pub pass: bool,
    /// One plain sentence: what was measured and, when the check failed, what to change.
    pub summary: String,
    /// The measured numbers by name (levels in dB, times in seconds). Only finite numbers are listed: one that
    /// could not be measured is left out.
    pub numbers: BTreeMap<String, f64>,
}

impl CheckResult {
    /// A result with its numbers; a number that is not finite is left out.
    pub(crate) fn new(name: &'static str, pass: bool, summary: String, numbers: &[(&str, f64)]) -> CheckResult {
        let mut r = CheckResult {
            name,
            pass,
            summary,
            numbers: BTreeMap::new(),
        };
        for &(key, value) in numbers {
            r.put(key, value);
        }
        r
    }

    /// A failed result without numbers: the check could not measure its input.
    pub(crate) fn failed(name: &'static str, summary: String) -> CheckResult {
        CheckResult::new(name, false, summary, &[])
    }

    /// Lists a number, unless it is not finite.
    pub(crate) fn put(&mut self, key: &str, value: f64) {
        if value.is_finite() {
            self.numbers.insert(key.to_string(), value);
        }
    }
}

// ---------------------------------------------------------------- the sound as the checks read it
/// A stereo buffer as the kit's sound checks read it back from the 16-bit file `reelkit.audio.write` makes of
/// it: each sample clipped to [-1, 1], scaled by 32,767 and cut toward zero, then divided by 32,768.
pub fn as_wav16(x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    x.iter()
        .map(|f| {
            [
                f64::from(sample_i16(f[0])) / 32768.0,
                f64::from(sample_i16(f[1])) / 32768.0,
            ]
        })
        .collect()
}

/// The first sample of `x` a check cannot measure, as `(frame, why)`: one that is not a number, or one far
/// outside -1 to 1 (a decoded file never holds either).
pub(crate) fn bad_sample(x: &[[f64; 2]]) -> Option<(usize, &'static str)> {
    x.iter().enumerate().find_map(|(i, f)| {
        if !(f[0].is_finite() && f[1].is_finite()) {
            Some((i, "is not a number"))
        } else if f[0].abs() > 2.0 || f[1].abs() > 2.0 {
            Some((i, "is far outside the range -1 to 1"))
        } else {
            None
        }
    })
}

/// A count with a thousands separator: `5,503`.
pub(crate) fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The failed result for a master no sound check can measure (empty, or with a [`bad_sample`]); `None` for a
/// master that can be measured.
pub(crate) fn unusable(name: &'static str, title: &str, master: &[[f64; 2]]) -> Option<CheckResult> {
    if master.is_empty() {
        return Some(CheckResult::failed(
            name,
            format!("{title}: the soundtrack is empty, so there is nothing to measure; build the soundtrack again."),
        ));
    }
    bad_sample(master).map(|(i, why)| {
        CheckResult::failed(
            name,
            format!(
                "{title}: the soundtrack holds a value at {:.3} s that {why}, so it cannot be measured; build the \
                 soundtrack again, and if this stays, one of the instruments is faulty.",
                i as f64 / SRF
            ),
        )
    })
}

/// `x.mean(1)` of a stereo array: the mean of the two channels of each frame.
pub(crate) fn mono(x: &[[f64; 2]]) -> Vec<f64> {
    x.iter().map(|f| (f[0] + f[1]) / 2.0).collect()
}

/// `(x ** 2).mean()`: numpy's pairwise mean of the squares. NaN for no samples.
pub(crate) fn mean_square(x: &[f64]) -> f64 {
    let squares: Vec<f64> = x.iter().map(|v| v * v).collect();
    dsp::mean(&squares)
}

/// `20 * np.log10(np.sqrt((x ** 2).mean()) + floor)`: a level in dB. NaN for no samples.
pub(crate) fn level_db(x: &[f64], floor: f64) -> f64 {
    20.0 * dmath::log10(mean_square(x).sqrt() + floor)
}

/// `np.median(x)`: the middle value, or the mean of the two middle values. NaN for no values or when one is NaN.
pub(crate) fn median(x: &[f64]) -> f64 {
    if x.is_empty() || x.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    let mut v = x.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        // np.mean of the two: (a + b) / 2
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// `int(t * sr)` as an index into a buffer of `len` samples, the way a Python slice bound is read: cut toward
/// zero and held inside the buffer.
pub(crate) fn sample_at(t: f64, len: usize) -> usize {
    let i = t * SRF;
    if i.is_nan() || i <= 0.0 {
        0
    } else if i >= len as f64 {
        len
    } else {
        i as usize
    }
}

/// What a caught panic said.
pub(crate) fn panic_text(payload: Box<dyn std::any::Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(text) => *text,
        Err(payload) => match payload.downcast::<&'static str>() {
            Ok(text) => (*text).to_string(),
            Err(_) => "it stopped without a message".to_string(),
        },
    }
}

// ---------------------------------------------------------------- all of them
/// What the band check needs to know of a fight: the kit's `M.SOUNDS` and `M.bed.buses`.
#[derive(Clone, Debug, PartialEq)]
pub struct BandPlan {
    /// Per fighter (left corner first): its bus (`Sound.bus`, its name in lower case) and its band in Hz
    /// (`Sound.band`).
    pub sounds: [(String, (f64, f64)); 2],
    /// The buses the fighters must sit above: the bed's own, in the order the theme lists them, then `fx`
    /// (`list(M.bed.buses) + ['fx']`).
    pub bed_buses: Vec<String>,
}

impl BandPlan {
    /// The plan of the fight file `ff`: each fighter's part gives its bus and band, the theme's arrangement its
    /// buses.
    ///
    /// # Errors
    /// The fight file names a theme, an arrangement or a fighter this app has no sound for, in the words
    /// `bf_content::validate` uses.
    pub fn of(ff: &FightFile) -> Result<BandPlan, ContentError> {
        bf_content::validate(ff)?;
        // validate() has checked all of these; the errors below cannot happen after it
        let missing = |what: &str| ContentError::Sound(format!("the fight file's {what} has no sound in this app"));
        let theme = bf_content::themes::theme(&ff.music.theme).ok_or_else(|| missing("theme"))?;
        let bed = theme.bed(&ff.music.arrangement, &theme.options_of(ff)?)?;
        let part = |k: usize| -> Result<(String, (f64, f64)), ContentError> {
            let name = ff.fighters.get(k).ok_or_else(|| missing("fighter list"))?;
            let sound = bf_content::fighters::sound(name, k, ff).ok_or_else(|| missing("fighter"))?;
            // `Sound::bus` needs the music context of a recording, which a fight file alone does not give;
            // it is the fighter's name in lower case (`self.name.lower()`), and no part overrides it
            Ok((name.to_lowercase(), sound.band()))
        };
        let mut bed_buses: Vec<String> = bed.buses().iter().map(|(bus, _)| (*bus).to_string()).collect();
        bed_buses.push("fx".to_string());
        Ok(BandPlan {
            sounds: [part(0)?, part(1)?],
            bed_buses,
        })
    }
}

/// What one drawn frame gives the checks.
struct Drawn {
    log: Option<DrawLog>,
    lum: Option<Gray32>,
    rgb: Option<Rgb8>,
}

/// Every check a video passes before its one full render, in the kit's order: `focus`, `safezone`, `levels`,
/// `band`, `loudness`, `flash`, `loop_picture`, `loop_audio`.
///
/// The frames the picture checks need (a frame every 0.25 s for the safe zone, the busiest stretch for the
/// flashes, the last 5 s and the first second for the loop: 673 frames of the example episode) are drawn once,
/// on `workers` threads (at least one); the sound checks run beside them. The results do not depend on
/// `workers`. `story` is the fight's story (its first uses of the abilities place the flash check's stretch) and
/// `plan` the fighters' bands and the bed's buses ([`BandPlan::of`]).
///
/// A frame that cannot be drawn (a panic in a look) fails the checks that needed it, with the frame named.
///
/// `cancel` is looked at before each frame is started and between the sound checks: once it is set no new
/// frame is drawn and `None` is returned as soon as the frames being drawn and the sound check running are done
/// (about a second at most on the reference PC, the band check's 3 s aside).
pub fn pre_render(
    pic: &Picture,
    st: &Soundtrack,
    show: &Show,
    story: &Story,
    plan: &BandPlan,
    workers: usize,
    cancel: &AtomicBool,
) -> Option<Vec<CheckResult>> {
    pre_render_with(&|f| pic.frame_logged(f), pic, st, show, story, plan, workers, cancel)
}

/// The frames [`pre_render_keeping`] drew, by number, each with its report: frame `f` exactly as
/// `pic.frame_reported(f)` gives it.
pub type Kept = BTreeMap<usize, (Rgb8, FrameReport)>;

/// [`pre_render`] that also gives back every frame it drew (673 of the example episode's 1,830, about 465 MB), so
/// the render can use them instead of drawing them again. A frame is a pure function of its number, so the video
/// made from them is the same file. The results are [`pre_render`]'s; a frame that could not be drawn is not
/// kept.
pub fn pre_render_keeping(
    pic: &Picture,
    st: &Soundtrack,
    show: &Show,
    story: &Story,
    plan: &BandPlan,
    workers: usize,
    cancel: &AtomicBool,
) -> Option<(Vec<CheckResult>, Kept)> {
    let kept: Mutex<Kept> = Mutex::new(BTreeMap::new());
    let draw = |f: usize| {
        let (img, log, report) = pic.frame_reported(f);
        // as `frame_logged` does
        debug_assert_eq!(
            report.hull_errors, 0,
            "frame {f}: a hull scipy would refuse was left out"
        );
        kept.lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(f, (img.clone(), report));
        (img, log)
    };
    let results = pre_render_with(&draw, pic, st, show, story, plan, workers, cancel)?;
    Some((results, kept.into_inner().unwrap_or_else(|e| e.into_inner())))
}

/// [`pre_render`] with the drawing of a frame given: `draw_frame(f)` is `pic.frame_logged(f)` there, and a
/// stand-in in the tests.
#[allow(clippy::too_many_arguments)]
fn pre_render_with(
    draw_frame: &(dyn Fn(usize) -> (Rgb8, DrawLog) + Sync),
    pic: &Picture,
    st: &Soundtrack,
    show: &Show,
    story: &Story,
    plan: &BandPlan,
    workers: usize,
    cancel: &AtomicBool,
) -> Option<Vec<CheckResult>> {
    let cancelled = || cancel.load(Ordering::SeqCst);
    let nf = pic.nf();
    let fps = show.grid.fps;
    let cfg = &pic.ctx().cfg;
    let (f0, f1) = flash_span(show, story, [&cfg[0].low.name, &cfg[1].low.name]);
    let (f0, f1) = (f0.min(nf), f1.min(nf));
    let safe = safezone::frames(nf, fps);
    let nt = ((loopcheck::TAIL_S * f64::from(fps)) as usize).min(nf);
    let nh = ((loopcheck::HEAD_S * f64::from(fps)) as usize).min(nf);
    let safe_set: BTreeSet<usize> = safe.iter().map(|&(_, f)| f).collect();
    let loop_frames: Vec<usize> = (nf - nt..nf).chain(0..nh).collect();
    let loop_set: BTreeSet<usize> = loop_frames.iter().copied().collect();
    let todo: Vec<usize> = safe_set
        .iter()
        .copied()
        .chain(f0..f1)
        .chain(loop_frames.iter().copied())
        .collect::<BTreeSet<usize>>()
        .into_iter()
        .collect();

    let draw = |f: usize| -> Result<Drawn, String> {
        match catch_unwind(AssertUnwindSafe(|| draw_frame(f))) {
            Ok((rgb, log)) => Ok(Drawn {
                log: safe_set.contains(&f).then_some(log),
                lum: (f0..f1).contains(&f).then(|| flash::rel_lum(&rgb)),
                rgb: loop_set.contains(&f).then_some(rgb),
            }),
            Err(payload) => Err(panic_text(payload)),
        }
    };
    let next = AtomicUsize::new(0);
    let drawn: Mutex<BTreeMap<usize, Result<Drawn, String>>> = Mutex::new(BTreeMap::new());
    let sound = std::thread::scope(|s| {
        for _ in 0..workers.max(1) {
            s.spawn(|| {
                loop {
                    if cancelled() {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&f) = todo.get(i) else { break };
                    let d = draw(f);
                    drawn.lock().unwrap_or_else(|e| e.into_inner()).insert(f, d);
                }
            });
        }
        // the sound checks and the camera maths, beside the frames
        let master = as_wav16(&st.master);
        let sounds = [
            (plan.sounds[0].0.as_str(), plan.sounds[0].1),
            (plan.sounds[1].0.as_str(), plan.sounds[1].1),
        ];
        let bed: Vec<&str> = plan.bed_buses.iter().map(String::as_str).collect();
        let checks: [&dyn Fn() -> CheckResult; 5] = [
            &|| focus(pic),
            &|| levels(&master),
            &|| band(st, show, sounds, &bed),
            &|| loudness(&master, show),
            &|| loop_audio(&master),
        ];
        let mut out = Vec::with_capacity(checks.len());
        for check in checks {
            if cancelled() {
                return None;
            }
            out.push(check());
        }
        Some(out)
    });
    if cancelled() {
        return None;
    }
    let mut drawn = drawn.into_inner().unwrap_or_else(|e| e.into_inner());
    let [focus_r, levels_r, band_r, loudness_r, loop_audio_r]: [CheckResult; 5] = sound?.try_into().ok()?;

    // The first frame of `frames` that could not be drawn, as the failed result of the check `name`.
    let broken = |name: &'static str, title: &str, frames: &[usize]| -> Option<CheckResult> {
        frames.iter().find_map(|&f| match drawn.get(&f) {
            Some(Err(why)) => Some(CheckResult::failed(
                name,
                format!(
                    "{title}: frame {f} (at {:.2} s) could not be drawn ({}), so this check did not run; this is \
                     a fault in the app, not in your fight.",
                    f as f64 / f64::from(fps),
                    why.replace('\n', " ").trim_end_matches('.')
                ),
            )),
            _ => None,
        })
    };
    let safe_broken = broken(
        "safezone",
        "Safe zone",
        &safe.iter().map(|&(_, f)| f).collect::<Vec<_>>(),
    );
    let flash_broken = broken("flash", "Flashes", &(f0..f1).collect::<Vec<_>>());
    let loop_broken = broken("loop_picture", "Loop point (picture)", &loop_frames);

    let safe_r = safe_broken.unwrap_or_else(|| {
        let logs: Vec<(f64, DrawLog)> = safe
            .iter()
            .map(|&(t, f)| {
                let log = match drawn.get(&f) {
                    Some(Ok(d)) => d.log.clone().unwrap_or_default(),
                    _ => DrawLog::new(),
                };
                (t, log)
            })
            .collect();
        safezone(&logs)
    });
    let flash_r = flash_broken.unwrap_or_else(|| {
        let lum: Vec<Gray32> = (f0..f1)
            .filter_map(|f| match drawn.get_mut(&f) {
                Some(Ok(d)) => d.lum.take(),
                _ => None,
            })
            .collect();
        flash::flash_from(&lum, f0)
    });
    let loop_r = loop_broken.unwrap_or_else(|| {
        let pics: Vec<Rgb8> = loop_frames
            .iter()
            .filter_map(|f| match drawn.get(f) {
                Some(Ok(d)) => d.rgb.clone(),
                _ => None,
            })
            .collect();
        let (tail, head) = pics.split_at(nt.min(pics.len()));
        loop_picture(tail, head)
    });
    Some(vec![
        focus_r,
        safe_r,
        levels_r,
        band_r,
        loudness_r,
        flash_r,
        loop_r,
        loop_audio_r,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wav_rounding_is_the_kits() {
        // (np.clip(x, -1, 1) * 32767).astype(np.int16) / 32768
        let x = [[0.5, -0.5], [1.5, -1.5], [0.00002, -0.00002], [1.0, 0.0]];
        let want = [
            [16383.0 / 32768.0, -16383.0 / 32768.0],
            [32767.0 / 32768.0, -32767.0 / 32768.0],
            [0.0, 0.0],
            [32767.0 / 32768.0, 0.0],
        ];
        assert_eq!(as_wav16(&x), want);
    }

    #[test]
    fn median_is_numpys() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert!(median(&[]).is_nan());
        assert!(median(&[1.0, f64::NAN]).is_nan());
    }

    #[test]
    fn slice_bounds_stay_inside() {
        assert_eq!(sample_at(1.0, 100_000), 48_000);
        assert_eq!(sample_at(-1.0, 100_000), 0);
        assert_eq!(sample_at(9.0, 100_000), 100_000);
        assert_eq!(sample_at(f64::NAN, 100_000), 0);
        // int(3.5882352941176467 * 48000) = 172235
        assert_eq!(sample_at(3.588_235_294_117_646_7, 3_000_000), 172_235);
    }

    #[test]
    fn a_master_that_cannot_be_measured_is_named() {
        assert!(unusable("levels", "Levels", &[[0.0, 0.0]]).is_none());
        let r = unusable("levels", "Levels", &[]).unwrap();
        assert!(!r.pass && r.summary.contains("empty"));
        let mut x = vec![[0.0, 0.0]; 96_000];
        x[48_000][1] = f64::NAN;
        let r = unusable("levels", "Levels", &x).unwrap();
        assert!(!r.pass && r.summary.contains("1.000 s") && r.summary.contains("not a number"));
        x[48_000][1] = 1.0e200;
        let r = unusable("levels", "Levels", &x).unwrap();
        assert!(!r.pass && r.summary.contains("outside"));
    }

    #[test]
    fn counts_are_grouped_in_thousands() {
        let shown: Vec<String> = [0, 244, 1000, 5503, 1_234_567].into_iter().map(thousands).collect();
        assert_eq!(shown, ["0", "244", "1,000", "5,503", "1,234,567"]);
    }

    #[test]
    fn a_frame_that_cannot_be_drawn_fails_the_checks_that_need_it() {
        use std::sync::Arc;

        use bf_core::recording::Recording;

        let rec = Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec")).unwrap();
        let rec = Arc::new(rec);
        let path = bf_testdata::repo_root().join("studio/fights/flute_vs_trombone.toml");
        let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        let show = Show::new(&rec, &ff).unwrap();
        let pic = bf_content::build_picture(Arc::clone(&rec), &ff).unwrap();
        let st = bf_content::build_soundtrack(&rec, &ff).unwrap();
        let cfg = &pic.ctx().cfg;
        let lows = [cfg[0].low.name.as_str(), cfg[1].low.name.as_str()];
        let sheet = bf_story::BeatSheet::series([&cfg[0], &cfg[1]]);
        let story = bf_story::story(&rec, &show.grid, &sheet, lows);
        let plan = BandPlan::of(&ff).unwrap();
        // Frame 0 is one of the safe zone's and of the loop's, frame 1300 one of the flash stretch's; every
        // other frame is a grey picture with one caption inside the safe zone.
        let stand_in = |f: usize| -> (Rgb8, DrawLog) {
            assert!(f != 0 && f != 1300, "the look of frame {f} lost its footing.");
            let mut log = DrawLog::new();
            log.note(100, 100, 200, 120, bf_picture::ui::Kind::Caption, "FLUTE");
            (Rgb8::filled(36, 64, &[90, 90, 90]), log)
        };
        let rs = pre_render_with(&stand_in, &pic, &st, &show, &story, &plan, 4, &AtomicBool::new(false))
            .expect("not cancelled");
        let by_name = |name: &str| rs.iter().find(|r| r.name == name).unwrap();
        for (name, frame) in [
            ("safezone", "frame 0 (at 0.00 s)"),
            ("flash", "frame 1300 (at 43.33 s)"),
        ] {
            let r = by_name(name);
            assert!(!r.pass && r.summary.contains(frame), "{}", r.summary);
            assert!(
                r.summary.contains("lost its footing), so this check did not run"),
                "{}",
                r.summary
            );
            assert!(r.summary.ends_with('.') && !r.summary.contains(". "), "{}", r.summary);
        }
        let r = by_name("loop_picture");
        assert!(
            !r.pass && r.summary.contains("frame 0 (at 0.00 s) could not be drawn"),
            "{}",
            r.summary
        );
        // the camera maths and the sound checks ran all the same
        for name in ["focus", "levels", "band", "loudness", "loop_audio"] {
            assert!(by_name(name).pass, "{}", by_name(name).summary);
        }
        assert_eq!(rs.len(), 8);
        // with every frame drawn the stand-in's pictures pass
        let calm = |_: usize| stand_in(1);
        let rs =
            pre_render_with(&calm, &pic, &st, &show, &story, &plan, 4, &AtomicBool::new(false)).expect("not cancelled");
        assert!(rs.iter().all(|r| r.pass), "{rs:?}");
    }

    #[test]
    fn numbers_are_finite() {
        let r = CheckResult::new("levels", true, String::new(), &[("a", 1.0), ("b", f64::NAN)]);
        assert_eq!(r.numbers.len(), 1);
    }
}
