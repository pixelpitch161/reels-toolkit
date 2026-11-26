//! SYNTH's part in the score: the `Sound` class of its reference kit module (`tools/studio_export/older/fighters/
//! synth.py`, #31 v3's `audio.py`, the SYNTH's part only).
//!
//! Every voltage pulse is an arpeggio note in the synth's own Pixel Pitch voice (the first cable plays the arp, the
//! others the notes a third above, quieter), toned like the waveform on its cables (sine, square, saw by level),
//! skipping notes when hurt; a landed plug clicks in with an accent, a SAWTOOTH cut an accent and a thud, WHITE
//! NOISE's hit a thud; the cable unspooling, a snapped cable's crack, the leash's thrum; LFO's wobble bass, SAWTOOTH's
//! supersaw sweep, WHITE NOISE's blast and high note; the title blast, the count-in, the fall-apart, the stinger, the
//! solo, the reassembly run.
//!
//! Under the Composed arrangement (sound upgrade, 2026-11-25) the pulses play the written line instead of the arp:
//! the first cable the line's note of its 16th, the others the chord's tones under it, and the accents of a landed
//! plug or cut the line's note at the hit; notes above C6 are softened. The moves and the endings keep their notes.
//!
//! Its notes are #31's own (`note` with `_fit`'s 0.03 s fade, its own cache and gain table: `MusicCtx::own_note`,
//! the module's `note_helper`). #31's arp table and fixed notes are the module's own rules (its docstring).

use std::sync::Arc;

use bf_core::dmath;
use bf_core::py::{fmod_py, round_half_even};
use bf_core::recording::Array;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, OwnNote};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "SYNTH";
/// #31's `_fit` fade, in seconds.
pub const FADE: f64 = 0.03;
/// An arp note's cap: cut before its built-in echo.
const ARP_CAP: f64 = 0.34;

/// #31 timeline.py `ARP`, for its four chords.
const ARP_31: [(&str, [i32; 8]); 4] = [
    ("Fm", [65, 72, 68, 77, 75, 80, 79, 84]),
    ("Db", [65, 68, 73, 72, 77, 80, 75, 85]),
    ("Bbm", [65, 70, 73, 77, 72, 82, 80, 85]),
    ("C", [64, 67, 72, 70, 76, 79, 74, 82]),
];
const TITLE_31: [i32; 4] = [65, 72, 77, 80];
const COUNT_31: [i32; 3] = [60, 64, 67];
const FALL_31: [i32; 8] = [84, 79, 75, 72, 68, 63, 60, 56];
const RUN_31: [i32; 8] = [65, 68, 72, 75, 77, 80, 84, 89];

/// `h01_31(*xs)`: #31's deterministic hash in [0, 1).
pub fn h01_31(xs: &[f64]) -> f64 {
    let mut v: i64 = 0;
    for &x in xs {
        // int(x * 1000): toward zero
        v = (v * 1_000_003 + (x * 1000.0) as i64 + 7).rem_euclid(2_147_483_647);
    }
    ((v * 48271).rem_euclid(2_147_483_647)) as f64 / 2_147_483_647.0
}

fn bad<E: std::fmt::Display>(e: E) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `np.cumsum`.
fn cumsum(x: &[f64]) -> Vec<f64> {
    let mut s = 0.0;
    x.iter()
        .map(|v| {
            s += v;
            s
        })
        .collect()
}

/// `np.sign`.
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// `jack_click(seed)`: a patch cable plugging in, a metallic click and a tiny hum.
pub fn jack_click(seed: u64) -> Vec<f64> {
    let n = (0.08 * SRF) as usize;
    let t = synth::tt(n);
    let b = synth::bp(&synth::noise(seed, n), 2000.0, 9000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let x = b[i] * dmath::exp(-t[i] / 0.005);
            x + 0.4 * dmath::sin(2.0 * dmath::PI * 120.0 * t[i]) * dmath::exp(-t[i] / 0.03)
        })
        .collect()
}

/// `zip_out(dur, seed)`: the cable unspooling behind a plug, a short rising zip.
pub fn zip_out(dur: f64, seed: u64) -> Vec<f64> {
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    let f: Vec<f64> = t.iter().map(|&t| 800.0 + 3000.0 * t / dur).collect();
    let c = cumsum(&f);
    let b = synth::bp(&synth::noise(seed, n), 1500.0, 6000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let x = sign(dmath::sin(2.0 * dmath::PI * c[i] / SRF)) * 0.3 + b[i];
            x * dmath::sin(dmath::PI * t[i] / dur) * 0.6
        })
        .collect()
}

/// `cable_snap(seed)`: a snapped patch cable, a crack and a falling electric zap.
pub fn cable_snap(seed: u64) -> Vec<f64> {
    let n = (0.4 * SRF) as usize;
    let t = synth::tt(n);
    let b = synth::bp(&synth::noise(seed, n), 1500.0, 10000.0, FILTER_O);
    let f: Vec<f64> = t.iter().map(|&t| 1800.0 * dmath::exp(-t / 0.08) + 90.0).collect();
    let c = cumsum(&f);
    let zap: Vec<f64> = (0..n)
        .map(|i| sign(dmath::sin(2.0 * dmath::PI * c[i] / SRF)) * dmath::exp(-t[i] / 0.12) * 0.5)
        .collect();
    let z = synth::lp(&zap, 5000.0, FILTER_O);
    (0..n).map(|i| b[i] * dmath::exp(-t[i] / 0.02) + z[i]).collect()
}

/// `supersaw_sweep(dur, m0, m1)`.
pub fn supersaw_sweep(dur: f64, m0: f64, m1: f64) -> Vec<f64> {
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    let (f0, f1) = (synth::mtof(m0), synth::mtof(m1));
    let f: Vec<f64> = t.iter().map(|&t| f0 * dmath::pow(f1 / f0, t / dur)).collect();
    let ph: Vec<f64> = cumsum(&f).iter().map(|c| c / SRF).collect();
    let x: Vec<f64> = ph
        .iter()
        .map(|&p| {
            let mut s = 0.0;
            for d in [-0.01, -0.004, 0.0, 0.004, 0.01] {
                s += 2.0 * fmod_py(p * (1.0 + d) + d * 3.0, 1.0) - 1.0;
            }
            s / 5.0
        })
        .collect();
    let y = synth::hp(&x, 300.0, FILTER_O);
    (0..n)
        .map(|i| {
            let tail = if t[i] - dur * 0.6 > 0.0 { t[i] - dur * 0.6 } else { 0.0 };
            y[i] * clip(t[i] / 0.02, 0.0, 1.0) * dmath::exp(-tail / (dur * 0.2))
        })
        .collect()
}

/// `noise_blast(dur)`.
pub fn noise_blast(dur: f64) -> Vec<f64> {
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    let x = synth::noise(21, n);
    let a = synth::hp(&x, 1200.0, FILTER_O);
    let b = synth::bp(&x, 300.0, 3000.0, FILTER_O);
    (0..n)
        .map(|i| {
            let y = a[i] * 0.7 + b[i] * 0.5;
            y * clip(t[i] / 0.01, 0.0, 1.0)
                * dmath::exp(-t[i] / (dur * 0.35))
                * (1.0 + 0.5 * sign(dmath::sin(2.0 * dmath::PI * 9.0 * t[i])))
        })
        .collect()
}

/// `wobble(m, dur, rate_beats=0.5)`: LFO's wobble bass, the filter swinging once per half beat.
pub fn wobble(m: f64, dur: f64, beat: f64) -> Vec<f64> {
    let rate_beats = 0.5;
    let n = (dur * SRF) as usize;
    let t = synth::tt(n);
    let x = synth::saw(synth::mtof(m), n, 0.01, 2);
    let lfo: Vec<f64> = t
        .iter()
        .map(|&t| 0.5 - 0.5 * dmath::cos(2.0 * dmath::PI * t / (rate_beats * beat)))
        .collect();
    let mut out = vec![0.0; n];
    let nb = (dur / 0.02) as usize;
    for k in 0..nb {
        let (a, b) = (k * n / nb, (k + 1) * n / nb);
        let y = synth::lp(&x[a.saturating_sub(512)..b], 150.0 + 1800.0 * lfo[(a + b) / 2], 2);
        out[a..b].copy_from_slice(&y[y.len() - (b - a)..]);
    }
    let env = synth::adsr(n, 0.01, 0.1, 0.9, 0.05);
    (0..n).map(|i| dmath::np_tanh(2.0 * out[i]) * env[i]).collect()
}

/// SYNTH's part on side `k`.
#[derive(Clone, Debug)]
pub struct SynthSound {
    pub k: usize,
}

impl SynthSound {
    fn own(&self) -> OwnNote {
        OwnNote {
            side: self.k,
            fade: FADE,
        }
    }

    /// `arp_note(m, vel, dur)`: `note(pp, m, dur, 0.34, vel)` with #31's own `_fit`.
    pub fn arp_note(&self, ctx: &MusicCtx, m: f64, vel: f64, dur: f64) -> Result<Arc<[f64]>, MusicError> {
        ctx.own_note(self.own(), &ctx.cfg[self.k].pp, m, dur, ARP_CAP, vel)
    }

    /// `arp_lv(m, lvl, vel)`: an arp note toned like its cables' waveform (sine LV1, square LV2, saw LV3).
    pub fn arp_lv(&self, ctx: &MusicCtx, m: f64, lvl: i64, vel: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.arp_note(ctx, m, vel, 0.12)?;
        let n = x.len();
        let f = synth::mtof(m);
        let peak = x.iter().fold(0.0f64, |a, v| if v.abs() > a { v.abs() } else { a });
        let env: Vec<f64> = synth::expenv(n, 0.12, 0.003).iter().map(|e| peak * e).collect();
        Ok(if lvl <= 1 {
            let l = synth::lp(&x, 1100.0, FILTER_O);
            let t = synth::tt(n);
            (0..n)
                .map(|i| l[i] * 0.8 + dmath::sin(2.0 * dmath::PI * f * t[i]) * env[i] * 0.5)
                .collect()
        } else if lvl == 2 {
            let s = synth::lp(&synth::square(f, n, 0.5), 3500.0, FILTER_O);
            (0..n).map(|i| x[i] * 0.85 + s[i] * env[i] * 0.35).collect()
        } else {
            let s = synth::lp(&synth::saw(f, n, 0.006, 2), 5000.0, FILTER_O);
            (0..n).map(|i| x[i] * 0.85 + s[i] * env[i] * 0.4).collect()
        })
    }

    /// The arp of chord `ch`: #31's table, else the chord's tones climbing from E4 (eight of them).
    pub fn arp(&self, ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        if let Some((_, a)) = ARP_31.iter().find(|(c, _)| *c == ch) {
            return Ok(a.to_vec());
        }
        let tones = ctx.harmony.chord_tones(64, 100, ch).map_err(bad)?;
        Ok(tones.into_iter().take(8).collect())
    }

    /// The written line's note for a pulse or an accent at `tq` (sound upgrade: under the Composed arrangement the
    /// synth plays its line). `None` under Classic and on a rest of the line: the arp's own note stands then.
    fn written(&self, ctx: &MusicCtx, tq: f64) -> Result<Option<i32>, MusicError> {
        if ctx.composed_on() {
            ctx.play_line(self.name(ctx), tq)
        } else {
            Ok(None)
        }
    }

    /// The semitones from F to the tonic chord's root, in -6..5.
    pub fn shift(&self, ctx: &MusicCtx) -> Result<i32, MusicError> {
        let ch = ctx
            .harmony
            .chords
            .first()
            .ok_or_else(|| MusicError::new(H, "no bars"))?;
        let r = bf_core::harmony::parse_chord(ch).map_err(bad)?.root;
        Ok((r - 5 + 6).rem_euclid(12) - 6)
    }

    /// `int(RUN['LV'][sim_i(t), k])`.
    fn level(&self, ctx: &MusicCtx, t: f64) -> Result<i64, MusicError> {
        let at = ctx.sim_i(t) * 2 + self.k;
        match ctx.rec.arrays.get("LV") {
            Some(Array::I64 { data, .. }) => data.get(at).copied(),
            Some(Array::F64 { data, .. }) => data.get(at).map(|v| *v as i64),
            _ => None,
        }
        .ok_or_else(|| MusicError::new(H, "the recording has no LV"))
    }
}

/// `float(np.clip(x / 3.5, -0.6, 0.6))`.
fn pan_of(x: f64) -> f64 {
    clip(x / 3.5, -0.6, 0.6)
}

impl Sound for SynthSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        2.0
    }

    fn send(&self) -> f64 {
        0.5
    }

    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.arp_note(ctx, m, vel, dur)
    }

    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let sh = self.shift(ctx)?;
        let bus = self.bus(ctx);
        for m in TITLE_31 {
            let x = self.arp_lv(ctx, f64::from(m + sh), 3, 1.0)?;
            mix.place(&bus, &Sig::Mono(x), t, 0.16, 0.2);
        }
        let x = supersaw_sweep(0.45, f64::from(65 + sh), f64::from(89 + sh));
        mix.place(&bus, &Sig::Mono(x), t, 0.14, 0.2);
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let sh = self.shift(ctx)?;
        let bus = self.bus(ctx);
        for (tc, _) in ctx.show.grid.count.iter().take(3) {
            for m in COUNT_31 {
                let x = self.arp_note(ctx, f64::from(m + sh), 0.9, 0.18)?;
                mix.place(&bus, &Sig::Mono(x.to_vec()), *tc, 0.20, 0.2);
            }
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (name, bus) = (self.name(ctx), self.bus(ctx));
        let s16 = ctx.show.grid.s16;
        let t_ko = ctx.show.t_ko;
        let args = self.colour_args();
        let cfg = ctx.cfg[k];
        let names = [
            cfg.unlock1.name.as_str(),
            cfg.unlock2.name.as_str(),
            cfg.low.name.as_str(),
        ];
        // the pulses, by 16th (in slot order)
        let mut by_tick: std::collections::BTreeMap<i64, Vec<&bf_core::event::Event>> = Default::default();
        for e in &ctx.rec.events {
            if e.kind == "volt" && e.who() == Some(k) && e.t <= t_ko + 0.01 {
                let slot = round_half_even(ctx.q16(e.t) / s16) as i64;
                by_tick.entry(slot).or_default().push(e);
            }
        }
        for (slot, evs) in &by_tick {
            let tq = *slot as f64 * s16;
            let hp = ctx.hp_at(k, tq) / 100.0;
            if h01_31(&[tq, 3.0]) < 0.55 * dmath::pow(1.0 - hp, 1.6) {
                continue; // skips notes when hurt
            }
            let n = evs[0].i(4);
            let arp = self.arp(ctx, ctx.harmony.chord_at(tq))?;
            let lvl = self.level(ctx, tq)?;
            // Composed: the first cable plays the line's note of this 16th, the others the chord's tones under it
            // (the nearest first, at least a minor third under), so the line is the top and the loudest
            let line = self.written(ctx, tq)?;
            let under = match line {
                Some(l) => ctx
                    .harmony
                    .chord_tones(l - 24, l - 3, ctx.harmony.chord_at(tq))
                    .map_err(bad)?,
                None => Vec::new(),
            };
            for (j, _) in evs.iter().enumerate() {
                let m = match line {
                    Some(l) if j == 0 => l,
                    Some(l) => under.iter().rev().nth(j - 1).copied().unwrap_or(l - 12),
                    None => arp[(n + 2 * j as i64).rem_euclid(8) as usize],
                };
                let x = self.arp_lv(ctx, f64::from(m), lvl, if j == 0 { 0.8 } else { 0.6 })?;
                let (sig, g, _) = self.colour(ctx, &x, tq, 1.0, &args)?;
                let gain = (if j == 0 { 0.22 } else { 0.10 }) * g * ctx.soften(name, m);
                mix.place(&bus, &Sig::Mono(sig), tq, gain, 0.2 + 0.15 * dmath::sin(j as f64 * 1.7));
            }
        }
        for e in &ctx.rec.events {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "hit" => {
                    let tq = ctx.q16(t);
                    let pan = pan_of(e.f(5));
                    let arp = self.arp(ctx, ctx.harmony.chord_at(tq))?;
                    let lvl = self.level(ctx, tq)?;
                    match e.s(7) {
                        "plug" => {
                            mix.place("fx", &Sig::Mono(jack_click((t * 31.0) as u64)), t, 0.16, pan);
                            // Composed: the accent is the line's note at the hit (the line already sits where
                            // the kit's accents do, an octave over the arp); the kit's: the arp's first note, up
                            let m = match self.written(ctx, tq)? {
                                Some(m) => m,
                                None => arp[0] + 12,
                            };
                            let x = self.arp_lv(ctx, f64::from(m), lvl, 1.0)?;
                            let sig = self.colour(ctx, &x, t, 1.0, &args)?.0;
                            mix.place(&bus, &Sig::Mono(sig), tq, 0.22 * ctx.soften(name, m), pan);
                        }
                        "lash" => {
                            let m = match self.written(ctx, tq)? {
                                Some(m) => m,
                                None => arp[e.i(8).rem_euclid(8) as usize] + 12,
                            };
                            let x = self.arp_lv(ctx, f64::from(m), 3, 1.0)?;
                            let sig = self.colour(ctx, &x, t, 1.0, &args)?.0;
                            mix.place(&bus, &Sig::Mono(sig), tq, 0.26 * ctx.soften(name, m), pan);
                            let th = synth::thud(0.2, 160.0, 60.0, (t * 7.0) as u64);
                            mix.place("fx", &Sig::Mono(th), tq, 0.10, 0.0);
                        }
                        "wave" => {
                            mix.place(
                                "fx",
                                &Sig::Mono(synth::thud(0.45, 110.0, 40.0, THUD_SEED)),
                                tq,
                                0.36,
                                0.0,
                            );
                        }
                        _ => {}
                    }
                }
                "plug_fire" => {
                    mix.place("fx", &Sig::Mono(zip_out(0.16, (t * 17.0) as u64)), t, 0.05, 0.2);
                }
                "snap" => {
                    mix.place("fx", &Sig::Mono(cable_snap((t * 23.0) as u64)), t, 0.30, pan_of(e.f(4)));
                }
                "leash" => {
                    let th = synth::thud(0.14, 75.0, 45.0, (t * 9.0) as u64);
                    mix.place("fx", &Sig::Mono(th), t, 0.035, pan_of(e.f(3)));
                }
                "ability" => {
                    let name = e.s(3);
                    let tq = ctx.q16(t);
                    if name == names[0] {
                        let dur = cfg
                            .unlock1
                            .get("dur")
                            .ok_or_else(|| MusicError::new(H, "LFO has no dur"))?;
                        let root = ctx.harmony.root_above(ctx.harmony.chord_at(tq), 24).map_err(bad)?;
                        let x = wobble(f64::from(root + 12), dur, ctx.show.grid.beat);
                        mix.place("bass", &Sig::Mono(x), tq, 0.20, 0.0);
                    } else if name == names[1] {
                        mix.place(
                            &bus,
                            &Sig::Mono(supersaw_sweep(ctx.show.grid.beat, 53.0, 89.0)),
                            tq,
                            0.16,
                            0.0,
                        );
                    } else if name == names[2] {
                        mix.place("fx", &Sig::Mono(noise_blast(1.6)), tq, 0.20, 0.0);
                        let x = self.arp_note(ctx, 89.0, 1.0, 0.3)?;
                        let sig = self.colour(ctx, &x, t, 2.0, &args)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.16, 0.0);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let sh = self.shift(ctx)?;
        let bus = self.bus(ctx);
        for (j, m) in FALL_31.iter().enumerate() {
            let x: Vec<f64> = self
                .arp_note(ctx, f64::from(m + sh), 0.9, 0.1)?
                .iter()
                .map(|v| dmath::np_tanh(3.0 * v))
                .collect();
            let jf = j as f64;
            mix.place(
                &bus,
                &Sig::Mono(x),
                ctx.show.t_hs + 0.07 * jf,
                0.20 * (1.0 - jf / 10.0),
                (jf - 3.5) / 8.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let sh = self.shift(ctx)?;
        for m in TITLE_31 {
            let x = self.arp_note(ctx, f64::from(m + sh), 1.0, 0.4)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tw, 0.20, 0.0);
        }
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let beat = ctx.show.grid.beat;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let dur = d * beat * 0.9;
            let x = self.arp_note(ctx, f64::from(m), 0.95, if dur > 0.1 { dur } else { 0.1 })?;
            mix.place("solo", &Sig::Mono(x.to_vec()), t0 + bt * beat, 0.34, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let sh = self.shift(ctx)?;
        let bus = self.bus(ctx);
        let g = &ctx.show.grid;
        for (j, m) in RUN_31.iter().enumerate() {
            let tj = ctx.q16(g.t_reassemble + (g.t_glide - g.t_reassemble) * j as f64 / RUN_31.len() as f64);
            let x = self.arp_note(ctx, f64::from(m + sh), 0.7, 0.12)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), tj, 0.18, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h01_31_is_31s_linear_hash() {
        // python: h01(1.25, 3) with #31's audio.py
        // v = 0, then (v * 1000003 + int(x * 1000) + 7) % 2147483647 for x = 1.25 and 3
        let v0: i64 = 1250 + 7;
        let v = (v0 * 1_000_003 + 3000 + 7) % 2_147_483_647;
        assert_eq!(
            h01_31(&[1.25, 3.0]),
            ((v * 48271) % 2_147_483_647) as f64 / 2_147_483_647.0
        );
        assert!((0.0..1.0).contains(&h01_31(&[52.1, 3.0])));
    }

    #[test]
    fn the_sound_effects_have_their_lengths_and_no_nan() {
        for (x, n) in [
            (jack_click(5), 3840),
            (zip_out(0.16, 3), 7680),
            (cable_snap(2), 19200),
            (supersaw_sweep(0.45, 65.0, 89.0), 21600),
            (noise_blast(1.6), 76800),
            (wobble(41.0, 3.0, 0.4357), 144000),
        ] {
            assert_eq!(x.len(), n);
            assert!(x.iter().all(|v| v.is_finite()));
        }
    }
}
