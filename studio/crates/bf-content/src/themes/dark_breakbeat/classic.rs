//! The dark breakbeat's classic arrangement: the bed of `projects/ballfight_turntables_vs_synth/audio.py`'s
//! `main()` (the parts that belong to the bed, not to TURNTABLES or SYNTH), section by section as the score calls
//! them. A quick port: by ear, not sample for sample.

use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, riser};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{bass_root, cold_pad, crunch, kick, pad, reese, reese_plain, snare_hit, sub_drop, thunder_crack};

/// The name the bed's errors carry.
const H: &str = "dark_breakbeat";

/// The break: (16th, piece, velocity). `K` kick, `S` snare, `G` ghost snare, `H` hat.
const BREAK_A: [(usize, char, f64); 11] = [
    (0, 'K', 1.0),
    (2, 'H', 0.5),
    (4, 'S', 1.0),
    (6, 'H', 0.5),
    (7, 'G', 0.35),
    (8, 'H', 0.55),
    (10, 'K', 0.95),
    (11, 'K', 0.6),
    (12, 'S', 1.0),
    (14, 'H', 0.5),
    (15, 'G', 0.4),
];
/// The chop on every fourth bar: a stutter into the next bar.
const BREAK_B: [(usize, char, f64); 11] = [
    (0, 'K', 1.0),
    (2, 'H', 0.5),
    (4, 'S', 1.0),
    (6, 'K', 0.7),
    (7, 'G', 0.35),
    (8, 'H', 0.55),
    (10, 'K', 0.95),
    (12, 'S', 1.0),
    (13, 'S', 0.5),
    (14, 'S', 0.7),
    (15, 'S', 0.85),
];

/// The dark breakbeat's bed. The crunched kick and snare and the hat are the same on every hit, so they are made
/// once and scaled per stroke.
#[derive(Clone, Debug)]
pub struct DarkBreakbeatClassic {
    kick: Vec<f64>,
    snare: Vec<f64>,
    hat: Vec<f64>,
}

impl Default for DarkBreakbeatClassic {
    fn default() -> DarkBreakbeatClassic {
        DarkBreakbeatClassic::new()
    }
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

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

fn scaled(x: &[f64], v: f64) -> Vec<f64> {
    x.iter().map(|s| s * v).collect()
}

/// The key's root in the reese's octave (F1 in F).
fn key_bass(ctx: &MusicCtx) -> i32 {
    24 + ctx.harmony.key_pc
}

impl DarkBreakbeatClassic {
    /// The bed, with its every-hit sounds made.
    pub fn new() -> DarkBreakbeatClassic {
        DarkBreakbeatClassic {
            kick: kick(1.0),
            snare: snare_hit(),
            hat: synth::hat(0.03, false),
        }
    }

    /// `crunch(kick(v))`.
    fn ck(&self, v: f64) -> Vec<f64> {
        crunch(&scaled(&self.kick, v))
    }

    /// `crunch(snare(v))`.
    fn cs(&self, v: f64) -> Vec<f64> {
        crunch(&scaled(&self.snare, v))
    }

    /// `break_bar(mix, b, k, light)`.
    fn break_bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, k: f64, light: bool) {
        let g = &ctx.show.grid;
        let t0 = b as f64 * g.bar;
        let pat = if b % 4 == 3 && !light { &BREAK_B } else { &BREAK_A };
        for &(st, pc, v) in pat {
            let t = t0 + st as f64 * g.s16 + if st % 2 == 1 { 0.012 } else { 0.0 };
            match pc {
                'K' => put(mix, "break", self.ck(v), t, 0.55 * k, 0.0),
                'S' => put(mix, "break", self.cs(v), t, 0.42 * k, 0.05),
                'G' => put(mix, "break", self.cs(v * 0.6), t, 0.25 * k, -0.1),
                _ => put(mix, "break", scaled(&self.hat, v), t, 0.22 * k, 0.25),
            }
            if pc == 'H' && (st == 2 || st == 10) && !light {
                put(mix, "break", scaled(&self.hat, 0.3), t + g.s16, 0.16 * k, 0.25);
            }
        }
    }

    /// `bed(mix, b, intensity, light)`: one bar: the break, the reese (the root held, a push on the "and" of 3, an
    /// octave jump into the next bar later on), the cold pad.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let beat = ctx.show.grid.beat;
        let t0 = b as f64 * ctx.show.grid.bar;
        let ch = chord(ctx, b)?;
        let r = bass_root(ch).map_err(bad)?;
        self.break_bar(ctx, mix, b, 1.0, light);
        put(mix, "bass", reese_plain(r, 2.5 * beat, 1.0), t0, 0.40, 0.0);
        put(
            mix,
            "bass",
            reese(r, beat, 0.8, 400.0, 1300.0),
            t0 + 2.5 * beat,
            0.34,
            0.0,
        );
        if intensity > 1.3 && !light {
            put(
                mix,
                "bass",
                reese(r + 12, 0.45 * beat, 0.7, 500.0, 1500.0),
                t0 + 3.5 * beat,
                0.26,
                0.0,
            );
        }
        if !light {
            let notes = pad(&ctx.harmony, ch).map_err(bad)?;
            let g = 0.07 + 0.03 * (intensity - 1.0).min(1.0);
            put(
                mix,
                "pad",
                cold_pad(&notes, ctx.show.grid.bar + 0.1, 1100.0),
                t0,
                g,
                0.0,
            );
        }
        Ok(())
    }
}

impl Bed for DarkBreakbeatClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// `BUS_GAIN` and the reverb sends of the episode's master, for the bed's buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("break", (1.6, 0.1)), ("bass", (0.9, 0.0)), ("pad", (0.9, 0.6))]
    }

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

    /// A big drum hit: the crunched kick and a short sub drop under it.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "break", self.ck(1.0), t, g, 0.0);
        put(mix, "bass", sub_drop(0.6, 95.0, 30.0), t, g * 0.6, 0.0);
        Ok(())
    }

    /// `mix.place('fx', A.chime(91, 0.25), t, 0.05)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// The band on every word of the VS card: the kick, a reese stab, a boom.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let r = key_bass(ctx);
        for tw in times {
            put(mix, "break", self.ck(1.0), tw, 0.55, 0.0);
            put(mix, "bass", reese(r, 0.42, 1.0, 500.0, 1400.0), tw, 0.42, 0.0);
            let seed = (tw * 50.0) as u64 + 3;
            put(mix, "fx", synth::thud(0.9, 120.0, 32.0, seed), tw, 0.50, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder_crack(), t, 0.42, 0.0);
        put(mix, "fx", synth::crash(1.6), t, 0.10, 0.0);
        Ok(())
    }

    /// The cold pad under the whole card (never quiet), a snare pickup as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let notes = pad(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        put(mix, "pad", cold_pad(&notes, 3.0 * g.beat, 1400.0), 0.0, 0.10, 0.0);
        for (st, v) in [(12, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)] {
            put(mix, "break", self.cs(v), st as f64 * g.s16, 0.36, 0.0);
        }
        Ok(())
    }

    /// The count-in on the second chord: a reese stab and a kick per count, a snare pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let r = bass_root(chord(ctx, 1)?).map_err(bad)?;
        for (tc, _) in g.count.iter().take(3) {
            put(mix, "bass", reese_plain(r, 0.36, 0.9), *tc, 0.40, 0.0);
            put(mix, "break", self.ck(0.9), *tc, 0.45, 0.0);
        }
        for (st, v) in [(12, 0.45), (13, 0.6), (14, 0.75), (15, 0.9)] {
            put(mix, "break", self.cs(v), g.bar + st as f64 * g.s16, 0.36, 0.0);
        }
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        put(mix, "fx", synth::crash(2.0), ctx.show.grid.t_fight, 0.18, 0.0);
        Ok(())
    }

    /// The bed from FIGHT! to the K.O. (denser as it goes), a whoosh into the final exchange, a tape stop at the
    /// K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko.max(1) as f64), false)?;
        }
        let t_ko = ctx.show.t_ko;
        let t_whoosh = if t_ko > 44.0 { 44.0 } else { t_ko - 0.5 } - 3.6;
        if t_whoosh > ctx.show.grid.t_fight {
            let w = synth::whoosh(3.6, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", w, t_whoosh, 0.10, 0.0);
        }
        for bus in ["break", "bass", "pad"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "fx", synth::crash(2.0), t_ko, 0.22, 0.0);
        put(mix, "bass", sub_drop(1.4, 80.0, 26.0), t_ko, 0.62, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.7, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        put(mix, "bass", reese_plain(key_bass(ctx), 1.2, 1.0), tw, 0.5, 0.0);
        put(mix, "fx", synth::crash(1.8), tw, 0.14, 0.0);
        Ok(())
    }

    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        let w = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, tw + 0.1, 0.10, 0.0);
        Ok(())
    }

    /// The bare break and the bass under the solo, up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.0, true)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a snare roll, a bass climb, a riser, the pad, kicks.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let t_last = g.n_bars.saturating_sub(1) as f64 * g.bar;
        let ch = chord(ctx, g.n_bars.saturating_sub(1))?;
        for st in 0..16 {
            let v = 0.3 + 0.7 * st as f64 / 15.0;
            put(
                mix,
                "break",
                self.cs(v),
                t_last + st as f64 * g.s16,
                0.30 * (0.5 + 0.5 * v),
                0.0,
            );
            if st % 2 == 1 {
                let t = t_last + (st as f64 + 0.5) * g.s16;
                put(mix, "break", self.cs(v * 0.6), t, 0.18 * v, 0.0);
            }
        }
        put(
            mix,
            "fx",
            riser(g.dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.22,
            0.0,
        );
        let r = bass_root(ch).map_err(bad)?;
        for (bt, step) in [0, 2, 4, 7].into_iter().enumerate() {
            let t = t_last + bt as f64 * g.beat;
            put(mix, "bass", reese_plain(r + step, 0.9 * g.beat, 0.9), t, 0.40, 0.0);
        }
        let mut notes = pad(&ctx.harmony, ch).map_err(bad)?;
        if let Some(&m) = notes.get(1) {
            notes.push(m + 12);
        }
        put(
            mix,
            "pad",
            cold_pad(&notes, g.dur - t_last + 0.02, 1600.0),
            t_last,
            0.08,
            0.0,
        );
        for st in [0, 8, 12, 14] {
            put(mix, "break", self.ck(0.9), t_last + st as f64 * g.s16, 0.45, 0.0);
        }
        Ok(())
    }
}
