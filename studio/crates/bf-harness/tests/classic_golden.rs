//! The Classic lock (sound upgrade, 2026-11-25): every pinned Classic fight still renders the soundtrack
//! `golden/classic.txt` holds, sample for sample, and the file still covers every theme and every fighter that can
//! play. A change that moves Classic fails here with the fights it moved.

use bf_harness::golden::{fights, hash_all, parse};

#[test]
fn classic_is_unchanged() {
    let pinned: Vec<_> = include_str!("golden/classic.txt")
        .lines()
        .map(|l| parse(l).unwrap_or_else(|| panic!("a line of classic.txt does not read: {l}")))
        .collect();
    let list: Vec<_> = pinned.iter().map(|(g, _)| g.clone()).collect();
    assert_eq!(
        list,
        fights(),
        "classic.txt lists other fights than the content gives: write it again"
    );
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut moved = Vec::new();
    for ((g, want), got) in pinned.iter().zip(hash_all(&list, threads)) {
        match got {
            Ok(got) if &got == want => {}
            Ok(got) => moved.push(format!(
                "{} ({} v {}): {}",
                g.theme,
                g.a,
                g.b,
                if got.seed != want.seed {
                    "another fight"
                } else if got.pcm == want.pcm {
                    "differs below 16 bits"
                } else {
                    "sounds different"
                }
            )),
            Err(e) => moved.push(format!("{} ({} v {}): {e}", g.theme, g.a, g.b)),
        }
    }
    assert!(
        moved.is_empty(),
        "Classic changed in {} of {} fights:\n{}",
        moved.len(),
        pinned.len(),
        moved.join("\n")
    );
}
