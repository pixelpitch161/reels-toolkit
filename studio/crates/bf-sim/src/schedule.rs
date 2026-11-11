//! The order of a step: `Fight.run()` (`fight.py` 5735 to 6222), transcribed. This is the one place that says what
//! happens when, and the only place that calls the weapon kinds and the abilities' step. (An ability is fired inside
//! stage 12, by `Fight::abilities_step`.)
//!
//! `fight.py` runs every kind any fighter could have, each behind an `if`. The stages below are numbered in the
//! Python order and every stage names the lines it mirrors. A stage whose kinds are not ported yet is a `SLOT`
//! comment: the next kind's call goes exactly there, and nothing already present moves. Within a stage, Python's
//! order is the order of the lines named.
//!
//! One step, in order:
//!
//! 1. the clock: `t = t_fight + (step + 1) * DT`;
//! 2. the beat grid: on a beat, each fighter's on-the-beat weapon trigger, in ball order;
//! 3. the 16th-note counters, one per kind, in the Python order (each kind has its own counter);
//! 4. the timers: shield, haste and slow run down;
//! 5. steps that steer a ball before it moves (pull, shadow, held notes);
//! 6. `move`: the balls fly, bounce and clash;
//! 7. the rewind step;
//! 8. each fighter's weapon step, in ball order;
//! 9. lashes, beams and hazards;
//! 10. `effects_step`: waves, slams and arcs;
//! 11. the later kinds' steps, in the Python order;
//! 12. `abilities_step`: levels, then on a beat the low-HP ability and the unlocks;
//! 13. `step += 1`.
//!
//! After each step [`run`] tests for a K.O. and records a frame every [`Schedule::steps_per_frame`] steps.

use bf_core::event::Value;
use bf_core::py;

use crate::abilities;
use crate::fight::Fight;
use crate::geom;
use crate::kinds::{blade, striker};
use crate::record::Frames;

/// The counters of the fight loop: what `run()` sets up before its `while` (`fight.py` 5739 to 5914) and moves
/// forward in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Schedule {
    /// `steps_per_frame`: steps between two recorded frames, `int(round(1.0 / (FPS * DT)))` (4).
    pub steps_per_frame: i64,
    /// `k0`: the fight's first beat, `int(ceil(t_fight / beat - 1e-6))`.
    pub k0: i64,
    /// `next_beat`: the next beat to reach, counted from the start of the video.
    pub next_beat: i64,
    /// `s16`: a sixteenth, `beat / 4.0`, in seconds.
    pub s16: f64,
    /// `k16_0`: the fight's first 16th, `int(ceil(t_fight / s16 - 1e-6))`. Every kind's 16th counter starts here.
    pub k16_0: i64,
    /// `step`: steps taken so far.
    pub step: i64,
    /// `on_beat`: whether the step just taken reached a beat.
    pub on_beat: bool,
    /// `kb`: the number of the last beat reached, counted from the fight's first beat (-1 before it). On a step with
    /// `on_beat`, this is the `kb` the weapon triggers of stage 2 get.
    pub kb: i64,
    /// `next16k`: the strikers' own 16th counter (`fight.py` 5772): the next 16th of the video their strike grid
    /// reaches. It starts at `k16_0` and moves only while a fighter is a striker.
    pub next16k: i64,
    /// `next16t`: the thrusters' own 16th counter (`fight.py` 5776): the next 16th of the video the thrust grid
    /// reaches. It starts at `k16_0` and moves only while a fighter's weapon thrusts.
    pub next16t: i64,
    // Python keeps one counter per kind (`next16`, `dc_next`, `n16b`, `next16c`, `next16k`, `next16t`, `n16p`, ...):
    // each moves only while a fighter has its kind, so they are separate fields, never one shared counter.
}

impl Schedule {
    /// `run()` up to its loop (`fight.py` 5739 to 5914): the grid counters, then each kind's setup in the Python
    /// order.
    pub fn new(fight: &mut Fight) -> Schedule {
        let r = &fight.rules;
        let steps_per_frame = r.steps_per_frame();
        // first beat of the fight
        let k0 = (r.t_fight / r.beat - 1e-6).ceil() as i64;
        let s16 = r.beat / 4.0;
        // first 16th of the fight
        let k16_0 = (r.t_fight / s16 - 1e-6).ceil() as i64;
        // Kind setups, in the Python order. Each is `if a fighter has the kind: setup(k)`, some with a record key.
        //   5756 to 5764  combo, flail, inflate setups (kinds not ported)
        // 5765 to 5774: the strikers, in ball order (the same loop sets up `swallow`, a kind not ported). `setup`
        // also starts `t_strike` at the weapon's (5770); `next16k = k16_0` is below; the record gets `COB` when a
        // fighter is a striker (`record.rs`).
        for k in 0..2 {
            if striker::is_striker(fight, k) {
                striker::setup(fight, k);
            }
        }
        // 5775 to 5776: the thrusters need no setup (the lance's state is the ball's, from `Ball.__init__`);
        // `next16t = k16_0` is below. The record's `EXT`, `SLW`, `CRV` (5779 to 5780) follow `Fight::uses_ext`
        // (`record.rs`).
        //   5777 to 5911  tether, pack, echo, jump, tune, snip, lights_out, mortar, anchor, flick, tiptoe, rattle,
        //     wrecking, snap, hook, lightning, rests, grapple, swan, stampede, prelude, walk, roll, spotlight, crowd,
        //     drill, roll_charge (kinds not ported), in that order.
        Schedule {
            steps_per_frame,
            k0,
            next_beat: k0,
            s16,
            k16_0,
            step: 0,
            on_beat: false,
            kb: -1,
            next16k: k16_0,
            next16t: k16_0,
        }
    }

    /// One pass of the loop body, `fight.py` 5919 to 6204: everything from the clock to `step += 1`. The K.O. test
    /// and the frame record follow in [`run`].
    pub fn step(&mut self, fight: &mut Fight) {
        let dt = fight.rules.dt;

        // 1. The clock (5919). The time is recomputed from the step count, never accumulated.
        fight.t = fight.rules.t_fight + (self.step + 1) as f64 * dt;

        // 2. The beat grid (5920 to 5945).
        self.on_beat = false;
        if fight.t + 1e-9 >= self.next_beat as f64 * fight.rules.beat {
            self.on_beat = true;
            self.kb = self.next_beat - self.k0;
            self.next_beat += 1;
            // For k in ball order (5925), skipping a stunned fighter (5926, `_stunned`: stun and shadow, kinds not
            // ported), by weapon kind:
            for k in 0..2 {
                //   5928 to 5937  volley (not on16), beam, combo, swallow (kinds not ported)
                if striker::is_striker(fight, k) {
                    // 5938 to 5939: basket drops
                    striker::beat(fight, k, self.kb);
                }
                //   5940 to 5945  line / trap, scratch_floor, tether (kinds not ported)
            }
            // The thrust blade has no entry here: it starts on the 16th grid (stage 3).
        }

        // 3. The 16th-note counters (5946 to 6095), each `if <fighters of the kind> and t + 1e-9 >= <counter> * s16`,
        //    in this order:
        //   5946 to 5986  pulse / volley on16 (`next16`), dash / cone / surf / rake (`dc_next`), aura / bellows /
        //     inflate / bite (`n16b`), tether (`next16c`) (kinds not ported)
        // 5987 to 5993: the strikers' cobras lunge on the 16th grid, every `every` beats (haste divides it). The
        // counter moves only when a fighter is a striker (`strikers`, 5771, is the list of those fighters).
        let strikers = [0, 1].map(|k| striker::is_striker(fight, k));
        if strikers.contains(&true) && fight.t + 1e-9 >= self.next16k as f64 * self.s16 {
            let k16k = self.next16k - self.k16_0;
            self.next16k += 1;
            for k in (0..2).filter(|&k| strikers[k]) {
                // main16 = int(round(4 * w['every'])); k16k % max(1, int(round(main16 / ab_mult(k)))) == 0
                let main16 = striker::main16(fight, k);
                let every16 = (py::round_half_even(main16 as f64 / fight.ab_mult(k)) as i64).max(1);
                // (`and not self._stunned(k)`: stun and shadow, kinds not ported)
                if k16k % every16 == 0 {
                    striker::fire(fight, k);
                }
            }
        }
        // 5994 to 6000: thrusts start on the 16th grid, every `every` beats (haste divides it). `thrusters` (5775)
        // is the list of the fighters whose weapon has `mode = 'thrust'`; the counter moves only when there is one.
        // Python fixes the list once before the loop and this recomputes it every step: the two are the same only
        // while a fighter's weapon cannot change during a fight, so a later kind that swaps weapons revisits this.
        let thrusters = [0, 1].map(|k| fight.balls[k].thrusts());
        if thrusters.contains(&true) && fight.t + 1e-9 >= self.next16t as f64 * self.s16 {
            let k16t = self.next16t - self.k16_0;
            self.next16t += 1;
            for k in (0..2).filter(|&k| thrusters[k]) {
                // main16 = int(round(4 * w['every'])); k16t % max(1, int(round(main16 / ab_mult(k)))) == 0
                let main16 = blade::main16(fight, k);
                let every16 = (py::round_half_even(main16 as f64 / fight.ab_mult(k)) as i64).max(1);
                // (`and not self._stunned(k)`: stun and shadow, kinds not ported)
                if k16t % every16 == 0 {
                    blade::start_thrust(fight, k);
                }
            }
        }
        //   6001 to 6095  pack, echo, jump, tune, mortar, anchor, rattle, wrecking, rests, lightning, tiptoe,
        //     grapple, walk, roll, spotlight, crowd, drill, roll_charge (kinds not ported), in that order.

        // 4. The timers (6096 to 6107).
        for b in &mut fight.balls {
            b.shield = geom::py_max(0.0, b.shield - dt);
            b.haste = geom::py_max(0.0, b.haste - dt);
            b.slow = geom::py_max(0.0, b.slow - dt);
            // 6100 to 6107: stun, detune and the smashed timer with its `rebuild` event (kinds not ported).
        }

        // 5. Steps that steer a ball before it moves (6108 to 6114): `pull_step` for a weapon with `pull`,
        //    `lights_out_step`, `hold_step` (kinds not ported).

        // 6. Motion (6115).
        fight.move_step(dt);

        // 7. `rewind_step` (6116 to 6117) (kind not ported).

        // 8. Each fighter's weapon step (6118 to 6136), for k in ball order, by weapon kind. It is one loop over the
        //    balls: with TROMBONE on the left, its blade steps before the FLUTE's cobras.
        for k in 0..2 {
            // an `if` / `elif` chain on `B.w['kind']`, in this order:
            if blade::is_blade(fight, k) {
                // 6119 to 6120: `blade_step`. In thrust mode that is the thrust step, which is also where a live
                // lance swats the enemy's cobras (`swat_cobras`, fight.py 753 to 754).
                blade::step(fight, k, dt);
            } else if striker::is_striker(fight, k) {
                // 6121 to 6134 come first: orbit, combo, flail, scratch_floor, tether, inflate, swallow (kinds not
                // ported). 6135 to 6136:
                striker::step(fight, k, dt);
            }
        }

        // 9. `lash_step`, `beams_step`, `hazards_step` (6137 to 6142) (kinds not ported).

        // 10. `effects_step` (6143): the rings spread and hit, then the slams land. It runs on every step, whatever
        //     the fighters are.
        abilities::effects_step(fight, dt);

        // 11. The later kinds' steps (6144 to 6202), in this order: pack, echo, flick, tune, snip, shells, anchor,
        //     hook, rattle, wrecking, snap, tiptoe, walk, grapple, prelude, swan, stampede, spotlight, crowd, drill,
        //     roll_charge, notes, splinters, dash / cone, surf / rake, suction, clamps (kinds not ported).

        // 12. Levels and abilities (6203).
        fight.abilities_step(self.on_beat);

        // 13. (6204)
        self.step += 1;
    }
}

/// `Fight.run()`: the whole fight. Returns the recorded frames when the fight records.
///
/// The loop (`fight.py` 5912 to 6215): frame `f0` is recorded before the first step; then steps until `t_max`, with
/// the K.O. test after each one. A K.O. is recorded as a last frame of its own, whenever it falls; otherwise a frame
/// is recorded every `steps_per_frame` steps.
pub(crate) fn run(fight: &mut Fight) -> Option<Frames> {
    let mut sched = Schedule::new(fight);
    let mut rec = fight.record.then(|| Frames::new(fight));
    fight.t = fight.rules.t_fight;
    if let Some(rec) = &mut rec {
        rec.snap(fight); // frame f0
    }
    while fight.t < fight.rules.t_max {
        sched.step(fight);
        // The K.O. test (6205 to 6213): whoever is at 0 HP or below; of two, the lower (the left one if equal).
        let dead: Vec<usize> = (0..2).filter(|&k| fight.balls[k].hp <= 0.0).collect();
        if let Some(&first) = dead.first() {
            let lo = dead[1..].iter().fold(
                first,
                |lo, &k| {
                    if fight.balls[k].hp < fight.balls[lo].hp { k } else { lo }
                },
            );
            fight.winner = Some(1 - lo);
            fight.t_ko = Some(fight.t);
            fight.ev("ko", lo as i64, vec![Value::Int((1 - lo) as i64)]);
            if let Some(rec) = &mut rec {
                rec.snap(fight);
            }
            break;
        }
        if let Some(rec) = &mut rec
            && sched.step % sched.steps_per_frame == 0
        {
            rec.snap(fight);
        }
    }
    rec
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fight::run as run_fight;
    use crate::testing;

    #[test]
    fn the_grid_counters_start_on_the_fights_first_beat() {
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
        let sched = Schedule::new(&mut fight);
        // the fight starts on the bar-2 downbeat: beat 8, 16th 32 of the video
        assert_eq!(
            (sched.steps_per_frame, sched.k0, sched.next_beat, sched.k16_0),
            (4, 8, 8, 32)
        );
        assert_eq!(sched.s16.to_bits(), (fight.rules.beat / 4.0).to_bits());
        assert_eq!((sched.step, sched.on_beat, sched.kb), (0, false, -1));
        // a start between beats: the first beat is the next one
        let mut setup = testing::bare_setup(&bare, 1);
        setup.rules.t_fight += 0.1;
        let sched = Schedule::new(&mut Fight::new(&setup, false));
        assert_eq!((sched.k0, sched.k16_0), (9, 33));
    }

    /// The first step of the fight is beat 0, and beats then come every `beat` seconds: at 120 steps a second and
    /// 0.4485 s a beat, after 54 or 53 steps.
    #[test]
    fn beats_fall_on_the_grid() {
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
        let mut sched = Schedule::new(&mut fight);
        let (t_fight, dt, beat) = (fight.rules.t_fight, fight.rules.dt, fight.rules.beat);
        let mut beats = Vec::new();
        for n in 1..=200i64 {
            sched.step(&mut fight);
            assert_eq!(sched.step, n);
            assert_eq!(fight.t.to_bits(), (t_fight + n as f64 * dt).to_bits());
            if sched.on_beat {
                beats.push((n, sched.kb));
                // the first step at or after the beat (within the 1e-9 slack); the fight's first step is its beat 0
                let t_beat = (8 + sched.kb) as f64 * beat;
                assert!(fight.t + 1e-9 >= t_beat, "step {n}");
                assert!(n == 1 || fight.t - dt + 1e-9 < t_beat, "step {n}");
            }
        }
        assert_eq!(beats, vec![(1, 0), (54, 1), (108, 2), (162, 3)]);
        assert_eq!((sched.next_beat, sched.kb), (12, 3));
    }

    #[test]
    fn the_timers_run_down_to_zero() {
        let bare = testing::bare_pair();
        let mut fight = Fight::new(&testing::bare_setup(&bare, 1), false);
        let mut sched = Schedule::new(&mut fight);
        let dt = fight.rules.dt;
        fight.balls[0].shield = 2.5 * dt;
        fight.balls[0].haste = 0.5 * dt;
        fight.balls[1].slow = 1.5 * dt;
        sched.step(&mut fight);
        assert_eq!(
            (fight.balls[0].shield, fight.balls[0].haste, fight.balls[1].slow),
            (2.5 * dt - dt, 0.0, 1.5 * dt - dt)
        );
        sched.step(&mut fight);
        sched.step(&mut fight);
        let b = &fight.balls;
        assert_eq!(
            (b[0].shield, b[0].haste, b[0].slow, b[1].shield, b[1].haste, b[1].slow),
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        );
    }

    /// With nobody able to hit, the fight runs to `t_max`: no winner, no K.O. time, and the frame count the Python
    /// fight with no K.O. has (`no_ko.bfrec`: 1,633 frames; 6,530 steps, a frame every 4, plus frame `f0`).
    #[test]
    fn a_fight_without_a_ko_runs_to_t_max() {
        let bare = testing::bare_pair();
        let rec = run_fight(&testing::bare_setup(&bare, 392), true);
        assert_eq!((rec.winner, rec.t_ko), (None, None));
        assert_eq!(rec.frames(), testing::python_recording("no_ko.bfrec").frames());
        assert_eq!(rec.frames(), 1633);
        assert!(rec.events.iter().all(|e| e.kind == "rope" || e.kind == "clash"));
        assert!(rec.events.iter().any(|e| e.kind == "rope") && rec.events.iter().any(|e| e.kind == "clash"));
        // events are in time order, all inside the fight
        assert!(rec.events.windows(2).all(|w| w[0].t <= w[1].t));
        let last = rec.events.last().expect("there are events").t;
        assert!(last > 50.0 && last <= 58.01, "{last}");
    }

    /// A fighter at 0 HP or below after a step is knocked out at that step's time: the `ko` event names the loser
    /// and the winner, and the state right after that step is the last frame.
    #[test]
    fn a_ko_ends_the_fight_with_a_last_frame() {
        let bare = testing::bare_pair();
        let setup = testing::bare_setup(&bare, 1);
        let mut fight = Fight::new(&setup, true);
        fight.balls[1].hp = 0.0;
        let frames = run(&mut fight).expect("the fight records");
        let t1 = setup.rules.t_fight + setup.rules.dt;
        assert_eq!((fight.winner, fight.t_ko), (Some(0), Some(t1)));
        // The fight's first step is a beat, and a fighter below LOW_HP has its low turn there: the bare fighter's
        // ability is announced (and does nothing: its kind is not ported) before the K.O. is found, as in Python.
        assert_eq!(fight.events.len(), 2);
        let (low, e) = (&fight.events[0], &fight.events[1]);
        assert_eq!(
            (low.t, low.kind.as_str(), low.fields.as_slice()),
            (
                bf_core::py::round_nd(t1, 5),
                "ability",
                &[
                    Value::Int(1),
                    Value::Str("NOTHING".into()),
                    Value::Str("none".into()),
                    Value::Int(1)
                ][..]
            )
        );
        assert_eq!(
            (e.t, e.kind.as_str(), e.fields.as_slice()),
            (bf_core::py::round_nd(t1, 5), "ko", &[Value::Int(1), Value::Int(0)][..])
        );
        // frame f0 and the K.O. frame, one step later (not a regular frame time)
        assert_eq!(frames.len(), 2);

        // both down in the same step: the lower HP loses
        let mut fight = Fight::new(&setup, false);
        fight.balls[0].hp = -3.0;
        fight.balls[1].hp = -1.0;
        assert!(run(&mut fight).is_none(), "nothing is recorded");
        assert_eq!(fight.winner, Some(1));
        let ko = fight.events.last().expect("the K.O.");
        assert_eq!(
            (ko.kind.as_str(), &ko.fields),
            ("ko", &vec![Value::Int(0), Value::Int(1)])
        );
        let mut fight = Fight::new(&setup, false);
        fight.balls[0].hp = -1.0;
        fight.balls[1].hp = -3.0;
        run(&mut fight);
        assert_eq!(fight.winner, Some(0));
        // equal: Python's min() keeps the first, the left fighter loses
        let mut fight = Fight::new(&setup, false);
        fight.balls[0].hp = -2.0;
        fight.balls[1].hp = -2.0;
        run(&mut fight);
        assert_eq!(fight.winner, Some(1));
    }

    /// A K.O. that falls on a regular frame's step is recorded once, not twice. (At 120 frames a second every step
    /// is a frame's step.)
    #[test]
    fn a_ko_on_a_frame_step_is_one_frame() {
        let bare = testing::bare_pair();
        let mut setup = testing::bare_setup(&bare, 1);
        setup.rules.fps = 120;
        let mut fight = Fight::new(&setup, true);
        assert_eq!(Schedule::new(&mut fight).steps_per_frame, 1);
        fight.balls[0].hp = 0.0;
        let frames = run(&mut fight).expect("the fight records");
        assert_eq!(frames.len(), 2);
        assert_eq!(fight.winner, Some(1));
        // and without the K.O. every step is a frame
        let mut setup = setup.clone();
        setup.rules.t_max = setup.rules.t_fight + 0.1;
        let rec = run_fight(&setup, true);
        assert_eq!(rec.frames(), 1 + 12, "0.1 s is 12 steps of 1/120 s");
    }
}
