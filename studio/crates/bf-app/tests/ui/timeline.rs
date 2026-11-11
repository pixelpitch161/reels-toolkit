//! The stage column (the preview and its caption) and the story timeline (the transport row, the lanes, the
//! playhead, scrubbing and the keys).

use std::sync::Arc;

use bf_app::app::BfApp;
use bf_app::fixtures::base;
use bf_app::model::take::{Frame, SoundState, TransportAction};
use bf_app::model::{Step, View};
use bf_app::screens::{stage, timeline};
use bf_app::seam::sample;
use bf_app::state::{Action, ViewModel};
use bf_app::theme::{ON_LIGHT, TEXT_STRONG};
use bf_app::widgets::{fields, ribbon};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, SnapshotResults};

use crate::common::{MIN, REF, app_harness, gpu, pixels, widget_harness};

const EMPTY: &str = "The fight's story appears here once you pick a seed.";
const TITLE_CARD: &str = "Title card, also saved as the cover image";
const ABILITIES: [&str; 5] = [
    "Flutter tongue",
    "Sad trombone",
    "Power slide",
    "Snake dance",
    "Pedal tone",
];

/// At the reference size: the lanes start after the 120 wide name column and fill the rest of the 1440 window.
const LANES_LEFT: f32 = 120.0;
const LANES_W: f32 = 1320.0;
/// The timeline is 764 to 876; under its 1 px top line the transport row is 30 tall, then the story's three lanes
/// (20, 41 and 20 tall) from 795.
const STORY_TOP: f32 = 795.0;
/// A y inside each of the three lanes.
const LANE_YS: [u32; 3] = [805, 835, 866];

/// The shell showing `vm` as it is.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    app_harness(size, move |cc| BfApp::showing(cc, vm))
}

/// A plain picture of one colour, 360 x 640.
fn plain(value: u8) -> Frame {
    Frame {
        w: 360,
        h: 640,
        rgb: Arc::from(vec![value; 360 * 640 * 3]),
    }
}

/// The transport actions logged so far.
fn transport(harness: &Harness<'_, BfApp>) -> Vec<TransportAction> {
    harness
        .state()
        .log
        .iter()
        .filter_map(|a| match a {
            Action::Transport(t) => Some(*t),
            _ => None,
        })
        .collect()
}

fn press(harness: &Harness<'_, BfApp>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
}

fn release(harness: &Harness<'_, BfApp>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
}

fn is_disabled(harness: &Harness<'_, BfApp>, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_disabled()
}

/// The time a scrub action carries, if it is one.
fn scrub_time(action: TransportAction) -> Option<f32> {
    match action {
        TransportAction::ScrubTo(t) => Some(t),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Before a seed is picked
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn the_empty_state_says_what_to_do() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    assert!(harness.query_by_label(EMPTY).is_some());
    for label in ["Play", "Loop", "Safe zone"] {
        assert!(is_disabled(&harness, label), "{label} is disabled");
    }
    assert!(harness.query_by_label("0:00.00 / 1:01.00").is_some());
    assert!(
        harness.query_by_label("Story timeline").is_none(),
        "nothing to scrub yet"
    );
    harness.get_by_label("Play").click();
    harness.get_by_label("Loop").click();
    harness.run();
    assert!(harness.state().log.is_empty(), "disabled controls do nothing");
}

#[test]
fn the_title_card_caption() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    let caption = harness.get_by_label(TITLE_CARD).rect();
    let stage = harness.state().layout.stage;
    assert!(stage.contains_rect(caption), "{caption:?} inside {stage:?}");
}

#[test]
fn the_caption_follows_the_take() {
    let vm = base::vm();
    assert_eq!(stage::caption(&vm), TITLE_CARD);

    let mut loading = base::vm();
    loading.draft.seed = Some(5627);
    loading.take.loading = true;
    assert_eq!(stage::caption(&loading), "Drawing seed 5627");

    let mut failed = base::with_take();
    failed.take.story = None;
    failed.take.problem = Some("This seed could not be fought.".to_string());
    assert_eq!(stage::caption(&failed), "This seed could not be fought.");

    let mut take = base::with_take();
    assert_eq!(stage::caption(&take), "Seed 18433, the real picture and sound");
    take.view = View::Step(Step::Music);
    assert_eq!(stage::caption(&take), "Seed 18433, the real picture and sound");
    take.view = View::Step(Step::Render);
    assert_eq!(stage::caption(&take), "Flute vs Trombone, seed 18433");
    take.view = View::Library;
    assert_eq!(stage::caption(&take), "Flute vs Trombone, seed 18433");
}

#[test]
fn space_does_nothing_without_a_take() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::vm());
    harness.run();
    for key in [Key::Space, Key::ArrowLeft, Key::ArrowRight] {
        harness.key_press(key);
        harness.run();
    }
    assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
}

// ---------------------------------------------------------------------------------------------------------------
// With a take
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn the_time_reads_the_frame() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    assert!(harness.query_by_label("0:31.40 / 1:01.00").is_some());
    assert!(!is_disabled(&harness, "Play"));
    assert!(
        harness.query_by_label("Drag anywhere on the story to scrub").is_some(),
        "the Seeds hint"
    );
    let node = harness.get_by_label("Story timeline");
    assert_eq!(node.accesskit_node().role(), Role::Slider);
    let value = node.accesskit_node().numeric_value().expect("a value in seconds");
    assert!((value - 31.4).abs() < 1e-3, "{value}");
}

#[test]
fn the_sound_says_when_it_is_not_ready() {
    let _gpu = gpu();
    let building = "Building the sound";
    let silent = "No sound device was found, so the preview is silent.";
    let mut harness = showing(REF, base::with_take());
    harness.run();
    assert!(harness.query_by_label(building).is_none());
    assert!(harness.query_by_label(silent).is_none());
    drop(harness);
    for (sound, line) in [(SoundState::Building, building), (SoundState::NoDevice, silent)] {
        let mut vm = base::with_take();
        vm.transport.sound = sound;
        vm.status = String::new();
        let mut harness = showing(REF, vm);
        harness.run();
        assert!(harness.query_by_label(line).is_some(), "{sound:?}");
    }
}

#[test]
fn the_lanes_are_named() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    for name in ["Flute", "Who leads", "Trombone"] {
        let in_names = harness
            .query_all_by_label(name)
            .any(|n| n.rect().max.x <= LANES_LEFT && n.rect().min.y >= STORY_TOP);
        assert!(in_names, "{name} names a lane");
    }
    for name in ABILITIES {
        assert!(harness.query_by_label(name).is_some(), "{name}");
    }
    let snake = harness.get_by_label("Snake dance").rect();
    let want = ribbon::x_of(30.62, LANES_LEFT, LANES_W) + 7.0;
    assert!((snake.min.x - want).abs() <= 2.0, "{} against {want}", snake.min.x);
    assert!(snake.max.y <= STORY_TOP + 20.0, "Flute's ability is in the top lane");
    let pedal = harness.get_by_label("Pedal tone").rect();
    assert!(
        pedal.min.y >= STORY_TOP + 61.0,
        "Trombone's ability is in the bottom lane"
    );
    assert!(
        harness.query_by_label("K.O. window").is_some(),
        "the window is named on Seeds"
    );
}

#[test]
fn the_k_o_window_is_named_only_on_seeds() {
    let _gpu = gpu();
    let mut vm = base::with_take();
    vm.view = View::Step(Step::Music);
    vm.step = Step::Music;
    let mut harness = showing(REF, vm);
    harness.run();
    assert!(harness.query_by_label("K.O. window").is_none());
    assert!(harness.query_by_label("Drag anywhere on the story to scrub").is_none());
    assert!(harness.query_by_label("Snake dance").is_some());
}

#[test]
fn the_playhead_is_at_the_frame() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    let x = ribbon::x_of(31.4, LANES_LEFT, LANES_W) as u32;
    let at: Vec<(u32, u32)> = LANE_YS.iter().map(|&y| (x, y)).collect();
    let white = [TEXT_STRONG.r(), TEXT_STRONG.g(), TEXT_STRONG.b()];
    for (got, y) in pixels(&mut harness, &at).into_iter().zip(LANE_YS) {
        assert_eq!(got, white, "the playhead at ({x}, {y})");
    }
    // One pixel wide: the columns beside it are not white in the top lane.
    let beside = pixels(&mut harness, &[(x - 2, LANE_YS[0]), (x + 2, LANE_YS[0])]);
    assert!(beside.iter().all(|&p| p != white), "{beside:?}");
}

#[test]
fn clicking_the_story_scrubs() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    let at = pos2(780.0, 835.0);
    press(&harness, at);
    harness.run();
    release(&harness, at);
    harness.run();
    let log = transport(&harness);
    assert_eq!(log.len(), 2, "{log:?}");
    let t = scrub_time(log[0]).expect("a scrub");
    assert!((t - 30.5).abs() < 0.05, "{t}");
    assert_eq!(log[1], TransportAction::ScrubEnd);

    // A press and its release in the same frame is a scrub too.
    press(&harness, at);
    release(&harness, at);
    harness.run();
    let log = transport(&harness);
    assert_eq!(log.len(), 4, "{log:?}");
    assert!(scrub_time(log[2]).is_some_and(|t| (t - 30.5).abs() < 0.05));
    assert_eq!(log[3], TransportAction::ScrubEnd);
}

#[test]
fn dragging_scrubs_while_it_moves_and_ends_once_outside() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    press(&harness, pos2(780.0, 835.0));
    harness.run();
    harness.event(Event::PointerMoved(pos2(900.0, 835.0)));
    harness.run();
    // Held still: no new scrub.
    harness.run();
    // Out of the window's top half and released there: the scrub ends, kept inside the take.
    harness.event(Event::PointerMoved(pos2(-50.0, 400.0)));
    harness.run();
    release(&harness, pos2(-50.0, 400.0));
    harness.run();
    harness.run();
    let log = transport(&harness);
    let times: Vec<f32> = log.iter().filter_map(|&a| scrub_time(a)).collect();
    assert_eq!(times.len(), 3, "{log:?}");
    assert!((times[0] - 30.5).abs() < 0.05);
    let want = ribbon::t_of(900.0, LANES_LEFT, LANES_W);
    assert!((times[1] - want).abs() < 0.05, "{} against {want}", times[1]);
    assert_eq!(times[2], 0.0, "left of the lanes is the start");
    assert_eq!(log.last(), Some(&TransportAction::ScrubEnd));
    assert_eq!(log.iter().filter(|&&a| a == TransportAction::ScrubEnd).count(), 1);
}

#[test]
fn losing_the_window_ends_a_scrub() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    let at = pos2(780.0, 835.0);
    press(&harness, at);
    harness.run();
    harness.input_mut().focused = false;
    harness.event(Event::WindowFocused(false));
    harness.run();
    let ended = transport(&harness);
    assert_eq!(ended.last(), Some(&TransportAction::ScrubEnd), "{ended:?}");
    // Back in the window with the button still held: no scrub starts by itself, and the release ends nothing more.
    harness.input_mut().focused = true;
    harness.event(Event::WindowFocused(true));
    harness.run();
    harness.event(Event::PointerMoved(pos2(900.0, 835.0)));
    harness.run();
    release(&harness, pos2(900.0, 835.0));
    harness.run();
    assert_eq!(transport(&harness), ended);
}

#[test]
fn space_plays_and_pauses() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(transport(&harness), vec![TransportAction::PlayPause]);
    harness.get_by_label("Play").click();
    harness.run();
    assert_eq!(
        transport(&harness),
        vec![TransportAction::PlayPause, TransportAction::PlayPause]
    );
}

#[test]
fn the_button_says_pause_while_playing() {
    let _gpu = gpu();
    let mut vm = base::with_take();
    vm.transport.playing = true;
    let mut harness = showing(REF, vm);
    harness.run();
    assert!(harness.query_by_label("Pause").is_some());
    assert!(harness.query_by_label("Play").is_none());
}

#[test]
fn arrows_step_a_frame() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    // Ctrl and an arrow is not a step.
    harness.key_press_modifiers(Modifiers::CTRL, Key::ArrowRight);
    harness.run();
    assert_eq!(
        transport(&harness),
        vec![TransportAction::Step(-1), TransportAction::Step(1)]
    );
    // With the story timeline itself focused the arrows still step, and the keyboard stays on it.
    harness.get_by_label("Story timeline").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(transport(&harness).last(), Some(&TransportAction::Step(1)));
    assert!(harness.get_by_label("Story timeline").is_focused());
}

#[test]
fn the_keys_leave_a_text_field_alone() {
    let _gpu = gpu();
    let vm = base::with_take();
    let mut text = String::new();
    let mut out = Vec::new();
    let mut harness = widget_harness((1440.0, 200.0), |ui| {
        fields::text_line(ui, "Caption", &mut text, "Write a caption", 200.0);
        timeline::show(ui, &vm, &mut out);
    });
    harness.run();
    harness.get_by_label("Caption").click();
    harness.run();
    harness.key_press(Key::Space);
    harness.event(Event::Text(" ".to_string()));
    harness.key_press(Key::ArrowLeft);
    harness.key_press(Key::Num3);
    harness.run();
    drop(harness);
    assert!(out.is_empty(), "{out:?}");
    assert_eq!(text, " ");
}

#[test]
fn space_presses_a_focused_button_and_nothing_else() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    harness.get_by_label("Loop").focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert_eq!(
        transport(&harness),
        vec![TransportAction::SetLoop(false)],
        "Space pressed Loop, and did not also play"
    );
}

#[test]
fn loop_and_safe_zone_toggle() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    let node = harness.get_by_label("Loop");
    assert_eq!(node.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
    let rect = node.rect();
    // Loop is on: inverted, with its label in ON_LIGHT glyph pixels inside the fill.
    let at: Vec<(u32, u32)> = (rect.min.x as u32 + 8..rect.max.x as u32 - 8)
        .flat_map(|x| (rect.min.y as u32 + 6..rect.max.y as u32 - 6).map(move |y| (x, y)))
        .collect();
    let dark = [ON_LIGHT.r(), ON_LIGHT.g(), ON_LIGHT.b()];
    assert!(pixels(&mut harness, &at).contains(&dark), "ON_LIGHT glyphs on the fill");
    harness.get_by_label("Loop").click();
    harness.run();
    harness.get_by_label("Safe zone").click();
    harness.run();
    assert_eq!(
        transport(&harness),
        vec![TransportAction::SetLoop(false), TransportAction::SetSafeZone(true)]
    );
}

#[test]
fn the_story_timeline_shows_its_focus() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    // The ring and its gap take the 4 px just inside the lanes, so no clip cuts them; this pixel is left of FIGHT!,
    // in the upper half of the ribbon lane: plain panel until the ring is drawn there.
    let ring = (LANES_LEFT as u32 + 1, 825);
    let white = [TEXT_STRONG.r(), TEXT_STRONG.g(), TEXT_STRONG.b()];
    assert_ne!(pixels(&mut harness, &[ring])[0], white);
    harness.get_by_label("Story timeline").focus();
    harness.run();
    assert_eq!(pixels(&mut harness, &[ring])[0], white, "the ring is inside the lanes");
}

// ---------------------------------------------------------------------------------------------------------------
// The preview
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn the_preview_is_360x640_at_the_reference_size() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.state_mut().show_frame(plain(255));
    harness.run();
    let preview = harness.get_by_label("Preview").rect();
    assert_eq!((preview.width(), preview.height()), (360.0, 640.0));
    let stage = harness.state().layout.stage;
    assert!(stage.contains_rect(preview));
    assert_eq!(preview.center().x, stage.center().x, "centred");
    let caption = harness.get_by_label("Seed 18433, the real picture and sound").rect();
    assert!(
        caption.min.y >= preview.max.y + 8.0 - 0.5,
        "8 below the picture: {caption:?}"
    );
    assert!(stage.contains_rect(caption));
}

#[test]
fn it_fits_at_the_minimum_size() {
    let _gpu = gpu();
    let mut harness = showing(MIN, base::with_take());
    harness.state_mut().show_frame(plain(255));
    harness.run();
    let preview = harness.get_by_label("Preview").rect();
    assert_eq!((preview.width(), preview.height()), (310.0, 552.0));
    let layout = harness.state().layout;
    assert!(layout.stage.contains_rect(preview), "{preview:?} in {:?}", layout.stage);
    let caption = harness.get_by_label("Seed 18433, the real picture and sound").rect();
    assert!(layout.stage.contains_rect(caption), "{caption:?}");
    // The whole transport row can still be reached.
    for label in ["Play", "Loop", "Safe zone", "Story timeline"] {
        let r = harness.get_by_label(label).rect();
        assert!(layout.timeline.contains_rect(r), "{label} at {r:?}");
    }
}

#[test]
fn no_picture_draws_the_empty_stage() {
    let _gpu = gpu();
    let mut harness = showing(REF, base::with_take());
    harness.run();
    assert!(harness.query_by_label("Preview").is_none());
    let stage = harness.state().layout.stage;
    let middle = (stage.center().x as u32, stage.center().y as u32);
    assert_eq!(pixels(&mut harness, &[middle])[0], [0x2b, 0x2b, 0x2b], "STAGE");
}

#[test]
fn the_safe_zone_shades_the_outside() {
    let _gpu = gpu();
    let mut vm = base::with_take();
    vm.transport.safe_zone = true;
    let mut harness = showing(REF, vm);
    harness.state_mut().show_frame(plain(255));
    harness.run();
    let preview = harness.get_by_label("Preview").rect();
    let internal = |x: f32, y: f32| ((preview.min.x + x) as u32, (preview.min.y + y) as u32);
    let got = pixels(
        &mut harness,
        &[internal(10.0, 10.0), internal(180.0, 300.0), internal(320.0, 400.0)],
    );
    assert!(got[0].iter().all(|&c| c < 200), "outside is shaded: {:?}", got[0]);
    assert_eq!(got[1], [255; 3], "inside is not");
    assert!(
        got[2].iter().all(|&c| c < 200),
        "the buttons' corner is shaded: {:?}",
        got[2]
    );
    drop(harness);

    let mut harness = showing(REF, base::with_take());
    harness.state_mut().show_frame(plain(255));
    harness.run();
    let preview = harness.get_by_label("Preview").rect();
    let corner = ((preview.min.x + 10.0) as u32, (preview.min.y + 10.0) as u32);
    assert_eq!(
        pixels(&mut harness, &[corner])[0],
        [255; 3],
        "the toggle off shades nothing"
    );
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    for vm in [base::vm(), base::with_take()] {
        let mut harness = showing(REF, vm);
        harness.state_mut().show_frame(plain(255));
        harness.run();
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn the_timeline_matches() {
    let _gpu = gpu();
    // The workspace is masked: its screens are drawn and pictured by their own tasks. The two harnesses' results
    // are gathered, so both pictures are compared (and blessed) before the test fails.
    let mut results = SnapshotResults::new();
    let mut harness = showing(REF, base::vm());
    let vm = &harness.state().state.vm;
    let masks = [0, 1].map(|side| vm.fighter(side).expect("a fighter").mask.clone());
    let card = sample::title_card(vm.colors(), [&masks[0], &masks[1]]);
    harness.state_mut().show_frame(card);
    harness.run();
    let workspace: Rect = harness.state().layout.workspace;
    harness.mask(workspace);
    harness.snapshot("timeline_empty");
    results.extend_harness(&mut harness);
    drop(harness);

    let mut harness = showing(REF, base::with_take());
    let frame = sample::frames(harness.state().state.vm.colors()).draw(942);
    harness.state_mut().show_frame(frame);
    harness.run();
    let workspace: Rect = harness.state().layout.workspace;
    harness.mask(workspace);
    harness.snapshot("timeline_story");
    results.extend_harness(&mut harness);
    drop(harness);
    results.unwrap();
}
