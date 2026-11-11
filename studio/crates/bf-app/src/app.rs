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
use std::time::Instant;

use egui::{CentralPanel, Color32, Frame as Fill, Margin, Panel, Rect, Stroke, TextureHandle, TextureOptions, Ui};

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
    link: Link,
    /// Effects waiting for the first `logic` (the start-up ones).
    pending: Vec<Effect>,
    /// The preview's one texture, 360 x 640, uploaded again for every picture the player gives.
    texture: Option<TextureHandle>,
    ctx: egui::Context,
    /// `shutdown` has run.
    closed: bool,
}

/// Builds the real app in the creator passed to `eframe::run_native`: the settings and the queue, read once
/// through `paths` (the same `paths` the writer is started with), their notices kept to be shown, the sample engine
/// (until the real one is behind the seam) woken through the window's context, and the sound output that
/// `open_audio` gives (it may wait up to 3 s for the device here, once). `BfApp::new` then installs the theme: the
/// one call on this path.
pub fn start(
    cc: &eframe::CreationContext<'_>,
    paths: Option<Paths>,
    open_audio: impl FnOnce() -> Box<dyn AudioOut>,
) -> BfApp {
    let cores = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let mut notices = Vec::new();
    let (settings, queue, writer) = match paths {
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
    let engine = fake::spawn(waker(&cc.egui_ctx), Pace::Live);
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

/// Asks the window to repaint, from any thread: `logic` then runs even while the window is minimised.
fn waker(ctx: &egui::Context) -> Waker {
    let ctx = ctx.clone();
    Arc::new(move || ctx.request_repaint())
}

impl BfApp {
    /// The app on `boot`: the theme installed, the state made, the player started with the settings' `workers` as
    /// its threads. The state's start-up effects are carried out by the first `logic`.
    pub fn new(cc: &eframe::CreationContext<'_>, boot: Boot) -> BfApp {
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
            link,
            pending,
            texture: None,
            ctx: cc.egui_ctx.clone(),
            closed: false,
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

    /// Tells the engine the app is closing and waits until everything asked to be saved is on disk. `on_exit`
    /// calls it; it does its work once.
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
/// with 1 px `LINE_SOFT` dividers, drawn inside the panel they belong to.
fn panels(ui: &mut Ui, vm: &ViewModel, picture: Option<&TextureHandle>, out: &mut Vec<Action>) -> ShellLayout {
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
        .show(ui, |ui| shell::status_bar(ui, vm))
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
        self.layout = panels(ui, &self.state.vm, picture.as_ref(), &mut out);
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
        self.shutdown();
    }
}
