//! Prints the melody survey the test `melodies` checks (see `bf_harness::melody`):
//! `cargo run --release -p bf-harness --example melodies`.

fn main() {
    for seed in bf_harness::melody::SEEDS {
        let r = bf_harness::melody::survey(&[seed]);
        println!("{} pairings, seed {seed}, two harmonies", r.pairings);
        for s in &r.styles {
            println!(
                "{:14} clashes {:3}  out of register {:3}  written repeats {:4}  changes a bar {:.4}  bar repeats \
                 {:.4}  worst leaps {:4.1}% ({})",
                s.style.key(),
                s.clashes,
                s.out_of_register,
                s.repeats,
                s.changes_per_bar,
                s.bar_repeats,
                100.0 * s.worst_leaps,
                s.worst_leaper
            );
        }
        println!(
            "melody: two styles differ in at least {:.1}% ({})",
            100.0 * r.melody_least_differ,
            r.melody_least
        );
        println!(
            "bass:   two styles differ in at least {:.1}% ({})",
            100.0 * r.bass_least_differ,
            r.bass_least
        );
    }
}
