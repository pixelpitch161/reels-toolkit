//! OBOE's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/oboe.py` (#37).
//!
//! The higher, nasal line (role `line`): its tuning A (A4, whatever the key) with every beam; when a beam lands the
//! A rings out over a sour twin a quarter-tone flat in the ENEMY's own voice, on the enemy's bus; every sting is one
//! of its running 8ths (the fight file's line); ENDLESS NOTE holds the A; DOUBLE REED is a reed crow and a scissor
//! snip; ILL WIND a bent, beating A over a gust.
//!
//! Its notes are not `M.note`'s: the part calls the Pixel Pitch module's `double_reed` itself, fits and levels each
//! note, and keeps them in a cache of its own (`_vc`), keyed by the rounded pitch, length and velocity. A later
//! note with the same rounded key is the first one's samples, as in Python, so the cache is part of the sound. The
//! score makes one part per soundtrack, as `kit.music` makes one `Sound` per build.
//!
//! The sour twin needs the other side's part (`M.SOUNDS[self.enemy]`): [`Sound::play_with`] hands it over (ruling
//! M3-R15).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath;
use bf_core::event::Event;
use bf_core::py::{round_half_even, round_nd};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    ColourArgs, MusicCtx, MusicError, bend, clank, event_int, event_is, event_is_str, event_number, fit, gliss,
    noise_seed, thin_37, thunder,
};
use bf_sound::synth::{self, FILTER_O, WHOOSH_SEED};
use bf_sound::traits::{Bed, Played, SOUND_VOICE_VEL, Sound};
use bf_sound::voice::pp;

use super::voice::double_reed;

/// The name the part's errors carry.
const H: &str = "OBOE";

/// `A4 = 69`: the tuning A, its idiom, whatever the key.
pub const A4: f64 = 69.0;

/// `scissor`'s default noise seed.
pub const SCISSOR_SEED: u64 = 0;

/// The key of `_vc`: `(round(m, 3), round(dur, 3), round(vel, 2))`, each as its bits.
type NoteKey = [u64; 3];

/// OBOE's part.
#[derive(Debug)]
pub struct OboeSound {
    k: usize,
    /// `self._vc`: the notes made so far.
    notes: Mutex<BTreeMap<NoteKey, Arc<[f64]>>>,
}

/// `scissor(seed)`: DOUBLE REED's snip: two bright metallic scrapes a few milliseconds apart. One generator of
/// `seed` gives the first scrape's noise, then the second's.
pub fn scissor(seed: u64) -> Vec<f64> {
    let n = (0.12 * SRF) as usize;
    let t = synth::tt(n);
    let mut rng = Generator::new(seed);
    let mut draw = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut draw);
    let a = synth::bp(&draw, 3000.0, 9000.0, FILTER_O);
    rng.fill_normal(0.0, 1.0, &mut draw);
    let b = synth::bp(&draw, 2500.0, 7000.0, FILTER_O);
    let b: Vec<f64> = b.iter().zip(&t).map(|(v, &t)| v * dmath::exp(-t / 0.018)).collect();
    // np.roll(b, int(0.028 * A.SR))
    let shift = (0.028 * SRF) as usize;
    (0..n)
        .map(|i| {
            let rolled = b[(i + n - shift % n) % n];
            a[i] * dmath::exp(-t[i] / 0.012) + rolled * 0.9
        })
        .collect()
}

/// `x * np.linspace(a, b, len(x))`.
fn ramp(x: &[f64], a: f64, b: f64) -> Vec<f64> {
    x.iter().zip(dsp::linspace(a, b, x.len())).map(|(v, g)| v * g).collect()
}

impl OboeSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> OboeSound {
        OboeSound {
            k,
            notes: Mutex::new(BTreeMap::new()),
        }
    }

    /// `oboe(m, dur, vel)`: one note of the oboe: the module's `double_reed` at 48 kHz, fitted to `dur + 0.12`
    /// seconds with a 30 ms fade, levelled on its first quarter second. A note asked for before (the same pitch
    /// to 3 places, length to 3, velocity to 2) is that note again.
    ///
    /// # Errors
    /// Where Python raises: a length the reed or the fit cannot make.
    pub fn oboe(&self, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = [round_nd(m, 3), round_nd(dur, 3), round_nd(vel, 2)].map(f64::to_bits);
        let mut notes = self
            .notes
            .lock()
            .map_err(|_| MusicError::new(H, "a note of the oboe panicked earlier"))?;
        if let Some(x) = notes.get(&key) {
            return Ok(x.clone());
        }
        if !pp::has_len(dur + 0.12) || !(m.is_finite() && vel.is_finite()) {
            return Err(MusicError::new(H, format!("no note of {dur} s at {m}")));
        }
        let raw = pp::to_48k(&double_reed(pp::midi_hz(m), dur, vel));
        let x = fit(&raw, dur + 0.12, 0.03)?;
        // x * (0.16 / (np.sqrt((x[:int(0.25 * M.SR)] ** 2).mean()) + 1e-9)) * 0.85
        let head = &x[..x.len().min((0.25 * SRF) as usize)];
        let sq: Vec<f64> = head.iter().map(|v| v * v).collect();
        let g = 0.16 / (dsp::mean(&sq).sqrt() + 1e-9);
        let x: Arc<[f64]> = x.iter().map(|v| v * g * 0.85).collect();
        notes.insert(key, x.clone());
        Ok(x)
    }

    /// `note(m, dur, vel, t)`: `self.colour(self.oboe(m, dur, vel), t)`: `(signal, gain, lead)`.
    fn note(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64, t: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let x = self.oboe(m, dur, vel)?;
        self.colour(ctx, &x, t, 1.0, &self.colour_args())
    }

    /// `play(mix)`, with the victim's part: the run's events up to the K.O.
    fn events(&self, ctx: &MusicCtx, mix: &mut Mix, victim: &dyn Sound) -> Played {
        let k = self.k;
        let (name, bus) = (self.name(ctx), self.bus(ctx));
        let (beat, s16) = (ctx.show.grid.beat, ctx.show.grid.s16);
        let low = &ctx.cfg[k].low.name;
        let mine = |e: &Event, kind: &str| -> Result<bool, MusicError> { Ok(e.kind == kind && event_is(H, e, 2, k)?) };
        let hit =
            |e: &Event, how: &str| -> Result<bool, MusicError> { Ok(mine(e, "hit")? && event_is_str(H, e, 7, how)?) };
        let mut o_dropped = false;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if mine(e, "tune")? {
                // the tuning A with every beam
                let tq = ctx.q16(t);
                let (sig, g, lead) = self.note(ctx, A4, 0.75 * beat, 0.95, t)?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.48 * g, 0.22);
                if lead > 0.04 {
                    let high = self.note(ctx, A4 + 12.0, 0.75 * beat, 0.6, t)?.0;
                    mix.place(&bus, &Sig::Mono(high), tq, 0.14 * g, 0.25);
                }
            } else if hit(e, "tune")? {
                // it lands: the A rings out against a sour twin
                let tq = ctx.q16(t);
                let ring = self.note(ctx, A4 + 12.0, 0.25, 0.9, t)?.0;
                mix.place(&bus, &Sig::Mono(ring), tq + s16, 0.22, 0.2);
                // in the enemy's own voice
                let sour: Vec<f64> = victim.voice(ctx, A4 - 0.5, 0.3, 0.7)?.iter().map(|v| v * 0.5).collect();
                let twin = ctx.colour(&sour, victim.k(), t, 1.0, &victim.colour_args())?.0;
                mix.place(&victim.bus(ctx), &Sig::Mono(twin), tq + s16, 0.18, -0.15);
            } else if hit(e, "sting")? {
                // each sting: one of the oboe's running 8ths
                let tq = ctx.q16(t);
                let hp = ctx.hp_at(k, t) / 100.0;
                o_dropped = thin_37(round_half_even(tq / s16), 11.0, hp, o_dropped);
                if o_dropped {
                    continue;
                }
                let m = ctx.line_at(name, tq)?.ok_or_else(|| {
                    MusicError::new(
                        H,
                        format!("{name}'s line has a rest at {tq} s: this part plays a note on every sting"),
                    )
                })?;
                let (sig, g, _) = self.note(ctx, f64::from(m), 0.13, 0.78, t)?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.30 * g, 0.25);
            } else if mine(e, "endless")? {
                // ENDLESS NOTE: the A held for a bar (circular breathing)
                let tq = ctx.q16(t);
                let dur = event_number(H, e, 3)? - t;
                let (sig, g, _) = self.note(ctx, A4, dur, 1.0, t)?;
                mix.place(&bus, &Sig::Mono(ramp(&sig, 0.85, 1.1)), tq, 0.46 * g, 0.2);
                let under = self.note(ctx, A4 - 12.0, dur, 0.7, t)?.0;
                mix.place(&bus, &Sig::Mono(under), tq, 0.14 * g, 0.2);
            } else if mine(e, "snip_open")? {
                // DOUBLE REED: a reed crow as the blades open
                let crow = gliss(&self.oboe(86.0, 0.12, 1.0)?, 3.0, 0.06)?;
                mix.place(&bus, &Sig::Mono(crow), ctx.q16(t), 0.22, 0.25);
            } else if mine(e, "snip")? {
                let landed = event_int(H, e, 3)? != 0;
                let snip = scissor(noise_seed(H, t * 10.0)?);
                mix.place(
                    "fx",
                    &Sig::Mono(snip),
                    ctx.q16(t),
                    if landed { 0.30 } else { 0.16 },
                    0.15,
                );
                if landed {
                    let thud = synth::thud(0.3, 160.0, 60.0, noise_seed(H, t * 7.0)?);
                    mix.place("fx", &Sig::Mono(thud), ctx.q16(t), 0.25, 0.0);
                }
            } else if mine(e, "ability")? && event_is_str(H, e, 3, low)? {
                // ILL WIND: a bent, beating A
                let tq = ctx.q16(t);
                let dur = 2.0 * beat;
                let y = bend(&self.oboe(A4 + 12.0, dur, 1.0)?, -1.5, 0.05, dur)?;
                let y2 = bend(&self.oboe(A4 + 11.5, dur, 0.8)?, -1.8, 0.05, dur)?;
                if y.len() != y2.len() {
                    return Err(MusicError::new(H, "the two bent notes of ILL WIND differ in length"));
                }
                let both: Vec<f64> = y.iter().zip(&y2).map(|(a, b)| a + 0.6 * b).collect();
                let wind = self.colour(ctx, &both, t, 1.0, &self.colour_args())?.0;
                mix.place(&bus, &Sig::Mono(wind), tq, 0.36, 0.2);
                let gust = synth::whoosh(1.2, 200.0, 3000.0, false, WHOOSH_SEED);
                mix.place("fx", &Sig::Mono(gust), tq, 0.22, 0.0);
                mix.place("fx", &Sig::Mono(thunder(1.4, 33, false)?), tq, 0.30, 0.0);
            }
        }
        Ok(())
    }
}

impl Sound for OboeSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.3
    }

    fn send(&self) -> f64 {
        0.3
    }

    /// Its nasal line.
    fn band(&self) -> (f64, f64) {
        (900.0, 3500.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 60.0,
            cut_span: 260.0,
            drive: 4.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: its own note ([`OboeSound::oboe`]), not the roster voice's.
    fn voice(&self, _ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.oboe(m, dur, vel)
    }

    /// Its A, held (the tuning note), with the octave.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        // hits = self.T.TITLE_HITS.get(self.name, (self.A4, self.A4 + 12))
        let (first, last) = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(hits) => match (hits.first(), hits.last()) {
                (Some(&a), Some(&b)) => (f64::from(a), f64::from(b)),
                _ => return Err(MusicError::new(H, "the title hits are an empty list")),
            },
            None => (A4, A4 + 12.0),
        };
        let a_hold = 1.6 * ctx.show.grid.beat;
        mix.place(&bus, &Sig::Mono(self.oboe(first, a_hold, 1.0)?.to_vec()), t, 0.52, 0.0);
        mix.place(&bus, &Sig::Mono(self.oboe(last, a_hold, 0.9)?.to_vec()), t, 0.30, 0.0);
        Ok(())
    }

    /// Without the other side's part there is no voice for the sour twin: the score calls [`Sound::play_with`].
    fn play(&self, _ctx: &MusicCtx, _: &dyn Bed, _mix: &mut Mix) -> Played {
        Err(MusicError::new(
            H,
            "the oboe's part needs the other side's part for its sour twin: call play_with",
        ))
    }

    fn play_with(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, enemy: &dyn Sound) -> Played {
        self.events(ctx, mix, enemy)
    }

    /// A falling, sour A, keys clattering.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_hs = ctx.show.t_hs;
        let x = self.oboe(A4, 1.0, 1.0)?;
        let fall = ramp(&bend(&x, -8.0, 0.05, 1.0)?, 1.0, 0.4);
        mix.place(&self.bus(ctx), &Sig::Mono(fall), t_hs, 0.40, 0.0);
        for (j, m) in [86.0, 81.0, 91.0, 77.0, 84.0].into_iter().enumerate() {
            let key = clank(m, 20 + j as u64, 0.35, &[(1.0, 0.10), (2.76, 0.05), (5.4, 0.025)])?;
            let at = t_hs + 0.08 * j as f64;
            mix.place("fx", &Sig::Mono(key), at, 0.07, (j as f64 - 2.0) / 5.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let _ = ctx;
        mix.place("solo", &Sig::Mono(self.oboe(A4, 0.9, 1.0)?.to_vec()), tw, 0.45, 0.0);
        mix.place(
            "solo",
            &Sig::Mono(self.oboe(A4 + 5.0, 0.9, 0.9)?.to_vec()),
            tw,
            0.30,
            0.0,
        );
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.oboe(f64::from(m), d * grid.beat * 0.95, 0.95)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.52, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let run = [62.0, 65.0, 69.0, 74.0, 77.0, 81.0];
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            mix.place(&bus, &Sig::Mono(self.oboe(m, 0.12, 0.7)?.to_vec()), tj, 0.20, 0.0);
        }
        Ok(())
    }
}

/// `Sound.voice`'s default velocity, which the oboe's `voice` keeps.
pub const VOICE_VEL: f64 = SOUND_VOICE_VEL;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_asked_for_again_is_the_first_one() {
        let part = OboeSound::new(0);
        let a = part.oboe(69.0, 0.25, 0.9).unwrap();
        // int((0.25 + 0.12) * 48000) samples
        assert_eq!(a.len(), 17760);
        // the same rounded key: the first note's samples, though the length differs in the 5th place
        let b = part.oboe(69.0, 0.25001, 0.9).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        // levelled: the first quarter second has an RMS of 0.16 x 0.85
        let sq: Vec<f64> = a[..12000].iter().map(|v| v * v).collect();
        let rms = dsp::mean(&sq).sqrt();
        assert!((rms - 0.16 * 0.85).abs() < 1e-9, "{rms}");
        assert!(part.oboe(69.0, -0.5, 0.9).is_err());
    }

    #[test]
    fn the_scissor_is_two_scrapes_28_ms_apart() {
        let x = scissor(3);
        assert_eq!(x.len(), 5760);
        assert_eq!(scissor(3), x);
        assert_ne!(scissor(4), x);
        assert!(x.iter().all(|v| v.is_finite()));
    }
}
