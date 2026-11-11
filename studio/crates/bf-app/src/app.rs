//! The app as eframe runs it: events in (`logic`), paint (`ui`), actions out.
//!
//! How a frame runs. `logic` (eframe calls it before every `ui`, and also while the window is hidden when a repaint
//! was asked for, so the engine's events are taken and the next queued video starts with nobody watching): take
//! every waiting engine event and apply it, carry out the effects, let the player move on and upload its new
//! picture, copy the player's view into the state, tell the engine when playback starts or stops, and turn the
//! writer's problems into notices. `ui`: the keys, then the panels, each screen pushing actions; then each action is
//! dispatched in order, logged, and its effects carried out. Every effect is carried out once: `Send` to the
//! engine, `Player` to the player, `SaveSettings` and `SaveQueue` to the writer.

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::wgpu;
use egui::{CentralPanel, Color32, Frame as Fill, Margin, Panel, Rect, Stroke, TextureHandle, TextureOptions, Ui};

use crate::engine::{self, EngineConfig, STOP_WAIT};
use crate::model::queue::Queue;
use crate::model::take::Frame;
use crate::model::{Notice, Settings, Step, View};
use crate::preview::audio::AudioOut;
use crate::preview::player::{Now, Player};
use crate::screens::{balance, library, matchup, music, render, seeds, settings, shell, stage, timeline};
use crate::seam::fake::{self, Pace};
use crate::seam::{Command, EngineHandle, Waker};
use crate::state::{Action, AppState, Effect, ViewModel};
use crate::store::queue_file::load_queue;
use crate::store::settings::{default_settings, load_settings};
use crate::store::{Paths, Writer};
use crate::theme::{self, BOUT_H, GROUND, LINE_SOFT, PANEL, RAISED, STAGE, STAGE_W, STATUS_H, TIMELINE_H, TOOLBAR_H};

/// The notice strip's height.
const NOTICE_H: f32 = 34.0;
/// The side padding of the toolbar, the status bar and the notice strip.
const PAD_X: i8 = 10;
/// The side padding of the bout bar.
const BOUT_PAD_X: i8 = 16;
/// What the notice says when the PC gives the app no folder for its files.
pub const NO_FOLDER: &str =
    "This PC gave the app no folder for its settings, so changes are kept only until it closes.";
/// The most actions `log` holds.
pub const LOG_CAP: usize = 10_000;
/// How many of the newest actions `log` keeps when it reaches `LOG_CAP`.
pub const LOG_KEEP: usize = 5_000;

/// What the app starts with, made in the creator passed to `eframe::run_native` (where the window's context
/// exists, so the engine's waker can wake it).
pub struct Boot {
    pub engine: EngineHandle,
    /// Where settings and the queue are saved; `None` when the PC gives the app no folder for them.
    pub writer: Option<Writer>,
    pub audio: Box<dyn AudioOut>,
    pub settings: Settings,
    pub queue: Queue,
    /// What loading the settings and the queue had to say.
    pub notices: Vec<Notice>,
    /// The PC's cores.
    pub cores: usize,
}

/// Where the shell's regions were drawn in the last frame, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLayout {
    pub toolbar: Rect,
    pub bout: Rect,
    pub workspace: Rect,
    pub stage: Rect,
    pub timeline: Rect,
    pub status: Rect,
}

impl ShellLayout {
    /// Before the first frame: nowhere.
    const NONE: ShellLayout = ShellLayout {
        toolbar: Rect::NOTHING,
        bout: Rect::NOTHING,
        workspace: Rect::NOTHING,
        stage: Rect::NOTHING,
        timeline: Rect::NOTHING,
        status: Rect::NOTHING,
    };
}

/// The app's links to the world outside the interface.
enum Link {
    /// Paints the window's ground and nothing else (the harness's own checks).
    Blank,
    /// Screen tests: commands are kept for `sent`, player and store effects are dropped.
    Showing { sent: Vec<Command> },
    /// The running app.
    Live(Box<Live>),
}

/// What the running app talks to. The player lives as long as the app (dropping it joins its threads).
struct Live {
    engine: EngineHandle,
    writer: Option<Writer>,
    player: Player,
    /// Whether the preview was playing when last looked at, so `PreviewBusy` is sent when it changes.
    playing: bool,
}

/// The running app.
pub struct BfApp {
    pub state: AppState,
    /// Where the regions were drawn in the last frame.
    pub layout: ShellLayout,
    /// Every action the screens pushed, in order, since the app was built. In a long session it never grows past
    /// `LOG_CAP`: at that size the oldest are dropped, keeping the newest `LOG_KEEP`.
    pub log: Vec<Action>,
    /// The longest one `logic` call or one action's dispatch has taken, in milliseconds.
    pub slowest_update_ms: f32,
    /// A note the status bar's right side shows in place of the roster's counts (`--fixtures`: "Sample data, not a
    /// real fight"). `None` shows the counts. It is not the status line, which `AppState::status` works out.
    pub status_note: Option<String>,
    link: Link,
    /// Effects waiting for the first `logic` (the start-up ones).
    pending: Vec<Effect>,
    /// The preview's one texture, 360 x 640, uploaded again for every picture the player gives.
    texture: Option<TextureHandle>,
    ctx: egui::Context,
    /// `shutdown` has run.
    closed: bool,
    /// `close` has run: whether the engine stopped in time.
    engine_stopped: Option<bool>,
}

/// Which engine the app runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind {
    /// The real engine (`engine::spawn`): real fights, sound and videos.
    Real,
    /// The sample engine (`seam::fake`, at a real engine's pace): `--fixtures`.
    Sample,
}

/// How long the app's exit waits for the engine to stop: the engine's own wait for its threads, and a moment more.
pub const ENGINE_STOP: Duration = Duration::from_millis(STOP_WAIT.as_millis() as u64 + 500);

/// Where the real engine keeps its shortlists when the PC gives the app no folder of its own.
fn fallback_paths() -> Paths {
    Paths::under(&std::env::temp_dir().join("BALLFIGHT Studio"))
}

/// What the real engine is started with: the settings' workers, the app's storage `paths` (or, when the PC gives the
/// app none, a folder under the system temp folder for the shortlists), every frame of a video and no limit on a
/// search.
pub fn real_config(settings: &Settings, paths: Option<Paths>) -> EngineConfig {
    EngineConfig {
        workers: settings.workers,
        paths: paths.unwrap_or_else(fallback_paths),
        clip: None,
        search_limit: None,
    }
}

/// Builds the real app in the creator passed to `eframe::run_native`: the settings and the queue, read once
/// through `paths` (the same `paths` the writer is started with), their notices kept to be shown, the engine
/// `kind` names woken through the window's context (the real one with the settings' workers, the same `paths`, every
/// frame of a video and no limit on a search), and the sound output that `open_audio` gives (it may wait up to 3 s
/// for the device here, once). `BfApp::new` then installs the theme: the one call on this path. The state's first
/// effects ask the engine for the roster, ffmpeg and the output folder.
pub fn start(
    cc: &eframe::CreationContext<'_>,
    paths: Option<Paths>,
    open_audio: impl FnOnce() -> Box<dyn AudioOut>,
    kind: EngineKind,
) -> BfApp {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let mut notices = Vec::new();
    let (settings, queue, writer) = match paths.clone() {
        Some(paths) => {
            let (settings, settings_notice) = load_settings(&paths.settings(), cores);
            let (queue, queue_notice) = load_queue(&paths.queue());
            notices.extend(settings_notice);
            notices.extend(queue_notice);
            (settings, queue, Some(Writer::start(paths)))
        }
        None => {
            notices.push(Notice {
                text: NO_FOLDER.to_string(),
            });
            (default_settings(cores), Queue::new(), None)
        }
    };
    let engine = match kind {
        EngineKind::Real => engine::spawn(waker(&cc.egui_ctx), real_config(&settings, paths)),
        EngineKind::Sample => fake::spawn(waker(&cc.egui_ctx), Pace::Live),
    };
    let boot = Boot {
        engine,
        writer,
        audio: open_audio(),
        settings,
        queue,
        notices,
        cores,
    };
    BfApp::new(cc, boot)
}

/// Has the window draw with DirectX 12 alone on a PC that has a DirectX 12 graphics card, keeping wgpu's default
/// set of backends only on one that has none, or when `WGPU_BACKEND` names the backends (for tracking a graphics
/// problem down). Called on `NativeOptions::wgpu_options` before `eframe::run_native`.
///
/// Why: wgpu's default set also starts Vulkan and OpenGL, and starting the Vulkan loader pulls every "implicit
/// layer" registered on the PC into the process: the overlay and capture hooks of programs such as Overwolf, OBS,
/// Steam and Epic, and with OpenGL more driver DLLs. The window never draws through them. Such a hook is the suspect
/// (not proven) for a rare crash while the UI test program closes (Task 24d), so the Studio keeps them out of its
/// process. The tests do the same in `tests/ui/common.rs`.
pub fn prefer_dx12(options: &mut eframe::egui_wgpu::WgpuConfiguration) {
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_setup {
        let default = setup.instance_descriptor.backends;
        setup.instance_descriptor.backends = backends(wgpu::Backends::from_env(), has_dx12_card, default);
    }
}

/// The backends to start: those `WGPU_BACKEND` names (`env`), else DirectX 12 alone when `has_dx12` says this PC
/// has a DirectX 12 graphics card, else `default`.
pub fn backends(
    env: Option<wgpu::Backends>,
    has_dx12: impl FnOnce() -> bool,
    default: wgpu::Backends,
) -> wgpu::Backends {
    match env {
        Some(chosen) => chosen,
        None if has_dx12() => wgpu::Backends::DX12,
        None => default,
    }
}

/// Whether DirectX 12 offers a graphics card on this PC. The software adapter (WARP) that Windows always offers does
/// not count: a PC with nothing better keeps the default set, which may find a card through Vulkan or OpenGL.
fn has_dx12_card() -> bool {
    if !cfg!(windows) {
        return false;
    }
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    ready(instance.enumerate_adapters(wgpu::Backends::DX12))
        .iter()
        .any(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu)
}

/// The output of `future`, which on a native backend is ready when first polled (wgpu lists the adapters before it
/// returns the future).
fn ready<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(out) = future.as_mut().poll(&mut cx) {
            return out;
        }
        std::thread::yield_now();
    }
}

/// Puts the calling thread (the interface's: `BfApp::new` runs on it) above the app's other threads on Windows, so
/// the preview's drawing threads, a render and its ffmpeg, which can keep every core busy together, never keep the
/// interface waiting for a turn on a core. It changes nothing else: the work threads still use every core the
/// interface leaves.
fn interface_first() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL,
        };
        // SAFETY: `GetCurrentThread` returns a pseudo handle that is always valid for the calling thread and never
        // needs closing; `SetThreadPriority` only reads it. A refusal leaves the priority as it was.
        unsafe {
            SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
        }
    }
}
/// Asks the window to repaint, from any thread: `logic` then runs even while the window is minimised.
fn waker(ctx: &egui::Context) -> Waker {
    let ctx = ctx.clone();
    Arc::new(move || ctx.request_repaint())
}

impl BfApp {
    /// The app on `boot`: the theme installed, the state made, the player started with the settings' `workers` as
    /// its threads. The state's start-up effects are carried out by the first `logic`.
    pub fn new(cc: &eframe::CreationContext<'_>, boot: Boot) -> BfApp {
        interface_first();
        theme::install(&cc.egui_ctx);
        let workers = boot.settings.workers;
        let (state, pending) = AppState::new(boot.settings, boot.queue, boot.notices, boot.cores);
        let started = Instant::now();
        let now: Now = Box::new(move || started.elapsed());
        let player = Player::new(workers, boot.audio, now, waker(&cc.egui_ctx));
        let live = Live {
            engine: boot.engine,
            writer: boot.writer,
            player,
            playing: false,
        };
        BfApp::with(cc, state, Link::Live(Box::new(live)), pending)
    }

    /// The shell showing `vm` as given, for screen tests. Actions are dispatched, so the state changes as it
    /// would, but their effects are not carried out: commands are kept for `sent`, player and store effects are
    /// dropped, and the view model's transport stays what it was given. The preview has a picture only once
    /// `show_frame` gives it one.
    pub fn showing(cc: &eframe::CreationContext<'_>, vm: ViewModel) -> BfApp {
        theme::install(&cc.egui_ctx);
        let link = Link::Showing { sent: Vec::new() };
        BfApp::with(cc, AppState::showing(vm), link, Vec::new())
    }

    /// An app that only paints the window's ground.
    pub fn blank(cc: &eframe::CreationContext<'_>) -> BfApp {
        let settings = Settings {
            output_folder: std::path::PathBuf::new(),
            workers: 1,
        };
        let (state, _) = AppState::new(settings, Queue::new(), Vec::new(), 1);
        BfApp::with(cc, state, Link::Blank, Vec::new())
    }

    fn with(cc: &eframe::CreationContext<'_>, state: AppState, link: Link, pending: Vec<Effect>) -> BfApp {
        BfApp {
            state,
            layout: ShellLayout::NONE,
            log: Vec::new(),
            slowest_update_ms: 0.0,
            status_note: None,
            link,
            pending,
            texture: None,
            ctx: cc.egui_ctx.clone(),
            closed: false,
            engine_stopped: None,
        }
    }

    /// In `showing` mode: the commands sent since the last call. Empty in any other mode.
    pub fn sent(&mut self) -> Vec<Command> {
        match &mut self.link {
            Link::Showing { sent } => std::mem::take(sent),
            _ => Vec::new(),
        }
    }

    /// Loads `frame` into the preview texture (one upload, nearest sampling). A frame whose pixels do not fill it
    /// is ignored.
    pub fn show_frame(&mut self, frame: Frame) {
        let (w, h) = (frame.w as usize, frame.h as usize);
        if w == 0 || h == 0 || frame.rgb.len() != w * h * 3 {
            return;
        }
        let image = egui::ColorImage::from_rgb([w, h], &frame.rgb);
        match &mut self.texture {
            Some(texture) => texture.set(image, TextureOptions::NEAREST),
            None => self.texture = Some(self.ctx.load_texture("preview", image, TextureOptions::NEAREST)),
        }
    }

    /// Takes one thing the owner did, as a screen pushing it would: dispatches it, logs it and carries out its
    /// effects.
    pub fn act(&mut self, action: Action) {
        let began = Instant::now();
        let effects = self.state.dispatch(action.clone());
        carry_out(&mut self.link, effects);
        if self.log.len() >= LOG_CAP {
            self.log.drain(..LOG_CAP - LOG_KEEP);
        }
        self.log.push(action);
        self.note(began);
    }

    /// The app's exit (`on_exit`): [`BfApp::shutdown`], then a wait of up to [`ENGINE_STOP`] for the engine to
    /// stop, so a render, and the ffmpeg it started, never outlives the window. Returns whether the engine stopped
    /// in time (always true when the app has no engine). It does its work once; `Drop` calls it too.
    pub fn close(&mut self) -> bool {
        if let Some(stopped) = self.engine_stopped {
            return stopped;
        }
        self.shutdown();
        let stopped = match &self.link {
            Link::Live(live) => live.engine.stop(ENGINE_STOP),
            _ => true,
        };
        self.engine_stopped = Some(stopped);
        stopped
    }

    /// Tells the engine the app is closing and waits until everything asked to be saved is on disk. `close` calls
    /// it; it does its work once.
    pub fn shutdown(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        match &mut self.link {
            Link::Live(live) => {
                live.engine.send(Command::Shutdown);
                if let Some(writer) = &live.writer {
                    writer.flush();
                    for text in writer.problems() {
                        self.state.vm.notices.push(Notice { text });
                    }
                }
            }
            Link::Showing { sent } => sent.push(Command::Shutdown),
            Link::Blank => {}
        }
    }

    /// Whether the stage shows a picture now (a title card, or a frame of the take).
    pub fn has_picture(&self) -> bool {
        self.picture().is_some()
    }

    /// The picture to show: the texture once something is to be shown (the player says so; in `showing` mode once
    /// `show_frame` has given one).
    fn picture(&self) -> Option<TextureHandle> {
        let shown = match &self.link {
            Link::Live(live) => live.player.has_picture(),
            _ => true,
        };
        self.texture.clone().filter(|_| shown)
    }

    fn note(&mut self, began: Instant) {
        let ms = began.elapsed().as_secs_f32() * 1000.0;
        self.slowest_update_ms = self.slowest_update_ms.max(ms);
    }
}

/// Carries out `effects`, each once.
fn carry_out(link: &mut Link, effects: Vec<Effect>) {
    for effect in effects {
        match (&mut *link, effect) {
            (Link::Live(live), Effect::Send(command)) => live.engine.send(command),
            (Link::Live(live), Effect::Player(cmd)) => live.player.run(cmd),
            (Link::Live(live), Effect::SaveSettings(settings)) => {
                if let Some(writer) = &live.writer {
                    writer.settings(&settings);
                }
            }
            (Link::Live(live), Effect::SaveQueue(queue)) => {
                if let Some(writer) = &live.writer {
                    writer.queue(&queue);
                }
            }
            (Link::Showing { sent }, Effect::Send(command)) => sent.push(command),
            (Link::Showing { .. } | Link::Blank, _) => {}
        }
    }
}

/// The screen for `vm.view`.
fn workspace(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    match vm.view {
        View::Step(Step::Matchup) => matchup::show(ui, vm, out),
        View::Step(Step::Balance) => balance::show(ui, vm, out),
        View::Step(Step::Seeds) => seeds::show(ui, vm, out),
        View::Step(Step::Music) => music::show(ui, vm, out),
        View::Step(Step::Render) => render::show(ui, vm, out),
        View::Library => library::show(ui, vm, out),
        View::Settings => settings::show(ui, vm, out),
    }
}

/// A panel's fill with `pad_x` at each side.
fn fill(colour: Color32, pad_x: i8) -> Fill {
    Fill::new().fill(colour).inner_margin(Margin::symmetric(pad_x, 0))
}

/// The regions of section 4, each screen pushing what the owner does into `out`: the toolbar 40 and the bout bar 48
/// at the top, the status bar 24 and the timeline 112 at the bottom, the stage column 410 on the right, the notice
/// strip (34, only while there is a notice) above the workspace, and the workspace in what is left. Panels touch
/// with 1 px `LINE_SOFT` dividers, drawn inside the panel they belong to. `note` is the status bar's right-hand
/// note, if any (`BfApp::status_note`).
fn panels(
    ui: &mut Ui,
    vm: &ViewModel,
    note: Option<&str>,
    picture: Option<&TextureHandle>,
    out: &mut Vec<Action>,
) -> ShellLayout {
    let top = |id: &'static str, size: f32| {
        Panel::top(id)
            .exact_size(size)
            .resizable(false)
            .show_separator_line(false)
    };
    let bottom = |id: &'static str, size: f32| {
        Panel::bottom(id)
            .exact_size(size)
            .resizable(false)
            .show_separator_line(false)
    };
    let toolbar = top("toolbar", TOOLBAR_H)
        .frame(fill(PANEL, PAD_X))
        .show(ui, |ui| shell::toolbar(ui, vm, out))
        .response
        .rect;
    let bout = top("bout", BOUT_H)
        .frame(fill(PANEL, BOUT_PAD_X))
        .show(ui, |ui| shell::bout_bar(ui, vm))
        .response
        .rect;
    let status = bottom("status", STATUS_H)
        .frame(fill(GROUND, PAD_X))
        .show(ui, |ui| shell::status_bar(ui, vm, note))
        .response
        .rect;
    let timeline = bottom("timeline", TIMELINE_H)
        .frame(fill(PANEL, 0))
        .show(ui, |ui| timeline::show(ui, vm, out))
        .response
        .rect;
    let stage = Panel::right("stage")
        .exact_size(STAGE_W)
        .resizable(false)
        .show_separator_line(false)
        .frame(fill(STAGE, 0))
        .show(ui, |ui| stage::show(ui, vm, picture))
        .response
        .rect;
    let notice = (!vm.notices.is_empty()).then(|| {
        top("notice", NOTICE_H)
            .frame(fill(RAISED, PAD_X))
            .show(ui, |ui| shell::notice(ui, vm, out))
            .response
            .rect
    });
    let workspace = CentralPanel::default()
        .frame(fill(PANEL, 0))
        .show(ui, |ui| workspace(ui, vm, out))
        .response
        .rect;

    let painter = ui.painter();
    let line = Stroke::new(1.0, LINE_SOFT);
    for below in [Some(toolbar), Some(bout), notice].into_iter().flatten() {
        painter.hline(below.x_range(), below.bottom() - 0.5, line);
    }
    painter.hline(timeline.x_range(), timeline.top() + 0.5, line);
    painter.vline(stage.left() + 0.5, stage.y_range(), line);

    ShellLayout {
        toolbar,
        bout,
        workspace,
        stage,
        timeline,
        status,
    }
}

impl Drop for BfApp {
    /// A dropped app closes as its window would (`close`, which does its work once): also when the interface thread
    /// panicked or the window ended without `on_exit`, the engine is stopped, and waited for, before the process
    /// exits.
    fn drop(&mut self) {
        self.close();
    }
}

impl eframe::App for BfApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let began = Instant::now();
        let pending = std::mem::take(&mut self.pending);
        carry_out(&mut self.link, pending);
        if let Link::Live(live) = &mut self.link {
            let events: Vec<_> = std::iter::from_fn(|| live.engine.poll()).collect();
            for event in events {
                let effects = self.state.apply(event);
                carry_out(&mut self.link, effects);
            }
        }
        if let Link::Live(live) = &mut self.link {
            let picture = live.player.tick();
            let view = live.player.view();
            if view.playing != live.playing {
                live.playing = view.playing;
                live.engine.send(Command::PreviewBusy(view.playing));
            }
            self.state.player_view(view, live.player.failed_frame());
            if let Some(writer) = &live.writer {
                for text in writer.problems() {
                    self.state.vm.notices.push(Notice { text });
                }
            }
            if live.player.wants_repaint() {
                ctx.request_repaint();
            }
            if let Some(frame) = picture {
                self.show_frame(frame);
            }
        }
        self.note(began);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        if matches!(self.link, Link::Blank) {
            ui.painter().rect_filled(ui.max_rect(), 0.0, GROUND);
            return;
        }
        let mut out = Vec::new();
        shell::keys(ui.ctx(), &mut out);
        let picture = self.picture();
        let note = self.status_note.as_deref();
        self.layout = panels(ui, &self.state.vm, note, picture.as_ref(), &mut out);
        if !out.is_empty() {
            // The frame was drawn from the state before these actions: draw the next one from after.
            ui.ctx().request_repaint();
        }
        for action in out {
            self.act(action);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        GROUND.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_real_engine_renders_whole_videos_with_the_settings_workers() {
        let settings = Settings {
            output_folder: std::path::PathBuf::from("Videos"),
            workers: 7,
        };
        let paths = Paths::under(std::path::Path::new("studio data"));
        let config = real_config(&settings, Some(paths.clone()));
        assert_eq!(
            config,
            EngineConfig {
                workers: 7,
                paths,
                clip: None,
                search_limit: None,
            }
        );
        // No folder from the PC: the shortlists go under the temp folder.
        let config = real_config(&settings, None);
        assert_eq!(
            config.paths,
            Paths::under(&std::env::temp_dir().join("BALLFIGHT Studio"))
        );
        assert_eq!((config.clip, config.search_limit, config.workers), (None, None, 7));
    }

    #[cfg(windows)]
    #[test]
    fn the_interface_thread_comes_first() {
        use windows_sys::Win32::System::Threading::{
            GetCurrentThread, GetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL, THREAD_PRIORITY_NORMAL,
        };
        let priority = || {
            // SAFETY: the calling thread's pseudo handle is always valid and only read.
            unsafe { GetThreadPriority(GetCurrentThread()) }
        };
        let (before, after) = std::thread::spawn(move || {
            let before = priority();
            interface_first();
            (before, priority())
        })
        .join()
        .unwrap();
        assert_eq!(before, THREAD_PRIORITY_NORMAL);
        assert_eq!(after, THREAD_PRIORITY_ABOVE_NORMAL);
    }
    #[test]
    fn dx12_alone_where_the_pc_has_a_dx12_card() {
        let default = wgpu::Backends::PRIMARY | wgpu::Backends::GL;
        assert_eq!(backends(None, || true, default), wgpu::Backends::DX12);
        assert_eq!(
            backends(None, || false, default),
            default,
            "no DirectX 12 card: the default set"
        );
        let named = wgpu::Backends::VULKAN;
        let never = || -> bool { panic!("WGPU_BACKEND wins without a look at the cards") };
        assert_eq!(backends(Some(named), never, default), named);
    }
}
