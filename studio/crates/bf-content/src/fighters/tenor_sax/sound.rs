//! TENOR SAX's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/tenor_sax.py` (#51).
//!
//! The husky, honking low line. Each pose is a big honk on the root, each throw a note of its riff (its line, the
//! notes thinned as its HP falls), a landed throw a growl an octave up and a squish; WALK THE BAR honks up the line
//! on every beat over a low thud; SUBTONE is a breathy subtone note over the crowd's hush, then one fat growl and
//! the crowd's roar; STAGE DIVE is a rising scream into the roar, a high honk at the launch and the lowest honk at
//! the landing.
//!
//! Every note is the tone of its own voice (`voice::tenor_tone`) with the part's own `honk` and `sub`, resampled,
//! fitted to the note's length plus a quarter of a second and brought to the series' level (`M.inst_gain`), and
//! kept by `(note, length, velocity, honk, sub)` as the Python part keeps them: two calls whose length differs only
//! past the third decimal share the first one's samples.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Thinner, bend, clank, fit, gliss, noise_seed, roar, vibrato};
use bf_sound::synth;
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;

use super::state;
use super::voice::{VOICE_HONK, midi_hz, tenor_tone};

/// The name the part's errors carry.
const H: &str = "TENOR SAX";

/// `clank`'s default `decays`.
const CLANK_DECAYS: [(f64, f64); 3] = [(1.0, 0.12), (2.76, 0.06), (5.4, 0.03)];

/// The key `tone` caches by: `(int(m), round(dur, 3), round(vel, 2), round(honk, 2), round(sub, 2))`, the floats by
/// their bits.
type ToneKey = (i32, u64, u64, u64, u64);

fn bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// Python's `int(round(x))` for a count of beats or 16ths.
fn slot(x: f64) -> Result<i64, MusicError> {
    let r = round_half_even(x);
    if r.abs() < 9.0e15 {
        Ok(r as i64)
    } else {
        Err(MusicError::new(H, format!("{x} is not a place on the grid")))
    }
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

/// `squish(seed)`: a throw landing: 0.16 s of low-passed noise dying away over a 95 Hz thump.
fn squish(seed: u64) -> Vec<f64> {
    let n = (0.16 * SRF) as usize;
    let t = synth::tt(n);
    // x = A.lp(np.random.default_rng(seed).normal(0, 1, n), 900) * np.exp(-t / 0.05)
    let low = synth::lp(&synth::noise(seed, n), 900.0, 2);
    let w = 2.0 * PI * 95.0;
    // x + np.sin(2 * np.pi * 95 * t) * np.exp(-t / 0.04) * 0.6
    (0..n)
        .map(|i| {
            let x = low[i] * dmath::exp(-t[i] / 0.05);
            x + dmath::sin(w * t[i]) * dmath::exp(-t[i] / 0.04) * 0.6
        })
        .collect()
}

/// TENOR SAX's part.
#[derive(Debug)]
pub struct TenorSaxSound {
    k: usize,
    /// `self._c`: the tones made so far.
    tones: Mutex<BTreeMap<ToneKey, Arc<[f64]>>>,
}

impl TenorSaxSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> TenorSaxSound {
        TenorSaxSound {
            k,
            tones: Mutex::new(BTreeMap::new()),
        }
    }

    /// `self.pan`: a little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.15 } else { 0.15 }
    }

    /// `self.lo, self.hi = M.CFG[k]['register']`.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// `tone(m, dur, vel=0.85, honk=0.35, sub=0.0)`: one note of its voice, at 48 kHz, `dur + 0.25` s long and at
    /// the series' level: `M.fit(_resample(tenor_tone(_midi_hz(int(m)), dur, vel, honk=honk, sub=sub)), dur + 0.25)
    /// * g0`, kept by `(int(m), round(dur, 3), round(vel, 2), round(honk, 2), round(sub, 2))`.
    ///
    /// # Errors
    /// An argument is not a number, or the note would have a negative number of samples (Python raises there); the
    /// fit has no room for its fade; the voice's level cannot be measured.
    pub fn tone(
        &self,
        ctx: &MusicCtx,
        m: i32,
        dur: f64,
        vel: f64,
        honk: f64,
        sub: f64,
    ) -> Result<Arc<[f64]>, MusicError> {
        for (what, v) in [("dur", dur), ("vel", vel), ("honk", honk), ("sub", sub)] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("a tone's {what} is {v}, not a number")));
            }
        }
        // self.g0 = M.inst_gain(M.CFG[k]['pp']), measured when the part is made: the voice draws no shared noise,
        // so measuring it here (once: the context keeps it) gives the same soundtrack
        let g0 = ctx.inst_gain(&ctx.cfg[self.k].pp)?;
        let key = (
            m,
            bits(round_nd(dur, 3)),
            bits(round_nd(vel, 2)),
            bits(round_nd(honk, 2)),
            bits(round_nd(sub, 2)),
        );
        let mut tones = self
            .tones
            .lock()
            .map_err(|_| MusicError::new(H, "a tone panicked earlier"))?;
        if let Some(x) = tones.get(&key) {
            return Ok(Arc::clone(x));
        }
        if !pp::has_len(dur + 0.18) {
            return Err(MusicError::new(
                H,
                format!("no tone for a held length (dur) of {dur} s: it would end before it starts"),
            ));
        }
        let x = tenor_tone(midi_hz(f64::from(m)), dur, vel, honk, sub);
        let fitted = fit(&pp::to_48k(&x), dur + 0.25, 0.04)?;
        let out: Arc<[f64]> = fitted.iter().map(|v| v * g0).collect();
        tones.insert(key, Arc::clone(&out));
        Ok(out)
    }

    /// [`TenorSaxSound::tone`] as a mono sound to place.
    fn sig(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, honk: f64, sub: f64) -> Result<Sig, MusicError> {
        Ok(Sig::Mono(self.tone(ctx, m, dur, vel, honk, sub)?.to_vec()))
    }

    /// `fit_reg(m)` with its defaults: into its whole register.
    fn in_reg(&self, ctx: &MusicCtx, m: i32) -> i32 {
        let (lo, hi) = self.register(ctx);
        fit_reg(m, lo, hi)
    }

    /// `line(t)`: its line's note at `t`, in the bottom octave and a fifth of its register.
    ///
    /// # Errors
    /// The line has a rest there (Python cannot move `None` by octaves), or as [`MusicCtx::line_at`].
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let name = self.name(ctx);
        let lo = self.register(ctx).0;
        let m = ctx.line_at(name, t)?.ok_or_else(|| {
            MusicError::new(
                H,
                format!("{name}'s line has a rest at {t} s, where its riff needs a note"),
            )
        })?;
        Ok(fit_reg(m, lo, lo + 19))
    }

    /// `root(t)`: the root of the chord at `t`, the lowest at or above its bottom note.
    fn root(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let lo = self.register(ctx).0;
        ctx.harmony.root_above(ctx.harmony.chord_at(t), lo).map_err(bad)
    }

    /// `self.fit_reg(self.T.root_above(self.T.CHORDS[0], self.lo))`: the first bar's root, in its register.
    fn first_root(&self, ctx: &MusicCtx) -> Result<i32, MusicError> {
        let lo = self.register(ctx).0;
        let chord = ctx
            .harmony
            .chords
            .first()
            .ok_or_else(|| MusicError::new(H, "the fight has no chords"))?;
        Ok(self.in_reg(ctx, ctx.harmony.root_above(chord, lo).map_err(bad)?))
    }

    /// `col(x, t)`: `self.colour(x, t)`: the tone and the gain at time `t`.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let (sig, g, _lead) = self.colour(ctx, x, t, 1.0, &self.colour_args())?;
        Ok((sig, g))
    }
}

impl Sound for TenorSaxSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        2.0
    }

    fn send(&self) -> f64 {
        0.22
    }

    /// The honking low line's fundamentals and husk.
    fn band(&self) -> (f64, f64) {
        (160.0, 700.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 60.0,
            cut_span: 200.0,
            drive: 5.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.tone(m, dur, vel)`, at the voice's own honk.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        if !m.is_finite() {
            return Err(MusicError::new(H, format!("a note's midi is {m}, not a number")));
        }
        self.tone(ctx, m.trunc() as i32, dur, vel, VOICE_HONK, 0.0)
    }

    /// A big rock 'n' roll honk (the fight file's title hits: a low honk with an octave and a growl on top).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        // hits = self.T.TITLE_HITS.get(self.name) or (self.root(0.0),)
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) if !h.is_empty() => h.clone(),
            _ => vec![self.root(ctx, 0.0)?],
        };
        let x = self.tone(ctx, hits[0], 0.6, 1.0, 1.0, 0.0)?;
        mix.place(&bus, &Sig::Mono(gliss(&x, 2.0, 0.07)?), t, 0.62, pan);
        for (j, &m) in hits.iter().skip(1).enumerate() {
            let at = t + 0.02 * (j as f64 + 1.0);
            mix.place(&bus, &self.sig(ctx, m, 0.45, 0.9, 0.7, 0.0)?, at, 0.30, pan);
        }
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (bar, beat) = (grid.bar, grid.beat);
        let r = self.root(ctx, bar)?;
        let notes = [
            (0.0, r),
            (beat, r),
            (2.0 * beat, r + 7),
            (3.0 * beat, r + 5),
            (3.5 * beat, r + 3),
        ];
        for (dt, m) in notes {
            let x = self.sig(ctx, self.in_reg(ctx, m), beat * 0.4, 0.9, 0.8, 0.0)?;
            mix.place(&bus, &x, bar + dt, 0.42, pan);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (k, bus, pan) = (self.k, self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let (lo, hi) = self.register(ctx);
        let t_ko = ctx.show.t_ko;
        let ev = &ctx.rec.events;
        let mut th = Thinner::new(530.0 + k as f64);
        let mut riff_j: BTreeMap<i64, i64> = BTreeMap::new();
        for e in ev {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            let tq = ctx.q16(t);
            match e.kind.as_str() {
                "pose" => {
                    // the pose: a big honk on the root
                    let m = self.root(ctx, tq)?;
                    let x = self.tone(ctx, m, beat * 0.7, 1.0, 1.0, 0.0)?;
                    let (sig, g) = self.col(ctx, &x, t)?;
                    mix.place(&bus, &Sig::Mono(gliss(&sig, 1.5, 0.06)?), tq, 0.55 * g, pan);
                }
                "throw" => {
                    // each throw: a note of the riff (8ths of its line)
                    let vol = e.i(11);
                    let j = riff_j.get(&vol).copied().unwrap_or(0);
                    riff_j.insert(vol, j + 1);
                    if !th.keep(round_half_even(tq / s16), ctx.hp_at(k, tq) / 100.0) {
                        continue;
                    }
                    let m = self.line(ctx, tq)?;
                    let x = self.tone(ctx, m, s16 * 1.7, 0.9, 0.45, 0.0)?;
                    let (sig, g) = self.col(ctx, &x, t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.44 * g, pan + 0.1 * dmath::sin(j as f64));
                }
                "splat" => {
                    let seed = noise_seed(H, (t * 100.0).trunc())?;
                    if e.at(6).truthy() {
                        // a landed throw: a growl stab + the squish
                        let m = fit_reg(self.line(ctx, tq)? + 12, lo + 7, hi);
                        mix.place(&bus, &self.sig(ctx, m, s16 * 1.2, 1.0, 1.0, 0.0)?, tq, 0.28, pan);
                        mix.place("fx", &Sig::Mono(squish(seed)), tq, 0.10, 0.0);
                    } else {
                        mix.place("fx", &Sig::Mono(squish(seed)), tq, 0.04, 0.0);
                    }
                }
                "hush" => {
                    // SUBTONE: a breathy subtone over the crowd's hush
                    let t1 = e.f(3);
                    let m = self.in_reg(ctx, self.root(ctx, tq)? + 7);
                    let dur = py_max(0.2, t1 - tq);
                    mix.place(&bus, &self.sig(ctx, m, dur, 0.6, 0.0, 1.0)?, tq, 0.40, pan);
                    let n = (dur * SRF) as usize;
                    // sh = A.bp(np.random.default_rng(7).normal(0, 1, n), 2500, 8000) * np.linspace(1, 0.2, n)
                    let band = synth::bp(&synth::noise(7, n), 2500.0, 8000.0, 2);
                    let fade = dsp::linspace(1.0, 0.2, n);
                    let sh: Vec<f64> = band.iter().zip(&fade).map(|(b, f)| b * f).collect();
                    mix.place("fx", &Sig::Mono(sh), tq, 0.05, 0.0);
                }
                "subtone" => {
                    // ... then one fat growl as the crowd lets fly
                    let m = self.root(ctx, tq)?;
                    mix.place(&bus, &self.sig(ctx, m, beat, 1.0, 1.0, 0.0)?, tq, 0.58, pan);
                    mix.place(&bus, &self.sig(ctx, m + 12, beat * 0.8, 0.9, 0.8, 0.0)?, tq, 0.25, pan);
                    let seed = noise_seed(H, (t * 3.0).trunc())?;
                    mix.place("fx", &Sig::Mono(roar(1.4, seed)?), tq, 0.06, 0.0);
                }
                "runway" => {
                    // WALK THE BAR: honks up the line on every beat
                    let t0 = tq;
                    let t1 = state::runway_end(ev, k, e.f(8), t);
                    let nb = slot((t1 - t0) / beat)?;
                    for j in 0..nb {
                        let tj = t0 + j as f64 * beat;
                        if tj > t_ko {
                            break;
                        }
                        let up = if j % 2 == 0 { 0 } else { 5 };
                        let m = fit_reg(self.line(ctx, tj)? + up, lo, lo + 19);
                        let x = self.tone(ctx, m, beat * 0.55, 1.0, 0.9, 0.0)?;
                        let (sig, g) = self.col(ctx, &x, tj)?;
                        mix.place(&bus, &Sig::Mono(sig), tj, 0.48 * g, pan);
                        mix.place("fx", &Sig::Mono(synth::thud(0.2, 90.0, 50.0, j as u64)), tj, 0.08, 0.0);
                    }
                }
                "dive" => {
                    // STAGE DIVE: a rising scream into the crowd's roar
                    let m = fit_reg(self.root(ctx, tq)? + 12, lo + 12, hi);
                    let x = self.tone(ctx, m, beat * 1.6, 1.0, 0.9, 0.0)?;
                    mix.place(&bus, &Sig::Mono(gliss(&x, 7.0, 0.35)?), tq, 0.45, pan);
                    mix.place("fx", &Sig::Mono(roar(2.5, 5)?), tq + 0.2, 0.10, 0.0);
                }
                "dive_launch" => {
                    let m = fit_reg(self.root(ctx, tq)? + 19, lo + 12, hi);
                    mix.place(&bus, &self.sig(ctx, m, beat * 0.8, 1.0, 1.0, 0.0)?, tq, 0.42, pan);
                    let whoosh = synth::whoosh(0.5, 300.0, 6000.0, true, 6);
                    mix.place("fx", &Sig::Mono(whoosh), tq, 0.14, 0.0);
                }
                "dive_land" => {
                    mix.place(&bus, &self.sig(ctx, lo, beat * 1.2, 1.0, 1.0, 0.0)?, tq, 0.62, pan);
                    mix.place("fx", &Sig::Mono(synth::thud(0.8, 110.0, 28.0, 9)), tq, 0.40, 0.0);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The honk sags a fifth and the keys rattle.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let m = self.root(ctx, t0)?;
        let x = self.tone(ctx, m, 1.3, 1.0, 1.0, 0.0)?;
        mix.place(
            &self.bus(ctx),
            &Sig::Mono(bend(&x, -7.0, 0.1, 1.2)?),
            t0,
            0.5,
            self.pan(),
        );
        for j in 0..4u64 {
            let c = clank(84.0 - 3.0 * j as f64, j + 4, 0.25, &CLANK_DECAYS)?;
            mix.place("fx", &Sig::Mono(c), t0 + 0.06 + 0.07 * j as f64, 0.05, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.first_root(ctx)?;
        let x = self.tone(ctx, r, 0.9, 1.0, 1.0, 0.0)?;
        mix.place("solo", &Sig::Mono(gliss(&x, 2.0, 0.08)?), tw, 0.6, 0.0);
        mix.place("solo", &self.sig(ctx, r + 12, 0.7, 0.9, 0.7, 0.0)?, tw, 0.25, 0.0);
        Ok(())
    }

    /// The fight file's solo: the short notes honked, the long ones held with a vibrato.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let honk = if d < 1.5 { 0.6 } else { 0.3 };
            let x = self.tone(ctx, m, py_max(0.1, d * grid.beat * 0.95), 0.95, honk, 0.0)?;
            let x = if d >= 1.5 {
                vibrato(&x, 0.35, 5.0, 0.2)?
            } else {
                x.to_vec()
            };
            mix.place("solo", &Sig::Mono(x), ts, 0.62, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let r = self.first_root(ctx)?;
        let run = [r, r + 3, r + 5, r + 7, r + 10, r + 12, r + 15, r + 19];
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / 8.0);
            let x = self.sig(ctx, self.in_reg(ctx, m), 0.12, 0.8, 0.4, 0.0)?;
            mix.place("solo", &x, tj, 0.30, 0.0);
        }
        Ok(())
    }

    /// The honking low line climbs into the title slam: root, fifth, flat seventh, octave honks on the beats.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let root = ctx.harmony.root_above(ctx.harmony.chord_at(t_last), lo).map_err(bad)?;
        let r = self.in_reg(ctx, root);
        for (bt, m) in [r, r + 7, r + 10, r + 12].into_iter().enumerate() {
            let x = self.sig(ctx, self.in_reg(ctx, m), grid.beat * 0.6, 1.0, 0.9, 0.0)?;
            mix.place(&bus, &x, t_last + bt as f64 * grid.beat, 0.50, pan);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_moved_into_the_register_by_octaves() {
        assert_eq!(fit_reg(50, 44, 76), 50);
        assert_eq!(fit_reg(90, 44, 76), 66);
        assert_eq!(fit_reg(30, 44, 76), 54);
        // the riff's window, an octave and a fifth: down first, then up, as the Python loops run
        assert_eq!(fit_reg(64, 44, 63), 52);
        assert_eq!(fit_reg(63, 44, 63), 63);
    }

    #[test]
    fn the_squish_is_a_sixth_of_a_second_and_seeded() {
        let x = squish(1234);
        assert_eq!(x.len(), 7680);
        assert_eq!(x, squish(1234));
        assert_ne!(x, squish(1235));
        assert!(x.iter().all(|v| v.is_finite()));
        // it has died away by its end
        let tail = x[7200..].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(tail < 0.05, "{tail}");
    }
}
