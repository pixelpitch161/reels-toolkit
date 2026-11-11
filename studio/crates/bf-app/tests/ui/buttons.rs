//! The basic controls: buttons, segments, the switch, the dropdown, the icons and the progress bar. Sizes, fills,
//! label colours in every inverted state, the focus ring, names, and a picture of them all.

use std::cell::Cell;

use bf_app::theme::{self, FAINT, LINE, ON_LIGHT, PANEL, RAISED, TEXT, TEXT_STRONG};
use bf_app::widgets::buttons;
use bf_app::widgets::icons::{self, Icon};
use egui::accesskit::{Role, Toggled};
use egui::{Rect, Ui};
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

fn brightness([r, g, b]: [u8; 3]) -> u32 {
    u32::from(r) + u32::from(g) + u32::from(b)
}

/// The pixel `dx`, `dy` from the top-left corner of `rect`.
fn at(harness: &mut Harness<'_>, rect: Rect, dx: f32, dy: f32) -> [u8; 3] {
    let x = (rect.min.x + dx).round() as u32;
    let y = (rect.min.y + dy).round() as u32;
    common::pixels(harness, &[(x, y)])[0]
}

/// An inverted control: its fill is `TEXT` and its label is drawn in `ON_LIGHT`, so the darkest pixel inside the
/// fill is exactly `ON_LIGHT` and there are glyph pixels at all (a label left in `TEXT` would vanish into the fill).
fn assert_inverted_with_on_light_label(harness: &mut Harness<'_>, rect: Rect, what: &str) {
    let inside = region(harness, rect.shrink(2.0));
    let fill = inside.iter().filter(|&&p| p == rgb(TEXT)).count();
    assert!(fill * 2 > inside.len(), "{what}: most of it is the TEXT fill");
    let glyphs = inside.iter().filter(|&&p| brightness(p) < 3 * 0x80).count();
    assert!(
        glyphs >= 10,
        "{what}: dark glyph pixels inside the fill, found {glyphs}"
    );
    let darkest = *inside.iter().min_by_key(|&&p| brightness(p)).expect("pixels");
    assert_eq!(darkest, rgb(ON_LIGHT), "{what}: the label is ON_LIGHT");
}

/// The brightest pixel inside `rect`: the colour a label is drawn in, on a darker fill.
fn brightest(harness: &mut Harness<'_>, rect: Rect) -> [u8; 3] {
    region(harness, rect)
        .into_iter()
        .max_by_key(|&p| brightness(p))
        .expect("pixels")
}

type Make = fn(&mut Ui, &str) -> egui::Response;

/// The five buttons, each called with the label it is given.
fn kinds() -> [(&'static str, Make); 5] {
    [
        ("flat", |ui, label| buttons::flat(ui, label)),
        ("toggle", |ui, label| buttons::toggle(ui, label, false)),
        ("boxed", |ui, label| buttons::boxed(ui, label)),
        ("primary", |ui, label| buttons::primary(ui, label)),
        ("disabled", |ui, label| {
            buttons::disabled(ui, label, "Updates from inside the app arrive in a later version.")
        }),
    ]
}

#[test]
fn heights_are_26() {
    let _gpu = common::gpu();
    for (kind, make) in kinds() {
        let mut harness = common::widget_harness((240.0, 60.0), move |ui| {
            on_panel(ui);
            make(ui, "Swap sides");
        });
        harness.run();
        assert_eq!(
            harness.get_by_label("Swap sides").rect().height(),
            26.0,
            "{kind} is 26 tall"
        );
    }
}

#[test]
fn fills_match_the_tokens() {
    let _gpu = common::gpu();
    let cases: [(&str, Make, [u8; 3]); 4] = [
        ("flat", |ui, label| buttons::flat(ui, label), rgb(PANEL)),
        ("primary", |ui, label| buttons::primary(ui, label), [0xe2; 3]),
        ("toggle on", |ui, label| buttons::toggle(ui, label, true), [0xe2; 3]),
        ("boxed", |ui, label| buttons::boxed(ui, label), [0x33; 3]),
    ];
    for (kind, make, fill) in cases {
        let mut harness = common::widget_harness((240.0, 60.0), move |ui| {
            on_panel(ui);
            make(ui, "Swap sides");
        });
        harness.run();
        let rect = harness.get_by_label("Swap sides").rect();
        assert_eq!(at(&mut harness, rect, 3.0, 3.0), fill, "{kind}");
    }
}

#[test]
fn a_box_button_has_a_line_border_and_a_flat_one_hovers() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((240.0, 80.0), |ui| {
        on_panel(ui);
        buttons::boxed(ui, "Swap sides");
        buttons::flat(ui, "Undo");
    });
    harness.run();
    let boxed = harness.get_by_label("Swap sides").rect();
    assert_eq!(at(&mut harness, boxed, 0.0, 13.0), rgb(LINE), "1 px LINE border");
    let flat = harness.get_by_label("Undo").rect();
    assert_eq!(at(&mut harness, flat, 3.0, 3.0), rgb(PANEL));
    harness.get_by_label("Undo").hover();
    harness.run();
    assert_eq!(at(&mut harness, flat, 3.0, 3.0), rgb(theme::HOVER), "hovered");
}

#[test]
fn inverted_buttons_draw_their_label_on_light() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((240.0, 80.0), |ui| {
        on_panel(ui);
        buttons::primary(ui, "Search seeds");
        buttons::toggle(ui, "Loop", true);
    });
    harness.run();
    let primary = harness.get_by_label("Search seeds").rect();
    assert_inverted_with_on_light_label(&mut harness, primary, "primary");
    let toggle = harness.get_by_label("Loop");
    assert_eq!(toggle.accesskit_node().toggled(), Some(Toggled::True));
    let toggle = toggle.rect();
    assert_inverted_with_on_light_label(&mut harness, toggle, "toggle on");
}

#[test]
fn a_toggle_that_is_off_is_flat() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((240.0, 60.0), |ui| {
        on_panel(ui);
        buttons::toggle(ui, "Loop", false);
    });
    harness.run();
    let toggle = harness.get_by_label("Loop");
    assert_eq!(toggle.accesskit_node().toggled(), Some(Toggled::False));
    let rect = toggle.rect();
    assert_eq!(at(&mut harness, rect, 3.0, 3.0), rgb(PANEL));
    assert_eq!(brightest(&mut harness, rect), rgb(TEXT), "its label is TEXT");
}

#[test]
fn a_disabled_button_is_faint_says_why_and_does_not_hover() {
    let _gpu = common::gpu();
    let clicked = Cell::new(false);
    let why = "Updates from inside the app arrive in a later version.";
    let mut harness = common::widget_harness((420.0, 120.0), |ui| {
        on_panel(ui);
        if buttons::disabled(ui, "Update", why).clicked() {
            clicked.set(true);
        }
    });
    harness.run();
    let node = harness.get_by_label("Update");
    assert!(node.accesskit_node().is_disabled());
    let rect = node.rect();
    assert_eq!(brightest(&mut harness, rect), rgb(FAINT), "its label is FAINT");
    harness.get_by_label("Update").hover();
    harness.run();
    assert_eq!(at(&mut harness, rect, 3.0, 3.0), rgb(PANEL), "no hover fill");
    assert!(harness.query_by_label(why).is_some(), "hovering says why");
    harness.get_by_label("Update").click();
    harness.run();
    drop(harness);
    assert!(!clicked.get(), "a disabled button does nothing");
}

fn winner(ui: &mut Ui, flute: Option<&str>, chosen: Option<usize>) -> Option<usize> {
    buttons::segmented(
        ui,
        "Winner",
        &[("Any", None), ("Flute", flute), ("Trombone", None)],
        chosen,
        206.0,
    )
}

#[test]
fn a_segment_click_returns_its_index() {
    let _gpu = common::gpu();
    let clicked = Cell::new(None);
    let mut harness = common::widget_harness((260.0, 60.0), |ui| {
        on_panel(ui);
        if let Some(index) = winner(ui, None, Some(0)) {
            clicked.set(Some(index));
        }
    });
    harness.run();
    harness.get_by_label("Trombone").click();
    harness.run();
    drop(harness);
    assert_eq!(clicked.get(), Some(2));

    let clicked = Cell::new(None);
    let mut harness = common::widget_harness((260.0, 60.0), |ui| {
        on_panel(ui);
        if let Some(index) = winner(ui, Some("Flute has no bass part"), Some(0)) {
            clicked.set(Some(index));
        }
    });
    harness.run();
    assert!(harness.get_by_label("Flute").accesskit_node().is_disabled());
    harness.get_by_label("Flute").click();
    harness.run();
    drop(harness);
    assert_eq!(clicked.get(), None, "a segment that cannot be chosen returns nothing");
}

#[test]
fn segments_share_one_border_and_the_chosen_one_is_inverted() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((260.0, 60.0), |ui| {
        on_panel(ui);
        winner(ui, Some("Flute has no bass part"), Some(2));
    });
    harness.run();
    let any = harness.get_by_label("Any").rect();
    let flute = harness.get_by_label("Flute").rect();
    let trombone = harness.get_by_label("Trombone");
    assert_eq!(trombone.accesskit_node().toggled(), Some(Toggled::True));
    let trombone = trombone.rect();
    assert_eq!(any.height(), 24.0, "segments are 24 tall");
    assert_eq!(any.max.x, flute.min.x, "no gap");
    assert_eq!(trombone.max.x - any.min.x, 206.0, "the given width");
    assert_eq!(at(&mut harness, any, 0.0, 12.0), rgb(LINE), "the outer border");
    assert_eq!(at(&mut harness, flute, 0.0, 12.0), rgb(LINE), "the shared border");
    assert_eq!(at(&mut harness, any, 3.0, 3.0), rgb(PANEL));
    assert_inverted_with_on_light_label(&mut harness, trombone, "the chosen segment");
    assert_eq!(
        brightest(&mut harness, flute.shrink(2.0)),
        rgb(FAINT),
        "a segment that cannot be chosen"
    );
    harness.get_by_label("Flute").hover();
    harness.run();
    assert!(
        harness.query_by_label("Flute has no bass part").is_some(),
        "hovering says why"
    );
}

#[test]
fn the_switch_reports_clicks_and_its_state() {
    let _gpu = common::gpu();
    let clicked = Cell::new(false);
    let label = "Show fights outside the beat sheet";
    let mut harness = common::widget_harness((320.0, 60.0), |ui| {
        on_panel(ui);
        if buttons::switch(ui, label, false).clicked() {
            clicked.set(true);
        }
    });
    harness.run();
    let node = harness.get_by_label(label);
    assert_eq!(node.accesskit_node().role(), Role::Switch);
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::False));
    node.click();
    harness.run();
    drop(harness);
    assert!(clicked.get());
}

#[test]
fn the_switch_is_faint_when_off_and_text_when_on() {
    let _gpu = common::gpu();
    for (on, colour) in [(false, FAINT), (true, TEXT)] {
        let mut harness = common::widget_harness((320.0, 60.0), move |ui| {
            on_panel(ui);
            buttons::switch(ui, "Loop", on);
        });
        harness.run();
        let rect = harness.get_by_label("Loop").rect();
        let top = rect.center().y - 8.0;
        let outline = at(&mut harness, rect, 15.0, top - rect.min.y);
        assert_eq!(outline, rgb(colour), "the outline, on = {on}");
        let (knob_off, knob_on) = (8.0, 22.0);
        let knob = if on { knob_on } else { knob_off };
        let empty = if on { knob_off } else { knob_on };
        let middle = rect.center().y - rect.min.y;
        assert_eq!(at(&mut harness, rect, knob, middle), rgb(colour), "the knob, on = {on}");
        assert_eq!(
            at(&mut harness, rect, empty, middle),
            rgb(PANEL),
            "the other end, on = {on}"
        );
    }
}

fn theme_dropdown(ui: &mut Ui) -> Option<usize> {
    buttons::dropdown(ui, "Theme", &["Big top".into(), "Dark disco".into()], 0, 220.0)
}

#[test]
fn the_dropdown_lists_and_returns() {
    let _gpu = common::gpu();
    let picked = Cell::new(None);
    let mut harness = common::widget_harness((300.0, 160.0), |ui| {
        on_panel(ui);
        if let Some(index) = theme_dropdown(ui) {
            picked.set(Some(index));
        }
    });
    harness.run();
    let node = harness.get_by_label("Theme");
    assert_eq!(node.value().as_deref(), Some("Big top"), "it says what is chosen");
    node.click();
    harness.run();
    harness.get_by_label("Dark disco").click();
    harness.run();
    assert!(harness.query_by_label("Dark disco").is_none(), "the list closes");
    drop(harness);
    assert_eq!(picked.get(), Some(1));
}

#[test]
fn the_dropdown_looks_as_the_reference_says() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((300.0, 160.0), |ui| {
        on_panel(ui);
        theme_dropdown(ui);
    });
    harness.run();
    let rect = harness.get_by_label("Theme").rect();
    assert_eq!(rect.width(), 220.0);
    assert_eq!(rect.height(), theme::VALUE_H);
    assert_eq!(at(&mut harness, rect, 0.0, 11.0), rgb(LINE), "1 px LINE border");
    assert_eq!(at(&mut harness, rect, 3.0, 3.0), rgb(RAISED), "RAISED fill");
    let chevron = Rect::from_min_max(
        rect.right_top() + egui::vec2(-22.0, 2.0),
        rect.right_bottom() - egui::vec2(2.0, 2.0),
    );
    // A 1.5 px diagonal line is smoothed, so no pixel is fully covered: its brightest pixel is MUTED faded a little
    // towards the fill, and never brighter than MUTED.
    let mark = brightest(&mut harness, chevron);
    assert!(
        mark[0] == mark[1] && mark[1] == mark[2],
        "a grey chevron, found {mark:?}"
    );
    assert!((0x90..=0xa0).contains(&mark[0]), "a MUTED chevron, found {mark:?}");

    harness.get_by_label("Theme").click();
    harness.run();
    assert_inverted_with_on_light_label(&mut harness, rect, "the open dropdown's button");
    let chosen = harness.get_by_label("Big top").rect();
    assert_eq!(chosen.height(), 24.0, "items are 24 tall");
    assert_inverted_with_on_light_label(&mut harness, chosen, "the chosen item");
    let other = harness.get_by_label("Dark disco").rect();
    assert_eq!(
        at(&mut harness, other, 2.0, 2.0),
        rgb(RAISED),
        "the list fills with RAISED"
    );
    assert_eq!(brightest(&mut harness, other), rgb(TEXT));
    assert_eq!(
        at(&mut harness, other, -4.0, 12.0),
        rgb(LINE),
        "the list's 1 px LINE border"
    );

    harness.get_by_label("Dark disco").hover();
    harness.run();
    assert_inverted_with_on_light_label(&mut harness, other, "the hovered item");
    assert_eq!(
        at(&mut harness, chosen, 2.0, 2.0),
        rgb(RAISED),
        "only one item is inverted"
    );
}

/// One control of each kind; returns nothing.
fn gallery(ui: &mut Ui) {
    on_panel(ui);
    ui.horizontal(|ui| {
        buttons::flat(ui, "Undo");
        buttons::toggle(ui, "Loop", true);
        buttons::toggle(ui, "Mute", false);
        buttons::boxed(ui, "Swap sides");
    });
    ui.horizontal(|ui| {
        buttons::primary(ui, "Search seeds");
        buttons::disabled(ui, "Update", "Updates from inside the app arrive in a later version.");
    });
    buttons::segmented(
        ui,
        "Winner",
        &[
            ("Any", None),
            ("Flute", Some("Flute has no bass part")),
            ("Trombone", None),
        ],
        Some(0),
        206.0,
    );
    buttons::switch(ui, "Show fights outside the beat sheet", false);
    buttons::switch(ui, "Loop the fight", true);
    buttons::dropdown(ui, "Theme", &["Big top".into(), "Dark disco".into()], 0, 220.0);
    buttons::progress(ui, "Drawing frames", 0.64, 200.0);
    ui.horizontal(|ui| {
        icons::icon_button(ui, Icon::Play, "Play", true);
        icons::icon_button(ui, Icon::Pause, "Pause", true);
        icons::icon_button(ui, Icon::Play, "Play from the start", false);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(100.0, 26.0), egui::Sense::hover());
        for (i, icon) in [Icon::Done, Icon::Now, Icon::Todo, Icon::Chevron]
            .into_iter()
            .enumerate()
        {
            let left = rect.min.x + 4.0 + 24.0 * i as f32;
            let side = if icon == Icon::Chevron { 10.0 } else { 12.0 };
            let top = (rect.center().y - side / 2.0).round();
            let at = Rect::from_min_size(egui::pos2(left, top), egui::Vec2::splat(side));
            icons::paint(ui.painter(), at, icon, TEXT);
        }
    });
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((420.0, 330.0), gallery);
    harness.run();
    assert!(harness.unnamed_widgets().is_empty(), "{:?}", harness.unnamed_widgets());
    let progress = harness.get_by_label("Drawing frames");
    assert_eq!(progress.accesskit_node().role(), Role::ProgressIndicator);
    assert_eq!(harness.get_by_label("Play").accesskit_node().role(), Role::Button);
    assert!(
        harness
            .get_by_label("Play from the start")
            .accesskit_node()
            .is_disabled()
    );
}

#[test]
fn the_progress_bar_is_4_tall_with_a_text_fill_over_a_line_track() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((240.0, 40.0), |ui| {
        on_panel(ui);
        buttons::progress(ui, "Drawing frames", 0.64, 200.0);
    });
    harness.run();
    let rect = harness.get_by_label("Drawing frames").rect();
    assert_eq!((rect.width(), rect.height()), (200.0, 4.0));
    assert_eq!(at(&mut harness, rect, 10.0, 2.0), rgb(TEXT), "the filled share");
    assert_eq!(at(&mut harness, rect, 126.0, 2.0), rgb(TEXT), "just inside 64 %");
    assert_eq!(at(&mut harness, rect, 130.0, 2.0), rgb(LINE), "just past 64 %");
    assert_eq!(at(&mut harness, rect, 190.0, 2.0), rgb(LINE), "the track");
}

#[test]
fn icons_are_painted_where_the_reference_puts_them() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((120.0, 60.0), |ui| {
        on_panel(ui);
        let at = |x: f32| Rect::from_min_size(egui::pos2(x, 20.0), egui::Vec2::splat(12.0));
        icons::paint(ui.painter(), at(10.0), Icon::Play, TEXT);
        icons::paint(ui.painter(), at(30.0), Icon::Pause, TEXT);
        icons::paint(ui.painter(), at(50.0), Icon::Todo, TEXT);
        icons::paint(ui.painter(), at(70.0), Icon::Now, TEXT);
    });
    harness.run();
    let px = common::pixels(
        &mut harness,
        &[
            (14, 26), // inside the play triangle
            (21, 21), // left of the triangle's tip row, outside it
            (33, 26), // the first pause bar (x 2 to 5)
            (36, 26), // the gap between the bars (x 5 to 7)
            (38, 26), // the second pause bar (x 7 to 10)
            (56, 26), // the dash, at y 6
            (56, 22), // above the dash
            (76, 26), // the middle of the ring is empty
        ],
    );
    let (text, panel) = (rgb(TEXT), rgb(PANEL));
    assert_eq!(px[0], text, "play");
    assert_eq!(px[1], panel, "outside play");
    assert_eq!(px[2], text, "pause, first bar");
    assert_eq!(px[3], panel, "pause, the gap");
    assert_eq!(px[4], text, "pause, second bar");
    assert_ne!(px[5], panel, "to do");
    assert_eq!(px[6], panel, "above to do");
    assert_eq!(px[7], panel, "the ring is hollow");
}

#[test]
fn focusable_controls_paint_their_focus_ring() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((420.0, 330.0), gallery);
    harness.run();
    for name in [
        "Swap sides",
        "Search seeds",
        "Trombone",
        "Loop the fight",
        "Theme",
        "Pause",
    ] {
        let rect = harness.get_by_label(name).rect();
        let outside = |h: &mut Harness<'_>| at(h, rect, -3.0, rect.height() / 2.0);
        assert_ne!(outside(&mut harness), rgb(TEXT_STRONG), "{name}: no ring before focus");
        harness.get_by_label(name).focus();
        harness.run();
        assert!(harness.get_by_label(name).is_focused(), "{name} takes the focus");
        assert_eq!(
            outside(&mut harness),
            rgb(TEXT_STRONG),
            "{name}: 2 px TEXT_STRONG ring, 2 px outside"
        );
        assert_ne!(
            at(&mut harness, rect, -1.0, rect.height() / 2.0),
            rgb(TEXT_STRONG),
            "{name}: the ring leaves a 2 px gap"
        );
    }
}

#[test]
fn the_gallery_matches() {
    let _gpu = common::gpu();
    let mut harness = common::widget_harness((420.0, 330.0), gallery);
    harness.run();
    harness.snapshot("widgets_buttons");
}
