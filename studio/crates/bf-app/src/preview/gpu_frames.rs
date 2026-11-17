//! Engine 2's preview frames: `bf2-render` draws a take's frames on the window's own wgpu device (ruling E6-R4),
//! at half the video's size, through the same [`FrameSource`] the current engine's picture uses, so the scheduler
//! and the player do not change.
//!
//! One painter draws at a time (it owns one renderer); the scheduler's other threads wait on its lock, and its
//! `pace()` sees the longer frame time and draws ahead less. A draw that panics (a lost device, say) is a failed
//! frame: the scheduler catches it (widget rule 11), and the lock, poisoned by it, is taken back for the next one.

use std::sync::{Arc, Mutex, OnceLock};

use bf_core::fightfile::FightFile;
use bf_export::job2::Frames2;
use bf2_render::Gpu;
use bf2_sim::Recording2;

use crate::model::take::{Frame, FrameSource};

/// The preview's frame width: half the video's 1080, the same passes.
pub const PREVIEW_W: u32 = 540;
/// The preview's frame height: half the video's 1920.
pub const PREVIEW_H: u32 = 960;

/// The take's words when no device can draw Engine 2.
pub const NO_GPU: &str = "This PC's graphics card cannot draw Engine 2 fights.";

/// A wgpu device the engine may share between takes: the window's (from eframe), or a headless one.
#[derive(Clone)]
pub struct SharedGpu(pub Arc<Gpu>);

impl std::fmt::Debug for SharedGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SharedGpu").field(&self.0.adapter_name).finish()
    }
}

impl PartialEq for SharedGpu {
    /// The same device.
    fn eq(&self, other: &SharedGpu) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The window's device, when eframe draws with wgpu: called once as the window is made.
pub fn shared_gpu(cc: &eframe::CreationContext<'_>) -> Option<SharedGpu> {
    let rs = cc.wgpu_render_state.as_ref()?;
    Some(SharedGpu(Arc::new(Gpu::from_device(
        rs.device.clone(),
        rs.queue.clone(),
        rs.adapter.get_info().name,
    ))))
}

/// The device an Engine 2 take draws on: the window's (`window`), or else one made by `headless` (once for the
/// app, by the caller); [`NO_GPU`] when there is neither.
///
/// # Errors
/// [`NO_GPU`].
pub fn take_gpu(window: Option<&SharedGpu>, headless: impl FnOnce() -> Option<SharedGpu>) -> Result<SharedGpu, String> {
    match window {
        Some(g) => Ok(g.clone()),
        None => headless().ok_or_else(|| NO_GPU.to_string()),
    }
}

/// A headless device made the first time a take needs one and kept for the app's life; `None` when this PC has
/// none.
pub fn headless_once() -> Option<SharedGpu> {
    static HEADLESS: OnceLock<Option<SharedGpu>> = OnceLock::new();
    HEADLESS
        .get_or_init(|| Gpu::headless().ok().map(|g| SharedGpu(Arc::new(g))))
        .clone()
}

/// The frames of an Engine 2 take at [`PREVIEW_W`] x [`PREVIEW_H`].
pub struct Engine2Frames {
    painter: Mutex<Frames2>,
    nf: u32,
    /// Tests: this frame's draw fails.
    #[cfg(test)]
    fail_at: Option<u32>,
}

impl Engine2Frames {
    /// The frames of the fight `rec` of `ff`, drawn on `gpu`.
    ///
    /// # Errors
    /// The renderer's sentence (a fighter without an Engine 2 look, say).
    pub fn new(gpu: &Gpu, ff: &FightFile, rec: Arc<Recording2>) -> Result<Engine2Frames, String> {
        let painter = Frames2::new(gpu, ff, rec, PREVIEW_W, PREVIEW_H).map_err(|e| e.to_string())?;
        let nf = u32::try_from(painter.nf()).unwrap_or(u32::MAX);
        Ok(Engine2Frames {
            painter: Mutex::new(painter),
            nf,
            #[cfg(test)]
            fail_at: None,
        })
    }
}

impl FrameSource for Engine2Frames {
    fn frames(&self) -> u32 {
        self.nf
    }

    fn draw(&self, f: u32) -> Frame {
        #[cfg(test)]
        if self.fail_at == Some(f) {
            panic!("frame {f}: the test's draw fails");
        }
        let img = {
            let mut painter = self.painter.lock().unwrap_or_else(|e| e.into_inner());
            painter.frame(f as usize)
        };
        assert!(
            img.c == 3 && img.data.len() == img.w * img.h * 3,
            "frame {f}: the renderer gave {} x {} x {} bytes",
            img.w,
            img.h,
            img.c
        );
        Frame {
            w: u32::try_from(img.w).unwrap_or(u32::MAX),
            h: u32::try_from(img.h).unwrap_or(u32::MAX),
            rgb: img.data.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::preview::scheduler::Scheduler;

    fn dummy() -> (FightFile, Arc<Recording2>) {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fights/engine2/test_dummy.toml");
        let ff = FightFile::parse(&std::fs::read_to_string(path).expect("the dummy's file")).expect("it parses");
        let rec = match bf_engine::fight(&ff, ff.seed.expect("a seed"), true).expect("the dummy fights") {
            bf_engine::Fought::Two(r) => r,
            bf_engine::Fought::One(_) => unreachable!("an Engine 2 file"),
        };
        (ff, Arc::new(rec))
    }

    fn gpu() -> Option<Gpu> {
        let g = Gpu::headless().ok();
        if g.is_none() {
            eprintln!("no GPU here: skipped");
        }
        g
    }

    #[test]
    fn engine2_frames_draw_at_preview_size() {
        let Some(gpu) = gpu() else { return };
        let (ff, rec) = dummy();
        let frames = Engine2Frames::new(&gpu, &ff, rec).expect("the frames");
        assert_eq!(frames.frames(), 1830);
        for f in [0, 900, 1829] {
            let fr = frames.draw(f);
            assert_eq!((fr.w, fr.h), (PREVIEW_W, PREVIEW_H), "frame {f}");
            assert_eq!(fr.rgb.len(), (PREVIEW_W * PREVIEW_H * 3) as usize);
            if f == 900 {
                assert!(
                    fr.rgb.chunks(3).any(|p| p != &fr.rgb[..3]),
                    "frame 900 is one flat colour"
                );
            }
        }
    }

    #[test]
    fn engine2_frames_match_the_job() {
        let Some(gpu) = gpu() else { return };
        let (ff, rec) = dummy();
        let small = Engine2Frames::new(&gpu, &ff, rec.clone())
            .expect("the frames")
            .draw(900);
        let mut job = Frames2::new(&gpu, &ff, rec, 1080, 1920).expect("the job's frames");
        let big = job.frame(900);
        let (w, h) = (PREVIEW_W as usize, PREVIEW_H as usize);
        let mut diff = 0u64;
        for y in 0..h {
            for x in 0..w {
                for c in 0..3 {
                    let at = |dx: usize, dy: usize| u32::from(big.data[((2 * y + dy) * 1080 + 2 * x + dx) * 3 + c]);
                    let boxed = (at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4;
                    diff += u64::from(boxed.abs_diff(u32::from(small.rgb[(y * w + x) * 3 + c])));
                }
            }
        }
        let mean = diff as f64 / (w * h * 3) as f64;
        // Not a scaled copy: the same passes at half the size. What is sized in pixels (line widths, the haze the
        // glow leaves above the top rope) weighs twice as much; frame 900 measured 7.2 on lavapipe (ledger, E6
        // Task 6). The bound pins that, so a preview that drifts from the video is caught.
        assert!(mean <= 8.0, "mean absolute difference {mean:.2} per channel");
    }

    #[test]
    fn engine2_frame_failure_is_a_failed_frame() {
        let Some(gpu) = gpu() else { return };
        let (ff, rec) = dummy();
        let mut frames = Engine2Frames::new(&gpu, &ff, rec).expect("the frames");
        frames.fail_at = Some(10);
        let src: Arc<dyn FrameSource> = Arc::new(frames);
        let s = Scheduler::new(2, Arc::new(|| {}));
        s.set_source(1, Some(src));
        let until = |what: &str, done: &dyn Fn() -> bool| {
            let deadline = Instant::now() + Duration::from_secs(120);
            while !done() {
                assert!(Instant::now() < deadline, "{what} did not happen");
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        s.want(10, false, true);
        until("frame 10 failing", &|| s.failed(10));
        assert!(s.get(10).is_none());
        // the lock the panic poisoned is taken back: the next frame draws
        s.want(11, false, true);
        until("frame 11 drawing", &|| s.get(11).is_some());
        assert!(!s.failed(11));
        assert_eq!(s.get(11).map(|f| (f.w, f.h)), Some((PREVIEW_W, PREVIEW_H)));
    }

    #[test]
    fn engine2_take_gpu_falls_back_then_fails_in_words() {
        let window = SharedGpu(Arc::new(match gpu() {
            Some(g) => g,
            None => return,
        }));
        // the window's device first, without making another
        let got = take_gpu(Some(&window), || panic!("no headless device is made")).expect("the window's");
        assert_eq!(got, window);
        // without one, the headless device
        let other = window.clone();
        assert_eq!(take_gpu(None, || Some(other)), Ok(window));
        // without either, the sentence
        assert_eq!(take_gpu(None, || None), Err(NO_GPU.to_string()));
    }
}
