use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;

use bf_content::Sound2;
use bf_core::fightfile::FightFile;
use bf_picture::img::{Img, Rgb8};
use bf_picture::title::TitleCard;
use bf_story::BeatSheet;
use bf2_direct::CameraTrack;
use bf2_render::{RowDef, Ui2};
use bf2_sim::{FighterDef, Recording2};

use super::*;
use crate::checks::BandPlan;
use crate::job2::{Directed, direct, ui_parts};

struct Fixture {
    ff: FightFile,
    rec: Recording2,
    directed: Directed,
    sound: Sound2,
    card: TitleCard,
    rows: [Vec<RowDef>; 2],
    colours: [[f64; 3]; 2],
}

fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fights/engine2/test_dummy.toml");
        let ff = FightFile::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
        let defs: [std::sync::Arc<FighterDef>; 2] =
            [0, 1].map(|k| bf_content::fighters2::def(&ff.fighters[k]).unwrap());
        let mut rules = bf2_sim::Rules2::new(ff.grid().t_fight);
        rules.dmg_scale = ff.dmg_scale;
        rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
        let setup = bf2_sim::Setup2 {
            fighters: defs.clone(),
            rules,
            seed: ff.seed.unwrap(),
        };
        let rec = bf2_sim::run(&setup, true).unwrap();
        let directed = direct(&ff, &rec);
        let sound = bf_content::build_soundtrack2(&rec, [&defs[0], &defs[1]], &directed.map, &ff).unwrap();
        let (card, rows, colours) = ui_parts(&ff, &sound.show, [&defs[0], &defs[1]], rec.dt).unwrap();
        Fixture {
            ff,
            rec,
            directed,
            sound,
            card,
            rows,
            colours,
        }
    })
}

fn sheet() -> BeatSheet {
    let ff = &fixture().ff;
    let cfg = |k: usize| bf_content::fighters2::cfg(&ff.fighters[k]).unwrap();
    BeatSheet::series([cfg(0), cfg(1)])
}

#[test]
fn focus2_passes_on_the_dummy() {
    let f = fixture();
    let r = focus2(&f.rec, &f.directed.map, &f.directed.camera, &f.sound.show);
    assert!(r.pass, "{}", r.summary);
    assert_eq!(r.name, "focus");
    assert!(r.numbers["inside_frame"] >= 1.0);
    assert!(r.numbers["inside_safe"] >= INSIDE_SAFE);
}

/// The camera turned round for 2 s of the fight: both balls are behind it, so they leave the picture.
#[test]
fn focus2_fails_when_a_ball_leaves() {
    let f = fixture();
    let mut cams: CameraTrack = f.directed.camera.clone();
    for p in &mut cams.poses[600..660] {
        p.target = [0, 1, 2].map(|i| 2.0 * p.eye[i] - p.target[i]);
    }
    let r = focus2(&f.rec, &f.directed.map, &cams, &f.sound.show);
    assert!(!r.pass, "{}", r.summary);
    assert!(r.numbers["inside_frame"] < 1.0);
    let t = r.numbers["worst_t"];
    assert!((20.0..22.0).contains(&t), "worst at {t}");
}

#[test]
fn ko_window_bounds() {
    let sheet = sheet();
    let (a, b) = sheet.t_ko;
    assert!(ko_window(Some((a + b) / 2.0), &sheet).pass);
    assert!(ko_window(Some(a), &sheet).pass);
    assert!(!ko_window(Some(a - 0.01), &sheet).pass);
    assert!(!ko_window(Some(b + 0.01), &sheet).pass);
    let none = ko_window(None, &sheet);
    assert!(!none.pass && none.summary.contains("no K.O."), "{}", none.summary);
    // the dummy's review seed: its K.O. comes at 49.57 s of video, inside the window
    let t = fixture().directed.map.ko_video_t().unwrap();
    let real = ko_window(Some(t), &sheet);
    assert_eq!(real.pass, (a..=b).contains(&t), "{}", real.summary);
    assert!((real.numbers["t_ko"] - t).abs() < 1e-12);
}

/// Every check but the flash check, in order, on the real soundtrack and UI layer with stand-in frames (flat grey,
/// so the loop picture matches itself).
#[test]
fn pre_render2_runs_every_check_but_flash() {
    let f = fixture();
    let ui = Ui2::new(
        &f.sound.show,
        &f.sound.shadow,
        &f.rec,
        &f.directed.map,
        &f.directed.camera,
        f.rows.clone(),
        f.colours,
        &f.card,
    );
    let mut asked = Vec::new();
    let mut frames = |i: usize| {
        asked.push(i);
        let mut img: Rgb8 = Img::new(1080, 1920, 3);
        img.data.fill(90);
        img
    };
    let plan = BandPlan::of(&f.ff).unwrap();
    let focus = focus2(&f.rec, &f.directed.map, &f.directed.camera, &f.sound.show);
    let ko = ko_window(f.directed.map.ko_video_t(), &sheet());
    let out = pre_render2(&ui, &mut frames, &f.sound, focus, ko, &plan, &AtomicBool::new(false)).unwrap();
    let names: Vec<&str> = out.iter().map(|r| r.name).collect();
    assert_eq!(
        names,
        [
            "safezone",
            "levels",
            "band",
            "loudness",
            "loop_picture",
            "loop_audio",
            "focus",
            "ko_window"
        ]
    );
    for r in &out {
        eprintln!("{} {}: {}", r.name, r.pass, r.summary);
    }
    // the ones this harness decides (the sound's and the K.O.'s are the dummy's own, ledgered)
    for r in out
        .iter()
        .filter(|r| ["safezone", "loop_picture", "focus"].contains(&r.name))
    {
        assert!(r.pass, "{}: {}", r.name, r.summary);
    }
    // the last 5 s and the first second, each frame once
    assert_eq!(asked.len(), 180);
    assert_eq!(asked[0], NF - 150);
    // cancelled: nothing
    let cancel = AtomicBool::new(true);
    let focus = focus2(&f.rec, &f.directed.map, &f.directed.camera, &f.sound.show);
    let ko = ko_window(None, &sheet());
    assert!(
        pre_render2(
            &ui,
            &mut |_| Img::new(1080, 1920, 3),
            &f.sound,
            focus,
            ko,
            &plan,
            &cancel
        )
        .is_none()
    );
}

#[test]
fn third_averages_3x3_boxes() {
    let mut img: Rgb8 = Img::new(6, 3, 3);
    for (i, p) in img.data.iter_mut().enumerate() {
        *p = if (i / 3) % 6 < 3 { 9 } else { 18 };
    }
    let t = third(&img);
    assert_eq!((t.w, t.h), (2, 1));
    assert_eq!(t.px(0, 0), &[9, 9, 9]);
    assert_eq!(t.px(1, 0), &[18, 18, 18]);
}
