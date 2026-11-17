use std::sync::Arc;

use bf2_sim::{Bone, Brain, Choice, Ev, HitDef, MoveDef, Rig, Rules2, Setup2, run};

use super::*;

/// A ball with one arm and one poke.
fn fixture() -> FighterDef {
    FighterDef {
        name: "FIXTURE".into(),
        r: 0.6,
        mass: 1.0,
        hp_scale: 1.0,
        rig: Rig::new(vec![Bone {
            name: "arm",
            parent: None,
            length: 0.6,
            radius: 0.15,
        }])
        .unwrap(),
        moves: vec![MoveDef {
            name: "POKE",
            tags: vec![Tag::Light, Tag::Melee],
            windup: 6,
            active: 5,
            recovery: 10,
            keys: vec![],
            hits: vec![HitDef {
                bones: vec![0],
                window: (0, 5),
                damage: 6.0,
                hitstop: 6,
                knock: 4.0,
                lift: 0.0,
                hitstun: 10,
                strength: 0.5,
                multi: false,
                grab: None,
            }],
            spawns: vec![],
            cues: vec![],
            guard: 1.0,
            armor: false,
            lunge: 0.0,
            hop: 0.0,
        }],
        brain: Brain {
            choices: vec![Choice {
                mv: 0,
                range: (0.0, 2.2),
                weight: 1.0,
                cooldown: 20,
                needs_meter: false,
                air: None,
            }],
            speed: 2.6,
            think: 6,
            low: None,
            orbit: 0.2,
        },
        colour: [200, 120, 60],
    }
}

fn fight() -> (Recording2, Arc<FighterDef>) {
    let f = Arc::new(fixture());
    let mut rules = Rules2::new(4.5);
    rules.t_max = 30.0;
    let rec = run(
        &Setup2 {
            fighters: [f.clone(), f.clone()],
            rules,
            seed: 5,
        },
        true,
    )
    .expect("the fixture fights");
    (rec, f)
}

#[test]
fn wire_maps_every_contact() {
    let (rec, f) = fight();
    let to_t = |s: u32| rec.t_fight + f64::from(s) * rec.dt;
    let mut tl = FxTimeline::new();
    let made = wire(&rec, [&f, &f], [[1.0, 0.5, 0.2], [0.2, 0.5, 1.0]], &to_t, &mut tl).unwrap();
    let contacts: Vec<usize> = rec
        .events
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.kind, EvKind::Contact { .. }))
        .map(|(i, _)| i)
        .collect();
    assert!(contacts.len() >= 3, "the fixture lands hits: {}", contacts.len());
    for i in &contacts {
        assert!(made.contains(i), "contact {i} made a preset group");
        let t0 = to_t(rec.events[*i].step);
        assert!(
            tl.records
                .iter()
                .any(|r| r.t0 == t0 && matches!(r.kind, FxKind::Burst(_))),
            "contact {i} has its sparks"
        );
    }
    assert_eq!(tl.check_all(), Ok(()));
    // Same fight, same records.
    let mut again = FxTimeline::new();
    wire(&rec, [&f, &f], [[1.0, 0.5, 0.2], [0.2, 0.5, 1.0]], &to_t, &mut again).unwrap();
    assert_eq!(again.records, tl.records);
}

#[test]
fn late_records_finish_after_the_fight() {
    let (mut rec, f) = fight();
    let last = rec.steps.len() as u32 - 1;
    rec.events.push(Ev {
        step: last,
        who: 0,
        kind: EvKind::Ko { target: 1 },
    });
    let to_t = |s: u32| rec.t_fight + f64::from(s) * rec.dt;
    let mut tl = FxTimeline::new();
    wire(&rec, [&f, &f], [[1.0; 3], [1.0; 3]], &to_t, &mut tl).unwrap();
    let end = to_t(last);
    let ko: Vec<&FxRecord> = tl.records.iter().filter(|r| r.t0 == end).collect();
    assert!(!ko.is_empty());
    // Past the last step, the K.O.'s records keep their whole life and fall to nothing.
    let mut live = Vec::new();
    tl.active(end + 0.5, &mut live);
    assert!(
        live.iter()
            .any(|&i| tl.records[i].t0 == end && tl.records[i].env_at(end + 0.5) > 0.0)
    );
    for r in ko {
        assert!(r.t0 + f64::from(r.life) > end);
        assert!(r.env_at(r.t0 + f64::from(r.life) - 1.0 / 240.0) < 0.1);
    }
}

struct NoLook;

impl bf2_render::Look2 for NoLook {
    fn setup(&mut self, _r: &mut bf2_render::Renderer, _gpu: &bf2_render::Gpu) {}
    fn draw(&self, _f: &bf2_render::FightFrame<'_>, _k: usize, _out: &mut bf2_render::Scene) -> Vec<u32> {
        Vec::new()
    }
}

#[test]
fn a_wired_hit_lights_its_frame() {
    use crate::frame::FxFrame;
    let (rec, f) = fight();
    let map = bf2_direct::TimeMap::from_recording(&rec, &bf2_direct::DirectCfg::default());
    let to_t = |s: u32| map.step_video_t(s);
    let mut tl = FxTimeline::new();
    wire(&rec, [&f, &f], [[1.0, 0.5, 0.2], [0.2, 0.5, 1.0]], &to_t, &mut tl).unwrap();
    let first = rec
        .events
        .iter()
        .find(|e| matches!(e.kind, EvKind::Contact { .. }))
        .expect("a contact");
    let frame = (to_t(first.step) * 30.0).ceil() as usize + 1;
    let gpu = bf2_render::Gpu::headless().unwrap();
    let mut r = bf2_render::Renderer::new(&gpu);
    let stage = bf2_render::Stage::new(&mut r, &gpu);
    let looks: [Box<dyn bf2_render::Look2>; 2] = [Box::new(NoLook), Box::new(NoLook)];
    let mut fx = FxFrame::new(&gpu, &r);
    let colours = [[1.0, 0.5, 0.2], [0.2, 0.5, 1.0]];
    let size = (270, 480);
    let lit = fx.render(
        &mut r,
        &gpu,
        &stage,
        &looks,
        [&f, &f],
        &rec,
        &map,
        &tl,
        colours,
        frame,
        size,
    );
    let bare = fx.render(
        &mut r,
        &gpu,
        &stage,
        &looks,
        [&f, &f],
        &rec,
        &map,
        &FxTimeline::new(),
        colours,
        frame,
        size,
    );
    let sum = |img: &bf_picture::img::Rgb8| img.data.iter().map(|&v| f64::from(v)).sum::<f64>();
    crate::testkit::dump(&lit, "wired_hit");
    assert!(
        sum(&lit) > sum(&bare) * 1.02,
        "the hit adds light: {} vs {}",
        sum(&lit),
        sum(&bare)
    );
    // Stateless: the same frame again is the same picture.
    let again = fx.render(
        &mut r,
        &gpu,
        &stage,
        &looks,
        [&f, &f],
        &rec,
        &map,
        &tl,
        colours,
        frame,
        size,
    );
    assert!(again.data == lit.data);
}
