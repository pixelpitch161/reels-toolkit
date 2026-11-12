//! ORGAN's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/organ.py` (#35's `audio.py`,
//! the organ's part only).
//!
//! The Pixel Pitch organ plays the chords: every pipe row is a short chord stab on the beat (the chord of the bar,
//! never held; it loses voices below 50 and 25 HP), a landed row adds the pedal and a high accent, the leader gets
//! the pedal an octave up; CATHEDRAL is a held full-organ chord, PULL ALL STOPS a swell through the stops, TOCCATA
//! a falling 16th run over the telegraph and the lowest, fullest chord on the stomp (with the bed's `boom`); the
//! title blast is the full organ, pedal to mixture, the count-in a stab per count; it falls apart as clattering
//! pipes down a diminished arpeggio; the stinger, the toccata solo, the reassembly run. Role `bass`: every row is
//! a stab and the pedal note, and the pedal climbs the last chord into the title slam.
//!
//! The organ's voice draws no noise, but its notes are still asked for in the kit's order: the first of them
//! measures the voice's level.
//!
//! One thing of the kit's part has no way into the Studio: a solo note that is a chord symbol (`('dim', r)`, or a
//! chord name and a root). The fight file's solos hold plain notes, which the kit doubles an octave down.

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, py_min, round_nd};
use bf_core::rng::Generator;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Samples, event_is_str, event_number, noise_seed};
use bf_sound::synth::{self, THUD_SEED, clip};
use bf_sound::traits::{Bed, BedHook, Played, Sound};

use crate::fighters::shared::hash::h01;

/// The name the part's errors carry.
const H: &str = "ORGAN";

/// `chord`'s default held length, in seconds.
pub const CHORD_DUR: f64 = 0.17;
/// `chord`'s default velocity.
pub const CHORD_VEL: f64 = 0.85;

/// What a fight file can change when one of its chords is too small for the organ's voicing.
const CHORD_FIELDS: &str = "music.chords_intro or music.chord_loop";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `n ** 0.6` for a chord of 1 to 9 notes, as Python computes it on the reference PC (CPython 3.13: the C runtime's
/// `pow`), bit for bit: `(n ** 0.6).hex()`. They are constants of the kit (its chords have 1, 2, 3, 6, 7 or 8
/// notes), held here because `bf_core::dmath::pow(6.0, 0.6)` is one step above the C runtime's
/// (`0x1.770f5a7edd25cp+1` against `0x1.770f5a7edd25bp+1`); for the other eight the two agree.
const BALANCE: [u64; 9] = [
    0x3FF0_0000_0000_0000,
    0x3FF8_4060_03B2_AE5C,
    0x3FFE_EE50_4BC3_2424,
    0x4002_6111_86BA_E674,
    0x4005_0321_026F_9746,
    0x4007_70F5_A7ED_D25B,
    0x4009_B677_E285_EAF0,
    0x400B_DB8C_DADB_E120,
    0x400D_E5C5_593E_5600,
];

/// `len(ms) ** 0.6`: [`BALANCE`] for the chords the kit plays, `dmath::pow` for a longer list (a fight file's
/// title hits can be one).
fn balance(n: usize) -> f64 {
    match n.checked_sub(1).and_then(|i| BALANCE.get(i)) {
        Some(&bits) => f64::from_bits(bits),
        None => dmath::pow(n as f64, 0.6),
    }
}

/// `clank(seed, vel)`: a tin pipe clattering on the canvas (#35): four decaying partials of 620 Hz, each up to
/// 5 % off, 0.4 s at 48 kHz.
pub fn clank(seed: u64, vel: f64) -> Vec<f64> {
    // n = int(0.4 * A.SR)
    let n = (0.4 * bf_sound::SRF) as usize;
    let t = synth::tt(n);
    let mut rng = Generator::new(seed);
    let mut x = vec![0.0; n];
    for r in [1.0, 2.76, 5.4, 8.9] {
        // x += np.sin(2 * np.pi * 620 * r * (1 + rng.uniform(-0.05, 0.05)) * t) * np.exp(-t * (8 + 4 * r))
        let w = 2.0 * dmath::PI * 620.0 * r * (1.0 + rng.uniform(-0.05, 0.05));
        let decay = 8.0 + 4.0 * r;
        for (o, &t) in x.iter_mut().zip(&t) {
            *o += dmath::sin(w * t) * dmath::exp(-t * decay);
        }
    }
    x.iter().map(|&s| s * vel * 0.5).collect()
}

/// ORGAN's part.
#[derive(Clone, Copy, Debug)]
pub struct OrganSound {
    k: usize,
}

impl OrganSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> OrganSound {
        OrganSound { k }
    }

    /// `voicing(ch)`: the chord's tones in G3..F#4: #35's VOICING (Dm 57 62 65, Bb 58 62 65, Gm 55 58 62, A 57 61
    /// 64).
    ///
    /// # Errors
    /// `ch` is not a chord name.
    pub fn voicing(&self, ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        ctx.harmony.chord_tones(55, 66, ch).map_err(bad)
    }

    /// `pedal(ch)`: the chord's root in A1..G#2: #35's PEDAL (Dm 38, Bb 34, Gm 43, A 33).
    ///
    /// # Errors
    /// `ch` is not a chord name.
    pub fn pedal(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        ctx.harmony.root_above(ch, 33).map_err(bad)
    }

    /// `full(ch)`: the full organ, pedal to mixture: the root in three octaves under the voicing and its lower
    /// two an octave up, each note once (#35's title blast on Dm: 26 38 50 57 62 65 69 74).
    ///
    /// # Errors
    /// `ch` is not a chord name, or has fewer than two tones in the voicing (Python raises an `IndexError`).
    pub fn full(&self, ctx: &MusicCtx, ch: &str) -> Result<Vec<i32>, MusicError> {
        let r = ctx.harmony.root_above(ch, 48).map_err(bad)?;
        let vo = self.voicing(ctx, ch)?;
        let (Some(&v0), Some(&v1)) = (vo.first(), vo.get(1)) else {
            return Err(too_few(ch, vo.len(), 2));
        };
        // list(dict.fromkeys([r - 24, r - 12, r] + list(vo) + [vo[0] + 12, vo[1] + 12]))
        let mut out: Vec<i32> = Vec::new();
        for m in [r - 24, r - 12, r]
            .into_iter()
            .chain(vo.iter().copied())
            .chain([v0 + 12, v1 + 12])
        {
            if !out.contains(&m) {
                out.push(m);
            }
        }
        Ok(out)
    }

    /// `toccata_run()`: a falling 16th run over an octave from the key's fifth, the leading tone raised (#35 in D
    /// minor: A5 G F E D C# Bb A).
    pub fn toccata_run(&self, ctx: &MusicCtx) -> Vec<i32> {
        let key = ctx.harmony.key_pc;
        let mut pcs: Vec<i32> = ctx.harmony.scale.iter().map(|s| (key + s).rem_euclid(12)).collect();
        if let Some(i) = pcs.iter().position(|&p| p == (key + 10).rem_euclid(12)) {
            pcs[i] = (key + 11).rem_euclid(12);
        }
        // the fifth in C5..B5 (D minor: A5 = 81)
        let top = 72 + ((key + 7) - 72).rem_euclid(12);
        (top - 12..=top)
            .rev()
            .filter(|m| pcs.contains(&m.rem_euclid(12)))
            .collect()
    }

    /// `chord(ms, dur, vel, cap)`: #35's `organ_chord`: the Pixel Pitch organ's notes summed, balanced by
    /// `1 / n ** 0.6`. Python's defaults are [`CHORD_DUR`], [`CHORD_VEL`] and no `cap` (then `round(dur + 0.22,
    /// 2)`; a `cap` of 0 counts as none, as in Python).
    ///
    /// # Errors
    /// As [`MusicCtx::note`]; no notes (Python's sum is then the number 0, which no bus takes); notes of
    /// different lengths (numpy refuses to add them).
    pub fn chord(&self, ctx: &MusicCtx, ms: &[i32], dur: f64, vel: f64, cap: Option<f64>) -> Samples {
        let cap = match cap {
            Some(c) if c != 0.0 => c,
            _ => round_nd(dur + 0.22, 2),
        };
        let pp = ctx.cfg[self.k].pp.as_str();
        let mut sum: Option<Vec<f64>> = None;
        for &m in ms {
            let x = ctx.note(pp, f64::from(m), dur, cap, vel)?;
            match &mut sum {
                // sum(...) starts from 0
                None => sum = Some(x.iter().map(|&v| 0.0 + v).collect()),
                Some(s) => {
                    if s.len() != x.len() {
                        return Err(MusicError::new(
                            H,
                            format!("a chord's notes have {} and {} samples", s.len(), x.len()),
                        ));
                    }
                    for (o, &v) in s.iter_mut().zip(x.iter()) {
                        *o += v;
                    }
                }
            }
        }
        let mut sum = sum.ok_or_else(|| MusicError::new(H, "a chord of no notes"))?;
        // / max(1.0, len(ms) ** 0.6)
        let by = py_max(1.0, balance(ms.len()));
        for o in &mut sum {
            *o /= by;
        }
        Ok(sum)
    }

    /// `-1.0 if k == 0 else 1.0`: #35's organ sat left: it pans toward its own side.
    fn side(&self) -> f64 {
        if self.k == 0 { -1.0 } else { 1.0 }
    }

    /// `self.colour(x, t, drive_k)`, or with `low=True`.
    fn tone(
        &self,
        ctx: &MusicCtx,
        x: &[f64],
        t: f64,
        drive_k: f64,
        low: bool,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let args = ColourArgs {
            low,
            ..self.colour_args()
        };
        self.colour(ctx, x, t, drive_k, &args)
    }
}

/// A chord with fewer tones in the voicing than the part reads (Python raises an `IndexError`).
fn too_few(ch: &str, have: usize, need: usize) -> MusicError {
    MusicError::new(
        H,
        format!(
            "the chord {ch:?} has {have} tone(s) from G3 to F#4, and the organ's voicing needs {need}: use a chord \
             of three notes or more in {CHORD_FIELDS}"
        ),
    )
}

/// `vo[0]`.
fn lowest(ch: &str, vo: &[i32]) -> Result<i32, MusicError> {
    vo.first().copied().ok_or_else(|| too_few(ch, 0, 1))
}

/// `vo[-1]`.
fn highest(ch: &str, vo: &[i32]) -> Result<i32, MusicError> {
    vo.last().copied().ok_or_else(|| too_few(ch, 0, 1))
}

impl Sound for OrganSound {
    fn k(&self) -> usize {
        self.k
    }

    /// TOCCATA's stomp lands on `M.bed.boom` without asking (`organ.py` 662).
    fn bed_calls(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    /// #35's organ bus (x 2: short stabs every 2 beats).
    fn bus_gain(&self) -> f64 {
        2.0
    }

    fn send(&self) -> f64 {
        0.55
    }

    /// The stabs G3 to F#4 (and the reed's 2nd and 3rd harmonics).
    fn band(&self) -> (f64, f64) {
        (180.0, 1200.0)
    }

    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        // notes = self.T.TITLE_HITS.get(self.name) or self.full(self.T.CHORDS[0])
        let notes = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(hits) if !hits.is_empty() => hits.clone(),
            _ => {
                let ch = ctx
                    .harmony
                    .chords
                    .first()
                    .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
                self.full(ctx, ch)?
            }
        };
        let x = self.chord(ctx, &notes, 0.5, 1.0, Some(1.2))?;
        mix.place(&self.bus(ctx), &Sig::Mono(x), t, 0.40, 0.0);
        Ok(())
    }

    /// A stab on each count (on the count-in's chord).
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let ch = ctx
            .harmony
            .chords
            .get(1)
            .ok_or_else(|| MusicError::new(H, "the episode has no bar 1 to count in on"))?;
        for (tc, _) in ctx.show.grid.count.iter().take(3) {
            let x = self.chord(ctx, &self.voicing(ctx, ch)?, 0.18, 0.9, None)?;
            mix.place(&bus, &Sig::Mono(x), *tc, 0.30, 0.0);
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let side = self.side();
        let bass = self.role(ctx) == "bass";
        let cfg = ctx.cfg[k];
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            // if len(e) < 3 or e[2] != k or kind not in ('line', 'hit', 'ability', 'slam'): continue
            if e.who() != Some(k) || !matches!(e.kind.as_str(), "line" | "hit" | "ability" | "slam") {
                continue;
            }
            let tq = ctx.q16(t);
            let ch = ctx.harmony.chord_at(tq);
            match e.kind.as_str() {
                "line" => {
                    // a pipe row = a chord stab on the beat
                    let hp = ctx.hp_at(k, t) / 100.0;
                    let mut vo = self.voicing(ctx, ch)?;
                    if hp < 0.5 {
                        // vo[:2] if h01_35(t, 1) < 0.6 else vo[1:]
                        if h01(&[t, 1.0]) < 0.6 {
                            vo.truncate(2);
                        } else if !vo.is_empty() {
                            vo.remove(0);
                        }
                    }
                    if hp < 0.25 {
                        vo.truncate(1);
                    }
                    let x = self.chord(ctx, &vo, 0.17, 0.85, None)?;
                    let (sig, g, lead) = self.tone(ctx, &x, t, 0.8, false)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.34 * g, 0.15 * side);
                    if bass {
                        // the bass role: the pedal under every stab
                        let x = self.chord(ctx, &[self.pedal(ctx, ch)?], 0.3, 0.9, None)?;
                        let sig = self.tone(ctx, &x, t, 0.6, true)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.30 * g, 0.1 * side);
                    }
                    if lead > 0.04 {
                        // the leader: the pedal an octave up under it
                        let x = self.chord(ctx, &[self.pedal(ctx, ch)? + 12], 0.2, 0.7, None)?;
                        let sig = self.tone(ctx, &x, t, 1.0, false)?.0;
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.16 * g, 0.1 * side);
                    }
                }
                "hit" if event_is_str(H, e, 7, "pipe")? => {
                    // a row lands: the pedal + a high accent
                    let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
                    let tl = if ctx.q16(t) < t { tq + s16 } else { tq };
                    let top = highest(ch, &self.voicing(ctx, ch)?)? + 12;
                    if bass {
                        // (the pedal is already under every row)
                        let x = self.chord(ctx, &[top], 0.22, 1.0, None)?;
                        let (sig, g, _) = self.tone(ctx, &x, t, 0.6, false)?;
                        // 0.30 * g / 2 ** 0.6
                        mix.place(&bus, &Sig::Mono(sig), tl, 0.30 * g / dmath::pow(2.0, 0.6), pan);
                    } else {
                        let x = self.chord(ctx, &[self.pedal(ctx, ch)?, top], 0.22, 1.0, None)?;
                        let (sig, g, _) = self.tone(ctx, &x, t, 0.6, false)?;
                        mix.place(&bus, &Sig::Mono(sig), tl, 0.30 * g, pan);
                    }
                    let thump = synth::thud(0.3, 130.0, 55.0, noise_seed(H, t * 7.0)?);
                    mix.place("fx", &Sig::Mono(thump), tq, 0.14, 0.0);
                }
                "ability" => {
                    let is = |name: &str| event_is_str(H, e, 3, name);
                    let vo = self.voicing(ctx, ch)?;
                    if is(&cfg.unlock1.name)? {
                        // CATHEDRAL: a held full-organ chord under the dome
                        let mut ms = vec![self.pedal(ctx, ch)?, lowest(ch, &vo)? - 12];
                        ms.extend(&vo);
                        ms.push(highest(ch, &vo)? + 12);
                        let x = self.chord(ctx, &ms, 2.0 * beat, 0.8, Some(2.0 * beat + 0.6))?;
                        mix.place(&bus, &Sig::Mono(x), tq, 0.20, 0.0);
                    } else if is(&cfg.unlock2.name)? {
                        // PULL ALL STOPS: a swell, the chord climbing the stops
                        let mut ms = vec![self.pedal(ctx, ch)?];
                        ms.extend(&vo);
                        ms.push(highest(ch, &vo)? + 12);
                        for (j, m) in ms.into_iter().enumerate() {
                            let j = j as f64;
                            let x = self.chord(ctx, &[m], 2.0 * beat - j * s16, 0.75, Some(2.0 * beat + 0.3))?;
                            mix.place(&bus, &Sig::Mono(x), tq + j * s16, 0.12 + 0.03 * j, 0.0);
                        }
                    } else if is(&cfg.low.name)? {
                        // TOCCATA: a falling 16th run over the telegraph
                        for (j, m) in self.toccata_run(ctx).into_iter().enumerate() {
                            let x = self.chord(ctx, &[m, m - 12], 0.11, 0.9, None)?;
                            mix.place(&bus, &Sig::Mono(x), tq + j as f64 * s16, 0.22, 0.0);
                        }
                    }
                }
                "slam" => {
                    // TOCCATA's stomp: the lowest, fullest chord
                    let ts = e.t;
                    let at = ctx.harmony.chord_at(ts);
                    let vo = self.voicing(ctx, at)?;
                    let root = ctx.harmony.key_root;
                    let mut ms = vec![root - 12, root, lowest(at, &vo)? - 12];
                    ms.extend(&vo);
                    ms.push(highest(at, &vo)? + 12);
                    let x = self.chord(ctx, &ms, 0.6, 1.0, Some(1.4))?;
                    mix.place(&bus, &Sig::Mono(x), ts, 0.34, 0.0);
                    mix.place("fx", &Sig::Mono(synth::thud(0.7, 90.0, 30.0, THUD_SEED)), ts, 0.55, 0.0);
                    // (#35: its bed's timpani + bell)
                    bed.boom(ctx, mix, ts, 0.6)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The pipes clattering down a diminished arpeggio.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        let root = ctx.harmony.key_root;
        for j in 0..7i32 {
            let jf = f64::from(j);
            let pipe = clank(j as u64, 1.0 - 0.08 * jf);
            // pan=(j - 3) / 6
            mix.place("fx", &Sig::Mono(pipe), t_hs + 0.09 * jf, 0.20, f64::from(j - 3) / 6.0);
            let x = self.chord(ctx, &[root + 24 - 3 * j], 0.08, 0.6, None)?;
            mix.place(&bus, &Sig::Mono(x), t_hs + 0.09 * jf, 0.10, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let ch = ctx
            .harmony
            .chords
            .first()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        // self.full(self.T.CHORDS[0])[1:]
        let full = self.full(ctx, ch)?;
        let x = self.chord(ctx, &full[1..], 0.9, 1.0, Some(1.6))?;
        mix.place("solo", &Sig::Mono(x), tw, 0.30, 0.0);
        Ok(())
    }

    /// #35's toccata solo: single notes doubled an octave down; it stops at the reassembly.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            // a late K.O. leaves less room: stop at the reassembly
            if ts >= grid.t_reassemble - 0.05 {
                break;
            }
            let dur = py_min(d * grid.beat * 0.92, grid.t_reassemble - ts);
            let x = self.chord(ctx, &[m, m - 12], dur, 0.9, Some(round_nd(dur + 0.3, 2)))?;
            mix.place("solo", &Sig::Mono(x), ts, 0.34, 0.0);
        }
        Ok(())
    }

    /// Its pipes fly home: a rising run up the tonic chord.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let ch = ctx
            .harmony
            .chords
            .first()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let r = ctx.harmony.root_above(ch, 48).map_err(bad)?;
        let mut run = ctx.harmony.chord_tones(r, r + 36, ch).map_err(bad)?;
        run.truncate(8);
        let n = run.len() as f64;
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / n);
            let x = self.chord(ctx, &[m], 0.1, 0.7, None)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.16, 0.0);
        }
        Ok(())
    }

    /// The bass role: the pedal climbs the last chord (the dominant) into the title slam, root to octave.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        if self.role(ctx) != "bass" {
            return Ok(());
        }
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = ctx
            .harmony
            .chords
            .last()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let r = self.pedal(ctx, ch)?;
        let mut climb = ctx.harmony.chord_tones(r, r + 12, ch).map_err(bad)?;
        climb.truncate(4);
        if climb.is_empty() {
            // Python: `bt % len(climb)` divides by zero (the root is a tone of its chord, so no chord gets here)
            return Err(MusicError::new(
                H,
                format!("the last chord {ch:?} has no tone to climb"),
            ));
        }
        let side = self.side();
        for bt in 0..4i64 {
            let m = climb[imod_py(bt, climb.len() as i64) as usize];
            let x = self.chord(ctx, &[m, m + 12], 0.35, 0.9, None)?;
            mix.place(&bus, &Sig::Mono(x), t_last + bt as f64 * grid.beat, 0.34, 0.1 * side);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `n ** 0.6` balances a chord of `n` notes. The table holds Python's bits; `dmath::pow` gives the same for
    /// every size but 6, where it is one step above.
    #[test]
    fn the_chord_balance_is_pythons() {
        for (i, bits) in BALANCE.into_iter().enumerate() {
            let n = i + 1;
            assert_eq!(balance(n).to_bits(), bits, "{n} ** 0.6");
            let ours = dmath::pow(n as f64, 0.6).to_bits();
            assert_eq!(ours, if n == 6 { bits + 1 } else { bits }, "dmath::pow({n}, 0.6)");
        }
        assert_eq!(balance(10), dmath::pow(10.0, 0.6));
        assert!(balance(0).is_nan() || balance(0) == 0.0);
        // 0.30 / 2 ** 0.6 (a landed row in the bass role)
        assert_eq!((0.30 / dmath::pow(2.0, 0.6)).to_bits(), 0x3FC9_55A5_3144_CA82);
    }

    /// A clattering pipe is 0.4 s at 48 kHz, seeded, and scales with its level.
    #[test]
    fn a_pipe_clatters_for_four_tenths_of_a_second() {
        let a = clank(3, 1.0);
        assert_eq!(a.len(), 19_200);
        assert_eq!(a, clank(3, 1.0));
        assert_ne!(a, clank(4, 1.0));
        assert_eq!(a[0], 0.0);
        let quiet = clank(3, 0.5);
        assert_eq!(quiet[700], a[700] / 1.0 * 0.5);
        let end = a[a.len() - 480..].iter().fold(0.0f64, |p, v| p.max(v.abs()));
        assert!(end < 0.02, "{end}");
    }
}
