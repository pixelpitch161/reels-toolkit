//! BASS DRUM's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/bass_drum.py` (#56).
//!
//! It IS the band's bass drum: a kick on every beat on the chord's root in its own voice (its boom, its mallet and
//! its roll), off-beat pickups while it leads, thinning as its HP drops; every roll is a 32nd-note roll swelling into
//! the BOOM of its impact; HEARTBEAT two booms (the dub a fifth up); DRUM ROLL a thundering roll for its whole bar,
//! an octave doubled under it; 1812 OVERTURE fires its five cannons as five tuned booms playing the opening of "God
//! Save the Tsar!" (A. Lvov, 1833, public domain) in the episode's relative major, each with a tubular bell over it,
//! resolving to the tonic on the beat after the fifth.
//!
//! Every stroke is its voice's (`voice::bd_hit`, `voice::bd_roll`), resampled and brought to the series' level
//! (`M.inst_gain`), and kept by `(note, velocity, boom, length)` / `(note, length, velocity)` as the Python part keeps
//! them: two calls that differ only past the kept decimals share the first one's samples.
//!
//! What the Python part reads off its look (`M.P.LOOKS[k].SEGS`, `.DROLLS`, `.CANNONS`) it gets here from the
//! recording's events (`state.rs`). The pit's timpani and marimba double the hymn only when the bed has both
//! (`hasattr(b, 'timp') and hasattr(b, 'marimba')`, `_pit`: [`Bed::has_pit`]): of the kit's beds only
//! `dark_drumline` has both (`baroque_storm` has `timp` but no `marimba`, so there the kit's hymn is the bell alone).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, py_min, round_nd};
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Thinner, noise_seed, thunder};
use bf_sound::synth::{self, clip};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;

use super::state::{self, Seg};
use super::voice::{bd_hit, bd_roll};

/// The name the part's errors carry.
const H: &str = "BASS DRUM";

/// `HYMN`: "God Save the Tsar!", its opening, scale degrees 5 6 6 5 3 as semitones over the tonic.
pub const HYMN: [i32; 5] = [7, 9, 9, 7, 4];

/// The bell's partials: `(ratio, amplitude, decay)`.
const BELL: [(f64, f64, f64); 8] = [
    (0.5, 0.5, 1.6),
    (1.0, 1.0, 1.2),
    (1.19, 0.45, 0.9),
    (1.5, 0.4, 0.8),
    (2.0, 0.55, 0.7),
    (2.74, 0.3, 0.5),
    (3.0, 0.25, 0.4),
    (4.07, 0.18, 0.3),
];

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A float as the cache keeps it: by its bits (0.0 and -0.0 as one).
fn bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

/// The keys `self._c` keeps strokes by: `('h', int(m), round(vel, 2), bool(boom), round(dur, 3))`,
/// `('r', int(m), round(dur, 3), round(vel, 2))`, `('bell', int(m))`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Hit(i32, u64, bool, u64),
    Roll(i32, u64, u64),
    Bell(i32),
}

/// `fit_reg(m, lo, hi)`: `m` moved by octaves into `lo..=hi` (down first, then up, as the Python loops do).
pub fn fit_reg(mut m: i32, lo: i32, hi: i32) -> i32 {
    while m > hi {
        m -= 12;
    }
    while m < lo {
        m += 12;
    }
    m
}

/// `fit_reg(m)` of a note that need not be whole (the enemy's sour twin, OBOE's, plays a quarter tone off):
/// moved by whole octaves into `lo..=hi`, as the Python loops do on a float.
pub fn fit_reg_f(mut m: f64, lo: i32, hi: i32) -> f64 {
    let (lo, hi) = (f64::from(lo), f64::from(hi));
    while m > hi {
        m -= 12.0;
    }
    while m < lo {
        m += 12.0;
    }
    m
}

/// `int(m)` of a note.
fn whole(m: f64) -> Result<i32, MusicError> {
    if m.is_finite() && m.abs() < 1.0e9 {
        Ok(m.trunc() as i32)
    } else {
        Err(MusicError::new(H, format!("a note's midi is {m}, not a number")))
    }
}

/// BASS DRUM's part.
#[derive(Debug)]
pub struct BassDrumSound {
    k: usize,
    /// `self._c`: the strokes made so far.
    cache: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

impl BassDrumSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> BassDrumSound {
        BassDrumSound {
            k,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    /// `self.pan`: a little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.08 } else { 0.08 }
    }

    /// `self.lo, self.hi = M.CFG[k]['register']`.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// A stroke from the cache, made by `make` the first time its key is asked for.
    fn kept(&self, key: Key, make: impl FnOnce() -> Vec<f64>) -> Result<Arc<[f64]>, MusicError> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| MusicError::new(H, "a stroke panicked earlier"))?;
        if let Some(x) = cache.get(&key) {
            return Ok(Arc::clone(x));
        }
        let out: Arc<[f64]> = make().into();
        cache.insert(key, Arc::clone(&out));
        Ok(out)
    }

    /// `hit(m, vel=0.75, boom=False, dur=0.2)`: one stroke of its voice, at 48 kHz and the series' level:
    /// `_resample(bd_hit(midi_hz(int(m)), vel, boom=boom, dur=None if boom else dur)) * g0`.
    ///
    /// # Errors
    /// An argument is not a number; the voice's level cannot be measured.
    pub fn hit(&self, ctx: &MusicCtx, m: f64, vel: f64, boom: bool, dur: f64) -> Result<Arc<[f64]>, MusicError> {
        let m = whole(m)?;
        for (what, v) in [("vel", vel), ("dur", dur)] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("a stroke's {what} is {v}, not a number")));
            }
        }
        // self.g0 = M.inst_gain(M.CFG[k]['pp']), measured when the part is made: the voice draws no shared noise,
        // so measuring it here (once: the context keeps it) gives the same soundtrack
        let g0 = ctx.inst_gain(&ctx.cfg[self.k].pp)?;
        let key = Key::Hit(m, bits(round_nd(vel, 2)), boom, bits(round_nd(dur, 3)));
        self.kept(key, || {
            let x = bd_hit(
                pp::midi_hz(f64::from(m)),
                vel,
                boom,
                0,
                if boom { None } else { Some(dur) },
            );
            pp::to_48k(&x).iter().map(|v| v * g0).collect()
        })
    }

    /// `roll(m, dur, vel=0.8)`: a roll of its voice at the episode's tempo, at 48 kHz and the series' level:
    /// `_resample(bd_roll(midi_hz(int(m)), dur, vel, bpm=T.BPM)) * g0`.
    ///
    /// # Errors
    /// An argument is not a number, or `dur` so far below zero that the roll has no samples; the voice's level
    /// cannot be measured.
    pub fn roll(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let m = whole(m)?;
        if !(vel.is_finite() && dur.is_finite() && pp::has_len(dur + 1.4)) {
            return Err(MusicError::new(H, format!("no roll of {dur} s at velocity {vel}")));
        }
        let g0 = ctx.inst_gain(&ctx.cfg[self.k].pp)?;
        let bpm = ctx.show.grid.bpm;
        let key = Key::Roll(m, bits(round_nd(dur, 3)), bits(round_nd(vel, 2)));
        self.kept(key, || {
            let x = bd_roll(pp::midi_hz(f64::from(m)), dur, vel, bpm, 0);
            pp::to_48k(&x).iter().map(|v| v * g0).collect()
        })
    }

    /// `bell(m)`: a tubular bell on `m`, 2.4 s.
    fn bell(&self, m: i32) -> Result<Arc<[f64]>, MusicError> {
        self.kept(Key::Bell(m), || {
            let n = (2.4 * SRF) as usize;
            let t = synth::tt(n);
            let f = synth::mtof(f64::from(m));
            let mut x = vec![0.0; n];
            for (ratio, amp, dk) in BELL {
                let w = 2.0 * PI * f * ratio;
                for (x, &t) in x.iter_mut().zip(&t) {
                    *x += amp * dmath::sin(w * t) * dmath::exp(-t / dk);
                }
            }
            x.iter()
                .zip(&t)
                .map(|(x, &t)| x * clip(t / 0.002, 0.0, 1.0) * 0.35)
                .collect()
        })
    }

    /// A stroke as a mono sound to place.
    fn sig(x: &Arc<[f64]>) -> Sig {
        Sig::Mono(x.to_vec())
    }

    /// `root_of(t)`: the root of the chord at `t`, the lowest at or above a minor third over its bottom note.
    fn root_of(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let lo = self.register(ctx).0;
        ctx.harmony.root_above(ctx.harmony.chord_at(t), lo + 3).map_err(bad)
    }

    /// `col(x, t, **kw)`: `self.colour(x, t, low=True, **kw)`: the kick keeps its fundamental.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let args = ColourArgs {
            low: true,
            ..self.colour_args()
        };
        let (sig, g, _lead) = self.colour(ctx, x, t, drive_k, &args)?;
        Ok((sig, g))
    }

    /// `tonic_major()`: the hymn's tonic, the key's relative major when the key is minor.
    fn tonic_major(&self, ctx: &MusicCtx) -> i32 {
        let pc = ctx.harmony.key_pc;
        let third = imod_py(i64::from(pc) + 3, 12);
        let minor = ctx
            .harmony
            .scale
            .iter()
            .any(|&s| imod_py(i64::from(pc) + i64::from(s), 12) == third);
        pc + if minor { 3 } else { 0 }
    }

    /// `hymn_tonic()`: the hymn's tonic in the drum's register, low enough that its sixth still fits.
    fn hymn_tonic(&self, ctx: &MusicCtx) -> i32 {
        let (lo, hi) = self.register(ctx);
        let t0 = fit_reg(self.tonic_major(ctx) + 24, lo + 2, lo + 13);
        let top = HYMN.iter().copied().max().unwrap_or(0);
        if t0 + top > hi { t0 - 12 } else { t0 }
    }

    /// `hymn_note(i)`: the hymn's note `i`, the tonic past its end.
    fn hymn_note(&self, ctx: &MusicCtx, i: usize) -> i32 {
        self.hymn_tonic(ctx) + HYMN.get(i).copied().unwrap_or(0)
    }

    /// `_pit(mix, t, m)`: the hymn note doubled by the pit's timpani (moved into C2 to D3) and a 32nd-note marimba
    /// roll on it and its octave when the bed has a pit ([`Bed::has_pit`]), and a tubular bell over it. `s16` is
    /// `T.S16`.
    fn pit(&self, bed: &dyn Bed, mix: &mut Mix, t: f64, m: i32, s16: f64) -> Played {
        let mut hm = m;
        while hm < 72 {
            hm += 12;
        }
        if bed.has_pit() {
            let mut tm = m;
            while tm < 36 {
                tm += 12;
            }
            while tm > 50 {
                tm -= 12;
            }
            mix.place("pit", &Sig::Mono(bed.timp(tm)?), t, 0.42, 0.0);
            for dm in [0, 12] {
                for r in 0..6usize {
                    // 0.05 + 0.01 * (r_ == 0)
                    let g = 0.05 + 0.01 * if r == 0 { 1.0 } else { 0.0 };
                    mix.place(
                        "pit",
                        &Sig::Mono(bed.marimba(hm + dm)?),
                        t + r as f64 * s16 / 2.0,
                        g,
                        0.0,
                    );
                }
            }
        }
        mix.place("fx", &Self::sig(&self.bell(hm + 12)?), t, 0.10, 0.0);
        Ok(())
    }

    /// The kick on every beat, and the off-beat pickups while it leads (bass_drum.py 505 to 518).
    fn kick(&self, ctx: &MusicCtx, mix: &mut Mix, busy: &[(f64, f64)]) -> Played {
        let (k, bus, pan) = (self.k, self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let rolling = |t: f64| busy.iter().any(|&(a, b)| a - 1e-6 <= t && t < b);
        let mut th = Thinner::new(560.0 + k as f64);
        let nb0 = (grid.t_fight / beat).ceil() as i64;
        let nb1 = (ctx.show.t_ko / beat) as i64;
        for nb in nb0..=nb1 {
            let tq = nb as f64 * beat;
            if rolling(tq) {
                continue;
            }
            let hp = ctx.hp_at(k, tq) / 100.0;
            if !th.keep((nb * 4) as f64, hp) {
                continue;
            }
            let x = self.hit(ctx, f64::from(self.root_of(ctx, tq)?), 0.8, false, beat * 0.6)?;
            let (sig, g) = self.col(ctx, &x, tq, 1.0)?;
            mix.place(&bus, &Sig::Mono(sig), tq, 0.50 * g, pan);
            let t3 = tq + 3.0 * s16;
            if hp > ctx.hp_at(1 - k, tq) / 100.0 + 0.08 && imod_py(nb, 2) == 1 && !rolling(t3) {
                let x = self.hit(ctx, f64::from(self.root_of(ctx, tq)?), 0.62, false, s16)?;
                let (sig, g) = self.col(ctx, &x, t3, 1.0)?;
                mix.place(&bus, &Sig::Mono(sig), t3, 0.34 * g, pan);
            }
        }
        Ok(())
    }

    /// Every roll: a 32nd roll swelling into the boom; DRUM ROLL: a thundering roll for the whole bar
    /// (bass_drum.py 519 to 535).
    fn rolls(&self, ctx: &MusicCtx, mix: &mut Mix, segs: &[Seg], drolls: &[(f64, f64)]) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let t_ko = ctx.show.t_ko;
        let s16 = ctx.show.grid.s16;
        let lo = self.register(ctx).0;
        for s in segs {
            if s.t0 > t_ko {
                break;
            }
            let t0 = ctx.q16(s.t0);
            let t1 = py_max(ctx.q16(s.t_end), ctx.q16(s.t0) + s16);
            if s.kind == 0 {
                let x = self.roll(ctx, f64::from(self.root_of(ctx, t0)?), t1 - t0, 0.8)?;
                let (sig, g) = self.col(ctx, &x, t0, 1.0)?;
                mix.place(&bus, &Sig::Mono(sig), t0, 0.40 * g, pan);
            }
        }
        for &(t0, t1) in drolls {
            if t0 > t_ko {
                break;
            }
            let (a, b) = (ctx.q16(t0), py_min(ctx.q16(t1), t_ko));
            if b > a {
                let r = self.root_of(ctx, a)?;
                mix.place(
                    &bus,
                    &Self::sig(&self.roll(ctx, f64::from(r), b - a, 0.95)?),
                    a,
                    0.50,
                    pan,
                );
                let under = if r - 12 >= lo - 12 { r - 12 } else { r };
                let x = self.roll(ctx, f64::from(under), b - a, 0.8)?;
                mix.place(&bus, &Self::sig(&x), a + 0.012, 0.28, -pan);
                let rumble = thunder(b - a + 0.6, noise_seed(H, t0 * 7.0)?, false)?;
                mix.place("fx", &Sig::Mono(rumble), a, 0.16, 0.0);
            }
        }
        Ok(())
    }

    /// The BOOMs, HEARTBEAT and the cannons (bass_drum.py 536 to 564).
    fn events(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix, cannons: &[(f64, i64, f64, f64, f64)]) -> Played {
        let (k, bus, pan) = (self.k, self.bus(ctx), self.pan());
        let (beat, s16) = (ctx.show.grid.beat, ctx.show.grid.s16);
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            let tq = ctx.q16(t);
            match e.kind.as_str() {
                "roll_boom" => {
                    // the BOOM
                    let big = e.i(5) == 0;
                    let vel = if big { 1.0 } else { 0.9 };
                    let x = self.hit(ctx, f64::from(self.root_of(ctx, tq)?), vel, true, 0.2)?;
                    let (sig, g) = self.col(ctx, &x, t, 1.2)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, (if big { 0.70 } else { 0.48 }) * g, pan);
                    let thud = synth::thud(0.6, 110.0, 30.0, noise_seed(H, t * 11.0)?);
                    mix.place("fx", &Sig::Mono(thud), tq, if big { 0.26 } else { 0.14 }, 0.0);
                }
                "heartbeat" => {
                    // lub-dub
                    let r = self.root_of(ctx, tq)?;
                    let lub = e.i(3) == 0;
                    let x = self.hit(ctx, f64::from(if lub { r } else { r + 7 }), 0.98, true, 0.2)?;
                    let (sig, g) = self.col(ctx, &x, t, 1.0)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, (if lub { 0.62 } else { 0.52 }) * g, pan);
                    let thud = synth::thud(0.4, 90.0, 28.0, noise_seed(H, t * 13.0)?);
                    mix.place("fx", &Sig::Mono(thud), tq, 0.18, 0.0);
                }
                "slam" => {
                    // 1812 OVERTURE: a cannon = a tuned boom of the hymn
                    let i = cannons.iter().filter(|c| c.4 <= t + 1e-6).count() as i64 - 1;
                    let m = self.hymn_note(ctx, i.max(0) as usize);
                    let x = self.hit(ctx, f64::from(m), 1.0, true, 0.2)?;
                    mix.place(&bus, &Self::sig(&x), tq, 0.74, pan);
                    let roar = thunder(1.3, (60 + i) as u64, true)?;
                    mix.place("fx", &Sig::Mono(roar), tq, 0.30, 0.0);
                    let thud = synth::thud(0.8, 120.0, 26.0, (70 + i) as u64);
                    mix.place("fx", &Sig::Mono(thud), tq, 0.34, 0.0);
                    self.pit(bed, mix, tq, m, s16)?;
                    if i == HYMN.len() as i64 - 1 {
                        // the resolution: the tonic on the next beat
                        let tn = tq + beat;
                        let tonic = self.hymn_note(ctx, HYMN.len());
                        let x = self.hit(ctx, f64::from(tonic), 1.0, true, 0.2)?;
                        mix.place(&bus, &Self::sig(&x), tn, 0.6, pan);
                        self.pit(bed, mix, tn, tonic, s16)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Sound for BassDrumSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #56's bass drum bus.
    fn bus_gain(&self) -> f64 {
        1.5
    }

    fn send(&self) -> f64 {
        0.16
    }

    /// The kick's and booms' fundamentals (the bed's bass drums off).
    fn band(&self) -> (f64, f64) {
        (35.0, 160.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 60.0,
            cut_span: 240.0,
            drive: 2.4,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.hit(self.fit_reg(m), vel, boom=vel >= 0.9, dur=dur)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let (lo, hi) = self.register(ctx);
        // self.hit(self.fit_reg(m), ...): the note is moved by octaves as it is (a float), and `hit` cuts it
        self.hit(ctx, fit_reg_f(m, lo, hi), vel, vel >= 0.9, dur)
    }

    /// A huge boom on the tonic, then a short roll into the next word.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let (lo, hi) = self.register(ctx);
        let beat = ctx.show.grid.beat;
        // hits = T.TITLE_HITS.get(self.name) or (self.root_of(0.0),)
        let first = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) if !h.is_empty() => h[0],
            _ => self.root_of(ctx, 0.0)?,
        };
        let r = f64::from(fit_reg(first, lo, hi));
        mix.place(&bus, &Self::sig(&self.hit(ctx, r, 1.0, true, 0.2)?), t, 0.72, pan);
        mix.place("fx", &Sig::Mono(synth::thud(0.6, 100.0, 30.0, 5)), t, 0.22, 0.0);
        let x = self.roll(ctx, r, beat * 0.5, 0.6)?;
        mix.place(&bus, &Self::sig(&x), t + beat * 0.5, 0.30, pan);
        Ok(())
    }

    /// A boom on each count, a roll into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let r = f64::from(self.root_of(ctx, grid.bar)?);
        for j in 0..3 {
            let x = self.hit(ctx, r, 0.95, true, 0.2)?;
            mix.place(&bus, &Self::sig(&x), grid.bar + f64::from(j) * grid.beat, 0.52, pan);
        }
        let x = self.roll(ctx, r, grid.beat * 0.95, 0.85)?;
        mix.place(&bus, &Self::sig(&x), grid.bar + 3.0 * grid.beat, 0.42, pan);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let ev = &ctx.rec.events;
        let segs = state::segs(ev, self.k);
        let drolls = state::drolls(ev, self.k);
        // busy = [(S_['t0'], S_['t_end']) for S_ in look.SEGS] + list(look.DROLLS)
        let busy: Vec<(f64, f64)> = segs
            .iter()
            .map(|s| (s.t0, s.t_end))
            .chain(drolls.iter().copied())
            .collect();
        self.kick(ctx, mix, &busy)?;
        self.rolls(ctx, mix, &segs, &drolls)?;
        self.events(ctx, bed, mix, &state::cannons(ev, self.k))
    }

    /// The drum caves in: a dying roll, a last flat thud, the rods rattling.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let r = f64::from(self.root_of(ctx, t0)?);
        mix.place(
            &self.bus(ctx),
            &Self::sig(&self.roll(ctx, r, 0.8, 0.6)?),
            t0,
            0.32,
            self.pan(),
        );
        mix.place("fx", &Sig::Mono(synth::thud(0.5, 80.0, 30.0, 81)), t0 + 0.8, 0.22, 0.0);
        for j in 0..6u64 {
            let rattle = synth::thud(0.05, 1400.0, 900.0, 90 + j);
            mix.place("fx", &Sig::Mono(rattle), t0 + 0.1 + 0.08 * j as f64, 0.05, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = f64::from(self.root_of(ctx, 0.0)?);
        let s16 = ctx.show.grid.s16;
        mix.place("solo", &Self::sig(&self.hit(ctx, r, 0.95, true, 0.2)?), tw, 0.5, 0.0);
        mix.place(
            "solo",
            &Self::sig(&self.hit(ctx, r, 0.9, true, 0.2)?),
            tw + 2.0 * s16,
            0.46,
            0.0,
        );
        mix.place(
            "solo",
            &Self::sig(&self.hit(ctx, r, 1.0, true, 0.2)?),
            tw + 4.0 * s16,
            0.6,
            0.0,
        );
        Ok(())
    }

    /// The config's solo as a bass-drum cadence: under MIDI 24 a roll on MIDI + 24; a note of a beat or more booms.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        let (lo, hi) = self.register(ctx);
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            if m < 24 {
                let dur = py_max(0.1, d * grid.beat * 0.95);
                let x = self.roll(ctx, f64::from(fit_reg(m + 24, lo, hi)), dur, 0.85)?;
                mix.place("solo", &Self::sig(&x), ts, 0.46, 0.0);
            } else {
                let long = d >= 1.0;
                let vel = if long { 0.95 } else { 0.75 };
                let x = self.hit(ctx, f64::from(fit_reg(m, lo, hi)), vel, long, d * grid.beat * 0.8)?;
                mix.place("solo", &Self::sig(&x), ts, if long { 0.6 } else { 0.48 }, 0.0);
            }
        }
        Ok(())
    }

    /// Its pieces fly home: a roll climbing in pitch.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let r = self.root_of(ctx, 0.0)?;
        for (j, m) in [r, r + 3, r + 5, r + 7, r + 10, r + 12].into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / 6.0);
            let x = self.hit(ctx, f64::from(fit_reg(m, lo, lo + 26)), 0.7, false, 0.1)?;
            mix.place("solo", &Self::sig(&x), tj, 0.32, 0.0);
        }
        Ok(())
    }

    /// The band's bass drum in the build: a boom on every beat of the last bar, a roll into the title slam.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let r = f64::from(self.root_of(ctx, t_last)?);
        for j in 0..3 {
            let x = self.hit(ctx, r, 0.95, true, 0.2)?;
            mix.place(&bus, &Self::sig(&x), t_last + f64::from(j) * grid.beat, 0.5, pan);
        }
        let x = self.roll(ctx, r, grid.beat - 0.02, 0.9)?;
        mix.place(&bus, &Self::sig(&x), t_last + 3.0 * grid.beat, 0.42, pan);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_move_by_octaves_into_the_register() {
        assert_eq!(fit_reg(62, 28, 50), 50);
        assert_eq!(fit_reg(14, 28, 50), 38);
        assert_eq!(fit_reg(28, 28, 50), 28);
        assert_eq!(HYMN.iter().max(), Some(&9));
    }

    /// `self.hit(self.fit_reg(m), ...)` then `int(m)`: OBOE's sour twin, 68.5 with a register topping at 56, is
    /// moved to 44.5 and cut to 44. The order matters at the top of the register: 56.5 moves to 44.5 (44), where
    /// cutting first would keep 56.
    #[test]
    fn a_note_is_moved_into_the_register_before_it_is_cut() {
        assert_eq!(fit_reg_f(68.5, 28, 56), 44.5);
        assert_eq!(whole(fit_reg_f(68.5, 28, 56)).unwrap(), 44);
        assert_eq!(whole(fit_reg_f(56.5, 28, 56)).unwrap(), 44);
        assert_eq!(fit_reg(whole(56.5).unwrap(), 28, 56), 56);
        assert_eq!(fit_reg_f(14.0, 28, 50), 38.0);
    }

    /// A bed that plays nothing, with a pit or without one: its timpani stroke on `m` is ten samples of `m`, its
    /// marimba stroke five of `1000 + m`.
    struct PitBed(bool);

    impl Bed for PitBed {
        fn hooks(&self) -> &'static [bf_sound::traits::BedHook] {
            &[]
        }
        fn has_pit(&self) -> bool {
            self.0
        }
        fn timp(&self, m: i32) -> Result<Vec<f64>, MusicError> {
            assert!(self.0, "timp asked of a bed without a pit");
            Ok(vec![f64::from(m); 10])
        }
        fn marimba(&self, m: i32) -> Result<Vec<f64>, MusicError> {
            assert!(self.0, "marimba asked of a bed without a pit");
            Ok(vec![1000.0 + f64::from(m); 5])
        }
        fn buses(&self) -> &[(&'static str, (f64, f64))] {
            &[]
        }
        fn reverb(&self) -> (f64, f64, f64) {
            (1.0, 0.5, 0.2)
        }
        fn comp(&self) -> (f64, f64) {
            (0.2, 0.5)
        }
        fn roar_gain(&self) -> f64 {
            0.2
        }
        fn doors_gain(&self) -> f64 {
            0.5
        }
        fn landing(&self) -> (f64, f64) {
            (0.08, 0.06)
        }
        fn pop(&self) -> (f64, f64, f64) {
            (95.0, 102.0, 0.12)
        }
        fn boom(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: f64) -> Played {
            Ok(())
        }
        fn clash(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn title(&self, _: &MusicCtx, _: &mut Mix, _: [f64; 3]) -> Played {
            Ok(())
        }
        fn vs(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn title_rest(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn count_in(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn fight_start(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn fight(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
            Ok(())
        }
        fn ko(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
        fn stinger(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
            Ok(())
        }
        fn after_ko(&self, _: &MusicCtx, _: &mut Mix, _: usize, _: f64, _: f64) -> Played {
            Ok(())
        }
        fn victory(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
            Ok(())
        }
        fn last_bars(&self, _: &MusicCtx, _: &mut Mix) -> Played {
            Ok(())
        }
    }

    /// `_pit(mix, t, m)` (bass_drum.py 566 to 582): on a bed with a pit, the timpani on the note moved into C2 to D3
    /// at 0.42, then a six-stroke marimba roll on 32nds (0.06, then 0.05) on the note moved up to C5 or above and
    /// on its octave, then the bell on the fx bus; on a bed without one, the bell alone.
    #[test]
    fn the_hymn_is_doubled_in_the_pit_when_the_bed_has_one() {
        let (t, s16) = (10.0, 0.119);
        let part = BassDrumSound::new(0);
        // the pit bus the kit's `_pit` makes for the timpani note `tm` and the marimba note `hm`
        let want = |tm: i32, hm: i32| {
            let mut mix = Mix::new(12.0);
            mix.place("pit", &Sig::Mono(vec![f64::from(tm); 10]), t, 0.42, 0.0);
            for dm in [0, 12] {
                for r in 0..6usize {
                    let g = if r == 0 { 0.05 + 0.01 } else { 0.05 };
                    let x = Sig::Mono(vec![1000.0 + f64::from(hm + dm); 5]);
                    mix.place("pit", &x, t + r as f64 * s16 / 2.0, g, 0.0);
                }
            }
            mix.bus("pit").unwrap().to_vec()
        };
        for (m, tm, hm) in [(27, 39, 75), (64, 40, 76), (48, 48, 72)] {
            let mut plain = Mix::new(12.0);
            part.pit(&PitBed(false), &mut plain, t, m, s16).unwrap();
            assert!(plain.bus("pit").is_none(), "no pit, no pit bus");
            let mut mix = Mix::new(12.0);
            part.pit(&PitBed(true), &mut mix, t, m, s16).unwrap();
            assert_eq!(mix.bus("pit").unwrap(), &want(tm, hm)[..], "the pit on {m}");
            assert_eq!(mix.bus("fx"), plain.bus("fx"), "the same bell over it");
            assert!(mix.bus("fx").is_some());
        }
    }
}
