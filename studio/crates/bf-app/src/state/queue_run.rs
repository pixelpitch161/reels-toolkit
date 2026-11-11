//! The render queue, the output folder, the library and the settings.
//!
//! One rule decides the "Add to queue" button ([`why_not_add`]): the screen shows its reason and disables the
//! button, and `Action::AddToQueue` sends the video exactly when it gives none, so the button is never enabled
//! without doing something and never disabled without its reason.
//!
//! Late answers (Review Focus 3) change nothing:
//! - a `Folder` answer for another root, or for another pair's folder name (an empty name is about the folder
//!   itself, as at start or after creating it);
//! - a `Library` for a folder that is no longer the output folder, and a `LibraryCover` for a folder that is not
//!   among the library's entries;
//! - a `Job` update for a job that is not rendering (one the queue does not hold, a waiting one, or one already
//!   finished or cancelled);
//! - a `FightOpened` when no open is pending. Only the person cancels an open (ruling M2-R20): one open is pending
//!   at a time (a later one replaces it), and an action of theirs that changes the fight drops it. No engine event
//!   does: not a result that begins a generation meanwhile (a balance with a new handicap, a second roster), which
//!   the opened fight replaces, and not a `Problem`, which does not say what it answers. An open whose file could
//!   not be opened stays waiting, harmlessly, until the person's next open or fight change replaces it.
//!
//! An `Enqueued` is never late: the owner asked for that video, fixed when it was asked for, so it joins the queue
//! even if the draft has changed since.

use std::path::{Path, PathBuf};

use super::{Action, AppState, Effect, LibraryView, QueueAction, ViewModel};
use crate::model::draft::Draft;
use crate::model::library::FolderState;
use crate::model::queue::{JobState, JobUpdate};
use crate::model::{Gen, JobId, Notice, Step, View};
use crate::seam::{Command, Event};
use crate::wording;

/// What the queue, folder and library rules remember between actions and events.
#[derive(Debug, Default)]
pub(crate) struct QueueRun {
    /// The output folder, the pair's folder name and the seed the folder was last checked for.
    checked: Option<(PathBuf, String, Option<u64>)>,
    /// The rendering job's last frame count: id, frames done, of total, seconds since the first frame.
    frames: Option<(JobId, u32, u32, f32)>,
    /// The one fight asked to be opened and not answered yet.
    pending_open: Option<PendingOpen>,
    /// "Opened {title}", with the draft and view it was shown with: it stays while they do.
    opened: Option<(String, Draft, View)>,
    /// `vm.folder` came from an answer about the current pair's folder name (not only about the folder itself).
    folder_named: bool,
}

/// A fight asked to be opened: what was asked for, and the step it lands on.
#[derive(Debug)]
struct PendingOpen {
    /// For the record; the answer (`FightOpened`) does not echo it, so the last asked-for open is the one answered.
    _what: OpenWhat,
    step: Step,
}

#[derive(Debug)]
#[expect(
    dead_code,
    reason = "kept for the record and the state's Debug print; `FightOpened` does not echo what was opened"
)]
enum OpenWhat {
    /// A library video's fight file.
    File(PathBuf),
    /// A queued job's fight text.
    Text(String),
}

/// Why "Add to queue" cannot do anything now, in words; `None` when it can. The first of these that holds: a fight
/// problem; no seed; a take problem; the take still being drawn; the output folder not checked yet, missing or not
/// writable; ffmpeg not answered yet, or not found.
pub fn why_not_add(vm: &ViewModel) -> Option<String> {
    if let Some(problem) = &vm.fight_problem {
        return Some(problem.clone());
    }
    let Some(seed) = vm.draft.seed else {
        return Some("Pick a seed first.".to_string());
    };
    if let Some(problem) = &vm.take.problem {
        return Some(problem.clone());
    }
    if vm.take.story.is_none() {
        return Some(format!("Seed {seed} is still being drawn."));
    }
    let path = vm.settings.output_folder.display();
    match &vm.folder {
        FolderState::Unknown => return Some("Checking the output folder.".to_string()),
        FolderState::Missing => {
            return Some(format!(
                "The output folder {path} is missing. Create it, or choose another folder."
            ));
        }
        FolderState::NotWritable(_) => {
            return Some(format!(
                "The output folder {path} cannot be written to. Choose another folder."
            ));
        }
        FolderState::Ok { .. } => {}
    }
    match &vm.ffmpeg {
        None => Some("Looking for ffmpeg.".to_string()),
        Some(Err(_)) => Some(
            "ffmpeg was not found on this PC, so videos cannot be saved yet. Install ffmpeg, then start the app again."
                .to_string(),
        ),
        Some(Ok(_)) => None,
    }
}

impl AppState {
    /// At start: a job the app closed on waits again and the queue is paused if anything waits; the notices
    /// already given are kept and the restart's own follows them.
    pub(super) fn after_restart(&mut self) {
        let interrupted = self.vm.queue.after_restart() as u64;
        let waiting = self.vm.queue.counts().1 as u64;
        let text = if interrupted == 1 {
            "1 video was interrupted when the app closed. It is back in the queue. Resume the queue to render it."
                .to_string()
        } else if interrupted > 1 {
            format!(
                "{} were interrupted when the app closed. They are back in the queue. Resume the queue to render \
                 them.",
                wording::plural(interrupted, "video", "videos")
            )
        } else if waiting == 1 {
            "1 video is waiting in the queue. Resume the queue to render it.".to_string()
        } else if waiting > 1 {
            format!(
                "{} are waiting in the queue. Resume the queue to render them.",
                wording::plural(waiting, "video", "videos")
            )
        } else {
            return;
        };
        self.vm.notices.push(Notice { text });
    }

    /// The actions of the queue, the output folder, the library and the settings.
    pub(super) fn queue_action(&mut self, action: Action, fx: &mut Vec<Effect>) {
        match action {
            Action::ChangeFolder => fx.push(Effect::Send(Command::PickFolder {
                start: self.vm.settings.output_folder.clone(),
            })),
            Action::CreateFolder => fx.push(Effect::Send(Command::CreateFolder {
                root: self.vm.settings.output_folder.clone(),
            })),
            Action::AddToQueue => {
                if why_not_add(&self.vm).is_none() {
                    fx.push(Effect::Send(Command::Enqueue {
                        draft: self.vm.draft.clone(),
                        out_root: self.vm.settings.output_folder.clone(),
                    }));
                }
            }
            Action::Queue(a) => self.on_queue_action(a, fx),
            Action::RefreshLibrary => self.scan_library(fx),
            Action::SelectVideo(folder) => {
                if self.in_library(&folder) {
                    self.vm.library.selected = Some(folder);
                }
            }
            Action::Reopen(folder) => {
                // Only an entry of the library leads anywhere (ruling M2-R18), and only one that can be reopened.
                let path = self
                    .vm
                    .library
                    .entries
                    .iter()
                    .find(|e| e.folder == folder && e.problem.is_none())
                    .and_then(|e| e.fight_path.clone());
                if let Some(path) = path {
                    self.ask_open(OpenWhat::File(path.clone()), Step::Matchup);
                    fx.push(Effect::Send(Command::OpenFight { path }));
                }
            }
            Action::OpenVideoFolder(folder) => {
                if self.in_library(&folder) {
                    fx.push(Effect::Send(Command::RevealFolder { path: folder }));
                }
            }
            Action::SetWorkers(n) => {
                let n = n.clamp(1, self.vm.cores.max(1));
                self.vm.settings.workers = n;
                fx.push(Effect::SaveSettings(self.vm.settings.clone()));
                fx.push(Effect::Send(Command::SetWorkers(n)));
            }
            Action::Dismiss(i) if i < self.vm.notices.len() => {
                self.vm.notices.remove(i);
            }
            // The other actions are the fight's (`fight.rs`, `seeds.rs`); a notice that is not there stays gone.
            _ => {}
        }
    }

    fn on_queue_action(&mut self, action: QueueAction, fx: &mut Vec<Effect>) {
        let q = &mut self.vm.queue;
        match action {
            QueueAction::Pause => {
                q.paused = true;
                fx.push(Effect::SaveQueue(q.clone()));
            }
            QueueAction::Resume => {
                q.paused = false;
                self.save_and_run_next(fx);
            }
            QueueAction::Cancel(id) => {
                if q.rendering() == Some(id) {
                    fx.push(Effect::Send(Command::CancelJob { id }));
                }
            }
            QueueAction::Remove(id) => {
                if q.remove(id) {
                    if self.vm.selected_job == Some(id) {
                        self.vm.selected_job = None;
                    }
                    fx.push(Effect::SaveQueue(self.vm.queue.clone()));
                }
            }
            QueueAction::Move { id, to } => {
                if q.move_to(id, to) {
                    fx.push(Effect::SaveQueue(q.clone()));
                }
            }
            QueueAction::Requeue(id) => {
                if q.requeue(id) {
                    self.save_and_run_next(fx);
                }
            }
            QueueAction::Select(id) => {
                if q.job(id).is_some() {
                    self.vm.selected_job = Some(id);
                }
            }
            QueueAction::OpenFolder(id) => {
                if let Some(job) = q.job(id) {
                    let path = match &job.state {
                        JobState::Done { folder, .. } => folder.clone(),
                        _ => job.spec.out_root.clone(),
                    };
                    fx.push(Effect::Send(Command::RevealFolder { path }));
                }
            }
            QueueAction::OpenIn { id, step } => {
                if let Some(job) = q.job(id) {
                    let toml = job.spec.fight_toml.clone();
                    self.ask_open(OpenWhat::Text(toml.clone()), step);
                    fx.push(Effect::Send(Command::OpenFightText { toml }));
                }
            }
        }
    }

    /// The events of the queue, the output folder, the library and opened fights.
    pub(super) fn queue_event(&mut self, event: Event, fx: &mut Vec<Effect>) {
        match event {
            Event::Enqueued { spec } => {
                self.vm.queue.add(spec);
                self.save_and_run_next(fx);
            }
            Event::EnqueueFailed { problem } => self.vm.notices.push(Notice { text: problem }),
            Event::Job { id, update } => self.on_job(id, update, fx),
            Event::Library { root, entries, problem } => {
                if root != self.vm.settings.output_folder {
                    return;
                }
                let lib = &mut self.vm.library;
                lib.covers
                    .retain(|folder, _| entries.iter().any(|e| &e.folder == folder));
                if lib
                    .selected
                    .as_ref()
                    .is_some_and(|s| !entries.iter().any(|e| &e.folder == s))
                {
                    lib.selected = None;
                }
                lib.entries = entries;
                lib.problem = problem;
                lib.scanning = false;
            }
            Event::LibraryCover { folder, cover } => {
                if self.in_library(&folder) {
                    self.vm.library.covers.insert(folder, cover);
                }
            }
            Event::FightOpened { draft, title } => self.on_fight_opened(draft, title, fx),
            Event::Tools { ffmpeg } => self.vm.ffmpeg = Some(ffmpeg),
            Event::Folder { root, name, state } => {
                let stem = wording::folder_stem(&self.vm.draft.fighters);
                if root != self.vm.settings.output_folder || !(name.is_empty() || name == stem) {
                    return;
                }
                if !name.is_empty() {
                    self.run.folder_named = true;
                    self.vm.folder = state;
                    return;
                }
                // An answer about the folder itself says nothing about the pair's name: a `taken` already known
                // for the pair is kept.
                self.vm.folder = match (state, &self.vm.folder) {
                    (FolderState::Ok { .. }, FolderState::Ok { taken }) if self.run.folder_named => {
                        FolderState::Ok { taken: *taken }
                    }
                    (state, _) => {
                        self.run.folder_named = false;
                        state
                    }
                };
            }
            Event::FolderPicked { path: Some(path) } => {
                self.vm.settings.output_folder = path;
                fx.push(Effect::SaveSettings(self.vm.settings.clone()));
                self.vm.library = LibraryView::default();
                self.check_folder(fx);
                self.scan_library(fx);
            }
            Event::FolderPicked { path: None } => {}
            // A problem does not say what it answers, so it leaves a waiting open alone (ruling M2-R20).
            Event::Problem { text } => self.vm.notices.push(Notice { text }),
            _ => {}
        }
    }

    fn on_job(&mut self, id: JobId, update: JobUpdate, fx: &mut Vec<Effect>) {
        if self.vm.queue.rendering() != Some(id) {
            // Late: a job that is not rendering (Review Focus 3).
            return;
        }
        let ends = matches!(
            update,
            JobUpdate::Done { .. } | JobUpdate::Failed { .. } | JobUpdate::Cancelled
        );
        let is_done = matches!(update, JobUpdate::Done { .. });
        if let JobUpdate::Frames { done, total, seconds } = update {
            self.run.frames = Some((id, done, total, seconds));
        }
        self.vm.queue.apply(id, update);
        if ends {
            self.run.frames = None;
            self.vm.queue.trim();
            if self.vm.selected_job.is_some_and(|sel| self.vm.queue.job(sel).is_none()) {
                self.vm.selected_job = None;
            }
            // Review Focus 4: whatever the end, the queue goes on.
            self.save_and_run_next(fx);
            if is_done {
                self.scan_library(fx);
            }
        }
    }

    /// Saves the queue after a change, starting the next waiting job first when nothing renders and the queue is
    /// not paused, so the saved queue holds that job as rendering (a restart then knows it was interrupted).
    fn save_and_run_next(&mut self, fx: &mut Vec<Effect>) {
        let next = self.vm.queue.next_to_run();
        if let Some(id) = next {
            self.vm.queue.start(id);
        }
        fx.push(Effect::SaveQueue(self.vm.queue.clone()));
        if let Some(job) = next.and_then(|id| self.vm.queue.job(id)) {
            self.run.frames = None;
            fx.push(Effect::Send(Command::RunJob {
                id: job.id,
                fight_toml: job.spec.fight_toml.clone(),
                out_root: job.spec.out_root.clone(),
            }));
        }
    }

    /// A fight is asked to be opened, landing on `step`; it replaces any open still waiting (the last one wins).
    fn ask_open(&mut self, what: OpenWhat, step: Step) {
        self.run.pending_open = Some(PendingOpen { _what: what, step });
    }

    fn on_fight_opened(&mut self, draft: Draft, title: String, fx: &mut Vec<Effect>) {
        let Some(open) = self.run.pending_open.take() else {
            // Nobody asked, or the person moved on since (ruling M2-R20).
            return;
        };
        // The view first, so opening the draft does not start a search for the step the person is leaving (from
        // Seeds into Music, say); the opened fight replaces whatever the engine left meanwhile; then its step opens
        // as the person would open it (Balance balances, Seeds searches once).
        let view = View::Step(open.step);
        self.vm.view = view;
        self.vm.step = open.step;
        self.open_draft(draft, fx);
        self.go(view, fx);
        if self.vm.draft.seed.is_some() {
            self.ask_take(fx);
        }
        self.run.opened = Some((title, self.vm.draft.clone(), self.vm.view));
    }

    /// Asks for the videos under the output folder (the Library opening, "Refresh", a video done, a new folder).
    pub(super) fn scan_library(&mut self, fx: &mut Vec<Effect>) {
        self.vm.library.scanning = true;
        fx.push(Effect::Send(Command::ScanLibrary {
            root: self.vm.settings.output_folder.clone(),
        }));
    }

    fn in_library(&self, folder: &Path) -> bool {
        self.vm.library.entries.iter().any(|e| e.folder == folder)
    }

    /// After every action and event: the output folder is checked again when it, the fighters or the seed changed
    /// (what was known is forgotten when the folder or the pair's folder name changed), and the "Opened" line is
    /// dropped once the draft or the view moved on. `by_person` is the generation before the person's action
    /// (`None` for an engine event): an action of theirs that changed the fight drops a waiting open.
    pub(super) fn after_queue_run(&mut self, by_person: Option<Gen>, fx: &mut Vec<Effect>) {
        if by_person.is_some_and(|before| before != self.generation) {
            self.run.pending_open = None;
        }
        if self.vm.roster.is_some() {
            self.check_folder(fx);
        }
        if let Some((_, draft, view)) = &self.run.opened
            && (draft != &self.vm.draft || view != &self.vm.view)
        {
            self.run.opened = None;
        }
    }

    /// Asks about the output folder and the pair's folder name, unless it was last asked for this folder, name and
    /// seed.
    fn check_folder(&mut self, fx: &mut Vec<Effect>) {
        let root = self.vm.settings.output_folder.clone();
        let name = if self.vm.roster.is_some() {
            wording::folder_stem(&self.vm.draft.fighters)
        } else {
            String::new()
        };
        let key = (root.clone(), name.clone(), self.vm.draft.seed);
        if self.run.checked.as_ref() == Some(&key) {
            return;
        }
        if self
            .run
            .checked
            .as_ref()
            .is_some_and(|(r, n, _)| r != &root || n != &name)
        {
            self.vm.folder = FolderState::Unknown;
            self.run.folder_named = false;
        }
        self.run.checked = Some(key);
        fx.push(Effect::Send(Command::CheckFolder { root, name }));
    }

    /// Records the folder the shown state is about, without asking (for `AppState::new` and `showing`).
    pub(super) fn folder_checked_as_shown(&mut self) {
        let name = if self.vm.roster.is_some() {
            wording::folder_stem(&self.vm.draft.fighters)
        } else {
            String::new()
        };
        self.run.checked = Some((self.vm.settings.output_folder.clone(), name, self.vm.draft.seed));
    }

    /// The status line's rendering part: "Rendering {title}, about {n} s left", or "Rendering {title}" until a
    /// frame count can give a time (widget rule 9).
    pub(super) fn rendering_status(&self) -> Option<String> {
        let id = self.vm.queue.rendering()?;
        let title = &self.vm.queue.job(id)?.spec.title;
        if let Some((fid, done, total, seconds)) = self.run.frames
            && fid == id
            && done > 0
            && done <= total
            && seconds.is_finite()
            && seconds >= 0.0
        {
            // Finite and not negative here, so the cast is exact after rounding.
            let left = (f64::from(seconds) * f64::from(total - done) / f64::from(done)).round() as u64;
            return Some(format!("Rendering {title}, about {left} s left"));
        }
        Some(format!("Rendering {title}"))
    }

    /// "Opened {title}" while the opened fight's draft and view stand.
    pub(super) fn opened_status(&self) -> Option<String> {
        let (title, draft, view) = self.run.opened.as_ref()?;
        (draft == &self.vm.draft && view == &self.vm.view).then(|| format!("Opened {title}"))
    }
}

#[cfg(test)]
#[path = "queue_run_tests.rs"]
mod tests;
