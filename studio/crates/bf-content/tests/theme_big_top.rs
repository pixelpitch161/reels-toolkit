//! The big top under the general theme model (milestone 3, Task 2), against `studio/reference/themes/big_top/`
//! (`python -m tools.studio_export theme_big_top`):
//!
//! - `chords.json`: the chords of every kit config whose fighters are kit fighters (Table S4's 17 and milestone
//!   1's) rebuilt by `chords_for` from the fight file's intro, loop and tail, at the kit's bar count;
//! - `voicings.json`: the bed's `voice` and `organ` for the 12 roots x 13 qualities (the tables, then the rule);
//! - `theme.json`: the bed alone under two of the kit's base `Sound`s on the episode's recording: the 16-bit
//!   master's bytes (its SHA-256) and the buses.
//!
//! The episode's full soundtrack against the kit's `audio.wav` is milestone 1's gate, unchanged, in `stems.rs`.

mod common;

use bf_content::themes::{self, Preset, ThemeOptions};
use bf_content::{SoundBuild, name_status, theme_card, validate};
use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
use bf_core::harmony::{Harmony, chords_for, scale};
use bf_core::show::Show;
use bf_sound::traits::{BaseSound, BedHook, Sound};
use bf_sound::wav::sample_i16;
use serde_json::Value;
use sha2::{Digest, Sha256};

use common::{EPISODE, json};

const DIR: &str = "themes/big_top";

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap_or_else(|| panic!("a list: {v}"))
        .iter()
        .map(|s| s.as_str().unwrap_or_else(|| panic!("a text: {s}")).to_string())
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Every kit config's `CHORDS` is `[intro 2] + LOOP * n + tail`; `chords_for` rebuilds it from the three parts at
/// the bar count the kit derives from its tempo, and a fight file holding those parts at that tempo checks.
#[test]
fn every_kit_configs_chords_rebuild() {
    let doc = json(&format!("{DIR}/chords.json"));
    let configs = doc["configs"].as_array().expect("configs");
    assert_eq!(configs.len(), 18, "milestone 1's config and Table S4's 17");
    let mut tails = Vec::new();
    for c in configs {
        let name = c["config"].as_str().unwrap();
        let (intro, loop_, tail, chords) = (
            strings(&c["intro"]),
            strings(&c["loop"]),
            strings(&c["tail"]),
            strings(&c["chords"]),
        );
        let n_bars = c["n_bars"].as_u64().unwrap() as usize;
        let bpm = c["bpm"].as_f64().unwrap();
        assert_eq!(Grid::new(bpm).n_bars, n_bars, "{name}: the bar count at {bpm} BPM");
        assert_eq!(chords_for(&intro, &loop_, &tail, n_bars), chords, "{name}");
        // the same parts in a fight file: it reads them, and its chords are the config's
        let mut ff = EPISODE.ff.clone();
        ff.tempo_bpm = bpm;
        ff.music.key = c["key"].as_str().unwrap().to_string();
        ff.music.scale = c["scale"].as_str().unwrap().to_string();
        ff.music.chords_intro = intro;
        ff.music.chord_loop = loop_;
        ff.music.chords_tail = tail.clone();
        ff.music.lines.clear();
        ff.validate(&name_status).unwrap_or_else(|e| panic!("{name}: {e}"));
        let back = FightFile::parse(&ff.to_toml()).unwrap();
        assert_eq!(back.chords(back.grid().n_bars), chords, "{name}");
        tails.push((name.to_string(), tail.len()));
    }
    // Table S4's "loop + tail" column, for the configs it names with a tail.
    for (name, want) in [
        ("examples/clarinet_vs_oboe", 2),
        ("examples/trombone_vs_horn", 1),
        ("ballfight_horn_vs_trombone_rematch", 5),
        ("ballfight_electric_vs_acoustic", 6),
        ("ballfight_flute_vs_clarinet", 0),
    ] {
        assert!(tails.contains(&(name.to_string(), want)), "{name}: {tails:?}");
    }
}

/// The big top's card lists the kit configs that play it, as their `episode.py` set them.
#[test]
fn the_presets_are_the_kits_configs() {
    let doc = json(&format!("{DIR}/chords.json"));
    let card = theme_card("big_top").expect("the big top's card");
    assert_eq!(card.presets.len(), 3);
    let as_strings = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    for p in card.presets {
        let Preset {
            config,
            fighters,
            bpm,
            key,
            scale,
            intro,
            chord_loop,
            tail,
        } = p;
        let c = doc["configs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["config"] == *config)
            .unwrap_or_else(|| panic!("{config} is not in chords.json"));
        assert_eq!(c["mood"], "big_top", "{config}");
        assert_eq!(strings(&c["fighters"]), as_strings(fighters), "{config}");
        assert_eq!(c["bpm"].as_f64(), Some(*bpm), "{config}");
        assert_eq!(
            (c["key"].as_str(), c["scale"].as_str()),
            (Some(*key), Some(*scale)),
            "{config}"
        );
        assert_eq!(strings(&c["intro"]), as_strings(intro), "{config}");
        assert_eq!(strings(&c["loop"]), as_strings(chord_loop), "{config}");
        assert_eq!(strings(&c["tail"]), as_strings(tail), "{config}");
    }
}

/// The voicing rule: the kit's `voice` and `organ` for every chord the harmony parses (the tables for #41's four
/// chords, the chord's tones otherwise), each chord in a harmony that holds it, as the kit's episode does.
#[test]
fn the_voicings_are_the_kits_for_every_chord() {
    let doc = json(&format!("{DIR}/voicings.json"));
    let v = &doc["voicings"];
    let names: Vec<String> = v["voice"].as_object().unwrap().keys().cloned().collect();
    assert_eq!(names.len(), 12 * 13);
    assert_eq!(v["organ"].as_object().unwrap().len(), 12 * 13);
    let g = Grid::new(132.0);
    let notes = |x: &Value| -> Vec<i32> {
        x.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_i64().unwrap() as i32)
            .collect()
    };
    for ch in &names {
        let one = vec![ch.clone()];
        let h = Harmony::new(
            &g,
            "E",
            scale("harmonic minor").unwrap(),
            chords_for(&one, &one, &[], g.n_bars),
            Some(one.clone()),
            2,
        )
        .unwrap_or_else(|e| panic!("{ch}: {e}"));
        assert_eq!(
            themes::big_top::voice(&h, ch).unwrap(),
            notes(&v["voice"][ch]),
            "voice({ch})"
        );
        assert_eq!(
            themes::big_top::organ(&h, ch).unwrap(),
            notes(&v["organ"][ch]),
            "organ({ch})"
        );
    }
    // every one of them is a chord a fight file may ask the big top for
    let mut ff = EPISODE.ff.clone();
    for ch in &names {
        ff.music.chords_tail = vec![ch.clone()];
        assert_eq!(validate(&ff), Ok(()), "{ch}");
    }
}

/// The bed alone: the big top under two of the kit's base `Sound`s on the episode's own recording gives the kit's
/// 16-bit master, sample for sample (its SHA-256), with the buses in the kit's order.
#[test]
fn the_bed_alone_is_the_kits() {
    let doc = json(&format!("{DIR}/theme.json"));
    let configs = doc["configs"].as_array().unwrap();
    assert_eq!(configs.len(), 1);
    let c = &configs[0];
    assert_eq!(c["config"], "examples/flute_vs_trombone");
    assert_eq!(c["recording"], "fights/flute_vs_trombone/ep392.bfrec");
    let ep = &*EPISODE;
    let show = Show::new(&ep.rec, &ep.ff).expect("the episode has a K.O.");
    let mut build = SoundBuild::new(&show, &ep.rec, &ep.ff).expect("the build starts");
    build.sounds = [Box::new(BaseSound { k: 0 }), Box::new(BaseSound { k: 1 })];
    let mix = build.score().expect("the bed alone scores");
    assert_eq!(mix.bus_names(), strings(&c["bed_alone"]["buses"]));
    let st = build.master(&mix);
    let mut pcm = Vec::with_capacity(st.master.len() * 4);
    for frame in &st.master {
        for &x in frame {
            pcm.extend(sample_i16(x).to_le_bytes());
        }
    }
    assert_eq!(
        hex(&Sha256::digest(&pcm)),
        c["bed_alone"]["master_sha256"].as_str().unwrap(),
        "the bed alone's 16-bit master differs from the kit's"
    );
    let stems: Vec<&str> = st.stems.keys().map(String::as_str).collect();
    let mut want = strings(&c["bed_alone"]["stems"]);
    want.sort();
    assert_eq!(stems, want);
}

/// The general model's pieces for the big top: it is ported and offered, has no options, plays no fighter's
/// voice, and its bed has the one optional hook its class defines.
#[test]
fn the_big_top_in_the_model() {
    let t = themes::theme("big_top").unwrap();
    assert!(t.ported && themes::PORTED_THEMES.contains(&t.name));
    assert!(t.options.is_empty() && t.voices.is_empty());
    let bed = t.bed(themes::CLASSIC, &ThemeOptions::new()).unwrap();
    assert_eq!(bed.hooks(), [BedHook::Boom]);
    // the TROMBONE's part calls its boom; the FLUTE's calls none
    let trombone = bf_content::fighters::sound("TROMBONE", 1, &EPISODE.ff).unwrap();
    assert_eq!(trombone.bed_calls(), [BedHook::Boom]);
    assert!(
        bf_content::fighters::sound("FLUTE", 0, &EPISODE.ff)
            .unwrap()
            .bed_calls()
            .is_empty()
    );
    let _: &dyn Sound = &BaseSound { k: 0 };
}
