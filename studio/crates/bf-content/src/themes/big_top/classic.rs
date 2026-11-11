//! The big top's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/big_top.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in
//! that order, and the calliope's seeds count the bars).

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, MusicCtx, MusicError, RISER_OVER, RISER_SEED, clank, noise_seed, riser,
};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, Played};

use super::{CALLIOPE_ATT, CALLIOPE_VIB, bass_drum, calliope, choke, organ, roll, snare, tambourine, voice};

/// The name the bed's errors carry.
const H: &str = "big_top";

/// The big top's bed as the kit plays it.
#[derive(Clone, Copy, Debug, Default)]
pub struct BigTopClassic;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `T.CHORDS[b]`.
fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    chords.get(b).map(String::as_str).ok_or_else(|| {
        MusicError::new(
            H,
            format!("bar {b} is past the episode's last bar ({} bars)", chords.len()),
        )
    })
}

/// `T.CHORDS[-1]`.
fn last_chord<'a>(ctx: &'a MusicCtx) -> Result<&'a str, MusicError> {
    chord(ctx, ctx.harmony.chords.len().saturating_sub(1))
}

/// `self.calliope(self.voice(ch), ...)`.
fn chunk(ctx: &MusicCtx, ch: &str, dur: f64, vel: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
    let notes = voice(&ctx.harmony, ch).map_err(bad)?;
    Ok(calliope(&notes, dur, vel, seed, CALLIOPE_ATT, CALLIOPE_VIB))
}

/// `self.calliope(self.organ(ch), ...)`.
fn held(ctx: &MusicCtx, ch: &str, dur: f64, vel: f64, seed: u64, att: f64, vib: f64) -> Result<Vec<f64>, MusicError> {
    let notes = organ(&ctx.harmony, ch).map_err(bad)?;
    Ok(calliope(&notes, dur, vel, seed, att, vib))
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

impl BigTopClassic {
    /// `Bed.bar(mix, b, intensity, light)`: one bar of the big top (bar index `b`): the bass drum on beats 1 and
    /// 3, the choked cymbal and a snare on 2 and 4, the calliope's off-beat chords; the back half (`intensity`
    /// above 1.25) adds the march snare's 8ths, tambourine 16ths, the held band-organ chord and, on every other
    /// bar, a glockenspiel glint. Python's defaults are `intensity = 1.0` and `light = False`; the kit's bed
    /// never asks for a light bar.
    ///
    /// # Errors
    /// Where Python raises: bar `b` is past the episode's last bar.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b)?;
        let big = intensity > 1.25 && !light;
        for bt in 0..4usize {
            let tb = t0 + bt as f64 * beat;
            if bt == 0 || bt == 2 {
                put(mix, "drums", bass_drum(1.0), tb, if light { 0.50 } else { 0.62 }, 0.0);
            } else {
                put(mix, "drums", choke(0.9, (10 * b + bt) as u64), tb, 0.16, 0.0);
                put(mix, "drums", snare(0.8), tb, if big { 0.16 } else { 0.10 }, 0.0);
            }
            let pan = 0.15 * if bt % 2 == 1 { 1.0 } else { -1.0 };
            let pipes = chunk(ctx, ch, 0.13, 1.0, (b * 4 + bt) as u64)?;
            put(mix, "calliope", pipes, tb + 0.5 * beat, 0.20, pan);
            if big {
                // The march snare's 8ths, accent on the beat (beats 2 and 4 have their snare already).
                for half in [0.0, 0.5] {
                    if !(half == 0.0 && (bt == 1 || bt == 3)) {
                        let vel = 0.55 + 0.25 * if half == 0.0 { 1.0 } else { 0.0 };
                        put(mix, "drums", snare(vel), tb + half * beat, 0.07, 0.0);
                    }
                }
                for q in 0..4usize {
                    let vel = 0.6 + 0.4 * if q == 2 { 1.0 } else { 0.0 };
                    let shake = tambourine(vel, (40 + q + bt) as u64);
                    put(mix, "drums", shake, tb + q as f64 * s16, 0.05, 0.3);
                }
            }
        }
        if big {
            let chord = held(ctx, ch, bar - 0.05, 0.8, (200 + b) as u64, 0.06, 0.006)?;
            put(mix, "calliope", chord, t0, 0.08, 0.0);
            if b.is_multiple_of(2) {
                // The glockenspiel glints round the key (E6 in E).
                let g = 84 + ctx.harmony.key_pc;
                let step = [0, 5, 7, 4][(b / 2) % 4];
                put(
                    mix,
                    "glock",
                    synth::chime(f64::from(g + step + 12), 0.8),
                    t0,
                    0.035,
                    0.0,
                );
            }
        } else if light {
            let chord = held(ctx, ch, bar - 0.05, 0.8, (300 + b) as u64, 0.08, 0.006)?;
            put(mix, "calliope", chord, t0, 0.07, 0.0);
        }
        Ok(())
    }
}

impl Bed for BigTopClassic {
    /// `buses = {'drums': (1.0, 0.15), 'calliope': (1.2, 0.45), 'glock': (1.0, 0.5)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("drums", (1.0, 0.15)), ("calliope", (1.2, 0.45)), ("glock", (1.0, 0.5))]
    }

    /// A big tent.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.0, 0.55, 0.22)
    }

    fn comp(&self) -> (f64, f64) {
        (0.15, 0.62)
    }

    fn roar_gain(&self) -> f64 {
        0.15
    }

    fn doors_gain(&self) -> f64 {
        0.40
    }

    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 100.0, 0.10)
    }

    /// A big drum hit (a slam landing).
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "drums", bass_drum(1.0), t, g, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.M.clank(79, int(t * 3)), t, 0.04)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let ring = clank(79.0, noise_seed(H, t * 3.0)?, CLANK_DUR, &CLANK_DECAYS)?;
        put(mix, "fx", ring, t, 0.04, 0.0);
        Ok(())
    }

    /// A unison band hit under each word of the VS card.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for (j, tw) in times.into_iter().enumerate() {
            put(mix, "drums", bass_drum(1.0), tw, 0.70, 0.0);
            put(mix, "drums", synth::crash(1.0), tw, 0.06, 0.0);
            put(
                mix,
                "calliope",
                chunk(ctx, ch, 0.30, 1.0, (700 + j) as u64)?,
                tw,
                0.22,
                0.0,
            );
            // A.thud(0.8, 120, 34, seed=int(tw_ * 50) + 3)
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.30, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let nn = (1.4 * SRF) as usize;
        let tt = synth::tt(nn);
        let mut draw = vec![0.0; nn];
        Generator::new(91).fill_normal(0.0, 1.0, &mut draw);
        let crack = synth::bp(&draw, 1500.0, 9000.0, FILTER_O);
        Generator::new(92).fill_normal(0.0, 1.0, &mut draw);
        let rumble = synth::lp(&draw, 160.0, FILTER_O);
        let x = (0..nn)
            .map(|i| {
                let crack = crack[i] * dmath::exp(-tt[i] / 0.035);
                let rumble = rumble[i] * (tt[i] / 0.02).clamp(0.0, 1.0) * dmath::exp(-tt[i] / 0.5) * 3.0;
                crack * 0.9 + rumble
            })
            .collect();
        put(mix, "fx", x, t, 0.36, 0.0);
        Ok(())
    }

    /// The held organ under the card (never quiet), a snare roll on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let beat = grid.beat;
        let organ = held(ctx, chord(ctx, 0)?, 3.0 * beat, 0.8, 710, 0.03, CALLIOPE_VIB)?;
        put(mix, "calliope", organ, 0.0, 0.08, 0.0);
        put(
            mix,
            "drums",
            roll(beat * 0.95, 0.3, 0.9, grid.s16 / 2.0),
            3.0 * beat,
            0.20,
            0.0,
        );
        Ok(())
    }

    /// Bar 1: the count-in on the second chord (the dominant), a pickup and a roll into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let ch = chord(ctx, 1)?;
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            put(mix, "drums", bass_drum(1.0), *tc, 0.62, 0.0);
            put(mix, "drums", choke(1.0, (50 + j) as u64), *tc, 0.14, 0.0);
            put(
                mix,
                "calliope",
                chunk(ctx, ch, 0.18, 1.0, (720 + j) as u64)?,
                *tc,
                0.20,
                0.0,
            );
        }
        // The organ swells under the count: times np.linspace(0.6, 1.0, int((T.BAR + 0.06) * SR)).
        let organ = held(ctx, ch, bar, 0.7, 730, 0.2, CALLIOPE_VIB)?;
        let swell = dsp::linspace(0.6, 1.0, ((bar + 0.06) * SRF) as usize);
        debug_assert_eq!(organ.len(), swell.len());
        let organ = organ.iter().zip(&swell).map(|(v, s)| v * s).collect();
        put(mix, "calliope", organ, bar, 0.08, 0.0);
        // The calliope chunks the off-beats of the count too.
        for bt in 0..4usize {
            let pipes = chunk(ctx, ch, 0.13, 1.0, (740 + bt) as u64)?;
            put(mix, "calliope", pipes, bar + (bt as f64 + 0.5) * beat, 0.20, 0.0);
        }
        // Beat 4: the pickup, a band hit, and the roll into FIGHT!
        let t4 = bar + 3.0 * beat;
        put(mix, "drums", bass_drum(1.0), t4, 0.55, 0.0);
        put(mix, "calliope", chunk(ctx, ch, 0.18, 1.0, 745)?, t4, 0.18, 0.0);
        put(mix, "drums", roll(beat, 0.5, 1.0, grid.s16 / 2.0), t4, 0.30, 0.0);
        Ok(())
    }

    /// The band comes in.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The big top from FIGHT! to the K.O. (denser as it goes), a roll and a riser into the back half, a tape
    /// stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        // Every 8-bar phrase: a crash on its first beat.
        for b in (2..=bar_ko).step_by(8) {
            put(mix, "drums", synth::crash(1.6), b as f64 * bar, 0.08, 0.0);
        }
        // The back half (bar 18 in #41).
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        put(
            mix,
            "drums",
            roll(2.0 * beat, 0.3, 1.0, grid.s16 / 2.0),
            t_half - 2.0 * beat,
            0.20,
            0.0,
        );
        let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
        put(mix, "fx", rise, t_half - bar, 0.08, 0.0);
        put(mix, "drums", synth::crash(2.0), t_half, 0.12, 0.0);
        for bus in ["drums", "calliope", "glock"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "drums", synth::crash(2.4), t_ko, 0.16, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        put(
            mix,
            "calliope",
            chunk(ctx, chord(ctx, 0)?, 0.35, 1.0, 800)?,
            tw,
            0.22,
            0.0,
        );
        put(mix, "drums", bass_drum(1.0), tw, 0.65, 0.0);
        put(mix, "drums", synth::crash(1.8), tw, 0.10, 0.0);
        Ok(())
    }

    /// `A.whoosh(max(0.2, t_solo - tw - 0.1), 300, 5000)` from just after the stinger to the solo.
    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.08,
            0.0,
        );
        Ok(())
    }

    /// The band plays on under the solo (no march snare), a held organ chord, up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let grid = &ctx.show.grid;
        for b in b_solo..grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.1, false)?;
            let organ = held(ctx, chord(ctx, b)?, grid.bar - 0.05, 0.8, (300 + b) as u64, 0.08, 0.006)?;
            put(mix, "calliope", organ, b as f64 * grid.bar, 0.07, 0.0);
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a snare roll, the bass drum on every beat, the
    /// calliope off-beats, a riser and an organ swell right up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = last_chord(ctx)?;
        put(
            mix,
            "drums",
            roll(dur - t_last + 0.01, 0.4, 1.0, grid.s16 / 2.0),
            t_last,
            0.20,
            0.0,
        );
        for bt in 0..4usize {
            put(mix, "drums", bass_drum(0.95), t_last + bt as f64 * beat, 0.56, 0.0);
            let pipes = chunk(ctx, ch, 0.13, 1.0, (900 + bt) as u64)?;
            put(mix, "calliope", pipes, t_last + (bt as f64 + 0.5) * beat, 0.20, 0.0);
        }
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.18,
            0.0,
        );
        let swell = held(ctx, ch, dur - t_last + 0.012, 0.9, 950, dur - t_last, 0.008)?;
        put(mix, "calliope", swell, t_last, 0.10, 0.0);
        Ok(())
    }
}
