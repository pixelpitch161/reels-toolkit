//! The render queue's file, `config/queue.json`: the videos waiting and the last finished ones, so the queue survives
//! a restart (§9).

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{clear_unread, keep_copy, mark_unread, move_aside, next_aside_name, read_patiently, write_atomic};
use crate::model::queue::{Job, JobState, Queue};
use crate::model::{JobId, Notice};

/// The queue file's format. A file with a higher number was written by a newer app and is left alone.
pub const QUEUE_FORMAT: u32 = 1;

/// This app's version, written into the file: a fight is only promised to come out the same within one version.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Said when the file was saved by another version and videos wait in it.
const OTHER_VERSION: &str = "The queue was saved by another version of BALLFIGHT Studio. A queued video may come out \
                             as a different fight, so look at it before you render.";

/// The queue as the file holds it, for writing.
#[derive(Serialize)]
struct QueueFileOut<'a> {
    format: u32,
    app_version: &'a str,
    paused: bool,
    next_id: JobId,
    jobs: &'a [Job],
}

/// The queue as the file holds it, for reading (the format is looked at first, on its own).
#[derive(Deserialize)]
struct QueueFileIn {
    app_version: String,
    paused: bool,
    next_id: JobId,
    jobs: Vec<Job>,
}

/// What a queue file turned out to hold.
enum Read {
    /// The queue, and the version of the app that saved it.
    Good(Queue, String),
    /// Written by a newer app.
    Newer,
    /// Not text, not JSON, cut short, missing a value, or holding one id twice.
    Corrupt,
}

fn parse(bytes: &[u8]) -> Read {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Read::Corrupt;
    };
    // The format is looked at first: a newer file may hold its other values differently.
    match value.get("format").and_then(serde_json::Value::as_u64) {
        Some(format) if format > u64::from(QUEUE_FORMAT) => return Read::Newer,
        Some(format) if format == u64::from(QUEUE_FORMAT) => {}
        _ => return Read::Corrupt,
    }
    let Ok(file) = serde_json::from_value::<QueueFileIn>(value) else {
        return Read::Corrupt;
    };
    let mut ids = BTreeSet::new();
    if !file.jobs.iter().all(|job| ids.insert(job.id)) {
        return Read::Corrupt;
    }
    // A new job never takes an id already in the queue.
    let after_highest = ids.last().map_or(1, |&id| id.saturating_add(1));
    let queue = Queue {
        jobs: file.jobs,
        paused: file.paused,
        next_id: file.next_id.max(after_highest),
    };
    Read::Good(queue, file.app_version)
}

/// Reads the queue file. A missing file gives an empty queue and no notice. A file that cannot be understood (a
/// half-written one too) or was written by a newer app gives an empty queue and a notice in words, and is moved aside
/// as `queue.corrupt-N.json` or `queue.newer-N.json` rather than deleted (or, if another program holds it, copied
/// there). A file that cannot be opened at all gives an empty queue and a notice, and is left where it is; the writer
/// moves it aside as `queue.unread-N.json` before it saves the queue over it (or refuses to save). A file saved by
/// another version of the app keeps its jobs, and says so if any of them wait.
///
/// Called once, at start, before the first frame: it may wait a moment on a file another program holds open.
pub fn load_queue(path: &Path) -> (Queue, Option<Notice>) {
    let bytes = match read_patiently(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            clear_unread(path);
            return (Queue::new(), None);
        }
        Err(_) => {
            // Kept where it is; the writer moves it aside before it saves the queue over it.
            mark_unread(path);
            let text = format!(
                "The render queue file could not be opened, so the queue starts empty for now. It may be open in \
                 another program: close that program and start the app again to get your queue back. If you add a \
                 video first, the old file is kept as {}.",
                next_aside_name(path, "unread")
            );
            return (Queue::new(), Some(Notice { text }));
        }
    };
    clear_unread(path);
    let (tag, kept, not_kept) = match parse(&bytes) {
        Read::Good(queue, version) => {
            let waits = queue
                .jobs
                .iter()
                .any(|job| matches!(job.state, JobState::Waiting | JobState::Rendering { .. }));
            let notice = (version != APP_VERSION && waits).then(|| Notice {
                text: OTHER_VERSION.to_string(),
            });
            return (queue, notice);
        }
        Read::Newer => (
            "newer",
            "The render queue was saved by a newer version of BALLFIGHT Studio, so it was left alone as {}. Videos \
             you add now go in a fresh queue.",
            "The render queue was saved by a newer version of BALLFIGHT Studio, so the app starts with a fresh queue.",
        ),
        Read::Corrupt => (
            "corrupt",
            "The render queue could not be read, so it starts empty. The old file was kept as {}.",
            "The render queue could not be read, so it starts empty.",
        ),
    };
    let text = match move_aside(path, tag).or_else(|_| keep_copy(path, tag, &bytes)) {
        Ok(aside) => kept.replace("{}", &aside.file_name().unwrap_or_default().to_string_lossy()),
        Err(_) => format!(
            "{not_kept} The old file could not be moved aside, so copy {} somewhere safe before you add a video.",
            path.display()
        ),
    };
    (Queue::new(), Some(Notice { text }))
}

/// Writes the queue file (`format`, `app_version`, `paused`, `next_id`, `jobs`) atomically, making its folder if
/// needed. What is written is a trimmed copy (the 20 newest finished jobs), with a job that is rendering written as
/// waiting, first in line and marked as interrupted, so that after a restart it starts over. `q` itself is not
/// changed: the app calls `Queue::trim` on its own queue when a job finishes.
pub fn save_queue(path: &Path, q: &Queue) -> io::Result<()> {
    let mut saved = q.clone();
    saved.rendering_waits_again();
    saved.trim();
    let file = QueueFileOut {
        format: QUEUE_FORMAT,
        app_version: APP_VERSION,
        paused: saved.paused,
        next_id: saved.next_id,
        jobs: &saved.jobs,
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    write_atomic(path, &bytes)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::model::JobId;
    use crate::model::queue::{FailHint, JobSpec, JobState, JobUpdate, StepKey, StepState};
    use crate::store::{TempRoot, Writer, files_in};

    const OTHER_VERSION: &str = "The queue was saved by another version of BALLFIGHT Studio. A queued video may come out \
                                 as a different fight, so look at it before you render.";

    fn spec(title: &str) -> JobSpec {
        JobSpec {
            title: title.to_string(),
            colors: [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]],
            theme: "Big top".to_string(),
            seed: 18433,
            fight_toml: "seed = 18433\n[left]\nname = \"FLUTE\"\n".to_string(),
            out_root: PathBuf::from(r"C:\Users\Zoë Ångström\Videos\BALLFIGHT 動画"),
        }
    }

    fn done() -> JobUpdate {
        JobUpdate::Done {
            folder: r"C:\Users\Zoë Ångström\Videos\BALLFIGHT 動画\flute_vs_trombone".into(),
            seconds: 48.25,
        }
    }

    fn notice(text: &str) -> Option<Notice> {
        Some(Notice { text: text.to_string() })
    }

    /// A queue with a job of every finished kind and two waiting.
    fn mixed() -> Queue {
        let mut q = Queue::new();
        let a = q.add(spec("Flute vs Trombone"));
        let b = q.add(spec("Tuba vs Bass drum"));
        let c = q.add(spec("Oboe vs Harp"));
        q.add(spec("Alto sax vs Tenor sax"));
        q.add(spec("Piccolo vs Cello"));
        q.start(a);
        q.apply(a, done());
        q.start(b);
        q.apply(
            b,
            JobUpdate::Step {
                key: StepKey::Loudness,
                state: StepState::Failed,
                note: Some("2.1 dB".to_string()),
            },
        );
        q.apply(
            b,
            JobUpdate::Failed {
                headline: "Stopped at the loudness check".to_string(),
                detail: "Nothing was saved.".to_string(),
                hint: FailHint::OpenInMusic,
            },
        );
        q.start(c);
        q.apply(c, JobUpdate::Cancelled);
        q
    }

    fn one_rendering() -> (Queue, JobId, JobId) {
        let mut q = Queue::new();
        let a = q.add(spec("Flute vs Trombone"));
        let b = q.add(spec("Tuba vs Bass drum"));
        q.start(a);
        q.apply(
            a,
            JobUpdate::Frames {
                done: 1_171,
                total: 1_830,
                seconds: 40.0,
            },
        );
        (q, a, b)
    }

    #[test]
    fn queue_round_trip() {
        let root = TempRoot::new("queue_round_trip");
        let path = root.paths().queue();
        let q = mixed();
        save_queue(&path, &q).unwrap();
        assert_eq!(load_queue(&path), (q.clone(), None));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("{\n  \"format\": 1,\n  \"app_version\": "), "{text}");
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["app_version", "format", "jobs", "next_id", "paused"]);
        assert_eq!(value["format"], 1);
        assert_eq!(value["app_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["next_id"], 6);
        assert_eq!(value["paused"], false);
        assert_eq!(files_in(&root.paths().config), vec!["queue.json"]);
        // A paused queue stays paused.
        let mut paused = q;
        paused.paused = true;
        save_queue(&path, &paused).unwrap();
        assert_eq!(load_queue(&path), (paused, None));
    }

    #[test]
    fn a_rendering_job_is_saved_as_waiting() {
        let root = TempRoot::new("a_rendering_job_is_saved_as_waiting");
        let path = root.paths().queue();
        let (q, a, b) = one_rendering();
        let before = q.clone();
        save_queue(&path, &q).unwrap();
        // The queue in memory is not changed by saving it.
        assert_eq!(q, before);
        let (loaded, notice) = load_queue(&path);
        assert_eq!(notice, None);
        let job = loaded.job(a).unwrap();
        assert_eq!(job.state, JobState::Waiting);
        assert!(job.interrupted);
        assert!(job.steps.is_empty());
        assert_eq!(loaded.rendering(), None);
        assert_eq!(loaded.shown(), vec![a, b]);
    }

    // Review Focus 4
    #[test]
    fn a_saved_rendering_job_is_reported_after_loading() {
        let root = TempRoot::new("a_saved_rendering_job_is_reported_after_loading");
        let path = root.paths().queue();
        let (q, a, b) = one_rendering();
        save_queue(&path, &q).unwrap();
        let (mut loaded, _) = load_queue(&path);
        assert_eq!(loaded.after_restart(), 1);
        assert_eq!(loaded.counts(), (0, 2));
        assert_eq!(loaded.job(a).unwrap().state, JobState::Waiting);
        assert_eq!(loaded.job(b).unwrap().state, JobState::Waiting);
        assert!(loaded.paused);
        assert_eq!(loaded.shown(), vec![a, b]);
    }

    #[test]
    fn saving_trims_a_copy() {
        let root = TempRoot::new("saving_trims_a_copy");
        let path = root.paths().queue();
        let mut q = Queue::new();
        for n in 1..=24 {
            let id = q.add(spec(&format!("Job {n}")));
            q.start(id);
            q.apply(id, done());
        }
        let w = q.add(spec("Waiting"));
        save_queue(&path, &q).unwrap();
        assert_eq!(q.jobs.len(), 25);
        let (loaded, _) = load_queue(&path);
        let mut expected = vec![w];
        expected.extend((5..=24).rev());
        assert_eq!(loaded.shown(), expected);
        assert_eq!(loaded.next_id, 26);
    }

    #[test]
    fn a_missing_queue_is_empty_and_silent() {
        let root = TempRoot::new("a_missing_queue_is_empty_and_silent");
        let path = root.paths().queue();
        assert_eq!(load_queue(&path), (Queue::new(), None));
        assert!(!root.paths().config.exists());
    }

    // Review Focus 2
    #[test]
    fn a_newer_queue_is_left_alone() {
        let root = TempRoot::new("a_newer_queue_is_left_alone");
        let config = root.paths().config;
        let path = root.paths().queue();
        fs::create_dir_all(&config).unwrap();
        let newer = b"{\"format\": 2, \"app_version\": \"2.0.0\", \"queues\": [{\"name\": \"Tuesday\"}]}\n";
        fs::write(&path, newer).unwrap();
        assert_eq!(
            load_queue(&path),
            (
                Queue::new(),
                notice(
                    "The render queue was saved by a newer version of BALLFIGHT Studio, so it was left alone as \
                     queue.newer-1.json. Videos you add now go in a fresh queue."
                )
            )
        );
        assert_eq!(fs::read(config.join("queue.newer-1.json")).unwrap(), newer);
        assert!(!path.exists());
        assert_eq!(files_in(&config), vec!["queue.newer-1.json"]);
    }

    // Review Focus 2
    #[test]
    fn a_corrupt_queue_is_moved_aside() {
        let root = TempRoot::new("a_corrupt_queue_is_moved_aside");
        let config = root.paths().config;
        let path = root.paths().queue();
        fs::create_dir_all(&config).unwrap();
        fs::write(&path, b"{\"format\": 1, \"jobs\": [[[").unwrap();
        assert_eq!(
            load_queue(&path),
            (
                Queue::new(),
                notice(
                    "The render queue could not be read, so it starts empty. The old file was kept as \
                     queue.corrupt-1.json."
                )
            )
        );
        assert_eq!(
            fs::read(config.join("queue.corrupt-1.json")).unwrap(),
            b"{\"format\": 1, \"jobs\": [[["
        );
        assert!(!path.exists());
        // A second bad file takes the next number; the first is kept.
        fs::write(&path, [0xff, 0xfe, 0x00, 0x7b]).unwrap();
        let (q, second) = load_queue(&path);
        assert_eq!(q, Queue::new());
        assert!(
            second
                .unwrap()
                .text
                .ends_with("The old file was kept as queue.corrupt-2.json.")
        );
        assert_eq!(files_in(&config), vec!["queue.corrupt-1.json", "queue.corrupt-2.json"]);
    }

    // Review Focus 2
    #[test]
    fn a_half_written_queue_is_corrupt() {
        let root = TempRoot::new("a_half_written_queue_is_corrupt");
        let config = root.paths().config;
        let path = root.paths().queue();
        save_queue(&path, &mixed()).unwrap();
        let good = fs::read(&path).unwrap();
        for cut in [1, 10, good.len() / 2, good.len() - 2] {
            fs::write(&path, &good[..cut]).unwrap();
            let (q, notice) = load_queue(&path);
            assert_eq!(q, Queue::new(), "cut at {cut}");
            assert!(
                notice.unwrap().text.starts_with("The render queue could not be read"),
                "cut at {cut}"
            );
            assert!(!path.exists());
        }
        assert_eq!(fs::read(config.join("queue.corrupt-1.json")).unwrap(), &good[..1]);
        assert_eq!(
            fs::read(config.join("queue.corrupt-4.json")).unwrap(),
            &good[..good.len() - 2]
        );
    }

    #[test]
    fn a_queue_without_its_values_is_corrupt() {
        let root = TempRoot::new("a_queue_without_its_values_is_corrupt");
        let path = root.paths().queue();
        fs::create_dir_all(root.paths().config).unwrap();
        for text in [
            "",
            "[]",
            "{}",
            "{\"format\": \"1\", \"app_version\": \"0.1.0\", \"paused\": false, \"next_id\": 1, \"jobs\": []}",
            "{\"format\": 0, \"app_version\": \"0.1.0\", \"paused\": false, \"next_id\": 1, \"jobs\": []}",
            "{\"format\": 1, \"app_version\": \"0.1.0\", \"paused\": false, \"jobs\": []}",
            "{\"format\": 1, \"app_version\": \"0.1.0\", \"paused\": false, \"next_id\": 1, \"jobs\": [{\"id\": 1}]}",
        ] {
            fs::write(&path, text).unwrap();
            let (q, notice) = load_queue(&path);
            assert_eq!(q, Queue::new(), "{text}");
            assert!(
                notice.unwrap().text.starts_with("The render queue could not be read"),
                "{text}"
            );
        }
    }

    #[test]
    fn a_queue_with_one_id_twice_is_corrupt() {
        let root = TempRoot::new("a_queue_with_one_id_twice_is_corrupt");
        let path = root.paths().queue();
        let mut q = mixed();
        let copy = q.jobs[0].clone();
        q.jobs.push(copy);
        save_queue(&path, &q).unwrap();
        let (loaded, notice) = load_queue(&path);
        assert_eq!(loaded, Queue::new());
        assert!(notice.unwrap().text.starts_with("The render queue could not be read"));
    }

    #[test]
    fn ids_go_on_after_the_highest_in_the_file() {
        let root = TempRoot::new("ids_go_on_after_the_highest_in_the_file");
        let path = root.paths().queue();
        let mut q = mixed();
        q.next_id = 2;
        save_queue(&path, &q).unwrap();
        let (mut loaded, notice) = load_queue(&path);
        assert_eq!(notice, None);
        assert_eq!(loaded.next_id, 6);
        assert_eq!(loaded.add(spec("New")), 6);
    }

    /// Writes `q` as saved by app version `version`.
    fn save_as_version(path: &Path, q: &Queue, version: &str) {
        save_queue(path, q).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        value["app_version"] = serde_json::Value::from(version);
        fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }

    #[test]
    fn a_queue_from_another_version_says_so() {
        let root = TempRoot::new("a_queue_from_another_version_says_so");
        let path = root.paths().queue();
        let q = mixed();
        save_as_version(&path, &q, "0.0.9");
        assert_eq!(load_queue(&path), (q.clone(), notice(OTHER_VERSION)));
        // The file is the owner's queue: it stays where it is.
        assert_eq!(files_in(&root.paths().config), vec!["queue.json"]);
        // A job that was rendering when it was saved waits too.
        let (rendering, _, _) = one_rendering();
        save_as_version(&path, &rendering, "0.2.0");
        let (loaded, said) = load_queue(&path);
        assert_eq!(said, notice(OTHER_VERSION));
        assert_eq!(loaded.counts(), (0, 2));
    }

    #[test]
    fn another_version_with_nothing_waiting_is_silent() {
        let root = TempRoot::new("another_version_with_nothing_waiting_is_silent");
        let path = root.paths().queue();
        let mut q = mixed();
        q.jobs.retain(|job| job.state != JobState::Waiting);
        save_as_version(&path, &q, "0.0.9");
        assert_eq!(load_queue(&path), (q, None));
    }

    #[test]
    fn the_writer_saves_through_the_queue_file() {
        let root = TempRoot::new("the_writer_saves_through_the_queue_file");
        let paths = root.paths();
        let writer = Writer::start(paths.clone());
        let (q, a, _) = one_rendering();
        writer.queue(&q);
        writer.flush();
        assert!(writer.problems().is_empty());
        let (mut loaded, notice) = load_queue(&paths.queue());
        assert_eq!(notice, None);
        assert_eq!(loaded.after_restart(), 1);
        assert!(loaded.job(a).unwrap().interrupted);
    }

    #[cfg(windows)]
    fn open_shared(path: &Path, share: u32) -> fs::File {
        use std::os::windows::fs::OpenOptionsExt;
        fs::OpenOptions::new().read(true).share_mode(share).open(path).unwrap()
    }

    /// Loads `path` while another program holds it with no sharing, so it cannot be opened; returns the old bytes.
    #[cfg(windows)]
    fn load_while_locked(path: &Path) -> Vec<u8> {
        let before = fs::read(path).unwrap();
        let held = open_shared(path, 0);
        let (q, notice) = load_queue(path);
        drop(held);
        assert_eq!(q, Queue::new());
        assert!(
            notice
                .unwrap()
                .text
                .starts_with("The render queue file could not be opened")
        );
        before
    }

    #[cfg(windows)]
    #[test]
    fn a_queue_that_cannot_be_opened_is_left_alone() {
        let root = TempRoot::new("a_queue_that_cannot_be_opened_is_left_alone");
        let config = root.paths().config;
        let path = root.paths().queue();
        save_queue(&path, &mixed()).unwrap();
        let before = fs::read(&path).unwrap();
        let held = open_shared(&path, 0);
        let loaded = load_queue(&path);
        drop(held);
        assert_eq!(
            loaded,
            (
                Queue::new(),
                notice(
                    "The render queue file could not be opened, so the queue starts empty for now. It may be open in \
                     another program: close that program and start the app again to get your queue back. If you add \
                     a video first, the old file is kept as queue.unread-1.json."
                )
            )
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(files_in(&config), vec!["queue.json"]);
        // Once it can be read again, it loads as it was.
        assert_eq!(load_queue(&path), (mixed(), None));
    }

    // M2-R9 (a), for the queue
    #[cfg(windows)]
    #[test]
    fn an_unread_queue_is_kept_before_the_first_save() {
        let root = TempRoot::new("an_unread_queue_is_kept_before_the_first_save");
        let paths = root.paths();
        let path = paths.queue();
        save_queue(&path, &mixed()).unwrap();
        let before = load_while_locked(&path);
        let writer = Writer::start(paths.clone());
        let mut fresh = Queue::new();
        fresh.add(spec("Flute vs Trombone"));
        writer.queue(&fresh);
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(fs::read(paths.config.join("queue.unread-1.json")).unwrap(), before);
        assert_eq!(load_queue(&path), (fresh.clone(), None));
        // Kept once: the next save simply writes.
        writer.queue(&fresh);
        writer.flush();
        assert_eq!(files_in(&paths.config), vec!["queue.json", "queue.unread-1.json"]);
    }

    // M2-R9 (b), for the queue
    #[cfg(windows)]
    #[test]
    fn a_queue_save_is_refused_while_the_unread_file_cannot_be_kept() {
        let root = TempRoot::new("a_queue_save_is_refused_while_the_unread_file_cannot_be_kept");
        let paths = root.paths();
        let path = paths.queue();
        save_queue(&path, &mixed()).unwrap();
        let before = fs::read(&path).unwrap();
        let held = open_shared(&path, 0);
        assert!(load_queue(&path).1.is_some());
        let writer = Writer::start(paths.clone());
        writer.queue(&Queue::new());
        writer.flush();
        let problems = writer.problems();
        drop(held);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].starts_with(&format!("The render queue was not saved to {}", path.display())),
            "{}",
            problems[0]
        );
        assert!(problems[0].contains("could not be kept safe"), "{}", problems[0]);
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(files_in(&paths.config), vec!["queue.json"]);
        // Once the other program lets go, the next save keeps the old file aside and goes ahead.
        let mut fresh = Queue::new();
        fresh.add(spec("Tuba vs Bass drum"));
        writer.queue(&fresh);
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(fs::read(paths.config.join("queue.unread-1.json")).unwrap(), before);
        assert_eq!(load_queue(&path), (fresh, None));
    }

    // M2-R9 (c), for the queue
    #[test]
    fn a_queue_that_loaded_is_never_moved_by_a_save() {
        let root = TempRoot::new("a_queue_that_loaded_is_never_moved_by_a_save");
        let paths = root.paths();
        let path = paths.queue();
        save_queue(&path, &mixed()).unwrap();
        assert_eq!(load_queue(&path), (mixed(), None));
        let writer = Writer::start(paths.clone());
        writer.queue(&Queue::new());
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(files_in(&paths.config), vec!["queue.json"]);
    }

    // M2-R9 (d), for the queue
    #[cfg(windows)]
    #[test]
    fn unread_queues_are_kept_per_path() {
        let a = TempRoot::new("unread_queues_are_kept_per_path a");
        let b = TempRoot::new("unread_queues_are_kept_per_path b");
        save_queue(&a.paths().queue(), &mixed()).unwrap();
        save_queue(&b.paths().queue(), &mixed()).unwrap();
        load_while_locked(&a.paths().queue());
        assert_eq!(load_queue(&b.paths().queue()), (mixed(), None));
        let writer_b = Writer::start(b.paths());
        writer_b.queue(&Queue::new());
        writer_b.flush();
        assert_eq!(files_in(&b.paths().config), vec!["queue.json"]);
        let writer_a = Writer::start(a.paths());
        writer_a.queue(&Queue::new());
        writer_a.flush();
        assert!(writer_a.problems().is_empty());
        assert_eq!(files_in(&a.paths().config), vec!["queue.json", "queue.unread-1.json"]);
    }

    /// An unread queue file does not make the settings beside it be moved aside, nor the other way round.
    #[cfg(windows)]
    #[test]
    fn an_unread_queue_leaves_the_settings_alone() {
        use crate::store::settings::{default_settings, save_settings};
        let root = TempRoot::new("an_unread_queue_leaves_the_settings_alone");
        let paths = root.paths();
        save_queue(&paths.queue(), &mixed()).unwrap();
        save_settings(&paths.settings(), &default_settings(4)).unwrap();
        load_while_locked(&paths.queue());
        let writer = Writer::start(paths.clone());
        writer.settings(&default_settings(8));
        writer.flush();
        assert!(writer.problems().is_empty());
        assert_eq!(files_in(&paths.config), vec!["queue.json", "settings.toml"]);
        writer.queue(&Queue::new());
        writer.flush();
        assert_eq!(
            files_in(&paths.config),
            vec!["queue.json", "queue.unread-1.json", "settings.toml"]
        );
    }
}
