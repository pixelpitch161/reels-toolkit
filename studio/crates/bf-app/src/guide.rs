//! `--guide-pictures <folder>`: the guide's pictures, taken from the real app in its real window.
//!
//! The app runs on the real engine and is driven through [`Action`]s, never through the mouse. Each picture waits for
//! its state on the view model, with a time limit, then a few frames more so the window has settled, and is saved
//! from the window's own pixels (`ViewportCommand::Screenshot`, so Windows' own fonts are in it). The window is 1440 x
//! 900 pixels: the zoom is set so one point is one pixel whatever the PC's display scale.
//!
//! The fight is the guide's own match-up, [`LEFT`] against [`RIGHT`]; the seed is the first row the search finds
//! whose take passes every check (never a fixed seed). The Library, Settings, the update row and the stopped job are
//! the screen fixtures (`fixtures::library`, `fixtures::settings`, `fixtures::render`), shown in the same window:
//! the real run leaves no finished video, and the fixtures name no folder of this PC.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use egui::{ColorImage, Id, Rect};

use crate::app::BfApp;
use crate::fixtures;
use crate::model::queue::JobState;
use crate::model::take::TransportAction;
use crate::model::{Step, View};
use crate::state::{Action, AppState, COMPOSED, QueueAction};
use crate::update::UpdateState;

/// The guide's match-up: the left corner.
pub const LEFT: &str = "FLUTE";
/// The guide's match-up: the right corner.
pub const RIGHT: &str = "TROMBONE";
/// The window's size in pixels.
pub const SIZE: (f32, f32) = (1440.0, 900.0);
/// The mark the Settings screen leaves on its "Updates" row.
pub const UPDATES_ROW: &str = "updates row";
/// The mark the Render screen leaves on a stopped job's row.
pub const FAILED_ROW: &str = "failed row";
/// How many rows of the shortlist are tried for a take that passes every check.
const TRIES: usize = 8;
/// Frames painted after a state is reached and before its picture is taken.
const SETTLE: u32 = 30;
/// Room left around a cropped row, in pixels.
const CROP_ROOM: f32 = 8.0;

/// Leaves a mark on `rect` under `name`, for a picture cropped to it.
pub fn mark(ctx: &egui::Context, name: &'static str, rect: Rect) {
    ctx.data_mut(|d| d.insert_temp(Id::unique(("guide mark", name)), rect));
}

fn marked(ctx: &egui::Context, name: &'static str) -> Option<Rect> {
    ctx.data(|d| d.get_temp::<Rect>(Id::unique(("guide mark", name))))
}

/// The pictures in the order they are taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shot {
    Matchup,
    Balance,
    Seeds,
    Music,
    Render,
    Library,
    Settings,
    UpdateReady,
    FailedJob,
}

const SHOTS: [Shot; 9] = [
    Shot::Matchup,
    Shot::Balance,
    Shot::Seeds,
    Shot::Music,
    Shot::Render,
    Shot::Library,
    Shot::Settings,
    Shot::UpdateReady,
    Shot::FailedJob,
];

impl Shot {
    fn file(self) -> &'static str {
        match self {
            Shot::Matchup => "1-matchup.png",
            Shot::Balance => "2-balance.png",
            Shot::Seeds => "3-seeds.png",
            Shot::Music => "4-music.png",
            Shot::Render => "5-render.png",
            Shot::Library => "library.png",
            Shot::Settings => "settings.png",
            Shot::UpdateReady => "update-ready.png",
            Shot::FailedJob => "failed-job.png",
        }
    }

    /// How long its state may take to come.
    fn limit(self) -> Duration {
        Duration::from_secs(match self {
            Shot::Matchup => 60,
            Shot::Balance => 900,
            Shot::Seeds => 300,
            Shot::Music => 900,
            Shot::Render => 300,
            _ => 30,
        })
    }

    fn crop(self) -> Option<&'static str> {
        match self {
            Shot::UpdateReady => Some(UPDATES_ROW),
            Shot::FailedJob => Some(FAILED_ROW),
            _ => None,
        }
    }
}

/// Where a picture stands.
enum Phase {
    /// Its first actions are still to be sent.
    Start,
    /// Waiting for its state, since `.0`.
    Waiting(Instant),
    /// The state is there: frames painted since.
    Settling(u32),
    /// The picture was asked for.
    Taking,
}

/// The app, driven to each picture in turn. What went wrong, if anything, is put in `outcome`.
pub struct Pictures {
    app: BfApp,
    folder: PathBuf,
    at: usize,
    phase: Phase,
    /// The shortlist row whose take is being tried.
    row: usize,
    zoomed: bool,
    /// "Create folder" was pressed.
    asked_folder: bool,
    image: Arc<Mutex<Option<Arc<ColorImage>>>>,
    outcome: Arc<Mutex<Option<String>>>,
    closing: bool,
}

impl Pictures {
    pub fn new(app: BfApp, folder: PathBuf, outcome: Arc<Mutex<Option<String>>>) -> Pictures {
        Pictures {
            app,
            folder,
            at: 0,
            phase: Phase::Start,
            row: 0,
            zoomed: false,
            asked_folder: false,
            image: Arc::default(),
            outcome,
            closing: false,
        }
    }

    fn act(&mut self, action: Action) {
        self.app.act(action);
    }

    fn stop(&mut self, ctx: &egui::Context, problem: Option<String>) {
        if let Some(p) = &problem {
            eprintln!("The guide pictures stopped: {p}");
        }
        *self.outcome.lock().unwrap_or_else(PoisonError::into_inner) = problem;
        self.closing = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// The seed of the shortlist's row `i`, best first.
    fn row_seed(&self, i: usize) -> Option<u64> {
        self.app.state.vm.shortlist.rows.get(i).map(|r| r.seed)
    }

    /// The first actions of `shot`.
    fn begin(&mut self, shot: Shot) {
        match shot {
            Shot::Matchup => {}
            Shot::Balance => self.act(Action::Go(View::Step(Step::Balance))),
            Shot::Seeds => self.act(Action::Go(View::Step(Step::Seeds))),
            Shot::Music => {
                if self.app.state.vm.transport.playing {
                    self.act(Action::Transport(TransportAction::PlayPause));
                }
                self.act(Action::UseSeed);
            }
            Shot::Render => {
                self.act(Action::Go(View::Step(Step::Render)));
            }
            Shot::Library => {
                if let Some(id) = self.app.state.vm.queue.rendering() {
                    self.act(Action::Queue(QueueAction::Cancel(id)));
                }
                self.app.state = AppState::showing(fixtures::library::vm());
            }
            Shot::Settings => {
                self.app.state = AppState::showing(fixtures::settings::with_update(UpdateState::UpToDate));
            }
            Shot::UpdateReady => {
                self.app.state = AppState::showing(fixtures::settings::with_update(UpdateState::Available {
                    version: "0.2.0".to_string(),
                }));
            }
            Shot::FailedJob => self.app.state = AppState::showing(fixtures::render::vm()),
        }
    }

    /// Whether the state of `shot` is there; sends what moves it on when it is not.
    fn ready(&mut self, shot: Shot) -> bool {
        let vm = &self.app.state.vm;
        match shot {
            Shot::Matchup => {
                if vm.roster.is_none() {
                    return false;
                }
                let wanted = [LEFT.to_string(), RIGHT.to_string()];
                if vm.draft.fighters != wanted {
                    let picks: Vec<usize> = (0..2).filter(|&s| vm.draft.fighters[s] != wanted[s]).collect();
                    for side in picks {
                        self.act(Action::PickFighter {
                            side,
                            name: wanted[side].clone(),
                        });
                    }
                    return false;
                }
                self.app.has_picture()
            }
            Shot::Balance => {
                let b = &vm.balance;
                if b.running.is_some() {
                    return false;
                }
                match &b.stats {
                    Some(s) if s.in_range() => true,
                    Some(_) => {
                        self.act(Action::AutoBalance);
                        false
                    }
                    None => false,
                }
            }
            Shot::Seeds => {
                let Some(seed) = self.row_seed(0) else {
                    return false;
                };
                let vm = &self.app.state.vm;
                if vm.draft.seed != Some(seed) {
                    self.act(Action::SelectSeed(seed));
                    return false;
                }
                if vm.take.loading || !vm.transport.enabled {
                    return false;
                }
                if !vm.transport.playing {
                    self.act(Action::Transport(TransportAction::PlayPause));
                    return false;
                }
                vm.transport.frame > 120
            }
            Shot::Music => {
                // The guide shows Composed, the arrangement new match-ups start on.
                if vm.view == View::Step(Step::Music) && vm.draft.arrangement.as_deref() != Some(COMPOSED) {
                    self.act(Action::SetArrangement(COMPOSED.to_string()));
                    return false;
                }
                let Some(music) = &vm.take.music else {
                    return false;
                };
                if vm.take.loading || music.checks.is_empty() || vm.view != View::Step(Step::Music) {
                    return false;
                }
                if music.checks.iter().all(|c| c.pass) {
                    return true;
                }
                // A fight that fits the story can still fail a check: the next row is tried.
                self.row += 1;
                match self.row_seed(self.row).filter(|_| self.row < TRIES) {
                    Some(seed) => {
                        self.act(Action::SelectSeed(seed));
                        self.act(Action::UseSeed);
                    }
                    None => self.phase = Phase::Waiting(Instant::now() - shot.limit()),
                }
                false
            }
            Shot::Render if vm.queue.jobs.is_empty() => {
                // Added once the screen's checks (the folder, ffmpeg) have answered. The data folder's `Videos` is
                // made as the owner would: "Create folder".
                if vm.folder == crate::model::library::FolderState::Missing && !self.asked_folder {
                    self.asked_folder = true;
                    self.act(Action::CreateFolder);
                } else if crate::state::queue_run::why_not_add(vm).is_none() {
                    self.act(Action::AddToQueue);
                }
                false
            }
            Shot::Render => vm.queue.jobs.iter().any(|j| match j.state {
                JobState::Rendering { done, .. } => done > 60,
                _ => false,
            }),
            Shot::Library | Shot::Settings | Shot::UpdateReady | Shot::FailedJob => true,
        }
    }

    /// Saves `image` for `shot`, cropped to its mark when it has one.
    fn keep(&self, ctx: &egui::Context, shot: Shot, image: &ColorImage) -> Result<(), String> {
        let [w, h] = image.size;
        let (mut x0, mut y0, mut x1, mut y1) = (0, 0, w, h);
        if let Some(name) = shot.crop() {
            let rect = marked(ctx, name).ok_or_else(|| format!("no mark {name:?} for {}", shot.file()))?;
            let ppp = ctx.pixels_per_point();
            let at = |v: f32, most: usize| ((v * ppp).round().max(0.0) as usize).min(most);
            x0 = at(rect.min.x - CROP_ROOM, w);
            y0 = at(rect.min.y - CROP_ROOM, h);
            x1 = at(rect.max.x + CROP_ROOM, w);
            y1 = at(rect.max.y + CROP_ROOM, h);
        }
        let mut rgb = Vec::with_capacity((x1 - x0) * (y1 - y0) * 3);
        for y in y0..y1 {
            for p in &image.pixels[y * w + x0..y * w + x1] {
                rgb.extend_from_slice(&[p.r(), p.g(), p.b()]);
            }
        }
        write_png(&self.folder.join(shot.file()), (x1 - x0) as u32, (y1 - y0) as u32, &rgb)
    }
}

fn write_png(path: &Path, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgb).map_err(|e| e.to_string())
}

impl eframe::App for Pictures {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
        let ctx = ui.ctx().clone();
        ctx.request_repaint();
        if self.closing {
            return;
        }
        // One point is one pixel, and the window 1440 x 900 of them.
        if !self.zoomed {
            self.zoomed = true;
            if let Some(native) = ctx.native_pixels_per_point() {
                ctx.set_zoom_factor(1.0 / native);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(SIZE.0, SIZE.1)));
        }
        let Some(&shot) = SHOTS.get(self.at) else {
            self.stop(&ctx, None);
            return;
        };
        match self.phase {
            Phase::Start => {
                self.begin(shot);
                self.phase = Phase::Waiting(Instant::now());
            }
            Phase::Waiting(since) => {
                if self.ready(shot) {
                    self.phase = Phase::Settling(0);
                } else if since.elapsed() > shot.limit() {
                    let vm = &self.app.state.vm;
                    let problem = format!(
                        "waited {} s for {} (the status bar said {:?}; adding to the queue: {:?})",
                        shot.limit().as_secs(),
                        shot.file(),
                        vm.status,
                        crate::state::queue_run::why_not_add(vm),
                    );
                    self.stop(&ctx, Some(problem));
                }
            }
            Phase::Settling(n) if n < SETTLE => self.phase = Phase::Settling(n + 1),
            Phase::Settling(_) => {
                let slot = Arc::clone(&self.image);
                ctx.request_screenshot(move |image| {
                    *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(image);
                });
                self.phase = Phase::Taking;
            }
            Phase::Taking => {
                let image = self.image.lock().unwrap_or_else(PoisonError::into_inner).take();
                if let Some(image) = image {
                    match self.keep(&ctx, shot, &image) {
                        Ok(()) => {
                            eprintln!("saved {} ({} x {})", shot.file(), image.size[0], image.size[1]);
                            self.at += 1;
                            self.phase = Phase::Start;
                        }
                        Err(problem) => self.stop(&ctx, Some(problem)),
                    }
                }
            }
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color(visuals)
    }

    fn on_exit(&mut self) {
        self.app.on_exit();
    }
}
