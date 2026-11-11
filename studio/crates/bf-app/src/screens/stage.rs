//! The stage column (interface reference, section 4): the 9:16 preview centred in the column, and 8 below it one
//! `small` `MUTED` line saying what the preview shows.
//!
//! The picture is drawn at `scale::preview_size` of the column, less 24 at the bottom for the line, in physical
//! pixels: 360 x 640 at the reference window (whole-number scale), 310 x 552 at the minimum (a fractional scale, by
//! design). It sits on whole physical pixels and is sampled nearest, so a whole-number scale stays crisp. With the
//! safe zone on, the five rectangles outside the safe zone are shaded with `GROUND` at 60 % over it.
//!
//! Whether there is a picture is the shell's decision (`Player::has_picture()`): with none the column is the empty
//! stage and only the line is drawn. The texture belongs to the shell; nothing is uploaded here.

use egui::accesskit::Role;
use egui::{
    Align, Color32, Label, Layout, Rect, RichText, Sense, TextureHandle, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use crate::model::{Step, View};
use crate::preview::scale::{preview_size, shade};
use crate::state::ViewModel;
use crate::theme::{GROUND, MUTED, small};

/// The room under the picture: 8, then the 16 tall line.
const CAPTION_ROOM: f32 = 24.0;
/// The gap between the picture and the line.
const CAPTION_GAP: f32 = 8.0;
/// The line's side padding.
const PAD_X: f32 = 10.0;
/// The internal picture's size.
const PICTURE: (f32, f32) = (360.0, 640.0);
/// The shade over the outside of the safe zone: `GROUND` at 60 %.
const SHADE_ALPHA: f32 = 0.6;

/// Draws the column into `ui` from `vm`, with `picture` when the player has one to show.
pub fn show(ui: &mut Ui, vm: &ViewModel, picture: Option<&TextureHandle>) {
    let column = ui.max_rect();
    let ppp = ui.ctx().pixels_per_point();
    let snap = |v: f32| (v * ppp).round() / ppp;
    let area_h = (column.height() - CAPTION_ROOM).max(0.0);
    let (pw, ph) = preview_size((column.width() * ppp, area_h * ppp));
    let size = vec2(pw / ppp, ph / ppp);
    let min = pos2(
        snap(column.center().x - size.x / 2.0),
        snap(column.min.y + (area_h - size.y) / 2.0),
    );
    let rect = Rect::from_min_size(min, size);

    if let Some(texture) = picture {
        let response = ui.allocate_rect(rect, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Image, true, "Preview"));
        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            painter.image(texture.id(), rect, uv, Color32::WHITE);
            if vm.transport.safe_zone {
                let (sx, sy) = (rect.width() / PICTURE.0, rect.height() / PICTURE.1);
                let shade_colour = GROUND.gamma_multiply(SHADE_ALPHA);
                for [x0, y0, x1, y1] in shade() {
                    let at = |x: i32, y: i32| pos2(snap(rect.min.x + x as f32 * sx), snap(rect.min.y + y as f32 * sy));
                    painter.rect_filled(Rect::from_min_max(at(x0, y0), at(x1, y1)), 0.0, shade_colour);
                }
            }
        }
    }

    let line_top = rect.max.y + CAPTION_GAP;
    let cell = Rect::from_min_max(
        pos2(column.min.x + PAD_X, line_top),
        pos2(
            column.max.x - PAD_X,
            (line_top + CAPTION_ROOM - CAPTION_GAP).min(column.max.y),
        ),
    );
    let layout = Layout::top_down(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(cell).layout(layout), |ui| {
        let text = RichText::new(caption(vm)).font(small()).color(MUTED);
        ui.add(Label::new(text).selectable(false).truncate());
    });
}

/// The line under the preview: the problem, when there is one (why the pair cannot fight, else why the take cannot
/// be made, in the status line's order); "Drawing seed {n}" while a take loads; for a take, "Seed {n}, the real
/// picture and sound" on Seeds and Music and "{Left} vs {Right}, seed {n}" anywhere else; without one, "Title card,
/// also saved as the cover image".
pub fn caption(vm: &ViewModel) -> String {
    let take = &vm.take;
    if let Some(problem) = vm.fight_problem.as_ref().or(take.problem.as_ref()) {
        return problem.clone();
    }
    if take.loading
        && let Some(seed) = vm.draft.seed
    {
        return format!("Drawing seed {seed}");
    }
    let Some(story) = &take.story else {
        return "Title card, also saved as the cover image".to_string();
    };
    if matches!(vm.view, View::Step(Step::Seeds | Step::Music)) {
        format!("Seed {}, the real picture and sound", story.seed)
    } else {
        let [left, right] = vm.names();
        format!("{left} vs {right}, seed {}", story.seed)
    }
}
