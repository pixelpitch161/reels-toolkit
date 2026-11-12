//! The fight half of Template F: `fighters/<slug>/fight.json` against the Rust engine.
//!
//! Every fight the suites run goes through [`traced`]: the engine's traced run, whose socket log must reach no
//! `stub:` socket (a socket a port has left empty after setting its kind's `PORTED`, Task 3's guard).

use std::collections::BTreeMap;

use bf_core::event::{Event, Value as Ev};
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::FighterCfg;
use bf_sim::parity::{BreakKind, PREFIX_FLOOR_S, PREFIX_TOL, prefix_window};
use bf_sim::{Rules, Setup, SimError, setup_from};
use bf_story::{BeatSheet, Stats, stats as stats_of};
use serde_json::Value;

use crate::Sim;
use crate::refs::{
    cached_recording, committed_recording, fighter_folder, index, json, list, num, rules_map, slug, studio, text, value,
};

/// `bf_sim::run_traced`'s type.
pub type Traced = fn(&Setup, bool) -> Result<(Recording, Vec<&'static str>), SimError>;

/// The engine's fight of `setup`, traced: fails when any socket stub was reached (the stub's name says which kind
/// a port left empty).
pub fn traced(sim: &Sim, setup: &Setup, record: bool, what: &str) -> Recording {
    let (rec, log) = (sim.run_traced)(setup, record).unwrap_or_else(|e| panic!("{what}: the engine refuses: {e}"));
    let stubs: Vec<&str> = log.iter().copied().filter(|s| s.starts_with("stub:")).collect();
    if !stubs.is_empty() {
        let mut seen: Vec<&str> = stubs.clone();
        seen.sort_unstable();
        seen.dedup();
        panic!(
            "{what}: the fight reached socket stubs that are still empty ({} calls): {seen:?}. A kind whose PORTED is \
             true must fill every socket stub of its module (Task 3, studio/docs/porting-a-fighter.md)",
            stubs.len()
        );
    }
    rec
}

/// The roster the engine reads (the app's compiled-in one).
pub fn roster() -> &'static [FighterCfg] {
    bf_content::roster()
}

pub fn fighter(name: &str) -> &'static FighterCfg {
    roster()
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the roster"))
}

/// A fighter's `fight.json`.
pub fn doc(name: &str) -> Value {
    let d = json(&fighter_folder(name), "fight.json");
    assert_eq!(text(&d["fighter"]), name);
    d
}

/// The fight file of the folder's fights: the base example's (`studio/fights/<base>.toml`) with the config's
/// fighters, handicap, damage scale and rules; `swapped` puts the opponent in the left corner. Music settings the
/// base names for a fighter not in this fight are left out (the kit reads them by name and never meets them).
pub fn fight_file(doc: &Value, swapped: bool) -> FightFile {
    let c = &doc["config"];
    let mut fighters: Vec<String> = list(&c["fighters"]).iter().map(|f| text(f).to_string()).collect();
    if swapped {
        fighters.reverse();
    }
    fight_file_of(doc, &fighters)
}

/// The fight file of the folder's config with these two fighters (a scenario's, which may name another
/// opponent): the config's handicap, reversed when the fighter is not in the left corner.
pub fn fight_file_of(doc: &Value, fighters: &[String]) -> FightFile {
    let c = &doc["config"];
    let base = text(&c["base"]);
    let path = studio(&format!("fights/{base}.toml"));
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut ff = FightFile::parse(&src).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let fighters = fighters.to_vec();
    let mut handicap: Vec<f64> = list(&c["handicap"]).iter().map(num).collect();
    if fighters[0] != text(&doc["fighter"]) {
        handicap.reverse();
    }
    ff.fighters = fighters.clone();
    ff.handicap = handicap;
    ff.dmg_scale = num(&c["dmg_scale"]);
    ff.pair_rules = c["rules_on"]
        .as_object()
        .expect("rules_on")
        .iter()
        .map(|(k, v)| (k.clone(), value(v)))
        .collect();
    let keep = |k: &String| fighters.contains(k);
    ff.music.roles.retain(|k, _| keep(k));
    ff.music.lines.retain(|k, _| {
        fighters
            .iter()
            .any(|f| k == f || bf_core::fightfile::is_extra_line(k, f))
    });
    ff.music.solos.retain(|k, _| keep(k));
    ff.music.title_hits.retain(|k, _| keep(k));
    ff
}

/// The same fight as a Python recording: its fighters in its order, its rules, its seed (milestone 1's
/// `setup_like`).
pub fn setup_like(py: &Recording) -> Setup<'static> {
    let rule = |k: &str| match py.rules.get(k) {
        Some(Ev::Float(x)) => *x,
        Some(Ev::Int(n)) => *n as f64,
        other => panic!("rules.{k} is {other:?}"),
    };
    let mut rules = Rules::new(rule("bpm"), rule("t_fight"));
    rules.apply(&py.rules).expect("the recording's rules apply");
    Setup {
        fighters: [fighter(&py.names[0]), fighter(&py.names[1])],
        rules,
        seed: py.seed,
    }
}

/// A scenario of `fight.json` (by name) as the engine's setup of one seed: its fighters and its full rules.
fn scenario_setup(doc: &Value, scenario: &str, seed: u64) -> Setup<'static> {
    let s = list(&doc["scenarios"])
        .iter()
        .find(|s| text(&s["name"]) == scenario)
        .unwrap_or_else(|| panic!("fight.json has no scenario {scenario:?}"));
    setup_of(&s["fighters"], &s["rules"], seed)
}

/// The setup of two fighters (a JSON list of names) under a rules table, for one seed.
pub fn setup_of(fighters: &Value, rules: &Value, seed: u64) -> Setup<'static> {
    let map = rules_map(rules);
    let f = |k: &str| match map.get(k) {
        Some(Ev::Float(x)) => *x,
        Some(Ev::Int(n)) => *n as f64,
        other => panic!("rules.{k} is {other:?}"),
    };
    let mut r = Rules::new(f("bpm"), f("t_fight"));
    r.apply(&map).unwrap_or_else(|e| panic!("the scenario's rules: {e}"));
    let names = list(fighters);
    Setup {
        fighters: [fighter(text(&names[0])), fighter(text(&names[1]))],
        rules: r,
        seed,
    }
}

/// The item an event stands for in a coverage table: its kind, `hit/<what hit>`, `ability/<name>`.
pub fn items_of(e: &Event) -> Vec<String> {
    match e.kind.as_str() {
        "hit" => vec!["hit".to_string(), format!("hit/{}", e.s(7))],
        "ability" => vec!["ability".to_string(), format!("ability/{}", e.s(3))],
        kind => vec![kind.to_string()],
    }
}

/// An event of a reference table: `[t, kind, who, ...]`.
pub fn event(j: &Value) -> Event {
    let items = list(j);
    Event {
        t: num(&items[0]),
        kind: text(&items[1]).to_string(),
        fields: items[2..].iter().map(value).collect(),
    }
}

/// Kind, time, whole numbers and texts exactly, floats to 1e-6.
pub fn same_event(a: &Event, b: &Event) -> bool {
    fn close(a: &Ev, b: &Ev) -> bool {
        match (a, b) {
            (Ev::Float(x), Ev::Float(y)) => (x - y).abs() <= PREFIX_TOL,
            (Ev::List(x), Ev::List(y)) => x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(p, q)),
            _ => a == b,
        }
    }
    a.kind == b.kind
        && a.t.to_bits() == b.t.to_bits()
        && a.fields.len() == b.fields.len()
        && a.fields.iter().zip(&b.fields).all(|(p, q)| close(p, q))
}

/// A row of a coverage table (`item`, `index`, `event`) occurs in the engine's run of `setup`: the same event at the
/// same place, the first of its item there. `None` when it does, else what was found.
pub fn row_occurs(rs: &Recording, row: &Value) -> Option<String> {
    let item = text(&row["item"]);
    let want = event(&row["event"]);
    let i = index(&row["index"]);
    let first = rs.events.iter().position(|e| items_of(e).iter().any(|x| x == item));
    let found = rs.events.get(i).is_some_and(|e| same_event(e, &want));
    if found && first == Some(i) {
        return None;
    }
    let near: Vec<String> = rs
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| (e.t - want.t).abs() < 0.05)
        .map(|(j, e)| format!("\n     {j}: {e:?}"))
        .collect();
    Some(format!(
        "{item}: Python's event {i} is {want:?}\n   Rust event {i}: {:?}; the first {item} in the Rust fight is event \
         {first:?}\n   Rust events within 0.05 s:{}",
        rs.events.get(i),
        near.concat()
    ))
}

/// Every row of `fight.json`'s `coverage` occurs in the Rust run of its scenario (its seed and full rules; the run
/// needs no cache) at its time with its fields, as the first of its item there; every required item has a row
/// within the window; every ability of the fighter is among them. Always runs.
pub fn coverage(sim: &Sim, name: &str) {
    let doc = doc(name);
    let cov = &doc["coverage"];
    let window = num(&cov["window"]);
    let rows = list(&cov["rows"]);
    let required: Vec<&str> = list(&cov["required"]).iter().map(text).collect();
    let mut runs: BTreeMap<(String, u64), Recording> = BTreeMap::new();
    let mut missing = Vec::new();
    let mut covered = Vec::new();
    for row in rows {
        let (scenario, seed) = (
            text(&row["scenario"]).to_string(),
            row["seed"].as_u64().expect("a seed"),
        );
        let rs = runs.entry((scenario.clone(), seed)).or_insert_with(|| {
            traced(
                sim,
                &scenario_setup(&doc, &scenario, seed),
                false,
                &format!("{name} {scenario} seed {seed}"),
            )
        });
        assert!(num(&row["after_fight"]) <= window || !row["required"].as_bool().unwrap_or(false));
        match row_occurs(rs, row) {
            None => covered.push(text(&row["item"]).to_string()),
            Some(why) => missing.push(format!("{scenario} seed {seed}: {why}")),
        }
    }
    assert!(
        missing.is_empty(),
        "{name}: {} of {} coverage rows do not occur in the Rust fight as in Python's:\n\n{}",
        missing.len(),
        rows.len(),
        missing.join("\n\n")
    );
    for item in &required {
        assert!(
            covered.iter().any(|c| c == item),
            "{name}: no coverage row covers the required {item}"
        );
    }
    for ab in list(&doc["abilities"]) {
        let want = format!("ability/{}", text(ab));
        assert!(required.contains(&want.as_str()), "{name}: {want} is not required");
    }
    assert!(
        !rows.is_empty() && required.len() >= 4,
        "{name}: an empty coverage table proves nothing"
    );
    println!(
        "{name}: {} coverage rows of {} scenario fights occur in the Rust fights ({} required)",
        rows.len(),
        runs.len(),
        required.len()
    );
}

/// The seeds of the self-replay file: three small ones, the episode's search range and the first balance seed.
pub const REPLAY_SEEDS: [u64; 5] = [1, 2, 3, 500, 1 << 40];

fn replay_setups(doc: &Value) -> Vec<(FightFile, u64)> {
    let mut out = Vec::new();
    for swapped in [false, true] {
        let ff = fight_file(doc, swapped);
        for seed in REPLAY_SEEDS {
            out.push((ff.clone(), seed));
        }
    }
    out
}

/// `studio/replay/fighters/<slug>.txt`: the fighter's fights at the folder's config, both side orders, seeds
/// [`REPLAY_SEEDS`], one line each (`left right seed winner t_ko n_events hash`, milestone 1's `replay.rs`).
/// `BF_BLESS=1` writes the file instead of checking it; a missing file fails with that command.
pub fn replay(sim: &Sim, name: &str) {
    let doc = doc(name);
    let mut lines = Vec::new();
    for (ff, seed) in replay_setups(&doc) {
        let setup = setup_from(&ff, roster(), seed).unwrap_or_else(|e| panic!("{name}: the fight file: {e}"));
        let rec = traced(sim, &setup, true, &format!("{name} replay seed {seed}"));
        let dash = |x: Option<String>| x.unwrap_or_else(|| "-".to_string());
        let l = format!(
            "{} {} {} {} {} {} {}",
            setup.fighters[0].name,
            setup.fighters[1].name,
            setup.seed,
            dash(rec.winner.map(|w| w.to_string())),
            dash(rec.t_ko.map(|t| t.to_string())),
            rec.events.len(),
            rec.hash()
        );
        println!("{l}");
        lines.push(l);
    }
    let header = format!(
        "# The engine's self-replay hashes of {name} (studio/reference/fighters/{}/fight.json's config): both side \
         orders,\n# seeds 1, 2, 3, 500 and 1 << 40. Written by the proving harness with BF_BLESS=1 on the Windows \
         reference PC;\n# every PC and every build must give the same lines. Do not edit by hand.\n# left right seed \
         winner t_ko n_events hash\n",
        slug(name)
    );
    let body = format!("{header}{}\n", lines.join("\n"));
    let path = studio(&format!("replay/fighters/{}.txt", slug(name)));
    if std::env::var_os("BF_BLESS").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("studio/replay/fighters");
        std::fs::write(&path, &body).expect("write the replay file");
        eprintln!("blessed {}", path.display());
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "studio/replay/fighters/{}.txt is missing: bless it once the fights agree with the kit (BF_BLESS=1 cargo \
             test -p bf-harness --test fighter_{} replay)",
            slug(name),
            slug(name)
        )
    });
    assert_eq!(
        want, body,
        "{name}: the engine's fights are not the blessed ones (studio/replay/fighters/)"
    );
}

/// The events, the winner and the K.O. are the same with the recording on and off.
pub fn record_flag(sim: &Sim, name: &str) {
    let doc = doc(name);
    for (ff, seed) in replay_setups(&doc) {
        let setup = setup_from(&ff, roster(), seed).expect("the fight file sets up");
        let seen = traced(sim, &setup, true, name);
        let blind = traced(sim, &setup, false, name);
        assert!(seen.frames() > 0 && blind.arrays.is_empty(), "{name} seed {seed}");
        assert_eq!(seen.events, blind.events, "{name} seed {seed}");
        assert_eq!(
            (seen.winner, seen.t_ko.map(f64::to_bits)),
            (blind.winner, blind.t_ko.map(f64::to_bits)),
            "{name} seed {seed}"
        );
    }
}

/// The committed `left` fight is the fight of the folder's fight file at its seed (the fight file the look and
/// sound gates draw with), and the engine agrees with it for the prefix floor.
pub fn left_fight(sim: &Sim, name: &str) {
    let doc = doc(name);
    let left = &doc["left"];
    let py = committed_recording(text(&left["file"]), Some(text(&left["hash"])));
    let ff = fight_file(&doc, false);
    let setup = setup_from(&ff, roster(), py.seed).expect("the fight file sets up");
    let like = setup_like(&py);
    assert_eq!(
        format!("{:?}", setup.rules),
        format!("{:?}", like.rules),
        "{name}: the fight file's rules"
    );
    let rs = traced(sim, &setup, true, &format!("{name} left"));
    window_ok(sim, name, "left", &rs, &py, &setup);
}

fn window_ok(sim: &Sim, name: &str, what: &str, rs: &Recording, py: &Recording, setup: &Setup) -> f64 {
    let report = prefix_window(rs, py);
    let t_fight = match py.rules.get("t_fight") {
        Some(Ev::Float(x)) => *x,
        _ => 0.0,
    };
    let lasts = report.agree_until_s - t_fight;
    let mut words = report.first_break.clone().unwrap_or_default();
    if report.kind == Some(BreakKind::KnifeEdge) && words.contains("a \"clash\" at") {
        match (sim.clash_margin)(setup, report.agree_until_s) {
            Some(m) => words.push_str(&format!(
                "\n the Rust balls are {m:e} from R + R after that step's flight"
            )),
            None => words.push_str("\n the Rust fight has no step at that time"),
        }
    }
    println!(
        "{name} {what}: agrees for {lasts:.2} s of fighting ({} frames, {} events, max |d| {:.1e}); ended by {}",
        report.frames,
        report.events,
        report.worst,
        if words.is_empty() {
            "nothing (the whole fight)"
        } else {
            &words
        }
    );
    assert!(
        report.first_break.is_none() || lasts >= PREFIX_FLOOR_S,
        "{name} {what}: the window ends after {lasts:.3} s of fighting, before the floor of {PREFIX_FLOOR_S} s \
         (ruling M3-R5: a bug until the events around it show a knife-edge with its margin):\n {words}"
    );
    lasts
}

/// Every fight of the folder (`left`, the cached `right` and every cached scenario recording) agrees with the Rust
/// fight of the same setup for [`PREFIX_FLOOR_S`] of fighting, or to its end. Ignored: reads the cache.
pub fn prefix(sim: &Sim, name: &str) {
    let doc = doc(name);
    let mut fights: Vec<(String, Recording)> = Vec::new();
    let left = &doc["left"];
    fights.push((
        "left".into(),
        committed_recording(text(&left["file"]), Some(text(&left["hash"]))),
    ));
    let right = cached_recording(text(&doc["right"]["cache"]));
    assert_eq!(right.hash(), text(&doc["right"]["hash"]));
    fights.push(("right".into(), right));
    for s in list(&doc["scenarios"]) {
        for (file, hash) in list(&s["cache"]).iter().zip(list(&s["hashes"])) {
            let rec = cached_recording(text(file));
            assert_eq!(rec.hash(), text(hash), "{}", text(file));
            fights.push((text(file).rsplit('/').next().unwrap().to_string(), rec));
        }
    }
    let n = fights.len();
    for (what, py) in &fights {
        let setup = setup_like(py);
        let rs = traced(sim, &setup, true, &format!("{name} {what}"));
        window_ok(sim, name, what, &rs, py, &setup);
    }
    println!("{name}: {n} fights agree with the kit's for the prefix floor");
}

/// The Rust statistics of the fighter in the left corner against its opponent, at the folder's config, against the
/// kit's pooled figures with milestone 1's bars. `wide`: 20,000 seeds (an independent sample, the proof); else the
/// kit's own 2,000 seeds (a regression guard). Release builds only.
pub fn stats(name: &str, wide: bool) {
    let doc = doc(name);
    let st = &doc["stats"];
    let n = list(&st["seeds"])[1].as_u64().expect("the last seed");
    let seeds = if wide { 1..20_001 } else { 1..n + 1 };
    let ff = fight_file(&doc, false);
    let base = setup_from(&ff, roster(), 0).expect("the fight file sets up");
    let sheet = BeatSheet::series(base.fighters);
    let workers = std::thread::available_parallelism().map_or(3, |w| w.get()).min(3);
    let rs = stats_of(&|seed| Setup { seed, ..base.clone() }, &sheet, seeds.clone(), workers).expect("the fights");
    let py = pooled(&st["pooled"]);
    println!("{name} kit  (seeds 1 to {n}): {py:?}\n{name} Rust (seeds {seeds:?}): {rs:?}");
    assert_the_kits(name, &rs, &py);
}

/// The kit's pooled figures as a `Stats`.
pub fn pooled(p: &Value) -> Stats {
    let f = |k: &str| num(&p[k]);
    Stats {
        n: index(&p["n"]),
        decided: index(&p["decided"]),
        win_share: f("win_share"),
        ko_median: f("ko_median"),
        ko_p10: f("ko_p10"),
        ko_p90: f("ko_p90"),
        no_ko_share: f("no_ko_share"),
        lead_changes_mean: f("lead_changes_mean"),
        fit_rate: f("fit_rate"),
        ko_hist: Default::default(),
    }
}

/// Milestone 1's bars (`m1-measurements.md` section 5, plan section 2): win share 3 points, median K.O. 0.7 s, mean
/// lead changes 0.25 (0.06 in release), fit rate 1.5 points (0.5 in release), 10th percentile K.O. 1 s, no K.O. 2
/// points.
pub fn assert_the_kits(what: &str, rs: &Stats, py: &Stats) {
    let point = 0.01;
    let release = !cfg!(debug_assertions);
    let (lead_bar, fit_bar) = if release {
        (0.06, 0.5 * point)
    } else {
        (0.25, 1.5 * point)
    };
    for (key, rust, kit, bar) in [
        ("win share", rs.win_share, py.win_share, 3.0 * point),
        ("median K.O.", rs.ko_median, py.ko_median, 0.7),
        (
            "mean lead changes",
            rs.lead_changes_mean,
            py.lead_changes_mean,
            lead_bar,
        ),
        ("fit rate", rs.fit_rate, py.fit_rate, fit_bar),
        ("10th percentile K.O.", rs.ko_p10, py.ko_p10, 1.0),
        ("share with no K.O.", rs.no_ko_share, py.no_ko_share, 2.0 * point),
    ] {
        println!(
            "  {what} {key}: Rust {rust:.5}, kit {kit:.5}, apart {:.5} (bar {bar})",
            (rust - kit).abs()
        );
        assert!(
            (rust - kit).abs() <= bar,
            "{what} {key}: Rust {rust}, the kit {kit}, more than {bar} apart"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_setup() -> Setup<'static> {
        Setup {
            fighters: [fighter("FLUTE"), fighter("TROMBONE")],
            rules: Rules::new(132.0, 3.6),
            seed: 1,
        }
    }

    fn reaches_a_stub(_: &Setup, _: bool) -> Result<(Recording, Vec<&'static str>), SimError> {
        let rec = Recording::read(&crate::refs::reference("fights/flute_vs_trombone/seed1.bfrec")).expect("seed1");
        Ok((rec, vec!["clock", "stub:kinds::swallow::beat", "tick.striker"]))
    }

    fn reaches_none(_: &Setup, _: bool) -> Result<(Recording, Vec<&'static str>), SimError> {
        let rec = Recording::read(&crate::refs::reference("fights/flute_vs_trombone/seed1.bfrec")).expect("seed1");
        Ok((rec, vec!["clock", "tick.striker"]))
    }

    fn no_margin(_: &Setup, _: f64) -> Option<f64> {
        None
    }

    #[test]
    #[should_panic(expected = "socket stubs that are still empty (1 calls): [\"stub:kinds::swallow::beat\"]")]
    fn a_fight_that_reaches_a_stub_fails() {
        let sim = Sim {
            run_traced: reaches_a_stub,
            clash_margin: no_margin,
        };
        traced(&sim, &a_setup(), false, "TUBA");
    }

    #[test]
    fn a_fight_that_reaches_no_stub_passes() {
        let sim = Sim {
            run_traced: reaches_none,
            clash_margin: no_margin,
        };
        assert!(!traced(&sim, &a_setup(), false, "FLUTE").events.is_empty());
    }
}
