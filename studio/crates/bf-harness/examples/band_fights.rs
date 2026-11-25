//! Writes the fight file each theme's band is measured on (`bf_harness::band`), for listening to it: the theme's
//! pinned pair under Composed, with the seed of its first fight with a K.O. It renders nothing.
//!
//! ```text
//! cargo run --release -p bf-harness --example band_fights -- <folder> <theme> ...
//! ```

use bf_content::roster;
use bf_content::themes::COMPOSED;
use bf_harness::{band, golden};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("a folder"));
    std::fs::create_dir_all(&dir).expect("the folder");
    for theme in args {
        let g = band::fight_of(&theme).expect("a pair");
        let mut ff = golden::fight_file(&g).expect("a fight file");
        ff.music.arrangement = COMPOSED.to_string();
        let seed = (1..=12u64).find(|&seed| {
            bf_sim::setup_from(&ff, roster(), seed)
                .ok()
                .and_then(|s| bf_sim::run(&s, true).ok())
                .is_some_and(|r| r.t_ko.is_some())
        });
        ff.seed = seed;
        let path = dir.join(format!("{theme}.toml"));
        std::fs::write(&path, ff.to_toml()).expect("the file");
        println!("{} {} v {} seed {:?}", path.display(), g.a, g.b, seed);
    }
}
