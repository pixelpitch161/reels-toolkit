//! The Settings view (spec section 4; plan M6, Task 13): one column 560 wide, padded 28, of rows 16 apart, each a
//! `MUTED` label 150 wide and what it sets or says. The column scrolls when the window is too short for it.
//! - "Output folder": its full path in a read field (cut short when it does not fit, all of it on hover) and the box
//!   button "Change folder" (`ChangeFolder`);
//! - "Worker threads": a slider from 1 to the PC's cores in whole steps and a value field (`SetWorkers`, which the
//!   state also keeps within the cores), with what the number means under them;
//! - "Video encoding": the segments "Best quality" and "Faster" (`SetEncode`), and what they mean;
//! - "Video encoder": the engine's line for the ffmpeg that was found (it says where from), or what to do when
//!   none was or the app's own copy is broken;
//! - "Version": the app's version, commit and date (`vm.app_line`);
//! - "Updates": where the update stands, with the one button that moves it on, the kept version's button and the
//!   last request's note (`Upkeep(..)`);
//! - "Problems": where fault reports are saved, and "Open the reports folder" (`OpenReports`);
//! - "Guide": "Open the guide" (`Upkeep(OpenGuide)`);
//! - "Remove": "Remove BALLFIGHT Studio from this PC" (`AskRemove`), which asks first: what would be removed and
//!   what is kept, then "Remove the app" (`Upkeep(Remove)`) and "Keep it" (`KeepApp`, where the keyboard starts).
//!   After a removal, its words and "Close BALLFIGHT Studio".
//!
//! The view reads the view model and pushes actions; it keeps nothing of its own beyond egui's widget memory (and
//! whether the removal question has had its first frame, so the keyboard starts on "Keep it" once).

use egui::{Align, Frame, Label, Layout, Margin, Response, RichText, ScrollArea, Ui, ViewportCommand, vec2};

use crate::model::EncodeChoice;
use crate::state::upkeep::{
    self, CHECKING, LOOKING_AT_REMOVAL, NOT_INSTALLED, REMOVING, RemovalView, UP_TO_DATE, why_not_go_back,
    why_not_remove, why_not_restart,
};
use crate::state::{Action, ViewModel};
use crate::theme::{CONTROL_H, MUTED, TEXT, VALUE_W, body, small};
use crate::update::UpdateState;
use crate::update::worker::Request;
use crate::widgets::buttons;
use crate::widgets::fields::{Unit, read_field, value_field};
use crate::widgets::slider::bar_slider;
use crate::wording;

/// What the encoder row says when ffmpeg was not found.
pub const FFMPEG_MISSING: &str =
    "ffmpeg was not found in the app's folder, so videos cannot be saved. Ask for the app to be installed again.";
/// What it says until the engine has looked.
pub const LOOKING_FOR_FFMPEG: &str = "Looking for ffmpeg.";
/// What the two encoder settings mean (no saving in seconds is promised: it was measured on a busy PC).
pub const ENCODING_NOTE: &str = "Best quality is the series' own setting. Faster makes the video sooner and the file \
                                 about a quarter larger. A change applies to the next video that starts.";
/// The "Problems" row when the PC gave the app no folder for its files (so none for its reports either).
pub const REPORTS_SOMEWHERE: &str = "If something goes wrong, a report is saved in the app's logs folder.";
/// Why "Open the reports folder" cannot be used then.
pub const NO_REPORTS_FOLDER: &str = "This copy does not know where its reports folder is.";
/// Why "Open the guide" cannot be used in a copy without one.
pub const NO_GUIDE: &str = "This copy has no guide.";

/// "If something goes wrong, a report is saved in {logs}."
pub fn reports_line(logs: &std::path::Path) -> String {
    format!("If something goes wrong, a report is saved in {}.", logs.display())
}

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
/// The "Video encoding" segments' width.
const ENCODING_W: f32 = 240.0;
/// A segment's height, as the buttons widget draws it.
const SEGMENT_H: f32 = 24.0;
/// How the worker count is written: a whole number.
const WHOLE: Unit<'static> = Unit {
    prefix: "",
    suffix: "",
    decimals: 0,
};

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let area = ScrollArea::vertical()
        .id_salt("settings column")
        .auto_shrink([false, false]);
    buttons::scrolled(ui, area, |ui| {
        Frame::new().inner_margin(Margin::same(PAD as i8)).show(ui, |ui| {
            let width = COLUMN_W.min(ui.available_width()).max(0.0);
            ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = ROW_GAP;
                column(ui, vm, out);
            });
        });
    });
}

/// The rows, top to bottom. Once the app is to close, every row but "Remove" (which then holds the closing words and
/// "Close BALLFIGHT Studio") is shown disabled.
fn column(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    ui.add_enabled_ui(!upkeep::closing(vm), |ui| settings_rows(ui, vm, out));
    row(ui, "Remove", |ui| remove(ui, vm, out));
}

/// Every row above "Remove".
fn settings_rows(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    row(ui, "Output folder", |ui| output_folder(ui, vm, out));
    row(ui, "Worker threads", |ui| worker_threads(ui, vm, out));
    row(ui, "Video encoding", |ui| encoding(ui, vm, out));
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
    row(ui, "Version", |ui| words(ui, vm.app_line.clone()));
    row(ui, "Updates", |ui| updates(ui, vm, out));
    row(ui, "Problems", |ui| problems(ui, vm, out));
    row(ui, "Guide", |ui| {
        let opened = match vm.update.guide {
            Some(_) => buttons::boxed(ui, "Open the guide"),
            None => buttons::disabled(ui, "Open the guide", NO_GUIDE),
        };
        if seen(opened).clicked() {
            out.push(Action::Upkeep(Request::OpenGuide));
        }
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

/// Small `MUTED` text that wraps at the row's width.
fn note(ui: &mut Ui, text: &str) {
    ui.add(
        Label::new(RichText::new(text).font(small()).color(MUTED))
            .wrap()
            .selectable(false),
    );
}

/// `response`, scrolled into view when the keyboard has just come to it (a row below the fold of a short window).
fn seen(response: Response) -> Response {
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
    response
}

/// A row of buttons under a row's words.
fn buttons_row<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = ITEM_GAP;
        add(ui)
    })
    .inner
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
    note(
        ui,
        &format!(
            "One core stays free for the preview. This PC has {}.",
            wording::plural(cores as u64, "core", "cores")
        ),
    );
}

/// "Best quality" and "Faster", the chosen one inverted, then what they mean. A click on the other pushes
/// `SetEncode`.
fn encoding(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    ui.add_space((CONTROL_H - SEGMENT_H) / 2.0);
    let options: Vec<(&str, Option<&str>)> = EncodeChoice::ALL.iter().map(|c| (c.label(), None)).collect();
    let chosen = EncodeChoice::ALL.iter().position(|c| *c == vm.settings.encode);
    let width = ENCODING_W.min(ui.available_width());
    if let Some(i) = buttons::segmented(ui, "Video encoding", &options, chosen, width)
        && Some(i) != chosen
    {
        out.push(Action::SetEncode(EncodeChoice::ALL[i]));
    }
    note(ui, ENCODING_NOTE);
}

/// Where the update stands, and the one button that moves it on; then the kept version's button and the note.
fn updates(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let u = &vm.update;
    let (text, button): (String, Option<(&str, Request)>) = match &u.state {
        UpdateState::NotInstalled => (NOT_INSTALLED.to_string(), None),
        UpdateState::Checking => (CHECKING.to_string(), None),
        UpdateState::UpToDate => (UP_TO_DATE.to_string(), Some(("Look again", Request::Check))),
        UpdateState::Available { version } => (upkeep::available(version), None),
        UpdateState::Building { version, seconds, .. } => (
            upkeep::building(version, *seconds),
            Some(("Stop building", Request::StopBuild)),
        ),
        UpdateState::Ready { version } => (upkeep::ready(version), None),
        UpdateState::Failed { text } => (text.clone(), Some(("Try again", Request::Check))),
    };
    words(ui, text);
    if let UpdateState::Building { doing, .. } = &u.state
        && !doing.is_empty()
    {
        note(ui, doing);
    }
    let build = match &u.state {
        UpdateState::Available { version } => Some(format!("Build version {version}")),
        _ => None,
    };
    let installed = u.state != UpdateState::NotInstalled;
    // Once the app is to close, nothing here moves it on: no button is shown.
    let has_buttons = !upkeep::closing(vm)
        && (button.is_some()
            || build.is_some()
            || matches!(u.state, UpdateState::Ready { .. })
            || (installed && u.previous_button.is_some()));
    if has_buttons {
        buttons_row(ui, |ui| {
            if let Some(label) = &build
                && seen(buttons::primary(ui, label)).clicked()
            {
                out.push(Action::Upkeep(Request::Build));
            }
            if matches!(u.state, UpdateState::Ready { .. }) {
                let restart = match why_not_restart(vm) {
                    None => buttons::primary(ui, "Restart now"),
                    Some(why) => buttons::disabled(ui, "Restart now", why),
                };
                if seen(restart).clicked() {
                    out.push(Action::Upkeep(Request::Restart));
                }
            }
            if let Some((label, request)) = button
                && seen(buttons::boxed(ui, label)).clicked()
            {
                out.push(Action::Upkeep(request));
            }
            if installed && let Some(label) = &u.previous_button {
                let back = match why_not_go_back(vm) {
                    None => buttons::boxed(ui, label),
                    Some(why) => buttons::disabled(ui, label, why),
                };
                if seen(back).clicked() {
                    out.push(Action::Upkeep(Request::GoBack));
                }
            }
        });
    }
    if let Some(said) = &u.note {
        note(ui, said);
    }
}

/// Where fault reports are saved, and the button that opens that folder.
fn problems(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let label = "Open the reports folder";
    let open = match &vm.update.reports {
        Some(logs) => {
            words(ui, reports_line(logs));
            seen(buttons::boxed(ui, label))
        }
        None => {
            words(ui, REPORTS_SOMEWHERE.to_string());
            seen(buttons::disabled(ui, label, NO_REPORTS_FOLDER))
        }
    };
    if open.clicked() {
        out.push(Action::OpenReports);
    }
}

/// The "Remove" row: the button, the question, the removal on its way, or what was removed.
fn remove(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    // Whether the question has had its first frame, so the keyboard starts on "Keep it" once.
    let asked_key = ui.scope_id().with("removal question shown");
    let asking = matches!(vm.removal, RemovalView::Asking { .. });
    if !asking {
        ui.data_mut(|d| d.insert_temp(asked_key, false));
    }
    if let Some(said) = vm.update.close.as_ref().filter(|w| !w.is_empty()) {
        for line in said {
            words(ui, line.clone());
        }
        if seen(buttons::primary(ui, "Close BALLFIGHT Studio")).clicked() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
        return;
    }
    let keep = |ui: &mut Ui, out: &mut Vec<Action>| {
        let r = seen(buttons::boxed(ui, "Keep it"));
        if r.clicked() {
            out.push(Action::KeepApp);
        }
        r
    };
    match &vm.removal {
        RemovalView::Closed => {
            let ask = match why_not_remove(vm) {
                None => buttons::boxed(ui, "Remove BALLFIGHT Studio from this PC"),
                Some(why) => buttons::disabled(ui, "Remove BALLFIGHT Studio from this PC", why),
            };
            if seen(ask).clicked() {
                out.push(Action::AskRemove);
            }
        }
        RemovalView::Looking => {
            words(ui, LOOKING_AT_REMOVAL.to_string());
            keep(ui, out);
        }
        RemovalView::Refused(text) => {
            words(ui, text.clone());
            keep(ui, out);
        }
        RemovalView::Asking { words: lines, problem } => {
            for line in lines {
                words(ui, line.clone());
            }
            // Why the last "Remove the app" did not remove it.
            if let Some(why) = problem {
                words(ui, why.clone());
            }
            buttons_row(ui, |ui| {
                let go = match why_not_remove(vm) {
                    None => buttons::primary(ui, "Remove the app"),
                    Some(why) => buttons::disabled(ui, "Remove the app", why),
                };
                if seen(go).clicked() {
                    out.push(Action::Upkeep(Request::Remove));
                }
                let kept = keep(ui, out);
                if !ui.data(|d| d.get_temp::<bool>(asked_key).unwrap_or(false)) {
                    kept.request_focus();
                    kept.scroll_to_me(None);
                    ui.data_mut(|d| d.insert_temp(asked_key, true));
                }
            });
        }
        RemovalView::Removing { .. } => words(ui, REMOVING.to_string()),
    }
}
