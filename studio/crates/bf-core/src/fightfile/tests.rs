//! The fight file's tests: the example episode, round trips, every error text, the search key.

use super::*;

const EPISODE: &str = include_str!("../../../../fights/flute_vs_trombone.toml");

/// The example as milestones 1 and 2 wrote it (format 1), kept as it was.
const FORMAT1: &str = include_str!("flute_vs_trombone.format1.toml");

fn episode() -> FightFile {
    FightFile::parse(EPISODE).expect("the example fight file parses")
}

/// The example with `old` replaced by `new` (which must be there exactly once).
fn with(old: &str, new: &str) -> String {
    assert_eq!(EPISODE.matches(old).count(), 1, "{old:?} is in the example once");
    EPISODE.replace(old, new)
}

/// The error `text` gives, as one line.
fn parse_err(text: &str) -> String {
    let e = FightFile::parse(text)
        .expect_err("this fight file is rejected")
        .to_string();
    assert!(!e.contains('\n'), "one line: {e:?}");
    e
}

/// The roster as the content registry reports it, for `validate`.
fn names(n: &str) -> NameStatus {
    match n {
        "FLUTE" | "TROMBONE" | "TUBA" => NameStatus::Known,
        "GUITAR" | "SAX" => NameStatus::Retired,
        _ => NameStatus::Unknown,
    }
}

fn validate_err(ff: &FightFile) -> String {
    let e = ff
        .validate(&names)
        .expect_err("this fight file is rejected")
        .to_string();
    assert!(!e.contains('\n'), "one line: {e:?}");
    e
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parses_the_episode() {
    let ff = episode();
    assert_eq!(ff.format, FIGHT_FORMAT);
    assert_eq!(ff.app_version, "0.1.0");
    assert_eq!(ff.fighters, strings(&["FLUTE", "TROMBONE"]));
    assert_eq!(ff.arena, "ring");
    assert_eq!(ff.handicap.len(), 2);
    assert_eq!(ff.handicap[0].to_bits(), 2.0f64.to_bits());
    assert_eq!(ff.handicap[1].to_bits(), 0.40f64.to_bits());
    assert_eq!(ff.dmg_scale.to_bits(), 1.0f64.to_bits());
    assert_eq!(ff.pair_rules.len(), 1);
    assert_eq!(ff.pair_rules["swat_cobras"], Value::Bool(true));
    assert_eq!(ff.tempo_bpm.to_bits(), 132.0f64.to_bits());
    assert_eq!(ff.seed, Some(392));
    assert_eq!(ff.series_tag.as_deref(), Some("BALLFIGHT #15"));
    let m = &ff.music;
    assert_eq!(m.theme, "big_top");
    assert_eq!(m.arrangement, "classic");
    assert_eq!(m.key, "E");
    assert_eq!(m.scale, "harmonic minor");
    assert_eq!(m.chords_intro, strings(&["Em", "B7"]));
    assert_eq!(m.chord_loop, strings(&["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]));
    assert_eq!(m.music_seed, 1);
    assert_eq!(m.roles.len(), 2);
    assert_eq!(m.roles["TROMBONE"], "bass");
    assert_eq!(m.roles["FLUTE"], "melody");
    assert_eq!(m.lines.len(), 1);
    let line = &m.lines["FLUTE"];
    assert_eq!(line.len(), 64);
    assert_eq!(&line[..8], &[76, 79, 83, 79, 76, 75, 76, 71].map(Some));
    assert_eq!(&line[56..], &[78, 75, 71, 75, 78, 81, 83, 75].map(Some));
    assert_eq!(line.iter().map(|n| n.unwrap()).sum::<i32>(), 5087); // Python: sum(LINES["FLUTE"])
    let flute = &m.solos["FLUTE"];
    assert_eq!(flute.len(), 15);
    assert_eq!(flute[0], (0.0, 83, 0.5));
    assert_eq!(flute[14], (6.0, 76, 1.5));
    let trombone = &m.solos["TROMBONE"];
    assert_eq!(trombone.len(), 22);
    assert_eq!(trombone[9], (5.0, 60, 0.75));
    assert_eq!(trombone[21], (11.0, 45, 1.0));
    assert_eq!(m.title_hits["FLUTE"], vec![76, 79, 83, 84, 86, 87, 88]);
    assert_eq!(m.title_hits["TROMBONE"], vec![40, 52]);
    assert_eq!(ff.picture.shard_seed, 271);
    assert_eq!(ff.picture.fx_seed, Some(41));
    assert_eq!(ff.fx_seed(), 41);
    let c = &ff.captions;
    assert_eq!(
        c.caption,
        "The front row vs the back row 🪈🎺 Comment the next match-up 👇"
    );
    assert!(c.instagram.starts_with("#ballfight #flute") && c.instagram.ends_with("#satisfying #music"));
    assert_eq!(
        c.tiktok,
        "#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music"
    );
    assert_eq!(c.shorts.len(), 3);
    assert_eq!(c.shorts[0], "Flute vs Trombone: the front row fights back");
    assert!(m.chords_tail.is_empty() && m.mix.is_empty() && m.theme_options.is_empty());
    let p = &ff.picture;
    assert!(p.title_lines.is_empty() && p.title_balls.is_none() && p.middle.is_none());
    assert_eq!(ff.grid().n_bars, 34);
    assert_eq!(ff.chords(34), FightFile::parse(FORMAT1).unwrap().chords(34));
    assert_eq!(ff.validate(&names), Ok(()));
}

#[test]
fn to_toml_round_trips() {
    let ff = episode();
    let text = ff.to_toml();
    // the example is written in the canonical layout, so it comes back byte for byte
    assert_eq!(text, EPISODE);
    let back = FightFile::parse(&text).expect("to_toml output parses");
    assert_eq!(back, ff);
    assert_eq!(back.to_toml(), text);
}

#[test]
fn floats_round_trip_bit_for_bit() {
    let mut ff = episode();
    let odd = [
        0.1 + 0.2,
        1.0 / 3.0,
        5e-324,
        2.2250738585072014e-308,
        1.7976931348623157e308,
        1e16,
        123456789012345680.0,
        1e-7,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for (i, x) in odd.iter().enumerate() {
        ff.pair_rules.insert(format!("odd_{i:02}"), Value::Float(*x));
    }
    ff.pair_rules.insert(
        "list".into(),
        Value::List(vec![
            Value::Int(-7),
            Value::Str("a \"q\" \\ \t é".into()),
            Value::Bool(false),
        ]),
    );
    ff.handicap = vec![0.1 + 0.2, 1.0 / 3.0];
    ff.dmg_scale = 0.7000000000000001;
    ff.tempo_bpm = 133.7704918032787;
    ff.music.solos.get_mut("FLUTE").unwrap()[3] = (1e-7, 81, 2.0 / 3.0);
    let back = FightFile::parse(&ff.to_toml()).expect("parses");
    assert_eq!(back, ff);
    for (i, x) in odd.iter().enumerate() {
        match &back.pair_rules[&format!("odd_{i:02}")] {
            Value::Float(y) => assert_eq!(y.to_bits(), x.to_bits(), "{x:?}"),
            v => panic!("{v:?}"),
        }
    }
    assert_eq!(back.handicap[0].to_bits(), (0.1f64 + 0.2).to_bits());
    assert_eq!(back.to_toml(), ff.to_toml());
}

#[test]
fn a_whole_number_is_accepted_where_a_decimal_is_expected() {
    let ff = FightFile::parse(&with("tempo_bpm = 132.0", "tempo_bpm = 132")).expect("parses");
    assert_eq!(ff.tempo_bpm.to_bits(), 132.0f64.to_bits());
    let ff = FightFile::parse(&with("handicap = [2.0, 0.4]", "handicap = [2, 0.4]")).expect("parses");
    assert_eq!(ff.handicap[0].to_bits(), 2.0f64.to_bits());
    let ff = FightFile::parse(&with("[0.0, 83, 0.5]", "[0, 83, 0.5]")).expect("parses");
    assert_eq!(ff.music.solos["FLUTE"][0], (0.0, 83, 0.5));
    // the canonical text writes it back as a decimal
    assert_eq!(ff.to_toml(), EPISODE);
}

#[test]
fn optional_settings_take_the_series_defaults() {
    let text = [
        "format = 2",
        "app_version = \"0.1.0\"",
        "fighters = [\"FLUTE\", \"TROMBONE\"]",
        "tempo_bpm = 132",
        "[music]",
        "theme = \"big_top\"",
        "key = \"E\"",
        "chords_intro = [\"Em\", \"B7\"]",
        "chord_loop = [\"Em\", \"Am\", \"B7\", \"Em\", \"C\", \"Am\", \"B7\", \"B7\"]",
        "music_seed = 7",
    ]
    .join("\n");
    let ff = FightFile::parse(&text).expect("parses");
    assert_eq!(ff.arena, "ring");
    assert_eq!(ff.handicap, vec![1.0, 1.0]);
    assert_eq!(ff.dmg_scale, 1.0);
    assert!(ff.pair_rules.is_empty());
    assert_eq!(ff.seed, None);
    assert_eq!(ff.series_tag, None);
    assert_eq!(ff.music.arrangement, "classic");
    assert_eq!(ff.music.scale, "minor");
    assert!(ff.music.roles.is_empty() && ff.music.lines.is_empty());
    assert!(ff.music.solos.is_empty() && ff.music.title_hits.is_empty());
    assert_eq!(ff.picture.shard_seed, 271);
    assert_eq!(ff.picture.fx_seed, None);
    // Python: sum(map(ord, 'FLUTE' + 'TROMBONE')) % 1000
    assert_eq!(ff.fx_seed(), 998);
    assert_eq!(ff.captions.caption, "");
    assert!(ff.captions.shorts.is_empty());
    // None stays None through the canonical text
    assert_eq!(FightFile::parse(&ff.to_toml()), Ok(ff));
}

#[test]
fn search_key_ignores_music() {
    let ff = episode();
    let key = ff.search_key(&[false, false]);
    assert!(!key.is_empty());
    let mut m = ff.clone();
    m.music.theme = "noir_jazz".into();
    m.music.key = "D".into();
    m.music.music_seed = 99;
    m.music.chord_loop[4] = "Am".into();
    m.music.roles.clear();
    m.seed = Some(7);
    m.captions.caption = "other".into();
    m.picture.fx_seed = None;
    m.series_tag = None;
    assert_eq!(
        m.search_key(&[false, false]),
        key,
        "music, seed, captions and picture do not feed the fight"
    );

    let changed = |f: &dyn Fn(&mut FightFile)| {
        let mut c = ff.clone();
        f(&mut c);
        c.search_key(&[false, false])
    };
    assert_ne!(changed(&|c| c.handicap[1] = 0.41), key);
    assert_ne!(changed(&|c| c.dmg_scale = 1.1), key);
    assert_ne!(changed(&|c| c.tempo_bpm = 120.0), key);
    assert_ne!(changed(&|c| c.fighters.reverse()), key, "sides");
    assert_ne!(changed(&|c| c.fighters[1] = "TUBA".into()), key);
    assert_ne!(changed(&|c| c.arena = "cage".into()), key);
    assert_ne!(
        changed(&|c| _ = c.pair_rules.insert("swat_cobras".into(), Value::Bool(false))),
        key
    );
    assert_ne!(changed(&|c| c.pair_rules.clear()), key);
    assert_ne!(changed(&|c| _ = c.pair_rules.insert("x".into(), Value::Int(1))), key);
    // an Int and a Float that are numerically equal are different rules
    let i = changed(&|c| _ = c.pair_rules.insert("n".into(), Value::Int(1)));
    let f = changed(&|c| _ = c.pair_rules.insert("n".into(), Value::Float(1.0)));
    assert_ne!(i, f);

    // a fighter whose notes are its weapon: the chord loop, key and music seed feed the fight
    let fed = ff.search_key(&[true, false]);
    assert_ne!(fed, key);
    let mut c = ff.clone();
    c.music.chord_loop[4] = "Am".into();
    assert_ne!(c.search_key(&[true, false]), fed);
    let mut c = ff.clone();
    c.music.key = "D".into();
    assert_ne!(c.search_key(&[true, false]), fed);
    let mut c = ff.clone();
    c.music.music_seed = 2;
    assert_ne!(c.search_key(&[true, false]), fed);
    let mut c = ff.clone();
    c.music.theme = "noir_jazz".into();
    assert_eq!(c.search_key(&[true, false]), fed, "the theme never feeds the fight");
}

#[test]
fn search_key_is_a_fixed_string() {
    // the same inputs give these bytes on any PC and any run
    assert_eq!(
        episode().search_key(&[false, false]),
        "fight-key 2;fighters=[5:FLUTE,8:TROMBONE];arena=4:ring;handicap=[4000000000000000,3fd999999999999a];\
         dmg_scale=3ff0000000000000;tempo_bpm=4060800000000000;pair_rules=[11:swat_cobras=b:true];music=none"
    );
    let fed = episode().search_key(&[false, true]);
    assert!(fed.ends_with(
        "music=[key=1:E;scale=14:harmonic minor;chords_intro=[2:Em,2:B7];\
         chord_loop=[2:Em,2:Am,2:B7,2:Em,1:C,2:Am,2:B7,2:B7];chords_tail=[];music_seed=1;lines=[]]"
    ));
    // Version 1 of the key, which milestone 2's shortlists were cached under, wrote the same string with
    // `fight-key 1` at its start and, when music fed the fight, no `chords_tail`: the version 2 key of a fight
    // is a new key (its shortlist is searched once more), and two fights share a version 2 key exactly when they
    // shared a version 1 key.
    let v1 = |key: &str| {
        key.replacen("fight-key 2;", "fight-key 1;", 1)
            .replacen(";chords_tail=[]", "", 1)
    };
    assert_eq!(
        v1(&episode().search_key(&[false, false])),
        "fight-key 1;fighters=[5:FLUTE,8:TROMBONE];arena=4:ring;handicap=[4000000000000000,3fd999999999999a];\
         dmg_scale=3ff0000000000000;tempo_bpm=4060800000000000;pair_rules=[11:swat_cobras=b:true];music=none"
    );
    assert!(v1(&fed).ends_with("chord_loop=[2:Em,2:Am,2:B7,2:Em,1:C,2:Am,2:B7,2:B7];music_seed=1;lines=[]]"));
    // separators inside names cannot fake another key
    let mut a = episode();
    a.fighters = strings(&["A,1:B", "C"]);
    let mut b = episode();
    b.fighters = strings(&["A", "B,1:C"]);
    assert_ne!(a.search_key(&[false, false]), b.search_key(&[false, false]));
}

#[test]
fn unknown_fighter() {
    let mut ff = episode();
    ff.fighters[1] = "KAZOO".into();
    assert_eq!(validate_err(&ff), r#"fighters[1]: "KAZOO" is not in the roster"#);
    // the file names KAZOO everywhere TROMBONE was (its music too), so only the roster can refuse it
    let text = EPISODE.replace("TROMBONE", "KAZOO");
    let parsed = FightFile::parse(&text).expect("the file itself is well formed");
    assert_eq!(validate_err(&parsed), r#"fighters[1]: "KAZOO" is not in the roster"#);
}

#[test]
fn retired_fighter() {
    let text = with(
        r#"fighters = ["FLUTE", "TROMBONE"]"#,
        r#"fighters = ["SAX", "TROMBONE"]"#,
    );
    assert_eq!(parse_err(&text), r#"fighters[0]: "SAX" is retired"#);
    let text = with(
        r#"fighters = ["FLUTE", "TROMBONE"]"#,
        r#"fighters = ["FLUTE", "GUITAR"]"#,
    );
    assert_eq!(parse_err(&text), r#"fighters[1]: "GUITAR" is retired"#);
    let mut ff = episode();
    ff.fighters[0] = "SAX".into();
    assert_eq!(validate_err(&ff), r#"fighters[0]: "SAX" is retired"#);
    // the registry's word counts too
    let mut ff = episode();
    ff.fighters[1] = "OLD".into();
    let e = ff.validate(&|n| if n == "OLD" { NameStatus::Retired } else { names(n) });
    assert_eq!(e.unwrap_err().to_string(), r#"fighters[1]: "OLD" is retired"#);
}

#[test]
fn the_same_fighter_twice() {
    let text = with(
        r#"fighters = ["FLUTE", "TROMBONE"]"#,
        r#"fighters = ["FLUTE", "FLUTE"]"#,
    );
    assert_eq!(parse_err(&text), r#"fighters[1]: "FLUTE" is already fighters[0]"#);
}

#[test]
fn one_fighter_or_three() {
    let text = with(r#"fighters = ["FLUTE", "TROMBONE"]"#, r#"fighters = ["FLUTE"]"#);
    assert_eq!(parse_err(&text), "fighters: 2 fighters needed, found 1");
    let text = with(
        r#"fighters = ["FLUTE", "TROMBONE"]"#,
        r#"fighters = ["FLUTE", "TROMBONE", "TUBA"]"#,
    );
    assert_eq!(parse_err(&text), "fighters: 2 fighters needed, found 3");
}

#[test]
fn format_newer_than_the_app() {
    let text = with("format = 2", "format = 3");
    assert_eq!(
        parse_err(&text),
        "format: this app reads fight files up to format 2 (this file is format 3)"
    );
    // a newer file is refused for its format, before any setting this app does not know
    let text = with("format = 2", "format = 7\nwings = true");
    assert_eq!(
        parse_err(&text),
        "format: this app reads fight files up to format 2 (this file is format 7)"
    );
    // and one built in code
    let mut ff = episode();
    ff.format = 3;
    assert_eq!(
        validate_err(&ff),
        "format: this app reads fight files up to format 2 (this file is format 3)"
    );
}

#[test]
fn format_zero_or_missing() {
    assert_eq!(
        parse_err(&with("format = 2", "format = 0")),
        "format: 0 is not a fight file format (they start at 1)"
    );
    assert_eq!(
        parse_err(&with("format = 2\n", "")),
        "format: missing (a fight file starts with format = 2)"
    );
    assert_eq!(
        parse_err(&with("format = 2", "format = -1")),
        "format: must be a whole number from 0 up, found -1"
    );
    assert_eq!(
        parse_err(&with("format = 2", "format = \"1\"")),
        "format: must be a whole number, found a text"
    );
}

#[test]
fn a_seven_chord_loop() {
    let text = with(
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7"]"#,
    );
    assert_eq!(parse_err(&text), "music.chord_loop: 4 or 8 chords needed, found 7");
}

#[test]
fn an_unparseable_chord() {
    let text = with(
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
        r#"chord_loop = ["Em", "Am", "H7", "Em", "C", "Am", "B7", "B7"]"#,
    );
    assert_eq!(
        parse_err(&text),
        r#"music.chord_loop[2]: "H7" is not a chord name (a root such as E, F# or Bb, then a quality)"#
    );
    let text = with(r#"chords_intro = ["Em", "B7"]"#, r#"chords_intro = ["Em", "B13"]"#);
    assert_eq!(
        parse_err(&text),
        r#"music.chords_intro[1]: "B13" is not a chord name (a root such as E, F# or Bb, then a quality)"#
    );
    let text = with(r#"key = "E""#, r#"key = "X""#);
    assert_eq!(
        parse_err(&text),
        r#"music.key: "X" is not a note name (A to G, then # or b)"#
    );
    let text = with(r#"scale = "harmonic minor""#, r#"scale = "harmonic""#);
    assert_eq!(parse_err(&text), r#"music.scale: "harmonic" is not a known scale"#);
}

#[test]
fn an_empty_file() {
    assert_eq!(parse_err(""), "format: missing (a fight file starts with format = 2)");
    assert_eq!(
        parse_err("\n\n  \n"),
        "format: missing (a fight file starts with format = 2)"
    );
}

#[test]
fn text_that_is_not_toml() {
    let e = parse_err("format = 1\nthis is not a fight file\n");
    assert!(e.starts_with("line 2: not TOML: "), "{e}");
    let e = parse_err("{\"format\": 1}");
    assert!(e.starts_with("line 1: not TOML: "), "{e}");
    let e = parse_err(&with("[captions]", "[captions"));
    assert!(e.contains(": not TOML: "), "{e}");
}

#[test]
fn a_wrong_type() {
    assert_eq!(
        parse_err(&with("tempo_bpm = 132.0", "tempo_bpm = \"fast\"")),
        "tempo_bpm: must be a number, found a text"
    );
    assert_eq!(
        parse_err(&with("seed = 392", "seed = 392.5")),
        "seed: must be a whole number, found a decimal"
    );
    assert_eq!(
        parse_err(&with("fighters = [\"FLUTE\", \"TROMBONE\"]", "fighters = \"FLUTE\"")),
        "fighters: must be a list, found a text"
    );
    assert_eq!(
        parse_err(&with("swat_cobras = true", "swat_cobras = { on = true }")),
        "pair_rules.swat_cobras: must be true or false, a number, a text or a list, found a table"
    );
    assert_eq!(
        parse_err(&with("[0.0, 83, 0.5]", "[0.0, 83]")),
        "music.solos.FLUTE[0]: must be [beat, note, beats]"
    );
    assert_eq!(
        parse_err(&with(
            "76, 79, 83, 79, 76, 75, 76, 71,",
            "76, 79, 83, 79, 76, 75, 76, 7.1,"
        )),
        "music.lines.FLUTE[7]: must be a MIDI note (0 to 127) or \"rest\", found a decimal"
    );
}

#[test]
fn a_rest_in_a_line() {
    let ff = FightFile::parse(&with(
        "76, 79, 83, 79, 76, 75, 76, 71,",
        "76, \"rest\", 83, 79, 76, 75, 76, 71,",
    ))
    .expect("parses");
    assert_eq!(ff.music.lines["FLUTE"][1], None);
    assert_eq!(FightFile::parse(&ff.to_toml()), Ok(ff));
}

#[test]
fn a_negative_handicap() {
    let text = with("handicap = [2.0, 0.4]", "handicap = [2.0, -0.4]");
    assert_eq!(parse_err(&text), "handicap[1]: must be above 0, found -0.4");
    let text = with("handicap = [2.0, 0.4]", "handicap = [0, 0.4]");
    assert_eq!(parse_err(&text), "handicap[0]: must be above 0, found 0.0");
    let text = with("handicap = [2.0, 0.4]", "handicap = [nan, 0.4]");
    assert_eq!(parse_err(&text), "handicap[0]: must be above 0, found NaN");
}

#[test]
fn a_handicap_of_the_wrong_length() {
    let text = with("handicap = [2.0, 0.4]", "handicap = [2.0, 0.4, 1.0]");
    assert_eq!(
        parse_err(&text),
        "handicap: 2 numbers needed (one per fighter), found 3"
    );
    let text = with("handicap = [2.0, 0.4]", "handicap = [2.0]");
    assert_eq!(
        parse_err(&text),
        "handicap: 2 numbers needed (one per fighter), found 1"
    );
}

#[test]
fn unknown_settings_are_named() {
    assert_eq!(
        parse_err(&with("seed = 392", "seed = 392\ntempo = 120")),
        "tempo: unknown setting (check the spelling)"
    );
    assert_eq!(
        parse_err(&with("music_seed = 1", "music_seed = 1\nmusic_sead = 2")),
        "music.music_sead: unknown setting (check the spelling)"
    );
    assert_eq!(
        parse_err(&with("fx_seed = 41", "fx_seed = 41\nfx_sed = 41")),
        "picture.fx_sed: unknown setting (check the spelling)"
    );
    assert_eq!(
        parse_err(&with("[captions]", "[captions]\ntwitter = \"x\"")),
        "captions.twitter: unknown setting (check the spelling)"
    );
    assert_eq!(
        parse_err(&with("[picture]", "[pictures]")),
        "pictures: unknown setting (check the spelling)"
    );
}

#[test]
fn music_for_someone_not_fighting() {
    let text = with("FLUTE = \"melody\"", "TUBA = \"melody\"");
    assert_eq!(parse_err(&text), "music.roles.TUBA: TUBA is not one of the fighters");
    let text = with("TROMBONE = [40, 52]", "TROMBONE = [40, 152]");
    assert_eq!(
        parse_err(&text),
        "music.title_hits.TROMBONE[1]: 152 is not a MIDI note (0 to 127)"
    );
}

#[test]
fn other_values_out_of_range() {
    assert_eq!(
        parse_err(&with("tempo_bpm = 132.0", "tempo_bpm = 1e300")),
        "tempo_bpm: must be from 30 to 300, found 1e300"
    );
    assert_eq!(
        parse_err(&with("dmg_scale = 1.0", "dmg_scale = -1.0")),
        "dmg_scale: must be above 0, found -1.0"
    );
    assert_eq!(
        parse_err(&with("arena = \"ring\"", "arena = \"cage\"")),
        r#"arena: "cage" is not an arena (this app has: ring)"#
    );
    assert_eq!(
        parse_err(&with("arrangement = \"classic\"", "arrangement = \"remix\"")),
        r#"music.arrangement: "remix" is not an arrangement (this app has: classic)"#
    );
    assert_eq!(
        parse_err(&with(r#"chords_intro = ["Em", "B7"]"#, r#"chords_intro = ["Em"]"#)),
        "music.chords_intro: 2 chords needed (the title card and the count-in), found 1"
    );
    assert_eq!(
        parse_err(&with("seed = 392", "seed = -392")),
        "seed: must be a whole number from 0 up, found -392"
    );
    assert_eq!(parse_err(&with("theme = \"big_top\"\n", "")), "music.theme: missing");
    assert_eq!(
        parse_err(&with("[music]", "[musik]")),
        "musik: unknown setting (check the spelling)"
    );
}

#[test]
fn no_input_text_panics() {
    // every prefix of the example, and of a few broken files, gives a result, never a panic
    let mut cut: Vec<usize> = EPISODE.char_indices().map(|(i, _)| i).collect();
    cut.push(EPISODE.len());
    for i in cut {
        let _ = FightFile::parse(&EPISODE[..i]);
    }
    for text in [
        "\u{0}",
        "format = 1\n[[music]]\n",
        "format = 9999999999999",
        "format = 1\nfighters = [[[]]]",
        "format = 2\n[music]\nmix = { a = [1] }\n",
        "format = 2\n[picture]\nmiddle = [[], []]\ntitle_balls = [[1, 2], 3]\n",
    ] {
        let _ = FightFile::parse(text);
    }
    let mut cut: Vec<usize> = FORMAT1.char_indices().map(|(i, _)| i).collect();
    cut.push(FORMAT1.len());
    for i in cut {
        let _ = FightFile::parse(&FORMAT1[..i]);
    }
}

// ---------------------------------------------------------------- format 2

/// The format 1 text with `old` replaced by `new` (which must be there exactly once).
fn with1(old: &str, new: &str) -> String {
    assert_eq!(
        FORMAT1.matches(old).count(),
        1,
        "{old:?} is in the format 1 example once"
    );
    FORMAT1.replace(old, new)
}

/// The rematch's settings (`projects/ballfight_horn_vs_trombone_rematch/episode.py`) as a format 2 text, from the
/// example: an 8-chord loop with a 5-bar tail, `MIX`, `TRAILER`, `MIDDLE`, and a line of an extra name.
fn format2_text() -> String {
    with(
        r#"chords_tail = []"#,
        r#"chords_tail = ["Dm", "Bb", "Gm", "A", "A"]"#,
    )
    .replace(
        "[music.mix]\n",
        "[music.mix]\n\"bass clarinet\" = [1.95, 0.25]\ntrombone = [2.3, 0.2]\n",
    )
    .replace("[music.theme_options]\n", "[music.theme_options]\nplugged = \"FLUTE\"\nsequel = true\n")
    .replace(
        "fx_seed = 41\n",
        "fx_seed = 41\ntitle_balls = [[74.0, 250.0, 54.0], [282.0, 396.0, 56.0]]\nmiddle = [\"REMATCH\", \"divider\"]\n",
    )
    .replace("[picture.title_lines]\n", "[picture.title_lines]\nFLUTE = [\"SILVER\", \"FLUTE\"]\n")
    .replace(
        "]\n\n[music.solos]",
        "]\n\"FLUTE THEME\" = [\n    76, 79, 83, 79, 76, 75, 76, 71,\n]\n\n[music.solos]",
    )
}

#[test]
fn a_format_1_file_reads_to_the_same_fight() {
    // The example as milestone 1 wrote it and as it is written now are one fight.
    let old = FightFile::parse(FORMAT1).expect("the format 1 example parses");
    assert_eq!(old, episode());
    // A format 1 file has no tail, no mix overrides, no theme options and no title opt-ins, and it is written
    // back as format 2 with the same grid and chords: that text is the format 2 example byte for byte.
    assert_eq!(old.format, FIGHT_FORMAT);
    assert!(old.music.chords_tail.is_empty() && old.music.mix.is_empty() && old.music.theme_options.is_empty());
    assert_eq!(old.to_toml(), EPISODE);
    let again = FightFile::parse(&old.to_toml()).unwrap();
    assert_eq!(again.grid(), old.grid());
    assert_eq!(again.chords(again.grid().n_bars), old.chords(old.grid().n_bars));
    // Its search key is the format 2 example's.
    for feeds in [[false, false], [true, false], [false, true]] {
        assert_eq!(old.search_key(&feeds), episode().search_key(&feeds));
    }
    // Reading writes nothing: the text is a `&str`, and the reader has no file to touch. Only the canonical
    // writer makes format 2 text, and only when it is asked to.
    assert!(FORMAT1.starts_with("format = 1\n"));
}

#[test]
fn a_format_1_file_cannot_hold_format_2_settings() {
    let text = with1("chord_loop = [", "chords_tail = []\nchord_loop = [");
    assert_eq!(
        parse_err(&text),
        "music.chords_tail: not a setting of format 1 fight files (it came in format 2: a file with it says format \
         = 2)"
    );
    let text = with1("[picture]\n", "[picture]\nmiddle = [\"REMATCH\", \"divider\"]\n");
    assert_eq!(
        parse_err(&text),
        "picture.middle: not a setting of format 1 fight files (it came in format 2: a file with it says format = \
         2)"
    );
    let text = with1("[picture]\n", "[music.mix]\ntrombone = [2.3, 0.2]\n\n[picture]\n");
    assert!(parse_err(&text).starts_with("music.mix: not a setting of format 1"));
    // A format 1 file with a 4-chord loop is fine: the loop's length is checked by the same rule in both.
    let text = with1(
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
        r#"chord_loop = ["Em", "Am", "B7", "B7"]"#,
    );
    assert_eq!(FightFile::parse(&text).unwrap().music.chord_loop.len(), 4);
}

#[test]
fn format_2_settings_read_and_write_back() {
    let text = format2_text();
    let ff = FightFile::parse(&text).expect("the format 2 text parses");
    let m = &ff.music;
    assert_eq!(m.chords_tail, strings(&["Dm", "Bb", "Gm", "A", "A"]));
    assert_eq!(m.mix["trombone"], (2.3, 0.2));
    assert_eq!(m.mix["bass clarinet"], (1.95, 0.25));
    assert_eq!(m.theme_options["sequel"], OptionValue::Bool(true));
    assert_eq!(m.theme_options["plugged"], OptionValue::Str("FLUTE".into()));
    assert_eq!(m.lines["FLUTE THEME"].len(), 8);
    let p = &ff.picture;
    assert_eq!(p.title_balls, Some([[74.0, 250.0, 54.0], [282.0, 396.0, 56.0]]));
    assert_eq!(p.middle, Some(("REMATCH".to_string(), Middle::Divider)));
    assert_eq!(p.title_lines["FLUTE"], ("SILVER".to_string(), "FLUTE".to_string()));
    // the canonical text, and back
    assert_eq!(ff.to_toml(), text);
    assert_eq!(FightFile::parse(&ff.to_toml()), Ok(ff.clone()));
    // a turned word instead of the divider
    let mut turned = ff.clone();
    turned.picture.middle = Some(("BOUT 2".into(), Middle::Degrees(-12.5)));
    assert!(turned.to_toml().contains("middle = [\"BOUT 2\", -12.5]\n"));
    assert_eq!(FightFile::parse(&turned.to_toml()), Ok(turned));
    // the tail sits on the last bars: 34 bars are the intro, 27 bars of the loop, the tail
    let chords = ff.chords(34);
    assert_eq!(chords.len(), 34);
    assert_eq!(&chords[29..], &strings(&["Dm", "Bb", "Gm", "A", "A"])[..]);
    assert_eq!(&chords[..2], &strings(&["Em", "B7"])[..]);
    assert_eq!(chords[2 + 24..29], ff.music.chord_loop[..3]);
}

#[test]
fn format_2_settings_are_checked() {
    let text = format2_text();
    let bad = |old: &str, new: &str| {
        assert_eq!(text.matches(old).count(), 1, "{old:?}");
        parse_err(&text.replace(old, new))
    };
    assert_eq!(
        bad(
            r#"chords_tail = ["Dm", "Bb", "Gm", "A", "A"]"#,
            r#"chords_tail = ["Dm", "Bb", "Gm", "A", "A", "A", "A", "A", "A"]"#
        ),
        "music.chords_tail: at most 8 chords (the bars after the last whole loop), found 9"
    );
    assert_eq!(
        bad(r#""Gm", "A", "A"]"#, r#""Gm", "A", "Q"]"#),
        r#"music.chords_tail[4]: "Q" is not a chord name (a root such as E, F# or Bb, then a quality)"#
    );
    assert_eq!(
        bad("trombone = [2.3, 0.2]", "trombone = [2.3]"),
        "music.mix.trombone: must be [gain, reverb send]"
    );
    assert_eq!(
        bad("trombone = [2.3, 0.2]", "trombone = [-2.3, 0.2]"),
        "music.mix.trombone: the gain must be 0 or more, found -2.3"
    );
    assert_eq!(
        bad("trombone = [2.3, 0.2]", "trombone = [2.3, nan]"),
        "music.mix.trombone: the reverb send must be 0 or more, found NaN"
    );
    assert_eq!(
        bad("sequel = true", "sequel = [true]"),
        "music.theme_options.sequel: must be true or false, a number or a text, found a list"
    );
    assert_eq!(
        bad("sequel = true", "sequel = inf"),
        "music.theme_options.sequel: must be a finite number, found inf"
    );
    assert_eq!(
        bad("FLUTE = [\"SILVER\", \"FLUTE\"]", "TUBA = [\"BIG\", \"TUBA\"]"),
        "picture.title_lines.TUBA: TUBA is not one of the fighters"
    );
    assert_eq!(
        bad("FLUTE = [\"SILVER\", \"FLUTE\"]", "FLUTE = [\"SILVER\"]"),
        "picture.title_lines.FLUTE: must be two texts (the first line and the second)"
    );
    assert_eq!(
        bad("[282.0, 396.0, 56.0]]", "[282.0, 396.0, 0.0]]"),
        "picture.title_balls[1]: the radius must be above 0, found 0.0"
    );
    assert_eq!(
        bad("[282.0, 396.0, 56.0]]", "[282.0, 396.0]]"),
        "picture.title_balls: must be two balls, each [cx, cy, r]"
    );
    assert_eq!(
        bad(
            r#"middle = ["REMATCH", "divider"]"#,
            r#"middle = ["REMATCH", "diagonal"]"#
        ),
        r#"picture.middle: must be [word, "divider"] or [word, an angle in degrees]"#
    );
    assert_eq!(
        bad(r#"middle = ["REMATCH", "divider"]"#, r#"middle = ["", "divider"]"#),
        "picture.middle: the word needs a text"
    );
}

#[test]
fn a_4_chord_loop_and_the_lines_that_fit_it() {
    let four = with(
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
        r#"chord_loop = ["Em", "Am", "B7", "B7"]"#,
    );
    // the 64-note line is 16 a bar over 4 chords: it fits
    let ff = FightFile::parse(&four).expect("a 4-chord loop");
    assert_eq!(
        ff.chords(10),
        strings(&["Em", "B7", "Em", "Am", "B7", "B7", "Em", "Am", "B7", "B7"])
    );
    // a line must fill every bar of the loop with the same number of notes
    let text = with("    78, 75, 71, 75, 78, 81, 83, 75,\n", "    78, 75, 71, 75, 78, 81,\n");
    assert_eq!(
        parse_err(&text),
        "music.lines.FLUTE: 62 notes do not fit the 8-chord loop (every bar needs the same number of notes)"
    );
    let short = "FLUTE = [\n    76, 79, 83,\n]\n";
    let text = EPISODE.replacen(
        &EPISODE[EPISODE.find("FLUTE = [\n    76").unwrap()..EPISODE.find("\n[music.solos]").unwrap()],
        short,
        1,
    );
    assert_eq!(
        parse_err(&text),
        "music.lines.FLUTE: at least 8 notes needed (one a bar), found 3"
    );
    // an extra line is named after its fighter; any other name is refused
    let text = format2_text().replace("\"FLUTE THEME\" = [", "\"TUBA THEME\" = [");
    assert_eq!(
        parse_err(&text),
        "music.lines.TUBA THEME: TUBA THEME is not one of the fighters (an extra line is named after its fighter, \
         such as \"HORN THEME\")"
    );
    for name in ["FLUTEY", "FLUTE ", "FLUTE  "] {
        let text = format2_text().replace("\"FLUTE THEME\" = [", &format!("\"{name}\" = ["));
        assert!(
            parse_err(&text).starts_with(&format!("music.lines.{name}: ")),
            "{name:?}"
        );
    }
    assert!(is_extra_line("HORN THEME", "HORN") && !is_extra_line("HORN", "HORN"));
    assert!(!is_extra_line("HORNTHEME", "HORN") && !is_extra_line("HORN  ", "HORN"));
}

#[test]
fn search_key_counts_the_tail_only_when_music_feeds_the_fight() {
    let ff = episode();
    let mut tail = ff.clone();
    tail.music.chords_tail = strings(&["Am", "B7"]);
    assert_eq!(tail.search_key(&[false, false]), ff.search_key(&[false, false]));
    assert_ne!(tail.search_key(&[true, false]), ff.search_key(&[true, false]));
    assert!(tail.search_key(&[true, false]).contains(";chords_tail=[2:Am,2:B7];"));
    // the mix, the theme's options and the title card's opt-ins never feed the fight
    let mut other = ff.clone();
    other.music.mix.insert("trombone".into(), (2.3, 0.2));
    other
        .music
        .theme_options
        .insert("sequel".into(), OptionValue::Bool(true));
    other.picture.middle = Some(("REMATCH".into(), Middle::Divider));
    other.picture.title_balls = Some([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    other
        .picture
        .title_lines
        .insert("FLUTE".into(), ("A".into(), "B".into()));
    for feeds in [[false, false], [true, true]] {
        assert_eq!(other.search_key(&feeds), ff.search_key(&feeds));
    }
}

#[test]
fn check_line_is_the_line_check() {
    assert_eq!(check_line("FLUTE", &[Some(60); 64], 8), Ok(()));
    assert_eq!(check_line("HORN THEME", &[Some(60); 8], 4), Ok(()));
    assert_eq!(check_line("PIANO", &[Some(60), None, Some(62), None], 4), Ok(()));
    assert_eq!(
        check_line("FLUTE", &[Some(60); 30], 8).unwrap_err().to_string(),
        "music.lines.FLUTE: 30 notes do not fit the 8-chord loop (every bar needs the same number of notes)"
    );
    assert_eq!(
        check_line("FLUTE", &[Some(60); 3], 4).unwrap_err().to_string(),
        "music.lines.FLUTE: at least 4 notes needed (one a bar), found 3"
    );
    assert_eq!(
        check_line("FLUTE", &[Some(60), Some(128), None, None], 4)
            .unwrap_err()
            .to_string(),
        "music.lines.FLUTE[1]: 128 is not a MIDI note (0 to 127)"
    );
    // the fight file's own check says the same
    let text = with(
        "    78, 75, 71, 75, 78, 81, 83, 75,
",
        "    78, 75, 71, 75, 78, 81,
",
    );
    let mut short = episode().music.lines["FLUTE"].clone();
    short.truncate(62);
    assert_eq!(
        parse_err(&text),
        check_line("FLUTE", &short, 8).unwrap_err().to_string()
    );
}

#[test]
fn to_toml_writes_the_newest_format() {
    let mut ff = episode();
    ff.format = 1;
    assert!(ff.to_toml().starts_with(
        "format = 2
"
    ));
    assert_eq!(FightFile::parse(&ff.to_toml()).unwrap().format, FIGHT_FORMAT);
}

#[test]
fn mix_levels_have_bounds() {
    let mut ff = episode();
    ff.music.mix.insert("trombone".into(), (10.0, 1.0));
    assert_eq!(ff.validate(&names), Ok(()));
    ff.music.mix.insert("trombone".into(), (10.5, 0.2));
    assert_eq!(
        validate_err(&ff),
        "music.mix.trombone: the gain must be at most 10.0, found 10.5"
    );
    ff.music.mix.insert("trombone".into(), (2.3, 1.5));
    assert_eq!(
        validate_err(&ff),
        "music.mix.trombone: the reverb send must be at most 1.0, found 1.5"
    );
}

/// A theme option is always a value TOML can write (there is no null to drop): every kind writes and reads back.
#[test]
fn every_theme_option_writes_and_reads_back() {
    let mut ff = episode();
    for (k, v) in [
        ("a", OptionValue::Bool(false)),
        ("b", OptionValue::Int(-3)),
        ("c", OptionValue::Float(0.1 + 0.2)),
        ("d", OptionValue::Str("ELECTRIC".into())),
    ] {
        ff.music.theme_options.insert(k.into(), v);
    }
    let text = ff.to_toml();
    assert!(text.contains(
        "[music.theme_options]
a = false
b = -3
c = 0.30000000000000004
d = \"ELECTRIC\"
"
    ));
    assert_eq!(FightFile::parse(&text), Ok(ff));
}
