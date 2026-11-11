//! The app as eframe runs it: events in (`logic`), paint (`ui`), actions out.

/// The window's ground colour, #1e1e1e (the theme's `ground`).
const GROUND: egui::Color32 = egui::Color32::from_rgb(0x1e, 0x1e, 0x1e);

/// The running app.
pub struct BfApp {}

impl BfApp {
    /// An app that only paints the window's ground.
    pub fn blank(_cc: &eframe::CreationContext<'_>) -> BfApp {
        BfApp {}
    }
}

impl eframe::App for BfApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.painter().rect_filled(ui.max_rect(), 0.0, GROUND);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        GROUND.to_normalized_gamma_f32()
    }
}
