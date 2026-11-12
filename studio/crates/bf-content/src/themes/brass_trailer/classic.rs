//! The brass trailer's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/brass_trailer.py`,
//! section by section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a
//! bus in that order), with the sequel's additions (`TRAILER = dict(sequel=True)`) where the kit makes them.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_min, round_half_even, round_nd};
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, MusicCtx, MusicError, RISER_OVER, RISER_SEED, clank, noise_seed, riser,
};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{
    BRAAM_SWELL, BRAAM_VEL, OST, STRINGS_LOW_VEL, braam, pad, pad_notes, root, strings_low, sub, taiko, tick, trem,
    tremolo,
};

/// The name the bed's errors carry.
const H: &str = "brass_trailer";

/// `buses` of #34: `(gain, reverb send)`; `pad` plays only in the sequel.
const BUSES: [(&str, (f64, f64)); 5] = [
    ("strings", (0.85, 0.3)),
    ("taiko", (1.2, 0.25)),
    ("braam", (1.0, 0.3)),
    ("tremolo", (1.0, 0.6)),
    ("pad", (0.9, 0.35)),
];

/// The sequel's `buses`: `dict(self.buses, braam=(1.15, 0.32), taiko=(1.3, 0.25))` (the order stays #34's).
const SEQUEL_BUSES: [(&str, (f64, f64)); 5] = [
    ("strings", (0.85, 0.3)),
    ("taiko", (1.3, 0.25)),
    ("braam", (1.15, 0.32)),
    ("tremolo", (1.0, 0.6)),
    ("pad", (0.9, 0.35)),
];

/// A key of the bed's sound cache. `Sub` and `Pad` are the kit's own `self._c` keys (`('sub', root, round(dur,
/// 3))`, `('pad', tuple(ms), round(dur, 3))`: a later call whose key matches gets the first call's sound); the
/// others hold every argument exactly (the kit makes those anew on each call, which gives the same sound), so they
/// only save time. Floats by their bits (`-0.0` as `0.0` in the kit's keys, which Python's dict takes for the same
/// key).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Sub(i32, u64),
    Pad(Vec<i32>, u64),
    Strings(i32, u64, u64),
    Tremolo(Vec<i32>, u64, u64),
    Braam(i32, u64, u64, u64),
    Taiko(bool, u64),
    Tick(u64),
}

fn bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

/// The brass trailer's bed as the kit plays it, plain (#34) or as the sequel (#54).
#[derive(Debug)]
pub struct BrassTrailerClassic {
    /// `self.seq`: `TRAILER.sequel`.
    seq: bool,
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
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

/// `self.key_root()`: `T.KEY_ROOT`, the key's root in the C2 octave (D is 38).
fn key_root(ctx: &MusicCtx) -> i32 {
    ctx.harmony.key_root
}

/// The braam's root for a bar's root: `r0 - 12 if r0 > 30 else r0`.
fn low(r0: i32) -> i32 {
    if r0 > 30 { r0 - 12 } else { r0 }
}

/// A seed the fighters or the bed pass as a Python int: numpy refuses one below zero.
fn seed_of(seed: i64) -> Result<u64, MusicError> {
    u64::try_from(seed).map_err(|_| MusicError::new(H, format!("{seed} is not a noise seed")))
}

fn put(mix: &mut Mix, bus: &str, x: &[f64], t: f64, gain: f64) {
    mix.place(bus, &Sig::Mono(x.to_vec()), t, gain, 0.0);
}

/// `x * vel` for every sample.
fn scaled(x: &[f64], vel: f64) -> Vec<f64> {
    x.iter().map(|v| v * vel).collect()
}

impl BrassTrailerClassic {
    /// The bed, plain (`sequel` false: #34's, note for note) or as the sequel, with nothing cached yet.
    pub fn new(sequel: bool) -> BrassTrailerClassic {
        BrassTrailerClassic {
            seq: sequel,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// The cached sound for `key`, made by `make` the first time.
    fn cached(&self, key: Key, make: impl FnOnce() -> Vec<f64>) -> Arc<Vec<f64>> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(x) = cache.get(&key) {
            return Arc::clone(x);
        }
        let x = Arc::new(make());
        cache.insert(key, Arc::clone(&x));
        x
    }

    /// `self.strings_low(m, dur, vel)`.
    fn strings_low(&self, m: i32, dur: f64, vel: f64) -> Arc<Vec<f64>> {
        self.cached(Key::Strings(m, dur.to_bits(), vel.to_bits()), || {
            strings_low(m, dur, vel)
        })
    }

    /// `self.tremolo(ms, dur, att)`.
    fn tremolo(&self, ms: &[i32], dur: f64, att: f64) -> Arc<Vec<f64>> {
        self.cached(Key::Tremolo(ms.to_vec(), dur.to_bits(), att.to_bits()), || {
            tremolo(ms, dur, att)
        })
    }

    /// `self.braam(root, dur, vel, swell)`.
    fn braam(&self, root: i32, dur: f64, vel: f64, swell: f64) -> Arc<Vec<f64>> {
        let key = Key::Braam(root, dur.to_bits(), vel.to_bits(), swell.to_bits());
        self.cached(key, || braam(root, dur, vel, swell))
    }

    /// `self.taiko(vel, big, seed)`.
    fn taiko(&self, vel: f64, big: bool, seed: u64) -> Vec<f64> {
        scaled(&self.cached(Key::Taiko(big, seed), || taiko(big, seed)), vel)
    }

    /// `self.tick(vel, seed)`.
    fn tick(&self, vel: f64, seed: u64) -> Vec<f64> {
        scaled(&self.cached(Key::Tick(seed), || tick(seed)), vel)
    }

    /// `self.sub(root, dur)`: the kit's cache, keyed `('sub', root, round(dur, 3))`.
    fn sub(&self, root: i32, dur: f64) -> Arc<Vec<f64>> {
        self.cached(Key::Sub(root, bits(round_nd(dur, 3))), || sub(root, dur))
    }

    /// `self.pad(ms, dur)`: the kit's cache, keyed `('pad', tuple(ms), round(dur, 3))`.
    fn pad(&self, ms: &[i32], dur: f64) -> Arc<Vec<f64>> {
        self.cached(Key::Pad(ms.to_vec(), bits(round_nd(dur, 3))), || pad(ms, dur))
    }

    /// `self.trem(ch)`.
    fn trem(ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        trem(&ctx.harmony, ch).map_err(bad)
    }

    /// `self.pad_notes(ch)`.
    fn pad_notes(ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        pad_notes(&ctx.harmony, ch).map_err(bad)
    }

    /// `Bed.bar(mix, b, intensity, light)`: one bar of the trailer bed (bar index `b`, its chord `T.CHORDS[b]`):
    /// the low-string ostinato, the taiko (denser as `intensity` grows), a braam on every loop's downbeat, the
    /// high tremolo and the ticking; a `light` bar (under the solo) thins them. The sequel adds its own bar.
    ///
    /// # Errors
    /// Where Python raises: a bar past the last, a chord the harmony cannot voice.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let s16 = grid.s16;
        let t0 = b as f64 * grid.bar;
        let ch = chord(ctx, b)?;
        let r0 = root_of(ctx, ch)?;
        let r = if r0 < 36 { r0 + 12 } else { r0 };
        // low strings: spiccato 16ths
        for (st, iv) in OST.into_iter().enumerate() {
            if light && st % 2 == 1 {
                continue;
            }
            let acc = if st % 4 == 0 { 1.0 } else { 0.7 };
            let g = 0.11 * if light { 0.8 } else { 1.0 };
            put(
                mix,
                "strings",
                &self.strings_low(r + iv, s16 * 0.9, acc),
                t0 + st as f64 * s16,
                g,
            );
        }
        // taiko: the trailer pattern, denser as the fight builds
        let mut pat = vec![(0.0, 1.0, true), (2.0, 0.85, false)];
        if intensity > 1.15 {
            pat.extend([(1.5, 0.6, false), (3.5, 0.7, false)]);
        }
        if intensity > 1.4 && !light {
            pat.extend([(2.75, 0.5, false), (3.0, 0.65, false), (3.25, 0.55, false)]);
        }
        for (bt, v, big) in pat {
            // seed=int(bt * 4) + b
            let seed = noise_seed(H, bt * 4.0)? + b as u64;
            let g = if light { 0.30 } else { 0.42 };
            put(mix, "taiko", &self.taiko(v, big, seed), t0 + bt * grid.beat, g);
        }
        if (b as i64 - 1).rem_euclid(4) == 0 && !light {
            // braam on every loop's downbeat
            let vel = 0.8 + 0.2 * py_min(1.0, intensity - 1.0);
            put(mix, "braam", &self.braam(low(r0), 1.7, vel, BRAAM_SWELL), t0, 0.20);
        }
        if intensity > 1.3 || light {
            // high string tremolo: the back half's tension
            let k = if light {
                1.0
            } else {
                py_min(1.0, (intensity - 1.3) / 0.3)
            };
            let x = self.tremolo(&Self::trem(ctx, ch)?, grid.bar + 0.05, 0.3);
            put(mix, "tremolo", &x, t0, 0.05 * k);
        }
        if intensity > 1.2 && !light {
            // ticking 8ths
            for st in (0..16usize).step_by(2) {
                let vel = 0.6 + 0.4 * if st % 4 == 0 { 1.0 } else { 0.0 };
                put(mix, "fx", &self.tick(vel, st as u64), t0 + st as f64 * s16, 0.04);
            }
        }
        if self.seq {
            self.sequel_bar(ctx, mix, b, intensity, light)?;
        }
        Ok(())
    }

    /// `Bed.sequel_bar(mix, b, intensity, light)`: #54's same bar, darker and bigger: a contrabass octave under the
    /// 8ths, the cello pad, braams and subs on every second bar, the taiko ensemble and a war-drum fill into each
    /// loop.
    ///
    /// # Errors
    /// As [`BrassTrailerClassic::bar`].
    pub fn sequel_bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, intensity: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let t0 = b as f64 * grid.bar;
        let ch = chord(ctx, b)?;
        let r0 = root_of(ctx, ch)?;
        for st in (0..16usize).step_by(2) {
            // a contrabass octave under the 8ths
            let acc = if st % 4 == 0 { 1.0 } else { 0.75 };
            let g = 0.09 * if light { 0.8 } else { 1.0 };
            put(
                mix,
                "strings",
                &self.strings_low(r0, s16 * 1.6, acc),
                t0 + st as f64 * s16,
                g,
            );
        }
        let g = if light { 0.06 } else { 0.07 };
        put(mix, "pad", &self.pad(&Self::pad_notes(ctx, ch)?, grid.bar), t0, g);
        let odd = b % 2 == 1;
        if odd && (b as i64 - 1).rem_euclid(4) != 0 && !light {
            // braams on every second bar (and #34's)
            put(mix, "braam", &self.braam(low(r0), 1.9, 0.9, BRAAM_SWELL), t0, 0.16);
        }
        if odd && !light {
            // a sub under every braam
            put(mix, "braam", &self.sub(low(r0), 1.9), t0, 0.22);
        }
        if !light {
            // the ensemble's big hit on 3 from the start
            put(
                mix,
                "taiko",
                &self.taiko(0.9, true, 300 + b as u64),
                t0 + 2.0 * beat,
                0.30,
            );
            if intensity <= 1.4 {
                // the roll into 4 from the start
                for (j, bt) in [2.75, 3.0, 3.25].into_iter().enumerate() {
                    let x = self.taiko(0.45 + 0.1 * j as f64, false, 310 + j as u64 + b as u64);
                    put(mix, "taiko", &x, t0 + bt * beat, 0.30);
                }
            }
            let nb = ctx.harmony.loop_.len() as i64;
            if (b as i64 - ctx.harmony.loop_start as i64).rem_euclid(nb) == nb - 1 {
                // a war-drum fill into each loop
                for j in 0..8usize {
                    let x = self.taiko(0.5 + 0.06 * j as f64, false, 330 + j as u64);
                    put(mix, "taiko", &x, t0 + 3.0 * beat + j as f64 * s16 / 2.0, 0.30);
                }
            }
        }
        Ok(())
    }
}

impl Bed for BrassTrailerClassic {
    /// The mood's class defines `boom` and `trailer_hit` and none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::TrailerHit]
    }

    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        if self.seq { &SEQUEL_BUSES } else { &BUSES }
    }

    /// #34's reelkit default hall, wet 0.22; the sequel's is darker: `(2.8, 0.62, 0.24)`.
    fn reverb(&self) -> (f64, f64, f64) {
        if self.seq { (2.8, 0.62, 0.24) } else { (2.4, 0.6, 0.22) }
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

    /// A slam landing (PEDAL TONE's bell): #34's stomp, a big taiko and a braam:
    /// `self.trailer_hit(mix, t, 0.7 * g / 0.8, 90, 1.2, 0.30 * g / 0.8)`.
    fn boom(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        self.trailer_hit(ctx, mix, t, 0.7 * g / 0.8, 90, Some(1.2), 0.30 * g / 0.8)
    }

    /// A heavy hit: a big taiko (and a braam on the key's root when `braam_dur` is given and not zero): #34's
    /// flick, rope and stomp hits.
    fn trailer_hit(
        &self,
        ctx: &MusicCtx,
        mix: &mut Mix,
        t: f64,
        taiko_g: f64,
        seed: i64,
        braam: Option<f64>,
        braam_g: f64,
    ) -> Played {
        put(mix, "taiko", &self.taiko(1.0, true, seed_of(seed)?), t, taiko_g);
        if let Some(dur) = braam.filter(|&d| d != 0.0) {
            put(
                mix,
                "braam",
                &self.braam(key_root(ctx) - 12, dur, 1.0, 0.005),
                t,
                braam_g,
            );
        }
        Ok(())
    }

    /// `mix.place('fx', self.M.clank(79, int(t * 3)), t, 0.04)`.
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let x = clank(79.0, noise_seed(H, t * 3.0)?, CLANK_DUR, &CLANK_DECAYS)?;
        put(mix, "fx", &x, t, 0.04);
        Ok(())
    }

    /// One unison band hit per word: a taiko, a braam stab, a boom, low strings (and the sequel's sub).
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let kr = key_root(ctx);
        for tw in times {
            let seed = noise_seed(H, tw * 100.0)? + 7;
            put(mix, "taiko", &self.taiko(1.0, true, seed), tw, 0.55);
            put(mix, "braam", &self.braam(kr - 12, 0.55, 1.0, 0.008), tw, 0.22);
            let seed = noise_seed(H, tw * 50.0)? + 3;
            put(mix, "fx", &synth::thud(0.9, 120.0, 32.0, seed), tw, 0.45);
            put(mix, "strings", &self.strings_low(kr, 0.4, 1.0), tw, 0.18);
            if self.seq {
                put(mix, "braam", &self.sub(kr - 12, 0.8), tw, 0.22);
            }
        }
        Ok(())
    }

    /// A thunder crack where the middle word splits the divider (in the sequel REMATCH lands with a braam).
    fn vs(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let nn = (1.4 * bf_sound::SRF) as usize;
        let tt = synth::tt(nn);
        let crack = synth::bp(&synth::noise(91, nn), 1500.0, 9000.0, synth::FILTER_O);
        let rumble = synth::lp(&synth::noise(92, nn), 160.0, synth::FILTER_O);
        let x: Vec<f64> = (0..nn)
            .map(|i| {
                let t = tt[i];
                let crack = crack[i] * bf_core::dmath::exp(-t / 0.035);
                let rumble = rumble[i] * synth::clip(t / 0.02, 0.0, 1.0) * bf_core::dmath::exp(-t / 0.5) * 3.0;
                crack * 0.9 + rumble
            })
            .collect();
        put(mix, "fx", &x, t, 0.40);
        put(mix, "fx", &synth::crash(1.6), t, 0.10);
        if self.seq {
            let kr = key_root(ctx);
            put(mix, "braam", &self.braam(kr - 12, 1.4, 1.0, 0.01), t, 0.26);
            put(mix, "braam", &self.sub(kr - 12, 1.4), t, 0.26);
        }
        Ok(())
    }

    /// Beat 4: the hold and a taiko pickup fill as the doors open; a string tremolo under the whole card (and the
    /// sequel's pad).
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let tonic = chord(ctx, 0)?;
        let x = self.tremolo(&Self::trem(ctx, tonic)?, 3.0 * grid.beat + 0.1, 0.05);
        put(mix, "tremolo", &x, 0.0, 0.07);
        for (st, v) in [(12u64, 0.5), (13, 0.62), (14, 0.74), (15, 0.86)] {
            put(mix, "taiko", &self.taiko(v, false, 40 + st), st as f64 * grid.s16, 0.36);
        }
        if self.seq {
            put(
                mix,
                "pad",
                &self.pad(&Self::pad_notes(ctx, tonic)?, grid.bar),
                0.0,
                0.07,
            );
        }
        Ok(())
    }

    /// Bar 1, on the dominant: a taiko, a braam stab and low strings per count, the taiko pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let ch = chord(ctx, 1)?;
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            let tc = *tc;
            put(mix, "taiko", &self.taiko(1.0, true, 50 + j as u64), tc, 0.5);
            put(mix, "braam", &self.braam(root_of(ctx, ch)?, 0.42, 0.9, 0.01), tc, 0.18);
            let m = ctx.harmony.root_above(ch, 45).map_err(bad)?;
            put(mix, "strings", &self.strings_low(m, 0.3, STRINGS_LOW_VEL), tc, 0.14);
        }
        for (st, v) in [(12u64, 0.5), (13, 0.65), (14, 0.8), (15, 0.95)] {
            let t = grid.bar + st as f64 * grid.s16;
            put(mix, "taiko", &self.taiko(v, false, 60 + st), t, 0.40);
        }
        if self.seq {
            put(
                mix,
                "pad",
                &self.pad(&Self::pad_notes(ctx, ch)?, grid.bar),
                grid.bar,
                0.07,
            );
        }
        Ok(())
    }

    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_fight = ctx.show.grid.t_fight;
        let kr = key_root(ctx);
        put(
            mix,
            "braam",
            &self.braam(kr - 12, 2.2, BRAAM_VEL, BRAAM_SWELL),
            t_fight,
            0.30,
        );
        put(mix, "taiko", &self.taiko(1.0, true, 70), t_fight, 0.55);
        put(mix, "fx", &synth::crash(1.6), t_fight, 0.10);
        if self.seq {
            put(mix, "braam", &self.sub(kr - 12, 2.2), t_fight, 0.24);
        }
        Ok(())
    }

    /// The trailer from FIGHT! to the K.O., building bar by bar; a whoosh into the end; a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in 2..=bar_ko {
            // 1.0 + 0.6 * (b / bar_ko)
            self.bar(ctx, mix, b, 1.0 + 0.6 * (b as f64 / bar_ko as f64), false)?;
        }
        let whoosh = synth::whoosh(3.6, 300.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", &whoosh, 44.0 - 3.6, 0.10);
        for bus in ["strings", "taiko", "braam", "tremolo", "pad"] {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        put(mix, "taiko", &self.taiko(1.0, true, 99), t_ko, 0.6);
        put(
            mix,
            "braam",
            &self.braam(key_root(ctx) - 12, 1.6, 1.0, 0.005),
            t_ko,
            0.28,
        );
        put(mix, "fx", &synth::thud(0.9, 120.0, 30.0, THUD_SEED), t_ko, 0.7);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        put(
            mix,
            "braam",
            &self.braam(key_root(ctx) - 12, 1.4, 0.8, BRAAM_SWELL),
            tw,
            0.20,
        );
        put(mix, "taiko", &self.taiko(1.0, true, 101), tw, 0.45);
        Ok(())
    }

    /// A whoosh to the solo; in the sequel no gap: the ostinato restarts under the stinger on the tonic, a pad and
    /// a tremolo hold it, a taiko on every beat.
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, _bar_ko: usize, tw: f64, t_solo: f64) -> Played {
        let grid = &ctx.show.grid;
        let whoosh = synth::whoosh(t_solo - tw - 0.1, 300.0, 5000.0, WHOOSH_RISE, WHOOSH_SEED);
        put(mix, "fx", &whoosh, tw + 0.1, 0.10);
        if self.seq {
            let s16 = grid.s16;
            let ch = chord(ctx, 0)?;
            let r0 = root_of(ctx, ch)?;
            let r = if r0 < 36 { r0 + 12 } else { r0 };
            // int(round((t_solo - tw) / T.S16))
            let n16 = round_half_even((t_solo - tw) / s16);
            let n16 = if n16 > 0.0 { n16 as usize } else { 0 };
            for st in 0..n16 {
                let acc = if st % 4 == 0 { 1.0 } else { 0.7 };
                let x = self.strings_low(r + OST[st % 16], s16 * 0.9, acc);
                put(mix, "strings", &x, tw + st as f64 * s16, 0.11);
            }
            put(mix, "pad", &self.pad(&Self::pad_notes(ctx, ch)?, t_solo - tw), tw, 0.08);
            let x = self.tremolo(&Self::trem(ctx, ch)?, t_solo - tw + 0.05, 0.2);
            put(mix, "tremolo", &x, tw, 0.06);
            for st in (0..n16).step_by(4) {
                let x = self.taiko(0.8, st == 0, 500 + st as u64);
                put(mix, "taiko", &x, tw + st as f64 * s16, 0.36);
            }
        }
        Ok(())
    }

    /// Light bars under the winner's solo (in the sequel the trailer plays on, full), up to the last bar.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(1) {
            if self.seq {
                self.bar(ctx, mix, b, 1.45, false)?;
            } else {
                self.bar(ctx, mix, b, 1.0, true)?;
            }
        }
        Ok(())
    }

    /// The last bar builds into the title slam on sample 0: a taiko roll (8ths, then 16ths), a riser, the dominant
    /// seventh climbing in the low strings, a braam swelling right up to the seam, the tremolo (and the sequel's
    /// big taiko on every beat, a second braam swell and the pad).
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, dur, s16) = (grid.beat, grid.dur, grid.s16);
        let t30 = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = last_chord(ctx)?;
        let mut st = 0.0;
        while t30 + st < dur - 1e-4 {
            let v = 0.45 + 0.55 * st / (dur - t30);
            let seed = 200 + noise_seed(H, st * 10.0)?;
            put(mix, "taiko", &self.taiko(v, false, seed), t30 + st, 0.36);
            st += if st < 3.0 * beat - 1e-6 { beat / 2.0 } else { s16 };
        }
        put(mix, "fx", &riser(dur - t30, RISER_OVER, RISER_SEED)?, t30, 0.22);
        let r = ctx.harmony.root_above(ch, 45).map_err(bad)?;
        let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
        if pcs.len() < 2 {
            return Err(MusicError::new(H, format!("{ch} has no third")));
        }
        let third = (pcs[1] - pcs[0]).rem_euclid(12);
        for (bt, m) in [r, r + third, r + 7, r + 10].into_iter().enumerate() {
            let x = self.strings_low(m, beat * 0.9, 1.0);
            put(mix, "strings", &x, t30 + bt as f64 * beat, 0.16);
        }
        let r0 = root_of(ctx, ch)?;
        // a braam swelling right up to the seam
        let sw = self.braam(r0, dur - t30 + 0.012, 0.9, dur - t30);
        let ramp = dsp::linspace(0.3, 1.0, sw.len());
        let x: Vec<f64> = sw.iter().zip(&ramp).map(|(s, w)| s * w).collect();
        put(mix, "braam", &x, t30, 0.16);
        let x = self.tremolo(&Self::trem(ctx, ch)?, dur - t30 + 0.012, 1.2);
        put(mix, "tremolo", &x, t30, 0.06);
        if self.seq {
            for bt in 0..4u64 {
                put(
                    mix,
                    "taiko",
                    &self.taiko(1.0, true, 400 + bt),
                    t30 + bt as f64 * beat,
                    0.40,
                );
            }
            let sw2 = self.braam(r0 + 12, dur - t30 + 0.012, 0.9, dur - t30);
            let ramp = dsp::linspace(0.2, 1.0, sw2.len());
            let x: Vec<f64> = sw2.iter().zip(&ramp).map(|(s, w)| s * w).collect();
            put(mix, "braam", &x, t30, 0.10);
            let x = self.pad(&Self::pad_notes(ctx, ch)?, dur - t30 + 0.012);
            put(mix, "pad", &x, t30, 0.08);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kits_cache_keys_round_and_the_others_do_not() {
        let bed = BrassTrailerClassic::new(true);
        // ('sub', root, round(dur, 3)): 1.9 and 1.9004 are one key, the first call's sound
        let a = bed.sub(26, 1.9);
        assert_eq!(*bed.sub(26, 1.9004), *a);
        assert_eq!(*a, sub(26, 1.9));
        let p = bed.pad(&[45, 50, 53], 1.967);
        assert_eq!(*bed.pad(&[45, 50, 53], 1.9674), *p);
        // the others hold every argument: another length is another sound
        assert_ne!(
            bed.strings_low(50, 0.1, 1.0).len(),
            bed.strings_low(50, 0.11, 1.0).len()
        );
        // the velocity scales the made-once taiko
        let loud = bed.taiko(1.0, true, 7);
        let soft = bed.taiko(0.5, true, 7);
        assert!(loud.iter().zip(&soft).all(|(x, y)| (x * 0.5).to_bits() == y.to_bits()));
        assert_eq!(loud, taiko(true, 7));
        assert_eq!(bed.cache.lock().unwrap().len(), 5);
    }

    #[test]
    fn the_sequel_reweights_two_buses_and_keeps_the_order() {
        let plain = BrassTrailerClassic::new(false);
        let seq = BrassTrailerClassic::new(true);
        let names = |b: &BrassTrailerClassic| b.buses().iter().map(|(n, _)| *n).collect::<Vec<_>>();
        assert_eq!(names(&plain), names(&seq));
        assert_eq!(seq.buses()[1], ("taiko", (1.3, 0.25)));
        assert_eq!(seq.buses()[2], ("braam", (1.15, 0.32)));
        assert_eq!(plain.reverb(), (2.4, 0.6, 0.22));
        assert!(plain.has(BedHook::TrailerHit) && plain.has(BedHook::Boom) && !plain.has(BedHook::Hit));
    }
}
