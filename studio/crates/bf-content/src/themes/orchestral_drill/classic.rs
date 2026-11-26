//! The orchestral drill's classic arrangement: the bed of `projects/ballfight_viola_vs_violin/audio.py`'s `main()`
//! (the parts that belong to the bed, not to VIOLA or VIOLIN), section by section as the score calls them. A quick
//! port: by ear, not sample for sample. The cellos are the CELLO's Pixel Pitch voice through the music context's
//! `note`, `note('CELLO', m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))`.

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_SEED, riser};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{CELLO, CELLO_PAT, HAT_ROLLS, P808, clap, contrabass, crack, hat, rim, root_808, sub808};

/// The name the bed's errors carry.
const H: &str = "orchestral_drill";

/// The bed's own buses, which the K.O. tape-stops.
const BUSES: [&str; 5] = ["drums", "hats", "bass808", "cellos", "basses"];

/// The orchestral drill's bed.
#[derive(Clone, Debug, Default)]
pub struct OrchestralDrillClassic;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    chords
        .get(b % chords.len().max(1))
        .map(String::as_str)
        .ok_or_else(|| MusicError::new(H, format!("bar {b} has no chord ({} bars)", chords.len())))
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// `A.kick(punch, dec, f0, f1)[:cut]`.
fn kick(punch: f64, dec: f64, f0: f64, f1: f64, cut: f64) -> Vec<f64> {
    let mut k = synth::kick(punch, dec, f0, f1);
    k.truncate((cut * bf_sound::SRF) as usize);
    k
}

/// `cello(m, dur, vel)`: one note of the CELLO's voice.
fn cello(ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
    let x = ctx.note(
        CELLO,
        f64::from(m),
        round_nd(dur, 3),
        round_nd(dur + 0.2, 2),
        round_nd(vel, 1),
    )?;
    Ok(x.to_vec())
}

/// Two cello notes at once, the second at `k`.
fn cello2(ctx: &MusicCtx, a: i32, b: i32, dur: f64, k: f64) -> Result<Vec<f64>, MusicError> {
    let (x, y) = (cello(ctx, a, dur, 1.0)?, cello(ctx, b, dur, 1.0)?);
    Ok((0..x.len().max(y.len()))
        .map(|i| x.get(i).unwrap_or(&0.0) + k * y.get(i).unwrap_or(&0.0))
        .collect())
}

impl OrchestralDrillClassic {
    /// The bed.
    pub fn new() -> OrchestralDrillClassic {
        OrchestralDrillClassic
    }

    /// `bed(mix, b, density, light)`: one bar of the drill beat: the 808 and the kick, the half-time clap on beat
    /// 3, rim clicks, skittering hats, staccato cellos (and the basses' long notes in the back half).
    fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, density: f64, light: bool) -> Played {
        let g = &ctx.show.grid;
        let s16 = g.s16;
        let t0 = b as f64 * g.bar;
        let ch = chord(ctx, b)?;
        let nxt = chord(ctx, b + 1)?;
        let (root, nroot) = (root_808(ch).map_err(bad)?, root_808(nxt).map_err(bad)?);
        for (p, ln, deg, sl) in P808 {
            let m0 = root + deg;
            let m1 = if sl && p >= 14 {
                nroot
            } else if sl {
                root + if p == 6 { 12 } else { 0 }
            } else {
                m0
            };
            let x = sub808(
                m0,
                m1,
                ln as f64 * s16,
                if sl { 0.08 } else { 0.0 },
                if p == 0 { 1.0 } else { 0.8 },
            );
            put(mix, "bass808", x, t0 + p as f64 * s16, 0.62, 0.0);
        }
        put(mix, "drums", kick(0.9, 0.18, 140.0, 55.0, 0.3), t0, 0.48, 0.0);
        put(
            mix,
            "drums",
            kick(0.6, 0.14, 130.0, 55.0, 0.25),
            t0 + 10.0 * s16,
            0.30,
            0.0,
        );
        put(mix, "drums", clap(1.0), t0 + 8.0 * s16, 0.34, 0.0);
        if density > 1.2 && !light {
            put(mix, "drums", clap(0.5), t0 + 15.0 * s16, 0.12, 0.0);
        }
        let rolls = density >= 0.9 && !light;
        for p in (0..16usize).step_by(2) {
            if rolls && HAT_ROLLS.iter().any(|(r, _)| *r == p + 1) {
                continue;
            }
            put(
                mix,
                "hats",
                hat(if p % 4 == 0 { 0.8 } else { 0.6 }, false),
                t0 + p as f64 * s16,
                0.09,
                0.25,
            );
        }
        if rolls {
            for (p, trip) in HAT_ROLLS {
                let at = t0 + (p as f64 - 1.0) * s16;
                if trip {
                    for j in 0..3 {
                        let t = at + j as f64 * (2.0 * s16 / 3.0);
                        put(mix, "hats", hat(0.5 + 0.15 * j as f64, false), t, 0.08, 0.25);
                    }
                } else {
                    for j in 0..4 {
                        let t = at + j as f64 * (s16 / 2.0);
                        put(mix, "hats", hat(0.45 + 0.1 * j as f64, false), t, 0.07, 0.25);
                    }
                }
            }
        }
        put(mix, "drums", rim(0.8, b as u64), t0 + 6.0 * s16, 0.06, -0.3);
        put(mix, "drums", rim(0.6, b as u64 + 50), t0 + 13.0 * s16, 0.05, -0.3);
        let tones = ctx.harmony.chord_tones(36, 55, ch).map_err(bad)?;
        if !tones.is_empty() {
            for (p, j) in CELLO_PAT {
                if light && p != 0 && p != 8 {
                    continue;
                }
                let m = tones[(j + usize::from(p >= 8)) % tones.len()];
                let dur = if p % 4 != 0 { 0.09 } else { 0.13 };
                put(
                    mix,
                    "cellos",
                    cello(ctx, m, dur, 0.85)?,
                    t0 + p as f64 * s16,
                    0.17,
                    -0.2,
                );
                if density > 1.25 {
                    put(
                        mix,
                        "cellos",
                        cello(ctx, m + 12, 0.08, 0.7)?,
                        t0 + p as f64 * s16,
                        0.09,
                        0.2,
                    );
                }
            }
        }
        if density > 1.1 || light {
            put(mix, "basses", contrabass(root + 12, g.bar - 0.06, 0.8), t0, 0.12, 0.0);
        }
        Ok(())
    }

    /// The cellos' low root and fifth for `chord` (C2 and G2 in C minor).
    fn low_pair(&self, ctx: &MusicCtx, ch: &str) -> Result<(i32, i32), MusicError> {
        let r = ctx.harmony.root_above(ch, 36).map_err(bad)?;
        Ok((r, r + 7))
    }
}

impl Bed for OrchestralDrillClassic {
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// `BUS_GAIN` and the reverb sends of the episode's master, for the bed's buses.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[
            ("drums", (1.0, 0.1)),
            ("hats", (1.0, 0.0)),
            ("bass808", (1.0, 0.0)),
            ("cellos", (1.1, 0.35)),
            ("basses", (1.0, 0.0)),
        ]
    }

    /// A concert-hall arena.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.2, 0.55, 0.20)
    }

    fn comp(&self) -> (f64, f64) {
        (0.15, 0.62)
    }

    fn roar_gain(&self) -> f64 {
        0.14
    }

    fn doors_gain(&self) -> f64 {
        0.36
    }

    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 100.0, 0.10)
    }

    /// A big hit (a crush, a slam): the kick, a deep 808 and a crash.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let ch = ctx.harmony.chord_at(t + 0.01).to_string();
        let r = root_808(&ch).map_err(bad)?;
        put(mix, "drums", kick(1.0, 0.25, 150.0, 48.0, 0.4), t, 0.5 * g, 0.0);
        put(mix, "bass808", sub808(r - 12, r - 12, 0.9, 0.0, 1.0), t, 0.6 * g, 0.0);
        put(mix, "drums", synth::crash(2.0), t, 0.12 * g, 0.0);
        Ok(())
    }

    /// The two balls clashing: a small high bell.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", synth::chime(91.0, 0.25), t, 0.04, 0.0);
        Ok(())
    }

    /// One unison band hit under every word of the VS card: the kick, a crash, the cellos' low root and fifth, a
    /// thud.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let (r, f) = self.low_pair(ctx, chord(ctx, 0)?)?;
        let stab = cello2(ctx, r, f, 0.3, 0.8)?;
        for tw in times {
            put(mix, "drums", kick(1.0, 0.25, 150.0, 48.0, 0.4), tw, 0.50, 0.0);
            put(mix, "drums", synth::crash(1.0), tw, 0.05, 0.0);
            put(mix, "cellos", stab.clone(), tw, 0.24, 0.0);
            let seed = (tw * 50.0).max(0.0) as u64 + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.26, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", crack(), t, 0.32, 0.0);
        Ok(())
    }

    /// The drill beat is full from frame 0 (the levels rule), a whoosh as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.bar(ctx, mix, 0, 1.0, false)?;
        let (d0, d1) = ctx.show.grid.t_doors;
        let w = synth::whoosh(d1 - d0 + 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, d0 - 0.05, 0.12, 0.0);
        Ok(())
    }

    /// The count-in on the second chord: a kick and a cello on each count, climbing the chord, a hat roll and a
    /// whoosh into the drop.
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        self.bar(ctx, mix, 1, 1.0, false)?;
        let climb = ctx.harmony.chord_tones(43, 55, chord(ctx, 1)?).map_err(bad)?;
        for (j, (tc, _)) in g.count.iter().take(3).enumerate() {
            put(mix, "drums", kick(0.9, 0.2, 150.0, 50.0, 0.35), *tc, 0.40, 0.0);
            if let Some(&m) = climb.get(j).or(climb.last()) {
                put(mix, "cellos", cello(ctx, m, 0.18, 1.0)?, *tc, 0.26, 0.0);
            }
        }
        let t4 = g.bar + 3.0 * g.beat;
        for j in 0..8 {
            put(
                mix,
                "hats",
                hat(0.5 + 0.06 * j as f64, false),
                t4 + j as f64 * g.s16 / 2.0,
                0.10,
                0.25,
            );
        }
        let w = synth::whoosh(g.beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, t4, 0.12, 0.0);
        Ok(())
    }

    /// FIGHT!: the drop, a crash and a thud.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t, 0.10, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, synth::THUD_SEED), t, 0.30, 0.0);
        Ok(())
    }

    /// The drill beat from FIGHT! to the K.O., thickening as it goes, a crash on every 8-bar phrase, a riser into
    /// the middle, the tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let g = &ctx.show.grid;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko.max(1) as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(8) {
            put(mix, "drums", synth::crash(1.6), b as f64 * g.bar, 0.07, 0.0);
        }
        let mid = g.n_bars / 2;
        let t_mid = mid as f64 * g.bar;
        if mid > 2 && t_mid < ctx.show.t_ko {
            let r: Vec<f64> = riser(g.bar, 0.0, RISER_SEED + 1)?.iter().map(|v| v * 0.6).collect();
            put(mix, "fx", r, t_mid - g.bar, 0.07, 0.0);
            put(mix, "drums", synth::crash(2.0), t_mid, 0.10, 0.0);
        }
        let t_ko = ctx.show.t_ko;
        for bus in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    /// The K.O.: a long crash and a deep thud.
    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "drums", synth::crash(2.4), t_ko, 0.14, 0.0);
        put(
            mix,
            "fx",
            synth::thud(0.9, 120.0, 30.0, synth::THUD_SEED),
            t_ko,
            0.55,
            0.0,
        );
        Ok(())
    }

    /// Under the winner's stinger: the cellos' low root and fifth, the 808 and a crash.
    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        let ch = chord(ctx, 0)?;
        let (r, f) = self.low_pair(ctx, ch)?;
        put(mix, "cellos", cello2(ctx, r, f, 0.5, 0.8)?, tw, 0.24, 0.0);
        let r8 = root_808(ch).map_err(bad)?;
        put(mix, "bass808", sub808(r8, r8, 0.8, 0.0, 1.0), tw, 0.6, 0.0);
        put(mix, "drums", synth::crash(1.8), tw, 0.09, 0.0);
        Ok(())
    }

    fn after_ko(&self, _: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        let w = synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, tw + 0.1, 0.08, 0.0);
        Ok(())
    }

    /// A lighter drill beat under the solo, up to the last two bars.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(2) {
            self.bar(ctx, mix, b, 1.0, true)?;
        }
        Ok(())
    }

    /// The last two bars: the beat comes back and builds, the cellos climb the last chord into the title slam on
    /// sample 0, a hat roll speeding into the seam, a riser.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let g = &ctx.show.grid;
        let n = g.n_bars;
        if n >= 2 {
            self.bar(ctx, mix, n - 2, 1.3, false)?;
        }
        let b_last = n.saturating_sub(1);
        self.bar(ctx, mix, b_last, 1.5, false)?;
        let t_last = b_last as f64 * g.bar;
        let climb = ctx.harmony.chord_tones(43, 60, chord(ctx, b_last)?).map_err(bad)?;
        for (bt, &m) in climb.iter().take(4).enumerate() {
            put(
                mix,
                "cellos",
                cello(ctx, m, 0.3, 1.0)?,
                t_last + bt as f64 * g.beat,
                0.26,
                0.0,
            );
        }
        for j in 0..16 {
            let t = t_last + 2.0 * g.beat + j as f64 * g.s16 / 2.0;
            put(mix, "hats", hat(0.4 + 0.035 * j as f64, false), t, 0.10, 0.25);
        }
        put(mix, "fx", riser(g.dur - t_last, 0.012, RISER_SEED)?, t_last, 0.16, 0.0);
        Ok(())
    }
}
