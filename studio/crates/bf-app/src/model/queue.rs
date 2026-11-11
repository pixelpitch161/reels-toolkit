//! The render queue: the videos waiting, rendering and done, and the updates a rendering job sends.
//!
//! Types only; the queue's methods are in a later task.

use serde::{Deserialize, Serialize};

use super::JobId;

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

/// The render queue, in order. Its methods are in a later task.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Queue {
    pub jobs: Vec<Job>,
    /// Holds the next waiting job; the one rendering finishes.
    pub paused: bool,
    /// The id the next queued job gets.
    pub next_id: JobId,
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
}
