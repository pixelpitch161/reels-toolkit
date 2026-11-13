//! The Render screen (interface reference, section 5.5): this fight's captions and folder, "Add to queue", the
//! queue and the check list of the video shown.

use std::path::PathBuf;

use bf_app::app::BfApp;
use bf_app::fixtures::render::{self, ALTO, CELLO, FAILED_DETAIL, FAILED_HEADLINE, OBOE, TUBA};
use bf_app::model::library::FolderState;
use bf_app::model::queue::{FailHint, JobState, STEP_ORDER, StepKey};
use bf_app::model::{JobId, Platform, Step};
use bf_app::screens::render::{CHECKS_W, step_note, step_title};
use bf_app::seam::Command;
use bf_app::state::queue_run::why_not_add;
use bf_app::state::{Action, QueueAction, TakeView, ViewModel};
use bf_app::theme::LINE_SOFT;
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{MIN, REF, app_harness, gpu, pixels};

const TUBA_ROW: &str = "Tuba vs Bass drum, seed 9120";
const ALTO_ROW: &str = "Alto sax vs Tenor sax, seed 15007";
const OBOE_ROW: &str = "Oboe vs Bassoon, seed 662";
const CELLO_ROW: &str = "Cello vs Contrabass, seed 4471";
const CHECKS_NOTE: &str =
    "Every video goes through these in order. A failed check stops that video and the queue moves on.";
const NO_CHECKS: &str = "Nothing is rendering. A video's checks show here while it renders.";
const INSTAGRAM_BOX: &str = "Text and tags for Instagram";
const FILES: &str = "flute_vs_trombone.mp4, cover.png, caption.txt and the fight file";

/// The shell showing `vm` as it is, settled.
fn showing<'a>(size: (f32, f32), vm: ViewModel) -> Harness<'a, BfApp> {
    let mut harness = app_harness(size, move |cc| BfApp::showing(cc, vm));
    harness.run();
    harness
}

/// Whether a node with exactly `label` is on screen (there may be several).
fn has(harness: &Harness<'_, BfApp>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The control named `label` that is not a plain label (a label and a control may share a name).
fn control<'t>(harness: &'t Harness<'_, BfApp>, label: &'t str) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(label)
        .find(|n| n.accesskit_node().role() != Role::Label)
        .unwrap_or_else(|| panic!("a control named {label}"))
}

/// The node named `label` lying inside the row named `row`.
fn in_row<'t>(harness: &'t Harness<'_, BfApp>, row: &str, label: &'t str) -> Option<egui_kittest::Node<'t>> {
    let rect = control(harness, row).rect();
    harness.query_all_by_label(label).find(|n| rect.contains_rect(n.rect()))
}

/// The text a node shows: a label keeps it in its value, any other node in its name.
fn text_of(node: &egui_kittest::Node<'_>) -> String {
    let node = node.accesskit_node();
    let text = if node.role() == Role::Label {
        node.value()
    } else {
        node.label()
    };
    text.unwrap_or_default()
}

/// Every text on screen: labels' values, other nodes' names and values.
fn all_text(harness: &Harness<'_, BfApp>) -> Vec<String> {
    harness
        .query_all_by(|_| true)
        .flat_map(|n| {
            let node = n.accesskit_node();
            [node.label(), node.value()]
        })
        .flatten()
        .collect()
}

/// The caption box named `name` ("Text and tags for Instagram").
fn caption_box<'t>(harness: &'t Harness<'_, BfApp>, name: &'t str) -> egui_kittest::Node<'t> {
    harness
        .query_all_by_label(name)
        .find(|n| matches!(n.accesskit_node().role(), Role::MultilineTextInput | Role::TextInput))
        .unwrap_or_else(|| panic!("a text box named {name}"))
}

/// Where the words "Text and tags for Instagram" over the caption box are.
fn box_label(harness: &Harness<'_, BfApp>) -> Rect {
    harness
        .query_all_by_label(INSTAGRAM_BOX)
        .find(|n| n.accesskit_node().role() == Role::Label)
        .expect("the box's label")
        .rect()
}

fn queue_actions(harness: &Harness<'_, BfApp>) -> Vec<QueueAction> {
    harness
        .state()
        .log
        .iter()
        .filter_map(|a| match a {
            Action::Queue(q) => Some(q.clone()),
            _ => None,
        })
        .collect()
}

fn press(harness: &Harness<'_, BfApp>, at: Pos2, pressed: bool) {
    harness.event(Event::PointerMoved(at));
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// The fixture with two more waiting jobs after Alto sax vs Tenor sax; returns it and the two new ids.
fn three_waiting() -> (ViewModel, JobId, JobId) {
    let mut vm = render::vm();
    let piano = vm.queue.add(render::spec(
        "Piano vs Organ",
        [[0x54, 0x9c, 0xff], [0xc4, 0x34, 0x68]],
        "Big top",
        77,
    ));
    let harp = vm.queue.add(render::spec(
        "Harp vs Violin",
        [[0xf4, 0xe4, 0xbe], [0xb0, 0x62, 0xff]],
        "Big top",
        88,
    ));
    (vm, piano, harp)
}

#[test]
fn every_word_of_the_mock_up() {
    let _gpu = gpu();
    let harness = showing(REF, render::vm());
    for text in [
        "This fight",
        "Seed 18433, music take 0417",
        "Caption",
        "Instagram",
        "TikTok",
        "Shorts",
        "Drafted for you. Edit freely.",
        INSTAGRAM_BOX,
        "Saves to",
        "Change folder",
        FILES,
        "Add to queue",
        "Queue",
        "1 rendering, 1 waiting",
        "Pause queue",
        "Tuba vs Bass drum",
        "Dark drumline, seed 9120",
        "Drawing frames 64%",
        "Cancel",
        "Alto sax vs Tenor sax",
        "Dark disco, seed 15007",
        "Waiting",
        "Remove",
        "Oboe vs Bassoon",
        "Heist, seed 662",
        FAILED_HEADLINE,
        FAILED_DETAIL,
        "Queue again",
        "Open in Music",
        "Cello vs Contrabass",
        "Dark tango, seed 4471",
        "Done in 48 s. All checks passed.",
        "Open folder",
        CHECKS_NOTE,
        "Soundtrack built",
        "Safe zone",
        "Captions and HUD clear of the apps\u{2019} buttons",
        "Focus",
        "Both fighters sharp on every frame",
        "Loudness",
        "Opening within 0.6 dB of the ending",
        "Fighters above the music",
        "Flashes",
        "Busiest stretch peaks at 17% of the frame",
        "Loop point",
        "The last frame flows into the first",
        "Drawing frames",
        "1,171 of 1,830",
        "Encoding",
        "Checking the finished video",
        "Length, loop and flashes again, on the mp4",
    ] {
        assert!(has(&harness, text), "{text}");
    }
    let folder = control(&harness, "Saves to");
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(
        folder.value(),
        Some(format!("Videos{sep}BALLFIGHT{sep}flute_vs_trombone{sep}"))
    );
    let caption = caption_box(&harness, INSTAGRAM_BOX).value().unwrap_or_default();
    assert!(caption.starts_with("The front row vs the back row "), "{caption:?}");
    assert!(caption.ends_with("#satisfying #music"), "{caption:?}");
    assert!(!control(&harness, "Add to queue").accesskit_node().is_disabled());
    assert!(
        harness.query_all_by_label("Tuba vs Bass drum").count() >= 2,
        "its row and the check list's heading"
    );
    assert!(!has(&harness, NO_CHECKS));
    assert!(
        !has(&harness, "Drag a waiting video to change the order"),
        "one waiting"
    );
}

#[test]
fn the_step_table() {
    let titles: Vec<&str> = STEP_ORDER.iter().map(|&k| step_title(k)).collect();
    assert_eq!(
        titles,
        [
            "Soundtrack built",
            "Safe zone",
            "Focus",
            "Loudness",
            "Fighters above the music",
            "Flashes",
            "Loop point",
            "Drawing frames",
            "Encoding",
            "Checking the finished video",
        ]
    );
    assert_eq!(step_note(StepKey::Soundtrack), "");
    assert_eq!(step_note(StepKey::Band), "");
    assert_eq!(step_note(StepKey::Encode), "");
    assert_eq!(step_note(StepKey::Frames), "", "the frame count is the job's own");
    assert_eq!(step_note(StepKey::Verify), "Length, loop and flashes again, on the mp4");
}

#[test]
fn the_check_list_is_330_wide_with_ten_lines() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, render::vm());
        let workspace = harness.state().layout.workspace;
        let left = workspace.max.x - CHECKS_W;
        assert_eq!(CHECKS_W, 330.0);
        let note = harness.get_by_label(CHECKS_NOTE).rect();
        assert_eq!(note.min.x, left + 14.0, "padded 14 at the side");
        let titles: Vec<Rect> = STEP_ORDER
            .iter()
            .map(|&k| {
                harness
                    .query_all_by_label(step_title(k))
                    .map(|n| n.rect())
                    .find(|r| r.min.x >= left && r.max.x <= workspace.max.x)
                    .unwrap_or_else(|| panic!("{} in the check list", step_title(k)))
            })
            .collect();
        for pair in titles.windows(2) {
            assert!(pair[1].min.y - pair[0].min.y >= 26.0, "at least 26 apart: {pair:?}");
        }
        assert!(titles[0].min.y > note.max.y, "under the note");
        assert!(titles[9].max.y <= workspace.max.y, "all ten fit at {size:?}");
        let y = (workspace.min.y + 200.0) as u32;
        let rgb = |c: egui::Color32| [c.r(), c.g(), c.b()];
        assert_eq!(
            pixels(&mut harness, &[(left as u32, y)]),
            vec![rgb(LINE_SOFT)],
            "a 1 px line on its left at {size:?}"
        );
    }
}

#[test]
fn the_check_list_follows_the_selected_job() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    control(&harness, CELLO_ROW).click();
    harness.run();
    assert_eq!(queue_actions(&harness), vec![QueueAction::Select(CELLO)]);
    assert_eq!(harness.query_all_by_label("Cello vs Contrabass").count(), 2);
    assert!(
        !has(&harness, "1,171 of 1,830"),
        "the rendering job's count is not shown"
    );
    drop(harness);

    // Nothing rendering and nothing selected.
    let mut vm = render::vm();
    vm.queue.jobs.retain(|j| j.id != TUBA);
    let harness = showing(REF, vm);
    assert!(has(&harness, NO_CHECKS));
    assert!(!has(&harness, CHECKS_NOTE));
}

#[test]
fn the_caption_tabs_switch_the_text() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    let tiktok = harness
        .query_all_by_label("TikTok")
        .find(|n| n.accesskit_node().role() == Role::RadioButton)
        .expect("the TikTok segment");
    tiktok.click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SetCaptionTab(Platform::TikTok)]);
    assert!(has(&harness, "Text and tags for TikTok"));
    assert!(!has(&harness, INSTAGRAM_BOX));
    let text = caption_box(&harness, "Text and tags for TikTok")
        .value()
        .unwrap_or_default();
    assert!(text.ends_with("#bandtok #music"), "{text:?}");
    let chosen = harness
        .query_all_by_label("TikTok")
        .find(|n| n.accesskit_node().role() == Role::RadioButton)
        .unwrap();
    assert_eq!(chosen.accesskit_node().toggled(), Some(egui::accesskit::Toggled::True));
}

#[test]
fn typing_edits_the_caption() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    caption_box(&harness, INSTAGRAM_BOX).focus();
    harness.run();
    caption_box(&harness, INSTAGRAM_BOX).type_text("!");
    harness.run();
    let last = harness.state().log.last().cloned();
    match last {
        Some(Action::SetCaption { platform, text }) => {
            assert_eq!(platform, Platform::Instagram);
            assert!(text.ends_with("#music!"), "{text:?}");
        }
        other => panic!("SetCaption was pushed: {other:?}"),
    }
    assert!(
        caption_box(&harness, INSTAGRAM_BOX)
            .value()
            .is_some_and(|v| v.ends_with('!'))
    );
}

#[test]
fn a_character_no_font_has_keeps_its_place_in_the_text() {
    let _gpu = gpu();
    // The sample caption carries emoji (U+1FA88, U+1F3BA, U+1F447); no font built into the app has them, and the
    // test harness has no system fonts, so drawing them would panic. The box keeps them in its text.
    let mut harness = showing(REF, render::vm());
    let text = caption_box(&harness, INSTAGRAM_BOX).value().unwrap_or_default();
    assert!(text.contains('\u{1fa88}') && text.contains('\u{1f447}'), "{text:?}");
    caption_box(&harness, INSTAGRAM_BOX).focus();
    harness.run();
    caption_box(&harness, INSTAGRAM_BOX).type_text("!");
    harness.run();
    match harness.state().log.last() {
        Some(Action::SetCaption { text: edited, .. }) => {
            assert_eq!(
                edited,
                &format!("{text}!"),
                "every character kept, the new one at the end"
            )
        }
        other => panic!("SetCaption was pushed: {other:?}"),
    }
}

#[test]
fn a_long_caption_scrolls_inside_its_box() {
    let _gpu = gpu();
    let mut vm = render::vm();
    vm.draft.captions.instagram = (1..=30).map(|i| format!("Line {i}")).collect::<Vec<_>>().join("\n");
    let harness = showing(MIN, vm);
    let label = box_label(&harness);
    let saves = harness.get_by_label("Change folder").rect();
    assert!(
        saves.min.y - label.max.y < 4.0 + 66.0 + 6.0 + 1.0,
        "the box stays three lines tall: {label:?} {saves:?}"
    );
}

#[test]
fn keys_do_not_fire_while_typing() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    caption_box(&harness, INSTAGRAM_BOX).focus();
    harness.run();
    harness.key_press(Key::Num3);
    harness.event(Event::Text("3".to_string()));
    harness.run();
    harness.key_press(Key::Space);
    harness.event(Event::Text(" ".to_string()));
    harness.run();
    let log = &harness.state().log;
    assert!(
        log.iter().all(|a| !matches!(a, Action::Go(_) | Action::Transport(_))),
        "{log:?}"
    );
    assert!(log.iter().any(|a| matches!(a, Action::SetCaption { .. })), "{log:?}");
}

#[test]
fn add_to_queue_pushes() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    harness.state_mut().sent();
    control(&harness, "Add to queue").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::AddToQueue]);
    let sent = harness.state_mut().sent();
    assert!(sent.iter().any(|c| matches!(c, Command::Enqueue { .. })), "{sent:?}");
}

#[test]
fn add_to_queue_waits_with_the_reason() {
    let _gpu = gpu();
    let mut no_seed = render::vm();
    no_seed.draft.seed = None;
    no_seed.take = TakeView::default();
    let mut checking = render::vm();
    checking.folder = FolderState::Unknown;
    let mut no_ffmpeg = render::vm();
    no_ffmpeg.ffmpeg = Some(Err("not found".to_string()));
    for (vm, starts) in [
        (no_seed, "Pick a seed first."),
        (checking, "Checking the output folder."),
        (no_ffmpeg, "ffmpeg was not found on this PC"),
    ] {
        let reason = why_not_add(&vm).expect("a reason");
        assert!(reason.starts_with(starts), "{reason}");
        for size in [REF, MIN] {
            let mut harness = showing(size, vm.clone());
            let label = harness
                .query_all_by_label(&reason)
                .find(|n| n.accesskit_node().role() == Role::Label)
                .unwrap_or_else(|| panic!("{reason} is on screen"));
            let words = label.rect();
            let button = control(&harness, "Add to queue");
            assert!(button.accesskit_node().is_disabled(), "{reason}");
            let button_rect = button.rect();
            assert!(words.max.x <= button_rect.min.x, "beside it: {words:?} {button_rect:?}");
            assert!(
                harness.state().layout.workspace.contains_rect(words),
                "inside the workspace at {size:?}"
            );
            control(&harness, "Add to queue").click();
            harness.run();
            assert!(harness.state().log.is_empty(), "{:?}", harness.state().log);
        }
    }
}

#[test]
fn a_missing_folder_offers_to_create_it() {
    let _gpu = gpu();
    let mut vm = render::vm();
    vm.folder = FolderState::Missing;
    let reason = why_not_add(&vm).expect("a reason");
    assert!(reason.contains("is missing"), "{reason}");
    let mut harness = showing(REF, vm);
    assert!(has(&harness, &reason), "the folder is named and what to do is said");
    assert!(control(&harness, "Add to queue").accesskit_node().is_disabled());
    control(&harness, "Create folder").click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::CreateFolder]);
    assert!(
        harness
            .state_mut()
            .sent()
            .iter()
            .any(|c| matches!(c, Command::CreateFolder { .. }))
    );
    // "Change folder" of this fight (a failed job's hint has the same words), not a row's.
    let change = harness
        .query_all_by_label("Change folder")
        .min_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap();
    change.click();
    harness.run();
    assert_eq!(harness.state().log.last(), Some(&Action::ChangeFolder));
    drop(harness);

    let mut vm = render::vm();
    vm.folder = FolderState::NotWritable("Access is denied.".to_string());
    let reason = why_not_add(&vm).expect("a reason");
    let harness = showing(REF, vm);
    assert!(has(&harness, &reason));
    assert!(!has(&harness, "Create folder"), "only a missing folder can be created");
}

#[test]
fn a_taken_folder_name_is_explained() {
    let _gpu = gpu();
    let harness = showing(REF, render::vm());
    assert!(!has(
        &harness,
        "That folder exists, so this one will be named flute_vs_trombone_seed18433."
    ));
    drop(harness);
    let mut vm = render::vm();
    vm.folder = FolderState::Ok { taken: true };
    let harness = showing(REF, vm);
    assert!(has(
        &harness,
        "That folder exists, so this one will be named flute_vs_trombone_seed18433."
    ));
    assert!(!control(&harness, "Add to queue").accesskit_node().is_disabled());
}

#[test]
fn each_job_state_has_its_words_and_buttons() {
    let _gpu = gpu();
    let mut vm = render::vm();
    let cancelled = vm.queue.add(render::spec(
        "Piano vs Organ",
        [[0x54, 0x9c, 0xff], [0xc4, 0x34, 0x68]],
        "Big top",
        77,
    ));
    for job in &mut vm.queue.jobs {
        if job.id == cancelled {
            job.state = JobState::Cancelled;
        }
        if job.id == ALTO {
            job.interrupted = true;
        }
    }
    let harness = showing(REF, vm);
    let piano = "Piano vs Organ, seed 77";
    // Under the title, and the buttons each state offers (and only those).
    let rows: [(&str, &str, &[&str]); 5] = [
        (TUBA_ROW, "Drawing frames 64%", &["Cancel"]),
        (
            ALTO_ROW,
            "Waiting. It was interrupted when the app closed.",
            &["Remove"],
        ),
        (piano, "Cancelled. Nothing was saved.", &["Queue again", "Remove"]),
        (OBOE_ROW, FAILED_HEADLINE, &["Queue again", "Open in Music", "Remove"]),
        (CELLO_ROW, "Done in 48 s. All checks passed.", &["Open folder"]),
    ];
    let all = ["Cancel", "Remove", "Queue again", "Open in Music", "Open folder"];
    for (row, words, buttons) in rows {
        assert!(in_row(&harness, row, words).is_some(), "{row}: {words}");
        for button in all {
            assert_eq!(
                in_row(&harness, row, button).is_some(),
                buttons.contains(&button),
                "{row}: {button}"
            );
        }
    }
    // The order: rendering, waiting, finished newest first.
    let tops: Vec<f32> = [TUBA_ROW, ALTO_ROW, piano, OBOE_ROW, CELLO_ROW]
        .iter()
        .map(|r| control(&harness, r).rect().min.y)
        .collect();
    assert!(tops.windows(2).all(|p| p[0] < p[1]), "{tops:?}");
    assert!(
        harness.state().layout.workspace.max.x - CHECKS_W >= control(&harness, TUBA_ROW).rect().max.x,
        "the rows stop at the check list"
    );
}

#[test]
fn the_failed_job_names_the_check_and_the_numbers() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    assert!(in_row(&harness, OBOE_ROW, FAILED_HEADLINE).is_some());
    assert!(in_row(&harness, OBOE_ROW, FAILED_DETAIL).is_some());
    in_row(&harness, OBOE_ROW, "Open in Music").unwrap().click();
    harness.run();
    in_row(&harness, OBOE_ROW, "Queue again").unwrap().click();
    harness.run();
    assert_eq!(
        queue_actions(&harness),
        vec![
            QueueAction::OpenIn {
                id: OBOE,
                step: Step::Music
            },
            QueueAction::Requeue(OBOE)
        ]
    );
    drop(harness);

    // Each hint has its own button, and some have none.
    for (hint, button) in [
        (FailHint::OpenInSeeds, Some("Open in Seeds")),
        (FailHint::CheckFolder, Some("Change folder")),
        (FailHint::InstallFfmpeg, None),
        (FailHint::None, None),
    ] {
        let mut vm = render::vm();
        for job in &mut vm.queue.jobs {
            if let JobState::Failed { hint: h, .. } = &mut job.state {
                *h = hint;
            }
        }
        let mut harness = showing(REF, vm);
        for other in ["Open in Seeds", "Open in Music", "Change folder"] {
            assert_eq!(
                in_row(&harness, OBOE_ROW, other).is_some(),
                button == Some(other),
                "{hint:?}: {other}"
            );
        }
        if let Some(button) = button {
            in_row(&harness, OBOE_ROW, button).unwrap().click();
            harness.run();
            let expected = match hint {
                FailHint::OpenInSeeds => Action::Queue(QueueAction::OpenIn {
                    id: OBOE,
                    step: Step::Seeds,
                }),
                _ => Action::ChangeFolder,
            };
            assert_eq!(harness.state().log, vec![expected]);
        }
    }
}

#[test]
fn the_row_buttons_push_their_actions() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    in_row(&harness, TUBA_ROW, "Cancel").unwrap().click();
    harness.run();
    in_row(&harness, CELLO_ROW, "Open folder").unwrap().click();
    harness.run();
    in_row(&harness, ALTO_ROW, "Remove").unwrap().click();
    harness.run();
    assert_eq!(
        queue_actions(&harness),
        vec![
            QueueAction::Cancel(TUBA),
            QueueAction::OpenFolder(CELLO),
            QueueAction::Remove(ALTO)
        ]
    );
    assert!(!has(&harness, ALTO_ROW), "removed");
}

#[test]
fn pause_and_resume() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    control(&harness, "Pause queue").click();
    harness.run();
    assert_eq!(queue_actions(&harness), vec![QueueAction::Pause]);
    assert!(has(&harness, "Paused. 1 waiting"));
    assert!(!has(&harness, "Pause queue"));
    control(&harness, "Resume queue").click();
    harness.run();
    assert_eq!(queue_actions(&harness), vec![QueueAction::Pause, QueueAction::Resume]);
    assert!(has(&harness, "1 rendering, 1 waiting"));
    drop(harness);

    let mut vm = render::vm();
    vm.queue.jobs.retain(|j| matches!(j.state, JobState::Done { .. }));
    let harness = showing(REF, vm);
    assert!(has(&harness, "Nothing is queued. Add this fight to render it."));
    drop(harness);

    let (vm, _, _) = three_waiting();
    let harness = showing(REF, vm);
    assert!(has(&harness, "1 rendering, 3 waiting"));
    assert!(has(&harness, "Drag a waiting video to change the order"));
}

/// Right-clicks the row named `row` and clicks `item` in its menu.
fn from_the_menu(harness: &mut Harness<'_, BfApp>, row: &str, item: &str) {
    control(harness, row).click_secondary();
    harness.run();
    control(harness, item).click();
    harness.run();
}

#[test]
fn the_menu_moves_a_waiting_job() {
    let _gpu = gpu();
    let (vm, piano, harp) = three_waiting();
    let mut harness = showing(REF, vm);
    // The first waiting job is already first in line: it cannot move up or be rendered sooner.
    control(&harness, ALTO_ROW).click_secondary();
    harness.run();
    for item in ["Move up", "Render next"] {
        assert!(control(&harness, item).accesskit_node().is_disabled(), "{item}");
    }
    assert!(!control(&harness, "Move down").accesskit_node().is_disabled());
    control(&harness, "Move up").click();
    harness.run();
    assert!(queue_actions(&harness).is_empty());
    harness.key_press(Key::Escape);
    harness.run();

    from_the_menu(&mut harness, "Piano vs Organ, seed 77", "Move down");
    assert_eq!(queue_actions(&harness), vec![QueueAction::Move { id: piano, to: 2 }]);
    from_the_menu(&mut harness, "Piano vs Organ, seed 77", "Move up");
    from_the_menu(&mut harness, "Harp vs Violin, seed 88", "Render next");
    assert_eq!(
        queue_actions(&harness)[1..],
        [
            QueueAction::Move { id: piano, to: 1 },
            QueueAction::Move { id: harp, to: 0 }
        ]
    );
    // The last in line cannot move down.
    let last = harness.state().state.vm.queue.shown()[3];
    let name = match last {
        id if id == piano => "Piano vs Organ, seed 77",
        id if id == ALTO => ALTO_ROW,
        _ => "Harp vs Violin, seed 88",
    };
    control(&harness, name).click_secondary();
    harness.run();
    assert!(control(&harness, "Move down").accesskit_node().is_disabled());
}

#[test]
fn only_waiting_jobs_have_the_menu() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    for row in [TUBA_ROW, OBOE_ROW, CELLO_ROW] {
        control(&harness, row).click_secondary();
        harness.run();
        assert!(!has(&harness, "Render next"), "{row}");
    }
}

#[test]
fn dragging_reorders() {
    let _gpu = gpu();
    let (vm, _, harp) = three_waiting();
    let mut harness = showing(REF, vm);
    let from = control(&harness, "Harp vs Violin, seed 88").rect().center();
    let to = control(&harness, ALTO_ROW).rect().center();
    press(&harness, from, true);
    harness.run();
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        harness.event(Event::PointerMoved(pos2(from.x, from.y + (to.y - from.y) * t)));
        harness.run();
    }
    press(&harness, to, false);
    harness.run();
    assert_eq!(queue_actions(&harness), vec![QueueAction::Move { id: harp, to: 0 }]);
    assert_eq!(harness.state().state.vm.queue.shown()[1], harp, "it is next in line");

    // A click still selects the row.
    control(&harness, ALTO_ROW).click();
    harness.run();
    assert_eq!(queue_actions(&harness).last(), Some(&QueueAction::Select(ALTO)));
}

#[test]
fn numbers_that_are_not_numbers_never_read_nan() {
    let _gpu = gpu();
    let mut vm = render::vm();
    for job in &mut vm.queue.jobs {
        match &mut job.state {
            JobState::Done { seconds, .. } => *seconds = f64::NAN,
            JobState::Rendering { done, total } => (*done, *total) = (5, 0),
            _ => {}
        }
    }
    let harness = showing(REF, vm);
    let texts = all_text(&harness);
    assert!(texts.iter().any(|t| t == "Done. All checks passed."), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Drawing frames"), "{texts:?}");
    for text in &texts {
        for bad in ["NaN", "inf", "-0", "%%"] {
            assert!(!text.contains(bad), "{text:?}");
        }
    }
    assert!(!texts.iter().any(|t| t.contains("Done in")), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "5 of 0"), "no count of no frames: {texts:?}");
}

#[test]
fn long_paths_and_titles_are_cut_short_and_shown_whole_on_hover() {
    let _gpu = gpu();
    let long_title = "Bass clarinet vs Contrabass, the longest match-up the roster can make, and then some more";
    let mut vm = render::vm();
    vm.settings.output_folder = PathBuf::from("Videos")
        .join("BALLFIGHT")
        .join("a folder with a very long name that goes on and on past the end of the field");
    for job in &mut vm.queue.jobs {
        if job.id == ALTO {
            job.spec.title = long_title.to_string();
        }
    }
    let mut harness = showing(MIN, vm);
    let workspace = harness.state().layout.workspace;
    let title = harness.get_by_label(long_title).rect();
    assert!(title.max.x <= workspace.max.x - CHECKS_W, "{title:?}");
    harness.get_by_label(long_title).hover();
    harness.run();
    assert_eq!(
        harness.query_all_by_label(long_title).count(),
        2,
        "hovering shows all of it"
    );
    let field = control(&harness, "Saves to");
    let path = field.value().unwrap_or_default();
    assert!(path.ends_with(&format!("flute_vs_trombone{}", std::path::MAIN_SEPARATOR)));
    field.hover();
    harness.run();
    assert!(has(&harness, &path), "hovering shows the whole path");
}

#[test]
fn at_the_minimum_size() {
    let _gpu = gpu();
    let (vm, _, _) = three_waiting();
    let mut harness = showing(MIN, vm);
    let workspace = harness.state().layout.workspace;
    for label in ["Add to queue", "Pause queue", "Change folder"] {
        let r = control(&harness, label).rect();
        assert!(workspace.contains_rect(r), "{label} at {r:?} inside {workspace:?}");
    }
    let header = harness.get_by_label("Pause queue").rect();
    // Every row's buttons can be scrolled to, and then lie inside the list.
    for (row, button) in [
        (TUBA_ROW, "Cancel"),
        (ALTO_ROW, "Remove"),
        ("Harp vs Violin, seed 88", "Remove"),
        (OBOE_ROW, "Queue again"),
        (OBOE_ROW, "Open in Music"),
        (OBOE_ROW, "Remove"),
        (CELLO_ROW, "Open folder"),
    ] {
        control(&harness, row).scroll_to_me();
        harness.run();
        let r = in_row(&harness, row, button)
            .unwrap_or_else(|| panic!("{row}: {button}"))
            .rect();
        assert!(
            r.min.y >= header.max.y && r.max.y <= workspace.max.y && r.max.x <= workspace.max.x - CHECKS_W,
            "{row}: {button} at {r:?}"
        );
    }
}

/// Whether a `TEXT_STRONG` pixel lies 1 to 5 px outside `rect` on the side `side`, at `along` of that side.
fn ring_beside(harness: &mut Harness<'_, BfApp>, rect: Rect, side: &str, along: f32) -> bool {
    let at: Vec<(u32, u32)> = (1..=5)
        .map(|d| {
            let d = d as f32;
            let x = rect.min.x + along * rect.width();
            let y = rect.min.y + along * rect.height();
            match side {
                "above" => (x, rect.min.y - d),
                "below" => (x, rect.max.y - 1.0 + d),
                "left" => (rect.min.x - d, y),
                _ => (rect.max.x - 1.0 + d, y),
            }
        })
        .map(|(x, y)| (x as u32, y as u32))
        .collect();
    pixels(harness, &at).contains(&[255; 3])
}

const SIDES: [&str; 4] = ["above", "below", "left", "right"];

/// Gives the keyboard to a control with `focus` and checks its ring on all four sides of the rect `find` gives.
fn assert_ring(
    harness: &mut Harness<'_, BfApp>,
    what: &str,
    size: (f32, f32),
    find: impl Fn(&Harness<'_, BfApp>) -> Rect,
    focus: impl Fn(&Harness<'_, BfApp>),
) {
    focus(harness);
    harness.run();
    let rect = find(harness);
    for side in SIDES {
        assert!(ring_beside(harness, rect, side, 0.3), "{what}: ring {side} at {size:?}");
    }
}

#[test]
fn every_control_shows_its_focus_ring() {
    let _gpu = gpu();
    for size in [REF, MIN] {
        let mut harness = showing(size, render::vm());
        // With nothing focused there is no ring.
        for label in [
            "Change folder",
            "Add to queue",
            "Pause queue",
            TUBA_ROW,
            "Cancel",
            ALTO_ROW,
        ] {
            let rect = control(&harness, label).rect();
            for side in SIDES {
                assert!(
                    !ring_beside(&mut harness, rect, side, 0.3),
                    "{label}: no ring {side} before focus"
                );
            }
        }
        for segment in ["Instagram", "TikTok", "Shorts"] {
            let find = move |h: &Harness<'_, BfApp>| {
                h.query_all_by_label(segment)
                    .find(|n| n.accesskit_node().role() == Role::RadioButton)
                    .unwrap()
                    .rect()
            };
            let focus = move |h: &Harness<'_, BfApp>| {
                h.query_all_by_label(segment)
                    .find(|n| n.accesskit_node().role() == Role::RadioButton)
                    .unwrap()
                    .focus()
            };
            assert_ring(&mut harness, segment, size, find, focus);
        }
        // The caption box's ring goes round its well: 4 under its label, 66 tall, to the column's padding.
        let well = |h: &Harness<'_, BfApp>| {
            let label = box_label(h);
            let right = h.state().layout.workspace.max.x - CHECKS_W - 14.0;
            Rect::from_min_max(
                pos2(label.min.x, label.max.y + 4.0),
                pos2(right, label.max.y + 4.0 + 66.0),
            )
        };
        assert_ring(&mut harness, "the caption box", size, well, |h| {
            caption_box(h, INSTAGRAM_BOX).focus()
        });
        for label in ["Change folder", "Add to queue", "Pause queue"] {
            assert_ring(
                &mut harness,
                label,
                size,
                move |h| control(h, label).rect(),
                move |h| control(h, label).focus(),
            );
        }
        for (row, button) in [
            (TUBA_ROW, None),
            (TUBA_ROW, Some("Cancel")),
            (ALTO_ROW, Some("Remove")),
            (OBOE_ROW, Some("Queue again")),
            (OBOE_ROW, Some("Open in Music")),
            (CELLO_ROW, None),
            (CELLO_ROW, Some("Open folder")),
        ] {
            control(&harness, row).scroll_to_me();
            harness.run();
            let what = format!("{row} {button:?}");
            match button {
                None => assert_ring(
                    &mut harness,
                    &what,
                    size,
                    move |h| control(h, row).rect(),
                    move |h| control(h, row).focus(),
                ),
                Some(button) => assert_ring(
                    &mut harness,
                    &what,
                    size,
                    move |h| in_row(h, row, button).unwrap().rect(),
                    move |h| in_row(h, row, button).unwrap().focus(),
                ),
            }
        }
        drop(harness);

        let mut vm = render::vm();
        vm.folder = FolderState::Missing;
        let mut harness = showing(size, vm);
        assert_ring(
            &mut harness,
            "Create folder",
            size,
            |h| control(h, "Create folder").rect(),
            |h| control(h, "Create folder").focus(),
        );
    }
}

#[test]
fn tab_walks_the_screen_in_order() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    // Each stop: its name, and the row it lies in (for the names that repeat).
    let order: [(&str, Option<&str>); 17] = [
        ("Instagram", None),
        ("TikTok", None),
        ("Shorts", None),
        (INSTAGRAM_BOX, None),
        ("Change folder", None),
        ("Add to queue", None),
        ("Pause queue", None),
        (TUBA_ROW, None),
        ("Cancel", Some(TUBA_ROW)),
        (ALTO_ROW, None),
        ("Remove", Some(ALTO_ROW)),
        (OBOE_ROW, None),
        ("Queue again", Some(OBOE_ROW)),
        ("Open in Music", Some(OBOE_ROW)),
        ("Remove", Some(OBOE_ROW)),
        (CELLO_ROW, None),
        ("Open folder", Some(CELLO_ROW)),
    ];
    let focused = |h: &Harness<'_, BfApp>| {
        let node = h.get_by(|n| n.is_focused());
        (text_of(&node), node.rect())
    };
    harness
        .query_all_by_label("Instagram")
        .find(|n| n.accesskit_node().role() == Role::RadioButton)
        .unwrap()
        .focus();
    harness.run();
    for (i, (name, row)) in order.iter().enumerate() {
        if i > 0 {
            harness.key_press(Key::Tab);
            harness.run();
        }
        let (label, rect) = focused(&harness);
        assert_eq!(&label, name, "stop {i}");
        if let Some(row) = row {
            assert!(
                control(&harness, row).rect().contains_rect(rect),
                "stop {i}: {name} in {row}"
            );
        }
    }
    assert!(
        harness
            .state()
            .log
            .iter()
            .all(|a| !matches!(a, Action::Queue(_) | Action::AddToQueue)),
        "walking through does nothing: {:?}",
        harness.state().log
    );
}

#[test]
fn nothing_is_unnamed() {
    let _gpu = gpu();
    let mut missing = render::vm();
    missing.folder = FolderState::Missing;
    let mut no_seed = render::vm();
    no_seed.draft.seed = None;
    no_seed.take = TakeView::default();
    let mut empty = render::vm();
    empty.queue.jobs.clear();
    let (three, _, _) = three_waiting();
    for vm in [render::vm(), missing, no_seed, empty, three] {
        let harness = showing(REF, vm);
        assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
    }
}

#[test]
fn render_matches() {
    let _gpu = gpu();
    let mut harness = showing(REF, render::vm());
    harness.snapshot("render_1440x900");
}

#[test]
fn the_fixture_ids_are_the_shown_order() {
    assert_eq!(render::vm().queue.shown(), vec![TUBA, ALTO, OBOE, CELLO]);
}

#[test]
fn the_queue_shows_its_scroll_bar_when_it_overflows() {
    let _gpu = gpu();
    let (vm, _, _) = three_waiting();
    let mut harness = showing(MIN, vm);
    let workspace = harness.state().layout.workspace;
    let header = harness.get_by_label("Pause queue").rect();
    let row = control(&harness, TUBA_ROW).rect();
    // The bar: 4 past the rows, 6 wide; its handle is FAINT without hovering.
    let x = (row.max.x + 4.0 + 3.0) as u32;
    assert!(
        x as f32 <= workspace.max.x - CHECKS_W,
        "the bar lies in the queue's column"
    );
    let column: Vec<(u32, u32)> = (header.max.y as u32 + 8..workspace.max.y as u32 - 2)
        .map(|y| (x, y))
        .collect();
    let px = pixels(&mut harness, &column);
    assert!(
        px.contains(&[0x77, 0x77, 0x77]),
        "the handle is drawn FAINT without hovering"
    );
}

#[test]
fn a_done_time_that_is_infinite_or_negative_is_not_shown() {
    let _gpu = gpu();
    for seconds in [f64::INFINITY, f64::NEG_INFINITY, -3.0, -0.0001] {
        let mut vm = render::vm();
        for job in &mut vm.queue.jobs {
            if let JobState::Done { seconds: s, .. } = &mut job.state {
                *s = seconds;
            }
        }
        let harness = showing(REF, vm);
        let texts = all_text(&harness);
        assert!(
            in_row(&harness, CELLO_ROW, "Done. All checks passed.").is_some(),
            "{seconds}: {texts:?}"
        );
        assert!(!texts.iter().any(|t| t.contains("Done in")), "{seconds}: {texts:?}");
        for text in &texts {
            for bad in ["NaN", "inf", "-0", "-3"] {
                assert!(!text.contains(bad), "{seconds}: {text:?}");
            }
        }
    }
}

#[test]
fn a_flag_and_a_joined_sequence_keep_every_character_through_an_edit() {
    let _gpu = gpu();
    // A flag is two regional indicators (U+1F1EB U+1F1F7, France); a joined sequence is three emoji held together by
    // zero-width joiners (U+1F468 U+200D U+1F469 U+200D U+1F467). As in the sample caption, the test harness has no
    // font with them and they are written here as escapes; the box must keep every one of them.
    let caption =
        "Flute vs Trombone \u{1f1eb}\u{1f1f7} for the family \u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} #music";
    let mut vm = render::vm();
    vm.draft.captions.instagram = caption.to_string();
    let mut harness = showing(REF, vm);
    assert_eq!(caption_box(&harness, INSTAGRAM_BOX).value().as_deref(), Some(caption));
    caption_box(&harness, INSTAGRAM_BOX).focus();
    harness.run();
    caption_box(&harness, INSTAGRAM_BOX).type_text("!");
    harness.run();
    let typed = format!("{caption}!");
    match harness.state().log.last() {
        Some(Action::SetCaption { text, .. }) => assert_eq!(text, &typed, "every character kept, the new one last"),
        other => panic!("SetCaption was pushed: {other:?}"),
    }
    harness.key_press(Key::Backspace);
    harness.run();
    match harness.state().log.last() {
        Some(Action::SetCaption { text, .. }) => assert_eq!(text, caption, "Backspace takes away only the new one"),
        other => panic!("SetCaption was pushed: {other:?}"),
    }
    // From the start of the box, the arrows step over the characters one by one and an edit there keeps the rest.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Home);
    harness.run();
    caption_box(&harness, INSTAGRAM_BOX).type_text("A ");
    harness.run();
    match harness.state().log.last() {
        Some(Action::SetCaption { text, .. }) => assert_eq!(text, &format!("A {caption}")),
        other => panic!("SetCaption was pushed: {other:?}"),
    }
}

/// Note (c) of Task 60: a take whose soundtrack fails a check would stop at that check when it renders, so "Add to
/// queue" says so and offers the next fight that fits.
#[test]
fn a_take_that_fails_a_check_is_not_queued_and_offers_the_next_fit() {
    let _gpu = gpu();
    let mut vm = render::vm();
    assert_eq!(why_not_add(&vm), None);
    vm.shortlist.rows = bf_app::seam::sample::stories();
    let seed = vm.draft.seed.expect("a seed");
    let at = vm
        .shortlist
        .rows
        .iter()
        .position(|r| r.seed == seed)
        .expect("on the list");
    let next = vm.shortlist.rows[(at + 1) % vm.shortlist.rows.len()].seed;
    vm.take.music.as_mut().expect("its sound").checks[2].pass = false;
    let title = vm.take.music.as_ref().unwrap().checks[2].title.clone();
    let words = format!(
        "Seed {seed} fails the check \"{title}\", so its video would stop there and nothing would be saved. Try seed {next}, the next fight that fits."
    );
    assert_eq!(why_not_add(&vm).as_deref(), Some(words.as_str()));
    let mut harness = showing(REF, vm);
    assert!(
        harness
            .query_all_by_label("Add to queue")
            .any(|n| n.accesskit_node().is_disabled())
    );
    assert!(harness.query_all_by_label(&words).next().is_some(), "{words}");
    harness.get_by_label(&format!("Use seed {next}")).click();
    harness.run();
    assert_eq!(harness.state().log, vec![Action::SelectSeed(next)]);
    assert_eq!(harness.unnamed_widgets(), Vec::<String>::new());
}

/// The Render screen of the kit's BASS DRUM against TUBA, its own seed 132 and take: its captions, its folder.
#[test]
fn render_matches_for_bassdrum_vs_tuba() {
    let _gpu = gpu();
    let kit = crate::music::kit_vm("bassdrum_vs_tuba", Step::Render, true);
    let mut vm = render::vm();
    vm.roster = kit.roster;
    vm.draft = kit.draft;
    vm.pair_rules = kit.pair_rules;
    vm.pair_music = kit.pair_music;
    vm.take = kit.take;
    vm.status = kit.status;
    assert_eq!(vm.draft.seed, Some(132));
    assert_eq!(why_not_add(&vm), None, "its take passes every check");
    let mut harness = showing(REF, vm);
    harness.snapshot("render_bassdrum_vs_tuba_1440x900");
}
