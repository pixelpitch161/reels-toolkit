//! The dark disco's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/dark_disco.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back (times the velocity) for every later call with the same key. The
//! keys are loose: a duration is rounded to 3 decimals, a clap's noise is keyed by its seed modulo 3 and a hat's by
//! its seed modulo 4. So `clap(seed=7)` sounds like whichever of seeds 1, 4, 7, ... was asked for first, and the
//! cache is part of the music. [`DarkDiscoClassic`] keeps the same cache under the same keys, filled in the same
//! order (one bed per build, as the kit makes one bed per soundtrack).
//!
//! **The mirror ball.** Its glints sit on 16ths hashed by `M.h01(b, g, ...)`, milestone 1's splitmix hash
//! ([`bf_sound::music::h01`]).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{floor_div, py_max, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, h01, noise_seed, riser, thunder};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played, no_hook};

use super::{STAB_DUR, STAB_LO, bass, clap, glint, hat, kick, pad, root, stab};

/// The name the bed's errors carry.
const H: &str = "dark_disco";

/// The bed's own buses, in the order the mood lists them (`for k in self.buses` tape-stops them in it).
const BUSES: [(&str, (f64, f64)); 5] = [
    ("drums", (1.30, 0.10)),
    ("bass", (1.45, 0.04)),
    ("strings", (0.90, 0.30)),
    ("pad", (0.70, 0.35)),
    ("shimmer", (0.45, 0.50)),
];

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` (by its bits;
/// a rounded zero is +0.0, as Python's `-0.0 == 0.0`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('kick', big)`
    Kick(bool),
    /// `('clap', seed % 3)`
    Clap(u64),
    /// `('hat', open_, seed % 4)`
    Hat(bool, u64),
    /// `('bass', int(m), round(dur, 3))`
    Bass(i32, u64),
    /// `('stab', ch, round(dur, 3), lo)`
    Stab(String, u64, i32),
    /// `('pad', ch, round(dur, 3))`
    Pad(String, u64),
    /// `('gl', int(m))`
    Glint(i32),
}

/// `round(x, 3)` as a key.
fn r3(x: f64) -> u64 {
    let r = round_nd(x, 3);
    if r == 0.0 { 0.0f64.to_bits() } else { r.to_bits() }
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn scaled(x: &[f64], v: f64) -> Vec<f64> {
    x.iter().map(|a| a * v).collect()
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

/// The dark disco's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct DarkDiscoClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl DarkDiscoClassic {
    /// The bed, its cache empty.
    pub fn new() -> DarkDiscoClassic {
        DarkDiscoClassic::default()
    }

    /// `self._cache(key, fn) * v`: the sound under `key`, made by `make` the first time, times `v`.
    fn cached(
        &self,
        key: Key,
        v: f64,
        make: impl FnOnce() -> Result<Vec<f64>, MusicError>,
    ) -> Result<Vec<f64>, MusicError> {
        let hit = self.cache.lock().expect("the cache").get(&key).cloned();
        let x = match hit {
            Some(x) => x,
            None => {
                let x = Arc::new(make()?);
                Arc::clone(self.cache.lock().expect("the cache").entry(key).or_insert(x))
            }
        };
        Ok(scaled(&x, v))
    }

    /// `self.kick(v, big)`.
    fn kick(&self, v: f64, big: bool) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Kick(big), v, || Ok(kick(big)))
    }

    /// `self.clap(v, seed)`.
    fn clap(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Clap(seed % 3), v, || Ok(clap(seed)))
    }

    /// `self.hat(v, open_, seed)`.
    fn hat(&self, v: f64, open: bool, seed: u64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Hat(open, seed % 4), v, || Ok(hat(open, seed)))
    }

    /// `self.bass(m, dur, v)`.
    fn bass(&self, m: i32, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Bass(m, r3(dur)), v, || Ok(bass(m, dur)))
    }

    /// `self.stab(ch, v, dur, lo)`.
    fn stab(&self, ctx: &MusicCtx, ch: &str, v: f64, dur: f64, lo: i32) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Stab(ch.to_string(), r3(dur), lo), v, || {
            stab(&ctx.harmony, ch, dur, lo).map_err(bad)
        })
    }

    /// `self.pad(ch, dur, v)`.
    fn pad(&self, ctx: &MusicCtx, ch: &str, dur: f64, v: f64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Pad(ch.to_string(), r3(dur)), v, || {
            pad(&ctx.harmony, ch, dur).map_err(bad)
        })
    }

    /// `self.glint(m, v)`.
    fn glint(&self, m: i32, v: f64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Glint(m), v, || Ok(glint(m)))
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the groove (bar index `b`): the kick on every beat, open hats
    /// on the off-beats, the clap on 2 and 4; the octave bass on the 8ths; string stabs on the and of 2 and on 4;
    /// the pad over the bar; the mirror ball's glints. The back half (`intensity` above 1.25 and not `light`) adds
    /// closed 8th hats, the bass's chromatic pickup into a new chord, off-beat stabs on 1 and 3 and two more glints.
    /// Python's defaults are `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let nb = grid.n_bars;
        let ch = chord(ctx, b % nb)?;
        let nxt = chord(ctx, (b + 1) % nb)?;
        let big = intensity > 1.25 && !light;
        let e8 = beat / 2.0;
        for bt in 0..4u64 {
            let tb = t0 + bt as f64 * beat;
            put(
                mix,
                "drums",
                self.kick(1.0, false)?,
                tb,
                if light { 0.46 } else { 0.55 },
                0.0,
            );
            put(mix, "drums", self.hat(1.0, true, bt)?, tb + e8, 0.075, 0.25);
            if bt == 1 || bt == 3 {
                put(mix, "drums", self.clap(1.0, b as u64 + bt)?, tb, 0.26, -0.05);
            }
        }
        if big {
            for j in (0..16u64).filter(|j| j % 2 == 0) {
                let g = if j % 4 != 0 { 0.035 } else { 0.05 };
                put(mix, "drums", self.hat(1.0, false, j)?, t0 + j as f64 * s16, g, 0.3);
            }
        }
        let r = self.root(ctx, ch)?;
        for j in 0..8usize {
            let mut m = if j % 2 == 0 { r } else { r + 12 };
            if big && j == 7 && nxt != ch {
                let nr = self.root(ctx, nxt)?;
                m = if nr + 12 - 1 > r { nr + 12 - 1 } else { nr + 12 + 1 };
            }
            let g = if j % 2 == 0 { 0.42 } else { 0.34 };
            put(mix, "bass", self.bass(m, e8 * 0.8, 1.0)?, t0 + j as f64 * e8, g, 0.0);
        }
        let stabs: &[f64] = if big { &[1.5, 3.0, 0.5, 2.5] } else { &[1.5, 3.0] };
        for &t in stabs {
            let g = if t == 1.5 || t == 3.0 { 0.20 } else { 0.14 };
            let s = self.stab(ctx, ch, 1.0, STAB_DUR, STAB_LO)?;
            put(mix, "strings", s, t0 + t * beat, g, 0.15);
        }
        put(mix, "pad", self.pad(ctx, ch, bar * 0.98, 1.0)?, t0, 0.12, 0.0);
        let ng = 2 + if big { 2 } else { 0 };
        let bf = b as f64;
        for g in 0..ng {
            let gf = f64::from(g);
            let j = (h01(&[bf, gf, 7.0]) * 16.0) as i32;
            let m = 96 + (h01(&[bf, gf, 11.0]) * 4.0) as i32 * 3;
            let pan = h01(&[bf, gf]) * 1.2 - 0.6;
            put(mix, "shimmer", self.glint(m, 1.0)?, t0 + f64::from(j) * s16, 0.05, pan);
        }
        Ok(())
    }
}

impl Bed for DarkDiscoClassic {
    /// The mood's class defines `hit` and none of the other optional hooks: no `boom` (ruling M3-R2).
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Hit]
    }

    /// `buses = {'drums': (1.30, 0.10), 'bass': (1.45, 0.04), 'strings': (0.90, 0.30), 'pad': (0.70, 0.35),
    /// 'shimmer': (0.45, 0.50)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// A club.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.8, 0.55, 0.16)
    }

    fn comp(&self) -> (f64, f64) {
        (0.18, 0.55)
    }

    fn roar_gain(&self) -> f64 {
        0.15
    }

    fn doors_gain(&self) -> f64 {
        0.40
    }

    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (93.0, 100.0, 0.10)
    }

    /// The class has no `boom`: a part that plays one under this theme is refused before the build.
    fn boom(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: f64) -> Played {
        no_hook(BedHook::Boom)
    }

    /// A full band hit: the big kick, a clap, a long string stab and the bass's low root. The kit's method has no
    /// `ring`: it is not used.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, _ring: f64) -> Played {
        put(mix, "drums", self.kick(1.0, true)?, t, 0.6 * g, 0.0);
        put(
            mix,
            "drums",
            self.clap(1.0, noise_seed(H, t * 10.0)?)?,
            t,
            0.28 * g,
            0.0,
        );
        put(
            mix,
            "strings",
            self.stab(ctx, ch, 1.0, 0.4, STAB_LO)?,
            t,
            0.28 * g,
            0.15,
        );
        let r = self.root(ctx, ch)?;
        put(mix, "bass", self.bass(r, 0.5, 1.0)?, t, 0.5 * g, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.hat(1.0, open_=False, seed=2), t, 0.10)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", self.hat(1.0, false, 2)?, t, 0.10, 0.0);
        Ok(())
    }

    /// A full band hit under each word of the VS card, a crash and a thud.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for tw in times {
            self.hit(ctx, mix, tw, ch, 1.0, 0.0)?;
            put(mix, "drums", synth::crash(1.2), tw, 0.07, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.28, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.40, 0.0);
        Ok(())
    }

    /// The held chord under the card (never quiet): the pad, open hats ticking, the kick keeping the floor, the
    /// bass's octave pickup and a hiss swelling on the shimmer bus.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let ch = chord(ctx, 0)?;
        put(mix, "pad", self.pad(ctx, ch, 2.9 * beat, 1.0)?, 0.05, 0.22, 0.0);
        for bt in 0..4u64 {
            let h = self.hat(1.0, true, bt)?;
            put(mix, "drums", h, bt as f64 * beat + beat / 2.0, 0.08, 0.25);
        }
        put(mix, "drums", self.kick(0.9, false)?, 3.0 * beat, 0.45, 0.0);
        let r = self.root(ctx, ch)?;
        put(
            mix,
            "bass",
            self.bass(r + 12, beat * 0.4, 1.0)?,
            3.0 * beat + beat / 2.0,
            0.34,
            0.0,
        );
        let n = length(3.0 * beat * SRF);
        let swell = dsp::linspace(0.2, 1.0, n);
        let sw = synth::hp(&synth::noise(5, n), 5000.0, FILTER_O)
            .iter()
            .zip(&swell)
            .map(|(x, l)| x * (l * l))
            .collect();
        put(mix, "shimmer", sw, 0.0, 0.03, 0.0);
        Ok(())
    }

    /// Bar 1: the groove on the count-in chord (pushed, intensity 1.3) and a string stab on each count.
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.bar(ctx, mix, 1, 1.3, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &ctx.show.grid.count[..3] {
            put(mix, "strings", self.stab(ctx, ch, 1.0, 0.22, STAB_LO)?, *tc, 0.18, 0.15);
        }
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.32, 0.0);
        Ok(())
    }

    /// The groove from FIGHT! to the K.O. (thicker as it goes), crashes on the phrase lines, a riser and a crash
    /// into the back half, a tape stop of the bed's buses at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let bar = ctx.show.grid.bar;
        let t_ko = ctx.show.t_ko;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(4) {
            put(mix, "drums", synth::crash(1.4), b as f64 * bar, 0.07, 0.0);
        }
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        if t_half < t_ko {
            let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
            put(mix, "fx", rise, t_half - bar, 0.08, 0.0);
            put(mix, "drums", synth::crash(2.0), t_half, 0.12, 0.0);
        }
        for (bus, _) in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "drums", synth::crash(2.4), t_ko, 0.16, 0.0);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.0)?;
        put(mix, "drums", synth::crash(1.8), tw, 0.10, 0.0);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo, and the groove (intensity 1.2) in the bars between.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let bar = ctx.show.grid.bar;
        let dur = py_max(0.2, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.08,
            0.0,
        );
        // range(int(tw // T.BAR) + 1, int(round(t_solo / T.BAR)))
        let from = floor_div(tw, bar) as i64 + 1;
        let to = round_half_even(t_solo / bar) as i64;
        for b in from..to {
            let b = usize::try_from(b).map_err(|_| MusicError::new(H, format!("bar {b} before the start")))?;
            self.bar(ctx, mix, b, 1.2, false)?;
        }
        Ok(())
    }

    /// The groove at full density under the solo, up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.4, false)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: the kick on every beat, the octave bass climbing in 8ths
    /// to the tonic, stabs on every off-beat, the pad's filter sweeping open, a riser right up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = ctx.harmony.chord_at(t_last);
        let tgt = self.root(ctx, chord(ctx, 0)?)?;
        let e8 = beat / 2.0;
        for bt in 0..4u64 {
            let tb = t_last + bt as f64 * beat;
            put(mix, "drums", self.kick(1.0, false)?, tb, 0.55, 0.0);
            put(mix, "drums", self.hat(1.0, true, bt)?, tb + e8, 0.08, 0.25);
            if bt == 1 || bt == 3 {
                put(mix, "drums", self.clap(1.0, bt)?, tb, 0.26, 0.0);
            }
            put(
                mix,
                "strings",
                self.stab(ctx, ch, 1.0, STAB_DUR, STAB_LO)?,
                tb + e8,
                0.18,
                0.15,
            );
        }
        let walk = [tgt - 5, tgt + 7, tgt - 4, tgt + 8, tgt - 3, tgt + 9, tgt - 1, tgt + 11];
        for (j, &m) in walk.iter().enumerate() {
            put(
                mix,
                "bass",
                self.bass(m, e8 * 0.8, 1.0)?,
                t_last + j as f64 * e8,
                0.40,
                0.0,
            );
        }
        put(
            mix,
            "pad",
            self.pad(ctx, ch, dur - t_last + 0.012, 1.0)?,
            t_last,
            0.18,
            0.0,
        );
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.18,
            0.0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_keys_are_the_kits() {
        assert_eq!(r3(0.1234), r3(0.1226));
        assert_eq!(r3(-0.0001), r3(0.0));
        // a clap keyed by seed % 3 is the first seed's noise; a hat by seed % 4
        let bed = DarkDiscoClassic::new();
        let first = bed.clap(1.0, 7).unwrap();
        assert_eq!(first, clap(7));
        assert_eq!(bed.clap(0.5, 1).unwrap(), scaled(&first, 0.5));
        assert_ne!(clap(1), clap(7));
        let h = bed.hat(1.0, true, 6).unwrap();
        assert_eq!(bed.hat(1.0, true, 2).unwrap(), h);
        assert_ne!(bed.hat(1.0, false, 2).unwrap(), h);
    }

    #[test]
    fn it_has_hit_and_no_boom() {
        let bed = DarkDiscoClassic::new();
        assert!(bed.has(BedHook::Hit) && !bed.has(BedHook::Boom));
    }
}
