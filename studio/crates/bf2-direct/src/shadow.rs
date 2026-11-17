//! The shadow recording (Engine 2, milestone E5): an Engine 2 fight on the current score's terms.
//!
//! [`shadow`] turns a `Recording2` and its [`TimeMap`] into a current-engine `bf_core::recording::Recording`, so the
//! whole current soundtrack and text pipeline (`Show::new` and its captions, the music context, the story signal,
//! `score()`, `master()`, the story search) runs on an Engine 2 fight unchanged. Its events use the current
//! engine's layouts (`hit`, `block`, `clash`, `rope`, `ability`), with the v2 detail the current engine has no slot
//! for riding as trailing fields of `hit` (strength, the move's weight, its hitstop) and as new `v2.*` events that
//! no current-engine code matches.
//!
//! It is built on one of two clocks: the video clock (event times through the time map, as the picture shows them)
//! or the fight clock (`t_fight + step·dt`, as if there were no slow-mo).

use std::collections::BTreeMap;

use bf_core::event::{Event, Value};
use bf_core::grid::Grid;
use bf_core::py;
use bf_core::recording::{Array, RECORDING_FORMAT, Recording, Source};
use bf2_sim::{Ev, EvKind, FighterDef, MoveDef, Recording2, Rules2, Surface, Tag};

use crate::time::TimeMap;

/// Which clock a shadow recording's times are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    /// Video seconds, through the time map: what the picture shows (slow-mo stretched, the K.O. freeze held).
    Video,
    /// `t_fight + step·dt`: the fight as if it had no slow-mo.
    Fight,
}

/// Frames per second of the `HP` array.
const FPS: u32 = 30;

/// A rope bounce's speed when there are no steps to read it from (the current engine's typical bounce, which its
/// rope thud is scaled by).
const ROPE_SPEED: f64 = 2.6;

/// A ProjHit with no Contact after it (its owner is out) still sounds, at this strength.
const PROJ_STRENGTH: f64 = 0.3;

/// Hitstops of at least this many steps (1/30 s) get a `v2.freeze`.
const FREEZE_STEPS: u32 = 4;

/// HP on the current engine's scale: out of [`Rules2`]'s 100 whatever the fighter's `hp_scale`, at least 0.
fn hp_share(hp: f64, def: &FighterDef) -> f64 {
    (hp / def.hp_scale).max(0.0)
}

/// The name of `def`'s low-HP move (its first `Tag::Low` move), or `""`.
pub fn low_name(def: &FighterDef) -> &str {
    def.moves.iter().find(|m| m.has(Tag::Low)).map_or("", |m| m.name)
}

/// A move's weight as the score reads it: `"special"`, `"heavy"` or `"light"`.
fn weight(m: &MoveDef) -> &'static str {
    if m.has(Tag::Special) {
        "special"
    } else if m.has(Tag::Heavy) {
        "heavy"
    } else {
        "light"
    }
}

/// A move's largest hitstop (its hits' and its projectiles'), capped as the sim caps it, in steps.
fn hitstop(m: &MoveDef, max: u32) -> u32 {
    m.hits
        .iter()
        .map(|h| h.hitstop)
        .chain(m.spawns.iter().map(|s| s.proj.hit.hitstop))
        .max()
        .unwrap_or(0)
        .min(max)
}

/// The shadow recording of `rec` (fought by `defs`, left corner first) on `clock`, for the fight file whose grid is
/// `grid`:
///
/// - `names`, `seed`, `winner`, `fps` 30, `f0` (FIGHT!'s video frame) and `rules` `bpm`, `beat`, `t_fight` from
///   the grid; `t_ko` is the time map's K.O. on the video clock, the fight's own on the fight clock;
/// - `HP`: one frame per 1/30 s on the clock from FIGHT! to 1 s past the K.O., each the HP of the step shown then
///   (at least 0); left out when the fight did not record its steps (the seed search);
/// - the events in step order, each at its clock time rounded to 5 places, in the current engine's layouts plus
///   the v2 detail (see the module).
pub fn shadow(rec: &Recording2, defs: [&FighterDef; 2], map: &TimeMap, clock: Clock, grid: &Grid) -> Recording {
    let dt = rec.dt;
    let at_step = |step: u32| -> f64 {
        match clock {
            Clock::Video => map.step_video_t(step),
            Clock::Fight => rec.t_fight + step as f64 * dt,
        }
    };
    let t_ko = match clock {
        Clock::Video => map.ko_video_t(),
        Clock::Fight => rec.outcome.t_ko,
    };

    let mut rules = BTreeMap::new();
    rules.insert("bpm".to_string(), Value::Float(grid.bpm));
    rules.insert("beat".to_string(), Value::Float(grid.beat));
    rules.insert("t_fight".to_string(), Value::Float(grid.t_fight));

    let mut arrays = BTreeMap::new();
    if !rec.steps.is_empty() {
        let last = rec.steps.len() - 1;
        let end = t_ko.unwrap_or_else(|| at_step(last as u32)) + 1.0;
        let n = ((end - grid.t_fight) * f64::from(FPS)).floor().max(0.0) as usize + 1;
        let mut data = Vec::with_capacity(2 * n);
        for i in 0..n {
            let c = grid.t_fight + i as f64 / f64::from(FPS);
            let f = match clock {
                Clock::Video => map.fight_t(c),
                Clock::Fight => c - grid.t_fight,
            };
            let s = ((f / dt + 1e-9).floor().max(0.0) as usize).min(last);
            let hp = rec.steps[s].hp;
            data.extend((0..2).map(|k| hp_share(hp[k], defs[k])));
        }
        arrays.insert(
            "HP".to_string(),
            Array::F64 {
                shape: vec![n, 2],
                data,
            },
        );
    }

    Recording {
        format: RECORDING_FORMAT,
        source: Source::studio(),
        names: rec.names.to_vec(),
        seed: rec.seed,
        winner: rec.outcome.winner,
        t_ko,
        f0: py::round_half_even(grid.t_fight * f64::from(FPS)) as usize,
        fps: FPS,
        corner: Vec::new(),
        rules,
        events: events(rec, defs, &at_step, clock),
        arrays,
    }
}

/// The shadow events of `rec`, at `at_step` times.
fn events(rec: &Recording2, defs: [&FighterDef; 2], at_step: &dyn Fn(u32) -> f64, clock: Clock) -> Vec<Event> {
    let dt = rec.dt;
    // a span of `n` steps from `step`, in clock seconds
    let span = |step: u32, n: u32| match clock {
        Clock::Video => at_step(step + n) - at_step(step),
        Clock::Fight => n as f64 * dt,
    };
    let max_hitstop = Rules2::new(rec.t_fight).max_hitstop;
    let (int, float) = (|n: usize| Value::Int(n as i64), Value::Float);
    let text = |s: &str| Value::Str(s.to_string());
    let pos = |step: u32, who: usize| rec.steps.get(step as usize).map(|s| s.pos[who]);
    let x_of = |step: u32, who: usize| pos(step, who).map_or(0.0, |p| p[0]);
    // the move each fighter's last Active started, and the move each projectile came from
    let mut active: [Option<usize>; 2] = [None, None];
    let mut proj_move: BTreeMap<usize, usize> = BTreeMap::new();
    let mut consumed = vec![false; rec.events.len()];
    let mut out = Vec::new();

    for (i, e) in rec.events.iter().enumerate() {
        let who = e.who.min(1);
        let def = defs[who];
        let t = py::round_nd(at_step(e.step), 5);
        let mut put = |kind: &str, fields: Vec<Value>| {
            out.push(Event {
                t,
                kind: kind.to_string(),
                fields,
            })
        };
        let hit = |put: &mut dyn FnMut(&str, Vec<Value>), mv: usize, target: usize, point: [f64; 3], s: f64, d: f64| {
            let m = &def.moves[mv];
            let hs = hitstop(m, max_hitstop);
            let hs_s = hs as f64 * dt;
            put(
                "hit",
                vec![
                    int(who),
                    int(target),
                    float(d / defs[target.min(1)].hp_scale),
                    float(point[0]),
                    float(point[1]),
                    text(m.name),
                    Value::Int(0),
                    float(s),
                    text(weight(m)),
                    float(hs_s),
                ],
            );
            if hs >= FREEZE_STEPS {
                put(
                    "v2.freeze",
                    vec![int(who), int(target), float(s), float(hs_s), float(point[0])],
                );
            }
        };
        match &e.kind {
            EvKind::Active { mv } => active[who] = Some(*mv),
            EvKind::Spawn { proj } => {
                if let Some(mv) = active[who] {
                    proj_move.insert(*proj, mv);
                }
            }
            EvKind::Contact {
                mv,
                target,
                point,
                strength,
                damage,
                ..
            } if !consumed[i] => hit(&mut put, *mv, *target, *point, *strength, *damage),
            EvKind::ProjHit { proj, target, point } => {
                // what the hit did comes next in the same step: a Contact (it landed) or a Block
                let next = rec.events[i + 1..]
                    .iter()
                    .enumerate()
                    .take_while(|(_, n)| n.step == e.step)
                    .find(|(_, n)| {
                        n.who == e.who
                            && matches!(n.kind, EvKind::Contact { target: t, .. } | EvKind::Block { target: t, .. } if t == *target)
                    });
                match next {
                    Some((
                        k,
                        Ev {
                            kind:
                                EvKind::Contact {
                                    mv, strength, damage, ..
                                },
                            ..
                        },
                    )) => {
                        consumed[i + 1 + k] = true;
                        hit(&mut put, *mv, *target, *point, *strength, *damage);
                    }
                    Some(_) => {}
                    None => {
                        if let Some(&mv) = proj_move.get(proj) {
                            hit(&mut put, mv, *target, *point, PROJ_STRENGTH, 0.0);
                        }
                    }
                }
            }
            EvKind::Block { target, point } => put(
                "block",
                vec![int(*target), int(who), float(point[0]), float(point[1]), text("guard")],
            ),
            EvKind::Clash { point, .. } => put("clash", vec![Value::Int(-1), float(point[0]), float(point[1])]),
            EvKind::Bounce { what: Surface::Rope } => {
                let snap = rec.steps.get(e.step as usize);
                let (p, speed) = snap.map_or(([0.0; 3], ROPE_SPEED), |s| {
                    (s.pos[who], s.vel[who][0].hypot(s.vel[who][1]))
                });
                let ax = usize::from(p[1].abs() > p[0].abs());
                let side = if p[ax].is_sign_negative() { -1 } else { 1 };
                put(
                    "rope",
                    vec![
                        int(who),
                        int(ax),
                        Value::Int(side),
                        float(p[0]),
                        float(p[1]),
                        float(speed),
                    ],
                );
            }
            EvKind::Ability { mv, first } => {
                let m = &def.moves[*mv];
                let kind = if m.has(Tag::Low) { "low" } else { "special" };
                put(
                    "ability",
                    vec![int(who), text(m.name), text(kind), Value::Int(i64::from(*first))],
                );
            }
            EvKind::Launch { target, up } => put("v2.launch", vec![int(who), int(*target), float(*up)]),
            EvKind::Land => put("v2.land", vec![int(who), float(x_of(e.step, who))]),
            EvKind::Slam { target } => put(
                "v2.slam",
                vec![int(who), int(*target), float(x_of(e.step, (*target).min(1)))],
            ),
            EvKind::Windup { mv } => {
                let m = &def.moves[*mv];
                if m.has(Tag::Heavy) || m.has(Tag::Special) {
                    let len = span(e.step, m.windup);
                    put("v2.windup", vec![int(who), text(m.name), text(weight(m)), float(len)]);
                }
            }
            EvKind::Ko { target } => put("v2.ko", vec![int(who), int(*target)]),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::DirectCfg;
    use crate::testkit::{DT, dummy, ev, standing, synthetic};
    use bf_core::fightfile::FightFile;
    use bf_core::grid::Grid;
    use bf_core::py;
    use bf_core::show::Show;
    use bf_sound::music::{event_int, event_is_str, event_number};
    use bf2_sim::{EvKind, FighterDef, Recording2};
    use std::sync::Arc;

    fn def() -> Arc<FighterDef> {
        bf_content::fighters2::def("TEST DUMMY").expect("the dummy")
    }

    fn grid() -> Grid {
        Grid::new(132.0)
    }

    /// The first dummy seed from 1 on that ends in a K.O.
    fn ko_fight(record: bool) -> Recording2 {
        (1..40)
            .map(|s| dummy(s, record))
            .find(|r| r.outcome.t_ko.is_some())
            .expect("a seed ends in a K.O.")
    }

    fn both(rec: &Recording2, clock: Clock) -> (Recording, TimeMap) {
        let d = def();
        let map = TimeMap::from_recording(rec, &DirectCfg::default());
        (shadow(rec, [&d, &d], &map, clock, &grid()), map)
    }

    fn fight_file() -> FightFile {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fights/flute_vs_trombone.toml"
        ))
        .expect("the example fight file");
        let mut ff = FightFile::parse(&text).expect("it parses");
        ff.fighters = vec!["TEST DUMMY".into(), "TEST DUMMY".into()];
        ff
    }

    /// The steps of the `hit` events, in order: the Contacts and ProjHits that made them.
    fn hit_steps(rec: &Recording2) -> Vec<u32> {
        let mut out = Vec::new();
        for (i, e) in rec.events.iter().enumerate() {
            match e.kind {
                EvKind::ProjHit { .. } => out.push(e.step),
                EvKind::Contact { .. } if !after_proj_hit(&rec.events, i) => out.push(e.step),
                _ => {}
            }
        }
        out
    }

    fn after_proj_hit(evs: &[bf2_sim::Ev], i: usize) -> bool {
        evs[..i]
            .iter()
            .rev()
            .take_while(|p| p.step == evs[i].step)
            .any(|p| matches!(p.kind, EvKind::ProjHit { .. }))
    }

    #[test]
    fn shadow_passes_show_new() {
        let rec = ko_fight(true);
        let (sh, map) = both(&rec, Clock::Video);
        let show = Show::new(&sh, &fight_file()).expect("the show builds");
        assert_eq!(show.t_ko.to_bits(), map.ko_video_t().unwrap().to_bits());
        assert_eq!(Some(show.win), rec.outcome.winner);
        assert_eq!(sh.names, rec.names.to_vec());
        assert_eq!(sh.f0, 108);
        assert_eq!(sh.fps, 30);
        // the fight clock's show too, with the fight's own K.O. time
        let (sf, _) = both(&rec, Clock::Fight);
        assert_eq!(Show::new(&sf, &fight_file()).unwrap().t_ko, rec.outcome.t_ko.unwrap());
    }

    #[test]
    fn video_clock_times_follow_the_map() {
        let rec = ko_fight(true);
        let (sh, map) = both(&rec, Clock::Video);
        let hits: Vec<f64> = sh.events.iter().filter(|e| e.kind == "hit").map(|e| e.t).collect();
        let steps = hit_steps(&rec);
        assert!(!steps.is_empty());
        assert_eq!(hits.len(), steps.len());
        for (t, s) in hits.iter().zip(&steps) {
            assert_eq!(t.to_bits(), py::round_nd(map.step_video_t(*s), 5).to_bits());
        }
        // in time order
        assert!(sh.events.windows(2).all(|w| w[0].t <= w[1].t));
        // the slow-mo stretches the video clock: the K.O. is later than on the fight clock
        assert!(map.extra() > 0.0);
        assert!(sh.t_ko.unwrap() > rec.outcome.t_ko.unwrap());
    }

    #[test]
    fn fight_clock_times_are_steps() {
        let rec = ko_fight(true);
        let (sh, _) = both(&rec, Clock::Fight);
        let hits: Vec<f64> = sh.events.iter().filter(|e| e.kind == "hit").map(|e| e.t).collect();
        for (t, s) in hits.iter().zip(hit_steps(&rec)) {
            assert_eq!(t.to_bits(), py::round_nd(rec.t_fight + s as f64 * rec.dt, 5).to_bits());
        }
        assert_eq!(sh.t_ko, rec.outcome.t_ko);
    }

    #[test]
    fn hit_layout_reads_like_v1() {
        let rec = ko_fight(true);
        let d = def();
        let (sh, _) = both(&rec, Clock::Video);
        // the hits in order, each from a Contact or a ProjHit (whose Contact it takes)
        let sources: Vec<_> = rec
            .events
            .iter()
            .enumerate()
            .filter(|(i, e)| match e.kind {
                EvKind::ProjHit { .. } => true,
                EvKind::Contact { .. } => !after_proj_hit(&rec.events, *i),
                _ => false,
            })
            .map(|(_, e)| e)
            .collect();
        let hits: Vec<_> = sh.events.iter().filter(|e| e.kind == "hit").collect();
        assert_eq!(sources.len(), hits.len());
        let mut checked = 0;
        for (c, h) in sources.into_iter().zip(hits) {
            let EvKind::Contact {
                mv,
                target,
                damage,
                strength,
                point,
                ..
            } = c.kind
            else {
                continue;
            };
            assert_eq!(h.who(), Some(c.who));
            assert_eq!(event_number("t", h, 4).unwrap(), damage / d.hp_scale);
            assert_eq!(event_int("t", h, 3).unwrap(), target as i64);
            assert_eq!(event_number("t", h, 5).unwrap(), point[0]);
            assert_eq!(event_number("t", h, 6).unwrap(), point[1]);
            assert!(event_is_str("t", h, 7, d.moves[mv].name).unwrap());
            assert_eq!(event_int("t", h, 8).unwrap(), 0);
            assert_eq!(event_number("t", h, 9).unwrap(), strength);
            let tag = if d.moves[mv].has(bf2_sim::Tag::Special) {
                "special"
            } else if d.moves[mv].has(bf2_sim::Tag::Heavy) {
                "heavy"
            } else {
                "light"
            };
            assert!(event_is_str("t", h, 10, tag).unwrap());
            let hs = d.moves[mv].hits.iter().map(|h| h.hitstop).max().unwrap();
            assert_eq!(event_number("t", h, 11).unwrap(), hs as f64 * rec.dt);
            checked += 1;
        }
        assert!(checked > 3);
        // the ability captions read as v1's: e[3] the name, e[5] the first use
        let abilities: Vec<_> = sh.events.iter().filter(|e| e.kind == "ability").collect();
        for a in &abilities {
            let name = a.s(3);
            assert!(d.moves.iter().any(|m| m.name == name));
            let kind = if name == low_name(&d) { "low" } else { "special" };
            assert_eq!(a.s(4), kind);
            assert!(matches!(a.at(5), Value::Int(0 | 1)));
        }
    }

    #[test]
    fn hp_frames_match_steps() {
        let rec = ko_fight(true);
        for clock in [Clock::Video, Clock::Fight] {
            let (sh, map) = both(&rec, clock);
            let Some(Array::F64 { shape, data }) = sh.arrays.get("HP") else {
                panic!("an HP array")
            };
            assert_eq!(shape[1], 2);
            let n = shape[0];
            assert_eq!(data.len(), 2 * n);
            assert_eq!(&data[..2], &[100.0, 100.0], "out of 100, as the current engine's");
            let ko_step = rec
                .events
                .iter()
                .find(|e| matches!(e.kind, EvKind::Ko { .. }))
                .unwrap()
                .step;
            let last = rec.steps.len() - 1;
            let share = |hp: [f64; 2]| hp.map(|h| (h / def().hp_scale).max(0.0));
            let want = share(rec.steps[last].hp);
            assert_eq!(&data[2 * n - 2..], &want);
            assert!(want.contains(&0.0), "someone is out");
            assert_eq!(share(rec.steps[ko_step as usize].hp), want);
            // one frame a 1/30 s from the fight start to 1 s past the K.O.
            let end = sh.t_ko.unwrap() + 1.0;
            assert_eq!(n, ((end - grid().t_fight) * 30.0).floor() as usize + 1);
            // a frame mid-fight reads the step it shows
            let i = n / 2;
            let c = grid().t_fight + i as f64 / 30.0;
            let f = match clock {
                Clock::Video => map.fight_t(c),
                Clock::Fight => c - grid().t_fight,
            };
            let s = ((f / rec.dt + 1e-9).floor() as usize).min(last);
            assert_eq!(&data[2 * i..2 * i + 2], &share(rec.steps[s].hp));
            assert_eq!(sh.frames(), n);
        }
    }

    #[test]
    fn no_steps_no_hp() {
        let full = ko_fight(true);
        let bare = dummy(full.seed, false);
        assert!(bare.steps.is_empty());
        for clock in [Clock::Video, Clock::Fight] {
            let (a, _) = both(&full, clock);
            let (b, _) = both(&bare, clock);
            assert!(b.arrays.is_empty());
            assert_eq!(b.t_ko.map(f64::to_bits), a.t_ko.map(f64::to_bits));
            assert_eq!(a.events.len(), b.events.len());
            for (x, y) in a.events.iter().zip(&b.events) {
                assert_eq!((x.t.to_bits(), &x.kind), (y.t.to_bits(), &y.kind));
                // positions and speeds come from the steps; everything else from the events
                if !matches!(x.kind.as_str(), "rope" | "v2.land" | "v2.slam") {
                    assert_eq!(x, y);
                }
            }
            let rope = b.events.iter().find(|e| e.kind == "rope");
            if let Some(r) = rope {
                assert_eq!(r.f(7), 2.6);
            }
        }
    }

    #[test]
    fn freeze_follows_big_contacts_only() {
        // a light move with a 2-step hitstop and a heavy one with 10
        let mut d = (*def()).clone();
        let jab = d.moves.iter().position(|m| m.name == "JAB").unwrap();
        let hay = d.moves.iter().position(|m| m.name == "HAYMAKER").unwrap();
        for h in &mut d.moves[jab].hits {
            h.hitstop = 2;
        }
        for h in &mut d.moves[hay].hits {
            h.hitstop = 10;
        }
        let contact = |mv: usize, strength: f64| EvKind::Contact {
            mv,
            target: 1,
            point: [1.0, 0.5, 0.6],
            normal: [1.0, 0.0, 0.0],
            strength,
            damage: 3.0,
            bones: vec![1],
        };
        let rec = synthetic(
            1200,
            standing,
            vec![
                ev(100, 0, contact(jab, 0.3)),
                ev(400, 0, contact(hay, 0.7)),
                ev(900, 0, EvKind::Ko { target: 1 }),
            ],
            Some((1, 900)),
        );
        let map = TimeMap::from_recording(&rec, &DirectCfg::default());
        let sh = shadow(&rec, [&d, &d], &map, Clock::Fight, &grid());
        let kinds: Vec<&str> = sh.events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, ["hit", "hit", "v2.freeze", "v2.ko"]);
        let fr = &sh.events[2];
        assert_eq!(fr.t, sh.events[1].t);
        assert_eq!(event_int("t", fr, 2).unwrap(), 0);
        assert_eq!(event_int("t", fr, 3).unwrap(), 1);
        assert_eq!(fr.f(4), 0.7);
        assert_eq!(fr.f(5), 10.0 * DT);
        assert_eq!(fr.f(6), 1.0);
    }

    #[test]
    fn the_other_kinds_map_to_their_layouts() {
        let d = def();
        let toss = d.moves.iter().position(|m| m.name == "TOSS").unwrap();
        let buster = d.moves.iter().position(|m| m.name == "DUMMY BUSTER").unwrap();
        let wind = d.moves.iter().position(|m| m.name == "SECOND WIND").unwrap();
        let jab = d.moves.iter().position(|m| m.name == "JAB").unwrap();
        let p = [0.5, -0.25, 0.6];
        let rec = synthetic(
            1200,
            standing,
            vec![
                ev(10, 0, EvKind::Active { mv: toss }),
                ev(12, 0, EvKind::Spawn { proj: 7 }),
                ev(
                    50,
                    0,
                    EvKind::ProjHit {
                        proj: 7,
                        target: 1,
                        point: p,
                    },
                ),
                ev(
                    50,
                    0,
                    EvKind::Contact {
                        mv: toss,
                        target: 1,
                        point: p,
                        normal: [1.0, 0.0, 0.0],
                        strength: 0.45,
                        damage: 2.5,
                        bones: vec![],
                    },
                ),
                ev(60, 0, EvKind::Block { target: 1, point: p }),
                ev(70, 1, EvKind::Clash { other: 0, point: p }),
                ev(
                    80,
                    1,
                    EvKind::Bounce {
                        what: bf2_sim::Surface::Rope,
                    },
                ),
                ev(
                    81,
                    1,
                    EvKind::Bounce {
                        what: bf2_sim::Surface::Floor,
                    },
                ),
                ev(
                    90,
                    0,
                    EvKind::Ability {
                        mv: buster,
                        first: true,
                    },
                ),
                ev(91, 1, EvKind::Ability { mv: wind, first: false }),
                ev(100, 0, EvKind::Launch { target: 1, up: 6.5 }),
                ev(110, 1, EvKind::Land),
                ev(120, 0, EvKind::Slam { target: 1 }),
                ev(130, 0, EvKind::Windup { mv: buster }),
                ev(131, 0, EvKind::Windup { mv: jab }),
                ev(140, 0, EvKind::LowHp),
                ev(900, 0, EvKind::Ko { target: 1 }),
            ],
            Some((1, 900)),
        );
        let map = TimeMap::from_recording(&rec, &DirectCfg::default());
        let sh = shadow(&rec, [&d, &d], &map, Clock::Fight, &grid());
        let f = Value::Float;
        let i = Value::Int;
        let s = |x: &str| Value::Str(x.to_string());
        let got: Vec<(&str, &[Value])> = sh.events.iter().map(|e| (e.kind.as_str(), &e.fields[..])).collect();
        let hs = d.moves[toss].spawns[0].proj.hit.hitstop as f64 * DT;
        let want: Vec<(&str, Vec<Value>)> = vec![
            (
                "hit",
                vec![
                    i(0),
                    i(1),
                    f(2.5 / d.hp_scale),
                    f(0.5),
                    f(-0.25),
                    s("TOSS"),
                    i(0),
                    f(0.45),
                    s("light"),
                    f(hs),
                ],
            ),
            ("v2.freeze", vec![i(0), i(1), f(0.45), f(hs), f(0.5)]),
            ("block", vec![i(1), i(0), f(0.5), f(-0.25), s("guard")]),
            ("clash", vec![i(-1), f(0.5), f(-0.25)]),
            ("rope", vec![i(1), i(0), i(1), f(1.5), f(0.0), f(0.0)]),
            ("ability", vec![i(0), s("DUMMY BUSTER"), s("special"), i(1)]),
            ("ability", vec![i(1), s("SECOND WIND"), s("low"), i(0)]),
            ("v2.launch", vec![i(0), i(1), f(6.5)]),
            ("v2.land", vec![i(1), f(1.5)]),
            ("v2.slam", vec![i(0), i(1), f(1.5)]),
            (
                "v2.windup",
                vec![
                    i(0),
                    s("DUMMY BUSTER"),
                    s("special"),
                    f(d.moves[buster].windup as f64 * DT),
                ],
            ),
            ("v2.ko", vec![i(0), i(1)]),
        ];
        let want: Vec<(&str, &[Value])> = want.iter().map(|(k, v)| (*k, &v[..])).collect();
        assert_eq!(got, want);
        assert_eq!(low_name(&d), "SECOND WIND");
    }
}
