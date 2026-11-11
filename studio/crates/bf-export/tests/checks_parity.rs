//! The checks against the Python kit's on the example episode (FLUTE against TROMBONE, seed 392).
//!
//! `studio/reference/checks/flute_vs_trombone.json` (the export tool's `checks`) holds what each check of
//! `kit/checks.py` gave: its verdict and the numbers behind the lines it printed. Each check here runs on the
//! same inputs: the kit's own `audio.wav`, the stems of the Rust soundtrack (equal to the kit's, see
//! `bf-content/tests/stems.rs`) and the frames the Rust picture draws (equal to the kit's, see
//! `bf-content/tests/golden_frames.rs`). The bars: every verdict equal; levels and loudness within 0.01 dB; band
//! within 0.1 dB; the flash counts equal and the largest area within 0.1 point; the loop numbers within 1 %; the
//! worst circle of confusion equal; the safe zone passing with the same boxes.
//!
//! The counts of the reference file are asserted (31 windows, 2 x 5 band sections, 315 flash frames, 178 loop
//! steps, 244 safe-zone frames with 5,503 boxes, 1,830 focus frames with 39,671 points), so an empty file cannot
//! pass. Run with `-- --nocapture` to see each check's sentence and how long it took.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

use bf_content::{build_picture, build_soundtrack};
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_export::checks::flash::{analyse, flash_from, rel_lum};
use bf_export::checks::levels::profile;
use bf_export::checks::safezone::frames as safezone_frames;
use bf_export::checks::{
    BandPlan, CheckResult, as_wav16, band, flash, flash_span, focus, levels, loop_audio, loop_picture, loudness,
    pre_render, safezone,
};
use bf_picture::ring::Picture;
use bf_picture::ui::{DrawLog, Kind};
use bf_picture::{Gray32, Rgb8};
use bf_sound::master::Soundtrack;
use bf_sound::{NA, SR};
use bf_story::{BeatSheet, Story, story};
use bf_testdata::{reference, repo_root};
use serde_json::Value;

// ---------------------------------------------------------------- the episode and the reference
struct Episode {
    rec: Arc<Recording>,
    ff: FightFile,
    show: Show,
    story: Story,
    pic: Picture,
    plan: BandPlan,
    low_names: [String; 2],
}

static EPISODE: LazyLock<Episode> = LazyLock::new(|| {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let rec = Arc::new(rec);
    let path = repo_root().join("studio/fights/flute_vs_trombone.toml");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ff = FightFile::parse(&text).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a show");
    let pic = build_picture(Arc::clone(&rec), &ff).expect("the episode's picture builds");
    let cfg = &pic.ctx().cfg;
    let low_names = [cfg[0].low.name.clone(), cfg[1].low.name.clone()];
    let sheet = BeatSheet::series([&cfg[0], &cfg[1]]);
    let story = story(&rec, &show.grid, &sheet, [&low_names[0], &low_names[1]]);
    let plan = BandPlan::of(&ff).expect("the episode's fighters and theme are in the app");
    Episode {
        rec,
        ff,
        show,
        story,
        pic,
        plan,
        low_names,
    }
});

/// `checks` of the reference file, after its facts were checked against the episode.
static KIT: LazyLock<Value> = LazyLock::new(|| {
    let path = reference("checks/flute_vs_trombone.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let ep = &*EPISODE;
    assert_eq!(doc["episode"], "flute_vs_trombone");
    assert_eq!(doc["episode_facts"]["seed"].as_u64(), Some(ep.rec.seed));
    assert_eq!(doc["episode_facts"]["t_ko"].as_f64(), Some(ep.show.t_ko));
    assert_eq!(doc["episode_facts"]["t_fight"].as_f64(), Some(ep.show.grid.t_fight));
    assert_eq!(doc["episode_facts"]["frames"].as_u64(), Some(ep.pic.nf() as u64));
    let checks = doc["checks"].clone();
    let names: Vec<&str> = checks
        .as_object()
        .expect("the checks")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        names,
        ["band", "flash", "focus", "levels", "loop", "loudness", "safezone"]
    );
    checks
});

/// The kit's `audio.wav` as the checks read it: each 16-bit sample divided by 32,768.
static WAV: LazyLock<Vec<[f64; 2]>> = LazyLock::new(|| {
    let path = reference("sound/flute_vs_trombone/audio.wav");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!(
        (&bytes[0..4], &bytes[8..16], &bytes[36..40]),
        (&b"RIFF"[..], &b"WAVEfmt "[..], &b"data"[..])
    );
    let frames: Vec<[f64; 2]> = bytes[44..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|f| {
            let (l, r) = (i16::from_le_bytes([f[0], f[1]]), i16::from_le_bytes([f[2], f[3]]));
            [f64::from(l) / 32768.0, f64::from(r) / 32768.0]
        })
        .collect();
    assert_eq!(frames.len(), NA, "audio.wav's length");
    frames
});

/// The episode's soundtrack as the Studio builds it.
static BUILT: LazyLock<Soundtrack> = LazyLock::new(|| {
    let ep = &*EPISODE;
    build_soundtrack(&ep.rec, &ep.ff).expect("the episode's soundtrack builds")
});

fn workers() -> usize {
    std::thread::available_parallelism().map_or(4, usize::from).min(12)
}

/// Frames of the episode's picture with their draw logs, drawn once and kept for every test.
static DRAWN: LazyLock<Mutex<BTreeMap<usize, Frame>>> = LazyLock::new(|| Mutex::new(BTreeMap::new()));

/// A drawn frame and what the picture filed while it drew it.
type Frame = Arc<(Rgb8, DrawLog)>;

fn frames(want: &[usize]) -> Vec<Frame> {
    let pic = &EPISODE.pic;
    let mut kept = DRAWN.lock().unwrap();
    let todo: Vec<usize> = want
        .iter()
        .copied()
        .collect::<BTreeSet<usize>>()
        .into_iter()
        .filter(|f| !kept.contains_key(f))
        .collect();
    if !todo.is_empty() {
        let started = Instant::now();
        let next = AtomicUsize::new(0);
        let out = Mutex::new(Vec::new());
        std::thread::scope(|s| {
            for _ in 0..workers() {
                s.spawn(|| {
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&f) = todo.get(i) else { break };
                        let drawn = pic.frame_logged(f);
                        out.lock().unwrap().push((f, Arc::new(drawn)));
                    }
                });
            }
        });
        kept.extend(out.into_inner().unwrap());
        println!(
            "drew {} frames in {:.1} s on {} threads",
            todo.len(),
            started.elapsed().as_secs_f64(),
            workers()
        );
    }
    want.iter().map(|f| Arc::clone(&kept[f])).collect()
}

// ---------------------------------------------------------------- comparing
fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("{v} is not a number"))
}

fn list(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("{v} is not a list"))
}

fn got(r: &CheckResult, key: &str) -> f64 {
    *r.numbers
        .get(key)
        .unwrap_or_else(|| panic!("{}: no number {key:?} among {:?}", r.name, r.numbers.keys()))
}

/// `got` within `bar` of `want` (dB, or a share).
fn within(what: &str, got: f64, want: f64, bar: f64) {
    assert!(
        (got - want).abs() <= bar,
        "{what}: {got} against the kit's {want} (bar {bar})"
    );
}

/// `got` within 1 % of `want`.
fn one_percent(what: &str, got: f64, want: f64) {
    assert!(
        (got - want).abs() <= 0.01 * want.abs(),
        "{what}: {got} against the kit's {want} (bar 1 %)"
    );
}

fn verdict(r: &CheckResult, kit: &Value) {
    assert_eq!(
        Some(r.pass),
        kit["pass"].as_bool(),
        "{}: the verdict is not the kit's: {}",
        r.name,
        r.summary
    );
}

/// One plain sentence: a capital first, one full stop at the very end, no line break.
fn one_sentence(r: &CheckResult) {
    let s = &r.summary;
    assert!(s.chars().next().is_some_and(char::is_uppercase), "{}: {s:?}", r.name);
    assert!(s.ends_with('.') && !s.contains('\n'), "{}: {s:?}", r.name);
    assert!(!s.contains(". "), "{}: more than one sentence: {s:?}", r.name);
    assert!(!s.contains("NaN") && !s.contains("inf"), "{}: {s:?}", r.name);
}

fn timed<T>(what: &str, run: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let out = run();
    println!("{what}: {:.2} s", started.elapsed().as_secs_f64());
    out
}

fn say(r: &CheckResult) {
    println!("{}: {} | {}", r.name, if r.pass { "PASS" } else { "FAIL" }, r.summary);
    one_sentence(r);
}

// ---------------------------------------------------------------- levels
#[test]
fn levels_match_the_kit() {
    let kit = &KIT["levels"];
    let r = timed("levels", || levels(&WAV));
    say(&r);
    assert_eq!(r.name, "levels");
    verdict(&r, kit);
    for key in ["ending", "opening", "opening_minus_ending", "build", "spread"] {
        within(&format!("levels {key}"), got(&r, key), num(&kit[key]), 0.01);
    }
    assert_eq!(got(&r, "dips"), list(&kit["dips"]).len() as f64);
    assert_eq!(got(&r, "opening_hits_per_s"), num(&kit["opening_hits_per_s"]));
    // the profile, window by window
    let mono: Vec<f64> = WAV.iter().map(|f| (f[0] + f[1]) / 2.0).collect();
    let rows = profile(&mono, 62.0);
    let want = list(&kit["rows"]);
    assert_eq!(want.len(), 31, "the kit's windows");
    assert_eq!(rows.len(), want.len());
    let mut worst = 0.0f64;
    for (row, w) in rows.iter().zip(want) {
        assert_eq!(row.t, num(&w["t"]));
        within(&format!("the window at {} s", row.t), row.dbfs, num(&w["dbfs"]), 0.01);
        assert_eq!(
            row.hits_per_s,
            num(&w["hits_per_s"]),
            "hits in the window at {} s",
            row.t
        );
        worst = worst.max((row.dbfs - num(&w["dbfs"])).abs());
    }
    println!("levels: 31 windows, worst difference {worst:.3e} dB; every hit count equal");
}

#[test]
fn a_quiet_opening_fails_levels() {
    // The brief's case: the first 2 s at half level. In the kit that is the one deliberate dip the rule allows
    // (the opening's 10 s mean drops by 1.2 dB, inside its 1.5 dB), so it passes there and here. What
    // `python -m reelkit.levels` printed for audio.wav changed this way (run by hand on the reference PC):
    //   2 s:  opening -1.2 dB OK, build +0.4 OK, spread 3.6 OK, dips 1 at 0 s (-5.9) OK: PASS
    //   4 s:  opening -2.4 dB TOO QUIET, build +0.8 OK, spread 8.4 UNEVEN, dips 2 at 0 s, 2 s TOO MANY: FAIL
    //   10 s: opening -6.0 dB TOO QUIET, build +2.1 TOO MUCH, spread 8.4 UNEVEN, dips 5 TOO MANY: FAIL
    let mut x = WAV.clone();
    for f in &mut x[..2 * SR] {
        *f = [f[0] * 0.5, f[1] * 0.5];
    }
    let r = levels(&x);
    say(&r);
    assert!(r.pass, "{}", r.summary);
    assert_eq!(got(&r, "dips"), 1.0);
    within("opening", got(&r, "opening_minus_ending"), 0.047 - 6.0206 / 5.0, 0.01);
    // The first 4 s at half level are two dips, and the opening is 2.4 dB under the ending: both named.
    for f in &mut x[2 * SR..4 * SR] {
        *f = [f[0] * 0.5, f[1] * 0.5];
    }
    let r = levels(&x);
    say(&r);
    assert!(!r.pass);
    assert!(r.summary.contains("opening"), "{}", r.summary);
    assert!(r.summary.contains("2.4 dB"), "{}", r.summary);
    assert_eq!(got(&r, "dips"), 2.0);
    within("spread", got(&r, "spread"), 8.4, 0.05);
    within("build", got(&r, "build"), 0.8, 0.05);
    assert!(
        r.summary.contains("loudest") && !r.summary.contains("second half"),
        "{}",
        r.summary
    );
    // The whole opening (10 s) at half level.
    let mut x = WAV.clone();
    for f in &mut x[..10 * SR] {
        *f = [f[0] * 0.5, f[1] * 0.5];
    }
    let r = levels(&x);
    say(&r);
    assert!(!r.pass);
    assert!(r.summary.contains("opening"), "{}", r.summary);
    within("opening", got(&r, "opening_minus_ending"), 0.047 - 6.0206, 0.01);
    assert_eq!(got(&r, "dips"), 5.0);
    within("build", got(&r, "build"), 2.1, 0.05);
    assert!(
        r.summary.contains("second half") && r.summary.contains("5 windows"),
        "{}",
        r.summary
    );
}

// ---------------------------------------------------------------- band
#[test]
fn band_matches_the_kit() {
    let kit = &KIT["band"];
    let ep = &*EPISODE;
    // the plan the fight file gives is the kit's: its fighters' buses and bands, its bed buses in order
    let fighters = list(&kit["fighters"]);
    assert_eq!(fighters.len(), 2);
    for (k, f) in fighters.iter().enumerate() {
        assert_eq!(f["bus"], ep.plan.sounds[k].0.as_str());
        assert_eq!(num(&f["band"][0]), ep.plan.sounds[k].1.0);
        assert_eq!(num(&f["band"][1]), ep.plan.sounds[k].1.1);
    }
    let bed: Vec<&str> = list(&kit["bed_buses"]).iter().map(|b| b.as_str().unwrap()).collect();
    assert_eq!(bed, ep.plan.bed_buses);
    let sounds = [
        (ep.plan.sounds[0].0.as_str(), ep.plan.sounds[0].1),
        (ep.plan.sounds[1].0.as_str(), ep.plan.sounds[1].1),
    ];
    let r = timed("band", || band(&BUILT, &ep.show, sounds, &bed));
    say(&r);
    assert_eq!(r.name, "band");
    verdict(&r, kit);
    let (mut n, mut worst) = (0, 0.0f64);
    for f in fighters {
        let bus = f["bus"].as_str().unwrap();
        for (kind, key) in [("section", "section"), ("while it sounds", "while_it_sounds")] {
            let want = list(&f[key]);
            assert_eq!(want.len(), 5, "the kit's sections");
            for (i, w) in want.iter().enumerate() {
                let g = got(&r, &format!("{bus} {kind} {}", i + 1));
                within(&format!("band {bus} {kind} {}", i + 1), g, num(w), 0.1);
                worst = worst.max((g - num(w)).abs());
                n += 1;
            }
        }
    }
    assert_eq!(n, 20);
    println!("band: 20 numbers, worst difference {worst:.3e} dB");
}

#[test]
fn a_fighter_under_the_bed_fails_band() {
    let ep = &*EPISODE;
    let mut st = BUILT.clone();
    for f in st.stems.get_mut("trombone").expect("the trombone's stem") {
        *f = [f[0] * 0.25, f[1] * 0.25];
    }
    let sounds = [
        (ep.plan.sounds[0].0.as_str(), ep.plan.sounds[0].1),
        (ep.plan.sounds[1].0.as_str(), ep.plan.sounds[1].1),
    ];
    let bed: Vec<&str> = ep.plan.bed_buses.iter().map(String::as_str).collect();
    let r = band(&st, &ep.show, sounds, &bed);
    say(&r);
    assert!(!r.pass);
    assert!(
        r.summary.contains("TROMBONE") && !r.summary.contains("FLUTE"),
        "{}",
        r.summary
    );
}

// ---------------------------------------------------------------- loudness
#[test]
fn loudness_matches_the_kit() {
    let kit = &KIT["loudness"];
    let r = timed("loudness", || loudness(&WAV, &EPISODE.show));
    say(&r);
    assert_eq!(r.name, "loudness");
    // the kit sets no pass mark for this number: it is measured and shown
    assert!(kit.get("pass").is_none());
    assert!(r.pass);
    for key in ["intro_lufs", "fight_lufs", "intro_minus_fight"] {
        within(&format!("loudness {key}"), got(&r, key), num(&kit[key]), 0.01);
        println!(
            "loudness {key}: {:.3e} from the kit's",
            (got(&r, key) - num(&kit[key])).abs()
        );
    }
}

// ---------------------------------------------------------------- flash
fn kit_flash_span() -> (usize, usize) {
    let kit = &KIT["flash"];
    (
        kit["f0"].as_u64().unwrap() as usize,
        kit["f1"].as_u64().unwrap() as usize,
    )
}

#[test]
fn flash_matches_the_kit() {
    let kit = &KIT["flash"];
    let ep = &*EPISODE;
    let span = flash_span(&ep.show, &ep.story, [&ep.low_names[0], &ep.low_names[1]]);
    assert_eq!(span, kit_flash_span());
    assert_eq!(span, (1230, 1545));
    let want: Vec<usize> = (span.0..span.1).collect();
    let lum: Vec<Gray32> = frames(&want).iter().map(|d| rel_lum(&d.0)).collect();
    let r = timed("flash", || flash(&lum));
    say(&r);
    assert_eq!(r.name, "flash");
    verdict(&r, kit);
    assert_eq!(got(&r, "general_flashes"), num(&kit["general_flashes"]));
    assert_eq!(got(&r, "worst_in_1s"), num(&kit["worst_in_1s"]));
    // 0.1 point of the frame's share
    within(
        "largest area",
        got(&r, "largest_area"),
        num(&kit["largest_area"]),
        0.001,
    );
    assert_eq!(
        got(&r, "largest_area_frame") + span.0 as f64,
        num(&kit["largest_area_frame"])
    );
    // frame by frame
    let a = analyse(&lum, 30);
    let area = list(&kit["area"]);
    assert_eq!(area.len(), 315, "the kit's frames");
    assert_eq!(a.area.len(), area.len());
    let mut worst = 0.0f64;
    for (j, (g, w)) in a.area.iter().zip(area).enumerate() {
        within(&format!("the area of frame {}", span.0 + j), *g, num(w), 0.001);
        worst = worst.max((g - num(w)).abs());
    }
    let flashes: Vec<usize> = list(&kit["flash_frames"])
        .iter()
        .map(|f| f.as_u64().unwrap() as usize)
        .collect();
    assert_eq!(a.flashes.iter().map(|j| span.0 + j).collect::<Vec<_>>(), flashes);
    println!("flash: 315 frames, worst difference of a frame's share {worst:.3e}");
}

#[test]
fn four_white_frames_in_a_second_fail_flash() {
    let (black, white) = (
        Rgb8::filled(360, 640, &[0, 0, 0]),
        Rgb8::filled(360, 640, &[255, 255, 255]),
    );
    let lum: Vec<Gray32> = (0..30)
        .map(|f| rel_lum(if [3, 9, 15, 21].contains(&f) { &white } else { &black }))
        .collect();
    let r = flash(&lum);
    say(&r);
    assert!(!r.pass);
    assert_eq!(got(&r, "general_flashes"), 4.0);
    assert_eq!(got(&r, "worst_in_1s"), 4.0);
    assert_eq!(got(&r, "largest_area"), 1.0);
    assert!(r.summary.contains("4 flashes"), "{}", r.summary);
    // three in a second are allowed
    let lum: Vec<Gray32> = (0..30)
        .map(|f| rel_lum(if [3, 9, 15].contains(&f) { &white } else { &black }))
        .collect();
    let r = flash(&lum);
    assert!(r.pass, "{}", r.summary);
    assert_eq!(got(&r, "worst_in_1s"), 3.0);
}

// ---------------------------------------------------------------- loop
#[test]
fn loop_matches_the_kit() {
    let kit = &KIT["loop"];
    let nf = EPISODE.pic.nf();
    assert_eq!(kit["frames"]["tail"], serde_json::json!([nf - 150, nf]));
    assert_eq!(kit["frames"]["head"], serde_json::json!([0, 30]));
    let want: Vec<usize> = (nf - 150..nf).chain(0..30).collect();
    let drawn = frames(&want);
    let pics: Vec<Rgb8> = drawn.iter().map(|d| d.0.clone()).collect();
    let (tail, head) = pics.split_at(150);
    let p = timed("loop picture", || loop_picture(tail, head));
    say(&p);
    assert_eq!(p.name, "loop_picture");
    let a = timed("loop audio", || loop_audio(&WAV));
    say(&a);
    assert_eq!(a.name, "loop_audio");
    // the kit's one verdict covers both halves
    assert_eq!(Some(p.pass && a.pass), kit["pass"].as_bool());
    let kp = &kit["picture"];
    assert_eq!(got(&p, "fails"), num(&kp["fails"]));
    for key in ["seam", "normal", "ratio"] {
        one_percent(&format!("loop {key}"), got(&p, key), num(&kp[key]));
    }
    assert_eq!(got(&p, "big_jumps"), list(&kp["big_jumps"]).len() as f64);
    assert_eq!(list(&kp["big_jumps"]).len(), 26);
    one_percent(
        "the frame nearest to frame 0",
        got(&p, "nearest_to_frame_0"),
        num(&kp["nearest_to_frame_0"]["mad"]),
    );
    // step by step
    let steps = bf_export::checks::loopcheck::steps(tail, head);
    let want = list(&kp["steps"]);
    assert_eq!(want.len(), 178, "the kit's steps");
    assert_eq!(steps.len(), want.len());
    for (i, (g, w)) in steps.iter().zip(want).enumerate() {
        one_percent(&format!("step {i}"), *g, num(w));
    }
    let exact = steps.iter().zip(want).filter(|(g, w)| **g == num(w)).count();
    println!("loop picture: {exact} of 178 steps are the kit's to the bit");
    let ka = &kit["audio"];
    assert_eq!(got(&a, "fails"), num(&ka["fails"]));
    for key in ["last_20ms_db", "first_20ms_db", "step_db", "typical_step", "jump"] {
        one_percent(&format!("loop audio {key}"), got(&a, key), num(&ka[key]));
        println!(
            "loop audio {key}: {:.3e} from the kit's",
            (got(&a, key) - num(&ka[key])).abs()
        );
    }
}

#[test]
fn a_cut_before_the_end_fails_loop() {
    // a white frame two seconds before the end: a one-off jump, as a reset before the file ends would be
    let nf = EPISODE.pic.nf();
    let want: Vec<usize> = (nf - 150..nf).chain(0..30).collect();
    let mut pics: Vec<Rgb8> = frames(&want).iter().map(|d| d.0.clone()).collect();
    let white = Rgb8::filled(360, 640, &[255, 255, 255]);
    pics[90] = white;
    let (tail, head) = pics.split_at(150);
    let r = loop_picture(tail, head);
    say(&r);
    assert!(!r.pass);
    assert!(r.summary.contains("2.00 s before the end"), "{}", r.summary);
    // a click at the seam
    let mut x = WAV.clone();
    let last = x.len() - 1;
    x[last] = [0.9, 0.9];
    let r = loop_audio(&x);
    say(&r);
    assert!(!r.pass);
    assert!(r.summary.contains("click"), "{}", r.summary);
}

// ---------------------------------------------------------------- focus
#[test]
fn focus_matches_the_kit() {
    let kit = &KIT["focus"];
    let r = timed("focus", || focus(&EPISODE.pic));
    say(&r);
    assert_eq!(r.name, "focus");
    verdict(&r, kit);
    assert_eq!(got(&r, "worst_coc"), num(&kit["worst_coc"]));
    assert_eq!(got(&r, "frames"), num(&kit["frames"]));
    assert_eq!(got(&r, "frames"), 1830.0);
    assert_eq!(got(&r, "points"), num(&kit["points"]));
    assert_eq!(got(&r, "points"), 39671.0);
}

// ---------------------------------------------------------------- safe zone
fn safe_logs() -> Vec<(f64, DrawLog)> {
    let times = safezone_frames(EPISODE.pic.nf(), 30);
    let numbers: Vec<usize> = times.iter().map(|&(_, f)| f).collect();
    frames(&numbers)
        .iter()
        .zip(&times)
        .map(|(d, &(t, _))| (t, d.1.clone()))
        .collect()
}

#[test]
fn safezone_matches_the_kit() {
    let kit = &KIT["safezone"];
    let logs = safe_logs();
    let r = timed("safezone", || safezone(&logs));
    say(&r);
    assert_eq!(r.name, "safezone");
    verdict(&r, kit);
    assert_eq!(got(&r, "frames_checked"), num(&kit["frames_checked"]));
    assert_eq!(got(&r, "frames_checked"), 244.0);
    assert_eq!(got(&r, "tracked"), num(&kit["tracked"]));
    assert_eq!(got(&r, "tracked"), 5503.0);
    assert_eq!(got(&r, "outside"), list(&kit["outside"]).len() as f64);
    assert_eq!(got(&r, "overlaps"), list(&kit["overlaps"]).len() as f64);
    // every box is the kit's: the same times, the same frames, the same boxes in drawing order
    let want = list(&kit["boxes"]);
    assert_eq!(want.len(), 244);
    let times = safezone_frames(EPISODE.pic.nf(), 30);
    let mut n = 0;
    for ((w, (t, log)), (_, f)) in want.iter().zip(&logs).zip(&times) {
        assert_eq!(num(&w["t"]), *t);
        assert_eq!(w["frame"].as_u64(), Some(*f as u64));
        let boxes = list(&w["boxes"]);
        assert_eq!(boxes.len(), log.boxes.len(), "the boxes of frame {f}");
        for (b, g) in boxes.iter().zip(&log.boxes) {
            let ours = serde_json::json!([g.x0, g.y0, g.x1, g.y1, g.kind.as_str(), g.info]);
            assert_eq!(*b, ours, "a box of frame {f}");
            n += 1;
        }
    }
    assert_eq!(n, 5503);
}

#[test]
fn a_caption_at_x_345_fails_safezone() {
    let mut logs = safe_logs();
    // the first caption line of the video, moved so that it starts at x = 345
    let (t, b) = logs
        .iter_mut()
        .find_map(|(t, log)| log.boxes.iter_mut().find(|b| b.kind == Kind::Caption).map(|b| (*t, b)))
        .expect("a caption");
    let width = b.x1 - b.x0;
    (b.x0, b.x1) = (345, 345 + width);
    let info = b.info.clone();
    let r = safezone(&logs);
    say(&r);
    assert!(!r.pass);
    assert_eq!(got(&r, "outside"), 1.0);
    assert_eq!(got(&r, "worst_px"), f64::from(345 + width - 340));
    assert!(r.summary.contains(&info), "{}", r.summary);
    assert!(r.summary.contains(&format!("{t:.1} s")), "{}", r.summary);
}

// ---------------------------------------------------------------- fights the episode is not
#[test]
fn an_early_or_a_missing_ko_does_not_panic() {
    let ep = &*EPISODE;
    let sounds = [
        (ep.plan.sounds[0].0.as_str(), ep.plan.sounds[0].1),
        (ep.plan.sounds[1].0.as_str(), ep.plan.sounds[1].1),
    ];
    let bed: Vec<&str> = ep.plan.bed_buses.iter().map(String::as_str).collect();
    let lows = [ep.low_names[0].as_str(), ep.low_names[1].as_str()];
    // a K.O. at 30 s: the last section (44 s to the K.O.) does not exist and is left out, where the kit would stop
    // with a Python error; the four fixed sections are measured as ever
    let mut show = ep.show.clone();
    show.t_ko = 30.0;
    let r = band(&BUILT, &show, sounds, &bed);
    say(&r);
    assert!(r.pass);
    assert_eq!(r.numbers.len(), 16, "{:?}", r.numbers.keys());
    assert!(!r.numbers.contains_key("flute section 5"));
    assert_eq!(
        got(&r, "flute section 2"),
        got(&band(&BUILT, &ep.show, sounds, &bed), "flute section 2")
    );
    assert!(loudness(&WAV, &show).pass);
    // the stretch is 8 s before the K.O. less 1 s, to 1.5 s after it
    assert_eq!(flash_span(&show, &ep.story, lows), (630, 945));
    // no first use of a desperation move at all: the same rule, on the episode's own K.O.
    let mut story = ep.story.clone();
    story.firsts.clear();
    let ko = ep.show.t_ko;
    assert_eq!(
        flash_span(&ep.show, &story, lows),
        (((ko - 9.0) * 30.0) as usize, ((ko + 1.5) * 30.0) as usize)
    );
    // a K.O. so late that the stretch would pass the end of the video stops there
    show.t_ko = 60.5;
    assert_eq!(flash_span(&show, &story, lows).1, 1830);
    // a K.O. time that is not a number (no recording gives one): every check still answers
    show.t_ko = f64::NAN;
    let r = loudness(&WAV, &show);
    say(&r);
    assert!(!r.pass && r.summary.contains("cannot be compared"), "{}", r.summary);
    assert!(band(&BUILT, &show, sounds, &bed).pass);
    let (f0, f1) = flash_span(&show, &ep.story, lows);
    assert!(f0 <= f1 && f1 <= 1830);
}

// ---------------------------------------------------------------- all of them, as the render job runs them
#[test]
fn pre_render_runs_every_check() {
    let ep = &*EPISODE;
    let rs = timed("pre_render", || {
        pre_render(&ep.pic, &BUILT, &ep.show, &ep.story, &ep.plan, workers())
    });
    let names: Vec<&str> = rs.iter().map(|r| r.name).collect();
    assert_eq!(
        names,
        [
            "focus",
            "safezone",
            "levels",
            "band",
            "loudness",
            "flash",
            "loop_picture",
            "loop_audio"
        ]
    );
    for r in &rs {
        say(r);
        assert!(r.pass, "{}: {}", r.name, r.summary);
    }
    // the same results as each check on its own inputs: the Studio's master as the wav file holds it is the
    // kit's audio.wav sample for sample, and the frames do not depend on which thread drew them
    assert_eq!(as_wav16(&BUILT.master), *WAV);
    let nf = ep.pic.nf();
    let span = kit_flash_span();
    let lum: Vec<Gray32> = frames(&(span.0..span.1).collect::<Vec<_>>())
        .iter()
        .map(|d| rel_lum(&d.0))
        .collect();
    let pics: Vec<Rgb8> = frames(&(nf - 150..nf).chain(0..30).collect::<Vec<_>>())
        .iter()
        .map(|d| d.0.clone())
        .collect();
    let (tail, head) = pics.split_at(150);
    let sounds = [
        (ep.plan.sounds[0].0.as_str(), ep.plan.sounds[0].1),
        (ep.plan.sounds[1].0.as_str(), ep.plan.sounds[1].1),
    ];
    let bed: Vec<&str> = ep.plan.bed_buses.iter().map(String::as_str).collect();
    let alone = vec![
        focus(&ep.pic),
        safezone(&safe_logs()),
        levels(&WAV),
        band(&BUILT, &ep.show, sounds, &bed),
        loudness(&WAV, &ep.show),
        flash_from(&lum, span.0),
        loop_picture(tail, head),
        loop_audio(&WAV),
    ];
    assert_eq!(rs, alone);
    // one worker gives the same
    if std::env::var_os("CHECKS_ONE_WORKER").is_some() {
        let one = timed("pre_render, one worker", || {
            pre_render(&ep.pic, &BUILT, &ep.show, &ep.story, &ep.plan, 1)
        });
        assert_eq!(one, rs);
    }
}
