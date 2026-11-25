//! Writes the Classic lock, `tests/golden/classic.txt` (see `bf_harness::golden`):
//! `cargo run --release -p bf-harness --example golden [-- <out file>]`. Run it only on code whose Classic sound is
//! known good; the test `classic_golden` then holds every later change to it.

use bf_harness::golden::{fights, hash_all, line};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/classic.txt").to_string());
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let fights = fights();
    let mut text = String::new();
    for (g, h) in fights.iter().zip(hash_all(&fights, threads)) {
        match h {
            Ok(h) => text.push_str(&(line(g, &h) + "\n")),
            Err(e) => {
                eprintln!("{} ({} v {}): {e}", g.theme, g.a, g.b);
                std::process::exit(1);
            }
        }
    }
    std::fs::write(&out, &text).expect("the file is written");
    println!("{} fights pinned in {out}", fights.len());
}
