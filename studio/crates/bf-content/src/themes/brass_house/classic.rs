//! The brass house's classic arrangement: the bed of #36's `audio.py` (`bed()` and the bed's lines of `main()`) on
//! the kit's beat sheet (the VS card on bar 0, the count-in on bar 1, FIGHT! on bar 2), section by section as
//! `kit.music.score()` calls them. Quick-ported by ear: the same instruments, patterns and levels, not the same
//! samples. The bed has no bass and no brass: #36's TROMBONE and TUBA blasts on the card, the count-in and the last
//! bar's climb are the fighters' own parts here, so the bed sounds the same under any pair.

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, noise_seed, riser};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{hat, kick, pad_bar, pad_voicing, ride, shaker, stab, stab_voicing};

/// How much louder the bars under the solo play (9.5 dB): #36's master lifted its victory about 5 dB over the fight
/// (`GAIN_PTS`), the kit's has no such lift, so the bed does it.
const VICTORY_LIFT: f64 = 3.0;

/// The name the bed's errors carry.
const H: &str = "brass_house";

/// The bed's own buses: `BUS_GAIN` and the reverb sends of #36's `master()` (its `drums` bus is `kit` here, so a
/// DRUMS fighter keeps its own).
const BUSES: [(&str, (f64, f64)); 3] = [("kit", (1.0, 0.12)), ("pad", (1.6, 0.4)), ("stabs", (1.2, 0.5))];

/// The syncopated 16ths of the stabs (the "and"s around beats 2 and 4).
const STAB_SLOTS: [usize; 4] = [3, 6, 10, 14];

/// The brass house's bed.
#[derive(Clone, Debug, Default)]
pub struct BrassHouseClassic;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
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

/// The VS crack: band-passed noise with a low rumble under it (seeds 91 and 92, as #36).
fn crack() -> Vec<f64> {
    let nn = (1.4 * SRF) as usize;
    let tt = synth::tt(nn);
    let mut draw = vec![0.0; nn];
    Generator::new(91).fill_normal(0.0, 1.0, &mut draw);
    let crack = synth::bp(&draw, 1500.0, 9000.0, FILTER_O);
    Generator::new(92).fill_normal(0.0, 1.0, &mut draw);
    let rumble = synth::lp(&draw, 160.0, FILTER_O);
    (0..nn)
        .map(|i| {
            let crack = crack[i] * dmath::exp(-tt[i] / 0.035);
            let rumble = rumble[i] * (tt[i] / 0.02).clamp(0.0, 1.0) * dmath::exp(-tt[i] / 0.5) * 3.0;
            crack * 0.9 + rumble
        })
        .collect()
}

fn clap(v: f64) -> Vec<f64> {
    synth::clap().iter().map(|s| s * v).collect()
}

impl BrassHouseClassic {
    /// The bed.
    pub fn new() -> BrassHouseClassic {
        BrassHouseClassic
    }

    /// `bed(mix, b, intensity, light)`: one bar of the groove: kick, claps, hats (16ths, open hats and a ride in
    /// the back half), the pumping pad, the chord stabs.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool, lift: f64) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b)?;
        let big = intensity > 1.3 && !light;
        for bt in 0..4u64 {
            let tb = t0 + bt as f64 * beat;
            put(mix, "kit", kick(1.0), tb, if light { 0.38 } else { 0.44 }, 0.0);
            put(mix, "kit", hat(0.9, false, 10 + bt), tb + beat / 2.0, 0.13, 0.0);
            if bt % 2 == 1 {
                put(mix, "kit", synth::clap(), tb, 0.30, 0.0);
            }
            if big {
                put(mix, "kit", hat(0.9, true, 20 + bt), tb + beat / 2.0, 0.07, 0.0);
                put(mix, "kit", ride(0.8, 30 + bt), tb, 0.06, 0.0);
            }
        }
        if intensity > 1.15 || light {
            for st in (1..16u64).step_by(2) {
                let v = 0.6 + if st % 4 == 3 { 0.4 } else { 0.0 };
                put(
                    mix,
                    "kit",
                    shaker(v, 40 + st),
                    t0 + st as f64 * s16,
                    if light { 0.04 } else { 0.06 },
                    0.0,
                );
            }
        }
        let cut = 900.0 + 1500.0 * ((intensity - 1.0) / 0.6).clamp(0.0, 1.0) + if big { 900.0 } else { 0.0 };
        let pv = pad_voicing(&ctx.harmony, ch).map_err(bad)?;
        let pad = pad_bar(&pv, t0, beat, bar + 0.05, if light { 1300.0 } else { cut }, 0.06);
        put(mix, "pad", pad, t0, 0.11, 0.0);
        if !light {
            let sv = stab_voicing(&ctx.harmony, ch).map_err(bad)?;
            let slots: Vec<usize> = if big {
                STAB_SLOTS.to_vec()
            } else {
                STAB_SLOTS.iter().skip(1).step_by(2).copied().collect()
            };
            for st in slots {
                let pan = 0.25 * if st % 8 > 4 { 1.0 } else { -1.0 };
                let x = stab(&sv, 0.10, if big { 2200.0 } else { 1500.0 });
                put(mix, "stabs", x, t0 + st as f64 * s16, 0.07, pan);
            }
        }
        Ok(())
    }
}

impl Bed for BrassHouseClassic {
    /// A slam lands on the kick, the thud and a crash (#36's PEDAL TONE and SUBSONIC stomps).
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// `A.reverb(send) * 0.20` (the default room).
    fn reverb(&self) -> (f64, f64, f64) {
        (synth::REVERB_DUR, synth::REVERB_DECAY, 0.20)
    }

    fn comp(&self) -> (f64, f64) {
        (0.22, 0.35)
    }

    fn roar_gain(&self) -> f64 {
        0.16
    }

    fn doors_gain(&self) -> f64 {
        0.45
    }

    fn landing(&self) -> (f64, f64) {
        (0.08, 0.06)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 102.0, 0.12)
    }

    /// A big drum hit: the kick, the thud and a crash.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "kit", kick(1.0), t, g * 0.5, 0.0);
        put(mix, "kit", synth::crash(1.6), t, g * 0.10, 0.0);
        put(mix, "fx", synth::thud(0.8, 90.0, 30.0, 7), t, g * 0.35, 0.0);
        Ok(())
    }

    /// `A.chime(91, 0.25)` at 0.05.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// One unison house hit per word: the kick, a bright stab on the tonic, a thud and a little crash.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let sv = stab_voicing(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        for tw in times {
            put(mix, "kit", kick(1.0), tw, 0.70, 0.0);
            put(mix, "stabs", stab(&sv, 0.22, 3200.0), tw, 0.14, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.9, 120.0, 32.0, seed), tw, 0.40, 0.0);
            put(mix, "kit", synth::crash(1.2), tw, 0.06, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", crack(), t, 0.40, 0.0);
        Ok(())
    }

    /// The pad holds the tonic under the card (never quiet), a clap pickup on beat 4 and a whoosh as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let pv: Vec<f64> = pad_voicing(&ctx.harmony, chord(ctx, 0)?)
            .map_err(bad)?
            .into_iter()
            .map(f64::from)
            .collect();
        put(
            mix,
            "pad",
            synth::pad(&pv, 3.0 * grid.beat + 0.1, 1100.0, 0.03),
            0.0,
            0.10,
            0.0,
        );
        for (st, v) in [(12.0, 0.5), (13.0, 0.65), (14.0, 0.8), (15.0, 0.95)] {
            put(mix, "kit", clap(v), st * grid.s16, 0.22, 0.0);
        }
        let (d0, d1) = grid.t_doors;
        let whoosh = synth::whoosh(d1 - d0 + 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", whoosh, py_max(0.0, d0 - 0.05), 0.16, 0.0);
        Ok(())
    }

    /// Bar 1, on the dominant: a kick and a stab on each count, the pad swelling, a clap pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let ch = chord(ctx, 1)?;
        let sv = stab_voicing(&ctx.harmony, ch).map_err(bad)?;
        for (tc, _) in &grid.count[..3] {
            put(mix, "kit", kick(1.0), *tc, 0.60, 0.0);
            put(mix, "stabs", stab(&sv, 0.16, 2800.0), *tc, 0.11, 0.0);
        }
        let pv: Vec<f64> = pad_voicing(&ctx.harmony, ch)
            .map_err(bad)?
            .into_iter()
            .map(f64::from)
            .collect();
        let pad = synth::pad(&pv, bar, 900.0, 0.4);
        let n = pad.len().max(2);
        let swell = pad
            .iter()
            .enumerate()
            .map(|(i, v)| v * (0.4 + 0.6 * i as f64 / (n - 1) as f64))
            .collect();
        put(mix, "pad", swell, bar, 0.10, 0.0);
        for (st, v) in [(12.0, 0.5), (13.0, 0.62), (14.0, 0.76), (15.0, 0.9)] {
            put(mix, "kit", clap(v), bar + st * s16, 0.28, 0.0);
        }
        let whoosh = synth::whoosh(beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", whoosh, bar + 3.0 * beat, 0.14, 0.0);
        Ok(())
    }

    /// FIGHT!: the drop, a crash and a thud.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "kit", synth::crash(1.8), t, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, 7), t, 0.40, 0.0);
        Ok(())
    }

    /// The groove from FIGHT! to the K.O. (`intensity = 1 + 0.6 * b / bar_ko`), a filter-sweep riser into the
    /// back half (#36's bar 17 of 32) and a whoosh later, a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        for b in grid.fight_bar..=bar_ko {
            let u = b as f64 / bar_ko.max(1) as f64;
            // #36's master rose about 3 dB through the back half (`GAIN_PTS`); the kit's does not, so the bed does.
            let lift = 1.0 + 0.4 * ((u - 0.5) / 0.5).clamp(0.0, 1.0);
            self.bar(ctx, mix, b, 1.0 + 0.6 * u, false, lift)?;
        }
        let half = (grid.n_bars as f64 * 17.0 / 32.0).round() * grid.bar;
        if half < ctx.show.t_ko {
            let r: Vec<f64> = riser(2.0 * grid.bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
            put(mix, "fx", r, py_max(0.0, half - 2.0 * grid.bar), 0.10, 0.0);
            put(mix, "kit", synth::crash(2.0), half, 0.10, 0.0);
        }
        let late = grid.dur * 44.0 / 61.0;
        if late < ctx.show.t_ko {
            let whoosh = synth::whoosh(3.6, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", whoosh, py_max(0.0, late - 3.6), 0.10, 0.0);
        }
        for (bus, _) in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a crash and the thud.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.t_ko;
        put(mix, "kit", synth::crash(2.4), t, 0.16, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, 7), t, 0.7, 0.0);
        Ok(())
    }

    /// A bright tonic stab, a kick and a crash under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let sv = stab_voicing(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        put(mix, "stabs", stab(&sv, 0.3, 3200.0), tw, 0.12, 0.0);
        put(mix, "kit", kick(1.0), tw, 0.55, 0.0);
        put(mix, "kit", synth::crash(1.8), tw, 0.10, 0.0);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo.
    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.3, t_solo - tw - 0.1);
        let whoosh = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", whoosh, tw + 0.1, 0.10, 0.0);
        Ok(())
    }

    /// The lighter groove under the solo, up to the build (#36 lifted it in its master; the kit's has no lift, so
    /// the bed plays [`VICTORY_LIFT`] louder).
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let n = ctx.show.grid.n_bars;
        for b in b_solo..n.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.0, true, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a clap roll (8ths, then 16ths for the last beat), the
    /// kick on every beat, a riser and the pad swelling right up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur, s16) = (grid.beat, grid.bar, grid.dur, grid.s16);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let rest = dur - t_last;
        let mut st = 0.0;
        while t_last + st < dur - 1e-4 {
            let v = 0.45 + 0.55 * st / rest;
            put(mix, "kit", clap(v), t_last + st, 0.26, 0.0);
            st += if st < 3.0 * beat - 1e-6 { beat / 2.0 } else { s16 };
        }
        for bt in 0..4 {
            put(mix, "kit", kick(0.9), t_last + f64::from(bt) * beat, 0.46, 0.0);
        }
        put(mix, "fx", riser(rest, RISER_OVER, RISER_SEED)?, t_last, 0.22, 0.0);
        let ch = chord(ctx, grid.n_bars.saturating_sub(1))?;
        let pv: Vec<f64> = pad_voicing(&ctx.harmony, ch)
            .map_err(bad)?
            .into_iter()
            .map(f64::from)
            .collect();
        let sw = synth::pad(&pv, rest + 0.012, 2400.0, rest);
        let n = sw.len().max(2);
        let sw = sw
            .iter()
            .enumerate()
            .map(|(i, v)| v * (0.3 + 0.7 * i as f64 / (n - 1) as f64))
            .collect();
        put(mix, "pad", sw, t_last, 0.12, 0.0);
        Ok(())
    }
}
