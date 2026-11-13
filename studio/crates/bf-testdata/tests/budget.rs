//! The reference set's size budgets (plan milestone 4, section 4) and its manifest layout: every file git tracks
//! (or would track) under `studio/reference` is listed in the manifest shard it belongs to, and the set is inside
//! its budgets. Always runs; reads only the working tree and `git ls-files`.

use std::collections::BTreeMap;

use bf_testdata::{
    TRIPWIRE, budget_report, budget_rows, manifest_entries, reference_files, shard_of, tripwire_warning,
};

#[test]
fn the_set_is_inside_its_budget() {
    let report = budget_report();
    let mut over = Vec::new();
    for b in &report {
        let mark = if b.bytes > b.limit { "OVER" } else { "ok" };
        println!(
            "budget: {:40} {:>12} of {:>12} bytes  {mark}",
            b.folder, b.bytes, b.limit
        );
        if b.bytes > b.limit {
            over.push(format!("{} ({} > {})", b.folder, b.bytes, b.limit));
        }
    }
    let total = report.last().expect("the report ends with the whole set");
    assert_eq!(total.folder, "studio/reference");
    assert_eq!(total.limit, 147_000_000);
    if let Some(warning) = tripwire_warning(total.bytes) {
        println!("{warning}");
    }
    let names: Vec<&str> = report.iter().map(|b| b.folder.as_str()).collect();
    for want in ["themes/heist", "themes/horror_trap", "episodes/oboe_vs_bassoon"] {
        assert!(names.contains(&want), "no row for {want}");
    }
    let stray: Vec<&str> = names.iter().copied().filter(|n| n.ends_with(".bfrec")).collect();
    assert!(
        stray.is_empty(),
        "committed recordings outside fights/ (every other recording is the pinned cache file): {}",
        stray.join(", ")
    );
    assert!(over.is_empty(), "over budget: {}", over.join(", "));
}

#[test]
fn the_tripwire_warns() {
    assert_eq!(TRIPWIRE, 145_500_000);
    assert_eq!(tripwire_warning(145_500_000), None);
    assert_eq!(
        tripwire_warning(145_600_000).as_deref(),
        Some("warning: the reference set is 145.60 MB, past the 145.5 MB tripwire (plan m4 Section 4)")
    );
    // past the tripwire and inside the cap: a warning, not a failure
    let rows = budget_rows(&[("fights/flute_vs_trombone/ep392.bfrec".to_string(), 145_600_000)]);
    let total = rows.last().unwrap();
    assert!(
        total.bytes <= total.limit && tripwire_warning(total.bytes).is_some(),
        "{rows:?}"
    );
    let rows = budget_rows(&[("fights/flute_vs_trombone/ep392.bfrec".to_string(), 147_000_001)]);
    let total = rows.last().unwrap();
    assert!(total.bytes > total.limit, "{rows:?}");
}

#[test]
fn every_reference_file_is_in_its_shard() {
    let mut listed: BTreeMap<String, String> = BTreeMap::new();
    let mut problems = Vec::new();
    for (shard, e) in manifest_entries() {
        if shard != shard_of(&e.path) {
            problems.push(format!(
                "{} is listed in {shard}, belongs in {}",
                e.path,
                shard_of(&e.path)
            ));
        }
        if let Some(first) = listed.insert(e.path.clone(), shard.clone()) {
            problems.push(format!("{} is listed twice ({first} and {shard})", e.path));
        }
    }
    let mut n = 0;
    for (rel, _) in reference_files() {
        let path = format!("studio/reference/{rel}");
        if rel == "manifest.json" || (rel.ends_with("/manifest.json") && shard_of(&path) == path) {
            continue;
        }
        n += 1;
        if !listed.contains_key(&path) {
            problems.push(format!("not in any manifest: {path}"));
        }
    }
    println!("{n} reference files, {} manifest entries", listed.len());
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
