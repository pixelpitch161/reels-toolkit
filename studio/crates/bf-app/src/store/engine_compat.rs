//! Engine 2 E6, Review Focus 1: the files today's app writes (settings, queue, fight file, matchup history and a
//! draft) read as the current engine, and saving them again writes the same bytes (no `engine` key on a
//! current-engine row).

use std::fs;

use bf_core::Engine;
use bf_core::fightfile::FightFile;

use super::TempRoot;
use super::history::{load_history, save_history};
use super::queue_file::{load_queue, save_queue};
use super::settings::{load_settings, save_settings};
use crate::model::draft::Draft;
use crate::model::history::History;

const FIGHT: &str = include_str!("../../tests/compat/0.1.0/fight.toml");
const QUEUE: &[u8] = include_bytes!("../../tests/compat/0.1.0/queue.json");
const SETTINGS: &[u8] = include_bytes!("../../tests/compat/0.1.0/settings.toml");

/// A matchup history as the app wrote it before Engine 2 (`save_history`).
const HISTORY: &str = r#"next_id = 2

[[fights]]
id = 0
fighters = ["FLUTE", "TROMBONE"]
seed = 2210
winner = "FLUTE"
made = 1794744000
uploaded = true

[[fights]]
id = 1
fighters = ["TUBA", "PIANO"]
seed = 77
made = 1794745000
uploaded = false
"#;

/// A draft as the app serialised it before Engine 2.
const DRAFT: &str = r#"{"fighters":["FLUTE","TROMBONE"],"theme":"big_top","bars":34,"handicap":[1.0,1.0],"seed":20418,"chord_loop":0,"bass":1,"music_seed":1,"arrangement":null,"pinned":null,"captions":{"instagram":"Flute vs Trombone 🪈🎺 Comment the next match-up 👇\n#ballfight #flute #trombone #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly #gamedev #satisfying #music","tiktok":"Flute vs Trombone 🪈🎺 Comment the next match-up 👇\n#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music","shorts":"Flute vs Trombone\nFlute vs Trombone, every hit in key. Comment the next match-up\n#shorts #ballfight #flute #trombone #music #pixelart"},"captions_edited":true,"base":null}"#;

#[test]
fn old_files_load_as_engine_one() {
    let ff = FightFile::parse(FIGHT).expect("the 0.1.0 fight file reads");
    assert_eq!(ff.engine, Engine::One);
    assert_eq!(crate::engine::convert::draft_of(&ff, FIGHT).engine, Engine::One);

    let temp = TempRoot::new("old_files_load_as_engine_one");
    let queue_path = temp.0.join("queue.json");
    fs::write(&queue_path, QUEUE).unwrap();
    let (queue, _) = load_queue(&queue_path);
    assert!(!queue.jobs.is_empty());
    for job in &queue.jobs {
        assert_eq!(job.spec.engine(), Engine::One);
    }

    let settings_path = temp.0.join("settings.toml");
    fs::write(&settings_path, SETTINGS).unwrap();
    let (_, notice) = load_settings(&settings_path, 4);
    assert_eq!(notice, None);

    let history: History = toml::from_str(HISTORY).expect("the history reads");
    assert!(!history.fights.is_empty());
    assert!(history.fights.iter().all(|row| row.engine == Engine::One));
    let history_path = temp.0.join("history.toml");
    fs::write(&history_path, HISTORY).unwrap();
    let (loaded, notice) = load_history(&history_path);
    assert_eq!((loaded, notice), (history, None));

    let draft: Draft = serde_json::from_str(DRAFT).expect("the draft reads");
    assert_eq!(draft.engine, Engine::One);
}

#[test]
fn engine_one_round_trips_byte_identical() {
    // the fight file: once written by this build, it reads and writes the same bytes, with no `engine` line
    let written = FightFile::parse(FIGHT).unwrap().to_toml();
    assert_eq!(FightFile::parse(&written).unwrap().to_toml(), written);
    assert!(!written.contains("engine"), "{written}");

    let temp = TempRoot::new("engine_one_round_trips_byte_identical");
    // the queue and the settings: saved once, then loaded and saved again, byte for byte
    let queue_path = temp.0.join("queue.json");
    fs::write(&queue_path, QUEUE).unwrap();
    let (queue, _) = load_queue(&queue_path);
    save_queue(&queue_path, &queue).unwrap();
    let once = fs::read(&queue_path).unwrap();
    save_queue(&queue_path, &load_queue(&queue_path).0).unwrap();
    assert_eq!(fs::read(&queue_path).unwrap(), once);
    assert!(!String::from_utf8_lossy(&once).contains("engine"));

    let settings_path = temp.0.join("settings.toml");
    fs::write(&settings_path, SETTINGS).unwrap();
    let (settings, _) = load_settings(&settings_path, 4);
    save_settings(&settings_path, &settings).unwrap();
    let once = fs::read(&settings_path).unwrap();
    save_settings(&settings_path, &load_settings(&settings_path, 4).0).unwrap();
    assert_eq!(fs::read(&settings_path).unwrap(), once);

    // the history and the draft: exactly the bytes the app wrote before Engine 2
    let history_path = temp.0.join("history.toml");
    fs::write(&history_path, HISTORY).unwrap();
    save_history(&history_path, &load_history(&history_path).0).unwrap();
    assert_eq!(fs::read_to_string(&history_path).unwrap(), HISTORY);
    let draft: Draft = serde_json::from_str(DRAFT).unwrap();
    assert_eq!(serde_json::to_string(&draft).unwrap(), DRAFT);
}

#[test]
fn engine_two_rows_write_their_engine() {
    let mut h = History::default();
    h.record(["TEST DUMMY".into(), "TEST DUMMY 2".into()], Engine::Two, 26, None, 100);
    let text = toml::to_string(&h).unwrap();
    assert!(text.contains("engine = 2"), "{text}");
    assert_eq!(toml::from_str::<History>(&text).unwrap(), h);
    let bad = text.replace("engine = 2", "engine = 3");
    assert!(toml::from_str::<History>(&bad).is_err());
}
