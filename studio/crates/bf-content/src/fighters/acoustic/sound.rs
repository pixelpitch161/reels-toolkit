//! ACOUSTIC's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/acoustic.py` (#33), with the
//! module's chord helpers (`guitar_chord`, `h01_33`, `bass_pair`).
//!
//! Every stroke is a strum in the stroke's own direction (down: the full chord low to high; up: the top strings high
//! to low), CAPO's extra strokes bright and short; a landed stroke adds a bass-string pop and the strings' slap; the
//! leader adds the boom-chick alternating bass; it loses strings (6 down to 3) as it gets hurt and its up-strokes
//! drop out; FINGERPICK's whips are single picked notes up the chord (a landed one a harmonic ping); BIG STRUM is one
//! giant slow strum over the low E. Every note is its Pixel Pitch voice through `M.note`. The part reads the run's
//! events, nothing of its look.
//!
//! The kit's `solo` also strums a chord where the config's `SOLO` entry is a tuple; a fight file's solo holds single
//! notes only (`music.solos`), so that branch has nothing to play here.

use bf_core::dmath;
use bf_core::harmony::{HarmonyError, parse_chord};
use bf_core::py::{py_max, py_min, round_half_even, round_nd};
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{MusicCtx, MusicError, Samples, event_int, event_is, event_is_str, event_number, noise_seed};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, Played, Sound};

/// The name the part's errors carry.
const H: &str = "ACOUSTIC";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `OPEN`: the open chord shapes (EADGBE), low string first.
const OPEN: &[(&str, &[i32])] = &[
    ("E", &[40, 47, 52, 56, 59, 64]),
    ("Em", &[40, 47, 52, 55, 59, 64]),
    ("E7", &[40, 47, 50, 56, 59, 64]),
    ("Am", &[45, 52, 57, 60, 64]),
    ("A", &[45, 52, 57, 61, 64]),
    ("A7", &[45, 52, 55, 61, 64]),
    ("Am7", &[45, 52, 55, 60, 64]),
    ("C", &[48, 52, 55, 60, 64]),
    ("D", &[50, 57, 62, 66]),
    ("Dm", &[50, 57, 62, 65]),
    ("D7", &[50, 57, 60, 66]),
    ("G", &[43, 47, 50, 55, 59, 67]),
    ("F", &[41, 48, 53, 57, 60, 65]),
    ("B7", &[47, 51, 57, 59, 66]),
    ("Bm", &[47, 54, 59, 62, 66]),
];
/// `E_SHAPE`: the barre shapes on the low E string, by quality.
const E_SHAPE: &[(&str, &[i32])] = &[
    ("", &[0, 7, 12, 16, 19, 24]),
    ("m", &[0, 7, 12, 15, 19, 24]),
    ("7", &[0, 7, 10, 16, 19, 24]),
    ("m7", &[0, 7, 10, 15, 19, 24]),
];
/// `A_SHAPE`: the barre shapes on the A string, by quality.
const A_SHAPE: &[(&str, &[i32])] = &[
    ("", &[0, 7, 12, 16, 19]),
    ("m", &[0, 7, 12, 15, 19]),
    ("7", &[0, 7, 10, 16, 19]),
    ("m7", &[0, 7, 10, 15, 19]),
];

fn shape<'a>(table: &'a [(&str, &'a [i32])], key: &str) -> Option<&'a [i32]> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// `guitar_chord(ch)`: the chord as a strummed guitar shape (MIDI, low string first): the open shape if it has one,
/// else the E- or A-shape barre lower on the neck, else its tones stacked from the low strings.
///
/// # Errors
/// `ch` is not a chord name (Python's `parse_chord` raises).
pub fn guitar_chord(ch: &str) -> Result<Vec<i32>, MusicError> {
    if let Some(open) = shape(OPEN, ch) {
        return Ok(open.to_vec());
    }
    let chord = parse_chord(ch).map_err(bad)?;
    // n = 2 if len(ch) > 1 and ch[1] in '#b' else 1; q = ch[n:]
    let mut it = ch.char_indices();
    it.next();
    let split = match it.next() {
        Some((i, c)) if c == '#' || c == 'b' => i + c.len_utf8(),
        Some((i, _)) => i,
        None => ch.len(),
    };
    let q = &ch[split..];
    let root = chord.root;
    let (fe, fa) = ((root - 4).rem_euclid(12), (root - 9).rem_euclid(12));
    if let Some(e) = shape(E_SHAPE, q)
        && fe <= fa
    {
        return Ok(e.iter().map(|i| 40 + fe + i).collect());
    }
    if let Some(a) = shape(A_SHAPE, q) {
        return Ok(a.iter().map(|i| 45 + fa + i).collect());
    }
    let pcs = chord.pcs();
    let lo = 40 + fe;
    let mut out = vec![lo];
    out.extend((lo + 1..68).filter(|m| pcs.contains(&m.rem_euclid(12))).take(4));
    Ok(out)
}

/// `h01_33(*xs)`: #33's deterministic hash in [0, 1) (its up-stroke dropout).
pub fn h01_33(xs: &[f64]) -> f64 {
    const P: i64 = 2_147_483_647;
    let mut v: i64 = 0;
    for &x in xs {
        // v = (v * 1000003 + int(x * 1000) + 7) % 2147483647 (Python's % is never negative)
        v = (v * 1_000_003 + (x * 1000.0) as i64 + 7).rem_euclid(P);
    }
    ((v * 48271).rem_euclid(P)) as f64 / 2_147_483_647.0
}

/// `bass_pair(ch)`: `(root on 1, fifth on 3)` for the boom-chick.
///
/// # Errors
/// `ch` is not a chord name.
pub fn bass_pair(ch: &str) -> Result<(i32, i32), MusicError> {
    let r = 40 + (parse_chord(ch).map_err(bad)?.root - 4).rem_euclid(12);
    Ok((r, r + 7))
}

/// `strings_left(hp)`: it loses strings as it gets hurt.
pub fn strings_left(hp: f64) -> usize {
    if hp > 60.0 {
        6
    } else if hp > 35.0 {
        5
    } else if hp > 15.0 {
        4
    } else {
        3
    }
}

/// Python's `int(x * SR)` for a length in samples, cut toward zero.
fn samples(seconds: f64) -> Result<usize, MusicError> {
    let n = seconds * SRF;
    if n.is_finite() && n > -1.0 && n < 1.0e9 {
        Ok(n as usize)
    } else {
        Err(MusicError::new(H, format!("{seconds} s is not a length")))
    }
}

/// `A.hp(np.random.default_rng(seed).normal(0, 1, n), f) * np.exp(-A.tt(n) / tau)`: a click of filtered noise.
fn click(seed: u64, n: usize, f: f64, tau: f64) -> Vec<f64> {
    let x = synth::hp(&synth::noise(seed, n), f, FILTER_O);
    x.iter()
        .zip(synth::tt(n))
        .map(|(v, t)| v * dmath::exp(-t / tau))
        .collect()
}

/// The ACOUSTIC's part.
#[derive(Clone, Copy, Debug)]
pub struct AcousticSound {
    k: usize,
}

impl AcousticSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> AcousticSound {
        AcousticSound { k }
    }

    /// `self.pan`: -0.25 on the left, 0.25 on the right.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.25 } else { 0.25 }
    }

    /// `note(m, dur=0.3, cap=0.9, vel=0.85)`: `M.note(self.pp, int(m), dur, cap, vel)`.
    fn note(&self, ctx: &MusicCtx, m: f64, dur: f64, cap: f64, vel: f64) -> Samples {
        let pp = ctx.cfg[self.k].pp.as_str();
        Ok(ctx.note(pp, m.trunc(), dur, cap, vel)?.to_vec())
    }

    /// `strum(notes, vel=0.85, spread=0.011, up=False, dur=0.40, cap=1.0)`: the chord's strings a few ms apart,
    /// low to high (down) or high to low (up).
    #[allow(clippy::too_many_arguments)] // the Python signature
    fn strum(&self, ctx: &MusicCtx, notes: &[i32], vel: f64, spread: f64, up: bool, dur: f64, cap: f64) -> Samples {
        let mut ns = notes.to_vec();
        if up {
            ns.reverse();
        }
        let d = samples(spread)?;
        let mut out = vec![0.0; samples(cap)? + d * ns.len()];
        for (j, &m) in ns.iter().enumerate() {
            let v = round_nd(vel * if j == 0 { 1.0 } else { 0.9 }, 2);
            let x = self.note(ctx, f64::from(m), dur, cap, v)?;
            // out[j * d:j * d + len(x)] += x
            for (o, s) in out[j * d..].iter_mut().zip(&x) {
                *o += s;
            }
        }
        let root = (ns.len() as f64).sqrt();
        Ok(out.iter().map(|v| v / root * 1.5).collect())
    }

    /// `self.colour(x, t, drive_k)` with its colour arguments: `(signal, gain, lead)`.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64, f64), MusicError> {
        self.colour(ctx, x, t, drive_k, &self.colour_args())
    }

    /// `guitar_chord(T.CHORDS[i])`.
    fn chord_of_bar(&self, ctx: &MusicCtx, i: usize) -> Result<Vec<i32>, MusicError> {
        let ch = ctx
            .harmony
            .chords
            .get(i)
            .ok_or_else(|| MusicError::new(H, format!("the episode has no bar {i}")))?;
        guitar_chord(ch)
    }

    /// `tuple(ch) + ((ch[-1] + 5,) if len(ch) < 6 else ())`: a shape of fewer than six strings gets a fourth over
    /// its top note.
    fn full_six(ch: &[i32]) -> Vec<i32> {
        let mut out = ch.to_vec();
        if ch.len() < 6
            && let Some(&top) = ch.last()
        {
            out.push(top + 5);
        }
        out
    }
}

impl Sound for AcousticSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.45
    }

    fn send(&self) -> f64 {
        0.25
    }

    /// #33's band check.
    fn band(&self) -> (f64, f64) {
        (300.0, 5000.0)
    }

    /// `voice(m, dur, vel=0.85)`: `self.note(m, dur, max(0.6, dur + 0.3), vel)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<std::sync::Arc<[f64]>, MusicError> {
        Ok(self.note(ctx, m, dur, py_max(0.6, dur + 0.3), vel)?.into())
    }

    /// The full open chord rung out, the bass root under it (the fight file's title hits: the chord's notes).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let ch = self.chord_of_bar(ctx, 0)?;
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) => h.clone(),
            None => Self::full_six(&ch),
        };
        let Some(&low) = hits.first() else {
            return Err(MusicError::new(H, "the title hits are an empty list"));
        };
        let bus = self.bus(ctx);
        let pan = self.pan();
        mix.place(
            &bus,
            &Sig::Mono(self.strum(ctx, &hits, 1.0, 0.016, false, 0.9, 1.8)?),
            t,
            0.55,
            pan,
        );
        mix.place(
            &bus,
            &Sig::Mono(self.note(ctx, f64::from(low), 0.9, 1.6, 1.0)?),
            t,
            0.30,
            pan,
        );
        Ok(())
    }

    /// Bar 1's chord strummed on the count-in.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let ch = self.chord_of_bar(ctx, 1)?;
        let bus = self.bus(ctx);
        for (tc, _) in ctx.show.grid.count.iter().take(3) {
            mix.place(
                &bus,
                &Sig::Mono(self.strum(ctx, &ch, 0.9, 0.013, false, 0.40, 1.0)?),
                *tc,
                0.30,
                self.pan(),
            );
        }
        Ok(())
    }

    /// SIX-STRING RAKE: every stroke is a strum in its own direction; a landed stroke adds a bass pop and a slap;
    /// FINGERPICK whips are single picked notes up the chord, a landed whip a harmonic ping; BIG STRUM one giant
    /// strum; CAPO a metal click and a ring.
    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let pan = self.pan();
        let other_pan = -pan;
        let beat = ctx.show.grid.beat;
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            let kind = e.kind.as_str();
            if kind == "rake" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let hp = ctx.hp_at(k, t);
                let ch = ctx.harmony.chord_at(tq);
                let full = guitar_chord(ch)?;
                // vo = full[:max(3, self.strings_left(hp) - (6 - len(full)))]: broken strings go from the top
                let keep = (strings_left(hp) as i64 - (6 - full.len() as i64)).max(3) as usize;
                let vo = &full[..keep.min(full.len())];
                let main = event_int(H, e, 3)? != 0;
                let down = event_number(H, e, 8)? > 0.0;
                let lead = (hp - ctx.hp_at(1 - k, t)) / 100.0;
                let (mut sig, gain) = if down {
                    let s = if main {
                        self.strum(ctx, vo, 0.92, 0.013, false, 0.40, 1.0)?
                    } else {
                        self.strum(ctx, vo, 0.7, 0.009, false, 0.2, 0.6)?
                    };
                    (s, if main { 0.36 } else { 0.24 })
                } else {
                    // the up-strokes drop out when hurt
                    if main && h01_33(&[tq, 7.0]) < 0.6 * dmath::pow(1.0 - hp / 100.0, 1.6) {
                        continue;
                    }
                    let top = &vo[vo.len() - vo.len().min(4)..];
                    let s = self.strum(ctx, top, if main { 0.74 } else { 0.6 }, 0.008, true, 0.2, 0.6)?;
                    (s, if main { 0.27 } else { 0.2 })
                };
                if !main {
                    // CAPO: bright and short
                    sig = synth::hp(&sig, 350.0, FILTER_O);
                }
                let (y, g, _) = self.col(ctx, &sig, t, 0.7)?;
                mix.place(&bus, &Sig::Mono(y), tq, gain * g, pan);
                if main && down && lead > 0.04 {
                    // the leader: boom-chick alternating bass under it
                    let (r, f5) = bass_pair(ch)?;
                    let pos = (round_half_even(tq / beat) as i64).rem_euclid(4);
                    let m = if pos == 0 || pos == 1 { r } else { f5 };
                    let x = self.note(ctx, f64::from(m), 0.4, 0.7, 1.0)?;
                    mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 1.0)?.0), tq, 0.24 * g, pan);
                }
            } else if kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, "rake")? {
                let tq = ctx.q16(t);
                let r = bass_pair(ctx.harmony.chord_at(tq))?.0;
                let m = if r - 12 >= 40 { r - 12 } else { r };
                let x = self.note(ctx, f64::from(m), 0.3, 0.6, 1.0)?;
                mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 1.0)?.0), tq, 0.18, pan);
                // the strings' slap
                let thud = synth::thud(0.25, 180.0, 80.0, noise_seed(H, t * 9.0)?);
                mix.place("fx", &Sig::Mono(thud), tq, 0.12, other_pan);
            } else if kind == "whip" && event_is(H, e, 2, k)? {
                // FINGERPICK: one picked note per whip, up the chord
                let tq = ctx.q16(t);
                let ch = guitar_chord(ctx.harmony.chord_at(tq))?;
                // ms = list(ch[2:]) + [ch[-1] + 12, ch[-2] + 12]
                let n_ = ch.len();
                let mut ms: Vec<i32> = ch[2.min(n_)..].to_vec();
                ms.push(ch[n_ - 1] + 12);
                ms.push(ch[n_ - 2] + 12);
                let j = event_int(H, e, 3)?;
                let m = ms[j.rem_euclid(ms.len() as i64) as usize];
                let x = self.note(ctx, f64::from(m), 0.3, 0.7, 0.9)?;
                let p = pan + 0.08 * (j as f64 - 2.5) / 2.5;
                mix.place(&bus, &Sig::Mono(self.col(ctx, &x, t, 1.0)?.0), tq, 0.30, p);
                // the crack of the string whip
                let n = samples(0.03)?;
                mix.place(
                    "fx",
                    &Sig::Mono(click(noise_seed(H, t * 50.0)?, n, 4000.0, 0.004)),
                    tq,
                    0.05,
                    pan,
                );
            } else if kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, "whip")? {
                // a whip lands: a harmonic ping
                let tq = ctx.q16(t);
                let notes = ctx
                    .harmony
                    .scale_notes(76, 93, Some(ctx.harmony.chord_at(tq)))
                    .map_err(bad)?;
                let i = event_int(H, e, 8)?.rem_euclid(6) as usize;
                let m = *notes
                    .get(i)
                    .ok_or_else(|| MusicError::new(H, format!("the scale from 76 to 93 has no note {i}")))?;
                mix.place(&bus, &Sig::Mono(synth::chime(f64::from(m), 0.6)), tq, 0.10, other_pan);
            } else if kind == "sweep" && event_is(H, e, 2, k)? {
                // BIG STRUM: one giant slow strum across the ring
                let ts = ctx.q16(e.t);
                let dur = event_number(H, e, 7)?;
                let ch = guitar_chord(ctx.harmony.chord_at(ts))?;
                let strum = self.strum(ctx, &ch, 1.0, py_min(0.09, dur / 7.0), false, 1.0, 2.0)?;
                mix.place(&bus, &Sig::Mono(strum), ts, 0.62, pan);
                mix.place(&bus, &Sig::Mono(self.note(ctx, 40.0, 1.0, 1.8, 1.0)?), ts, 0.40, pan);
                let whoosh = synth::whoosh(dur, 200.0, 3500.0, WHOOSH_RISE, WHOOSH_SEED);
                mix.place("fx", &Sig::Mono(whoosh), ts, 0.16, 0.0);
            } else if kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, "sweep")? {
                let thud = synth::thud(0.7, 90.0, 32.0, THUD_SEED);
                mix.place("fx", &Sig::Mono(thud), e.t, 0.55, 0.0);
            } else if kind == "ability" && event_is(H, e, 2, k)? && event_is_str(H, e, 4, "haste")? {
                // CAPO: the clamp, a metal click and a ring
                let tq = ctx.q16(t);
                let n = samples(0.05)?;
                mix.place("fx", &Sig::Mono(click(8, n, 3000.0, 0.004)), tq, 0.18, 0.0);
                mix.place("fx", &Sig::Mono(synth::chime(96.0, 0.4)), tq, 0.06, 0.0);
            }
        }
        Ok(())
    }

    /// The strings snapping, top to bottom, and the body cracking.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t_hs = ctx.show.t_hs;
        let snap = [64, 59, 55, 50, 45, 40];
        for (j, &m) in snap.iter().enumerate() {
            let j_ = j as f64;
            let x = synth::hp(&self.note(ctx, f64::from(m), 0.2, 0.5, 1.0)?, 200.0, FILTER_O);
            let pan = (j_ - snap.len() as f64 / 2.0) / 6.0;
            mix.place("solo", &Sig::Mono(x), t_hs + 0.09 * j_, 0.30, pan);
            let n = samples(0.04)?;
            mix.place(
                "fx",
                &Sig::Mono(click(40 + j as u64, n, 3000.0, 0.006)),
                t_hs + 0.09 * j_,
                0.10,
                0.0,
            );
        }
        let n = samples(0.3)?;
        let crack: Vec<f64> = synth::bp(&synth::noise(77, n), 150.0, 1200.0, FILTER_O)
            .iter()
            .zip(synth::tt(n))
            .map(|(v, t)| v * dmath::exp(-t / 0.05))
            .collect();
        mix.place("fx", &Sig::Mono(crack), t_hs, 0.35, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = Self::full_six(&self.chord_of_bar(ctx, 0)?);
        mix.place(
            "solo",
            &Sig::Mono(self.strum(ctx, &ch, 1.0, 0.014, false, 0.9, 1.8)?),
            tw,
            0.46,
            0.0,
        );
        Ok(())
    }

    /// The fight file's solo: each number a picked note, up to the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.note(ctx, f64::from(m), py_max(0.1, d * grid.beat * 0.9), 0.8, 0.9)?;
            mix.place("solo", &Sig::Mono(x), ts, 0.48, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as a rising run.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let run = [52, 57, 60, 64, 69, 72, 76, 81];
        for (j, &m) in run.iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            mix.place(
                "solo",
                &Sig::Mono(self.note(ctx, f64::from(m), 0.2, 0.5, 0.7)?),
                tj,
                0.22,
                0.0,
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shapes: open where there is one, else an E- or A-shape barre, else the chord's tones stacked.
    #[test]
    fn chords_are_open_shapes_then_barres() {
        assert_eq!(guitar_chord("Am").unwrap(), vec![45, 52, 57, 60, 64]);
        // F#m: root 6, fe 2 <= fa 9: the E shape at the 2nd fret
        assert_eq!(guitar_chord("F#m").unwrap(), vec![42, 49, 54, 57, 61, 66]);
        // Cm: root 0, fe 8 > fa 3: the A shape at the 3rd fret
        assert_eq!(guitar_chord("Cm").unwrap(), vec![48, 55, 60, 63, 67]);
        // Bdim: no shape: B (43, the low E string's 7th fret) and the next four tones of B D F
        assert_eq!(guitar_chord("Bdim").unwrap(), vec![47, 50, 53, 59, 62]);
        // more, as acoustic.py gives them
        assert_eq!(guitar_chord("Bb").unwrap(), vec![46, 53, 58, 62, 65]);
        assert_eq!(guitar_chord("G7").unwrap(), vec![43, 50, 53, 59, 62, 67]);
        assert_eq!(guitar_chord("Ebm7").unwrap(), vec![51, 58, 61, 66, 70]);
        assert_eq!(guitar_chord("Csus4").unwrap(), vec![48, 53, 55, 60, 65]);
        assert!(guitar_chord("H").is_err());
        assert_eq!(bass_pair("Am").unwrap(), (45, 52));
        assert_eq!(bass_pair("E").unwrap(), (40, 47));
        assert_eq!(strings_left(61.0), 6);
        assert_eq!(strings_left(15.0), 3);
    }

    /// #33's hash: the numbers CPython gives.
    #[test]
    fn the_dropout_hash_is_pythons() {
        // h01_33(0.0) = ((7 * 48271) % 2147483647) / 2147483647.0
        assert_eq!(h01_33(&[0.0]), (7.0 * 48271.0) / 2_147_483_647.0);
        // h01_33(0.5, 7): v = 507, then 507 * 1000003 + 7000 + 7
        let v = 507 * 1_000_003i64 + 7007;
        assert_eq!(
            h01_33(&[0.5, 7.0]),
            ((v * 48271) % 2_147_483_647) as f64 / 2_147_483_647.0
        );
        assert_eq!(h01_33(&[0.5, 7.0]), 0.5052489584801947);
        assert_eq!(h01_33(&[12.345, 7.0]), 0.45604203709216884);
    }
}
