//! TURNTABLES against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), against its
//! reference kit module (tools/studio_export/older/fighters/turntables.py, proven against #31 v3 by Gate O).

bf_harness::fighter_suite!("TURNTABLES");

/// TURNTABLES's kinds against the kit, whole fights: every recording of its pinned cache (the left and right fights,
/// every scenario seed) run again by the engine gives every event (kind, time to the bit, fields), the winner and the
/// K.O. of the kit's. Stronger than `prefix` (which stops at the 10 s floor); it needs the cache (`python -m
/// tools.studio_export cache fighters/turntables`).
#[test]
#[ignore = "needs the pinned cache; run by name in release"]
fn whole_fights_are_the_kits() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/.cache/m3/fighters/turntables");
    let mut files: Vec<std::path::PathBuf> = vec![dir.join("left.bfrec"), dir.join("right.bfrec")];
    let mut more: Vec<_> = std::fs::read_dir(dir.join("scenarios"))
        .expect("the cache's scenarios: run `python -m tools.studio_export cache fighters/turntables`")
        .map(|e| e.expect("an entry").path())
        .collect();
    more.sort();
    files.extend(more);
    let (mut n, mut heals) = (0, 0);
    for f in &files {
        let py = bf_core::recording::Recording::read(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let setup = setup_of(&py);
        let rs = bf_sim::run(&setup, true).expect("the fight runs");
        let what = f.file_name().unwrap().to_string_lossy();
        let first = rs
            .events
            .iter()
            .zip(&py.events)
            .position(|(a, b)| !bf_harness::fight::same_event(a, b));
        assert!(
            first.is_none() && rs.events.len() == py.events.len(),
            "{what}: the events part at {first:?} (Rust {:?}, kit {:?}); {} against {} events",
            first.map(|i| &rs.events[i]),
            first.map(|i| &py.events[i]),
            rs.events.len(),
            py.events.len()
        );
        assert_eq!((rs.winner, rs.t_ko), (py.winner, py.t_ko), "{what}");
        // REWIND waits for damage worth undoing: every rewind gives back at least `min_undo` (5 HP)
        for e in rs.events.iter().filter(|e| e.kind == "heal") {
            assert!(e.f(3) >= 5.0, "{what}: a rewind at {} gave back only {}", e.t, e.f(3));
            heals += 1;
        }
        n += 1;
    }
    assert!(n >= 3 && heals > 0, "{n} fights, {heals} rewinds");
    println!("{n} whole fights agree with the kit's ({heals} rewinds)");
}

/// `fight::setup_like` with TURNTABLES's roster entry from `bf-sim/tests/older_roster.json` (the compiled-in roster
/// has the older ten only from their register task on), the other fighter from the compiled-in roster.
fn setup_of(py: &bf_core::recording::Recording) -> bf_sim::Setup<'static> {
    use bf_core::event::Value as Ev;
    static OLDER: std::sync::OnceLock<Vec<bf_core::roster::FighterCfg>> = std::sync::OnceLock::new();
    let older = OLDER.get_or_init(|| {
        bf_core::roster::parse_roster(include_str!("../../bf-sim/tests/older_roster.json")).expect("the older roster")
    });
    let find = |name: &str| -> &'static bf_core::roster::FighterCfg {
        older
            .iter()
            .chain(bf_harness::fight::roster())
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{name} is in neither roster"))
    };
    let rule = |k: &str| match py.rules.get(k) {
        Some(Ev::Float(x)) => *x,
        Some(Ev::Int(n)) => *n as f64,
        other => panic!("rules.{k} is {other:?}"),
    };
    let mut rules = bf_sim::rules::Rules::new(rule("bpm"), rule("t_fight"));
    rules.apply(&py.rules).expect("the recording's rules apply");
    bf_sim::Setup {
        fighters: [find(&py.names[0]), find(&py.names[1])],
        rules,
        seed: py.seed,
    }
}

/// The kit's 2,000 statistics fights (`stats_rows.json` of the pinned cache: TURNTABLES left against FLUTE at the
/// folder's rules, seeds 1 to 2,000): the engine gives each one's winner and K.O. time to the bit.
#[test]
#[ignore = "needs the pinned cache; run by name in release"]
fn the_kits_statistics_fights_end_as_the_kits() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/.cache/m3/fighters/turntables");
    let left = bf_core::recording::Recording::read(&dir.join("left.bfrec")).expect("the cache's left.bfrec");
    let rows: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("stats_rows.json")).expect("stats_rows.json"))
            .expect("JSON");
    let rows = rows["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 2000);
    let base = setup_of(&left);
    let mut wins = 0;
    for r in rows {
        let seed = r["seed"].as_u64().expect("seed");
        let setup = bf_sim::Setup { seed, ..base.clone() };
        let rs = bf_sim::run(&setup, false).expect("runs");
        let want_w = r["winner"].as_u64().map(|w| w as usize);
        let want_t = r["t_ko"].as_f64();
        assert_eq!((rs.winner, rs.t_ko), (want_w, want_t), "seed {seed}");
        wins += usize::from(rs.winner == Some(0));
    }
    println!("2000 fights end as the kit's: TURNTABLES wins {wins}");
}
