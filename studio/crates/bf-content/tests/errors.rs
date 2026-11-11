//! What a fight file or a recording can do wrong, through `bf_content::validate` and `build_soundtrack`: an
//! unknown theme or arrangement, a chord that is no chord, a fighter whose part is not here, empty
//! title hits, a fight with no K.O. or another fight's recording, and a panic inside the sound code. Each is an
//! error in words, never a panic.

mod common;

use bf_content::{ContentError, build_soundtrack, theme_names, validate};
use bf_core::recording::{Array, Recording};
use bf_testdata::reference;

use common::EPISODE;

#[test]
fn the_registry_lists_the_themes() {
    assert_eq!(theme_names(), ["big_top"]);
    assert_eq!(validate(&EPISODE.ff), Ok(()));
}

#[test]
fn unknown_theme_is_an_error() {
    let mut ff = EPISODE.ff.clone();
    ff.music.theme = "polka".into();
    let want = r#"music.theme: "polka" is not a theme"#;
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
}

/// Every chord the harmony parses is voiced (milestone 3: the voicing rule replaces milestone 1's list of the big
/// top's four chords, ruling R33), so a chord is only refused for not being a chord, in the fight file's words.
#[test]
fn every_chord_is_voiced() {
    let mut ff = EPISODE.ff.clone();
    ff.music.chord_loop[4] = "F#m".into();
    ff.music.chords_intro[1] = "G7".into();
    ff.music.chords_tail = vec!["Bbm".into(), "Ebmaj7".into()];
    assert_eq!(validate(&ff), Ok(()));
    // What the fight file's own checks refuse comes first, in their words.
    ff.music.chord_loop.pop();
    assert_eq!(
        validate(&ff).unwrap_err().to_string(),
        "music.chord_loop: 4 or 8 chords needed, found 7"
    );
    ff.music.chord_loop = vec!["Em".into(), "Am".into(), "B7".into(), "H".into()];
    assert_eq!(
        validate(&ff).unwrap_err().to_string(),
        r#"music.chord_loop[3]: "H" is not a chord name (a root such as E, F# or Bb, then a quality)"#
    );
}

#[test]
fn a_fighter_without_a_part_is_an_error() {
    // PIANO is in the roster, so the fight file is valid, but its part in the score is not ported.
    let mut ff = EPISODE.ff.clone();
    ff.fighters[0] = "PIANO".into();
    ff.music.lines.clear();
    ff.music.solos.clear();
    ff.music.title_hits.clear();
    ff.music.roles.clear();
    let mut rec = EPISODE.rec.clone();
    rec.names[0] = "PIANO".into();
    // The app can refuse the file before it renders anything: `validate` says what the build would.
    let want = "fighters[0]: PIANO has no part in the score yet";
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&rec, &ff).unwrap_err().to_string(), want);
    // The other corner too.
    let mut ff = EPISODE.ff.clone();
    ff.fighters[1] = "PIANO".into();
    ff.music.lines.clear();
    ff.music.solos.clear();
    ff.music.title_hits.clear();
    ff.music.roles.clear();
    assert_eq!(
        validate(&ff).unwrap_err().to_string(),
        "fighters[1]: PIANO has no part in the score yet"
    );
}

#[test]
fn unknown_arrangement_is_an_error() {
    let mut ff = EPISODE.ff.clone();
    ff.music.arrangement = "jazz".into();
    // `FightFile::validate` knows the arrangements already, in its words.
    let want = r#"music.arrangement: "jazz" is not an arrangement (this app has: classic)"#;
    assert_eq!(validate(&ff).unwrap_err().to_string(), want);
    assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
}

#[test]
fn empty_title_hits_are_an_error() {
    // Every part plays its title hits on the VS card (the first and the last note): an empty list cannot be
    // played (Python raises an IndexError), whoever wins.
    for name in ["FLUTE", "TROMBONE"] {
        let mut ff = EPISODE.ff.clone();
        ff.music.title_hits.insert(name.into(), Vec::new());
        let want = format!("music.title_hits.{name}: the title hits are an empty list");
        assert_eq!(validate(&ff).unwrap_err().to_string(), want);
        assert_eq!(build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string(), want);
    }
}

#[test]
fn a_fight_without_a_ko_is_an_error() {
    let rec = Recording::read(&reference("fights/flute_vs_trombone/no_ko.bfrec")).expect("the recording reads");
    let err = build_soundtrack(&rec, &EPISODE.ff).unwrap_err();
    assert!(matches!(err, ContentError::Show(_)), "{err:?}");
    assert_eq!(err.to_string(), "the fight has no K.O. (no winner and K.O. time)");
    // Another fight's recording.
    let mut ff = EPISODE.ff.clone();
    ff.fighters.reverse();
    let err = build_soundtrack(&EPISODE.rec, &ff).unwrap_err().to_string();
    assert!(err.starts_with("the recording's fighters"), "{err}");
}

#[test]
fn a_panic_in_the_sound_code_is_an_error() {
    // A recording whose HP is above 100: the note thinning cannot take it (Python raises there, the port
    // panics), and the build must hand that back as an error, not take its caller down.
    let mut rec = EPISODE.rec.clone();
    let Some(Array::F64 { data, .. }) = rec.arrays.get_mut("HP") else {
        panic!("the recording has no HP");
    };
    data.fill(150.0);
    let err = build_soundtrack(&rec, &EPISODE.ff).unwrap_err();
    assert!(matches!(err, ContentError::Sound(_)), "{err:?}");
    let text = err.to_string();
    assert!(
        text.starts_with("the soundtrack could not be built: ") && text.contains("hp01 is 1.5, above 1"),
        "{text}"
    );
    // A recording that lacks what a part reads is an error too.
    let mut rec = EPISODE.rec.clone();
    rec.arrays.remove("HA");
    let text = build_soundtrack(&rec, &EPISODE.ff).unwrap_err().to_string();
    assert!(text.contains("HA"), "{text}");
}
