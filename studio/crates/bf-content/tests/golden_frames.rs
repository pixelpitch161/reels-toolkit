//! The golden-frame gate: the example episode drawn through [`bf_content::build_picture`] against the frames
//! the Python kit drew (`studio/reference/frames/flute_vs_trombone/`, written by the export tool's `frames`).
//!
//! Every frame listed in `frames.json` must be Python's bytes. A frame that cannot be (a last-bit maths gap
//! between Python on the reference PC and `bf_core::dmath`) is listed in `studio/reference/frames/exceptions.toml`
//! with its reason and must stay within the bar: at most 2 levels on at most 0.5 % of its pixels. A listed frame
//! that matches exactly is a stale entry and fails too. (The VS title card's own frames, drawn over noise, are
//! held by `bf-picture/tests/title_frames.rs`; the card as the episode shows it is in the first frames here.)
//!
//! Set `GOLDEN_OUT` to a folder to have every frame that differs written there as `f<frame>_rust.png`.
//! `timings` (ignored: `cargo test --release -p bf-content --test golden_frames -- --ignored --nocapture`)
//! measures a frame on one thread.

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use bf_content::{ContentError, build_picture};
use bf_core::recording::{Array, Recording};
use bf_picture::ring::Picture;
use bf_testdata::{ImgDiff, diff_u8, read_png, reference, write_png_rgb};

use common::EPISODE;

const EPISODE_NAME: &str = "flute_vs_trombone";
/// The frames `frames.json` lists (the export picks them by rule; the brief asked for at least 40).
const GOLDEN: usize = 43;
/// The bar of an exception: the largest channel difference, in levels.
const BAR_LEVELS: u8 = 2;
/// The bar of an exception: the share of pixels that may differ.
const BAR_SHARE: f64 = 0.005;

// ---------------------------------------------------------------- the reference
/// One golden frame: its number and its file.
struct Golden {
    frame: usize,
    file: String,
}

fn golden() -> Vec<Golden> {
    let doc = common::json(&format!("frames/{EPISODE_NAME}/frames.json"));
    assert_eq!(doc["episode"], EPISODE_NAME);
    assert_eq!(doc["nf"], 1830);
    assert_eq!(doc["size"], serde_json::json!([360, 640]));
    assert_eq!(
        doc["recording"]["file"],
        "studio/reference/fights/flute_vs_trombone/ep392.bfrec"
    );
    assert_eq!(
        doc["recording"]["hash"].as_str(),
        Some(EPISODE.rec.hash().as_str()),
        "not the recording Python drew the frames from"
    );
    let frames: Vec<Golden> = doc["frames"]
        .as_array()
        .expect("frames.json lists its frames")
        .iter()
        .map(|f| {
            let frame = f["frame"].as_u64().expect("a frame number") as usize;
            let file = f["file"].as_str().expect("a file name").to_string();
            assert_eq!(file, format!("f{frame:04}.png"));
            Golden { frame, file }
        })
        .collect();
    assert_eq!(frames.len(), GOLDEN, "the number of golden frames");
    assert!(
        frames.windows(2).all(|w| w[0].frame < w[1].frame),
        "the frames are listed once each, in order"
    );
    // The folder holds exactly these pictures: none is left unlisted.
    let dir = reference(&format!("frames/{EPISODE_NAME}"));
    let pngs = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
        .count();
    assert_eq!(pngs, GOLDEN, "the PNG files in {}", dir.display());
    frames
}

/// `exceptions.toml`: frame number to reason, for this episode. Panics on an entry that is not well formed.
fn exceptions() -> BTreeMap<usize, String> {
    let path = reference("frames/exceptions.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: toml::Value = toml::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let list = doc
        .get("exceptions")
        .and_then(toml::Value::as_array)
        .expect("exceptions.toml has the list `exceptions`");
    let mut out = BTreeMap::new();
    for entry in list {
        let episode = entry.get("episode").and_then(toml::Value::as_str);
        assert_eq!(episode, Some(EPISODE_NAME), "an exception names its episode: {entry}");
        let frame = entry
            .get("frame")
            .and_then(toml::Value::as_integer)
            .and_then(|f| usize::try_from(f).ok())
            .unwrap_or_else(|| panic!("an exception names its frame: {entry}"));
        let reason = entry.get("reason").and_then(toml::Value::as_str).unwrap_or("").trim();
        assert!(!reason.is_empty(), "the exception for frame {frame} has no reason");
        assert!(
            out.insert(frame, reason.to_string()).is_none(),
            "frame {frame} is listed twice"
        );
    }
    out
}

// ---------------------------------------------------------------- the picture
fn picture() -> &'static Picture {
    static PIC: OnceLock<Picture> = OnceLock::new();
    PIC.get_or_init(|| build_picture(Arc::new(EPISODE.rec.clone()), &EPISODE.ff).expect("the episode's picture builds"))
}

/// Every golden frame drawn here against Python's, in the list's order; drawn once for the two tests.
fn diffs() -> &'static [(usize, ImgDiff)] {
    static DIFFS: OnceLock<Vec<(usize, ImgDiff)>> = OnceLock::new();
    DIFFS.get_or_init(|| {
        let frames = golden();
        let pic = picture();
        let out = std::env::var_os("GOLDEN_OUT").map(std::path::PathBuf::from);
        let next = AtomicUsize::new(0);
        let workers = std::thread::available_parallelism().map_or(4, usize::from).min(8);
        let mut found: Vec<(usize, ImgDiff)> = std::thread::scope(|s| {
            let jobs: Vec<_> = (0..workers)
                .map(|_| {
                    s.spawn(|| {
                        let mut mine = Vec::new();
                        loop {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            let Some(g) = frames.get(i) else { break };
                            let path = reference(&format!("frames/{EPISODE_NAME}/{}", g.file));
                            let (w, h, c, want) = read_png(&path);
                            assert_eq!((w, h, c), (360, 640, 3), "{}", g.file);
                            let (got, _, report) = pic.frame_reported(g.frame);
                            assert_eq!((got.w, got.h, got.c), (w, h, c), "frame {}", g.frame);
                            assert_eq!(
                                (report.hull_fallbacks, report.hull_errors),
                                (0, 0),
                                "frame {}: a hull the kit's order is not known for",
                                g.frame
                            );
                            let d = diff_u8(&got.data, &want, w, h, c);
                            if d.differing != 0
                                && let Some(dir) = &out
                            {
                                std::fs::create_dir_all(dir).expect("GOLDEN_OUT can be made");
                                write_png_rgb(&dir.join(format!("f{:04}_rust.png", g.frame)), w, h, &got.data);
                            }
                            mine.push((g.frame, d));
                        }
                        mine
                    })
                })
                .collect();
            jobs.into_iter()
                .flat_map(|j| j.join().expect("a frame was drawn"))
                .collect()
        });
        found.sort_by_key(|(frame, _)| *frame);
        found
    })
}

fn within_the_bar(d: &ImgDiff) -> bool {
    d.max_diff <= BAR_LEVELS && d.differing as f64 <= BAR_SHARE * d.total as f64
}

// ---------------------------------------------------------------- the gate
/// Every golden frame is Python's bytes, or is listed in `exceptions.toml` with a reason and within the bar.
#[test]
fn all_golden_frames() {
    let listed = exceptions();
    let diffs = diffs();
    assert_eq!(diffs.len(), GOLDEN, "the number of frames drawn");
    let (mut exact, mut excepted, mut failing) = (0, 0, Vec::new());
    for (frame, d) in diffs {
        if d.differing == 0 {
            exact += 1;
        } else if listed.contains_key(frame) && within_the_bar(d) {
            excepted += 1;
            println!("frame {frame}: {d} (listed: {})", listed[frame]);
        } else {
            println!("frame {frame}: {d}");
            failing.push(*frame);
        }
    }
    println!(
        "golden frames: {exact} of {GOLDEN} are Python's bytes, {excepted} listed exceptions within the bar, {} \
         failing",
        failing.len()
    );
    assert!(failing.is_empty(), "frames that are not Python's: {failing:?}");
    assert_eq!(exact + excepted, GOLDEN);
}

/// Every entry of `exceptions.toml` names a golden frame, has a reason, is within the bar (at most 2 levels on
/// at most 0.5 % of the pixels) and still differs: an entry for a frame that matches exactly is stale.
#[test]
fn exceptions_are_within_the_bar() {
    let listed = exceptions();
    let diffs: BTreeMap<usize, ImgDiff> = diffs().iter().copied().collect();
    let mut wrong = Vec::new();
    for (frame, reason) in &listed {
        match diffs.get(frame) {
            None => wrong.push(format!("frame {frame} is listed but is no golden frame")),
            Some(d) if d.differing == 0 => {
                wrong.push(format!(
                    "frame {frame} is listed ({reason}) but matches exactly: remove the entry"
                ));
            }
            Some(d) if !within_the_bar(d) => wrong.push(format!("frame {frame} is outside the bar: {d}")),
            Some(d) => println!("frame {frame}: {d}, within the bar ({reason})"),
        }
    }
    println!("exceptions: {} listed", listed.len());
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The bar itself: 2 levels and 0.5 % of 360 x 640 pixels (1,152) pass, a level or a pixel more does not.
#[test]
fn the_bar_is_two_levels_on_half_a_percent() {
    let d = |differing, max_diff| ImgDiff {
        differing,
        max_diff,
        bbox: None,
        total: 360 * 640,
    };
    assert!(within_the_bar(&d(1152, 2)));
    assert!(!within_the_bar(&d(1153, 1)));
    assert!(!within_the_bar(&d(1, 3)));
}

// ---------------------------------------------------------------- what build_picture refuses
fn refusal(rec: &Recording, ff: &bf_core::fightfile::FightFile) -> ContentError {
    match build_picture(Arc::new(rec.clone()), ff) {
        Ok(_) => panic!("the picture was built"),
        Err(e) => e,
    }
}

/// The picture is shared by the preview and the render job's threads.
#[test]
fn the_picture_is_send_and_sync() {
    fn shared<T: Send + Sync>() {}
    shared::<Picture>();
}

#[test]
fn build_picture_refuses_what_validate_refuses() {
    let mut ff = EPISODE.ff.clone();
    ff.music.theme = "polka".into();
    assert_eq!(
        refusal(&EPISODE.rec, &ff).to_string(),
        r#"music.theme: "polka" is not a theme"#
    );
    assert_eq!(
        bf_content::validate(&ff).unwrap_err().to_string(),
        refusal(&EPISODE.rec, &ff).to_string()
    );
    // A fighter of the roster that is not ported (found at run time): the file is valid, but there is nothing to
    // draw it with. Once every fighter is ported, a name the registry does not know is refused the same way.
    let name = common::unported(0).unwrap_or_else(|| {
        println!("every roster fighter has its voice and part: refusing a name outside the roster instead");
        "KAZOO"
    });
    let ff = common::with_fighter(0, name);
    let err = refusal(&EPISODE.rec, &ff);
    assert!(matches!(err, ContentError::FightFile(_)), "{err:?}");
    assert_eq!(
        err.to_string(),
        bf_content::validate(&ff).unwrap_err().to_string(),
        "build_picture refuses what validate refuses"
    );
    if name != "KAZOO" {
        assert_eq!(
            err.to_string(),
            format!("fighters[0]: {name} has no part in the score yet")
        );
    }
}

#[test]
fn build_picture_refuses_a_pair_that_does_not_belong_together() {
    // No K.O.: no show, no picture.
    let rec = Recording::read(&reference("fights/flute_vs_trombone/no_ko.bfrec")).expect("the recording reads");
    let err = refusal(&rec, &EPISODE.ff);
    assert!(matches!(err, ContentError::Show(_)), "{err:?}");
    assert_eq!(err.to_string(), "the fight has no K.O. (no winner and K.O. time)");
    // The fight file of the same two fighters in the other corners.
    let mut ff = EPISODE.ff.clone();
    ff.fighters.reverse();
    let err = refusal(&EPISODE.rec, &ff);
    assert!(matches!(err, ContentError::Show(_)), "{err:?}");
    assert!(err.to_string().starts_with("the recording's fighters"), "{err}");
    // Another tempo.
    let mut ff = EPISODE.ff.clone();
    ff.tempo_bpm = 120.0;
    let err = refusal(&EPISODE.rec, &ff);
    assert!(matches!(err, ContentError::Show(_)), "{err:?}");
}

#[test]
fn build_picture_refuses_a_recording_without_what_a_frame_reads() {
    // A recording without its positions.
    let mut rec = EPISODE.rec.clone();
    rec.arrays.remove("P");
    let err = refusal(&rec, &EPISODE.ff);
    assert!(matches!(err, ContentError::Picture(_)), "{err:?}");
    assert!(!err.to_string().is_empty());
    // Values no fight gives must come back as a picture or an error, never as a panic.
    let mut rec = EPISODE.rec.clone();
    let Some(Array::F64 { data, .. }) = rec.arrays.get_mut("HP") else {
        panic!("the recording has no HP");
    };
    data.fill(150.0);
    if let Ok(pic) = build_picture(Arc::new(rec), &EPISODE.ff) {
        assert_eq!(pic.nf(), 1830);
    }
}

/// Every other recording of the two fighters in the reference set builds and draws with both real looks (every
/// 37th frame and the frames round the K.O.): no panic, no hull out of the kit's order. The swapped fights are
/// drawn under the fight file with its corners swapped.
#[test]
fn every_fight_of_the_pair_draws() {
    let mut files: Vec<String> = ["seed1", "seed2", "seed3", "seed500"].map(String::from).to_vec();
    let dir = reference("fights/flute_vs_trombone/scenarios");
    let mut scenarios: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            Some(format!("scenarios/{}", name.strip_suffix(".bfrec")?))
        })
        .collect();
    scenarios.sort();
    assert_eq!(scenarios.len(), 15, "the scenario recordings");
    files.extend(scenarios);
    let jobs: Vec<(usize, usize)> = std::thread::scope(|s| {
        let handles: Vec<_> = files
            .iter()
            .map(|file| {
                s.spawn(move || {
                    let path = reference(&format!("fights/flute_vs_trombone/{file}.bfrec"));
                    let rec = Recording::read(&path).expect("the recording reads");
                    let mut ff = EPISODE.ff.clone();
                    if rec.names[..] != ff.fighters[..] {
                        ff.fighters.swap(0, 1);
                    }
                    let swapped = usize::from(ff.fighters[0] != EPISODE.ff.fighters[0]);
                    let Some(t_ko) = rec.t_ko else {
                        // A fight nobody won has no video: said in words.
                        let err = refusal(&rec, &ff);
                        assert!(matches!(err, ContentError::Show(_)), "{file}: {err:?}");
                        return (swapped, 0);
                    };
                    let ko = (t_ko * 30.0) as usize;
                    let pic = build_picture(Arc::new(rec), &ff).unwrap_or_else(|e| panic!("{file}: {e}"));
                    let mut drawn = 0;
                    for f in (0..pic.nf()).step_by(37).chain(ko - 1..ko + 6) {
                        let (img, _, report) = pic.frame_reported(f);
                        assert_eq!((img.w, img.h, img.c), (360, 640, 3), "{file}, frame {f}");
                        assert_eq!((report.hull_fallbacks, report.hull_errors), (0, 0), "{file}, frame {f}");
                        drawn += 1;
                    }
                    (swapped, drawn)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("a fight drew")).collect()
    });
    let swapped: usize = jobs.iter().map(|j| j.0).sum();
    let frames: usize = jobs.iter().map(|j| j.1).sum();
    let no_ko = jobs.iter().filter(|j| j.1 == 0).count();
    println!(
        "{} fights ({swapped} with the corners swapped, {no_ko} without a K.O. refused), {frames} frames drawn",
        jobs.len()
    );
    assert_eq!((jobs.len(), swapped, no_ko), (19, 3, 1));
}

// ---------------------------------------------------------------- measurements
/// One frame through `build_picture` on one thread, over every 15th frame: the mean, the 95th percentile and
/// the hull fallbacks. Run it in release.
#[test]
#[ignore = "a measurement: cargo test --release -p bf-content --test golden_frames -- --ignored --nocapture"]
fn timings() {
    let t = Instant::now();
    let pic = build_picture(Arc::new(EPISODE.rec.clone()), &EPISODE.ff).expect("the episode's picture builds");
    println!("build_picture: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    let (mut ms, mut fallbacks, mut errors) = (Vec::new(), 0, 0);
    for f in (0..pic.nf()).step_by(15) {
        let t = Instant::now();
        let (_, _, report) = pic.frame_reported(f);
        ms.push((t.elapsed().as_secs_f64() * 1e3, f));
        fallbacks += report.hull_fallbacks;
        errors += report.hull_errors;
    }
    let mean = ms.iter().map(|m| m.0).sum::<f64>() / ms.len() as f64;
    ms.sort_by(|a, b| a.0.total_cmp(&b.0));
    let p95 = ms[(ms.len() * 95).div_ceil(100) - 1];
    let (lo, hi) = (ms[0], ms[ms.len() - 1]);
    println!(
        "{} frames (every 15th), one thread: mean {mean:.1} ms, 95th percentile {:.1} ms (frame {}), fastest {:.1} \
         ms (frame {}), slowest {:.1} ms (frame {})",
        ms.len(),
        p95.0,
        p95.1,
        lo.0,
        lo.1,
        hi.0,
        hi.1
    );
    println!("hull fallbacks: {fallbacks}, hull errors: {errors}");
}
