//! The Render screen's view model: the mock-up's four queued videos (interface reference, section 11).

use std::path::PathBuf;

use super::base;
use crate::model::library::FolderState;
use crate::model::queue::{FailHint, JobSpec, JobState, JobUpdate, Queue, STEP_ORDER, StepKey, StepLine, StepState};
use crate::model::{JobId, NF, Step, View};
use crate::seam::{Event, sample};
use crate::state::{AppState, ViewModel};

/// The failed job's headline and detail, as the mock-up has them with the advice of section 9.
pub const FAILED_HEADLINE: &str = "Stopped at the loudness check";
pub const FAILED_DETAIL: &str = "The opening is 2.1 dB quieter than the ending, and the limit is 1.5 dB. Nothing was \
                                 saved. Open it in Music to see the levels, or try another seed, then queue it again.";

/// The four jobs' ids. The queue shows them Tuba, Alto, Oboe, Cello.
pub const TUBA: JobId = 1;
pub const ALTO: JobId = 2;
pub const CELLO: JobId = 3;
pub const OBOE: JobId = 4;

/// Frames the rendering job has drawn, and the seconds since its first frame: about 20 s left.
const FRAMES_DONE: u32 = 1_171;
const FRAMES_SECONDS: f32 = 35.5;

/// The sample output folder, `Videos\BALLFIGHT`.
fn videos() -> PathBuf {
    PathBuf::from("Videos").join("BALLFIGHT")
}

/// A job's spec: `title`, the two colours, the theme's shown name and the seed, saved under `Videos\BALLFIGHT`.
pub fn spec(title: &str, colors: [[u8; 3]; 2], theme: &str, seed: u64) -> JobSpec {
    JobSpec {
        title: title.to_string(),
        colors,
        theme: theme.to_string(),
        seed,
        fight_toml: format!("seed = {seed}\n"),
        out_root: videos(),
    }
}

/// The ten lines of a check list, each in the state `state(key)`, with `note(key)`.
fn lines(state: impl Fn(StepKey) -> StepState, note: impl Fn(StepKey) -> Option<&'static str>) -> Vec<StepLine> {
    STEP_ORDER
        .iter()
        .map(|&key| StepLine {
            key,
            state: state(key),
            note: note(key).map(str::to_string),
        })
        .collect()
}

/// Where `key` is in the check list, from 0.
fn place(key: StepKey) -> usize {
    STEP_ORDER.iter().position(|k| *k == key).unwrap_or(0)
}

/// The mock-up's queue: Tuba vs Bass drum rendering with its first seven lines done, Alto sax vs Tenor sax waiting,
/// Oboe vs Bassoon stopped at the loudness check, Cello vs Contrabass done in 48 s. Finished jobs sit at the end of
/// the list in the order they finished, so Oboe vs Bassoon (shown first of the two) finished last.
pub fn queue() -> Queue {
    let mut q = Queue::new();
    q.add(spec(
        "Tuba vs Bass drum",
        [[0x68, 0x5c, 0xec], [0x80, 0x80, 0x8c]],
        "Dark drumline",
        9120,
    ));
    q.add(spec(
        "Alto sax vs Tenor sax",
        [[0xc8, 0x50, 0xf0], [0xe6, 0x6e, 0x1e]],
        "Dark disco",
        15007,
    ));
    q.add(spec(
        "Cello vs Contrabass",
        [[0xc4, 0x54, 0x28], [0x46, 0x6e, 0xc8]],
        "Dark tango",
        4471,
    ));
    q.add(spec(
        "Oboe vs Bassoon",
        [[0x9c, 0xb0, 0x34], [0x1e, 0x82, 0x4c]],
        "Heist",
        662,
    ));
    for job in &mut q.jobs {
        match job.id {
            TUBA => {
                job.state = JobState::Rendering {
                    done: FRAMES_DONE,
                    total: NF,
                };
                job.steps = lines(
                    |key| match place(key) {
                        0..7 => StepState::Done,
                        7 => StepState::Now,
                        _ => StepState::Todo,
                    },
                    |key| match key {
                        StepKey::Loudness => Some("Opening within 0.6 dB of the ending"),
                        StepKey::Flashes => Some("Busiest stretch peaks at 17% of the frame"),
                        _ => None,
                    },
                );
            }
            CELLO => {
                job.state = JobState::Done {
                    folder: videos().join("cello_vs_contrabass"),
                    seconds: 48.0,
                };
                job.steps = lines(|_| StepState::Done, |_| None);
            }
            OBOE => {
                job.state = JobState::Failed {
                    headline: FAILED_HEADLINE.to_string(),
                    detail: FAILED_DETAIL.to_string(),
                    hint: FailHint::OpenInMusic,
                };
                job.steps = lines(
                    |key| match place(key) {
                        0..3 => StepState::Done,
                        3 => StepState::Failed,
                        _ => StepState::Todo,
                    },
                    |key| (key == StepKey::Loudness).then_some("The opening is 2.1 dB quieter than the ending"),
                );
            }
            _ => {}
        }
    }
    q
}

/// The mock-up's Render screen: the take base (seed 18433, handicap `[2.0, 0.4]`) on Render, music take 0417, the
/// drafted sample captions on the Instagram tab, the output folder `Videos\BALLFIGHT` writable with the pair's
/// folder name free, ffmpeg found, and the mock-up's queue. The rendering job's frame count comes in as the engine
/// sends it, so the status line is the state's own: "Rendering Tuba vs Bass drum, about 20 s left".
pub fn vm() -> ViewModel {
    let mut vm = base::with_take();
    vm.view = View::Step(Step::Render);
    vm.step = Step::Render;
    vm.draft.music_seed = 417;
    vm.draft.captions = sample::captions();
    vm.folder = FolderState::Ok { taken: false };
    vm.ffmpeg = Some(Ok("ffmpeg 9.0.2".to_string()));
    vm.queue = queue();
    let mut state = AppState::showing(vm);
    state.apply(Event::Job {
        id: TUBA,
        update: JobUpdate::Frames {
            done: FRAMES_DONE,
            total: NF,
            seconds: FRAMES_SECONDS,
        },
    });
    state.vm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Platform;
    use crate::state::queue_run::why_not_add;

    #[test]
    fn the_render_fixture_is_the_mock_up() {
        let vm = vm();
        assert_eq!(vm.view, View::Step(Step::Render));
        assert_eq!(vm.draft.seed, Some(18433));
        assert_eq!(vm.draft.music_seed, 417);
        assert_eq!(vm.caption_tab, Platform::Instagram);
        assert_eq!(vm.draft.captions, sample::captions());
        assert_eq!(vm.folder, FolderState::Ok { taken: false });
        assert!(matches!(vm.ffmpeg, Some(Ok(_))));
        assert_eq!(vm.queue.shown(), vec![TUBA, ALTO, OBOE, CELLO]);
        assert_eq!(vm.queue.counts(), (1, 1));
        assert_eq!(vm.queue.rendering(), Some(TUBA));
        assert_eq!(
            vm.queue.job(TUBA).map(|j| j.state.clone()),
            Some(JobState::Rendering { done: 1_171, total: NF })
        );
        assert!(!vm.queue.paused);
        assert_eq!(vm.selected_job, None);
        assert_eq!(vm.status, "Rendering Tuba vs Bass drum, about 20 s left");
        assert_eq!(why_not_add(&vm), None);
    }
}
