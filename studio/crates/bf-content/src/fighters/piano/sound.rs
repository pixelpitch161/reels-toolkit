//! PIANO's part in the score: the `Sound` half of `kit/fighters/piano.py` (#45).
//!
//! Every fired row of the roll IS the piano's line: its pitches, as the holes (the melody note plus the harmony a
//! denser row adds; a hurt piano drops its harmony first); SUSTAIN's rows ring on with the pedal down until they fly;
//! a landed key adds a bright accent; GLISSANDO runs up the keyboard with the arc; 88 KEYS is a whole-keyboard smash
//! and a glissando down; the solo is the fight file's solo with left-hand chords on the bar lines.
//!
//! The piano's voice draws no noise, but its notes are still asked for in the kit's order: the first of them
//! measures the voice's level.

use bf_core::dmath;
use bf_core::event::{Event, Value};
use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    MusicCtx, MusicError, Samples, Thinner, bend, clank, event_at, event_int, event_is, event_is_str, noise_seed,
};
use bf_sound::synth::{self, THUD_SEED};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "PIANO";

/// `keys`' default velocity.
pub const KEYS_VEL: f64 = 0.85;

/// PIANO's part.
#[derive(Clone, Copy, Debug)]
pub struct PianoSound {
    k: usize,
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// Python's `round(x)` (or `int(round(x))`) for a count of bars or 16ths.
fn slot(x: f64) -> Result<i64, MusicError> {
    let r = round_half_even(x);
    if r.abs() < 9.0e15 {
        Ok(r as i64)
    } else {
        Err(MusicError::new(H, format!("{x} is not a place on the grid")))
    }
}

/// `keys(ms, dur, vel, cap)`: a chord (or one note) in the Pixel Pitch piano voice: each note is
/// `M.note('PIANO', int(m), round(dur, 3), cap, round(vel * (1.0 if j == 0 else 0.8), 1))`, the first at full
/// velocity; their sum over `max(1.0, len(ms) ** 0.6)`. `cap` of `None` (or 0, as in Python) is `round(dur + 0.35,
/// 2)`.
///
/// # Errors
/// As [`MusicCtx::note`]; and an empty chord, which Python cannot place (its sum is the number 0).
pub fn keys(ctx: &MusicCtx, ms: &[i32], dur: f64, vel: f64, cap: Option<f64>) -> Samples {
    let cap = match cap {
        Some(c) if c != 0.0 => c,
        _ => round_nd(dur + 0.35, 2),
    };
    if ms.is_empty() {
        return Err(MusicError::new(H, "a chord with no notes cannot be played"));
    }
    let mut x: Vec<f64> = Vec::new();
    for (j, &m) in ms.iter().enumerate() {
        let v = round_nd(vel * if j == 0 { 1.0 } else { 0.8 }, 1);
        let note = ctx.note("PIANO", f64::from(m), round_nd(dur, 3), cap, v)?;
        if j == 0 {
            x = note.to_vec();
        } else {
            if note.len() != x.len() {
                return Err(MusicError::new(H, "the notes of a chord have different lengths"));
            }
            for (s, n) in x.iter_mut().zip(note.iter()) {
                *s += n;
            }
        }
    }
    // `len(ms) ** 0.6` is CPython's `pow`, which `dmath::pow` is to the bit (ruling M3-R38)
    let div = py_max(1.0, dmath::pow(ms.len() as f64, 0.6));
    Ok(x.into_iter().map(|s| s / div).collect())
}

/// `tuple(e[6])` of a `roll` event: the pitches the row fired.
fn pitches(e: &Event) -> Result<Vec<i32>, MusicError> {
    let wrong = || {
        MusicError::new(
            H,
            format!("the roll event at {} s has no list of pitches as element 6", e.t),
        )
    };
    let Value::List(list) = event_at(H, e, 6)? else {
        return Err(wrong());
    };
    list.iter()
        .map(|v| match v {
            Value::Int(m) => i32::try_from(*m).map_err(|_| wrong()),
            _ => Err(wrong()),
        })
        .collect()
}

impl PianoSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> PianoSound {
        PianoSound { k }
    }

    /// `self.colour(x, t)`.
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, 1.0, &self.colour_args())
    }
}

impl Sound for PianoSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.7
    }

    fn send(&self) -> f64 {
        0.3
    }

    /// The piano's line (C4 to C6) and its attack.
    fn band(&self) -> (f64, f64) {
        (260.0, 4200.0)
    }

    /// `self.bus = 'piano'`.
    fn bus(&self, _ctx: &MusicCtx) -> String {
        "piano".to_string()
    }

    /// A hammered low chord: the key's root in octaves and the fifth, struck hard.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let r = ctx.harmony.key_root;
        let default = [r, r + 7, r + 12, r + 19, r + 24];
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .map_or(&default[..], Vec::as_slice);
        let Some(&first) = hits.first() else {
            return Err(MusicError::new(H, "the title hits are an empty list"));
        };
        mix.place(&bus, &Sig::Mono(keys(ctx, hits, 0.9, 1.0, Some(1.3))?), t, 0.55, 0.0);
        let low = keys(ctx, &[first - 12], 0.9, 1.0, Some(1.3))?;
        mix.place(&bus, &Sig::Mono(low), t, 0.35, 0.0);
        Ok(())
    }

    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the episode has no bar 1 to count in on"))?;
        let mut ct = ctx.harmony.chord_tones(60, 76, ch).map_err(bad)?;
        ct.truncate(3);
        for (tc, _) in &grid.count[..3] {
            mix.place(&bus, &Sig::Mono(keys(ctx, &ct, 0.18, 0.9, None)?), *tc, 0.36, 0.0);
        }
        // a pickup run up into FIGHT!
        let scale = ctx.harmony.scale_notes(64, 84, Some(ch)).map_err(bad)?;
        let run = &scale[scale.len().saturating_sub(8)..];
        for (j, &m) in run.iter().enumerate() {
            let t = grid.bar + 3.0 * grid.beat + j as f64 * grid.beat / 8.0;
            mix.place(&bus, &Sig::Mono(keys(ctx, &[m], 0.08, 0.75, None)?), t, 0.24, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let cfg = ctx.cfg[k];
        let mut thin = Thinner::new(45.0);
        let ab1 = &cfg.unlock1;
        // hang = ab['unlock1'].get('out16', 2) * T.BEAT / 4 + ab['unlock1'].get('hang_beats', 1) * T.BEAT
        let hang = ab1.get("out16").unwrap_or(2.0) * beat / 4.0 + ab1.get("hang_beats").unwrap_or(1.0) * beat;
        let mut last_acc: Option<i64> = None;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind == "roll" && event_is(H, e, 2, k)? {
                // the row = the line (+ its harmony): what you see
                let tq = ctx.q16(t);
                let slot = slot(tq / s16)?;
                let mut ps = pitches(e)?;
                let hp = ctx.hp_at(k, t) / 100.0;
                // a hurt piano drops its harmony first
                if ps.len() > 1 && !thin.keep(slot as f64, hp) {
                    ps.truncate(1);
                }
                let sus = event_at(H, e, 5)?.truthy();
                let dur = if sus { hang } else { 0.24 };
                let cap = round_nd(dur + if sus { 0.6 } else { 0.3 }, 2);
                let (sig, g, lead) = self.tone(ctx, &keys(ctx, &ps, dur, 0.9, Some(cap))?, t)?;
                mix.place(&bus, &Sig::Mono(sig), tq, 0.40 * g, -0.2);
                if lead > 0.04 {
                    // the leader: the melody doubled an octave up
                    let up = self.tone(ctx, &keys(ctx, &[ps[0] + 12], 0.16, 0.6, None)?, t)?.0;
                    mix.place(&bus, &Sig::Mono(up), tq, 0.10 * g, -0.3);
                }
            } else if e.kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, "key")? {
                // a landed key: a bright accent (one per 16th)
                let tq = ctx.q16(t);
                let slot = slot(tq / s16)?;
                if last_acc == Some(slot) {
                    continue;
                }
                last_acc = Some(slot);
                let sub = event_int(H, e, 8)?;
                let m = if sub >= 30 {
                    i32::try_from(sub).map_err(|_| MusicError::new(H, format!("{sub} is not a pitch")))?
                } else {
                    // T.scale_notes(72, 96, T.chord_at(tq))[int(e[8]) % 7]
                    let scale = ctx
                        .harmony
                        .scale_notes(72, 96, Some(ctx.harmony.chord_at(tq)))
                        .map_err(bad)?;
                    *scale
                        .get(sub.rem_euclid(7) as usize)
                        .ok_or_else(|| MusicError::new(H, "the scale has fewer than 7 notes from 72 to 96"))?
                };
                let acc = self.tone(ctx, &keys(ctx, &[m + 12], 0.08, 0.7, None)?, t)?.0;
                mix.place(&bus, &Sig::Mono(acc), tq, 0.12, 0.1);
                let thump = synth::thud(0.12, 220.0, 120.0, noise_seed(H, t * 13.0)?);
                mix.place("fx", &Sig::Mono(thump), tq, 0.035, 0.0);
            } else if e.kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let is = |name: &str| event_is_str(H, e, 3, name);
                if is(&cfg.unlock1.name)? {
                    // SUSTAIN: the pedal goes down, the strings ring
                    let pedal = synth::thud(0.3, 90.0, 50.0, THUD_SEED);
                    mix.place("fx", &Sig::Mono(pedal), tq, 0.10, 0.0);
                    let ch = ctx.harmony.chord_at(tq);
                    let mut ct = ctx.harmony.chord_tones(48, 64, ch).map_err(bad)?;
                    ct.truncate(4);
                    let cap = round_nd(2.0 * beat + 0.8, 2);
                    let x = keys(ctx, &ct, 2.0 * beat, 0.5, Some(cap))?;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.14, 0.0);
                } else if is(&cfg.unlock2.name)? {
                    // GLISSANDO: up the white keys with the arc
                    let ab2 = &cfg.unlock2;
                    let need = |name: &str| {
                        ab2.get(name)
                            .ok_or_else(|| MusicError::new(H, format!("the ability {} has no {name}", ab2.name)))
                    };
                    let n = need("n")?;
                    let mut run = ctx
                        .harmony
                        .scale_notes(60, 100, Some(ctx.harmony.chord_at(tq)))
                        .map_err(bad)?;
                    // [:n]
                    run.truncate(if n > 0.0 { n as usize } else { 0 });
                    let span = need("beats")? * beat;
                    let steps = py_max(1.0, n - 1.0);
                    for (j, &m) in run.iter().enumerate() {
                        let j = j as f64;
                        let x = self.tone(ctx, &keys(ctx, &[m], 0.09, 0.8, None)?, t)?.0;
                        mix.place(&bus, &Sig::Mono(x), tq + span * j / steps, 0.22, -0.4 + 0.8 * j / steps);
                    }
                } else if is(&cfg.low.name)? {
                    // 88 KEYS: the whole keyboard at once, then down
                    let (lo, hi) = cfg.register;
                    // list(range(lo - 12, hi + 1, 3))
                    let cluster: Vec<i32> = (lo - 12..hi + 1).step_by(3).collect();
                    let smash: Vec<f64> = keys(ctx, &cluster, 1.2, 1.0, Some(1.8))?
                        .into_iter()
                        .map(|s| s * 1.6)
                        .collect();
                    mix.place(&bus, &Sig::Mono(smash), tq, 0.42, 0.0);
                    let low = keys(ctx, &[lo - 24, lo - 12], 1.4, 1.0, Some(2.0))?;
                    mix.place(&bus, &Sig::Mono(low), tq, 0.40, 0.0);
                    // range(hi + 12, lo - 13, -2)
                    let mut m = hi + 12;
                    let mut j = 0.0;
                    while m > lo - 13 {
                        let x = keys(ctx, &[m], 0.06, 0.7, None)?;
                        mix.place(&bus, &Sig::Mono(x), tq + 0.12 + j * 0.022, 0.16, 0.5 - 1.0 * j / 40.0);
                        m -= 2;
                        j += 1.0;
                    }
                    let boom = synth::thud(0.8, 100.0, 34.0, THUD_SEED);
                    mix.place("fx", &Sig::Mono(boom), tq, 0.4, 0.0);
                }
            }
        }
        Ok(())
    }

    /// The piano falls apart: a cluster slumping down, strings twanging loose, keys clattering.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let x = keys(ctx, &[40, 46, 51, 57], 1.0, 0.9, Some(1.4))?;
        mix.place(&self.bus(ctx), &Sig::Mono(bend(&x, -7.0, 0.05, 1.0)?), t0, 0.30, 0.0);
        for (j, m) in [84.0, 90.0, 79.0, 95.0, 72.0].into_iter().enumerate() {
            let twang = clank(m, 60 + j as u64, 0.25, &[(1.0, 0.05), (2.3, 0.03)])?;
            mix.place(
                "fx",
                &Sig::Mono(twang),
                t0 + 0.07 * j as f64,
                0.06,
                (j as f64 - 2.0) / 4.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = ctx
            .harmony
            .chords
            .first()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        // [T.root_above(ch, 36)] + T.chord_tones(55, 79, ch)[:5]
        let mut ms = vec![ctx.harmony.root_above(ch, 36).map_err(bad)?];
        ms.extend(ctx.harmony.chord_tones(55, 79, ch).map_err(bad)?.into_iter().take(5));
        mix.place("solo", &Sig::Mono(keys(ctx, &ms, 1.2, 1.0, Some(1.8))?), tw, 0.45, 0.0);
        Ok(())
    }

    /// The fight file's solo in the right hand, the chord of each bar in the left hand on the bar line.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let notes = ctx.solo(self.name(ctx))?;
        for (bt, m, d) in notes {
            let ts = t0 + bt * beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = keys(ctx, &[m], py_max(0.1, d * beat * 0.9), 0.95, None)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.46, 0.0);
        }
        // range(int(round(t0 / T.BAR)), int(T.T_REASSEMBLE / T.BAR + 0.5))
        let b0 = slot(t0 / bar)?;
        let b1 = slot((grid.t_reassemble / bar + 0.5).trunc())?;
        let chords = &ctx.harmony.chords;
        for b in b0..b1 {
            // T.CHORDS[b] (a negative bar counts from the end, as in a Python list)
            let at = if b < 0 { b + chords.len() as i64 } else { b };
            let ch = usize::try_from(at)
                .ok()
                .and_then(|at| chords.get(at))
                .ok_or_else(|| MusicError::new(H, format!("the episode has no bar {b} for the solo's left hand")))?;
            let root = ctx.harmony.root_above(ch, 36).map_err(bad)?;
            let mut lh = vec![root, root + 12];
            lh.extend(ctx.harmony.chord_tones(52, 64, ch).map_err(bad)?.into_iter().take(2));
            let x = keys(ctx, &lh, bar * 0.8, 0.8, Some(round_nd(bar + 0.3, 2)))?;
            mix.place("solo", &Sig::Mono(x), b as f64 * bar, 0.26, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        // T.scale_notes(52, 88, T.CHORDS[-2])[::2][:8]
        let chords = &ctx.harmony.chords;
        let ch = chords
            .len()
            .checked_sub(2)
            .and_then(|at| chords.get(at))
            .ok_or_else(|| MusicError::new(H, "the episode has fewer than two bars"))?;
        let scale = ctx.harmony.scale_notes(52, 88, Some(ch)).map_err(bad)?;
        let run: Vec<i32> = scale.into_iter().step_by(2).take(8).collect();
        let n = run.len() as f64;
        for (j, &m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            mix.place(&bus, &Sig::Mono(keys(ctx, &[m], 0.12, 0.75, None)?), tj, 0.22, 0.0);
        }
        Ok(())
    }

    /// The melody's run up the last chord into the title slam (16ths over the last beat).
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = ctx
            .harmony
            .chords
            .last()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let tones = ctx.harmony.chord_tones(60, 88, ch).map_err(bad)?;
        if tones.is_empty() {
            return Err(MusicError::new(
                H,
                format!("the last chord {ch:?} has no tone from 60 to 88"),
            ));
        }
        for j in 0..8usize {
            let m = tones[j.min(tones.len() - 1)];
            let t = t_last + 2.0 * grid.beat + j as f64 * grid.beat / 4.0;
            mix.place(&bus, &Sig::Mono(keys(ctx, &[m], 0.1, 0.8, None)?), t, 0.24, 0.0);
        }
        Ok(())
    }
}
