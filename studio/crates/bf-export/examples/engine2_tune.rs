//! Engine 2 stills for tuning and review: the fight of a fight file (default `fights/engine2/test_dummy.toml`)
//! through the render job's own frames (`Frames2`: effects, screen chain and UI layer), drawn at the frames given.
//!
//! `cargo run --release -p bf-export --example engine2_tune -- <folder> [--fight F] [--seed N] [--size WxH]
//! [--list] [--checks] [f1 f2 ...]`
//!
//! `--checks` builds the soundtrack and runs every check before the render (`pre_render2`: it draws the 180
//! frames the loop check reads), printing each with its numbers, for judging a seed without a whole render.
//!
//! `--list` prints the fight's effect events (contacts with their tier, slams, launches, the K.O.) with their video
//! frame, so stills can be picked. Each still is written as `<folder>/f<frame>.png` with its draw time.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use bf_content::{build_soundtrack2, fighters2};
use bf_export::checks::BandPlan;
use bf_export::checks::engine2::{focus2, pre_render2};
use bf_core::fightfile::FightFile;
use bf_export::job2::{Frames2, Scene2, ko_check};
use bf2_fx::presets::Tier;
use bf2_render::Gpu;
use bf2_sim::{EvKind, Rules2, Setup2, Tag};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = None;
    let mut fight = bf_testdata::repo_root().join("studio/fights/engine2/test_dummy.toml");
    let mut seed = None;
    let mut size = (bf2_render::W, bf2_render::H);
    let mut list = false;
    let mut checks = false;
    let mut frames = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--fight" => fight = it.next().expect("--fight F").into(),
            "--seed" => seed = Some(it.next().expect("--seed N").parse().expect("a seed")),
            "--size" => {
                let s = it.next().expect("--size WxH");
                let (w, h) = s.split_once('x').expect("WxH");
                size = (w.parse().expect("W"), h.parse().expect("H"));
            }
            "--list" => list = true,
            "--checks" => checks = true,
            x if out.is_none() => out = Some(std::path::PathBuf::from(x)),
            x => frames.push(x.parse::<usize>().expect("a frame number")),
        }
    }
    let out = out.unwrap_or_else(|| ".".into());
    std::fs::create_dir_all(&out).expect("the output folder");
    let mut ff = FightFile::parse(&std::fs::read_to_string(&fight).unwrap()).expect("the fight file parses");
    let defs: [Arc<bf2_sim::FighterDef>; 2] = [0, 1].map(|k| fighters2::def(&ff.fighters[k]).expect("a fighter"));
    let mut rules = Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
    let seed = seed.or(ff.seed).expect("a seed (the file's or --seed)");
    ff.seed = Some(seed);
    let setup = Setup2 {
        fighters: defs.clone(),
        rules,
        seed,
    };
    let rec = Arc::new(bf2_sim::run(&setup, true).expect("the fight runs"));
    let scene = Scene2::new(&ff, rec.clone()).expect("the scene");
    let map = &scene.directed.map;
    println!(
        "seed {seed}: K.O. at {:?} s of fight, {:?} s of video, card frame {}",
        rec.outcome.t_ko,
        map.ko_video_t(),
        scene.card().cover_frame()
    );
    if list {
        for e in &rec.events {
            let what = match &e.kind {
                EvKind::Contact { mv, strength, .. } => {
                    let m = &defs[e.who.min(1)].moves[*mv];
                    let tier = Tier::of(*strength as f32, m.tags.contains(&Tag::Special));
                    format!("contact {} {tier:?} {strength:.2}", m.name)
                }
                EvKind::Slam { .. } => "slam".into(),
                EvKind::Launch { .. } => "launch".into(),
                EvKind::Ko { .. } => "K.O.".into(),
                EvKind::Clash { .. } => "clash".into(),
                _ => continue,
            };
            let t = map.step_video_t(e.step);
            println!("f{:5} {t:7.2} s  who {}  {what}", (t * 30.0).round() as usize, e.who);
        }
    }
    if frames.is_empty() && !checks {
        return;
    }
    let sound = checks.then(|| {
        let dref = [&*defs[0], &*defs[1]];
        build_soundtrack2(&rec, dref, map, &ff).expect("the soundtrack")
    });
    let focus = focus2(&rec, map, &scene.directed.camera, &scene.show);
    let ko = ko_check(&ff, &scene).expect("the K.O. window");
    let gpu = Gpu::headless().expect("a GPU");
    let mut fr = Frames2::of_scene(&gpu, scene, size.0, size.1);
    if let Some(sound) = sound {
        let plan = BandPlan::of(&ff).expect("the band plan");
        let results = fr.with_ui(|ui, draw| {
            pre_render2(ui, draw, &sound, focus, ko, &plan, &AtomicBool::new(false)).expect("not cancelled")
        });
        for c in results {
            let nums: Vec<String> = c.numbers.iter().map(|(k, v)| format!("{k}={v:.3}")).collect();
            let pass = if c.pass { "PASS" } else { "FAIL" };
            println!("{} {pass}: {} [{}]", c.name, c.summary, nums.join(", "));
        }
    }
    for f in frames {
        let t0 = Instant::now();
        let img = fr.frame(f);
        let file = out.join(format!("f{f:04}.png"));
        std::fs::write(&file, bf_export::output::png_bytes(&img)).expect("the still is written");
        println!("{} ({:.1} s)", file.display(), t0.elapsed().as_secs_f64());
    }
}
