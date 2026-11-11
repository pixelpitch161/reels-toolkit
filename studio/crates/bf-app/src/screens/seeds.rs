//! The Seeds step's workspace (interface reference, section 5.3): the filters on the left, the shortlist with its
//! header and footer in the rest.
//!
//! The screen is drawn from the view model alone and pushes what the owner does; the only things it remembers are
//! egui's own: the text typed into the watch box, and where the list is scrolled, which row has the keyboard and
//! which seed was chosen when last drawn (so a newly chosen row is scrolled into view).
//!
//! The shortlist can hold a few hundred rows, so only the rows in view are laid out (`ScrollArea::show_rows`). The
//! arrows up and down move the choice while a row has the keyboard: the next row is chosen (`SelectSeed`), scrolled
//! into view and given the keyboard. The transport's keys (Space, left and right) are left alone, and the transport
//! itself yields while a row has the keyboard.

use egui::text::LayoutJob;
use egui::{
    Align, EventFilter, Id, Key, Label, Layout, Modifiers, Painter, Rect, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, vec2,
};

use crate::model::shortlist::{FilterSpec, Finish, SearchState};
use crate::model::story::StoryView;
use crate::state::{Action, ViewModel};
use crate::theme::{
    LINE_SOFT, MUTED, RADIUS_ROW, TEXT, TEXT_STRONG, VALUE_W, body, rgb, section, small, strong, value as mono,
};
use crate::widgets::buttons;
use crate::widgets::chips::{fighter_bar, object_row};
use crate::widgets::fields::{self, Unit};
use crate::widgets::ribbon;
use crate::wording;

/// The filters column's width.
pub const FILTERS_W: f32 = 230.0;
/// Its padding: 12 at the sides, 14 above and below.
const FILTERS_PAD: (f32, f32) = (12.0, 14.0);
/// The gap between two groups of the filters column.
const GROUP_GAP: f32 = 14.0;
/// The gap between a group's label and its control.
const LABEL_GAP: f32 = 5.0;
/// The shortlist's header and footer.
const HEADER_H: f32 = 30.0;
const FOOTER_H: f32 = 46.0;
/// The side padding of the header and the footer.
const SIDE_PAD: f32 = 14.0;
/// The list's padding: 4 at the sides and 4 above and below (the reference says 2 x 4; 4 leaves room for the focus
/// ring, 2 px wide and 2 px outside, of the top and bottom rows).
const LIST_PAD: (f32, f32) = (4.0, 4.0);
/// The focus ring of a row, as the object row paints it: 2 px `TEXT_STRONG`, 2 px outside, corner 8 + 2.
const RING_W: f32 = 2.0;
const RING_GAP: f32 = 2.0;
/// A row's height and its side padding.
const ROW_H: f32 = 38.0;
const ROW_PAD: f32 = 10.0;
/// The gap between two columns.
const COL_GAP: f32 = 12.0;
/// The fixed columns: rank, seed, K.O., lead changes.
const RANK_W: f32 = 30.0;
const SEED_W: f32 = 62.0;
const KO_W: f32 = 58.0;
const CHANGES_W: f32 = 58.0;
/// The least the story score's column takes.
const SCORE_W: f32 = 48.0;
/// What a row's fixed parts take: its padding, the rank, seed, K.O. and changes columns, the six gaps and the
/// story score: 348.
const FIXED: f32 = 2.0 * ROW_PAD + RANK_W + SEED_W + KO_W + CHANGES_W + 6.0 * COL_GAP + SCORE_W;
/// The winner column, with and without the HP left; the room it needs to show the HP.
const WINNER_WIDE: f32 = 150.0;
const WINNER_NARROW: f32 = 96.0;
const HP_ROOM: f32 = 350.0;
/// The ribbon column's bounds.
const RIBBON_MIN: f32 = 120.0;
const RIBBON_MAX: f32 = 400.0;
/// The header's colour swatches.
const SWATCH: f32 = 8.0;
/// The gap between the watch box and its button.
const WATCH_GAP: f32 = 4.0;
/// The K.O. filter's range, in seconds (the state keeps the bounds inside it too), and the lead changes'. A value
/// field steps a hundredth of its range per arrow press, so 0 to 100 steps the whole number by one.
const KO_RANGE: (f64, f64) = (30.0, 58.0);
const CHANGES_RANGE: (f64, f64) = (0.0, 100.0);

/// The switch's words.
pub const SHOW_OUTSIDE: &str = "Show fights outside the beat sheet";
/// Why "Use seed" cannot be used yet.
pub const PICK_FIRST: &str = "Pick a fight first";
/// Why "Watch" cannot be used.
pub const WHOLE_NUMBER: &str = "Type a whole number";
/// The empty list while the search runs.
pub const EMPTY_SEARCHING: &str = "No fights fit the beat sheet yet. The search is still running.";
/// What stands in for a value that is not there: a rank outside the beat sheet, a missing K.O.
const NONE: &str = "–";

/// How wide the shortlist's flexible columns are for a row `row_width` wide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeedColumns {
    /// The lead ribbon.
    pub ribbon: f32,
    /// The winner.
    pub winner: f32,
    /// The winner column has room for "16 HP left".
    pub hp_left: bool,
}

/// The fixed parts take 348; with what is left `r`, the winner column is 150 and shows the HP left when `r >= 350`,
/// else 96 without it; the ribbon takes `r - winner`, kept in 120 to 400. A width that is not a number counts as 0.
pub fn seed_columns(row_width: f32) -> SeedColumns {
    let rest = if row_width.is_finite() {
        (row_width - FIXED).max(0.0)
    } else {
        0.0
    };
    let hp_left = rest >= HP_ROOM;
    let winner = if hp_left { WINNER_WIDE } else { WINNER_NARROW };
    SeedColumns {
        ribbon: (rest - winner).clamp(RIBBON_MIN, RIBBON_MAX),
        winner,
        hp_left,
    }
}

/// A seed typed into the watch box: a whole number from 0 to 9,223,372,036,854,775,807, digits only, with spaces
/// around it allowed.
pub fn parse_seed(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok().filter(|&seed| seed <= i64::MAX as u64)
}

/// What the screen keeps in egui's memory between frames.
#[derive(Clone, Copy, Debug, Default)]
struct ListMemory {
    /// The list's scroll offset when last drawn.
    offset: f32,
    /// The row that had the keyboard when last drawn: its id and seed.
    focused: Option<(Id, u64)>,
    /// The row to give the keyboard to when it is drawn.
    focus_next: Option<u64>,
    /// The chosen seed when last drawn.
    chosen: Option<u64>,
}

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let full = ui.max_rect();
    let split = (full.min.x + FILTERS_W).min(full.max.x);
    let filters = Rect::from_x_y_ranges(full.min.x..=split, full.y_range());
    let list = Rect::from_x_y_ranges(split..=full.max.x.max(split), full.y_range());
    ui.painter().rect_filled(
        Rect::from_x_y_ranges(split - 1.0..=split, full.y_range()),
        0.0,
        LINE_SOFT,
    );
    let mut column = ui.new_child(UiBuilder::new().id_salt("seeds filters").max_rect(filters));
    filters_column(&mut column, vm, out);
    let mut shortlist = ui.new_child(UiBuilder::new().id_salt("seeds shortlist").max_rect(list));
    shortlist_column(&mut shortlist, vm, out);
}

// ---------------------------------------------------------------------------------------------------------------
// The filters
// ---------------------------------------------------------------------------------------------------------------

/// Text that is read, not clicked or selected, cut short with an ellipsis when it does not fit.
fn plain(ui: &mut Ui, text: RichText) -> egui::Response {
    ui.add(Label::new(text).selectable(false).truncate())
}

/// A group's label: body, `MUTED`.
fn group_label(ui: &mut Ui, text: &str) {
    plain(ui, RichText::new(text).font(body()).color(MUTED));
    ui.add_space(LABEL_GAP);
}

/// A 1 px `LINE_SOFT` divider across `ui`.
fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, LINE_SOFT);
}

/// Pushes `spec` when it differs from what the view model has.
fn push_filters(vm: &ViewModel, spec: FilterSpec, out: &mut Vec<Action>) {
    if spec != vm.filters {
        out.push(Action::SetFilters(spec));
    }
}

fn filters_column(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let inner = ui.max_rect().shrink2(vec2(FILTERS_PAD.0, FILTERS_PAD.1));
    let width = inner.width().max(0.0);
    let mut top = ui.new_child(
        UiBuilder::new()
            .id_salt("top")
            .max_rect(inner)
            .layout(Layout::top_down(Align::Min)),
    );
    top.spacing_mut().item_spacing.y = 0.0;
    let ui = &mut top;
    let names = vm.names();
    let filters = &vm.filters;

    plain(ui, RichText::new("Filters").font(section()).color(TEXT));
    ui.add_space(GROUP_GAP);

    group_label(ui, "Winner");
    let options = [("Any", None), (names[0].as_str(), None), (names[1].as_str(), None)];
    let chosen = filters.winner.filter(|&w| w < 2).map_or(0, |w| w + 1);
    if let Some(i) = buttons::segmented(ui, "Winner", &options, Some(chosen), width) {
        let winner = i.checked_sub(1);
        push_filters(
            vm,
            FilterSpec {
                winner,
                ..filters.clone()
            },
            out,
        );
    }
    ui.add_space(GROUP_GAP);

    group_label(ui, "K.O. lands between");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let and = ui.painter().layout_no_wrap("and".to_owned(), body(), MUTED).size().x;
        let field_w = ((width - and - 16.0) / 2.0).floor();
        let unit = Unit {
            prefix: "",
            suffix: " s",
            decimals: 1,
        };
        let (mut from, mut to) = filters.ko;
        fields::value_field(ui, "Earliest K.O.", &mut from, KO_RANGE, unit, field_w);
        plain(ui, RichText::new("and").font(body()).color(MUTED));
        fields::value_field(ui, "Latest K.O.", &mut to, KO_RANGE, unit, field_w);
        if (from, to) != filters.ko {
            push_filters(
                vm,
                FilterSpec {
                    ko: (from, to),
                    ..filters.clone()
                },
                out,
            );
        }
    });
    ui.add_space(GROUP_GAP);

    group_label(ui, "Lead changes");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        plain(ui, RichText::new("At least").font(body()).color(MUTED));
        let mut changes = f64::from(filters.min_lead_changes);
        let unit = Unit {
            prefix: "",
            suffix: "",
            decimals: 0,
        };
        fields::value_field(ui, "Lead changes, at least", &mut changes, CHANGES_RANGE, unit, VALUE_W);
        let changes = changes.round() as u32;
        if changes != filters.min_lead_changes {
            push_filters(
                vm,
                FilterSpec {
                    min_lead_changes: changes,
                    ..filters.clone()
                },
                out,
            );
        }
    });
    ui.add_space(GROUP_GAP);

    group_label(ui, "Finish");
    let finishes = [Finish::Any, Finish::Close, Finish::Comeback];
    let options = [("Any", None), ("Close", None), ("Comeback", None)];
    let chosen = finishes.iter().position(|f| *f == filters.finish);
    if let Some(i) = buttons::segmented(ui, "Finish", &options, chosen, width) {
        push_filters(
            vm,
            FilterSpec {
                finish: finishes[i],
                ..filters.clone()
            },
            out,
        );
    }
    ui.add_space(GROUP_GAP);

    group_label(ui, "This ability lands");
    // "Any ability", then the left card's three abilities and the right card's.
    let abilities: Vec<(&str, &str)> = [0, 1]
        .iter()
        .filter_map(|&side| vm.fighter(side)?.card.as_ref())
        .flat_map(|card| card.abilities.iter().map(|a| (a.name.as_str(), a.key.as_str())))
        .collect();
    let mut options = vec!["Any ability".to_string()];
    options.extend(abilities.iter().map(|(name, _)| (*name).to_string()));
    let chosen = filters
        .ability
        .as_deref()
        .and_then(|key| abilities.iter().position(|(_, k)| *k == key))
        .map_or(0, |i| i + 1);
    if let Some(i) = buttons::dropdown(ui, "This ability lands", &options, chosen, width) {
        let ability = i.checked_sub(1).map(|i| abilities[i].1.to_string());
        push_filters(
            vm,
            FilterSpec {
                ability,
                ..filters.clone()
            },
            out,
        );
    }
    ui.add_space(GROUP_GAP);

    divider(ui);
    ui.add_space(GROUP_GAP);
    show_outside(ui, vm, out);

    watch_box(ui, inner, out);
}

/// The switch "Show fights outside the beat sheet". Its words take two lines in the column, so the switch is drawn
/// without them and named by them, and the words beside it, wrapped, also turn it over when clicked.
fn show_outside(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let on = vm.filters.show_outside;
    let clicked = ui
        .horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let switch = buttons::switch(ui, "", on).accessible_name(SHOW_OUTSIDE);
            ui.vertical(|ui| {
                // The first line sits level with the switch, which is centred in its 26.
                ui.add_space(4.0);
                let words = Label::new(RichText::new(SHOW_OUTSIDE).font(body()).color(TEXT))
                    .wrap()
                    .selectable(false)
                    .sense(Sense::CLICK);
                let words = ui.add(words);
                // The switch carries the name; the words are not read out a second time.
                ui.ctx().accesskit_node_builder(words.id, |node| node.set_hidden());
                words.clicked()
            })
            .inner
                || switch.clicked()
        })
        .inner;
    if clicked {
        push_filters(
            vm,
            FilterSpec {
                show_outside: !on,
                ..vm.filters.clone()
            },
            out,
        );
    }
}

/// At the bottom of the column, under a divider: "Watch any seed", the box for a seed number and "Watch", which
/// asks for that seed (Enter in the box does the same) and says why when what is typed is not a seed.
fn watch_box(ui: &mut Ui, inner: Rect, out: &mut Vec<Action>) {
    let mut bottom = ui.new_child(
        UiBuilder::new()
            .id_salt("watch")
            .max_rect(inner)
            .layout(Layout::bottom_up(Align::Min)),
    );
    let ui = &mut bottom;
    ui.spacing_mut().item_spacing.y = 0.0;
    let text_id = ui.scope_id().with("seed to watch");
    let mut text: String = ui.data(|d| d.get_temp(text_id)).unwrap_or_default();
    let seed = parse_seed(&text);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = WATCH_GAP;
        let button_w = ui.painter().layout_no_wrap("Watch".to_owned(), body(), TEXT).size().x + 20.0;
        let field_w = (inner.width() - button_w.ceil() - WATCH_GAP).floor();
        let field = fields::text_line(ui, "Seed to watch", &mut text, "Seed number", field_w);
        let entered = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let watch = match seed {
            Some(_) => buttons::boxed(ui, "Watch").clicked(),
            None => {
                buttons::disabled(ui, "Watch", WHOLE_NUMBER);
                false
            }
        };
        if let Some(seed) = seed.filter(|_| watch || entered) {
            out.push(Action::SelectSeed(seed));
        }
    });
    ui.data_mut(|d| d.insert_temp(text_id, text));
    ui.add_space(LABEL_GAP);
    plain(ui, RichText::new("Watch any seed").font(body()).color(MUTED));
    ui.add_space(GROUP_GAP);
    divider(ui);
}

// ---------------------------------------------------------------------------------------------------------------
// The shortlist
// ---------------------------------------------------------------------------------------------------------------

/// The seven columns of a row (or of the header) whose inside is `inside`: rank, seed, ribbon, winner, K.O.,
/// changes and the story score, which takes the rest.
fn column_rects(inside: Rect, cols: SeedColumns) -> [Rect; 7] {
    let mut x = inside.min.x;
    let mut next = |w: f32| {
        let r = Rect::from_x_y_ranges(x..=x + w, inside.y_range());
        x += w + COL_GAP;
        r
    };
    let rank = next(RANK_W);
    let seed = next(SEED_W);
    let lead = next(cols.ribbon);
    let winner = next(cols.winner);
    let ko = next(KO_W);
    let changes = next(CHANGES_W);
    let story = Rect::from_x_y_ranges(x.min(inside.max.x)..=inside.max.x, inside.y_range());
    [rank, seed, lead, winner, ko, changes, story]
}

/// A child of `ui` in `rect`, laid out from the left (or from the right when `right`), centred in its height and
/// clipped to it.
fn cell<R>(ui: &mut Ui, rect: Rect, salt: usize, right: bool, add: impl FnOnce(&mut Ui) -> R) -> R {
    let layout = if right {
        Layout::right_to_left(Align::Center)
    } else {
        Layout::left_to_right(Align::Center)
    };
    let mut child = ui.new_child(UiBuilder::new().id_salt(("cell", salt)).max_rect(rect).layout(layout));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    add(&mut child)
}

fn shortlist_column(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let area = ui.max_rect();
    let header = Rect::from_min_size(area.min, vec2(area.width(), HEADER_H));
    let footer = Rect::from_x_y_ranges(area.x_range(), (area.max.y - FOOTER_H)..=area.max.y);
    let list = Rect::from_x_y_ranges(area.x_range(), header.max.y..=footer.min.y.max(header.max.y));
    let painter = ui.painter().clone();
    painter.rect_filled(
        Rect::from_x_y_ranges(area.x_range(), header.max.y - 1.0..=header.max.y),
        0.0,
        LINE_SOFT,
    );
    painter.rect_filled(
        Rect::from_x_y_ranges(area.x_range(), footer.min.y..=footer.min.y + 1.0),
        0.0,
        LINE_SOFT,
    );
    let rows_rect = list.shrink2(vec2(LIST_PAD.0, LIST_PAD.1));
    let cols = seed_columns(rows_rect.width());
    header_row(ui, vm, header.shrink2(vec2(SIDE_PAD, 0.0)), cols);
    if vm.shortlist.rows.is_empty() {
        empty_list(ui, vm, list);
    } else {
        rows(ui, vm, rows_rect, cols, out);
    }
    footer_row(ui, vm, footer.shrink2(vec2(SIDE_PAD, 0.0)), out);
}

/// The header: no title over the rank, "Seed", "Who leads" with a swatch and "Flute above", a swatch and "Trombone
/// below", "Winner", and right-aligned "K.O.", "Changes" and "Story", all small `MUTED`.
fn header_row(ui: &mut Ui, vm: &ViewModel, inside: Rect, cols: SeedColumns) {
    let [_, seed, lead, winner, ko, changes, story] = column_rects(inside, cols);
    let word = |ui: &mut Ui, text: &str| {
        plain(ui, RichText::new(text).font(small()).color(MUTED));
    };
    cell(ui, seed, 1, false, |ui| word(ui, "Seed"));
    cell(ui, lead, 2, false, |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let names = vm.names();
        let colors = vm.colors();
        word(ui, "Who leads");
        for (side, place) in [(0, "above"), (1, "below")] {
            let (rect, _) = ui.allocate_exact_size(vec2(SWATCH, SWATCH), Sense::hover());
            ui.painter().rect_filled(rect, 0.0, rgb(colors[side]));
            word(ui, &format!("{} {place}", names[side]));
        }
    });
    cell(ui, winner, 3, false, |ui| word(ui, "Winner"));
    cell(ui, ko, 4, true, |ui| word(ui, "K.O."));
    cell(ui, changes, 5, true, |ui| word(ui, "Changes"));
    cell(ui, story, 6, true, |ui| word(ui, "Story"));
}

/// The list with no rows: one centred `MUTED` line that says what to do.
fn empty_list(ui: &mut Ui, vm: &ViewModel, list: Rect) {
    let text = match vm.shortlist.state {
        SearchState::Searching | SearchState::Idle => EMPTY_SEARCHING.to_string(),
        SearchState::Stopped | SearchState::Full => format!(
            "No fights fit the beat sheet in {} checked. Keep searching, or use the switch on the left to show \
             fights outside the beat sheet.",
            wording::thousands(vm.shortlist.checked)
        ),
    };
    let wrap = (list.width() - 2.0 * SIDE_PAD).max(0.0);
    let mut job = LayoutJob::simple(text.clone(), body(), MUTED, wrap);
    job.halign = Align::Center;
    let height = ui.painter().layout_job(job).size().y;
    let rect = Rect::from_center_size(list.center(), vec2(wrap, height));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("empty")
            .max_rect(rect)
            .layout(Layout::top_down(Align::Center)),
    );
    child.add(
        Label::new(RichText::new(text).font(body()).color(MUTED))
            .wrap()
            .selectable(false)
            .halign(Align::Center),
    );
}

/// What a row's winner and K.O. say: the winner's index and the K.O. time, when the fight has both and they can be
/// shown.
fn ending(story: &StoryView) -> Option<(usize, f64)> {
    let winner = story.winner.filter(|&w| w < 2)?;
    let t = story.t_ko.filter(|t| t.is_finite() && *t >= 0.0)?;
    Some((winner, t))
}

/// A row's accessible name: "Seed 18433, Trombone wins at 49.1 s", or "Seed 13, no K.O. by 58 s".
fn row_name(story: &StoryView, names: &[String; 2]) -> String {
    match ending(story) {
        Some((winner, t)) => format!("Seed {}, {} wins at {}", story.seed, names[winner], wording::seconds(t)),
        None => format!("Seed {}, no K.O. by {:.0} s", story.seed, KO_RANGE.1),
    }
}

/// The rows in view, in a scroll area over `rect`. Keeps the chosen row in view when the choice changes, and moves
/// the choice with the arrows while a row has the keyboard.
fn rows(ui: &mut Ui, vm: &ViewModel, rect: Rect, cols: SeedColumns, out: &mut Vec<Action>) {
    let list = &vm.shortlist.rows;
    let memory_id = ui.scope_id().with("shortlist memory");
    let mut memory: ListMemory = ui.data(|d| d.get_temp(memory_id)).unwrap_or_default();
    let index_of = |seed: u64| list.iter().position(|s| s.seed == seed);

    // The arrows, while a row has the keyboard.
    let mut reveal = None;
    let focused_now = ui.memory(|m| m.focused());
    if let Some((_, seed)) = memory.focused.filter(|(id, _)| Some(*id) == focused_now)
        && let Some(at) = index_of(seed)
    {
        let (down, up) = ui.input_mut(|i| {
            (
                i.count_and_consume_key(Modifiers::NONE, Key::ArrowDown),
                i.count_and_consume_key(Modifiers::NONE, Key::ArrowUp),
            )
        });
        let to = (at + down).saturating_sub(up).min(list.len() - 1);
        if to != at {
            let seed = list[to].seed;
            if vm.draft.seed != Some(seed) {
                out.push(Action::SelectSeed(seed));
            }
            memory.focus_next = Some(seed);
            reveal = Some(to);
        }
    }
    // A choice made anywhere else (a click, the watch box) is scrolled into view once.
    if vm.draft.seed != memory.chosen {
        memory.chosen = vm.draft.seed;
        reveal = reveal.or_else(|| vm.draft.seed.and_then(index_of));
    }

    let mut area = ScrollArea::vertical()
        .id_salt("shortlist rows")
        .auto_shrink([false, false]);
    if let Some(i) = reveal {
        let top = i as f32 * ROW_H;
        let view = rect.height();
        let offset = if top < memory.offset {
            top
        } else if top + ROW_H > memory.offset + view {
            top + ROW_H - view
        } else {
            memory.offset
        };
        area = area.vertical_scroll_offset(offset.max(0.0));
    }

    let names = vm.names();
    let colors = vm.colors();
    let mut child = ui.new_child(UiBuilder::new().id_salt("rows").max_rect(rect));
    child.spacing_mut().item_spacing.y = 0.0;
    let mut focused = None;
    let mut ring = None;
    let output = area.show_rows(&mut child, ROW_H, list.len(), |ui, range| {
        // Ranks count only the fights that fit the beat sheet.
        let mut rank = list[..range.start].iter().filter(|s| s.fits).count();
        for story in &list[range] {
            if story.fits {
                rank += 1;
            }
            let shown_rank = story.fits.then_some(rank);
            let chosen = vm.draft.seed == Some(story.seed);
            let name = row_name(story, &names);
            let id = memory_id.with(("row", story.seed));
            let response = ui
                .scope_builder(UiBuilder::new().scope_id(id), |ui| {
                    object_row(ui, &name, chosen, ROW_H, |ui| {
                        row_cells(ui, story, shown_rank, chosen, cols, &names, colors);
                    })
                    .0
                })
                .inner;
            if memory.focus_next == Some(story.seed) {
                response.request_focus();
                memory.focus_next = None;
            }
            if ui.memory(|m| m.has_focus(response.id)) {
                // The arrows move the choice; without this egui would also move the keyboard to a neighbour.
                let filter = EventFilter {
                    vertical_arrows: true,
                    ..EventFilter::default()
                };
                ui.memory_mut(|m| m.set_focus_lock_filter(response.id, filter));
                focused = Some((response.id, story.seed));
                ring = Some(response.rect);
            }
            if response.clicked() {
                // A clicked row takes the keyboard, so the arrows go on from it.
                response.request_focus();
                if !chosen {
                    out.push(Action::SelectSeed(story.seed));
                }
            }
        }
    });
    // The focused row's ring once more, over the rows after it (the chosen row's fill would cover its lower edge),
    // clipped to the list and its padding.
    if let Some(row) = ring {
        let clip = rect.expand(LIST_PAD.1).intersect(ui.clip_rect());
        let painter = Painter::new(ui.ctx().clone(), child.layer_id(), clip);
        painter.rect_stroke(
            row.expand(RING_GAP),
            f32::from(RADIUS_ROW) + RING_GAP,
            Stroke::new(RING_W, TEXT_STRONG),
            StrokeKind::Outside,
        );
    }
    memory.offset = output.state.clamped_offset().y;
    memory.focused = focused;
    ui.data_mut(|d| d.insert_temp(memory_id, memory));
}

/// One row's content: rank (mono `MUTED`, "–" outside the beat sheet), seed (mono), the lead ribbon, the winner (its
/// bar, its name, "16 HP left" in small `MUTED` when there is room), the K.O. time, the lead changes and the story
/// score (mono, two decimals), or in place of the score the first miss in `MUTED` for a fight outside the beat sheet.
fn row_cells(
    ui: &mut Ui,
    story: &StoryView,
    rank: Option<usize>,
    chosen: bool,
    cols: SeedColumns,
    names: &[String; 2],
    colors: [[u8; 3]; 2],
) {
    let [rank_r, seed_r, lead_r, winner_r, ko_r, changes_r, story_r] = column_rects(ui.max_rect(), cols);
    let mono_text = |text: String| RichText::new(text).font(mono());
    let rank = rank.map_or_else(|| NONE.to_string(), |r| r.to_string());
    cell(ui, rank_r, 0, false, |ui| plain(ui, mono_text(rank).color(MUTED)));
    cell(ui, seed_r, 1, false, |ui| plain(ui, mono_text(story.seed.to_string())));
    cell(ui, lead_r, 2, false, |ui| {
        ribbon::small(ui, story, colors, lead_r.width())
    });
    let ending = ending(story);
    cell(ui, winner_r, 3, false, |ui| {
        let Some((winner, _)) = ending else {
            plain(ui, mono_text(NONE.to_string()).color(MUTED));
            return;
        };
        fighter_bar(ui, colors[winner], 18.0);
        let font = if chosen { strong() } else { body() };
        plain(ui, RichText::new(&names[winner]).font(font));
        if cols.hp_left && story.winner_hp.is_finite() && story.winner_hp >= 0.0 {
            let hp = format!("{} HP left", wording::thousands(story.winner_hp.round() as u64));
            plain(ui, RichText::new(hp).font(small()).color(MUTED));
        }
    });
    // A fight outside the beat sheet shows its first miss across the K.O., changes and story columns (ruling
    // M2-R23), cut short when it does not fit; hovering it shows all of it, and a screen reader gets all of it.
    if let Some(miss) = story.misses.first().filter(|_| !story.fits) {
        let span = Rect::from_x_y_ranges(ko_r.min.x..=story_r.max.x, ko_r.y_range());
        cell(ui, span, 4, false, |ui| {
            let words = Label::new(RichText::new(miss).font(small()).color(MUTED))
                .selectable(false)
                .truncate()
                .show_tooltip_when_elided(false);
            ui.add(words).on_hover_text(miss);
        });
        return;
    }
    let ko = ending.map_or_else(|| NONE.to_string(), |(_, t)| wording::seconds(t));
    cell(ui, ko_r, 4, true, |ui| plain(ui, mono_text(ko)));
    cell(ui, changes_r, 5, true, |ui| {
        plain(ui, mono_text(story.lead_changes.to_string()))
    });
    cell(ui, story_r, 6, true, |ui| {
        if story.score.is_finite() {
            plain(ui, mono_text(format!("{:.2}", story.score)))
        } else {
            plain(ui, mono_text(NONE.to_string()).color(MUTED))
        }
    });
}

/// The footer: what the search has found, the button that stops or resumes it, and "Use seed {n}".
fn footer_row(ui: &mut Ui, vm: &ViewModel, inside: Rect, out: &mut Vec<Action>) {
    let shortlist = &vm.shortlist;
    let fits = wording::plural(u64::from(shortlist.fits_total), "fight fits", "fights fit");
    let checked = wording::thousands(shortlist.checked);
    let (state, button) = match shortlist.state {
        SearchState::Searching | SearchState::Idle => ("Still searching.", Some(("Stop search", Action::StopSearch))),
        SearchState::Stopped => ("Search stopped.", Some(("Keep searching", Action::SearchOn))),
        SearchState::Full => ("The shortlist is full.", None),
    };
    let words = format!("{fits} the beat sheet, from {checked} checked. {state}");
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt("footer")
            .max_rect(inside)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let ui = &mut child;
    match (vm.draft.seed, &vm.take.problem) {
        (None, _) => {
            buttons::disabled(ui, "Use seed", PICK_FIRST);
        }
        (Some(seed), Some(problem)) => {
            buttons::disabled(ui, &format!("Use seed {seed}"), problem);
        }
        (Some(seed), None) => {
            if buttons::primary(ui, &format!("Use seed {seed}")).clicked() {
                out.push(Action::UseSeed);
            }
        }
    }
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let button_w = button.as_ref().map_or(0.0, |(label, _)| {
            let w = ui.painter().layout_no_wrap((*label).to_owned(), body(), TEXT).size().x;
            (w + 20.0).ceil() + ui.spacing().item_spacing.x
        });
        let text_w = (ui.available_width() - button_w).max(0.0);
        ui.allocate_ui_with_layout(
            vec2(text_w, inside.height()),
            Layout::left_to_right(Align::Center),
            |ui| {
                plain(ui, RichText::new(words).font(body()).color(MUTED));
            },
        );
        if let Some((label, action)) = button
            && buttons::boxed(ui, label).clicked()
        {
            out.push(action);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixed_parts_add_up() {
        assert_eq!(FIXED, 348.0);
    }

    #[test]
    fn a_row_with_no_ko_is_named_so() {
        let mut story = crate::seam::sample::story(18433);
        let names = ["Flute".to_string(), "Trombone".to_string()];
        assert_eq!(row_name(&story, &names), "Seed 18433, Trombone wins at 49.1 s");
        story.t_ko = Some(f64::NAN);
        assert_eq!(row_name(&story, &names), "Seed 18433, no K.O. by 58 s");
        story.t_ko = None;
        story.winner = None;
        assert_eq!(row_name(&story, &names), "Seed 18433, no K.O. by 58 s");
    }
}
