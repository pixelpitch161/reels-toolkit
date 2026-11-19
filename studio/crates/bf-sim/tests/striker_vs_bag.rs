//! Striker parity with the Python kit, past what the FLUTE against TROMBONE recordings can show: the FLUTE against
//! a bag.
//!
//! In every FLUTE against TROMBONE recording the lance lands before a cobra has lunged, so the span the lance leaves
//! alone holds basket drops only (`striker_prefix.rs`). In `studio/reference/fights/striker_vs_bag/` the FLUTE
//! fights a bag, a fighter with no weapon and abilities that do nothing, so nothing but the engine core and the
//! striker acts: baskets drop and are taken back, cobras rise, lunge, bite and miss, the reach grows to its cap and
//! the lunge shortens to its floor, FLUTTER TONGUE doubles the strike grid, SNAKE DANCE takes the cobras off their
//! baskets, and a SHIELD BAG blocks bites. The recordings are Python's (`python -m tools.studio_export striker_bag`,
//! whose docstring has the rules of each kind of fight); each holds the first 10 s of a fight, and `index.json`
//! lists them with the two bag entries the export added to the kit's roster table.
//!
//! Each test replays the Rust fight of a recording's seed, fighters and rules and compares EVERY frame (`P`, `V`,
//! `HP`, `BL`, `SH`, `HA`, `LV` and the `COB` rows) and EVERY event to 1e-9 (times, kinds and whole numbers
//! exactly).
//!
//! The engine fires the abilities itself: FLUTTER TONGUE and SNAKE DANCE are compared like everything else, their
//! `ability` events too, and so is the SHIELD BAG's `shield` (BRACE), ported in milestone 3 (Task 3): the engine
//! raises it itself.
//!
//! The windows end long before the two engines drift apart. When the striker was ported the same comparison was run
//! over whole fights for seeds 1 to 40 in both corners, with and without early levels (160 fights): every fight
//! agreed to 1e-9 for at least 13.4 s of fighting (half of them for 22 s), every first difference was a float
//! drifting past the tolerance in an event that was otherwise the same, and the kinds, times and whole numbers of
//! all events were equal for at least 27 s of fighting in every fight. The drift starts from last-bit differences in
//! the arithmetic (the maths library, numpy's fused dot product) and grows with the fight's chaos. The recordings
//! are Python's and do not change with the engine: if a change to the engine's arithmetic ever moves a window past
//! the tolerance, shorten `WINDOW` in the export, with the cause written down.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bf_core::event::{Event, Value};
use bf_core::recording::{Array, Recording};
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::schedule::Schedule;
use bf_sim::{Fight, Rules, Setup};
use serde_json::Value as Json;

/// Positions, velocities, hit points, the reach, the timers, the cobra rows and the float fields of events agree to
/// this.
const TOL: f64 = 1e-9;

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../reference/fights/striker_vs_bag")
}

/// One fight of `index.json`.
#[derive(Debug)]
struct Row {
    /// Its name: `<kind>_<corner>_s<seed>`.
    name: String,
    /// `episode`, `even`, `early` or `shield`.
    kind: String,
    /// The FLUTE's corner: 0 left, 1 right.
    flute: usize,
    /// The seed.
    seed: u64,
    /// The bag it fights: `BAG` or `SHIELD BAG`.
    bag: String,
    /// Its events by kind, as the export counted them.
    events: BTreeMap<String, usize>,
    /// What the export says it covers.
    covers: Vec<String>,
}

/// `index.json`: the two bags (roster entries, read by the roster's own reader), the fights, what the set must
/// cover, and the seconds of fighting each recording holds.
struct Index {
    bags: Vec<FighterCfg>,
    fights: Vec<Row>,
    must_cover: Vec<String>,
    window: f64,
}

fn index() -> Index {
    let path = folder().join("index.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: Json = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let text_of = |j: &Json| j.as_str().unwrap_or_else(|| panic!("{j} is not a text")).to_string();
    let whole = |j: &Json| j.as_u64().unwrap_or_else(|| panic!("{j} is not a whole number"));
    let list = |j: &Json| j.as_array().unwrap_or_else(|| panic!("{j} is not a list")).clone();
    let row = |f: &Json| Row {
        name: text_of(&f["name"]),
        kind: text_of(&f["kind"]),
        flute: whole(&f["flute"]) as usize,
        seed: whole(&f["seed"]),
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
        bags: parse_roster(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
        fights: list(&doc["fights"]).iter().map(row).collect(),
        must_cover: list(&doc["must_cover"]).iter().map(text_of).collect(),
        window: doc["window"].as_f64().expect("the window in seconds"),
    }
}

fn rule(rec: &Recording, key: &str) -> f64 {
    match rec.rules.get(key) {
        Some(Value::Float(x)) => *x,
        Some(Value::Int(n)) => *n as f64,
        other => panic!("rules.{key} is {other:?}"),
    }
}

fn floats<'a>(rec: &'a Recording, key: &str) -> &'a [f64] {
    match rec.arrays.get(key) {
        Some(Array::F64 { data, .. }) => data,
        other => panic!("{key} is not a float array: {other:?}"),
    }
}

/// What a frame holds of the fight (`Fight.snap`), for the keys compared.
struct Frame {
    /// `P`: x, y of the left ball, then of the right one.
    p: [f64; 4],
    /// `V`.
    v: [f64; 4],
    /// `HP`.
    hp: [f64; 2],
    /// `BL`: the reach.
    bl: [f64; 2],
    /// `SH`: seconds of shield left.
    sh: [f64; 2],
    /// `HA`: seconds of haste left.
    ha: [f64; 2],
    /// `LV`.
    lv: [i64; 2],
    /// The `COB` row: for each fighter its baskets as `[id, home x, home y, cobra x, cobra y, t_drop]`.
    cob: Value,
}

fn snap(fight: &Fight) -> Frame {
    let [a, b] = &fight.balls;
    let sides = fight.striker.iter().map(|st| {
        let baskets = st.iter().flat_map(|st| &st.baskets).map(|bk| {
            Value::List(vec![
                Value::Int(bk.id),
                Value::Float(bk.home[0]),
                Value::Float(bk.home[1]),
                Value::Float(bk.cp[0]),
                Value::Float(bk.cp[1]),
                Value::Float(bk.t_drop),
            ])
        });
        Value::List(baskets.collect())
    });
    Frame {
        p: [a.p[0], a.p[1], b.p[0], b.p[1]],
        v: [a.v[0], a.v[1], b.v[0], b.v[1]],
        hp: [a.hp.max(0.0), b.hp.max(0.0)],
        bl: [a.blade_len, b.blade_len],
        sh: [a.shield, b.shield],
        ha: [a.haste, b.haste],
        lv: [a.level, b.level],
        cob: Value::List(sides.collect()),
    }
}

/// The largest difference between two lists of floats; panics past `TOL`.
fn compare_floats(what: &str, rs: &[f64], py: &[f64]) -> f64 {
    assert_eq!(rs.len(), py.len(), "{what}");
    let mut worst = 0.0f64;
    for (a, b) in rs.iter().zip(py) {
        let d = (a - b).abs();
        assert!(d <= TOL, "{what}: {a} vs Python's {b} ({d:e} apart)");
        worst = worst.max(d);
    }
    worst
}

/// The largest float difference between two values of the same layout; panics where the layouts differ or a float
/// differs by more than `TOL`.
fn compare_values(what: &str, a: &Value, b: &Value) -> f64 {
    match (a, b) {
        (Value::List(x), Value::List(y)) => {
            assert_eq!(x.len(), y.len(), "{what}: {a:?} vs Python's {b:?}");
            x.iter()
                .zip(y)
                .map(|(p, q)| compare_values(what, p, q))
                .fold(0.0, f64::max)
        }
        (Value::Float(x), Value::Float(y)) => {
            let d = (x - y).abs();
            assert!(d <= TOL, "{what}: {x} vs Python's {y} ({d:e} apart)");
            d
        }
        _ => {
            assert_eq!(a, b, "{what}");
            0.0
        }
    }
}

/// What one comparison covered.
struct Span {
    /// Events matched, by kind.
    kinds: BTreeMap<String, usize>,
    /// The abilities the engine fired, by name.
    abilities: Vec<String>,
    /// Frames in which a cobra is off its basket.
    frames_off_basket: usize,
    /// The FLUTE's largest reach.
    reach: f64,
    /// The shortest time from a lunge's start to its bite.
    quickest: f64,
}

/// Runs the Rust fight of a recording (its fighters, rules and seed), and compares every frame and every event.
fn compare(row: &Row, bags: &[FighterCfg]) -> Span {
    let name = row.name.as_str();
    let path = folder().join(format!("{name}.bfrec"));
    let py = Recording::read(&path).unwrap_or_else(|e| panic!("{e}"));
    let roster = parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads");
    let find = |who: &str| {
        roster
            .iter()
            .chain(bags)
            .find(|f| f.name == who)
            .unwrap_or_else(|| panic!("{name}: {who} is neither in the roster nor a bag"))
    };
    let mut names = [row.bag.as_str(); 2];
    names[row.flute] = "FLUTE";
    assert_eq!(py.names, names, "{name}");
    assert_eq!((py.seed, py.winner, py.t_ko), (row.seed, None, None), "{name}");
    let mut rules = Rules::new(rule(&py, "bpm"), rule(&py, "t_fight"));
    rules.apply(&py.rules).expect("the recording's rules apply");
    let setup = Setup {
        fighters: [find(names[0]), find(names[1])],
        rules,
        seed: py.seed,
    };

    // the fight loop (`Fight.run`): a frame before the first step and after every `steps_per_frame` steps
    let mut fight = Fight::with_unported(&setup, false).expect("the engine runs the bags (kinds not ported)");
    let mut sched = Schedule::new(&mut fight);
    fight.t = fight.rules.t_fight;
    let mut frames = vec![snap(&fight)];
    let mut fired = Vec::new();
    while fight.t < fight.rules.t_max {
        let seen = fight.events.len();
        sched.step(&mut fight);
        // the abilities the engine fired in this step (all three kinds of these fights are ported, the bag's shield too)
        let new: Vec<Event> = fight.events[seen..]
            .iter()
            .filter(|e| e.kind == "ability")
            .cloned()
            .collect();
        for e in new {
            match e.s(4) {
                "haste" | "dance" | "shield" => {}
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
    let (mut kinds, mut max_dev, mut quickest) = (BTreeMap::new(), 0.0f64, f64::INFINITY);
    for (i, (a, b)) in rs_events.iter().zip(py_events).enumerate() {
        let what = format!("{name}: event {i}:\n Rust   {a:?}\n Python {b:?}\n");
        assert_eq!((a.t.to_bits(), &a.kind), (b.t.to_bits(), &b.kind), "{what}");
        let (a_fields, b_fields) = (Value::List(a.fields.clone()), Value::List(b.fields.clone()));
        max_dev = max_dev.max(compare_values(&what, &a_fields, &b_fields));
        *kinds.entry(b.kind.clone()).or_insert(0) += 1;
        if b.kind == "strike" {
            quickest = quickest.min(b.f(6));
        }
    }
    assert_eq!(
        rs_events.len(),
        py_events.len(),
        "{name}: after the first {}, Rust has {:?} and Python {:?}",
        rs_events.len().min(py_events.len()),
        rs_events.get(py_events.len()),
        py_events.get(rs_events.len())
    );
    // the export's own count of this recording's events
    assert_eq!(kinds, row.events, "{name}: the index's event counts");
    assert_eq!(kinds.get("ability").copied().unwrap_or(0), fired.len(), "{name}");

    // every frame
    assert_eq!(frames.len(), py.frames(), "{name}: frames");
    let (p, v, hp, bl) = (floats(&py, "P"), floats(&py, "V"), floats(&py, "HP"), floats(&py, "BL"));
    let (sh, ha) = (floats(&py, "SH"), floats(&py, "HA"));
    let Some(Array::I64 { data: lv, .. }) = py.arrays.get("LV") else {
        panic!("{name}: LV is not a whole-number array")
    };
    let Some(Array::Nested(Value::List(cob))) = py.arrays.get("COB") else {
        panic!("{name}: COB is not a list per frame")
    };
    let mut worst = [0.0f64; 7];
    let (mut frames_off_basket, mut reach) = (0, 0.0f64);
    for (i, f) in frames.iter().enumerate() {
        let what = |key: &str| format!("{name}: {key} at frame {i}");
        let d = [
            compare_floats(&what("P"), &f.p, &p[4 * i..4 * i + 4]),
            compare_floats(&what("V"), &f.v, &v[4 * i..4 * i + 4]),
            compare_floats(&what("HP"), &f.hp, &hp[2 * i..2 * i + 2]),
            compare_floats(&what("BL"), &f.bl, &bl[2 * i..2 * i + 2]),
            compare_floats(&what("SH"), &f.sh, &sh[2 * i..2 * i + 2]),
            compare_floats(&what("HA"), &f.ha, &ha[2 * i..2 * i + 2]),
            compare_values(&what("COB"), &f.cob, &cob[i]),
        ];
        for (w, d) in worst.iter_mut().zip(d) {
            *w = w.max(d);
        }
        assert_eq!(f.lv[..], lv[2 * i..2 * i + 2], "{}", what("LV"));
        let Value::List(sides) = &cob[i] else {
            panic!("{}: {:?}", what("COB"), cob[i])
        };
        let Value::List(baskets) = &sides[row.flute] else {
            panic!("{}: {:?}", what("COB"), cob[i])
        };
        // a row is [id, home x, home y, cobra x, cobra y, t_drop]
        let off = |b: &Value| matches!(b, Value::List(f) if f[1] != f[3] || f[2] != f[4]);
        frames_off_basket += usize::from(baskets.iter().any(off));
        reach = reach.max(bl[2 * i + row.flute]);
    }
    let [dp, dv, dhp, dbl, dsh, dha, dcob] = worst;
    println!(
        "{name}: {} frames and {} events agree ({kinds:?}; abilities: {fired:?}); max |d|: events {max_dev:.1e}, P \
         {dp:.1e}, V {dv:.1e}, HP {dhp:.1e}, BL {dbl:.1e}, SH {dsh:.1e}, HA {dha:.1e}, COB {dcob:.1e}",
        frames.len(),
        py_events.len(),
    );
    Span {
        kinds,
        abilities: fired,
        frames_off_basket,
        reach,
        quickest,
    }
}

/// The fights of one kind, compared; panics if the index has none, or has the FLUTE in one corner only.
fn compare_kind(kind: &str) -> Vec<(Row, Span)> {
    let index = index();
    let rows: Vec<Row> = index.fights.into_iter().filter(|f| f.kind == kind).collect();
    assert!(!rows.is_empty(), "the index has no {kind} fight");
    let spans: Vec<Span> = rows.iter().map(|row| compare(row, &index.bags)).collect();
    for corner in 0..2 {
        assert!(
            rows.iter().any(|row| row.flute == corner),
            "{kind}: no fight has the FLUTE in corner {corner}"
        );
    }
    rows.into_iter().zip(spans).collect()
}

fn count(span: &Span, kind: &str) -> usize {
    span.kinds.get(kind).copied().unwrap_or(0)
}

fn fired(span: &Span, name: &str) -> bool {
    span.abilities.iter().any(|a| a == name)
}

/// The episode's own rules (the FLUTE's bites take 2.16): lunges, bites, the first level and FLUTTER TONGUE (lunges
/// on the 8th notes at half the time), in both corners; the reach grows to its cap in two of them.
#[test]
fn episode_fights_match_python() {
    let fights = compare_kind("episode");
    assert_eq!(fights.len(), 4);
    for (row, span) in &fights {
        assert!(
            count(span, "strike") >= 13 && count(span, "hit") >= 11,
            "{}: {:?}",
            row.name,
            span.kinds
        );
        assert_eq!(span.abilities, ["FLUTTER TONGUE"], "{}", row.name);
        assert!(span.quickest < 0.06, "{}: {}", row.name, span.quickest);
    }
    assert!(fights.iter().filter(|(_, s)| s.reach == 2.4).count() >= 2);
}

/// No handicap and the engine's own levels: the longest runs of plain lunges, no ability at all.
#[test]
fn even_fights_match_python() {
    let fights = compare_kind("even");
    assert_eq!(fights.len(), 2);
    for (row, span) in &fights {
        assert!(
            count(span, "strike") >= 12 && count(span, "hit") >= 10,
            "{}: {:?}",
            row.name,
            span.kinds
        );
        assert!(span.abilities.is_empty() && span.frames_off_basket == 0, "{}", row.name);
        assert!(span.quickest >= 0.07, "{}: no haste", row.name);
    }
}

/// Early levels: both unlocks in every fight. With FLUTTER TONGUE the cobras lunge on the 8th notes at half the
/// time, with SNAKE DANCE they leave their baskets; the reach reaches its cap (2.4) and the lunge its floor (0.07,
/// so 0.035 under haste).
#[test]
fn early_fights_match_python() {
    let fights = compare_kind("early");
    assert_eq!(fights.len(), 4);
    for (row, span) in &fights {
        assert!(
            count(span, "strike") >= 19 && count(span, "level") == 2,
            "{}: {:?}",
            row.name,
            span.kinds
        );
        assert!(
            fired(span, "FLUTTER TONGUE") && fired(span, "SNAKE DANCE"),
            "{}: {:?}",
            row.name,
            span.abilities
        );
        assert!(span.frames_off_basket >= 20, "{}: {}", row.name, span.frames_off_basket);
        assert_eq!(span.reach, 2.4, "{}: the reach is at max_reach", row.name);
        assert!(span.quickest <= 0.07, "{}: {}", row.name, span.quickest);
    }
    assert!(
        fights.iter().any(|(_, s)| s.quickest == 0.035),
        "a hasted lunge at t_min"
    );
}

/// Against the SHIELD BAG: once its shield is up (after its fourth bite, for 8 beats) the bites are blocked: a
/// `block` event each, no damage, no knock, no growth.
#[test]
fn shield_fights_match_python() {
    let fights = compare_kind("shield");
    assert_eq!(fights.len(), 2);
    for (row, span) in &fights {
        assert!(
            count(span, "block") >= 9 && count(span, "hit") >= 4,
            "{}: {:?}",
            row.name,
            span.kinds
        );
        assert!(fired(span, "BRACE"), "{}: {:?}", row.name, span.abilities);
        // a blocked bite grows nothing: the reach stays under its cap
        assert!(span.reach < 2.4, "{}: {}", row.name, span.reach);
    }
}

/// `index.json` and the folder say the same: the listed recordings and no other, each the first `window` seconds of
/// a fight between the FLUTE and a bag under the rules of its kind, and together they cover what the export
/// promises.
#[test]
fn the_index_lists_the_fights() {
    let index = index();
    assert_eq!(index.fights.len(), 12);
    let bag_names: Vec<&str> = index.bags.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(bag_names, ["BAG", "SHIELD BAG"]);
    let mut on_disk: Vec<String> = std::fs::read_dir(folder())
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
        let corner = ["left", "right"][row.flute];
        assert_eq!(row.name, format!("{}_{corner}_s{}", row.kind, row.seed));
        assert_eq!(
            row.bag,
            if row.kind == "shield" { "SHIELD BAG" } else { "BAG" },
            "{}",
            row.name
        );
        let py = Recording::read(&folder().join(format!("{}.bfrec", row.name))).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            rule(&py, "t_max"),
            rule(&py, "t_fight") + index.window,
            "{}: the window",
            row.name
        );
        // 10 s at 30 frames a second, and frame f0
        assert_eq!(py.frames(), 301, "{}", row.name);
        // what each kind overrides: the levels, LOW_HP and the handicap
        let (want_lv, want_low, want_side) = match (row.kind.as_str(), row.flute) {
            ("episode", 0) => (pair(14.0, 36.0), -1.0, pair(2.0, 0.4)),
            ("episode", _) => (pair(14.0, 36.0), -1.0, pair(0.4, 2.0)),
            ("even", _) => (pair(14.0, 36.0), -1.0, pair(1.0, 1.0)),
            ("early", _) => (pair(2.0, 5.0), -1.0, pair(1.0, 1.0)),
            ("shield", _) => (pair(2.0, 5.0), 96.0, pair(1.0, 1.0)),
            (other, _) => panic!("{}: a fight of kind {other}", row.name),
        };
        assert_eq!(py.rules.get("LV_DMG"), Some(&want_lv), "{}", row.name);
        assert_eq!(rule(&py, "LOW_HP"), want_low, "{}", row.name);
        assert_eq!(py.rules.get("dmg_side"), Some(&want_side), "{}", row.name);
        // only the core and the striker act, and only the FLUTE lunges
        for e in &py.events {
            match e.kind.as_str() {
                "basket" | "strike" | "level" => assert_eq!(e.who(), Some(row.flute), "{}: {e:?}", row.name),
                "hit" => assert_eq!((e.who(), e.s(7)), (Some(row.flute), "fang"), "{}: {e:?}", row.name),
                "block" => assert_eq!((e.who(), e.s(6)), (Some(1 - row.flute), "fang"), "{}: {e:?}", row.name),
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
        "ability/FLUTTER TONGUE",
        "ability/SNAKE DANCE",
        "ability/BRACE",
        "block/fang",
        "miss",
        "taken_back",
        "reach_cap",
        "t_min",
    ] {
        assert!(index.must_cover.iter().any(|c| c == item), "{item} is not required");
    }
}
