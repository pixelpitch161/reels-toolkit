//! The heist's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/heist.py`, section by section
//! as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that order,
//! buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back (times the velocity) for every later call with the same key. The
//! keys are loose: a duration is rounded to 3 decimals, a brush's noise is keyed by its seed modulo 3 and a rim's by
//! its seed modulo 2. So `brush(seed=7)` sounds like whichever of seeds 1, 4, 7, ... was asked for first, and the
//! cache is part of the music. [`HeistClassic`] keeps the same cache under the same keys, filled in the same order
//! (one bed per build, as the kit makes one bed per soundtrack).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{floor_div, py_max, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played, no_hook};

use super::{brush, kick, pizz, ride, rim, root, tick, trem, upright, vibes, voicing_from, walk};

/// The name the bed's errors carry.
const H: &str = "heist";

/// The bed's own buses, in the order the mood lists them (`for k in self.buses` tape-stops them in it).
const BUSES: [(&str, (f64, f64)); 5] = [
    ("drums", (1.25, 0.12)),
    ("bass", (1.50, 0.06)),
    ("pizz", (1.00, 0.28)),
    ("vibes", (0.80, 0.40)),
    ("tension", (0.55, 0.45)),
];

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` (by its bits;
/// a rounded zero is +0.0, as Python's `-0.0 == 0.0`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `'kick'`
    Kick,
    /// `('brush', seed % 3)`
    Brush(u64),
    /// `('rim', seed % 2)`
    Rim(u64),
    /// `'ride'`
    Ride,
    /// `('up', int(m), round(dur, 3))`
    Upright(i32, u64),
    /// `('pz', tuple(ms))`
    Pizz(Vec<i32>),
    /// `('vb', tuple(ms), round(dur, 3))`
    Vibes(Vec<i32>, u64),
    /// `('tr', int(m), round(dur, 3))`
    Trem(i32, u64),
    /// `'tick'`
    Tick,
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

/// `xs[i]` (a negative `i` from the end), or Python's `IndexError` as a music error.
fn at(xs: &[i32], i: isize, what: &str) -> Result<i32, MusicError> {
    let j = if i < 0 {
        xs.len().checked_sub(i.unsigned_abs())
    } else {
        Some(i.unsigned_abs())
    };
    j.and_then(|j| xs.get(j).copied())
        .ok_or_else(|| MusicError::new(H, format!("{what} has {} notes, no note {i}", xs.len())))
}

/// `T.chord_tones(76, 88, ch)[-1]`: the tension's high note.
fn top(ctx: &MusicCtx, ch: &str) -> Result<Option<i32>, MusicError> {
    Ok(ctx.harmony.chord_tones(76, 88, ch).map_err(bad)?.last().copied())
}

/// The heist's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct HeistClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl HeistClassic {
    /// The bed, its cache empty.
    pub fn new() -> HeistClassic {
        HeistClassic::default()
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

    /// `self.kick(v)`.
    fn kick(&self, v: f64) -> Vec<f64> {
        self.cached(Key::Kick, v, kick)
    }

    /// `self.brush(v, seed)`.
    fn brush(&self, v: f64, seed: u64) -> Vec<f64> {
        self.cached(Key::Brush(seed % 3), v, || brush(seed))
    }

    /// `self.rim(v, seed)`.
    fn rim(&self, v: f64, seed: u64) -> Vec<f64> {
        self.cached(Key::Rim(seed % 2), v, || rim(seed))
    }

    /// `self.ride(v)`.
    fn ride(&self, v: f64) -> Vec<f64> {
        self.cached(Key::Ride, v, ride)
    }

    /// `self.upright(m, dur, v)`.
    fn upright(&self, m: i32, dur: f64, v: f64) -> Vec<f64> {
        self.cached(Key::Upright(m, r3(dur)), v, || upright(m, dur))
    }

    /// `self.pizz(ms, v)`.
    fn pizz(&self, ms: &[i32], v: f64) -> Vec<f64> {
        self.cached(Key::Pizz(ms.to_vec()), v, || pizz(ms))
    }

    /// `self.vibes(ms, dur, v)`.
    fn vibes(&self, ms: &[i32], dur: f64, v: f64) -> Vec<f64> {
        self.cached(Key::Vibes(ms.to_vec(), r3(dur)), v, || vibes(ms, dur))
    }

    /// `self.trem(m, dur, v)`.
    fn trem(&self, m: i32, dur: f64, v: f64) -> Vec<f64> {
        self.cached(Key::Trem(m, r3(dur)), v, || trem(m, dur))
    }

    /// `self.tick(v)`.
    fn tick(&self, v: f64) -> Vec<f64> {
        self.cached(Key::Tick, v, tick)
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `self.voicing(ch, lo, n)`.
    fn voicing(&self, ctx: &MusicCtx, ch: &str, lo: i32, n: usize) -> Result<Vec<i32>, MusicError> {
        voicing_from(&ctx.harmony, ch, lo, n).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the caper (bar index `b`): the kick on 1, brushes on 2 and 4,
    /// rim clicks on the swung ands of 1 and 3 (and of 4 in odd bars); the walking upright; off-beat pizz chords;
    /// muted vibes in odd bars; the tremolo on the chord's top. The back half (`intensity` above 1.25 and not
    /// `light`) adds the kick on 3, the ride, the creeping pizz arpeggio and vibes every bar; above 1.5 the clock tick
    /// on the 16ths. Python's defaults are `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, or a voicing too short for the arpeggio.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let nb = grid.n_bars;
        let ch = chord(ctx, b % nb)?;
        let nxt = chord(ctx, (b + 1) % nb)?;
        let dense = intensity > 1.25 && !light;
        let e8 = beat / 2.0;
        let sw = e8 * 1.25;
        for bt in 0..4u64 {
            let tb = t0 + bt as f64 * beat;
            if bt == 0 || (dense && bt == 2) {
                put(mix, "drums", self.kick(1.0), tb, 0.55, 0.0);
            }
            if bt == 1 || bt == 3 {
                put(mix, "drums", self.brush(1.0, b as u64 + bt), tb, 0.34, -0.1);
            }
            if bt == 0 || bt == 2 || (bt == 3 && b % 2 == 1) {
                put(mix, "drums", self.rim(1.0, bt), tb + sw, 0.15, 0.2);
            }
            if dense {
                put(mix, "drums", self.ride(1.0), tb, 0.10, 0.3);
                put(mix, "drums", self.ride(1.0), tb + sw, 0.06, 0.3);
            }
        }
        let notes = walk(&ctx.harmony, ch, nxt).map_err(bad)?;
        for (j, &m) in notes.iter().enumerate() {
            let g = if j == 0 { 0.50 } else { 0.44 };
            put(
                mix,
                "bass",
                self.upright(m, beat * 0.9, 1.0),
                t0 + j as f64 * beat,
                g,
                0.0,
            );
        }
        let vo = self.voicing(ctx, ch, 55, 4)?;
        if dense {
            let v = |i: isize| at(&vo, i, "the pizz voicing");
            let arp = [v(0)?, v(1)?, v(2)?, v(1)?, v(-1)?, v(1)?, v(2)?, v(0)? + 12];
            for (j, &m) in arp.iter().enumerate() {
                // `(sw - e8) * (j % 2)`: an int times a float
                let t = t0 + j as f64 * e8 + (sw - e8) * (j % 2) as f64;
                let g = if j % 2 == 0 { 0.20 } else { 0.15 };
                put(mix, "pizz", self.pizz(&[m], 1.0), t, g, -0.25);
            }
        } else {
            for bt in 0..4 {
                let t = t0 + f64::from(bt) * beat + sw;
                put(mix, "pizz", self.pizz(&vo, 1.0), t, 0.20, -0.25);
            }
        }
        if dense || b % 2 == 1 {
            let vv = self.voicing(ctx, ch, 62, 3)?;
            put(mix, "vibes", self.vibes(&vv, 0.4, 1.0), t0 + 1.5 * beat, 0.16, 0.3);
            put(mix, "vibes", self.vibes(&vv, 0.6, 1.0), t0 + 3.0 * beat, 0.14, 0.3);
        }
        if let Some(fifth) = top(ctx, ch)? {
            put(mix, "tension", self.trem(fifth, bar * 0.96, 1.0), t0, 0.05, 0.0);
        }
        if intensity > 1.5 && !light {
            for j in 0..16u32 {
                let g = if j % 4 != 0 { 0.05 } else { 0.08 };
                put(mix, "tension", self.tick(1.0), t0 + f64::from(j) * s16, g, 0.35);
            }
        }
        Ok(())
    }
}

impl Bed for HeistClassic {
    /// The mood's class defines `hit` and none of the other optional hooks: no `boom` (ruling M3-R2).
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Hit]
    }

    /// `buses = {'drums': (1.25, 0.12), 'bass': (1.50, 0.06), 'pizz': (1.00, 0.28), 'vibes': (0.80, 0.40),
    /// 'tension': (0.55, 0.45)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// A dim back room.
    fn reverb(&self) -> (f64, f64, f64) {
        (1.6, 0.5, 0.15)
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

    /// A full caper hit: kick, rim, brush, a vibes and a pizz chord and the upright's low root. The kit's method has
    /// no `ring`: it is not used.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, _ring: f64) -> Played {
        put(mix, "drums", self.kick(1.0), t, 0.62 * g, 0.0);
        put(mix, "drums", self.rim(1.0, 0), t, 0.24 * g, 0.0);
        put(mix, "drums", self.brush(1.0, 2), t, 0.28 * g, 0.0);
        let vv = self.voicing(ctx, ch, 62, 3)?;
        put(mix, "vibes", self.vibes(&vv, 0.8, 1.0), t, 0.22 * g, 0.3);
        let vo = self.voicing(ctx, ch, 50, 4)?;
        put(mix, "pizz", self.pizz(&vo, 1.0), t, 0.30 * g, -0.25);
        let r = self.root(ctx, ch)?;
        put(mix, "bass", self.upright(r, 0.6, 1.0), t, 0.55 * g, 0.0);
        Ok(())
    }

    /// `mix.place('fx', self.rim(1.0, seed=1), t, 0.12)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", self.rim(1.0, 1), t, 0.12, 0.0);
        Ok(())
    }

    /// A full caper hit under each word of the VS card, a crash and a thud.
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

    /// Under the card (never quiet): the tremolo tension, rim clicks tiptoeing, a brush, the upright creeping up.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let beat = ctx.show.grid.beat;
        let ch = chord(ctx, 0)?;
        let hi = ctx.harmony.chord_tones(76, 88, ch).map_err(bad)?;
        let hi = at(&hi, -1, "the tension's chord tones")?;
        put(mix, "tension", self.trem(hi, 3.0 * beat, 1.0), 0.05, 0.08, 0.0);
        for bt in 0..4u64 {
            let t = bt as f64 * beat + beat / 2.0 * 1.25;
            put(mix, "drums", self.rim(1.0, bt), t, 0.15, 0.2);
        }
        put(mix, "drums", self.brush(1.0, 1), 3.0 * beat, 0.34, 0.0);
        let r = self.root(ctx, ch)?;
        put(
            mix,
            "bass",
            self.upright(r + 7, beat * 0.45, 1.0),
            3.0 * beat,
            0.44,
            0.0,
        );
        put(
            mix,
            "bass",
            self.upright(r + 8, beat * 0.45, 1.0),
            3.5 * beat,
            0.44,
            0.0,
        );
        Ok(())
    }

    /// Bar 1: the caper on the count-in chord (intensity 1.1) and a vibes stab on each count.
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        self.bar(ctx, mix, 1, 1.1, false)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &ctx.show.grid.count[..3] {
            let vv = self.voicing(ctx, ch, 62, 3)?;
            put(mix, "vibes", self.vibes(&vv, 0.3, 1.0), *tc, 0.18, 0.3);
        }
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12, 0.0);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.32, 0.0);
        Ok(())
    }

    /// The caper from FIGHT! to the K.O. (thicker as it goes), crashes on the phrase lines, a riser and a crash into
    /// the second pass of the loop, a tape stop of the bed's buses at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let bar = ctx.show.grid.bar;
        let t_ko = ctx.show.t_ko;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.65 * (b as f64 / bar_ko as f64), false)?;
        }
        for b in (2..=bar_ko).step_by(4) {
            put(mix, "drums", synth::crash(1.4), b as f64 * bar, 0.06, 0.0);
        }
        let t_half = (ctx.harmony.loop_start + ctx.harmony.loop_.len()) as f64 * bar;
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

    /// A whoosh from just after the stinger to the solo, and the caper (intensity 1.2) in the bars between.
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

    /// The caper (intensity 1.3) under the solo, up to the last bar.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.3, false)?;
        }
        Ok(())
    }

    /// The last bar creeps into the title slam: the upright walking chromatically up in 8ths to the tonic, rim
    /// clicks on every and, brushes and the kick on every beat, a vibes stab and a pizz chord, the tremolo on the
    /// chord's top, a riser right up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, dur) = (grid.beat, grid.bar, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = ctx.harmony.chord_at(t_last);
        let tgt = self.root(ctx, chord(ctx, 0)?)? + 12;
        let e8 = beat / 2.0;
        for bt in 0..4u64 {
            let tb = t_last + bt as f64 * beat;
            put(mix, "drums", self.kick(1.0), tb, 0.50, 0.0);
            put(mix, "drums", self.brush(1.0, bt), tb, 0.30, 0.0);
            put(mix, "drums", self.rim(1.0, bt), tb + e8, 0.16, 0.2);
        }
        for j in 0..8i32 {
            put(
                mix,
                "bass",
                self.upright(tgt - 8 + j, e8 * 0.85, 1.0),
                t_last + f64::from(j) * e8,
                0.46,
                0.0,
            );
        }
        let vv = self.voicing(ctx, ch, 62, 3)?;
        put(mix, "vibes", self.vibes(&vv, 0.4, 1.0), t_last + 1.5 * beat, 0.16, 0.3);
        let vo = self.voicing(ctx, ch, 55, 4)?;
        put(mix, "pizz", self.pizz(&vo, 1.0), t_last + 3.5 * beat, 0.22, -0.25);
        let hi = ctx.harmony.chord_tones(76, 88, ch).map_err(bad)?;
        let hi = at(&hi, -1, "the tension's chord tones")?;
        put(mix, "tension", self.trem(hi, dur - t_last, 1.0), t_last, 0.08, 0.0);
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
        // a brush keyed by seed % 3 is the first seed's noise; a rim by seed % 2
        let bed = HeistClassic::new();
        let first = bed.brush(1.0, 7);
        assert_eq!(first, brush(7));
        assert_eq!(bed.brush(0.5, 1), scaled(&first, 0.5));
        assert_ne!(brush(1), brush(7));
        let r = bed.rim(1.0, 3);
        assert_eq!(bed.rim(1.0, 1), r);
        assert_ne!(bed.rim(1.0, 0), r);
        // a duration is keyed by round(dur, 3): the first call's length wins
        let u = bed.upright(38, 0.5004, 1.0);
        assert_eq!(bed.upright(38, 0.4996, 1.0), u);
    }

    #[test]
    fn it_has_hit_and_no_boom() {
        let bed = HeistClassic::new();
        assert!(bed.has(BedHook::Hit) && !bed.has(BedHook::Boom));
    }

    #[test]
    fn a_negative_index_counts_from_the_end() {
        assert_eq!(at(&[1, 2, 3], -1, "x").unwrap(), 3);
        assert_eq!(at(&[1, 2, 3], 0, "x").unwrap(), 1);
        assert!(at(&[1, 2], 2, "x").is_err());
        assert!(at(&[], -1, "x").is_err());
    }
}
