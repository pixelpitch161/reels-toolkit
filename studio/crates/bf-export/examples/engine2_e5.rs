//! Engine 2 E5's review stills and measurements: the TEST DUMMY fight of `fights/engine2/test_dummy.toml` through
//! the render job's parts (the soundtrack, the UI layer over the GPU frame). Writes four stills (the VS card, the
//! count-in, a captioned special, the K.O.) as PNGs and the soundtrack of each `music.slowmo` (`follow` and
//! `steady`, as WAVs) into the folder given, and prints the soundtrack build times (two builds and one), the UI
//! layer's time per frame, the frame time of the stills and, with `--checks`, every check before the render with its
//! numbers (it draws the 180 frames the loop check reads).
//!
//! `cargo run --release -p bf-export --example engine2_e5 -- <folder> [--checks]`

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use bf_content::{build_soundtrack2, fighters2};
use bf_core::fightfile::{FightFile, SlowmoMusic};
use bf_export::checks::BandPlan;
use bf_export::checks::engine2::{focus2, ko_window, pre_render2};
use bf_export::job2::{direct, linear, ui_parts};
use bf_story::BeatSheet;
use bf2_fx::FxTimeline;
use bf2_fx::frame::FxFrame;
use bf2_render::{Gpu, Look2, Renderer, Stage, Ui2};
use bf2_sim::{Rules2, Setup2};

/// The best of `n` timed runs of `f`, in seconds, and its last result.
fn best<T>(n: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut out = None;
    let mut t_best = f64::INFINITY;
    for _ in 0..n {
        let t0 = Instant::now();
        let v = f();
        t_best = t_best.min(t0.elapsed().as_secs_f64());
        out = Some(v);
    }
    (t_best, out.expect("at least one run"))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(
        args.iter()
            .find(|a| !a.starts_with("--"))
            .cloned()
            .unwrap_or(".".into()),
    );
    let checks = args.iter().any(|a| a == "--checks");
    std::fs::create_dir_all(&out).expect("the output folder");
    let path = bf_testdata::repo_root().join("studio/fights/engine2/test_dummy.toml");
    let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).expect("the dummy's file parses");
    let defs: [Arc<bf2_sim::FighterDef>; 2] = [0, 1].map(|k| fighters2::def(&ff.fighters[k]).expect("a fighter"));
    let mut rules = Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
    let seed = ff.seed.expect("the file names its seed");
    let setup = Setup2 {
        fighters: defs.clone(),
        rules,
        seed,
    };
    let rec = bf2_sim::run(&setup, true).expect("the dummy fights");
    let d = direct(&ff, &rec);
    let dref = [&*defs[0], &*defs[1]];
    println!(
        "seed {seed}: K.O. at {:?} s of fight, {:?} s of video; {:.2} s added",
        rec.outcome.t_ko,
        d.map.ko_video_t(),
        d.map.extra()
    );

    // The soundtrack: follow (two builds when the slow-mo moved the K.O.) and steady (one).
    let (t_follow, sound) = best(3, || {
        build_soundtrack2(&rec, dref, &d.map, &ff).expect("the soundtrack")
    });
    let mut steady_ff = ff.clone();
    steady_ff.music.slowmo = SlowmoMusic::Steady;
    let (t_steady, steady) = best(3, || {
        build_soundtrack2(&rec, dref, &d.map, &steady_ff).expect("the soundtrack")
    });
    println!(
        "soundtrack follow: {} builds, {t_follow:.2} s (best of 3)",
        sound.builds
    );
    println!(
        "soundtrack steady: {} build, {t_steady:.2} s (best of 3)",
        steady.builds
    );
    for (name, s) in [("follow", &sound), ("steady", &steady)] {
        let file = out.join(format!("e5_soundtrack_{name}.wav"));
        std::fs::write(&file, bf_sound::wav::wav16_bytes(&s.soundtrack.master)).expect("the wav is written");
        println!("{}", file.display());
    }

    // The UI layer: its time per frame over the safe-zone check's 244 frames.
    let (card, rows, ui_colours) = ui_parts(&ff, &sound.show, dref, rec.dt).expect("the UI parts");
    let ui = Ui2::new(
        &sound.show,
        &sound.shadow,
        &rec,
        &d.map,
        &d.camera,
        rows,
        ui_colours,
        &card,
    );
    let quarters: Vec<usize> = (0..244)
        .map(|i| ((i as f64 * 7.5).round() as usize).min(1829))
        .collect();
    let t0 = Instant::now();
    for &f in &quarters {
        std::hint::black_box(ui.draw(f));
    }
    println!(
        "UI layer: {:.2} ms per frame (mean of {} frames)",
        1000.0 * t0.elapsed().as_secs_f64() / quarters.len() as f64,
        quarters.len()
    );

    // The GPU frame with the UI layer composited.
    let dir = d.direction();
    let step_to_t = |s: u32| d.map.step_video_t(s);
    let colours = [0, 1].map(|k| linear(defs[k].colour));
    let mut timeline = FxTimeline::new();
    let mut looks: Vec<Box<dyn Look2>> = Vec::new();
    for (k, name) in ff.fighters.iter().enumerate() {
        fighters2::look_fx(name)
            .expect("an Engine 2 look")
            .fx(k, &rec, &step_to_t, &mut timeline)
            .expect("the look's effects");
        looks.push(fighters2::look(name).expect("an Engine 2 look"));
    }
    bf2_fx::wire::wire(&rec, dref, colours, &step_to_t, &mut timeline).expect("the effects");
    let mut looks: [Box<dyn Look2>; 2] = looks.try_into().unwrap_or_else(|_| unreachable!("two fighters"));
    let gpu = Gpu::headless().expect("a GPU");
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    for look in &mut looks {
        look.setup(&mut r, &gpu);
    }
    let mut fx = FxFrame::new(&gpu, &r);
    let mut draw = |f: usize| {
        r.set_overlay(&gpu, Some(&ui.draw(f).0));
        fx.render(
            &mut r,
            &gpu,
            &stage,
            &looks,
            dref,
            &rec,
            &dir,
            &timeline,
            colours,
            f,
            (bf2_render::W, bf2_render::H),
        )
    };

    let show = &sound.show;
    let frame = |t: f64| ((t * 30.0).round() as usize).min(1829);
    let mut picks = vec![("vs_card", card.cover_frame())];
    // the count-in's last number, a quarter beat after it lands
    let (t_one, _) = show.grid.count[2];
    picks.push(("count_in", frame(t_one + show.grid.beat / 4.0)));
    if let Some(c) = show.caps_ability.iter().flatten().map(|c| c.t0).reduce(f64::min) {
        picks.push(("special_caption", frame(c + 0.6)));
    }
    if let Some(t) = d.map.ko_video_t() {
        picks.push(("ko", frame(t + 0.5)));
    }
    let mut secs = Vec::new();
    for (name, f) in picks {
        let t0 = Instant::now();
        let img = draw(f);
        secs.push(t0.elapsed().as_secs_f64());
        let file = out.join(format!("e5_{name}_f{f}.png"));
        std::fs::write(&file, bf_export::output::png_bytes(&img)).expect("the still is written");
        println!("{} ({:.2} s)", file.display(), secs.last().unwrap());
    }
    let mean = secs[1..].iter().sum::<f64>() / (secs.len() - 1) as f64;
    println!("frame: {mean:.2} s each after the first ({:.2} fps)", 1.0 / mean);

    if checks {
        let cfg = |k: usize| fighters2::cfg(&ff.fighters[k]).expect("a roster entry");
        let sheet = BeatSheet::series([cfg(0), cfg(1)]);
        let plan = BandPlan::of(&ff).expect("the band plan");
        let focus = focus2(&rec, &d.map, &d.camera, show);
        let ko = ko_window(d.map.ko_video_t(), &sheet);
        let t0 = Instant::now();
        let results =
            pre_render2(&ui, &mut draw, &sound, focus, ko, &plan, &AtomicBool::new(false)).expect("not cancelled");
        println!("checks: {:.1} s", t0.elapsed().as_secs_f64());
        for c in results {
            let nums: Vec<String> = c.numbers.iter().map(|(k, v)| format!("{k}={v:.4}")).collect();
            println!(
                "{} {}: {} [{}]",
                c.name,
                if c.pass { "PASS" } else { "FAIL" },
                c.summary,
                nums.join(", ")
            );
        }
    }
}
