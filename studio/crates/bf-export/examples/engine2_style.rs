//! Engine 2 stills of the TEST DUMMY fight (`fights/engine2/test_dummy.toml`) through the render job's frame path
//! with its UI layer, for comparing with the current engine's frames (E7, "same style, higher fidelity"). With no
//! frames given it lists the fight's moments worth a still (heavy hits, the K.O.).
//!
//! `cargo run -p bf-export --example engine2_style -- <folder> [frame ...]`

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf2_render::Gpu;
use bf2_sim::EvKind;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let frames: Vec<usize> = args.map(|a| a.parse().expect("a frame number")).collect();
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
    // pixel-style spike: BF2_SIZE=360x640 draws small, then each pixel is blown up to 3 x 3
    let (w, h) = std::env::var("BF2_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(a, b)| (a.parse().unwrap(), b.parse().unwrap())))
        .unwrap_or((bf2_render::W, bf2_render::H));
    let mut fr = bf_export::job2::Frames2::of_scene(&gpu, scene, w, h);
    std::fs::create_dir_all(&out).expect("the output folder");
    for f in frames {
        let t0 = std::time::Instant::now();
        let mut img = fr.frame(f);
        let (bw, bh) = (bf2_render::W as usize, bf2_render::H as usize);
        if img.w != bw {
            let k = bw / img.w;
            let c = img.c;
            let mut big = bf_picture::img::Rgb8::new(bw, bh, c);
            for y in 0..bh {
                for x in 0..bw {
                    let s = ((y / k) * img.w + x / k) * c;
                    let d = (y * bw + x) * c;
                    big.data[d..d + c].copy_from_slice(&img.data[s..s + c]);
                }
            }
            img = big;
        }
        let file = out.join(format!("engine2_f{f}.png"));
        std::fs::write(&file, bf_export::output::png_bytes(&img)).expect("the still is written");
        println!("{} ({:.1} s)", file.display(), t0.elapsed().as_secs_f64());
    }
}
