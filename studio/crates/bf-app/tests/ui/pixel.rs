//! Pixel text, fighter marks, chips and rows: exact pixels, crisp pictures from data, pictures from bad data, the
//! texture cache, clicks from the mouse and the keyboard, the focus ring, names, and a picture of them all.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use bf_app::model::roster::{PixelMask, Sprite};
use bf_app::model::take::Frame;
use bf_app::seam::sample;
use bf_app::theme::{self, FAINT, PANEL, RAISED, SELECTED, TEXT, TEXT_STRONG};
use bf_app::widgets::chips::{self, ChipOut};
use bf_app::widgets::pixel;
use egui::accesskit::{Role, Toggled};
use egui::{Color32, Rect, Ui, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common;

const FLUTE: [u8; 3] = [0xde, 0xe8, 0xf6];
const TROMBONE: [u8; 3] = [0x78, 0xf0, 0xb8];
const OBOE: [u8; 3] = [0x9c, 0xb0, 0x34];
const TUBA: [u8; 3] = [0x68, 0x5c, 0xec];
const NOT_BUILT: &str = "Not built yet. It arrives in an update.";

/// Paints the whole window `PANEL`, the ground every control sits on.
fn on_panel(ui: &mut Ui) {
    ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, PANEL);
}

fn rgb(colour: Color32) -> [u8; 3] {
    [colour.r(), colour.g(), colour.b()]
}

fn brightness([r, g, b]: [u8; 3]) -> u32 {
    u32::from(r) + u32::from(g) + u32::from(b)
}

/// Every pixel inside `rect` (whole pixels only), from one render.
fn region<S>(harness: &mut Harness<'_, S>, rect: Rect) -> Vec<[u8; 3]> {
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

/// The pixel `dx`, `dy` from the top-left corner of `rect`.
fn at(harness: &mut Harness<'_>, rect: Rect, dx: f32, dy: f32) -> [u8; 3] {
    let x = (rect.min.x + dx).round() as u32;
    let y = (rect.min.y + dy).round() as u32;
    common::pixels(harness, &[(x, y)])[0]
}

fn brightest(harness: &mut Harness<'_>, rect: Rect) -> [u8; 3] {
    region(harness, rect)
        .into_iter()
        .max_by_key(|&p| brightness(p))
        .expect("pixels")
}

/// A mask from rows of `#` (lit) and `.` (dark).
fn mask_of(rows: &[&str]) -> PixelMask {
    let w = rows[0].len();
    let on = rows.iter().flat_map(|row| row.chars().map(|c| c == '#')).collect();
    PixelMask { w, h: rows.len(), on }
}

fn letter_f() -> PixelMask {
    mask_of(&["#####", "#....", "#....", "####.", "#....", "#....", "#...."])
}

#[test]
fn pixel_text_is_exact() {
    let _gpu = common::gpu();
    let mask = letter_f();
    let mut harness = common::widget_harness((80.0, 60.0), move |ui| {
        on_panel(ui);
        pixel::pixel_text(ui, &mask, 3, Color32::WHITE, "Letter F");
    });
    harness.run();
    let node = harness.get_by_label("Letter F");
    assert_eq!(node.accesskit_node().role(), Role::Image);
    let rect = node.rect();
    assert_eq!((rect.width(), rect.height()), (15.0, 21.0));
    assert_eq!(at(&mut harness, rect, 1.0, 1.0), [0xff; 3], "the first lit square");
    assert_eq!(at(&mut harness, rect, 13.0, 4.0), rgb(PANEL), "row 1, column 4 is dark");
    let inside = region(&mut harness, rect);
    let lit = inside.iter().filter(|&&p| p == [0xff; 3]).count();
    assert_eq!(lit, 14 * 9, "14 lit pixels, each a 3 x 3 square");
    assert!(
        inside.iter().all(|&p| p == [0xff; 3] || p == rgb(PANEL)),
        "no pixel is blended"
    );
}

/// A harness like `common::widget_harness`, at `zoom` pixels per point. The caller holds `common::gpu()`.
fn zoomed_harness<'a>(zoom: f32, size: (f32, f32), ui: impl FnMut(&mut Ui) + 'a) -> Harness<'a> {
    Harness::builder()
        .with_size(vec2(size.0, size.1))
        .with_pixels_per_point(zoom)
        .with_render_cursor(false)
        .wgpu()
        .with_context_setup(theme::install)
        .build_ui(ui)
}

#[test]
fn pixel_text_is_crisp_at_a_zoom_that_is_not_whole() {
    let _gpu = common::gpu();
    let mask = letter_f();
    let mut harness = zoomed_harness(1.25, (80.0, 60.0), move |ui| {
        on_panel(ui);
        ui.add_space(0.3);
        pixel::pixel_text(ui, &mask, 2, Color32::WHITE, "Letter F");
    });
    harness.run();
    let rect = harness.get_by_label("Letter F").rect();
    let physical = Rect::from_min_max(rect.min * 1.25, rect.max * 1.25).expand(2.0);
    let image = harness.render().expect("the harness renders");
    let mut lit = 0;
    for y in physical.min.y as u32..physical.max.y as u32 {
        for x in physical.min.x as u32..physical.max.x as u32 {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            let p = [r, g, b];
            assert!(p == [0xff; 3] || p == rgb(PANEL), "pixel {x}, {y} is blended: {p:?}");
            lit += usize::from(p == [0xff; 3]);
        }
    }
    assert!(lit > 0, "the letter is drawn");
}

#[test]
fn a_mask_whose_buffer_does_not_match_draws_what_it_has() {
    let _gpu = common::gpu();
    let short = PixelMask {
        w: 4,
        h: 2,
        on: vec![true, false, true],
    };
    let long = PixelMask {
        w: 1,
        h: 1,
        on: vec![false, true, true],
    };
    let empty = PixelMask {
        w: 0,
        h: 7,
        on: Vec::new(),
    };
    let mut harness = common::widget_harness((120.0, 80.0), move |ui| {
        on_panel(ui);
        pixel::pixel_text(ui, &short, 4, Color32::WHITE, "Short");
        pixel::pixel_text(ui, &long, 4, Color32::WHITE, "Long");
        pixel::pixel_text(ui, &empty, 4, Color32::WHITE, "Empty");
    });
    harness.run();
    let short = harness.get_by_label("Short").rect();
    assert_eq!(
        (short.width(), short.height()),
        (16.0, 8.0),
        "the mask's size, whatever its buffer"
    );
    assert_eq!(
        at(&mut harness, short, 1.0, 1.0),
        [0xff; 3],
        "the pixels it has are drawn"
    );
    assert_eq!(at(&mut harness, short, 5.0, 1.0), rgb(PANEL));
    assert_eq!(at(&mut harness, short, 9.0, 1.0), [0xff; 3]);
    assert_eq!(
        at(&mut harness, short, 13.0, 1.0),
        rgb(PANEL),
        "a missing pixel is dark"
    );
    assert_eq!(at(&mut harness, short, 1.0, 5.0), rgb(PANEL), "the missing row is dark");
    let long = harness.get_by_label("Long").rect();
    assert_eq!((long.width(), long.height()), (4.0, 4.0));
    assert_eq!(at(&mut harness, long, 1.0, 1.0), rgb(PANEL), "extra pixels are ignored");
    assert_eq!(
        harness.get_by_label("Empty").rect().width(),
        0.0,
        "an empty mask takes no width"
    );
}

#[test]
fn the_series_names_fit_the_corner_card() {
    assert_eq!(sample::mask("FLUTE").w, 29);
    assert_eq!(sample::mask("TROMBONE").w, 47);
    assert_eq!(pixel::fit_scale(29, 4, 172.0), 4);
    assert_eq!(pixel::fit_scale(47, 4, 172.0), 3);
}

/// One Flute chip, 122 wide, reporting what it got into `out`.
fn chip_harness(side: Option<usize>, available: bool, out: &Cell<ChipOut>) -> Harness<'_> {
    common::widget_harness((260.0, 80.0), move |ui| {
        on_panel(ui);
        let got = chips::roster_chip(ui, "Flute", FLUTE, side, available, 122.0);
        if got.left || got.right {
            out.set(got);
        }
    })
}

fn nothing() -> ChipOut {
    ChipOut {
        left: false,
        right: false,
    }
}

#[test]
fn a_chip_reports_left_and_right_clicks() {
    let _gpu = common::gpu();
    let out = Cell::new(nothing());
    let mut harness = chip_harness(None, true, &out);
    harness.run();
    let node = harness.get_by_label("Flute");
    assert_eq!(node.accesskit_node().role(), Role::Button);
    let rect = node.rect();
    assert_eq!((rect.width(), rect.height()), (122.0, theme::ROW_H));
    node.click();
    harness.run();
    assert!(out.get().left && !out.get().right, "a click picks the left corner");
    out.set(nothing());
    harness.get_by_label("Flute").click_secondary();
    harness.run();
    assert!(
        out.get().right && !out.get().left,
        "a right-click picks the right corner"
    );
}

#[test]
fn a_chip_works_from_the_keyboard() {
    let _gpu = common::gpu();
    let out = Cell::new(nothing());
    let mut harness = chip_harness(None, true, &out);
    harness.run();
    for key in [egui::Key::Space, egui::Key::Enter] {
        out.set(nothing());
        harness.get_by_label("Flute").focus();
        harness.run();
        harness.key_press(key);
        harness.run();
        assert!(out.get().left && !out.get().right, "{key:?} picks the left corner");
    }
    out.set(nothing());
    harness.key_press(egui::Key::Menu);
    harness.run();
    assert!(
        out.get().right && !out.get().left,
        "the menu key picks the right corner"
    );
    out.set(nothing());
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F10);
    harness.run();
    assert!(out.get().right && !out.get().left, "Shift+F10 picks the right corner");
}

#[test]
fn a_chip_that_is_not_built_takes_no_clicks() {
    let _gpu = common::gpu();
    let out = Cell::new(nothing());
    let mut harness = chip_harness(None, false, &out);
    harness.run();
    let node = harness.get_by_label("Flute");
    assert!(node.accesskit_node().is_disabled());
    let rect = node.rect();
    node.click();
    harness.run();
    harness.get_by_label("Flute").click_secondary();
    harness.run();
    harness.get_by_label("Flute").focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.key_press(egui::Key::Menu);
    harness.run();
    assert!(!out.get().left && !out.get().right, "it takes no clicks");
    assert_eq!(brightest(&mut harness, rect), rgb(FAINT), "it is drawn FAINT");
    harness.get_by_label("Flute").hover();
    harness.run();
    assert!(harness.query_by_label(NOT_BUILT).is_some(), "hovering says why");
}

#[test]
fn the_selected_chip_is_ringed() {
    let _gpu = common::gpu();
    let out = Cell::new(nothing());
    let mut harness = chip_harness(Some(0), true, &out);
    harness.run();
    let node = harness.get_by_label("Flute, left corner");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
    let rect = node.rect();
    let middle = rect.height() / 2.0;
    assert_eq!(
        at(&mut harness, rect, 1.0, middle),
        FLUTE,
        "the fighter's bar at the left edge"
    );
    assert_eq!(at(&mut harness, rect, 3.0, middle), FLUTE, "the bar is 4 wide");
    assert_eq!(at(&mut harness, rect, 6.0, middle), rgb(SELECTED), "the selected fill");
    assert_eq!(
        at(&mut harness, rect, rect.width() / 2.0, 0.0),
        rgb(TEXT),
        "the ring, at the top"
    );
    assert_eq!(
        at(&mut harness, rect, rect.width() - 1.0, middle),
        rgb(TEXT),
        "the ring, at the right"
    );
    assert!(harness.query_by_label("Left").is_some(), "the small label Left");
    let name = Rect::from_min_max(rect.min + vec2(30.0, 4.0), rect.max - vec2(40.0, 4.0));
    assert_eq!(
        brightest(&mut harness, name),
        rgb(TEXT_STRONG),
        "the name is TEXT_STRONG"
    );

    drop(harness);
    let mut harness = chip_harness(Some(1), true, &out);
    harness.run();
    assert!(harness.query_by_label("Flute, right corner").is_some());
    assert!(harness.query_by_label("Right").is_some(), "the small label Right");
}

#[test]
fn a_plain_chip_has_no_ring() {
    let _gpu = common::gpu();
    let out = Cell::new(nothing());
    let mut harness = chip_harness(None, true, &out);
    harness.run();
    let node = harness.get_by_label("Flute");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::False));
    let rect = node.rect();
    assert_eq!(at(&mut harness, rect, rect.width() / 2.0, 0.0), rgb(PANEL), "no ring");
    assert_eq!(at(&mut harness, rect, 6.0, rect.height() / 2.0), rgb(PANEL), "no fill");
    assert_eq!(at(&mut harness, rect, 1.0, rect.height() / 2.0), FLUTE, "the bar");
    assert!(harness.query_by_label("Left").is_none());
    let name = Rect::from_min_max(rect.min + vec2(30.0, 4.0), rect.max - vec2(10.0, 4.0));
    assert_eq!(brightest(&mut harness, name), rgb(TEXT), "the name is TEXT");
}

#[test]
fn chips_and_rows_paint_their_focus_ring_even_at_a_panel_edge() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((300.0, 140.0), |ui| {
        on_panel(ui);
        // Each control sits in a child whose clip rectangle is exactly the control, as at a panel's edge.
        let clipped = |ui: &mut Ui, add: &dyn Fn(&mut Ui)| {
            let rect = Rect::from_min_size(ui.cursor().min + vec2(10.0, 4.0), vec2(150.0, 38.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(rect);
                add(ui);
            });
            ui.add_space(12.0);
        };
        clipped(ui, &|ui| {
            chips::roster_chip(ui, "Trombone", TROMBONE, None, true, 150.0);
        });
        clipped(ui, &|ui| {
            chips::object_row(ui, "Seed 18433", false, 38.0, |ui| ui.label("18433"));
        });
    });
    harness.run();
    for name in ["Trombone", "Seed 18433"] {
        let rect = harness.get_by_label(name).rect();
        let outside = |h: &mut Harness<'_>| at(h, rect, -3.0, rect.height() / 2.0);
        assert_ne!(outside(&mut harness), rgb(TEXT_STRONG), "{name}: no ring before focus");
        harness.get_by_label(name).focus();
        harness.run();
        assert!(harness.get_by_label(name).is_focused(), "{name} takes the focus");
        assert_eq!(
            outside(&mut harness),
            rgb(TEXT_STRONG),
            "{name}: the ring is not cut off"
        );
        assert_eq!(
            at(&mut harness, rect, rect.width() + 2.0, rect.height() / 2.0),
            rgb(TEXT_STRONG),
            "{name}: the ring on the right"
        );
        assert_ne!(
            at(&mut harness, rect, -1.0, rect.height() / 2.0),
            rgb(TEXT_STRONG),
            "{name}: the ring leaves a 2 px gap"
        );
    }
}

#[test]
fn a_chip_in_a_disabled_parent_is_faint_and_takes_no_input() {
    let _gpu = common::gpu();
    let out = Cell::new(nothing());
    let row = Cell::new(false);
    let mut harness = common::widget_harness((300.0, 120.0), |ui| {
        on_panel(ui);
        ui.add_enabled_ui(false, |ui| {
            let got = chips::roster_chip(ui, "Oboe", OBOE, None, true, 122.0);
            if got.left || got.right {
                out.set(got);
            }
            let (response, ()) = chips::object_row(ui, "Seed 902", false, 38.0, |_| ());
            if response.clicked() {
                row.set(true);
            }
        });
    });
    harness.run();
    let node = harness.get_by_label("Oboe");
    assert!(node.accesskit_node().is_disabled());
    let rect = node.rect();
    assert_eq!(brightest(&mut harness, rect), rgb(FAINT), "drawn FAINT, not faded");
    assert_eq!(
        at(&mut harness, rect, 1.0, rect.height() / 2.0),
        rgb(FAINT),
        "its bar too"
    );
    harness.get_by_label("Oboe").click();
    harness.run();
    harness.get_by_label("Oboe").click_secondary();
    harness.run();
    assert!(harness.get_by_label("Seed 902").accesskit_node().is_disabled());
    harness.get_by_label("Seed 902").click();
    harness.run();
    drop(harness);
    assert!(!out.get().left && !out.get().right, "the chip takes no clicks");
    assert!(!row.get(), "the row takes no clicks");
}

#[test]
fn a_long_name_stays_inside_its_chip() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((300.0, 60.0), |ui| {
        on_panel(ui);
        chips::roster_chip(ui, "Bass clarinet and a very long name", OBOE, Some(1), true, 120.0);
    });
    harness.run();
    let rect = harness
        .get_by_label("Bass clarinet and a very long name, right corner")
        .rect();
    let beyond = Rect::from_min_max(rect.right_top() + vec2(1.0, 0.0), rect.right_bottom() + vec2(80.0, 0.0));
    assert!(
        region(&mut harness, beyond).iter().all(|&p| p == rgb(PANEL)),
        "nothing is drawn past the chip's right edge"
    );
    let side = harness.get_by_label("Right").rect();
    let name = Rect::from_x_y_ranges(
        rect.min.x + 30.0..=side.min.x - 1.0,
        rect.min.y + 3.0..=rect.max.y - 3.0,
    );
    assert_ends_in_ellipsis(&mut harness, name, "the chip's name");
}

#[test]
fn a_long_label_in_a_row_is_cut_with_an_ellipsis() {
    let _gpu = common::gpu();
    let long = "A label far too long for this row, so it has to be cut short";
    let mut harness = common::widget_harness((300.0, 80.0), |ui| {
        on_panel(ui);
        ui.allocate_ui(vec2(200.0, 38.0), |ui| {
            chips::object_row(ui, "Seed 1", false, 38.0, |ui| ui.label(long));
        });
    });
    harness.run();
    let rect = harness.get_by_label("Seed 1").rect();
    assert_eq!(rect.width(), 200.0);
    let beyond = Rect::from_min_max(rect.right_top() + vec2(1.0, 0.0), rect.right_bottom() + vec2(80.0, 0.0));
    assert!(
        region(&mut harness, beyond).iter().all(|&p| p == rgb(PANEL)),
        "nothing is drawn past the row's right edge"
    );
    assert_ends_in_ellipsis(&mut harness, rect.shrink2(vec2(10.0, 2.0)), "the row's label");
}

/// The text in `area` (one line, bright on dark) ends in an ellipsis: its last 6 lit columns hold only dots, low on
/// the line, where any letter would reach higher.
fn assert_ends_in_ellipsis(harness: &mut Harness<'_>, area: Rect, what: &str) {
    let image = harness.render().expect("the harness renders");
    let mut lit = Vec::new();
    for y in area.min.y.ceil() as u32..area.max.y.floor() as u32 {
        for x in area.min.x.ceil() as u32..area.max.x.floor() as u32 {
            let [r, g, b, _] = image.get_pixel(x, y).0;
            if brightness([r, g, b]) > 3 * 0x80 {
                lit.push((x, y));
            }
        }
    }
    assert!(!lit.is_empty(), "{what}: text is drawn");
    let top = lit.iter().map(|p| p.1).min().unwrap_or(0) as f32;
    let bottom = lit.iter().map(|p| p.1).max().unwrap_or(0) as f32;
    let last = lit.iter().map(|p| p.0).max().unwrap_or(0);
    let tail: Vec<_> = lit.iter().filter(|p| p.0 + 6 > last).collect();
    assert!(
        tail.iter().all(|p| p.1 as f32 > top + 0.6 * (bottom - top)),
        "{what}: it ends in low dots, an ellipsis (top {top}, bottom {bottom}, tail {tail:?})"
    );
}

#[test]
fn two_chips_with_one_name_do_not_clash() {
    let _gpu = common::gpu();
    let clicks = Cell::new([0u32; 2]);
    let mut harness = common::widget_harness((300.0, 100.0), |ui| {
        on_panel(ui);
        for i in 0..2 {
            if chips::roster_chip(ui, "Flute", FLUTE, None, true, 122.0).left {
                let mut got = clicks.get();
                got[i] += 1;
                clicks.set(got);
            }
        }
    });
    harness.run();
    let second = harness.query_all_by_label("Flute").nth(1).expect("two chips");
    second.click();
    harness.run();
    drop(harness);
    assert_eq!(clicks.get(), [0, 1], "only the second chip was clicked");
}

#[test]
fn an_object_row_reports_clicks_and_holds_its_content() {
    let _gpu = common::gpu();
    let clicked = Cell::new(0);
    let name = "Seed 18433, Flute wins at 49.1 s";
    let mut harness = common::widget_harness((420.0, 120.0), |ui| {
        on_panel(ui);
        let (response, inner) = chips::object_row(ui, name, true, 38.0, |ui| {
            ui.label("18433");
            7
        });
        assert_eq!(inner, 7, "the content's value comes back");
        if response.clicked() {
            clicked.set(clicked.get() + 1);
        }
    });
    harness.run();
    let node = harness.get_by_label(name);
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
    let rect = node.rect();
    assert_eq!(rect.height(), 38.0);
    assert!(harness.query_by_label("18433").is_some(), "the content is drawn");
    harness.get_by_label("18433").click();
    harness.run();
    harness.get_by_label(name).focus();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(clicked.get(), 2, "a click on its content and Enter both click the row");
    let middle = rect.height() / 2.0;
    assert_eq!(at(&mut harness, rect, 4.0, middle), rgb(SELECTED), "the selected fill");
    assert_eq!(at(&mut harness, rect, 0.0, middle), rgb(TEXT), "the 1.5 px ring inside");
    assert_eq!(at(&mut harness, rect, rect.width() / 2.0, 0.0), rgb(TEXT));
    let text = harness.get_by_label("18433").rect();
    assert_eq!(brightest(&mut harness, text), rgb(TEXT_STRONG), "a selected row's text");
}

#[test]
fn a_plain_object_row_is_flat_until_hovered() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((420.0, 120.0), |ui| {
        on_panel(ui);
        chips::object_row(ui, "Seed 902", false, 38.0, |ui| ui.label("902"));
    });
    harness.run();
    let node = harness.get_by_label("Seed 902");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::False));
    let rect = node.rect();
    assert_eq!(at(&mut harness, rect, 4.0, 19.0), rgb(PANEL));
    assert_eq!(at(&mut harness, rect, 0.0, 19.0), rgb(PANEL), "no ring");
    let text = harness.get_by_label("902").rect();
    assert_eq!(brightest(&mut harness, text), rgb(TEXT));
    harness.get_by_label("Seed 902").hover();
    harness.run();
    assert_eq!(at(&mut harness, rect, 4.0, 19.0), rgb(theme::HOVER), "hovered");
}

#[test]
fn the_fighter_bar_is_4_wide_in_the_full_colour() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((100.0, 60.0), |ui| {
        on_panel(ui);
        chips::fighter_bar(ui, TROMBONE, 24.0);
    });
    harness.run();
    let image = harness.render().expect("the harness renders");
    let colour = |x: u32, y: u32| {
        let [r, g, b, _] = image.get_pixel(x, y).0;
        [r, g, b]
    };
    let columns: Vec<u32> = (0..30).filter(|&x| colour(x, 20) == TROMBONE).collect();
    assert_eq!(columns.len(), 4, "4 wide: {columns:?}");
    let rows = (0..60).filter(|&y| colour(columns[1], y) == TROMBONE).count();
    assert_eq!(rows, 24, "24 tall");
}

#[test]
fn a_portrait_without_a_sprite_is_a_ringed_disc() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((100.0, 100.0), |ui| {
        on_panel(ui);
        chips::portrait(ui, FLUTE, None, 52.0, "Flute");
    });
    harness.run();
    let node = harness.get_by_label("Flute");
    assert_eq!(node.accesskit_node().role(), Role::Image);
    let rect = node.rect();
    assert_eq!((rect.width(), rect.height()), (52.0, 52.0));
    let fill = rgb(theme::large_fill(FLUTE));
    assert_eq!(at(&mut harness, rect, 26.0, 26.0), fill, "the large fill");
    assert_eq!(at(&mut harness, rect, 1.0, 26.0), FLUTE, "the 3 px ring");
    assert_eq!(at(&mut harness, rect, 2.0, 26.0), FLUTE);
    assert_eq!(at(&mut harness, rect, 5.0, 26.0), fill, "inside the ring");
    assert_eq!(at(&mut harness, rect, 1.0, 1.0), rgb(PANEL), "round");
}

/// A 2 x 2 picture: red, green / blue, white.
fn quad_rgba() -> Arc<[u8]> {
    Arc::from(vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255])
}

fn quad_rgb() -> Arc<[u8]> {
    Arc::from(vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255])
}

const QUAD: [[u8; 3]; 4] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];

/// Each pixel of `rect` is exactly its quadrant's colour of a 2 x 2 picture split `split` points in.
fn assert_crisp_quad<S>(harness: &mut Harness<'_, S>, rect: Rect, split: f32) {
    let image = harness.render().expect("the harness renders");
    for y in rect.min.y as u32..rect.max.y as u32 {
        for x in rect.min.x as u32..rect.max.x as u32 {
            let right = (x as f32) >= rect.min.x + split;
            let below = (y as f32) >= rect.min.y + split;
            let want = QUAD[usize::from(below) * 2 + usize::from(right)];
            let [r, g, b, _] = image.get_pixel(x, y).0;
            assert_eq!([r, g, b], want, "pixel {x}, {y}");
        }
    }
}

#[test]
fn a_portrait_sprite_is_drawn_crisp_at_a_whole_scale() {
    let _gpu = common::gpu();
    let sprite = Sprite {
        w: 2,
        h: 2,
        rgba: quad_rgba(),
    };
    let mut harness = common::widget_harness((100.0, 100.0), move |ui| {
        on_panel(ui);
        ui.add_space(0.4);
        chips::portrait(ui, FLUTE, Some(&sprite), 53.0, "Flute");
    });
    harness.run();
    let rect = harness.get_by_label("Flute").rect();
    // 53 / 2 rounds down to a scale of 26: a 52 x 52 sprite centred in the 53 box, on whole pixels.
    let top_left = (rect.min + vec2(0.5, 0.5)).round();
    assert_crisp_quad(&mut harness, Rect::from_min_size(top_left, vec2(52.0, 52.0)), 26.0);
}

#[test]
fn a_sprite_that_does_not_match_its_buffer_falls_back_to_the_disc() {
    let _gpu = common::gpu();
    let bad = [
        Sprite {
            w: 3,
            h: 2,
            rgba: quad_rgba(),
        },
        Sprite {
            w: 0,
            h: 0,
            rgba: Arc::from(Vec::new()),
        },
        Sprite {
            w: usize::MAX,
            h: 2,
            rgba: quad_rgba(),
        },
    ];
    for sprite in bad {
        let mut harness = common::widget_harness((100.0, 100.0), move |ui| {
            on_panel(ui);
            chips::portrait(ui, TUBA, Some(&sprite), 52.0, "Tuba");
        });
        harness.run();
        let rect = harness.get_by_label("Tuba").rect();
        assert_eq!(at(&mut harness, rect, 26.0, 26.0), rgb(theme::large_fill(TUBA)));
    }
}

#[test]
fn a_picture_is_drawn_crisp_with_nearest_sampling() {
    let _gpu = common::gpu();
    let frame = Frame {
        w: 2,
        h: 2,
        rgb: quad_rgb(),
    };
    let mut harness = common::widget_harness((100.0, 100.0), move |ui| {
        on_panel(ui);
        ui.add_space(0.4);
        chips::picture(ui, "quad", &frame, vec2(20.0, 20.0), "Cover");
    });
    harness.run();
    let node = harness.get_by_label("Cover");
    assert_eq!(node.accesskit_node().role(), Role::Image);
    let rect = node.rect();
    let snapped = Rect::from_min_size(rect.min.round(), vec2(20.0, 20.0));
    assert_crisp_quad(&mut harness, snapped, 10.0);
}

#[test]
fn a_picture_from_bad_data_is_a_raised_block() {
    let _gpu = common::gpu();
    let bad = [
        Frame {
            w: 3,
            h: 2,
            rgb: quad_rgb(),
        },
        Frame {
            w: 0,
            h: 0,
            rgb: Arc::from(Vec::new()),
        },
        Frame {
            w: u32::MAX,
            h: u32::MAX,
            rgb: quad_rgb(),
        },
        Frame {
            w: 100_000,
            h: 1,
            rgb: Arc::from(vec![0; 300_000]),
        },
    ];
    for frame in bad {
        let mut harness = common::widget_harness((100.0, 100.0), move |ui| {
            on_panel(ui);
            chips::picture(ui, "bad", &frame, vec2(40.0, 60.0), "Cover");
        });
        harness.run();
        let rect = harness.get_by_label("Cover").rect();
        assert_eq!((rect.width(), rect.height()), (40.0, 60.0), "it keeps its place");
        assert!(
            region(&mut harness, rect).iter().all(|&p| p == rgb(RAISED)),
            "a RAISED block"
        );
    }
}

/// The textures the widgets made: every allocated texture whose name says it is the app's.
fn picture_textures<S>(harness: &Harness<'_, S>) -> Vec<egui::TextureId> {
    let manager = harness.ctx.tex_manager();
    let manager = manager.read();
    let mut ids: Vec<_> = manager
        .allocated()
        .filter(|(_, meta)| meta.name.starts_with("bf-app "))
        .map(|(id, _)| *id)
        .collect();
    ids.sort_by_key(|id| format!("{id:?}"));
    ids
}

#[test]
fn a_picture_texture_is_made_once_and_again_for_new_pixels() {
    let _gpu = common::gpu();
    let shown = Rc::new(RefCell::new(Frame {
        w: 2,
        h: 2,
        rgb: quad_rgb(),
    }));
    let source = shown.clone();
    let mut harness = common::widget_harness((100.0, 100.0), move |ui| {
        on_panel(ui);
        chips::picture(ui, "cover", &source.borrow(), vec2(20.0, 20.0), "Cover");
    });
    harness.run();
    let ids = picture_textures(&harness);
    assert_eq!(ids.len(), 1, "one texture");
    for _ in 0..5 {
        harness.step();
    }
    assert_eq!(picture_textures(&harness), ids, "the same texture, frame after frame");
    let rect = harness.get_by_label("Cover").rect();
    assert_eq!(at(&mut harness, rect, 2.0, 2.0), [255, 0, 0]);

    // Other pixels, in another allocation, are shown on the same texture.
    *shown.borrow_mut() = Frame {
        w: 2,
        h: 2,
        rgb: Arc::from([0, 0, 255].repeat(4)),
    };
    harness.run();
    assert_eq!(picture_textures(&harness), ids, "the texture is reused, not added");
    assert_eq!(
        at(&mut harness, rect, 2.0, 2.0),
        [0, 0, 255],
        "the new pixels are shown"
    );
}

/// The texture made for the picture keyed `key`, if one is allocated.
fn texture_of<S>(harness: &Harness<'_, S>, key: &str) -> Option<egui::TextureId> {
    let manager = harness.ctx.tex_manager();
    let manager = manager.read();
    let name = format!("bf-app picture {key}");
    manager
        .allocated()
        .find(|(_, meta)| meta.name == name)
        .map(|(id, _)| *id)
}

/// Texture ids are never reused, so a picture uploaded again shows as a new id: comparing the ids held across
/// frames counts uploads.
#[test]
fn pictures_on_screen_are_not_made_again_however_many() {
    let _gpu = common::gpu();
    for count in [65usize, 80] {
        let frames: Vec<Frame> = (0..count)
            .map(|i| Frame {
                w: 1,
                h: 1,
                rgb: Arc::from(vec![i as u8; 3]),
            })
            .collect();
        let shown = Rc::new(Cell::new(count));
        let showing = shown.clone();
        let mut harness = common::widget_harness((900.0, 200.0), move |ui| {
            on_panel(ui);
            ui.horizontal_wrapped(|ui| {
                for (i, frame) in frames.iter().enumerate().take(showing.get()) {
                    chips::picture(ui, &format!("tile {i}"), frame, vec2(8.0, 8.0), &format!("Tile {i}"));
                }
            });
        });
        harness.run();
        let first = picture_textures(&harness);
        assert_eq!(first.len(), count, "{count} on screen: one texture each");
        for _ in 0..5 {
            harness.step();
        }
        assert_eq!(
            picture_textures(&harness),
            first,
            "{count} on screen: no upload after the first frame"
        );

        let ten: Vec<_> = (0..10).map(|i| texture_of(&harness, &format!("tile {i}"))).collect();
        shown.set(10);
        for _ in 0..3 {
            harness.step();
        }
        assert_eq!(
            picture_textures(&harness).len(),
            64,
            "{count}: back to 64 once fewer are on screen"
        );
        let still: Vec<_> = (0..10).map(|i| texture_of(&harness, &format!("tile {i}"))).collect();
        assert_eq!(still, ten, "{count}: the ten still on screen were not made again");
    }
}

#[test]
fn a_portrait_or_picture_without_a_size_draws_nothing() {
    let _gpu = common::gpu();
    for size in [f32::NAN, 0.0, -5.0, f32::INFINITY] {
        let frame = Frame {
            w: 2,
            h: 2,
            rgb: quad_rgb(),
        };
        let mut harness = common::widget_harness((100.0, 100.0), move |ui| {
            on_panel(ui);
            chips::portrait(ui, FLUTE, None, size, "Flute");
            chips::picture(ui, "no size", &frame, vec2(size, 20.0), "Cover");
            chips::picture(ui, "no size 2", &frame, vec2(20.0, size), "Cover 2");
        });
        harness.run();
        for name in ["Flute", "Cover", "Cover 2"] {
            let rect = harness.get_by_label(name).rect();
            assert!(
                rect.width() == 0.0 || rect.height() == 0.0,
                "{size}: {name} takes no space"
            );
        }
        let window = Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(100.0, 100.0));
        assert!(
            region(&mut harness, window).iter().all(|&p| p == rgb(PANEL)),
            "{size}: nothing is drawn"
        );
    }
}

/// The gallery: both names at scales 3 and 4, both portraits, two fighter bars, four chips (one chosen, one not
/// built), one selected and one plain row.
fn gallery(ui: &mut Ui) {
    on_panel(ui);
    let flute = sample::mask("FLUTE");
    let trombone = sample::mask("TROMBONE");
    for scale in [3, 4] {
        ui.horizontal(|ui| {
            pixel::pixel_text(ui, &flute, scale, theme::rgb(FLUTE), "Flute");
            pixel::pixel_text(ui, &trombone, scale, theme::rgb(TROMBONE), "Trombone");
        });
    }
    ui.horizontal(|ui| {
        chips::portrait(ui, FLUTE, None, 52.0, "Flute");
        chips::portrait(ui, TROMBONE, None, 52.0, "Trombone");
        chips::fighter_bar(ui, FLUTE, 24.0);
        chips::fighter_bar(ui, TROMBONE, 24.0);
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        chips::roster_chip(ui, "Flute", FLUTE, Some(0), true, 122.0);
        chips::roster_chip(ui, "Oboe", OBOE, None, true, 122.0);
        chips::roster_chip(ui, "Trombone", TROMBONE, None, true, 122.0);
        chips::roster_chip(ui, "Tuba", TUBA, None, false, 122.0);
    });
    let row = |ui: &mut Ui, seed: &str, winner: &str, colour: [u8; 3]| {
        ui.label(egui::RichText::new(seed).font(theme::value()));
        chips::fighter_bar(ui, colour, 18.0);
        ui.label(winner);
    };
    chips::object_row(ui, "Seed 18433, Flute wins at 49.1 s", true, 38.0, |ui| {
        row(ui, "18433", "Flute", FLUTE);
    });
    chips::object_row(ui, "Seed 7731, Trombone wins at 50.2 s", false, 38.0, |ui| {
        row(ui, "7731", "Trombone", TROMBONE);
    });
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((540.0, 330.0), gallery);
    harness.run();
    assert!(harness.unnamed_widgets().is_empty(), "{:?}", harness.unnamed_widgets());
    assert!(harness.query_by_label("Flute, left corner").is_some());
    assert!(harness.get_by_label("Tuba").accesskit_node().is_disabled());
    assert_eq!(
        harness.query_all_by_label("Trombone").count(),
        5,
        "two names, a portrait, a chip and a row's winner"
    );
}

#[test]
fn the_gallery_matches() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((540.0, 330.0), gallery);
    harness.run();
    harness.snapshot("widgets_chips");
}
