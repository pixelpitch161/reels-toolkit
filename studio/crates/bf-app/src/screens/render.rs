//! The Render step's workspace (interface reference, section 5.5): "This fight" (the captions, the output folder and
//! "Add to queue") over the queue on the left, and the check list of the video shown on the right.
//!
//! The screen is drawn from the view model alone and pushes what the owner does; the only things it remembers are
//! egui's own: where the caption box and the queue are scrolled, which control has the keyboard, and a row being
//! dragged.
//!
//! - "Add to queue" asks `why_not_add` (Task 12): enabled exactly when it gives no reason; otherwise it is drawn
//!   disabled and the reason stands beside it, in words.
//! - The caption box scrolls inside its three lines. A character no font on this PC can draw (an emoji, on a PC
//!   without an emoji font) is shown as a stand-in of one character, so the text the owner edits keeps every
//!   character it had; only the picture of it differs.
//! - The queue's rows follow `Queue::shown()`: the one rendering, the waiting ones in line, the finished ones newest
//!   first. A waiting row can be dragged onto another waiting row, or moved from its right-click menu.
//! - The check list shows the selected job, else the one rendering.

use std::path::MAIN_SEPARATOR;

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Label, Layout, Margin, Painter, Popup, Rect, Response, RichText, ScrollArea, Sense, Shape, Stroke,
    StrokeKind, Style, TextBuffer, TextEdit, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use crate::model::library::FolderState;
use crate::model::queue::{FailHint, Job, JobState, STEP_ORDER, StepKey, StepLine, StepState};
use crate::model::{JobId, Platform, Step};
use crate::state::queue_run::why_not_add;
use crate::state::{Action, QueueAction, ViewModel};
use crate::theme::{
    CONTROL_H, FAINT, GROUND, HAIRLINE, LINE, LINE_SOFT, MUTED, ON_LIGHT, RADIUS, RADIUS_ROW, RAISED, TEXT,
    TEXT_STRONG, body, rgb, section, small, value as mono,
};
use crate::widgets::buttons;
use crate::widgets::chips::{fighter_bar, object_row};
use crate::widgets::fields::{self, ring_above};
use crate::widgets::icons::{self, Icon};
use crate::wording;

/// The check list's width.
pub const CHECKS_W: f32 = 330.0;

/// "This fight": its padding, the gap under its heading, the gap between its rows.
const FIGHT_PAD: f32 = 14.0;
const HEADING_GAP: f32 = 10.0;
const ROW_GAP: f32 = 6.0;
/// The width of the labels "Caption" and "Saves to", and where the lines under them start.
const LABEL_W: f32 = 70.0;
const INDENT: f32 = 78.0;
/// The caption tabs.
const SEGMENTS_W: f32 = 270.0;
/// The gap between the box's label and the box.
const BOX_LABEL_GAP: f32 = 4.0;
/// The caption box: three lines of body text, padded 8 at the sides and 6 above and below.
const BOX_LINES: f32 = 3.0;
const BOX_PAD: (f32, f32) = (8.0, 6.0);
/// The gap between small controls in one row, and between a label and its control.
const GAP: f32 = 4.0;
const LABEL_GAP: f32 = 8.0;
/// The queue's header.
const HEADER_H: f32 = 40.0;
const HEADER_PAD: f32 = 14.0;
/// The queue list's inset: the room the focus ring of a row takes (2 px gap, 2 px ring).
const RING_ROOM: f32 = 4.0;
/// A job row: its padding above and below, the gap between its parts, the thumbnail.
const JOB_PAD_Y: f32 = 8.0;
const JOB_GAP: f32 = 12.0;
const THUMB: (f32, f32) = (27.0, 48.0);
/// The gap between a row's title line and what is under it.
const TITLE_GAP: f32 = 4.0;
/// The rendering row's step and percent, right-aligned.
const PROGRESS_TEXT_W: f32 = 150.0;
/// The check list: its padding, the gap between its parts, the gap under its note, a line's least height.
const CHECKS_PAD: (f32, f32) = (14.0, 12.0);
const CHECKS_GAP: f32 = 4.0;
const CHECKS_NOTE_GAP: f32 = 8.0;
const CHECK_H: f32 = 26.0;
/// A check line's icon and the gap after it.
const ICON: f32 = 12.0;
const ICON_GAP: f32 = 8.0;
/// A menu's items.
const MENU_ITEM: (f32, f32) = (150.0, 24.0);
const MENU_PAD_X: f32 = 8.0;

/// The check list's note.
pub const CHECKS_NOTE: &str =
    "Every video goes through these in order. A failed check stops that video and the queue moves on.";
/// The check list with no video to show.
pub const NO_CHECKS: &str = "Nothing is rendering. A video's checks show here while it renders.";
/// The queue's header with nothing waiting or rendering.
pub const NOTHING_QUEUED: &str = "Nothing is queued. Add this fight to render it.";
/// The header's hint with two or more waiting.
pub const DRAG_HINT: &str = "Drag a waiting video to change the order";

/// A check line's name (interface reference, 5.5).
pub fn step_title(key: StepKey) -> &'static str {
    match key {
        StepKey::Soundtrack => "Soundtrack built",
        StepKey::SafeZone => "Safe zone",
        StepKey::Focus => "Focus",
        StepKey::Loudness => "Loudness",
        StepKey::Band => "Fighters above the music",
        StepKey::Flashes => "Flashes",
        StepKey::LoopPoint => "Loop point",
        StepKey::Frames => "Drawing frames",
        StepKey::Encode => "Encoding",
        StepKey::Verify => "Checking the finished video",
    }
}

/// A check line's note when the job has none of its own (interface reference, 5.5); `""` for none. "Drawing
/// frames" has none here: its line reads the job's own frame count.
pub fn step_note(key: StepKey) -> &'static str {
    match key {
        StepKey::SafeZone => "Captions and HUD clear of the apps\u{2019} buttons",
        StepKey::Focus => "Both fighters sharp on every frame",
        StepKey::Loudness => "Opening within 0.6 dB of the ending",
        StepKey::Flashes => "Busiest stretch peaks at 17% of the frame",
        StepKey::LoopPoint => "The last frame flows into the first",
        StepKey::Verify => "Length, loop and flashes again, on the mp4",
        StepKey::Soundtrack | StepKey::Band | StepKey::Frames | StepKey::Encode => "",
    }
}

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let full = ui.max_rect();
    let split = (full.max.x - CHECKS_W).max(full.min.x);
    let left = Rect::from_x_y_ranges(full.min.x..=split, full.y_range());
    let right = Rect::from_x_y_ranges(split..=full.max.x, full.y_range());

    let mut column = ui.new_child(UiBuilder::new().id_salt("render fight and queue").max_rect(left));
    let fight_bottom = this_fight(&mut column, vm, left, out);
    let painter = ui.painter();
    painter.rect_filled(
        Rect::from_x_y_ranges(left.x_range(), fight_bottom - 1.0..=fight_bottom),
        0.0,
        LINE_SOFT,
    );
    let queue_rect = Rect::from_x_y_ranges(left.x_range(), fight_bottom..=full.max.y.max(fight_bottom));
    let mut column = ui.new_child(UiBuilder::new().id_salt("render queue").max_rect(queue_rect));
    queue(&mut column, vm, queue_rect, out);

    ui.painter().rect_filled(
        Rect::from_x_y_ranges(split..=split + 1.0, full.y_range()),
        0.0,
        LINE_SOFT,
    );
    let mut column = ui.new_child(UiBuilder::new().id_salt("render checks").max_rect(right));
    checks(&mut column, vm, right);
}

// ---------------------------------------------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------------------------------------------

/// Text that is read, not clicked or selected, cut short with an ellipsis when it does not fit (hovering it then
/// shows all of it).
fn plain(ui: &mut Ui, text: RichText) -> Response {
    ui.add(Label::new(text).selectable(false).truncate())
}

/// Text that is read and wraps onto as many lines as it needs.
fn wrapped(ui: &mut Ui, text: RichText) -> Response {
    ui.add(Label::new(text).selectable(false).wrap())
}

/// A child of `ui` in `rect` laid out by `layout`, its items 4 apart.
fn child(ui: &mut Ui, rect: Rect, salt: &str, layout: Layout) -> Ui {
    let mut child = ui.new_child(UiBuilder::new().id_salt(salt).max_rect(rect).layout(layout));
    child.spacing_mut().item_spacing = vec2(GAP, 0.0);
    child
}

/// The width a button labelled `label` takes: the label and 10 at each side.
fn button_w(ui: &Ui, label: &str) -> f32 {
    let w = ui.painter().layout_no_wrap(label.to_owned(), body(), TEXT).size().x;
    (w + 20.0).ceil()
}

/// The height of one line of `font`.
fn line_h(ui: &Ui, font: &egui::FontId) -> f32 {
    ui.fonts_mut(|f| f.row_height(font))
}

/// The height `text` takes in `font`, wrapped at `width`.
fn wrapped_h(ui: &Ui, text: &str, font: egui::FontId, width: f32) -> f32 {
    let job = LayoutJob::simple(text.to_owned(), font, TEXT, width.max(1.0));
    ui.fonts_mut(|f| f.layout_job(job)).size().y
}

// ---------------------------------------------------------------------------------------------------------------
// This fight
// ---------------------------------------------------------------------------------------------------------------

/// The platform's name as its tab shows it.
fn platform_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Instagram => "Instagram",
        Platform::TikTok => "TikTok",
        Platform::Shorts => "Shorts",
    }
}

/// "This fight", in `column` (the left column), padded 14; returns the y its divider lies on.
fn this_fight(ui: &mut Ui, vm: &ViewModel, column: Rect, out: &mut Vec<Action>) -> f32 {
    let inner = column.shrink(FIGHT_PAD);
    let (x0, x1) = (inner.min.x, inner.max.x.max(inner.min.x));
    let mut y = inner.min.y;
    let row = |y: f32, h: f32| Rect::from_x_y_ranges(x0..=x1, y..=y + h);
    let indented = |y: f32, h: f32| Rect::from_x_y_ranges((x0 + INDENT).min(x1)..=x1, y..=y + h);
    let centred = Layout::left_to_right(Align::Center);

    // The heading.
    let heading_h = line_h(ui, &section());
    let mut c = child(ui, row(y, heading_h), "heading", centred);
    c.spacing_mut().item_spacing.x = LABEL_GAP;
    plain(&mut c, RichText::new("This fight").font(section()).color(TEXT));
    let take = match vm.draft.seed {
        Some(seed) => format!("Seed {seed}, music take {:04}", vm.draft.music_seed),
        None => "No seed picked yet".to_string(),
    };
    plain(&mut c, RichText::new(take).font(small()).color(MUTED));
    y += heading_h + HEADING_GAP;

    // The caption tabs.
    let mut c = child(ui, row(y, CONTROL_H), "caption tabs", centred);
    c.spacing_mut().item_spacing.x = 0.0;
    label_cell(&mut c, "Caption");
    c.add_space(LABEL_GAP);
    let tabs = [Platform::Instagram, Platform::TikTok, Platform::Shorts];
    let options = tabs.map(|p| (platform_name(p), None));
    let chosen = tabs.iter().position(|p| *p == vm.caption_tab);
    let width = SEGMENTS_W.min(c.available_width());
    if let Some(i) = buttons::segmented(&mut c, "Caption", &options, chosen, width)
        && tabs[i] != vm.caption_tab
    {
        out.push(Action::SetCaptionTab(tabs[i]));
    }
    c.add_space(LABEL_GAP);
    plain(
        &mut c,
        RichText::new("Drafted for you. Edit freely.")
            .font(small())
            .color(MUTED),
    );
    y += CONTROL_H + ROW_GAP;

    // The caption box.
    let name = format!("Text and tags for {}", platform_name(vm.caption_tab));
    let small_h = line_h(ui, &small());
    let mut c = child(ui, indented(y, small_h), "caption label", centred);
    plain(&mut c, RichText::new(&name).font(small()).color(MUTED));
    y += small_h + BOX_LABEL_GAP;
    let box_h = (BOX_LINES * line_h(ui, &body()) + 2.0 * BOX_PAD.1).round();
    caption_box(ui, vm, indented(y, box_h), &name, out);
    y += box_h + ROW_GAP;

    // Where it saves.
    let mut c = child(ui, row(y, CONTROL_H), "saves to", centred);
    c.spacing_mut().item_spacing.x = 0.0;
    label_cell(&mut c, "Saves to");
    c.add_space(LABEL_GAP);
    let stem = wording::folder_stem(&vm.draft.fighters);
    let folder = format!(
        "{}{MAIN_SEPARATOR}{stem}{MAIN_SEPARATOR}",
        vm.settings.output_folder.display()
    );
    let change = button_w(&c, "Change folder");
    let field_w = (c.available_width() - change - LABEL_GAP).max(0.0);
    fields::read_field(&mut c, "Saves to", &folder, field_w);
    c.add_space(LABEL_GAP);
    if buttons::boxed(&mut c, "Change folder").clicked() {
        out.push(Action::ChangeFolder);
    }
    y += CONTROL_H + ROW_GAP;
    if let (FolderState::Ok { taken: true }, Some(seed)) = (&vm.folder, vm.draft.seed) {
        let mut c = child(ui, indented(y, small_h), "taken", centred);
        let text = format!("That folder exists, so this one will be named {stem}_seed{seed}.");
        plain(&mut c, RichText::new(text).font(small()).color(MUTED));
        y += small_h + ROW_GAP;
    }

    // What it saves, and "Add to queue".
    let files = format!("{stem}.mp4, cover.png, caption.txt and the fight file");
    let files = RichText::new(files).font(small()).color(MUTED);
    match why_not_add(vm) {
        None => {
            let rect = indented(y, CONTROL_H);
            let mut c = child(ui, rect, "add", Layout::right_to_left(Align::Center));
            if buttons::primary(&mut c, "Add to queue").clicked() {
                out.push(Action::AddToQueue);
            }
            c.add_space(LABEL_GAP);
            c.with_layout(centred, |c| plain(c, files));
            y += CONTROL_H;
        }
        Some(reason) => {
            let mut c = child(ui, indented(y, small_h), "files", centred);
            plain(&mut c, files);
            y += small_h + ROW_GAP;
            y += waiting_to_add(ui, vm, indented(y, CONTROL_H), &reason, out);
        }
    }
    y + FIGHT_PAD
}

/// A label of the "This fight" rows: body `MUTED`, 70 wide.
fn label_cell(ui: &mut Ui, text: &str) {
    ui.allocate_ui_with_layout(vec2(LABEL_W, CONTROL_H), Layout::left_to_right(Align::Center), |ui| {
        plain(ui, RichText::new(text).font(body()).color(MUTED));
    });
}

/// "Add to queue" disabled, with `reason` beside it in `TEXT_STRONG` (wrapped onto as many lines as it needs) and,
/// with the folder missing, "Create folder" between them. Laid from the right in `rect`'s width from its top;
/// returns the height it took.
fn waiting_to_add(ui: &mut Ui, vm: &ViewModel, rect: Rect, reason: &str, out: &mut Vec<Action>) -> f32 {
    let create = vm.folder == FolderState::Missing;
    let mut buttons_w = button_w(ui, "Add to queue");
    if create {
        buttons_w += GAP + button_w(ui, "Create folder");
    }
    let words_w = (rect.width() - buttons_w - LABEL_GAP).max(0.0);
    let words_h = wrapped_h(ui, reason, body(), words_w);
    let height = words_h.max(CONTROL_H);
    let buttons = Rect::from_x_y_ranges(rect.max.x - buttons_w..=rect.max.x, rect.min.y..=rect.min.y + CONTROL_H);
    // The buttons sit level with the words' first line when the words take more than one.
    let mut c = child(ui, buttons, "add buttons", Layout::left_to_right(Align::Center));
    if create && buttons::boxed(&mut c, "Create folder").clicked() {
        out.push(Action::CreateFolder);
    }
    buttons::disabled(&mut c, "Add to queue", reason);
    let top = if words_h < CONTROL_H {
        rect.min.y + (CONTROL_H - words_h) / 2.0
    } else {
        rect.min.y
    };
    let words = Rect::from_min_size(pos2(rect.min.x, top), vec2(words_w, words_h));
    let mut c = child(ui, words, "add reason", Layout::top_down(Align::Max));
    c.add(
        Label::new(RichText::new(reason).font(body()).color(TEXT_STRONG))
            .selectable(false)
            .wrap()
            .halign(Align::Max),
    );
    height
}

/// The caption of `vm.caption_tab` in a three-line box over `well`, named `name`; it scrolls inside the box, and
/// every edit pushes `SetCaption`.
fn caption_box(ui: &mut Ui, vm: &ViewModel, well: Rect, name: &str, out: &mut Vec<Action>) {
    let platform = vm.caption_tab;
    let captions = &vm.draft.captions;
    let mut text = match platform {
        Platform::Instagram => captions.instagram.clone(),
        Platform::TikTok => captions.tiktok.clone(),
        Platform::Shorts => captions.shorts.clone(),
    };
    let painter = ui.painter().clone();
    painter.rect_filled(well, RADIUS, GROUND);
    let inside = well.shrink2(vec2(BOX_PAD.0, BOX_PAD.1));
    let mut c = ui.new_child(
        UiBuilder::new()
            .id_salt(("caption box", platform_name(platform)))
            .max_rect(inside),
    );
    let mut layouter = |ui: &Ui, buffer: &dyn TextBuffer, wrap_width: f32| {
        let shown = drawable(ui, buffer.as_str());
        let mut job = LayoutJob::simple(shown, body(), TEXT, wrap_width);
        job.wrap = TextWrapping::wrap_at_width(wrap_width);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let response = ScrollArea::vertical()
        .id_salt("caption scroll")
        .auto_shrink([false, false])
        .max_height(inside.height())
        .show(&mut c, |ui| {
            ui.add(
                TextEdit::multiline(&mut text)
                    .id_salt("caption text")
                    .frame(egui::Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .desired_rows(BOX_LINES as usize)
                    .text_color(TEXT)
                    .layouter(&mut layouter),
            )
            .accessible_name(name)
        })
        .inner;
    painter.rect_stroke(well, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
    if response.changed() {
        out.push(Action::SetCaption { platform, text });
    }
    if response.has_focus() {
        ring_above(ui, well);
    }
}

/// `text` with each character no font of `ui` can draw replaced by "?", one for one, so the picture of the text has
/// as many characters as the text (the box's cursor counts characters). The fonts include the PC's own (eframe's
/// system fonts), so on a PC with an emoji font the emoji are drawn as they are. Characters that join the one before
/// them into one picture are left alone: the character they join decides what is drawn.
fn drawable(ui: &Ui, text: &str) -> String {
    ui.fonts_mut(|f| {
        let font = body();
        text.chars()
            .map(|c| {
                if c.is_control() || joins(c) || f.has_glyph(&font, c) {
                    c
                } else {
                    '?'
                }
            })
            .collect()
    })
}

/// A character that joins the one before it into one picture: a zero-width joiner, a variation selector, a skin
/// tone, an emoji tag, a combining mark.
fn joins(c: char) -> bool {
    matches!(
        c,
        '\u{200d}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{0300}'..='\u{036f}'
            | '\u{20d0}'..='\u{20ff}'
            | '\u{1f3fb}'..='\u{1f3ff}'
            | '\u{e0020}'..='\u{e007f}'
    )
}

// ---------------------------------------------------------------------------------------------------------------
// The queue
// ---------------------------------------------------------------------------------------------------------------

/// What a job row offers: a button's label and what it pushes.
type Offer = (&'static str, Action);

/// The header's words: "{r} rendering, {w} waiting", "Paused. {w} waiting", or the empty queue's line.
fn queue_words(vm: &ViewModel) -> String {
    let (rendering, waiting) = vm.queue.counts();
    if rendering == 0 && waiting == 0 {
        NOTHING_QUEUED.to_string()
    } else if vm.queue.paused {
        format!("Paused. {} waiting", wording::thousands(waiting as u64))
    } else {
        format!(
            "{} rendering, {} waiting",
            wording::thousands(rendering as u64),
            wording::thousands(waiting as u64)
        )
    }
}

/// The queue: its header, then its rows in a scroll area.
fn queue(ui: &mut Ui, vm: &ViewModel, rect: Rect, out: &mut Vec<Action>) {
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), HEADER_H.min(rect.height())));
    ui.painter().rect_filled(
        Rect::from_x_y_ranges(rect.x_range(), header.max.y - 1.0..=header.max.y),
        0.0,
        LINE_SOFT,
    );
    let inside = header.shrink2(vec2(HEADER_PAD, 0.0));
    let mut c = child(ui, inside, "queue header", Layout::right_to_left(Align::Center));
    let (label, action) = if vm.queue.paused {
        ("Resume queue", QueueAction::Resume)
    } else {
        ("Pause queue", QueueAction::Pause)
    };
    if buttons::boxed(&mut c, label).clicked() {
        out.push(Action::Queue(action));
    }
    c.add_space(LABEL_GAP);
    c.with_layout(Layout::left_to_right(Align::Center), |c| {
        c.spacing_mut().item_spacing.x = LABEL_GAP;
        plain(c, RichText::new("Queue").font(section()).color(TEXT));
        plain(c, RichText::new(queue_words(vm)).font(small()).color(MUTED));
        if vm.queue.counts().1 >= 2 {
            plain(c, RichText::new(DRAG_HINT).font(small()).color(MUTED));
        }
    });

    let list = Rect::from_x_y_ranges(rect.x_range(), header.max.y..=rect.max.y.max(header.max.y));
    rows(ui, vm, list, out);
}

/// The job whose check list shows: the selected one, else the one rendering.
fn shown_job(vm: &ViewModel) -> Option<&Job> {
    vm.selected_job
        .and_then(|id| vm.queue.job(id))
        .or_else(|| vm.queue.rendering().and_then(|id| vm.queue.job(id)))
}

/// A row being dragged: the waiting job's id.
#[derive(Clone, Copy, Debug)]
struct Dragged(JobId);

/// The rows, one per job in `Queue::shown()` order, in a scroll area over `list`, inset 4 for the focus rings.
fn rows(ui: &mut Ui, vm: &ViewModel, list: Rect, out: &mut Vec<Action>) {
    let area = list.shrink2(vec2(RING_ROOM, 0.0));
    let mut c = ui.new_child(UiBuilder::new().id_salt("queue rows").max_rect(area));
    c.spacing_mut().item_spacing = vec2(GAP, 0.0);
    let shown = vm.queue.shown();
    let waiting: Vec<JobId> = shown
        .iter()
        .copied()
        .filter(|id| vm.queue.job(*id).is_some_and(|j| j.state == JobState::Waiting))
        .collect();
    let lit = shown_job(vm).map(|j| j.id);
    let mut ring = None;
    ScrollArea::vertical()
        .id_salt("queue scroll")
        .auto_shrink([false, false])
        .show(&mut c, |ui| {
            ui.add_space(RING_ROOM);
            for id in &shown {
                let Some(job) = vm.queue.job(*id) else {
                    continue;
                };
                let place = waiting.iter().position(|w| w == id).map(|at| (at, waiting.len()));
                let row = ui
                    .scope_builder(UiBuilder::new().id_salt(("job", *id)), |ui| {
                        job_row(ui, vm, job, lit == Some(*id), place, out)
                    })
                    .inner;
                if row.has_focus() {
                    ring = Some(row.rect);
                }
                let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(line, 0.0, HAIRLINE);
            }
            ui.add_space(RING_ROOM);
        });
    // The focused row's ring once more, over the rows after it, clipped to the list.
    if let Some(row) = ring {
        let painter = Painter::new(ui.ctx().clone(), c.layer_id(), list.intersect(ui.clip_rect()));
        painter.rect_stroke(
            row.expand(2.0),
            f32::from(RADIUS_ROW) + 2.0,
            Stroke::new(2.0, TEXT_STRONG),
            StrokeKind::Outside,
        );
    }
}

/// What a job's row says under its title, and what it offers.
struct RowState {
    lines: Vec<RichText>,
    offers: Vec<Offer>,
}

/// The step a rendering job is at, and the percent of its frames when that step is the frames: "Drawing frames
/// 64%".
fn progress_words(job: &Job) -> String {
    let at = job
        .steps
        .iter()
        .find(|l| l.state == StepState::Now)
        .or_else(|| job.steps.iter().find(|l| l.state == StepState::Todo));
    let Some(line) = at else {
        return String::new();
    };
    let title = step_title(line.key);
    match (&job.state, line.key) {
        (JobState::Rendering { done, total }, StepKey::Frames) if *total > 0 && done <= total => {
            format!("{title} {}", wording::percent(f64::from(*done) / f64::from(*total)))
        }
        _ => title.to_string(),
    }
}

/// The share of frames drawn, 0 when it cannot be told.
fn frames_share(job: &Job) -> f32 {
    match job.state {
        JobState::Rendering { done, total } if total > 0 => (done.min(total) as f32) / total as f32,
        _ => 0.0,
    }
}

fn row_state(job: &Job) -> RowState {
    let id = job.id;
    let muted = |text: &str| RichText::new(text).font(body()).color(MUTED);
    let again = ("Queue again", Action::Queue(QueueAction::Requeue(id)));
    let remove = ("Remove", Action::Queue(QueueAction::Remove(id)));
    match &job.state {
        JobState::Rendering { .. } => RowState {
            lines: Vec::new(),
            offers: vec![("Cancel", Action::Queue(QueueAction::Cancel(id)))],
        },
        JobState::Waiting => RowState {
            lines: vec![muted(if job.interrupted {
                "Waiting. It was interrupted when the app closed."
            } else {
                "Waiting"
            })],
            offers: vec![remove],
        },
        JobState::Failed { headline, detail, hint } => {
            let mut offers = vec![again];
            let open_in = |step| Action::Queue(QueueAction::OpenIn { id, step });
            match hint {
                FailHint::OpenInSeeds => offers.push(("Open in Seeds", open_in(Step::Seeds))),
                FailHint::OpenInMusic => offers.push(("Open in Music", open_in(Step::Music))),
                FailHint::CheckFolder => offers.push(("Change folder", Action::ChangeFolder)),
                FailHint::InstallFfmpeg | FailHint::None => {}
            }
            offers.push(remove);
            RowState {
                lines: vec![RichText::new(headline).font(body()).color(TEXT_STRONG), muted(detail)],
                offers,
            }
        }
        JobState::Cancelled => RowState {
            lines: vec![muted("Cancelled. Nothing was saved.")],
            offers: vec![again, remove],
        },
        JobState::Done { seconds, .. } => {
            let words = if seconds.is_finite() && *seconds >= 0.0 {
                format!(
                    "Done in {} s. All checks passed.",
                    wording::thousands(seconds.round() as u64)
                )
            } else {
                "Done. All checks passed.".to_string()
            };
            RowState {
                lines: vec![muted(&words)],
                offers: vec![("Open folder", Action::Queue(QueueAction::OpenFolder(id)))],
            }
        }
    }
}

/// One job's row: the thumbnail, the title line, what its state says, and its buttons. `place` is a waiting job's
/// place in line, of how many wait. Returns the row's own response.
fn job_row(
    ui: &mut Ui,
    vm: &ViewModel,
    job: &Job,
    lit: bool,
    place: Option<(usize, usize)>,
    out: &mut Vec<Action>,
) -> Response {
    let state = row_state(job);
    let width = ui.available_width();
    // The row's inside, as `object_row` pads it (10 at each side).
    let inside_w = (width - 20.0).max(0.0);
    let offers_w: f32 = state.offers.iter().map(|(label, _)| button_w(ui, label)).sum::<f32>()
        + GAP * state.offers.len().saturating_sub(1) as f32;
    let text_w = (inside_w - THUMB.0 - JOB_GAP - offers_w - JOB_GAP).max(0.0);
    let title_h = line_h(ui, &section());
    let body_h = line_h(ui, &body());
    let under_h = match &job.state {
        JobState::Rendering { .. } => line_h(ui, &mono()),
        _ => state
            .lines
            .iter()
            .map(|t| wrapped_h(ui, t.text(), body(), text_w).max(body_h))
            .sum::<f32>(),
    };
    let content_h = title_h + TITLE_GAP + under_h;
    let height = (content_h.max(THUMB.1) + 2.0 * JOB_PAD_Y).ceil();
    let name = format!("{}, seed {}", job.spec.title, job.spec.seed);

    // A waiting row senses drags too. The drag is registered here, on the id and the place `object_row` is about to
    // take, so the row comes before its buttons in the Tab order (adding the drag after the buttons would put the
    // row after them).
    let dragging = place.map(|_| {
        let rect = Rect::from_min_size(ui.cursor().min, vec2(width, height));
        ui.interact(rect, ui.next_auto_id(), Sense::click_and_drag())
    });

    let (response, ()) = object_row(ui, &name, lit, height, |ui| {
        let inside = ui.max_rect();
        let mid = inside.center().y;
        let thumb = Rect::from_min_size(
            pos2(inside.min.x, (mid - THUMB.1 / 2.0).round()),
            vec2(THUMB.0, THUMB.1),
        );
        thumbnail(ui.painter(), thumb, job.spec.colors);
        let text_x = thumb.max.x + JOB_GAP;
        let top = (mid - content_h / 2.0).round();
        let title = Rect::from_x_y_ranges(text_x..=text_x + text_w, top..=top + title_h);
        let mut c = child(ui, title, "title", Layout::left_to_right(Align::Center));
        c.set_clip_rect(title.intersect(ui.clip_rect()));
        c.spacing_mut().item_spacing.x = 6.0;
        fighter_bar(&mut c, job.spec.colors[0], 16.0);
        plain(&mut c, RichText::new(&job.spec.title).font(section()));
        fighter_bar(&mut c, job.spec.colors[1], 16.0);
        let about = format!("{}, seed {}", job.spec.theme, job.spec.seed);
        plain(&mut c, RichText::new(about).font(small()).color(MUTED));

        let under = Rect::from_x_y_ranges(text_x..=text_x + text_w, title.max.y + TITLE_GAP..=inside.max.y);
        if let JobState::Rendering { .. } = job.state {
            let line = Rect::from_x_y_ranges(under.x_range(), under.min.y..=under.min.y + under_h);
            let mut c = child(ui, line, "progress", Layout::right_to_left(Align::Center));
            c.allocate_ui_with_layout(
                vec2(PROGRESS_TEXT_W.min(text_w), under_h),
                Layout::right_to_left(Align::Center),
                |c| plain(c, RichText::new(progress_words(job)).font(mono()).color(TEXT)),
            );
            let bar_w = (c.available_width() - JOB_GAP).max(0.0);
            c.add_space(JOB_GAP);
            c.with_layout(Layout::left_to_right(Align::Center), |c| {
                buttons::progress(c, &format!("Progress of {}", job.spec.title), frames_share(job), bar_w)
            });
        } else {
            let mut c = child(ui, under, "under", Layout::top_down(Align::Min));
            c.set_width(text_w);
            for (i, line) in state.lines.iter().enumerate() {
                // The first line is one line (cut short with an ellipsis); a failed job's detail wraps.
                if i == 0 {
                    plain(&mut c, line.clone());
                } else {
                    wrapped(&mut c, line.clone());
                }
            }
        }

        let offers = Rect::from_x_y_ranges(inside.max.x - offers_w..=inside.max.x, inside.y_range());
        let mut c = child(ui, offers, "offers", Layout::left_to_right(Align::Center));
        for (label, action) in &state.offers {
            if buttons::boxed(&mut c, label).clicked() {
                out.push(action.clone());
            }
        }
    });

    let response = match dragging {
        Some(dragging) => {
            debug_assert_eq!(
                (dragging.id, dragging.rect),
                (response.id, response.rect),
                "the drag is the row's own"
            );
            dragging
        }
        None => response,
    };
    if response.clicked() && vm.selected_job != Some(job.id) {
        out.push(Action::Queue(QueueAction::Select(job.id)));
    }
    if let Some((at, count)) = place {
        waiting_moves(ui, &response, job.id, at, count, out);
    }
    response
}

/// A waiting row's drag and right-click menu. `at` is its place among the `count` waiting jobs.
fn waiting_moves(ui: &Ui, response: &Response, id: JobId, at: usize, count: usize, out: &mut Vec<Action>) {
    response.dnd_set_drag_payload(Dragged(id));
    if let Some(dragged) = response.dnd_hover_payload::<Dragged>()
        && dragged.0 != id
    {
        // Where it would go: a 2 px line along the top of this row.
        let r = response.rect;
        let line = Rect::from_x_y_ranges(r.x_range(), r.min.y..=r.min.y + 2.0);
        ui.painter().rect_filled(line, 0.0, TEXT);
    }
    if let Some(dragged) = response.dnd_release_payload::<Dragged>()
        && dragged.0 != id
    {
        out.push(Action::Queue(QueueAction::Move { id: dragged.0, to: at }));
    }
    let first = "It is already first in line.";
    let last = "It is already last in line.";
    let items: [(&str, Option<usize>, &str); 3] = [
        ("Move up", at.checked_sub(1), first),
        ("Move down", (at + 1 < count).then_some(at + 1), last),
        ("Render next", (at > 0).then_some(0), first),
    ];
    Popup::context_menu(response).style(menu_style).show(|ui| {
        for (label, to, why) in items {
            if menu_item(ui, label, to.is_some(), why).clicked()
                && let Some(to) = to
            {
                out.push(Action::Queue(QueueAction::Move { id, to }));
                ui.close();
            }
        }
    });
}

/// A menu: `RAISED`, padding 3, items with no gap between them.
fn menu_style(style: &mut Style) {
    style.visuals.window_fill = RAISED;
    style.spacing.menu_margin = Margin::same(3);
    style.spacing.item_spacing.y = 0.0;
}

/// A menu item, 24 tall, padded 8: inverted while hovered or focused; `FAINT` and saying `why` on hover when it
/// cannot be used.
fn menu_item(ui: &mut Ui, label: &str, enabled: bool, why: &str) -> Response {
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, mut response) = ui.allocate_exact_size(vec2(MENU_ITEM.0, MENU_ITEM.1), sense);
    response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let lit = enabled && (response.hovered() || response.has_focus());
        let colour = if !enabled {
            FAINT
        } else if lit {
            painter.rect_filled(rect, RADIUS, TEXT);
            ON_LIGHT
        } else {
            TEXT
        };
        let galley = painter.layout_no_wrap(label.to_owned(), body(), colour);
        let at = pos2(rect.min.x + MENU_PAD_X, rect.center().y - galley.size().y / 2.0).round();
        painter.galley(at, galley, colour);
    }
    if !enabled {
        response = response.on_hover_text(why);
    }
    response
}

/// The 27 x 48 thumbnail: the two fighters' colours split on a diagonal, the left one above it.
fn thumbnail(painter: &Painter, rect: Rect, colors: [[u8; 3]; 2]) {
    let [a, b] = colors.map(rgb);
    painter.add(Shape::convex_polygon(
        vec![rect.left_top(), rect.right_top(), rect.left_bottom()],
        a,
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![rect.right_top(), rect.right_bottom(), rect.left_bottom()],
        b,
        Stroke::NONE,
    ));
}

// ---------------------------------------------------------------------------------------------------------------
// The check list
// ---------------------------------------------------------------------------------------------------------------

/// The check list of the job shown, in `column`, padded 14 x 12.
fn checks(ui: &mut Ui, vm: &ViewModel, column: Rect) {
    let inner = column.shrink2(vec2(CHECKS_PAD.0, CHECKS_PAD.1));
    let mut c = ui.new_child(
        UiBuilder::new()
            .id_salt("check list")
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
    );
    c.set_clip_rect(column.intersect(ui.clip_rect()));
    c.spacing_mut().item_spacing = vec2(ICON_GAP, 0.0);
    let ui = &mut c;
    let Some(job) = shown_job(vm) else {
        wrapped(ui, RichText::new(NO_CHECKS).font(body()).color(MUTED));
        return;
    };
    plain(ui, RichText::new(&job.spec.title).font(section()).color(TEXT));
    ui.add_space(CHECKS_GAP);
    wrapped(ui, RichText::new(CHECKS_NOTE).font(small()).color(MUTED));
    ui.add_space(CHECKS_NOTE_GAP);
    for (i, key) in STEP_ORDER.iter().enumerate() {
        if i > 0 {
            ui.add_space(CHECKS_GAP);
        }
        let line = job.steps.iter().find(|l| l.key == *key);
        check_line(ui, job, *key, line);
    }
}

/// The note of `key`'s line: the frame count while the job renders (once it has frames to count), else the job's
/// own, else `step_note`.
fn check_note(job: &Job, key: StepKey, line: Option<&StepLine>) -> String {
    if let (StepKey::Frames, JobState::Rendering { done, total }) = (key, &job.state)
        && *total > 0
        && done <= total
    {
        return format!(
            "{} of {}",
            wording::thousands(u64::from(*done)),
            wording::thousands(u64::from(*total))
        );
    }
    match line.and_then(|l| l.note.clone()) {
        Some(note) => note,
        None => step_note(key).to_string(),
    }
}

/// One line of the check list, at least 26 tall: its icon and name, and its note under the name in small `MUTED`.
/// To do: `MUTED` with a `FAINT` icon; in progress (and the line a job failed at): `TEXT_STRONG`; done: `TEXT`.
fn check_line(ui: &mut Ui, job: &Job, key: StepKey, line: Option<&StepLine>) {
    let state = line.map_or(StepState::Todo, |l| l.state);
    let (icon, icon_colour, colour) = match state {
        StepState::Done => (Icon::Done, TEXT, TEXT),
        StepState::Now | StepState::Failed => (Icon::Now, TEXT_STRONG, TEXT_STRONG),
        StepState::Todo => (Icon::Todo, FAINT, MUTED),
    };
    let note = check_note(job, key, line);
    let name_h = line_h(ui, &body());
    let note_h = if note.is_empty() { 0.0 } else { line_h(ui, &small()) };
    // The name sits where it would in the middle of a 26 tall line; a note adds its height under it.
    let pad = ((CHECK_H - name_h) / 2.0).max(0.0).floor();
    let height = (pad + name_h + note_h + pad).max(CHECK_H);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let top = rect.min.y + pad;
    let icon_rect = Rect::from_min_size(
        pos2(rect.min.x, (top + (name_h - ICON) / 2.0).round()),
        vec2(ICON, ICON),
    );
    icons::paint(ui.painter(), icon_rect, icon, icon_colour);
    let text_x = icon_rect.max.x + ICON_GAP;
    let words = Rect::from_x_y_ranges(text_x..=rect.max.x.max(text_x), top..=top + name_h + note_h);
    let mut c = child(ui, words, step_title(key), Layout::top_down(Align::Min));
    plain(&mut c, RichText::new(step_title(key)).font(body()).color(colour));
    if !note.is_empty() {
        plain(&mut c, RichText::new(note).font(small()).color(MUTED));
    }
}
