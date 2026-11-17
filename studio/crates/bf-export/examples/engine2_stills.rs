//! Engine 2 stills through the render job's frame path (the director's time map, camera, shake and arena, the
//! effects): the TEST DUMMY fight of `fights/engine2/test_dummy.toml`, one still for each of the director's
//! signature shots (the first punch-in at its strike, a launch follow, a slam's low angle, the K.O. orbit), at
//! 1080 x 1920, as PNGs in the folder given (the default is the current folder).
//!
//! `cargo run -p bf-export --example engine2_stills -- <folder>`

use std::sync::Arc;

use bf_content::fighters2;
use bf_core::fightfile::FightFile;
use bf2_direct::ShotKind;
use bf2_fx::FxTimeline;
use bf2_fx::frame::FxFrame;
use bf2_render::{Gpu, Look2, Renderer, Stage};
use bf2_sim::{Rules2, Setup2};

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let path = bf_testdata::repo_root().join("studio/fights/engine2/test_dummy.toml");
    let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).expect("the dummy's file parses");
    let defs: [Arc<bf2_sim::FighterDef>; 2] = [0, 1].map(|k| fighters2::def(&ff.fighters[k]).expect("a fighter"));
    let mut rules = Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
    let seed = ff.seed.expect("the file names its seed");
    let rec = bf2_sim::run(
        &Setup2 {
            fighters: defs.clone(),
            rules,
            seed,
        },
        true,
    )
    .expect("the dummy fights");
    let d = bf_export::job2::direct(&ff, &rec);
    let dir = d.direction();
    let step_to_t = |s: u32| d.map.step_video_t(s);
    let colours = [0, 1].map(|k| bf_export::job2::linear(defs[k].colour));
    let mut timeline = FxTimeline::new();
    let mut looks: Vec<Box<dyn Look2>> = Vec::new();
    let mut fxs = Vec::new();
    for (k, name) in ff.fighters.iter().enumerate() {
        let fx = fighters2::look_fx(name).expect("an Engine 2 look");
        fx.fx(k, &rec, &step_to_t, &mut timeline).expect("the look's effects");
        fxs.push(fx);
        looks.push(fighters2::look(name).expect("an Engine 2 look"));
    }
    let (t0, t1) = (
        |e: &bf2_sim::EvKind| fxs[0].tone(e),
        |e: &bf2_sim::EvKind| fxs[1].tone(e),
    );
    bf2_fx::wire::wire_toned(
        &rec,
        [&defs[0], &defs[1]],
        colours,
        &step_to_t,
        [&t0, &t1],
        &mut timeline,
    )
    .expect("the effects");
    let mut looks: [Box<dyn Look2>; 2] = looks.try_into().unwrap_or_else(|_| unreachable!("two fighters"));

    println!(
        "seed {seed}: K.O. at {:?} s of fight, {:?} s of video; {:.2} s added; {} guard snaps",
        rec.outcome.t_ko,
        d.map.ko_video_t(),
        d.map.extra(),
        d.camera.guarded
    );
    for s in &d.camera.shots {
        println!(
            "  {:?} on {} ({:?}): {:.2} / {:.2} / {:.2}",
            s.kind, s.subject, s.priority, s.t0, s.t_peak, s.t1
        );
    }
    let first = |k: fn(&ShotKind) -> bool| d.camera.shots.iter().find(|s| k(&s.kind)).copied();
    let mut picks = Vec::new();
    if let Some(s) = first(|k| matches!(k, ShotKind::PunchIn)) {
        picks.push(("punch_in", s.t_peak));
    }
    if let Some(s) = first(|k| matches!(k, ShotKind::Launch)) {
        picks.push(("launch_follow", s.t0 + 0.35 * (s.t1 - s.t0)));
    }
    if let Some(s) = first(|k| matches!(k, ShotKind::Slam)) {
        picks.push(("slam_low_angle", s.t0 + 0.3));
    }
    if let Some(s) = first(|k| matches!(k, ShotKind::Ko)) {
        picks.push(("ko_orbit", s.t0 + 0.7 * (s.t_peak - s.t0)));
    }

    let gpu = Gpu::headless().expect("a GPU");
    let mut r = Renderer::new(&gpu);
    let stage = Stage::new(&mut r, &gpu);
    for look in &mut looks {
        look.setup(&mut r, &gpu);
    }
    let mut fx = FxFrame::new(&gpu, &r);
    std::fs::create_dir_all(&out).expect("the output folder");
    for (name, t) in picks {
        let f = ((t * 30.0).round() as usize).min(1829);
        let t0 = std::time::Instant::now();
        let img = fx.render(
            &mut r,
            &gpu,
            &stage,
            &looks,
            [&defs[0], &defs[1]],
            &rec,
            &dir,
            &timeline,
            colours,
            f,
            (bf2_render::W, bf2_render::H),
        );
        let file = out.join(format!("e4_{name}_f{f}.png"));
        std::fs::write(&file, bf_export::output::png_bytes(&img)).expect("the still is written");
        println!("{} ({:.1} s)", file.display(), t0.elapsed().as_secs_f64());
    }
}
