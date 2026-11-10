//! The fight file's tests: the example episode, round trips, every error text, the search key.

use super::*;

const EPISODE: &str = include_str!("../../../../fights/flute_vs_trombone.toml");

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
    assert_eq!(ff.format, 1);
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
    assert_eq!(ff.grid().n_bars, 34);
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
        "format = 1",
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
        "fight-key 1;fighters=[5:FLUTE,8:TROMBONE];arena=4:ring;handicap=[4000000000000000,3fd999999999999a];\
         dmg_scale=3ff0000000000000;tempo_bpm=4060800000000000;pair_rules=[11:swat_cobras=b:true];music=none"
    );
    let fed = episode().search_key(&[false, true]);
    assert!(fed.ends_with(
        "music=[key=1:E;scale=14:harmonic minor;chords_intro=[2:Em,2:B7];\
         chord_loop=[2:Em,2:Am,2:B7,2:Em,1:C,2:Am,2:B7,2:B7];music_seed=1;lines=[]]"
    ));
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
    let text = with("format = 1", "format = 2");
    assert_eq!(
        parse_err(&text),
        "format: this app reads fight files up to format 1 (this file is format 2)"
    );
    // a newer file is refused for its format, before any setting this app does not know
    let text = with("format = 1", "format = 7\nwings = true");
    assert_eq!(
        parse_err(&text),
        "format: this app reads fight files up to format 1 (this file is format 7)"
    );
}

#[test]
fn format_zero_or_missing() {
    assert_eq!(
        parse_err(&with("format = 1", "format = 0")),
        "format: 0 is not a fight file format (they start at 1)"
    );
    assert_eq!(
        parse_err(&with("format = 1\n", "")),
        "format: missing (a fight file starts with format = 1)"
    );
    assert_eq!(
        parse_err(&with("format = 1", "format = -1")),
        "format: must be a whole number from 0 up, found -1"
    );
    assert_eq!(
        parse_err(&with("format = 1", "format = \"1\"")),
        "format: must be a whole number, found a text"
    );
}

#[test]
fn a_seven_chord_loop() {
    let text = with(
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]"#,
        r#"chord_loop = ["Em", "Am", "B7", "Em", "C", "Am", "B7"]"#,
    );
    assert_eq!(parse_err(&text), "music.chord_loop: 8 chords needed, found 7");
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
    assert_eq!(parse_err(""), "format: missing (a fight file starts with format = 1)");
    assert_eq!(
        parse_err("\n\n  \n"),
        "format: missing (a fight file starts with format = 1)"
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
    ] {
        let _ = FightFile::parse(text);
    }
}
