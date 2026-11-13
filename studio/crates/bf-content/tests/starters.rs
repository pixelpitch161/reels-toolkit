//! Starters for every ordered pair of kit fighters, the series' handicaps and theme compatibility (milestone 3 Task
//! 59): every pair's starter is a fight the Studio accepts; the kit pairs start from their fight files in both
//! orders; six crossovers (pairs the series never wrote music for) start from the config the kit itself plays, and
//! their soundtracks and frames are the kit's (`studio/reference/episodes/crossovers/`, made by
//! `python -m tools.studio_export crossovers`).

use std::collections::BTreeMap;
use std::sync::Arc;

use bf_content::{available, card, home_config, kit_pair_fights, pair_rules, starter, themes_for, validate};
use bf_core::fightfile::{FightFile, OptionValue};
use bf_core::recording::Recording;
use bf_core::show::Show;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The themes every pair is offered but those whose bed has no `boom` (nor a `trailer_hit`).
const NO_BOOM: [&str; 2] = ["dark_disco", "heist"];

/// The fighters whose part plays the bed's `boom` (the HORN when the bed has no `trailer_hit`).
const PLAYS_A_BOOM: [&str; 5] = ["TROMBONE", "ORGAN", "TUBA", "EUPHONIUM", "HORN"];

fn crossovers() -> Value {
    let path = bf_testdata::reference("episodes/crossovers/crossovers.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}: run python -m tools.studio_export crossovers", path.display()));
    serde_json::from_str(&text).expect("crossovers.json")
}

fn texts(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap_or_else(|| panic!("{v} is not a list"))
        .iter()
        .map(|s| s.as_str().expect("text").to_string())
        .collect()
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// A kit config name's pair folder (`ballfight_` and `examples/` dropped, as m3refs.config_slug does).
fn slug(config: &str) -> &str {
    let name = config.rsplit('/').next().unwrap_or(config);
    name.strip_prefix("ballfight_").unwrap_or(name)
}

/// Every ordered pair of two different kit fighters.
fn ordered_pairs() -> Vec<(&'static str, &'static str)> {
    let names = available();
    let mut out = Vec::new();
    for &a in &names {
        for &b in &names {
            if a != b {
                out.push((a, b));
            }
        }
    }
    out
}

/// What every starter sets on top of the fight it begins from.
fn as_a_starter(mut ff: FightFile, left: &str, right: &str) -> FightFile {
    ff.app_version = env!("CARGO_PKG_VERSION").to_string();
    ff.seed = None;
    ff.series_tag = None;
    ff.picture.fx_seed = None;
    ff.pair_rules = pair_rules(left, right);
    ff.captions = bf_content::draft_captions(left, right);
    ff
}

#[test]
fn every_ordered_pair_has_a_starter_the_studio_accepts() {
    let names = available();
    assert_eq!(names.len(), 20, "the kit fighters offered");
    let pairs = ordered_pairs();
    assert_eq!(pairs.len(), names.len() * (names.len() - 1));
    assert_eq!(pairs.len(), 380);
    let mut problems = Vec::new();
    for &(a, b) in &pairs {
        let ff = match starter(a, b) {
            Ok(ff) => ff,
            Err(e) => {
                problems.push(format!("{a} vs {b}: {e}"));
                continue;
            }
        };
        if ff.fighters != [a, b] {
            problems.push(format!("{a} vs {b}: fighters {:?}", ff.fighters));
        }
        if let Err(e) = validate(&ff) {
            problems.push(format!("{a} vs {b}: {e}"));
        }
        if !themes_for(a, b).contains(&ff.music.theme.as_str()) {
            problems.push(format!("{a} vs {b}: theme {} is not offered", ff.music.theme));
        }
        match FightFile::parse(&ff.to_toml()) {
            Ok(back) if back == ff => {}
            Ok(_) => problems.push(format!("{a} vs {b}: the written file reads back as another fight")),
            Err(e) => problems.push(format!("{a} vs {b}: the written file: {e}")),
        }
    }
    assert!(
        problems.is_empty(),
        "{} of 380:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// The 17 kit pairs (18 configs: HORN and TROMBONE have two) start from their fight files in both orders: the
/// series' handicap and damage scale, swapped with the sides.
#[test]
fn the_kit_pairs_start_from_their_fight_files_in_both_orders() {
    let fights = kit_pair_fights();
    assert_eq!(fights.len(), 18);
    let mut pairs: Vec<[String; 2]> = fights
        .iter()
        .map(|(_, ff)| {
            let mut p = [ff.fighters[0].clone(), ff.fighters[1].clone()];
            p.sort();
            p
        })
        .collect();
    pairs.sort();
    pairs.dedup();
    assert_eq!(pairs.len(), 17, "the kit pairs");
    for (pair, ff) in &fights {
        let (a, b) = (ff.fighters[0].as_str(), ff.fighters[1].as_str());
        assert_eq!(
            ff.pair_rules,
            pair_rules(a, b),
            "{pair}: the fight file's rules are the series' table"
        );
        let got = starter(a, b).unwrap_or_else(|e| panic!("{pair}: {e}"));
        assert_eq!(got, as_a_starter((*ff).clone(), a, b), "{pair} as written");
        assert_eq!(
            (got.handicap.clone(), got.dmg_scale),
            (ff.handicap.clone(), ff.dmg_scale)
        );
        // the other order: the pair's other config when it has one (HORN and TROMBONE), else this one swapped
        let other = fights
            .iter()
            .find(|(_, o)| o.fighters == [b, a])
            .map(|(_, o)| (*o).clone());
        let want = other.unwrap_or_else(|| {
            let mut s = (*ff).clone();
            s.fighters.reverse();
            s.handicap.reverse();
            s
        });
        let got = starter(b, a).unwrap_or_else(|e| panic!("{pair} swapped: {e}"));
        assert_eq!(got, as_a_starter(want, b, a), "{pair} swapped");
    }
    // the two configs of HORN and TROMBONE, each from its own left fighter
    assert_eq!(starter("HORN", "TROMBONE").unwrap().music.chords_tail.len(), 5);
    assert_eq!(starter("TROMBONE", "HORN").unwrap().music.chords_tail.len(), 1);
    // the series' handicap, swapped with the sides
    assert_eq!(starter("FLUTE", "CLARINET").unwrap().handicap, [5.3, 0.36]);
    assert_eq!(starter("CLARINET", "FLUTE").unwrap().handicap, [0.36, 5.3]);
    assert_eq!(starter("OBOE", "CLARINET").unwrap().handicap, [1.0, 1.0]);
    assert_eq!(starter("OBOE", "CLARINET").unwrap().dmg_scale, 0.55);
}

/// Every kit fighter's home config is the one the kit's files give (the export reads the episode numbers from the
/// configs' notes), and the cards' "can play bass" is the list the kit side of the rule uses.
#[test]
fn every_fighter_has_the_kits_home_config() {
    let doc = crossovers();
    let mut names: Vec<&str> = available();
    names.sort_unstable();
    assert_eq!(texts(&doc["fighters"]), names, "the kit fighters");
    for name in &names {
        let want = doc["homes"][*name]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: no home"));
        assert_eq!(home_config(name), Some(slug(want)), "{name}");
    }
    let mut bass: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| card(n).unwrap_or_else(|| panic!("{n}: no card")).can_play_bass)
        .collect();
    bass.sort_unstable();
    assert_eq!(texts(&doc["can_play_bass"]), bass);
}

fn options_of(v: &Value) -> BTreeMap<String, OptionValue> {
    v.as_object()
        .expect("theme_options")
        .iter()
        .map(|(k, v)| {
            let o = match v {
                Value::Bool(b) => OptionValue::Bool(*b),
                Value::String(s) => OptionValue::Str(s.clone()),
                other => panic!("{k}: {other}"),
            };
            (k.clone(), o)
        })
        .collect()
}

/// The six crossovers' fight files are the configs the export gave the kit, value for value.
#[test]
fn the_crossovers_are_the_kit_configs() {
    let doc = crossovers();
    let rows = doc["crossovers"].as_array().expect("crossovers");
    assert_eq!(rows.len(), 6);
    let mut themes = Vec::new();
    for c in rows {
        let f = texts(&c["fighters"]);
        let (a, b) = (f[0].as_str(), f[1].as_str());
        let ff = starter(a, b).unwrap_or_else(|e| panic!("{a} vs {b}: {e}"));
        let mut wrong = Vec::new();
        let mut eq = |what: &str, ok: bool| {
            if !ok {
                wrong.push(what.to_string());
            }
        };
        eq("fighters", ff.fighters == f);
        eq("theme", ff.music.theme == c["mood"].as_str().unwrap());
        eq("tempo", ff.tempo_bpm == c["bpm"].as_f64().unwrap());
        eq("key", ff.music.key == c["key"].as_str().unwrap());
        eq("scale", ff.music.scale == c["scale"].as_str().unwrap());
        eq("intro", ff.music.chords_intro == texts(&c["intro"]));
        eq("loop", ff.music.chord_loop == texts(&c["loop"]));
        eq("tail", ff.music.chords_tail == texts(&c["tail"]));
        let n_bars = c["n_bars"].as_u64().unwrap() as usize;
        eq("bars", ff.grid().n_bars == n_bars);
        let roles: BTreeMap<String, String> = c["roles"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
            .collect();
        eq("roles", ff.music.roles == roles);
        eq(
            "theme_options",
            ff.music.theme_options == options_of(&c["theme_options"]),
        );
        eq(
            "rules",
            ff.pair_rules.len() == c["rules"].as_object().unwrap().len()
                && ff.pair_rules.keys().all(|k| c["rules"][k] == Value::Bool(true)),
        );
        let handicap: Vec<f64> = c["handicap"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        eq("handicap", ff.handicap == handicap);
        eq("dmg_scale", ff.dmg_scale == c["dmg_scale"].as_f64().unwrap());
        // the kit's sketch lines and solos, no title hits, no mix, no title-card opt-ins
        eq("lines", ff.music.lines.is_empty());
        eq("solos", ff.music.solos.is_empty());
        eq("title_hits", ff.music.title_hits.is_empty());
        eq("mix", ff.music.mix.is_empty());
        eq(
            "picture",
            ff.picture.title_lines.is_empty() && ff.picture.title_balls.is_none() && ff.picture.middle.is_none(),
        );
        eq("home", home_config(a) == Some(slug(c["home"].as_str().unwrap())));
        assert!(
            wrong.is_empty(),
            "{a} vs {b} is not the kit config: {}",
            wrong.join(", ")
        );
        themes.push((ff.music.theme.clone(), c["theme_from"].as_str().unwrap().to_string()));
    }
    // one per theme family, and the fallback to the right fighter's home theme is exercised (ALTO SAX's dark disco
    // has no boom for the ORGAN)
    let mut distinct: Vec<&str> = themes.iter().map(|(t, _)| t.as_str()).collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert!(distinct.len() >= 5, "{themes:?}");
    assert!(themes.iter().any(|(_, from)| from == "right"), "{themes:?}");
}

/// The 16-bit master's SHA-256, as the kit's `A.write` writes the samples (and bf-harness's `pcm_sha`).
fn pcm_sha(master: &[[f64; 2]]) -> String {
    let mut pcm = Vec::with_capacity(master.len() * 4);
    for f in master {
        for &x in f {
            pcm.extend(bf_sound::wav::sample_i16(x).to_le_bytes());
        }
    }
    sha(&pcm)
}

/// The kit's first fit of each crossover (its own search from seed 1; the recording in the pinned cache), played
/// and drawn by the Studio from the starter's fight file: the 16-bit master and 8 frames are the kit's, with no hull
/// fallback.
#[test]
fn the_crossovers_soundtracks_and_frames_are_the_kits() {
    let doc = crossovers();
    let mut failures = Vec::new();
    for c in doc["crossovers"].as_array().expect("crossovers") {
        let f = texts(&c["fighters"]);
        let (a, b) = (f[0].as_str(), f[1].as_str());
        let rel = c["recording"]["cache"].as_str().expect("the recording");
        let path = bf_testdata::cache_file(rel);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(sha(&bytes), c["recording"]["sha256"].as_str().unwrap(), "{rel}");
        let rec = Recording::read(&path).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let seed = c["seed"].as_u64().unwrap();
        assert_eq!(
            (rec.seed, rec.hash()),
            (seed, c["rec_hash"].as_str().unwrap().to_string()),
            "{rel}"
        );
        let mut ff = starter(a, b).unwrap_or_else(|e| panic!("{a} vs {b}: {e}"));
        ff.seed = Some(seed);
        // the soundtrack
        let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("{a} vs {b}: the show: {e}"));
        let build = bf_content::SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("{a} vs {b}: {e}"));
        let mix = build.score().unwrap_or_else(|e| panic!("{a} vs {b}: the score: {e}"));
        let buses: Vec<String> = mix.bus_names().iter().map(|s| s.to_string()).collect();
        if buses != texts(&c["buses"]) {
            failures.push(format!("{a} vs {b}: buses {buses:?}, the kit {:?}", texts(&c["buses"])));
        }
        let st = build.master(&mix);
        if pcm_sha(&st.master) != c["master_sha256"].as_str().unwrap() {
            failures.push(format!("{a} vs {b} (seed {seed}): the 16-bit master is not the kit's"));
        }
        // the frames
        let pic = bf_content::build_picture(Arc::new(rec.clone()), &ff)
            .unwrap_or_else(|e| panic!("{a} vs {b}: the picture: {e}"));
        let frames = c["frames"].as_array().expect("frames");
        assert_eq!(frames.len(), 8, "{a} vs {b}: 8 frames");
        for row in frames {
            let n = row[0].as_u64().unwrap() as usize;
            let (img, _, report) = pic.frame_reported(n);
            assert_eq!(
                report.hull_fallbacks, 0,
                "{a} vs {b} frame {n}: a hull fallback (ruling M3-R4)"
            );
            if sha(&img.data) != row[1].as_str().unwrap() {
                failures.push(format!("{a} vs {b} (seed {seed}): frame {n} is not the kit's"));
            }
        }
        println!(
            "{a} vs {b}: theme {}, seed {seed} (a fit: {}, {} seeds searched), winner {}, K.O. {}: master and 8 \
             frames checked",
            ff.music.theme, c["fit"], c["searched"], c["winner"], c["t_ko"]
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Ruling M3-R2: a pair with a fighter whose part plays a boom is offered neither `dark_disco` nor `heist`; every
/// other pair is offered every theme; `validate` refuses a theme left out, in Review Focus 1's words.
#[test]
fn themes_for_leaves_out_exactly_the_themes_without_a_boom() {
    let all = bf_content::theme_names();
    assert_eq!(all.len(), 14);
    for (a, b) in ordered_pairs() {
        let booms = PLAYS_A_BOOM.contains(&a) || PLAYS_A_BOOM.contains(&b);
        let want: Vec<&str> = all
            .iter()
            .copied()
            .filter(|t| !(booms && NO_BOOM.contains(t)))
            .collect();
        assert_eq!(themes_for(a, b), want, "{a} vs {b}");
    }
    let mut ff = starter("PIANO", "TUBA").expect("piano vs tuba");
    for theme in NO_BOOM {
        ff.music.theme = theme.to_string();
        ff.music.theme_options.clear();
        let display = bf_content::themes::theme(theme).expect("the theme").display();
        assert_eq!(
            validate(&ff).map_err(|e| e.to_string()),
            Err(format!(
                "music.theme: {display} has no boom (a big drum hit), which the TUBA's part plays; pick another theme"
            ))
        );
    }
    let mut ff = starter("PIANO", "FLUTE").expect("piano vs flute");
    for theme in NO_BOOM {
        ff.music.theme = theme.to_string();
        assert_eq!(validate(&ff), Ok(()), "{theme}");
    }
}
