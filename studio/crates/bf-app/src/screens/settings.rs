//! The Settings view (spec section 4): one column 560 wide, padded 28, of four rows 16 apart, each a `MUTED` label
//! 150 wide and what it sets or says:
//! - "Output folder": its full path in a read field (cut short when it does not fit, all of it on hover) and the box
//!   button "Change folder" (`ChangeFolder`);
//! - "Worker threads": a slider from 1 to the PC's cores in whole steps and a value field (`SetWorkers`, which the
//!   state also keeps within the cores), with what the number means under them;
//! - "Video encoder": the engine's line for the ffmpeg that was found (it says where from), or what to do when
//!   none was or the app's own copy is broken;
//! - "Version": the app's version, and that the Update button works in a later version.
//!
//! The view reads the view model and pushes actions; it keeps nothing of its own beyond egui's widget memory.

use egui::{Align, Label, Layout, Rect, RichText, Ui, UiBuilder, vec2};

use crate::state::{Action, ViewModel};
use crate::theme::{CONTROL_H, MUTED, TEXT, VALUE_W, body, small};
use crate::widgets::buttons;
use crate::widgets::fields::{Unit, read_field, value_field};
use crate::widgets::slider::bar_slider;
use crate::wording;

/// What the encoder row says when ffmpeg was not found.
pub const FFMPEG_MISSING: &str =
    "ffmpeg was not found. Install ffmpeg, then start the app again. A later version brings its own.";
/// What it says until the engine has looked.
pub const LOOKING_FOR_FFMPEG: &str = "Looking for ffmpeg.";

/// The column's width and padding, the gap between rows and the label column's width.
const COLUMN_W: f32 = 560.0;
const PAD: f32 = 28.0;
const ROW_GAP: f32 = 16.0;
const LABEL_W: f32 = 150.0;
/// The gap between the controls of a row, and between the worker controls and the line under them.
const ITEM_GAP: f32 = 8.0;
const NOTE_GAP: f32 = 4.0;
/// How far a line of body text sits below the top of a 26 tall row, so it lines up with the row's label.
const TEXT_TOP: f32 = 4.0;
/// A button's side padding, as the buttons widget pads its label.
const BUTTON_PAD_X: f32 = 10.0;
/// How the worker count is written: a whole number.
const WHOLE: Unit<'static> = Unit {
    prefix: "",
    suffix: "",
    decimals: 0,
};

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let inner = ui.max_rect().shrink(PAD);
    let column = Rect::from_min_size(inner.min, vec2(COLUMN_W.min(inner.width()).max(0.0), inner.height()));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("settings column")
            .max_rect(column)
            .layout(Layout::top_down(Align::Min)),
    );
    let ui = &mut child;
    ui.spacing_mut().item_spacing.y = ROW_GAP;
    row(ui, "Output folder", |ui| output_folder(ui, vm, out));
    row(ui, "Worker threads", |ui| worker_threads(ui, vm, out));
    row(ui, "Video encoder", |ui| {
        let text = match &vm.ffmpeg {
            None => LOOKING_FOR_FFMPEG.to_string(),
            // the engine's line already says where it came from ("..., comes with the app")
            Some(Ok(line)) => line.clone(),
            // a broken copy of the app's own: the export's sentence, which says to install the app again
            Some(Err(said)) if bf_export::is_about_the_apps_own(said) => said.clone(),
            Some(Err(_)) => FFMPEG_MISSING.to_string(),
        };
        words(ui, text);
    });
    row(ui, "Version", |ui| {
        let version = vm
            .roster
            .as_ref()
            .map_or(env!("CARGO_PKG_VERSION"), |r| r.app_version.as_str());
        words(
            ui,
            format!("{version}. The Update button starts working in a later version."),
        );
    });
}

/// One row: `label` (body `MUTED`) in a cell 150 wide and 26 tall, then `add` in the rest of the column, laid out
/// from the top.
fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    let width = ui.available_width();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.allocate_ui_with_layout(vec2(LABEL_W, CONTROL_H), Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_size(vec2(LABEL_W, CONTROL_H));
            ui.add(
                Label::new(RichText::new(label).font(body()).color(MUTED))
                    .selectable(false)
                    .truncate(),
            );
        });
        let rest = (width - LABEL_W).max(0.0);
        ui.allocate_ui_with_layout(vec2(rest, CONTROL_H), Layout::top_down(Align::Min), |ui| {
            ui.set_width(rest);
            ui.spacing_mut().item_spacing = vec2(ITEM_GAP, NOTE_GAP);
            add(ui);
        });
    });
}

/// Body `TEXT` that wraps at the row's width, its first line level with the row's label.
fn words(ui: &mut Ui, text: String) {
    ui.add_space(TEXT_TOP);
    ui.add(
        Label::new(RichText::new(text).font(body()).color(TEXT))
            .wrap()
            .selectable(false),
    );
}

/// The output folder's full path in a read field, and "Change folder" at the column's right edge.
fn output_folder(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let change = "Change folder";
    let button_w = (ui.painter().layout_no_wrap(change.to_owned(), body(), TEXT).size().x + 2.0 * BUTTON_PAD_X).ceil();
    let field_w = (ui.available_width() - button_w - ITEM_GAP).floor();
    ui.horizontal(|ui| {
        let path = vm.settings.output_folder.display().to_string();
        read_field(ui, "Output folder", &path, field_w);
        if buttons::boxed(ui, change).clicked() {
            out.push(Action::ChangeFolder);
        }
    });
}

/// The worker slider (1 to the cores, whole steps) and its value field, then what the number means. A change pushes
/// `SetWorkers` with the new whole number.
fn worker_threads(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let cores = vm.cores.max(1);
    let range = (1.0, cores as f64);
    let current = vm.settings.workers;
    let slider_w = (ui.available_width() - VALUE_W - ITEM_GAP).floor();
    ui.horizontal(|ui| {
        let mut on_slider = current as f64;
        let moved = bar_slider(ui, "Worker threads", &mut on_slider, range, 1.0, slider_w);
        let mut in_field = current as f64;
        value_field(ui, "Worker threads value", &mut in_field, range, WHOLE, VALUE_W);
        // Only what the person did is pushed: a count saved on a PC with more cores is shown at this PC's end of the
        // range, and left as it is until they change it.
        let wanted = if moved.changed {
            Some(on_slider)
        } else {
            (in_field != current as f64).then_some(in_field)
        };
        if let Some(wanted) = wanted.map(|n| n.round().clamp(range.0, range.1) as usize)
            && wanted != current
        {
            out.push(Action::SetWorkers(wanted));
        }
    });
    let note = format!(
        "One core stays free for the preview. This PC has {}.",
        wording::plural(cores as u64, "core", "cores")
    );
    ui.add(
        Label::new(RichText::new(note).font(small()).color(MUTED))
            .wrap()
            .selectable(false),
    );
}
