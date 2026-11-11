//! The app's look: the design tokens (`tokens`, re-exported here), the built-in fonts and the egui style made from
//! them. `install` puts it all into a context.

pub mod fonts;
pub mod tokens;
pub mod visuals;

pub use tokens::*;

/// Puts the app's fonts, visuals, spacing and text styles into `ctx`, for every egui theme, so the look does not
/// change with the PC's light or dark setting.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts::definitions());
    ctx.all_styles_mut(visuals::apply);
}
