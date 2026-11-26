//! Self-replay: the engine gives the same fight, bit for bit, on every PC and in every build.
//!
//! `hashes_are_stable` runs FLUTE against TROMBONE at the episode's rules (the example fight file
//! `studio/fights/flute_vs_trombone.toml`, through `setup_from`) for seeds 1, 2, 3, 500 and `1 << 40`, in both side
//! orders, and prints one line per fight: `left right seed winner t_ko n_events hash` (the winner as the fighter's
//! index, 0 for the left corner; `-` for no winner or no K.O.; the hash is the recording's content hash,
//! `recording-format.md` section 7, over every event and every per-frame value). The lines must equal
//! `studio/replay/hashes.txt`.
//!
//! The file was blessed on the Windows reference PC: `BF_BLESS=1 cargo test -p bf-sim --test replay` (run from
//! `studio/`) writes it instead of checking it. The fight's maths is pure Rust (`bf_core::dmath` on `libm` with soft
//! floats, no fused multiply-add), so the Linux and the Windows CI jobs must both reproduce exactly the same lines,
//! in debug and in release builds.
//!
//! `record_flag_does_not_change_the_fight`: recording the frames is only looking; the events, the winner and the
//! K.O. time are the same with `record` false.

use std::path::PathBuf;

use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_sim::roster::{FighterCfg, parse_roster};
use bf_sim::{Setup, run, setup_from};

/// The seeds the brief names: three small ones, the episode's own search range, and the first balance seed.
const SEEDS: [u64; 5] = [1, 2, 3, 500, 1 << 40];

/// `studio/replay/hashes.txt`.
fn hashes_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../replay/hashes.txt")
}

fn roster() -> Vec<FighterCfg> {
    parse_roster(include_str!("../../bf-content/data/roster.json")).expect("the roster loads")
}

/// The example fight file, and the same with the corners swapped (the fighters and their handicaps).
fn fight_files() -> [FightFile; 2] {
    let ff = FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the example parses");
    let mut swapped = ff.clone();
    swapped.fighters.reverse();
    swapped.handicap.reverse();
    [ff, swapped]
}

/// Every fight of the file, in its order: each side order, each seed.
fn setups<'a>(files: &[FightFile; 2], roster: &'a [FighterCfg]) -> Vec<Setup<'a>> {
    let mut out = Vec::new();
    for ff in files {
        for seed in SEEDS {
            out.push(setup_from(ff, roster, seed).expect("the example fight file sets up"));
        }
    }
    out
}

fn line(setup: &Setup, rec: &Recording) -> String {
    let or_dash = |x: Option<String>| x.unwrap_or_else(|| "-".to_string());
    format!(
        "{} {} {} {} {} {} {}",
        setup.fighters[0].name,
        setup.fighters[1].name,
        setup.seed,
        or_dash(rec.winner.map(|w| w.to_string())),
        or_dash(rec.t_ko.map(|t| t.to_string())),
        rec.events.len(),
        rec.hash()
    )
}

const HEADER: &str = "\
# The engine's self-replay hashes: FLUTE against TROMBONE at the episode's rules (studio/fights/flute_vs_trombone.toml),
# both side orders, seeds 1, 2, 3, 500 and 1 << 40. Written by crates/bf-sim/tests/replay.rs with BF_BLESS=1 on the
# Windows reference PC; every PC and every build must give the same lines. Do not edit by hand.
# left right seed winner t_ko n_events hash
";

/// Brief, step 1: the fights' lines equal `studio/replay/hashes.txt`.
#[test]
fn hashes_are_stable() {
    let (files, roster) = (fight_files(), roster());
    let lines: Vec<String> = setups(&files, &roster)
        .iter()
        .map(|setup| {
            let rec = run(setup, true).expect("the engine runs the example fight");
            let l = line(setup, &rec);
            println!("{l}");
            l
        })
        .collect();
    let text = format!("{HEADER}{}\n", lines.join("\n"));
    let path = hashes_file();
    if std::env::var_os("BF_BLESS").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().expect("the file has a folder")).expect("create studio/replay");
        std::fs::write(&path, &text).expect("write hashes.txt");
        eprintln!("blessed {}", path.display());
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e} (bless it with BF_BLESS=1 on the reference PC)",
            path.display()
        )
    });
    let want_lines: Vec<&str> = want.lines().filter(|l| !l.starts_with('#')).collect();
    let differ: Vec<String> = lines
        .iter()
        .zip(want_lines.iter().copied().chain(std::iter::repeat("(missing)")))
        .filter(|(got, want)| got.as_str() != *want)
        .map(|(got, want)| format!(" got:  {got}\n file: {want}"))
        .collect();
    assert!(
        differ.is_empty() && want_lines.len() == lines.len(),
        "{} of {} fights differ from {} ({} lines in the file):\n{}",
        differ.len(),
        lines.len(),
        path.display(),
        want_lines.len(),
        differ.join("\n")
    );
    assert_eq!(
        want, text,
        "the fights agree, but the file's header or layout is not the blessed one"
    );
}

/// Brief, step 1: the events, the winner and the K.O. time are equal with `record` true and false.
#[test]
fn record_flag_does_not_change_the_fight() {
    let (files, roster) = (fight_files(), roster());
    let mut kos = 0;
    for setup in setups(&files, &roster) {
        let seen = run(&setup, true).expect("runs");
        let blind = run(&setup, false).expect("runs");
        let what = format!(
            "{} {} seed {}",
            setup.fighters[0].name, setup.fighters[1].name, setup.seed
        );
        assert!(
            seen.frames() > 0 && !seen.arrays.is_empty(),
            "{what}: the frames are recorded"
        );
        assert!(blind.arrays.is_empty(), "{what}: no frames without record");
        assert_eq!(seen.events, blind.events, "{what}");
        assert_eq!(
            (seen.winner, seen.t_ko.map(f64::to_bits)),
            (blind.winner, blind.t_ko.map(f64::to_bits)),
            "{what}"
        );
        assert!(!seen.events.is_empty(), "{what}");
        kos += usize::from(seen.t_ko.is_some());
    }
    assert!(kos > 0, "some of the fights end in a K.O.");
}
