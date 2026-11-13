//! The lead ribbon: its sizes, its fills on each side of the centre line, the K.O. bar and window, data that cannot
//! be used, a disabled parent, and a picture of the small and the timeline ribbon.

use std::cell::Cell;

use bf_app::model::story::{AbilityMark, StoryView};
use bf_app::theme::{FAINT, LINE, PANEL, RAISED, TEXT_STRONG};
use bf_app::widgets::ribbon;
use egui::{Rect, Sense, Ui, pos2, vec2};
use egui_kittest::Harness;

use crate::common;

const FLUTE: [u8; 3] = [0xde, 0xe8, 0xf6];
const TROMBONE: [u8; 3] = [0x78, 0xf0, 0xb8];
const COLORS: [[u8; 3]; 2] = [FLUTE, TROMBONE];
const PANEL_RGB: [u8; 3] = [PANEL.r(), PANEL.g(), PANEL.b()];

fn rgb(colour: egui::Color32) -> [u8; 3] {
    [colour.r(), colour.g(), colour.b()]
}

/// Paints the whole window `PANEL`, the ground the ribbon sits on.
fn on_panel(ui: &mut Ui) {
    ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, PANEL);
}

/// The brief's story: FIGHT! at 3.6 s, Flute 21 HP ahead at 10 s, Trombone 21 ahead at 20 s, K.O. at 51 s.
fn story() -> StoryView {
    StoryView {
        seed: 18433,
        winner: Some(1),
        t_ko: Some(51.0),
        winner_hp: 16.0,
        lead_changes: 1,
        score: 7.84,
        fits: true,
        misses: Vec::new(),
        t_fight: 3.6,
        lead: vec![(10.0, 21.0), (20.0, -21.0)],
        firsts: vec![AbilityMark {
            t: 12.0,
            who: 0,
            name: "Flutter tongue".to_string(),
        }],
    }
}

/// The pixel at time `t` of the ribbon at `rect`, `dy` below its top.
fn at_time(harness: &mut Harness<'_>, rect: Rect, t: f32, dy: f32) -> [u8; 3] {
    let x = ribbon::x_of(t, rect.min.x, rect.width()).floor() as u32;
    let y = (rect.min.y + dy).floor() as u32;
    common::pixels(harness, &[(x, y)])[0]
}

/// A harness holding one small ribbon `width` wide, inside a disabled parent when `disabled`. Its rect, and whether
/// it was ever clicked, hovered-and-focused or dragged, are kept in the cells.
fn small_harness<'a>(width: f32, disabled: bool, rect: &'a Cell<Rect>, input: &'a Cell<bool>) -> Harness<'a> {
    common::widget_harness((440.0, 60.0), move |ui| {
        on_panel(ui);
        if disabled {
            ui.disable();
        }
        let response = ribbon::small(ui, &story(), COLORS, width);
        rect.set(response.rect);
        if response.clicked() || response.has_focus() || response.dragged() {
            input.set(true);
        }
    })
}

#[test]
fn the_small_ribbon_is_28_tall() {
    let _gpu = common::gpu();
    let (rect, input) = (Cell::new(Rect::NOTHING), Cell::new(false));
    let mut harness = small_harness(400.0, false, &rect, &input);
    harness.run();
    let r = rect.get();
    assert_eq!((r.width(), r.height()), (400.0, 28.0));

    // The left fighter's colour above the line while it leads, the right one's below.
    assert_eq!(
        at_time(&mut harness, r, 10.0, 11.0),
        FLUTE,
        "Flute leads at 10 s, above"
    );
    assert_eq!(
        at_time(&mut harness, r, 10.0, 17.0),
        PANEL_RGB,
        "nothing below while Flute leads"
    );
    assert_eq!(
        at_time(&mut harness, r, 20.0, 17.0),
        TROMBONE,
        "Trombone leads at 20 s, below"
    );
    assert_eq!(
        at_time(&mut harness, r, 20.0, 11.0),
        PANEL_RGB,
        "nothing above while Trombone leads"
    );
    // 21 HP fills half of the 13 a side may fill: nothing reaches 3 px from the top at the peak.
    assert_eq!(at_time(&mut harness, r, 10.0, 3.0), PANEL_RGB);
    // Before FIGHT! only the 1 px centre line shows.
    assert_eq!(at_time(&mut harness, r, 1.0, 14.0), rgb(LINE), "the centre line");
    assert_eq!(at_time(&mut harness, r, 1.0, 12.0), PANEL_RGB);
    // The 2 px K.O. bar at 51 s, inset 2 from top and bottom; the small ribbon has no K.O. window.
    assert_eq!(
        at_time(&mut harness, r, 51.0, 2.0),
        rgb(TEXT_STRONG),
        "the K.O. bar starts 2 down"
    );
    assert_eq!(at_time(&mut harness, r, 51.0, 25.0), rgb(TEXT_STRONG), "and ends 2 up");
    assert_eq!(at_time(&mut harness, r, 51.0, 1.0), PANEL_RGB);
    assert_eq!(at_time(&mut harness, r, 51.0, 26.0), PANEL_RGB);
    assert_eq!(
        at_time(&mut harness, r, 50.0, 5.0),
        PANEL_RGB,
        "no K.O. window on a small ribbon"
    );

    // It takes no input: a click, a drag and the keyboard all pass it by. It is not a control, so it has no node
    // the harness's name check could ask a name of.
    assert!(harness.unnamed_widgets().is_empty(), "{:?}", harness.unnamed_widgets());
    harness.drag_at(r.center());
    harness.run();
    harness.drop_at(r.center());
    harness.run();
    harness.drag_at(r.center());
    harness.run();
    harness.drop_at(r.center() + vec2(40.0, 0.0));
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(!input.get(), "the small ribbon takes no input");
}

#[test]
fn the_small_ribbon_senses_hover_only() {
    let _gpu = common::gpu();
    let sense = Cell::new(Sense::click_and_drag());
    let mut harness = common::widget_harness((440.0, 60.0), |ui| {
        sense.set(ribbon::small(ui, &story(), COLORS, 400.0).sense);
    });
    harness.run();
    drop(harness);
    assert_eq!(sense.get(), Sense::hover());
}

#[test]
fn inside_a_disabled_parent_it_draws_faint() {
    let _gpu = common::gpu();
    let (rect, input) = (Cell::new(Rect::NOTHING), Cell::new(false));
    let mut harness = small_harness(400.0, true, &rect, &input);
    harness.run();
    let r = rect.get();
    assert_eq!(
        at_time(&mut harness, r, 10.0, 11.0),
        rgb(FAINT),
        "Flute's fill is FAINT, not faded"
    );
    assert_eq!(at_time(&mut harness, r, 20.0, 17.0), rgb(FAINT), "so is Trombone's");
    assert_eq!(at_time(&mut harness, r, 51.0, 5.0), rgb(FAINT), "and the K.O. bar");
    assert_eq!(
        at_time(&mut harness, r, 1.0, 14.0),
        rgb(LINE),
        "the centre line keeps its grey"
    );
    harness.drag_at(r.center());
    harness.run();
    harness.drop_at(r.center());
    harness.run();
    assert!(!input.get());
}

/// A harness that paints the timeline's ribbon, 1320 x 41 with the K.O. window, its rect kept in the cell.
fn timeline_harness<'a>(story: StoryView, rect: &'a Cell<Rect>) -> Harness<'a> {
    common::widget_harness((1340.0, 60.0), move |ui| {
        on_panel(ui);
        let (r, _) = ui.allocate_exact_size(vec2(1320.0, 41.0), Sense::hover());
        ribbon::paint(ui.painter(), r, &story, COLORS, true);
        rect.set(r);
    })
}

#[test]
fn the_timeline_ribbon_shades_the_ko_window() {
    let _gpu = common::gpu();
    let rect = Cell::new(Rect::NOTHING);
    let mut harness = timeline_harness(story(), &rect);
    harness.run();
    let r = rect.get();
    // 49 to 53 s, full height, under the fills and the line; Trombone's fill runs below the line there.
    assert_eq!(at_time(&mut harness, r, 49.2, 0.0), rgb(RAISED), "from the very top");
    assert_eq!(at_time(&mut harness, r, 52.8, 40.0), rgb(RAISED), "to the very bottom");
    assert_eq!(at_time(&mut harness, r, 50.0, 10.0), rgb(RAISED));
    assert_eq!(at_time(&mut harness, r, 48.7, 10.0), PANEL_RGB, "not before 49 s");
    assert_eq!(at_time(&mut harness, r, 53.3, 10.0), PANEL_RGB, "nor after 53 s");
    assert_eq!(
        at_time(&mut harness, r, 52.0, 20.0),
        rgb(LINE),
        "the centre line crosses it"
    );
    assert_eq!(
        at_time(&mut harness, r, 51.0, 3.0),
        rgb(TEXT_STRONG),
        "the K.O. bar is on top"
    );
    assert_eq!(at_time(&mut harness, r, 51.0, 1.0), rgb(RAISED));
    // A 42 HP lead would fill a side; 21 HP fills half of the 19.5 points.
    assert_eq!(at_time(&mut harness, r, 10.0, 12.0), FLUTE);
    assert_eq!(at_time(&mut harness, r, 10.0, 9.0), PANEL_RGB);
}

#[test]
fn without_a_ko_there_is_no_bar() {
    let _gpu = common::gpu();
    let rect = Cell::new(Rect::NOTHING);
    let story = StoryView { t_ko: None, ..story() };
    let mut harness = timeline_harness(story, &rect);
    harness.run();
    let r = rect.get();
    for t in [20.0, 30.0, 51.0, 60.9] {
        let column: Vec<[u8; 3]> = (0..41).map(|dy| at_time(&mut harness, r, t, dy as f32)).collect();
        assert!(!column.contains(&rgb(TEXT_STRONG)), "no bar at {t} s");
    }
    // The curve ends at the last hit, so Trombone's fill stops at 20 s.
    assert_eq!(at_time(&mut harness, r, 19.8, 22.0), TROMBONE);
    assert_eq!(at_time(&mut harness, r, 25.0, 22.0), PANEL_RGB);
}

/// Every pixel in the 1 px ring around `rect`.
fn ring_around(harness: &mut Harness<'_>, rect: Rect) -> Vec<[u8; 3]> {
    let outer = rect.expand(1.0);
    let (x0, y0) = (outer.min.x as u32, outer.min.y as u32);
    let (x1, y1) = (outer.max.x as u32 - 1, outer.max.y as u32 - 1);
    let mut at = Vec::new();
    for x in x0..=x1 {
        at.push((x, y0));
        at.push((x, y1));
    }
    for y in y0..=y1 {
        at.push((x0, y));
        at.push((x1, y));
    }
    common::pixels(harness, &at)
}

/// A story nobody should send: leads far beyond 42 HP, not-a-number and infinite values, hits out of order, on one
/// instant, before FIGHT! and after the end, a K.O. on the last instant and ability marks outside the fight.
fn wild() -> StoryView {
    StoryView {
        t_fight: -3.0,
        t_ko: Some(61.0),
        lead: vec![
            (40.0, -1e9),
            (5.0, 1e9),
            (5.0, -1e9),
            (f32::NAN, 9.0),
            (30.0, f32::INFINITY),
            (-9.0, 30.0),
            (70.0, -30.0),
            (61.0, 60.0),
        ],
        firsts: vec![AbilityMark {
            t: 99.0,
            who: 9,
            name: "Pedal tone".to_string(),
        }],
        ..story()
    }
}

#[test]
fn nothing_is_painted_outside_the_ribbon() {
    let _gpu = common::gpu();
    let rect = Cell::new(Rect::NOTHING);
    let mut harness = timeline_harness(wild(), &rect);
    harness.run();
    let r = rect.get();
    let ring = ring_around(&mut harness, r);
    assert!(
        ring.iter().all(|&p| p == PANEL_RGB),
        "{:?}",
        ring.iter().find(|&&p| p != PANEL_RGB)
    );
    // The K.O. on the last instant is drawn inside the right edge.
    let edge = common::pixels(&mut harness, &[(r.max.x as u32 - 1, (r.min.y + 5.0) as u32)])[0];
    assert_eq!(edge, rgb(TEXT_STRONG));
}

#[test]
fn a_ribbon_with_no_room_paints_nothing() {
    let _gpu = common::gpu();
    let rects = [
        Rect::from_min_size(pos2(40.0, 20.0), vec2(0.0, 41.0)),
        Rect::from_min_size(pos2(40.0, 20.0), vec2(300.0, 0.0)),
        Rect::from_min_max(pos2(340.0, 20.0), pos2(40.0, 61.0)),
        Rect::from_min_max(pos2(40.0, 61.0), pos2(340.0, 20.0)),
        Rect::from_min_max(pos2(f32::NAN, 20.0), pos2(340.0, 61.0)),
        Rect::from_min_max(pos2(40.0, 20.0), pos2(f32::INFINITY, 61.0)),
        Rect::NOTHING,
        Rect::EVERYTHING,
    ];
    for bad in rects {
        let mut harness = common::widget_harness((400.0, 80.0), move |ui| {
            on_panel(ui);
            ribbon::paint(ui.painter(), bad, &wild(), COLORS, true);
        });
        harness.run();
        let image = harness.render().expect("the harness renders");
        let painted = image.pixels().any(|p| [p.0[0], p.0[1], p.0[2]] != PANEL_RGB);
        assert!(!painted, "{bad:?} painted something");
    }
}

#[test]
fn a_small_ribbon_with_no_width_takes_no_room() {
    let _gpu = common::gpu();
    for width in [0.0, -20.0, f32::NAN, f32::NEG_INFINITY, f32::INFINITY] {
        let (rect, input) = (Cell::new(Rect::NOTHING), Cell::new(false));
        let mut harness = small_harness(width, false, &rect, &input);
        harness.run();
        let r = rect.get();
        assert_eq!((r.width(), r.height()), (0.0, 28.0), "{width}");
        let image = harness.render().expect("the harness renders");
        assert!(image.pixels().all(|p| [p.0[0], p.0[1], p.0[2]] == PANEL_RGB), "{width}");
    }
}

/// The small ribbon at 400 x 28 above the timeline's at 1320 x 41 with the K.O. window, on `PANEL`.
fn gallery(ui: &mut Ui) {
    on_panel(ui);
    ribbon::small(ui, &story(), COLORS, 400.0);
    ui.add_space(10.0);
    let (rect, _) = ui.allocate_exact_size(vec2(1320.0, 41.0), Sense::hover());
    ribbon::paint(ui.painter(), rect, &story(), COLORS, true);
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((1340.0, 110.0), gallery);
    harness.run();
    assert!(harness.unnamed_widgets().is_empty(), "{:?}", harness.unnamed_widgets());
}

#[test]
fn the_gallery_matches() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((1340.0, 110.0), gallery);
    harness.run();
    harness.snapshot("widgets_ribbon");
}
