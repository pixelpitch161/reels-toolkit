//! The sea shanty's classic arrangement: the bed of #30 (`projects/ballfight_bagpipes_vs_accordion/audio.py`),
//! section by section as the score calls them, by ear (the fighters' own lines of that episode are their parts').

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, noise_seed, riser};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED, noise};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{Vowel, bass, bass_voice, choir, choir_voicing, frame_drum, gang_clap_hands, stomp, walk};

/// The name the bed's errors carry.
const H: &str = "sea_shanty";

/// How much louder the bars under the solo are: #30 lifted its master there by about 4 dB (its `GAIN_PTS`), which
/// the kit's master has no means for, so the bed lifts itself.
const VICTORY_LIFT: f64 = 1.6;

/// The sea shanty's bed. The sounds that are the same every time (the gang clap and the two frame drum strokes)
/// are made once, when the bed is made, and scaled per stroke.
#[derive(Clone, Debug)]
pub struct SeaShantyClassic {
    clap: Vec<f64>,
    drum_low: Vec<f64>,
    drum_rim: Vec<f64>,
}

impl Default for SeaShantyClassic {
    fn default() -> SeaShantyClassic {
        SeaShantyClassic::new()
    }
}

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

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

fn scaled(x: &[f64], vel: f64) -> Vec<f64> {
    x.iter().map(|v| v * vel).collect()
}

/// The choir on `ch`'s spread chord.
fn choir_on(ctx: &MusicCtx, ch: &str, dur: f64, att: f64, rel: f64, vowel: Vowel) -> Result<Vec<f64>, MusicError> {
    Ok(choir(
        &choir_voicing(&ctx.harmony, ch).map_err(bad)?,
        dur,
        att,
        rel,
        vowel,
    ))
}

impl SeaShantyClassic {
    /// The bed, with its every-beat sounds made.
    pub fn new() -> SeaShantyClassic {
        SeaShantyClassic {
            clap: gang_clap_hands(),
            drum_low: frame_drum(true),
            drum_rim: frame_drum(false),
        }
    }

    fn gang_clap(&self, vel: f64) -> Vec<f64> {
        scaled(&self.clap, vel)
    }

    /// `bed(mix, b, intensity)`: one bar: stomps on 1 and 3 with the bass singer, gang claps on 2 and 4, the
    /// choir's hum (an "oh" once the fight is hot), a stomp pickup on the "and" of 4, the frame drum's 8ths in the
    /// back half.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, lift: f64) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let t0 = b as f64 * grid.bar;
        let ch = chord(ctx, b)?;
        let (r, f5) = bass(&ctx.harmony, ch).map_err(bad)?;
        for bt in [0usize, 2] {
            let tb = t0 + bt as f64 * beat;
            let vel = if bt == 0 { 1.0 } else { 0.85 };
            put(mix, "stomp", stomp(vel, (b * 4 + bt) as u64), tb, 0.50, 0.0);
            let m = if bt == 0 { r } else { f5 };
            put(mix, "choir", bass_voice(m, 0.34, 1.0), tb, 0.22, 0.0);
        }
        for bt in [1usize, 3] {
            let vel = if bt == 1 { 0.9 } else { 1.0 };
            let pan = 0.15 * if bt == 1 { 1.0 } else { -1.0 };
            let g = 0.34;
            put(mix, "stomp", self.gang_clap(vel), t0 + bt as f64 * beat, g, pan);
        }
        if intensity > 1.15 {
            put(mix, "stomp", stomp(0.6, (b * 4 + 7) as u64), t0 + 3.5 * beat, 0.26, 0.0);
        }
        let vowel = if intensity < 1.3 { Vowel::M } else { Vowel::O };
        let hum = choir_on(ctx, ch, grid.bar, 0.3, 0.35, vowel)?;
        put(mix, "choir", hum, t0, 0.10, 0.0);
        if intensity > 1.25 {
            let k = ((intensity - 1.25) / 0.25).min(1.0);
            for st in (0..16usize).step_by(2) {
                let low = st % 4 == 0;
                let vel = 0.8 + 0.2 * if st % 8 == 0 { 1.0 } else { 0.0 };
                let x = scaled(if low { &self.drum_low } else { &self.drum_rim }, vel);
                put(
                    mix,
                    "drum",
                    x,
                    t0 + st as f64 * s16,
                    (if low { 0.20 } else { 0.10 }) * k,
                    0.0,
                );
            }
        }
        Ok(())
    }
}

impl Bed for SeaShantyClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `BUS_GAIN` and the reverb sends of #30's master for the bed's own buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("stomp", (1.1, 0.25)), ("choir", (2.2, 0.6)), ("drum", (1.3, 0.0))]
    }

    /// A wooden hold below deck.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.4, 0.6, 0.22)
    }

    fn comp(&self) -> (f64, f64) {
        (0.22, 0.35)
    }

    fn roar_gain(&self) -> f64 {
        0.16
    }

    fn doors_gain(&self) -> f64 {
        0.55
    }

    fn landing(&self) -> (f64, f64) {
        (0.08, 0.06)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 102.0, 0.12)
    }

    /// A body slam: a boot on the deck and the frame drum's skin.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "stomp", stomp(1.0, noise_seed(H, t * 3.0)?), t, g, 0.0);
        put(mix, "drum", self.drum_low.clone(), t, 0.5 * g, 0.0);
        Ok(())
    }

    /// A crew's "hey!" on the chord: the choir's open vowel over a stomp.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let hey = choir_on(ctx, ch, py_max(ring, 0.18), 0.01, 0.2, Vowel::O)?;
        put(mix, "choir", hey, t, 0.20 * g, 0.0);
        put(mix, "stomp", stomp(1.0, noise_seed(H, t * 7.0)?), t, 0.45 * g, 0.0);
        Ok(())
    }

    /// #30 lets the hits score the contact: a clash plays nothing of the bed's.
    fn clash(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
        Ok(())
    }

    /// The band on every word: a stomp, a boom, a choir "hey" and the bass singer under it.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let r = ctx.harmony.root_above(chord(ctx, 0)?, 40).map_err(bad)?;
        for tw in times {
            put(mix, "stomp", stomp(1.0, noise_seed(H, tw * 100.0)? + 7), tw, 0.58, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.9, 120.0, 32.0, seed), tw, 0.55, 0.0);
            let hey = choir(&[r, r + 7, r + 12, r + 19], 0.30, 0.008, 0.25, Vowel::O);
            put(mix, "choir", hey, tw, 0.20, 0.0);
            put(mix, "choir", bass_voice(r - 12, 0.5, 1.0), tw, 0.30, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider, and a crash.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let nn = (1.4 * SRF) as usize;
        let tt = synth::tt(nn);
        let crack = synth::bp(&noise(91, nn), 1500.0, 9000.0, FILTER_O);
        let rumble = synth::lp(&noise(92, nn), 160.0, FILTER_O);
        let x = (0..nn)
            .map(|i| {
                let crack = crack[i] * dmath::exp(-tt[i] / 0.035);
                let rumble = rumble[i] * (tt[i] / 0.02).clamp(0.0, 1.0) * dmath::exp(-tt[i] / 0.5) * 3.0;
                crack * 0.9 + rumble
            })
            .collect();
        put(mix, "fx", x, t, 0.42, 0.0);
        put(mix, "fx", synth::crash(1.6), t, 0.10, 0.0);
        Ok(())
    }

    /// The choir's hum under the whole card, a stomp pickup as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let hum = choir_on(ctx, chord(ctx, 0)?, 3.0 * grid.beat, 0.05, 0.3, Vowel::M)?;
        put(mix, "choir", hum, 0.0, 0.10, 0.0);
        for st in 12..16usize {
            let vel = 0.5 + 0.12 * (st - 12) as f64;
            put(
                mix,
                "stomp",
                stomp(vel, (70 + st) as u64),
                st as f64 * grid.s16,
                0.40,
                0.0,
            );
        }
        Ok(())
    }

    /// Bar 1 on the second chord: a stomp, a gang clap and the bass singer per count, the choir's "oh", a stomp
    /// run into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ch = chord(ctx, 1)?;
        let (r, _) = bass(&ctx.harmony, ch).map_err(bad)?;
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            put(mix, "stomp", stomp(1.0, (90 + j) as u64), *tc, 0.52, 0.0);
            put(mix, "stomp", self.gang_clap(1.0), *tc, 0.30, 0.0);
            put(mix, "choir", bass_voice(r, 0.36, 1.0), *tc, 0.24, 0.0);
        }
        let oh = choir_on(ctx, ch, grid.bar, 0.2, 0.05, Vowel::O)?;
        put(mix, "choir", oh, grid.bar, 0.10, 0.0);
        for (st, v) in [(12usize, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)] {
            put(
                mix,
                "stomp",
                stomp(v, (80 + st) as u64),
                grid.bar + st as f64 * grid.s16,
                0.40,
                0.0,
            );
        }
        Ok(())
    }

    /// FIGHT!: a thud and the choir's "oh" an octave up on the loop's first chord.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "fx", synth::thud(0.8, 110.0, 34.0, synth::THUD_SEED), t, 0.55, 0.0);
        let first = chord(ctx, ctx.harmony.loop_start)?;
        let up: Vec<i32> = choir_voicing(&ctx.harmony, first)
            .map_err(bad)?
            .iter()
            .map(|m| m + 12)
            .collect();
        put(mix, "choir", choir(&up, 0.5, 0.01, 0.25, Vowel::O), t, 0.22, 0.0);
        Ok(())
    }

    /// The shanty from FIGHT! to the K.O., hotter as it goes, tape-stopped at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let b0 = bf_core::py::round_half_even(grid.t_fight / grid.bar) as usize;
        let span = (bar_ko + 1).saturating_sub(b0).max(1) as f64;
        for b in b0..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * ((b - b0 + 1) as f64 / span), 1.0)?;
        }
        for bus in ["stomp", "choir", "drum"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(
            mix,
            "fx",
            synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED),
            t_ko,
            0.7,
            0.0,
        );
        put(mix, "stomp", stomp(1.0, 61), t_ko, 0.6, 0.0);
        Ok(())
    }

    /// The choir's "oh" on the home chord under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let oh = choir_on(ctx, chord(ctx, 0)?, 1.4, 0.05, 0.6, Vowel::O)?;
        put(mix, "choir", oh, tw, 0.18, 0.0);
        Ok(())
    }

    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        let x = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", x, tw + 0.1, 0.10, 0.0);
        Ok(())
    }

    /// The shanty under the solo, up to the build: #30 lightened it and lifted the master there, so here the bars
    /// keep their full level and are lifted by [`VICTORY_LIFT`] (no pickup and no frame drum).
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.0, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar: the stomps double up (8ths, then 16ths), claps on 2 and 4, a riser, the choir swelling on the
    /// last chord and the bass singer walking up its arpeggio into the title slam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, dur) = (grid.beat, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        for st in 0..16usize {
            if st < 8 && st % 2 == 1 {
                continue;
            }
            let vel = 0.55 + 0.45 * st as f64 / 15.0;
            put(
                mix,
                "stomp",
                stomp(vel, (70 + st) as u64),
                t_last + st as f64 * grid.s16,
                0.42,
                0.0,
            );
        }
        for bt in [1usize, 3] {
            put(mix, "stomp", self.gang_clap(1.0), t_last + bt as f64 * beat, 0.30, 0.0);
        }
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.22,
            0.0,
        );
        let last = chord(ctx, grid.n_bars.saturating_sub(1))?;
        let swell = choir_on(ctx, last, dur - t_last + 0.02, 0.9, 0.001, Vowel::O)?;
        put(mix, "choir", swell, t_last, 0.13, 0.0);
        let steps = walk(&ctx.harmony, last).map_err(bad)?;
        for (bt, m) in steps.into_iter().enumerate() {
            put(
                mix,
                "choir",
                bass_voice(m, 0.32, 1.0),
                t_last + bt as f64 * beat,
                0.22,
                0.0,
            );
        }
        Ok(())
    }
}
