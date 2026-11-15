//! The villain march's classic arrangement: the bed of #32 v3 (`projects/ballfight_tuba_vs_flute/audio.py`),
//! section by section as the score calls them, by ear (the tuba's and the flute's own lines are their parts').

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, riser};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED, noise};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{bass_drum_boom, brass, horn_pad, lullaby, march_snare_stroke, music_box, roll, root, timpani, voice};

/// The name the bed's errors carry.
const H: &str = "villain_march";

/// How much louder the bars under the solo are: #32 lifted its master there (its `GAIN_PTS`), which the kit's
/// master has no means for, so the bed lifts itself.
const VICTORY_LIFT: f64 = 2.0;

/// `brass`'s defaults: the attack, the filter's cut and the bite.
const ATT: f64 = 0.012;
const CUT: f64 = 1500.0;

/// `SNARE16`: the snare ostinato, the 16ths of a bar with their velocities.
const SNARE16: [(usize, f64); 10] = [
    (0, 1.0),
    (3, 0.45),
    (4, 0.75),
    (6, 0.5),
    (8, 0.9),
    (11, 0.45),
    (12, 0.7),
    (13, 0.4),
    (14, 0.5),
    (15, 0.62),
];

/// The villain march's bed. The bass drum and the march snare are the same at every velocity, so they are made
/// once, when the bed is made, and scaled per stroke.
#[derive(Clone, Debug)]
pub struct VillainMarchClassic {
    boom: Vec<f64>,
    snare: Vec<f64>,
}

impl Default for VillainMarchClassic {
    fn default() -> VillainMarchClassic {
        VillainMarchClassic::new()
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

/// The brass chord of the home chord spread over an octave: the root, its fifth, the root an octave up and the
/// third above it (and the fifth above that, for `wide`).
fn spread(ctx: &MusicCtx, ch: &str, wide: bool) -> Result<Vec<i32>, MusicError> {
    let r = root(&ctx.harmony, ch).map_err(bad)?;
    let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
    let up = |i: usize, or: i32| pcs.get(i).map_or(or, |p| (p - pcs[0]).rem_euclid(12));
    let mut ms = vec![r, r + up(2, 7), r + 12, r + 12 + up(1, 3)];
    if wide {
        ms.push(r + 12 + up(2, 7));
    }
    Ok(ms)
}

impl VillainMarchClassic {
    /// The bed, with its every-beat sounds made.
    pub fn new() -> VillainMarchClassic {
        VillainMarchClassic {
            boom: bass_drum_boom(),
            snare: march_snare_stroke(),
        }
    }

    fn bass_drum(&self, vel: f64) -> Vec<f64> {
        scaled(&self.boom, vel)
    }

    fn march_snare(&self, vel: f64) -> Vec<f64> {
        scaled(&self.snare, vel)
    }

    /// `bed(mix, b, intensity)`: one bar of the march: the snare ostinato and the bass drum, the brass "pah"
    /// on 2 and 4 (and its off-beat echo when hot), the horn swell (from FIGHT!, a bar earlier than #32, to keep
    /// the opening as loud as the rest), the lullaby, timpani and crashes in the back
    /// half.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, lift: f64) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let t0 = b as f64 * grid.bar;
        let ch = chord(ctx, b)?;
        for (st, v) in SNARE16 {
            let g = 0.24 * (0.8 + 0.2 * intensity);
            put(mix, "drums", self.march_snare(v), t0 + st as f64 * s16, g, -0.1);
        }
        for bt in [0usize, 2] {
            let vel = if bt == 0 { 1.0 } else { 0.8 };
            let g = 0.21;
            put(mix, "drums", self.bass_drum(vel), t0 + bt as f64 * beat, g, 0.0);
        }
        let vo = voice(&ctx.harmony, ch).map_err(bad)?;
        for bt in [1usize, 3] {
            let g = 0.24;
            put(
                mix,
                "brass",
                brass(&vo, 0.22, ATT, CUT, 1.0),
                t0 + bt as f64 * beat,
                g,
                0.0,
            );
            if intensity > 1.35 {
                let echo = brass(&vo, 0.12, ATT, CUT, 0.6);
                put(mix, "brass", echo, t0 + (bt as f64 + 0.5) * beat, 0.12, 0.0);
            }
        }
        if b >= 2 {
            let top: Vec<i32> = vo.iter().copied().take(3).collect();
            put(mix, "pad", horn_pad(&top, grid.bar), t0, 0.07, 0.0);
        }
        if b >= 4 {
            let line = lullaby(&ctx.harmony, b, ch).map_err(bad)?;
            for (q, m) in line.into_iter().enumerate() {
                put(
                    mix,
                    "musicbox",
                    music_box(m + 12, 0.9),
                    t0 + q as f64 * beat,
                    0.06,
                    0.35,
                );
            }
        }
        if intensity > 1.3 {
            let r = ctx.harmony.root_above(ch, 36).map_err(bad)?;
            let g = 0.16 * ((intensity - 1.3) / 0.2).min(1.0);
            put(mix, "drums", timpani(r + 12, 0.9), t0, g, 0.0);
            if b % 4 == 1 {
                put(mix, "drums", synth::crash(1.6), t0, 0.07, 0.0);
            }
        }
        Ok(())
    }
}

impl Bed for VillainMarchClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// The reverb sends of #32's master for the bed's own buses; the gains are #32's raised by about 5 dB, since the
    /// kit's master has no gain automation (#32 lifted the whole mix under the solo, where the march is alone).
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("drums", (1.6, 0.15)),
            ("brass", (5.0, 0.4)),
            ("pad", (3.4, 0.5)),
            ("musicbox", (2.0, 0.7)),
        ]
    }

    /// A stone parade ground.
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

    /// A slam: the bass drum.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "drums", self.bass_drum(1.0), t, g, 0.0);
        Ok(())
    }

    /// A band hit: the brass on the chord over the bass drum and a snare.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let vo = voice(&ctx.harmony, ch).map_err(bad)?;
        put(
            mix,
            "brass",
            brass(&vo, py_max(ring, 0.2), ATT, CUT, 1.0),
            t,
            0.30 * g,
            0.0,
        );
        put(mix, "drums", self.bass_drum(1.0), t, 0.45 * g, 0.0);
        put(mix, "drums", self.march_snare(1.0), t, 0.25 * g, 0.0);
        Ok(())
    }

    /// A small music-box tine where the balls touch.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// The whole band on every word: bass drum, snare, the brass on the home chord, timpani, a thud.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        let ms = spread(ctx, ch, false)?;
        let r = root(&ctx.harmony, ch).map_err(bad)?;
        for tw in times {
            put(mix, "drums", self.bass_drum(1.0), tw, 0.55, 0.0);
            put(mix, "drums", self.march_snare(1.0), tw, 0.30, 0.0);
            put(mix, "brass", brass(&ms, 0.42, ATT, CUT, 1.0), tw, 0.30, 0.0);
            put(mix, "drums", timpani(r - 12, 1.0), tw, 0.30, 0.0);
            let seed = bf_sound::music::noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.9, 120.0, 32.0, seed), tw, 0.45, 0.0);
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
        put(mix, "drums", synth::crash(1.6), t, 0.10, 0.0);
        Ok(())
    }

    /// A horn pad under the whole card, a snare pickup as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ms = spread(ctx, chord(ctx, 0)?, false)?;
        put(mix, "pad", horn_pad(&ms, 3.0 * grid.beat), 0.0, 0.10, 0.0);
        for (st, v) in [(12usize, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)] {
            put(mix, "drums", self.march_snare(v), st as f64 * grid.s16, 0.30, 0.0);
        }
        Ok(())
    }

    /// Bar 1 on the second chord: the snare and a brass chord per count, a snare pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let vo = voice(&ctx.harmony, chord(ctx, 1)?).map_err(bad)?;
        for (tc, _) in &grid.count[..3] {
            put(mix, "drums", self.march_snare(1.0), *tc, 0.28, 0.0);
            put(mix, "brass", brass(&vo, 0.16, ATT, CUT, 1.0), *tc, 0.20, 0.0);
        }
        for (st, v) in [(12usize, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)] {
            put(
                mix,
                "drums",
                self.march_snare(v),
                grid.bar + st as f64 * grid.s16,
                0.30,
                0.0,
            );
        }
        Ok(())
    }

    /// FIGHT!: a crash and the bass drum.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(2.0), t, 0.14, 0.0);
        put(mix, "drums", self.bass_drum(1.0), t, 0.5, 0.0);
        Ok(())
    }

    /// The march from FIGHT! to the K.O., heavier as it goes, tape-stopped at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko.max(1) as f64), 1.0)?;
        }
        for bus in ["drums", "brass", "pad", "musicbox"] {
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
        put(
            mix,
            "fx",
            synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED),
            t_ko,
            0.7,
            0.0,
        );
        Ok(())
    }

    /// The brass's wide home chord, the bass drum and a crash under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ms = spread(ctx, chord(ctx, 0)?, true)?;
        put(mix, "brass", brass(&ms, 1.2, ATT, CUT, 1.0), tw, 0.24, 0.0);
        put(mix, "drums", self.bass_drum(1.0), tw, 0.5, 0.0);
        put(mix, "drums", synth::crash(2.0), tw, 0.12, 0.0);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo, and (not in #32, whose master lifted this stretch) the horns
    /// holding the home chord under it, so the march does not fall quiet between the K.O. and the solo.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.3, t_solo - tw - 0.1);
        let x = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", x, tw + 0.1, 0.10, 0.0);
        let ms = spread(ctx, chord(ctx, 0)?, false)?;
        put(mix, "pad", horn_pad(&ms, dur + 0.3), tw + 0.1, 0.16, 0.0);
        Ok(())
    }

    /// The march under the solo, up to the build: #32 lightened it and lifted the master there, so here the bars
    /// keep their full level and are lifted by [`VICTORY_LIFT`] (no off-beat brass, no timpani).
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let n_bars = ctx.show.grid.n_bars;
        for b in b_solo..n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.0, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar: a snare roll, a riser, the brass on the off-beats of the last chord, a horn pad with its root
    /// under it, the bass drum driving into the title slam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, dur) = (grid.beat, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let last = chord(ctx, grid.n_bars.saturating_sub(1))?;
        let x = roll(&self.snare, dur - t_last + 0.03, 0.15, grid.s16 / 2.0);
        put(mix, "drums", x, t_last, 0.30, 0.0);
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.22,
            0.0,
        );
        let vo = voice(&ctx.harmony, last).map_err(bad)?;
        for bt in 0..4usize {
            let t = t_last + bt as f64 * beat + beat / 2.0;
            put(mix, "brass", brass(&vo, 0.18, ATT, CUT, 1.0), t, 0.18, 0.0);
        }
        let mut pad = vec![ctx.harmony.root_above(last, 36).map_err(bad)?];
        pad.extend(ctx.harmony.chord_tones(55, 66, last).map_err(bad)?);
        put(mix, "pad", horn_pad(&pad, dur - t_last + 0.02), t_last, 0.08, 0.0);
        for st in [0usize, 8, 12, 14] {
            put(
                mix,
                "drums",
                self.bass_drum(0.9),
                t_last + st as f64 * grid.s16,
                0.42,
                0.0,
            );
        }
        Ok(())
    }
}
