//! ALTO SAX's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/alto_sax.py`.
//!
//! The bright lead. Every sweep plays its line (8ths) over the light's run, each burn is a staccato squeal a chord
//! tone up; OCTAVE KEY doubles the line an octave up, ALTISSIMO's burns are altissimo screams with a scoop; ENCORE
//! is a nonstop 16th run up and down the scale, with the phones as little chimes; a rising scoop announces OCTAVE
//! KEY and ALTISSIMO. The title blast is a squeal sliding up into the top note, the count-in four 8ths up the
//! line; it falls apart as one long falling note and clanking keys; the stinger, the solo, the reassembly run, and
//! a rising line into the title slam.
//!
//! It plays its own tones, not the cached notes of `M.note`: `tone(m, dur, vel, growl)` calls the rebuilt alto's
//! `alto_tone` itself, resamples it, fits it to `dur + 0.25` s and scales it by the voice's measured level. Before
//! every tone it has not made yet it replaces the old alto module's generator with
//! `default_rng(int(m) * 1000 + int(round(dur * 1000)))`, so a tone's breath depends only on its note and length
//! ("deterministic breath"), and it keeps the tones by `(int(m), round(dur, 3), round(vel, 2), growl)`.
//!
//! The kit measures the voice's level (`M.inst_gain`) when the part is made, before the score starts: a note of the
//! alto drawn from the alto generator. So does this part, in `made`, which the build calls before any bed method;
//! a bed whose alto shares that generator (the mambo's) draws its first note after it, as in the kit.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath;
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, round_half_even, round_nd};
use bf_core::rng::Generator;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    CLANK_DECAYS, ColourArgs, FIT_FADE, MusicCtx, MusicError, Thinner, bend, clank, event_int, event_is_str,
    event_number, fit, gliss, vibrato,
};
use bf_sound::synth;
use bf_sound::traits::{Bed, Played, SOUND_COLOUR_DRIVE_K, Sound};
use bf_sound::voice::pp::alto::{AltoTone, alto_tone, part_seed};
use bf_sound::voice::pp::{self};

use super::voice::{TAIL, TONE};

/// The name the part's errors carry.
const H: &str = "ALTO SAX";

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// The key `tone` keeps a tone by: `(int(m), round(dur, 3), round(vel, 2), growl)`, the floats by their bits
/// (`-0.0` as `0.0`, which Python's dict takes for the same key).
type ToneKey = (i64, u64, u64, Option<u64>);

fn key_bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

/// ALTO SAX's part.
#[derive(Debug)]
pub struct AltoSaxSound {
    k: usize,
    /// `self._c`: the tones made so far.
    tones: Mutex<BTreeMap<ToneKey, Arc<[f64]>>>,
}

impl AltoSaxSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> AltoSaxSound {
        AltoSaxSound {
            k,
            tones: Mutex::new(BTreeMap::new()),
        }
    }

    /// `self.pan`: -0.18 on the left, 0.18 on the right.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.18 } else { 0.18 }
    }

    /// `self.lo, self.hi = M.CFG[k]['register']`.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// `tone(m, dur, vel=0.85, **kw)`: one tone of the rebuilt alto, `dur + 0.25` s at 48 kHz, level-matched; the
    /// only keyword the part passes is `growl`.
    ///
    /// # Errors
    /// Where Python raises: `m`, `dur` or `vel` is not a number, `m` is below 0 (numpy refuses a negative seed), or
    /// `dur` is so far below zero that the tone has a negative number of samples or no room for its fade. And as
    /// [`MusicCtx::inst_gain`].
    pub fn tone(
        &self,
        ctx: &MusicCtx,
        m: f64,
        dur: f64,
        vel: f64,
        growl: Option<f64>,
    ) -> Result<Arc<[f64]>, MusicError> {
        for (what, v) in [("m", m), ("dur", dur), ("vel", vel), ("growl", growl.unwrap_or(0.0))] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("a tone's {what} is {v}, not a number")));
            }
        }
        // key = (int(m), round(dur, 3), round(vel, 2), tuple(sorted(kw.items())))
        let mi = m.trunc();
        let key: ToneKey = (
            mi as i64,
            key_bits(round_nd(dur, 3)),
            key_bits(round_nd(vel, 2)),
            growl.map(key_bits),
        );
        let held = |e| MusicError::new(H, format!("the part's tones cannot be read ({e})"));
        if let Some(x) = self.tones.lock().map_err(held)?.get(&key) {
            return Ok(x.clone());
        }
        // self.g0: the level `made` measured (the context keeps it)
        let g0 = ctx.inst_gain(&ctx.cfg[self.k].pp)?;
        let seed_ms = round_half_even(dur * 1000.0);
        if mi < 0.0 || mi * 1000.0 + seed_ms < 0.0 || !pp::has_len(dur + TAIL) {
            return Err(MusicError::new(
                H,
                format!("no tone of MIDI note {m} held for {dur} s (the note or its length is below zero)"),
            ));
        }
        let tone = AltoTone {
            growl: growl.unwrap_or(TONE.growl),
            ..TONE
        };
        let x = ctx.with_noise(|n| {
            // self.mod.AS._RN = np.random.default_rng(int(m) * 1000 + int(round(dur * 1000))): deterministic breath
            n.1.alto = Generator::new(part_seed(mi, dur));
            // self.mod.alto_tone(self.mod._midi_hz(int(m)), dur, vel, **kw)
            alto_tone(&mut n.1.alto, pp::midi_hz(mi), dur, vel, &tone)
        })?;
        // self.M.fit(self.B._resample(np.asarray(x, float)), dur + 0.25) * self.g0
        let y: Arc<[f64]> = fit(&pp::to_48k(&x), dur + 0.25, FIT_FADE)?
            .iter()
            .map(|v| v * g0)
            .collect();
        self.tones.lock().map_err(held)?.insert(key, y.clone());
        Ok(y)
    }

    /// `fit_reg(m, lo, hi)`: `m` moved by octaves into `lo..hi`.
    pub fn fit_reg(mut m: i32, lo: i32, hi: i32) -> i32 {
        while m > hi {
            m -= 12;
        }
        while m < lo {
            m += 12;
        }
        m
    }

    /// `line(t)`: the note of its line at `t`, in its register.
    ///
    /// # Errors
    /// As [`MusicCtx::line_at`]; and a rest in its line (Python raises comparing `None` with a number).
    pub fn line(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let (lo, hi) = self.register(ctx);
        match ctx.line_at(self.name(ctx), t)? {
            Some(m) => Ok(Self::fit_reg(m, lo, hi)),
            None => Err(MusicError::new(
                H,
                format!("its line has a rest at {t:.3} s, and its part plays the line's note there (music.lines)"),
            )),
        }
    }

    /// `up_tone(t, m)`: the next chord tone above `m` (the burn's squeal), `m + 12` when the chord has none from
    /// `m + 3` to `m + 12`.
    ///
    /// # Errors
    /// The chord at `t` is not a chord name.
    pub fn up_tone(&self, ctx: &MusicCtx, t: f64, m: i32) -> Result<i32, MusicError> {
        let ch = ctx.harmony.chord_at(t);
        let ups = ctx.harmony.chord_tones(m + 3, m + 12, ch).map_err(bad)?;
        Ok(ups.first().copied().unwrap_or(m + 12))
    }

    /// `self.colour(x, t)`.
    fn coloured(&self, ctx: &MusicCtx, x: &[f64], t: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let (sig, g, _) = self.colour(ctx, x, t, SOUND_COLOUR_DRIVE_K, &self.colour_args())?;
        Ok((sig, g))
    }

    /// `self.T.CHORDS[0]`.
    fn first_chord<'a>(&self, ctx: &'a MusicCtx) -> Result<&'a str, MusicError> {
        ctx.harmony
            .chords
            .first()
            .map(String::as_str)
            .ok_or_else(|| MusicError::new(H, "the episode has no bars"))
    }
}

impl Sound for AltoSaxSound {
    fn k(&self) -> usize {
        self.k
    }

    /// `__init__`: `self.g0 = M.inst_gain(M.CFG[k]['pp'])`, its voice's level, measured when the part is made
    /// (the music context keeps it for every later call).
    fn made(&self, ctx: &MusicCtx) -> Played {
        ctx.inst_gain(&ctx.cfg[self.k].pp).map(|_| ())
    }

    fn bus_gain(&self) -> f64 {
        1.55
    }

    fn send(&self) -> f64 {
        0.30
    }

    /// The bright lead's band.
    fn band(&self) -> (f64, f64) {
        (900.0, 3500.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 180.0,
            cut_span: 600.0,
            drive: 4.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel)`: one of its own tones.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.tone(ctx, m, dur, vel, None)
    }

    /// A bright alto squeal sliding up into the top note (the fight file's title hits).
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        // hits = self.T.TITLE_HITS.get(self.name) or (self.line(0.0),)
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(hits) if !hits.is_empty() => hits.clone(),
            _ => vec![self.line(ctx, 0.0)?],
        };
        let bus = self.bus(ctx);
        for (j, &m) in hits.iter().enumerate() {
            let jf = j as f64;
            let x = gliss(
                &self.tone(ctx, f64::from(m), 0.55 - 0.1 * jf, 1.0, Some(0.1))?,
                3.0,
                0.08,
            )?;
            mix.place(&bus, &Sig::Mono(x), t + 0.03 * jf, 0.55 - 0.08 * jf, self.pan());
        }
        Ok(())
    }

    /// A pickup into FIGHT!: four 8ths up the line.
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let bus = self.bus(ctx);
        for j in 0..4 {
            let jf = f64::from(j);
            let tj = grid.bar + 3.0 * grid.beat + jf * grid.beat / 4.0;
            let m = self.line(ctx, grid.t_fight + jf * grid.beat / 2.0)?;
            let x = self.tone(ctx, f64::from(m), grid.s16 * 0.9, 0.85, None)?;
            mix.place(&bus, &Sig::Mono(x.to_vec()), tj, 0.36, self.pan());
        }
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let (lo, hi) = self.register(ctx);
        let pan = self.pan();
        let mut th = Thinner::new(510.0 + k as f64);
        let mine = |e: &bf_core::event::Event| e.who() == Some(k);
        // hot_spans = [(e[0], e[3]) for e in M.EV if e[1] == 'altissimo' and e[2] == k]
        let mut hot_spans = Vec::new();
        for e in ctx.rec.events.iter().filter(|e| e.kind == "altissimo" && mine(e)) {
            hot_spans.push((e.t, event_number(H, e, 3)?));
        }
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if !mine(e) {
                continue;
            }
            match e.kind.as_str() {
                "spot" => {
                    // the sweep plays the line over the light's run (8ths)
                    let t0 = ctx.q16(event_number(H, e, 8)?);
                    let t1 = event_number(H, e, 9)?;
                    let sk = event_int(H, e, 11)?;
                    let up = if sk == 1 { 12 } else { 0 };
                    // n8 = max(1, int(round((t1 - t0) / (T.BEAT / 2))))
                    let n8 = (round_half_even((t1 - t0) / (beat / 2.0)) as i64).max(1);
                    for j in 0..n8 {
                        let tj = t0 + j as f64 * beat / 2.0;
                        // th.keep(round(tj / T.S16) + 7 * sk, M.hp_at(k, tj) / 100.0)
                        let slot = round_half_even(tj / s16) + 7.0 * sk as f64;
                        if !th.keep(slot, ctx.hp_at(k, tj) / 100.0) {
                            continue;
                        }
                        let m = Self::fit_reg(self.line(ctx, tj)? + up, lo, hi + up);
                        let x = self.tone(ctx, f64::from(m), beat * 0.45, 0.85, None)?;
                        let (sig, g) = self.coloured(ctx, &x, tj)?;
                        let gain = if sk == 0 { 0.40 } else { 0.22 };
                        let side = if sk != 0 { 0.15 } else { 0.0 };
                        mix.place(&bus, &Sig::Mono(sig), tj, gain * g, pan + side);
                    }
                }
                "burn" => {
                    // each burn: a staccato squeal a chord tone up
                    let tq = ctx.q16(t);
                    let hot = hot_spans.iter().any(|&(a, b)| a <= t && t < b);
                    let m = self.up_tone(ctx, tq, self.line(ctx, tq)?)?;
                    let (x, g) = if hot {
                        let scream = self.tone(ctx, f64::from(m + 12), s16 * 1.6, 1.0, Some(0.15))?;
                        (gliss(&scream, 1.5, 0.05)?, 0.34)
                    } else {
                        (self.tone(ctx, f64::from(m), s16 * 0.8, 0.9, None)?.to_vec(), 0.24)
                    };
                    let (sig, gg) = self.coloured(ctx, &x, t)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, g * gg, pan * 0.6);
                }
                "encore" => {
                    // ENCORE: a nonstop 16th run, up and down the scale
                    let t0 = ctx.q16(t);
                    let t1 = event_number(H, e, 3)?;
                    // n = int((t1 - t0) / T.S16)
                    let n = ((t1 - t0) / s16).trunc() as i64;
                    for j in 0..n {
                        let tj = t0 + j as f64 * s16;
                        if tj > ctx.show.t_ko {
                            break;
                        }
                        let ch = ctx.harmony.chord_at(tj);
                        let sc = ctx.harmony.scale_notes(lo + 7, hi, Some(ch)).map_err(bad)?;
                        let len = sc.len() as i64;
                        if len < 2 {
                            // Python: `j % (2 * len(sc) - 2)` divides by zero, or `sc[0]` of an empty list
                            return Err(MusicError::new(
                                H,
                                format!(
                                    "its register holds {len} note(s) of the scale from {} up: ENCORE's run needs two",
                                    lo + 7
                                ),
                            ));
                        }
                        let ph = imod_py(j, 2 * len - 2);
                        let m = if ph < len {
                            sc[ph as usize]
                        } else {
                            sc[(2 * len - 2 - ph) as usize]
                        };
                        let x = self.tone(ctx, f64::from(m), s16 * 0.95, 0.9, None)?;
                        let (sig, g) = self.coloured(ctx, &x, tj)?;
                        mix.place(&bus, &Sig::Mono(sig), tj, 0.30 * g, pan * dmath::cos(j as f64 * 0.4));
                    }
                }
                "phones" => {
                    // the phones lighting up: a little chime sparkle
                    let tq = ctx.q16(t);
                    // M.A.chime(96 + (int(t * 8) % 3) * 2, 0.25)
                    let m = 96 + imod_py((t * 8.0).trunc() as i64, 3) * 2;
                    let x = synth::chime(m as f64, 0.25);
                    mix.place("fx", &Sig::Mono(x), tq, 0.035, 0.3 * dmath::sin(t * 5.0));
                }
                "ability" if event_is_str(H, e, 4, "octave")? || event_is_str(H, e, 4, "altissimo")? => {
                    // a rising scoop announcing it
                    let tq = ctx.q16(t);
                    let m = Self::fit_reg(self.line(ctx, tq)? + 12, lo, hi + 7);
                    let x = gliss(&self.tone(ctx, f64::from(m), beat * 0.9, 1.0, None)?, 5.0, 0.12)?;
                    mix.place(&bus, &Sig::Mono(x), tq, 0.40, pan);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let m = self.line(ctx, t0)?;
        let x = bend(
            &self.tone(ctx, f64::from(m + 12), 1.2, 1.0, Some(0.3))?,
            -12.0,
            0.08,
            1.1,
        )?;
        mix.place(&self.bus(ctx), &Sig::Mono(x), t0, 0.45, self.pan());
        for j in 0..4u64 {
            let jf = j as f64;
            // M.clank(88 - 3 * j, seed=j, dur=0.25)
            let key = clank(88.0 - 3.0 * jf, j, 0.25, &CLANK_DECAYS)?;
            mix.place("fx", &Sig::Mono(key), t0 + 0.06 + 0.07 * jf, 0.05, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let (lo, hi) = self.register(ctx);
        let root = ctx.harmony.root_above(self.first_chord(ctx)?, lo + 12).map_err(bad)?;
        let m = Self::fit_reg(root, lo, hi);
        let x = gliss(&self.tone(ctx, f64::from(m + 12), 0.9, 1.0, None)?, 4.0, 0.1)?;
        mix.place("solo", &Sig::Mono(x), tw, 0.55, 0.0);
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let tone = self.tone(ctx, f64::from(m), py_max(0.1, d * grid.beat * 0.95), 0.95, None)?;
            let x = if d >= 1.5 {
                vibrato(&tone, 0.3, 5.4, 0.2)?
            } else {
                tone.to_vec()
            };
            mix.place("solo", &Sig::Mono(x), ts, 0.60, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (lo, hi) = self.register(ctx);
        let sc = ctx
            .harmony
            .scale_notes(lo + 5, hi, Some(self.first_chord(ctx)?))
            .map_err(bad)?;
        for (j, &m) in sc.iter().take(8).enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / 8.0);
            let x = self.tone(ctx, f64::from(m), 0.12, 0.8, None)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), tj, 0.30, 0.0);
        }
        Ok(())
    }

    /// A rising lead line into the title slam: the scale climbing in 8ths to the top tonic on frame 0.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let bus = self.bus(ctx);
        let (lo, hi) = self.register(ctx);
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let root = ctx.harmony.root_above(self.first_chord(ctx)?, lo + 12).map_err(bad)?;
        let tgt = Self::fit_reg(root, lo + 10, hi);
        let ch = ctx.harmony.chord_at(t_last);
        // [m for m in T.scale_notes(tgt - 14, tgt - 1, T.chord_at(t_last))][-8:]
        let sc = ctx.harmony.scale_notes(tgt - 14, tgt - 1, Some(ch)).map_err(bad)?;
        let sc = &sc[sc.len().saturating_sub(8)..];
        for (j, &m) in sc.iter().enumerate() {
            let x = self.tone(ctx, f64::from(m), grid.beat * 0.45, 0.9, None)?;
            mix.place(
                &bus,
                &Sig::Mono(x.to_vec()),
                t_last + j as f64 * grid.beat / 2.0,
                0.34,
                self.pan(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AltoSaxSound;

    #[test]
    fn a_note_is_moved_by_octaves_into_the_register() {
        assert_eq!(AltoSaxSound::fit_reg(64, 49, 81), 64);
        assert_eq!(AltoSaxSound::fit_reg(88, 49, 81), 76);
        assert_eq!(AltoSaxSound::fit_reg(40, 49, 81), 52);
        // down first, then up: a note above a range narrower than an octave ends at or above its low end
        assert_eq!(AltoSaxSound::fit_reg(95, 59, 69), 59);
        assert_eq!(AltoSaxSound::fit_reg(94, 59, 69), 70);
    }
}
