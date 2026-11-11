//! The story timeline (interface reference, section 4): the transport row and the lanes under it.
//!
//! The transport row (30 tall under the panel's 1 px top line, padded 10 at each side, a 1 px `LINE_SOFT` below):
//! the play or pause button, the time ("0:31.40 / 1:01.00"), what the sound is doing when it cannot be heard yet, a
//! spacer, on Seeds the hint about scrubbing, then the "Loop" and "Safe zone" toggles. Under it the story, 81 tall:
//! a 120 wide column naming the three lanes, then the lanes, 0 to 61 s left to right: each fighter's first use of each
//! ability (left fighter above, right below) and the lead ribbon between them, with the 1 px playhead across all
//! three. Before a seed is picked the story holds one line saying so and the controls are disabled.
//!
//! The lanes are one control, "Story timeline" (a slider whose value is the playhead's time in seconds). Pressing on
//! it pushes `ScrubTo` at once and again whenever the time under the pointer changes while it is held; `ScrubEnd`
//! is pushed once when the scrub ends, which is when the button is released (wherever the pointer is then, inside
//! the window or out of it, since a drag keeps the pointer), when the window loses the keyboard focus mid-drag (no
//! release may ever arrive then; the button must be pressed again to scrub again), or when the lanes stop being
//! there (the take was cleared). Times are kept inside the take, 0 to 61 s (`ribbon::t_of`). Only the pointer
//! scrubs: Enter or a screen reader's click on the focused lanes does nothing. Inside a disabled parent the lanes
//! take no input and the keys are left alone.
//!
//! The keys (section 7): Space pushes `PlayPause`, the left and right arrows `Step(-1)` and `Step(1)`, each taken
//! from the widgets so nothing else acts on it too. They do nothing while the transport is not enabled, and nothing
//! while a control other than the story timeline has the keyboard: a text field types them, a button or switch is
//! pressed by Space, and a slider or value field moves by the arrows, so taking them there would do two things at
//! once. 1 to 5 are the shell's and are never touched here.
//!
//! Nothing here clones the view model or grows with the fight: the story is read where it is, and only its ability
//! marks (a handful) and the ribbon's points are drawn.

use egui::{
    Align, Context, EventFilter, Id, Key, Label, Layout, Modifiers, PointerButton, Rect, Response, RichText, Sense,
    Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use crate::model::story::StoryView;
use crate::model::take::{SoundState, TransportAction, TransportView};
use crate::model::{FPS, NF, Step, View};
use crate::preview::clock::{TOTAL, timecode};
use crate::state::{Action, ViewModel};
use crate::theme::{
    LANE_NAME_W, LINE_SOFT, MUTED, TEXT, TEXT_STRONG, TRANSPORT_H, body, rgb, small, timecode as timecode_font,
};
use crate::widgets::buttons::{self, focus_ring};
use crate::widgets::icons::{self, Icon};
use crate::widgets::ribbon::{self, DUR};
use crate::wording::NO_DEVICE;

/// The line the story holds before a seed is picked.
pub const EMPTY: &str = "The fight's story appears here once you pick a seed.";
/// The hint on Seeds.
pub const SCRUB_HINT: &str = "Drag anywhere on the story to scrub";
/// What the transport says while the sound is being built.
pub const BUILDING: &str = "Building the sound";
/// What hovering a transport control says before there is a take to play.
pub const NO_TAKE: &str = "Pick a seed first.";
/// The words beside the shaded K.O. window, on Seeds.
pub const KO_WINDOW: &str = "K.O. window";

/// Side padding of the transport row and of the lane names.
const PAD_X: f32 = 10.0;
/// The heights of the three lanes: the left fighter's abilities, the lead ribbon, the right fighter's abilities.
const LANE_H: [f32; 3] = [20.0, 41.0, 20.0];
/// An ability mark: 2 wide, 14 tall from 3 below the lane's top, its name 5 after it.
const MARK_W: f32 = 2.0;
const MARK_TOP: f32 = 3.0;
const MARK_H: f32 = 14.0;
const NAME_GAP: f32 = 5.0;
/// The room kept between an ability's name and the next mark in its lane.
const NAME_ROOM: f32 = 4.0;
/// The end of the K.O. window, in seconds, and the gap before its words.
const KO_WINDOW_END: f32 = 53.0;
const KO_WORDS_GAP: f32 = 4.0;
/// Room kept inside the lanes for the focus ring (2 px of gap, then 2 px of ring), so no clip cuts it off.
const RING_ROOM: f32 = 4.0;

/// Where a scrub of the lanes stands, kept between frames.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scrub {
    Idle,
    /// The button is held on the lanes; the last time sent.
    Held(f32),
    /// The scrub was ended without a release (the window lost the keyboard focus); nothing happens until the
    /// button is let go.
    Ended,
}

/// Draws it into `ui` from `vm`, pushing what the owner does to `out`.
pub fn show(ui: &mut Ui, vm: &ViewModel, out: &mut Vec<Action>) {
    let full = ui.max_rect();
    let scrub_id = ui.make_persistent_id("story timeline");
    if ui.is_enabled() {
        keys(ui.ctx(), &vm.transport, scrub_id, out);
    }

    let top = full.min.y + 1.0;
    let row = Rect::from_min_max(pos2(full.min.x, top), pos2(full.max.x, top + TRANSPORT_H));
    transport_row(ui, vm, row, out);
    ui.painter()
        .hline(row.x_range(), row.max.y - 0.5, Stroke::new(1.0, LINE_SOFT));

    let story_rect = Rect::from_min_max(pos2(full.min.x, row.max.y), full.max);
    let scrubbed = match &vm.take.story {
        Some(story) => {
            story_lanes(ui, vm, story, story_rect, scrub_id, out);
            true
        }
        None => {
            centred_line(ui, story_rect, EMPTY);
            false
        }
    };
    if !scrubbed {
        // The lanes are gone (or never came): a scrub that was going on ends here.
        end_unseen_scrub(ui.ctx(), scrub_id, out);
    }
}

/// Space and the arrows, taken from the widgets when they are the transport's (see the module's notes).
fn keys(ctx: &Context, transport: &TransportView, scrub_id: Id, out: &mut Vec<Action>) {
    if !transport.enabled || ctx.text_edit_focused() {
        return;
    }
    if ctx.memory(|m| m.focused()).is_some_and(|id| id != scrub_id) {
        return;
    }
    ctx.input_mut(|input| {
        if input.consume_key(Modifiers::NONE, Key::Space) {
            out.push(Action::Transport(TransportAction::PlayPause));
        }
        for (key, by) in [(Key::ArrowLeft, -1), (Key::ArrowRight, 1)] {
            for _ in 0..input.count_and_consume_key(Modifiers::NONE, key) {
                out.push(Action::Transport(TransportAction::Step(by)));
            }
        }
    });
}

/// The frame the transport shows: the playhead's, kept inside the video; 0 when nothing can be played.
fn shown_frame(transport: &TransportView) -> u32 {
    if transport.enabled {
        transport.frame.min(NF - 1)
    } else {
        0
    }
}

/// The transport row in `row`: play or pause, the time and the sound's state on the left; the Seeds hint, "Loop"
/// and "Safe zone" on the right.
fn transport_row(ui: &mut Ui, vm: &ViewModel, row: Rect, out: &mut Vec<Action>) {
    let t = &vm.transport;
    let inner = row.shrink2(vec2(PAD_X, 0.0));
    let layout = Layout::left_to_right(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(inner).layout(layout), |ui| {
        let (icon, name) = if t.playing {
            (Icon::Pause, "Pause")
        } else {
            (Icon::Play, "Play")
        };
        if icons::icon_button(ui, icon, name, t.enabled).clicked() {
            out.push(Action::Transport(TransportAction::PlayPause));
        }
        let time = format!("{} / {TOTAL}", timecode(shown_frame(t)));
        let colour = if t.enabled { TEXT } else { MUTED };
        plain(ui, RichText::new(time).font(timecode_font()).color(colour));
        let sound = match t.sound {
            SoundState::Building => Some(BUILDING),
            SoundState::NoDevice => Some(NO_DEVICE),
            SoundState::None | SoundState::Ready => None,
        };
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if t.enabled {
                if buttons::toggle(ui, "Safe zone", t.safe_zone).clicked() {
                    out.push(Action::Transport(TransportAction::SetSafeZone(!t.safe_zone)));
                }
                if buttons::toggle(ui, "Loop", t.looping).clicked() {
                    out.push(Action::Transport(TransportAction::SetLoop(!t.looping)));
                }
            } else {
                buttons::disabled(ui, "Safe zone", NO_TAKE);
                buttons::disabled(ui, "Loop", NO_TAKE);
            }
            ui.spacing_mut().item_spacing.x = 8.0;
            if vm.view == View::Step(Step::Seeds) && vm.take.story.is_some() {
                ui.add_space(8.0);
                plain(ui, RichText::new(SCRUB_HINT).font(small()).color(MUTED));
            }
            // What the sound is doing takes what is left between the time and the hint, cut short if it must be.
            if let Some(sound) = sound {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let text = RichText::new(sound).font(small()).color(MUTED);
                    ui.add(Label::new(text).selectable(false).truncate());
                });
            }
        });
    });
}

/// The story of a take in `rect`: the lane names, the lanes (one control), the ability marks, the ribbon, the K.O.
/// window's words on Seeds, and the playhead over them all.
fn story_lanes(ui: &mut Ui, vm: &ViewModel, story: &StoryView, rect: Rect, scrub_id: Id, out: &mut Vec<Action>) {
    let names_right = rect.min.x + LANE_NAME_W;
    let lanes = Rect::from_min_max(pos2(names_right, rect.min.y), rect.max);
    let lane = |i: usize| {
        let y0 = rect.min.y + LANE_H[..i].iter().sum::<f32>();
        (y0, y0 + LANE_H[i])
    };

    // The control first, so the words drawn over it take no input from it.
    // Inside a disabled parent it takes no input and says so.
    let enabled = vm.transport.enabled && ui.is_enabled();
    let response = scrub(ui, lanes, scrub_id, enabled, out);
    let seconds = f64::from(shown_frame(&vm.transport)) / f64::from(FPS);
    response.widget_info(|| WidgetInfo::slider(enabled, seconds, "Story timeline"));

    // The lane names.
    let painter = ui.painter().clone();
    painter.vline(names_right - 0.5, rect.y_range(), Stroke::new(1.0, LINE_SOFT));
    let [left, right] = vm.names();
    for (i, name) in [left.as_str(), "Who leads", right.as_str()].into_iter().enumerate() {
        let (y0, y1) = lane(i);
        let cell = Rect::from_min_max(pos2(rect.min.x + PAD_X, y0), pos2(names_right - PAD_X, y1));
        words(ui, cell, RichText::new(name).font(small()).color(MUTED));
    }

    // The ribbon, then each fighter's first use of each ability.
    let colors = vm.colors();
    let (r0, r1) = lane(1);
    let ribbon_rect = Rect::from_x_y_ranges(lanes.x_range(), r0..=r1);
    ribbon::paint(&painter, ribbon_rect, story, colors, true);
    let x_of = |t: f32| ribbon::x_of(t, lanes.min.x, lanes.width());
    for mark in &story.firsts {
        if !(mark.who < 2 && mark.t.is_finite() && (0.0..=DUR).contains(&mark.t)) {
            continue;
        }
        let (y0, y1) = lane(if mark.who == 0 { 0 } else { 2 });
        let colour = rgb(colors[mark.who]);
        let x = x_of(mark.t).round().min(lanes.max.x - MARK_W);
        painter.rect_filled(
            Rect::from_min_size(pos2(x, y0 + MARK_TOP), vec2(MARK_W, MARK_H)),
            0.0,
            colour,
        );
        // The name runs up to the next mark in the same lane, or to the end of the lanes.
        let next = story
            .firsts
            .iter()
            .filter(|m| m.who == mark.who && m.t.is_finite() && m.t > mark.t && m.t <= DUR)
            .map(|m| x_of(m.t).round() - NAME_ROOM)
            .fold(lanes.max.x - NAME_ROOM, f32::min);
        let name_left = x + MARK_W + NAME_GAP;
        if next > name_left {
            let cell = Rect::from_min_max(pos2(name_left, y0), pos2(next, y1));
            words(ui, cell, RichText::new(&mark.name).font(small()).color(colour));
        }
    }
    if vm.view == View::Step(Step::Seeds) {
        let left = x_of(KO_WINDOW_END).round() + KO_WORDS_GAP;
        let cell = Rect::from_min_max(pos2(left, r0), pos2(lanes.max.x - NAME_ROOM, (r0 + r1) / 2.0));
        if cell.width() > 0.0 {
            words(ui, cell, RichText::new(KO_WINDOW).font(small()).color(MUTED));
        }
    }

    // The playhead, over everything in the lanes.
    let x = x_of(seconds as f32).floor().clamp(lanes.min.x, lanes.max.x - 1.0);
    painter.rect_filled(Rect::from_x_y_ranges(x..=x + 1.0, rect.y_range()), 0.0, TEXT_STRONG);
    if response.has_focus() {
        focus_ring(&painter, lanes.shrink(RING_ROOM));
    }
}

/// The lanes as one control: scrubbing with the pointer (see the module's notes), and the arrows kept on it while it
/// has the keyboard.
fn scrub(ui: &mut Ui, lanes: Rect, id: Id, enabled: bool, out: &mut Vec<Action>) -> Response {
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let response = ui.interact(lanes, id, sense);
    let state_id = id.with("scrub");
    let was = ui.data(|d| d.get_temp::<Scrub>(state_id)).unwrap_or(Scrub::Idle);
    if response.has_focus() {
        // The arrows step the playhead while the lanes have the keyboard; without this egui would also move the
        // focus to a neighbouring control on the same press.
        let filter = EventFilter {
            horizontal_arrows: true,
            ..EventFilter::default()
        };
        ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
    }
    let window_lost =
        ui.input(|i| !i.focused || i.events.iter().any(|e| matches!(e, egui::Event::WindowFocused(false))));
    let down = enabled && response.is_pointer_button_down_on();
    // Only the pointer's own click or release counts: Enter (or a screen reader's click) on the focused lanes also
    // makes `clicked()` true, at whatever stale place the pointer last was.
    let released = enabled && (response.clicked_by(PointerButton::Primary) || response.drag_stopped());
    let at = response
        .interact_pointer_pos()
        .map(|p| ribbon::t_of(p.x, lanes.min.x, lanes.width()));
    let send = |out: &mut Vec<Action>, a| out.push(Action::Transport(a));

    let next = match was {
        // Ended without a release: wait for the button to come up.
        Scrub::Ended => {
            if down {
                Scrub::Ended
            } else {
                Scrub::Idle
            }
        }
        _ if window_lost => {
            if matches!(was, Scrub::Held(_)) {
                send(out, TransportAction::ScrubEnd);
            }
            if down { Scrub::Ended } else { Scrub::Idle }
        }
        _ => match (down, released, at) {
            (true, _, Some(t)) => {
                if was != Scrub::Held(t) {
                    send(out, TransportAction::ScrubTo(t));
                }
                Scrub::Held(t)
            }
            (false, true, Some(t)) => {
                // The release (or a press and release in one frame): the last place, then the end.
                if was != Scrub::Held(t) {
                    send(out, TransportAction::ScrubTo(t));
                }
                send(out, TransportAction::ScrubEnd);
                Scrub::Idle
            }
            _ => {
                if matches!(was, Scrub::Held(_)) {
                    send(out, TransportAction::ScrubEnd);
                }
                Scrub::Idle
            }
        },
    };
    if next != was {
        ui.data_mut(|d| d.insert_temp(state_id, next));
    }
    response
}

/// A scrub still held when the lanes are not drawn ends now.
fn end_unseen_scrub(ctx: &Context, scrub_id: Id, out: &mut Vec<Action>) {
    let state_id = scrub_id.with("scrub");
    let was = ctx.data(|d| d.get_temp::<Scrub>(state_id));
    if let Some(was) = was.filter(|&s| s != Scrub::Idle) {
        if matches!(was, Scrub::Held(_)) {
            out.push(Action::Transport(TransportAction::ScrubEnd));
        }
        ctx.data_mut(|d| d.insert_temp(state_id, Scrub::Idle));
    }
}

/// One line of words in `cell`, from its left edge and in the middle of its height, cut short with an ellipsis when
/// it does not fit.
fn words(ui: &mut Ui, cell: Rect, text: RichText) {
    let layout = Layout::left_to_right(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(cell).layout(layout), |ui| {
        ui.add(Label::new(text).selectable(false).truncate());
    });
}

/// One centred body `MUTED` line in `rect`.
fn centred_line(ui: &mut Ui, rect: Rect, line: &str) {
    let layout = Layout::centered_and_justified(egui::Direction::LeftToRight);
    let inner = rect.shrink2(vec2(PAD_X, 0.0));
    ui.scope_builder(UiBuilder::new().max_rect(inner).layout(layout), |ui| {
        let text = RichText::new(line).font(body()).color(MUTED);
        ui.add(Label::new(text).selectable(false).truncate());
    });
}

/// Text that is read, not clicked or selected.
fn plain(ui: &mut Ui, text: RichText) {
    ui.add(Label::new(text).selectable(false));
}
