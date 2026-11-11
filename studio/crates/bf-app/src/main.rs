//! BALLFIGHT Studio.
//!
//! - `--fixtures` runs the whole interface on the sample engine (`seam::fake`, at a real engine's pace) with real
//!   sound out, its files in a temp folder of its own (removed when the window has closed), and the status bar's
//!   right side reading "Sample data, not a real fight".
//! - Without it the window says that the engine is not connected in this build (Task 35 connects it).
//! - `--data-dir <folder>` keeps the settings, the queue and the cache in `<folder>` instead (created if missing; a
//!   folder that cannot be used stops the app with one plain sentence and exit code 2).
//! - `--smoke` opens the window, paints 30 frames, closes it and exits with code 0; it keeps its files in a temp
//!   folder of its own and has no sound. With `--fixtures` it runs the sample interface that way.
//!
//! At start the app writes one line to stderr naming the graphics adapter the window draws with.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use bf_app::app;
use bf_app::preview::audio::{NO_DEVICE, NoAudio, open_audio};
use bf_app::store::Paths;
use bf_app::theme::{self, GROUND, TEXT, body};
use egui::{Label, RichText};

/// How many painted frames `--smoke` waits for before it closes the window.
const SMOKE_FRAMES: u32 = 30;
/// The status bar's right side in `--fixtures`.
const SAMPLE_NOTE: &str = "Sample data, not a real fight";
/// What the window says without `--fixtures`, until the real engine is behind the seam (Task 35).
const NOT_CONNECTED: &str = "The engine is not connected in this build. Start it with --fixtures to see the interface.";

/// What the command line asks for.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    smoke: bool,
    fixtures: bool,
    data_dir: Option<PathBuf>,
}

/// Reads the command line (without the program's own name). A word it does not know, or `--data-dir` with no
/// folder after it, is an error in one plain sentence.
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--smoke") => parsed.smoke = true,
            Some("--fixtures") => parsed.fixtures = true,
            Some("--data-dir") => match args.next() {
                // An option in its place ("--data-dir --fixtures") is not a folder.
                Some(dir) if !dir.is_empty() && !dir.to_string_lossy().starts_with("--") => {
                    parsed.data_dir = Some(PathBuf::from(dir));
                }
                _ => return Err("--data-dir needs a folder after it.".to_string()),
            },
            _ => {
                return Err(format!(
                    "BALLFIGHT Studio does not know the option {}.",
                    arg.to_string_lossy()
                ));
            }
        }
    }
    Ok(parsed)
}

/// Makes sure the app can keep its files in `dir`: creates it when missing and checks that a file can be written
/// there. Says why not in one plain sentence.
fn usable(dir: &Path) -> Result<(), String> {
    let refuse = || {
        format!(
            "BALLFIGHT Studio cannot keep its files in {}, so it did not start.",
            dir.display()
        )
    };
    std::fs::create_dir_all(dir).map_err(|_| refuse())?;
    if !dir.is_dir() {
        return Err(refuse());
    }
    let probe = dir.join(format!(".bf-write-check-{}", std::process::id()));
    std::fs::write(&probe, b"").map_err(|_| refuse())?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

/// Runs `app` and closes its window after `SMOKE_FRAMES` painted frames.
struct Smoke {
    app: Box<dyn eframe::App>,
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

/// The window without an engine: one centred line saying so, on the window's ground.
struct NotConnected;

impl eframe::App for NotConnected {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter().rect_filled(ui.max_rect(), 0.0, GROUND);
        ui.centered_and_justified(|ui| {
            ui.add(Label::new(RichText::new(NOT_CONNECTED).font(body()).color(TEXT)).selectable(false));
        });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        GROUND.to_normalized_gamma_f32()
    }
}

fn main() -> std::process::ExitCode {
    let args = match parse(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(problem) => {
            eprintln!("{problem}");
            return std::process::ExitCode::from(2);
        }
    };
    if let Some(dir) = &args.data_dir
        && let Err(problem) = usable(dir)
    {
        eprintln!("{problem}");
        return std::process::ExitCode::from(2);
    }
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
    // A smoke or fixtures run never touches the owner's own settings folder: its files go in a folder of its own
    // under the system temp folder, removed when the window has closed. `--data-dir` names the folder instead.
    let temp_root = (args.data_dir.is_none() && (args.smoke || args.fixtures)).then(|| {
        let kind = if args.smoke { "smoke" } else { "fixtures" };
        std::env::temp_dir().join(format!("bf-{kind}-{}", std::process::id()))
    });
    let paths = match (&args.data_dir, &temp_root) {
        (Some(dir), _) | (None, Some(dir)) => Some(Paths::under(dir)),
        (None, None) => Paths::system(),
    };
    let Args { smoke, fixtures, .. } = args;
    let result = eframe::run_native(
        "BALLFIGHT Studio",
        options,
        Box::new(move |cc| {
            if let Some(render) = &cc.wgpu_render_state {
                let info = render.adapter.get_info();
                eprintln!(
                    "BALLFIGHT Studio draws with {} ({:?}, {:?})",
                    info.name, info.backend, info.device_type
                );
            }
            // The theme is installed, and the settings and the queue are read, here, before the first frame; the
            // sample engine is made here too, where the window's context exists for its waker.
            let inner: Box<dyn eframe::App> = if fixtures {
                let mut app = if smoke {
                    app::start(cc, paths, || Box::new(NoAudio::new(NO_DEVICE)))
                } else {
                    app::start(cc, paths, open_audio)
                };
                app.status_note = Some(SAMPLE_NOTE.to_string());
                Box::new(app)
            } else {
                theme::install(&cc.egui_ctx);
                Box::new(NotConnected)
            };
            let app: Box<dyn eframe::App> = if smoke {
                Box::new(Smoke { app: inner, painted: 0 })
            } else {
                inner
            };
            Ok(app)
        }),
    );
    if let Some(root) = temp_root {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &[&str]) -> Vec<OsString> {
        line.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_command_line() {
        assert_eq!(parse(words(&[])), Ok(Args::default()));
        assert_eq!(
            parse(words(&["--fixtures", "--smoke"])),
            Ok(Args {
                smoke: true,
                fixtures: true,
                data_dir: None
            })
        );
        assert_eq!(
            parse(words(&["--data-dir", "D:\\Studio files", "--fixtures"])),
            Ok(Args {
                smoke: false,
                fixtures: true,
                data_dir: Some(PathBuf::from("D:\\Studio files"))
            })
        );
        assert_eq!(
            parse(words(&["--data-dir"])),
            Err("--data-dir needs a folder after it.".to_string())
        );
        assert_eq!(
            parse(words(&["--data-dir", "--fixtures"])),
            Err("--data-dir needs a folder after it.".to_string())
        );
        assert_eq!(
            parse(words(&["--data-dir", ""])),
            Err("--data-dir needs a folder after it.".to_string())
        );
        assert_eq!(
            parse(words(&["--fixture"])),
            Err("BALLFIGHT Studio does not know the option --fixture.".to_string())
        );
    }

    #[test]
    fn a_data_folder_is_made_or_refused_in_words() {
        let root = std::env::temp_dir().join(format!("bf-app data-dir {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("made").join("for the studio");
        assert_eq!(usable(&dir), Ok(()));
        assert!(dir.is_dir(), "a missing folder is created");
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            0,
            "the write check leaves nothing behind"
        );
        // A file where the folder should be cannot be used.
        let file = root.join("a file");
        std::fs::write(&file, b"not a folder").unwrap();
        let problem = usable(&file).expect_err("a file is not a folder");
        assert_eq!(
            problem,
            format!(
                "BALLFIGHT Studio cannot keep its files in {}, so it did not start.",
                file.display()
            )
        );
        assert_eq!(problem.matches(". ").count(), 0, "one sentence");
        let _ = std::fs::remove_dir_all(&root);
    }
}
