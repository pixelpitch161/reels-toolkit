//! Prints the composed lines of fights as JSON, one object a line, for measuring the melodies (sound upgrade, C10,
//! C2 and C11: clashes, note changes a bar, bar repeats, leaps, how far two styles differ):
//! `cargo run --release -p bf-harness --example lines -- <music seeds, e.g. 1,2,3> <fight.toml>...`
//!
//! Each fight is fought once (its first seed of 1 to 12 with a K.O.) and composed for every melody style and every
//! music seed given, as a render would: `songs` holds each fighter's whole-song line after the harmony pass, 8 notes
//! a bar from bar 0.

use bf_content::{SoundBuild, roster};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_sound::music::Style;
use serde_json::json;

fn main() {
    let mut args = std::env::args().skip(1);
    let seeds: Vec<u64> = args
        .next()
        .expect("the music seeds, such as 1,2,3")
        .split(',')
        .map(|s| s.parse().expect("a music seed"))
        .collect();
    for path in args {
        let text = std::fs::read_to_string(&path).expect("the fight file");
        let mut ff = FightFile::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
        ff.music.arrangement = bf_content::themes::COMPOSED.to_string();
        let rec = (1..=12u64)
            .find_map(|seed| {
                let setup = bf_sim::setup_from(&ff, roster(), seed).ok()?;
                let rec = bf_sim::run(&setup, true).ok()?;
                rec.t_ko.is_some().then_some(rec)
            })
            .unwrap_or_else(|| panic!("{path}: no K.O. in 12 seeds"));
        for style in Style::ALL {
            for &seed in &seeds {
                ff.music.music_seed = seed;
                ff.music.melody_style = Some(style.key().to_string());
                let show = Show::new(&rec, &ff).expect("a show");
                let build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("{path}: {e}"));
                let ctx = &build.ctx;
                let songs: serde_json::Map<String, serde_json::Value> =
                    ctx.composed.iter().map(|(n, p)| (n.clone(), json!(p.song))).collect();
                println!(
                    "{}",
                    json!({
                        "fight": path,
                        "style": style.key(),
                        "music_seed": seed,
                        "names": show.names,
                        "roles": ff.music.roles,
                        "loop_start": ctx.harmony.loop_start,
                        "loop_bars": ctx.harmony.loop_.len(),
                        "songs": songs,
                    })
                );
            }
        }
    }
}
