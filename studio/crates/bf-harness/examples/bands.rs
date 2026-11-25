//! The dev command of M6 (sound upgrade, stage 2): each theme's Composed band as a phone plays it, and the bus
//! levels that would bring it under the target (`bf_harness::band`). It renders, it is not a test run:
//!
//! ```text
//! cargo run --release -p bf-harness --example bands -- [theme ...]
//! ```
//!
//! With no theme it measures every theme. One JSON line a theme: the phone loss, the band's loudness, each bus's
//! part of the band, and the proposal (the change of each bus in dB, to add to the theme's groove's `mix`, and
//! the phone loss it predicts, with M2's overtones on the buses named under `lows`, for the groove's `lows`).

use bf_content::themes::PORTED_THEMES;
use bf_harness::band::{self, PHONE_TARGET};
use serde_json::json;

fn round(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let themes: Vec<String> = if args.is_empty() {
        PORTED_THEMES.iter().map(|t| t.to_string()).collect()
    } else {
        args
    };
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).min(6);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out = std::sync::Mutex::new(());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(theme) = themes.get(i) else { break };
                    let line = match band::measure(theme) {
                        Ok(f) => {
                            let all: f64 = f.buses.iter().map(|b| b.energy).sum();
                            // the buses a phone loses most of get M2's overtones (where the groove does not
                            // give them already), then the levels
                            let lows = band::lows_candidates(&f);
                            let p = band::propose(&band::with_lows(&f, &lows), 6.0);
                            json!({
                                "theme": f.theme,
                                "fighters": f.fighters,
                                "seed": f.seed,
                                "grooved": f.grooved,
                                "phone_loss": round(f.phone_loss),
                                "pass": f.phone_loss <= PHONE_TARGET,
                                "loudness": round(f.loudness),
                                "share": f.buses.iter().map(|b| (b.name.clone(), round(b.energy / all))).collect::<Vec<_>>(),
                                "bus_phone_loss": f.buses.iter().map(|b| (b.name.clone(), round(b.loss()))).collect::<Vec<_>>(),
                                "lows": lows,
                                "propose": p.changes,
                                "predicted": round(p.phone_loss),
                                "balance_before": p.balance_before.map(|v| v.round()),
                                "balance_after": p.balance_after.map(|v| v.round()),
                            })
                        }
                        Err(e) => json!({"theme": theme, "error": e}),
                    };
                    let _held = out.lock().expect("stdout");
                    println!("{line}");
                }
            });
        }
    });
}
