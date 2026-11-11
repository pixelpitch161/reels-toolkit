//! What every interface test shares: the one-at-a-time drawing lock and the harness builders.

use std::sync::{Mutex, MutexGuard};

use bf_app::app::BfApp;
use egui_kittest::Harness;

/// The reference window, in points (one pixel per point in tests).
pub const REF: (f32, f32) = (1440.0, 900.0);
/// The smallest window the app supports.
pub const MIN: (f32, f32) = (1280.0, 800.0);

static GPU: Mutex<()> = Mutex::new(());

/// Tests that draw with wgpu run one at a time: two software-adapter devices drawing at once crash the test process
/// on Windows. Every such test holds this guard for its whole length. A test that panicked while holding it does
/// not stop the others, so poisoning is ignored.
pub fn gpu() -> MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The whole app in a window of `size`: edge to edge, one pixel per point, no cursor drawn, drawn with wgpu.
pub fn app_harness<'a>(
    size: (f32, f32),
    make: impl FnOnce(&mut eframe::CreationContext<'a>) -> BfApp,
) -> Harness<'a, BfApp> {
    Harness::builder()
        .with_size(egui::Vec2::new(size.0, size.1))
        .with_pixels_per_point(1.0)
        .with_render_cursor(false)
        .wgpu()
        .build_eframe(make)
}

/// One piece of interface in a window of `size`, with the app's theme installed, drawn the same way.
pub fn widget_harness<'a>(size: (f32, f32), ui: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
    Harness::builder()
        .with_size(egui::Vec2::new(size.0, size.1))
        .with_pixels_per_point(1.0)
        .with_render_cursor(false)
        .wgpu()
        .with_context_setup(bf_app::theme::install)
        .build_ui(ui)
}

/// Renders the harness once and reads the colour of each pixel at `at` (x, y), alpha dropped.
pub fn pixels<S>(harness: &mut Harness<'_, S>, at: &[(u32, u32)]) -> Vec<[u8; 3]> {
    let image = harness.render().expect("the harness renders");
    at.iter()
        .map(|&(x, y)| {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            [r, g, b]
        })
        .collect()
}
