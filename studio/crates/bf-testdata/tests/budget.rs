//! The reference set's size budgets (plan milestone 3, section 4) and its manifest layout: every file git tracks
//! (or would track) under `studio/reference` is listed in the manifest shard it belongs to, and the set is inside
//! its budgets. Always runs; reads only the working tree and `git ls-files`.

use std::collections::BTreeMap;

use bf_testdata::{budget_report, manifest_entries, reference_files, shard_of};

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
    assert!(over.is_empty(), "over budget: {}", over.join(", "));
}

#[test]
fn every_reference_file_is_in_its_shard() {
    let listed: BTreeMap<String, String> = manifest_entries()
        .into_iter()
        .map(|(shard, e)| (e.path, shard))
        .collect();
    let mut problems = Vec::new();
    let mut n = 0;
    for (rel, _) in reference_files() {
        let path = format!("studio/reference/{rel}");
        if rel == "manifest.json" || (rel.ends_with("/manifest.json") && shard_of(&path) == path) {
            continue;
        }
        n += 1;
        match listed.get(&path) {
            None => problems.push(format!("not in any manifest: {path}")),
            Some(shard) if *shard != shard_of(&path) => {
                problems.push(format!("{path} is listed in {shard}, belongs in {}", shard_of(&path)))
            }
            Some(_) => {}
        }
    }
    println!("{n} reference files, {} manifest entries", listed.len());
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
