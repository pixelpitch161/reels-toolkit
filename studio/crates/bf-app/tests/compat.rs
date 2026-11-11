//! Files saved by earlier versions of the app still read (packaging, Task 15).
//!
//! `tests/compat/<version>/` holds the three files a version of the app writes, exactly as that version wrote them:
//! `settings.toml` and `queue.json` (saved through the store's own `Writer`, as the app saves them) and `fight.toml`
//! (the fight file a render leaves in a video's folder). Each folder is made once, by
//! `make_this_versions_files` run with `BF_BLESS=1`, and then frozen: its files' SHA-256s are written below, and no
//! folder is ever made again or changed. Every release adds a folder for its own version.
//!
//! The tests read the frozen files with the code as it is now: the settings and the queue through the store, the
//! fight file through the real engine (the Library's scan and "Open", as the app does). They also show what happens
//! to a file the app cannot use: one from a newer version, and one that is cut short. Both are kept aside, byte for
//! byte, and never written over or deleted.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bf_app::engine::{self, EngineConfig};
use bf_app::model::draft::Draft;
use bf_app::model::queue::{JobSpec, JobState, JobUpdate, Queue, STEP_ORDER, StepState};
use bf_app::model::{Notice, Settings};
use bf_app::seam::{Command, EngineHandle, Event};
use bf_app::store::queue_file::{QUEUE_FORMAT, load_queue};
use bf_app::store::settings::{SETTINGS_FORMAT, default_settings, load_settings};
use bf_app::store::{Paths, Writer};
use bf_core::fightfile::{FIGHT_FORMAT, FightFile};
use sha2::{Digest, Sha256};

/// This build's version.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The frozen files of each version: (version, [(file, SHA-256)]). A folder under `tests/compat` that is not listed
/// here, or a file that does not match, fails `the_frozen_files_are_unchanged`.
const FROZEN: &[(&str, &[(&str, &str)])] = &[(
    "0.1.0",
    &[
        (
            "settings.toml",
            "77e34f78e6869735c75c11c83b3dc231dbec92dc5cc44d7596b8d5c171ec6c55",
        ),
        (
            "queue.json",
            "3ac384f3072a5a848d33b730e515c203808d0d3d37283f63ac6b8e7e4e46793b",
        ),
        (
            "fight.toml",
            "62e8d3080e2b91150374049254c1f549c4097af8b4c35564e66e5b008673e6ef",
        ),
    ],
)];

/// The three files of a version's folder.
const FILES: [&str; 3] = ["settings.toml", "queue.json", "fight.toml"];

/// Said when the queue was saved by another version and a video waits in it (`store::queue_file`).
const OTHER_VERSION: &str = "The queue was saved by another version of BALLFIGHT Studio. A queued video may come out \
                             as a different fight, so look at it before you render.";

// What `make_this_versions_files` saves, and so what the files of 0.1.0 mean.

/// The settings saved.
const OUTPUT_FOLDER: &str = r"D:\Vidéos du studio\BALLFIGHT";
const WORKERS: usize = 3;
/// The cores the settings are read with: more than `WORKERS`, so the value is not cut down.
const CORES: usize = 8;
/// The finished video's seed (its folder holds `fight.toml`) and the waiting video's seed.
const DONE_SEED: u64 = 20418;
const WAITING_SEED: u64 = 18433;
/// How long the finished video took, and where it was saved.
const DONE_SECONDS: f64 = 82.25;
const VIDEO_FOLDER: &str = "flute_vs_trombone";

/// The fighters' colours, from the roster.
const FLUTE: [u8; 3] = [222, 232, 246];
const TROMBONE: [u8; 3] = [120, 240, 184];

/// The folder of every version's files.
fn compat_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("compat")
}

/// The folder of one version's files.
fn folder_of(version: &str) -> PathBuf {
    compat_root().join(version)
}

/// The version folders under `tests/compat`, in name order.
fn saved_versions() -> Vec<String> {
    let mut versions: Vec<String> = fs::read_dir(compat_root())
        .map(|dir| {
            dir.filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    versions.sort();
    versions
}

/// One of a version's frozen files.
fn frozen(version: &str, file: &str) -> Vec<u8> {
    let path = folder_of(version).join(file);
    fs::read(&path).unwrap_or_else(|err| panic!("{} cannot be read: {err}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A fresh folder for one test under the system temp folder, removed when dropped. Its name holds spaces and letters
/// outside ASCII, as the owner's own folders may.
struct Temp(PathBuf);

impl Temp {
    fn new(test: &str) -> Temp {
        let root = std::env::temp_dir().join(format!("bf-app compat Zoë {test} {}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Temp(root)
    }

    fn paths(&self) -> Paths {
        Paths::under(&self.0)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The names in `dir`, sorted.
fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The real engine, keeping what it needs under `root`.
fn start_engine(root: &Path) -> EngineHandle {
    engine::spawn(
        Arc::new(|| {}),
        EngineConfig {
            workers: 1,
            paths: Paths::under(root),
            clip: None,
            search_limit: None,
        },
    )
}

/// The first event `pick` takes, waiting up to 60 s; other events are passed over.
fn wait_for(engine: &EngineHandle, what: &str, pick: impl Fn(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        while let Some(event) = engine.poll() {
            if pick(&event) {
                return event;
            }
        }
        assert!(Instant::now() < deadline, "{what} did not come within 60 s");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// What opening a fight answers: the draft and title, or the problem in words.
fn open(engine: &EngineHandle, command: Command) -> Result<(Draft, String), String> {
    engine.send(command);
    match wait_for(engine, "the answer to an open", |e| {
        matches!(e, Event::FightOpened { .. } | Event::Problem { .. })
    }) {
        Event::FightOpened { draft, title } => Ok((draft, title)),
        Event::Problem { text } => Err(text),
        _ => unreachable!(),
    }
}

/// The Library of `root`: its entries, or its problem.
fn library(engine: &EngineHandle, root: &Path) -> Vec<bf_app::model::library::LibraryEntry> {
    engine.send(Command::ScanLibrary {
        root: root.to_path_buf(),
    });
    let Event::Library { entries, problem, .. } = wait_for(
        engine,
        "the Library",
        |e| matches!(e, Event::Library { root: r, .. } if r == root),
    ) else {
        unreachable!()
    };
    assert_eq!(problem, None);
    entries
}

/// A video folder as a render leaves it, under `videos`: a cover and the fight file.
fn video_folder(videos: &Path, fight: &[u8]) -> PathBuf {
    let folder = videos.join(VIDEO_FOLDER);
    fs::create_dir_all(&folder).unwrap();
    // The Library lists a folder with a `cover.png`; the picture itself is not needed to list it.
    fs::write(folder.join("cover.png"), b"a cover").unwrap();
    fs::write(folder.join(format!("{VIDEO_FOLDER}.toml")), fight).unwrap();
    folder
}

/// Checks the draft a 0.1.0 fight of seed `seed` opens as.
fn check_draft_0_1_0(draft: &Draft, title: &str, seed: u64) {
    assert_eq!(draft.fighters, ["FLUTE", "TROMBONE"]);
    assert_eq!(draft.theme, THEME_0_1_0);
    assert_eq!(draft.bars, BARS_0_1_0);
    assert_eq!(draft.handicap, [1.0, 1.0]);
    assert_eq!(draft.seed, Some(seed));
    assert_eq!(draft.chord_loop, 0);
    assert_eq!(draft.bass, BASS_0_1_0);
    assert_eq!(draft.music_seed, 1);
    assert_eq!(draft.captions.instagram, INSTAGRAM_0_1_0);
    assert_eq!(draft.captions.tiktok, TIKTOK_0_1_0);
    assert_eq!(draft.captions.shorts, SHORTS_0_1_0);
    assert_eq!(title, format!("Flute vs Trombone, seed {seed}"));
}

/// What the fight of 0.1.0 opens as, beyond its seed (the starting pair's first theme, as a new match-up starts).
const THEME_0_1_0: &str = "big_top";
const BARS_0_1_0: u32 = 34;
/// The trombone plays bass.
const BASS_0_1_0: Option<usize> = Some(1);
const INSTAGRAM_0_1_0: &str = "Flute vs Trombone 🪈🎺 Comment the next match-up 👇\n#ballfight #flute #trombone \
                               #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly #gamedev #satisfying \
                               #music";
const TIKTOK_0_1_0: &str = "Flute vs Trombone 🪈🎺 Comment the next match-up 👇\n#fyp #ballfight #flute #trombone \
                            #bandkid #musictok #pixelart #satisfying #bandtok #music";
const SHORTS_0_1_0: &str = "Flute vs Trombone\nFlute vs Trombone, every hit in key. Comment the next match-up\n#shorts \
                            #ballfight #flute #trombone #music #pixelart";

#[test]
fn the_frozen_files_are_unchanged() {
    let listed: Vec<&str> = FROZEN.iter().map(|(version, _)| *version).collect();
    assert_eq!(
        saved_versions(),
        listed,
        "every folder under tests/compat has its SHA-256s in FROZEN"
    );
    for (version, files) in FROZEN {
        let names: Vec<&str> = files.iter().map(|(file, _)| *file).collect();
        assert_eq!(names, FILES, "{version}");
        for (file, sum) in *files {
            assert_eq!(sha256(&frozen(version, file)), *sum, "{version}/{file} was changed");
        }
    }
}

#[test]
fn settings_from_0_1_0_read() {
    let temp = Temp::new("settings_from_0_1_0_read");
    let path = temp.paths().settings();
    fs::create_dir_all(temp.paths().config).unwrap();
    fs::write(&path, frozen("0.1.0", "settings.toml")).unwrap();
    assert_eq!(
        load_settings(&path, CORES),
        (
            Settings {
                output_folder: PathBuf::from(OUTPUT_FOLDER),
                workers: WORKERS,
            },
            None
        )
    );
    // Read, so it stays where it is.
    assert_eq!(files_in(&temp.paths().config), vec!["settings.toml"]);
}

#[test]
fn a_queue_from_0_1_0_reads_with_its_warning() {
    let temp = Temp::new("a_queue_from_0_1_0_reads_with_its_warning");
    let path = temp.paths().queue();
    fs::create_dir_all(temp.paths().config).unwrap();
    fs::write(&path, frozen("0.1.0", "queue.json")).unwrap();
    let (mut queue, notice) = load_queue(&path);
    // Only a version other than 0.1.0 warns that the waiting video may come out as a different fight.
    let warning = (VERSION != "0.1.0").then(|| Notice {
        text: OTHER_VERSION.to_string(),
    });
    assert_eq!(notice, warning);
    assert_eq!(queue.jobs.len(), 2, "{queue:?}");
    assert!(!queue.paused);
    assert_eq!(queue.next_id, 3);
    let spec = |seed| JobSpec {
        title: "Flute vs Trombone".to_string(),
        colors: [FLUTE, TROMBONE],
        theme: THEME_DISPLAY_0_1_0.to_string(),
        seed,
        // Checked below by opening it.
        fight_toml: String::new(),
        out_root: PathBuf::from(OUTPUT_FOLDER),
    };
    let done = queue.job(1).expect("the finished video").clone();
    assert_eq!(
        done.state,
        JobState::Done {
            folder: Path::new(OUTPUT_FOLDER).join(VIDEO_FOLDER),
            seconds: DONE_SECONDS,
        }
    );
    assert_eq!(
        JobSpec {
            fight_toml: String::new(),
            ..done.spec.clone()
        },
        spec(DONE_SEED)
    );
    assert_eq!(done.steps.len(), STEP_ORDER.len());
    assert!(
        done.steps.iter().all(|line| line.state == StepState::Done),
        "{:?}",
        done.steps
    );
    assert!(!done.interrupted);
    let waiting = queue.job(2).expect("the waiting video").clone();
    assert_eq!(waiting.state, JobState::Waiting);
    assert_eq!(
        JobSpec {
            fight_toml: String::new(),
            ..waiting.spec.clone()
        },
        spec(WAITING_SEED)
    );
    assert!(waiting.steps.is_empty());
    assert!(!waiting.interrupted);
    // The queue shows the waiting video first, and after a restart it waits for the owner.
    assert_eq!(queue.shown(), vec![2, 1]);
    assert_eq!(queue.after_restart(), 0);
    assert!(queue.paused);
    // Both jobs' fights still open, as the same fights.
    let engine_home = Temp::new("a_queue_from_0_1_0_reads_with_its_warning engine");
    let engine = start_engine(&engine_home.0);
    for (job, seed) in [(&done, DONE_SEED), (&waiting, WAITING_SEED)] {
        let (draft, title) = open(
            &engine,
            Command::OpenFightText {
                toml: job.spec.fight_toml.clone(),
            },
        )
        .unwrap_or_else(|problem| panic!("seed {seed}: {problem}"));
        check_draft_0_1_0(&draft, &title, seed);
    }
    // The finished video's fight is the one its folder holds.
    let fight = String::from_utf8(frozen("0.1.0", "fight.toml")).unwrap();
    assert_eq!(
        FightFile::parse(&done.spec.fight_toml).unwrap(),
        FightFile::parse(&fight).unwrap()
    );
    engine.send(Command::Shutdown);
    // Read, so it stays where it is.
    assert_eq!(files_in(&temp.paths().config), vec!["queue.json"]);
}

/// The theme's shown name, as the queue holds it.
const THEME_DISPLAY_0_1_0: &str = "Big top";

#[test]
fn a_fight_file_from_0_1_0_reopens() {
    let temp = Temp::new("a_fight_file_from_0_1_0_reopens");
    let videos = temp.0.join("Vidéos ! BALLFIGHT");
    let bytes = frozen("0.1.0", "fight.toml");
    let folder = video_folder(&videos, &bytes);
    let engine_home = Temp::new("a_fight_file_from_0_1_0_reopens engine");
    let engine = start_engine(&engine_home.0);
    // The Library lists it as a video that can be reopened.
    let entries = library(&engine, &videos);
    assert_eq!(entries.len(), 1, "{entries:?}");
    let entry = &entries[0];
    assert_eq!(entry.problem, None);
    assert_eq!(entry.title, "Flute vs Trombone");
    assert_eq!(entry.seed, Some(DONE_SEED));
    assert_eq!(entry.colors, [FLUTE, TROMBONE]);
    let path = folder.join(format!("{VIDEO_FOLDER}.toml"));
    assert_eq!(entry.fight_path.as_deref(), Some(path.as_path()));
    // "Open" gives the fight back.
    let (draft, title) = open(&engine, Command::OpenFight { path: path.clone() }).unwrap();
    check_draft_0_1_0(&draft, &title, DONE_SEED);
    assert_eq!(draft.base.as_deref().map(str::as_bytes), Some(bytes.as_slice()));
    assert!(draft.captions_edited);
    engine.send(Command::Shutdown);
    // Nothing was written.
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

/// Every version's files read with this build, whatever version saved them: the settings with no notice, the queue
/// with its jobs (and at most the other-version warning), and the fight files of the queue and of the video folder.
#[test]
fn every_saved_version_still_reads() {
    let engine_home = Temp::new("every_saved_version_still_reads engine");
    let engine = start_engine(&engine_home.0);
    let versions = saved_versions();
    assert!(versions.iter().any(|v| v == "0.1.0"), "{versions:?}");
    for version in &versions {
        let temp = Temp::new(&format!("every_saved_version_still_reads {version}"));
        let paths = temp.paths();
        fs::create_dir_all(&paths.config).unwrap();
        fs::write(paths.settings(), frozen(version, "settings.toml")).unwrap();
        fs::write(paths.queue(), frozen(version, "queue.json")).unwrap();
        let (settings, notice) = load_settings(&paths.settings(), CORES);
        assert_eq!(notice, None, "{version}");
        assert_ne!(settings, default_settings(CORES), "{version}");
        let (queue, notice) = load_queue(&paths.queue());
        assert!(!queue.jobs.is_empty(), "{version}");
        if let Some(notice) = notice {
            assert_eq!(notice.text, OTHER_VERSION, "{version}");
            assert_ne!(version, VERSION);
        }
        for job in &queue.jobs {
            let opened = open(
                &engine,
                Command::OpenFightText {
                    toml: job.spec.fight_toml.clone(),
                },
            );
            assert!(opened.is_ok(), "{version}, job {}: {opened:?}", job.id);
        }
        let videos = temp.0.join("videos");
        let bytes = frozen(version, "fight.toml");
        let folder = video_folder(&videos, &bytes);
        let entries = library(&engine, &videos);
        assert_eq!(entries.len(), 1, "{version}");
        assert_eq!(entries[0].problem, None, "{version}");
        let path = folder.join(format!("{VIDEO_FOLDER}.toml"));
        assert!(open(&engine, Command::OpenFight { path }).is_ok(), "{version}");
        assert_eq!(
            files_in(&paths.config),
            vec!["queue.json", "settings.toml"],
            "{version}"
        );
    }
    engine.send(Command::Shutdown);
}

/// The two files of the app's config folder after a save: the settings and the queue are saved again through the
/// `Writer`, as the app saves them when anything changes.
fn save_both(paths: &Paths) {
    let writer = Writer::start(paths.clone());
    writer.settings(&default_settings(CORES));
    writer.queue(&Queue::new());
    writer.flush();
    assert_eq!(writer.problems(), Vec::<String>::new());
}

/// A file the app cannot use is kept aside under `kept` with its bytes as they were, and the next saves never write
/// over it or delete it; a fight file in a video folder is never written at all.
fn check_kept(test: &str, settings: &[u8], queue: &[u8], fight: &[u8], tag: &str) {
    let temp = Temp::new(test);
    let paths = temp.paths();
    fs::create_dir_all(&paths.config).unwrap();
    fs::write(paths.settings(), settings).unwrap();
    fs::write(paths.queue(), queue).unwrap();
    let (loaded, notice) = load_settings(&paths.settings(), CORES);
    assert_eq!(loaded, default_settings(CORES));
    let kept_settings = format!("settings.{tag}-1.toml");
    assert!(notice.expect("a notice").text.ends_with(&format!(" {kept_settings}.")));
    let (loaded, notice) = load_queue(&paths.queue());
    assert_eq!(loaded, Queue::new());
    let kept_queue = format!("queue.{tag}-1.json");
    assert!(notice.expect("a notice").text.contains(&kept_queue));
    assert_eq!(files_in(&paths.config), vec![kept_queue.clone(), kept_settings.clone()]);
    // The app saves over the places they were, twice; the kept files stay as they were.
    save_both(&paths);
    save_both(&paths);
    let mut expected = vec![
        "queue.json".to_string(),
        kept_queue.clone(),
        "settings.toml".to_string(),
        kept_settings.clone(),
    ];
    expected.sort();
    assert_eq!(files_in(&paths.config), expected);
    assert_eq!(fs::read(paths.config.join(&kept_settings)).unwrap(), settings);
    assert_eq!(fs::read(paths.config.join(&kept_queue)).unwrap(), queue);
    // The fight file: the Library names the problem, "Open" refuses it in words, and the file is left as it was.
    let videos = temp.0.join("videos");
    let folder = video_folder(&videos, fight);
    let engine_home = Temp::new(&format!("{test} engine"));
    let engine = start_engine(&engine_home.0);
    let entries = library(&engine, &videos);
    assert_eq!(entries.len(), 1);
    assert!(entries[0].problem.is_some(), "{entries:?}");
    let path = folder.join(format!("{VIDEO_FOLDER}.toml"));
    let refused = open(&engine, Command::OpenFight { path: path.clone() }).unwrap_err();
    assert!(
        refused.starts_with(&format!("{VIDEO_FOLDER}.toml cannot be opened: ")),
        "{refused}"
    );
    engine.send(Command::Shutdown);
    assert_eq!(fs::read(&path).unwrap(), fight);
    assert_eq!(
        files_in(&folder),
        vec!["cover.png".to_string(), format!("{VIDEO_FOLDER}.toml")]
    );
}

/// Replaces the first `from` in `bytes` with `to`, which must be there.
fn replaced(bytes: &[u8], from: &str, to: &str) -> Vec<u8> {
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(text.contains(from), "{from:?} is not in {text}");
    text.replacen(from, to, 1).into_bytes()
}

/// A newer version's files (0.1.0's, with a format number above the newest this build reads): the app runs on its
/// defaults, keeps them aside as `*.newer-1.*`, and never writes over them.
#[test]
fn files_from_a_newer_version_are_kept_and_never_written_over() {
    let above = |n: u32| n + 1;
    check_kept(
        "files_from_a_newer_version_are_kept_and_never_written_over",
        &replaced(
            &frozen("0.1.0", "settings.toml"),
            "format = 1",
            &format!("format = {}", above(SETTINGS_FORMAT)),
        ),
        &replaced(
            &frozen("0.1.0", "queue.json"),
            "\"format\": 1",
            &format!("\"format\": {}", above(QUEUE_FORMAT)),
        ),
        &replaced(
            &frozen("0.1.0", "fight.toml"),
            "format = 1",
            &format!("format = {}", above(FIGHT_FORMAT)),
        ),
        "newer",
    );
}

/// Files cut short, as a full drive or a crash in another program could leave them: kept aside as
/// `*.corrupt-1.*`, and never written over.
#[test]
fn files_cut_short_are_kept_and_never_written_over() {
    let half = |file: &str| {
        let bytes = frozen("0.1.0", file);
        bytes[..bytes.len() / 2].to_vec()
    };
    check_kept(
        "files_cut_short_are_kept_and_never_written_over",
        &half("settings.toml"),
        &half("queue.json"),
        &half("fight.toml"),
        "corrupt",
    );
}

/// Makes this version's folder of files, once, with the app's own code: run with `BF_BLESS=1`
/// (`cargo test -p bf-app --test compat make_this_versions_files`). Without it, does nothing. Refuses to touch a
/// folder that is already there: a version's files are made once and then frozen.
///
/// The fight is made as the app makes a new match-up: the roster's starting pair and first theme, the drafted
/// captions, then a seed, queued with the engine's `Enqueue`. The settings and the queue (the first seed's video
/// finished, the second's waiting) are saved by the store's `Writer`. The video folder's fight file is what a render
/// writes there: the queued fight file read and written out again (`bf_export::run_job` writes `ff.to_toml()` of the
/// fight it was given).
#[test]
fn make_this_versions_files() {
    if std::env::var_os("BF_BLESS").is_none_or(|v| v != "1") {
        return;
    }
    let target = folder_of(VERSION);
    assert!(
        FILES.iter().all(|file| !target.join(file).exists()),
        "{} already holds saved files: they are frozen and are never made again",
        target.display()
    );
    let engine_home = Temp::new("make_this_versions_files engine");
    let engine = start_engine(&engine_home.0);
    // A new match-up, as the interface starts one when the roster arrives.
    engine.send(Command::LoadRoster);
    let Event::Roster(roster) = wait_for(&engine, "the roster", |e| matches!(e, Event::Roster(_))) else {
        unreachable!()
    };
    let theme = roster.themes.first().expect("a theme");
    let can_bass = |side: usize| {
        roster
            .fighters
            .iter()
            .find(|f| f.name == roster.start[side])
            .and_then(|f| f.card.as_ref())
            .is_some_and(|card| card.can_play_bass)
    };
    let mut draft = Draft {
        fighters: roster.start.clone(),
        theme: theme.name.clone(),
        bars: 34u32.max(theme.bars.0).min(theme.bars.1),
        handicap: [1.0, 1.0],
        seed: None,
        chord_loop: 0,
        bass: (0..2).find(|&side| can_bass(side)),
        music_seed: 1,
        captions: bf_app::model::draft::CaptionTexts {
            instagram: String::new(),
            tiktok: String::new(),
            shorts: String::new(),
        },
        captions_edited: false,
        base: None,
    };
    assert_eq!(draft.fighters, ["FLUTE", "TROMBONE"], "the starting pair");
    engine.send(Command::DescribeFight {
        r#gen: 1,
        draft: draft.clone(),
    });
    let Event::FightInfo { captions, problem, .. } = wait_for(&engine, "the fight's description", |e| {
        matches!(e, Event::FightInfo { .. })
    }) else {
        unreachable!()
    };
    assert_eq!(problem, None);
    draft.captions = captions;
    let out_root = PathBuf::from(OUTPUT_FOLDER);
    let mut queue_spec = |seed: u64| {
        draft.seed = Some(seed);
        // The seed has a K.O., so its video can be made.
        let ff = bf_app::engine::convert::fight_file(&draft).unwrap();
        let setup = bf_sim::setup_from(&ff, bf_content::roster(), seed).unwrap();
        let rec = bf_sim::run(&setup, true).unwrap();
        assert!(rec.winner.is_some() && rec.t_ko.is_some(), "seed {seed} has no K.O.");
        engine.send(Command::Enqueue {
            draft: draft.clone(),
            out_root: out_root.clone(),
        });
        match wait_for(&engine, "the queued job", |e| {
            matches!(e, Event::Enqueued { .. } | Event::EnqueueFailed { .. })
        }) {
            Event::Enqueued { spec } => spec,
            other => panic!("seed {seed}: {other:?}"),
        }
    };
    let done_spec = queue_spec(DONE_SEED);
    let waiting_spec = queue_spec(WAITING_SEED);
    engine.send(Command::Shutdown);
    let mut queue = Queue::new();
    let done = queue.add(done_spec.clone());
    queue.start(done);
    for key in STEP_ORDER {
        queue.apply(
            done,
            JobUpdate::Step {
                key,
                state: StepState::Done,
                note: None,
            },
        );
    }
    queue.apply(
        done,
        JobUpdate::Done {
            folder: out_root.join(VIDEO_FOLDER),
            seconds: DONE_SECONDS,
        },
    );
    queue.add(waiting_spec);
    // Saved as the app saves them.
    let temp = Temp::new("make_this_versions_files");
    let paths = temp.paths();
    let writer = Writer::start(paths.clone());
    writer.settings(&Settings {
        output_folder: out_root.clone(),
        workers: WORKERS,
    });
    writer.queue(&queue);
    writer.flush();
    assert_eq!(writer.problems(), Vec::<String>::new());
    drop(writer);
    let fight = FightFile::parse(&done_spec.fight_toml).unwrap().to_toml();
    fs::create_dir_all(&target).unwrap();
    fs::copy(paths.settings(), target.join("settings.toml")).unwrap();
    fs::copy(paths.queue(), target.join("queue.json")).unwrap();
    fs::write(target.join("fight.toml"), fight).unwrap();
    for file in FILES {
        println!("{file} {}", sha256(&fs::read(target.join(file)).unwrap()));
    }
}
