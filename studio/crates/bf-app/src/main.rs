//! BALLFIGHT Studio. `--smoke` opens the window, paints 30 frames, closes it and exits with code 0; it keeps its
//! files in a temp folder of its own and has no sound.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bf_app::app::{self, BfApp};
use bf_app::preview::audio::{NO_DEVICE, NoAudio, open_audio};
use bf_app::store::Paths;

/// How many painted frames `--smoke` waits for before it closes the window.
const SMOKE_FRAMES: u32 = 30;

/// Runs the app and closes its window after `SMOKE_FRAMES` painted frames.
struct Smoke {
    app: BfApp,
    painted: u32,
}

impl eframe::App for Smoke {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
        self.painted += 1;
        if self.painted >= SMOKE_FRAMES {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ui.ctx().request_repaint();
        }
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color(visuals)
    }

    fn on_exit(&mut self) {
        self.app.on_exit();
    }
}

fn main() -> std::process::ExitCode {
    let smoke = std::env::args().skip(1).any(|arg| arg == "--smoke");
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BALLFIGHT Studio")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    // DirectX 12 alone where the PC has a DirectX 12 card, so overlay and capture hooks that ride on Vulkan and
    // OpenGL stay out of the Studio (see `app::prefer_dx12`).
    app::prefer_dx12(&mut options.wgpu_options);
    // A smoke run never touches the owner's own settings folder or sound device: its files go in a folder of its
    // own under the system temp folder, removed when the window has closed, and it has no sound.
    let smoke_root = smoke.then(|| std::env::temp_dir().join(format!("bf-smoke-{}", std::process::id())));
    let paths = match &smoke_root {
        Some(root) => Some(Paths::under(root)),
        None => Paths::system(),
    };
    let result = eframe::run_native(
        "BALLFIGHT Studio",
        options,
        Box::new(move |cc| {
            // The theme is installed, and the settings and the queue are read, here, before the first frame.
            let app: Box<dyn eframe::App> = if smoke {
                let app = app::start(cc, paths, || Box::new(NoAudio::new(NO_DEVICE)));
                Box::new(Smoke { app, painted: 0 })
            } else {
                Box::new(app::start(cc, paths, open_audio))
            };
            Ok(app)
        }),
    );
    if let Some(root) = smoke_root {
        let _ = std::fs::remove_dir_all(root);
    }
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("BALLFIGHT Studio could not open its window: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
