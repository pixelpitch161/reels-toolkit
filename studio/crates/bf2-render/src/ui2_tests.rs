//! Engine 2's UI layer on the TEST DUMMY's own fight (`fights/engine2/test_dummy.toml`, its seed).

use std::sync::{Arc, OnceLock};

use bf_core::fightfile::FightFile;
use bf_picture::config::safe_overshoot;
use bf_picture::look::DefaultLook;
use bf_picture::sprite::Mesh;
use bf_picture::title::layout::Layout;
use bf_picture::title::{CardLook, TitleCard};
use bf_picture::ui::Kind;
use bf2_direct::{Clock, DirectCfg, shadow};

use super::*;
use crate::overlay::composite_cpu;

struct Fixture {
    ff: FightFile,
    rec: Recording2,
    defs: [Arc<FighterDef>; 2],
    map: TimeMap,
    cams: CameraTrack,
    shadow: Recording,
    show: Show,
    card: TitleCard,
    colours: [[f64; 3]; 2],
}

fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let ff = FightFile::parse(include_str!("../../../fights/engine2/test_dummy.toml")).unwrap();
        let defs = [0, 1].map(|k| bf_content::fighters2::def(&ff.fighters[k]).unwrap());
        let mut rules = bf2_sim::Rules2::new(ff.grid().t_fight);
        rules.dmg_scale = ff.dmg_scale;
        rules.dmg_side = [ff.handicap[0], ff.handicap[1]];
        let setup = bf2_sim::Setup2 {
            fighters: defs.clone(),
            rules,
            seed: ff.seed.unwrap(),
        };
        let rec = bf2_sim::run(&setup, true).unwrap();
        let cfg = DirectCfg::default();
        let map = TimeMap::from_recording(&rec, &cfg);
        let cams = CameraTrack::build(&rec, &map, &cfg);
        let shadow = shadow(&rec, [&defs[0], &defs[1]], &map, Clock::Video, &ff.grid());
        let show = Show::new(&shadow, &ff).expect("the dummy's seed K.O.s");
        let cfgs = [0, 1].map(|k| bf_content::fighters2::cfg(&ff.fighters[k]).unwrap());
        let colours = cfgs.map(|c| c.color.map(f64::from));
        let mesh: Mesh = bf_content::fighters2::card_mesh(&ff.fighters[0]).unwrap();
        let look = CardLook {
            look: &DefaultLook,
            mesh: &mesh,
            yaw: 0.0,
            roll: 0.0,
        };
        let layout = Layout::new(&show.names[0], &show.names[1], &ff.picture).unwrap();
        let card = TitleCard::with_layout(
            &show,
            colours,
            cfgs.map(|c| c.dark.map(f64::from)),
            [look, look],
            ff.series_tag.as_deref(),
            layout,
        )
        .unwrap();
        Fixture {
            ff,
            rec,
            defs,
            map,
            cams,
            shadow,
            show,
            card,
            colours,
        }
    })
}

fn ui() -> Ui2<'static> {
    let f = fixture();
    let rows = [0, 1].map(|k| rows_of(&f.defs[k], f.rec.dt));
    Ui2::new(&f.show, &f.shadow, &f.rec, &f.map, &f.cams, rows, f.colours, &f.card)
}

/// Every 0.25 s of the video, as the safe-zone check looks.
fn quarter_frames() -> impl Iterator<Item = usize> {
    (0..244).map(|i| bf_core::py::round_half_even(i as f64 * 0.25 * FPS) as usize)
}

#[test]
fn rows_name_the_dummys_moves() {
    let f = fixture();
    let rows = rows_of(&f.defs[0], f.rec.dt);
    let names: Vec<(&str, &str)> = rows.iter().map(|r| (r.tag, r.name.as_str())).collect();
    assert_eq!(names, [("SP1", "DUMMY BUSTER"), ("LOW", "SECOND WIND")]);
    assert!((rows[0].hold - 60.0 * f.rec.dt).abs() < 1e-12);
}

/// Review Focus 3: compositing the matte equals drawing straight onto the picture, within ±2 per channel, over 5
/// backgrounds at every 0.25 s frame (but where the card is shut: there the card is taken whole).
#[test]
fn matte_matches_direct_draw() {
    let ui = ui();
    let mut worst = 0;
    for f in quarter_frames() {
        let t = f as f64 / FPS;
        if fixture().card.door_offset(t) == 0.0 {
            continue;
        }
        let (ov, _) = ui.draw(f);
        for b in 0..5u64 {
            // a busy background: a different pattern each time
            let mut bg: Rgb8 = Img::new(UI_W, UI_H, 3);
            for (i, p) in bg.data.iter_mut().enumerate() {
                let x = (i as u64).wrapping_mul(2_654_435_761).wrapping_add(b * 977) ^ (f as u64 * 31);
                *p = (x >> 7) as u8;
            }
            let mut direct = bg.clone();
            ui.draw_on(&mut direct, &mut DrawLog::new(), f);
            composite_cpu(&mut bg, &ov);
            for (x, y) in direct.data.iter().zip(&bg.data) {
                worst = worst.max((i32::from(*x) - i32::from(*y)).abs());
            }
        }
    }
    assert!(worst <= 2, "{worst}");
}

#[test]
fn card_covers_the_first_bar() {
    let (ov, _) = ui().draw(0);
    assert!(ov.rgba.iter().all(|p| p[3] == 255));
}

#[test]
fn countdown_shows_before_fight() {
    let g = &fixture().show.grid;
    let f = ((g.t_fight - g.beat) * FPS).ceil() as usize + 1;
    let (_, log) = ui().draw(f);
    assert!(
        log.boxes.iter().any(|b| b.kind == Kind::Caption && b.info == "1"),
        "{:?}",
        log.boxes
    );
    let (_, log) = ui().draw(((g.t_fight + 0.1) * FPS) as usize);
    assert!(log.boxes.iter().any(|b| b.info == "FIGHT!"));
}

#[test]
fn hud_hp_follows_the_fight() {
    let f = fixture();
    let ui = ui();
    let hit = f.shadow.events.iter().find(|e| e.kind == "hit").expect("a hit");
    let target = hit.i(3) as usize;
    let before = (hit.t * FPS).floor() as usize - 2;
    let after = (hit.t * FPS).ceil() as usize + 20;
    assert_eq!(ui.hp_at(target, before as f64 / FPS), 100.0);
    assert!(ui.hp_at(target, after as f64 / FPS) < 100.0);
    // the HUD's rows differ
    let rows = |ov: &Overlay| ov.rgba[440 * UI_W..500 * UI_W].to_vec();
    assert_ne!(rows(&ui.draw(before).0), rows(&ui.draw(after).0));
}

#[test]
fn ko_caption_at_ko_video_t() {
    let f = fixture();
    let ko = f.map.ko_video_t().unwrap();
    let first = (ko * FPS).ceil() as usize;
    let ui = ui();
    let has = |fr: usize| {
        ui.draw(fr)
            .1
            .boxes
            .iter()
            .any(|b| b.kind == Kind::Caption && b.info.contains("K.O."))
    };
    assert!(!has(first - 1) && has(first));
}

#[test]
fn ui_log_inside_safe_zone() {
    let ui = ui();
    for f in quarter_frames() {
        for b in &ui.draw(f).1.boxes {
            if b.kind != Kind::Caption && (b.x0 < 0 || b.y0 < 0 || b.x1 > UI_W as i32 || b.y1 > UI_H as i32) {
                continue;
            }
            assert_eq!(safe_overshoot(b.x0, b.y0, b.x1, b.y1), 0, "frame {f}: {b:?}");
        }
    }
    let _ = &fixture().ff;
}

/// The loop's run-up: the card dims the arena to 0.55 as its doors start to shut, so the UI layer dims the
/// picture to the same level over the half second before, in even steps; the first dimmed frame is no longer a
/// one-off jump that the loop check reads as the video starting over early.
#[test]
fn the_card_dim_is_ramped_in() {
    let g = &fixture().show.grid;
    let ui = ui();
    // the median alpha of the layer: the dim, not the HUD's or the doors' own pixels
    let dim = |f: usize| {
        let mut a: Vec<u8> = ui.draw(f).0.rgba.iter().map(|p| p[3]).collect();
        a.sort_unstable();
        i32::from(a[a.len() / 2])
    };
    let close = ((g.dur - g.t_close) * FPS).round() as usize;
    let start = close - (PRE_CLOSE_DIM_S * FPS).round() as usize;
    assert_eq!(dim(start - 1), 0, "no dim before the run-up");
    let card = dim(close + 1);
    assert!((110..=120).contains(&card), "the card's own dim: {card}");
    let mut last = 0;
    for f in start..=close + 1 {
        let d = dim(f);
        assert!(d >= last && d - last <= 12, "frame {f}: {last} to {d}");
        last = d;
    }
}
