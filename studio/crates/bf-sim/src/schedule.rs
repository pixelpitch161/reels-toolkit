//! The order of a step: `Fight.run()` (`fight.py` 5735 to 6222), transcribed. This is the one place that says what
//! happens when, and the only place that calls the weapon kinds and the abilities' step. (An ability is fired inside
//! stage 12, by `Fight::abilities_step`.)
//!
//! `fight.py` runs every kind any fighter could have, each behind an `if`. The stages below are numbered in the
//! Python order and every stage names the lines it mirrors. Every kind of the kit has its call here already (a
//! socket), behind the same test Python makes (the fighters of the kind, `B.w['kind'] == ...`), calling the kind's
//! module; a kind not ported is a stub that no fighter reaches (its `is` is false until the roster reads the kind).
//! The older ten fighters' kinds (milestone 4) have their sockets too, calling stubs until their ports; the kinds
//! of frozen versions and of the retired fighters (`volley`, `orbit`, the `beam` weapon, `flail`, `aura`, `bellows`,
//! `dash`, `cone`, `pack`, `rattle`, `quake`, `charge`, `pull`) are `SLOT` comments at their place. Within a stage,
//! Python's order is the order of the lines named. With `Fight::trace_sockets` on, every socket logs its name
//! (`tests/sockets.rs` and `tests/sockets_older.rs` check the order against `tests/sockets_order.txt`).
//!
//! One step, in order:
//!
//! 1. the clock: `t = t_fight + (step + 1) * DT`;
//! 2. the beat grid: on a beat, each fighter's on-the-beat weapon trigger, in ball order (a stunned one has none);
//! 3. the 16th-note counters, one per kind, in the Python order (each kind has its own counter);
//! 4. the timers: shield, haste and slow run down, then stun, detune and the smashed timer (with `rebuild`);
//! 5. steps that steer a ball before it moves (pull, shadow, held notes);
//! 6. `move`: the balls fly, bounce and clash, then the projectiles;
//! 7. the rewind step;
//! 8. each fighter's weapon step, in ball order;
//! 9. lashes, beams and hazards;
//! 10. `effects_step`: waves, slams and arcs;
//! 11. the later kinds' steps, in the Python order;
//! 12. `abilities_step`: charged beams, levels, then on a beat the low-HP ability and the unlocks;
//! 13. `step += 1`.
//!
//! After each step [`run`] tests for a K.O. and records a frame every [`Schedule::steps_per_frame`] steps.

use bf_core::event::Value;
use bf_core::py;

use crate::abilities::{
    self, beam, clamp, flick, hook, lash, lights_out, prelude, rewind, smash, snap, snip, stampede, swan, x33,
};
use crate::fight::{Fight, stunned};
use crate::geom;
use crate::kinds::{
    anchor, bite, blade, combo, crowd, drill, echo, grapple, inflate, jump, lightning, line, mortar, pulse, rake,
    rests, roll, roll_charge, scratch_floor, spotlight, striker, surf, swallow, tether, tiptoe, trap, tune, walk,
    wrecking,
};
use crate::record::Frames;
use crate::roster::AbilityKind;

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
    /// `next16`: the pulse weapons' 16th counter (5745; DRUMS, milestone 4; the `on16` volleys share it, a frozen
    /// kind's). It moves only while a fighter has a pulse.
    pub next16: i64,
    /// `dc_next`: the dash / cone / surf / rake weapons' 16th counter (5752 to 5753; its `dc_k0` is `k16_0`, the
    /// same formula). Of those kinds the kit has the rake (ACOUSTIC) and UKULELE's surf (milestone 4).
    pub dc_next: i64,
    /// `n16b`: the `grid16` weapons' counter (5755: `inflate` and `bite`, milestone 4; `aura` and `bellows` are
    /// frozen kinds).
    pub n16b: i64,
    /// `next16c`: the tethers' (SYNTH, 5778).
    pub next16c: i64,
    /// `n16w`: the wreckers' (BARI SAX, 5845).
    pub n16w: i64,
    /// `n16v`: the resters' (VIOLA, 5861).
    pub n16v: i64,
    /// `n16e`: the echo's (HORN, 5790).
    pub n16e: i64,
    /// `n16j`: the jump's (CLARINET, 5796).
    pub n16j: i64,
    /// `n16u`: the tune's (OBOE, 5800).
    pub n16u: i64,
    /// `n16m`: the mortar's (EUPHONIUM, 5815).
    pub n16m: i64,
    /// `n16a`: the anchor's (BASS CLARINET, 5824).
    pub n16a: i64,
    /// `n16r`: the roll charge's (BASS DRUM, 5909). Python sets it up for the older `rattle` kind first (5839) and
    /// again here, and both blocks move it (6035 to 6039, 6091 to 6095): one counter, as in Python.
    pub n16r: i64,
    /// `n16l`: the lightning's (ELECTRIC, 5857).
    pub n16l: i64,
    /// `n16h`: the tiptoe's (BASSOON, 5833).
    pub n16h: i64,
    /// `n16g`: the grapple's (CELLO, 5867).
    pub n16g: i64,
    /// `n16k`: the walk's (CONTRABASS, 5879; not the strikers' `next16k`).
    pub n16k: i64,
    /// `n16o`: the roll's (PIANO, 5885). The roll gets the counter itself, not its distance from `k16_0`.
    pub n16o: i64,
    /// `n16s`: the spotlight's (ALTO SAX, 5891).
    pub n16s: i64,
    /// `n16c`: the crowd's (TENOR SAX, 5897).
    pub n16c: i64,
    /// `n16d`: the drill's (SOUSAPHONE, 5903).
    pub n16d: i64,
    // Python keeps one counter per kind (`next16`, `dc_next`, `n16b`, `next16c`, `next16k`, `next16t`, `n16p`, ...):
    // each moves only while a fighter has its kind, so they are separate fields, never one shared counter. The frozen
    // `pack`'s `n16p` is not kept.
}

/// The fighters (in ball order) that pass `is`, taken before a loop over them, as Python makes its lists (`echoers`,
/// `snippers`, ...) before it walks them.
fn which(fight: &Fight, is: impl Fn(&Fight, usize) -> bool) -> Vec<usize> {
    (0..2).filter(|&k| is(fight, k)).collect()
}

/// Fighter `k` has an ability of this kind (Python's lists `snippers`, `hookers`, `flickers`, ...).
fn has(fight: &Fight, k: usize, kind: AbilityKind) -> bool {
    abilities::has(fight, k, &kind)
}

impl Schedule {
    /// `run()` up to its loop (`fight.py` 5739 to 5914): the grid counters, then each kind's setup in the Python
    /// order. (The record's keys, which `run()` adds beside the setups, are `Frames::new`'s.)
    pub fn new(fight: &mut Fight) -> Schedule {
        let r = &fight.rules;
        let steps_per_frame = r.steps_per_frame();
        // first beat of the fight
        let k0 = (r.t_fight / r.beat - 1e-6).ceil() as i64;
        let s16 = r.beat / 4.0;
        // first 16th of the fight
        let k16_0 = (r.t_fight / s16 - 1e-6).ceil() as i64;
        // Kind setups, in the Python order. Each is `if a fighter has the kind: setup(k)`, some with a record key.
        // 5746 to 5755: the lists `pulse`, `rewinders`, `dc` and `grid16` are made by each block's own `is` test below
        // (`pullers` and `v16` are frozen kinds'); their counters start at `k16_0`.
        // 5756 to 5762: combo, then inflate (BAGPIPES, milestone 4), in ball order (flail is a frozen kind's: SLOT)
        fight.socket("setup.combo");
        fight.socket("setup.inflate");
        for k in 0..2 {
            if combo::is(fight, k) {
                combo::setup(fight, k);
            } else if inflate::is(fight, k) {
                inflate::setup(fight, k);
            }
        }
        // 5763 to 5764: `RAD` when a fighter inflates (`Frames::new`)
        // 5765 to 5774: the swallowers and the strikers, in ball order, one loop. The striker's `setup` also starts
        // `t_strike` at the weapon's (5770); `next16k = k16_0` is below; the record gets `COB` when a fighter is a
        // striker (`record.rs`).
        fight.socket("setup.swallow_striker");
        for k in 0..2 {
            if swallow::is(fight, k) {
                swallow::setup(fight, k);
            } else if striker::is_striker(fight, k) {
                striker::setup(fight, k);
            }
        }
        // 5775 to 5776: the thrusters need no setup (the lance's state is the ball's, from `Ball.__init__`);
        // `next16t = k16_0` is below. The record's `EXT`, `SLW`, `CRV` (5779 to 5780) follow `Fight::uses_ext`
        // (`record.rs`).
        // 5777 to 5778: the tethers need no setup (`next16c`, below; `_tether` makes the state on first use)
        //   5781 to 5786  pack (a frozen kind)
        // 5787 to 5792: echo
        fight.socket("setup.echo");
        for k in which(fight, echo::is) {
            echo::setup(fight, k);
        }
        // 5793 to 5796: jump
        fight.socket("setup.jump");
        for k in which(fight, jump::is) {
            jump::setup(fight, k);
        }
        // 5797 to 5802: tune
        fight.socket("setup.tune");
        for k in which(fight, tune::is) {
            tune::setup(fight, k);
        }
        // 5803 to 5808: the snippers and the dimmers need no setup (their keys `SNIP`, `LO`: `record.rs`)
        // 5809 to 5817: the shelling fight's `shells`, `next_salvo`, `salvo_landed` (`Fight`'s, empty) and the mortars
        fight.socket("setup.mortar");
        for k in which(fight, mortar::is) {
            mortar::setup(fight, k);
        }
        // 5818 to 5826: anchor (5821 to 5823, `anc = None` on both balls, is `Ball::anc`'s default)
        fight.socket("setup.anchor");
        for k in which(fight, anchor::is) {
            anchor::setup(fight, k);
        }
        // 5827 to 5829: the flickers need no setup (`FLICK`)
        // 5830 to 5835: tiptoe
        fight.socket("setup.tiptoe");
        for k in which(fight, tiptoe::is) {
            tiptoe::setup(fight, k);
        }
        //   5836 to 5841  rattle (a frozen kind)
        // 5842 to 5845: wrecking (BARI SAX, milestone 4); `WRECK` and the snappers' `SNAP` (5846 to 5850) are
        // `Frames::new`'s
        fight.socket("setup.wrecking");
        for k in which(fight, wrecking::is) {
            wrecking::setup(fight, k);
        }
        // 5851 to 5853: the hookers need no setup (`HOOK`)
        // 5854 to 5857: lightning
        fight.socket("setup.lightning");
        for k in which(fight, lightning::is) {
            lightning::setup(fight, k);
        }
        // 5858 to 5863: rests (VIOLA, milestone 4; `REST` and `NOTES` are `Frames::new`'s)
        fight.socket("setup.rests");
        for k in which(fight, rests::is) {
            rests::setup(fight, k);
        }
        // 5864 to 5869: grapple
        fight.socket("setup.grapple");
        for k in which(fight, grapple::is) {
            grapple::setup(fight, k);
        }
        // 5870 to 5875: the swans, stampeders and preluders need no setup
        // 5876 to 5881: walk
        fight.socket("setup.walk");
        for k in which(fight, walk::is) {
            walk::setup(fight, k);
        }
        // 5882 to 5887: roll
        fight.socket("setup.roll");
        for k in which(fight, roll::is) {
            roll::setup(fight, k);
        }
        // 5888 to 5893: spotlight
        fight.socket("setup.spotlight");
        for k in which(fight, spotlight::is) {
            spotlight::setup(fight, k);
        }
        // 5894 to 5899: crowd
        fight.socket("setup.crowd");
        for k in which(fight, crowd::is) {
            crowd::setup(fight, k);
        }
        // 5900 to 5905: drill
        fight.socket("setup.drill");
        for k in which(fight, drill::is) {
            drill::setup(fight, k);
        }
        // 5906 to 5911: roll_charge
        fight.socket("setup.roll_charge");
        for k in which(fight, roll_charge::is) {
            roll_charge::setup(fight, k);
        }
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
            dc_next: k16_0,
            n16e: k16_0,
            n16j: k16_0,
            n16u: k16_0,
            n16m: k16_0,
            n16a: k16_0,
            n16r: k16_0,
            n16l: k16_0,
            n16h: k16_0,
            n16g: k16_0,
            n16k: k16_0,
            n16o: k16_0,
            n16s: k16_0,
            n16c: k16_0,
            n16d: k16_0,
            next16: k16_0,
            n16b: k16_0,
            next16c: k16_0,
            n16w: k16_0,
            n16v: k16_0,
        }
    }

    /// One pass of the loop body, `fight.py` 5919 to 6204: everything from the clock to `step += 1`. The K.O. test
    /// and the frame record follow in [`run`].
    pub fn step(&mut self, fight: &mut Fight) {
        let dt = fight.rules.dt;

        // 1. The clock (5919). The time is recomputed from the step count, never accumulated.
        fight.socket("clock");
        fight.t = fight.rules.t_fight + (self.step + 1) as f64 * dt;

        // 2. The beat grid (5920 to 5945).
        self.on_beat = false;
        if fight.t + 1e-9 >= self.next_beat as f64 * fight.rules.beat {
            self.on_beat = true;
            self.kb = self.next_beat - self.k0;
            self.next_beat += 1;
            // For k in ball order (5925), by weapon kind (an `if` / `elif` chain; a ball has one weapon, so the `if`s
            // below are the same):
            for k in 0..2 {
                // 5926 to 5927: a stunned fighter starts no attack
                fight.socket("beat.stunned");
                if stunned(fight, k) {
                    continue;
                }
                //   5928 to 5933  volley (not on16), beam weapon (frozen kinds)
                // 5934 to 5935
                fight.socket("beat.combo");
                if combo::is(fight, k) {
                    combo::beat(fight, k, self.kb);
                }
                // 5936 to 5937
                fight.socket("beat.swallow");
                if swallow::is(fight, k) {
                    swallow::beat(fight, k, self.kb);
                }
                // 5938 to 5939: basket drops
                fight.socket("beat.striker");
                if striker::is_striker(fight, k) {
                    striker::beat(fight, k, self.kb);
                }
                // 5940 to 5941: `hazard_beat`, for a `line` or a `trap` (HARP, milestone 4: its branch)
                fight.socket("beat.line");
                if line::is(fight, k) {
                    line::beat(fight, k, self.kb);
                } else if trap::is(fight, k) {
                    trap::beat(fight, k, self.kb);
                }
                // 5942 to 5943 (TURNTABLES, milestone 4)
                fight.socket("beat.scratch_floor");
                if scratch_floor::is(fight, k) {
                    scratch_floor::beat(fight, k, self.kb);
                }
                // 5944 to 5945 (SYNTH, milestone 4)
                fight.socket("beat.tether");
                if tether::is(fight, k) {
                    tether::beat(fight, k, self.kb);
                }
            }
            // The thrust blade has no entry here: it starts on the 16th grid (stage 3).
        }

        // 3. The 16th-note counters (5946 to 6095), each `if <fighters of the kind> and t + 1e-9 >= <counter> * s16`,
        //    in this order:
        // 5946 to 5963: the pulses (`next16`, DRUMS, milestone 4), a stunned fighter skipped (the `on16` volleys of
        // 5958 to 5963 are a frozen kind's)
        fight.socket("tick.pulse");
        let pulsers = [0, 1].map(|k| pulse::is(fight, k));
        if pulsers.contains(&true) && fight.t + 1e-9 >= self.next16 as f64 * self.s16 {
            let k16 = self.next16 - self.k16_0;
            self.next16 += 1;
            for k in (0..2).filter(|&k| pulsers[k]) {
                if !stunned(fight, k) {
                    pulse::tick(fight, k, k16);
                }
            }
        }
        // 5964 to 5974: dash / cone / surf / rake (`dc_next`), a stunned fighter skipped; each kind's own block body
        // (`fire_<kind>`: the rake's, and UKULELE's surf, milestone 4; dash and cone are frozen kinds')
        fight.socket("tick.rake");
        let dc = [0, 1].map(|k| rake::is(fight, k) || surf::is(fight, k));
        if dc.contains(&true) && fight.t + 1e-9 >= self.dc_next as f64 * self.s16 {
            let dc_k = self.dc_next - self.k16_0;
            self.dc_next += 1;
            for k in (0..2).filter(|&k| dc[k]) {
                if stunned(fight, k) {
                    continue;
                }
                if rake::is(fight, k) {
                    rake::tick(fight, k, dc_k);
                } else {
                    surf::tick(fight, k, dc_k);
                }
            }
        }
        // 5975 to 5980: `grid16_fire` (`n16b`) for an `inflate` or a `bite` (BAGPIPES, ACCORDION, milestone 4: each
        // kind's branch), a stunned fighter skipped (`aura` and `bellows` are frozen kinds')
        fight.socket("tick.grid16");
        let grid16 = [0, 1].map(|k| inflate::is(fight, k) || bite::is(fight, k));
        if grid16.contains(&true) && fight.t + 1e-9 >= self.n16b as f64 * self.s16 {
            let k16b = self.n16b - self.k16_0;
            self.n16b += 1;
            for k in (0..2).filter(|&k| grid16[k]) {
                if stunned(fight, k) {
                    continue;
                }
                if inflate::is(fight, k) {
                    inflate::grid16_fire(fight, k, k16b);
                } else {
                    bite::grid16_fire(fight, k, k16b);
                }
            }
        }
        // 5981 to 5986: the tethers' cable pulses (`next16c`, SYNTH, milestone 4), a stunned fighter skipped
        fight.socket("tick.tether");
        self.counter(
            fight,
            tether::is,
            |s| &mut s.next16c,
            |f, k, k16| {
                if !stunned(f, k) {
                    tether::tick(f, k, k16);
                }
            },
        );
        // 5987 to 5993: the strikers' cobras lunge on the 16th grid, every `every` beats (haste divides it). The
        // counter moves only when a fighter is a striker (`strikers`, 5771, is the list of those fighters).
        fight.socket("tick.striker");
        let strikers = [0, 1].map(|k| striker::is_striker(fight, k));
        if strikers.contains(&true) && fight.t + 1e-9 >= self.next16k as f64 * self.s16 {
            let k16k = self.next16k - self.k16_0;
            self.next16k += 1;
            for k in (0..2).filter(|&k| strikers[k]) {
                // main16 = int(round(4 * w['every'])); k16k % max(1, int(round(main16 / ab_mult(k)))) == 0
                let main16 = striker::main16(fight, k);
                let every16 = (py::round_half_even(main16 as f64 / fight.ab_mult(k)) as i64).max(1);
                // `and not self._stunned(k)`
                if k16k % every16 == 0 && !stunned(fight, k) {
                    striker::fire(fight, k);
                }
            }
        }
        // 5994 to 6000: thrusts start on the 16th grid, every `every` beats (haste divides it). `thrusters` (5775)
        // is the list of the fighters whose weapon has `mode = 'thrust'`; the counter moves only when there is one.
        // Python fixes the list once before the loop and this recomputes it every step: the two are the same only
        // while a fighter's weapon cannot change during a fight, so a later kind that swaps weapons revisits this.
        fight.socket("tick.thrust");
        let thrusters = [0, 1].map(|k| fight.balls[k].thrusts());
        if thrusters.contains(&true) && fight.t + 1e-9 >= self.next16t as f64 * self.s16 {
            let k16t = self.next16t - self.k16_0;
            self.next16t += 1;
            for k in (0..2).filter(|&k| thrusters[k]) {
                // main16 = int(round(4 * w['every'])); k16t % max(1, int(round(main16 / ab_mult(k)))) == 0
                let main16 = blade::main16(fight, k);
                let every16 = (py::round_half_even(main16 as f64 / fight.ab_mult(k)) as i64).max(1);
                // `and not self._stunned(k)`
                if k16t % every16 == 0 && !stunned(fight, k) {
                    blade::start_thrust(fight, k);
                }
            }
        }
        //   6001 to 6008  pack (`n16p`, a frozen kind)
        // 6009 to 6014: echo, a stunned fighter skipped
        fight.socket("tick.echo");
        self.counter(
            fight,
            echo::is,
            |s| &mut s.n16e,
            |f, k, k16| {
                if !stunned(f, k) {
                    echo::tick(f, k, k16);
                }
            },
        );
        // 6015 to 6019: jump
        fight.socket("tick.jump");
        self.counter(fight, jump::is, |s| &mut s.n16j, jump::tick);
        // 6020 to 6024: tune
        fight.socket("tick.tune");
        self.counter(fight, tune::is, |s| &mut s.n16u, tune::tick);
        // 6025 to 6029: mortar
        fight.socket("tick.mortar");
        self.counter(fight, mortar::is, |s| &mut s.n16m, mortar::tick);
        // 6030 to 6034: anchor
        fight.socket("tick.anchor");
        self.counter(fight, anchor::is, |s| &mut s.n16a, anchor::tick);
        //   6035 to 6039  rattle (`n16r`, a frozen kind)
        // 6040 to 6044: wrecking (`n16w`, BARI SAX, milestone 4)
        fight.socket("tick.wrecking");
        self.counter(fight, wrecking::is, |s| &mut s.n16w, wrecking::tick);
        // 6045 to 6049: rests (`n16v`, VIOLA, milestone 4)
        fight.socket("tick.rests");
        self.counter(fight, rests::is, |s| &mut s.n16v, rests::tick);
        // 6050 to 6054: lightning
        fight.socket("tick.lightning");
        self.counter(fight, lightning::is, |s| &mut s.n16l, lightning::tick);
        // 6055 to 6059: tiptoe
        fight.socket("tick.tiptoe");
        self.counter(fight, tiptoe::is, |s| &mut s.n16h, tiptoe::tick);
        // 6060 to 6064: grapple
        fight.socket("tick.grapple");
        self.counter(fight, grapple::is, |s| &mut s.n16g, grapple::tick);
        // 6065 to 6069: walk
        fight.socket("tick.walk");
        self.counter(fight, walk::is, |s| &mut s.n16k, walk::tick);
        // 6070 to 6075: roll, a stunned fighter skipped; the roll gets the counter itself (`n16a_ = n16o`)
        fight.socket("tick.roll");
        let rollers = [0, 1].map(|k| roll::is(fight, k));
        if rollers.contains(&true) && fight.t + 1e-9 >= self.n16o as f64 * self.s16 {
            let n16 = self.n16o;
            self.n16o += 1;
            for k in (0..2).filter(|&k| rollers[k]) {
                if !stunned(fight, k) {
                    roll::tick(fight, k, n16);
                }
            }
        }
        // 6076 to 6080: spotlight
        fight.socket("tick.spotlight");
        self.counter(fight, spotlight::is, |s| &mut s.n16s, spotlight::tick);
        // 6081 to 6085: crowd
        fight.socket("tick.crowd");
        self.counter(fight, crowd::is, |s| &mut s.n16c, crowd::tick);
        // 6086 to 6090: drill
        fight.socket("tick.drill");
        self.counter(fight, drill::is, |s| &mut s.n16d, drill::tick);
        // 6091 to 6095: roll_charge (on `n16r`, the counter the older `rattle` shares)
        fight.socket("tick.roll_charge");
        self.counter(fight, roll_charge::is, |s| &mut s.n16r, roll_charge::tick);

        // 4. The timers (6096 to 6107).
        for k in 0..2 {
            let b = &mut fight.balls[k];
            b.shield = geom::py_max(0.0, b.shield - dt);
            b.haste = geom::py_max(0.0, b.haste - dt);
            b.slow = geom::py_max(0.0, b.slow - dt);
            fight.socket("timers");
            let b = &mut fight.balls[k];
            // 6100 to 6103: `if hasattr(B, 'stun')`, `if hasattr(B, 'detune_t')`: a ball without the attribute is at
            // 0.0 here, which stays 0.0
            b.stun = geom::py_max(0.0, b.stun - dt);
            b.detune_t = geom::py_max(0.0, b.detune_t - dt);
            // 6104 to 6107: the pieces snap back together (#44)
            if b.smashed > 0.0 {
                b.smashed = geom::py_max(0.0, b.smashed - dt);
                if b.smashed == 0.0 {
                    fight.ev("rebuild", k as i64, Vec::new());
                }
            }
        }

        // 5. Steps that steer a ball before it moves (6108 to 6114).
        //   6108 to 6109  `pull_step` for a weapon with `pull` (a frozen version's option)
        // 6110 to 6112: LIGHTS OUT, for either ball (a copied one too) while any fighter has the kind
        fight.socket("steer.lights_out");
        if (0..2).any(|k| has(fight, k, AbilityKind::LightsOut)) {
            for k in 0..2 {
                lights_out::lights_out_step(fight, k, dt);
            }
        }
        // 6113 to 6114: SUSTAIN's mines stop, hang, re-aim (the roll's)
        fight.socket("steer.hold");
        if (0..2).any(|k| roll::is(fight, k)) {
            roll::hold_step(fight);
        }

        // 6. Motion (6115).
        fight.move_step(dt);

        // 7. `rewind_step` (6116 to 6117): `if rewinders` (the fighters with a `rewind` ability: TURNTABLES, milestone
        //    4)
        fight.socket("rewind.step");
        let rewinders = which(fight, |f, k| has(f, k, AbilityKind::Rewind));
        if !rewinders.is_empty() {
            rewind::rewind_step(fight, dt, &rewinders);
        }

        // 8. Each fighter's weapon step (6118 to 6136), for k in ball order, by weapon kind. It is one loop over the
        //    balls: with TROMBONE on the left, its blade steps before the FLUTE's cobras.
        for k in 0..2 {
            // an `if` / `elif` chain on `B.w['kind']`, in this order (a ball has one weapon: the `if`s are the same)
            // 6119 to 6120: `blade_step`. In thrust mode that is the thrust step, which is also where a live lance
            // swats the enemy's cobras (`swat_cobras`, fight.py 753 to 754) and shells (`swat_shells`, 755 to 756).
            fight.socket("weapon.blade");
            if blade::is_blade(fight, k) {
                blade::step(fight, k, dt);
            }
            //   6121 to 6122  orbit (a frozen kind)
            // 6123 to 6124
            fight.socket("weapon.combo");
            if combo::is(fight, k) {
                combo::step(fight, k, dt);
            }
            //   6125 to 6126  flail (a frozen kind)
            // 6127 to 6128 (TURNTABLES, milestone 4)
            fight.socket("weapon.scratch_floor");
            if scratch_floor::is(fight, k) {
                scratch_floor::step(fight, k, dt);
            }
            // 6129 to 6130 (SYNTH, milestone 4)
            fight.socket("weapon.tether");
            if tether::is(fight, k) {
                tether::step(fight, k, dt);
            }
            // 6131 to 6132 (BAGPIPES, milestone 4)
            fight.socket("weapon.inflate");
            if inflate::is(fight, k) {
                inflate::step(fight, k, dt);
            }
            // 6133 to 6134
            fight.socket("weapon.swallow");
            if swallow::is(fight, k) {
                swallow::step(fight, k, dt);
            }
            // 6135 to 6136
            fight.socket("weapon.striker");
            if striker::is_striker(fight, k) {
                striker::step(fight, k, dt);
            }
        }

        // 9. Lashes, beams and hazards (6137 to 6142).
        // 6137 to 6138: `if getattr(self, 'lashes', None)` (SYNTH's SAWTOOTH, milestone 4)
        fight.socket("stage9.lashes");
        if lash::live(fight) {
            lash::lash_step(fight, dt);
        }
        // 6139 to 6140: `if self.beams or any(B.w['kind'] == 'beam' ...)`: the ability beams (TRUMPET's HIGH C; the
        // `beam` weapon is an older kind)
        fight.socket("stage9.beams");
        if beam::live(fight) {
            beam::beams_step(fight, dt);
        }
        // 6141 to 6142: `if self.lines or self.strings`: `hazards_step`, the pipes' half (1393 to 1416), then the
        // strings' half (1417 to 1434, HARP's `trap`, milestone 4: run only while there are strings, which is the same
        // as Python's loop over an empty list)
        fight.socket("stage9.hazards");
        if line::live(fight) || trap::live(fight) {
            line::hazards_step(fight, dt);
            if trap::live(fight) {
                trap::strings_step(fight, dt);
            }
        }

        // 10. `effects_step` (6143): the rings spread and hit, then the slams land, then the arcs emit. It runs on
        //     every step, whatever the fighters are.
        abilities::effects_step(fight, dt);

        // 11. The later kinds' steps (6144 to 6202), in this order.
        //   6144 to 6145  pack (a frozen kind)
        // 6146 to 6147
        fight.socket("later.echo");
        for k in which(fight, echo::is) {
            echo::step(fight, k, dt);
        }
        // 6148 to 6149
        fight.socket("later.flick");
        for k in which(fight, |f, k| has(f, k, AbilityKind::Flick)) {
            flick::flick_step(fight, k, dt);
        }
        // 6150 to 6151
        fight.socket("later.tune");
        for k in which(fight, tune::is) {
            tune::step(fight, k, dt);
        }
        // 6152 to 6153: the snippers
        fight.socket("later.snip");
        let snippers = [0, 1].map(|k| has(fight, k, AbilityKind::Snip));
        for k in (0..2).filter(|&k| snippers[k]) {
            snip::snip_step(fight, k, dt);
        }
        // 6154 to 6156: a snip IDENTITY CRISIS copied, for a fighter that is not a snipper
        fight.socket("later.extra_snip");
        let extra = fight.extra_snip;
        for k in (0..2).filter(|&k| extra[k] && !snippers[k]) {
            snip::snip_step(fight, k, dt);
        }
        // 6157 to 6158: `if shelling`
        fight.socket("later.shells");
        if mortar::shelling(fight) {
            mortar::shells_step(fight);
        }
        // 6159 to 6162: the whirl, for either ball whose `anc` is set (a copied one too), while a fighter anchors
        fight.socket("later.anchor");
        if (0..2).any(|k| anchor::is(fight, k)) {
            for k in which(fight, |f, k| f.balls[k].anc.is_some()) {
                anchor::step(fight, k, dt);
            }
        }
        // 6163 to 6164
        fight.socket("later.hook");
        for k in which(fight, |f, k| has(f, k, AbilityKind::Hook)) {
            hook::hook_step(fight, k, dt);
        }
        //   6165 to 6166  rattle (a frozen kind)
        // 6167 to 6168 (BARI SAX, milestone 4)
        fight.socket("later.wrecking");
        for k in which(fight, wrecking::is) {
            wrecking::step(fight, k, dt);
        }
        // 6169 to 6170: the snappers (BARI SAX's CABLE SNAP, milestone 4)
        fight.socket("later.snap");
        for k in which(fight, |f, k| has(f, k, AbilityKind::Snap)) {
            snap::snap_step(fight, k, dt);
        }
        // 6171 to 6172
        fight.socket("later.tiptoe");
        for k in which(fight, tiptoe::is) {
            tiptoe::step(fight, k, dt);
        }
        // 6173 to 6174
        fight.socket("later.walk");
        for k in which(fight, walk::is) {
            walk::step(fight, k, dt);
        }
        // 6175 to 6176
        fight.socket("later.grapple");
        for k in which(fight, grapple::is) {
            grapple::step(fight, k, dt);
        }
        // 6177 to 6178
        fight.socket("later.prelude");
        for k in which(fight, |f, k| has(f, k, AbilityKind::Prelude)) {
            prelude::prelude_step(fight, k);
        }
        // 6179 to 6180
        fight.socket("later.swan");
        for k in which(fight, |f, k| has(f, k, AbilityKind::Swan)) {
            swan::swan_step(fight, k);
        }
        // 6181 to 6182
        fight.socket("later.stampede");
        for k in which(fight, |f, k| has(f, k, AbilityKind::Stampede)) {
            stampede::stampede_step(fight, k, dt);
        }
        // 6183 to 6184
        fight.socket("later.spotlight");
        for k in which(fight, spotlight::is) {
            spotlight::step(fight, k, dt);
        }
        // 6185 to 6186
        fight.socket("later.crowd");
        for k in which(fight, crowd::is) {
            crowd::step(fight, k, dt);
        }
        // 6187 to 6188
        fight.socket("later.drill");
        for k in which(fight, drill::is) {
            drill::step(fight, k, dt);
        }
        // 6189 to 6190
        fight.socket("later.roll_charge");
        for k in which(fight, roll_charge::is) {
            roll_charge::step(fight, k, dt);
        }
        // 6191 to 6192: `if getattr(self, 'notes', None)` (VIOLA's whole notes, milestone 4)
        fight.socket("later.notes");
        if rests::live(fight) {
            rests::notes_step(fight, dt);
        }
        // 6193 to 6194: `if getattr(self, 'splinters', None)` (the smash's)
        fight.socket("later.splinters");
        if smash::live(fight) {
            smash::splinters_step(fight, dt);
        }
        // 6195 to 6196: `if dc` (the rake's branches of `dash_cone_step`)
        fight.socket("later.dash_cone");
        if dc.contains(&true) {
            rake::dash_cone_step(fight, dt);
        }
        // 6197 to 6198: `if dc or self.whips or self.sweeps or self.spouts` (the spouts are UKULELE's, milestone 4)
        fight.socket("later.surf_rake");
        if dc.contains(&true) || x33::live(fight) {
            rake::surf_rake_step(fight, dt);
        }
        //   6199 to 6200  suction (a frozen kind)
        // 6201 to 6202: `if self.clamps` (ACCORDION's FULL BELLOWS, milestone 4)
        fight.socket("later.clamps");
        if clamp::live(fight) {
            clamp::clamps_step(fight);
        }

        // 12. Levels and abilities (6203).
        fight.socket("abilities");
        fight.abilities_step(self.on_beat);

        // 13. (6204)
        self.step += 1;
    }

    /// One of `run()`'s plain 16th blocks (6009 to 6095): `if <the fighters of the kind> and t + 1e-9 >= n16 * s16:
    /// k16 = n16 - k16_0; n16 += 1; for k in <them>: <kind>_tick(k, k16)`, with the kind's own counter. The list is
    /// made before the block, as Python's is made before the loop.
    fn counter(
        &mut self,
        fight: &mut Fight,
        is: fn(&Fight, usize) -> bool,
        n16: fn(&mut Schedule) -> &mut i64,
        tick: fn(&mut Fight, usize, i64),
    ) {
        let who = [0, 1].map(|k| is(fight, k));
        let (s16, k16_0) = (self.s16, self.k16_0);
        let next = n16(self);
        if who.contains(&true) && fight.t + 1e-9 >= *next as f64 * s16 {
            let k16 = *next - k16_0;
            *next += 1;
            for k in (0..2).filter(|&k| who[k]) {
                tick(fight, k, k16);
            }
        }
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
    use crate::fight::run_with_unported as run_fight;
    use crate::testing;

    #[test]
    fn the_grid_counters_start_on_the_fights_first_beat() {
        let bare = testing::bare_pair();
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
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
        let sched = Schedule::new(&mut testing::fight(&setup, false));
        assert_eq!((sched.k0, sched.k16_0), (9, 33));
    }

    /// The first step of the fight is beat 0, and beats then come every `beat` seconds: at 120 steps a second and
    /// 0.4485 s a beat, after 54 or 53 steps.
    #[test]
    fn beats_fall_on_the_grid() {
        let bare = testing::bare_pair();
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
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
        let mut fight = testing::fight(&testing::bare_setup(&bare, 1), false);
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
        let rec = run_fight(&testing::bare_setup(&bare, 392), true).expect("the bare pair runs");
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
        let mut fight = testing::fight(&setup, true);
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
        let mut fight = testing::fight(&setup, false);
        fight.balls[0].hp = -3.0;
        fight.balls[1].hp = -1.0;
        assert!(run(&mut fight).is_none(), "nothing is recorded");
        assert_eq!(fight.winner, Some(1));
        let ko = fight.events.last().expect("the K.O.");
        assert_eq!(
            (ko.kind.as_str(), &ko.fields),
            ("ko", &vec![Value::Int(0), Value::Int(1)])
        );
        let mut fight = testing::fight(&setup, false);
        fight.balls[0].hp = -1.0;
        fight.balls[1].hp = -3.0;
        run(&mut fight);
        assert_eq!(fight.winner, Some(0));
        // equal: Python's min() keeps the first, the left fighter loses
        let mut fight = testing::fight(&setup, false);
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
        let mut fight = testing::fight(&setup, true);
        assert_eq!(Schedule::new(&mut fight).steps_per_frame, 1);
        fight.balls[0].hp = 0.0;
        let frames = run(&mut fight).expect("the fight records");
        assert_eq!(frames.len(), 2);
        assert_eq!(fight.winner, Some(1));
        // and without the K.O. every step is a frame
        let mut setup = setup.clone();
        setup.rules.t_max = setup.rules.t_fight + 0.1;
        let rec = run_fight(&setup, true).expect("the bare pair runs");
        assert_eq!(rec.frames(), 1 + 12, "0.1 s is 12 steps of 1/120 s");
    }
}
