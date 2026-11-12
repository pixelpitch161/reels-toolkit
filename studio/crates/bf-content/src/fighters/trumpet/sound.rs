//! The TRUMPET's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/trumpet.py` (#29 v3).
//!
//! Every combo's three punches play three notes of ONE valve fingering (the open B flat trumpet's harmonic series
//! lowered by the valves, chosen to fit the chord: partials 4-6-8 or 3-4-6), a valve hiss on each; landed punches
//! ring out, misses are ghosts (dropping out when hurt); the leader adds the octave below; DOUBLE TONGUE adds
//! ta-ka graces; FANFARE is a rising call on the key's minor triad; HIGH C swells through its charge and screams
//! with a shake when it fires. With a line in the fight file (`music.lines.TRUMPET`, 8 notes a bar) the punches
//! play that line instead: the trumpet takes the melody (#46).
//!
//! The trumpet's voice draws no noise, but its notes are still asked for in the kit's order: the first of them
//! measures the voice's level.

use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::harmony::{HarmonyError, parse_chord};
use bf_core::py::{py_max, py_min, round_nd};
use bf_core::rng::Generator;
use bf_sound::SRF;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Samples, event_int, event_is, event_is_str, noise_seed};
use bf_sound::synth::{self, FILTER_O, THUD_SEED, clip};
use bf_sound::traits::{Bed, Played, Sound};

use crate::fighters::shared::hash::h01;

/// The name the part's errors carry.
const H: &str = "TRUMPET";

/// `tpt`'s default held length, in seconds.
pub const TPT_DUR: f64 = 0.16;
/// `tpt`'s default velocity.
pub const TPT_VEL: f64 = 0.9;
/// `FINGERING`: semitones down -> the valves pressed.
pub const FINGERING: [&str; 7] = ["0", "2", "1", "12", "23", "13", "123"];

/// `OPEN_PARTIALS[p]`: the concert pitch of the open B flat trumpet's partial `p` (3, 4, 6 or 8).
fn open_partial(p: u8) -> i32 {
    match p {
        3 => 65,
        4 => 70,
        6 => 77,
        _ => 82,
    }
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// `valve_shift(chord)`: `(semitones down, valves)` of the fingering for a chord: partial 4 (B flat 4 open, down
/// to E4 with all three valves) lands on the chord's root when it can, else on its fifth (#29's table: Cm -> 3
/// `12`, Ab7 -> 2 `1`, Fm -> 5 `13`, G7 -> 3 `12`).
///
/// # Errors
/// `chord` is not a chord name.
pub fn valve_shift(chord: &str) -> Result<(i32, &'static str), MusicError> {
    let root = parse_chord(chord).map_err(bad)?.root;
    for tgt in [root, (root + 7).rem_euclid(12)] {
        for (s, valves) in (0i32..).zip(FINGERING) {
            if (70 - s).rem_euclid(12) == tgt {
                return Ok((s, valves));
            }
        }
    }
    Ok((0, FINGERING[0]))
}

/// The TRUMPET's part.
#[derive(Clone, Copy, Debug)]
pub struct TrumpetSound {
    k: usize,
}

impl TrumpetSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> TrumpetSound {
        TrumpetSound { k }
    }

    /// `self.pan`: a quarter to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.25 } else { 0.25 }
    }

    /// `tpt(m, dur, vel, cap)`: `M.note(pp, int(np.clip(m, 58, 86)), round(dur, 3), round(cap or dur + 0.22, 2),
    /// round(vel, 1))`. Python's defaults are [`TPT_DUR`], [`TPT_VEL`] and no `cap` (a `cap` of 0 counts as none).
    ///
    /// # Errors
    /// As [`MusicCtx::note`].
    pub fn tpt(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, cap: Option<f64>) -> Samples {
        let cap = match cap {
            Some(c) if c != 0.0 => c,
            _ => dur + 0.22,
        };
        let x = ctx.note(
            &ctx.cfg[self.k].pp,
            f64::from(m.clamp(58, 86)),
            round_nd(dur, 3),
            round_nd(cap, 2),
            round_nd(vel, 1),
        )?;
        Ok(x.to_vec())
    }

    /// `hiss(dur, seed)`: steam escaping a valve: `default_rng(300 + seed)` noise from 3.5 to 11 kHz under a short
    /// envelope.
    pub fn hiss(dur: f64, seed: u64) -> Vec<f64> {
        let n = (dur * SRF) as usize;
        let band = synth::bp(&synth::noise(300 + seed, n), 3500.0, 11000.0, FILTER_O);
        let env = synth::expenv(n, dur * 0.4, 0.003);
        band.iter().zip(&env).map(|(b, e)| b * e).collect()
    }

    /// `key_clicks(n, seed)`: valves and caps clacking: `n` short clicks in half a second.
    pub fn key_clicks(n: usize, seed: u64) -> Vec<f64> {
        let sr = SRF;
        let mut out = vec![0.0; (0.5 * sr) as usize];
        let mut rng = Generator::new(seed);
        let m = (0.02 * sr) as usize;
        let t = synth::tt(m);
        for _ in 0..n {
            let a = (rng.uniform(0.0, 0.4) * sr) as usize;
            let mut nz = vec![0.0; m];
            rng.fill_normal(0.0, 1.0, &mut nz);
            let c = synth::bp(&nz, 900.0, 4000.0, FILTER_O);
            // out[a:a + m] += c (numpy cuts the slice at the end of `out`)
            for (i, slot) in out.iter_mut().skip(a).take(m).enumerate() {
                *slot += c[i] * dmath::exp(-t[i] / 0.004);
            }
        }
        out
    }

    /// `swell(x, dur)`: a muted note opening up over `dur`: a crescendo, and the harmon mute easing off (low-pass
    /// to open).
    pub fn swell(x: &[f64], dur: f64) -> Vec<f64> {
        let t = synth::tt(x.len());
        let dark = synth::lp(x, 1100.0, FILTER_O);
        let span = py_max(1e-3, dur);
        (0..x.len())
            .map(|i| {
                let k = clip(t[i] / span, 0.0, 1.0);
                let y = dark[i] * (1.0 - k) + x[i] * k;
                y * (0.18 + 0.82 * dmath::pow(k, 1.6))
            })
            .collect()
    }

    /// `combo_notes(t, combo_no)`: the three notes for the combo that starts at `t`: partials 4-6-8 (or 3-4-6 on
    /// alternate combos) of the chord's fingering.
    ///
    /// # Errors
    /// The chord at `t` is not a chord name.
    pub fn combo_notes(&self, ctx: &MusicCtx, t: f64, combo_no: i64) -> Result<[i32; 3], MusicError> {
        let (s, _fingering) = valve_shift(ctx.harmony.chord_at(t))?;
        let parts: [u8; 3] = if combo_no.rem_euclid(2) != 0 {
            [4, 6, 8]
        } else {
            [3, 4, 6]
        };
        Ok(parts.map(|p| open_partial(p) - s))
    }

    /// `key_root(lo)`: the key's root at or above MIDI `lo` (Python's default is 72: C gives 72).
    fn key_root(&self, ctx: &MusicCtx, lo: i32) -> i32 {
        lo + ctx.harmony.key_pc.rem_euclid(12)
    }

    /// `self.colour(x, t, drive_k)[0]`.
    fn tone(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Samples {
        Ok(self.colour(ctx, x, t, drive_k, &self.colour_args())?.0)
    }
}

impl Sound for TrumpetSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #29's trumpet bus.
    fn bus_gain(&self) -> f64 {
        1.8
    }

    fn send(&self) -> f64 {
        0.5
    }

    /// #29's horn band check.
    fn band(&self) -> (f64, f64) {
        (300.0, 5000.0)
    }

    /// #29's colour (a little less drive than the default).
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 90.0,
            cut_span: 520.0,
            drive: 4.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel)`: `self.tpt(m, dur, vel)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<std::sync::Arc<[f64]>, MusicError> {
        Ok(self.tpt(ctx, m as i32, dur, vel, None)?.into())
    }

    /// A lead trumpet's rip up to a high note, held with a shake (the fight file's title hits: the rip's notes,
    /// the last one held).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let pan = self.pan();
        let r = self.key_root(ctx, 72);
        let own = [r - 12, r - 5, r, r + 3, r + 7];
        let hits = ctx
            .ff
            .music
            .title_hits
            .get(self.name(ctx))
            .map_or(&own[..], Vec::as_slice);
        let Some((&last, rip)) = hits.split_last() else {
            return Err(MusicError::new(H, "the title hits are an empty list"));
        };
        for (j, &m) in rip.iter().enumerate() {
            let x = self.tpt(ctx, m, 0.06, 0.85, None)?;
            mix.place(&bus, &Sig::Mono(x), t + j as f64 * 0.028, 0.26, pan);
        }
        let th = t + (hits.len() - 1) as f64 * 0.028;
        let held = self.tpt(ctx, last, 0.75, 1.0, Some(1.1))?;
        mix.place(&bus, &Sig::Mono(held), th, 0.40, pan);
        // the shake: the note above flicked in on the 32nds
        for j in (1..12).step_by(2) {
            let x = self.tpt(ctx, last + 3, 0.04, 0.7, None)?;
            mix.place(
                &bus,
                &Sig::Mono(x),
                th + f64::from(j) * ctx.show.grid.s16 / 2.0,
                0.08,
                pan,
            );
        }
        mix.place("fx", &Sig::Mono(synth::thud(0.5, 120.0, 40.0, THUD_SEED)), t, 0.20, 0.0);
        Ok(())
    }

    /// A stab on each count: the count-in chord's third nearest B4.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        for &(tc, _) in &ctx.show.grid.count[..3] {
            let tc = py_max(tc, 0.004);
            let ch = ctx.harmony.chord_at(tc);
            let pcs = ctx.harmony.chord_pcs(ch).map_err(bad)?;
            let third = *pcs
                .get(1)
                .ok_or_else(|| MusicError::new(H, format!("the chord {ch:?} has no second note for the count-in")))?;
            // min((x for x in range(60, 84) if x % 12 == third), key=lambda x: (abs(x - 71), x))
            let m = (60..84)
                .filter(|x: &i32| x.rem_euclid(12) == third)
                .min_by_key(|&x| ((x - 71).abs(), x))
                .ok_or_else(|| MusicError::new(H, format!("no note from 60 to 83 has the pitch class {third}")))?;
            let x = self.tpt(ctx, m, 0.14, 0.9, None)?;
            mix.place(&bus, &Sig::Mono(x), tc, 0.30, self.pan());
        }
        Ok(())
    }

    /// VALVE COMBO and its abilities from the run.
    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let (name, bus) = (self.name(ctx), self.bus(ctx));
        let pan = self.pan();
        let grid = &ctx.show.grid;
        let s16 = grid.s16;
        let ev = &ctx.rec.events;
        let cfg = ctx.cfg[k];
        // the landed punches' times, by the punch's place in its combo
        let mut hit_t: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
        for e in ev {
            if e.kind == "hit" && event_is(H, e, 2, k)? && event_is_str(H, e, 7, "piston")? {
                hit_t.entry(event_int(H, e, 8)?).or_default().push(e.t);
            }
        }
        let mut combo_start: BTreeMap<i64, f64> = BTreeMap::new();
        for e in ev {
            if e.kind == "combo" && event_is(H, e, 2, k)? {
                combo_start.insert(event_int(H, e, 3)?, e.t);
            }
        }
        let line = ctx.ff.music.lines.contains_key(name);
        for e in ev {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.kind == "punch" && event_is(H, e, 2, k)? {
                let (i, cn) = (event_int(H, e, 3)?, event_int(H, e, 6)?);
                let tq = ctx.q16(t);
                let m = if line {
                    // the melody: the fight file's line's 8th
                    ctx.line_at(name, tq)?.ok_or_else(|| {
                        MusicError::new(
                            H,
                            format!("{name}'s line has a rest at {tq} s, where a punch needs a note"),
                        )
                    })?
                } else {
                    let notes = self.combo_notes(ctx, combo_start.get(&cn).copied().unwrap_or(t), cn)?;
                    // notes[min(i, 2)] (the engine counts a combo's punches from 0)
                    notes[i.clamp(0, 2) as usize]
                };
                let landed = hit_t
                    .get(&i)
                    .is_some_and(|ht| ht.iter().any(|&h| h >= t - 1e-6 && h <= t + 0.2));
                let hp = ctx.hp_at(k, t) / 100.0;
                // thinning when hurt: ghosts drop
                if !landed && hp < 0.5 && h01(&[t, 4.0]) < 0.5 * dmath::pow(1.0 - hp, 1.5) {
                    continue;
                }
                let last = i == 2;
                let vel = if landed { 1.0 } else { 0.7 };
                let x = self.tpt(ctx, m, if last { 0.2 } else { 0.12 }, vel, None)?;
                let (sig, g, lead) = self.colour(ctx, &x, t, 0.8, &self.colour_args())?;
                let gain = (if landed { 0.38 } else { 0.17 }) * (if last { 1.15 } else { 1.0 }) * g;
                mix.place(&bus, &Sig::Mono(sig), tq, gain, pan);
                // self.hiss(0.07, int(t * 50) % 97)
                let seed = noise_seed(H, t * 50.0)? % 97;
                let side = if k == 0 { 1.0 } else { -1.0 };
                mix.place("fx", &Sig::Mono(Self::hiss(0.07, seed)), tq, 0.035, pan - 0.05 * side);
                if landed {
                    if lead > 0.04 {
                        // the leader: the octave below joins
                        let low = self.tpt(ctx, m - 12, 0.12, 0.8, None)?;
                        mix.place(&bus, &Sig::Mono(self.tone(ctx, &low, t, 1.0)?), tq, 0.14 * g, pan);
                    }
                    let thump = synth::thud(if last { 0.32 } else { 0.22 }, 150.0, 60.0, noise_seed(H, t * 9.0)?);
                    mix.place("fx", &Sig::Mono(thump), tq, if last { 0.16 } else { 0.09 }, -0.8 * pan);
                }
                if ctx.run_at("HA", t, k)? > 0.0 && !last {
                    // DOUBLE TONGUE: a ta-ka grace on each punch
                    let grace = self.tpt(ctx, m, 0.05, 0.7, None)?;
                    mix.place(
                        &bus,
                        &Sig::Mono(self.tone(ctx, &grace, t, 1.0)?),
                        tq + s16 / 2.0,
                        0.12,
                        pan,
                    );
                }
            } else if e.kind == "ability" && event_is(H, e, 2, k)? {
                let tq = ctx.q16(t);
                let is = |name: &str| event_is_str(H, e, 3, name);
                if is(&cfg.unlock1.name)? {
                    // FANFARE: a rising call on the key's minor triad
                    let r = self.key_root(ctx, 72);
                    let third = if ctx.harmony.scale.contains(&3) { 3 } else { 4 };
                    for (j, m) in [r, r + third, r + 7, r + 12].into_iter().enumerate() {
                        let d = if j < 3 { 0.09 } else { 0.6 };
                        let x = self.tpt(ctx, m, d, 1.0, None)?;
                        mix.place(
                            &bus,
                            &Sig::Mono(self.tone(ctx, &x, t, 1.0)?),
                            tq + j as f64 * s16,
                            0.36,
                            pan,
                        );
                    }
                    mix.place("drums", &Sig::Mono(synth::crash(1.4)), tq + 3.0 * s16, 0.10, 0.3);
                    mix.place(
                        "fx",
                        &Sig::Mono(synth::thud(0.4, 120.0, 45.0, THUD_SEED)),
                        tq,
                        0.25,
                        0.0,
                    );
                } else if is(&cfg.unlock2.name)? {
                    // DOUBLE TONGUE: ta-ka-ta-ka on the fifth
                    for j in 0..8 {
                        let vel = 0.7 + 0.3 * if j % 2 == 0 { 1.0 } else { 0.0 };
                        let x = self.tpt(ctx, self.key_root(ctx, 72) + 7, 0.05, vel, None)?;
                        let at = tq + f64::from(j) * s16 / 2.0;
                        mix.place(&bus, &Sig::Mono(self.tone(ctx, &x, t, 1.0)?), at, 0.22, pan);
                    }
                } else if is(&cfg.low.name)? {
                    // HIGH C: the charge, the high C swelling open
                    let tc = grid.beat;
                    let x = Self::swell(&self.tpt(ctx, 84, tc * 0.95, 0.8, Some(tc + 0.1))?, tc);
                    mix.place(&bus, &Sig::Mono(self.tone(ctx, &x, t, 1.2)?), tq, 0.26, pan);
                }
            } else if e.kind == "beam" && event_is(H, e, 2, k)? {
                // HIGH C fires: the scream with a shake, and the boom
                let tq = ctx.q16(t);
                let dur = 1.4;
                let x = self.tpt(ctx, 84, dur, 1.0, Some(dur + 0.3))?;
                mix.place(&bus, &Sig::Mono(self.tone(ctx, &x, t, 1.5)?), tq, 0.32, pan);
                let mut j = 0;
                while f64::from(j) * s16 / 2.0 < dur {
                    if j % 2 != 0 {
                        let x = self.tpt(ctx, 87, 0.05, 0.7, None)?;
                        let gain = 0.10 * py_min(1.0, f64::from(j) / 6.0);
                        mix.place(&bus, &Sig::Mono(x), tq + f64::from(j) * s16 / 2.0, gain, pan);
                    }
                    j += 1;
                }
                mix.place(
                    "fx",
                    &Sig::Mono(synth::thud(0.7, 120.0, 34.0, THUD_SEED)),
                    tq,
                    0.40,
                    0.0,
                );
                mix.place("drums", &Sig::Mono(synth::crash(2.0)), tq, 0.14, 0.3);
            }
        }
        Ok(())
    }

    /// The trumpet cracks: a split note squealing down, the valves clattering.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        let r = self.key_root(ctx, 72);
        for (j, m) in [r + 12, r + 8, r + 5, r, r - 4, r - 7, r - 12].into_iter().enumerate() {
            let x = self.tpt(ctx, m, 0.06, 0.8, None)?;
            let j = j as f64;
            mix.place(
                &bus,
                &Sig::Mono(x),
                t_hs + 0.07 * j,
                0.20 * (1.0 - j / 9.0),
                0.8 * self.pan(),
            );
        }
        mix.place("fx", &Sig::Mono(Self::key_clicks(5, 5)), t_hs, 0.14, 0.0);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let r = self.key_root(ctx, 72);
        for m in [r, r + 7, r + 12] {
            let x = self.tpt(ctx, m, 0.5, 1.0, Some(0.9))?;
            mix.place("solo", &Sig::Mono(x), tw, 0.20, 0.0);
        }
        Ok(())
    }

    /// The fight file's solo, open horn.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let dur = d * grid.beat * 0.9;
            let x = self.tpt(ctx, m, dur, 0.95, Some(dur + 0.3))?;
            mix.place("solo", &Sig::Mono(x), ts, 0.46, 0.0);
        }
        Ok(())
    }

    /// Its pieces fly home as a rising run of soft staccato notes up the minor pentatonic.
    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let r = self.key_root(ctx, 60);
        let run = [0, 3, 5, 7, 10, 12, 15, 19].map(|i| r + i);
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.tpt(ctx, m, 0.05, 0.6, None)?;
            mix.place(&bus, &Sig::Mono(x), tj, 0.12, 0.8 * self.pan());
            mix.place("fx", &Sig::Mono(Self::key_clicks(2, 20 + j as u64)), tj, 0.05, 0.0);
        }
        Ok(())
    }

    /// It climbs into the count on the off-beats: the last chord's tones up from C5.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let ch = ctx
            .harmony
            .chords
            .last()
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))?;
        let mut tones = ctx.harmony.chord_tones(72, 86, ch).map_err(bad)?;
        tones.truncate(4);
        for (j, m) in tones.into_iter().enumerate() {
            let j = j as f64;
            let x = self.tpt(ctx, m, 0.10, 0.6 + 0.1 * j, None)?;
            let at = t_last + j * grid.beat + grid.beat / 2.0;
            mix.place(&bus, &Sig::Mono(x), at, 0.14 + 0.03 * j, self.pan());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #29's chord table is what the fingering rule gives.
    #[test]
    fn the_fingering_fits_the_chord() {
        assert_eq!(valve_shift("Cm").unwrap(), (3, "12"));
        assert_eq!(valve_shift("Ab7").unwrap(), (2, "1"));
        assert_eq!(valve_shift("Fm").unwrap(), (5, "13"));
        assert_eq!(valve_shift("G7").unwrap(), (3, "12"));
        // B flat: the open horn
        assert_eq!(valve_shift("Bb").unwrap(), (0, "0"));
        // B: no fingering reaches the root (7 down would be needed), so the fifth, F sharp: 4 down
        assert_eq!(valve_shift("B").unwrap(), (4, "23"));
        assert!(valve_shift("H7").is_err());
    }

    #[test]
    fn the_hiss_and_the_clicks_have_their_lengths() {
        assert_eq!(TrumpetSound::hiss(0.07, 3).len(), 3360);
        let clicks = TrumpetSound::key_clicks(5, 5);
        assert_eq!(clicks.len(), 24000);
        assert!(clicks.iter().any(|&v| v != 0.0));
        // the same seed is the same clatter
        assert_eq!(clicks, TrumpetSound::key_clicks(5, 5));
        assert_ne!(clicks, TrumpetSound::key_clicks(5, 6));
    }

    #[test]
    fn the_swell_opens_up() {
        let x = vec![1.0; 4800];
        let y = TrumpetSound::swell(&x, 0.05);
        assert_eq!(y.len(), x.len());
        // it starts quiet and ends at the note itself
        assert!(y[0].abs() < 0.2);
        assert!((y[4799] - 1.0).abs() < 1e-12);
    }
}
