//! The FLUTE's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/flute.py` (#41).
//!
//! The high melody (role `melody`): while its cobras are up, one note of its line per 8th; FLUTTER TONGUE
//! flutter-tongues it with 16th grace notes, SNAKE DANCE slithers a semitone down after each note; a hiss-click
//! on every strike, a high accent on every bite, PICCOLO a piercing shriek; a slapped cobra hisses.
//!
//! Every note is asked for in the kit's order: each first-time FLUTE note draws from the shared noise stream.
//!
//! The part reads two things the kit keeps on the fighter's `Look` (`setup`): when its baskets were dropped and
//! when SNAKE DANCE ran. Both come straight from the run's events and are worked out here from them.

use bf_core::dmath::{self, PI};
use bf_core::event::{Event, Value};
use bf_core::py::{imod_py, py_max, py_min, round_nd};
use bf_core::rng::Generator;
use bf_core::roster::WeaponCfg;
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    HISS_DEC, HISS_HI, HISS_LO, MusicCtx, MusicError, Samples, Thinner, event_at, event_int, event_is, event_is_str,
    event_number, hiss, noise_seed,
};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, clip, noise};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "FLUTE";

/// `flute`'s default held length, in seconds.
pub const FLUTE_DUR: f64 = 0.1;
/// `flute`'s default velocity.
pub const FLUTE_VEL: f64 = 0.8;
/// `breath`'s default noise seed.
pub const BREATH_SEED: u64 = 5;
/// `flutter`'s default rate, in Hz.
pub const FLUTTER_RATE: f64 = 24.0;
/// `shriek`'s default length, in seconds.
pub const SHRIEK_DUR: f64 = 1.6;
/// `shriek`'s default pitch, in Hz (E7).
pub const SHRIEK_F0: f64 = 2637.0;
/// `swat_sound`'s default noise seed.
pub const SWAT_SOUND_SEED: u64 = 0;
/// `title_blast`'s notes when the fight file gives none: a shrieking flurry up to a high E.
pub const TITLE_HITS: [i32; 7] = [76, 79, 83, 84, 86, 87, 88];

/// The FLUTE's part.
#[derive(Clone, Copy, Debug)]
pub struct FluteSound {
    k: usize,
}

/// `int(seconds * SR)` as the length of a sound that goes through a filter: at least one sample.
fn samples(what: &'static str, seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if (1.0..600.0 * SRF).contains(&n) {
        Ok(n as usize)
    } else {
        Err(MusicError::new(what, format!("a length of {seconds} s")))
    }
}

/// `breath(dur, seed)`: flute breath: airy band noise (#32).
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn breath(dur: f64, seed: u64) -> Samples {
    let n = samples("breath", dur)?;
    let air = synth::bp(&noise(seed, n), 1500.0, 7000.0, FILTER_O);
    let env = synth::expenv(n, dur * 0.5, 0.01);
    Ok(air.iter().zip(&env).map(|(a, e)| a * e).collect())
}

/// `flutter(x, rate)`: flutter-tongue: a fast growl on the tone (#32): the tone at full level and at a tenth of
/// it in turn, `rate` times a second.
pub fn flutter(x: &[f64], rate: f64) -> Vec<f64> {
    let w = 2.0 * PI * rate;
    x.iter()
        .zip(synth::tt(x.len()))
        .map(|(v, t)| {
            // np.sign: 1, -1, or 0 at a zero of the sine.
            let s = dmath::sin(w * t);
            let sign = if s > 0.0 {
                1.0
            } else if s < 0.0 {
                -1.0
            } else {
                s * 0.0
            };
            v * (0.55 + 0.45 * sign)
        })
        .collect()
}

/// `shriek(dur, f0)`: PICCOLO: a piercing, wobbling high whistle with a hard edge (#32), on E7 by default, over
/// a breath of noise (seed 21).
///
/// # Errors
/// Where Python raises: `dur` is under one sample or not a number.
pub fn shriek(dur: f64, f0: f64) -> Samples {
    let n = samples("shriek", dur)?;
    let tt = synth::tt(n);
    let f: Vec<f64> = tt
        .iter()
        .map(|&t| f0 * (1.0 + 0.012 * dmath::sin(2.0 * PI * 7.5 * t)) * (1.0 + 0.06 * clip(t / 0.08, 0.0, 1.0)))
        .collect();
    let air = breath(dur, 21)?;
    Ok(dsp::cumsum(&f)
        .iter()
        .zip(&tt)
        .zip(&air)
        .map(|((&c, &t), &air)| {
            let ph = 2.0 * PI * c / SRF;
            let x = dmath::sin(ph) + 0.3 * dmath::sin(2.0 * ph) + 0.12 * dmath::sin(3.0 * ph);
            let x = dmath::tanh(1.8 * x) * clip(t / 0.02, 0.0, 1.0) * dmath::exp(-t / (dur * 0.6));
            x + 0.25 * air
        })
        .collect())
}

/// `swat_sound(seed)`: a lance slaps a cobra flat: a leathery slap (a short low-mid smack) and a startled,
/// falling hiss. One generator of `seed` gives the smack's noise, then the hiss's.
pub fn swat_sound(seed: u64) -> Vec<f64> {
    let n = (0.3 * SRF) as usize;
    let tt = synth::tt(n);
    let mut rng = Generator::new(seed);
    let mut draw = vec![0.0; n];
    rng.fill_normal(0.0, 1.0, &mut draw);
    let smack = synth::bp(&draw, 600.0, 3500.0, FILTER_O);
    rng.fill_normal(0.0, 1.0, &mut draw);
    let hs = synth::bp(&draw, 2500.0, 9000.0, FILTER_O);
    let fall: Vec<f64> = tt.iter().map(|&t| 260.0 * dmath::exp(-t / 0.04) + 120.0).collect();
    dsp::cumsum(&fall)
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let t = tt[i];
            let smack = smack[i] * dmath::exp(-t / 0.012);
            let body = dmath::sin(2.0 * PI * c / SRF) * dmath::exp(-t / 0.05);
            // np.maximum(tt_ - 0.04, 0)
            let late = if t - 0.04 > 0.0 { t - 0.04 } else { 0.0 };
            let hs = hs[i] * clip((t - 0.02) / 0.02, 0.0, 1.0) * dmath::exp(-late / 0.07);
            smack * 1.2 + body * 0.6 + hs * 0.5
        })
        .collect()
}

/// What the part reads of the FLUTE's `Look.setup()`: when its first basket landed, and when SNAKE DANCE ran.
struct Charmer {
    /// `min([b['t_drop'] for b in look.BASKETS.values()] + [99.0])`.
    first_drop: f64,
    /// `look.DANCES`: `(start, end)` of each SNAKE DANCE.
    dances: Vec<(f64, f64)>,
}

impl Charmer {
    fn new(ctx: &MusicCtx, k: usize) -> Result<Charmer, MusicError> {
        // BASKETS: basket id -> its drop time; a basket dropped again under the same id replaces the first.
        let mut baskets: Vec<(&Value, f64)> = Vec::new();
        for e in &ctx.rec.events {
            if e.kind == "basket" && event_is(H, e, 2, k)? {
                let id = event_at(H, e, 3)?;
                // (the kit also reads the basket's place and the one it replaces: elements 4 to 6)
                event_at(H, e, 6)?;
                match baskets.iter_mut().find(|(seen, _)| *seen == id) {
                    Some(slot) => slot.1 = e.t,
                    None => baskets.push((id, e.t)),
                }
            }
        }
        // Python's min: the first value, unless a later one is smaller.
        let first_drop = baskets
            .iter()
            .map(|&(_, t)| t)
            .chain([99.0])
            .reduce(py_min)
            .unwrap_or(99.0);
        // DANCES: every 'ability' event of kind 'dance' of its own, for the ability's length in beats.
        let beats = ctx.cfg[k].unlock2.get("dur_beats").unwrap_or(4.0);
        let mut dances = Vec::new();
        for e in &ctx.rec.events {
            if e.kind == "ability" && event_is_str(H, e, 4, "dance")? && event_is(H, e, 2, k)? {
                dances.push((e.t, e.t + beats * ctx.show.grid.beat));
            }
        }
        Ok(Charmer { first_drop, dances })
    }
}

impl FluteSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> FluteSound {
        FluteSound { k }
    }

    /// `flute(m, dur, vel)`: one note of the flute: `M.note('FLUTE', m, round(max(0.06, dur), 3),
    /// round(max(0.06, dur) + 0.18, 2), round(vel, 1))`. Python's defaults are [`FLUTE_DUR`] and [`FLUTE_VEL`].
    ///
    /// # Errors
    /// As [`MusicCtx::note`].
    pub fn flute(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64) -> Samples {
        let held = py_max(0.06, dur);
        let x = ctx.note(
            "FLUTE",
            f64::from(m),
            round_nd(held, 3),
            round_nd(held + 0.18, 2),
            round_nd(vel, 1),
        )?;
        Ok(x.to_vec())
    }

    /// `self.colour(x, t, drive_k)` with the part's own colour arguments.
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }

    /// The melody of `play`: one note per 8th while the cobras are up (flute.py 586-618).
    fn melody(&self, ctx: &MusicCtx, mix: &mut Mix, look: &Charmer) -> Played {
        let k = self.k;
        let (name, bus) = (self.name(ctx), self.bus(ctx));
        let s16 = ctx.show.grid.s16;
        let WeaponCfg::Striker(weapon) = &ctx.cfg[k].weapon else {
            return Err(MusicError::new(H, format!("{name} has no cobras to rise")));
        };
        let rise = weapon.rise * ctx.show.grid.beat;
        let t_up = look.first_drop + rise;
        let mut thin = Thinner::new(5.0);
        // k0, k1 = int(math.ceil(t_up / (2 * S16))), int(M.T_KO / (2 * S16))
        let (k0, k1) = ((t_up / (2.0 * s16)).ceil(), (ctx.show.t_ko / (2.0 * s16)).trunc());
        if !(k0.abs() < 1e9 && k1.abs() < 1e9) {
            return Err(MusicError::new(
                H,
                format!("the melody would run from 8th {k0} to {k1}"),
            ));
        }
        for s8 in k0 as i64..=k1 as i64 {
            let tq = (s8 * 2) as f64 * s16;
            let line = ctx.line_at(name, tq)?;
            let hp = ctx.hp_at(k, tq) / 100.0;
            if !thin.keep(s8 as f64, hp) {
                continue;
            }
            let m = line.ok_or_else(|| rest(name, tq))?;
            let hasted = ctx.run_at("HA", tq, k)? > 0.0;
            let dancing = look.dances.iter().any(|&(a, b)| a <= tq && tq < b);
            let dur = if hasted || dancing { 0.11 } else { 0.2 };
            let mut x = self.flute(ctx, m, dur, 0.8)?;
            if hasted {
                x = flutter(&x, FLUTTER_RATE);
            }
            let (sig, g, lead) = self.tone(ctx, &x, tq, 0.9)?;
            mix.place(&bus, &Sig::Mono(sig), tq, 0.26 * g, -0.2);
            if hasted || dancing {
                // The 16th between: a grace note. SNAKE DANCE slithers a semitone down.
                let m2 = if dancing {
                    m - 1
                } else {
                    let t2 = tq + 2.0 * s16;
                    ctx.line_at(name, t2)?.ok_or_else(|| rest(name, t2))?
                };
                if m2 != m || dancing {
                    let mut x2 = self.flute(ctx, m2, 0.1, 0.7)?;
                    if hasted {
                        x2 = flutter(&x2, FLUTTER_RATE);
                    }
                    let grace = self.tone(ctx, &x2, tq, 1.0)?.0;
                    mix.place(&bus, &Sig::Mono(grace), tq + s16, 0.20 * g, -0.2);
                }
            }
            if (1.0 - hp) > 0.3 {
                // breath(0.1, s8): the 8th's number is the seed.
                let seed = u64::try_from(s8).map_err(|_| MusicError::new(H, format!("8th {s8} is no noise seed")))?;
                mix.place(&bus, &Sig::Mono(breath(0.1, seed)?), tq, 0.07 * (1.0 - hp), -0.2);
            }
            if lead > 0.04 {
                // The leader: a thin octave double underneath.
                let low = self.flute(ctx, m - 12, dur, 0.6)?;
                let double = self.tone(ctx, &low, tq, 1.0)?.0;
                mix.place(&bus, &Sig::Mono(double), tq, 0.08 * g, -0.25);
            }
        }
        Ok(())
    }

    /// The events of `play`: strikes, bites, baskets, the wave, swats and the abilities (flute.py 619-654).
    fn events(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let s16 = ctx.show.grid.s16;
        let cfg = ctx.cfg[k];
        let hit = |e: &Event, how: &str| -> Result<bool, MusicError> {
            Ok(e.kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, how)?)
        };
        let side = |e: &Event| -> Result<f64, MusicError> { Ok(clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6)) };
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind == "strike" && event_is(H, e, 2, k)? {
                // A hiss-click on every strike (#32).
                let tq = ctx.q16(t);
                let n = (0.09 * SRF) as usize;
                let puff = hiss(0.09, noise_seed(H, t * 50.0)?, HISS_LO, HISS_HI, HISS_DEC)?;
                let x: Vec<f64> = synth::tt(n)
                    .iter()
                    .zip(&puff)
                    .map(|(&tt, &h)| {
                        let click = dmath::sin(2.0 * PI * 2800.0 * tt) * dmath::exp(-tt / 0.004);
                        h * 0.8 + click * 0.5
                    })
                    .collect();
                let pan = clip(dmath::cos(event_number(H, e, 4)?) * 0.4, -0.5, 0.5);
                mix.place("fx", &Sig::Mono(x), tq, 0.08, pan);
            } else if hit(e, "fang")? {
                // A bite lands: a high accent, a tone of the chord.
                let tq = ctx.q16(t);
                let chord = ctx.harmony.chord_at(tq + 0.01);
                let ct = ctx
                    .harmony
                    .chord_tones(84, 93, chord)
                    .map_err(|e| MusicError::new(H, e.to_string()))?;
                let m = if ct.is_empty() {
                    88
                } else {
                    ct[imod_py(event_int(H, e, 8)?, ct.len() as i64) as usize]
                };
                let (sig, g, _) = self.tone(ctx, &self.flute(ctx, m, 0.07, 1.0)?, t, 0.9)?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.16 * g, side(e)?);
            } else if e.kind == "basket" && event_is(H, e, 2, k)? {
                // A basket lands: a soft wicker thump.
                let thump = synth::thud(0.18, 220.0, 120.0, noise_seed(H, t * 9.0)?);
                mix.place("fx", &Sig::Mono(thump), t + 0.05, 0.07, 0.0);
            } else if hit(e, "wave")? {
                let thump = synth::thud(0.45, 140.0, 60.0, THUD_SEED);
                mix.place("fx", &Sig::Mono(thump), ctx.q16(t), 0.26, 0.0);
            } else if e.kind == "swat" && event_is(H, e, 3, k)? {
                // A lance slaps one of its cobras flat (#41).
                let tq = ctx.q16(t);
                let slap = swat_sound(noise_seed(H, t * 31.0)?);
                mix.place("fx", &Sig::Mono(slap), tq, 0.16, side(e)?);
                let startled = hiss(0.16, noise_seed(H, t * 17.0)?, 3000.0, 9000.0, 0.05)?;
                mix.place(&bus, &Sig::Mono(startled), tq + 0.02, 0.05, -0.2);
            } else if e.kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let is = |name: &str| event_is_str(H, e, 3, name);
                if is(&cfg.unlock1.name)? {
                    // FLUTTER TONGUE
                    let x = flutter(&self.flute(ctx, 88, 0.5, 0.9)?, 26.0);
                    mix.place(&bus, &Sig::Mono(x), tq, 0.20, -0.2);
                } else if is(&cfg.unlock2.name)? {
                    // SNAKE DANCE: a rattle of hisses as the cobras leave.
                    for j in 0..4u64 {
                        let rattle = hiss(0.12, 70 + j, 4000.0, 11000.0, 0.05)?;
                        let pan = 0.3 * if j % 2 == 1 { 1.0 } else { -1.0 };
                        mix.place(&bus, &Sig::Mono(rattle), tq + j as f64 * s16, 0.09, pan);
                    }
                } else if is(&cfg.low.name)? {
                    // PICCOLO: the highest note, a piercing shriek on E7.
                    mix.place(&bus, &Sig::Mono(shriek(1.7, SHRIEK_F0)?), tq, 0.13, -0.2);
                    let top = self.tone(ctx, &self.flute(ctx, 93, 0.9, 1.0)?, t, 2.0)?.0;
                    mix.place(&bus, &Sig::Mono(top), tq, 0.16, 0.0);
                }
            }
        }
        Ok(())
    }
}

/// A rest where the flute needs a note (Python hands `None` to `int()` there).
fn rest(name: &str, t: f64) -> MusicError {
    MusicError::new(
        H,
        format!("{name}'s line has a rest at {t} s: this part plays a note on every 8th"),
    )
}

impl Sound for FluteSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.7
    }

    fn send(&self) -> f64 {
        0.45
    }

    /// The high melody and its breath.
    fn band(&self) -> (f64, f64) {
        (600.0, 3200.0)
    }

    /// A shrieking flurry (the fight file's title hits) up to a flutter-tongued top note.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let s16 = ctx.show.grid.s16;
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .map_or(&TITLE_HITS[..], Vec::as_slice);
        for (j, &m) in hits.iter().enumerate() {
            let x = self.flute(ctx, m, 0.06, 1.0)?;
            mix.place(&bus, &Sig::Mono(x), t + j as f64 * s16 / 2.0, 0.30, -0.15);
        }
        let Some(&top) = hits.last() else {
            return Err(MusicError::new(H, "the title hits are an empty list"));
        };
        let x = flutter(&self.flute(ctx, top, 0.5, 1.0)?, 26.0);
        mix.place(&bus, &Sig::Mono(x), t + 3.5 * s16, 0.32, -0.15);
        mix.place(&bus, &Sig::Mono(breath(0.4, 81)?), t, 0.10, -0.15);
        Ok(())
    }

    /// Three rising chord tones of the count chord on 3, 2, 1, the middle one again on the pickup.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let bad = |problem: String| MusicError::new(H, problem);
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| bad("the episode has no bar 1 to count in on".into()))?;
        let (lo, hi) = ctx.cfg[self.k].register;
        let tones = ctx
            .harmony
            .chord_tones((lo + hi).div_euclid(2) + 1, hi + 12, ch)
            .map_err(|e| bad(e.to_string()))?;
        if tones.len() < 3 {
            return Err(bad(format!(
                "{ch} has fewer than three tones above the register's middle"
            )));
        }
        for (j, (tc, _)) in grid.count[..3].iter().enumerate() {
            let x = self.flute(ctx, tones[j], 0.12, 0.9)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.26, -0.2);
        }
        let x = self.flute(ctx, tones[1], 0.25, 0.9)?;
        mix.place(&bus, &Sig::Mono(x), grid.bar + 3.0 * grid.beat, 0.20, -0.2);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let look = Charmer::new(ctx, self.k)?;
        self.melody(ctx, mix, &look)?;
        self.events(ctx, mix)
    }

    /// The flute falls apart: a broken run down (#32).
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        for (j, m) in [93, 89, 84, 80, 77, 74].into_iter().enumerate() {
            let x = self.flute(ctx, m, 0.08, 0.8)?;
            let at = t_hs + 0.09 * j as f64;
            mix.place(&bus, &Sig::Mono(x), at, 0.20, (j as f64 - 2.5) / 6.0);
            mix.place(&bus, &Sig::Mono(breath(0.08, 40 + j as u64)?), at, 0.06, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let s16 = ctx.show.grid.s16;
        for (j, m) in [76, 79, 83, 88].into_iter().enumerate() {
            let x = self.flute(ctx, m, if j < 3 { 0.12 } else { 0.8 }, 1.0)?;
            mix.place("solo", &Sig::Mono(x), tw + j as f64 * s16 / 2.0, 0.30, 0.0);
        }
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            // A late K.O. leaves less room: stop at the reassembly.
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.flute(ctx, m, d * grid.beat * 0.9, 0.9)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.50, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        for (j, m) in [76, 79, 83, 84, 88, 91].into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / 6.0);
            let x = self.flute(ctx, m, 0.08, 0.7)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.16, 0.0);
        }
        Ok(())
    }
}
