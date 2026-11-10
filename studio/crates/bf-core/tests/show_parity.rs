//! `bf_core::show` against what `projects/ballfight/kit/picture.py` derives at import (`studio/reference/show/`,
//! written by `python -m tools.studio_export show`): for the five reference recordings and the example fight file,
//! every `Show` value equals the kit's, integers and strings exactly and times bit for bit (they are sums and
//! products, no transcendental). `captions()` equals `story.captions()`. A fight with no K.O. is an error, never a
//! panic (Review Focus 1).

use std::fs;
use std::path::{Path, PathBuf};

use bf_core::event::Value as EvValue;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::{CAP_GAP, Cap, HITSTOP, Show, ShowError, captions};
use serde_json::Value;

fn studio() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fights() -> PathBuf {
    studio().join("reference/fights/flute_vs_trombone")
}

fn vectors() -> Value {
    let text = fs::read_to_string(studio().join("reference/show/flute_vs_trombone.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn fight_file(seed: u64) -> FightFile {
    let text = fs::read_to_string(studio().join("fights/flute_vs_trombone.toml")).unwrap();
    let mut ff = FightFile::parse(&text).unwrap();
    ff.seed = Some(seed);
    ff
}

fn recording(name: &str) -> Recording {
    Recording::read(&fights().join(name)).unwrap()
}

/// A Python `float.hex()` string as the `f64` with exactly those bits.
fn hexf(s: &str) -> f64 {
    let bad = || -> ! { panic!("not a float.hex() string: {s:?}") };
    let (negative, rest) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let rest = rest.strip_prefix("0x").unwrap_or_else(|| bad());
    let (mantissa, exponent) = rest.split_once('p').unwrap_or_else(|| bad());
    let (lead, fraction) = mantissa.split_once('.').unwrap_or_else(|| bad());
    if fraction.is_empty() || fraction.len() > 13 {
        bad();
    }
    let digits = u64::from_str_radix(fraction, 16).unwrap_or_else(|_| bad());
    let fraction_bits = digits << (4 * (13 - fraction.len()));
    let exponent: i64 = exponent.parse().unwrap_or_else(|_| bad());
    let magnitude = match lead {
        "1" if (-1022..=1023).contains(&exponent) => (((exponent + 1023) as u64) << 52) | fraction_bits,
        "0" if exponent == -1022 || (exponent == 0 && fraction_bits == 0) => fraction_bits,
        _ => bad(),
    };
    f64::from_bits(magnitude | (u64::from(negative) << 63))
}

#[track_caller]
fn same_bits(got: f64, want: &Value, what: &str) {
    let text = want
        .as_str()
        .unwrap_or_else(|| panic!("{what}: not a hex string: {want}"));
    let want = hexf(text);
    assert!(
        got.to_bits() == want.to_bits(),
        "{what}: got {got:?} ({:#018x}), the kit gives {want:?} = {text} ({:#018x})",
        got.to_bits(),
        want.to_bits()
    );
}

fn caps_equal(got: &[Cap], want: &Value, what: &str) {
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "{what}: number of windows");
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        same_bits(g.t0, &w["t0"], &format!("{what}[{i}].t0"));
        same_bits(g.t1, &w["t1"], &format!("{what}[{i}].t1"));
        let lines: Vec<&str> = w["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l.as_str().unwrap())
            .collect();
        assert_eq!(g.lines, lines, "{what}[{i}].lines");
    }
}

fn captions_equal(got: &[(f64, usize, String)], want: &Value, what: &str) {
    let want = want.as_array().unwrap();
    assert_eq!(got.len(), want.len(), "{what}: number of captions");
    for (i, ((t, who, name), w)) in got.iter().zip(want).enumerate() {
        same_bits(*t, &w[0], &format!("{what}[{i}].t"));
        assert_eq!(*who as u64, w[1].as_u64().unwrap(), "{what}[{i}].who");
        assert_eq!(name, w[2].as_str().unwrap(), "{what}[{i}].name");
    }
}

const REFERENCE: [&str; 5] = [
    "ep392.bfrec",
    "seed1.bfrec",
    "seed2.bfrec",
    "seed3.bfrec",
    "seed500.bfrec",
];

#[test]
fn the_vectors_cover_the_five_reference_recordings_and_no_ko() {
    let v = vectors();
    let recs = v["recordings"].as_object().unwrap();
    for name in REFERENCE.iter().chain(&["no_ko.bfrec"]) {
        assert!(recs.contains_key(*name), "{name} missing from the vectors");
    }
    assert_eq!(recs.len(), 6);
}

#[test]
fn show_equals_the_kit_for_every_reference_recording() {
    let v = vectors();
    for name in REFERENCE {
        let want = &v["recordings"][name];
        let rec = recording(name);
        assert_eq!(want["seed"].as_u64().unwrap(), rec.seed, "{name}: seed");
        let ff = fight_file(rec.seed);
        let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert_eq!(show.grid, ff.grid(), "{name}: grid");
        let names: Vec<&str> = want["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap())
            .collect();
        assert_eq!(show.names.to_vec(), names, "{name}: names");
        assert_eq!(show.win as u64, want["win"].as_u64().unwrap(), "{name}: win");
        assert_eq!(show.lose as u64, want["lose"].as_u64().unwrap(), "{name}: lose");
        same_bits(show.t_ko, &want["t_ko"], &format!("{name}: t_ko"));
        same_bits(HITSTOP, &want["hitstop"], &format!("{name}: hitstop"));
        same_bits(show.t_hs, &want["t_hs"], &format!("{name}: t_hs"));
        same_bits(show.t_solo, &want["t_solo"], &format!("{name}: t_solo"));
        same_bits(CAP_GAP, &want["cap_gap"], &format!("{name}: cap_gap"));
        same_bits(
            show.grid.t_reassemble,
            &want["t_reassemble"],
            &format!("{name}: t_reassemble"),
        );

        assert!(
            want["cap_title"].as_array().unwrap().is_empty(),
            "{name}: the kit has a title caption"
        );
        caps_equal(
            &show.caps_ability[0],
            &want["cap_ability"][0],
            &format!("{name}: caps_ability[0]"),
        );
        caps_equal(
            &show.caps_ability[1],
            &want["cap_ability"][1],
            &format!("{name}: caps_ability[1]"),
        );
        caps_equal(&show.cap_ko, &want["cap_ko"], &format!("{name}: cap_ko"));
        caps_equal(&show.cap_wins, &want["cap_wins"], &format!("{name}: cap_wins"));
        caps_equal(&show.cap_comment, &want["cap_comment"], &format!("{name}: cap_comment"));
        caps_equal(&show.captions, &want["all"], &format!("{name}: captions"));

        // SOLO_T: the winner's solo notes from T_SOLO, as picture.py lays them out
        let solo = &ff.music.solos[&show.names[show.win]];
        let got = show.solo_times(solo);
        let want_solo = want["solo_t"].as_array().unwrap();
        assert_eq!(got.len(), want_solo.len(), "{name}: solo notes");
        for (i, (g, w)) in got.iter().zip(want_solo).enumerate() {
            same_bits(*g, w, &format!("{name}: solo_t[{i}]"));
        }
    }
}

#[test]
fn captions_equal_story_captions_for_every_recording() {
    let v = vectors();
    for name in REFERENCE.iter().chain(&["no_ko.bfrec"]) {
        let rec = recording(name);
        let got = captions(&rec.events, CAP_GAP);
        captions_equal(
            &got,
            &v["recordings"][*name]["captions"],
            &format!("{name}: captions()"),
        );
    }
}

#[test]
fn a_real_fight_with_no_ko_is_an_error_not_a_panic() {
    let rec = recording("no_ko.bfrec");
    assert_eq!((rec.winner, rec.t_ko), (None, None));
    let v = vectors();
    assert!(
        v["recordings"]["no_ko.bfrec"]["error"]
            .as_str()
            .unwrap()
            .starts_with("TypeError")
    );
    let err = Show::new(&rec, &fight_file(rec.seed)).unwrap_err();
    assert_eq!(err, ShowError::NoKo);
    assert!(err.to_string().contains("no K.O."), "{err}");
}

#[test]
fn a_cleared_recording_is_an_error_not_a_panic() {
    let full = recording("ep392.bfrec");
    let ff = fight_file(full.seed);
    let mut both = full.clone();
    both.winner = None;
    both.t_ko = None;
    assert_eq!(Show::new(&both, &ff).unwrap_err(), ShowError::NoKo);
    let mut no_winner = full.clone();
    no_winner.winner = None;
    assert_eq!(Show::new(&no_winner, &ff).unwrap_err(), ShowError::NoKo);
    let mut no_time = full.clone();
    no_time.t_ko = None;
    assert_eq!(Show::new(&no_time, &ff).unwrap_err(), ShowError::NoKo);
}

#[test]
fn other_bad_inputs_are_errors_naming_the_cause() {
    let full = recording("ep392.bfrec");
    let ff = fight_file(full.seed);

    let mut odd = full.clone();
    odd.winner = Some(2);
    let err = Show::new(&odd, &ff).unwrap_err();
    assert_eq!(err, ShowError::Winner(2));
    assert!(err.to_string().contains("winner 2"), "{err}");

    let mut three = full.clone();
    three.names.push("OBOE".into());
    let err = Show::new(&three, &ff).unwrap_err();
    assert!(matches!(err, ShowError::Names { .. }), "{err:?}");
    assert!(err.to_string().contains("OBOE"), "{err}");

    let mut swapped = ff.clone();
    swapped.fighters.reverse();
    let err = Show::new(&full, &swapped).unwrap_err();
    assert!(matches!(err, ShowError::Names { .. }), "{err:?}");
    assert!(err.to_string().contains("TROMBONE"), "{err}");

    // the first kept ability event (FLUTTER TONGUE) with element 5 gone, or naming no fighter
    let first = full
        .events
        .iter()
        .position(|e| e.kind == "ability" && e.fields.get(3) == Some(&EvValue::Int(1)))
        .unwrap();
    let mut short = full.clone();
    short.events[first].fields.truncate(2);
    let err = Show::new(&short, &ff).unwrap_err();
    assert!(
        matches!(err, ShowError::Event { index, .. } if index == first),
        "{err:?}"
    );
    assert!(err.to_string().contains(&format!("event {first}")), "{err}");
    let mut nobody = full.clone();
    nobody.events[first].fields[0] = EvValue::Int(5);
    let err = Show::new(&nobody, &ff).unwrap_err();
    assert!(
        matches!(err, ShowError::Event { index, .. } if index == first),
        "{err:?}"
    );
}

#[test]
fn captions_keep_first_uses_at_least_gap_apart() {
    use bf_core::event::Event;
    let ab = |t: f64, who: i64, name: &str, first: EvValue| Event {
        t,
        kind: "ability".into(),
        fields: vec![
            EvValue::Int(who),
            EvValue::Str(name.into()),
            EvValue::Str("x".into()),
            first,
        ],
    };
    let hit = Event {
        t: 5.0,
        kind: "hit".into(),
        fields: vec![EvValue::Int(0)],
    };
    let events = vec![
        ab(4.0, 0, "A", EvValue::Int(1)),
        hit,
        ab(6.0, 1, "B", EvValue::Int(1)), // 2 s after A: dropped
        ab(8.0, 1, "C", EvValue::Int(0)), // not a first use
        ab(8.0, 1, "D", EvValue::Bool(true)),
        ab(11.9, 0, "E", EvValue::Int(1)), // 3.9 s after D
        ab(12.0, 0, "F", EvValue::Int(1)), // exactly 4 s after D: kept
    ];
    let got = captions(&events, 4.0);
    assert_eq!(
        got,
        vec![
            (4.0, 0, "A".to_string()),
            (8.0, 1, "D".to_string()),
            (12.0, 0, "F".to_string())
        ]
    );
    assert!(captions(&[], 4.0).is_empty());
}
