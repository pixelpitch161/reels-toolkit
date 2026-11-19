//! EUPHONIUM's part in the score: the `Sound` half of `kit/fighters/euphonium.py` (#38's `audio.py`).
//!
//! The warm tenor melody: every salvo plays its line's note on that beat (a salvo every beat under MARCHING ORDERS is
//! the whole line), a landed salvo a fat chord tone a sixth under, the mortar's pop and the landings' booms; MARCHING
//! ORDERS a fanfare triplet (with the bed's press roll when it has one); NOT A TUBA a falling run, one note per
//! landing; IDENTITY CRISIS a puzzled rising bend and a ghost of the enemy in the enemy's own voice, then the copied
//! move in the euphonium's voice (a copied SAD TROMBONE is the plunger wah-wah down to the tonic, a copied slam's bell
//! stomps on the bed's `boom`); a shell a lance swats is a brass ping.
//!
//! Its notes come straight from the series' synth (`euphonium_lowpoly.euph`, [`euph`]) and the bass clarinet's slap
//! (`shared::slap`), through a cache of its own ([`EuphoniumSound::eup`], [`EuphoniumSound::slap`]): the kit keys
//! that cache by the rounded numbers and keeps the first note made under a key, so the cache is part of the sound.
//! What the kit part reads off its look (`M.P.LOOKS[k].SALVOS`, `.LANDS`) it gets from the recording ([`state`]);
//! the ghost of the enemy is the enemy's part's `voice` ([`Sound::play_with`]). Nothing draws from the shared noise
//! stream but the enemy's ghost, as the enemy's own voice does.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::event::{Event, Value};
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_min, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, ColourArgs, MusicCtx, MusicError, bend, clank, event_at, event_int, event_is,
    event_number, gliss, noise_seed, thin_37, thunder, vibrato, wah,
};
use bf_sound::synth::{self, THUD_SEED, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played, Sound};
use bf_sound::voice::pp;

use super::state;
use super::voice::euph;
use crate::fighters::shared::slap::{self, DUR as SLAP_DUR};

/// The name the part's errors carry.
const H: &str = "EUPHONIUM";

/// A note of the cache: `('e', round(m, 3), round(dur, 3), round(vel, 2))` or `('s', round(m, 2), round(vel, 2))`,
/// the numbers by their bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Eup(u64, u64, u64),
    Slap(u64, u64),
}

/// `round(x, nd)` as a key (+ 0.0: a rounded -0.0 is the key 0.0, as Python's 0.0 == -0.0).
fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// Element `i` of an event as a text (a name, a kind).
fn event_str<'e>(helper: &'static str, e: &'e Event, i: usize) -> Result<&'e str, MusicError> {
    match event_at(helper, e, i)? {
        Value::Str(s) => Ok(s),
        other => Err(MusicError::new(
            helper,
            format!("the {} event's element {i} is {other:?}, not a text", e.kind),
        )),
    }
}

/// `x * k` on every sample.
fn scaled(x: &[f64], k: f64) -> Vec<f64> {
    x.iter().map(|v| v * k).collect()
}

/// The landings of one 16th: `lands[(slot, kk, kind)] = dict(t, n, hits, kind, salvo)`.
#[derive(Clone, Debug)]
struct Landings {
    t: f64,
    n: i64,
    hits: i64,
}

/// The EUPHONIUM's part.
#[derive(Debug)]
pub struct EuphoniumSound {
    k: usize,
    /// `_VC`: every note made so far, by its key.
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl EuphoniumSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> EuphoniumSound {
        EuphoniumSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    fn cached(&self, key: Key, make: impl FnOnce() -> Result<Vec<f64>, MusicError>) -> Result<Arc<[f64]>, MusicError> {
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(Arc::clone(x));
        }
        let x: Arc<[f64]> = make()?.into();
        notes.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// `eup(m, dur, vel=0.85)`: one note of the euphonium held `dur` seconds (#38's `eup()`, level-matched):
    /// resampled to 48 kHz, cut (or padded) to `dur + 0.16` s with a 30 ms fade, and levelled so its first 0.25 s
    /// have an RMS of 0.16. The first note made under `(round(m, 3), round(dur, 3), round(vel, 2))` is every later
    /// one's.
    ///
    /// # Errors
    /// Where Python raises: a `dur` the synth cannot make a note of, a note too short to fade.
    pub fn eup(&self, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = Key::Eup(bits(m, 3), bits(dur, 3), bits(vel, 2));
        self.cached(key, || {
            let n = (dur + 0.16) * SRF;
            if !(pp::has_len(dur + 0.16) && n.is_finite() && n > -1.0) {
                return Err(MusicError::new(H, format!("a note held {dur} s")));
            }
            let x = pp::to_48k(&euph(pp::midi_hz(m), dur, vel));
            // n = int((dur + 0.16) * A.SR); x = x[:n].copy(), padded with zeros to n
            let n = n as usize;
            let mut x: Vec<f64> = x.into_iter().take(n).collect();
            x.resize(n, 0.0);
            // fd = min(len(x) // 3, int(0.03 * A.SR)); x[-fd:] *= np.linspace(1, 0, fd)
            let fd = (x.len() / 3).min((0.03 * SRF) as usize);
            if fd == 0 && !x.is_empty() {
                return Err(MusicError::new(H, format!("a note held {dur} s is too short to fade")));
            }
            let len = x.len();
            for (v, k) in x[len - fd..].iter_mut().zip(dsp::linspace(1.0, 0.0, fd)) {
                *v *= k;
            }
            // r = np.sqrt((x[:int(0.25 * A.SR)] ** 2).mean()) + 1e-9; x * (0.16 / r)
            let head: Vec<f64> = x[..len.min((0.25 * SRF) as usize)].iter().map(|v| v * v).collect();
            let r = dsp::mean(&head).sqrt() + 1e-9;
            let level = 0.16 / r;
            Ok(x.iter().map(|v| v * level).collect())
        })
    }

    /// `slap(m, vel=1.0)`: #38's bass-clarinet slap (a copied SLAP TONGUE's crack): `_resample(slap(_midi_hz(m),
    /// vel)) * 0.9`, kept by `(round(m, 2), round(vel, 2))`.
    ///
    /// # Errors
    /// None in practice (the cache's lock).
    pub fn slap(&self, m: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.cached(Key::Slap(bits(m, 2), bits(vel, 2)), || {
            Ok(scaled(&pp::to_48k(&slap::slap(pp::midi_hz(m), vel, SLAP_DUR)), 0.9))
        })
    }

    /// `boom(seed=0, dur=0.5)`: a shell landing: a short thud and a crunchy noise burst (#38).
    pub fn boom(seed: u64, dur: f64) -> Vec<f64> {
        let n = (dur * SRF) as usize;
        let t = synth::tt(n);
        // th = A.lp(A.thud(dur, 110, 40, seed=seed)[:n], 140), padded to n
        let thud: Vec<f64> = synth::thud(dur, 110.0, 40.0, seed).into_iter().take(n).collect();
        let mut th = synth::lp(&thud, 140.0, 2);
        th.resize(n, 0.0);
        // crunch = A.bp(rng(seed + 9).normal(0, 1, n), 900, 5000) * np.exp(-t / 0.05)
        let crunch = synth::bp(&synth::noise(seed + 9, n), 900.0, 5000.0, 2);
        th.iter()
            .zip(&crunch)
            .zip(&t)
            .map(|((a, c), &t)| a + 0.5 * (c * bf_core::dmath::exp(-t / 0.05)))
            .collect()
    }

    /// `top()`: the key's tonic at the top of its register (C5 = 72 in #38).
    fn top(&self, ctx: &MusicCtx) -> i32 {
        let hi = ctx.cfg[self.k].register.1;
        hi - (hi - ctx.harmony.key_pc).rem_euclid(12)
    }

    /// `melody_at(t)`: its line's note at `t` (a rest is an error where the part plays it: Python cannot make a note
    /// of `None`).
    fn melody_at(&self, ctx: &MusicCtx, t: f64) -> Result<Option<i32>, MusicError> {
        ctx.line_at(self.name(ctx), t)
    }

    fn note_at(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let name = self.name(ctx);
        self.melody_at(ctx, t)?
            .ok_or_else(|| MusicError::new(H, format!("{name}'s line has a rest at {t} s, where it plays a note")))
    }

    /// `ab_dur(ab)` at the fight's tempo: `ab['dur']` or `ab['dur_beats'] * BEAT`.
    fn ab_dur(ctx: &MusicCtx, ab: &bf_core::roster::AbilityCfg) -> Result<f64, MusicError> {
        match (ab.get("dur"), ab.get("dur_beats")) {
            (Some(d), _) => Ok(d),
            (None, Some(b)) => Ok(b * ctx.show.grid.beat),
            _ => Err(MusicError::new(H, format!("{} has neither dur nor dur_beats", ab.name))),
        }
    }

    /// `windows(kind)`: `[(t0, t1)]` its own and copied abilities of a kind (MARCHING ORDERS: the haste).
    fn windows(&self, ctx: &MusicCtx, kind: &str) -> Result<Vec<(f64, f64)>, MusicError> {
        let (k, e_) = (self.k, self.enemy());
        let mut out = Vec::new();
        for e in &ctx.rec.events {
            if e.t > ctx.show.t_ko + 0.01 {
                break;
            }
            let of = |side: usize| {
                let c = &ctx.cfg[side];
                [&c.unlock1, &c.unlock2, &c.low]
            };
            if e.kind == "ability" && event_is(H, e, 2, k)? && event_str(H, e, 4)? == kind {
                let name = event_str(H, e, 3)?;
                let ab = of(k)
                    .into_iter()
                    .find(|a| a.name == name)
                    .ok_or_else(|| MusicError::new(H, format!("no ability {name} of its own")))?;
                out.push((e.t, e.t + Self::ab_dur(ctx, ab)?));
            } else if e.kind == "copy" && event_is(H, e, 2, k)? && event_str(H, e, 4)? == kind {
                let name = event_str(H, e, 3)?;
                if let Some(ab) = of(e_).into_iter().find(|a| a.name == name) {
                    out.push((e.t, e.t + Self::ab_dur(ctx, ab)?));
                }
            }
        }
        Ok(out)
    }

    /// `twin_on(t)`: a `twin` event of its own covers `t` (a copied MULTIPHONICS).
    fn twin_on(&self, ctx: &MusicCtx, t: f64) -> Result<bool, MusicError> {
        for e in &ctx.rec.events {
            if e.kind == "twin" && event_is(H, e, 2, self.k)? && e.t <= t && t < event_number(H, e, 3)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `self.colour(x, t)`: `(signal, gain, lead)`.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, 1.0, &self.colour_args())
    }

    /// The salvos and the landings of `play` (`euphonium.py` 540 to 580).
    fn salvos(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16, t_ko) = (grid.beat, grid.s16, ctx.show.t_ko);
        let haste_w = self.windows(ctx, "haste")?;
        let mut dropped = false;
        let mut lands: BTreeMap<(i64, String), Landings> = BTreeMap::new();
        for l in state::lands(&ctx.rec.events, k) {
            if l.t > t_ko + 0.01 {
                break;
            }
            // (round(q16(t) / T.S16), kk, kind): kk is always its own side here
            let slot = round_half_even(ctx.q16(l.t) / s16) as i64;
            let q = lands
                .entry((slot, l.kind.clone()))
                .or_insert(Landings { t: l.t, n: 0, hits: 0 });
            q.n += 1;
            q.hits += l.hit;
        }
        for s in state::salvos(&ctx.rec.events, k) {
            let t0 = s.t;
            if t0 > t_ko + 0.01 {
                break;
            }
            let tq = ctx.q16(t0);
            let slot = round_half_even(tq / s16);
            let hp = ctx.hp_at(k, t0) / 100.0;
            let haste = haste_w.iter().any(|&(a, b)| a <= t0 && t0 < b);
            dropped = thin_37(slot, 11.0, hp, dropped);
            let m = self.melody_at(ctx, tq)?;
            if dropped && haste {
                // thinned: a note of the every-beat run drops out
                continue;
            }
            let mut dur = if haste { 0.85 } else { 1.7 } * beat;
            if dropped {
                // (on the 2-beat grid it is clipped, not dropped: no hole while it is still firing)
                dur = 0.5 * beat;
            }
            let m = f64::from(m.ok_or_else(|| {
                MusicError::new(
                    H,
                    format!("{}'s line has a rest at {tq} s, where a salvo plays", self.name(ctx)),
                )
            })?);
            let n = s.n.min(5) as f64;
            let (sig, g, lead) = self.col(ctx, &self.eup(m, dur, 0.75 + 0.04 * n)?, t0)?;
            mix.place(&bus, &Sig::Mono(sig), tq, if dropped { 0.26 } else { 0.40 } * g, 0.2);
            if self.twin_on(ctx, t0)? {
                // a copied MULTIPHONICS: two notes at once
                let (sig, _, _) = self.col(ctx, &self.eup(m + 7.0, dur, 0.6)?, t0)?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.20 * g, 0.25);
            }
            if lead > 0.04 {
                // the leader: a third under (fuller)
                let mi = m as i32;
                let under = ctx
                    .harmony
                    .chord_tones(mi - 5, mi - 3, ctx.harmony.chord_at(tq))
                    .map_err(bad)?;
                if let Some(&u) = under.last() {
                    let (sig, _, _) = self.col(ctx, &self.eup(f64::from(u), dur, 0.55)?, t0)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.15 * g, 0.25);
                }
            }
            // the mortar's pop
            let pop = synth::thud(0.18, 220.0, 90.0, noise_seed(H, t0 * 11.0)?);
            mix.place("fx", &Sig::Mono(pop), tq, 0.06 + 0.01 * s.n.min(6) as f64, 0.0);
        }
        for ((slot, kind), q) in &lands {
            let t = q.t;
            let tq = *slot as f64 * s16;
            let g_n = py_min(1.0, 0.5 + 0.12 * q.n as f64);
            let seed = imod_py(noise_seed(H, t * 13.0)? as i64, 1000) as u64;
            mix.place(
                "fx",
                &Sig::Mono(Self::boom(seed, 0.45)),
                tq,
                if q.hits != 0 { 0.16 } else { 0.09 } * g_n,
                0.1,
            );
            if q.hits != 0 && kind == "mortar" {
                // landed: a fat chord tone a sixth under
                let m = self.note_at(ctx, tq)?;
                let tones = ctx
                    .harmony
                    .chord_tones(m - 10, m - 7, ctx.harmony.chord_at(tq))
                    .map_err(bad)?;
                if let Some(&u) = tones.last() {
                    let (sig, g, _) = self.col(ctx, &self.eup(f64::from(u), 0.35, 0.95)?, t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.30 * g, 0.25);
                }
            }
        }
        Ok(())
    }

    /// The events of `play` (`euphonium.py` 581 to 641): NOT A TUBA, IDENTITY CRISIS and the copied moves, MARCHING
    /// ORDERS, the swatted shells.
    fn events(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, es: &dyn Sound) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let beat = grid.beat;
        let top = self.top(ctx);
        let topf = f64::from(top);
        let unlock1 = &ctx.cfg[k].unlock1;
        let lands = state::lands(&ctx.rec.events, k);
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            let mine = |e| event_is(H, e, 2, k);
            match e.kind.as_str() {
                "barrage" if mine(e)? => {
                    // NOT A TUBA: a falling run, one note per landing
                    let tq = ctx.q16(t);
                    mix.place(&bus, &Sig::Mono(self.eup(topf, beat, 1.0)?.to_vec()), tq, 0.46, 0.2);
                    mix.place(
                        &bus,
                        &Sig::Mono(self.eup(topf - 12.0, beat, 0.9)?.to_vec()),
                        tq,
                        0.30,
                        0.2,
                    );
                    let salvo = event_int(H, e, 4)?;
                    // down the natural minor (#38: C5 .. C4)
                    let seq = [0, 2, 4, 5, 7, 9, 10, 12].map(|d| top - d);
                    let mut seen: Vec<f64> = Vec::new();
                    for l in lands.iter().filter(|l| l.salvo == salvo) {
                        let tl = ctx.q16(l.t);
                        if seen.contains(&tl) {
                            continue;
                        }
                        seen.push(tl);
                        let m = seq[(seen.len() - 1).min(seq.len() - 1)];
                        let (sig, _, _) = self.col(ctx, &self.eup(f64::from(m), 0.4 * beat, 0.95)?, l.t)?;
                        mix.place(&bus, &Sig::Mono(sig), tl, 0.34, 0.2);
                    }
                    mix.place("fx", &Sig::Mono(thunder(1.4, 44, false)?), tq, 0.30, 0.0);
                }
                "copy" if mine(e)? => {
                    // IDENTITY CRISIS: "wait... who am I?"
                    let tq = ctx.q16(t);
                    let kind = event_str(H, e, 4)?;
                    if kind == "shockwave" {
                        // a copied SAD TROMBONE: the wah-wah in its own voice
                        self.wah_wah(ctx, mix, tq, t)?;
                        continue;
                    }
                    let x = self.eup(topf - 10.0, 0.8 * beat, 0.9)?;
                    mix.place(&bus, &Sig::Mono(bend(&x, 5.0, 0.08, 0.8 * beat)?), tq, 0.40, 0.2);
                    // a ghost of the enemy (#38: a bass-clarinet growl)
                    let lo = ctx.cfg[self.enemy()].register.0;
                    let ghost = es.voice(ctx, f64::from(lo + 3), 0.3, 0.6)?;
                    mix.place(&es.bus(ctx), &Sig::Mono(ghost.to_vec()), tq + beat * 0.5, 0.14, 0.25);
                    if kind == "haste" {
                        // a copied rip (POWER SLIDE): up an octave into it
                        let x = gliss(&self.eup(topf - 5.0, 0.45, 1.0)?, 12.0, 0.3)?;
                        let (sig, _, _) = self.col(ctx, &x, t)?;
                        mix.place(&bus, &Sig::Mono(sig), tq + beat, 0.30, 0.2);
                    } else if kind == "slam" {
                        // a copied PEDAL TONE: its low pedal under the bell
                        let tele = ctx.cfg[self.enemy()].low.get("tele").unwrap_or(2.0);
                        let tb = beat * tele;
                        mix.place(
                            &bus,
                            &Sig::Mono(self.eup(topf - 24.0, tb, 1.0)?.to_vec()),
                            tq,
                            0.40,
                            0.2,
                        );
                        let gust = synth::whoosh(tb, 150.0, 1800.0, true, WHOOSH_SEED);
                        mix.place("fx", &Sig::Mono(gust), tq, 0.10, 0.0);
                    }
                }
                "slam" if mine(e)? => {
                    // ... and its bell stomps
                    bed.boom(ctx, mix, t, 0.8)?;
                    mix.place("fx", &Sig::Mono(synth::thud(0.8, 90.0, 30.0, THUD_SEED)), t, 0.5, 0.0);
                    mix.place(
                        "fx",
                        &Sig::Mono(clank(64.0, 9, CLANK_DUR, &CLANK_DECAYS)?),
                        t,
                        0.10,
                        0.0,
                    );
                    mix.place(
                        &bus,
                        &Sig::Mono(self.eup(topf - 12.0, 0.5, 1.0)?.to_vec()),
                        t,
                        0.36,
                        0.2,
                    );
                }
                "snip" if mine(e)? => {
                    // a copied SLAP TONGUE: a brass slap
                    let tq = ctx.q16(t);
                    let m = f64::from(self.note_at(ctx, tq)? - 12);
                    mix.place(&bus, &Sig::Mono(self.eup(m, 0.12, 1.0)?.to_vec()), tq, 0.46, 0.2);
                    mix.place("fx", &Sig::Mono(scaled(&self.slap(72.0, 0.8)?, 0.6)), tq, 0.25, 0.0);
                    if event_int(H, e, 3)? != 0 {
                        let thud = synth::thud(0.35, 180.0, 60.0, noise_seed(H, t * 7.0)?);
                        mix.place("fx", &Sig::Mono(thud), tq, 0.26, 0.0);
                    }
                }
                "ability" if mine(e)? && event_str(H, e, 3)? == unlock1.name && unlock1.kind.name() == "haste" => {
                    // MARCHING ORDERS: a fanfare triplet + a roll
                    let tq = ctx.q16(t);
                    for (j, m) in [top - 5, top - 5, top].into_iter().enumerate() {
                        let x = self.eup(f64::from(m), if j < 2 { 0.13 } else { 0.5 }, 1.0)?;
                        mix.place(&bus, &Sig::Mono(x.to_vec()), tq + j as f64 * beat / 3.0, 0.40, 0.2);
                    }
                    if bed.has(BedHook::PressRoll) {
                        // (a bed with a snare: #38's drumline roll)
                        bed.press_roll(ctx, mix, tq, tq + beat, 0.10, 0.26)?;
                    }
                }
                "lights_out" if mine(e)? => {
                    // a copied LIGHTS OUT
                    let x = self.eup(topf - 26.0, event_number(H, e, 3)? - t, 0.9)?;
                    mix.place(&bus, &Sig::Mono(x.to_vec()), ctx.q16(t), 0.34, 0.2);
                }
                "erupt" if mine(e)? => {
                    let tq = ctx.q16(t);
                    mix.place(
                        &bus,
                        &Sig::Mono(self.eup(topf - 14.0, 0.4, 1.0)?.to_vec()),
                        tq,
                        0.5,
                        0.2,
                    );
                    mix.place(
                        "fx",
                        &Sig::Mono(thunder(1.4, noise_seed(H, t * 10.0)?, true)?),
                        tq,
                        0.4,
                        0.0,
                    );
                }
                "swat_shell" if event_is(H, e, 3, k)? => {
                    // #48: a shell swatted out of the air: a brass ping
                    let j = event_int(H, e, 5)?;
                    let ping = clank(
                        88.0 + (imod_py(j, 3) * 2) as f64,
                        noise_seed(H, t * 17.0)?,
                        CLANK_DUR,
                        &CLANK_DECAYS,
                    )?;
                    let pan = synth::clip(event_number(H, e, 6)? / 3.5, -0.6, 0.6);
                    mix.place("fx", &Sig::Mono(ping), t, 0.07, pan);
                    let thud = synth::thud(0.12, 260.0, 120.0, noise_seed(H, t * 5.0)?);
                    mix.place("fx", &Sig::Mono(thud), t, 0.04, 0.0);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// `wah_wah(mix, tq, t)`: IDENTITY CRISIS copies SAD TROMBONE: the euphonium plays the plunger wah-wah down to the
    /// tonic, in amber (#48: for a moment the euphonium IS a trombone).
    fn wah_wah(&self, ctx: &MusicCtx, mix: &mut Mix, tq: f64, t: f64) -> Played {
        let bus = self.bus(ctx);
        let beat = ctx.show.grid.beat;
        let mut m0 = self.top(ctx) - 12;
        while m0 - 1 < ctx.cfg[self.k].register.0 {
            m0 += 12;
        }
        for (j, m) in [m0 + 3, m0 + 2, m0 + 1, m0].into_iter().enumerate() {
            let d = if j < 3 { 0.5 * beat } else { 1.6 * beat };
            let note = self.eup(f64::from(m), d, 0.9)?;
            let mut x = if j > 0 { gliss(&note, 1.0, 0.04)? } else { note.to_vec() };
            if j == 3 {
                x = vibrato(&x, 0.5, 5.0, 0.2)?;
            }
            let x = wah(&x, &[(0.0, py_min(0.18, d * 0.6))])?;
            let (sig, _, _) = self.col(ctx, &x, t)?;
            mix.place(&bus, &Sig::Mono(sig), tq + j as f64 * 0.5 * beat, 0.42, 0.2);
        }
        let gust = synth::whoosh(0.6, 200.0, 1500.0, false, WHOOSH_SEED);
        mix.place("fx", &Sig::Mono(gust), tq, 0.08, 0.0);
        Ok(())
    }
}

impl Sound for EuphoniumSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #38's bus gain.
    fn bus_gain(&self) -> f64 {
        1.3
    }

    /// #38's reverb send.
    fn send(&self) -> f64 {
        0.35
    }

    /// The tenor line G3 to C5 and its brass harmonics.
    fn band(&self) -> (f64, f64) {
        (150.0, 1200.0)
    }

    /// #38's `colour()`: `dict(cut0=40, cut_span=200, drive=4.0)`.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 40.0,
            cut_span: 200.0,
            drive: 4.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `eup(m, dur, vel)`.
    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.eup(m, dur, vel)
    }

    /// The slam of a copied PEDAL TONE calls the bed's `boom` (not under `hasattr`); its press roll is asked for.
    fn bed_calls(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// A blast and a fat fifth (the fight file's title hits, else the tonic an octave under the top, its fourth under
    /// and two octaves under).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let beat = ctx.show.grid.beat;
        let top = self.top(ctx);
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => vec![top - 12, top - 5, top - 24],
        };
        for (m, g) in hits.iter().zip([0.50, 0.36, 0.30, 0.26, 0.22]) {
            let x = self.eup(f64::from(*m), 1.5 * beat, 1.0)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), t, g, 0.2);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _bed: &dyn Bed, _mix: &mut Mix) -> Played {
        let _ = ctx;
        Err(MusicError::new(
            H,
            "its part plays a ghost of the enemy in the enemy's voice: the score calls play_with",
        ))
    }

    fn play_with(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, enemy: &dyn Sound) -> Played {
        self.salvos(ctx, mix)?;
        self.events(ctx, bed, mix, enemy)
    }

    /// A falling blast, valves (#38).
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let x = self.eup(f64::from(self.top(ctx) - 14), 1.0, 1.0)?;
        // M.bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x))
        let fall: Vec<f64> = bend(&x, -9.0, 0.05, 1.0)?
            .iter()
            .zip(dsp::linspace(1.0, 0.4, x.len()))
            .map(|(v, k)| v * k)
            .collect();
        mix.place(&self.bus(ctx), &Sig::Mono(fall), ctx.show.t_hs, 0.42, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let top = f64::from(self.top(ctx));
        mix.place(
            "solo",
            &Sig::Mono(self.eup(top - 7.0, 0.9, 1.0)?.to_vec()),
            tw,
            0.45,
            0.0,
        );
        mix.place("solo", &Sig::Mono(self.eup(top, 0.9, 0.9)?.to_vec()), tw, 0.30, 0.0);
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.eup(f64::from(m), d * grid.beat * 0.95, 0.95)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.52, 0.0);
        }
        Ok(())
    }

    /// #38's run: G3 Bb3 C4 Eb4 G4 C5 (in its key).
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let top = self.top(ctx);
        let run = [-17, -14, -12, -9, -5, 0].map(|d| top + d);
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.eup(f64::from(m), 0.14, 0.7)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), tj, 0.22, 0.0);
        }
        Ok(())
    }

    /// Revving in its corner: the melody builds into the frame-0 slam (#38: from the second-last bar's beat 2).
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t30 = (grid.n_bars as f64 - 2.0) * grid.bar;
        for j in 1..8 {
            let tb = t30 + f64::from(j) * grid.beat;
            let m = self.note_at(ctx, tb)?;
            let x = self.eup(f64::from(m), 0.9 * grid.beat, 0.85)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), tb, 0.36, 0.2);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_cut_levelled_and_kept_by_its_rounded_key() {
        let part = EuphoniumSound::new(0);
        let x = part.eup(62.0, 0.3, 0.85).unwrap();
        // int((0.3 + 0.16) * 48000) samples
        assert_eq!(x.len(), 22080);
        // the first 0.25 s have an RMS of 0.16
        let rms = (dsp::mean(&x[..12000].iter().map(|v| v * v).collect::<Vec<_>>())).sqrt();
        assert!((rms - 0.16).abs() < 1e-6, "{rms}");
        assert_eq!(*x.last().unwrap(), 0.0);
        let again = part.eup(62.0004, 0.3004, 0.851).unwrap();
        assert!(Arc::ptr_eq(&x, &again));
        assert!(!Arc::ptr_eq(&x, &part.eup(62.0, 0.3, 0.6).unwrap()));
        let pop = part.slap(72.0, 0.8).unwrap();
        assert!(Arc::ptr_eq(&pop, &part.slap(72.001, 0.8).unwrap()));
        assert_eq!(part.notes.lock().unwrap().len(), 3);
        assert!(part.eup(62.0, -1.0, 0.85).is_err());
        assert!(part.eup(62.0, f64::NAN, 0.85).is_err());
    }

    #[test]
    fn its_mix_settings_are_the_kits() {
        let part = EuphoniumSound::new(1);
        assert_eq!((part.k(), part.enemy()), (1, 0));
        assert_eq!(
            (part.bus_gain(), part.send(), part.band()),
            (1.3, 0.35, (150.0, 1200.0))
        );
        let a = part.colour_args();
        assert_eq!((a.cut0, a.cut_span, a.drive, a.low), (40.0, 200.0, 4.0, false));
        assert_eq!(part.bed_calls(), &[BedHook::Boom]);
        // a landing's boom: 0.45 s of samples
        assert_eq!(EuphoniumSound::boom(3, 0.45).len(), (0.45 * SRF) as usize);
    }
}
