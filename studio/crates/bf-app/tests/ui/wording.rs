//! The wording rules of section 11, checked over every fixture of every screen: sentence case, no retired fighter,
//! no vague button.

use bf_app::app::BfApp;
use bf_app::fixtures::{balance, base, library, matchup, music, render, seeds, settings};
use bf_app::model::shortlist::SearchState;
use bf_app::state::ViewModel;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::common::{REF, app_harness, gpu};

/// Words that keep their capital letter anywhere in a line: first three that the interface reference and the plan
/// use and the brief's list left out (named in the task report), then the brief's list. "LU" (loudness units, "0.4 LU under
/// the fight"), "HUD" ("Captions and HUD clear of the apps' buttons") and the toolbar button "Update". Then the
/// programs and places the update and removal texts name (plan M6, Task 13): "Git" and "Rust" ("because Git (or
/// Rust) is not installed") and the Start menu ("its Start menu and desktop shortcuts").
const NAMES: [&str; 25] = [
    "LU",
    "HUD",
    "Update",
    "Git",
    "Rust",
    "Start",
    "BALLFIGHT",
    "Studio",
    "K.O.",
    "HP",
    "PC",
    "Instagram",
    "TikTok",
    "Shorts",
    "LV2",
    "LV3",
    "LOW",
    "Match-up",
    "Balance",
    "Seeds",
    "Music",
    "Render",
    "Library",
    "Settings",
    "Queue",
];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Every fixture view model of the screens (Tasks 17 to 24, and Settings' update and removal states of milestone 6
/// Task 13), with a name for the failure message.
fn fixtures() -> Vec<(&'static str, ViewModel)> {
    let mut all = screens();
    all.extend(settings::each_update_state());
    all.push(("settings asking to remove", settings::asking_removal()));
    all.push(("settings after a removal", settings::removed()));
    all
}

/// Every screen's own fixture view models.
fn screens() -> Vec<(&'static str, ViewModel)> {
    vec![
        ("base", base::vm()),
        ("base with a take", base::with_take()),
        ("match-up", matchup::vm()),
        ("balance", balance::vm()),
        ("balance running", balance::running()),
        ("seeds", seeds::vm()),
        ("seeds idle", seeds::empty(SearchState::Idle)),
        ("seeds searching", seeds::empty(SearchState::Searching)),
        ("seeds stopped", seeds::empty(SearchState::Stopped)),
        ("seeds full", seeds::empty(SearchState::Full)),
        ("music", music::vm()),
        ("music without a seed", music::no_seed()),
        ("render", render::vm()),
        ("library", library::vm()),
        ("library empty", library::empty()),
        ("library missing", library::missing()),
        ("settings", settings::vm()),
    ]
}

/// The shell showing `vm` as it is, settled.
fn showing<'a>(vm: ViewModel) -> Harness<'a, BfApp> {
    let mut harness = app_harness(REF, move |cc| BfApp::showing(cc, vm));
    harness.run();
    harness
}

/// Whether a node is the caption text box, or a line of text inside it: what is typed there is the series' own.
fn is_caption_box(node: &egui_kittest::Node<'_>) -> bool {
    let mut at = Some(*node);
    while let Some(n) = at {
        let a = n.accesskit_node();
        if matches!(a.role(), Role::MultilineTextInput | Role::TextInput)
            && a.label().is_some_and(|l| l.starts_with("Text and tags for "))
        {
            return true;
        }
        at = n.parent();
    }
    false
}

/// Every accessible name and every text on screen (a label keeps its text in its value), except what is typed in
/// the caption box.
fn texts(harness: &Harness<'_, BfApp>) -> Vec<String> {
    harness
        .query_all_by(|_| true)
        .flat_map(|n| {
            let value = if is_caption_box(&n) {
                None
            } else {
                n.accesskit_node().value()
            };
            [n.accesskit_node().label(), value]
        })
        .flatten()
        .filter(|t| !t.is_empty())
        .collect()
}

/// The words of the fighters' and themes' names as shown.
fn name_words(vm: &ViewModel) -> Vec<String> {
    let roster = vm.roster.as_ref().expect("every fixture has the roster");
    roster
        .fighters
        .iter()
        .map(|f| f.display.as_str())
        .chain(roster.themes.iter().map(|t| t.display.as_str()))
        .flat_map(str::split_whitespace)
        .map(str::to_string)
        .collect()
}

/// A note or chord name: `[A-G]`, an optional `#` or `b`, an optional `m`, `7`, `m7` or `maj7`, an optional octave
/// digit.
fn is_note(word: &str) -> bool {
    let mut chars = word.chars();
    if !chars.next().is_some_and(|c| ('A'..='G').contains(&c)) {
        return false;
    }
    let mut rest = chars.as_str();
    rest = rest.strip_prefix(['#', 'b']).unwrap_or(rest);
    for quality in ["maj7", "m7", "m", "7"] {
        if let Some(after) = rest.strip_prefix(quality) {
            rest = after;
            break;
        }
    }
    rest.is_empty() || (rest.len() == 1 && rest.chars().all(|c| c.is_ascii_digit()))
}

/// The capitalised words of `text` that break the sentence-case rule. After its first word, a word may start with
/// a capital only if it follows ". " or ": ", holds a digit, a backslash or an underscore, is a note or chord name,
/// one of `NAMES`, a three-letter month, or a word of `names` (the fighters' and themes' names as shown).
fn breaks(text: &str, names: &[String]) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut bad = Vec::new();
    for pair in words.windows(2) {
        let (before, word) = (pair[0], pair[1]);
        // Quotes and brackets around a word are not part of it.
        let core = word.trim_start_matches(['"', '\'', '(', '[', '\u{201c}', '\u{2018}']);
        if !core.chars().next().is_some_and(char::is_uppercase) {
            continue;
        }
        if before.ends_with('.') || before.ends_with(':') {
            continue;
        }
        if core.chars().any(|c| c.is_ascii_digit() || c == '\\' || c == '_') {
            continue;
        }
        let bare = core.trim_end_matches([
            ',', '.', ':', ';', ')', ']', '"', '\'', '?', '!', '\u{201d}', '\u{2019}',
        ]);
        let bare = bare.strip_suffix("'s").unwrap_or(bare);
        let allowed = |w: &str| is_note(w) || NAMES.contains(&w) || MONTHS.contains(&w) || names.iter().any(|n| n == w);
        // "K.O." keeps its points; "Flute," and "Flute's" are the name.
        if allowed(core) || allowed(bare) || allowed(core.trim_end_matches([',', ';', ')'])) {
            continue;
        }
        bad.push(word.to_string());
    }
    bad
}

#[test]
fn the_rule_bites() {
    let names = vec!["Flute".to_string(), "Big".to_string(), "top".to_string()];
    assert!(breaks("Search seeds", &names).is_empty());
    assert_eq!(breaks("Search Seeds now and Then", &names), ["Then"]);
    assert_eq!(breaks("Use the Output folder", &names), ["Output"]);
    assert_eq!(breaks("Pick a fight, Then go on", &names), ["Then"]);
    for fine in [
        "Flute wins at 49.1 s. Trombone is close",
        "Theme: Big top",
        "Plays Em Am B7 C#m7 Gmaj7 A4 Bb in the loop",
        "Saves to Videos\\BALLFIGHT\\flute_vs_trombone\\",
        "Made on Nov 10, 2026 with BALLFIGHT Studio",
        "Open it in Music to see the K.O. and the HP left on this PC",
        "Seed 18433 is ready",
        "The Flute's cobras",
        "Text and tags for TikTok",
    ] {
        assert!(breaks(fine, &names).is_empty(), "{fine}: {:?}", breaks(fine, &names));
    }
}

#[test]
fn sentence_case_everywhere() {
    let _gpu = gpu();
    let mut problems = Vec::new();
    for (name, vm) in fixtures() {
        let words = name_words(&vm);
        let harness = showing(vm);
        let all = texts(&harness);
        assert!(all.len() > 20, "{name}: the screen's texts were read ({})", all.len());
        for text in all {
            let bad = breaks(&text, &words);
            if !bad.is_empty() {
                problems.push(format!("{name}: {text:?} has {bad:?}"));
            }
        }
    }
    problems.sort();
    problems.dedup();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn no_retired_fighter_anywhere() {
    let _gpu = gpu();
    for (name, vm) in fixtures() {
        let harness = showing(vm);
        for text in texts(&harness) {
            assert!(!text.to_lowercase().contains("guitar"), "{name}: {text:?}");
            assert!(text != "Sax" && text != "SAX", "{name}: {text:?}");
        }
    }
}

#[test]
fn no_button_is_vague() {
    let _gpu = gpu();
    for (name, vm) in fixtures() {
        let harness = showing(vm);
        let buttons: Vec<String> = harness
            .query_all_by_role(Role::Button)
            .filter_map(|n| n.accesskit_node().label())
            .collect();
        assert!(!buttons.is_empty(), "{name}: the buttons were read");
        for label in buttons {
            assert!(
                !["OK", "Yes", "No", "Submit", "Apply"].contains(&label.trim()),
                "{name}: the button {label:?} does not say what happens"
            );
        }
    }
}

/// Texts the start-up shows a person outside any screen (message boxes and start-up notices), which no fixture holds.
fn start_up_texts() -> Vec<String> {
    use bf_app::update::swap;
    vec![
        swap::FINISH_WORDS.to_string(),
        swap::NOT_STAGED.to_string(),
        swap::RESUME_FAILED.to_string(),
        swap::RESTART_FAILED.to_string(),
        swap::UPDATE_BUSY.to_string(),
        swap::lock_unusable("the folder cannot be written to"),
    ]
}

#[test]
fn the_start_up_texts_are_in_sentence_case() {
    for text in start_up_texts() {
        assert!(breaks(&text, &[]).is_empty(), "{text:?} has {:?}", breaks(&text, &[]));
        assert!(text.ends_with('.'), "{text:?} is a whole sentence");
        let first = text.trim_start_matches('-').chars().next().unwrap();
        assert!(first.is_uppercase() || text.starts_with("--"), "{text:?}");
    }
}

/// The update, removal and Settings texts of milestone 6 (Task 13) that no fixture shows on its own: the update
/// engine's sentences (Task 12), the refusals, and the encoder's missing line. Folder paths are the made-up
/// install's, whose words hold a backslash.
fn upkeep_texts() -> Vec<String> {
    use bf_app::screens::settings as screen;
    use bf_app::state::upkeep;
    use bf_app::update;
    vec![
        update::NO_TOOLS.to_string(),
        update::RENDERING.to_string(),
        update::ANOTHER_COPY.to_string(),
        update::check_failed("git could not reach the repository"),
        update::build_failed("0.6.1", "0.6.0", "error: could not compile the app"),
        update::stage_failed("0.6.1", "0.6.0", "the disk is full"),
        update::check_staged_failed("0.6.1", "0.6.0", "the new program named another version"),
        update::swap_failed("0.6.1", "0.6.0"),
        update::restart_failed("it could not be started"),
        upkeep::NOT_INSTALLED.to_string(),
        upkeep::NOTHING_TO_REMOVE.to_string(),
        upkeep::REMOVE_RENDERING.to_string(),
        upkeep::WAIT_FOR_BUILD.to_string(),
        upkeep::CHECKING.to_string(),
        upkeep::UP_TO_DATE.to_string(),
        upkeep::LOOKING_AT_REMOVAL.to_string(),
        upkeep::REMOVING.to_string(),
        upkeep::available("0.6.1"),
        upkeep::building("0.6.1", 754),
        upkeep::ready("0.6.1"),
        screen::FFMPEG_MISSING.to_string(),
        screen::ENCODING_NOTE.to_string(),
        screen::REPORTS_SOMEWHERE.to_string(),
        screen::NO_REPORTS_FOLDER.to_string(),
        screen::NO_GUIDE.to_string(),
        upkeep::CLOSING.to_string(),
        upkeep::NOT_REMOVED.to_string(),
        bf_app::seam::upkeep::SAMPLE_KEPT.to_string(),
        bf_app::seam::upkeep::NO_FOLDERS.to_string(),
        bf_app::seam::upkeep::NO_PLAN.to_string(),
        // The reports line, its folder a made-up one (a path is words with a backslash).
        screen::reports_line(&bf_app::fixtures::settings::reports()),
        // A removal plan refused for an install that is not there (Task 11's words, nothing on the PC is touched).
        removal_refused(),
    ]
    .into_iter()
    .chain(bf_app::seam::upkeep::sample_removal_words())
    .chain(bf_app::seam::upkeep::sample_removed_words())
    .collect()
}

#[test]
fn the_update_texts_are_in_sentence_case() {
    let texts = upkeep_texts();
    assert!(texts.len() > 25);
    for text in texts {
        assert!(breaks(&text, &[]).is_empty(), "{text:?} has {:?}", breaks(&text, &[]));
        assert!(text.ends_with('.'), "{text:?} is a whole sentence");
        assert!(
            text.chars().next().is_some_and(char::is_uppercase) || text.starts_with("ffmpeg "),
            "{text:?}"
        );
        assert!(
            !text.contains("NaN") && !text.contains("Some(") && !text.contains("Err("),
            "{text:?}"
        );
    }
    // The rule bites on these words too.
    assert_eq!(breaks("Ask for Rust and Cargo", &[]), ["Cargo"]);
}

/// Task 11's refusal for a made-up install that is not on this PC, as Settings would show it.
fn removal_refused() -> String {
    let layout = bf_pack::install::Layout::under(std::path::Path::new(r"C:\Users\owner\AppData\Local\Not here"));
    let refused = bf_pack::remove::removal_plan(&layout).expect_err("nothing is there");
    bf_app::seam::upkeep::without_os_errors(&refused.to_string())
}
