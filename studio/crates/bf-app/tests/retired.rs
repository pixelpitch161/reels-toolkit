//! Review Focus 6 of milestone 3 (Task 60): a retired fighter's name never leaks into what the app writes or
//! shows. Two voice ids carry the retired names (ELECTRIC's voice and the mambo's alto, rulings R14 and M3-R9);
//! they must stay inside the engine.
//!
//! **Names, not words.** The retired fighters are the two named exactly as `bf_core::fightfile::RETIRED` names
//! them (the owner, Nov 12: only the original generic fighters are retired; the words may appear elsewhere). So a
//! text leaks a retired name when it names one of them as a fighter:
//!
//! 1. the roster form, all capitals, as a word of its own ("GUITAR", "SAX": what a voice id is), unless it is part
//!    of a longer roster name or ability ("ALTO SAX", "GUITAR SMASH");
//! 2. a side of a match-up: "{Name} vs" or "vs {Name}" in any case, and a folder stem's `{name}_vs_` or `_vs_{name}`;
//! 3. in a fight file, any field that holds a fighter's name: `fighters`, the keys of `roles`, `lines`, `solos` and
//!    `title_hits`, and a theme option that names a fighter.
//!
//! The words elsewhere (a band part the kit calls "guitar", a tag such as "#saxophone") are not names.
//!
//! The flow is the app's own on the real engine, per kit pair, at the fixtures' speed (a job renders a clip of 30
//! frames, a search stops at 256 seeds): describe the fight, search, make the take of the kit's own seed, queue
//! it and render it. Everything the engine says and every file it writes (the video's folder and files, the
//! fight file, the caption, the shortlist cache) is read. Run it in release:
//! `cargo test -p bf-app --release --test retired`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bf_app::engine::{self, EngineConfig, convert};
use bf_app::model::EncodeChoice;
use bf_app::model::queue::JobUpdate;
use bf_app::model::shortlist::{FilterSpec, SearchState};
use bf_app::seam::{Command, EngineHandle, Event};
use bf_app::store::Paths;
use bf_core::fightfile::{FightFile, OptionValue, RETIRED};

/// A fresh folder under the system temp folder, removed when dropped.
struct Temp(PathBuf);

impl Temp {
    fn new() -> Temp {
        let root = std::env::temp_dir().join(format!("bf-app retired {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Videos")).unwrap();
        Temp(root)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The roster names and ability names that hold a retired name as a word of their own ("ALTO SAX", "GUITAR
/// SMASH"), in every form the app writes them: as the roster has them, as shown ("Alto sax"), and as a file name
/// part ("alto_sax", "altosax"). Only the fighters the app offers count: a roster fighter not offered yet (an older
/// fighter before its port) is never written, so its names excuse nothing.
fn longer_names() -> Vec<String> {
    let mut names = Vec::new();
    let offered = bf_content::available();
    for f in bf_content::roster()
        .iter()
        .filter(|f| offered.contains(&f.name.as_str()))
    {
        for n in [&f.name, &f.unlock1.name, &f.unlock2.name, &f.low.name] {
            if n.split(' ').any(|w| RETIRED.contains(&w)) && !RETIRED.contains(&n.as_str()) {
                let lower = n.to_lowercase();
                names.push(n.clone());
                names.push(bf_app::wording::display_name(n));
                names.push(lower.replace(' ', "_"));
                names.push(lower.replace(' ', ""));
            }
        }
    }
    names
}

/// Where `word` stands in `text` as a word of its own (not inside a longer word), comparing `ascii` letters
/// without case when `any_case`.
fn words_at(text: &str, word: &str, any_case: bool) -> Vec<usize> {
    let (hay, needle) = if any_case {
        (text.to_ascii_lowercase(), word.to_ascii_lowercase())
    } else {
        (text.to_string(), word.to_string())
    };
    let is_letter = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    hay.match_indices(&needle)
        .map(|(i, _)| i)
        .filter(|&i| !is_letter(hay[..i].chars().next_back()) && !is_letter(hay[i + needle.len()..].chars().next()))
        .collect()
}

/// Whether the word at `at..at + len` of `text` is inside one of `longer` there.
fn inside_longer(text: &str, at: usize, len: usize, longer: &[String]) -> bool {
    let hay = text.to_ascii_lowercase();
    longer.iter().any(|name| {
        let name = name.to_ascii_lowercase();
        hay.match_indices(&name)
            .any(|(i, _)| i <= at && at + len <= i + name.len())
    })
}

/// The retired names `text` names a fighter by (rules 1 and 2 of the module's comment), with where.
fn leaks(text: &str, longer: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for name in RETIRED {
        // 1: the roster form, as a word of its own
        for at in words_at(text, name, false) {
            if !inside_longer(text, at, name.len(), longer) {
                out.push(format!("{name} at {at}"));
            }
        }
        // 2: a side of a match-up, in any case ("Guitar vs Flute", "flute_vs_sax")
        let hay = text.to_ascii_lowercase();
        for at in words_at(text, name, true) {
            let (before, after) = (&hay[..at], &hay[at + name.len()..]);
            let sided = before.ends_with("vs ")
                || before.ends_with("_vs_")
                || after.starts_with(" vs")
                || after.starts_with("_vs_");
            if sided && !inside_longer(text, at, name.len(), longer) {
                out.push(format!("{name} as a side at {at}"));
            }
        }
    }
    out
}

/// Rule 3: a fight file's fields that hold a fighter's name.
fn fight_file_leaks(toml: &str) -> Vec<String> {
    let Ok(ff) = FightFile::parse(toml) else {
        return vec!["a fight file the engine cannot read".to_string()];
    };
    let m = &ff.music;
    let names = ff
        .fighters
        .iter()
        .chain(m.roles.keys())
        .chain(m.lines.keys())
        .chain(m.solos.keys())
        .chain(m.title_hits.keys())
        .chain(m.theme_options.values().filter_map(|v| match v {
            OptionValue::Str(s) => Some(s),
            _ => None,
        }));
    names
        .filter(|n| RETIRED.iter().any(|r| n.eq_ignore_ascii_case(r)))
        .map(|n| format!("the fight file names {n}"))
        .collect()
}

/// What `seconds` of the engine's events hold until `stop` takes one, the texts of each kept in `said`.
fn until(engine: &EngineHandle, seconds: f64, said: &mut Vec<String>, mut stop: impl FnMut(&Event) -> bool) -> Event {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds);
    loop {
        assert!(Instant::now() < deadline, "no answer in {seconds} s");
        while let Some(e) = engine.poll() {
            said.extend(texts_of(&e));
            if stop(&e) {
                return e;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Every text an event carries (its words, names and files), without its pictures and sound.
fn texts_of(e: &Event) -> Vec<String> {
    match e {
        // the whole roster as the interface gets it (its portraits and pixel masks are numbers, which hold no names)
        Event::Roster(r) => vec![format!("{r:?}")],
        Event::FightInfo {
            pair_rules,
            captions,
            problem,
            music,
            ..
        } => {
            let mut t = pair_rules.clone();
            t.extend([
                captions.instagram.clone(),
                captions.tiktok.clone(),
                captions.shorts.clone(),
            ]);
            t.extend(problem.clone());
            t.push(format!("{music:?}"));
            t
        }
        Event::Shortlist { list, .. } => list
            .rows
            .iter()
            .flat_map(|r| r.misses.iter().cloned().chain(r.firsts.iter().map(|f| f.name.clone())))
            .collect(),
        Event::TakePicture { story, .. } => story
            .misses
            .iter()
            .cloned()
            .chain(story.firsts.iter().map(|f| f.name.clone()))
            .collect(),
        Event::TakeSound { music, .. } => music
            .lanes
            .iter()
            .map(|l| l.name.clone())
            .chain(music.markers.iter().map(|(_, m)| m.clone()))
            .chain(music.checks.iter().flat_map(|c| [c.title.clone(), c.note.clone()]))
            .collect(),
        Event::LibraryCover { .. } => Vec::new(),
        other => vec![format!("{other:?}")],
    }
}

/// Every file under `dir`: its path (from `dir`) and, for a text file, its text.
fn files(dir: &Path) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.push((path.display().to_string(), None));
            out.extend(files(&path));
        } else {
            let text = ["toml", "txt", "json", "log"]
                .iter()
                .any(|x| path.extension().is_some_and(|e| e == *x))
                .then(|| std::fs::read_to_string(&path).ok())
                .flatten();
            out.push((path.display().to_string(), text));
        }
    }
    out
}

#[test]
fn the_rules_find_names_and_leave_words() {
    let longer = longer_names();
    assert!(longer.iter().any(|n| n == "ALTO SAX") && longer.iter().any(|n| n == "GUITAR SMASH"));
    let [g, s] = RETIRED;
    for named in [
        g.to_string(),
        format!("voice {s}"),
        format!("{} vs Flute", bf_app::wording::display_name(g)),
        format!("Flute vs {}", bf_app::wording::display_name(s)),
        format!("flute_vs_{}", s.to_lowercase()),
        format!("{}_vs_oboe", g.to_lowercase()),
    ] {
        assert!(!leaks(&named, &longer).is_empty(), "{named:?} names a retired fighter");
    }
    for word in [
        "ALTO SAX",
        "Alto sax vs Tenor sax",
        "altosax_vs_tenorsax",
        "alto_sax_vs_tenor_sax",
        "GUITAR SMASH",
        "Guitar smash",
        "#saxophone",
        "Nylon guitar",
    ] {
        assert!(leaks(word, &longer).is_empty(), "{word:?} is a word, not a name");
    }
    let named = include_str!("../../../fights/flute_vs_trombone.toml").replace("TROMBONE", g);
    assert!(!fight_file_leaks(&named).is_empty());
    assert!(fight_file_leaks(include_str!("../../../fights/altosax_vs_tenorsax.toml")).is_empty());
}

/// Every kit pair's whole flow, at the fixtures' speed: no retired name in anything the engine says or writes.
#[test]
fn no_kit_pair_writes_a_retired_name() {
    let temp = Temp::new();
    let config = EngineConfig {
        workers: 4,
        paths: Paths::under(&temp.0.join("data")),
        clip: Some(0..30),
        search_limit: Some(256),
    };
    let engine = engine::spawn(Arc::new(|| {}), config);
    let longer = longer_names();
    let mut said: Vec<String> = Vec::new();
    engine.send(Command::LoadRoster);
    until(&engine, 30.0, &mut said, |e| matches!(e, Event::Roster(_)));
    let mut written: Vec<(String, Option<String>)> = Vec::new();
    let mut pairs = 0;
    for (r#gen, (pair, ff)) in (1..).zip(bf_content::kit_pair_fights()) {
        pairs += 1;
        let toml = ff.to_toml();
        let draft = convert::draft_of(ff, &toml);
        // the fight file the app writes for it, and the swapped order's
        for d in [draft.clone(), {
            let mut s = draft.clone();
            s.fighters.swap(0, 1);
            s.handicap.swap(0, 1);
            s
        }] {
            let text = convert::fight_file(&d)
                .unwrap_or_else(|e| panic!("{pair}: {e}"))
                .to_toml();
            written.push((format!("{pair}: the fight file"), Some(text)));
        }
        engine.send(Command::DescribeFight {
            r#gen,
            draft: draft.clone(),
        });
        until(&engine, 30.0, &mut said, |e| matches!(e, Event::FightInfo { .. }));
        let mut search = draft.clone();
        search.seed = None;
        engine.send(Command::StartSearch {
            r#gen,
            draft: search,
            filters: FilterSpec::default(),
        });
        until(&engine, 120.0, &mut said, |e| {
            matches!(e, Event::Shortlist { list, .. } if list.state != SearchState::Searching)
                || matches!(e, Event::Problem { .. })
        });
        let take = r#gen;
        engine.send(Command::LoadTake {
            take,
            draft: draft.clone(),
        });
        let sound = until(&engine, 180.0, &mut said, |e| {
            matches!(e, Event::TakeSound { .. } | Event::TakeFailed { .. })
        });
        // and the positive side: each fighter's lane (its stem) is named by its roster name as shown
        let Event::TakeSound { music, .. } = sound else {
            panic!("{pair}: {sound:?}");
        };
        let lanes: Vec<&str> = music
            .lanes
            .iter()
            .filter(|l| l.group == bf_app::model::take::LaneGroup::Fighters)
            .map(|l| l.name.as_str())
            .collect();
        let shown = draft.fighters.clone().map(|n| bf_app::wording::display_name(&n));
        assert_eq!(lanes, shown.iter().map(String::as_str).collect::<Vec<_>>(), "{pair}");
        let out_root = temp.0.join("Videos");
        engine.send(Command::Enqueue {
            draft: draft.clone(),
            out_root: out_root.clone(),
        });
        let spec = match until(&engine, 60.0, &mut said, |e| {
            matches!(e, Event::Enqueued { .. } | Event::EnqueueFailed { .. })
        }) {
            Event::Enqueued { spec } => spec,
            other => panic!("{pair}: {other:?}"),
        };
        // what a whole video's folder holds besides the mp4 (a clip's folder has only the mp4): the fight file and
        // the caption, made from the queued fight file as the render makes them
        let queued = FightFile::parse(&spec.fight_toml).unwrap_or_else(|e| panic!("{pair}: {e}"));
        written.push((format!("{pair}: the queued fight file"), Some(spec.fight_toml.clone())));
        written.push((
            format!("{pair}: caption.txt"),
            Some(bf_export::output::caption_text(&queued)),
        ));
        engine.send(Command::RunJob {
            id: take,
            fight_toml: spec.fight_toml.clone(),
            out_root,
            speed: EncodeChoice::Faster,
        });
        until(
            &engine,
            300.0,
            &mut said,
            |e| matches!(e, Event::Job { id, update: JobUpdate::Done { .. } | JobUpdate::Failed { .. } | JobUpdate::Cancelled } if *id == take),
        );
        eprintln!("{pair}: done");
    }
    assert_eq!(pairs, 18);
    engine.send(Command::Shutdown);
    drop(engine);
    written.extend(files(&temp.0));
    let mut found = Vec::new();
    for text in &said {
        for leak in leaks(text, &longer) {
            found.push(format!("said {leak}: {}", text.chars().take(200).collect::<String>()));
        }
    }
    let mut fight_files = 0;
    for (path, text) in &written {
        for leak in leaks(path, &longer) {
            found.push(format!("{path}: {leak}"));
        }
        if let Some(text) = text {
            for leak in leaks(text, &longer) {
                found.push(format!("{path} holds {leak}"));
            }
            if path.ends_with(".toml") || path.ends_with("fight file") {
                fight_files += 1;
                found.extend(fight_file_leaks(text).into_iter().map(|l| format!("{path}: {l}")));
            }
        }
    }
    assert!(fight_files >= 54, "{fight_files} fight files read");
    let videos = written.iter().filter(|(p, _)| p.ends_with(".mp4")).count();
    if bf_export::find_ffmpeg().is_ok() {
        assert_eq!(videos, 18, "every kit pair's clip was saved");
    } else {
        eprintln!("ffmpeg cannot be used here, so no clip was saved: the rest was read");
    }
    assert!(found.is_empty(), "{found:#?}");
}
