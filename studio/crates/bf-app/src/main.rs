//! BALLFIGHT Studio. `--smoke` opens the window, paints 30 frames, closes it and exits with code 0.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bf_app::app::BfApp;

/// How many painted frames `--smoke` waits for before it closes the window.
const SMOKE_FRAMES: u32 = 30;

/// Runs the app and closes its window after `SMOKE_FRAMES` painted frames.
struct Smoke {
    app: BfApp,
    painted: u32,
}

impl eframe::App for Smoke {
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
}

fn main() -> std::process::ExitCode {
    let smoke = std::env::args().skip(1).any(|arg| arg == "--smoke");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BALLFIGHT Studio")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "BALLFIGHT Studio",
        options,
        Box::new(move |cc| {
            let app = BfApp::blank(cc);
            let app: Box<dyn eframe::App> = if smoke {
                Box::new(Smoke { app, painted: 0 })
            } else {
                Box::new(app)
            };
            Ok(app)
        }),
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("BALLFIGHT Studio could not open its window: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}
