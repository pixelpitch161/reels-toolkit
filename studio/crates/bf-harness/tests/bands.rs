//! M6 and C4 of the sound upgrade (stage 2): every theme's Composed band plays the theme's own patterns (its
//! groove) and loses at most `PHONE_TARGET` (5 dB) on a phone, measured on the theme's fight from FIGHT! to the K.O.
//! (`bf_harness::band`). Renders every theme once, on several threads.

use bf_content::themes::PORTED_THEMES;
use bf_harness::band::{self, PHONE_TARGET};

#[test]
fn every_band_has_its_own_patterns_and_carries_on_a_phone() {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let found = std::sync::Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).min(4);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(theme) = PORTED_THEMES.get(i) else { break };
                    let line = match band::measure(theme) {
                        Ok(f) if !f.grooved => Some(format!("{theme}: plays the shared patterns")),
                        Ok(f) if f.phone_loss > PHONE_TARGET => {
                            Some(format!("{theme}: the band loses {:.2} dB on a phone", f.phone_loss))
                        }
                        Ok(_) => None,
                        Err(e) => Some(e),
                    };
                    if let Some(line) = line {
                        found.lock().expect("the findings").push(line);
                    }
                }
            });
        }
    });
    let mut found = found.into_inner().expect("the findings");
    found.sort();
    assert!(found.is_empty(), "{}", found.join("\n"));
}
