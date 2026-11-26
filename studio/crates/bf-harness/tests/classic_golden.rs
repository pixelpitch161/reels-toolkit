//! The Classic lock (sound upgrade, 2026-11-25): every pinned Classic fight still renders the soundtrack
//! `golden/classic.txt` holds, sample for sample, and the file still covers every theme and every fighter that can
//! play. A change that moves Classic fails here with the fights it moved.
//!
//! Windows and Linux round some floating-point functions differently in their last bits, so each has its own file:
//! `golden/classic-windows.txt` on Windows (the owner's PC, where the app runs) and `golden/classic.txt` elsewhere.
//! Both are written from Classic code known good; a failure prints each moved fight's line as it now renders, so a
//! file can be written again from the test's own output when that is wanted.

use bf_harness::golden::{fights, hash_all, line, parse};

/// The pinned fights of this platform.
#[cfg(windows)]
const PINNED: &str = include_str!("golden/classic-windows.txt");
#[cfg(not(windows))]
const PINNED: &str = include_str!("golden/classic.txt");

#[test]
fn classic_is_unchanged() {
    let pinned: Vec<_> = PINNED
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
    let mut now = Vec::new();
    for ((g, want), got) in pinned.iter().zip(hash_all(&list, threads)) {
        match got {
            Ok(got) if &got == want => {}
            Ok(got) => moved.push({
                now.push(line(g, &got));
                format!(
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
                )
            }),
            Err(e) => moved.push(format!("{} ({} v {}): {e}", g.theme, g.a, g.b)),
        }
    }
    assert!(
        moved.is_empty(),
        "Classic changed in {} of {} fights:\n{}\n\nAs they render now:\n{}",
        moved.len(),
        pinned.len(),
        moved.join("\n"),
        now.join("\n")
    );
}
