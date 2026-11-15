//! The drift phonk's classic arrangement: the bed of #27 (`projects/ballfight/audio.py`), section by section as the
//! score calls them, by ear (the episode's fighters' own lines are not part of the bed and are not here).

use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, noise_seed, riser};
use bf_sound::synth::{self, HAT_DEC, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{b808, bass_root, cowbell, kick, pad_voicing, riff, roll, shimmer};

/// The name the bed's errors carry.
const H: &str = "drift_phonk";

/// How much louder the bars under the solo are: #27 lifted its master there by about 4 dB over the fight (its
/// `GAIN_PTS`), which the kit's master has no means for, so the bed lifts its lighter groove itself.
const VICTORY_LIFT: f64 = 1.5;

/// The drift phonk's bed. The sounds that are the same every time (the kick, the clap, the snare and the hats)
/// are made once, when the bed is made, and scaled per stroke.
#[derive(Clone, Debug)]
pub struct DriftPhonkClassic {
    kick: Vec<f64>,
    clap: Vec<f64>,
    snare: Vec<f64>,
    hat: Vec<f64>,
    open_hat: Vec<f64>,
}

impl Default for DriftPhonkClassic {
    fn default() -> DriftPhonkClassic {
        DriftPhonkClassic::new()
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

impl DriftPhonkClassic {
    /// The bed, with its every-beat sounds made.
    pub fn new() -> DriftPhonkClassic {
        DriftPhonkClassic {
            kick: kick(1.0),
            clap: synth::clap(),
            snare: synth::snare(0.09),
            hat: synth::hat(0.025, false),
            open_hat: synth::hat(HAT_DEC, true),
        }
    }

    /// `bed(mix, b, light, intensity)`: one bar: kicks on 1, the "and" of 2, 3 and a late pickup, claps on 2 and
    /// 4, 16th hats (open offbeats once the fight is hot), the 808 holding the root and sliding into the next
    /// bar's, the cowbell riff (an octave layer up late in the fight), the dark pad once the fight is under way.
    #[allow(clippy::too_many_arguments)]
    fn bar(
        &self,
        ctx: &MusicCtx,
        mix: &mut Mix,
        b: usize,
        light: bool,
        intensity: f64,
        pad: bool,
        lift: f64,
    ) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let s16 = grid.s16;
        let t0 = b as f64 * grid.bar;
        let ch = chord(ctx, b)?;
        let nxt = chord(ctx, (b + 1) % grid.n_bars.max(1))?;
        for st in [0usize, 6, 8, if b % 2 == 1 { 11 } else { 14 }] {
            let g = if st == 0 || st == 8 { 0.62 } else { 0.42 };
            put(mix, "drums", self.kick.clone(), t0 + st as f64 * s16, g, 0.0);
        }
        for st in [4usize, 12] {
            put(mix, "drums", self.clap.clone(), t0 + st as f64 * s16, 0.42, 0.0);
            put(mix, "drums", self.snare.clone(), t0 + st as f64 * s16, 0.16, 0.0);
        }
        for st in 0..16usize {
            let g = if st % 2 == 1 { 0.07 } else { 0.10 };
            if !light && intensity > 1.2 && st % 4 == 2 {
                let x = self.open_hat.clone();
                put(mix, "drums", x, t0 + st as f64 * s16, 0.05 * intensity, 0.0);
            }
            let pan = if st % 2 == 1 { 0.25 } else { -0.15 };
            let g = g * if light { 0.7 } else { 1.0 };
            put(mix, "drums", self.hat.clone(), t0 + st as f64 * s16, g, pan);
        }
        let (r0, r1) = (
            bass_root(&ctx.harmony, ch).map_err(bad)?,
            bass_root(&ctx.harmony, nxt).map_err(bad)?,
        );
        let up = if b % 4 == 3 { r0 + 12 } else { r0 };
        put(mix, "bass", b808(r0, up, 6.0 * s16, 0.08), t0, 0.55, 0.0);
        put(mix, "bass", b808(r0, r1, 10.0 * s16, 0.12), t0 + 6.0 * s16, 0.48, 0.0);
        for (st, m) in riff(&ctx.harmony, b, ch).map_err(bad)?.into_iter().enumerate() {
            let Some(m) = m else { continue };
            let t = t0 + st as f64 * s16;
            put(
                mix,
                "cowbell",
                cowbell(m, 0.16),
                t,
                if light { 0.10 } else { 0.15 },
                0.15,
            );
            if intensity > 1.35 && !light {
                put(mix, "cowbell", cowbell(m + 12, 0.1), t, 0.05, -0.3);
            }
        }
        if pad || light {
            let tones = pad_voicing(&ctx.harmony, ch).map_err(bad)?.map(f64::from);
            put(mix, "pad", synth::pad(&tones, grid.bar, 900.0, 0.4), t0, 0.07, 0.0);
        }
        Ok(())
    }
}

impl Bed for DriftPhonkClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `BUS_GAIN` and the reverb sends of #27's master for the bed's own buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("drums", (0.75, 0.0)),
            ("bass", (0.62, 0.0)),
            ("cowbell", (0.72, 0.3)),
            ("pad", (0.8, 0.5)),
        ]
    }

    /// A car park at night.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.4, 0.6, 0.20)
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

    /// A slam: the hard kick and an 808 that drops an octave.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let r = bass_root(&ctx.harmony, ctx.harmony.chord_at(t)).map_err(bad)?;
        put(mix, "drums", kick(1.2), t, 0.9 * g, 0.0);
        put(mix, "bass", b808(r + 12, r, 0.5, 0.3), t, 0.6 * g, 0.0);
        let seed = noise_seed(H, t * 3.0)?;
        put(mix, "fx", synth::thud(0.6, 90.0, 32.0, seed), t, 0.5 * g, 0.0);
        Ok(())
    }

    /// A band hit on the chord: a clap, the kick, the 808 on the root and the cowbell on the chord's root and
    /// fifth.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        put(mix, "drums", self.clap.clone(), t, 0.42 * g, 0.0);
        put(mix, "drums", self.kick.clone(), t, 0.5 * g, 0.0);
        put(mix, "bass", b808(r, r, py_max(ring, 0.25), 0.05), t, 0.45 * g, 0.0);
        let top = ctx.harmony.root_above(ch, 76).map_err(bad)?;
        for m in [top, top + 7] {
            put(mix, "cowbell", cowbell(m, 0.16), t, 0.12 * g, 0.15);
        }
        Ok(())
    }

    /// #27's clash: a small chime.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.06, 0.0);
        Ok(())
    }

    /// Every word: the kick, the 808 on the home root and a cowbell stab.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        let top = ctx.harmony.root_above(ch, 76).map_err(bad)?;
        for tw in times {
            put(mix, "drums", kick(1.1), tw, 0.6, 0.0);
            put(mix, "bass", b808(r, r, 0.42, 0.05), tw, 0.6, 0.0);
            put(mix, "cowbell", cowbell(top, 0.2), tw, 0.14, 0.15);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.5, 120.0, 40.0, seed), tw, 0.3, 0.0);
        }
        Ok(())
    }

    /// VS: a thud and a crash.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED), t, 0.5, 0.0);
        put(mix, "fx", synth::crash(1.6), t, 0.14, 0.0);
        Ok(())
    }

    /// The dark pad under the card, a snare pickup as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let tones = pad_voicing(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?.map(f64::from);
        put(mix, "pad", synth::pad(&tones, grid.bar, 900.0, 0.4), 0.0, 0.07, 0.0);
        put(
            mix,
            "drums",
            roll(grid.beat, 0.2, grid.s16 / 2.0),
            3.0 * grid.beat,
            0.18,
            0.0,
        );
        Ok(())
    }

    /// Bar 1 on the second chord: the kick and the 808 per count, a cowbell on the root, a snare roll and a
    /// whoosh into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ch = chord(ctx, 1)?;
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        let top = ctx.harmony.root_above(ch, 76).map_err(bad)?;
        for (tc, _) in &grid.count[..3] {
            put(mix, "drums", kick(1.1), *tc, 0.6, 0.0);
            put(mix, "bass", b808(r, r, 0.42, 0.05), *tc, 0.6, 0.0);
            put(mix, "cowbell", cowbell(top, 0.16), *tc, 0.14, 0.15);
        }
        let t3 = grid.bar + 3.0 * grid.beat;
        put(mix, "drums", roll(grid.beat, 0.25, grid.s16 / 2.0), t3, 0.28, 0.0);
        put(
            mix,
            "fx",
            synth::whoosh(grid.beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED),
            t3,
            0.18,
            0.0,
        );
        Ok(())
    }

    /// FIGHT!: the crash and the groove drops.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "fx", synth::crash(1.6), t, 0.32, 0.0);
        Ok(())
    }

    /// The groove from FIGHT! to the K.O., hotter as it goes, a roll and a whoosh into the final exchange,
    /// tape-stopped at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let b0 = bf_core::py::round_half_even(grid.t_fight / grid.bar) as usize;
        let span = (bar_ko + 1).saturating_sub(b0).max(1) as f64;
        for b in b0..=bar_ko {
            let inten = 1.0 + 0.5 * ((b - b0 + 1) as f64 / span);
            self.bar(ctx, mix, b, false, inten, b >= b0 + 3, 1.0)?;
        }
        if bar_ko >= b0 + 4 {
            let t = (bar_ko - 2) as f64 * grid.bar;
            put(mix, "drums", roll(2.0 * grid.bar, 0.1, grid.s16), t, 0.10, 0.0);
            let w = 3.6f64.min(t);
            let x = synth::whoosh(w, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", x, t + 2.0 * grid.bar - w, 0.10, 0.0);
        }
        for bus in ["drums", "bass", "cowbell", "pad"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a crash, the 808 falling an octave and more, a thud.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        let r = bass_root(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        put(mix, "fx", synth::crash(2.4), t_ko, 0.42, 0.0);
        put(mix, "bass", b808(r + 3, r - 9, 1.4, 1.2), t_ko, 0.6, 0.0);
        let thud = synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED);
        put(mix, "fx", thud, t_ko, 0.7, 0.0);
        Ok(())
    }

    /// A crash and a soft shimmer on the home chord under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        put(mix, "fx", synth::crash(1.8), tw, 0.18, 0.0);
        let [r, third, fifth] = pad_voicing(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        put(
            mix,
            "fx",
            shimmer(&[r + 36, third + 36, fifth + 36], 1.6),
            tw,
            0.035,
            0.0,
        );
        Ok(())
    }

    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        let x = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", x, tw + 0.1, 0.10, 0.0);
        Ok(())
    }

    /// The lighter groove under the solo, up to the build, lifted by [`VICTORY_LIFT`].
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, true, 1.0, true, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar: a snare roll, a riser, the 808 sliding up an octave and kicks doubling up into the title
    /// slam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let dur = grid.dur;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let span = dur - t_last;
        put(mix, "drums", roll(span + 0.03, 0.15, grid.s16 / 2.0), t_last, 0.30, 0.0);
        put(mix, "fx", riser(span, RISER_OVER, RISER_SEED)?, t_last, 0.22, 0.0);
        let r = bass_root(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        put(mix, "bass", b808(r, r + 12, span - 0.02, 0.9), t_last, 0.40, 0.0);
        for st in [0usize, 4, 8, 10, 12, 14] {
            put(
                mix,
                "drums",
                scaled(&self.kick, 1.0),
                t_last + st as f64 * grid.s16,
                0.5,
                0.0,
            );
        }
        Ok(())
    }
}
