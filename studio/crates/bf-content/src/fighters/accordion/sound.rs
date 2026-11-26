//! ACCORDION's part in the score: #30's `audio.py` ACCORDION part, ported close enough for the ear (ruling M4-R16).
//!
//! Reed chords from its own synth (`voice.rs`, the golden reeds). BELLOWS BITE: the jaws open with the bass button
//! (oom) and a little inhale, and snap shut with a wheezy chord stab (pah); landed bites hit harder with a chomp.
//! SQUEEZEBOX folds shut with a squeal; FULL BELLOWS inhales for a beat and then blasts every reed at once. As its HP
//! drops the chords lose notes and wheeze. The chords follow the theme's harmony (`chord_at`), so any bed fits.
//!
//! Buses: its own `accordion` (`self.bus`), the shared `fx` and `solo`. Its own noise: every random draw is a
//! generator of its own seed.

use std::sync::Arc;

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::py::{py_max, round_nd};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, noise_seed};
use bf_sound::synth::{self, FILTER_O, WHOOSH_SEED};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;
use bf_sound::voice::real::{self, RealNotes};

use super::voice::{REWORK, reed};

/// The name the part's errors carry.
const H: &str = "ACCORDION";

fn n_of(seconds: f64) -> usize {
    (seconds * SRF) as usize
}

/// `_fit(x, cap)`: `x` cut or padded to `cap` seconds, the last 30 ms (or a third) faded out.
fn fit(x: &[f64], cap: f64) -> Vec<f64> {
    let n = n_of(cap);
    let mut y: Vec<f64> = x.iter().take(n).copied().collect();
    y.resize(n, 0.0);
    let fd = (y.len() / 3).min(n_of(0.03));
    if fd > 0 {
        let fade = dsp::linspace(1.0, 0.0, fd);
        let at = y.len() - fd;
        for (v, w) in y[at..].iter_mut().zip(&fade) {
            *v *= w;
        }
    }
    y
}

/// The reeds' level (`_G['acc']`): `min(6.0, 0.16 / rms)` of an E4, 0.2 s at 0.85, high-passed at 120 Hz.
fn reed_gain() -> f64 {
    let r = reed(pp::midi_hz(64.0), 0.2, 0.85, 1);
    let h = synth::hp(&r, 120.0, FILTER_O);
    let sq: Vec<f64> = h.iter().map(|v| v * v).collect();
    (0.16 / (dsp::mean(&sq).sqrt() + 1e-6)).min(6.0)
}

/// `reeds(ms, dur, vel, swell, wheeze, seed)`: an accordion chord in the golden reeds; `swell` is the pull's inhale
/// (the chord fades in), `wheeze` the bellows' air noise on top.
pub fn reeds(ms: &[f64], dur: f64, vel: f64, swell: bool, wheeze: f64, seed: u64) -> Vec<f64> {
    reeds_of(ms, dur, swell, wheeze, seed, &|m, s| {
        reed(pp::midi_hz(m), dur, vel, s).into()
    })
}

/// [`reeds`] with each reed of MIDI note `m` and seed `s` made by `one(m, s)`.
fn reeds_of(
    ms: &[f64],
    dur: f64,
    swell: bool,
    wheeze: f64,
    seed: u64,
    one: &dyn Fn(f64, u64) -> Arc<[f64]>,
) -> Vec<f64> {
    let n = n_of(dur + 0.3);
    let mut x = vec![0.0; n];
    for &m in ms {
        let y = one(m, (m as u64) * 3 + seed);
        for (o, v) in x.iter_mut().zip(y.iter()) {
            *o += v;
        }
    }
    let g = reed_gain() / (ms.len().max(1) as f64).sqrt();
    let t = synth::tt(n);
    for (o, &t) in x.iter_mut().zip(&t) {
        *o *= g;
        if swell {
            *o *= dmath::pow((t / py_max(0.05, dur * 0.85)).clamp(0.0, 1.0), 1.6);
        }
    }
    if wheeze > 0.0 {
        let air = synth::bp(&synth::noise(seed + 17, n), 900.0, 5200.0, FILTER_O);
        for (i, o) in x.iter_mut().enumerate() {
            let ti = t[i];
            let env = if swell {
                dmath::pow((ti / py_max(0.04, dur * 0.8)).clamp(0.0, 1.0), 1.4)
            } else {
                dmath::exp(-ti / 0.07)
            };
            if ti < dur + 0.08 {
                *o += wheeze * 0.35 * air[i] * env;
            }
        }
    }
    fit(&x, dur + 0.25)
}

/// `tear(dur, seed)`: the bellows tearing: a ripping burst of air and every reed sagging out of tune.
pub fn tear(dur: f64, seed: u64) -> Vec<f64> {
    let n = n_of(dur);
    let t = synth::tt(n);
    let rip = synth::bp(&synth::noise(seed, n), 700.0, 6000.0, FILTER_O);
    let mut x = vec![0.0; n];
    for m in [57.0, 60.0, 64.0, 69.0] {
        let f: Vec<f64> = t
            .iter()
            .map(|&t| synth::mtof(m) * (1.0 - 0.18 * dmath::pow(t / dur, 0.8)))
            .collect();
        let cs = dsp::cumsum(&f);
        for (o, c) in x.iter_mut().zip(cs) {
            let ph = (c / SRF).rem_euclid(1.0);
            *o += ((if ph < 0.3 { 1.0 } else { -0.43 }) + 0.4 * (2.0 * ph - 1.0)) * 0.25;
        }
    }
    synth::lp(&x, 2600.0, FILTER_O)
        .into_iter()
        .enumerate()
        .map(|(i, v)| v * dmath::exp(-t[i] / (dur * 0.4)) + rip[i] * dmath::exp(-t[i] / 0.18))
        .collect()
}

/// How many reeds of its chord still sound at `hp` (0 to 1): it loses notes as it gets hurt.
fn reeds_left(hp: f64) -> usize {
    if hp > 0.5 {
        4
    } else if hp > 0.25 {
        3
    } else {
        2
    }
}

/// `e[7]`, the `how` of a `hit` event.
fn how(e: &Event) -> &str {
    match e.fields.get(5) {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

/// ACCORDION's part.
#[derive(Debug)]
pub struct AccordionSound {
    k: usize,
    /// The Composed arrangement's reeds, reworked toward the real accordion (sound upgrade, stage 3, S1).
    real: RealNotes,
}

impl AccordionSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> AccordionSound {
        AccordionSound {
            k,
            real: RealNotes::default(),
        }
    }

    /// [`reeds`] as the arrangement plays them: its own under Classic; under Composed (sound upgrade, stage 3, S1)
    /// each reed reworked toward the real accordion ([`REWORK`]), its repeats taking turns.
    #[allow(clippy::too_many_arguments)]
    fn chord(&self, ctx: &MusicCtx, ms: &[f64], dur: f64, vel: f64, swell: bool, wheeze: f64, seed: u64) -> Vec<f64> {
        if !ctx.composed_on() {
            return reeds(ms, dur, vel, swell, wheeze, seed);
        }
        reeds_of(ms, dur, swell, wheeze, seed, &|m, s| {
            let key = [round_nd(m, 3), round_nd(dur, 3), round_nd(vel, 2)].map(|v| (v + 0.0).to_bits());
            let key = [key[0], key[1], key[2], s];
            self.real.get(&key, |v| {
                real::rework(&reed(pp::midi_hz(m), dur, vel, s), m, dur, SRF, &REWORK, v)
            })
        })
    }

    /// The chord at `t` as four reeds from G3 up (`VOICING`), and its bass button (`OOM`), from the theme's harmony.
    fn voicing(&self, ctx: &MusicCtx, t: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let ch = ctx.harmony.chord_at(t).to_string();
        let bad = |e: bf_core::harmony::HarmonyError| MusicError::new(H, e.to_string());
        let tones = ctx.harmony.chord_tones(55, 76, &ch).map_err(bad)?;
        let vo: Vec<f64> = tones.into_iter().take(4).map(f64::from).collect();
        if vo.is_empty() {
            return Err(MusicError::new(
                H,
                format!("the chord {ch:?} has no tone from G3 to E5"),
            ));
        }
        let oom = f64::from(ctx.harmony.root_above(&ch, 40).map_err(bad)?);
        Ok((vo, oom))
    }

    /// `acc_chord(t, hp, lead)`: the chord thinned as its HP drops, with a top note added when it leads.
    fn acc_chord(&self, ctx: &MusicCtx, t: f64, hp: f64, lead: f64) -> Result<Vec<f64>, MusicError> {
        let (mut vo, _) = self.voicing(ctx, t)?;
        vo.truncate(reeds_left(hp));
        if lead > 0.04 {
            vo.push(vo[0] + 12.0);
        }
        Ok(vo)
    }

    /// The note of the written line a bite or a snap at `t` sounds, under the Composed arrangement only (sound
    /// upgrade: every pitched fighter plays its line). `None` under Classic and on a rest of the line: the kit's own
    /// chord and bass button then.
    fn line(&self, ctx: &MusicCtx, t: f64) -> Result<Option<i32>, MusicError> {
        if ctx.composed_on() {
            ctx.play_line(self.name(ctx), t)
        } else {
            Ok(None)
        }
    }

    /// The Composed chord (sound upgrade): the harmony's chord at `t` voiced under the line's note `top`, so the
    /// line is the top reed of the hit. `keep` reeds in all: the chord's tones nearest under the note (none within a
    /// tone of it), then the note itself; the leader gets one reed more under it (where the kit adds a top note,
    /// which would cover the line).
    fn line_chord(&self, ctx: &MusicCtx, t: f64, top: i32, keep: usize, lead: bool) -> Result<Vec<f64>, MusicError> {
        let ch = ctx.harmony.chord_at(t).to_string();
        let bad = |e: bf_core::harmony::HarmonyError| MusicError::new(H, e.to_string());
        let under = ctx.harmony.chord_tones(top - 17, top - 3, &ch).map_err(bad)?;
        let n = (keep.max(1) - 1 + usize::from(lead)).min(under.len());
        let mut vo: Vec<f64> = under[under.len() - n..].iter().copied().map(f64::from).collect();
        vo.push(f64::from(top));
        Ok(vo)
    }

    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }
}

impl Sound for AccordionSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.0
    }

    fn send(&self) -> f64 {
        0.4
    }

    /// A full chord blast, every reed, the bellows wheezing.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let (vo, oom) = self.voicing(ctx, t)?;
        let mut ms = vec![oom, oom + 12.0];
        ms.extend(vo.iter().copied());
        ms.push(vo[0] + 12.0);
        mix.place(
            &bus,
            &Sig::Mono(self.chord(ctx, &ms, 0.62, 1.0, false, 0.8, 77)),
            t,
            0.44,
            0.15,
        );
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        for (j, &(tc, _)) in ctx.show.grid.count[..3].iter().enumerate() {
            let (vo, _) = self.voicing(ctx, tc)?;
            mix.place(
                &bus,
                &Sig::Mono(self.chord(ctx, &vo, 0.22, 0.9, false, 0.4, j as u64)),
                tc,
                0.38,
                0.25,
            );
        }
        Ok(())
    }

    /// Bites are oom (the opening) + pah (the snap); SQUEEZEBOX squeals; FULL BELLOWS inhales, then blasts.
    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (k, bus) = (self.k, self.bus(ctx));
        let t_ko = ctx.show.t_ko;
        let mut main_open = true;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "bite" => {
                    let t0 = ctx.q16(t);
                    let hp = ctx.hp_at(k, t0) / 100.0;
                    main_open = e.f(3) != 0.0;
                    let (_, oom) = self.voicing(ctx, t0)?;
                    // Composed (sound upgrade): the bass button is the line's note in the buttons' own octave (E2
                    // to D#3, where the kit puts the chord's root), and the little inhale has the line on top
                    let line = self.line(ctx, t0)?;
                    let oom = line.map_or(oom, |m| f64::from(40 + (m - 40).rem_euclid(12)));
                    let x = self
                        .tone(
                            ctx,
                            &self.chord(ctx, &[oom - 12.0, oom], 0.20, 0.85, false, 0.0, 3),
                            t0,
                            1.0,
                        )?
                        .0;
                    mix.place(&bus, &Sig::Mono(x), t0, if main_open { 0.24 } else { 0.16 }, 0.1);
                    let vo = match line {
                        Some(m) => self.line_chord(ctx, t0, m, 2, false)?,
                        None => {
                            let mut vo = self.acc_chord(ctx, t0, hp, 0.0)?;
                            vo.truncate(2);
                            vo
                        }
                    };
                    mix.place(
                        &bus,
                        &Sig::Mono(self.chord(ctx, &vo, 0.12, 0.6, true, 0.5, 5)),
                        t0,
                        0.06,
                        0.2,
                    );
                }
                "snap" => {
                    let tq = ctx.q16(t);
                    let hp = ctx.hp_at(k, tq) / 100.0;
                    let lead = (ctx.hp_at(k, tq) - ctx.hp_at(1 - k, tq)) / 100.0;
                    let wz = 0.3 + 0.9 * dmath::pow(1.0 - hp, 1.3);
                    let landed = e.f(4) != 0.0;
                    // Composed (sound upgrade): the stab keeps the harmony's chord, voiced under the line's note
                    let vo = match self.line(ctx, tq)? {
                        Some(m) => self.line_chord(ctx, tq, m, reeds_left(hp), lead > 0.04)?,
                        None => self.acc_chord(ctx, tq, hp, lead)?,
                    };
                    let seed = noise_seed(H, tq * 10.0)?;
                    let x = self.chord(ctx, &vo, 0.16, if landed { 0.95 } else { 0.8 }, false, wz, seed);
                    let (sig, gg, _) = self.tone(ctx, &x, tq, 0.8)?;
                    let g = if landed { 0.36 } else { 0.24 } * if main_open { 1.0 } else { 0.8 };
                    mix.place(&bus, &Sig::Mono(sig), tq, g * gg, 0.2);
                    if landed {
                        // the chomp
                        let th = synth::thud(0.22, 220.0, 70.0, noise_seed(H, tq * 13.0)?);
                        mix.place("fx", &Sig::Mono(th), tq, 0.16, 0.0);
                        let nc = n_of(0.05);
                        let tc = synth::tt(nc);
                        let ch: Vec<f64> =
                            synth::bp(&synth::noise(noise_seed(H, tq * 17.0)?, nc), 900.0, 4000.0, FILTER_O)
                                .into_iter()
                                .zip(&tc)
                                .map(|(v, &t)| v * dmath::exp(-t / 0.012))
                                .collect();
                        mix.place("fx", &Sig::Mono(ch), tq, 0.10, 0.0);
                    }
                }
                "ability" if e.s(4) == "shield" => {
                    // SQUEEZEBOX folds shut with a squeal
                    let tq = ctx.q16(t);
                    let (vo, _) = self.voicing(ctx, tq)?;
                    let top = vo[vo.len() - 1] + 12.0;
                    let sq = self.chord(ctx, &[top - 7.0, top - 6.0, top - 5.0, top], 0.26, 0.9, false, 0.9, 41);
                    mix.place(&bus, &Sig::Mono(sq), tq, 0.22, 0.2);
                    mix.place(
                        "fx",
                        &Sig::Mono(synth::thud(0.25, 180.0, 80.0, 7)),
                        tq + 0.26,
                        0.20,
                        0.0,
                    );
                }
                "clamp" => {
                    // FULL BELLOWS: a beat-long inhale ...
                    let d = e.f(4);
                    let tq = ctx.q16(t);
                    let (vo, oom) = self.voicing(ctx, tq)?;
                    let mut ms = vec![oom];
                    ms.extend(vo.iter().take(3).copied());
                    mix.place(
                        &bus,
                        &Sig::Mono(self.chord(ctx, &ms, d, 0.9, true, 1.2, 43)),
                        tq,
                        0.30,
                        0.1,
                    );
                    let w = synth::whoosh(d, 3000.0, 300.0, false, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(w), tq, 0.12, 0.0);
                }
                "clamp_snap" => {
                    // ... then every reed at once
                    let landed = e.f(3) != 0.0;
                    let (vo, oom) = self.voicing(ctx, t)?;
                    let mut ms = vec![oom - 12.0, oom, oom + 7.0];
                    ms.extend(vo.iter().copied());
                    ms.extend(vo.iter().take(2).map(|m| m + 12.0));
                    let x = self.chord(ctx, &ms, 0.55, 1.0, false, 1.4, 45);
                    mix.place(&bus, &Sig::Mono(x), t, if landed { 0.42 } else { 0.30 }, 0.0);
                    let th = synth::thud(0.7, 90.0, 30.0, 51);
                    mix.place("fx", &Sig::Mono(th), t, if landed { 0.55 } else { 0.32 }, 0.0);
                }
                "hit" if how(e) == "clamp" => {}
                _ => {}
            }
        }
        Ok(())
    }

    /// The bellows tear.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        mix.place(&bus, &Sig::Mono(tear(1.4, 23)), ctx.show.t_hs, 0.36, 0.0);
        Ok(())
    }

    /// On "(WINNER) WINS": a big open chord.
    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let (vo, oom) = self.voicing(ctx, tw)?;
        let mut ms = vec![oom + 12.0];
        ms.extend(vo.iter().copied());
        ms.push(vo[0] + 12.0);
        mix.place(
            "solo",
            &Sig::Mono(self.chord(ctx, &ms, 1.0, 1.0, false, 0.4, 51)),
            tw,
            0.36,
            0.0,
        );
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts >= grid.t_reassemble {
                break;
            }
            let dur = d * grid.beat * 0.95;
            let m = f64::from(m);
            let seed = noise_seed(H, bt * 4.0)?;
            mix.place(
                "solo",
                &Sig::Mono(self.chord(ctx, &[m, m - 12.0], dur, 0.95, false, 0.2, seed)),
                ts,
                0.44,
                0.0,
            );
        }
        Ok(())
    }

    /// Its pieces fly home as a rising reed run.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (vo, _) = self.voicing(ctx, grid.t_reassemble)?;
        let run: Vec<f64> = vo.iter().copied().chain(vo.iter().map(|m| m + 12.0)).collect();
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            mix.place(
                &bus,
                &Sig::Mono(self.chord(ctx, &[m], grid.s16 * 1.6, 0.6, false, 0.0, j as u64)),
                tj,
                0.20,
                0.0,
            );
        }
        Ok(())
    }
}
