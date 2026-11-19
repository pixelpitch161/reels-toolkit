//! The ska punk's classic arrangement: the band of #39's `audio.py` (`band_bar()`, `band_hit()` and the band's lines
//! of `main()`) on the kit's beat sheet (the VS card on bar 0, the count-in on bar 1, FIGHT! on bar 2), section by
//! section as `kit.music.score()` calls them. Quick-ported by ear: the same instruments, patterns and levels, not
//! the same samples. #39's BASSOON blat and BARI SAX honk on the card are the fighters' own parts here, and its
//! band hits under the captioned abilities are left to the fighters, so the bed sounds the same under any pair.
//! Like #39 it keeps one level from frame 0: the band builds with parts and density, never with level.

use bf_core::harmony::HarmonyError;
use bf_core::py::py_max;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, noise_seed, riser};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{kick, organ, organ_voicing, ride, rope_notes, rope_pluck, skank, skank_tones, snare, thunder, tom};

/// The name the bed's errors carry.
const H: &str = "ska_punk";

/// The bed's own buses: `BUS_GAIN` and the reverb sends of #39's `master()` (its `drums`, `guitar` and `organ`
/// buses are `kit`, `skank` and `bubble` here, so a DRUMS or ORGAN fighter keeps its own).
const BUSES: [(&str, (f64, f64)); 4] = [
    ("kit", (1.0, 0.12)),
    ("skank", (1.0, 0.3)),
    ("bubble", (1.0, 0.3)),
    ("ropes", (1.0, 0.1)),
];

/// The ska punk's band.
#[derive(Clone, Debug, Default)]
pub struct SkaPunkClassic;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// `T.CHORDS[b % N_BARS]`.
fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    if chords.is_empty() {
        return Err(MusicError::new(H, "the fight has no chords".to_string()));
    }
    Ok(chords[b % chords.len()].as_str())
}

impl SkaPunkClassic {
    /// The band.
    pub fn new() -> SkaPunkClassic {
        SkaPunkClassic
    }

    /// `band_bar(mix, b, dense)`: one bar of the band. `dense` 0 to 1 thickens the arrangement (every-beat kick,
    /// double-time snare, open hats, a ride, a crash every 2 bars), never the level.
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, dense: f64) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b)?;
        let sk = skank_tones(&ctx.harmony, ch).map_err(bad)?;
        let ov = organ_voicing(&ctx.harmony, ch).map_err(bad)?;
        for bt in 0..4usize {
            let tb = t0 + bt as f64 * beat;
            if bt % 2 == 0 {
                put(mix, "kit", kick(), tb, 0.42, 0.0);
            } else if dense > 0.75 {
                put(mix, "kit", kick(), tb, 0.20, 0.0);
            }
            if bt % 2 == 1 {
                put(mix, "kit", snare(), tb, 0.36, 0.0);
            } else if dense > 0.55 {
                put(mix, "kit", snare(), tb + beat / 2.0, 0.11, 0.0);
            }
            for e in 0..2 {
                let te = tb + f64::from(e) * beat / 2.0;
                if e == 1 && dense > 0.3 {
                    put(mix, "kit", synth::hat(0.04, true), te, 0.10, 0.25);
                } else {
                    put(mix, "kit", synth::hat(0.03, false), te, 0.13, 0.25);
                }
            }
            if dense > 0.45 {
                put(mix, "kit", ride(bt as u64), tb, 0.08, -0.3);
            }
            let seed = (b * 4 + bt) as u64;
            put(mix, "skank", skank(&sk, 1.0, seed), tb + beat / 2.0, 0.30, -0.35);
            put(mix, "bubble", organ(&ov, 0.07, 1.0), tb + beat / 2.0, 0.20, 0.35);
            put(mix, "bubble", organ(&ov, 0.05, 1.0), tb + 3.0 * s16, 0.11, 0.35);
        }
        if (b.is_multiple_of(2) && dense > 0.6) || b % 8 == 2 {
            put(mix, "kit", synth::crash(1.4), t0, 0.07, 0.0);
        }
        if b % 4 == 1 {
            for (j, m) in [50, 47, 43].into_iter().enumerate() {
                put(mix, "kit", tom(m), t0 + 3.0 * beat + (j as f64 + 1.0) * s16, 0.22, 0.0);
            }
        }
        Ok(())
    }

    /// `band_hit(mix, t, ch, g)`: a unison band hit: kick, crash, snare, a long skank, the organ and the ropes' root.
    fn band_hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64) -> Played {
        let sk = skank_tones(&ctx.harmony, ch).map_err(bad)?;
        let ov = organ_voicing(&ctx.harmony, ch).map_err(bad)?;
        let (r, _) = rope_notes(&ctx.harmony, ch).map_err(bad)?;
        let seed = noise_seed(H, t * 50.0)?;
        put(mix, "kit", kick(), t, 0.5 * g, 0.0);
        put(mix, "kit", synth::crash(1.6), t, 0.10 * g, 0.0);
        put(mix, "kit", snare(), t, 0.25 * g, 0.0);
        put(mix, "skank", skank(&sk, 1.0, seed), t, 0.36 * g, -0.3);
        put(mix, "bubble", organ(&ov, 0.3, 1.0), t, 0.26 * g, 0.3);
        put(
            mix,
            "ropes",
            rope_pluck(r, 1.0, 1.0, noise_seed(H, t * 9.0)?),
            t,
            0.40 * g,
            0.0,
        );
        put(mix, "fx", synth::thud(0.6, 110.0, 36.0, seed + 3), t, 0.15 * g, 0.0);
        Ok(())
    }

    /// The ropes keep a bass going under bar `b`: the root and fifth on every beat (`busy`) or on 1-2 and 3-4.
    fn ropes(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, busy: bool) -> Played {
        let grid = &ctx.show.grid;
        let (r, f5) = rope_notes(&ctx.harmony, chord(ctx, b)?).map_err(bad)?;
        let t0 = b as f64 * grid.bar;
        for bt in 0..4u64 {
            let tb = t0 + bt as f64 * grid.beat;
            if busy {
                let m = if bt % 2 == 0 { r } else { f5 };
                put(mix, "ropes", rope_pluck(m, 0.5, 1.0, bt), tb, 0.26, 0.0);
            } else if bt % 2 == 0 {
                put(mix, "ropes", rope_pluck(r, 0.7, 1.0, bt), tb, 0.34, 0.0);
            } else {
                put(mix, "ropes", rope_pluck(f5, 0.5, 1.0, bt + 1), tb, 0.22, 0.0);
            }
        }
        Ok(())
    }
}

impl Bed for SkaPunkClassic {
    /// A slam lands on a band hit and the thunder (#39's crater); a band hit is a band hit.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// `A.reverb(send, dur=1.6, decay=0.45) * 0.18`.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.6, 0.45, 0.18)
    }

    fn comp(&self) -> (f64, f64) {
        (0.15, 0.62)
    }

    fn roar_gain(&self) -> f64 {
        0.22
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

    /// A big drum hit: a band hit on the chord there and a rumble of thunder.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let ch = ctx.harmony.chord_at(t + 0.01).to_string();
        self.band_hit(ctx, mix, t, &ch, 0.9 * g)?;
        put(mix, "fx", thunder(2.0, 77, false), t, 0.40 * g, 0.0);
        Ok(())
    }

    /// The band hit.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, _ring: f64) -> Played {
        self.band_hit(ctx, mix, t, ch, g)
    }

    /// `A.chime(91, 0.25)` at 0.05.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.05, 0.0);
        Ok(())
    }

    /// A unison band hit under each word of the VS card.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?.to_string();
        for (tw, g) in times.into_iter().zip([0.85, 0.8, 0.8]) {
            self.band_hit(ctx, mix, tw, &ch, g)?;
        }
        Ok(())
    }

    /// VS: the thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, true), t, 0.55, 0.0);
        Ok(())
    }

    /// The band at full level under the card, a snare pickup on beat 4 and a whoosh as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.bar(ctx, mix, 0, 0.7)?;
        for j in 0..4 {
            let g = 0.18 + 0.05 * f64::from(j);
            put(mix, "kit", snare(), 3.0 * grid.beat + f64::from(j) * grid.s16, g, 0.0);
        }
        let (d0, d1) = grid.t_doors;
        let whoosh = synth::whoosh(d1 - d0 + 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", whoosh, py_max(0.0, d0 - 0.05), 0.14, 0.0);
        Ok(())
    }

    /// Bar 1, on the dominant: the band, a band hit on each count, a tom fill into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        self.bar(ctx, mix, 1, 0.4)?;
        let ch = chord(ctx, 1)?.to_string();
        for (tc, _) in &grid.count[..3] {
            self.band_hit(ctx, mix, *tc, &ch, 0.75)?;
        }
        for (j, m) in [50, 47, 45, 43].into_iter().enumerate() {
            put(mix, "kit", tom(m), bar + 3.0 * beat + j as f64 * s16, 0.26, 0.0);
        }
        let whoosh = synth::whoosh(beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", whoosh, bar + 3.0 * beat, 0.12, 0.0);
        Ok(())
    }

    /// FIGHT!: the drop, a band hit on the tonic and a rumble.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        let ch = chord(ctx, 0)?.to_string();
        self.band_hit(ctx, mix, t, &ch, 1.1)?;
        put(mix, "fx", thunder(1.6, 7, false), t, 0.35, 0.0);
        Ok(())
    }

    /// The band from FIGHT! to the K.O., thickening as it goes (fully by #39's bar 24 of 43), a tape stop at the
    /// K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let full = (grid.n_bars as f64 * 22.0 / 43.0).max(1.0);
        for b in grid.fight_bar..=bar_ko {
            let dense = ((b - grid.fight_bar) as f64 / full).min(1.0);
            self.bar(ctx, mix, b, dense)?;
        }
        for bus in ["kit", "skank", "bubble"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a band hit on the tonic and the thunder.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.t_ko;
        let ch = chord(ctx, 0)?.to_string();
        self.band_hit(ctx, mix, t, &ch, 1.0)?;
        put(mix, "fx", thunder(2.4, 51, true), t, 0.55, 0.0);
        Ok(())
    }

    /// A band hit on the tonic under the winner's stinger.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ch = chord(ctx, 0)?.to_string();
        self.band_hit(ctx, mix, tw, &ch, 1.0)
    }

    /// The band holds its breath but never goes quiet: a skank and the organ on every off-beat, hats, the kick on 1
    /// and 3 and the ropes' tonic, until the solo.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, _tw: f64, t_solo: f64) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let ch = chord(ctx, 0)?;
        let sk = skank_tones(&ctx.harmony, ch).map_err(bad)?;
        let ov = organ_voicing(&ctx.harmony, ch).map_err(bad)?;
        let (r, _) = rope_notes(&ctx.harmony, ch).map_err(bad)?;
        let b_solo = (t_solo / bar).round().max(0.0) as usize;
        for b in bar_ko + 1..b_solo {
            let tq = b as f64 * bar;
            for bt in 0..4usize {
                let tb = tq + bt as f64 * beat;
                put(
                    mix,
                    "skank",
                    skank(&sk, 1.0, (b * 4 + bt) as u64),
                    tb + beat / 2.0,
                    0.30,
                    -0.3,
                );
                put(mix, "bubble", organ(&ov, 0.07, 1.0), tb + beat / 2.0, 0.20, 0.3);
                put(mix, "kit", synth::hat(0.03, false), tb, 0.13, 0.0);
            }
            put(mix, "kit", kick(), tq, 0.42, 0.0);
            put(mix, "kit", kick(), tq + 2.0 * beat, 0.36, 0.0);
            put(mix, "ropes", rope_pluck(r, 1.3, 1.0, 1), tq, 0.30, 0.0);
        }
        Ok(())
    }

    /// A lighter band under the solo, the ropes keeping the bass going, up to the last two bars.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let n = ctx.show.grid.n_bars;
        for b in b_solo..n.saturating_sub(2) {
            self.bar(ctx, mix, b, 0.5)?;
            self.ropes(ctx, mix, b, false)?;
        }
        Ok(())
    }

    /// The last two bars: the band returns at its fullest with the ropes on every beat, a snare roll from the last
    /// bar's beat 3 and a riser into the title slam on sample 0.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur, s16) = (grid.beat, grid.bar, grid.dur, grid.s16);
        let n = grid.n_bars;
        for b in n.saturating_sub(2)..n {
            self.bar(ctx, mix, b, 1.0)?;
            self.ropes(ctx, mix, b, true)?;
        }
        let t_last = (n as f64 - 1.0) * bar;
        let mut st = 0.0;
        while t_last + 2.0 * beat + st < dur - 1e-4 {
            put(
                mix,
                "kit",
                snare(),
                t_last + 2.0 * beat + st,
                0.12 + 0.12 * st / (2.0 * beat),
                0.0,
            );
            st += s16;
        }
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.20,
            0.0,
        );
        Ok(())
    }
}
