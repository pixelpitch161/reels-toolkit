//! The slider, the value field and the text fields: dragging, clicking and the keyboard, numbers that cannot be
//! used, units, typing, wells and borders, the focus ring at a panel's edge, names, and a picture of them all.

use std::cell::{Cell, RefCell};

use bf_app::theme::{self, FAINT, GROUND, LABEL_W, LINE, MUTED, PANEL, TEXT, TEXT_STRONG};
use bf_app::widgets::buttons;
use bf_app::widgets::fields::{self, Unit};
use bf_app::widgets::slider::{self, SliderOut};
use egui::accesskit::Role;
use egui::{Rect, RichText, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common;

/// Paints the whole window `PANEL`, the ground every control sits on.
fn on_panel(ui: &mut Ui) {
    ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, PANEL);
}

fn rgb(colour: egui::Color32) -> [u8; 3] {
    [colour.r(), colour.g(), colour.b()]
}

fn brightness([r, g, b]: [u8; 3]) -> u32 {
    u32::from(r) + u32::from(g) + u32::from(b)
}

/// The pixel `dx`, `dy` from the top-left corner of `rect`.
fn at(harness: &mut Harness<'_>, rect: Rect, dx: f32, dy: f32) -> [u8; 3] {
    let x = (rect.min.x + dx).round() as u32;
    let y = (rect.min.y + dy).round() as u32;
    common::pixels(harness, &[(x, y)])[0]
}

/// Every pixel inside `rect` (whole pixels only), from one render.
fn region(harness: &mut Harness<'_>, rect: Rect) -> Vec<[u8; 3]> {
    let image = harness.render().expect("the harness renders");
    let (x0, y0) = (rect.min.x.ceil() as u32, rect.min.y.ceil() as u32);
    let (x1, y1) = (rect.max.x.floor() as u32, rect.max.y.floor() as u32);
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            out.push([r, g, b]);
        }
    }
    out
}

/// The brightest pixel inside `rect`: the colour a label is drawn in, on a darker fill.
fn brightest(harness: &mut Harness<'_>, rect: Rect) -> [u8; 3] {
    region(harness, rect)
        .into_iter()
        .max_by_key(|&p| brightness(p))
        .expect("pixels")
}

/// The slider's range and step in most tests: the gravity setting.
const RANGE: (f64, f64) = (0.05, 3.0);
const STEP: f64 = 0.01;
const WIDTH: f32 = 200.0;
/// The slider keeps 4 px free inside its row on each side, and its 3 px knob's centre travels inside that.
const ROOM: f32 = 4.0;
const HALF_KNOB: f32 = 1.5;

/// The x of the knob's centre for `value` on the slider at `rect`.
fn knob_centre(rect: Rect, value: f64) -> f32 {
    let left = rect.min.x + ROOM + HALF_KNOB;
    slider::knob_x(value, RANGE, left, rect.width() - 2.0 * (ROOM + HALF_KNOB))
}

/// A harness holding one slider named "Gravity", starting at `start`. The value, whether a change was seen and how
/// many frames reported a release are kept in the cells.
fn slider_harness<'a>(value: &'a Cell<f64>, changed: &'a Cell<bool>, released: &'a Cell<u32>) -> Harness<'a> {
    common::widget_harness((260.0, 60.0), move |ui| {
        on_panel(ui);
        let mut v = value.get();
        let out: SliderOut = slider::bar_slider(ui, "Gravity", &mut v, RANGE, STEP, WIDTH);
        value.set(v);
        if out.changed {
            changed.set(true);
        }
        if out.released {
            released.set(released.get() + 1);
        }
    })
}

#[test]
fn dragging_changes_then_releases() {
    let _gpu = common::gpu();
    let (value, changed, released) = (Cell::new(1.0), Cell::new(false), Cell::new(0));
    let mut harness = slider_harness(&value, &changed, &released);
    harness.run();
    let rect = harness.get_by_label("Gravity").rect();
    assert_eq!(
        (rect.width(), rect.height()),
        (WIDTH, theme::CONTROL_H),
        "the row is 26 tall"
    );
    let knob = pos2(knob_centre(rect, 1.0), rect.center().y);
    let end = pos2(rect.max.x - 1.0, rect.center().y);
    harness.drag_at(knob);
    harness.run();
    harness.hover_at(pos2(rect.center().x, rect.center().y));
    harness.run();
    harness.hover_at(end);
    harness.run();
    assert_eq!(released.get(), 0, "nothing is released while the button is down");
    harness.drop_at(end);
    harness.run();
    drop(harness);
    assert_eq!(value.get(), 3.0, "the value ends at the range's top");
    assert!(changed.get(), "the drag changed the value");
    assert_eq!(released.get(), 1, "released was seen exactly once");
}

#[test]
fn a_click_anywhere_on_the_row_moves_the_knob_there() {
    let _gpu = common::gpu();
    let (value, changed, released) = (Cell::new(1.0), Cell::new(false), Cell::new(0));
    let mut harness = slider_harness(&value, &changed, &released);
    harness.run();
    let rect = harness.get_by_label("Gravity").rect();
    // Low on the row, well away from the track: the whole row takes clicks.
    let spot = pos2(rect.min.x + 150.0, rect.max.y - 2.0);
    harness.drag_at(spot);
    harness.run();
    harness.drop_at(spot);
    harness.run();
    drop(harness);
    let left = rect.min.x + ROOM + HALF_KNOB;
    let expected = slider::value_at(spot.x, RANGE, STEP, left, WIDTH - 2.0 * (ROOM + HALF_KNOB));
    assert!(
        (value.get() - expected).abs() < 1e-9,
        "{} against {expected}",
        value.get()
    );
    assert!(changed.get());
    assert_eq!(released.get(), 1, "a click releases once");
}

#[test]
fn the_slider_works_from_the_keyboard() {
    let _gpu = common::gpu();
    let (value, changed, released) = (Cell::new(1.0), Cell::new(false), Cell::new(0));
    let mut harness = slider_harness(&value, &changed, &released);
    harness.run();
    harness.get_by_label("Gravity").focus();
    harness.run();
    assert!(harness.get_by_label("Gravity").is_focused());
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert!(
        (value.get() - 1.01).abs() < 1e-9,
        "right moves one step up: {}",
        value.get()
    );
    assert!(changed.get() && released.get() == 1, "a key press changes and releases");
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    assert!(
        (value.get() - 0.99).abs() < 1e-9,
        "left moves one step down: {}",
        value.get()
    );
    drop(harness);
    assert_eq!(released.get(), 3);
}

#[test]
fn the_slider_keeps_a_bad_value_inside_its_range() {
    let _gpu = common::gpu();
    let (value, changed, released) = (Cell::new(f64::NAN), Cell::new(false), Cell::new(0));
    let mut harness = slider_harness(&value, &changed, &released);
    harness.run();
    let node = harness.get_by_label("Gravity");
    assert_eq!(
        node.accesskit_node().numeric_value(),
        Some(0.05),
        "not a number shows as the low end"
    );
    assert!(!changed.get(), "showing is not changing");
    node.focus();
    harness.run();
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    drop(harness);
    assert_eq!(value.get(), 0.05, "a key press writes a value inside the range");
}

#[test]
fn the_slider_looks_as_the_reference_says() {
    let _gpu = common::gpu();
    let (value, changed, released) = (Cell::new(2.0), Cell::new(false), Cell::new(0));
    let mut harness = slider_harness(&value, &changed, &released);
    harness.run();
    let rect = harness.get_by_label("Gravity").rect();
    let middle = rect.height() / 2.0;
    let knob = knob_centre(rect, 2.0) - rect.min.x;
    let knob_left = (knob - HALF_KNOB).round();
    assert_eq!(
        at(&mut harness, rect, knob_left + 1.0, middle),
        rgb(TEXT_STRONG),
        "the knob"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 1.0, middle - 6.0),
        rgb(TEXT_STRONG),
        "14 tall"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 1.0, middle + 6.0),
        rgb(TEXT_STRONG),
        "14 tall"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 1.0, middle - 8.0),
        rgb(PANEL),
        "and no taller"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 4.0, middle - 4.0),
        rgb(PANEL),
        "3 wide"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left - 30.0, middle),
        rgb(MUTED),
        "the fill up to the knob"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 30.0, middle),
        rgb(LINE),
        "the track past it"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 30.0, middle - 1.0),
        rgb(LINE),
        "2 px"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 30.0, middle - 2.0),
        rgb(PANEL),
        "not 3"
    );
    assert_eq!(
        at(&mut harness, rect, knob_left + 30.0, middle + 1.0),
        rgb(PANEL),
        "not 3"
    );
}

#[test]
fn a_disabled_slider_is_faint_and_takes_no_input() {
    let _gpu = common::gpu();
    let value = Cell::new(2.0);
    let seen = Cell::new(SliderOut::default());
    let mut harness = common::widget_harness((260.0, 60.0), |ui| {
        on_panel(ui);
        ui.add_enabled_ui(false, |ui| {
            let mut v = value.get();
            let out = slider::bar_slider(ui, "Gravity", &mut v, RANGE, STEP, WIDTH);
            value.set(v);
            if out.changed || out.released {
                seen.set(out);
            }
        });
    });
    harness.run();
    let node = harness.get_by_label("Gravity");
    assert!(node.accesskit_node().is_disabled());
    let rect = node.rect();
    assert_eq!(brightest(&mut harness, rect), rgb(FAINT), "knob and fill are FAINT");
    harness.drag_at(pos2(rect.min.x + 20.0, rect.center().y));
    harness.run();
    harness.drop_at(pos2(rect.min.x + 20.0, rect.center().y));
    harness.run();
    drop(harness);
    assert_eq!(value.get(), 2.0, "a click does nothing");
    assert_eq!(seen.get(), SliderOut::default());
}

/// A harness holding one value field named "Gravity" with `unit`, `value` in `range`.
fn field_harness<'a>(value: &'a Cell<f64>, range: (f64, f64), unit: Unit<'static>) -> Harness<'a> {
    common::widget_harness((200.0, 60.0), move |ui| {
        on_panel(ui);
        let mut v = value.get();
        fields::value_field(ui, "Gravity", &mut v, range, unit, theme::VALUE_W);
        value.set(v);
    })
}

const TIMES: Unit<'static> = Unit {
    prefix: "×",
    suffix: "",
    decimals: 2,
};
const SECONDS: Unit<'static> = Unit {
    prefix: "",
    suffix: " s",
    decimals: 1,
};

#[test]
fn the_value_field_shows_its_unit() {
    let _gpu = common::gpu();
    let value = Cell::new(2.0);
    let mut harness = field_harness(&value, (0.05, 12.0), TIMES);
    harness.run();
    let node = harness.get_by_label("Gravity");
    assert_eq!(node.accesskit_node().role(), Role::SpinButton);
    assert_eq!(node.value().as_deref(), Some("×2.00"));
    let rect = node.rect();
    assert_eq!((rect.width(), rect.height()), (62.0, 22.0));
    assert!(harness.query_by_value("×2.00").is_some(), "the text is on screen");
    drop(harness);

    let value = Cell::new(49.0);
    let mut harness = field_harness(&value, (20.0, 61.0), SECONDS);
    harness.run();
    assert_eq!(harness.get_by_label("Gravity").value().as_deref(), Some("49.0 s"));
}

#[test]
fn the_value_field_is_a_ground_well_with_a_line_border_and_right_aligned_mono_text() {
    let _gpu = common::gpu();
    let value = Cell::new(2.0);
    let mut harness = field_harness(&value, (0.05, 12.0), TIMES);
    harness.run();
    let rect = harness.get_by_label("Gravity").rect();
    assert_eq!(at(&mut harness, rect, 0.0, 11.0), rgb(LINE), "1 px LINE border, left");
    assert_eq!(at(&mut harness, rect, 61.0, 11.0), rgb(LINE), "and right");
    assert_eq!(at(&mut harness, rect, 30.0, 0.0), rgb(LINE), "and top");
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), rgb(GROUND), "a GROUND well");
    let left = Rect::from_min_max(rect.min + vec2(2.0, 2.0), pos2(rect.min.x + 14.0, rect.max.y - 2.0));
    assert_eq!(
        brightest(&mut harness, left),
        rgb(GROUND),
        "the text keeps to the right"
    );
    let right = Rect::from_min_max(pos2(rect.max.x - 30.0, rect.min.y + 2.0), rect.max - vec2(2.0, 2.0));
    assert_eq!(brightest(&mut harness, right), rgb(TEXT), "TEXT on the right");
    harness.get_by_label("Gravity").hover();
    harness.run();
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), rgb(GROUND), "no hover fill");
}

#[test]
fn a_number_too_long_for_its_field_is_not_drawn_over_its_neighbours() {
    let _gpu = common::gpu();
    let bpm = Unit {
        prefix: "",
        suffix: " bpm",
        decimals: 2,
    };
    let mut harness = common::widget_harness((200.0, 60.0), |ui| {
        on_panel(ui);
        let mut tempo = 133.77;
        fields::value_field(ui, "Tempo", &mut tempo, (60.0, 200.0), bpm, theme::VALUE_W);
    });
    harness.run();
    assert_eq!(harness.get_by_label("Tempo").value().as_deref(), Some("133.77 bpm"));
    // The harness puts its first widget 8 points in from the top-left corner.
    let well = Rect::from_min_size(pos2(8.0, 8.0), vec2(theme::VALUE_W, theme::VALUE_H));
    assert_eq!(
        at(&mut harness, well, 0.0, 11.0),
        rgb(LINE),
        "the well is where it was given"
    );
    let after = Rect::from_min_size(well.right_top() + vec2(1.0, 0.0), vec2(80.0, well.height()));
    assert_eq!(
        brightest(&mut harness, after),
        rgb(PANEL),
        "nothing is drawn past its right edge"
    );
    let before = Rect::from_min_max(pos2(0.0, well.min.y), pos2(well.min.x - 1.0, well.max.y));
    assert_eq!(brightest(&mut harness, before), rgb(PANEL), "nor past its left edge");
}

/// Types `typed` into the field (its text is all selected when it takes the focus) and presses Enter.
fn type_into_field(harness: &mut Harness<'_>, typed: &str) {
    harness.get_by_label("Gravity").focus();
    harness.run();
    harness.get_by_label("Gravity").type_text(typed);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
}

#[test]
fn typed_numbers_stay_finite_and_inside_the_range() {
    let _gpu = common::gpu();
    let cases = [
        ("3.5", 3.5),
        ("×4.25", 4.25),
        ("99", 12.0),
        ("-5", 0.05),
        ("1e999", 2.0),
        ("inf", 2.0),
        ("-inf", 2.0),
        ("nan", 2.0),
        ("NaN", 2.0),
        ("abc", 2.0),
        ("", 2.0),
        ("1,5", 1.5),
        ("×2,25", 2.25),
        (" 3 , 75 ", 3.75),
        ("1,2,3", 2.0),
        ("1,234.5", 2.0),
        (",", 2.0),
        ("abc,def", 2.0),
    ];
    for (typed, expected) in cases {
        let value = Cell::new(2.0);
        let mut harness = field_harness(&value, (0.05, 12.0), TIMES);
        harness.run();
        type_into_field(&mut harness, typed);
        drop(harness);
        assert_eq!(value.get(), expected, "typing {typed:?}");
    }
}

#[test]
fn a_value_field_with_a_bad_range_never_gives_a_bad_number() {
    let _gpu = common::gpu();
    let cases = [
        ((12.0, 0.05), "99", 12.0),
        ((f64::NAN, 4.0), "1", 4.0),
        ((f64::NEG_INFINITY, f64::INFINITY), "7", 0.0),
        ((3.0, 3.0), "1", 3.0),
    ];
    for (range, typed, expected) in cases {
        let value = Cell::new(f64::NAN);
        let mut harness = field_harness(&value, range, TIMES);
        harness.run();
        type_into_field(&mut harness, typed);
        drop(harness);
        assert_eq!(value.get(), expected, "range {range:?}, typing {typed:?}");
    }
}

#[test]
fn dragging_the_value_field_sideways_changes_it_inside_the_range() {
    let _gpu = common::gpu();
    let value = Cell::new(2.0);
    let mut harness = field_harness(&value, (0.05, 12.0), TIMES);
    harness.run();
    let rect = harness.get_by_label("Gravity").rect();
    let start = rect.center();
    harness.drag_at(start);
    harness.run();
    harness.hover_at(start + vec2(10.0, 0.0));
    harness.run();
    harness.hover_at(start + vec2(20.0, 0.0));
    harness.run();
    let partway = value.get();
    assert!(partway > 2.0, "dragging right raises it: {partway}");
    harness.hover_at(start + vec2(4000.0, 0.0));
    harness.run();
    harness.drop_at(start + vec2(4000.0, 0.0));
    harness.run();
    drop(harness);
    assert_eq!(value.get(), 12.0, "and stops at the top");
}

#[test]
fn the_arrow_keys_step_the_value_field_inside_its_range() {
    let _gpu = common::gpu();
    let value = Cell::new(11.95);
    let mut harness = field_harness(&value, (0.05, 12.0), TIMES);
    harness.run();
    harness.get_by_label("Gravity").focus();
    harness.run();
    for _ in 0..3 {
        harness.key_press(egui::Key::ArrowUp);
        harness.run();
        let v = value.get();
        assert!(v.is_finite() && (0.05..=12.0).contains(&v), "{v}");
    }
    drop(harness);
    assert_eq!(value.get(), 12.0);
}

#[test]
fn typing_into_a_text_line() {
    let _gpu = common::gpu();
    let seed = RefCell::new(String::new());
    let mut harness = common::widget_harness((260.0, 60.0), |ui| {
        on_panel(ui);
        fields::text_line(ui, "Seed", &mut seed.borrow_mut(), "Seed number", 200.0);
    });
    harness.run();
    let field = harness.get_by(|node| node.placeholder() == Some("Seed number"));
    assert_eq!(field.accesskit_node().label().as_deref(), Some("Seed"));
    field.focus();
    harness.run();
    harness.get_by_label("Seed").type_text("20418");
    harness.run();
    drop(harness);
    assert_eq!(seed.borrow().as_str(), "20418");
}

#[test]
fn text_fields_are_ground_wells_with_a_line_border_and_a_faint_hint() {
    let _gpu = common::gpu();
    let (seed, caption) = (RefCell::new(String::new()), RefCell::new("Who wins?".to_owned()));
    let mut harness = common::widget_harness((300.0, 160.0), |ui| {
        on_panel(ui);
        fields::text_line(ui, "Seed", &mut seed.borrow_mut(), "Seed number", 200.0);
        fields::text_box(ui, "Caption", &mut caption.borrow_mut(), 3);
    });
    harness.run();
    let line = harness.get_by_label("Seed").rect();
    assert_eq!(
        (line.width(), line.height()),
        (200.0, theme::CONTROL_H),
        "a text line is 26 tall"
    );
    assert_eq!(at(&mut harness, line, 0.0, 13.0), rgb(LINE), "1 px LINE border");
    assert_eq!(at(&mut harness, line, 2.0, 2.0), rgb(GROUND), "a GROUND well");
    assert_eq!(
        brightest(&mut harness, line.shrink(2.0)),
        rgb(FAINT),
        "the hint is FAINT"
    );
    let node = harness.get_by_label("Caption");
    assert_eq!(node.accesskit_node().role(), Role::MultilineTextInput);
    let rect = node.rect();
    assert!(rect.height() >= 3.0 * 16.0, "three lines: {}", rect.height());
    let available = harness.ctx.content_rect().width() - 16.0;
    assert_eq!(rect.width(), available, "the box fills the width");
    assert_eq!(at(&mut harness, rect, 0.0, 20.0), rgb(LINE), "1 px LINE border");
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), rgb(GROUND), "a GROUND well");
    assert_eq!(brightest(&mut harness, rect.shrink(2.0)), rgb(TEXT), "its text is TEXT");
}

#[test]
fn a_read_field_shows_its_text_cut_short_and_cannot_be_edited() {
    let _gpu = common::gpu();
    let path = "F:\\Videos\\BALLFIGHT\\renders\\flute vs trombone\\take 18433\\final.mp4";
    let mut harness = common::widget_harness((300.0, 60.0), |ui| {
        on_panel(ui);
        fields::read_field(ui, "Output folder", path, 200.0);
    });
    harness.run();
    let node = harness.get_by_label("Output folder");
    assert!(node.accesskit_node().is_read_only(), "it cannot be edited");
    assert_eq!(node.value().as_deref(), Some(path), "a screen reader hears all of it");
    let rect = node.rect();
    assert_eq!((rect.width(), rect.height()), (200.0, theme::CONTROL_H));
    assert_eq!(at(&mut harness, rect, 0.0, 13.0), rgb(LINE), "1 px LINE border");
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), rgb(GROUND), "a GROUND well");
    let after = Rect::from_min_size(rect.right_top() + vec2(1.0, 0.0), vec2(60.0, rect.height()));
    assert_eq!(
        brightest(&mut harness, after),
        rgb(PANEL),
        "nothing is drawn past its edge"
    );
    harness.get_by_label("Output folder").hover();
    harness.run();
    assert!(harness.query_by_label(path).is_some(), "hovering shows the whole text");
}

/// The controls of this task side by side at the right edge of a left panel, the workspace beside it painted
/// `GROUND`: each one's ring must show past the panel's edge.
fn at_a_panel_edge(ui: &mut Ui, value: &mut f64, seed: &mut String, caption: &mut String) {
    egui::Panel::left("side")
        .exact_size(230.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().fill(PANEL))
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                slider::bar_slider(ui, "Gravity", value, RANGE, STEP, WIDTH);
                fields::value_field(ui, "Speed", value, RANGE, TIMES, theme::VALUE_W);
                fields::text_line(ui, "Seed", seed, "Seed number", 200.0);
                fields::text_box(ui, "Caption", caption, 3);
            });
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(GROUND))
        .show(ui, |_| {});
}

#[test]
fn focused_controls_paint_their_ring_even_at_a_panel_edge() {
    let _gpu = common::gpu();
    let (value, seed, caption) = (Cell::new(1.0), RefCell::new(String::new()), RefCell::new(String::new()));
    let mut harness = common::widget_harness((400.0, 220.0), |ui| {
        let mut v = value.get();
        at_a_panel_edge(ui, &mut v, &mut seed.borrow_mut(), &mut caption.borrow_mut());
        value.set(v);
    });
    harness.run();
    // The slider keeps its ring inside its row: 2 px of ring, then 2 px of gap, then the track and knob.
    let rect = harness.get_by_label("Gravity").rect();
    let middle = rect.height() / 2.0;
    assert_ne!(
        at(&mut harness, rect, 1.0, middle),
        rgb(TEXT_STRONG),
        "Gravity: no ring before focus"
    );
    harness.get_by_label("Gravity").focus();
    harness.run();
    assert!(harness.get_by_label("Gravity").is_focused());
    assert_eq!(
        at(&mut harness, rect, 1.0, middle),
        rgb(TEXT_STRONG),
        "Gravity: the ring"
    );
    assert_eq!(
        at(&mut harness, rect, rect.width() - 1.0, middle),
        rgb(TEXT_STRONG),
        "at the panel's edge"
    );
    assert_ne!(
        at(&mut harness, rect, 3.0, middle),
        rgb(TEXT_STRONG),
        "Gravity: a 2 px gap"
    );

    // The fields reach the panel's edge: their ring is painted past it, over the workspace.
    for name in ["Speed", "Seed", "Caption"] {
        let rect = harness.get_by_label(name).rect();
        let middle = rect.height() / 2.0;
        let outside = |h: &mut Harness<'_>| at(h, rect, rect.width() + 3.0, middle);
        assert_ne!(outside(&mut harness), rgb(TEXT_STRONG), "{name}: no ring before focus");
        harness.get_by_label(name).focus();
        harness.run();
        assert!(harness.get_by_label(name).is_focused(), "{name} takes the focus");
        assert_eq!(
            outside(&mut harness),
            rgb(TEXT_STRONG),
            "{name}: 2 px TEXT_STRONG ring past the edge"
        );
        assert_eq!(
            at(&mut harness, rect, -3.0, middle),
            rgb(TEXT_STRONG),
            "{name}: and on the left"
        );
        assert_ne!(
            at(&mut harness, rect, -1.0, middle),
            rgb(TEXT_STRONG),
            "{name}: a 2 px gap"
        );
        assert_eq!(
            at(&mut harness, rect, 0.0, middle),
            rgb(LINE),
            "{name}: the border stays LINE"
        );
    }
}

/// A setting row: a `MUTED` label in the label column, then the controls.
fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        let size = vec2(LABEL_W, theme::CONTROL_H);
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_size(size);
            ui.label(RichText::new(label).color(MUTED));
        });
        add(ui);
    });
}

/// One control of each kind, in the states the reference names.
fn gallery(ui: &mut Ui) {
    on_panel(ui);
    let (mut gravity, mut length, mut handicap) = (2.0, 49.0, 1.25);
    row(ui, "Gravity", |ui| {
        slider::bar_slider(ui, "Gravity", &mut gravity, (0.05, 3.0), 0.01, 200.0);
        fields::value_field(ui, "Gravity", &mut gravity, (0.05, 12.0), TIMES, theme::VALUE_W);
    });
    row(ui, "Fight length", |ui| {
        slider::bar_slider(ui, "Fight length", &mut length, (20.0, 61.0), 0.1, 200.0);
        fields::value_field(ui, "Fight length", &mut length, (20.0, 61.0), SECONDS, theme::VALUE_W);
    });
    ui.add_enabled_ui(false, |ui| {
        row(ui, "Handicap", |ui| {
            slider::bar_slider(ui, "Handicap", &mut handicap, (0.5, 2.0), 0.05, 200.0);
            fields::value_field(ui, "Handicap", &mut handicap, (0.5, 2.0), TIMES, theme::VALUE_W);
        });
    });
    let (mut seed, mut title) = (String::new(), "Flute vs Trombone".to_owned());
    row(ui, "Seed", |ui| {
        fields::text_line(ui, "Seed", &mut seed, "Seed number", 200.0);
    });
    row(ui, "Title", |ui| {
        fields::text_line(ui, "Title", &mut title, "Video title", 200.0);
    });
    row(ui, "Output folder", |ui| {
        let path = "F:\\Videos\\BALLFIGHT\\renders\\flute vs trombone";
        fields::read_field(ui, "Output folder", path, 200.0);
    });
    let mut caption = "Flute vs Trombone: who takes the round?\nNew fight every day.".to_owned();
    fields::text_box(ui, "Caption", &mut caption, 3);
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((440.0, 300.0), gallery);
    harness.run();
    assert!(harness.unnamed_widgets().is_empty(), "{:?}", harness.unnamed_widgets());
    let roles = [
        (Role::Slider, "Gravity"),
        (Role::SpinButton, "Gravity"),
        (Role::Slider, "Handicap"),
        (Role::SpinButton, "Fight length"),
        (Role::TextInput, "Seed"),
        (Role::TextInput, "Title"),
        (Role::TextInput, "Output folder"),
        (Role::MultilineTextInput, "Caption"),
    ];
    for (role, name) in roles {
        assert!(harness.query_by_role_and_label(role, name).is_some(), "{role:?} {name}");
    }
    let handicap = harness.get_by_role_and_label(Role::Slider, "Handicap");
    assert!(handicap.accesskit_node().is_disabled(), "inside a disabled parent");
}

#[test]
fn the_gallery_matches() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((440.0, 300.0), gallery);
    harness.run();
    harness.snapshot("widgets_fields");
}

/// A slider named "Gravity" between two box buttons, as in a setting row where the value field sits beside it.
fn slider_between_neighbours<'a>(value: &'a Cell<f64>, released: &'a Cell<u32>) -> Harness<'a> {
    common::widget_harness((420.0, 60.0), move |ui| {
        on_panel(ui);
        ui.horizontal(|ui| {
            buttons::boxed(ui, "Before");
            let mut v = value.get();
            if slider::bar_slider(ui, "Gravity", &mut v, RANGE, STEP, WIDTH).released {
                released.set(released.get() + 1);
            }
            value.set(v);
            buttons::boxed(ui, "After");
        });
    })
}

#[test]
fn the_slider_keeps_the_keyboard_between_neighbours() {
    let _gpu = common::gpu();
    let (value, released) = (Cell::new(1.0), Cell::new(0));
    let mut harness = slider_between_neighbours(&value, &released);
    harness.run();
    harness.get_by_label("Gravity").focus();
    harness.run();
    for (i, key) in [egui::Key::ArrowRight; 4]
        .into_iter()
        .chain([egui::Key::ArrowLeft; 3])
        .enumerate()
    {
        let before = value.get();
        harness.key_press(key);
        harness.run();
        assert!(
            harness.get_by_label("Gravity").is_focused(),
            "press {i} ({key:?}): the slider keeps the focus"
        );
        let moved = value.get() - before;
        let expected = if key == egui::Key::ArrowRight { STEP } else { -STEP };
        assert!((moved - expected).abs() < 1e-9, "press {i} ({key:?}) moved {moved}");
    }
    assert!(!harness.get_by_label("Before").is_focused() && !harness.get_by_label("After").is_focused());
    drop(harness);
    assert!((value.get() - 1.01).abs() < 1e-9, "{}", value.get());
    assert_eq!(released.get(), 7, "each press and release of a key releases once");
}

/// An arrow key event: down (first or repeated) or up.
fn arrow(key: egui::Key, pressed: bool, repeat: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn a_held_arrow_releases_once_when_it_goes_up() {
    let _gpu = common::gpu();
    let (value, released) = (Cell::new(1.0), Cell::new(0));
    let mut harness = slider_between_neighbours(&value, &released);
    harness.run();
    harness.get_by_label("Gravity").focus();
    harness.run();
    harness.event(arrow(egui::Key::ArrowRight, true, false));
    harness.run();
    for repeat in 1..=4 {
        harness.event(arrow(egui::Key::ArrowRight, true, true));
        harness.run();
        let expected = 1.0 + STEP * f64::from(repeat + 1);
        assert!(
            (value.get() - expected).abs() < 1e-9,
            "repeat {repeat} moves it: {}",
            value.get()
        );
        assert_eq!(released.get(), 0, "nothing is released while the key is held");
        assert!(harness.get_by_label("Gravity").is_focused());
    }
    harness.event(arrow(egui::Key::ArrowRight, false, false));
    harness.run();
    assert_eq!(released.get(), 1, "the key going up releases once");
    harness.run();
    drop(harness);
    assert_eq!(released.get(), 1, "and only once");
    assert!((value.get() - 1.05).abs() < 1e-9, "{}", value.get());
}

#[test]
fn the_value_field_keeps_the_keyboard_between_neighbours() {
    let _gpu = common::gpu();
    let value = Cell::new(2.0);
    let mut harness = common::widget_harness((300.0, 140.0), |ui| {
        on_panel(ui);
        buttons::boxed(ui, "Above");
        ui.horizontal(|ui| {
            buttons::boxed(ui, "Before");
            let mut v = value.get();
            fields::value_field(ui, "Speed", &mut v, (0.05, 12.0), TIMES, theme::VALUE_W);
            value.set(v);
            buttons::boxed(ui, "After");
        });
        buttons::boxed(ui, "Below");
    });
    harness.run();
    harness.get_by_label("Speed").focus();
    harness.run();
    let keys = [
        egui::Key::ArrowUp,
        egui::Key::ArrowUp,
        egui::Key::ArrowDown,
        egui::Key::ArrowLeft,
        egui::Key::ArrowRight,
    ];
    let mut last = value.get();
    for (i, key) in keys.into_iter().enumerate() {
        harness.key_press(key);
        harness.run();
        assert!(
            harness.get_by_label("Speed").is_focused(),
            "press {i} ({key:?}): the field keeps the focus"
        );
        let now = value.get();
        match key {
            egui::Key::ArrowUp => assert!(now > last, "press {i}: up raises it ({last} to {now})"),
            egui::Key::ArrowDown => assert!(now < last, "press {i}: down lowers it ({last} to {now})"),
            _ => assert_eq!(now, last, "press {i}: left and right move the caret only"),
        }
        last = now;
    }
}

#[test]
fn a_disabled_value_field_is_faint_and_takes_no_input() {
    let _gpu = common::gpu();
    let value = Cell::new(2.0);
    let mut harness = common::widget_harness((200.0, 60.0), |ui| {
        on_panel(ui);
        ui.add_enabled_ui(false, |ui| {
            let mut v = value.get();
            fields::value_field(ui, "Gravity", &mut v, (0.05, 12.0), TIMES, theme::VALUE_W);
            value.set(v);
        });
    });
    harness.run();
    let node = harness.get_by_label("Gravity");
    assert!(node.accesskit_node().is_disabled());
    let rect = node.rect();
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), rgb(GROUND), "still a GROUND well");
    assert_eq!(at(&mut harness, rect, 0.0, 11.0), rgb(LINE), "with its LINE border");
    assert_eq!(
        brightest(&mut harness, rect.shrink(1.0)),
        rgb(FAINT),
        "its text is FAINT"
    );
    type_into_field(&mut harness, "7");
    assert!(!harness.get_by_label("Gravity").is_focused(), "it takes no focus");
    let start = rect.center();
    harness.drag_at(start);
    harness.run();
    harness.hover_at(start + vec2(40.0, 0.0));
    harness.run();
    harness.drop_at(start + vec2(40.0, 0.0));
    harness.run();
    drop(harness);
    assert_eq!(value.get(), 2.0, "typing and dragging do nothing");
}

#[test]
fn a_disabled_text_field_is_faint_and_takes_no_input() {
    let _gpu = common::gpu();
    let title = RefCell::new("Flute".to_owned());
    let mut harness = common::widget_harness((260.0, 60.0), |ui| {
        on_panel(ui);
        ui.add_enabled_ui(false, |ui| {
            fields::text_line(ui, "Title", &mut title.borrow_mut(), "Video title", 200.0);
        });
    });
    harness.run();
    let node = harness.get_by_label("Title");
    assert!(node.accesskit_node().is_disabled());
    let rect = node.rect();
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), rgb(GROUND), "still a GROUND well");
    assert_eq!(at(&mut harness, rect, 0.0, 13.0), rgb(LINE), "with its LINE border");
    assert_eq!(
        brightest(&mut harness, rect.shrink(1.0)),
        rgb(FAINT),
        "its text is FAINT"
    );
    harness.get_by_label("Title").focus();
    harness.run();
    harness.get_by_label("Title").click();
    harness.run();
    harness.get_by_label("Title").type_text(" vs Trombone");
    harness.run();
    assert!(!harness.get_by_label("Title").is_focused(), "it takes no focus");
    drop(harness);
    assert_eq!(title.borrow().as_str(), "Flute", "typing does nothing");
}

#[test]
fn a_compact_read_field_is_a_value_well_and_a_reason_disables_it() {
    let _gpu = common::gpu();
    let why = "New takes arrive with the music system.";
    let mut harness = common::widget_harness((320.0, 120.0), |ui| {
        on_panel(ui);
        let compact = fields::ReadLook {
            compact: true,
            why: None,
        };
        fields::read_field_with(ui, "Take number", "0417", 62.0, compact);
        let disabled = fields::ReadLook {
            compact: true,
            why: Some(why),
        };
        fields::read_field_with(ui, "Music seed", "0417", 62.0, disabled);
    });
    harness.run();
    for name in ["Take number", "Music seed"] {
        let node = harness.get_by_label(name);
        assert_eq!(node.value().as_deref(), Some("0417"), "{name}");
        assert_eq!(node.rect().height(), theme::VALUE_H, "{name} is 22 tall");
    }
    assert!(!harness.get_by_label("Take number").accesskit_node().is_disabled());
    assert!(
        harness.get_by_label("Music seed").accesskit_node().is_disabled(),
        "a reason disables it"
    );
    let plain = harness.get_by_label("Take number").rect();
    let faint = harness.get_by_label("Music seed").rect();
    assert_eq!(brightest(&mut harness, plain.shrink(1.0)), rgb(TEXT));
    assert_eq!(
        brightest(&mut harness, faint.shrink(1.0)),
        rgb(FAINT),
        "its value is FAINT"
    );
    assert_eq!(at(&mut harness, faint, 0.0, 11.0), rgb(LINE), "the well's border");
    assert_eq!(at(&mut harness, faint, 3.0, 3.0), rgb(GROUND), "the well");
    // Right-aligned: the value's last glyph pixels are near the right edge, none near the left.
    let left_part = Rect::from_min_max(faint.min + vec2(2.0, 3.0), pos2(faint.min.x + 20.0, faint.max.y - 3.0));
    assert_eq!(
        brightest(&mut harness, left_part),
        rgb(GROUND),
        "the value is right-aligned"
    );
    assert!(harness.query_by_label(why).is_none(), "not before hovering");
    harness.get_by_label("Music seed").hover();
    harness.run();
    assert!(harness.query_by_label(why).is_some(), "hovering says why");
}
