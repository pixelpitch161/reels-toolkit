//! The second line's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/second_line.py`, section by
//! section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every drum's sound in a dict (`self._c`) under a key made when it is first
//! asked for, and hands that sound back (times the velocity) for every later call with the same key. The keys are
//! loose: a snare's noise is keyed by its seed modulo 4, a tambourine's and a clap's by theirs modulo 3, and a press
//! roll by its length and crescendo rounded to 3 decimals. So `snare(seed=9)` sounds like whichever of seeds 1, 5, 9,
//! ... was asked for first, and the cache is part of the music. [`SecondLineClassic`] keeps the same cache under the
//! same keys, filled in the same order (one bed per build, as the kit makes one bed per soundtrack).
//!
//! **The sousaphone** is the TUBA's Pixel Pitch voice through the music context's note cache
//! ([`MusicCtx::note`], the kit's `M.note`), which keeps its own keys and its own first-note gain.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{floor_div, py_max, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{
    BD_A, BD_B, GH_A, GH_B, GH_X, SWING16, TUBA, bass_drum, chick, clap, fifth, press, root, snare, swell, tambourine,
};

/// The name the bed's errors carry.
const H: &str = "second_line";

/// The bed's own buses, in the order the mood lists them (`for k in ('drums', 'sousa', 'perc')` tape-stops them in
/// it).
const BUSES: [(&str, (f64, f64)); 3] = [("drums", (1.40, 0.14)), ("sousa", (1.75, 0.10)), ("perc", (1.10, 0.22))];

/// A key of the kit's `self._c`: the method's tag and its key fields, a float as the bits of its rounding (a rounded
/// zero is +0.0, as Python's `-0.0 == 0.0`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('bd', big)`
    BassDrum(bool),
    /// `('ck', accent, seed)`
    Chick(bool, i64),
    /// `('sn', ghost, seed % 4)`
    Snare(bool, i64),
    /// `('pr', round(dur, 3), seed, None or round(v0 / max(v, 1e-6), 3))`
    Press(u64, i64, Option<u64>),
    /// `('tb', seed % 3)`
    Tambourine(i64),
    /// `('cl', seed % 3)`
    Clap(i64),
}

/// `round(x, 3)` as a key.
fn r3(x: f64) -> u64 {
    let r = round_nd(x, 3);
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

/// Python's `int(v)` as a seed that may be below zero (the seed of `int(t * 10)`): numpy refuses `base + seed`
/// below zero, which the caller checks.
fn int_seed(v: f64) -> Result<i64, MusicError> {
    if v.is_finite() && v.abs() < 9.0e15 {
        Ok(v.trunc() as i64)
    } else {
        Err(MusicError::new(H, format!("a seed of int({v})")))
    }
}

/// `default_rng(base + seed)` refuses a seed below zero: the error before any sound is made.
fn check_seed(base: i64, seed: i64) -> Result<i64, MusicError> {
    if base + seed < 0 {
        Err(MusicError::new(
            H,
            format!("a noise seed of {}: numpy refuses a negative seed", base + seed),
        ))
    } else {
        Ok(seed)
    }
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

/// The second line's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct SecondLineClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl SecondLineClassic {
    /// The bed, its cache empty.
    pub fn new() -> SecondLineClassic {
        SecondLineClassic::default()
    }

    /// `self._cache(key, fn) * v`: the sound under `key`, made by `make` the first time, times `v`.
    fn cached(&self, key: Key, v: f64, make: impl FnOnce() -> Vec<f64>) -> Vec<f64> {
        let hit = self.cache.lock().expect("the cache").get(&key).cloned();
        let x = match hit {
            Some(x) => x,
            None => {
                let x = Arc::new(make());
                Arc::clone(self.cache.lock().expect("the cache").entry(key).or_insert(x))
            }
        };
        scaled(&x, v)
    }

    /// `self.t16(b, j)`: the time of 16th `j` in bar `b`, swung (the off 16th of each 8th pair late).
    fn t16(ctx: &MusicCtx, b: usize, j: u32) -> f64 {
        let grid = &ctx.show.grid;
        let e8 = grid.beat / 2.0;
        b as f64 * grid.bar + f64::from(j / 2) * e8 + f64::from(j % 2) * SWING16 * e8
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `self.sousa(m, dur, vel)`: `M.note('TUBA', int(m), round(dur, 3), round(dur + 0.12, 2), round(vel, 1))`.
    fn sousa(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Result<Vec<f64>, MusicError> {
        let x = ctx.note(
            TUBA,
            f64::from(m),
            round_nd(dur, 3),
            round_nd(dur + 0.12, 2),
            round_nd(vel, 1),
        )?;
        Ok(x.to_vec())
    }

    /// `self.bass_drum(v, big)`.
    fn bass_drum(&self, v: f64, big: bool) -> Vec<f64> {
        self.cached(Key::BassDrum(big), v, || bass_drum(big))
    }

    /// `self.chick(v, accent, seed)`.
    fn chick(&self, v: f64, accent: bool, seed: i64) -> Vec<f64> {
        self.cached(Key::Chick(accent, seed), v, || chick(accent, seed))
    }

    /// `self.snare(v, ghost, seed)`.
    fn snare(&self, v: f64, ghost: bool, seed: i64) -> Result<Vec<f64>, MusicError> {
        // Python checks the seed only when it makes the sound, and a cached key never makes one.
        let key = Key::Snare(ghost, seed.rem_euclid(4));
        let made = self.cache.lock().expect("the cache").contains_key(&key);
        if !made {
            check_seed(50, seed)?;
        }
        Ok(self.cached(key, v, || snare(ghost, seed)))
    }

    /// `self.press(dur, v, seed, v0)`.
    fn press(&self, dur: f64, v: f64, seed: i64, v0: Option<f64>) -> Result<Vec<f64>, MusicError> {
        let key = Key::Press(r3(dur), seed, v0.map(|v0| r3(v0 / py_max(v, 1e-6))));
        let made = self.cache.lock().expect("the cache").contains_key(&key);
        if !made {
            check_seed(60, seed)?;
        }
        Ok(self.cached(key, v, || press(dur, v, seed, v0)))
    }

    /// `self.tambourine(v, seed)`.
    fn tambourine(&self, v: f64, seed: i64) -> Vec<f64> {
        self.cached(Key::Tambourine(seed.rem_euclid(3)), v, || tambourine(seed))
    }

    /// `self.clap(v, seed)`.
    fn clap(&self, v: f64, seed: i64) -> Vec<f64> {
        self.cached(Key::Clap(seed.rem_euclid(3)), v, || clap(seed))
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the second line (bar index `b`): the bass drum's figure (A on
    /// even bars, B on odd) with the cymbal's chick on every beat (accent on 2 and 4), stick shots on 2 and 4 and
    /// ghost notes, the sousaphone's tresillo. The back half (`intensity` above 1.25 and not `light`) adds ghosts, a
    /// press roll into beat 4, the tambourine's off-beats, claps on 2 and 4 and the sousaphone's approach into the
    /// next chord; a press roll leads into every phrase. Python's defaults are `intensity = 1.0`, `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, or a note the TUBA's voice refuses.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let t0 = b as f64 * bar;
        let nb = grid.n_bars;
        let ch = chord(ctx, b % nb)?;
        let nxt = chord(ctx, (b + 1) % nb)?;
        let big = intensity > 1.25 && !light;
        let r = self.root(ctx, ch)?;
        let f5 = fifth(&ctx.harmony, ch).map_err(bad)?;
        let odd = b % 2 == 1;
        let figure: &[u32] = if odd { &BD_B } else { &BD_A };
        for &j in figure {
            let v = if j == 0 || j == 12 { 1.0 } else { 0.85 };
            let g = if light { 0.42 } else { 0.50 };
            put(mix, "drums", self.bass_drum(v, false), Self::t16(ctx, b, j), g, 0.0);
        }
        for bt in 0..4i64 {
            let acc = bt == 1 || bt == 3;
            let tb = t0 + bt as f64 * beat;
            let g = if acc { 0.075 } else { 0.05 };
            put(mix, "drums", self.chick(1.0, acc, bt), tb, g, 0.3);
            if acc {
                let x = self.snare(1.0, false, b as i64 * 4 + bt)?;
                put(mix, "drums", x, tb, 0.30, -0.12);
            }
        }
        let mut ghosts: Vec<u32> = if odd { GH_B.to_vec() } else { GH_A.to_vec() };
        if big {
            ghosts.extend(GH_X);
        }
        for j in ghosts {
            let x = self.snare(1.0, true, i64::from(j) + b as i64)?;
            put(mix, "drums", x, Self::t16(ctx, b, j), 0.30, -0.12);
        }
        // the sousaphone: root on 1, the fifth on the and of 2, the root an octave up on 4 (where it fits)
        put(mix, "sousa", self.sousa(ctx, r, 0.80 * beat, 0.95)?, t0, 0.62, 0.0);
        put(
            mix,
            "sousa",
            self.sousa(ctx, f5, 0.40 * beat, 0.85)?,
            Self::t16(ctx, b, 6),
            0.52,
            0.0,
        );
        let top = if r + 12 <= 52 { r + 12 } else { r };
        put(
            mix,
            "sousa",
            self.sousa(ctx, top, 0.45 * beat, 0.85)?,
            t0 + 3.0 * beat,
            0.50,
            0.0,
        );
        if big {
            let nr = self.root(ctx, nxt)?;
            let pk = if nxt != ch { nr - 1 } else { f5 };
            put(
                mix,
                "sousa",
                self.sousa(ctx, pk, 0.28 * beat, 0.8)?,
                Self::t16(ctx, b, 14),
                0.42,
                0.0,
            );
            let x = self.press(0.85 * beat, 1.0, b as i64, Some(0.4))?;
            put(mix, "drums", x, Self::t16(ctx, b, 10), 0.13, -0.12);
            for bt in 0..4i64 {
                let t = t0 + (bt as f64 + SWING16 / 2.0 + 0.25) * beat;
                put(mix, "perc", self.tambourine(1.0, bt), t, 0.07, 0.35);
                if bt == 1 || bt == 3 {
                    put(
                        mix,
                        "perc",
                        self.clap(1.0, b as i64 + bt),
                        t0 + bt as f64 * beat,
                        0.12,
                        -0.3,
                    );
                }
            }
        }
        let ls = ctx.harmony.loop_start;
        if b >= ls && (b - ls) % 4 == 3 && !light {
            // a press roll into every phrase
            let x = self.press(beat * 0.95, 1.0, 500 + b as i64, Some(0.35))?;
            put(mix, "drums", x, t0 + 3.0 * beat, 0.17, -0.12);
        }
        Ok(())
    }
}

impl Bed for SecondLineClassic {
    /// The mood's class defines `boom`, `hit` and `press_roll`.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit, BedHook::PressRoll]
    }

    /// `buses = {'drums': (1.40, 0.14), 'sousa': (1.75, 0.10), 'perc': (1.10, 0.22)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// A street between brick walls.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.6, 0.5, 0.17)
    }

    fn comp(&self) -> (f64, f64) {
        (0.18, 0.55)
    }

    fn roar_gain(&self) -> f64 {
        0.16
    }

    fn doors_gain(&self) -> f64 {
        0.40
    }

    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (93.0, 98.0, 0.10)
    }

    /// A big drum hit (a slam landing): the big bass drum and the sousaphone's low root (an octave down where it can
    /// go, A1 and up).
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "drums", self.bass_drum(1.0, true), t, g * 0.7, 0.0);
        let r = self.root(ctx, ctx.harmony.chord_at(t))?;
        let m = if r - 12 >= 33 { r - 12 } else { r };
        put(mix, "sousa", self.sousa(ctx, m, 0.5, 1.0)?, t, 0.4 * g, 0.0);
        Ok(())
    }

    /// A full band hit: the sousaphone's root (an octave down where it can go) and its root, the big bass drum, a
    /// stick shot and the cymbal crashed open; `ring` is how long the sousaphone holds.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let r = self.root(ctx, ch)?;
        let low = if r - 12 >= 33 { r - 12 } else { r };
        put(mix, "sousa", self.sousa(ctx, low, ring, 1.0)?, t, 0.60 * g, 0.0);
        put(mix, "sousa", self.sousa(ctx, r, ring, 0.9)?, t, 0.32 * g, 0.0);
        put(mix, "drums", self.bass_drum(1.0, true), t, 0.62 * g, 0.0);
        let x = self.snare(1.0, false, int_seed(t * 10.0)?)?;
        put(mix, "drums", x, t, 0.32 * g, -0.12);
        put(mix, "drums", self.chick(1.0, true, 5), t, 0.12 * g, 0.3);
        Ok(())
    }

    /// A snare press-roll crescendo from `t0` to `t1` (a fighter's fanfare, such as MARCHING ORDERS).
    fn press_roll(&self, _: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played {
        let x = self.press(
            py_max(0.05, t1 - t0),
            1.0,
            int_seed(t0 * 10.0)?,
            Some(g0 / py_max(g1, 1e-6)),
        )?;
        put(mix, "drums", x, t0, g1 * 0.5, -0.12);
        Ok(())
    }

    /// `mix.place('fx', self.snare(1.0, ghost=True, seed=3), t, 0.10)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let x = self.snare(1.0, true, 3)?;
        put(mix, "fx", x, t, 0.10, 0.0);
        Ok(())
    }

    /// A full band hit under each word of the VS card, a crash and a thud.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for tw in times {
            self.hit(ctx, mix, tw, ch, 1.0, 0.45)?;
            put(mix, "drums", synth::crash(1.0), tw, 0.07, 0.0);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", synth::thud(0.8, 120.0, 34.0, seed), tw, 0.30, 0.0);
        }
        Ok(())
    }

    /// VS: a thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.40, 0.0);
        Ok(())
    }

    /// The held chord under the card (never quiet): a long sousaphone root, the cymbal swelling, the snare's press
    /// roll swelling into beat 4 as the doors open, the bass drum keeping the beat.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let ch = chord(ctx, 0)?;
        let r = self.root(ctx, ch)?;
        put(mix, "sousa", self.sousa(ctx, r, 2.9 * beat, 0.85)?, 0.05, 0.46, 0.0);
        let n = (3.0 * beat * bf_sound::SRF) as usize;
        put(mix, "drums", swell(n), 0.0, 0.04, 0.0);
        let x = self.press(2.0 * beat, 1.0, 6, Some(0.25))?;
        put(mix, "drums", x, beat, 0.14, -0.12);
        let x = self.press(beat * 0.95, 1.0, 7, Some(0.3))?;
        put(mix, "drums", x, 3.0 * beat, 0.22, -0.12);
        for bt in 1..4 {
            // the bass drum keeps the beat under the card
            put(
                mix,
                "drums",
                self.bass_drum(0.8, false),
                f64::from(bt) * beat + beat / 2.0,
                0.30,
                0.0,
            );
        }
        Ok(())
    }

    /// Bar 1: the band keeps the second line on the count-in chord (intensity 1.4), a band hit on each count, a press
    /// roll and a sousaphone walk-up into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        self.bar(ctx, mix, 1, 1.4, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &grid.count[..3] {
            self.hit(ctx, mix, *tc, ch, 0.55, 0.30)?;
        }
        let t4 = bar + 3.0 * beat;
        let x = self.press(beat * 0.98, 1.0, 9, Some(0.4))?;
        put(mix, "drums", x, t4, 0.26, -0.12);
        let r2 = self.root(ctx, chord(ctx, 2)?)?;
        put(mix, "sousa", self.sousa(ctx, r2 - 2, 0.3 * beat, 0.85)?, t4, 0.40, 0.0);
        put(
            mix,
            "sousa",
            self.sousa(ctx, r2 - 1, 0.3 * beat, 0.85)?,
            t4 + beat / 2.0,
            0.42,
            0.0,
        );
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.34, 0.0);
        Ok(())
    }

    /// The second line from FIGHT! to the K.O. (denser as it goes), crashes on the phrase lines, a long press roll
    /// and a riser into the back half (two passes of the loop in), a tape stop of the bed's buses at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let t_ko = ctx.show.t_ko;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(8) {
            put(mix, "drums", synth::crash(1.6), b as f64 * bar, 0.08, 0.0);
        }
        let t_half = (ctx.harmony.loop_start + 2 * ctx.harmony.loop_.len()) as f64 * bar;
        if t_half < t_ko {
            let x = self.press(2.0 * beat, 1.0, 33, Some(0.3))?;
            put(mix, "drums", x, t_half - 2.0 * beat, 0.18, -0.12);
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
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 0.8)?;
        put(mix, "drums", synth::crash(1.8), tw, 0.10, 0.0);
        Ok(())
    }

    /// From the stinger to the solo the band holds the groove (intensity 1.2; never a quiet stretch), under a whoosh.
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

    /// The band plays on under the solo at full tilt (intensity 1.4, a victory parade), up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.4, false)?;
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a press-roll crescendo right up to the seam, the
    /// sousaphone walking up chromatically into the tonic, the bass drum on every beat, the chick, claps, a riser.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let x = self.press(dur - t_last + 0.012, 1.0, 99, Some(0.35))?;
        put(mix, "drums", x, t_last, 0.24, -0.12);
        let tgt = self.root(ctx, chord(ctx, 0)?)?;
        for bt in 0..4i64 {
            let tb = t_last + bt as f64 * beat;
            put(
                mix,
                "sousa",
                self.sousa(ctx, tgt - 4 + bt as i32, 0.6 * beat, 0.9)?,
                tb,
                0.60,
                0.0,
            );
            put(mix, "drums", self.bass_drum(0.95, false), tb, 0.50, 0.0);
            put(mix, "drums", self.chick(1.0, bt % 2 == 1, bt), tb, 0.07, 0.3);
            put(mix, "perc", self.clap(1.0, bt), tb, 0.12, -0.3);
        }
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
        let bed = SecondLineClassic::new();
        // a snare keyed by seed % 4 is the first seed's noise; a tambourine and a clap by seed % 3
        let first = bed.snare(1.0, false, 9).unwrap();
        assert_eq!(first, snare(false, 9));
        assert_eq!(bed.snare(0.5, false, 1).unwrap(), scaled(&first, 0.5));
        assert_ne!(bed.snare(1.0, true, 1).unwrap(), first);
        assert_ne!(snare(false, 1), snare(false, 9));
        assert_eq!(bed.tambourine(1.0, 4), bed.tambourine(1.0, 1));
        assert_eq!(bed.clap(1.0, 5), clap(5));
        assert_eq!(bed.clap(1.0, 2), clap(5));
        // a chick is keyed by its seed itself
        assert_ne!(bed.chick(1.0, true, 5), bed.chick(1.0, true, 1));
        // a press roll by round(dur, 3), its seed and round(v0 / v, 3): the first call's length wins
        let p = bed.press(0.2004, 1.0, 6, Some(0.25)).unwrap();
        assert_eq!(bed.press(0.1996, 1.0, 6, Some(0.2504)).unwrap(), p);
        assert_ne!(bed.press(0.2, 1.0, 6, Some(0.3)).unwrap(), p);
    }

    #[test]
    fn a_seed_below_zero_is_refused_as_numpy_does() {
        let bed = SecondLineClassic::new();
        assert!(bed.press(0.2, 1.0, -61, Some(0.3)).is_err());
        // int(t0 * 10) of a time a little before zero is 0, and 60 - 5 is still a seed
        assert_eq!(int_seed(-0.05).unwrap(), 0);
        assert!(bed.press(0.2, 1.0, -5, Some(0.3)).is_ok());
        assert!(int_seed(f64::NAN).is_err());
    }

    #[test]
    fn it_has_boom_hit_and_press_roll() {
        let bed = SecondLineClassic::new();
        assert!(bed.has(BedHook::Boom) && bed.has(BedHook::Hit) && bed.has(BedHook::PressRoll));
        assert!(!bed.has(BedHook::Erupt) && !bed.has(BedHook::TrailerHit));
    }
}
