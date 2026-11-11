//! The frame every screen sits in (interface reference, section 4): the toolbar, the bout bar, the notice strip, the
//! status bar, and the keys that switch steps. The app (`app.rs`) places them in their panels.

use egui::accesskit::Role;
use egui::{Align, Context, Key, Label, Layout, Modifiers, Response, RichText, Sense, Ui, WidgetInfo, pos2, vec2};

use crate::model::{Step, View};
use crate::state::{Action, ViewModel};
use crate::theme::{CONTROL_H, HOVER, MUTED, RADIUS, TEXT, body, section, small, value};
use crate::widgets::buttons::{self, focus_ring};
use crate::widgets::chips::fighter_bar;
use crate::widgets::pixel::pixel_text;
use crate::wording;

/// The five steps as the toolbar names them, with the key that goes to each.
pub const STEPS: [(Step, &str, Key); 5] = [
    (Step::Matchup, "1 Match-up", Key::Num1),
    (Step::Balance, "2 Balance", Key::Num2),
    (Step::Seeds, "3 Seeds", Key::Num3),
    (Step::Music, "4 Music", Key::Num4),
    (Step::Render, "5 Render", Key::Num5),
];

/// What hovering the disabled "Update" button says.
pub const UPDATE_LATER: &str = "Updates from inside the app arrive in a later version.";

/// Side padding of a toolbar button's label.
const PAD_X: f32 = 10.0;
/// The gap between "Queue" and its count.
const COUNT_GAP: f32 = 4.0;
/// The gap between two label and value pairs of the bout bar, less the 8 every item gets.
const PAIR_GAP: f32 = 28.0 - 8.0;

/// The keys of section 7 that the shell owns: 1 to 5 go to the five steps. They are taken from the widgets
/// (`consume_key`), so nothing else acts on them too, and left alone while any widget has the keyboard
/// (`egui_wants_keyboard_input`: a text field being typed in, a slider moved with its keys), so typing "3" into a
/// caption types it. A number with Ctrl held is not a step key.
pub fn keys(ctx: &Context, out: &mut Vec<Action>) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    for (step, _, key) in STEPS {
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
            out.push(Action::Go(View::Step(step)));
        }
    }
}

/// The toolbar: the app's name, the five steps (the current one inverted, also while the Library or Settings is
/// open), then on the right "Library", "Queue" with the count of videos rendering and waiting, "Update" (disabled
/// until updates arrive) and "Settings". "Library" and "Settings" are inverted while their view is open.
pub fn toolbar(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        plain(ui, RichText::new("BALLFIGHT Studio").font(section()).color(TEXT));
        ui.add_space(16.0 - 2.0);
        for (step, label, _) in STEPS {
            if buttons::toggle(ui, label, vm.step == step).clicked() {
                out.push(Action::Go(View::Step(step)));
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if buttons::toggle(ui, "Settings", vm.view == View::Settings).clicked() {
                out.push(Action::Go(View::Settings));
            }
            buttons::disabled(ui, "Update", UPDATE_LATER);
            let (rendering, waiting) = vm.queue.counts();
            if queue_button(ui, rendering + waiting).clicked() {
                out.push(Action::Go(View::Step(Step::Render)));
            }
            if buttons::toggle(ui, "Library", vm.view == View::Library).clicked() {
                out.push(Action::Go(View::Library));
            }
        });
    });
}

/// The flat "Queue" button, followed by `count` in mono `MUTED` when it is above 0; named "Queue" or "Queue 2".
fn queue_button(ui: &mut Ui, count: usize) -> Response {
    let painter = ui.painter();
    let word = painter.layout_no_wrap("Queue".to_owned(), body(), TEXT);
    let number = (count > 0).then(|| painter.layout_no_wrap(count.to_string(), value(), MUTED));
    let number_w = number.as_ref().map_or(0.0, |n| COUNT_GAP + n.size().x);
    let size = vec2((2.0 * PAD_X + word.size().x + number_w).ceil(), CONTROL_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let name = if count > 0 {
        format!("Queue {count}")
    } else {
        "Queue".to_string()
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, &name));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if response.hovered() {
            painter.rect_filled(rect, RADIUS, HOVER);
        }
        let top = |h: f32| (rect.center().y - h / 2.0).round();
        let x = rect.min.x + PAD_X;
        let word_w = word.size().x;
        painter.galley(pos2(x.round(), top(word.size().y)), word, TEXT);
        if let Some(number) = number {
            let at = pos2((x + word_w + COUNT_GAP).round(), top(number.size().y));
            painter.galley(at, number, MUTED);
        }
        if response.has_focus() {
            focus_ring(painter, rect);
        }
    }
    response
}

/// The bout bar: each fighter's bar and name in pixel letters with "VS" between, then the draft's theme, tempo,
/// handicap (on Seeds, Music and Render only) and seed. Empty until the roster arrives.
pub fn bout_bar(ui: &mut Ui, vm: &ViewModel) {
    let Some(roster) = &vm.roster else {
        return;
    };
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let names = vm.names();
        let colors = vm.colors();
        fighter_bar(ui, colors[0], 24.0);
        if let Some(fighter) = vm.fighter(0) {
            pixel_text(ui, &fighter.mask, 3, TEXT, &names[0]);
        }
        pixel_text(ui, &roster.vs, 2, TEXT.gamma_multiply(0.6), "VS");
        if let Some(fighter) = vm.fighter(1) {
            pixel_text(ui, &fighter.mask, 3, TEXT, &names[1]);
        }
        fighter_bar(ui, colors[1], 24.0);

        // The pairs are laid from the right, so the last one ends at the padding.
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let draft = &vm.draft;
            match draft.seed {
                Some(seed) => plain(ui, RichText::new(seed.to_string()).font(value()).color(TEXT)),
                None => plain(ui, RichText::new("not picked yet").font(body()).color(MUTED)),
            }
            label(ui, "Seed");
            if matches!(vm.view, View::Step(Step::Seeds | Step::Music | Step::Render)) {
                ui.add_space(PAIR_GAP);
                let [left, right] = draft.handicap.map(wording::times);
                plain(ui, RichText::new(format!("{left} / {right}")).font(value()).color(TEXT));
                label(ui, "Handicap");
            }
            ui.add_space(PAIR_GAP);
            let tempo = format!("{:.2} bpm", draft.tempo_bpm());
            plain(ui, RichText::new(tempo).font(value()).color(TEXT));
            label(ui, "Tempo");
            if let Some(theme) = vm.theme() {
                ui.add_space(PAIR_GAP);
                plain(ui, RichText::new(&theme.display).font(body()).color(TEXT));
                label(ui, "Theme");
            }
        });
    });
}

/// A bout bar label: body, `MUTED`.
fn label(ui: &mut Ui, text: &str) {
    plain(ui, RichText::new(text).font(body()).color(MUTED));
}

/// Text that is read, not clicked or selected.
fn plain(ui: &mut Ui, text: RichText) {
    ui.add(Label::new(text).selectable(false));
}

/// The notice strip: the newest notice in body `TEXT` (cut short with an ellipsis when it does not fit, the whole
/// of it on hover), and a box button "Dismiss" that removes it. Nothing when there is no notice.
pub fn notice(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let Some(newest) = vm.notices.len().checked_sub(1) else {
        return;
    };
    ui.horizontal_centered(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if buttons::boxed(ui, "Dismiss").clicked() {
                out.push(Action::Dismiss(newest));
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let text = RichText::new(&vm.notices[newest].text).font(body()).color(TEXT);
                ui.add(Label::new(text).selectable(false).truncate());
            });
        });
    });
}

/// The status bar: what the app is doing or last did on the left, in small `MUTED` (cut short with an ellipsis
/// when it does not fit); on the right the fighters that can be picked and the themes, once the roster is here.
pub fn status_bar(ui: &mut Ui, vm: &ViewModel) {
    ui.horizontal_centered(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(roster) = &vm.roster {
                let fighters = roster.fighters.iter().filter(|f| f.available).count() as u64;
                let themes = roster.themes.len() as u64;
                let counts = format!(
                    "{}, {}",
                    wording::plural(fighters, "fighter", "fighters"),
                    wording::plural(themes, "theme", "themes")
                );
                plain(ui, RichText::new(counts).font(small()).color(MUTED));
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let status = RichText::new(&vm.status).font(small()).color(MUTED);
                ui.add(Label::new(status).selectable(false).truncate());
            });
        });
    });
}
