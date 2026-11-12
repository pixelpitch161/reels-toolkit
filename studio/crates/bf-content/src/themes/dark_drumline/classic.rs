//! The dark drumline's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/dark_drumline.py`,
//! section by section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a
//! bus in that order, and buses come into being in it).
//!
//! **The kit's cache.** The kit's bed keeps the snares, quads, bass drums, marimba bars and timpani strokes in a
//! dict (`self._c`) under `('sn', kind)`, `('tn', i)`, `('bd', int(m))`, `('ma', m)` and `('t', m)`. Every key holds
//! all of its sound's arguments and the bed always plays them at velocity 1, so here the cache only saves the work
//! (the choked cymbals are made anew each time, as in the kit).
//!
//! **The options.** `bass_drums = false` (`self.bd_on`) silences the bed's own five bass drums; `after_ko = true`
//! (`self.bd_tail`) turns them back on in `after_ko`, for the rest of the soundtrack (the kit sets `self.bd_on =
//! True` there, so the victory bars, the last bars and any `boom` a fighter lands after it have them).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bf_core::fightfile::OptionValue;
use bf_core::harmony::HarmonyError;
use bf_core::py::{py_min, round_half_even};
use bf_core::show::{CAP_GAP, captions};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, noise_seed, riser, thunder};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{BASS, SN, SN_LIGHT, TEN, bassdrum, bd_notes, choke, marimba, msnare, tenor, timp, troot};
use crate::error::ContentError;
use crate::themes::ThemeOptions;

/// The name the bed's errors carry.
const H: &str = "dark_drumline";

/// The bed's own buses, in the order the mood lists them.
const BUSES: [(&str, (f64, f64)); 2] = [("drums", (1.0, 0.18)), ("pit", (0.9, 0.30))];

/// A key of the kit's `self._c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('sn', kind)`
    Snare(char),
    /// `('tn', i)`
    Tenor(usize),
    /// `('bd', int(m))`
    BassDrum(i32),
    /// `('ma', m)`
    Marimba(i32),
    /// `('t', m)`
    Timp(i32),
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
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

/// `T.CHORDS[-1]`.
fn last_chord<'a>(ctx: &'a MusicCtx) -> Result<&'a str, MusicError> {
    match ctx.harmony.chords.len() {
        0 => Err(MusicError::new(H, "the episode has no chords")),
        n => chord(ctx, n - 1),
    }
}

/// `x[i]`, or the error Python's `IndexError` would be.
fn at(x: &[i32], i: usize, what: &str) -> Result<i32, MusicError> {
    x.get(i)
        .copied()
        .ok_or_else(|| MusicError::new(H, format!("{what} has {} notes, not {}", x.len(), i + 1)))
}

/// A `DRUMLINE` key of the fight's options: its value, or `default` when absent.
fn flag(options: &ThemeOptions, key: &str, default: bool) -> Result<bool, ContentError> {
    match options.get(key) {
        None => Ok(default),
        Some(OptionValue::Bool(b)) => Ok(*b),
        Some(_) => Err(ContentError::field(
            format!("music.theme_options.{key}"),
            "must be true or false",
        )),
    }
}

/// The classic bed for the fight's options (`DRUMLINE`'s `bass_drums`, default true, and `after_ko`, default
/// false).
///
/// # Errors
/// An option that is not true or false.
pub(super) fn classic(options: &ThemeOptions) -> Result<Box<dyn Bed>, ContentError> {
    let bd_on = flag(options, "bass_drums", true)?;
    let bd_tail = flag(options, "after_ko", false)?;
    Ok(Box::new(DarkDrumlineClassic::new(bd_on, bd_tail)))
}

/// The dark drumline's bed as the kit plays it.
#[derive(Debug)]
pub struct DarkDrumlineClassic {
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
    /// `self.bd_on`: whether the bed's own bass drums play (`after_ko` may turn them on).
    bd_on: AtomicBool,
    /// `self.bd_tail`: whether they come back from the K.O. on.
    bd_tail: bool,
}

impl Default for DarkDrumlineClassic {
    fn default() -> DarkDrumlineClassic {
        DarkDrumlineClassic::new(true, false)
    }
}

impl DarkDrumlineClassic {
    /// The bed for `DRUMLINE = dict(bass_drums=bd_on, after_ko=bd_tail)`, with nothing cached yet.
    pub fn new(bd_on: bool, bd_tail: bool) -> DarkDrumlineClassic {
        DarkDrumlineClassic {
            cache: Mutex::new(BTreeMap::new()),
            bd_on: AtomicBool::new(bd_on),
            bd_tail,
        }
    }

    fn bd_on(&self) -> bool {
        self.bd_on.load(Ordering::SeqCst)
    }

    /// The cached sound for `key`, made by `make` the first time.
    fn cached(&self, key: Key, make: impl FnOnce() -> Vec<f64>) -> Vec<f64> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        let x = Arc::clone(cache.entry(key).or_insert_with(|| Arc::new(make())));
        (*x).clone()
    }

    fn snare(&self, kind: char) -> Vec<f64> {
        self.cached(Key::Snare(kind), || msnare(kind))
    }

    fn tenor(&self, i: usize) -> Vec<f64> {
        self.cached(Key::Tenor(i), || tenor(i))
    }

    fn bassdrum(&self, m: i32) -> Vec<f64> {
        self.cached(Key::BassDrum(m), || bassdrum(m))
    }

    fn marimba_of(&self, m: i32) -> Vec<f64> {
        self.cached(Key::Marimba(m), || marimba(m))
    }

    fn timp_of(&self, m: i32) -> Vec<f64> {
        self.cached(Key::Timp(m), || timp(m))
    }

    /// `self.bd_notes(ch)`.
    fn bd(&self, ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        bd_notes(&ctx.harmony, ch).map_err(bad)
    }

    /// `self.troot(ch)`.
    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        troot(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.cadence(mix, b, dense, light)`: one bar of the drumline and the pit (bar index `b`, its chord
    /// `T.CHORDS[b % T.N_BARS]`). `dense` from 0 to 1 thickens the parts (never the level).
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony cannot voice, a bass drum split the chord's drums lack.
    pub fn cadence(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, dense: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (s16, beat) = (grid.s16, grid.beat);
        let t0 = b as f64 * grid.bar;
        if grid.n_bars == 0 {
            return Err(MusicError::new(H, "the episode has no bars"));
        }
        let ch = chord(ctx, b % grid.n_bars)?;
        let phrase = if b >= 2 { (b - 2) % 4 } else { b };
        // 'fill' on a phrase's last bar, else 'a' or 'b'
        let pk = if phrase == 3 { 2 } else { phrase % 2 };
        let sn = if light || dense < 0.3 { SN_LIGHT[pk] } else { SN[pk] };
        for (i, c) in sn.chars().enumerate() {
            let ts = t0 + i as f64 * s16;
            match c {
                '.' => {}
                'r' => {
                    // a diddle: two 32nds
                    for j in 0..2usize {
                        let g = 0.30 * if j == 0 { 1.0 } else { 0.85 };
                        put(mix, "drums", self.snare('t'), ts + j as f64 * s16 / 2.0, g, 0.05);
                    }
                }
                'f' => {
                    // a flam: the grace 18 ms early
                    put(mix, "drums", self.snare('g'), ts - 0.018, 0.30, 0.05);
                    put(mix, "drums", self.snare('A'), ts, 0.30, 0.05);
                }
                c => put(mix, "drums", self.snare(c), ts, 0.30, 0.05),
            }
        }
        if !light {
            let ten = TEN[usize::from(dense > 0.55)];
            for (i, c) in ten.chars().enumerate() {
                if let Some(d) = c.to_digit(10) {
                    let d = d as usize;
                    let pan = -0.25 + 0.15 * d as f64;
                    put(mix, "drums", self.tenor(d - 1), t0 + i as f64 * s16, 0.10, pan);
                }
            }
        }
        let bd = self.bd(ctx, ch)?;
        let bp = if self.bd_on() {
            BASS[usize::from(dense > 0.4 && !light)]
        } else {
            ""
        };
        for (i, c) in bp.chars().enumerate() {
            let ts = t0 + i as f64 * s16;
            if c == 'U' {
                for &m in &bd {
                    put(mix, "drums", self.bassdrum(m), ts, 0.10, 0.0);
                }
            } else if let Some(d) = c.to_digit(10) {
                let d = d as usize;
                let m = at(&bd, d - 1, "the bass drums")?;
                put(mix, "drums", self.bassdrum(m), ts, 0.22, -0.3 + 0.15 * d as f64);
            }
        }
        if phrase == 0 {
            put(
                mix,
                "drums",
                synth::crash(1.4),
                t0,
                if light { 0.05 } else { 0.07 },
                0.0,
            );
        }
        if dense > 0.65 && !light {
            // cymbal crunches on 2 and 4
            for bt in [1usize, 3] {
                let seed = (b * 4 + bt) as u64;
                put(mix, "drums", choke(seed), t0 + bt as f64 * beat, 0.07, 0.0);
            }
        }
        // the pit: marimba 8ths and timpani roots
        let tones = ctx.harmony.chord_tones(72, 91, ch).map_err(bad)?;
        if tones.is_empty() {
            return Err(MusicError::new(
                H,
                format!("{ch} has no tone from C5 to G6 for the marimba"),
            ));
        }
        for e8 in 0..8usize {
            let m = tones[(e8 * 2 + e8 / 4) % tones.len()];
            let g = if e8 % 2 == 0 { 0.06 } else { 0.045 };
            put(mix, "pit", self.marimba_of(m), t0 + e8 as f64 * beat / 2.0, g, 0.3);
        }
        let root = self.root(ctx, ch)?;
        put(mix, "pit", self.timp_of(root), t0, 0.30, 0.0);
        if dense > 0.5 && !light {
            let fifth = if root + 7 <= 50 { root + 7 } else { root - 5 };
            put(mix, "pit", self.timp_of(fifth), t0 + 2.0 * beat, 0.22, 0.0);
        }
        Ok(())
    }

    /// `Bed.unison(mix, t, ch, g)`: a drumline unison hit: every bass drum (when on), an accent snare, the quads, a
    /// crash, the timpani and a low thud.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony cannot voice, a time below zero (the thud's seed).
    pub fn unison(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64) -> Played {
        if self.bd_on() {
            for m in self.bd(ctx, ch)? {
                put(mix, "drums", self.bassdrum(m), t, 0.14 * g, 0.0);
            }
        }
        put(mix, "drums", self.snare('A'), t, 0.34 * g, 0.0);
        for i in 0..4usize {
            put(mix, "drums", self.tenor(i), t, 0.08 * g, 0.0);
        }
        put(mix, "drums", synth::crash(1.4), t, 0.08 * g, 0.0);
        let root = self.root(ctx, ch)?;
        put(mix, "pit", self.timp_of(root), t, 0.5 * g, 0.0);
        let seed = noise_seed(H, t * 50.0)? + 3;
        put(mix, "fx", synth::thud(0.7, 110.0, 34.0, seed), t, 0.30 * g, 0.0);
        Ok(())
    }

    /// `Bed.roll(mix, t0, t1, g0, g1)`: a snare diddle roll on 32nds from `t0` to `t1`, a crescendo from `g0` to
    /// `g1`.
    ///
    /// # Errors
    /// Where Python raises: a length that is not a number.
    pub fn roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played {
        let s16 = ctx.show.grid.s16;
        // max(1, int((t1 - t0) / (S16 / 2)))
        let q = (t1 - t0) / (s16 / 2.0);
        if !q.is_finite() {
            return Err(MusicError::new(H, format!("a roll from {t0} to {t1} s")));
        }
        let n = (q as i64).max(1);
        for j in 0..n {
            let u = j as f64 / (n - 1).max(1) as f64;
            let t = t0 + j as f64 * s16 / 2.0;
            put(mix, "drums", self.snare('t'), t, g0 + (g1 - g0) * u, 0.05);
        }
        Ok(())
    }
}

impl Bed for DarkDrumlineClassic {
    /// The mood's class defines `boom` and `press_roll` and none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::PressRoll]
    }

    /// `buses = {'drums': (1.0, 0.18), 'pit': (0.9, 0.30)}`.
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    fn reverb(&self) -> (f64, f64, f64) {
        (2.0, 0.5, 0.22)
    }

    fn comp(&self) -> (f64, f64) {
        (0.20, 0.5)
    }

    fn roar_gain(&self) -> f64 {
        0.22
    }

    fn doors_gain(&self) -> f64 {
        0.60
    }

    fn landing(&self) -> (f64, f64) {
        (0.08, 0.06)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 102.0, 0.12)
    }

    /// A unison hit on the chord at `t`, at `0.8 * g`.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        self.unison(ctx, mix, t, ctx.harmony.chord_at(t), 0.8 * g)
    }

    /// A fighter's fanfare roll: the snare roll.
    fn press_roll(&self, ctx: &MusicCtx, mix: &mut Mix, t0: f64, t1: f64, g0: f64, g1: f64) -> Played {
        self.roll(ctx, mix, t0, t1, g0, g1)
    }

    /// The bed has both `timp` and `marimba`.
    fn has_pit(&self) -> bool {
        true
    }

    fn timp(&self, m: i32) -> Result<Vec<f64>, MusicError> {
        if m + 5 < 0 {
            return Err(MusicError::new(H, format!("a timpani note of {m}")));
        }
        Ok(self.timp_of(m))
    }

    fn marimba(&self, m: i32) -> Result<Vec<f64>, MusicError> {
        if m < 0 {
            return Err(MusicError::new(H, format!("a marimba note of {m}")));
        }
        Ok(self.marimba_of(m))
    }

    /// `mix.place('fx', self.A.thud(0.15, 200, 90, seed=int(t * 3)), t, 0.06)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let thud = synth::thud(0.15, 200.0, 90.0, noise_seed(H, t * 3.0)?);
        put(mix, "fx", thud, t, 0.06, 0.0);
        Ok(())
    }

    /// Bar 0: the cadence at full level and a unison hit under each word of the VS card.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        self.cadence(ctx, mix, 0, 0.5, false)?;
        let ch = chord(ctx, 0)?;
        for (g, tw) in [0.75, 0.7, 0.72].into_iter().zip(times) {
            self.unison(ctx, mix, tw, ch, g)?;
        }
        Ok(())
    }

    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.4, 91, THUNDER_CRACK)?, t, 0.55, 0.0);
        Ok(())
    }

    /// Beat 4: a snare roll as the doors open, and a whoosh.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.roll(ctx, mix, 3.0 * grid.beat, grid.bar - 0.02, 0.10, 0.30)?;
        let (d0, d1) = grid.t_doors;
        let w = synth::whoosh(d1 - d0 + 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, d0 - 0.05, 0.14, 0.0);
        Ok(())
    }

    /// Bar 1: the snare line's light cadence on the count chord, a unison hit on each count, a roll into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        self.cadence(ctx, mix, 1, 0.3, true)?;
        let ch = chord(ctx, 1)?;
        for (tc, _) in &grid.count[..3] {
            self.unison(ctx, mix, *tc, ch, 0.75)?;
        }
        let t = grid.bar + 3.0 * grid.beat;
        self.roll(ctx, mix, t, grid.t_fight - 0.02, 0.12, 0.34)?;
        let w = synth::whoosh(grid.beat, 400.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", w, t, 0.12, 0.0);
        Ok(())
    }

    /// The drop: a unison hit on the third chord and a thunder.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        self.unison(ctx, mix, t_fight, chord(ctx, 2)?, 1.1)?;
        put(mix, "fx", thunder(1.8, 7, THUNDER_CRACK)?, t_fight, 0.36, 0.0);
        Ok(())
    }

    /// The cadence from FIGHT! to the K.O., thickening; a unison hit under every captioned ability (the story's
    /// captions: `M.P.STORY['caps']`); a tape stop of the drums and the pit at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in 2..=bar_ko {
            // min(1.0, 0.45 + (b - 2) / 24.0)
            let dense = py_min(1.0, 0.45 + (b as f64 - 2.0) / 24.0);
            self.cadence(ctx, mix, b, dense, false)?;
        }
        for (t, _, _) in captions(&ctx.rec.events, CAP_GAP) {
            self.unison(ctx, mix, ctx.q16(t), ctx.harmony.chord_at(t), 0.6)?;
        }
        for (bus, _) in BUSES {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        self.unison(ctx, mix, t_ko, ctx.harmony.chord_at(t_ko), 1.0)?;
        put(mix, "fx", thunder(2.4, 51, THUNDER_CRACK)?, t_ko, 0.50, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.unison(ctx, mix, tw, ctx.harmony.chord_at(tw), 0.9)
    }

    /// The drumline holds its breath (never quiet) until the solo; with `after_ko` its bass drums are back from
    /// here on.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, _tw: f64, t_solo: f64) -> Played {
        if self.bd_tail {
            self.bd_on.store(true, Ordering::SeqCst);
        }
        // range(bar_ko + 1, int(round(t_solo / T.BAR)))
        let end = round_half_even(t_solo / ctx.show.grid.bar);
        let end = if end > 0.0 { end as usize } else { 0 };
        for b in bar_ko + 1..end {
            self.cadence(ctx, mix, b, 0.6, false)?;
        }
        Ok(())
    }

    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(2) {
            self.cadence(ctx, mix, b, 0.85, false)?;
        }
        Ok(())
    }

    /// The last two bars build into the title slam: the full cadence, the bass drums on every beat of the last bar
    /// on its chord, a snare roll and a riser into the seam.
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let n = grid.n_bars;
        if n < 2 {
            return Err(MusicError::new(
                H,
                format!("the episode has {n} bars, fewer than the last two"),
            ));
        }
        for b in [n - 2, n - 1] {
            self.cadence(ctx, mix, b, 1.0, false)?;
        }
        let tl = (n - 1) as f64 * grid.bar;
        for bt in 0..4usize {
            if self.bd_on() {
                for m in self.bd(ctx, last_chord(ctx)?)? {
                    put(mix, "drums", self.bassdrum(m), tl + bt as f64 * grid.beat, 0.09, 0.0);
                }
            }
        }
        self.roll(ctx, mix, tl + 2.0 * grid.beat, grid.dur - 1e-3, 0.16, 0.48)?;
        put(mix, "fx", riser(grid.dur - tl, RISER_OVER, RISER_SEED)?, tl, 0.30, 0.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_hands_back_the_sound_made_anew() {
        let bed = DarkDrumlineClassic::default();
        assert_eq!(bed.snare('A'), msnare('A'));
        assert_eq!(bed.snare('A'), msnare('A'));
        assert_eq!(bed.bassdrum(43), bassdrum(43));
        assert_eq!(Bed::timp(&bed, 36).unwrap(), timp(36));
        assert_eq!(Bed::marimba(&bed, 84).unwrap(), marimba(84));
        assert_eq!(bed.cache.lock().unwrap().len(), 4);
    }

    #[test]
    fn its_hooks_and_its_pit() {
        let bed = DarkDrumlineClassic::default();
        assert!(bed.has(BedHook::Boom) && bed.has(BedHook::PressRoll) && !bed.has(BedHook::Hit));
        assert!(bed.has_pit());
    }

    #[test]
    fn the_options_set_the_bass_drums() {
        let mut o = ThemeOptions::new();
        let on = |o: &ThemeOptions| {
            let b = DarkDrumlineClassic::new(
                flag(o, "bass_drums", true).unwrap(),
                flag(o, "after_ko", false).unwrap(),
            );
            (b.bd_on(), b.bd_tail)
        };
        assert_eq!(on(&o), (true, false));
        o.insert("bass_drums".into(), OptionValue::Bool(false));
        o.insert("after_ko".into(), OptionValue::Bool(true));
        assert_eq!(on(&o), (false, true));
        o.insert("after_ko".into(), OptionValue::Str("yes".into()));
        assert_eq!(
            classic(&o).err().map(|e| e.to_string()),
            Some("music.theme_options.after_ko: must be true or false".to_string())
        );
    }
}
