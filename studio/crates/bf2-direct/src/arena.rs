//! The arena's reactions, precomputed for every frame: rope wobble, crowd jumps, LED flashes, the leader, the beat
//! and the house lights. `bf2-render`'s stage draws them.
//!
//! What happens in the ring follows fight time through the time map (a rope wobbles slowly in slow-mo and holds
//! still in the K.O. freeze); the beat follows video time on the episode's grid.

use bf_core::dmath;
use bf2_sim::{EvKind, Recording2, Surface};

use crate::cfg::{DirectCfg, FPS, FRAMES};
use crate::shots::{Priority, move_class};
use crate::time::TimeMap;

/// One frame of the arena.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArenaFrame {
    /// Each side's rope displacement (outward positive), in `bf2_render::stage::SIDES` order: sim +y (far), +x,
    /// −y (near), −x.
    pub rope_wobble: [f64; 4],
    /// How high the crowd is up, 0 to 1.
    pub crowd_jump: f64,
    /// The LEDs flashing in this fighter's colour, and how bright (0 to 1).
    pub led_flash: Option<(usize, f64)>,
    /// Who has more HP.
    pub leader: Option<usize>,
    /// A pulse on the grid's beat, 0 to 1.
    pub beat: f64,
    /// The house lights, 0 to 1; they dip to 0.35 through a special.
    pub house: f64,
}

impl Default for ArenaFrame {
    /// A quiet arena: still ropes, a seated crowd, no flash, no leader, no beat, the house lights up.
    fn default() -> Self {
        ArenaFrame {
            rope_wobble: [0.0; 4],
            crowd_jump: 0.0,
            led_flash: None,
            leader: None,
            beat: 0.0,
            house: 1.0,
        }
    }
}

/// The arena of every frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ArenaTrack {
    /// One a frame, [`FRAMES`] of them.
    pub frames: Vec<ArenaFrame>,
}

impl ArenaTrack {
    /// `bpm` is the grid's tempo (`Grid::bpm`, whose beats fall on multiples of `60 / bpm` from video time 0).
    pub fn build(rec: &Recording2, map: &TimeMap, bpm: f64, _cfg: &DirectCfg) -> ArenaTrack {
        let dt = rec.dt;
        let class = move_class(&rec.events);
        let mut ropes: Vec<(usize, f64, f64)> = Vec::new();
        let mut jumps: Vec<(f64, f64)> = Vec::new();
        let mut flashes: Vec<(usize, f64)> = Vec::new();
        let mut dims: Vec<(f64, f64)> = Vec::new();
        for (i, e) in rec.events.iter().enumerate() {
            let at = e.step as f64 * dt;
            let who = e.who.min(1);
            match e.kind {
                EvKind::Bounce { what: Surface::Rope } => {
                    let snap = rec.steps.get(e.step as usize);
                    let (pos, speed) = snap.map_or(([0.0; 3], ROPE_SPEED), |s| {
                        (s.pos[who], s.vel[who][0].hypot(s.vel[who][1]))
                    });
                    ropes.push((nearest_side(pos), at, (ROPE_AMP * speed).min(ROPE_MAX)));
                }
                EvKind::Contact { strength, .. } => {
                    if strength >= 0.6 {
                        jumps.push((at, strength.min(1.0)));
                    }
                    if strength >= 0.6 || class[i] >= Priority::Heavy {
                        flashes.push((who, at));
                    }
                }
                EvKind::Ko { .. } => jumps.push((at, 1.0)),
                EvKind::Windup { .. } if class[i] == Priority::Special => {
                    let rec_at = rec.events[i + 1..]
                        .iter()
                        .find(|r| r.who == e.who && matches!(r.kind, EvKind::Recovery { .. }))
                        .map_or(at + 0.5, |r| r.step as f64 * dt);
                    dims.push((at, rec_at.max(at)));
                }
                _ => {}
            }
        }

        let frames = (0..FRAMES)
            .map(|f| {
                let t = f as f64 / FPS;
                let ft = map.fight_t(t);
                let mut rope_wobble = [0.0; 4];
                for &(side, at, amp) in &ropes {
                    let tau = ft - at;
                    if (0.0..ROPE_LIFE).contains(&tau) {
                        rope_wobble[side] +=
                            amp * dmath::exp(-ROPE_DECAY * tau) * dmath::sin(dmath::TAU * ROPE_HZ * tau);
                    }
                }
                let crowd_jump = jumps.iter().map(|&(at, amp)| amp * jump(ft - at)).fold(0.0, f64::max);
                let led_flash = flashes
                    .iter()
                    .rev()
                    .find(|&&(_, at)| (0.0..FLASH_LIFE).contains(&(ft - at)))
                    .map(|&(who, at)| (who, dmath::exp(-(ft - at) / FLASH_DECAY)));
                let leader = (!rec.steps.is_empty()).then(|| {
                    let i = ((ft / dt).floor().max(0.0) as usize).min(rec.steps.len() - 1);
                    rec.steps[i].hp
                });
                let leader = match leader {
                    Some([a, b]) if a > b => Some(0),
                    Some([a, b]) if b > a => Some(1),
                    _ => None,
                };
                let dim = dims.iter().map(|&(from, to)| dim(ft, from, to)).fold(0.0, f64::max);
                ArenaFrame {
                    rope_wobble,
                    crowd_jump,
                    led_flash,
                    leader,
                    beat: beat(t, bpm),
                    house: 1.0 - (1.0 - HOUSE_LOW) * dim,
                }
            })
            .collect();
        ArenaTrack { frames }
    }
}

/// Rope wobble: amplitude per unit of the ball's speed, its most, the speed assumed without a recording, the
/// frequency, the decay per fight second, and when it is let go (by then under a 10,000th of its start).
const ROPE_AMP: f64 = 0.025;
const ROPE_MAX: f64 = 0.25;
const ROPE_SPEED: f64 = 6.0;
const ROPE_HZ: f64 = 4.0;
const ROPE_DECAY: f64 = 3.0;
const ROPE_LIFE: f64 = 3.0;
/// The crowd's jump: up in `JUMP_RISE`, down over `JUMP_FALL` (fight seconds).
const JUMP_RISE: f64 = 0.08;
const JUMP_FALL: f64 = 0.5;
/// An LED flash's decay and life, in fight seconds.
const FLASH_DECAY: f64 = 0.15;
const FLASH_LIFE: f64 = 0.6;
/// The house lights through a special: down over `DIM_IN`, back over `DIM_OUT` after its Recovery.
const HOUSE_LOW: f64 = 0.35;
const DIM_IN: f64 = 0.2;
const DIM_OUT: f64 = 0.5;
/// The beat pulse's decay, and its ramp up before each beat (as `Grid::beat_pulse`'s defaults).
const BEAT_DECAY: f64 = 0.12;
const BEAT_PRE: f64 = 0.12;

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The side whose ropes a ball at `pos` (sim) is nearest, in `SIDES` order: +y, +x, −y, −x.
fn nearest_side(pos: [f64; 3]) -> usize {
    if pos[0].abs() > pos[1].abs() {
        if pos[0] > 0.0 { 1 } else { 3 }
    } else if pos[1] > 0.0 {
        0
    } else {
        2
    }
}

/// The crowd's height `tau` fight seconds after a jump, 0 to 1.
fn jump(tau: f64) -> f64 {
    if tau < 0.0 {
        0.0
    } else if tau < JUMP_RISE {
        smoothstep(tau / JUMP_RISE)
    } else {
        1.0 - smoothstep((tau - JUMP_RISE) / JUMP_FALL)
    }
}

/// How far down the house lights are at `ft` for a special from `from` to its Recovery at `to`, 0 to 1.
fn dim(ft: f64, from: f64, to: f64) -> f64 {
    if ft < from {
        0.0
    } else if ft < to {
        smoothstep((ft - from) / DIM_IN)
    } else {
        // From wherever it got to, should the move have been shorter than the dim.
        smoothstep((to - from) / DIM_IN) * (1.0 - smoothstep((ft - to) / DIM_OUT))
    }
}

/// The beat pulse at video time `t`: 1 on each beat, decaying, and ramping up just before the next.
fn beat(t: f64, bpm: f64) -> f64 {
    if bpm.is_nan() || bpm <= 0.0 || t < 0.0 {
        return 0.0;
    }
    let period = 60.0 / bpm;
    let ph = t % period;
    let up = smoothstep((ph - (period - BEAT_PRE)) / BEAT_PRE);
    dmath::exp(-ph / BEAT_DECAY).max(up)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::*;
    use bf2_sim::{Cue, Ev, V3};

    fn arena(rec: &Recording2) -> (TimeMap, ArenaTrack) {
        let cfg = DirectCfg::default();
        let map = TimeMap::from_recording(rec, &cfg);
        let a = ArenaTrack::build(rec, &map, 120.0, &cfg);
        (map, a)
    }

    fn frame(t: f64) -> usize {
        (t * FPS).round() as usize
    }

    fn contact(step: u32, who: usize, strength: f64) -> Ev {
        ev(
            step,
            who,
            EvKind::Contact {
                mv: 0,
                target: 1 - who,
                point: [0.0; 3],
                normal: [1.0, 0.0, 0.0],
                strength,
                damage: 1.0,
                bones: vec![],
            },
        )
    }

    #[test]
    fn rope_wobbles_after_a_bounce_and_settles() {
        // Ball 1 hits the +x ropes at 8 a second.
        let pos = |_: u32| -> [V3; 2] { [[0.0, 0.0, 0.0], [2.4, 0.3, 0.0]] };
        let mut rec = synthetic(
            1200,
            pos,
            vec![ev(600, 1, EvKind::Bounce { what: Surface::Rope })],
            None,
        );
        rec.steps[600].vel[1] = [-8.0, 0.0, 0.0];
        let (map, a) = arena(&rec);
        let t = map.step_video_t(600);
        let w: Vec<[f64; 4]> = a.frames.iter().map(|f| f.rope_wobble).collect();
        assert_eq!(a.frames.len(), FRAMES);
        assert!(w[..frame(t)].iter().all(|x| *x == [0.0; 4]), "still before");
        let side: Vec<f64> = w[frame(t)..frame(t + 0.5)].iter().map(|x| x[1]).collect();
        let peak = side.iter().fold(0.0f64, |m, x| m.max(x.abs()));
        assert!(peak > 0.08 && peak < 0.3, "{peak}");
        assert!(
            side.iter().any(|x| *x > 0.02) && side.iter().any(|x| *x < -0.02),
            "it swings both ways"
        );
        assert!(
            w[frame(t)..].iter().all(|x| x[0] == 0.0 && x[2] == 0.0 && x[3] == 0.0),
            "only that side"
        );
        assert!(w[frame(t + 1.8)..].iter().all(|x| x[1].abs() < 0.002), "settles");
        assert!(w[frame(t + 3.5)..].iter().all(|x| x[1] == 0.0), "and stops");
        // a floor bounce moves no rope
        let rec = synthetic(
            1200,
            pos,
            vec![ev(600, 1, EvKind::Bounce { what: Surface::Floor })],
            None,
        );
        assert!(arena(&rec).1.frames.iter().all(|f| f.rope_wobble == [0.0; 4]));
    }

    #[test]
    fn crowd_jumps_on_big_hits_only() {
        let rec = synthetic(2400, standing, vec![contact(300, 0, 0.3), contact(1200, 1, 0.8)], None);
        let (map, a) = arena(&rec);
        let small = map.step_video_t(300);
        assert!(
            a.frames[frame(small)..frame(small + 1.0)]
                .iter()
                .all(|f| f.crowd_jump == 0.0)
        );
        let big = map.step_video_t(1200);
        let up = a.frames[frame(big + 0.08)].crowd_jump;
        assert!(up > 0.7 && up <= 0.8 + 1e-9, "{up}");
        assert!(a.frames[frame(big + 0.03)].crowd_jump < up, "it rises");
        assert!(a.frames[frame(big + 0.3)].crowd_jump < up, "and falls");
        assert!(a.frames[frame(big + 0.7)..].iter().all(|f| f.crowd_jump == 0.0));
        // the K.O. makes them jump all the way
        let rec = synthetic(
            1320,
            standing,
            vec![ev(1200, 0, EvKind::Ko { target: 1 })],
            Some((1, 1200)),
        );
        let (map, a) = arena(&rec);
        let ko = map.ko_video_t().unwrap();
        let top = a.frames[frame(ko)..frame(ko + 1.5)]
            .iter()
            .map(|f| f.crowd_jump)
            .fold(0.0, f64::max);
        assert!(top > 0.95, "{top}");
    }

    #[test]
    fn leds_flash_in_the_attackers_colour() {
        let rec = synthetic(2400, standing, vec![contact(300, 0, 0.3), contact(1200, 1, 0.7)], None);
        let (map, a) = arena(&rec);
        let small = map.step_video_t(300);
        assert!(
            a.frames[frame(small)..frame(small + 1.0)]
                .iter()
                .all(|f| f.led_flash.is_none())
        );
        let big = map.step_video_t(1200);
        let (who, v) = a.frames[frame(big) + 1].led_flash.expect("a flash");
        assert_eq!(who, 1);
        assert!(v > 0.5);
        assert!(a.frames[frame(big + 1.0)].led_flash.is_none());
    }

    #[test]
    fn leader_has_more_hp() {
        let mut rec = synthetic(1200, standing, vec![], None);
        for s in &mut rec.steps[600..] {
            s.hp = [80.0, 90.0];
        }
        let (map, a) = arena(&rec);
        assert_eq!(a.frames[frame(map.step_video_t(300))].leader, None);
        assert_eq!(a.frames[frame(map.step_video_t(700))].leader, Some(1));
    }

    #[test]
    fn house_dims_during_specials() {
        let special = vec![
            ev(600, 0, EvKind::MoveStart { mv: 4 }),
            ev(600, 0, EvKind::Windup { mv: 4 }),
            ev(636, 0, EvKind::Cue(Cue::SlowMo { scale: 0.5 })),
            ev(700, 0, EvKind::Recovery { mv: 4 }),
            ev(1200, 1, EvKind::MoveStart { mv: 1 }),
            ev(1200, 1, EvKind::Windup { mv: 1 }),
            ev(1220, 1, EvKind::Recovery { mv: 1 }),
        ];
        let rec = synthetic(2400, standing, special, None);
        let (map, a) = arena(&rec);
        let h = |s: u32| a.frames[frame(map.step_video_t(s))].house;
        assert_eq!(h(500), 1.0);
        assert!((h(680) - 0.35).abs() < 1e-9, "{}", h(680));
        assert!(h(620) < 1.0 && h(620) > 0.35, "it dims smoothly");
        assert!(h(710) > 0.35 && h(710) < 1.0, "and recovers smoothly");
        assert_eq!(h(800), 1.0);
        assert!((1150..1300).all(|s| h(s) == 1.0), "a plain move leaves the lights");
    }

    #[test]
    fn a_real_fight_stays_in_range() {
        let rec = dummy(3, true);
        let (map, a) = arena(&rec);
        let cfg = DirectCfg::default();
        assert_eq!(a, ArenaTrack::build(&rec, &map, 120.0, &cfg), "deterministic");
        assert!(a.frames.iter().all(|f| {
            f.rope_wobble.iter().all(|w| w.abs() <= ROPE_MAX * 2.0)
                && (0.0..=1.0).contains(&f.crowd_jump)
                && (HOUSE_LOW - 1e-12..=1.0).contains(&f.house)
                && (0.0..=1.0).contains(&f.beat)
                && f.led_flash.is_none_or(|(w, v)| w < 2 && (0.0..=1.0).contains(&v))
        }));
        // it reacts: the ropes, the crowd, the LEDs and the house lights all move at some point
        assert!(a.frames.iter().any(|f| f.rope_wobble.iter().any(|w| w.abs() > 0.01)));
        assert!(a.frames.iter().any(|f| f.crowd_jump > 0.5));
        assert!(a.frames.iter().any(|f| f.led_flash.is_some()));
        assert!(a.frames.iter().any(|f| f.house < 0.5));
        assert!(a.frames.iter().any(|f| f.leader.is_some()));
    }

    #[test]
    fn beat_follows_bpm() {
        let rec = synthetic(1200, standing, vec![], None);
        let (_, a) = arena(&rec);
        // 120 bpm: a beat every 0.5 s, every 15 frames
        for k in 0..100 {
            let f = 15 * k;
            assert!(a.frames[f].beat > 0.99, "{f}");
            assert!(a.frames[f + 7].beat < 0.15, "{f}");
        }
        assert!(a.frames[1].beat < a.frames[0].beat);
    }
}
