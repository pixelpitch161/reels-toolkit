//! CLARINET's part in the score: the `Sound` half of `kit/fighters/clarinet.py` (#37's `audio.py`).
//!
//! The lower, liquid line: every jump plays its line (the fight file's `lines`, one note a beat; CLARION's 8th jumps
//! add the passing notes), a landed cut adds the twelfth above (the register key); it plays flat with a beating second
//! reed while OUT OF TUNE; SQUEAK is the squeal, RHAPSODY five notes gliding up the registers; the stunned enemy gets
//! a dizzy "boing".
//!
//! Its notes do not come from its Pixel Pitch voice (`M.note`) but straight from the series' reed synth
//! (`clarinet60_lowpoly.reed`, [`reed`]) at fractional pitches, through a cache of its own ([`ClarinetSound::clar`]):
//! the kit keys that cache by the rounded pitch, length and velocity and keeps the first note made under a key, so the
//! cache is part of the sound and is kept here as the kit keeps it. Nothing draws from the shared noise stream.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, py_min, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    ColourArgs, MusicCtx, MusicError, bend, clank, event_int, event_is, event_is_str, fit, gliss, thin_37,
};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;

use super::voice::{BOEHM, GOODMAN, Tone, reed};

/// The name the part's errors carry.
const H: &str = "CLARINET";

/// `clar`'s default velocity.
pub const CLAR_VEL: f64 = 0.85;
/// `title_blast`'s notes when the fight file gives none: a glissando blast up through the registers.
pub const TITLE_HITS: [i32; 7] = [50, 53, 57, 62, 65, 69, 74];

/// A tone of the series' reed synth the part asks for by name (`CLM.TONES[tone]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reed {
    /// `'boehm'`: its own tone (`clar`'s default).
    Boehm,
    /// `'goodman'`: SQUEAK's squeal.
    Goodman,
}

impl Reed {
    fn tone(self) -> &'static Tone {
        match self {
            Reed::Boehm => &BOEHM,
            Reed::Goodman => &GOODMAN,
        }
    }
}

/// `clar`'s cache key: `(round(m, 3), round(dur, 3), round(vel, 2), tone)`, the numbers by their bits.
type Key = (u64, u64, u64, Reed);

/// The CLARINET's part.
#[derive(Debug)]
pub struct ClarinetSound {
    k: usize,
    /// `self._vc`: every note made so far, by its rounded pitch, length, velocity and tone.
    notes: Mutex<BTreeMap<Key, Arc<[f64]>>>,
}

/// A rest where the clarinet needs a note (Python hands `None` to arithmetic there).
fn rest(name: &str, t: f64) -> MusicError {
    MusicError::new(
        H,
        format!("{name}'s line has a rest at {t} s: this part plays a note on every jump"),
    )
}

impl ClarinetSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> ClarinetSound {
        ClarinetSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// `clar(m, dur, vel=0.85, tone='boehm')`: one clarinet note (a fractional `m` is a detuned one) held `dur`
    /// seconds, from the series' reed synth: resampled to 48 kHz, cut to `dur + 0.14` s with a 30 ms fade, and
    /// levelled so the first quarter second's RMS is 0.16. The first note made under a key
    /// `(round(m, 3), round(dur, 3), round(vel, 2), tone)` is the note of every later call with that key.
    ///
    /// # Errors
    /// Where Python raises: a `dur` the synth or `fit` cannot make a note of.
    pub fn clar(&self, m: f64, dur: f64, vel: f64, tone: Reed) -> Result<Arc<[f64]>, MusicError> {
        if !pp::has_len(dur + 0.14) {
            return Err(MusicError::new(H, format!("a note held {dur} s")));
        }
        // (+ 0.0: a rounded -0.0 is the key 0.0, as Python's 0.0 == -0.0)
        let bits = |x: f64, nd: i32| (round_nd(x, nd) + 0.0).to_bits();
        let key = (bits(m, 3), bits(dur, 3), bits(vel, 2), tone);
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "an earlier note panicked"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(x.clone());
        }
        // x = CLM.reed(CLM.midi_hz(m), dur, vel, **CLM.TONES[tone])
        let x = reed(pp::midi_hz(m), dur, vel, tone.tone());
        // x = M.fit(M.B._resample(np.asarray(x, float)), dur + 0.14, fade=0.03)
        let x = fit(&pp::to_48k(&x), dur + 0.14, 0.03)?;
        // x * (0.16 / (np.sqrt((x[:int(0.25 * M.SR)] ** 2).mean()) + 1e-9))
        let head = &x[..x.len().min((0.25 * SRF) as usize)];
        let squares: Vec<f64> = head.iter().map(|v| v * v).collect();
        let level = 0.16 / (dsp::mean(&squares).sqrt() + 1e-9);
        let x: Arc<[f64]> = x.iter().map(|v| v * level).collect();
        notes.insert(key, x.clone());
        Ok(x)
    }

    /// `note(m, dur, vel, t)`: the clarinet as the run has it at `t`: flat by up to a quarter-tone while OUT OF
    /// TUNE, with a beating second reed (sour), thinner as it is hurt. Returns `(signal, gain, lead)`.
    ///
    /// # Errors
    /// As [`ClarinetSound::clar`] and [`Sound::colour`].
    pub fn note(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64, t: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let dep = self.detune(ctx, t)?;
        let mut x = self.clar(m - 0.5 * dep, dur, vel, Reed::Boehm)?.to_vec();
        if dep > 0.25 {
            let sour = self.clar(m - 0.5 * dep - 0.22, dur, vel * 0.9, Reed::Boehm)?;
            // x + 0.32 * dep * sour
            let k = 0.32 * dep;
            for (v, s) in x.iter_mut().zip(sour.iter()) {
                *v += k * s;
            }
        }
        self.colour(ctx, &x, t, 1.0, &self.colour_args())
    }

    /// The jumps of `play`: each plays its line (clarinet.py 343 to 382).
    fn jumps(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let k = self.k;
        let (name, bus) = (self.name(ctx), self.bus(ctx));
        let grid = &ctx.show.grid;
        let (s16, beat) = (grid.s16, grid.beat);
        let harmony = |e: HarmonyError| MusicError::new(H, e.to_string());
        // jumps = [e for e in M.EV if e[1] == 'jump' and e[2] == k and e[0] <= M.T_KO + 0.01]
        let mut jumps = Vec::new();
        for e in &ctx.rec.events {
            if e.kind == "jump" && event_is(H, e, 2, k)? && e.t <= ctx.show.t_ko + 0.01 {
                jumps.push(e);
            }
        }
        let mut rh_n = 0usize;
        let mut last_slot = None;
        let mut dropped = false;
        for (j, e) in jumps.iter().enumerate() {
            let (t, hit, kind) = (e.t, event_int(H, e, 7)?, event_int(H, e, 8)?);
            let tq = ctx.q16(t);
            // slot = round(tq / S16)
            let slot = round_half_even(tq / s16) as i64;
            if last_slot == Some(slot) {
                continue;
            }
            last_slot = Some(slot);
            let gap = match jumps.get(j + 1) {
                Some(next) => next.t - t,
                None => beat,
            };
            if kind == 1 {
                // RHAPSODY: five rising notes gliding up the registers
                let m = [50, 57, 62, 69, 74][rh_n.min(4)];
                rh_n += 1;
                let (sig, g, _) = self.note(ctx, f64::from(m), 0.22, 1.0, t)?;
                mix.place(&bus, &Sig::Mono(gliss(&sig, 5.0, 0.07)?), tq, 0.46 * g, -0.2);
                let air = synth::whoosh(0.18, 600.0, 7000.0, WHOOSH_RISE, WHOOSH_SEED);
                mix.place("fx", &Sig::Mono(air), tq - 0.05, 0.06, 0.0);
                continue;
            }
            let hp = ctx.hp_at(k, t) / 100.0;
            // sparser when hurt (never two notes in a row)
            dropped = thin_37(slot as f64, 3.0, hp, dropped);
            if dropped {
                continue;
            }
            let on_beat = imod_py(slot, 4) == 0;
            let mut m = ctx.line_at(name, tq)?.ok_or_else(|| rest(name, tq))?;
            if !on_beat {
                // CLARION's 8ths: a passing note toward the next beat
                let t2 = tq + beat / 2.0;
                let nxt = ctx.line_at(name, t2)?.ok_or_else(|| rest(name, t2))?;
                let scale = ctx
                    .harmony
                    .scale_notes(m.min(nxt), m.max(nxt), Some(ctx.harmony.chord_at(tq)))
                    .map_err(harmony)?;
                let pas: Vec<i32> = scale.into_iter().filter(|&x| x != m && x != nxt).collect();
                m = if pas.is_empty() { nxt } else { pas[pas.len() / 2] };
            }
            let dur = py_min(0.40, py_max(0.12, gap * 0.92));
            let (vel, level) = if on_beat { (0.85, 0.40) } else { (0.7, 0.30) };
            let (sig, g, lead) = self.note(ctx, f64::from(m), dur, vel, t)?;
            mix.place(&bus, &Sig::Mono(sig), tq, level * g, -0.22);
            if lead > 0.04 && on_beat {
                // the leader: a third under (fuller)
                let under = ctx
                    .harmony
                    .chord_tones(m - 5, m - 3, ctx.harmony.chord_at(tq))
                    .map_err(harmony)?;
                if let Some(&low) = under.last() {
                    let sig = self.note(ctx, f64::from(low), dur, 0.6, t)?.0;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.16 * g, -0.3);
                }
            }
            if hit != 0 {
                // a landed cut: the twelfth above (the register key)
                let sig = self.note(ctx, f64::from(m + 19), 0.09, 0.85, t)?.0;
                mix.place(&bus, &Sig::Mono(sig), tq + 0.03, 0.26 * g, -0.1);
                let air = synth::whoosh(0.10, 1500.0, 9000.0, WHOOSH_RISE, WHOOSH_SEED);
                mix.place("fx", &Sig::Mono(air), tq - 0.02, 0.05, 0.0);
            }
        }
        Ok(())
    }

    /// The events of `play`: the stunned enemy's boing and the three abilities (clarinet.py 383 to 403).
    fn events(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let cfg = ctx.cfg[k];
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind == "stun" && event_is(H, e, 3, k)? {
                // the stunned enemy: a dizzy "boing"
                let n = (0.4 * SRF) as usize;
                let tt = synth::tt(n);
                // np.sin(2 * np.pi * np.cumsum(420 + 140 * np.sin(2 * np.pi * 9 * tt_)) / M.SR) * np.exp(-tt_ / 0.15)
                let wob = 2.0 * PI * 9.0;
                let f: Vec<f64> = tt.iter().map(|&t| 420.0 + 140.0 * dmath::sin(wob * t)).collect();
                let boing: Vec<f64> = dsp::cumsum(&f)
                    .iter()
                    .zip(&tt)
                    .map(|(&c, &t)| dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t / 0.15))
                    .collect();
                mix.place("fx", &Sig::Mono(boing), ctx.q16(t) + 0.03, 0.08, 0.0);
            } else if e.kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let is = |name: &str| event_is_str(H, e, 3, name);
                if is(&cfg.unlock1.name)? {
                    // SQUEAK: a shrill, wobbling squeal
                    let x = self.clar(93.0, 0.22, 1.0, Reed::Goodman)?;
                    // np.tanh(3.0 * M.gliss(x, -1.5, 0.05) / (np.abs(x).max() + 1e-9)) * 0.5
                    let top = x.iter().fold(0.0f64, |p, v| if v.abs() > p { v.abs() } else { p }) + 1e-9;
                    let squeal: Vec<f64> = gliss(&x, -1.5, 0.05)?
                        .iter()
                        .map(|v| dmath::np_tanh(3.0 * v / top) * 0.5)
                        .collect();
                    mix.place(&bus, &Sig::Mono(squeal), tq, 0.34, -0.2);
                } else if is(&cfg.unlock2.name)? {
                    // CLARION: a rip up a twelfth into the clarion register
                    for (j, m) in [57.0, 62.0, 69.0, 76.0].into_iter().enumerate() {
                        let sig = self.note(ctx, m, 0.09, 0.9, t)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq + j as f64 * 0.035, 0.30, -0.2);
                    }
                } else if is(&cfg.low.name)? {
                    // RHAPSODY
                    mix.place("drums", &Sig::Mono(synth::crash(1.6)), tq, 0.08, 0.0);
                }
            }
        }
        Ok(())
    }
}

impl Sound for ClarinetSound {
    fn k(&self) -> usize {
        self.k
    }

    /// Its chalumeau line.
    fn band(&self) -> (f64, f64) {
        (140.0, 700.0)
    }

    /// `dict(cut0=60, cut_span=260, drive=4.0)`.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 60.0,
            cut_span: 260.0,
            drive: 4.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: one note in its voice: its own reed, not the Pixel Pitch entry.
    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.clar(m, dur, vel, Reed::Boehm)
    }

    /// A glissando blast up through the registers (the fight file's title hits).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .map_or(&TITLE_HITS[..], Vec::as_slice);
        for (j, &m) in hits.iter().enumerate() {
            let x = self.clar(
                f64::from(m),
                if j + 1 < hits.len() { 0.10 } else { 0.5 },
                1.0,
                Reed::Boehm,
            )?;
            mix.place(&bus, &Sig::Mono(gliss(&x, 2.0, 0.04)?), t + j as f64 * 0.032, 0.42, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        self.jumps(ctx, mix)?;
        self.events(ctx, mix)
    }

    /// A falling bent note, keys clattering.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        let x = self.clar(62.0, 1.0, 1.0, Reed::Boehm)?;
        // M.bend(x, -8.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x))
        let fall: Vec<f64> = bend(&x, -8.0, 0.05, 1.0)?
            .iter()
            .zip(dsp::linspace(1.0, 0.4, x.len()))
            .map(|(v, k)| v * k)
            .collect();
        mix.place(&bus, &Sig::Mono(fall), t_hs, 0.40, 0.0);
        let decays = [(1.0, 0.10), (2.76, 0.05), (5.4, 0.025)];
        for (j, m) in [86.0, 81.0, 91.0, 77.0, 84.0].into_iter().enumerate() {
            let key = clank(m, 20 + j as u64, 0.35, &decays)?;
            // pan=(j - 2) / 5
            mix.place(
                "fx",
                &Sig::Mono(key),
                t_hs + 0.08 * j as f64,
                0.07,
                (j as f64 - 2.0) / 5.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let _ = ctx;
        for (j, m) in [50.0, 53.0, 57.0, 62.0].into_iter().enumerate() {
            let x = self.clar(m, 0.9 - 0.06 * j as f64, 1.0, Reed::Boehm)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw + j as f64 * 0.03, 0.30, 0.0);
        }
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            // a late K.O. leaves less room: stop at the reassembly
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.clar(f64::from(m), d * grid.beat * 0.95, 0.95, Reed::Boehm)?;
            mix.place("solo", &Sig::Mono(gliss(&x, 1.0, 0.03)?), ts, 0.50, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let run = [62.0, 65.0, 69.0, 74.0, 77.0, 81.0];
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.clar(m - 12.0, 0.12, 0.7, Reed::Boehm)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), tj, 0.22, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_cut_levelled_and_kept_by_its_rounded_key() {
        let part = ClarinetSound::new(0);
        let x = part.clar(62.0, 0.2, 0.85, Reed::Boehm).unwrap();
        // int((0.2 + 0.14) * 48000) samples
        assert_eq!(x.len(), 16320);
        // the first quarter second's RMS is 0.16
        let rms = (dsp::mean(&x[..12000].iter().map(|v| v * v).collect::<Vec<_>>())).sqrt();
        assert!((rms - 0.16).abs() < 1e-6, "{rms}");
        // the same key gives the same note back, even from numbers that only round to it
        let again = part.clar(62.0004, 0.2004, 0.851, Reed::Boehm).unwrap();
        assert!(Arc::ptr_eq(&x, &again));
        // another key is another note: a detuned pitch, another tone
        let flat = part.clar(61.78, 0.2, 0.85, Reed::Boehm).unwrap();
        assert!(!Arc::ptr_eq(&x, &flat) && x[5000] != flat[5000]);
        let squeal = part.clar(62.0, 0.2, 0.85, Reed::Goodman).unwrap();
        assert!(!Arc::ptr_eq(&x, &squeal));
        assert_eq!(part.notes.lock().unwrap().len(), 3);
        // a note that cannot be made is an error, not a panic
        assert!(part.clar(62.0, -1.0, 0.85, Reed::Boehm).is_err());
        assert!(part.clar(62.0, f64::NAN, 0.85, Reed::Boehm).is_err());
    }

    #[test]
    fn its_mix_settings_are_the_kits() {
        let part = ClarinetSound::new(1);
        assert_eq!((part.k(), part.enemy()), (1, 0));
        assert_eq!((part.bus_gain(), part.send(), part.band()), (1.5, 0.35, (140.0, 700.0)));
        let a = part.colour_args();
        assert_eq!((a.cut0, a.cut_span, a.drive, a.low), (60.0, 260.0, 4.0, false));
        assert!(part.bed_calls().is_empty());
    }
}
