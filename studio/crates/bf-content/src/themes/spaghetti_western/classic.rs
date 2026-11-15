//! The spaghetti western's classic arrangement: the bed of `projects/ballfight_ukulele_vs_acoustic/audio.py`'s
//! `main()` (the parts that belong to the bed, not to UKULELE or ACOUSTIC), section by section as the score calls
//! them. A quick port: by ear, not sample for sample.

use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, riser};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{
    WHIP_PRE, WHISTLE, bass, bell, choir, choir_notes, hoof, key_shift, strings_notes, thump, tremolo_strings, twang,
    whip, whistle,
};
use crate::themes::dark_breakbeat::thunder_crack;

/// The name the bed's errors carry.
const H: &str = "spaghetti_western";

/// The bed's own buses, which the K.O. tape-stops.
const BUSES: [&str; 6] = ["perc", "twang", "choir", "whistle", "strings", "bell"];

/// The spaghetti western's bed.
#[derive(Clone, Debug, Default)]
pub struct SpaghettiWesternClassic;

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

/// The bell's note: A2 in A, moved to the key.
fn bell_note(ctx: &MusicCtx) -> i32 {
    45 + key_shift(&ctx.harmony)
}

/// The bar where a section of #33's 33 bars starts (bars 10, 18 and 26 there), on a grid of `n_bars`.
fn section_start(n_bars: usize, at33: usize) -> usize {
    ((at33 * n_bars) as f64 / 33.0).round() as usize
}

impl SpaghettiWesternClassic {
    /// The bed.
    pub fn new() -> SpaghettiWesternClassic {
        SpaghettiWesternClassic
    }

    /// `bed(mix, b, light)`: one bar: the gallop, the twang bass, the choir, the bell, the strings and the snare by
    /// section.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, light: bool) -> Played {
        let g = &ctx.show.grid;
        let (beat, s16) = (g.beat, g.s16);
        let t0 = b as f64 * g.bar;
        let ch = chord(ctx, b)?;
        let nxt = chord(ctx, (b + 1) % ctx.harmony.chords.len())?;
        let (s1, s2, s3) = (
            section_start(g.n_bars, 10),
            section_start(g.n_bars, 18),
            section_start(g.n_bars, 26),
        );
        let sec = if light || b < s1 {
            0
        } else if b < s2 {
            1
        } else if b < s3 {
            2
        } else {
            3
        };
        let seed = (b * 4) as u64;
        for bt in 0..4usize {
            let tb = t0 + bt as f64 * beat;
            let on = bt == 0 || bt == 2;
            if on || sec >= 1 {
                put(mix, "perc", thump(if on { 1.0 } else { 0.6 }), tb, 0.36, 0.0);
            }
            let s = seed + bt as u64;
            put(mix, "perc", hoof(1.0, bt % 2 == 0, s), tb, 0.24, -0.2);
            put(mix, "perc", hoof(0.55, false, s + 50), tb + 2.0 * s16, 0.24, 0.2);
            put(mix, "perc", hoof(0.8, true, s + 90), tb + 3.0 * s16, 0.24, 0.0);
            if sec == 3 {
                put(mix, "perc", hoof(0.45, false, s + 130), tb + s16, 0.13, -0.1);
            }
        }
        let (r, f5) = bass(ch).map_err(bad)?;
        put(mix, "twang", twang(r, 1.4 * beat, 1.0), t0, 0.42, 0.0);
        put(mix, "twang", twang(f5, 1.2 * beat, 0.85), t0 + 2.0 * beat, 0.36, 0.0);
        if sec >= 1 {
            let tgt = bass(nxt).map_err(bad)?.0;
            let app = if tgt < r { tgt + 1 } else { tgt - 1 };
            put(mix, "twang", twang(app, 0.4 * beat, 0.7), t0 + 3.5 * beat, 0.30, 0.0);
        }
        if sec >= 1 || light {
            let notes = choir_notes(&ctx.harmony, ch).map_err(bad)?;
            let gain = if sec == 1 || light { 0.40 } else { 0.55 };
            put(mix, "choir", choir(&notes, g.bar + 0.15, 0.45, 0.5), t0, gain, 0.0);
        }
        if sec >= 2 && (b - s2).is_multiple_of(4) {
            put(mix, "bell", bell(bell_note(ctx), 1.0, 4.0), t0, 0.30, 0.0);
        }
        if sec == 3 {
            let k = ((b + 1 - s3) as f64 / 4.0).min(1.0);
            let notes = strings_notes(&ctx.harmony, ch).map_err(bad)?;
            let x = tremolo_strings(&notes, g.bar, 0.45 + 0.4 * k, 0.55 + 0.45 * k, s16);
            put(mix, "strings", x, t0, 0.22, 0.0);
            let snare = synth::hp(&synth::snare(0.06), 400.0, FILTER_O);
            for st in [0, 3, 6, 8, 11, 14] {
                put(mix, "perc", snare.clone(), t0 + st as f64 * s16, 0.05 + 0.03 * k, 0.0);
            }
        }
        Ok(())
    }

    /// `whistle_phrase(mix, b0)`: the theme over 8 bars from bar `b0`, with the dotted-8th echo on the other side.
    fn whistle_phrase(&self, ctx: &MusicCtx, mix: &mut Mix, b0: usize, gain: f64) {
        let g = &ctx.show.grid;
        let shift = key_shift(&ctx.harmony);
        let mut prev = None;
        for (bar, beat, m, d) in WHISTLE {
            let m = m + shift;
            let t = (b0 + bar) as f64 * g.bar + beat * g.beat;
            let x = whistle(m, d * g.beat * 0.96, 1.0, prev);
            let echo = synth::lp(&x, 3500.0, FILTER_O);
            put(mix, "whistle", x, t, gain, 0.08);
            put(mix, "whistle", echo, t + 0.75 * g.beat, gain * 0.24, -0.35);
            prev = Some(m);
        }
    }
}

impl Bed for SpaghettiWesternClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// `BUS_GAIN` and the reverb sends of the episode's master, for the bed's buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("perc", (1.0, 0.15)),
            ("twang", (0.55, 0.3)),
            ("choir", (1.0, 0.6)),
            ("bell", (0.9, 0.5)),
            ("strings", (1.0, 0.4)),
            ("whistle", (1.0, 0.9)),
        ]
    }

    /// A desert canyon.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.8, 0.8, 0.22)
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

    /// A big drum hit: the frame drum, full.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "perc", thump(1.0), t, g, 0.0);
        Ok(())
    }

    /// Two blades meeting: a small high bell.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// The band on every word of the VS card: the frame drum, a boom, a low twang, a choir stab.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let beat = ctx.show.grid.beat;
        let mut stab = choir_notes(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        if let Some(&m) = stab.first() {
            stab.push(m + 12);
        }
        let low = bass(chord(ctx, 0)?).map_err(bad)?.0;
        for tw in times {
            put(mix, "perc", thump(1.0), tw, 0.40, 0.0);
            let seed = (tw * 50.0) as u64 + 3;
            put(mix, "fx", synth::thud(0.9, 120.0, 32.0, seed), tw, 0.50, 0.0);
            put(mix, "twang", twang(low, 0.9 * beat, 1.0), tw, 0.40, 0.0);
            put(mix, "choir", choir(&stab, 0.32, 0.01, 0.25), tw, 0.45, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder_crack(), t, 0.42, 0.0);
        put(mix, "fx", synth::crash(1.6), t, 0.08, 0.0);
        Ok(())
    }

    /// The choir holds under the whole card (never quiet), a gallop pickup as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let notes = choir_notes(&ctx.harmony, chord(ctx, 0)?).map_err(bad)?;
        put(
            mix,
            "choir",
            choir(&notes, 3.0 * g.beat + 0.2, 0.05, 0.3),
            0.0,
            0.22,
            0.0,
        );
        for st in 12..16usize {
            let v = 0.55 + 0.15 * (st - 12) as f64;
            put(
                mix,
                "perc",
                hoof(v, st % 2 == 0, 200 + st as u64),
                st as f64 * g.s16,
                0.22,
                0.0,
            );
        }
        Ok(())
    }

    /// The count-in on the second chord: a whip crack, a low twang and the frame drum on each count, the gallop's
    /// pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let low = bass(chord(ctx, 1)?).map_err(bad)?.0;
        for (j, (tc, _)) in g.count.iter().take(3).enumerate() {
            put(mix, "fx", whip(1.0, j as u64), tc - WHIP_PRE, 0.30, 0.0);
            put(mix, "twang", twang(low, 0.9 * g.beat, 1.0), *tc, 0.42, 0.0);
            put(mix, "perc", thump(1.0), *tc, 0.36, 0.0);
        }
        for (st, v) in [(12usize, 0.5), (13, 0.65), (14, 0.8), (15, 1.0)] {
            let t = g.bar + st as f64 * g.s16;
            put(mix, "perc", hoof(v, st % 2 == 0, st as u64), t, 0.18, 0.0);
        }
        Ok(())
    }

    /// FIGHT!: the bell tolls and the whip cracks.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "bell", bell(bell_note(ctx), 1.0, 4.0), t, 0.34, 0.0);
        put(mix, "fx", whip(1.0, 9), t - WHIP_PRE, 0.30, 0.0);
        Ok(())
    }

    /// The bed from FIGHT! to the K.O., the whistled theme twice, a whoosh into the final exchange, a tape stop at
    /// the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let g = &ctx.show.grid;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, false)?;
        }
        self.whistle_phrase(ctx, mix, 2, 0.075);
        let s2 = section_start(g.n_bars, 18);
        if s2 + 8 < g.n_bars {
            self.whistle_phrase(ctx, mix, s2, 0.075);
        }
        let t_ko = ctx.show.t_ko;
        let t_whoosh = if t_ko > 44.0 { 44.0 } else { t_ko - 0.5 } - 3.6;
        if t_whoosh > g.t_fight {
            let w = synth::whoosh(3.6, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
            put(mix, "fx", w, t_whoosh, 0.10, 0.0);
        }
        for bus in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: the low bell, a whip crack, a boom.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "bell", bell(bell_note(ctx) - 5, 1.0, 5.0), t_ko, 0.40, 0.0);
        put(mix, "fx", whip(1.0, 13), t_ko - WHIP_PRE, 0.34, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.7, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        put(mix, "bell", bell(bell_note(ctx), 0.8, 4.0), tw, 0.26, 0.0);
        let low = bass(chord(ctx, 0)?).map_err(bad)?.0;
        put(mix, "twang", twang(low, 1.0, 1.0), tw, 0.45, 0.0);
        Ok(())
    }

    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        let w = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, tw + 0.1, 0.10, 0.0);
        Ok(())
    }

    /// A lighter gallop under the solo, up to the ride-out.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, true)?;
        }
        Ok(())
    }

    /// The last bar rides into the title slam on sample 0: the hooves break into a full gallop and swell, the
    /// frame drum and the twang climb the chord, a riser, the choir, the whistle's long note.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let t_last = g.n_bars.saturating_sub(1) as f64 * g.bar;
        let tail = g.dur - t_last;
        let ch = chord(ctx, g.n_bars.saturating_sub(1))?;
        let mut k = 0usize;
        while (k as f64) * g.s16 / 2.0 < tail {
            let u = k as f64 * g.s16 / 2.0 / tail;
            let t = t_last + k as f64 * g.s16 / 2.0;
            put(
                mix,
                "perc",
                hoof(0.4 + 0.6 * u, k.is_multiple_of(2), 300 + k as u64),
                t,
                0.15 + 0.05 * u,
                0.0,
            );
            k += 1;
        }
        let r = bass(ch).map_err(bad)?.0;
        let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
        let third = pcs.get(1).map_or(4, |p| (p - pcs[0]).rem_euclid(12));
        for (bt, step) in [0, third, 7, 12].into_iter().enumerate() {
            let t = t_last + bt as f64 * g.beat;
            put(mix, "perc", thump(0.8 + 0.2 * bt as f64 / 3.0), t, 0.36, 0.0);
            put(mix, "twang", twang(r + step, 0.9 * g.beat, 1.0), t, 0.40, 0.0);
        }
        put(mix, "fx", riser(tail, RISER_OVER, RISER_SEED)?, t_last, 0.22, 0.0);
        let mut notes = choir_notes(&ctx.harmony, ch).map_err(bad)?;
        if let Some(&m) = notes.first() {
            notes.push(m + 12);
        }
        put(mix, "choir", choir(&notes, tail + 0.02, 0.8, 0.001), t_last, 0.5, 0.0);
        let shift = key_shift(&ctx.harmony);
        let long = whistle(76 + shift, tail - 0.08, 1.0, Some(71 + shift));
        put(mix, "whistle", long, t_last + 0.05, 0.07, 0.0);
        Ok(())
    }
}
