//! The noir jazz's classic arrangement: the bed of #29 (`projects/ballfight_trumpet_vs_sax/audio.py`), section by
//! section as the score calls them, by ear (the episode's horns' own lines are not part of the bed and are not
//! here).

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, h01, noise_seed, riser};
use bf_sound::synth::{self, FILTER_O, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{
    PIANO, SWING, bass_root, bass_tones, brush_tap, comp_voicing, dark_pad, pad_voicing, ride, roll, swirl, upright,
};

/// The name the bed's errors carry.
const H: &str = "noir_jazz";

/// How much louder the bars under the solo are: #29 lifted its master there by about 4.5 dB over the fight (its
/// `GAIN_PTS`), which the kit's master has no means for, so the bed lifts its lighter band itself.
const VICTORY_LIFT: f64 = 1.6;

/// The share of the bars from which the drummer switches from brushes to sticks (#29: bar 22 of 34).
const STICKS_FROM: f64 = 22.0 / 34.0;

/// The noir jazz's bed. The ride and its bell are made once, when the bed is made, and scaled per stroke.
#[derive(Clone, Debug)]
pub struct NoirJazzClassic {
    ride: Vec<f64>,
    bell: Vec<f64>,
}

impl Default for NoirJazzClassic {
    fn default() -> NoirJazzClassic {
        NoirJazzClassic::new()
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

/// `piano_chord(ms, dur, vel)`: the muted comping stab, on the PIANO's voice.
fn piano_chord(ctx: &MusicCtx, ms: &[i32], dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
    let mut x: Vec<f64> = Vec::new();
    for &m in ms {
        let s = ctx.note(
            PIANO,
            f64::from(m),
            round_nd(dur, 3),
            round_nd(dur + 0.25, 2),
            round_nd(vel, 1),
        )?;
        if x.len() < s.len() {
            x.resize(s.len(), 0.0);
        }
        for (o, v) in x.iter_mut().zip(s.iter()) {
            *o += v;
        }
    }
    let k = 1.6 / ms.len().max(1) as f64;
    let x: Vec<f64> = x.iter().map(|v| v * k).collect();
    Ok(synth::lp(&x, 2400.0, FILTER_O))
}

/// `walk(b)`: four walking quarter notes for bar `b`: the root, two chord tones, a chromatic approach to the next
/// bar's root.
fn walk(ctx: &MusicCtx, b: usize) -> Result<[i32; 4], MusicError> {
    let n = ctx.show.grid.n_bars.max(1);
    let (ch, nxt) = (chord(ctx, b % n)?, chord(ctx, (b + 1) % n)?);
    let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
    let tones = bass_tones(&ctx.harmony, ch).map_err(bad)?;
    let bf = b as f64;
    let m2 = if h01(&[bf, 1.0]) < 0.5 {
        tones[1]
    } else if tones[3] - 12 >= 28 {
        tones[3] - 12
    } else {
        tones[3]
    };
    let tgt = bass_root(&ctx.harmony, nxt).map_err(bad)?;
    let m4 = tgt + if h01(&[bf, 2.0]) < 0.5 { 1 } else { -1 };
    Ok([r, m2, tones[2], m4])
}

impl NoirJazzClassic {
    /// The bed, with its ride made.
    pub fn new() -> NoirJazzClassic {
        NoirJazzClassic {
            ride: ride(false),
            bell: ride(true),
        }
    }

    /// `bed(mix, b, intensity, light, sticks)`: one bar: the walking bass, brushes (or sticks: ride, feathered
    /// kick, hi-hat foot, snare comping, a crash every four bars), piano comping on the Charleston (a third stab as
    /// it builds), the dark pad once the fight is under way.
    #[allow(clippy::too_many_arguments)]
    fn bar(
        &self,
        ctx: &MusicCtx,
        mix: &mut Mix,
        b: usize,
        intensity: f64,
        light: bool,
        sticks: bool,
        pad: bool,
        lift: f64,
    ) -> Played {
        let put = |mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, g: f64, pan: f64| put(mix, bus, x, t, g * lift, pan);
        let grid = &ctx.show.grid;
        let beat = grid.beat;
        let t0 = b as f64 * grid.bar;
        let ch = chord(ctx, b)?;
        for (j, m) in walk(ctx, b)?.into_iter().enumerate() {
            let vel = if j == 0 { 1.0 } else { 0.95 };
            put(mix, "bass", upright(m, 0.40, vel), t0 + j as f64 * beat, 0.60, 0.0);
        }
        if !sticks {
            put(mix, "drums", swirl(grid.bar, beat, b as u64), t0, 0.030, 0.0);
            for bt in [1usize, 3] {
                put(
                    mix,
                    "drums",
                    brush_tap(0.9, (b + bt) as u64),
                    t0 + bt as f64 * beat,
                    0.20,
                    0.0,
                );
            }
            for (bt, v) in [
                (0.0, 0.8),
                (1.0, 0.7),
                (1.0 + SWING, 0.45),
                (2.0, 0.8),
                (3.0, 0.7),
                (3.0 + SWING, 0.45),
            ] {
                put(mix, "drums", scaled(&self.ride, v), t0 + bt * beat, 0.07, 0.35);
            }
        } else {
            for (bt, v) in [
                (0.0, 1.0),
                (1.0, 0.85),
                (1.0 + SWING, 0.6),
                (2.0, 1.0),
                (3.0, 0.85),
                (3.0 + SWING, 0.6),
            ] {
                put(mix, "drums", scaled(&self.ride, v), t0 + bt * beat, 0.10, 0.35);
            }
            for bt in 0..4 {
                let k = synth::kick(0.5, 0.2, synth::KICK_F0, synth::KICK_F1);
                put(mix, "drums", k, t0 + f64::from(bt) * beat, 0.10, 0.0);
            }
            for bt in [1.0, 3.0] {
                put(mix, "drums", synth::hat(0.03, false), t0 + bt * beat, 0.10, -0.3);
            }
            for bt in [1.0 + SWING, 2.0 + SWING, 3.0 + SWING] {
                if h01(&[b as f64, bt]) < 0.45 {
                    let s = scaled(&synth::snare(0.09), 0.5);
                    put(mix, "drums", s, t0 + bt * beat, 0.10, -0.1);
                }
            }
            if b.is_multiple_of(4) {
                put(mix, "drums", synth::crash(1.6), t0, 0.07, 0.4);
            }
        }
        let vo = comp_voicing(&ctx.harmony, ch).map_err(bad)?;
        let mut hits = vec![(0.0, 0.26), (1.0 + SWING, 0.18)];
        if intensity > 1.3 && !light {
            hits.push((3.0 + SWING, 0.14));
        }
        for (bt, d) in hits {
            let x = piano_chord(ctx, &vo, d, 0.6)?;
            put(mix, "comp", x, t0 + bt * beat, if light { 0.20 } else { 0.24 }, -0.2);
        }
        if pad || light {
            let x = dark_pad(&pad_voicing(&ctx.harmony, ch).map_err(bad)?, grid.bar + 0.05, 900.0);
            put(mix, "pad", x, t0, 0.05, 0.0);
        }
        Ok(())
    }
}

impl Bed for NoirJazzClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    /// `BUS_GAIN` and the reverb sends of #29's master for the bed's own buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("drums", (1.8, 0.15)),
            ("bass", (0.8, 0.0)),
            ("comp", (1.4, 0.4)),
            ("pad", (1.2, 0.5)),
        ]
    }

    /// A smoky basement club.
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

    /// A slam: the kick, a crash and the upright's low root.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let r = bass_root(&ctx.harmony, ctx.harmony.chord_at(t)).map_err(bad)?;
        put(mix, "drums", synth::kick(1.0, 0.32, 150.0, 46.0), t, 0.5 * g, 0.0);
        put(mix, "drums", synth::crash(1.4), t, 0.12 * g, 0.3);
        put(mix, "bass", upright(r - 12, 0.6, 1.0), t, 0.5 * g, 0.0);
        let seed = noise_seed(H, t * 3.0)?;
        put(mix, "fx", synth::thud(0.6, 90.0, 32.0, seed), t, 0.45 * g, 0.0);
        Ok(())
    }

    /// A band hit on the chord: the piano's voicing over the upright's root, the ride's bell and a kick.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let vo = comp_voicing(&ctx.harmony, ch).map_err(bad)?;
        put(
            mix,
            "comp",
            piano_chord(ctx, &vo, py_max(ring, 0.18), 0.8)?,
            t,
            0.26 * g,
            -0.2,
        );
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        put(mix, "bass", upright(r, py_max(ring, 0.3), 1.0), t, 0.5 * g, 0.0);
        put(mix, "drums", self.bell.clone(), t, 0.10 * g, 0.35);
        put(mix, "drums", synth::kick(0.8, 0.25, 150.0, 46.0), t, 0.25 * g, 0.0);
        Ok(())
    }

    /// A clash: a tap on the ride's bell.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "drums", scaled(&self.bell, 0.6), t, 0.06, 0.35);
        Ok(())
    }

    /// Every word: a piano stab on the home chord over the upright's root, a brush slap and the ride's bell.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        let vo = comp_voicing(&ctx.harmony, ch).map_err(bad)?;
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        for (j, tw) in times.into_iter().enumerate() {
            put(mix, "comp", piano_chord(ctx, &vo, 0.3, 0.8)?, tw, 0.24, -0.2);
            put(mix, "bass", upright(r, 0.5, 1.0), tw, 0.62, 0.0);
            put(mix, "drums", brush_tap(1.0, j as u64), tw, 0.30, 0.0);
            put(mix, "drums", self.bell.clone(), tw, 0.08, 0.35);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.5, 120.0, 40.0, seed), tw, 0.25, 0.0);
        }
        Ok(())
    }

    /// VS: a thud and a crash.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED), t, 0.45, 0.0);
        put(mix, "drums", synth::crash(1.8), t, 0.12, 0.3);
        Ok(())
    }

    /// The dark pad and the brushes' swirl under the card.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let pad = dark_pad(
            &pad_voicing(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?,
            grid.bar,
            900.0,
        );
        put(mix, "pad", pad, 0.0, 0.05, 0.0);
        put(mix, "drums", swirl(grid.bar, grid.beat, 0), 0.0, 0.025, 0.0);
        Ok(())
    }

    /// Bar 1 on the second chord: a brush tap, the ride, the upright's root and a piano stab per count, a snare
    /// pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ch = chord(ctx, 1)?;
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        let vo = comp_voicing(&ctx.harmony, ch).map_err(bad)?;
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            put(mix, "drums", brush_tap(1.0, j as u64), *tc, 0.30, 0.0);
            put(mix, "drums", scaled(&self.ride, 0.8), *tc, 0.06, 0.35);
            put(mix, "bass", upright(r, 0.4, 1.0), *tc, 0.62, 0.0);
            put(mix, "comp", piano_chord(ctx, &vo, 0.14, 0.7)?, *tc, 0.20, -0.2);
        }
        for (st, v) in [(12usize, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)] {
            let t = grid.bar + st as f64 * grid.s16;
            put(mix, "drums", brush_tap(v, st as u64), t, 0.30, 0.0);
            put(mix, "drums", scaled(&synth::snare(0.09), v * 0.6), t, 0.14, 0.0);
        }
        let t3 = grid.bar + 3.0 * grid.beat;
        let w = synth::whoosh(grid.beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, t3, 0.14, 0.0);
        Ok(())
    }

    /// FIGHT!: a crash and a kick, and the band comes in.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(2.0), t, 0.16, 0.3);
        put(mix, "drums", synth::kick(1.0, 0.32, 150.0, 46.0), t, 0.30, 0.0);
        Ok(())
    }

    /// The band from FIGHT! to the K.O., hotter as it goes, sticks for the last third, tape-stopped at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let b0 = bf_core::py::round_half_even(grid.t_fight / grid.bar) as usize;
        let span = (bar_ko + 1).saturating_sub(b0).max(1) as f64;
        let sticks_from = (STICKS_FROM * grid.n_bars as f64) as usize;
        for b in b0..=bar_ko {
            let inten = 1.0 + 0.6 * ((b - b0 + 1) as f64 / span);
            self.bar(ctx, mix, b, inten, false, b >= sticks_from, b > b0, 1.0)?;
        }
        if sticks_from > b0 && sticks_from <= bar_ko {
            let t = sticks_from as f64 * grid.bar;
            let w = 3.6f64.min(t);
            let x = synth::whoosh(w, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", x, t - w, 0.08, 0.0);
        }
        for bus in ["bass", "drums", "comp", "pad"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a crash, the upright's lowest root, a thud.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        let r = ctx.harmony.root_above(chord(ctx, 0)?, 24).map_err(bad)?;
        put(mix, "drums", synth::crash(2.4), t_ko, 0.22, 0.0);
        put(mix, "bass", upright(r, 1.4, 1.0), t_ko, 0.62, 0.0);
        let thud = synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED);
        put(mix, "fx", thud, t_ko, 0.7, 0.0);
        Ok(())
    }

    /// The home chord on the piano (root and voicing), the upright's root and a crash under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ch = chord(ctx, 0)?;
        let mut ms = vec![ctx.harmony.root_above(ch, 55).map_err(bad)?];
        ms.extend(comp_voicing(&ctx.harmony, ch).map_err(bad)?);
        put(mix, "comp", piano_chord(ctx, &ms, 0.8, 0.8)?, tw, 0.24, 0.0);
        let r = bass_root(&ctx.harmony, ch).map_err(bad)?;
        put(mix, "bass", upright(r, 0.9, 1.0), tw, 0.6, 0.0);
        put(mix, "drums", synth::crash(1.8), tw, 0.14, 0.3);
        Ok(())
    }

    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        let x = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", x, tw + 0.1, 0.10, 0.0);
        Ok(())
    }

    /// The lighter band (brushes) under the solo, up to the build, lifted by [`VICTORY_LIFT`].
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.0, true, false, true, VICTORY_LIFT)?;
        }
        Ok(())
    }

    /// The last bar: a brush roll, a riser, the upright walking up the last chord's arpeggio, piano stabs on the
    /// Charleston, the pad swelling, kicks into the title slam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, dur) = (grid.beat, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let span = dur - t_last;
        put(mix, "drums", roll(span + 0.03, 0.15, grid.s16 / 2.0), t_last, 0.40, 0.0);
        put(mix, "fx", riser(span, RISER_OVER, RISER_SEED)?, t_last, 0.22, 0.0);
        let last = chord(ctx, grid.n_bars.saturating_sub(1))?;
        let r = bass_root(&ctx.harmony, last).map_err(bad)?;
        let [_, third, fifth, _] = bass_tones(&ctx.harmony, last).map_err(bad)?;
        let fifth = if fifth < r { fifth + 12 } else { fifth };
        for (bt, m) in [r, third, fifth, r + 12].into_iter().enumerate() {
            put(mix, "bass", upright(m, 0.36, 1.0), t_last + bt as f64 * beat, 0.58, 0.0);
        }
        let vo = comp_voicing(&ctx.harmony, last).map_err(bad)?;
        for bt in [0.0, 1.0 + SWING, 3.0 + SWING] {
            let x = piano_chord(ctx, &vo, 0.18, 0.7)?;
            put(mix, "comp", x, t_last + bt * beat, 0.22, -0.2);
        }
        let pad = dark_pad(&pad_voicing(&ctx.harmony, last).map_err(bad)?, span + 0.02, 1200.0);
        put(mix, "pad", pad, t_last, 0.07, 0.0);
        for st in [0usize, 8, 12, 14] {
            let k = synth::kick(0.9, 0.32, 150.0, 46.0);
            put(mix, "drums", k, t_last + st as f64 * grid.s16, 0.26, 0.0);
        }
        Ok(())
    }
}
