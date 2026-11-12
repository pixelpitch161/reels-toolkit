//! The horror trap's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/horror_trap.py`, section
//! by section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a bus in that
//! order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back (times the velocity) for every later call with the same key. The
//! keys are loose: a duration is rounded to 3 decimals, a clap's noise is keyed by its seed modulo 2 and a hat by its
//! seed modulo 3. So `clap(seed=7)` sounds like whichever of seeds 1, 3, 5, ... was asked for first, and the cache
//! is part of the music. [`HorrorTrapClassic`] keeps the same cache under the same keys, filled in the same order
//! (one bed per build, as the kit makes one bed per soundtrack).
//!
//! **The run's windows.** The kit's `__init__` reads LIGHTS OUT (`self.dark`) and MOUNTAIN KING (`self.mk`) from the
//! run's events; a bed here is made before it sees the run, so [`Windows::of`] reads them from `ctx.rec` inside the
//! methods that need them, the same way.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::event::Event;
use bf_core::harmony::HarmonyError;
use bf_core::py::{floor_div, imod_py, py_max, py_min, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, event_at, event_is, event_number, riser, thunder,
};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{BOX_HI, BOX_LO, STRINGS_LO, b808, box_motif, clap, hat, kick, rev_swell, root, strings, tine};

/// The name the bed's errors carry.
const H: &str = "horror_trap";

/// The bed's own buses, in the order the mood lists them (`for k in self.buses` tape-stops them in it).
const BUSES: [(&str, (f64, f64)); 5] = [
    ("808", (0.95, 0.04)),
    ("drums", (1.20, 0.10)),
    ("musicbox", (0.90, 0.50)),
    ("strings", (0.60, 0.40)),
    ("swell", (0.80, 0.35)),
];

/// `b808`'s default `slide`.
const SLIDE: f64 = 0.09;
/// `tine`'s default `det`.
const DET: f64 = 0.18;

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(x, 3)` (by its bits; a
/// rounded zero is +0.0, as Python's `-0.0 == 0.0`). Every MIDI note here is a whole number, so `round(m, 2)` is it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('808', round(m, 2), round(dur, 3), to, round(slide, 3))`
    B808(i32, u64, Option<i32>, u64),
    /// `'kick'`
    Kick,
    /// `('clap', seed % 2)`
    Clap(u64),
    /// `('hat', open_, seed % 3)`
    Hat(bool, u64),
    /// `('tine', round(m, 2), round(dur, 3), round(det, 3))`
    Tine(i32, u64, u64),
    /// `('str', tuple(ms), round(dur, 3), trem)`
    Str(Vec<i32>, u64, bool),
    /// `('rev', round(dur, 3), m)`
    Rev(u64, Option<i32>),
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

/// `mix.place(bus, x, t, gain, pan)`.
fn pan(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64, pan: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, pan);
}

/// `mix.place(bus, x, t, gain)`: centred.
fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64) {
    pan(mix, bus, x, t, gain, 0.0);
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

/// `T.chord_tones(88, 100, ch)`: the music box's tones of `ch`.
fn box_tones(ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
    ctx.harmony.chord_tones(BOX_LO, BOX_HI, ch).map_err(bad)
}

/// `T.chord_tones(88, 100, ch)[0]`.
fn box_low(ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
    box_tones(ctx, ch)?
        .first()
        .copied()
        .ok_or_else(|| MusicError::new(H, format!("{ch} has no tone from 88 to 100 (Python's IndexError)")))
}

/// Python's `q[2] == e[2]`: two events of the same fighter.
fn same_fighter(e: &Event, q: &Event) -> Result<bool, MusicError> {
    Ok(event_at(H, q, 2)? == event_at(H, e, 2)?)
}

/// The run's LIGHTS OUT and MOUNTAIN KING windows, as the kit's `__init__` reads them from `M.EV`.
#[derive(Clone, Debug, Default, PartialEq)]
struct Windows {
    /// `self.dark`: `(t0, t_erupt)` of each LIGHTS OUT before the K.O. (cut at the K.O.).
    dark: Vec<(f64, f64)>,
    /// `self.mk`: `(t0, t1, [hop times])` of each MOUNTAIN KING before the K.O.
    mk: Vec<(f64, f64, Vec<f64>)>,
}

impl Windows {
    /// The windows of the run in `ctx` (every element read where Python reads it, in its order).
    fn of(ctx: &MusicCtx) -> Result<Windows, MusicError> {
        let ev = &ctx.rec.events;
        let t_ko = ctx.show.t_ko;
        let mut w = Windows::default();
        for e in ev {
            if e.kind == "lights_out" && e.t < t_ko {
                w.dark.push((e.t, py_min(event_number(H, e, 3)?, t_ko)));
            }
        }
        for e in ev {
            if e.kind == "mountain" && e.t < t_ko {
                let mut end = None;
                for q in ev {
                    if q.kind == "mountain_end" && same_fighter(e, q)? && q.t >= e.t {
                        end = Some(q.t);
                        break;
                    }
                }
                let t1 = py_min(end.unwrap_or(t_ko), t_ko);
                let mut hops = Vec::new();
                for q in ev {
                    if q.kind == "hop" && same_fighter(e, q)? && e.t <= q.t && q.t <= t1 && event_is(H, q, 8, 2)? {
                        hops.push(q.t);
                    }
                }
                w.mk.push((e.t, t1, hops));
            }
        }
        Ok(w)
    }

    /// `in_dark(t)`: `any(a - 0.02 <= t < b for a, b in self.dark)`.
    fn in_dark(&self, t: f64) -> bool {
        self.dark.iter().any(|&(a, b)| a - 0.02 <= t && t < b)
    }

    /// `in_mk(t)`: `any(a <= t < b for a, b, _ in self.mk)`.
    fn in_mk(&self, t: f64) -> bool {
        self.mk.iter().any(|(a, b, _)| *a <= t && t < *b)
    }
}

/// The horror trap's bed as the kit plays it, with the kit's cache of sounds.
#[derive(Debug, Default)]
pub struct HorrorTrapClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl HorrorTrapClassic {
    /// The bed, its cache empty.
    pub fn new() -> HorrorTrapClassic {
        HorrorTrapClassic::default()
    }

    /// `self._cache(key, fn) * v`: the sound under `key`, made by `make` the first time, times `v`. The lock is not
    /// held while `make` runs (`rev_swell` asks the cache for a tine inside its own `make`, as the kit's does).
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

    /// `self.b808(m, dur, to, slide, v=1.0)`.
    fn b808(&self, m: i32, dur: f64, to: Option<i32>, slide: f64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::B808(m, r3(dur), to, r3(slide)), 1.0, || {
            Ok(b808(m, dur, to, slide))
        })
    }

    /// `self.kick(v)`.
    fn kick(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Kick, v, || Ok(kick()))
    }

    /// `self.clap(v, seed)`.
    fn clap(&self, v: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Clap(seed % 2), v, || Ok(clap(seed)))
    }

    /// `self.hat(v, open_, seed)`.
    fn hat(&self, v: f64, open: bool, seed: u64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Hat(open, seed % 3), v, || Ok(hat(open, seed)))
    }

    /// `self.tine(m, dur, v, det)`.
    fn tine(&self, m: i32, dur: f64, v: f64, det: f64) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Tine(m, r3(dur), r3(det)), v, || Ok(tine(m, dur, det)))
    }

    /// `self.strings(ms, dur, trem, v=1.0)`.
    fn strings(&self, ms: &[i32], dur: f64, trem: bool) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Str(ms.to_vec(), r3(dur), trem), 1.0, || Ok(strings(ms, dur, trem)))
    }

    /// `self.rev_swell(dur, m, v=1.0)`: its tine (when `m` is given) from the cache.
    fn rev_swell(&self, dur: f64, m: Option<i32>) -> Result<Vec<f64>, MusicError> {
        self.cached(Key::Rev(r3(dur), m), 1.0, || match m {
            Some(m) => {
                let tn = self.tine(m, dur, 1.0, 0.25)?;
                Ok(rev_swell(dur, Some(&tn)))
            }
            None => Ok(rev_swell(dur, None)),
        })
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `(sr, sr + 7, sr + 13)` for `sr = T.root_above(ch, 40)`: the strings' cluster (root, fifth, the minor ninth's
    /// rub).
    fn cluster(&self, ctx: &MusicCtx, ch: &str) -> Result<[i32; 3], MusicError> {
        let sr = ctx.harmony.root_above(ch, STRINGS_LO).map_err(bad)?;
        Ok([sr, sr + 7, sr + 13])
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the trap beat (bar index `b`); `intensity` 1.0 to 1.6 adds
    /// parts. Inside LIGHTS OUT only the music box and the 808 play; inside MOUNTAIN KING the hats follow the hops.
    /// Python's defaults are `intensity = 1.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, an event without the element the kit reads.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let w = Windows::of(ctx)?;
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let nb = grid.n_bars;
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b % nb)?;
        let nxt = chord(ctx, (b + 1) % nb)?;
        let dense = intensity > 1.25 && !light;
        let e8 = beat / 2.0;
        let phrase_end = imod_py(b as i64 - ctx.harmony.loop_start as i64, 4) == 3;
        let dark = |t: f64| w.in_dark(t);
        let r = self.root(ctx, ch)?;
        // ---- the 808: the root on 1 (sliding into the next root at a phrase end), a ghost on the "and of 3"
        let slide_to = if phrase_end && self.root(ctx, nxt)? != r {
            Some(self.root(ctx, nxt)?)
        } else {
            None
        };
        if w.in_mk(t0 + bar / 2.0) {
            // MOUNTAIN KING: pump the root on the beats
            for bt in 0..4 {
                let g = if bt == 0 { 0.48 } else { 0.40 };
                put(
                    mix,
                    "808",
                    self.b808(r, beat * 0.9, None, SLIDE)?,
                    t0 + f64::from(bt) * beat,
                    g,
                );
            }
        } else {
            let g808 = if dark(t0) { 0.62 } else { 0.52 };
            put(mix, "808", self.b808(r, beat * 2.4, None, SLIDE)?, t0, g808);
            let tg = t0 + 2.5 * beat;
            let g = if dark(tg) { 0.50 } else { 0.40 };
            put(mix, "808", self.b808(r, beat * 1.0, slide_to, 0.12)?, tg, g);
        }
        // ---- drums (half-time): kick 1 + the pickup, clap on 3, hats
        for bt in 0..4u32 {
            let tb = t0 + f64::from(bt) * beat;
            if bt == 0 && !dark(tb) {
                pan(mix, "drums", self.kick(1.0)?, tb, 0.55, 0.0);
            }
            if bt == 2 && !dark(tb) {
                pan(mix, "drums", self.clap(1.0, b as u64)?, tb, 0.50, 0.05);
            }
        }
        if !dark(t0 + 1.75 * beat) {
            // the syncopated pickup kick
            pan(mix, "drums", self.kick(1.0)?, t0 + 1.75 * beat, 0.44, 0.0);
        }
        if dense && !dark(t0 + 3.5 * beat) {
            pan(mix, "drums", self.kick(1.0)?, t0 + 3.5 * beat, 0.40, 0.0);
        }
        let mut hats: Vec<(f64, f64)> = Vec::new();
        let mk_win: Vec<&(f64, f64, Vec<f64>)> = w.mk.iter().filter(|m| m.0 < t0 + bar && m.1 > t0).collect();
        for j in 0..8u32 {
            hats.push((t0 + f64::from(j) * e8, if j % 2 == 0 { 0.20 } else { 0.15 }));
        }
        if phrase_end || dense {
            // skitter: 16ths over beat 3, a triplet roll into the next
            hats.push((t0 + 2.0 * beat + s16, 0.12));
            hats.push((t0 + 2.0 * beat + 3.0 * s16, 0.12));
            let tri = beat / 3.0 / 2.0;
            for j in 0..6u32 {
                let jf = f64::from(j);
                hats.push((t0 + 3.0 * beat + jf * tri, 0.10 + 0.012 * jf));
            }
        }
        if intensity > 1.45 && !light {
            // late: 16ths on beat 2 too
            hats.push((t0 + beat + s16, 0.11));
            hats.push((t0 + beat + 3.0 * s16, 0.11));
        }
        for (th, g) in hats {
            if dark(th) {
                continue;
            }
            if mk_win.iter().any(|m| m.0 <= th && th < m.1) {
                // MOUNTAIN KING: the hops take over the hats
                continue;
            }
            let seed = round_half_even((th - t0) / s16) as u64;
            pan(mix, "drums", self.hat(1.0, false, seed)?, th, g, 0.25);
        }
        for m in &mk_win {
            for &th in &m.2 {
                if t0 <= th && th < t0 + bar {
                    let seed = (th * 37.0) as u64;
                    pan(mix, "drums", self.hat(1.0, false, seed)?, th, 0.18, 0.25);
                }
            }
        }
        if dense && !dark(t0 + 3.5 * beat) && mk_win.is_empty() {
            pan(mix, "drums", self.hat(1.0, true, 0)?, t0 + 3.5 * beat, 0.10, 0.25);
        }
        // ---- the music box: the lullaby, two bars per phrase (in the dark: fuller, an octave doubled)
        let first = b - b % 2;
        let (bar1, bar2) =
            box_motif(&ctx.harmony, chord(ctx, first % nb)?, chord(ctx, (first + 1) % nb)?).map_err(bad)?;
        let notes = if b.is_multiple_of(2) { bar1 } else { bar2 };
        for (j, &m) in notes.iter().enumerate() {
            let tj = t0 + j as f64 * e8;
            let dk = dark(tj);
            let g = (if dk { 0.24 } else { 0.17 }) * (if j % 2 == 0 { 1.0 } else { 0.8 });
            pan(mix, "musicbox", self.tine(m, e8 * 1.8, 1.0, DET)?, tj, g, -0.3);
            if dk || dense {
                // the octave under (back half, and in the dark)
                pan(
                    mix,
                    "musicbox",
                    self.tine(m - 12, e8 * 1.8, 1.0, 0.22)?,
                    tj,
                    g * 0.6,
                    -0.35,
                );
            }
        }
        // ---- creeping low strings, swelling over 2 bars
        if b.is_multiple_of(2) && !(dark(t0) || dark(t0 + bar)) {
            let ms = self.cluster(ctx, ch)?;
            put(mix, "strings", self.strings(&ms, 2.0 * bar * 0.97, dense)?, t0, 0.42);
        }
        Ok(())
    }

    /// `self.hit(mix, t, ch, g)`: the full trap hit (also `title`, `erupt` and `stinger`).
    fn trap_hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64) -> Played {
        let r = self.root(ctx, ch)?;
        put(mix, "808", self.b808(r, 0.9, None, SLIDE)?, t, 0.60 * g);
        put(mix, "drums", self.kick(1.0)?, t, 0.62 * g);
        put(mix, "drums", self.clap(1.0, 0)?, t, 0.40 * g);
        for &m in box_tones(ctx, ch)?.iter().take(3) {
            pan(mix, "musicbox", self.tine(m, 0.8, 1.0, DET)?, t, 0.14 * g, -0.3);
        }
        Ok(())
    }
}

impl Bed for HorrorTrapClassic {
    /// The mood's class defines `boom`, `hit` and `erupt`.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit, BedHook::Erupt]
    }

    /// `buses = {'808': (0.95, 0.04), 'drums': (1.20, 0.10), 'musicbox': (0.90, 0.50), 'strings': (0.60, 0.40),
    /// 'swell': (0.80, 0.35)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// A big, cold room.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.4, 0.6, 0.18)
    }

    fn comp(&self) -> (f64, f64) {
        (0.17, 0.58)
    }

    fn roar_gain(&self) -> f64 {
        0.14
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

    /// The 808 on the root of the chord at `t`.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        let r = self.root(ctx, ctx.harmony.chord_at(t))?;
        put(mix, "808", self.b808(r, 0.8, None, SLIDE)?, t, 0.5 * g);
        Ok(())
    }

    /// A full trap hit: the 808, kick, clap, a music-box chord. The kit's method has no `ring`: it is not used.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, _ring: f64) -> Played {
        self.trap_hit(ctx, mix, t, ch, g)
    }

    /// LIGHTS OUT erupts: the whole kit slams back in, a reversed swell into it (the BASS CLARINET's part calls it at
    /// the eruption).
    fn erupt(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let ch = ctx.harmony.chord_at(t);
        let sw = py_min(1.2, ctx.show.grid.beat * 2.0);
        put(mix, "swell", self.rev_swell(sw, None)?, t - sw, 0.22);
        self.trap_hit(ctx, mix, t, ch, 1.1)?;
        put(mix, "drums", synth::crash(2.0), t, 0.12);
        Ok(())
    }

    /// `mix.place('fx', self.hat(1.0, seed=1), t, 0.10)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", self.hat(1.0, false, 1)?, t, 0.10);
        Ok(())
    }

    /// Three full trap hits under the VS card's words, each with a crash.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let ch = chord(ctx, 0)?;
        for tw in times {
            self.trap_hit(ctx, mix, tw, ch, 1.0)?;
            put(mix, "drums", synth::crash(1.2), tw, 0.06);
        }
        Ok(())
    }

    /// VS: a thunder crack.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.40);
        Ok(())
    }

    /// Under the card (never quiet): the lullaby ticking, the strings creeping, the hats, the 808 sliding up.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let ch = chord(ctx, 0)?;
        let ch1 = chord(ctx, 1)?;
        let (bar1, _) = box_motif(&ctx.harmony, ch, ch1).map_err(bad)?;
        for (j, &m) in bar1.iter().enumerate() {
            let tj = j as f64 * beat / 2.0;
            pan(mix, "musicbox", self.tine(m, beat * 0.9, 1.0, DET)?, tj, 0.15, -0.3);
        }
        for j in 0..8u64 {
            let g = if j % 2 == 0 { 0.16 } else { 0.12 };
            let th = j as f64 * beat / 2.0 + 0.004;
            pan(mix, "drums", self.hat(1.0, false, j)?, th, g, 0.25);
        }
        let ms = self.cluster(ctx, ch)?;
        put(mix, "strings", self.strings(&ms, bar * 0.97, false)?, 0.0, 0.42);
        let r = self.root(ctx, ch)?;
        let to = self.root(ctx, ch1)? + 12;
        put(mix, "808", self.b808(r, beat * 0.9, Some(to), 0.2)?, 3.0 * beat, 0.42);
        Ok(())
    }

    /// Bar 1 (intensity 1.1), a clap on every count, the hats rolling up into FIGHT! and a reversed swell into it.
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        self.bar(ctx, mix, 1, 1.1, false)?;
        for (tc, _) in &grid.count[..3] {
            put(mix, "drums", self.clap(1.0, 1)?, *tc, 0.30);
        }
        let tri = beat / 6.0;
        for j in 0..6u64 {
            let jf = j as f64;
            let th = bar + 3.0 * beat + jf * tri;
            pan(mix, "drums", self.hat(1.0, false, j)?, th, 0.10 + 0.02 * jf, 0.25);
        }
        let sw = beat;
        let m = box_low(ctx, chord(ctx, 2)?)?;
        put(mix, "swell", self.rev_swell(sw, Some(m))?, grid.t_fight - sw, 0.20);
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        put(mix, "drums", synth::crash(1.8), t_fight, 0.12);
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, THUD_SEED), t_fight, 0.30);
        Ok(())
    }

    /// The trap beat from FIGHT! to the K.O. (thicker as it goes), a reversed swell and a crash into every loop
    /// (not in the dark), a swell into the K.O. and a tape stop of the bed's buses there.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let t_ko = ctx.show.t_ko;
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, 1.0 + 0.65 * (b as f64 / bar_ko as f64), false)?;
        }
        let w = Windows::of(ctx)?;
        for b in (2 + 4..=bar_ko).step_by(4) {
            let tb = b as f64 * bar;
            if !w.in_dark(tb) {
                let sw = beat * 1.5;
                put(mix, "swell", self.rev_swell(sw, None)?, tb - sw, 0.16);
                put(mix, "drums", synth::crash(1.4), tb, 0.06);
            }
        }
        // ... and into the K.O.
        let sw = py_min(1.4, beat * 2.5);
        let m = box_low(ctx, ctx.harmony.chord_at(t_ko))?;
        put(mix, "swell", self.rev_swell(sw, Some(m))?, t_ko - sw, 0.22);
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
        put(mix, "drums", synth::crash(2.4), t_ko, 0.16);
        put(mix, "fx", synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.6);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.trap_hit(ctx, mix, tw, chord(ctx, 0)?, 1.0)?;
        put(mix, "drums", synth::crash(1.8), tw, 0.10);
        Ok(())
    }

    /// A whoosh from just after the stinger to the solo, and the beat (intensity 1.2) in the bars between.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let bar = ctx.show.grid.bar;
        let dur = py_max(0.2, t_solo - tw - 0.1);
        put(
            mix,
            "fx",
            synth::whoosh(dur, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED),
            tw + 0.1,
            0.08,
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

    /// The beat (intensity 1.3) under the solo, up to the last bar.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            self.bar(ctx, mix, b, 1.3, false)?;
        }
        Ok(())
    }

    /// The last bar swells into the title slam: hats 16ths then 32nds, claps on 2 and 4, the 808 sliding up to the
    /// tonic, the music box spiralling up, a reversed swell and a riser right up to the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16, dur) = (grid.beat, grid.bar, grid.s16, grid.dur);
        let t_last = (grid.n_bars as f64 - 1.0) * bar;
        let ch = ctx.harmony.chord_at(t_last);
        let r = self.root(ctx, ch)?;
        let tgt = self.root(ctx, chord(ctx, 0)?)?;
        let to = tgt + if tgt < r { 12 } else { 0 };
        put(mix, "808", self.b808(r, bar - 0.02, Some(to), beat)?, t_last, 0.52);
        put(mix, "drums", self.kick(1.0)?, t_last, 0.62);
        put(mix, "drums", self.kick(1.0)?, t_last + 1.75 * beat, 0.44);
        for bt in [1u64, 3] {
            put(mix, "drums", self.clap(1.0, bt)?, t_last + bt as f64 * beat, 0.42);
        }
        for j in 0..8u64 {
            pan(
                mix,
                "drums",
                self.hat(1.0, false, j)?,
                t_last + j as f64 * s16,
                0.13,
                0.25,
            );
        }
        let s32 = s16 / 2.0;
        for j in 0..16u64 {
            let jf = j as f64;
            let th = t_last + 2.0 * beat + jf * s32;
            if th < dur - 0.01 {
                pan(mix, "drums", self.hat(1.0, false, j)?, th, 0.10 + 0.006 * jf, 0.25);
            }
        }
        let top = box_tones(ctx, ch)?;
        if top.is_empty() {
            return Err(MusicError::new(
                H,
                format!("{ch} has no tone from 88 to 100 (Python's ZeroDivisionError)"),
            ));
        }
        let k = top.len();
        for j in 0..8usize {
            let m = top[j % k] + 12 * (j / k) as i32;
            let tj = t_last + j as f64 * beat / 2.0;
            pan(
                mix,
                "musicbox",
                self.tine(m.min(103), beat * 0.5, 1.0, DET)?,
                tj,
                0.15,
                -0.3,
            );
        }
        let ms = self.cluster(ctx, ch)?;
        put(mix, "strings", self.strings(&ms, bar, true)?, t_last, 0.42);
        let sw = bar - 0.02;
        put(mix, "swell", self.rev_swell(sw, None)?, dur - sw, 0.20);
        put(mix, "fx", riser(dur - t_last, RISER_OVER, RISER_SEED)?, t_last, 0.16);
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
        // a clap keyed by seed % 2 is the first seed's noise; a hat by seed % 3
        let bed = HorrorTrapClassic::new();
        let first = bed.clap(1.0, 3).unwrap();
        assert_eq!(first, clap(3));
        assert_eq!(bed.clap(0.5, 1).unwrap(), scaled(&first, 0.5));
        assert_ne!(clap(1), clap(3));
        let h = bed.hat(1.0, false, 5).unwrap();
        assert_eq!(bed.hat(1.0, false, 2).unwrap(), h);
        assert_ne!(bed.hat(1.0, true, 2).unwrap(), h);
        // a swell with a tine leaves the tine in the cache under its own key, as the kit's does
        let sw = bed.rev_swell(0.3, Some(88)).unwrap();
        assert_eq!(sw, rev_swell(0.3, Some(&tine(88, 0.3, 0.25))));
        assert!(
            bed.cache
                .lock()
                .unwrap()
                .contains_key(&Key::Tine(88, r3(0.3), r3(0.25)))
        );
    }

    #[test]
    fn it_has_boom_hit_and_erupt() {
        let bed = HorrorTrapClassic::new();
        assert!(bed.has(BedHook::Boom) && bed.has(BedHook::Hit) && bed.has(BedHook::Erupt));
        assert!(!bed.has(BedHook::PressRoll) && !bed.has(BedHook::TrailerHit));
    }

    #[test]
    fn the_windows_follow_the_kits_rules() {
        let w = Windows {
            dark: vec![(10.0, 11.5)],
            mk: vec![(20.0, 22.0, vec![20.1, 20.5])],
        };
        assert!(w.in_dark(9.985) && w.in_dark(11.49) && !w.in_dark(11.5) && !w.in_dark(9.97));
        assert!(w.in_mk(20.0) && !w.in_mk(22.0) && !w.in_mk(19.99));
    }
}
