//! The baroque storm's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/baroque_storm.py`,
//! section by section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a
//! bus in that order).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_min, round_half_even, round_nd};
use bf_core::show::{CAP_GAP, captions};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth;
use bf_sound::traits::{Bed, BedHook, Played};

use super::{STRNG_ATT, STRNG_VIB, harp_rh, harpsi, punch, root, run_offset, strng, timp, vln};

/// The name the bed's errors carry.
const H: &str = "baroque_storm";

/// `build_bars`: the bars of the build into the title slam.
const BUILD_BARS: usize = 2;

/// A key of the kit's sound cache `self._bc`: `('h', m, round(dur, 2))`, `('s', m, round(dur, 3), cut,
/// round(vib, 3), round(att, 3))` and `('t', m)`, the floats by their bits (`-0.0` as `0.0`, which Python's dict
/// takes for the same key).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Harpsi(i32, u64),
    Strng(i32, u64, u64, u64, u64),
    Timp(i32),
}

fn bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

/// The baroque storm's bed as the kit plays it.
///
/// Like the kit's class it makes each harpsichord string, string note and timpani stroke once per key of its cache
/// (the key rounds the length, as the kit's does: a later call whose key matches gets the first call's sound) and
/// scales the cached sound by the velocity. The modern punch (`A.hp(A.kick(...))`, the same sound on every call and
/// no noise) is made once.
#[derive(Debug)]
pub struct BaroqueStormClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
    punch: Vec<f64>,
}

impl Default for BaroqueStormClassic {
    fn default() -> BaroqueStormClassic {
        BaroqueStormClassic::new()
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

/// `T.CHORDS[-1]`.
fn last_chord<'a>(ctx: &'a MusicCtx) -> Result<&'a str, MusicError> {
    match ctx.harmony.chords.len() {
        0 => Err(MusicError::new(H, "the episode has no chords")),
        n => chord(ctx, n - 1),
    }
}

/// `self.root(ch)`.
fn root_of(ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
    root(&ctx.harmony, ch).map_err(bad)
}

/// `T.chord_tones(lo, hi, ch)`.
fn tones(ctx: &MusicCtx, lo: i32, hi: i32, ch: &str) -> Result<Vec<i32>, MusicError> {
    ctx.harmony.chord_tones(lo, hi, ch).map_err(bad)
}

/// `x * vel` for every sample: the cache's sound at a velocity.
fn scaled(x: &[f64], vel: f64) -> Vec<f64> {
    x.iter().map(|v| v * vel).collect()
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// `x[i]`, or the error Python's `IndexError` would be.
fn at(x: &[i32], i: usize, what: &str) -> Result<i32, MusicError> {
    x.get(i)
        .copied()
        .ok_or_else(|| MusicError::new(H, format!("{what} has {} notes, not {}", x.len(), i + 1)))
}

impl BaroqueStormClassic {
    /// The bed, with nothing cached yet.
    pub fn new() -> BaroqueStormClassic {
        BaroqueStormClassic {
            cache: Mutex::new(BTreeMap::new()),
            punch: punch(),
        }
    }

    /// The cached sound for `key`, made by `make` the first time.
    fn cached(&self, key: Key, make: impl FnOnce() -> Vec<f64>) -> Arc<Vec<f64>> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(cache.entry(key).or_insert_with(|| Arc::new(make())))
    }

    /// `self.harpsi(m, dur, vel)`.
    fn harpsi(&self, m: i32, dur: f64, vel: f64) -> Vec<f64> {
        let x = self.cached(Key::Harpsi(m, bits(round_nd(dur, 2))), || harpsi(m, dur));
        scaled(&x, vel)
    }

    /// `self.strng(m, dur, vel, cut, vib, att)`.
    fn strng(&self, m: i32, dur: f64, vel: f64, cut: f64, vib: f64, att: f64) -> Vec<f64> {
        let key = Key::Strng(
            m,
            bits(round_nd(dur, 3)),
            bits(cut),
            bits(round_nd(vib, 3)),
            bits(round_nd(att, 3)),
        );
        let x = self.cached(key, || strng(m, dur, cut, vib, att));
        scaled(&x, vel)
    }

    /// `self.strng(m, dur, vel, cut)` with the default vibrato and attack.
    fn bowed(&self, m: i32, dur: f64, vel: f64, cut: f64) -> Vec<f64> {
        self.strng(m, dur, vel, cut, STRNG_VIB, STRNG_ATT)
    }

    /// `self.timp(m, vel)`.
    fn timp(&self, m: i32, vel: f64) -> Vec<f64> {
        let x = self.cached(Key::Timp(m), || timp(m));
        scaled(&x, vel)
    }

    /// `Bed.bar(mix, b, dense, light)`: one bar of the storm (bar index `b`, its chord `T.CHORDS[b % T.N_BARS]`).
    /// `dense` from 0 to 1 thickens the arrangement (violin II, violas, 16th arpeggios, timpani on 3), never the
    /// level; a `light` bar (under the solo) drops the bowed bass, every other 16th of violin I and the chops.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony cannot voice.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, dense: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let n_bars = grid.n_bars;
        if n_bars == 0 {
            return Err(MusicError::new(H, "the episode has no bars"));
        }
        let ch = chord(ctx, b % n_bars)?;
        let root = root_of(ctx, ch)?;
        // the basses: the D1 to A1 region (and the octave)
        let lo = if root >= 43 { root - 12 } else { root };
        // the continuo and the cellos and basses: driving 8ths
        for e8 in 0..8usize {
            let te = t0 + e8 as f64 * beat / 2.0;
            let m = lo + if e8 % 2 == 1 { 12 } else { 0 };
            put(mix, "harpsi", self.harpsi(m, 0.5, 1.0), te, 0.16, -0.15);
            if !light {
                put(mix, "strings", self.bowed(lo, 0.17, 1.0, 900.0), te, 0.10, 0.1);
                put(mix, "strings", self.bowed(lo + 12, 0.17, 0.8, 1200.0), te, 0.08, 0.1);
            }
        }
        // the harpsichord's right hand: chords on 1 and 3
        let rh = harp_rh(&ctx.harmony, ch).map_err(bad)?;
        for bt in [0usize, 2] {
            for (j, &m) in rh.iter().enumerate() {
                let t = t0 + bt as f64 * beat + j as f64 * 0.008;
                put(mix, "harpsi", self.harpsi(m + 12, 0.6, 1.0), t, 0.07, 0.2);
            }
        }
        // violin I: the racing 16ths
        let cell = vln(&ctx.harmony, ch).map_err(bad)?;
        for st in 0..16usize {
            if light && st % 2 == 1 {
                continue;
            }
            let down = if (st / 4 == 1 || st / 4 == 3) && st % 4 == 2 {
                12
            } else {
                0
            };
            let m = at(&cell, st % 4, "the violins' cell")? - down;
            let acc = if st % 4 == 0 { 1.0 } else { 0.72 };
            let t = t0 + st as f64 * s16;
            put(mix, "strings", self.bowed(m, 0.075, acc, 2600.0), t, 0.075, -0.3);
            if dense > 0.35 && !light {
                // violin II: a third or a sixth under
                let m2 = tones(ctx, m - 9, m - 3, ch)?.into_iter().max().unwrap_or(m - 12);
                put(mix, "strings", self.bowed(m2, 0.075, acc * 0.9, 2300.0), t, 0.055, 0.35);
            }
        }
        if dense > 0.6 && !light {
            // the violas: a trembling chord (32nds)
            let mut violas = tones(ctx, 57, 69, ch)?;
            violas.truncate(2);
            for s32 in 0..32usize {
                let vel = 0.6 + 0.4 * if s32 % 2 == 0 { 1.0 } else { 0.0 };
                for &m in &violas {
                    let t = t0 + s32 as f64 * s16 / 2.0;
                    put(mix, "strings", self.bowed(m, 0.045, vel, 1800.0), t, 0.028, 0.0);
                }
            }
        }
        if dense > 0.5 && !light {
            // 16th arpeggios on the harpsichord
            for st in 0..16usize {
                let m = at(&rh, st % 4, "the right hand")? + 12 * ((st / 8 % 2) as i32);
                put(
                    mix,
                    "harpsi",
                    self.harpsi(m, 0.3, 1.0),
                    t0 + st as f64 * s16,
                    0.045,
                    0.3,
                );
            }
        }
        let low = if root > 40 { root - 12 } else { root };
        put(
            mix,
            "drums",
            self.timp(low, 1.0),
            t0,
            if light { 0.3 } else { 0.40 },
            0.0,
        );
        if dense > 0.45 || light {
            let fifth = if root - 5 >= 33 { root - 5 } else { root + 7 };
            put(mix, "drums", self.timp(fifth, 1.0), t0 + 2.0 * beat, 0.30, 0.0);
        }
        // the modern punch under
        put(mix, "drums", self.punch.clone(), t0, 0.22, 0.0);
        put(mix, "drums", self.punch.clone(), t0 + 2.0 * beat, 0.18, 0.0);
        if !light {
            // a snare-ish string chop on 2 and 4
            for bt in [1usize, 3] {
                let m = at(&tones(ctx, 62, 74, ch)?, 0, "the chop's chord")?;
                let chop = synth::hp(&self.bowed(m, 0.06, 1.0, 3500.0), 200.0, synth::FILTER_O);
                put(mix, "strings", chop, t0 + bt as f64 * beat, 0.07, 0.0);
            }
        }
        Ok(())
    }

    /// `Bed.tutti(mix, t, ch, g, dur)`: a storm hit on the chord `ch`: strings, harpsichord, timpani and a low
    /// thunder thud. The kit's defaults are `g = 1.0` and `dur = 0.25`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony cannot voice, a time below zero (the thud's seed).
    pub fn tutti(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, dur: f64) -> Played {
        for m in tones(ctx, 50, 77, ch)? {
            put(
                mix,
                "strings",
                self.strng(m, dur, 1.0, 3000.0, STRNG_VIB, 0.004),
                t,
                0.06 * g,
                0.0,
            );
        }
        for m in harp_rh(&ctx.harmony, ch).map_err(bad)? {
            put(mix, "harpsi", self.harpsi(m, 0.8, 1.0), t, 0.08 * g, 0.0);
            put(mix, "harpsi", self.harpsi(m + 12, 0.8, 1.0), t, 0.05 * g, 0.0);
        }
        let root = root_of(ctx, ch)?;
        let low = if root > 40 { root - 12 } else { root };
        put(mix, "drums", self.timp(low, 1.0), t, 0.6 * g, 0.0);
        // A.thud(0.8, 110, 32, seed=int(t * 50) + 3)
        let seed = noise_seed(H, t * 50.0)? + 3;
        put(mix, "fx", synth::thud(0.8, 110.0, 32.0, seed), t, 0.35 * g, 0.0);
        put(mix, "drums", synth::crash(1.2), t, 0.05 * g, 0.0);
        Ok(())
    }
}

impl Bed for BaroqueStormClassic {
    /// The mood's class defines `boom` and none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// `buses = {'strings': (1.6, 0.45), 'harpsi': (1.3, 0.3), 'drums': (1.0, 0.15)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("strings", (1.6, 0.45)), ("harpsi", (1.3, 0.3)), ("drums", (1.0, 0.15))]
    }

    fn reverb(&self) -> (f64, f64, f64) {
        (2.2, 0.5, 0.22)
    }

    fn comp(&self) -> (f64, f64) {
        (0.20, 0.5)
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

    /// A timpani stroke on the first chord's root.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let m = root_of(ctx, chord(ctx, 0)?)?;
        put(mix, "drums", self.timp(m, 1.0), t, g, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let thud = synth::thud(0.15, 200.0, 90.0, noise_seed(H, t * 3.0)?);
        put(mix, "fx", thud, t, 0.06, 0.0);
        Ok(())
    }

    /// The storm at full level on the card, a tutti under each word.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let tonic = chord(ctx, 0)?;
        self.bar(ctx, mix, 0, 0.7, false)?;
        for (tw, g) in times.into_iter().zip([0.8, 0.75, 0.75]) {
            self.tutti(ctx, mix, tw, tonic, g, 0.25)?;
        }
        Ok(())
    }

    /// The thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.55, 0.0);
        Ok(())
    }

    /// Beat 4: a string run up as the doors open (written in D, moved to the episode's key).
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let s16 = ctx.show.grid.s16;
        let off = run_offset(ctx.harmony.key_pc);
        for (st, m) in (12..16usize).zip([69, 72, 74, 77]) {
            put(
                mix,
                "strings",
                self.bowed(m + off, 0.08, 1.0, 3000.0),
                st as f64 * s16,
                0.12,
                0.0,
            );
        }
        Ok(())
    }

    /// Bar 1 on the second chord, a tutti on each count, and the run into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.bar(ctx, mix, 1, 0.4, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &grid.count[..3] {
            self.tutti(ctx, mix, *tc, ch, 0.8, 0.25)?;
        }
        let off = run_offset(ctx.harmony.key_pc);
        for (st, m) in (12..16usize).zip([69, 73, 76, 81]) {
            let t = grid.bar + st as f64 * grid.s16;
            put(mix, "strings", self.bowed(m + off, 0.08, 1.0, 3200.0), t, 0.12, 0.0);
        }
        Ok(())
    }

    /// The drop: a tutti on the loop's first chord and a thunder.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        let ch = chord(ctx, ctx.harmony.loop_start)?;
        self.tutti(ctx, mix, t_fight, ch, 1.1, 0.25)?;
        put(mix, "fx", thunder(1.8, 7, THUNDER_CRACK)?, t_fight, 0.40, 0.0);
        Ok(())
    }

    /// The storm from FIGHT! to the K.O., thicker bar by bar; a thunder under every captioned ability (the story's
    /// captions: `M.P.STORY['caps']`); a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in 2..=bar_ko {
            // min(1.0, (b - 2) / 16.0)
            let dense = py_min(1.0, (b as f64 - 2.0) / 16.0);
            self.bar(ctx, mix, b, dense, false)?;
        }
        for (t, _, _) in captions(&ctx.rec.events, CAP_GAP) {
            let rumble = thunder(1.6, noise_seed(H, t * 10.0)?, false)?;
            put(mix, "fx", rumble, ctx.q16(t), 0.35, 0.0);
        }
        for bus in ["strings", "harpsi", "drums"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        self.tutti(ctx, mix, t_ko, chord(ctx, 0)?, 1.0, 0.5)?;
        put(mix, "fx", thunder(2.4, 51, THUNDER_CRACK)?, t_ko, 0.55, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.tutti(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.6)
    }

    /// The storm holds its breath but never goes quiet: trembling strings and timpani until the solo.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, _tw: f64, t_solo: f64) -> Played {
        let grid = &ctx.show.grid;
        let tonic = chord(ctx, 0)?;
        let r = root_of(ctx, tonic)?;
        // range(bar_ko + 1, int(round(t_solo / T.BAR)))
        let end = round_half_even(t_solo / grid.bar);
        let end = if end > 0.0 { end as usize } else { 0 };
        for b in bar_ko + 1..end {
            let tq = b as f64 * grid.bar;
            for m in tones(ctx, 50, 69, tonic)? {
                for s32 in 0..32usize {
                    let t = tq + s32 as f64 * grid.s16 / 2.0;
                    put(mix, "strings", self.bowed(m, 0.045, 0.8, 2200.0), t, 0.035, 0.0);
                }
            }
            put(mix, "drums", self.timp(r, 1.0), tq, 0.4, 0.0);
            put(mix, "drums", self.timp(r + 7, 1.0), tq + 2.0 * grid.beat, 0.3, 0.0);
        }
        Ok(())
    }

    /// Light bars under the solo, a held string chord on each, up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        let grid = &ctx.show.grid;
        for b in b_solo..grid.n_bars.saturating_sub(BUILD_BARS) {
            self.bar(ctx, mix, b, 0.5, true)?;
            for m in tones(ctx, 50, 69, chord(ctx, b)?)? {
                let held = self.strng(m, grid.bar, 0.7, 1800.0, 0.004, 0.08);
                put(mix, "strings", held, b as f64 * grid.bar, 0.08, 0.0);
            }
        }
        Ok(())
    }

    /// The storm returns for the last bars and builds into the title slam: timpani 8ths, a roll into the seam and
    /// a riser.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        for b in grid.n_bars.saturating_sub(BUILD_BARS)..grid.n_bars {
            self.bar(ctx, mix, b, 1.0, false)?;
        }
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let dom = root_of(ctx, last_chord(ctx)?)?;
        for bt in 0..4usize {
            let bt = bt as f64;
            put(mix, "drums", self.timp(dom, 1.0), t_last + bt * beat, 0.36, 0.0);
            put(mix, "drums", self.timp(dom, 1.0), t_last + (bt + 0.5) * beat, 0.24, 0.0);
        }
        // a timpani roll into the seam
        let mut st = 0.0;
        while t_last + 2.0 * beat + st < dur - 1e-4 {
            put(mix, "drums", self.timp(dom, 0.6), t_last + 2.0 * beat + st, 0.20, 0.0);
            st += grid.s16;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_keys_round_as_the_kits() {
        let bed = BaroqueStormClassic::new();
        // ('h', m, round(dur, 2)): 0.501 and 0.5 are one key, the first call's sound
        let a = bed.harpsi(50, 0.5, 1.0);
        assert_eq!(bed.harpsi(50, 0.501, 1.0), a);
        assert_eq!(a.len(), 24000);
        // the velocity scales the cached sound
        let soft = bed.harpsi(50, 0.5, 0.5);
        assert!(a.iter().zip(&soft).all(|(x, y)| (x * 0.5).to_bits() == y.to_bits()));
        // the made-once sound is the sound made anew
        assert_eq!(bed.timp(38, 1.0), timp(38));
        assert_eq!(
            bed.bowed(62, 0.075, 1.0, 2600.0),
            strng(62, 0.075, 2600.0, STRNG_VIB, STRNG_ATT)
        );
        assert_eq!(bed.cache.lock().unwrap().len(), 3);
    }
}
