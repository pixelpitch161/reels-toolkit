//! Engine 2 stills of the TEST DUMMY fight (`fights/engine2/test_dummy.toml`) through the render job's frame path
//! with its UI layer, for comparing with the current engine's frames (E7, "same style, higher fidelity"). With no
//! frames given it lists the fight's moments worth a still (heavy hits, the K.O.).
//!
//! `cargo run -p bf-export --example engine2_style -- <folder> [--pixel=N] [frame ...]`
//!
//! `--pixel=N` draws art pixels of N output pixels (3, the default, is the current engine's 360 x 640 grid; the
//! stills of another size are named `engine2_f<frame>_px<N>.png`).

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf2_render::Gpu;
use bf2_sim::EvKind;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let mut pixel = bf2_render::post::RING_PIXEL;
    let mut frames: Vec<usize> = Vec::new();
    for a in args {
        match a.strip_prefix("--pixel=") {
            Some(n) => pixel = n.parse().expect("a pixel size"),
            None => frames.push(a.parse().expect("a frame number")),
        }
    }
    let path = bf_testdata::repo_root().join("studio/fights/engine2/test_dummy.toml");
    let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).expect("the dummy's file parses");
    let defs: [Arc<bf2_sim::FighterDef>; 2] =
        [0, 1].map(|k| bf_content::fighters2::def(&ff.fighters[k]).expect("a fighter"));
    let mut rules = bf2_sim::Rules2::new(ff.grid().t_fight);
    rules.dmg_scale = ff.dmg_scale;
    rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
    let setup = bf2_sim::Setup2 {
        fighters: defs,
        rules,
        seed: ff.seed.expect("the file names its seed"),
    };
    let rec = Arc::new(bf2_sim::run(&setup, true).expect("the dummy fights"));
    let scene = bf_export::job2::Scene2::new(&ff, rec.clone()).expect("the scene");
    let map = &scene.directed.map;
    println!("K.O. at {:?} s of video", map.ko_video_t());
    for e in &rec.events {
        match e.kind {
            EvKind::Contact { strength, .. } if strength >= 0.6 => {
                let t = map.step_video_t(e.step);
                println!(
                    "  heavy hit by {} ({strength:.2}) at {t:.2} s, frame {}",
                    e.who,
                    (t * 30.0).round()
                );
            }
            EvKind::Ko { .. } => {
                let t = map.step_video_t(e.step);
                println!("  K.O. at {t:.2} s, frame {}", (t * 30.0).round());
            }
            _ => {}
        }
    }
    for &f in &frames {
        let p = &scene.directed.camera.poses[f];
        let d: f64 = (0..3).map(|k| (p.eye[k] - p.target[k]).powi(2)).sum::<f64>().sqrt();
        let sh = scene.directed.camera.shake[f];
        println!(
            "  frame {f}: camera {:.1} from {:?}, fov {:.1}, shake {sh:?}",
            d, p.target, p.fov_deg
        );
    }
    if frames.is_empty() {
        return;
    }
    let gpu = Gpu::headless().expect("a GPU");
    let mut fr = bf_export::job2::Frames2::of_scene(&gpu, scene, bf2_render::W, bf2_render::H);
    fr.set_pixel(pixel);
    std::fs::create_dir_all(&out).expect("the output folder");
    for f in frames {
        let t0 = std::time::Instant::now();
        let img = fr.frame(f);
        let file = if pixel == bf2_render::post::RING_PIXEL {
            out.join(format!("engine2_f{f}.png"))
        } else {
            out.join(format!("engine2_f{f}_px{pixel}.png"))
        };
        std::fs::write(&file, bf_export::output::png_bytes(&img)).expect("the still is written");
        println!("{} ({:.1} s)", file.display(), t0.elapsed().as_secs_f64());
    }
}
