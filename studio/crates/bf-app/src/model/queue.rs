//! The render queue: the videos waiting, rendering and done, and the updates a rendering job sends.
//!
//! One job renders at a time (§9). The jobs are kept in one list: the waiting jobs' order in it is their order in
//! line, and a job that finishes moves to its end, so the finished jobs' order in it is the order they finished in.

use serde::{Deserialize, Serialize};

use super::{JobId, NF};

/// The ten lines of a job's check list, in the order a job goes through them.
pub const STEP_ORDER: [StepKey; 10] = [
    StepKey::Soundtrack,
    StepKey::SafeZone,
    StepKey::Focus,
    StepKey::Loudness,
    StepKey::Band,
    StepKey::Flashes,
    StepKey::LoopPoint,
    StepKey::Frames,
    StepKey::Encode,
    StepKey::Verify,
];

/// How many finished jobs the queue keeps.
const KEEP_FINISHED: usize = 20;

/// Everything a job needs to render one video, fixed when it is queued.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    /// "Flute vs Trombone".
    pub title: String,
    /// The two fighters' colours, left then right.
    pub colors: [[u8; 3]; 2],
    /// The theme's shown name.
    pub theme: String,
    pub seed: u64,
    /// The whole fight file, as TOML: the job fights it again when it runs.
    pub fight_toml: String,
    /// The output folder the video's own folder goes in.
    pub out_root: std::path::PathBuf,
}

/// One line of a job's check list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepKey {
    Soundtrack,
    SafeZone,
    Focus,
    Loudness,
    Band,
    Flashes,
    LoopPoint,
    Frames,
    Encode,
    Verify,
}

/// Where one line of the check list is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepState {
    Todo,
    Now,
    Done,
    Failed,
}

/// One line of a job's check list, with an optional note ("1,171 of 1,830").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepLine {
    pub key: StepKey,
    pub state: StepState,
    pub note: Option<String>,
}

/// What a failed job offers the owner to do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailHint {
    OpenInSeeds,
    OpenInMusic,
    CheckFolder,
    InstallFfmpeg,
    None,
}

/// Where a job is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum JobState {
    Waiting,
    /// Frames drawn so far, of `total`.
    Rendering {
        done: u32,
        total: u32,
    },
    /// Saved in `folder`, in `seconds`.
    Done {
        folder: std::path::PathBuf,
        seconds: f64,
    },
    /// Stopped: `headline` names the step, `detail` gives the numbers and what to do.
    Failed {
        headline: String,
        detail: String,
        hint: FailHint,
    },
    Cancelled,
}

/// One video in the queue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub spec: JobSpec,
    pub state: JobState,
    pub steps: Vec<StepLine>,
    /// The app closed while this job was rendering; it starts over.
    pub interrupted: bool,
}

/// The render queue, in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Queue {
    pub jobs: Vec<Job>,
    /// Holds the next waiting job; the one rendering finishes.
    pub paused: bool,
    /// The id the next queued job gets.
    pub next_id: JobId,
}

impl Default for Queue {
    fn default() -> Queue {
        Queue::new()
    }
}

impl Queue {
    /// An empty queue, not paused, whose first job gets id 1.
    pub fn new() -> Queue {
        Queue {
            jobs: Vec::new(),
            paused: false,
            next_id: 1,
        }
    }

    /// Queues a video: it waits, last in line. Its id is `next_id`, which then counts up. Ids never run out in
    /// practice (`load_queue` refuses a file whose ids are near the top), but if `next_id` has reached the top, or a
    /// job already holds it, the lowest id no job holds is used instead, so an id is never given twice and nothing
    /// overflows.
    pub fn add(&mut self, spec: JobSpec) -> JobId {
        let id = match self.next_id {
            next if next < JobId::MAX && self.job(next).is_none() => {
                self.next_id = next + 1;
                next
            }
            _ => (1..JobId::MAX).find(|&id| self.job(id).is_none()).unwrap_or(JobId::MAX),
        };
        self.jobs.push(Job {
            id,
            spec,
            state: JobState::Waiting,
            steps: Vec::new(),
            interrupted: false,
        });
        id
    }

    pub fn job(&self, id: JobId) -> Option<&Job> {
        self.jobs.iter().find(|job| job.id == id)
    }

    fn index(&self, id: JobId) -> Option<usize> {
        self.jobs.iter().position(|job| job.id == id)
    }

    /// Takes a job out of the queue. Refused (false) for the job being rendered, which is cancelled instead, and for
    /// an id the queue does not hold.
    pub fn remove(&mut self, id: JobId) -> bool {
        match self.index(id) {
            Some(i) if !is_rendering(&self.jobs[i]) => {
                self.jobs.remove(i);
                true
            }
            _ => false,
        }
    }

    /// Moves a waiting job to place `to` among the waiting jobs (0 is next; a place past the end is last). Refused
    /// (false) for a job that is not waiting.
    pub fn move_to(&mut self, id: JobId, to: usize) -> bool {
        let Some(from) = self.index(id).filter(|&i| self.jobs[i].state == JobState::Waiting) else {
            return false;
        };
        let job = self.jobs.remove(from);
        let waiting: Vec<usize> = (0..self.jobs.len())
            .filter(|&i| self.jobs[i].state == JobState::Waiting)
            .collect();
        let at = match (waiting.get(to), waiting.last()) {
            (Some(&i), _) => i,
            (None, Some(&last)) => last + 1,
            (None, None) => from,
        };
        self.jobs.insert(at, job);
        true
    }

    /// Queues a failed or cancelled job again: it waits, last in line, with its check list cleared. Refused (false)
    /// for any other job.
    pub fn requeue(&mut self, id: JobId) -> bool {
        let Some(i) = self
            .index(id)
            .filter(|&i| matches!(self.jobs[i].state, JobState::Failed { .. } | JobState::Cancelled))
        else {
            return false;
        };
        let mut job = self.jobs.remove(i);
        job.state = JobState::Waiting;
        job.steps.clear();
        job.interrupted = false;
        self.jobs.push(job);
        true
    }

    /// The job being rendered, if any.
    pub fn rendering(&self) -> Option<JobId> {
        self.jobs.iter().find(|job| is_rendering(job)).map(|job| job.id)
    }

    /// The job to start next: the first waiting one. None while the queue is paused or a job is rendering.
    pub fn next_to_run(&self) -> Option<JobId> {
        if self.paused || self.rendering().is_some() {
            return None;
        }
        self.jobs
            .iter()
            .find(|job| job.state == JobState::Waiting)
            .map(|job| job.id)
    }

    /// Starts a job: it renders from frame 0 of 1,830, its ten check lines are to do, and it is no longer marked as
    /// interrupted. Only a waiting job starts, and only while no other job renders; otherwise nothing changes.
    pub fn start(&mut self, id: JobId) {
        if self.rendering().is_some() {
            return;
        }
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) else {
            return;
        };
        if job.state != JobState::Waiting {
            return;
        }
        job.state = JobState::Rendering { done: 0, total: NF };
        job.steps = STEP_ORDER
            .iter()
            .map(|&key| StepLine {
                key,
                state: StepState::Todo,
                note: None,
            })
            .collect();
        job.interrupted = false;
    }

    /// Takes in what the rendering job reports. An update for a job that is not rendering (one that arrives after a
    /// cancel, say) is late and changes nothing. A job that finishes moves to the end of the list, so it is the
    /// newest finished one.
    pub fn apply(&mut self, id: JobId, update: JobUpdate) {
        let Some(i) = self.index(id).filter(|&i| is_rendering(&self.jobs[i])) else {
            return;
        };
        let job = &mut self.jobs[i];
        let finished = match update {
            JobUpdate::Step { key, state, note } => {
                match job.steps.iter_mut().find(|line| line.key == key) {
                    Some(line) => {
                        line.state = state;
                        line.note = note;
                    }
                    None => job.steps.push(StepLine { key, state, note }),
                }
                false
            }
            JobUpdate::Frames { done, total, .. } => {
                job.state = JobState::Rendering { done, total };
                false
            }
            JobUpdate::Done { folder, seconds } => {
                job.state = JobState::Done { folder, seconds };
                true
            }
            JobUpdate::Failed { headline, detail, hint } => {
                job.state = JobState::Failed { headline, detail, hint };
                true
            }
            JobUpdate::Cancelled => {
                job.state = JobState::Cancelled;
                true
            }
        };
        if finished {
            let job = self.jobs.remove(i);
            self.jobs.push(job);
        }
    }

    /// How many jobs are rendering and how many wait.
    pub fn counts(&self) -> (usize, usize) {
        let rendering = self.jobs.iter().filter(|job| is_rendering(job)).count();
        let waiting = self.jobs.iter().filter(|job| job.state == JobState::Waiting).count();
        (rendering, waiting)
    }

    /// The jobs in the order the queue shows them: the one rendering, then the waiting ones in line, then the
    /// finished ones, newest first.
    pub fn shown(&self) -> Vec<JobId> {
        let ids = |keep: fn(&Job) -> bool| self.jobs.iter().filter(move |job| keep(job)).map(|job| job.id);
        ids(is_rendering)
            .chain(ids(|job| job.state == JobState::Waiting))
            .chain(ids(is_finished).collect::<Vec<_>>().into_iter().rev())
            .collect()
    }

    /// A job marked as rendering waits again, first in line, marked as interrupted and with its check list cleared:
    /// it starts over. What `after_restart` does to such a job, and what the queue file holds for it.
    /// More than one such job (a hand-edited file) all go first in line, in the order the list held them.
    pub(crate) fn rendering_waits_again(&mut self) {
        let Some(first) = self.jobs.iter().position(is_rendering) else {
            return;
        };
        let (mut again, rest): (Vec<Job>, Vec<Job>) =
            std::mem::take(&mut self.jobs).into_iter().partition(is_rendering);
        self.jobs = rest;
        for job in &mut again {
            job.state = JobState::Waiting;
            job.steps.clear();
            job.interrupted = true;
        }
        // The jobs before `first` are where they were, so `first` is still a place in the shorter list.
        let first_waiting = self
            .jobs
            .iter()
            .position(|job| job.state == JobState::Waiting)
            .unwrap_or(self.jobs.len());
        let at = first_waiting.min(first);
        self.jobs.splice(at..at, again);
    }

    /// After the app starts with a saved queue: a job still marked as rendering waits again, first in line and marked
    /// as interrupted, and the queue is paused if anything waits, so nothing starts before the owner says so. Returns
    /// how many waiting jobs are marked as interrupted, by this call or by the file.
    pub fn after_restart(&mut self) -> usize {
        self.rendering_waits_again();
        if self.jobs.iter().any(|job| job.state == JobState::Waiting) {
            self.paused = true;
        }
        self.jobs
            .iter()
            .filter(|job| job.state == JobState::Waiting && job.interrupted)
            .count()
    }

    /// Keeps the 20 newest finished jobs and drops older ones.
    pub fn trim(&mut self) {
        let finished = self.jobs.iter().filter(|job| is_finished(job)).count();
        let mut drop = finished.saturating_sub(KEEP_FINISHED);
        self.jobs.retain(|job| {
            if drop > 0 && is_finished(job) {
                drop -= 1;
                false
            } else {
                true
            }
        });
    }
}

fn is_rendering(job: &Job) -> bool {
    matches!(job.state, JobState::Rendering { .. })
}

fn is_finished(job: &Job) -> bool {
    matches!(
        job.state,
        JobState::Done { .. } | JobState::Failed { .. } | JobState::Cancelled
    )
}

/// What a rendering job reports, through `seam::Event::Job`.
#[derive(Clone, Debug, PartialEq)]
pub enum JobUpdate {
    /// A line of the check list moved on.
    Step {
        key: StepKey,
        state: StepState,
        note: Option<String>,
    },
    /// Frames drawn so far; `seconds` since the first frame was drawn.
    Frames {
        done: u32,
        total: u32,
        seconds: f32,
    },
    Done {
        folder: std::path::PathBuf,
        seconds: f64,
    },
    Failed {
        headline: String,
        detail: String,
        hint: FailHint,
    },
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queue_round_trips_through_serde() {
        let spec = JobSpec {
            title: "Flute vs Trombone".to_string(),
            colors: [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]],
            theme: "Big top".to_string(),
            seed: 18433,
            fight_toml: "seed = 18433\n".to_string(),
            out_root: std::path::PathBuf::from("Videos/BALLFIGHT"),
        };
        let job = |id, state| Job {
            id,
            spec: spec.clone(),
            state,
            steps: vec![StepLine {
                key: StepKey::Loudness,
                state: StepState::Failed,
                note: Some("2.1 dB".to_string()),
            }],
            interrupted: id == 2,
        };
        let queue = Queue {
            jobs: vec![
                job(1, JobState::Waiting),
                job(
                    2,
                    JobState::Rendering {
                        done: 1_171,
                        total: 1_830,
                    },
                ),
                job(
                    3,
                    JobState::Done {
                        folder: "Videos/BALLFIGHT/flute_vs_trombone".into(),
                        seconds: 48.0,
                    },
                ),
                job(
                    4,
                    JobState::Failed {
                        headline: "Stopped at the loudness check".to_string(),
                        detail: "Nothing was saved.".to_string(),
                        hint: FailHint::OpenInMusic,
                    },
                ),
                job(5, JobState::Cancelled),
            ],
            paused: true,
            next_id: 6,
        };
        let text = serde_json::to_string(&queue).unwrap();
        assert_eq!(serde_json::from_str::<Queue>(&text).unwrap(), queue);
    }

    fn spec(title: &str) -> JobSpec {
        JobSpec {
            title: title.to_string(),
            colors: [[0xde, 0xe8, 0xf6], [0x78, 0xf0, 0xb8]],
            theme: "Big top".to_string(),
            seed: 18433,
            fight_toml: "seed = 18433\n".to_string(),
            out_root: std::path::PathBuf::from("Videos/BALLFIGHT"),
        }
    }

    fn done() -> JobUpdate {
        JobUpdate::Done {
            folder: "Videos/BALLFIGHT/flute_vs_trombone".into(),
            seconds: 48.0,
        }
    }

    fn failed() -> JobUpdate {
        JobUpdate::Failed {
            headline: "Stopped at the loudness check".to_string(),
            detail: "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB. Nothing was saved."
                .to_string(),
            hint: FailHint::OpenInMusic,
        }
    }

    fn loudness_failed() -> JobUpdate {
        JobUpdate::Step {
            key: StepKey::Loudness,
            state: StepState::Failed,
            note: Some("2.1 dB".to_string()),
        }
    }

    fn state(q: &Queue, id: JobId) -> JobState {
        q.job(id).unwrap().state.clone()
    }

    #[test]
    fn a_new_queue_is_empty() {
        let q = Queue::new();
        assert!(q.jobs.is_empty());
        assert!(!q.paused);
        assert_eq!(q.next_id, 1);
        assert_eq!(Queue::default(), q);
        assert_eq!(q.counts(), (0, 0));
        assert_eq!(q.next_to_run(), None);
    }

    #[test]
    fn ids_count_up() {
        let mut q = Queue::new();
        assert_eq!(q.add(spec("Flute vs Trombone")), 1);
        assert_eq!(q.add(spec("Tuba vs Bass drum")), 2);
        assert_eq!(q.add(spec("Oboe vs Harp")), 3);
        assert!(q.remove(3));
        // An id is never given out twice, even after its job is gone.
        assert_eq!(q.add(spec("Oboe vs Harp")), 4);
        assert_eq!(q.next_id, 5);
        assert_eq!(q.job(1).unwrap().spec.title, "Flute vs Trombone");
        assert_eq!(q.job(3), None);
        let job = q.job(4).unwrap();
        assert_eq!(job.state, JobState::Waiting);
        assert!(job.steps.is_empty());
        assert!(!job.interrupted);
        assert_eq!(q.shown(), vec![1, 2, 4]);
    }

    #[test]
    fn ids_at_the_top_never_overflow_or_repeat() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        q.next_id = JobId::MAX;
        // The top is not given out; the lowest free id is, and nothing panics.
        assert_eq!(q.add(spec("B")), 2);
        assert_eq!(q.add(spec("C")), 3);
        assert_eq!(q.next_id, JobId::MAX);
        assert!(q.remove(a));
        assert_eq!(q.add(spec("D")), 1);
        // A next id that a job already holds is not given twice either.
        let mut odd = Queue::new();
        odd.add(spec("E"));
        odd.next_id = 1;
        assert_eq!(odd.add(spec("F")), 2);
        let ids: Vec<JobId> = odd.jobs.iter().map(|job| job.id).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn one_job_at_a_time() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        assert_eq!(q.next_to_run(), Some(a));
        q.start(a);
        assert_eq!(q.rendering(), Some(a));
        assert_eq!(q.next_to_run(), None);
        assert_eq!(q.counts(), (1, 1));
        q.apply(a, done());
        assert_eq!(q.rendering(), None);
        assert_eq!(q.next_to_run(), Some(b));
        q.paused = true;
        assert_eq!(q.next_to_run(), None);
        q.paused = false;
        assert_eq!(q.next_to_run(), Some(b));
    }

    #[test]
    fn the_running_job_cannot_be_removed() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        q.start(a);
        assert!(!q.remove(a));
        assert_eq!(q.rendering(), Some(a));
        assert!(q.remove(b));
        assert!(!q.remove(b));
        assert_eq!(q.shown(), vec![a]);
        // Once it has finished it can go.
        q.apply(a, done());
        assert!(q.remove(a));
        assert!(q.jobs.is_empty());
    }

    #[test]
    fn moving_reorders_waiting_jobs_only() {
        let mut q = Queue::new();
        let d = q.add(spec("D"));
        let r = q.add(spec("R"));
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        let c = q.add(spec("C"));
        q.start(d);
        q.apply(d, done());
        q.start(r);
        assert!(q.move_to(c, 0));
        assert_eq!(q.shown(), vec![r, c, a, b, d]);
        assert!(q.move_to(c, 99));
        assert_eq!(q.shown(), vec![r, a, b, c, d]);
        assert!(q.move_to(c, 1));
        assert_eq!(q.shown(), vec![r, a, c, b, d]);
        // A finished job, the rendering job and an unknown id stay where they are.
        assert!(!q.move_to(d, 0));
        assert!(!q.move_to(r, 2));
        assert!(!q.move_to(77, 0));
        assert_eq!(q.shown(), vec![r, a, c, b, d]);
    }

    #[test]
    fn requeue_goes_last_with_clean_steps() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        let c = q.add(spec("C"));
        q.start(a);
        q.apply(a, loudness_failed());
        q.apply(a, failed());
        assert!(q.requeue(a));
        let job = q.job(a).unwrap();
        assert_eq!(job.state, JobState::Waiting);
        assert!(job.steps.is_empty());
        assert_eq!(q.shown(), vec![b, c, a]);
        // A cancelled job can be queued again too.
        q.start(b);
        q.apply(b, JobUpdate::Cancelled);
        assert!(q.requeue(b));
        assert_eq!(q.shown(), vec![c, a, b]);
        // A waiting, rendering or finished job cannot.
        assert!(!q.requeue(c));
        q.start(c);
        assert!(!q.requeue(c));
        q.apply(c, done());
        assert!(!q.requeue(c));
        assert!(!q.requeue(77));
    }

    #[test]
    fn updates_change_the_job() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        q.start(a);
        assert_eq!(state(&q, a), JobState::Rendering { done: 0, total: NF });
        let lines = &q.job(a).unwrap().steps;
        assert_eq!(lines.iter().map(|l| l.key).collect::<Vec<_>>(), STEP_ORDER.to_vec());
        assert!(lines.iter().all(|l| l.state == StepState::Todo && l.note.is_none()));

        q.apply(
            a,
            JobUpdate::Step {
                key: StepKey::Frames,
                state: StepState::Now,
                note: Some("1,171 of 1,830".to_string()),
            },
        );
        let frames = q.job(a).unwrap().steps[7].clone();
        assert_eq!(
            frames,
            StepLine {
                key: StepKey::Frames,
                state: StepState::Now,
                note: Some("1,171 of 1,830".to_string()),
            }
        );
        q.apply(
            a,
            JobUpdate::Frames {
                done: 1_171,
                total: 1_830,
                seconds: 40.5,
            },
        );
        assert_eq!(
            state(&q, a),
            JobState::Rendering {
                done: 1_171,
                total: 1_830
            }
        );
        q.apply(a, done());
        assert_eq!(
            state(&q, a),
            JobState::Done {
                folder: "Videos/BALLFIGHT/flute_vs_trombone".into(),
                seconds: 48.0,
            }
        );

        let b = q.add(spec("B"));
        q.start(b);
        q.apply(b, loudness_failed());
        q.apply(b, failed());
        let JobState::Failed { headline, hint, .. } = state(&q, b) else {
            panic!("{:?}", state(&q, b));
        };
        assert_eq!(headline, "Stopped at the loudness check");
        assert_eq!(hint, FailHint::OpenInMusic);
        // The failed step keeps its line.
        let loudness = &q.job(b).unwrap().steps[3];
        assert_eq!(loudness.key, StepKey::Loudness);
        assert_eq!(loudness.state, StepState::Failed);
        assert_eq!(loudness.note.as_deref(), Some("2.1 dB"));

        let c = q.add(spec("C"));
        q.start(c);
        q.apply(c, JobUpdate::Cancelled);
        assert_eq!(state(&q, c), JobState::Cancelled);
    }

    #[test]
    fn a_late_update_does_not_bring_a_job_back() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        q.start(a);
        q.apply(a, JobUpdate::Cancelled);
        // The engine's last frame count arrives after the cancel; the job stays cancelled and the next one may run.
        q.apply(
            a,
            JobUpdate::Frames {
                done: 12,
                total: 1_830,
                seconds: 0.4,
            },
        );
        q.apply(a, loudness_failed());
        q.apply(a, done());
        assert_eq!(state(&q, a), JobState::Cancelled);
        assert!(q.job(a).unwrap().steps.iter().all(|l| l.state == StepState::Todo));
        // A waiting job is not touched by updates either.
        q.apply(b, done());
        assert_eq!(state(&q, b), JobState::Waiting);
        assert_eq!(q.next_to_run(), Some(b));
    }

    // Review Focus 4
    #[test]
    fn a_failed_job_lets_the_next_one_run() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        let c = q.add(spec("C"));
        q.start(a);
        q.apply(a, loudness_failed());
        q.apply(a, failed());
        assert_eq!(q.rendering(), None);
        assert_eq!(q.next_to_run(), Some(b));
        q.start(b);
        q.apply(b, JobUpdate::Cancelled);
        assert_eq!(q.next_to_run(), Some(c));
        assert_eq!(q.counts(), (0, 1));
    }

    // Review Focus 4
    #[test]
    fn after_a_restart_the_interrupted_job_waits() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        let c = q.add(spec("C"));
        q.start(a);
        // C was moved to the front of the line while A rendered.
        assert!(q.move_to(c, 0));
        q.apply(
            a,
            JobUpdate::Frames {
                done: 600,
                total: 1_830,
                seconds: 20.0,
            },
        );
        assert_eq!(q.after_restart(), 1);
        let job = q.job(a).unwrap();
        assert_eq!(job.state, JobState::Waiting);
        assert!(job.interrupted);
        assert!(job.steps.is_empty());
        assert!(q.paused);
        assert_eq!(q.rendering(), None);
        // It was the one rendering, so it is first in line again.
        assert_eq!(q.shown(), vec![a, c, b]);
        // Paused: nothing starts until the owner says so.
        assert_eq!(q.next_to_run(), None);
        q.paused = false;
        assert_eq!(q.next_to_run(), Some(a));
        // Asking again counts the mark already there.
        assert_eq!(q.after_restart(), 1);
        q.start(a);
        assert!(!q.job(a).unwrap().interrupted);

        // With nothing waiting the queue is not paused.
        let mut idle = Queue::new();
        let d = idle.add(spec("D"));
        idle.start(d);
        idle.apply(d, done());
        assert_eq!(idle.after_restart(), 0);
        assert!(!idle.paused);
        assert_eq!(Queue::new().after_restart(), 0);
        assert!(!Queue::new().paused);

        // Jobs that simply wait pause the queue but are not counted as interrupted.
        let mut waiting = Queue::new();
        waiting.add(spec("E"));
        assert_eq!(waiting.after_restart(), 0);
        assert!(waiting.paused);
    }

    #[test]
    fn shown_order() {
        let mut q = Queue::new();
        let a = q.add(spec("A"));
        let b = q.add(spec("B"));
        let c = q.add(spec("C"));
        let d = q.add(spec("D"));
        let e = q.add(spec("E"));
        q.start(b);
        q.apply(b, failed());
        q.start(a);
        q.apply(a, done());
        q.start(c);
        assert!(q.move_to(e, 0));
        // Rendering, then waiting in line, then finished newest first (A finished after B).
        assert_eq!(q.shown(), vec![c, e, d, a, b]);
        assert_eq!(q.counts(), (1, 2));
    }

    #[test]
    fn trim_keeps_20_finished() {
        let mut q = Queue::new();
        for n in 1..=25 {
            let id = q.add(spec(&format!("Job {n}")));
            q.start(id);
            q.apply(id, if n % 3 == 0 { failed() } else { done() });
        }
        let w = q.add(spec("Waiting"));
        let r = q.add(spec("Rendering"));
        q.start(r);
        q.trim();
        let mut expected = vec![r, w];
        expected.extend((6..=25).rev());
        assert_eq!(q.shown(), expected);
        assert_eq!(q.jobs.len(), 22);
        // Trimming again changes nothing.
        let before = q.clone();
        q.trim();
        assert_eq!(q, before);
    }
}
