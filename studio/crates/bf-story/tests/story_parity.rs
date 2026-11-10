//! `bf_story::story` and `score` against the kit (`projects/ballfight/kit/story.py`):
//!
//! - `stories.json` (written by `python -m tools.studio_export stories`) holds the kit's whole `story()` dict and
//!   its `score()` verdict for all 21 committed recordings of the example episode (the five reference fights,
//!   `no_ko.bfrec` and the 15 scenario recordings, each under its own rules). Every field is compared, floats bit
//!   for bit: `story()` and `score()` are sums, differences, products, quotients and `abs`, no transcendental.
//! - `stats.json` rows (the kit's 4,000-seed table) for the five reference seeds.
//! - The kit's `seeds.txt` line (`kit/search.py`) for the five reference seeds, ep392 first: it is the line the
//!   example episode's `SEED = 392` was picked from ("ranked first (7.78)").
//! - The beat-sheet defaults against the constants the kit used.
//! - `Story.lead` (no Python counterpart) against the recording's `HP` array.
//! - Review Focus 1: a fight with no K.O. is "not a fit" with the reason "no K.O.", and nothing panics.

use std::fs;
use std::path::{Path, PathBuf};

use bf_core::fightfile::FightFile;
use bf_core::grid::Grid;
use bf_core::py::round_nd;
use bf_core::recording::{Array, Recording};
use bf_core::show::{Show, ShowError};
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_story::{BeatSheet, Story, Verdict, score, story};
use serde_json::Value;

/// The five reference recordings, ep392 (the example episode's seed) first.
const REFERENCE: [&str; 5] = [
    "ep392.bfrec",
    "seed1.bfrec",
    "seed2.bfrec",
    "seed3.bfrec",
    "seed500.bfrec",
];

fn studio() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fights() -> PathBuf {
    studio().join("reference/fights/flute_vs_trombone")
}

fn json(name: &str) -> Value {
    let path = fights().join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

fn recording(name: &str) -> Recording {
    Recording::read(&fights().join(name)).unwrap()
}

fn fight_file() -> FightFile {
    let text = fs::read_to_string(studio().join("fights/flute_vs_trombone.toml")).unwrap();
    FightFile::parse(&text).unwrap()
}

fn grid() -> Grid {
    fight_file().grid()
}

fn roster() -> Vec<FighterCfg> {
    let text = fs::read_to_string(studio().join("crates/bf-content/data/roster.json")).unwrap();
    parse_roster(&text).unwrap()
}

fn fighter<'a>(roster: &'a [FighterCfg], name: &str) -> &'a FighterCfg {
    roster
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the roster"))
}

/// The example episode's beat sheet (FLUTE left, TROMBONE right, as the kit builds `CAP_TARGETS`).
fn sheet() -> BeatSheet {
    let r = roster();
    BeatSheet::series([fighter(&r, "FLUTE"), fighter(&r, "TROMBONE")])
}

/// The low abilities' names in the recording's fighter order (`story.lows()` under that recording's episode).
fn low_names(rec: &Recording) -> [String; 2] {
    let r = roster();
    [
        fighter(&r, &rec.names[0]).low.name.clone(),
        fighter(&r, &rec.names[1]).low.name.clone(),
    ]
}

fn story_of(rec: &Recording) -> Story {
    let lows = low_names(rec);
    story(rec, &grid(), &sheet(), [&lows[0], &lows[1]])
}

fn bits(got: f64, want: &Value, what: &str) {
    let w = want
        .as_f64()
        .unwrap_or_else(|| panic!("{what}: {want} is not a number"));
    assert_eq!(got.to_bits(), w.to_bits(), "{what}: got {got:?}, the kit {w:?}");
}

fn opt_usize(v: &Value) -> Option<usize> {
    v.as_u64().map(|n| n as usize)
}

fn triples(got: &[(f64, usize, String)], want: &Value, what: &str) {
    let w = want.as_array().unwrap();
    assert_eq!(got.len(), w.len(), "{what}: {got:?} against {want}");
    for (i, (g, w)) in got.iter().zip(w).enumerate() {
        bits(g.0, &w[0], &format!("{what}[{i}].t"));
        assert_eq!(Some(g.1), opt_usize(&w[1]), "{what}[{i}].who");
        assert_eq!(g.2, w[2].as_str().unwrap(), "{what}[{i}].name");
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

/// Every field of the kit's story dict.
fn story_equals(st: &Story, want: &Value, name: &str) {
    let keys: Vec<&str> = want.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "caps",
            "firsts",
            "lc_t",
            "lead_changes",
            "lows",
            "max_gap",
            "seed",
            "t_ko",
            "winner",
            "winner_hp"
        ],
        "{name}: the kit's story dict has other fields than this test compares"
    );
    assert_eq!(Some(st.seed), want["seed"].as_u64(), "{name}: seed");
    assert_eq!(st.winner, opt_usize(&want["winner"]), "{name}: winner");
    match (st.t_ko, &want["t_ko"]) {
        (None, Value::Null) => {}
        (Some(t), w) => bits(t, w, &format!("{name}: t_ko")),
        (None, w) => panic!("{name}: t_ko None, the kit {w}"),
    }
    assert_eq!(
        Some(u64::from(st.lead_changes)),
        want["lead_changes"].as_u64(),
        "{name}: lead_changes"
    );
    let lc_t = want["lc_t"].as_array().unwrap();
    assert_eq!(st.lc_t.len(), lc_t.len(), "{name}: lc_t {:?} against {lc_t:?}", st.lc_t);
    for (i, (g, w)) in st.lc_t.iter().zip(lc_t).enumerate() {
        bits(*g, w, &format!("{name}: lc_t[{i}]"));
    }
    assert!(want["winner_hp"].is_f64(), "{name}: the kit's winner_hp is a float");
    bits(st.winner_hp, &want["winner_hp"], &format!("{name}: winner_hp"));
    triples(&st.firsts, &want["firsts"], &format!("{name}: firsts"));
    triples(&st.caps, &want["caps"], &format!("{name}: caps"));
    bits(st.max_gap, &want["max_gap"], &format!("{name}: max_gap"));
    assert_eq!(st.lows, strings(&want["lows"]), "{name}: lows");
}

fn all_names() -> Vec<String> {
    json("stories.json")["stories"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

#[test]
fn stories_json_covers_the_21_recordings() {
    let names = all_names();
    assert_eq!(names.len(), 21, "{names:?}");
    for n in REFERENCE.iter().chain(&["no_ko.bfrec"]) {
        assert!(names.iter().any(|m| m == n), "{n} missing from stories.json");
    }
    assert_eq!(names.iter().filter(|n| n.starts_with("scenarios/")).count(), 15);
}

#[test]
fn the_beat_sheet_defaults_are_the_kits() {
    let s = sheet();
    for file in ["stories.json", "stats.json"] {
        let b = &json(file)["beat_sheet"];
        bits(s.t_ko.0, &b["T_KO"][0], "T_KO[0]");
        bits(s.t_ko.1, &b["T_KO"][1], "T_KO[1]");
        assert_eq!(Some(u64::from(s.min_lead_changes)), b["MIN_LEAD_CHANGES"].as_u64());
        bits(s.max_winner_hp, &b["MAX_WINNER_HP"], "MAX_WINNER_HP");
        bits(s.cap_gap, &b["CAP_GAP"], "CAP_GAP");
        bits(s.max_gap, &b["MAX_GAP"], "MAX_GAP");
        assert_eq!(Some(s.min_caps as u64), b["MIN_CAPS"].as_u64());
        let targets = b["CAP_TARGETS"].as_object().unwrap();
        assert_eq!(
            s.cap_targets.keys().collect::<Vec<_>>(),
            targets.keys().collect::<Vec<_>>(),
            "{file}: CAP_TARGETS names"
        );
        for (k, v) in targets {
            bits(s.cap_targets[k], v, &format!("{file}: CAP_TARGETS[{k}]"));
        }
    }
    // The grid's FIGHT! time is the kit's T_FIGHT (the start of the first gap between hits).
    let b = &json("stories.json")["beat_sheet"];
    bits(grid().t_fight, &b["T_FIGHT"], "T_FIGHT");
    // The low names this test hands story() are the kit's story.lows().
    let r = roster();
    for (n, low) in b["LOWS"].as_object().unwrap() {
        assert_eq!(fighter(&r, n).low.name, low.as_str().unwrap(), "LOWS[{n}]");
    }
    // The swapped scenario's episode builds CAP_TARGETS in the other fighter order: the same sheet.
    assert_eq!(BeatSheet::series([fighter(&r, "TROMBONE"), fighter(&r, "FLUTE")]), s);
}

#[test]
fn every_story_equals_the_kits() {
    let doc = json("stories.json");
    for name in all_names() {
        let entry = &doc["stories"][&name];
        let rec = recording(&name);
        assert_eq!(rec.names, strings(&entry["names"]), "{name}: names");
        let st = story_of(&rec);
        story_equals(&st, &entry["story"], &name);
    }
}

#[test]
fn every_verdict_equals_the_kits_and_misses_explain_it() {
    let doc = json("stories.json");
    let sheet = sheet();
    let mut fits = 0;
    for name in all_names() {
        let entry = &doc["stories"][&name];
        let v: Verdict = score(&story_of(&recording(&name)), &sheet);
        assert_eq!(Some(v.fits), entry["fits"].as_bool(), "{name}: fits");
        bits(v.score, &entry["score"], &format!("{name}: score"));
        assert_eq!(
            v.misses.is_empty(),
            v.fits,
            "{name}: misses {:?} with fits {}",
            v.misses,
            v.fits
        );
        fits += usize::from(v.fits);
    }
    assert_eq!(fits, 1, "only ep392 fits among the 21");
}

#[test]
fn the_reference_seeds_match_their_stats_rows() {
    let stats = json("stats.json");
    let rows = stats["rows"].as_array().unwrap();
    let sheet = sheet();
    for name in REFERENCE {
        let rec = recording(name);
        let row = rows
            .iter()
            .find(|r| r["seed"].as_u64() == Some(rec.seed))
            .unwrap_or_else(|| panic!("no stats row for seed {}", rec.seed));
        let st = story_of(&rec);
        let v = score(&st, &sheet);
        assert_eq!(st.winner, opt_usize(&row["winner"]), "{name}: winner");
        bits(st.t_ko.unwrap(), &row["t_ko"], &format!("{name}: t_ko"));
        assert_eq!(Some(u64::from(st.lead_changes)), row["lead_changes"].as_u64(), "{name}");
        bits(st.winner_hp, &row["winner_hp"], &format!("{name}: winner_hp"));
        assert_eq!(Some(st.caps.len() as u64), row["n_caps"].as_u64(), "{name}: n_caps");
        bits(st.max_gap, &row["max_gap"], &format!("{name}: max_gap"));
        assert_eq!(st.lows, strings(&row["lows"]), "{name}: lows");
        assert_eq!(Some(v.fits), row["fits"].as_bool(), "{name}: fits");
        bits(v.score, &row["score"], &format!("{name}: score"));
    }
}

/// Python's `repr` of a float, for the magnitudes these lines hold (Rust's `{:?}` is the same shortest form).
fn py_float(x: f64) -> String {
    format!("{x:?}")
}

/// `kit/search.py`: `one()` rounds the times to 2 places, `main()` writes this line per fit to `check/seeds.txt`.
fn seeds_line(st: &Story, v: &Verdict) -> String {
    let lc_t: Vec<String> = st.lc_t.iter().map(|&t| py_float(round_nd(t, 2))).collect();
    let caps: Vec<String> = st
        .caps
        .iter()
        .map(|(t, w, n)| format!("({}, {w}, '{n}')", py_float(round_nd(*t, 2))))
        .collect();
    let lows: Vec<String> = st.lows.iter().map(|n| format!("'{n}'")).collect();
    format!(
        "{} score {} winner {} ko {:.2} hp {:.1} lc {} [{}] caps [{}] lows [{}]",
        st.seed,
        py_float(v.score),
        st.winner.unwrap(),
        st.t_ko.unwrap(),
        st.winner_hp,
        st.lead_changes,
        lc_t.join(", "),
        caps.join(", "),
        lows.join(", ")
    )
}

#[test]
fn the_reference_seeds_give_the_kits_seeds_txt_lines() {
    // Printed by the kit on this PC: kit/search.py `one(seed)` formatted as `main()` writes check/seeds.txt.
    let want = [
        "392 score 7.78 winner 1 ko 50.02 hp 4.1 lc 2 [22.47, 42.88] caps [(11.21, 0, 'FLUTTER TONGUE'), \
         (16.15, 1, 'SAD TROMBONE'), (26.02, 1, 'POWER SLIDE'), (30.05, 0, 'SNAKE DANCE'), (42.61, 0, 'PICCOLO')] \
         lows ['PEDAL TONE', 'PICCOLO']",
        "1 score -35.59 winner 0 ko 40.64 hp 43.2 lc 0 [] caps [(14.35, 0, 'FLUTTER TONGUE'), \
         (20.64, 0, 'SNAKE DANCE'), (31.85, 1, 'PEDAL TONE')] lows ['PEDAL TONE']",
        "2 score -9.16 winner 1 ko 52.87 hp 14.9 lc 0 [] caps [(14.35, 1, 'SAD TROMBONE'), \
         (24.22, 1, 'POWER SLIDE'), (39.47, 0, 'SNAKE DANCE'), (49.79, 1, 'PEDAL TONE')] lows ['PEDAL TONE', 'PICCOLO']",
        "3 score 3.56 winner 0 ko 50.95 hp 2.8 lc 1 [18.89] caps [(16.15, 1, 'SAD TROMBONE'), \
         (26.91, 0, 'SNAKE DANCE'), (42.61, 0, 'PICCOLO')] lows ['PEDAL TONE', 'PICCOLO']",
        "500 score -16.77 winner 0 ko 55.89 hp 16.2 lc 0 [] caps [(16.15, 0, 'FLUTTER TONGUE'), \
         (26.91, 0, 'SNAKE DANCE'), (31.85, 1, 'POWER SLIDE'), (47.55, 1, 'PEDAL TONE'), (54.72, 0, 'PICCOLO')] \
         lows ['PEDAL TONE', 'PICCOLO']",
    ];
    let sheet = sheet();
    for (name, want) in REFERENCE.iter().zip(want) {
        let st = story_of(&recording(name));
        assert_eq!(seeds_line(&st, &score(&st, &sheet)), want, "{name}");
    }
}

/// `HP[i]` of a recording as `(hp0, hp1)`.
fn hp_row(rec: &Recording, i: usize) -> (f64, f64) {
    match &rec.arrays["HP"] {
        Array::F64 { shape, data } => {
            assert_eq!(shape[1], 2);
            (data[2 * i], data[2 * i + 1])
        }
        other => panic!("HP is {other:?}"),
    }
}

#[test]
fn the_lead_follows_the_recorded_hp() {
    // A hit at time t shows in the first recorded frame at or after t (frame i is the state at t_fight + i / 30;
    // the last frame of a K.O. fight is the K.O.). Where the next hit shows in the same frame, the frame holds
    // both: that hit is skipped (only the last hit of each frame is compared), and the skips are counted.
    let g = grid();
    let mut checked = 0;
    let mut skipped = 0;
    let mut hits = 0;
    for name in all_names() {
        let rec = recording(&name);
        let st = story_of(&rec);
        let hit_t: Vec<f64> = rec.events.iter().filter(|e| e.kind == "hit").map(|e| e.t).collect();
        assert_eq!(st.lead.len(), hit_t.len(), "{name}: one lead point per hit");
        let last = rec.frames() - 1;
        let frame_t = |i: usize| g.t_fight + i as f64 * 4.0 / 120.0;
        let frame_of = |t: f64| {
            let mut i = 0;
            while i < last && frame_t(i) < t - 1e-4 {
                i += 1;
            }
            i
        };
        let frames: Vec<usize> = hit_t.iter().map(|&t| frame_of(t)).collect();
        for (k, &(t, lead)) in st.lead.iter().enumerate() {
            assert_eq!(t.to_bits(), hit_t[k].to_bits(), "{name}: lead[{k}] time");
            hits += 1;
            let i = frames[k];
            if frames.get(k + 1) == Some(&i) {
                assert!(hit_t[k + 1] >= t, "{name}: hits out of time order at {k}");
                skipped += 1;
                continue;
            }
            let (a, b) = hp_row(&rec, i);
            assert_eq!(
                lead.to_bits(),
                (a - b).to_bits(),
                "{name}: lead[{k}] at {t} against HP frame {i}: {lead} vs {}",
                a - b
            );
            checked += 1;
        }
    }
    // Pinned for the 21 committed recordings: every hit is either compared or shares its frame with the next one.
    assert!(hits > 0);
    assert_eq!((hits, checked, skipped), (1775, 1656, 119));
}

#[test]
fn no_ko_is_not_a_fit() {
    let sheet = sheet();
    let ff = fight_file();
    let real = recording("no_ko.bfrec");
    assert_eq!((real.winner, real.t_ko), (None, None));
    let mut cleared = recording("ep392.bfrec");
    cleared.winner = None;
    cleared.t_ko = None;
    for (what, rec) in [("no_ko.bfrec", &real), ("ep392 cleared", &cleared)] {
        let st = story_of(rec);
        assert_eq!((st.winner, st.t_ko), (None, None), "{what}");
        assert_eq!(st.winner_hp, 100.0, "{what}");
        let v = score(&st, &sheet);
        assert!(!v.fits, "{what}");
        assert_eq!(v.misses, ["no K.O."], "{what}");
        let mut ff = ff.clone();
        ff.seed = Some(rec.seed);
        assert_eq!(Show::new(rec, &ff).unwrap_err(), ShowError::NoKo, "{what}");
    }
}
