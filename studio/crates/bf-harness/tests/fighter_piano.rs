//! PIANO against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), and PIANO's own
//! tests: its music feeds its fight (the search key and the roll's rows follow the key, the chord loop and its line,
//! and no other fighter's do), and the TROMBONE's lance parries its keys as the kit's does.

use bf_core::event::{Event, Value};
use bf_core::fightfile::FightFile;
use bf_harness::fight::{doc, fight_file, fight_file_of, roster, same_event, setup_like, traced};
use bf_harness::refs::{cached_recording, list, text};
use bf_sim::setup_from;

bf_harness::fighter_suite!("PIANO");

/// The search key of a fight file, as the app makes it (`bf_content::music_feeds_fight` says whose music counts).
fn search_key(ff: &FightFile) -> String {
    let feeds: Vec<bool> = ff.fighters.iter().map(|n| bf_content::music_feeds_fight(n)).collect();
    ff.search_key(&feeds)
}

/// The three music changes that change a roll's rows: another key, another chord loop (the same chords in another
/// order, so every chord is still one of the episode's), another line for PIANO.
fn music_changes(ff: &FightFile) -> Vec<(&'static str, FightFile)> {
    let mut key = ff.clone();
    key.music.key = if ff.music.key == "D" { "E".into() } else { "D".into() };
    let mut chords = ff.clone();
    chords.music.chord_loop.swap(1, 2);
    assert_ne!(
        chords.music.chord_loop, ff.music.chord_loop,
        "the loop's second and third chords differ"
    );
    let mut line = ff.clone();
    let notes = 8 * ff.music.chord_loop.len();
    let tune = (0..notes).map(|i| (i % 3 != 1).then_some(60 + (i % 13) as i32));
    line.music.lines.insert("PIANO".into(), tune.collect());
    vec![("key", key), ("chord loop", chords), ("line", line)]
}

/// Review Focus 7: PIANO's music changes its fight. A key, loop or line change changes the search key of a PIANO
/// fight (on either side) and the rows its roll fires; it changes neither for a fight without PIANO.
#[test]
fn a_music_change_changes_a_piano_fights_search_key_and_no_other_fights() {
    assert!(bf_content::fighters::piano::MUSIC_FEEDS_FIGHT);
    assert!(bf_content::music_feeds_fight(NAME));
    let d = doc(NAME);
    for swapped in [false, true] {
        let ff = fight_file(&d, swapped);
        let side = ff.fighters.iter().position(|f| f == NAME).expect("PIANO fights");
        let rows = |ff: &FightFile| {
            let rules = setup_from(ff, roster(), 1).expect("sets up").rules;
            rules.roll_rows.expect("a roller's rows")[&side].clone()
        };
        for (what, changed) in music_changes(&ff) {
            assert_ne!(search_key(&changed), search_key(&ff), "{what}, PIANO on side {side}");
            // the key moves the sketch line's chord tones only when the chords move with it: a new key alone
            // leaves the chord loop, so the rows are compared for the loop and the line
            if what != "key" {
                assert_ne!(rows(&changed), rows(&ff), "{what}: the roll's rows");
            }
        }
    }
    // a fight without PIANO: the same changes leave its search key and its rules alone
    let plain = fight_file_of(&d, &["FLUTE".to_string(), "TROMBONE".to_string()]);
    assert!(!plain.fighters.iter().any(|f| bf_content::music_feeds_fight(f)));
    let rules = |ff: &FightFile| format!("{:?}", setup_from(ff, roster(), 1).expect("sets up").rules);
    for (what, changed) in music_changes(&plain) {
        assert_eq!(
            search_key(&changed),
            search_key(&plain),
            "{what}: FLUTE against TROMBONE"
        );
        assert_eq!(rules(&changed), rules(&plain), "{what}: FLUTE against TROMBONE");
    }
}

/// Review Focus 3: a PIANO line that is not 8 (or 4) notes a bar is refused, naming the line.
#[test]
fn a_piano_line_of_6_a_bar_is_refused_by_name() {
    let mut ff = fight_file(&doc(NAME), false);
    let bars = ff.music.chord_loop.len();
    ff.music.lines.insert("PIANO".into(), vec![Some(60); 6 * bars]);
    let e = setup_from(&ff, roster(), 1).expect_err("6 notes a bar");
    assert_eq!(e.field, "music.lines.PIANO");
    assert!(e.message.contains("8 notes a bar (or 4)"), "{}", e.message);
    // 4 a bar is a line of beats: an 8th's rest follows each note
    ff.music.lines.insert("PIANO".into(), vec![Some(60); 4 * bars]);
    let rules = setup_from(&ff, roster(), 1).expect("4 notes a bar").rules;
    let rows = &rules.roll_rows.expect("rows")[&0];
    assert_eq!(rows.len(), 8 * bars);
    assert!(rows.iter().step_by(2).all(|r| !r.is_empty()));
    assert!(rows.iter().skip(1).step_by(2).all(|r| r.is_empty()));
}

/// The TROMBONE's lance parries PIANO's keys (the kit's `parry` events, `fight.py` 746 to 752): in the scenarios
/// against the TROMBONE the kit's fights have them, and the engine's fights have the same ones for the first ten
/// seconds of fighting.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/piano"]
fn the_lance_parries_the_rolls_keys() {
    let d = doc(NAME);
    let sim = bf_harness::sim!();
    let mut seen = 0;
    for s in list(&d["scenarios"]) {
        if text(&s["opponent"]) != "TROMBONE" {
            continue;
        }
        for file in list(&s["cache"]) {
            let file = text(file);
            let py = cached_recording(file);
            let rs = traced(&sim, &setup_like(&py), true, file);
            let t_fight = match py.rules.get("t_fight") {
                Some(Value::Float(t)) => *t,
                other => panic!("{file}: rules.t_fight is {other:?}"),
            };
            let early = |e: &&Event| e.kind == "parry" && e.t < t_fight + 10.0;
            let want: Vec<&Event> = py.events.iter().filter(early).collect();
            let got: Vec<&Event> = rs.events.iter().filter(early).collect();
            assert!(
                !want.is_empty(),
                "{file}: the kit's lance parries no key in the first 10 s"
            );
            assert_eq!(got.len(), want.len(), "{file}: parries in the first 10 s");
            for (a, b) in got.iter().zip(&want) {
                // ('parry', the key's owner, the lance's owner, x, y, id)
                assert!(same_event(a, b), "{file}: {a:?} against the kit's {b:?}");
            }
            seen += want.len();
        }
    }
    assert!(seen > 0, "fight.json lists no scenario against TROMBONE");
    println!("{NAME}: {seen} parries of the kit's in the first 10 s, each the engine's too");
}
