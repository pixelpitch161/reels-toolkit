//! The queue, folder, library and settings tests: they drive `dispatch` and `apply` only.

use std::path::PathBuf;

use super::why_not_add;
use crate::model::library::FolderState;
use crate::model::queue::{FailHint, JobSpec, JobState, JobUpdate, Queue};
use crate::model::{JobId, Notice, Settings, Step, TakeId, View};
use crate::seam::{Command, Event, sample};
use crate::state::{Action, AppState, Effect, QueueAction};

fn root() -> PathBuf {
    PathBuf::from("Videos").join("BALLFIGHT")
}

fn settings() -> Settings {
    Settings {
        output_folder: root(),
        workers: 7,
    }
}

/// A new app that has its roster: FLUTE vs TROMBONE.
fn ready() -> AppState {
    let (mut s, _) = AppState::new(settings(), Queue::new(), Vec::new(), 8);
    s.apply(Event::Roster(sample::roster()));
    s
}

/// The commands among `effects`, in order.
fn sent(effects: &[Effect]) -> Vec<Command> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::Send(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// The queues among `effects` that are to be saved.
fn queue_saves(effects: &[Effect]) -> Vec<Queue> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::SaveQueue(q) => Some(q.clone()),
            _ => None,
        })
        .collect()
}

/// The settings among `effects` that are to be saved.
fn settings_saves(effects: &[Effect]) -> Vec<Settings> {
    effects
        .iter()
        .filter_map(|e| match e {
            Effect::SaveSettings(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// The take number the last `LoadTake` among `effects` asked for.
fn take_of(effects: &[Effect]) -> TakeId {
    sent(effects)
        .iter()
        .rev()
        .find_map(|c| match c {
            Command::LoadTake { take, .. } => Some(*take),
            _ => None,
        })
        .expect("a LoadTake")
}

fn folder(name: &str, state: FolderState) -> Event {
    Event::Folder {
        root: root(),
        name: name.to_string(),
        state,
    }
}

fn picture(take: TakeId, seed: u64) -> Event {
    Event::TakePicture {
        take,
        story: sample::story(seed),
        frames: sample::frames([[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]]),
    }
}

fn missing_text() -> String {
    format!(
        "The output folder {} is missing. Create it, or choose another folder.",
        root().display()
    )
}

/// Walks a fresh app through the nine reasons, in the brief's order, calling `at` with the state and the reason
/// expected at each stage (every earlier cause absent), and last with `None` once nothing is in the way.
fn each_stage(mut at: impl FnMut(&mut AppState, Option<String>)) {
    // 1. A fight problem, while every later cause holds too.
    let mut s = ready();
    let generation = s.generation();
    s.apply(Event::FightInfo {
        r#gen: generation,
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: None,
        problem: Some("These two cannot fight yet.".to_string()),
    });
    at(&mut s, Some("These two cannot fight yet.".to_string()));

    // 2. No seed.
    let mut s = ready();
    at(&mut s, Some("Pick a seed first.".to_string()));

    // 3. A take problem.
    let fx = s.dispatch(Action::SelectSeed(20418));
    s.apply(Event::TakeFailed {
        take: take_of(&fx),
        problem: "Seed 20418 could not be drawn.".to_string(),
    });
    at(&mut s, Some("Seed 20418 could not be drawn.".to_string()));

    // 4. The take's story not there yet.
    let fx = s.dispatch(Action::SelectSeed(20418));
    let take = take_of(&fx);
    at(&mut s, Some("Seed 20418 is still being drawn.".to_string()));

    // 5. The folder not checked yet.
    s.apply(picture(take, 20418));
    at(&mut s, Some("Checking the output folder.".to_string()));

    // 6. Missing.
    s.apply(folder("flute_vs_trombone", FolderState::Missing));
    at(&mut s, Some(missing_text()));

    // 7. Not writable.
    s.apply(folder(
        "flute_vs_trombone",
        FolderState::NotWritable("Access is denied.".to_string()),
    ));
    at(
        &mut s,
        Some(format!(
            "The output folder {} cannot be written to. Choose another folder.",
            root().display()
        )),
    );

    // 8. ffmpeg not answered yet.
    s.apply(folder("flute_vs_trombone", FolderState::Ok { taken: false }));
    at(&mut s, Some("Looking for ffmpeg.".to_string()));

    // 9. ffmpeg not found.
    s.apply(Event::Tools {
        ffmpeg: Err("Not on PATH.".to_string()),
    });
    at(
        &mut s,
        Some(
            "ffmpeg was not found on this PC, so videos cannot be saved yet. Install ffmpeg, then start the app again."
                .to_string(),
        ),
    );

    // Nothing in the way.
    s.apply(Event::Tools {
        ffmpeg: Ok("ffmpeg 9.0.2".to_string()),
    });
    at(&mut s, None);
}

/// An app where nothing is in the way of adding: seed 20418 drawn, the folder fine, ffmpeg found.
fn all_well() -> AppState {
    let mut s = ready();
    let fx = s.dispatch(Action::SelectSeed(20418));
    s.apply(picture(take_of(&fx), 20418));
    s.apply(folder("flute_vs_trombone", FolderState::Ok { taken: false }));
    s.apply(Event::Tools {
        ffmpeg: Ok("ffmpeg 9.0.2".to_string()),
    });
    assert_eq!(why_not_add(&s.vm), None);
    s
}

fn spec(title: &str) -> JobSpec {
    JobSpec {
        title: title.to_string(),
        colors: [[0x68, 0x5c, 0xec], [0x80, 0x80, 0x8c]],
        theme: "Big top".to_string(),
        seed: 9120,
        fight_toml: format!("# {title}\nseed = 9120\n"),
        out_root: root(),
    }
}

fn enqueued(title: &str) -> Event {
    Event::Enqueued { spec: spec(title) }
}

fn job(id: JobId, update: JobUpdate) -> Event {
    Event::Job { id, update }
}

fn done() -> JobUpdate {
    JobUpdate::Done {
        folder: root().join("tuba_vs_bass_drum"),
        seconds: 48.0,
    }
}

fn failed() -> JobUpdate {
    JobUpdate::Failed {
        headline: "Stopped at the loudness check".to_string(),
        detail: "Nothing was saved.".to_string(),
        hint: FailHint::OpenInMusic,
    }
}

fn frames(done: u32, total: u32, seconds: f32) -> JobUpdate {
    JobUpdate::Frames { done, total, seconds }
}

fn is_run(c: &Command, want: JobId) -> bool {
    matches!(c, Command::RunJob { id, fight_toml, out_root }
        if *id == want && fight_toml.contains("seed = 9120") && out_root == &root())
}

/// An app with two queued videos: job 1 rendering, job 2 waiting.
fn two_jobs() -> AppState {
    let mut s = ready();
    s.apply(enqueued("Tuba vs Bass drum"));
    s.apply(enqueued("Flute vs Trombone"));
    assert_eq!(s.vm.queue.rendering(), Some(1));
    s
}

fn state_of(s: &AppState, id: JobId) -> JobState {
    s.vm.queue.job(id).unwrap().state.clone()
}

fn library(s: &mut AppState) -> Vec<crate::model::library::LibraryEntry> {
    let entries: Vec<_> = sample::library().into_iter().map(|(e, _)| e).collect();
    s.apply(Event::Library {
        root: root(),
        entries: entries.clone(),
        problem: None,
    });
    entries
}

#[test]
fn why_not_add_has_nine_reasons_in_order() {
    let mut stages = 0;
    each_stage(|s, expected| {
        stages += 1;
        assert_eq!(why_not_add(&s.vm), expected, "stage {stages}");
    });
    assert_eq!(stages, 10);
}

#[test]
fn adding_sends_exactly_when_nothing_is_in_the_way() {
    each_stage(|s, expected| {
        let before = s.vm.clone();
        let fx = s.dispatch(Action::AddToQueue);
        match expected {
            Some(reason) => {
                assert!(fx.is_empty(), "{reason}: {fx:?}");
                assert_eq!(s.vm, before, "{reason}");
            }
            None => {
                assert_eq!(fx.len(), 1, "{fx:?}");
                assert!(matches!(&sent(&fx)[..], [Command::Enqueue { draft, out_root }]
                    if draft == &s.vm.draft && draft.seed == Some(20418) && out_root == &root()));
            }
        }
    });
}

#[test]
fn a_folder_answer_for_another_pair_is_ignored() {
    let mut s = ready();
    s.apply(folder("flute_vs_trombone", FolderState::Ok { taken: false }));
    assert_eq!(s.vm.folder, FolderState::Ok { taken: false });

    // A new pair: its folder is checked, and until the answer comes nothing is known about it.
    let fx = s.dispatch(Action::PickFighter {
        side: 1,
        name: "PIANO".to_string(),
    });
    assert!(sent(&fx).contains(&Command::CheckFolder {
        root: root(),
        name: "flute_vs_piano".to_string(),
    }));
    assert_eq!(s.vm.folder, FolderState::Unknown);

    // The answer about the pair before, or about another folder, changes nothing.
    assert!(
        s.apply(folder("flute_vs_trombone", FolderState::Ok { taken: true }))
            .is_empty()
    );
    assert_eq!(s.vm.folder, FolderState::Unknown);
    s.apply(Event::Folder {
        root: PathBuf::from("Elsewhere"),
        name: "flute_vs_piano".to_string(),
        state: FolderState::Missing,
    });
    assert_eq!(s.vm.folder, FolderState::Unknown);

    // The answer about this pair is taken.
    s.apply(folder("flute_vs_piano", FolderState::Ok { taken: true }));
    assert_eq!(s.vm.folder, FolderState::Ok { taken: true });

    // Swapping corners is another folder name; choosing a seed checks again and keeps what is known.
    let fx = s.dispatch(Action::SwapSides);
    assert!(sent(&fx).contains(&Command::CheckFolder {
        root: root(),
        name: "piano_vs_flute".to_string(),
    }));
    s.apply(folder("piano_vs_flute", FolderState::Ok { taken: false }));
    let fx = s.dispatch(Action::SelectSeed(7731));
    assert!(sent(&fx).contains(&Command::CheckFolder {
        root: root(),
        name: "piano_vs_flute".to_string(),
    }));
    assert_eq!(s.vm.folder, FolderState::Ok { taken: false });
    // An answer with no name (as at start, or after creating the folder) is about the folder itself.
    s.apply(folder("", FolderState::Missing));
    assert_eq!(s.vm.folder, FolderState::Missing);
}

#[test]
fn an_enqueued_job_starts_when_idle() {
    let mut s = all_well();
    let fx = s.apply(enqueued("Tuba vs Bass drum"));
    assert_eq!(fx.len(), 2, "{fx:?}");
    let saved = queue_saves(&fx);
    assert!(matches!(&fx[0], Effect::SaveQueue(_)));
    assert!(is_run(&sent(&fx)[0], 1));
    // The saved queue holds the job as rendering, so a restart knows it was interrupted.
    assert_eq!(saved[0], s.vm.queue);
    assert_eq!(s.vm.queue.rendering(), Some(1));
}

#[test]
fn a_second_job_waits() {
    let mut s = all_well();
    s.apply(enqueued("Tuba vs Bass drum"));
    let fx = s.apply(enqueued("Flute vs Trombone"));
    assert_eq!(fx.len(), 1, "{fx:?}");
    assert_eq!(queue_saves(&fx), vec![s.vm.queue.clone()]);
    assert_eq!(state_of(&s, 2), JobState::Waiting);
    assert_eq!(s.vm.queue.rendering(), Some(1));
}

#[test]
fn the_queue_goes_on_after_done_failed_and_cancelled_done() {
    let mut s = two_jobs();
    let fx = s.apply(job(1, done()));
    assert_eq!(queue_saves(&fx).len(), 1);
    let commands = sent(&fx);
    assert_eq!(commands.len(), 2, "{commands:?}");
    assert!(is_run(&commands[0], 2));
    assert_eq!(commands[1], Command::ScanLibrary { root: root() });
    assert!(matches!(state_of(&s, 1), JobState::Done { .. }));
    assert_eq!(s.vm.queue.rendering(), Some(2));
}

#[test]
fn the_queue_goes_on_after_done_failed_and_cancelled_failed() {
    let mut s = two_jobs();
    let fx = s.apply(job(1, failed()));
    assert_eq!(queue_saves(&fx).len(), 1);
    let commands = sent(&fx);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert!(is_run(&commands[0], 2));
    assert!(matches!(state_of(&s, 1), JobState::Failed { .. }));
}

#[test]
fn the_queue_goes_on_after_done_failed_and_cancelled_cancelled() {
    let mut s = two_jobs();
    let fx = s.apply(job(1, JobUpdate::Cancelled));
    assert_eq!(queue_saves(&fx).len(), 1);
    let commands = sent(&fx);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert!(is_run(&commands[0], 2));
    assert_eq!(state_of(&s, 1), JobState::Cancelled);
}

#[test]
fn the_status_counts_down() {
    let mut s = ready();
    s.apply(enqueued("Tuba vs Bass drum"));
    assert_eq!(s.vm.status, "Rendering Tuba vs Bass drum");
    let fx = s.apply(job(1, frames(610, 1830, 10.0)));
    assert!(fx.is_empty(), "a frame count is not saved: {fx:?}");
    assert_eq!(s.vm.status, "Rendering Tuba vs Bass drum, about 20 s left");
    // Numbers that cannot give a time left (widget rule 9) give the plain line.
    for (d, t, secs) in [
        (0, 1830, 0.0),
        (0, 1830, 5.0),
        (610, 1830, f32::NAN),
        (610, 1830, f32::INFINITY),
        (610, 1830, -3.0),
        (1900, 1830, 10.0),
    ] {
        s.apply(job(1, frames(d, t, secs)));
        assert_eq!(s.vm.status, "Rendering Tuba vs Bass drum", "{d} {t} {secs}");
    }
    // The last frame: nothing left.
    s.apply(job(1, frames(1830, 1830, 60.0)));
    assert_eq!(s.vm.status, "Rendering Tuba vs Bass drum, about 0 s left");
    // When it is done the status goes back to the fight.
    s.apply(job(1, done()));
    assert_eq!(s.vm.status, "Flute vs Trombone, not balanced yet");
}

#[test]
fn the_rendering_line_outranks_the_search_but_not_a_problem() {
    let mut s = ready();
    s.dispatch(Action::Go(View::Step(Step::Seeds)));
    assert_eq!(s.vm.status, "Searching on 7 cores");
    s.apply(enqueued("Tuba vs Bass drum"));
    assert_eq!(s.vm.status, "Rendering Tuba vs Bass drum");
    let generation = s.generation();
    s.apply(Event::FightInfo {
        r#gen: generation,
        pair_rules: Vec::new(),
        captions: sample::captions(),
        title_card: None,
        problem: Some("These two cannot fight yet.".to_string()),
    });
    assert_eq!(s.vm.status, "These two cannot fight yet.");
}

#[test]
fn pausing_lets_the_current_job_finish_and_holds_the_next() {
    let mut s = two_jobs();
    let fx = s.dispatch(Action::Queue(QueueAction::Pause));
    assert!(s.vm.queue.paused);
    assert!(sent(&fx).is_empty(), "the job being rendered is not stopped: {fx:?}");
    assert_eq!(queue_saves(&fx), vec![s.vm.queue.clone()]);
    let fx = s.apply(job(1, done()));
    assert!(matches!(state_of(&s, 1), JobState::Done { .. }));
    assert_eq!(state_of(&s, 2), JobState::Waiting);
    assert_eq!(sent(&fx), vec![Command::ScanLibrary { root: root() }]);
    assert_eq!(queue_saves(&fx).len(), 1);
}

#[test]
fn resuming_starts_it() {
    let mut s = two_jobs();
    s.dispatch(Action::Queue(QueueAction::Pause));
    s.apply(job(1, done()));
    let fx = s.dispatch(Action::Queue(QueueAction::Resume));
    assert!(!s.vm.queue.paused);
    let commands = sent(&fx);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert!(is_run(&commands[0], 2));
    assert_eq!(queue_saves(&fx), vec![s.vm.queue.clone()]);
    assert_eq!(s.vm.queue.rendering(), Some(2));
}

#[test]
fn cancel_reaches_only_the_rendering_job() {
    let mut s = two_jobs();
    assert!(s.dispatch(Action::Queue(QueueAction::Cancel(2))).is_empty());
    assert!(s.dispatch(Action::Queue(QueueAction::Cancel(77))).is_empty());
    let fx = s.dispatch(Action::Queue(QueueAction::Cancel(1)));
    assert_eq!(sent(&fx), vec![Command::CancelJob { id: 1 }]);
    assert_eq!(fx.len(), 1);
    // The job ends when the engine says so.
    assert_eq!(s.vm.queue.rendering(), Some(1));
}

#[test]
fn a_restart_explains_itself() {
    // A job was rendering when the app closed, and another waits.
    let mut q = Queue::new();
    q.add(spec("Tuba vs Bass drum"));
    q.add(spec("Flute vs Trombone"));
    q.start(1);
    let earlier = Notice {
        text: "The queue file was from another version.".to_string(),
    };
    let (s, _) = AppState::new(settings(), q, vec![earlier.clone()], 8);
    assert_eq!(
        s.vm.notices,
        vec![
            earlier,
            Notice {
                text: "1 video was interrupted when the app closed. It is back in the queue. Resume the queue to \
                       render it."
                    .to_string(),
            },
        ]
    );
    assert!(s.vm.queue.paused);
    assert_eq!(s.vm.queue.rendering(), None);
    assert!(s.vm.queue.job(1).unwrap().interrupted);

    // Two were rendering (a hand-edited file).
    let mut q = Queue::new();
    q.add(spec("A"));
    q.add(spec("B"));
    for job in &mut q.jobs {
        job.state = JobState::Rendering { done: 10, total: 1830 };
    }
    let (s, _) = AppState::new(settings(), q, Vec::new(), 8);
    assert_eq!(
        s.vm.notices,
        vec![Notice {
            text: "2 videos were interrupted when the app closed. They are back in the queue. Resume the queue to \
                   render them."
                .to_string(),
        }]
    );

    // Jobs that only wait.
    let mut q = Queue::new();
    q.add(spec("A"));
    q.add(spec("B"));
    let (s, _) = AppState::new(settings(), q, Vec::new(), 8);
    assert_eq!(
        s.vm.notices,
        vec![Notice {
            text: "2 videos are waiting in the queue. Resume the queue to render them.".to_string(),
        }]
    );
    assert!(s.vm.queue.paused);
    let mut q = Queue::new();
    q.add(spec("A"));
    let (s, _) = AppState::new(settings(), q, Vec::new(), 8);
    assert_eq!(
        s.vm.notices,
        vec![Notice {
            text: "1 video is waiting in the queue. Resume the queue to render it.".to_string(),
        }]
    );

    // Nothing waiting: nothing to say.
    let (s, _) = AppState::new(settings(), Queue::new(), Vec::new(), 8);
    assert!(s.vm.notices.is_empty());
    assert!(!s.vm.queue.paused);
}

#[test]
fn a_missing_folder_blocks_adding_and_can_be_created() {
    let mut s = all_well();
    s.apply(folder("flute_vs_trombone", FolderState::Missing));
    assert_eq!(why_not_add(&s.vm), Some(missing_text()));
    assert!(s.dispatch(Action::AddToQueue).is_empty());
    let fx = s.dispatch(Action::CreateFolder);
    assert_eq!(sent(&fx), vec![Command::CreateFolder { root: root() }]);
    // The engine answers about the folder it made.
    s.apply(folder("", FolderState::Ok { taken: false }));
    assert_eq!(why_not_add(&s.vm), None);
    assert_eq!(sent(&s.dispatch(Action::AddToQueue)).len(), 1);
}

#[test]
fn a_picked_folder_is_saved_checked_and_scanned() {
    let mut s = all_well();
    let fx = s.dispatch(Action::ChangeFolder);
    assert_eq!(sent(&fx), vec![Command::PickFolder { start: root() }]);
    assert_eq!(fx.len(), 1);
    // The dialog was closed without a choice.
    assert!(s.apply(Event::FolderPicked { path: None }).is_empty());
    assert_eq!(s.vm.settings.output_folder, root());

    let picked = PathBuf::from("D:").join("Clips");
    let fx = s.apply(Event::FolderPicked {
        path: Some(picked.clone()),
    });
    assert_eq!(s.vm.settings.output_folder, picked);
    assert_eq!(settings_saves(&fx), vec![s.vm.settings.clone()]);
    assert_eq!(
        sent(&fx),
        vec![
            Command::CheckFolder {
                root: picked.clone(),
                name: "flute_vs_trombone".to_string(),
            },
            Command::ScanLibrary { root: picked.clone() },
        ]
    );
    assert_eq!(s.vm.folder, FolderState::Unknown);
    assert!(s.vm.library.scanning);
    // An answer about the old folder is late.
    s.apply(folder("flute_vs_trombone", FolderState::Ok { taken: false }));
    assert_eq!(s.vm.folder, FolderState::Unknown);
    // Videos are added to the new folder.
    let fx = {
        s.apply(Event::Folder {
            root: picked.clone(),
            name: "flute_vs_trombone".to_string(),
            state: FolderState::Ok { taken: false },
        });
        s.dispatch(Action::AddToQueue)
    };
    assert!(matches!(&sent(&fx)[..], [Command::Enqueue { out_root, .. }] if out_root == &picked));
}

#[test]
fn reopening_loads_the_fight() {
    let mut s = ready();
    s.dispatch(Action::Go(View::Library));
    let entries = library(&mut s);
    let fx = s.dispatch(Action::Reopen(entries[0].folder.clone()));
    assert_eq!(
        sent(&fx),
        vec![Command::OpenFight {
            path: entries[0].fight_path.clone().unwrap(),
        }]
    );

    let before = s.generation();
    let mut draft = s.vm.draft.clone();
    draft.seed = Some(20418);
    let fx = s.apply(Event::FightOpened {
        draft: draft.clone(),
        title: "Flute vs Trombone, seed 20418".to_string(),
    });
    assert_eq!(s.generation(), before + 1);
    assert_eq!(s.vm.view, View::Step(Step::Matchup));
    assert_eq!(s.vm.step, Step::Matchup);
    assert_eq!(s.vm.draft.seed, Some(20418));
    assert_eq!(s.vm.status, "Opened Flute vs Trombone, seed 20418");
    let commands = sent(&fx);
    let describe = commands
        .iter()
        .filter(|c| matches!(c, Command::DescribeFight { r#gen, .. } if *r#gen == s.generation()))
        .count();
    let take = commands
        .iter()
        .filter(|c| matches!(c, Command::LoadTake { draft, .. } if draft.seed == Some(20418)))
        .count();
    assert_eq!((describe, take), (1, 1), "{commands:?}");

    // The take arriving keeps the line; the next thing the owner changes replaces it.
    s.apply(picture(take_of(&fx), 20418));
    assert_eq!(s.vm.status, "Opened Flute vs Trombone, seed 20418");
    s.dispatch(Action::Go(View::Step(Step::Music)));
    assert_eq!(s.vm.status, "Seed 20418 is ready");

    // A fight that opens without being asked for (or after the owner moved on) is late.
    let generation = s.generation();
    assert!(
        s.apply(Event::FightOpened {
            draft,
            title: "Flute vs Trombone, seed 20418".to_string(),
        })
        .is_empty()
    );
    assert_eq!(s.generation(), generation);
}

#[test]
fn an_opened_fight_is_late_once_the_owner_has_moved_on() {
    let mut s = ready();
    s.dispatch(Action::Go(View::Library));
    let entries = library(&mut s);
    s.dispatch(Action::Reopen(entries[1].folder.clone()));
    s.dispatch(Action::PickFighter {
        side: 1,
        name: "PIANO".to_string(),
    });
    let generation = s.generation();
    let mut draft = s.vm.draft.clone();
    draft.fighters = ["TUBA".to_string(), "BASS DRUM".to_string()];
    assert!(
        s.apply(Event::FightOpened {
            draft,
            title: "Tuba vs Bass drum, seed 9120".to_string(),
        })
        .is_empty()
    );
    assert_eq!(s.generation(), generation);
    assert_eq!(s.vm.draft.fighters[1], "PIANO");
}

#[test]
fn open_in_music_lands_on_music() {
    let mut s = two_jobs();
    s.apply(job(1, failed()));
    let fx = s.dispatch(Action::Queue(QueueAction::OpenIn {
        id: 1,
        step: Step::Music,
    }));
    assert_eq!(
        sent(&fx),
        vec![Command::OpenFightText {
            toml: spec("Tuba vs Bass drum").fight_toml,
        }]
    );
    let mut draft = s.vm.draft.clone();
    draft.seed = Some(9120);
    s.apply(Event::FightOpened {
        draft,
        title: "Tuba vs Bass drum, seed 9120".to_string(),
    });
    assert_eq!(s.vm.view, View::Step(Step::Music));
    assert_eq!(s.vm.step, Step::Music);
    // The job rendering outranks the opened line.
    assert_eq!(s.vm.status, "Rendering Flute vs Trombone");
    // An unknown job opens nothing.
    assert!(
        s.dispatch(Action::Queue(QueueAction::OpenIn {
            id: 77,
            step: Step::Seeds,
        }))
        .is_empty()
    );
}

#[test]
fn removing_moving_and_requeueing_are_saved() {
    let mut s = two_jobs();
    s.apply(enqueued("Oboe vs Harp"));
    // The rendering job cannot be removed; a waiting one can.
    assert!(s.dispatch(Action::Queue(QueueAction::Remove(1))).is_empty());
    s.dispatch(Action::Queue(QueueAction::Select(3)));
    assert_eq!(s.vm.selected_job, Some(3));
    let fx = s.dispatch(Action::Queue(QueueAction::Move { id: 3, to: 0 }));
    assert_eq!(queue_saves(&fx), vec![s.vm.queue.clone()]);
    assert_eq!(s.vm.queue.shown(), vec![1, 3, 2]);
    let fx = s.dispatch(Action::Queue(QueueAction::Remove(3)));
    assert_eq!(queue_saves(&fx), vec![s.vm.queue.clone()]);
    assert_eq!(s.vm.queue.job(3), None);
    assert_eq!(s.vm.selected_job, None);
    // Selecting a job the queue does not hold changes nothing.
    s.dispatch(Action::Queue(QueueAction::Select(77)));
    assert_eq!(s.vm.selected_job, None);

    // A failed job queued again starts when the queue is free.
    s.dispatch(Action::Queue(QueueAction::Pause));
    s.apply(job(1, failed()));
    let fx = s.dispatch(Action::Queue(QueueAction::Requeue(1)));
    assert_eq!(queue_saves(&fx), vec![s.vm.queue.clone()]);
    assert!(sent(&fx).is_empty(), "paused: {fx:?}");
    assert_eq!(s.vm.queue.shown()[..2], [2, 1]);
    assert!(s.dispatch(Action::Queue(QueueAction::Requeue(2))).is_empty());
}

#[test]
fn a_done_job_opens_its_folder() {
    let mut s = two_jobs();
    s.apply(job(1, done()));
    let fx = s.dispatch(Action::Queue(QueueAction::OpenFolder(1)));
    assert_eq!(
        sent(&fx),
        vec![Command::RevealFolder {
            path: root().join("tuba_vs_bass_drum"),
        }]
    );
    assert!(s.dispatch(Action::Queue(QueueAction::OpenFolder(77))).is_empty());
}

#[test]
fn a_late_job_update_changes_nothing() {
    let mut s = two_jobs();
    let before = s.vm.clone();
    assert!(s.apply(job(77, done())).is_empty());
    assert!(s.apply(job(2, done())).is_empty(), "job 2 is waiting");
    assert_eq!(s.vm, before);
    s.apply(job(1, JobUpdate::Cancelled));
    let before = s.vm.clone();
    assert!(s.apply(job(1, frames(12, 1830, 0.4))).is_empty());
    assert!(s.apply(job(1, done())).is_empty());
    assert_eq!(s.vm, before);
}

#[test]
fn the_library_is_scanned_and_late_answers_change_nothing() {
    let mut s = ready();
    let fx = s.dispatch(Action::Go(View::Library));
    assert_eq!(sent(&fx), vec![Command::ScanLibrary { root: root() }]);
    assert!(s.vm.library.scanning);
    let fx = s.dispatch(Action::RefreshLibrary);
    assert_eq!(sent(&fx), vec![Command::ScanLibrary { root: root() }]);

    // An answer about another folder is late.
    let entries: Vec<_> = sample::library().into_iter().map(|(e, _)| e).collect();
    s.apply(Event::Library {
        root: PathBuf::from("Elsewhere"),
        entries: entries.clone(),
        problem: None,
    });
    assert!(s.vm.library.entries.is_empty());
    assert!(s.vm.library.scanning);

    s.apply(Event::Library {
        root: root(),
        entries: entries.clone(),
        problem: Some("One folder could not be read.".to_string()),
    });
    assert_eq!(s.vm.library.entries, entries);
    assert_eq!(s.vm.library.problem.as_deref(), Some("One folder could not be read."));
    assert!(!s.vm.library.scanning);

    let (entry, cover) = sample::library().remove(0);
    s.apply(Event::LibraryCover {
        folder: entry.folder.clone(),
        cover: cover.clone(),
    });
    assert_eq!(s.vm.library.covers.get(&entry.folder), Some(&cover));
    // A cover for a folder that is not in the library is late.
    s.apply(Event::LibraryCover {
        folder: PathBuf::from("Elsewhere").join("flute_vs_trombone"),
        cover,
    });
    assert_eq!(s.vm.library.covers.len(), 1);

    s.dispatch(Action::SelectVideo(entry.folder.clone()));
    assert_eq!(s.vm.library.selected, Some(entry.folder.clone()));
    s.dispatch(Action::SelectVideo(PathBuf::from("Elsewhere")));
    assert_eq!(s.vm.library.selected, Some(entry.folder));
}

#[test]
fn library_paths_outside_the_entries_lead_nowhere() {
    let mut s = ready();
    let entries = library(&mut s);
    let outside = PathBuf::from("C:").join("Windows");
    assert!(s.dispatch(Action::Reopen(outside.clone())).is_empty());
    assert!(s.dispatch(Action::OpenVideoFolder(outside)).is_empty());
    // A video made by a newer version cannot be reopened, but its folder can be shown.
    let newer = entries.iter().find(|e| e.problem.is_some()).unwrap();
    assert!(s.dispatch(Action::Reopen(newer.folder.clone())).is_empty());
    let fx = s.dispatch(Action::OpenVideoFolder(newer.folder.clone()));
    assert_eq!(
        sent(&fx),
        vec![Command::RevealFolder {
            path: newer.folder.clone(),
        }]
    );
}

#[test]
fn workers_are_saved_and_sent() {
    let mut s = ready();
    for (asked, kept) in [(4, 4), (0, 1), (99, 8)] {
        let fx = s.dispatch(Action::SetWorkers(asked));
        assert_eq!(s.vm.settings.workers, kept);
        assert_eq!(settings_saves(&fx), vec![s.vm.settings.clone()]);
        assert_eq!(sent(&fx), vec![Command::SetWorkers(kept)]);
    }
}

#[test]
fn the_tools_answer_is_kept() {
    let mut s = ready();
    assert_eq!(s.vm.ffmpeg, None);
    s.apply(Event::Tools {
        ffmpeg: Ok("ffmpeg 9.0.2".to_string()),
    });
    assert_eq!(s.vm.ffmpeg, Some(Ok("ffmpeg 9.0.2".to_string())));
}

#[test]
fn a_problem_becomes_a_notice_and_can_be_dismissed() {
    let mut s = ready();
    s.apply(Event::Problem {
        text: "The fight file could not be read.".to_string(),
    });
    s.apply(Event::EnqueueFailed {
        problem: "The fight file could not be made.".to_string(),
    });
    let texts = |s: &AppState| s.vm.notices.iter().map(|n| n.text.clone()).collect::<Vec<_>>();
    assert_eq!(
        texts(&s),
        vec!["The fight file could not be read.", "The fight file could not be made."]
    );
    s.dispatch(Action::Dismiss(5));
    assert_eq!(s.vm.notices.len(), 2);
    s.dispatch(Action::Dismiss(0));
    assert_eq!(texts(&s), vec!["The fight file could not be made."]);
}

// Fix round 1 (ruling M2-R20): only the person can cancel an open.

fn opened(s: &AppState, fighters: [&str; 2], seed: u64) -> Event {
    let mut draft = s.vm.draft.clone();
    draft.fighters = fighters.map(str::to_string);
    draft.handicap = [1.0, 1.0];
    draft.seed = Some(seed);
    Event::FightOpened {
        draft,
        title: format!("{} vs {}, seed {seed}", fighters[0], fighters[1]),
    }
}

#[test]
fn a_background_result_does_not_cancel_an_open() {
    let mut s = ready();
    // A balance run is going when the owner reopens a video.
    s.dispatch(Action::Go(View::Step(Step::Balance)));
    assert!(s.vm.balance.running.is_some());
    s.dispatch(Action::Go(View::Library));
    let entries = library(&mut s);
    s.dispatch(Action::Reopen(entries[1].folder.clone()));
    let generation = s.generation();
    s.apply(Event::Balanced {
        r#gen: generation,
        handicap: [1.3, 1.0],
        stats: sample::stats(),
    });
    assert_eq!(s.generation(), generation + 1, "the balance began a generation");
    let fx = s.apply(opened(&s, ["TUBA", "BASS DRUM"], 9120));
    assert_eq!(s.vm.draft.fighters, ["TUBA".to_string(), "BASS DRUM".to_string()]);
    assert_eq!(
        s.vm.draft.handicap,
        [1.0, 1.0],
        "the opened fight replaces what the balance left"
    );
    assert_eq!(s.vm.draft.seed, Some(9120));
    assert_eq!(s.vm.view, View::Step(Step::Matchup));
    assert!(sent(&fx).iter().any(|c| matches!(c, Command::LoadTake { .. })));

    // A second roster arriving in between does not cancel one either.
    s.dispatch(Action::Go(View::Library));
    let entries = library(&mut s);
    s.dispatch(Action::Reopen(entries[0].folder.clone()));
    s.apply(Event::Roster(sample::roster()));
    s.apply(opened(&s, ["FLUTE", "TROMBONE"], 18433));
    assert_eq!(s.vm.draft.seed, Some(18433));
    assert_eq!(s.vm.status, "Opened FLUTE vs TROMBONE, seed 18433");
}

#[test]
fn two_opens_in_a_row_land_on_the_second_ones_step() {
    let mut s = two_jobs();
    s.apply(job(1, failed()));
    s.dispatch(Action::Queue(QueueAction::OpenIn {
        id: 1,
        step: Step::Music,
    }));
    s.dispatch(Action::Queue(QueueAction::OpenIn {
        id: 1,
        step: Step::Seeds,
    }));
    s.apply(opened(&s, ["TUBA", "BASS DRUM"], 9120));
    assert_eq!(s.vm.view, View::Step(Step::Seeds));
    // One open was waiting, so a second answer is late.
    let generation = s.generation();
    assert!(s.apply(opened(&s, ["TUBA", "BASS DRUM"], 9120)).is_empty());
    assert_eq!(s.generation(), generation);
}

#[test]
fn an_unrelated_problem_does_not_lose_an_open() {
    let mut s = ready();
    let entries = library(&mut s);
    s.dispatch(Action::Reopen(entries[1].folder.clone()));
    s.apply(Event::Problem {
        text: "The shortlist cache could not be read.".to_string(),
    });
    let generation = s.generation();
    s.apply(opened(&s, ["TUBA", "BASS DRUM"], 9120));
    assert_eq!(s.generation(), generation + 1);
    assert_eq!(s.vm.draft.fighters, ["TUBA".to_string(), "BASS DRUM".to_string()]);
    assert_eq!(s.vm.notices.len(), 1, "the problem is still told");
}

#[test]
fn on_seeds_an_open_into_music_starts_no_search() {
    let mut s = two_jobs();
    s.apply(job(1, failed()));
    s.dispatch(Action::Go(View::Step(Step::Seeds)));
    s.dispatch(Action::Queue(QueueAction::OpenIn {
        id: 1,
        step: Step::Music,
    }));
    let fx = s.apply(opened(&s, ["TUBA", "BASS DRUM"], 9120));
    assert_eq!(s.vm.view, View::Step(Step::Music));
    assert!(
        !sent(&fx).iter().any(|c| matches!(c, Command::StartSearch { .. })),
        "{fx:?}"
    );
}

#[test]
fn an_opened_fight_lands_through_its_step() {
    let starts = |fx: &[Effect], generation| {
        sent(fx)
            .iter()
            .filter(|c| matches!(c, Command::StartSearch { r#gen, .. } if *r#gen == generation))
            .count()
    };
    let mut s = two_jobs();
    s.apply(job(1, failed()));
    for step in [Step::Seeds, Step::Music, Step::Balance] {
        s.dispatch(Action::Go(View::Step(Step::Matchup)));
        s.dispatch(Action::Queue(QueueAction::OpenIn { id: 1, step }));
        let fx = s.apply(opened(&s, ["TUBA", "BASS DRUM"], 9120));
        assert_eq!(s.vm.view, View::Step(step));
        assert_eq!(s.vm.step, step);
        let balances = sent(&fx)
            .iter()
            .filter(|c| matches!(c, Command::AutoBalance { .. }))
            .count();
        match step {
            Step::Seeds => assert_eq!((starts(&fx, s.generation()), balances), (1, 0), "{fx:?}"),
            Step::Balance => assert_eq!((starts(&fx, s.generation()), balances), (0, 1), "{fx:?}"),
            _ => assert_eq!((starts(&fx, s.generation()), balances), (0, 0), "{fx:?}"),
        }
    }
}

#[test]
fn a_folder_answer_without_a_name_keeps_the_pairs_taken() {
    let mut s = ready();
    s.apply(folder("flute_vs_trombone", FolderState::Ok { taken: true }));
    s.apply(folder("", FolderState::Ok { taken: false }));
    assert_eq!(s.vm.folder, FolderState::Ok { taken: true });
    // A missing folder is still news.
    s.apply(folder("", FolderState::Missing));
    assert_eq!(s.vm.folder, FolderState::Missing);
    // Nothing known about a new pair: the folder's own answer is taken as it is.
    s.dispatch(Action::PickFighter {
        side: 1,
        name: "PIANO".to_string(),
    });
    s.apply(folder("", FolderState::Ok { taken: false }));
    assert_eq!(s.vm.folder, FolderState::Ok { taken: false });
}
