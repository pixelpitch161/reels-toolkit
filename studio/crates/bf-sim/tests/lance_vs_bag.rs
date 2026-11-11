//! Lance parity with the Python kit, past what the FLUTE against TROMBONE recordings can show before their first
//! ability: the TROMBONE's lance against a bag.
//!
//! The windows of `prefix_before_abilities.rs` hold thrusts, hits, growth and swats, but no blocked thrust (neither
//! fighter has a shield), no lance grown to its cap (ten landed thrusts at most before an ability fires) and no
//! thrust under haste. In `studio/reference/fights/lance_vs_bag/` the lance fights a bag, a fighter with no weapon
//! and abilities that do nothing, so nothing but the engine core and the lance acts: the lance tracks, thrusts on
//! its grid, hits while it goes out and while it is held, misses, grows a fret a hit to its cap and thrusts on at
//! that length; a SHIELD BAG blocks thrusts, which then pull straight back; and the LANCER (the TROMBONE with POWER
//! SLIDE as its only ability) thrusts under haste, on the 8th notes at twice the speed. The recordings are Python's
//! (`python -m tools.studio_export lance_bag`, whose docstring has the rules of each kind of fight); each holds the
//! first 16 s of a fight, and `index.json` lists them with the three entries the export added to the kit's roster
//! table.
//!
//! Each test replays the Rust fight of a recording's seed, fighters and rules and compares EVERY frame (`P`, `V`,
//! `TH`, `HP`, `BL`, `FR`, `EXT`, `SH`, `HA`, `LV`) and EVERY event (times, kinds and whole numbers exactly, floats
//! to 1e-6, the parity bar of a fight; the differences seen are printed, and are far smaller).
//!
//! The engine fires the abilities itself: POWER SLIDE is compared like everything else, its `ability` events too.
//! One kind in these fights is not ported: the SHIELD BAG's `shield` (BRACE). The engine announces it and does
//! nothing more, so the test makes its one assignment by hand (`B.shield = ab_dur(ab)`, `fight.py` 2847 to 2848)
//! in the step the ENGINE announces it; when and that it fires is the engine's own doing and is compared.
//!
//! The windows end long before the two engines drift apart. When the lance was ported the same comparison was run
//! over 120 such fights (seeds 1 to 20, both corners, the three kinds): every one agreed over its whole 16 s, with a
//! largest difference of 6.3e-9 and a median of 1.6e-11. The drift starts from last-bit differences in the
//! arithmetic (the maths library, numpy's fused dot product) and grows with the fight's chaos. The recordings are
//! Python's and do not change with the engine: if a change to the engine's arithmetic ever moves a window past the
//! tolerance, shorten `WINDOW` in the export, with the cause written down.

mod support;

use std::collections::BTreeMap;

use bf_core::event::{Event, Value};
use bf_core::recording::{Array, Recording};
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::schedule::Schedule;
use bf_sim::{Fight, Setup};
use serde_json::Value as Json;
use support::{TOL, compare_event, count_kinds, kind_of, rule};

/// The lance's reach at its cap: `base = 0.95` plus 19 times `grow = 0.08`, summed hit by hit as `thrust_step` does
/// (the next step would pass `max_len = 2.5`).
const REACH_CAP: f64 = 2.470000000000001;
/// The fret count then: `frets0 = 1` plus 19.
const FRETS_CAP: i64 = 20;

/// One fight of `index.json`.
#[derive(Debug)]
struct Row {
    /// Its name: `<kind>_<corner>_s<seed>`.
    name: String,
    /// `plain`, `shield` or `haste`.
    kind: String,
    /// The lance's corner: 0 left, 1 right.
    lance: usize,
    /// The seed.
    seed: u64,
    /// The lance's owner: `TROMBONE` or `LANCER`.
    fighter: String,
    /// The bag it fights: `BAG` or `SHIELD BAG`.
    bag: String,
    /// Its events by kind, as the export counted them.
    events: BTreeMap<String, usize>,
    /// What the export says it covers.
    covers: Vec<String>,
}

/// `index.json`: the entries the export added to the kit's roster table (read by the roster's own reader), the
/// fights, what the set must cover, and the seconds of fighting each recording holds.
struct Index {
    added: Vec<FighterCfg>,
    fights: Vec<Row>,
    must_cover: Vec<String>,
    window: f64,
}

fn index() -> Index {
    let path = support::reference("lance_vs_bag/index.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: Json = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text_of = |j: &Json| j.as_str().unwrap_or_else(|| panic!("{j} is not a text")).to_string();
    let whole = |j: &Json| j.as_u64().unwrap_or_else(|| panic!("{j} is not a whole number"));
    let list = |j: &Json| j.as_array().unwrap_or_else(|| panic!("{j} is not a list")).clone();
    let row = |f: &Json| Row {
        name: text_of(&f["name"]),
        kind: text_of(&f["kind"]),
        lance: whole(&f["lance"]) as usize,
        seed: whole(&f["seed"]),
        fighter: text_of(&f["fighter"]),
        bag: text_of(&f["bag"]),
        events: f["events"]
            .as_object()
            .expect("events by kind")
            .iter()
            .map(|(k, n)| (k.clone(), whole(n) as usize))
            .collect(),
        covers: list(&f["covers"]).iter().map(text_of).collect(),
    };
    Index {
        added: parse_roster(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
        fights: list(&doc["fights"]).iter().map(row).collect(),
        must_cover: list(&doc["must_cover"]).iter().map(text_of).collect(),
        window: doc["window"].as_f64().expect("the window in seconds"),
    }
}

/// What a frame holds of the fight (`Fight.snap`), for the keys compared: per key, the left fighter's value(s) then
/// the right one's.
struct Frame {
    floats: [(&'static str, Vec<f64>); 8],
    wholes: [(&'static str, [i64; 2]); 2],
}

fn snap(fight: &Fight) -> Frame {
    let [a, b] = &fight.balls;
    Frame {
        floats: [
            ("P", vec![a.p[0], a.p[1], b.p[0], b.p[1]]),
            ("V", vec![a.v[0], a.v[1], b.v[0], b.v[1]]),
            ("TH", vec![a.th, b.th]),
            ("HP", vec![a.hp.max(0.0), b.hp.max(0.0)]),
            ("BL", vec![a.blade_len, b.blade_len]),
            ("EXT", vec![a.ext, b.ext]),
            ("SH", vec![a.shield, b.shield]),
            ("HA", vec![a.haste, b.haste]),
        ],
        wholes: [("FR", [a.frets, b.frets]), ("LV", [a.level, b.level])],
    }
}

/// What `trigger` does for the one ability kind of these fights that the engine has not ported, the bag's `shield`:
/// `B.shield = ab_dur(ab)`, with `ab_dur(ab)` = `ab['dur']`, else `ab['dur_beats'] * beat`. The engine has just
/// announced the ability `name` of fighter `who` (an `ability` event of kind `shield`) and done nothing else.
fn shield_by_hand(fight: &mut Fight, who: usize, name: &str) {
    let (cfg, beat) = (fight.balls[who].cfg, fight.rules.beat);
    let ab = [&cfg.unlock1, &cfg.unlock2, &cfg.low]
        .into_iter()
        .find(|ab| ab.name == name)
        .unwrap_or_else(|| panic!("{} has no ability {name}", cfg.name));
    fight.balls[who].shield = match (ab.get("dur"), ab.get("dur_beats")) {
        (Some(dur), _) => dur,
        (None, Some(beats)) => beats * beat,
        (None, None) => panic!("{name} has no duration"),
    };
}

fn floats<'a>(rec: &'a Recording, key: &str) -> &'a [f64] {
    match rec.arrays.get(key) {
        Some(Array::F64 { data, .. }) => data,
        other => panic!("{key} is not a float array: {other:?}"),
    }
}

fn wholes<'a>(rec: &'a Recording, key: &str) -> &'a [i64] {
    match rec.arrays.get(key) {
        Some(Array::I64 { data, .. }) => data,
        other => panic!("{key} is not a whole-number array: {other:?}"),
    }
}

/// What one comparison covered.
struct Span {
    /// Events matched, by kind (`hit` and `block` with what hit).
    kinds: BTreeMap<String, usize>,
    /// The abilities the engine fired, by name.
    abilities: Vec<String>,
    /// The lance's largest reach.
    reach: f64,
    /// Landed thrusts of the lance at its cap (their sub is the capped fret count: nothing grew).
    hits_at_cap: usize,
    /// Thrusts that started less than three quarters of a beat after the one before: on an 8th note, under haste.
    thrusts_on_8ths: usize,
    /// Frames in which the lance's owner has haste.
    frames_hasted: usize,
    /// Frames in which the bag's shield is up.
    frames_shielded: usize,
}

/// Runs the Rust fight of a recording (its fighters, rules and seed), making the bag's shield by hand when the engine
/// announces it, and compares every frame and every event.
fn compare(row: &Row, added: &[FighterCfg]) -> Span {
    let name = row.name.as_str();
    let py = support::python_recording(&format!("lance_vs_bag/{name}.bfrec"));
    let mut fighters = support::roster();
    fighters.extend(added.iter().cloned());
    let mut names = [row.bag.as_str(); 2];
    names[row.lance] = &row.fighter;
    assert_eq!(py.names, names, "{name}");
    assert_eq!((py.seed, py.winner, py.t_ko), (row.seed, None, None), "{name}");
    let setup: Setup = support::setup_like(&py, &fighters);

    // the fight loop (`Fight.run`): a frame before the first step and after every `steps_per_frame` steps
    let mut fight = Fight::new(&setup, false);
    let mut sched = Schedule::new(&mut fight);
    fight.t = fight.rules.t_fight;
    let mut frames = vec![snap(&fight)];
    let mut fired = Vec::new();
    while fight.t < fight.rules.t_max {
        let seen = fight.events.len();
        sched.step(&mut fight);
        // the abilities the engine fired in this step; the one kind it has not ported is made by hand
        let new: Vec<Event> = fight.events[seen..]
            .iter()
            .filter(|e| e.kind == "ability")
            .cloned()
            .collect();
        for e in new {
            let who = e.who().unwrap_or_else(|| panic!("{name}: {e:?}"));
            match e.s(4) {
                "shield" => shield_by_hand(&mut fight, who, e.s(3)),
                "haste" => {}
                other => panic!("{name}: an ability of kind {other}: {e:?}"),
            }
            fired.push(e.s(3).to_string());
        }
        assert!(fight.balls.iter().all(|b| b.hp > 0.0), "{name}: no K.O. in the window");
        if sched.step % sched.steps_per_frame == 0 {
            frames.push(snap(&fight));
        }
    }

    // every event, the abilities too: in order, the same kinds, times and whole numbers
    let (rs_events, py_events) = (&fight.events, &py.events);
    let mut worst = 0.0f64;
    for (i, (a, b)) in rs_events.iter().zip(py_events).enumerate() {
        worst = worst.max(compare_event(&format!("{name}: event {i}"), a, b));
    }
    assert_eq!(
        rs_events.len(),
        py_events.len(),
        "{name}: after the first {}, Rust has {:?} and Python {:?}",
        rs_events.len().min(py_events.len()),
        rs_events.get(py_events.len()),
        py_events.get(rs_events.len())
    );
    // the export's own count of this recording's events (the abilities too)
    let mut counted: BTreeMap<String, usize> = BTreeMap::new();
    for e in &py.events {
        *counted.entry(e.kind.clone()).or_insert(0) += 1;
    }
    assert_eq!(counted, row.events, "{name}: the index's event counts");

    // every frame
    assert_eq!(frames.len(), py.frames(), "{name}: frames");
    let mut worst_by_key: BTreeMap<&str, f64> = BTreeMap::new();
    for (i, f) in frames.iter().enumerate() {
        for (key, got) in &f.floats {
            let n = got.len();
            let want = &floats(&py, key)[n * i..n * (i + 1)];
            for (a, b) in got.iter().zip(want) {
                let d = (a - b).abs();
                assert!(
                    d <= TOL,
                    "{name}: {key} at frame {i}: {a} vs Python's {b} ({d:e} apart)"
                );
                let w = worst_by_key.entry(key).or_insert(0.0);
                *w = w.max(d);
            }
        }
        for (key, got) in &f.wholes {
            assert_eq!(
                got[..],
                wholes(&py, key)[2 * i..2 * i + 2],
                "{name}: {key} at frame {i}"
            );
        }
    }
    worst = worst_by_key.values().fold(worst, |w, d| w.max(*d));

    let kinds = count_kinds(py_events);
    let py_fired: Vec<&str> = py_events
        .iter()
        .filter(|e| e.kind == "ability")
        .map(|e| e.s(3))
        .collect();
    assert_eq!(fired, py_fired, "{name}: the abilities, in order");
    let (bl, ha, sh) = (floats(&py, "BL"), floats(&py, "HA"), floats(&py, "SH"));
    let thrusts: Vec<f64> = py_events.iter().filter(|e| e.kind == "thrust").map(|e| e.t).collect();
    let beat = rule(&py, "beat");
    let span = Span {
        abilities: fired,
        reach: (0..py.frames()).map(|i| bl[2 * i + row.lance]).fold(0.0, f64::max),
        hits_at_cap: py_events
            .iter()
            .filter(|e| kind_of(e) == "hit/blade" && e.i(8) == FRETS_CAP)
            .count(),
        thrusts_on_8ths: thrusts.windows(2).filter(|w| w[1] - w[0] < 0.75 * beat).count(),
        frames_hasted: (0..py.frames()).filter(|i| ha[2 * i + row.lance] > 0.0).count(),
        frames_shielded: (0..py.frames()).filter(|i| sh[2 * i + 1 - row.lance] > 0.0).count(),
        kinds,
    };
    let by_key: Vec<String> = worst_by_key.iter().map(|(k, d)| format!("{k} {d:.1e}")).collect();
    println!(
        "{name}: {} frames and {} events agree ({:?}; abilities: {:?}); reach {}, {} hits at the cap, {} thrusts on \
         8ths; max |d|: events and frames {worst:.1e} ({})",
        frames.len(),
        py_events.len(),
        span.kinds,
        span.abilities,
        span.reach,
        span.hits_at_cap,
        span.thrusts_on_8ths,
        by_key.join(", "),
    );
    span
}

/// The fights of one kind, compared; panics if the index has none, or has the lance in one corner only.
fn compare_kind(kind: &str) -> Vec<(Row, Span)> {
    let index = index();
    let rows: Vec<Row> = index.fights.into_iter().filter(|f| f.kind == kind).collect();
    assert!(!rows.is_empty(), "the index has no {kind} fight");
    let spans: Vec<Span> = rows.iter().map(|row| compare(row, &index.added)).collect();
    for corner in 0..2 {
        assert!(
            rows.iter().any(|row| row.lance == corner),
            "{kind}: no fight has the lance in corner {corner}"
        );
    }
    rows.into_iter().zip(spans).collect()
}

fn count(span: &Span, kind: &str) -> usize {
    span.kinds.get(kind).copied().unwrap_or(0)
}

/// The TROMBONE against the plain bag with no level in reach: the lance alone for 16 s. A thrust on every beat (36
/// of them), hits and misses; in every one of these fights the lance grows to its cap (2.47, 20 frets) and lands
/// more thrusts at that length, which grow nothing.
#[test]
fn plain_fights_match_python() {
    let fights = compare_kind("plain");
    assert_eq!(fights.len(), 3);
    for (row, span) in &fights {
        assert_eq!(count(span, "thrust"), 36, "{}: a thrust on every beat", row.name);
        assert!(span.abilities.is_empty() && count(span, "level") == 0, "{}", row.name);
        assert_eq!(
            (span.thrusts_on_8ths, span.frames_hasted),
            (0, 0),
            "{}: no haste",
            row.name
        );
        assert_eq!(count(span, "block/blade"), 0, "{}", row.name);
        assert!(
            count(span, "hit/blade") < count(span, "thrust"),
            "{}: some thrusts miss",
            row.name
        );
        assert_eq!(
            span.reach.to_bits(),
            REACH_CAP.to_bits(),
            "{}: the reach at its cap",
            row.name
        );
        assert!(
            span.hits_at_cap >= 2,
            "{}: {} hits at the cap",
            row.name,
            span.hits_at_cap
        );
        // the growth is one fret a landed thrust until the cap: 19 of them, then the hits at the cap
        assert_eq!(count(span, "hit/blade"), 19 + span.hits_at_cap, "{}", row.name);
    }
}

/// Against the SHIELD BAG: once its shield is up (after its third hit, for 8 beats) the thrusts that reach it are
/// blocked: a `block` event each, no damage, no knock, no growth, and the lance pulls straight back.
#[test]
fn shield_fights_match_python() {
    let fights = compare_kind("shield");
    assert_eq!(fights.len(), 3);
    for (row, span) in &fights {
        assert_eq!(span.abilities, ["BRACE"], "{}", row.name);
        assert!(
            count(span, "block/blade") >= 5 && count(span, "hit/blade") >= 3,
            "{}: {:?}",
            row.name,
            span.kinds
        );
        // 8 beats of shield are 3.59 s: 107 or 108 frames
        assert!(
            (106..=109).contains(&span.frames_shielded),
            "{}: {}",
            row.name,
            span.frames_shielded
        );
        assert_eq!(count(span, "thrust"), 36, "{}", row.name);
    }
}

/// The LANCER against the plain bag, with a first level two hits away: POWER SLIDE (haste x 2 for 8 beats, again 6
/// bars later). Under it the lance thrusts on the 8th notes and each thrust runs in half the time.
#[test]
fn haste_fights_match_python() {
    let fights = compare_kind("haste");
    assert_eq!(fights.len(), 3);
    for (row, span) in &fights {
        assert!(
            !span.abilities.is_empty() && span.abilities.iter().all(|a| a == "POWER SLIDE"),
            "{}",
            row.name
        );
        assert_eq!(count(span, "level"), 1, "{}", row.name);
        // 8 beats of haste add 8 thrusts on the 8th notes between the beats (fewer when the window cuts the haste)
        assert!(span.thrusts_on_8ths >= 15, "{}: {}", row.name, span.thrusts_on_8ths);
        assert!(count(span, "thrust") >= 36 + 8, "{}: {:?}", row.name, span.kinds);
        assert!(span.frames_hasted >= 107, "{}: {}", row.name, span.frames_hasted);
    }
    assert!(
        fights.iter().any(|(_, s)| s.abilities.len() == 2),
        "a second POWER SLIDE, 6 bars after the first"
    );
    assert!(
        fights
            .iter()
            .any(|(_, s)| s.reach.to_bits() == REACH_CAP.to_bits() && s.hits_at_cap >= 1),
        "a hasted lance at its cap"
    );
}

/// `index.json` and the folder say the same: the listed recordings and no other, each the first `window` seconds of
/// a fight between a lance and a bag under the rules of its kind, and together they cover what the export promises.
#[test]
fn the_index_lists_the_fights() {
    let index = index();
    assert_eq!(index.fights.len(), 9);
    let added: Vec<&str> = index.added.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(added, ["BAG", "SHIELD BAG", "LANCER"]);
    // the LANCER is the roster's TROMBONE with POWER SLIDE as its first unlock and nothing else
    let roster = support::roster();
    let trombone = roster.iter().find(|f| f.name == "TROMBONE").expect("in the roster");
    let lancer = &index.added[2];
    assert_eq!(lancer.weapon, trombone.weapon);
    assert_eq!(lancer.unlock1, trombone.unlock2);
    assert_eq!(lancer.unlock1.name, "POWER SLIDE");
    assert_eq!(
        (lancer.unlock2.name.as_str(), lancer.low.name.as_str()),
        ("NOTHING", "NOTHING")
    );

    let folder = support::reference("lance_vs_bag");
    let mut on_disk: Vec<String> = std::fs::read_dir(&folder)
        .expect("the folder is there")
        .map(|e| e.expect("a folder entry").file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".bfrec"))
        .collect();
    on_disk.sort();
    let mut listed: Vec<String> = index.fights.iter().map(|f| format!("{}.bfrec", f.name)).collect();
    listed.sort();
    assert_eq!(on_disk, listed);
    let pair = |a: f64, b: f64| Value::List(vec![Value::Float(a), Value::Float(b)]);
    for row in &index.fights {
        let corner = ["left", "right"][row.lance];
        assert_eq!(row.name, format!("{}_{corner}_s{}", row.kind, row.seed));
        let py = support::python_recording(&format!("lance_vs_bag/{}.bfrec", row.name));
        assert_eq!(
            rule(&py, "t_max"),
            rule(&py, "t_fight") + index.window,
            "{}: the window",
            row.name
        );
        // 16 s at 30 frames a second, and frame f0
        assert_eq!(index.window, 16.0);
        assert_eq!(py.frames(), 481, "{}", row.name);
        // what each kind is: who fights, the levels, LOW_HP; the episode's handicap is on the lance's side
        let (who, bag, want_lv, want_low) = match row.kind.as_str() {
            "plain" => ("TROMBONE", "BAG", pair(1e9, 1e9), -1.0),
            "shield" => ("TROMBONE", "SHIELD BAG", pair(1e9, 1e9), 97.0),
            "haste" => ("LANCER", "BAG", pair(2.0, 1e9), -1.0),
            other => panic!("{}: a fight of kind {other}", row.name),
        };
        assert_eq!((row.fighter.as_str(), row.bag.as_str()), (who, bag), "{}", row.name);
        assert_eq!(py.rules.get("LV_DMG"), Some(&want_lv), "{}", row.name);
        assert_eq!(rule(&py, "LOW_HP"), want_low, "{}", row.name);
        let want_side = if row.lance == 0 { pair(0.4, 1.0) } else { pair(1.0, 0.4) };
        assert_eq!(py.rules.get("dmg_side"), Some(&want_side), "{}", row.name);
        // only the core and the lance act, and only the lance's owner thrusts
        for e in &py.events {
            match e.kind.as_str() {
                "thrust" | "level" => assert_eq!(e.who(), Some(row.lance), "{}: {e:?}", row.name),
                "hit" => assert_eq!((e.who(), e.s(7)), (Some(row.lance), "blade"), "{}: {e:?}", row.name),
                "block" => assert_eq!((e.who(), e.s(6)), (Some(1 - row.lance), "blade"), "{}: {e:?}", row.name),
                "ability" | "rope" | "clash" => {}
                other => panic!("{}: a {other} event", row.name),
            }
        }
    }
    for item in &index.must_cover {
        assert!(
            index.fights.iter().any(|f| f.covers.contains(item)),
            "no fight covers {item}"
        );
    }
    for item in [
        "left",
        "right",
        "ability/POWER SLIDE",
        "ability/BRACE",
        "block/blade",
        "miss",
        "hit_held",
        "thrust_on_8th",
        "reach_cap",
        "hit_at_cap",
    ] {
        assert!(index.must_cover.iter().any(|c| c == item), "{item} is not required");
    }
}
