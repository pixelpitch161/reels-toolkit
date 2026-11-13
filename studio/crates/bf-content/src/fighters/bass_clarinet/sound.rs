//! BASS CLARINET's part in the score: the `Sound` half of `kit/fighters/bass_clarinet.py` (#40's `audio.py`).
//!
//! The chalumeau growl: the peg going in plays the bar's root with a growl, every 8th of a whirl plays its line, it
//! growls a held fifth as it rolls on, a smash pops a slap tongue on the line's pitch; MULTIPHONICS adds the twelfth
//! and opens with a split, buzzing chord; SLAP TONGUE is the big slap; LIGHTS OUT a low growl swelling in the dark,
//! the eruption a huge slap over a thunder thud (and the bed's own crash when it has one).
//!
//! Its octave follows its role: `bass` plays the line at the bass (keeping its fundamental as it thins, and pumping
//! the root into the title slam); any other role plays it an octave up and rests in the last bar.
//!
//! Its notes come straight from the series' reed synth (`bassclarinet_lowpoly.bass_reed`, [`bass_reed`]) and slap
//! (`shared::slap`), through a cache of its own ([`BassClarinetSound::bcl`], [`BassClarinetSound::slap`]): the kit
//! keys that cache by the rounded numbers and keeps the first note made under a key, so the cache is part of the
//! sound. What the kit part reads off its look (`M.P.LOOKS[k].ANCH`) it gets from the recording ([`state::anch`]).
//! Nothing draws from the shared noise stream.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_min, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, CLANK_DUR, ColourArgs, MusicCtx, MusicError, bend, clank, event_int, event_is, event_is_str,
    event_number, noise_seed, thin_37, thunder,
};
use bf_sound::synth::{self, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played, Sound};
use bf_sound::voice::pp;

use super::state;
use super::voice::bass_reed;
use crate::fighters::shared::slap::{self, DUR as SLAP_DUR};

/// The name the part's errors carry.
const H: &str = "BASS CLARINET";

/// A note of the cache: `('b', round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))` or `('s', round(m, 2),
/// round(vel, 2))`, the numbers by their bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Bcl(u64, u64, u64, u64),
    Slap(u64, u64),
}

/// `round(x, nd)` as a key (+ 0.0: a rounded -0.0 is the key 0.0, as Python's 0.0 == -0.0).
fn bits(x: f64, nd: i32) -> u64 {
    (round_nd(x, nd) + 0.0).to_bits()
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// The BASS CLARINET's part.
#[derive(Debug)]
pub struct BassClarinetSound {
    k: usize,
    /// self.role == 'bass' when the part was made: the line at the bass, else an octave up.
    bass: bool,
    /// `_VC`: every note made so far, by its key.
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl BassClarinetSound {
    /// The part of the fighter on side `k` (0 is the left corner), playing the bass (`bass`) or an octave up.
    pub fn new(k: usize, bass: bool) -> BassClarinetSound {
        BassClarinetSound {
            k,
            bass,
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

    /// `bcl(m, dur, vel=0.85, growl=0.0)`: one note of the chalumeau reed held `dur` seconds: resampled to 48 kHz,
    /// cut (or padded) to `dur + 0.16` s with a 20 ms fade, and levelled so its first 0.12 s have an RMS of 0.16.
    /// The first note made under `(round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))` is every later
    /// one's.
    ///
    /// # Errors
    /// Where Python raises: a `dur` the synth cannot make a note of, a note too short to fade.
    pub fn bcl(&self, m: f64, dur: f64, vel: f64, growl: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = Key::Bcl(bits(m, 3), bits(dur, 3), bits(vel, 2), bits(growl, 2));
        self.cached(key, || {
            let n = (dur + 0.16) * SRF;
            if !(pp::has_len(dur + 0.16) && n.is_finite() && n > -1.0) {
                return Err(MusicError::new(H, format!("a note held {dur} s")));
            }
            let x = pp::to_48k(&bass_reed(pp::midi_hz(m), dur, vel, growl));
            // n = int((dur + 0.16) * A.SR); x = x[:n].copy(), padded with zeros to n
            let n = n as usize;
            let mut x: Vec<f64> = x.into_iter().take(n).collect();
            x.resize(n, 0.0);
            // fd = min(len(x) // 3, int(0.02 * A.SR)); x[-fd:] *= np.linspace(1, 0, fd)
            let fd = (x.len() / 3).min((0.02 * SRF) as usize);
            if fd == 0 && !x.is_empty() {
                return Err(MusicError::new(H, format!("a note held {dur} s is too short to fade")));
            }
            let len = x.len();
            for (v, k) in x[len - fd..].iter_mut().zip(dsp::linspace(1.0, 0.0, fd)) {
                *v *= k;
            }
            // r = np.sqrt((x[:int(0.12 * A.SR)] ** 2).mean()) + 1e-9; x * (0.16 / r)
            let head: Vec<f64> = x[..len.min((0.12 * SRF) as usize)].iter().map(|v| v * v).collect();
            let r = dsp::mean(&head).sqrt() + 1e-9;
            let level = 0.16 / r;
            Ok(x.iter().map(|v| v * level).collect())
        })
    }

    /// `slap(m, vel=1.0)`: the slap-tongue pop at `m`: `_resample(slap(_midi_hz(m), vel)) * 0.9`, kept by
    /// `(round(m, 2), round(vel, 2))`.
    ///
    /// # Errors
    /// None in practice (the cache's lock).
    pub fn slap(&self, m: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.cached(Key::Slap(bits(m, 2), bits(vel, 2)), || {
            Ok(pp::to_48k(&slap::slap(pp::midi_hz(m), vel, SLAP_DUR))
                .iter()
                .map(|v| v * 0.9)
                .collect())
        })
    }

    /// `self.oct`: 0 as the bass, an octave up otherwise.
    fn oct(&self) -> i32 {
        if self.bass { 0 } else { 12 }
    }

    /// `root(t)`: the chord root at `t` at the bottom of its register, an octave up unless it is the bass.
    fn root(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let lo = ctx.cfg[self.k].register.0;
        Ok(ctx.harmony.root_above(ctx.harmony.chord_at(t), lo).map_err(bad)? + self.oct())
    }

    /// `tonic()`: the title card's chord root in its register (an octave over its bottom).
    fn tonic(&self, ctx: &MusicCtx) -> Result<i32, MusicError> {
        let lo = ctx.cfg[self.k].register.0;
        let first = ctx
            .harmony
            .chords
            .first()
            .ok_or_else(|| MusicError::new(H, "the fight has no chords"))?;
        Ok(ctx.harmony.root_above(first, lo).map_err(bad)? + 12)
    }

    /// `line_at(t)`: its line's note at `t` (a rest is an error: Python cannot make a note of `None`).
    fn line_at(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let name = self.name(ctx);
        ctx.line_at(name, t)?.ok_or_else(|| {
            MusicError::new(
                H,
                format!("{name}'s line has a rest at {t} s, where the whirl plays a note"),
            )
        })
    }

    /// `twin_on(t)`: a `twin` event of its own covers `t`.
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

    /// The whirls of `play`: the roll out of its corner, each peg's root, the held fifth, the 8ths of the line
    /// (`bass_clarinet.py` 537 to 573).
    fn whirls(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16, t_ko) = (grid.beat, grid.s16, ctx.show.t_ko);
        let anch = state::anch(&ctx.rec.events, k);
        if anch.is_empty() {
            return Ok(());
        }
        let mut dropped = false;
        // first = min(a[0] for a in ANCH): FIGHT!: it rolls out of its corner, growling
        let first = anch.iter().map(|a| a.t).fold(f64::INFINITY, py_min);
        let x = self.bcl(
            f64::from(self.root(ctx, grid.t_fight)?),
            first - grid.t_fight - 0.05,
            0.75,
            0.2,
        )?;
        let (sig, _, _) = self.col(ctx, &x, grid.t_fight)?;
        mix.place(&bus, &Sig::Mono(sig), grid.t_fight, 0.26, -0.2);
        for a in &anch {
            let (t0, te) = (a.t, a.te);
            if t0 > t_ko + 0.01 {
                break;
            }
            let tq = ctx.q16(t0);
            // the peg goes in: the bar's root, growling
            let root = self.root(ctx, tq)?;
            let (sig, g, _) = self.col(ctx, &self.bcl(f64::from(root), 0.9 * beat, 0.95, 0.35)?, t0)?;
            mix.place(&bus, &Sig::Mono(sig), tq, 0.46 * g, -0.2);
            let clink = clank(84.0, noise_seed(H, t0 * 10.0)?, CLANK_DUR, &CLANK_DECAYS)?;
            mix.place("fx", &Sig::Mono(clink), tq, 0.05, -0.2);
            // it pulls the peg and rolls on: a held growl (the fifth)
            let tr = ctx.q16(te);
            let nxt = anch
                .iter()
                .filter(|b| b.t > t0 + 0.1)
                .map(|b| b.t)
                .chain([t_ko])
                .fold(f64::INFINITY, py_min);
            if nxt - tr > 0.3 {
                let x = self.bcl(
                    f64::from(self.root(ctx, tr)? + 7),
                    py_min(nxt - tr, 2.2 * beat) - 0.05,
                    0.7,
                    0.1,
                )?;
                let (sig, g, _) = self.col(ctx, &x, tr)?;
                mix.place(&bus, &Sig::Mono(sig), tr, 0.24 * g, -0.2);
            }
            // the rumble through the whirl, on the 8ths
            let mut e8 = ctx.q16(t0 + beat / 2.0);
            while e8 < te - 0.02 && e8 < t_ko {
                let slot = round_half_even(e8 / s16);
                let hp = ctx.hp_at(k, e8) / 100.0;
                dropped = thin_37(slot, 3.0, hp, dropped);
                if !dropped {
                    let m = f64::from(self.line_at(ctx, e8)?);
                    let (sig, g, lead) = self.col(ctx, &self.bcl(m, 0.42 * beat, 0.8, 0.25)?, e8)?;
                    mix.place(&bus, &Sig::Mono(sig), e8, 0.42 * g, -0.2);
                    if self.twin_on(ctx, e8)? {
                        // MULTIPHONICS: the twelfth above at once
                        let (sig, _, _) = self.col(ctx, &self.bcl(m + 19.0, 0.42 * beat, 0.6, 0.0)?, e8)?;
                        mix.place(&bus, &Sig::Mono(sig), e8, 0.16 * g, -0.1);
                    }
                    if lead > 0.04 && imod_py(slot as i64, 4) == 0 {
                        // the leader: the octave under (fuller)
                        let (sig, _, _) = self.col(ctx, &self.bcl(m - 12.0, 0.42 * beat, 0.6, 0.0)?, e8)?;
                        mix.place(&bus, &Sig::Mono(sig), e8, 0.10 * g, 0.0);
                    }
                }
                e8 += beat / 2.0;
            }
        }
        Ok(())
    }

    /// The events of `play`: the smashes, the three abilities and the eruption (`bass_clarinet.py` 574 to 605).
    fn events(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let multi = &ctx.cfg[k].unlock2.name;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            match e.kind.as_str() {
                "hit" if event_is(H, e, 2, k)? && event_is_str(H, e, 7, "anchor")? => {
                    // a smash: a slap-tongue pop on the line's pitch
                    let tq = ctx.q16(t);
                    let pop = self.slap(f64::from(self.line_at(ctx, tq)? + 12), 1.0)?;
                    let (sig, _, _) = self.col(ctx, &pop, t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.40, -0.15);
                    let thud = synth::thud(0.3, 150.0, 55.0, noise_seed(H, t * 9.0)?);
                    mix.place("fx", &Sig::Mono(thud), tq, 0.18, 0.0);
                }
                "snip" if event_is(H, e, 2, k)? => {
                    // SLAP TONGUE: the big slap
                    let tq = ctx.q16(t);
                    let landed = event_int(H, e, 3)? != 0;
                    let pop = self.slap(f64::from(self.root(ctx, tq)? + 12), 1.0)?;
                    mix.place(
                        &bus,
                        &Sig::Mono(pop.to_vec()),
                        tq,
                        if landed { 0.55 } else { 0.32 },
                        -0.15,
                    );
                    if landed {
                        let thud = synth::thud(0.35, 180.0, 60.0, noise_seed(H, t * 7.0)?);
                        mix.place("fx", &Sig::Mono(thud), tq, 0.30, 0.0);
                    }
                }
                "snip_open" if event_is(H, e, 2, k)? => {
                    // the tongue sticks to the reed: a tiny click
                    mix.place(&bus, &Sig::Mono(self.slap(84.0, 0.3)?.to_vec()), ctx.q16(t), 0.10, 0.0);
                }
                "ability" if event_is(H, e, 2, k)? && event_is_str(H, e, 3, multi)? => {
                    // a split, buzzing chord: root + twelfth + 11th
                    let tq = ctx.q16(t);
                    let root = f64::from(self.root(ctx, tq)?);
                    for (m, g) in [(root, 0.34), (root + 19.0, 0.20), (root + 17.5, 0.11)] {
                        let x = self.bcl(m, 1.5 * grid.beat, 0.9, 0.2)?;
                        mix.place(&bus, &Sig::Mono(x.to_vec()), tq, g, -0.2);
                    }
                }
                "lights_out" if event_is(H, e, 2, k)? => {
                    // LIGHTS OUT: a low growl swelling in the dark
                    let tq = ctx.q16(t);
                    let x = self.bcl(
                        f64::from(ctx.harmony.key_root + 3),
                        event_number(H, e, 3)? - t,
                        1.0,
                        0.6,
                    )?;
                    let swell: Vec<f64> = x
                        .iter()
                        .zip(dsp::linspace(0.5, 1.2, x.len()))
                        .map(|(v, s)| v * s)
                        .collect();
                    mix.place(&bus, &Sig::Mono(swell), tq, 0.40, -0.2);
                    let gust = synth::whoosh(0.8, 200.0, 4000.0, false, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(gust), tq, 0.18, 0.0);
                }
                "erupt" if event_is(H, e, 2, k)? => {
                    // ... and erupts
                    let tq = ctx.q16(t);
                    let pop = self.slap(f64::from(self.root(ctx, tq)? + 12), 1.0)?;
                    mix.place(&bus, &Sig::Mono(pop.to_vec()), tq, 0.65, -0.1);
                    let roll = thunder(1.6, noise_seed(H, t * 10.0)?, true)?;
                    mix.place("fx", &Sig::Mono(roll), tq, 0.45, 0.0);
                    // (the bed's own crash: #40's sheet metal)
                    if bed.has(BedHook::Erupt) {
                        bed.erupt(ctx, mix, tq)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Sound for BassClarinetSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #40's `BUS_GAIN['bcl']`.
    fn bus_gain(&self) -> f64 {
        1.5
    }

    /// #40's reverb send.
    fn send(&self) -> f64 {
        0.25
    }

    /// #40's band check (the chalumeau growl); as the bass (#55) `(60, 500)`: the line keeps its fundamental.
    fn band(&self) -> (f64, f64) {
        if self.bass { (60.0, 500.0) } else { (150.0, 900.0) }
    }

    /// #40's `colour()` for the bass clarinet: `dict(cut0=40, cut_span=200, drive=4.0)`; as the bass also `low=True,
    /// low_cut=(40, 70)`.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 40.0,
            cut_span: 200.0,
            drive: 4.0,
            low: self.bass,
            low_cut: if self.bass {
                (40.0, 70.0)
            } else {
                ColourArgs::default().low_cut
            },
        }
    }

    /// `voice(m, dur, vel=0.85)`: `bcl(m, dur, vel)`.
    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.bcl(m, dur, vel, 0.0)
    }

    /// A growl, its fifth and a slap (the fight file's title hits, else the tonic, its fifth and its octave).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let beat = ctx.show.grid.beat;
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => {
                let t0 = self.tonic(ctx)?;
                vec![t0, t0 + 7, t0 + 12]
            }
        };
        if hits.len() < 3 {
            return Err(MusicError::new(
                H,
                format!("its title hits {hits:?} need three notes: a growl, its fifth and a slap"),
            ));
        }
        let x = self.bcl(f64::from(hits[0]), 0.9 * beat, 1.0, 0.7)?;
        mix.place(&bus, &Sig::Mono(x.to_vec()), t, 0.50, -0.2);
        let x = self.bcl(f64::from(hits[1]), 0.9 * beat, 0.8, 0.4)?;
        mix.place(&bus, &Sig::Mono(x.to_vec()), t, 0.22, -0.2);
        let x = self.slap(f64::from(hits[2]), 1.0)?;
        mix.place(&bus, &Sig::Mono(x.to_vec()), t + 0.004, 0.50, -0.2);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        self.whirls(ctx, mix)?;
        self.events(ctx, bed, mix)
    }

    /// A falling growl.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let x = self.bcl(f64::from(self.tonic(ctx)?), 1.0, 1.0, 0.5)?;
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
        let t0 = self.tonic(ctx)?;
        for (j, m) in [t0, t0 + 7, t0 + 12].into_iter().enumerate() {
            let x = self.bcl(f64::from(m), 0.9 - 0.06 * j as f64, 1.0, 0.2)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw + j as f64 * 0.03, 0.30, 0.0);
        }
        let x = self.slap(f64::from(t0 + 12), 1.0)?;
        mix.place("solo", &Sig::Mono(x.to_vec()), tw, 0.3, 0.0);
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.bcl(f64::from(m), d * grid.beat * 0.95, 0.95, 0.15)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.62, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t0 = self.tonic(ctx)?;
        let run = [0, 3, 7, 10, 12, 15].map(|d| t0 + d);
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.bcl(f64::from(m), 0.12, 0.7, 0.0)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), tj, 0.24, -0.2);
        }
        Ok(())
    }

    /// As the bass: the root and fifth growled on the beats into the title slam; as a counter-line it rests.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        if !self.bass {
            return Ok(());
        }
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        for b in [grid.n_bars - 2, grid.n_bars - 1] {
            for bt in 0..4 {
                let tb = b as f64 * grid.bar + f64::from(bt) * grid.beat;
                if tb >= grid.dur - grid.beat / 2.0 {
                    continue;
                }
                let m = self.root(ctx, tb)? + if bt == 2 { 7 } else { 0 };
                let x = self.bcl(f64::from(m), 0.42 * grid.beat, 0.9, if bt == 0 { 0.4 } else { 0.25 })?;
                mix.place(
                    &bus,
                    &Sig::Mono(x.to_vec()),
                    tb,
                    if bt == 0 { 0.50 } else { 0.40 },
                    -0.2,
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_cut_levelled_and_kept_by_its_rounded_key() {
        let part = BassClarinetSound::new(0, false);
        let x = part.bcl(43.0, 0.3, 0.85, 0.25).unwrap();
        // int((0.3 + 0.16) * 48000) samples
        assert_eq!(x.len(), 22080);
        // the first 0.12 s have an RMS of 0.16
        let rms = (dsp::mean(&x[..5760].iter().map(|v| v * v).collect::<Vec<_>>())).sqrt();
        assert!((rms - 0.16).abs() < 1e-6, "{rms}");
        // it fades to nothing over its last 20 ms
        assert_eq!(*x.last().unwrap(), 0.0);
        // the same key gives the same note back, even from numbers that only round to it
        let again = part.bcl(43.0004, 0.3004, 0.851, 0.252).unwrap();
        assert!(Arc::ptr_eq(&x, &again));
        // another growl, another note; a slap is kept apart
        assert!(!Arc::ptr_eq(&x, &part.bcl(43.0, 0.3, 0.85, 0.0).unwrap()));
        let pop = part.slap(55.0, 1.0).unwrap();
        assert!(Arc::ptr_eq(&pop, &part.slap(55.001, 1.0).unwrap()));
        assert_eq!(part.notes.lock().unwrap().len(), 3);
        // a note that cannot be made is an error, not a panic
        assert!(part.bcl(43.0, -1.0, 0.85, 0.0).is_err());
        assert!(part.bcl(43.0, f64::NAN, 0.85, 0.0).is_err());
    }

    #[test]
    fn its_mix_settings_are_the_kits() {
        let part = BassClarinetSound::new(1, false);
        assert_eq!((part.k(), part.enemy()), (1, 0));
        assert_eq!((part.bus_gain(), part.send(), part.band()), (1.5, 0.25, (150.0, 900.0)));
        let a = part.colour_args();
        assert_eq!((a.cut0, a.cut_span, a.drive, a.low), (40.0, 200.0, 4.0, false));
        assert!(part.bed_calls().is_empty());
        // as the bass it keeps its fundamental
        let bass = BassClarinetSound::new(0, true);
        let a = bass.colour_args();
        assert_eq!(
            (a.low, a.low_cut, bass.band(), bass.oct()),
            (true, (40.0, 70.0), (60.0, 500.0), 0)
        );
        assert_eq!(part.oct(), 12);
    }
}
