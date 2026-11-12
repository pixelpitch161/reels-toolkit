//! The app on the real engine (`engine::spawn`), driven as the owner drives it: storage under a temp folder of its
//! own, no sound device, and every wait a bounded wait on a condition.
//!
//! The engine sits behind a seam of the test's own ([`Watched`]): every command the app sends passes through it on
//! its way to the engine, so these tests prove that nothing they do asks for the folder dialog or Explorer (which
//! would open real windows outside the app). Such a command is held back, never passed on, and the test fails.
//!
//! `a_whole_video_with_the_real_engine` renders a whole video (minutes) and needs ffmpeg: it is ignored in the
//! normal run and run by name, in release:
//! `cargo test -p bf-app --release --test ui real::a_whole_video_with_the_real_engine -- --ignored`.

use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use bf_app::app::{BfApp, Boot};
use bf_app::engine::{self, EngineConfig, content};
use bf_app::model::queue::Queue;
use bf_app::model::{Settings, Step, View};
use bf_app::preview::audio::NoAudio;
use bf_app::seam::{Command, EngineHandle, Waker, channel};
use bf_app::store::settings::default_settings;
use bf_app::store::{Paths, Writer};
use bf_core::fightfile::FightFile;
use egui::Rect;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{REF, app_harness, gpu};

/// How often a wait looks again.
const TICK: Duration = Duration::from_millis(20);
/// How long the relay waits for a command before it passes the engine's events on.
const RELAY_TICK: Duration = Duration::from_millis(5);

thread_local! {
    /// The commands the relay of this test's app held back, looked at by every wait.
    static HELD: std::cell::RefCell<Option<Refused>> = const { std::cell::RefCell::new(None) };
    /// When this test must have ended, at the latest.
    static DEADLINE: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
}

/// A fresh folder under the system temp folder, removed when dropped: the settings, the queue, the shortlists and
/// the videos of these tests live here, never in the owner's own folders.
struct Temp(PathBuf);

impl Temp {
    fn new(test: &str) -> Temp {
        let root = std::env::temp_dir().join(format!("bf-app real {test} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Videos")).unwrap();
        Temp(root)
    }

    fn paths(&self) -> Paths {
        Paths::under(&self.0)
    }

    fn videos(&self) -> PathBuf {
        self.0.join("Videos")
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The commands held back because they would open a window outside the app.
type Refused = Arc<Mutex<Vec<Command>>>;

/// The real engine behind the test's own seam: commands from the app pass through on their way to the engine,
/// except `PickFolder` and `RevealFolder`, which are kept in `refused` and never passed on; the engine's events are
/// passed back and wake the app. When the engine stops, so does this, so `BfApp::close` sees it stop.
struct Watched {
    refused: Refused,
    relay: Option<JoinHandle<()>>,
}

impl Watched {
    fn start(waker: Waker, config: EngineConfig) -> (EngineHandle, Watched) {
        let (app_end, side) = channel(waker);
        let real = engine::spawn(Arc::new(|| {}), config);
        let refused: Refused = Arc::default();
        let held = refused.clone();
        let relay = std::thread::spawn(move || {
            loop {
                match side.commands.recv_timeout(RELAY_TICK) {
                    Ok(c @ (Command::PickFolder { .. } | Command::RevealFolder { .. })) => {
                        held.lock().unwrap().push(c);
                    }
                    Ok(c) => real.send(c),
                    // The app has gone: dropping `real` ends the engine.
                    Err(RecvTimeoutError::Disconnected) => return,
                    Err(RecvTimeoutError::Timeout) => {}
                }
                while let Some(e) = real.poll() {
                    side.emit(e);
                }
                if real.has_stopped() {
                    while let Some(e) = real.poll() {
                        side.emit(e);
                    }
                    return;
                }
            }
        });
        (
            app_end,
            Watched {
                refused,
                relay: Some(relay),
            },
        )
    }

    /// Fails the test if anything was held back.
    fn assert_nothing_refused(&self) {
        let refused = self.refused.lock().unwrap();
        assert!(refused.is_empty(), "the app asked for a window outside it: {refused:?}");
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        if let Some(relay) = self.relay.take() {
            let _ = relay.join();
        }
    }
}

/// The engine relay a harness was built with, once the harness has made it.
type Relay = Arc<Mutex<Option<Watched>>>;

/// The real app on the real engine: storage and videos under `temp`, no sound device, `workers` engine threads.
fn real_app<'a>(temp: &Temp, workers: usize) -> (Harness<'a, BfApp>, Relay) {
    let paths = temp.paths();
    let settings = Settings {
        output_folder: temp.videos(),
        workers,
        encode: bf_app::model::EncodeChoice::Best,
    };
    let watched: Relay = Arc::default();
    let keep = watched.clone();
    let harness = app_harness(REF, move |cc| {
        let ctx = cc.egui_ctx.clone();
        let config = EngineConfig {
            workers,
            paths: paths.clone(),
            clip: None,
            search_limit: None,
        };
        let (engine, w) = Watched::start(Arc::new(move || ctx.request_repaint()), config);
        let refused = w.refused.clone();
        HELD.with(|h| *h.borrow_mut() = Some(refused));
        *keep.lock().unwrap() = Some(w);
        BfApp::new(
            cc,
            Boot {
                engine,
                writer: Some(Writer::start(paths)),
                audio: Box::new(NoAudio::new("No sound device in tests.")),
                settings,
                queue: Queue::new(),
                notices: Vec::new(),
                cores: cores(),
            },
        )
    });
    (harness, watched)
}

fn cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

/// Closes the app as its window would (`BfApp::close`: the engine is stopped and waited for), checks it stopped
/// in time and that nothing asked for a window outside the app, and ends the relay.
fn close(mut harness: Harness<'_, BfApp>, watched: &Relay) {
    assert!(harness.state_mut().close(), "the engine stopped within the app's wait");
    drop(harness);
    let w = watched.lock().unwrap().take().expect("the engine was started");
    w.assert_nothing_refused();
}

/// Steps the app, looking again every 20 ms, until `done` holds; fails with `what` after `seconds`, at once when the
/// app asked for a window outside it (the relay held the command back), and when the test's own deadline has passed.
fn wait_until(harness: &mut Harness<'_, BfApp>, what: &str, seconds: u64, done: impl Fn(&Harness<'_, BfApp>) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while !done(harness) {
        HELD.with(|h| {
            if let Some(refused) = &*h.borrow() {
                let refused = refused.lock().unwrap();
                assert!(
                    refused.is_empty(),
                    "while waiting for {what}, the app asked for a window outside it: {refused:?}"
                );
            }
        });
        if let Some(end) = DEADLINE.with(std::cell::Cell::get) {
            assert!(
                Instant::now() < end,
                "the test ran past its deadline while waiting for {what}"
            );
        }
        assert!(Instant::now() < deadline, "waited {seconds} s for {what}");
        harness.step();
        std::thread::sleep(TICK);
    }
}

/// Every text on screen and where it is: names, and a label's text (its value).
fn texts(harness: &Harness<'_, BfApp>) -> Vec<(String, Rect)> {
    harness
        .query_all_by(|n| n.bounding_box().is_some())
        .flat_map(|n| {
            let node = n.accesskit_node();
            let rect = n.rect();
            [node.label(), node.value()]
                .into_iter()
                .flatten()
                .map(move |t| (t, rect))
        })
        .collect()
}

/// Whether some text on screen starts with `start`.
fn shows(harness: &Harness<'_, BfApp>, start: &str) -> bool {
    texts(harness).iter().any(|(t, _)| t.starts_with(start))
}

/// Whether some text on screen inside `within` starts with `start`.
fn shows_in(harness: &Harness<'_, BfApp>, start: &str, within: Rect) -> bool {
    texts(harness)
        .iter()
        .any(|(t, rect)| t.starts_with(start) && within.contains_rect(*rect))
}

/// The enabled button named `label`, if it is on screen.
fn button<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str) -> Option<egui_kittest::Node<'t>> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() == Role::Button && !n.accesskit_node().is_disabled())
}

/// Waits (up to `seconds`) for the button named `label` to be on screen and enabled, clicks it, and lets the click
/// land.
fn press(harness: &mut Harness<'_, BfApp>, label: &str, seconds: u64) {
    wait_until(harness, &format!("the button {label:?}"), seconds, |h| {
        button(h, label).is_some()
    });
    button(harness, label).expect("the button").click();
    harness.step();
    harness.step();
}

/// The shortlist's rows on screen, top to bottom: their names ("Seed 18433, ...").
fn rows(harness: &Harness<'_, BfApp>) -> Vec<String> {
    let mut rows: Vec<(String, Rect)> = harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| {
            let label = n.accesskit_node().label()?;
            (label.starts_with("Seed ") && label.contains(',')).then(|| (label, n.rect()))
        })
        .collect();
    rows.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    rows.into_iter().map(|(label, _)| label).collect()
}

/// The seed a row's name starts with ("Seed 18,433, ..." or "Seed 18433, ...").
fn seed_of(row: &str) -> u64 {
    row.trim_start_matches("Seed ")
        .split(", ")
        .next()
        .and_then(|n| n.replace([',', ' '], "").parse().ok())
        .unwrap_or_else(|| panic!("a seed in {row:?}"))
}

#[test]
fn the_real_match_up_screen() {
    let _gpu = gpu();
    let temp = Temp::new("matchup");
    let (mut harness, watched) = real_app(&temp, 2);
    // the counts follow the registry: every fighter it offers is ready
    let roster = content::roster();
    let ready = roster.fighters.iter().filter(|f| f.available).count();
    assert!((2..=20).contains(&ready), "{ready} ready");
    // the roster header: "20 fighters" once every fighter is built, else "N of 20 ready"
    let ready_line = if ready == 20 {
        "20 fighters".to_string()
    } else {
        format!("{ready} of 20 ready")
    };
    let themes = roster.themes.len();
    let status_line = format!(
        "{ready} fighters, {themes} {}",
        if themes == 1 { "theme" } else { "themes" }
    );
    let wanted = [
        "Snake charmer",
        "Slide lance",
        "C5 to A6",
        "E2 to F4",
        ready_line.as_str(),
        "A live lance knocks a risen cobra flat.",
        status_line.as_str(),
    ];
    wait_until(&mut harness, "the real Match-up screen", 10, |h| {
        wanted.iter().all(|w| shows(h, w))
    });
    // The pair rules came with the title card: wait until the stage shows it.
    wait_until(&mut harness, "the title card on the stage", 10, |h| {
        h.state().has_picture()
    });
    harness.run_ok();
    for w in wanted {
        assert!(shows(&harness, w), "{w:?} is on screen");
    }
    // Every fighter offered can be picked (the two in the corners are named for their corner); the ones not built
    // yet are there, and cannot be.
    for f in roster.fighters.iter().filter(|f| f.available) {
        let names = [
            f.display.clone(),
            format!("{}, left corner", f.display),
            format!("{}, right corner", f.display),
        ];
        let chip = names
            .iter()
            .find_map(|name| {
                harness
                    .query_all_by_label(name)
                    .find(|n| n.accesskit_node().role() == Role::Button)
            })
            .unwrap_or_else(|| panic!("a chip named {}", f.display));
        assert!(!chip.accesskit_node().is_disabled(), "{} can be picked", f.display);
    }
    let waiting: Vec<_> = roster.fighters.iter().filter(|f| !f.available).collect();
    assert_eq!(waiting.len(), 20 - ready);
    for f in waiting {
        let chip = harness
            .query_all_by_label(&f.display)
            .find(|n| n.accesskit_node().role() == Role::Button)
            .unwrap_or_else(|| panic!("a chip named {}", f.display));
        assert!(chip.accesskit_node().is_disabled(), "{} is disabled", f.display);
    }
    assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    // The status bar shows the roster's counts, not the sample note.
    let status = harness.state().layout.status;
    assert!(!shows_in(&harness, "Sample data", status));
    harness.snapshot("real_matchup_1440x900");
    close(harness, &watched);
}

/// The fight's video, by clicks, on the real engine: balance, search, a seed whose take passes every check, the
/// queue, and the files a finished video leaves.
#[test]
#[ignore = "renders a whole video (minutes) and needs ffmpeg: run it by name, in release"]
fn a_whole_video_with_the_real_engine() {
    let _gpu = gpu();
    let began = Instant::now();
    // The whole test, at most: about twice what it takes on a busy PC.
    DEADLINE.with(|d| d.set(Some(began + Duration::from_secs(20 * 60))));
    let temp = Temp::new("whole");
    let workers = default_settings(cores()).workers;
    let (mut harness, watched) = real_app(&temp, workers);
    wait_until(&mut harness, "the roster", 10, |h| h.state().state.vm.roster.is_some());

    press(&mut harness, "Balance this match-up", 30);
    wait_until(&mut harness, "a balance in range", 300, |h| {
        let b = &h.state().state.vm.balance;
        b.running.is_none() && b.stats.as_ref().is_some_and(|s| s.in_range()) && shows(h, "In range")
    });
    eprintln!("balanced after {:.1} s", began.elapsed().as_secs_f64());

    press(&mut harness, "Search seeds", 30);
    wait_until(&mut harness, "a row in the shortlist", 120, |h| !rows(h).is_empty());

    // A fight that fits the story can still fail a quality check: try the rows in order (bounded) until the Music
    // screen's checks all pass.
    const TRIES: usize = 8;
    let mut chosen = None;
    for i in 0..TRIES {
        if harness.state().state.vm.view != View::Step(Step::Seeds) {
            press(&mut harness, "3 Seeds", 30);
        }
        wait_until(&mut harness, &format!("row {}", i + 1), 300, |h| rows(h).len() > i);
        let row = rows(&harness).swap_remove(i);
        let seed = seed_of(&row);
        press(&mut harness, &row, 30);
        let caption = format!("Seed {seed}, the real picture and sound");
        wait_until(&mut harness, &format!("the take of seed {seed}"), 60, |h| {
            shows(h, &caption)
        });
        press(&mut harness, &format!("Use seed {seed}"), 60);
        wait_until(&mut harness, &format!("the loudness lines of seed {seed}"), 120, |h| {
            shows(h, "Loudness")
                && h.state()
                    .state
                    .vm
                    .take
                    .music
                    .as_ref()
                    .is_some_and(|m| !m.checks.is_empty())
        });
        let checks = harness
            .state()
            .state
            .vm
            .take
            .music
            .clone()
            .expect("the take's music")
            .checks;
        eprintln!(
            "seed {seed}: {}",
            checks
                .iter()
                .map(|c| format!("{} {}", c.title, if c.pass { "passes" } else { "fails" }))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if checks.iter().all(|c| c.pass) {
            chosen = Some(seed);
            break;
        }
    }
    let seed = chosen.unwrap_or_else(|| panic!("none of the first {TRIES} rows passed every check"));
    eprintln!("seed {seed} chosen after {:.1} s", began.elapsed().as_secs_f64());

    press(&mut harness, "Go to render", 30);
    press(&mut harness, "Add to queue", 60);
    let name = format!("Flute vs Trombone, seed {seed}");
    wait_until(&mut harness, "the video done", 600, |h| {
        h.query_all_by_label(&name)
            .find(|n| n.accesskit_node().role() == Role::Button)
            .is_some_and(|row| shows_in(h, "Done in", row.rect()))
    });
    let slowest = harness.state().slowest_update_ms;
    let caption = harness.state().state.vm.draft.captions.instagram.clone();
    eprintln!(
        "whole video: {:.1} s, seed {seed}, slowest update {slowest:.2} ms",
        began.elapsed().as_secs_f64()
    );
    close(harness, &watched);

    // The video's folder holds the video, its cover, its caption and its fight file.
    let folders: Vec<PathBuf> = std::fs::read_dir(temp.videos())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .collect();
    let [folder] = &folders[..] else {
        panic!("one video folder, not {folders:?}");
    };
    let file = |name: &str| -> PathBuf { folder.join(name) };
    for name in [
        "flute_vs_trombone.mp4",
        "cover.png",
        "caption.txt",
        "flute_vs_trombone.toml",
    ] {
        assert!(file(name).is_file(), "{name} in {}", folder.display());
    }
    let drafted = bf_content::draft_captions("FLUTE", "TROMBONE").caption;
    let text = std::fs::read_to_string(file("caption.txt")).unwrap();
    assert_eq!(text.lines().next(), Some(drafted.as_str()));
    assert_eq!(
        caption.lines().next(),
        Some(drafted.as_str()),
        "the caption box held the drafted caption"
    );
    let ff = FightFile::parse(&std::fs::read_to_string(file("flute_vs_trombone.toml")).unwrap()).expect("a fight file");
    assert_eq!(ff.seed, Some(seed));
    assert_eq!(format!("{:.2}", duration(&file("flute_vs_trombone.mp4"))), "61.01");
    assert!(slowest < 20.0, "the slowest update took {slowest} ms");
}

/// The length of the video at `path` in seconds, as ffprobe reads it.
fn duration(path: &Path) -> f64 {
    let ffprobe = bf_export::find_ffmpeg().expect("ffmpeg and ffprobe").ffprobe;
    let out = std::process::Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("a duration, not {out:?}"))
}
