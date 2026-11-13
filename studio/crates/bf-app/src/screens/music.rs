//! The Music step's workspace (interface reference, section 5.4, with the placeholders of the plan's table "What is
//! live and what waits"): the controls on the left, the arrangement of the take's soundtrack in the rest, with the
//! take's loudness checks under the lanes and a footer that leads on to Render.
//!
//! Live: the theme, the chord loop (each of the theme's loops, and the fight file's own when it is none of them) and,
//! when both fighters' cards can play bass and one of them does, who plays bass. Read-only: the theme's options as
//! the fight file sets them ("The sequel", "No bass drums", "Plugged: Electric") and each fighter's part (ruling
//! M3-R8). Drawn but disabled, each saying why when hovered: the key (it follows the theme), the music seed and
//! "Re-roll music" (new takes arrive with the music system). The lanes are plain blocks showing where each part of
//! the built soundtrack sounds; they carry no names until the music system gives the parts names (section 9). A
//! take whose loudness checks fail says that its video would stop there, and offers the next fight that fits.
//!
//! The screen is drawn from the view model alone and pushes what the owner does; the only things it remembers are
//! egui's own (where the two scroll areas are scrolled, which control has the keyboard, whether the theme's list is
//! open).

use egui::accesskit::Role;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Frame, Label, Layout, Margin, Painter, Rect, Response, RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, WidgetInfo, pos2, vec2,
};

use crate::model::roster::{BASS_ROLE, part_words};
use crate::model::take::{Lane, LaneGroup, MusicView};
use crate::model::{Step, View};
use crate::state::{Action, ViewModel};
use crate::theme::{
    BAND_CLIP, CONTROL_H, FAINT, GROUP, HAIRLINE, HOVER, LINE, LINE_SOFT, MUTED, RADIUS, RAISED, TEXT, TEXT_STRONG,
    VALUE_W, body, large_fill, rgb, section, small, value as mono,
};
use crate::widgets::buttons::{self, widget_ring};
use crate::widgets::chips::{fighter_bar, paint_selected};
use crate::widgets::fields::{self, ReadLook};
use crate::widgets::icons::{self, Icon};
use crate::widgets::ribbon::{DUR, x_of};

/// The controls column's width.
pub const CONTROLS_W: f32 = 300.0;
/// The arrangement's lane-name column.
pub const NAMES_W: f32 = 112.0;

/// The controls column's padding: 12 at the sides, 14 above and below.
const CONTROLS_PAD: (i8, i8) = (12, 14);
/// The gap between two groups of the controls column.
const GROUP_GAP: f32 = 13.0;
/// The gap between a group's label and its control, and between a control and its hint.
const LABEL_GAP: f32 = 5.0;
/// The gap between the controls of one row.
const ROW_GAP: f32 = 8.0;
/// The key's dropdown.
const KEY_W: f32 = 70.0;
/// A chord loop's button, and the gap between two of them.
const LOOP_H: f32 = 28.0;
const LOOP_GAP: f32 = 4.0;
/// Side padding inside a chord loop's button.
const LOOP_PAD: f32 = 10.0;
/// A personality's fighter bar.
const PERSONALITY_BAR: f32 = 34.0;

/// The arrangement's header and footer, and their side padding.
const HEADER_H: f32 = 40.0;
const FOOTER_H: f32 = 46.0;
const SIDE_PAD: f32 = 14.0;
/// The gap between the header's title and its hint.
const HEADER_GAP: f32 = 12.0;
/// The lanes: the marker lane, a group header, a part's lane.
const MARKER_H: f32 = 22.0;
const GROUP_H: f32 = 22.0;
const LANE_H: f32 = 25.0;
/// The lane-name column's padding: 10 at the sides, 4 above.
const NAME_PAD: (f32, f32) = (10.0, 4.0);
/// A clip: from 2 below its lane's top, 20 tall, at least 6 wide (so a band hit is a 6 wide tick).
const CLIP_TOP: f32 = 2.0;
const CLIP_H: f32 = 20.0;
const CLIP_MIN_W: f32 = 6.0;
/// A marker's name starts 4 right of its line.
const MARKER_GAP: f32 = 4.0;
/// How far a marker's hover area reaches each side of its line.
const MARKER_HOVER: f32 = 3.0;
/// The block under the lanes: padded 14 at the sides and 16 above and below, 6 between its lines.
const UNDER_PAD: (i8, i8) = (14, 16);
const UNDER_GAP: f32 = 6.0;
/// A check's title column, and its icon.
const CHECK_TITLE_W: f32 = 150.0;
const ICON: f32 = 12.0;
/// What a focus ring takes outside its control: a 2 px gap, then 2 px of ring.
const RING_ROOM: f32 = 4.0;

/// Why the key cannot be changed.
pub const KEY_WHY: &str = "The key comes with the chord loop in this version.";
/// Why the music seed and "Re-roll music" cannot be used.
pub const TAKES_WHY: &str = "New takes arrive with the music system.";
/// The line under them.
pub const TAKES_LINE: &str = "New takes arrive with the music system. This version plays the series' own arrangement.";
/// Under the fight file's own chord loop.
pub const OWN_LOOP: &str = "The last loop is this match-up's own, from the series.";
/// Under the theme's options.
pub const OPTIONS_LINE: &str = "Set by the fight file. A new theme starts them at its own settings.";
/// The screen before a seed is picked.
pub const NO_SEED: &str = "Pick a seed first. The music is built for the fight you choose.";
/// What stands in for a value that is not there.
const NONE: &str = "–";

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let full = ui.max_rect();
    let split = (full.min.x + CONTROLS_W).min(full.max.x);
    let controls = Rect::from_x_y_ranges(full.min.x..=split, full.y_range());
    let rest = Rect::from_x_y_ranges(split..=full.max.x.max(split), full.y_range());
    ui.painter().rect_filled(
        Rect::from_x_y_ranges(split - 1.0..=split, full.y_range()),
        0.0,
        LINE_SOFT,
    );
    let mut column = ui.new_child(UiBuilder::new().id_salt("music controls").max_rect(controls));
    controls_column(&mut column, vm, out);
    let mut arrangement = ui.new_child(UiBuilder::new().id_salt("music arrangement").max_rect(rest));
    arrangement_column(&mut arrangement, vm, out);
}

// ---------------------------------------------------------------------------------------------------------------
// The controls
// ---------------------------------------------------------------------------------------------------------------

/// Text that is read, not clicked or selected, cut short with an ellipsis when it does not fit.
fn plain(ui: &mut Ui, text: RichText) -> Response {
    ui.add(Label::new(text).selectable(false).truncate())
}

/// Text that is read, wrapped to the width it is given.
fn wrapped(ui: &mut Ui, text: RichText) -> Response {
    ui.add(Label::new(text).selectable(false).wrap())
}

/// A group's label: body, `MUTED`, then the gap to its control.
fn group_label(ui: &mut Ui, text: &str) {
    plain(ui, RichText::new(text).font(body()).color(MUTED));
    ui.add_space(LABEL_GAP);
}

/// A small `MUTED` hint, wrapped.
fn hint(ui: &mut Ui, text: &str) -> Response {
    wrapped(ui, RichText::new(text).font(small()).color(MUTED))
}

/// A 1 px `LINE_SOFT` divider across `ui`.
fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, LINE_SOFT);
}

/// `text` in `font` and `colour`, cut short with an ellipsis at `width`, its left edge `left` points into `rect` and
/// centred in its height, on whole pixels.
fn paint_text(painter: &Painter, rect: Rect, text: &str, font: egui::FontId, colour: egui::Color32, left: f32) {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, colour);
    job.wrap = TextWrapping::truncate_at_width((rect.width() - 2.0 * left).max(0.0));
    let galley = painter.layout_job(job);
    let y = (rect.center().y - galley.size().y / 2.0).round();
    painter.with_clip_rect(rect.intersect(painter.clip_rect())).galley(
        pos2((rect.min.x + left).round(), y),
        galley,
        colour,
    );
}

fn controls_column(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let rect = ui.max_rect();
    let area = ScrollArea::vertical()
        .id_salt("music controls scroll")
        .auto_shrink([false, false]);
    buttons::scrolled(ui, area, |ui| {
        Frame::new()
            .inner_margin(Margin::symmetric(CONTROLS_PAD.0, CONTROLS_PAD.1))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(ROW_GAP, 0.0);
                let inner_h = rect.height() - 2.0 * f32::from(CONTROLS_PAD.1);
                controls(ui, vm, inner_h, out);
            });
    });
}

/// The groups of the controls column, top to bottom; the tempo line sits at the bottom of `inner_h` when there is
/// room, else after the rest.
fn controls(ui: &mut Ui, vm: &ViewModel, inner_h: f32, out: &mut Vec<Action>) {
    let top = ui.cursor().min.y;
    let width = ui.available_width();
    let theme = vm.theme();

    group_label(ui, "Theme");
    // only the themes this pair can be set in (ruling M3-R2)
    let themes = vm
        .roster
        .as_ref()
        .map_or_else(Vec::new, |r| r.themes_for(&vm.draft.fighters));
    let options: Vec<String> = themes.iter().map(|t| t.display.clone()).collect();
    let chosen = themes
        .iter()
        .position(|t| t.name == vm.draft.theme)
        .unwrap_or(usize::MAX);
    if let Some(i) = buttons::dropdown(ui, "Theme", &options, chosen, width)
        && let Some(picked) = themes.get(i)
        && picked.name != vm.draft.theme
    {
        out.push(Action::SetTheme(picked.name.clone()));
    }
    ui.add_space(GROUP_GAP);

    group_label(ui, "Key");
    ui.horizontal(|ui| {
        let key_scale = vm.key_scale();
        let key = key_scale.as_ref().map_or(NONE, |(k, _)| k.as_str());
        buttons::dropdown_disabled(ui, "Key", key, KEY_W, KEY_WHY);
        if let Some((_, scale)) = &key_scale {
            let line = format!("{scale}, from the theme");
            plain(ui, RichText::new(line).font(body()).color(MUTED));
        }
    });
    ui.add_space(GROUP_GAP);

    group_label(ui, "Chord loop");
    let loops = theme.map_or(&[][..], |t| &t.chord_loops[..]);
    // the theme's loops, then the fight file's own when it is none of them (one past the theme's last)
    let own = vm.own_loop();
    let all = loops.iter().map(|l| &l[..]).chain(own);
    for (i, chords) in all.enumerate() {
        if i > 0 {
            ui.add_space(LOOP_GAP);
        }
        let label = chords.join("  ");
        let chosen = vm.draft.chord_loop == i;
        if chord_loop(ui, &label, chosen, width).clicked() && !chosen {
            out.push(Action::SetChordLoop(i));
        }
    }
    if own.is_some() {
        ui.add_space(LABEL_GAP);
        hint(ui, OWN_LOOP);
    }
    ui.add_space(GROUP_GAP);

    let options = vm.option_words();
    if !options.is_empty() {
        group_label(ui, "Theme options");
        for (i, words) in options.iter().enumerate() {
            if i > 0 {
                ui.add_space(LABEL_GAP);
            }
            plain(ui, RichText::new(words).font(body()).color(TEXT));
        }
        ui.add_space(LABEL_GAP);
        hint(ui, OPTIONS_LINE);
        ui.add_space(GROUP_GAP);
    }

    let names = vm.names();
    let parts = vm.parts();
    let known = vm.known_parts();
    group_label(ui, "Parts");
    for side in [0, 1] {
        if side > 0 {
            ui.add_space(LABEL_GAP);
        }
        let line = format!("{} plays {}.", names[side], part_words(&parts[side], &known[side]));
        plain(ui, RichText::new(line).font(body()).color(TEXT));
    }
    ui.add_space(GROUP_GAP);

    if vm.bass_can_move() {
        group_label(ui, "Who plays bass");
        let options: Vec<(&str, Option<&str>)> = names.iter().map(|name| (name.as_str(), None)).collect();
        let bass = parts.iter().position(|p| p == BASS_ROLE);
        if let Some(side) = buttons::segmented(ui, "Who plays bass", &options, bass, width)
            && Some(side) != bass
        {
            out.push(Action::SetBass(side));
        }
        ui.add_space(LABEL_GAP);
        let other = bass.map_or(0, |side| 1 - side);
        hint(
            ui,
            &format!("The other fighter takes {}.", part_words(&parts[other], &known[other])),
        );
        ui.add_space(GROUP_GAP);
    }

    divider(ui);
    ui.add_space(GROUP_GAP);
    group_label(ui, "This take");
    ui.horizontal(|ui| {
        plain(ui, RichText::new("Music seed").font(body()).color(MUTED));
        let seed = format!("{:04}", vm.draft.music_seed);
        let look = ReadLook {
            compact: true,
            why: Some(TAKES_WHY),
        };
        fields::read_field_with(ui, "Music seed", &seed, VALUE_W, look);
        buttons::disabled(ui, "Re-roll music", TAKES_WHY);
    });
    ui.add_space(LABEL_GAP);
    hint(ui, TAKES_LINE);
    ui.add_space(GROUP_GAP);

    divider(ui);
    let colors = vm.colors();
    for side in [0, 1] {
        let Some(card) = vm.fighter(side).and_then(|f| f.card.as_ref()) else {
            continue;
        };
        ui.add_space(GROUP_GAP);
        ui.horizontal_top(|ui| {
            fighter_bar(ui, colors[side], PERSONALITY_BAR);
            ui.vertical(|ui| {
                plain(ui, RichText::new(&card.music_title).font(section()).color(TEXT));
                hint(ui, &card.music_text);
            });
        });
    }

    let tempo = "Tempo is set on Match-up, because it changes the fight.";
    let tempo_h = ui.painter().layout(tempo.to_owned(), small(), MUTED, width).size().y;
    let used = ui.cursor().min.y - top;
    let spare = inner_h - used - GROUP_GAP - tempo_h;
    ui.add_space(GROUP_GAP + if spare.is_finite() { spare.max(0.0) } else { 0.0 });
    hint(ui, tempo);
}

/// One chord loop's button, `width` wide and 28 tall, its chords in mono from the left. The chosen one has the
/// selected look (`SELECTED` fill, a 1.5 px `TEXT` ring inside, `TEXT_STRONG` text); the others are box buttons
/// (`RAISED`, `HOVER` when hovered, a 1 px `LINE` border). It paints its own focus ring.
fn chord_loop(ui: &mut Ui, label: &str, chosen: bool, width: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, LOOP_H), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, chosen, label));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let colour = if chosen {
            paint_selected(painter, rect, RADIUS, true)
        } else {
            painter.rect_filled(rect, RADIUS, if response.hovered() { HOVER } else { RAISED });
            painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, LINE), StrokeKind::Inside);
            TEXT
        };
        paint_text(painter, rect, label, mono(), colour, LOOP_PAD);
        if response.has_focus() {
            widget_ring(ui, rect, f32::from(RADIUS));
        }
    }
    response
}

// ---------------------------------------------------------------------------------------------------------------
// The arrangement
// ---------------------------------------------------------------------------------------------------------------

/// A child of `ui` in `rect` with `layout`, clipped to it and the 4 px round it that a focus ring takes.
fn cell<R>(
    ui: &mut Ui,
    rect: Rect,
    salt: impl std::hash::Hash + std::fmt::Debug,
    layout: Layout,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let mut child = ui.new_child(UiBuilder::new().id_salt(salt).max_rect(rect).layout(layout));
    child.set_clip_rect(rect.expand(RING_ROOM).intersect(ui.clip_rect()));
    add(&mut child)
}

fn arrangement_column(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let area = ui.max_rect();
    let Some(seed) = vm.draft.seed else {
        no_seed(ui, area, out);
        return;
    };
    let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER_H));
    let footer = Rect::from_x_y_ranges(area.x_range(), (area.max.y - FOOTER_H)..=area.max.y);
    let middle = Rect::from_x_y_ranges(area.x_range(), header.max.y..=footer.min.y.max(header.max.y));

    cell(
        ui,
        header.shrink2(vec2(SIDE_PAD, 0.0)),
        "header",
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = HEADER_GAP;
            plain(ui, RichText::new("Arrangement").font(section()).color(TEXT));
            plain(
                ui,
                RichText::new("How this take follows the fight you picked")
                    .font(small())
                    .color(MUTED),
            );
        },
    );

    let mut child = ui.new_child(UiBuilder::new().id_salt("lanes").max_rect(middle));
    let lanes_area = ScrollArea::vertical()
        .id_salt("music lanes scroll")
        .auto_shrink([false, false]);
    buttons::scrolled(&mut child, lanes_area, |ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        match &vm.take.music {
            Some(music) => lanes(ui, music),
            None => {
                Frame::new()
                    .inner_margin(Margin::symmetric(UNDER_PAD.0, UNDER_PAD.1))
                    .show(ui, |ui| {
                        let text = format!("Building the sound for seed {seed}");
                        plain(ui, RichText::new(text).font(body()).color(MUTED));
                    });
            }
        }
        loudness(ui, vm, out);
    });

    ui.painter().rect_filled(
        Rect::from_x_y_ranges(area.x_range(), footer.min.y..=footer.min.y + 1.0),
        0.0,
        LINE_SOFT,
    );
    footer_row(ui, footer.shrink2(vec2(SIDE_PAD, 0.0)), out);
}

/// The screen with no seed: one centred `MUTED` line that says what to do, and "Go to seeds" under it.
fn no_seed(ui: &mut Ui, area: Rect, out: &mut Vec<Action>) {
    let wrap = (area.width() - 2.0 * SIDE_PAD).max(0.0);
    let mut job = LayoutJob::simple(NO_SEED.to_owned(), body(), MUTED, wrap);
    job.halign = Align::Center;
    let text_h = ui.painter().layout_job(job).size().y;
    let height = text_h + 2.0 * LABEL_GAP + CONTROL_H;
    let rect = Rect::from_center_size(area.center(), vec2(wrap, height)).round();
    let text = Rect::from_min_size(rect.min, vec2(wrap, text_h));
    cell(ui, text, "no seed", Layout::top_down(Align::Center), |ui| {
        ui.add(
            Label::new(RichText::new(NO_SEED).font(body()).color(MUTED))
                .wrap()
                .selectable(false)
                .halign(Align::Center),
        );
    });
    // The button on whole pixels, so its focus ring is sharp.
    let label = "Go to seeds";
    let button_w = (ui.painter().layout_no_wrap(label.to_owned(), body(), TEXT).size().x + 20.0).ceil();
    let at = pos2((rect.center().x - button_w / 2.0).round(), rect.max.y - CONTROL_H);
    let button = Rect::from_min_size(at, vec2(button_w, CONTROL_H));
    cell(ui, button, "go to seeds", Layout::left_to_right(Align::Min), |ui| {
        if buttons::boxed(ui, label).clicked() {
            out.push(Action::Go(View::Step(Step::Seeds)));
        }
    });
}

/// One row of the lanes, `height` tall across `ui`, with a 1 px `HAIRLINE` under it. Returns its rect.
fn lane_row(ui: &mut Ui, height: f32, fill: Option<egui::Color32>) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter();
    if let Some(fill) = fill {
        painter.rect_filled(rect, 0.0, fill);
    }
    painter.rect_filled(
        Rect::from_x_y_ranges(rect.x_range(), rect.max.y - 1.0..=rect.max.y),
        0.0,
        HAIRLINE,
    );
    rect
}

/// The x range of a clip from `start` to `end` seconds on lanes starting at `left`, `width` wide: whole pixels, at
/// least `CLIP_MIN_W` wide, kept inside 0 to 61 s. A clip whose times are not numbers is not drawn.
fn clip_x(start: f32, end: f32, left: f32, width: f32) -> Option<(f32, f32)> {
    if !(start.is_finite() && end.is_finite()) {
        return None;
    }
    let start = start.clamp(0.0, DUR);
    let end = end.clamp(start, DUR);
    let x0 = x_of(start, left, width).round();
    let x1 = x_of(end, left, width).round().max(x0 + CLIP_MIN_W);
    Some((x0, x1))
}

/// The lanes: the marker lane, then each group's header and its parts.
fn lanes(ui: &mut Ui, music: &MusicView) {
    let row = lane_row(ui, MARKER_H, None);
    let left = row.min.x + NAMES_W;
    let width = (row.max.x - left).max(0.0);
    markers(ui, row, left, width, &music.markers);

    for (group, title) in [(LaneGroup::Fighters, "Fighters"), (LaneGroup::Band, "Band")] {
        let parts: Vec<&Lane> = music.lanes.iter().filter(|l| l.group == group).collect();
        if parts.is_empty() {
            continue;
        }
        let header = lane_row(ui, GROUP_H, Some(GROUP));
        cell(
            ui,
            header.shrink2(vec2(NAME_PAD.0, 0.0)),
            ("group", title),
            Layout::left_to_right(Align::Center),
            |ui| plain(ui, RichText::new(title).font(section()).color(TEXT)),
        );
        for (i, lane) in parts.into_iter().enumerate() {
            let rect = lane_row(ui, LANE_H, None);
            let names = Rect::from_x_y_ranges(rect.min.x..=left, rect.y_range());
            let names = Rect::from_min_max(
                names.min + vec2(NAME_PAD.0, NAME_PAD.1),
                names.max - vec2(NAME_PAD.0, 0.0),
            );
            cell(ui, names, ("lane", title, i), Layout::top_down(Align::Min), |ui| {
                plain(ui, RichText::new(&lane.name).font(small()).color(MUTED))
            });
            let (fill, edge) = match lane.color {
                Some(color) => (large_fill(color), rgb(color)),
                None => (BAND_CLIP, MUTED),
            };
            let painter = ui
                .painter()
                .with_clip_rect(Rect::from_x_y_ranges(left..=rect.max.x, rect.y_range()).intersect(ui.clip_rect()));
            for &(start, end) in &lane.clips {
                let Some((x0, x1)) = clip_x(start, end, left, width) else {
                    continue;
                };
                let top = rect.min.y + CLIP_TOP;
                painter.rect_filled(Rect::from_x_y_ranges(x0..=x1, top..=top + CLIP_H), 0.0, fill);
                painter.rect_filled(Rect::from_x_y_ranges(x0..=x1, top..=top + 1.0), 0.0, edge);
            }
        }
    }
}

/// The marker lane's markers: each a 1 px `FAINT` line with its small `MUTED` name 4 to its right when the name fits
/// whole before the next marker, and nothing else when it does not (ruling M2-R25). Every marker is a hover area
/// round its line that shows its whole name, and carries that name for screen readers (role `Mark`); the words
/// beside it are hidden from them, so the name is read once.
fn markers(ui: &mut Ui, row: Rect, left: f32, width: f32, markers: &[(f32, String)]) {
    let mut shown: Vec<(f32, &str)> = markers
        .iter()
        .filter(|(t, _)| t.is_finite())
        .map(|(t, name)| (x_of(t.clamp(0.0, DUR), left, width).round(), name.as_str()))
        .collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    let line_y = row.min.y..=row.max.y - 1.0;
    for (i, &(x, name)) in shown.iter().enumerate() {
        let x = x.min(row.max.x - 1.0);
        ui.painter()
            .rect_filled(Rect::from_x_y_ranges(x..=x + 1.0, line_y.clone()), 0.0, FAINT);
        let hover = Rect::from_x_y_ranges(x - MARKER_HOVER..=x + 1.0 + MARKER_HOVER, line_y.clone());
        let marker = ui.interact(hover, ui.scope_id().with(("marker", i)), Sense::hover());
        marker.widget_info(|| WidgetInfo::labeled(Role::Mark, true, name));
        marker.on_hover_text(name);
        let next = shown.get(i + 1).map_or(row.max.x, |&(next, _)| next);
        let from = x + 1.0 + MARKER_GAP;
        let room = next - MARKER_GAP - from;
        let needs = ui.painter().layout_no_wrap(name.to_owned(), small(), MUTED).size().x;
        if needs <= room {
            let text = Rect::from_x_y_ranges(from..=from + room, line_y.clone());
            let words = cell(
                ui,
                text,
                ("marker words", i),
                Layout::left_to_right(Align::Center),
                |ui| plain(ui, RichText::new(name).font(small()).color(MUTED)),
            );
            ui.ctx().accesskit_node_builder(words.id, |node| node.set_hidden());
        }
    }
}

/// The block under the lanes: "Loudness", what it is, one row per check, what a failed check means with the next
/// fight that fits ("Use seed 7731", or "Go to seeds" when no other fits), and what arrives later.
fn loudness(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let music = vm.take.music.as_ref();
    Frame::new()
        .inner_margin(Margin::symmetric(UNDER_PAD.0, UNDER_PAD.1))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = vec2(ROW_GAP, 0.0);
            plain(ui, RichText::new("Loudness").font(section()).color(TEXT));
            ui.add_space(UNDER_GAP);
            hint(
                ui,
                "Checked as soon as the take is built, so a problem shows here and not in the queue.",
            );
            for check in music.map_or(&[][..], |m| &m.checks[..]) {
                ui.add_space(UNDER_GAP);
                let (icon, colour) = if check.pass {
                    (Icon::Done, TEXT)
                } else {
                    (Icon::Todo, TEXT_STRONG)
                };
                ui.horizontal_top(|ui| {
                    let line_h = ui.text_style_height(&egui::TextStyle::Body);
                    let (slot, _) = ui.allocate_exact_size(vec2(ICON, line_h), Sense::hover());
                    let icon_box = Rect::from_center_size(slot.center(), vec2(ICON, ICON)).round();
                    icons::paint(ui.painter(), icon_box, icon, colour);
                    ui.allocate_ui_with_layout(vec2(CHECK_TITLE_W, line_h), Layout::left_to_right(Align::Min), |ui| {
                        ui.set_width(CHECK_TITLE_W);
                        plain(ui, RichText::new(&check.title).font(body()).color(colour));
                    });
                    ui.vertical(|ui| {
                        wrapped(ui, RichText::new(&check.note).font(body()).color(TEXT));
                    });
                });
            }
            if let Some(words) = vm.failed_take_words() {
                ui.add_space(UNDER_GAP);
                wrapped(ui, RichText::new(words).font(body()).color(TEXT_STRONG));
                ui.add_space(UNDER_GAP);
                match vm.next_fit() {
                    Some(next) => {
                        if buttons::boxed(ui, &format!("Use seed {next}")).clicked() {
                            out.push(Action::SelectSeed(next));
                        }
                    }
                    None => {
                        if buttons::boxed(ui, "Go to seeds").clicked() {
                            out.push(Action::Go(View::Step(Step::Seeds)));
                        }
                    }
                }
            }
            ui.add_space(UNDER_GAP);
            hint(
                ui,
                "Music that follows the fight, with fills on lead changes and fresh lines for every take, arrives \
                 with the music system.",
            );
        });
}

/// The footer: "Sounds right?" and "Go to render".
fn footer_row(ui: &mut Ui, inside: Rect, out: &mut Vec<Action>) {
    cell(ui, inside, "footer", Layout::right_to_left(Align::Center), |ui| {
        if buttons::primary(ui, "Go to render").clicked() {
            out.push(Action::Go(View::Step(Step::Render)));
        }
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            plain(ui, RichText::new("Sounds right?").font(body()).color(MUTED));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_are_whole_pixels_inside_the_video() {
        assert_eq!(clip_x(3.6, 20.0, 412.0, 610.0), Some((448.0, 612.0)));
        assert_eq!(clip_x(10.0, 10.1, 0.0, 61.0), Some((10.0, 16.0)), "at least 6 wide");
        assert_eq!(
            clip_x(-5.0, 99.0, 0.0, 61.0),
            Some((0.0, 61.0)),
            "kept inside 0 to 61 s"
        );
        assert_eq!(
            clip_x(30.0, 20.0, 0.0, 61.0),
            Some((30.0, 36.0)),
            "an end before its start"
        );
        assert_eq!(clip_x(f32::NAN, 20.0, 0.0, 61.0), None);
        assert_eq!(clip_x(1.0, f32::INFINITY, 0.0, 61.0), None);
    }
}
