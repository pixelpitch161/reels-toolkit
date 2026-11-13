//! The prog duel's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/prog_duel.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps the kick, each snare, each tom and each bass note in a dict
//! (`self._cache`) under a key made when it is first asked for. The kick, snares and toms are kept at velocity 1 and
//! scaled by each call's velocity; a bass note is kept with the velocity of its first call and handed back as it is.
//! The keys are loose: a snare's noise is keyed by its seed modulo 4, and a bass note by `(int(m), round(dur, 3),
//! round(vel, 2), round(drive, 2))`, so a later call whose velocity rounds the same (`0.7 + 0.2` is
//! 0.8999999999999999, keyed as 0.9) gets the first call's sound. [`ProgDuelClassic`] keeps the same cache under the
//! same keys, filled in the same order (one bed per build, as the kit makes one bed per soundtrack). The hats and the
//! ride are not cached by the kit; they are the same every call, so they are kept here too, which changes no sample.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_half_even, round_nd};
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, MusicCtx, MusicError, RISER_OVER, RISER_SEED, clank, noise_seed, riser,
};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{HEAVY_TOMS, TOMS, bass_note, fifth, hat, kick, ride, root, snare, tom, vs_crack};

/// The name the bed's errors carry.
const H: &str = "prog_duel";

/// The bed's own buses, in the order the mood lists them.
const BUSES: [(&str, (f64, f64)); 2] = [("drums", (1.0, 0.12)), ("bass", (1.15, 0.06))];

/// A key of the kit's `self._cache` (or of the sounds kept here that the kit makes each time).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `'kick'`
    Kick,
    /// `('snare', seed % 4)`
    Snare(u64),
    /// `('tom', int(f))`
    Tom(i32),
    /// `(int(m), round(dur, 3), round(vel, 2), round(drive, 2))`, each rounded float by its bits
    Bass(i32, u64, u64, u64),
    /// `A.hat(0.03, open_)`, not cached by the kit
    Hat(bool),
    /// `self.ride()` at velocity 1, not cached by the kit
    Ride,
}

/// `round(x, n)` as a key (a rounded zero is +0.0, as Python's `-0.0 == 0.0`).
fn rkey(x: f64, n: i32) -> u64 {
    let r = round_nd(x, n);
    if r == 0.0 { 0.0f64.to_bits() } else { r.to_bits() }
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

/// Python's `True`/`False` in arithmetic: 1 or 0.
fn one(b: bool) -> f64 {
    f64::from(u8::from(b))
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

/// `T.CHORDS[b % len(T.CHORDS)]`.
fn chord_mod<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let n = ctx.harmony.chords.len();
    if n == 0 {
        return Err(MusicError::new(H, "the episode has no chords"));
    }
    chord(ctx, b % n)
}

/// `T.CHORDS[-1]`.
fn last_chord<'a>(ctx: &'a MusicCtx) -> Result<&'a str, MusicError> {
    ctx.harmony
        .chords
        .last()
        .map(String::as_str)
        .ok_or_else(|| MusicError::new(H, "the episode has no chords"))
}

/// `(b - T.LOOP_START) % len(T.LOOP)`, Python's modulo.
fn loop_slot(ctx: &MusicCtx, b: usize) -> Result<i64, MusicError> {
    let n = ctx.harmony.loop_.len() as i64;
    if n == 0 {
        return Err(MusicError::new(H, "the episode has no loop"));
    }
    Ok((b as i64 - ctx.harmony.loop_start as i64).rem_euclid(n))
}

/// The prog duel's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct ProgDuelClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl ProgDuelClassic {
    /// The bed, its cache empty.
    pub fn new() -> ProgDuelClassic {
        ProgDuelClassic::default()
    }

    /// The sound under `key`, made by `make` the first time.
    fn kept(&self, key: Key, make: impl FnOnce() -> Vec<f64>) -> Arc<Vec<f64>> {
        let hit = self.cache.lock().expect("the cache").get(&key).cloned();
        match hit {
            Some(x) => x,
            None => {
                let x = Arc::new(make());
                Arc::clone(self.cache.lock().expect("the cache").entry(key).or_insert(x))
            }
        }
    }

    /// `self.kick(vel)`.
    fn kick(&self, vel: f64) -> Vec<f64> {
        scaled(&self.kept(Key::Kick, kick), vel)
    }

    /// `self.snare(vel, seed)`.
    fn snare(&self, vel: f64, seed: u64) -> Vec<f64> {
        scaled(&self.kept(Key::Snare(seed % 4), || snare(seed)), vel)
    }

    /// `self.hat(vel, open_)`.
    fn hat(&self, vel: f64, open: bool) -> Vec<f64> {
        scaled(&self.kept(Key::Hat(open), || hat(open)), vel)
    }

    /// `self.tom(f, vel)`.
    fn tom(&self, f: i32, vel: f64) -> Vec<f64> {
        scaled(&self.kept(Key::Tom(f), || tom(f)), vel)
    }

    /// `self.ride(vel)`.
    fn ride(&self, vel: f64) -> Vec<f64> {
        scaled(&self.kept(Key::Ride, ride), vel)
    }

    /// `self.bass_note(m, dur, vel, drive)`: the first call's sound for its key, as it is.
    fn bass_note(&self, m: i32, dur: f64, vel: f64, drive: f64) -> Vec<f64> {
        let key = Key::Bass(m, rkey(dur, 3), rkey(vel, 2), rkey(drive, 2));
        self.kept(key, || bass_note(m, dur, vel, drive)).to_vec()
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.fill(mix, t0, dur, heavy)`: a tom fill in 16ths down the toms over `dur`.
    fn fill(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, dur: f64, heavy: bool) {
        let s16 = ctx.show.grid.s16;
        let toms = if heavy { HEAVY_TOMS } else { TOMS };
        let n = round_half_even(dur / s16) as i64;
        let k = toms.len() as i64;
        for j in 0..n {
            let i = (k - 1).min(j * k / n.max(1));
            let f = toms[i as usize];
            let vel = 0.9 + 0.1 * one(j % 2 == 0);
            let gain = if heavy { 0.34 } else { 0.28 };
            let pan = 0.4 - 0.8 * j as f64 / (n - 1).max(1) as f64;
            put(mix, "drums", self.tom(f, vel), t0 + j as f64 * s16, gain, pan);
        }
    }

    /// `Bed.push_bar(b)`: the 3+3+2 push bars, every other bar of the loop (its 2nd, 4th, ...).
    fn push_bar(&self, ctx: &MusicCtx, b: usize) -> Result<bool, MusicError> {
        Ok(loop_slot(ctx, b)? % 2 == 1)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the groove (bar index `b`): straight 8ths with the backbeat,
    /// or a 3+3+2 push; a tom fill closing each 4-bar phrase; the bass guitar on 8ths. Above intensity 1.3 (and not
    /// `light`) 16th hats, a ride on the push accents, heavier fills and more drive. Python's defaults are
    /// `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let t0 = b as f64 * bar;
        let ch = chord_mod(ctx, b)?;
        let nxt = chord_mod(ctx, b + 1)?;
        let push = self.push_bar(ctx, b)? && !light;
        let big = intensity > 1.3 && !light;
        let r = self.root(ctx, ch)?;
        let e8 = beat / 2.0;
        // drums
        let kicks: &[usize] = if push {
            &[0, 3, 6]
        } else if !light {
            &[0, 4, 5]
        } else {
            &[0, 4]
        };
        let snares: &[usize] = if light { &[4] } else { &[2, 6] };
        for i in 0..8usize {
            let ti = t0 + i as f64 * e8;
            if kicks.contains(&i) {
                put(mix, "drums", self.kick(1.0), ti, if light { 0.50 } else { 0.60 }, 0.0);
            }
            if snares.contains(&i) {
                let g = if light { 0.24 } else { 0.30 };
                put(mix, "drums", self.snare(1.0, (b + i) as u64), ti, g, 0.0);
            }
            let acc = push && [0, 3, 6].contains(&i);
            let g = if acc { 0.09 } else { 0.07 };
            put(mix, "drums", self.hat(1.0, acc && i != 0), ti, g, 0.25);
            if big {
                put(mix, "drums", self.hat(0.6, false), ti + e8 / 2.0, 0.04, 0.25);
                if acc {
                    put(mix, "drums", self.ride(1.0), ti, 0.10, -0.3);
                }
            }
        }
        if loop_slot(ctx, b)? % 4 == 3 && !light {
            // a tom fill closes each 4-bar phrase
            self.fill(ctx, mix, t0 + 3.0 * beat, beat, big);
        }
        // the bass guitar: 8ths on the root, the push's accents jump an octave, the last 8th walks to the next root
        for i in 0..8usize {
            let ti = t0 + i as f64 * e8;
            let (m, v) = if push && (i == 3 || i == 6) {
                (r + 12, 1.0)
            } else if push && i == 0 {
                (r, 1.0)
            } else if i == 7 && !light {
                // a chromatic approach into the next bar
                let nr = self.root(ctx, nxt)?;
                (if nr + 1 != r { nr + 1 } else { nr - 1 }, 0.8)
            } else if i == 4 && !push {
                (fifth(&ctx.harmony, ch).map_err(bad)?, 0.85)
            } else {
                (r, if i % 2 == 1 { 0.75 } else { 0.9 })
            };
            let drive = 1.0 + 0.6 * one(big);
            put(mix, "bass", self.bass_note(m, e8 * 0.92, v, drive), ti, 0.42, 0.0);
        }
        Ok(())
    }
}

impl Bed for ProgDuelClassic {
    /// The mood's class defines `boom` and none of the other optional hooks (no `hit`).
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// `buses = {'drums': (1.0, 0.12), 'bass': (1.15, 0.06)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// An arena.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.6, 0.5, 0.18)
    }

    fn comp(&self) -> (f64, f64) {
        (0.15, 0.62)
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

    /// The kick and the low floor tom at half its gain.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "drums", self.kick(1.0), t, g, 0.0);
        put(mix, "drums", self.tom(87, 1.0), t, g * 0.5, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.M.clank(81, int(t * 3)), t, 0.035)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let seed = noise_seed(H, t * 3.0)?;
        put(mix, "fx", clank(81.0, seed, CLANK_DUR, &CLANK_DECAYS)?, t, 0.035, 0.0);
        Ok(())
    }

    /// A unison band hit under each word of the VS card: kick, crash, floor tom, the bass on the root, a thud.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let r = self.root(ctx, chord(ctx, 0)?)?;
        for tw in times {
            put(mix, "drums", self.kick(1.0), tw, 0.70, 0.0);
            put(mix, "drums", synth::crash(1.4), tw, 0.10, 0.0);
            put(mix, "drums", self.tom(87, 1.0), tw, 0.30, 0.0);
            put(mix, "bass", self.bass_note(r, 0.5, 1.0, 1.4), tw, 0.50, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.30, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", vs_crack(), t, 0.36, 0.0);
        Ok(())
    }

    /// The bass rings on under the card (never quiet), a heavy tom fill on beat 4 as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let r = self.root(ctx, chord(ctx, 0)?)?;
        for i in 0..6usize {
            let v = 0.7 + 0.2 * one(i % 2 == 0);
            let t = i as f64 * beat / 2.0 + 0.004;
            put(mix, "bass", self.bass_note(r, beat / 2.0 * 0.9, v, 1.0), t, 0.34, 0.0);
        }
        self.fill(ctx, mix, 3.0 * beat, beat, true);
        Ok(())
    }

    /// Bar 1: the band counts in on the dominant (kick, snare and a choked crash on each count, the bass on the
    /// root and its octave), a heavy fill on beat 4 and the bass climbing into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let r = self.root(ctx, chord(ctx, 1)?)?;
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            let tc = *tc;
            put(mix, "drums", self.kick(1.0), tc, 0.62, 0.0);
            put(mix, "drums", self.snare(1.0, 70 + j as u64), tc, 0.30, 0.0);
            put(mix, "drums", synth::crash(0.4), tc, 0.06, 0.0);
            put(mix, "bass", self.bass_note(r, beat * 0.45, 1.0, 1.2), tc, 0.46, 0.0);
            put(
                mix,
                "bass",
                self.bass_note(r + 12, beat * 0.4, 0.8, 1.0),
                tc + beat / 2.0,
                0.36,
                0.0,
            );
        }
        let t4 = bar + 3.0 * beat;
        put(mix, "drums", self.kick(1.0), t4, 0.55, 0.0);
        self.fill(ctx, mix, t4, beat, true);
        for (i, step) in [0, 3, 5, 7].into_iter().enumerate() {
            let m = r + step - 12 + 12;
            put(
                mix,
                "bass",
                self.bass_note(m, s16 * 0.9, 0.9, 1.2),
                t4 + i as f64 * s16,
                0.40,
                0.0,
            );
        }
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The groove from FIGHT! to the K.O. (denser as it goes), a crash on each 4-bar phrase, a riser and a crash
    /// into the back half, a tape stop of the drums and the bass at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let bar = ctx.show.grid.bar;
        let t_ko = ctx.show.t_ko;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko.max(1) as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(4) {
            put(mix, "drums", synth::crash(1.6), b as f64 * bar, 0.08, 0.0);
        }
        // the back half (bar 18)
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        let rise = riser(bar, 0.0, 12)?.iter().map(|v| v * 0.6).collect();
        put(mix, "fx", rise, t_half - bar, 0.08, 0.0);
        put(mix, "drums", synth::crash(2.0), t_half, 0.12, 0.0);
        for bus in ["drums", "bass"] {
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
        put(mix, "drums", self.kick(1.0), tw, 0.65, 0.0);
        put(mix, "drums", synth::crash(1.8), tw, 0.12, 0.0);
        let r = self.root(ctx, chord(ctx, 0)?)?;
        put(mix, "bass", self.bass_note(r, 0.8, 1.0, 1.4), tw, 0.50, 0.0);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo, and the kick keeping the pulse (an off-beat hat) up to it.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let dur = py_max(0.2, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.08,
            0.0,
        );
        let beat = ctx.show.grid.beat;
        // range(int(np.ceil((tw + 0.05) / T.BEAT)), int(round(t_solo / T.BEAT)))
        let from = ((tw + 0.05) / beat).ceil() as i64;
        let to = round_half_even(t_solo / beat) as i64;
        for j in from..to {
            let t = j as f64 * beat;
            put(mix, "drums", self.kick(0.9), t, 0.48, 0.0);
            put(mix, "drums", self.hat(1.0, false), t + beat / 2.0, 0.07, 0.0);
        }
        Ok(())
    }

    /// The band plays on under the solo (intensity 1.15), up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.15, false)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a 16th snare roll, the kick on every beat, the bass
    /// climbing the dominant in 8ths, a riser right up to the seam and a crash swelling into it.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16, dur) = (grid.beat, grid.bar, grid.s16, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let r = self.root(ctx, last_chord(ctx)?)?;
        let n = round_half_even((dur - t_last) / s16) as i64;
        for j in 0..n {
            let (jf, nf) = (j as f64, n as f64);
            let seed = u64::try_from(j).expect("a 16th from 0");
            put(
                mix,
                "drums",
                self.snare(0.5 + 0.5 * jf / nf, seed),
                t_last + jf * s16,
                0.16 + 0.12 * jf / nf,
                0.0,
            );
        }
        for bt in 0..4 {
            put(mix, "drums", self.kick(1.0), t_last + f64::from(bt) * beat, 0.58, 0.0);
        }
        let climb = [r, r, r + 4, r + 4, r + 7, r + 7, r + 10, r + 11];
        for (i, &m) in climb.iter().enumerate() {
            put(
                mix,
                "bass",
                self.bass_note(m, beat / 2.0 * 0.9, 0.95, 1.4),
                t_last + i as f64 * beat / 2.0,
                0.44,
                0.0,
            );
        }
        put(
            mix,
            "fx",
            riser(dur - t_last, RISER_OVER, RISER_SEED)?,
            t_last,
            0.18,
            0.0,
        );
        // A.crash(1.0) * np.linspace(0, 1, int(1.0 * SR)) ** 2: an array squared is x * x
        let ramp = dsp::linspace(0.0, 1.0, bf_sound::SR);
        let crash: Vec<f64> = synth::crash(1.0).iter().zip(&ramp).map(|(c, l)| c * (l * l)).collect();
        put(mix, "drums", crash, dur - 1.0 + 0.012, 0.06, 0.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_keys_are_the_kits() {
        assert_eq!(rkey(0.1234, 3), rkey(0.1226, 3));
        assert_eq!(rkey(-0.0001, 3), rkey(0.0, 3));
        assert_eq!(rkey(0.7 + 0.2, 2), rkey(0.9, 2));
        let bed = ProgDuelClassic::new();
        // a snare keyed by seed % 4 is the first seed's noise, scaled by each call's velocity
        let s = bed.snare(1.0, 6);
        assert_eq!(bed.snare(0.5, 2), scaled(&s, 0.5));
        // a bass note keeps the velocity of its first call
        let first = bed.bass_note(33, 0.2, 0.7 + 0.2, 1.0);
        assert_eq!(first, bass_note(33, 0.2, 0.7 + 0.2, 1.0));
        assert_eq!(bed.bass_note(33, 0.2004, 0.9, 1.0), first);
        assert_ne!(bass_note(33, 0.2, 0.9, 1.0), first);
    }

    #[test]
    fn it_has_boom_and_no_hit() {
        let bed = ProgDuelClassic::new();
        assert!(bed.has(BedHook::Boom) && !bed.has(BedHook::Hit));
    }
}
