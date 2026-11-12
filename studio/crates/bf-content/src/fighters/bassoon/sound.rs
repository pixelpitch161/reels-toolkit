//! BASSOON's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/bassoon.py` (#39's `audio.py`
//! part; its notes follow the fight's key).
//!
//! A staccato dot per hop landing from its line (8 a bar; STACCATO's 16th landings take the passing note), a
//! missed stomp ghosted, a backstab accented with a grace up the chord, the leader a sixth under; BOCAL HOOK a comic
//! blat sliding up and the crook's clank; MOUNTAIN KING is Grieg's theme in the key, one note per hop, so it
//! accelerates with the hops. While the enemy has it OUT OF TUNE every note plays flat by up to a quarter-tone
//! against a beating sour twin, the Grieg quote included.
//!
//! The part does not play through `pp_bridge.voice` (the roster's `pp`): it calls the Pixel Pitch bassoon's double
//! reed itself ([`super::voice::bassoon_reed`], with a growl for its blats), fits each note to its length plus
//! 0.09 s with a 20 ms fade and levels it to an RMS of 0.16 over its first 0.12 s (`bsn`). So none of its notes is
//! one of the build's "first-time notes", and it draws nothing from the shared noise.
//!
//! `bsn` keeps the kit's cache `_vc`, keyed by the rounded pitch, length, velocity and growl: two calls that round
//! to one key get the note of whichever came first, as in Python.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, round_half_even, round_nd};
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    ColourArgs, MusicCtx, MusicError, bend, clank, event_int, event_is_str, event_number, fit, gliss, noise_seed,
    thin_37,
};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;

use super::voice::bassoon_reed;

/// The name the part's errors carry.
const H: &str = "BASSOON";

/// `MK_THEME`: Grieg, In the Hall of the Mountain King (1875, public domain): the theme's opening, as semitones
/// above the key's tonic in the bassoon's tenor register: one note per hop.
pub const MK_THEME: [i32; 20] = [
    0, 2, 3, 5, 7, 3, 7, // E F# G A B G B
    6, 2, 6, // A# F# A#
    5, 1, 5, // A F A
    0, 2, 3, 5, 7, 3, 7, // E F# G A B G B
];
/// `MK_BASE`: above the key's root in the C2 octave (E2 40 -> E4 64).
pub const MK_BASE: i32 = 24;

/// #39's clank: a key or the crook hitting something, 0.3 s: `(frequency ratio, decay in seconds)`.
const CLANK_DECAYS: [(f64, f64); 3] = [(1.0, 0.09), (2.76, 0.045), (5.4, 0.02)];
/// The clank's length in seconds (`n = int(0.3 * A.SR)`).
const CLANK_DUR: f64 = 0.3;

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// A rest where the part needs a note of its line (Python then adds a number to `None`).
fn rest(name: &str, t: f64) -> MusicError {
    MusicError::new(
        H,
        format!("{name}'s line has a rest at {t} s, where a hop lands: give the line a note on every 8th"),
    )
}

/// The cache key of `bsn`: `(round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))`, compared as Python
/// compares floats in a tuple (`-0.0` is `0.0`).
fn key(m: f64, dur: f64, vel: f64, growl: f64) -> [u64; 4] {
    [round_nd(m, 3), round_nd(dur, 3), round_nd(vel, 2), round_nd(growl, 2)].map(|x| (x + 0.0).to_bits())
}

/// BASSOON's part.
#[derive(Debug)]
pub struct BassoonSound {
    k: usize,
    /// `_vc`: the notes made so far, by [`key`].
    vc: Mutex<BTreeMap<[u64; 4], Arc<[f64]>>>,
}

impl BassoonSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> BassoonSound {
        BassoonSound {
            k,
            vc: Mutex::new(BTreeMap::new()),
        }
    }

    /// `k0(off)`: a pitch in the key: semitones above the tonic in the C2 octave (#39, E minor: 40 = E2).
    pub fn k0(&self, ctx: &MusicCtx, off: i32) -> f64 {
        f64::from(ctx.harmony.key_root + off)
    }

    /// `bsn(m, dur, vel=0.85, growl=0.0)`: one bassoon note (staccato when short), `dur` seconds: the reed at
    /// 48 kHz, fitted to `dur + 0.09` s with a 20 ms fade, levelled to an RMS of 0.16 over its first 0.12 s.
    ///
    /// # Errors
    /// Where Python raises: a `dur` the reed has no note for (below -0.14 s, or not a number), or one `fit` has no
    /// room for.
    pub fn bsn(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64, growl: f64) -> Result<Arc<[f64]>, MusicError> {
        let _ = ctx;
        let key = key(m, dur, vel, growl);
        let mut vc = self
            .vc
            .lock()
            .map_err(|_| MusicError::new(H, "a note panicked earlier in this build"))?;
        if let Some(x) = vc.get(&key) {
            return Ok(Arc::clone(x));
        }
        if !pp::has_len(dur + 0.14) {
            return Err(MusicError::new(H, format!("a note cannot be held for {dur} s")));
        }
        let f = pp::midi_hz(m);
        if !(f * 10.0).is_finite() {
            return Err(MusicError::new(H, format!("{m} is not a pitch")));
        }
        let x = fit(&pp::to_48k(&bassoon_reed(f, dur, vel, growl)), dur + 0.09, 0.02)?;
        // x * (0.16 / (np.sqrt((x[:int(0.12 * M.SR)] ** 2).mean()) + 1e-9))
        let head = ((0.12 * bf_sound::SRF) as usize).min(x.len());
        let sq: Vec<f64> = x[..head].iter().map(|&v| v * v).collect();
        let level = 0.16 / (dsp::mean(&sq).sqrt() + 1e-9);
        let x: Arc<[f64]> = x.iter().map(|&v| v * level).collect();
        vc.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// `sour(m, dur, vel, t, growl=0.0)`: the bassoon as the run has it at `t`: while OUT OF TUNE flat by up to a
    /// quarter-tone with a beating sour twin (nothing detunes it in #39, so there it is the plain note).
    fn sour(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64, t: f64, growl: f64) -> Result<Vec<f64>, MusicError> {
        let dep = self.detune(ctx, t)?;
        if dep <= 0.0 {
            return Ok(self.bsn(ctx, m, dur, vel, growl)?.to_vec());
        }
        let mut x = self.bsn(ctx, m - 0.5 * dep, dur, vel, growl)?.to_vec();
        if dep > 0.25 {
            // x = x + 0.32 * dep * self.bsn(m - 0.5 * dep - 0.22, dur, vel * 0.9, growl)
            let twin = self.bsn(ctx, m - 0.5 * dep - 0.22, dur, vel * 0.9, growl)?;
            if twin.len() != x.len() {
                return Err(MusicError::new(
                    H,
                    format!("a note and its sour twin have {} and {} samples", x.len(), twin.len()),
                ));
            }
            let g = 0.32 * dep;
            for (o, &v) in x.iter_mut().zip(twin.iter()) {
                *o += g * v;
            }
        }
        Ok(x)
    }

    /// `note(m, dur, vel, t, drive_k=1.0, growl=0.0)`: `self.colour(self.sour(m, dur, vel, t, growl), t, drive_k)`:
    /// `(signal, gain, lead)`.
    fn note(
        &self,
        ctx: &MusicCtx,
        m: f64,
        dur: f64,
        vel: f64,
        t: f64,
        drive_k: f64,
        growl: f64,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let x = self.sour(ctx, m, dur, vel, t, growl)?;
        self.colour(ctx, &x, t, drive_k, &self.colour_args())
    }

    /// `clank(m, seed)`: #39's clank: a key or the crook hitting something, a short metallic ring.
    fn clank(&self, m: f64, seed: u64) -> Result<Vec<f64>, MusicError> {
        clank(m, seed, CLANK_DUR, &CLANK_DECAYS)
    }
}

impl Sound for BassoonSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #39's mix.
    fn bus_gain(&self) -> f64 {
        1.5
    }

    fn send(&self) -> f64 {
        0.25
    }

    /// #39's band check: the staccato line's fundamentals.
    fn band(&self) -> (f64, f64) {
        (300.0, 1400.0)
    }

    /// #39's `colour()`.
    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 40.0,
            cut_span: 200.0,
            drive: 4.0,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.bsn(m, dur, vel)` (the OBOE plays its victim's sour notes through this).
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.bsn(ctx, m, dur, vel, 0.0)
    }

    /// A comic low blat sliding up, then a dot an octave up.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        // hits = self.T.TITLE_HITS.get(self.name, (self.k0(0), self.k0(12)))
        let (first, last) = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(hits) => match (hits.first(), hits.last()) {
                (Some(&a), Some(&b)) => (f64::from(a), f64::from(b)),
                _ => return Err(MusicError::new(H, "its title hits are an empty list")),
            },
            None => (self.k0(ctx, 0), self.k0(ctx, 12)),
        };
        let blat = gliss(&self.bsn(ctx, first, 0.35, 1.0, 0.25)?, -3.0, 0.08)?;
        mix.place(&bus, &Sig::Mono(blat), t, 0.48, 0.0);
        let dot = self.bsn(ctx, last, 0.12, 0.9, 0.0)?.to_vec();
        mix.place(&bus, &Sig::Mono(dot), t + 0.20, 0.30, 0.0);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let name = self.name(ctx);
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let t_end = ctx.show.t_ko + 0.01;
        let events = &ctx.rec.events;
        let mut dropped = false;
        let mut last_slot: Option<f64> = None;
        let mut last_played = -9.0;
        let mut mk_i = 0usize;
        // mk_hops = [e for e in M.EV if e[1] == 'hop' and e[2] == k and e[8] == 2]
        let mut mk_hops = Vec::new();
        for e in events.iter().filter(|e| e.kind == "hop" && e.who() == Some(k)) {
            if event_int(H, e, 8)? == 2 {
                mk_hops.push(e.t);
            }
        }
        // a staccato dot per hop landing
        for e in events {
            let t = e.t;
            if t > t_end {
                break;
            }
            if e.kind != "stomp" || e.who() != Some(k) {
                continue;
            }
            let hit = event_int(H, e, 5)? != 0;
            let back = event_int(H, e, 6)? != 0;
            let kind = event_int(H, e, 8)?;
            let tq = ctx.q16(t);
            // slot = round(tq / S16)
            let slot = round_half_even(tq / s16);
            if kind == 2 {
                // MOUNTAIN KING: Grieg's theme, one note per hop
                let m = self.k0(ctx, MK_BASE + MK_THEME[mk_i.min(MK_THEME.len() - 1)]);
                let nxt = match mk_hops.get(mk_i + 1) {
                    Some(&t_hop) => t_hop,
                    None => t + beat / 2.0,
                };
                mk_i += 1;
                let dur = synth::clip((nxt - t) * 0.7, 0.06, 0.2);
                let (sig, g, _) = self.note(ctx, m, dur, 0.95, t, 0.5, 0.0)?;
                mix.place(&bus, &Sig::Mono(sig), t, 0.50 * g, -0.2);
                let low = self.note(ctx, m - 12.0, dur, 0.7, t, 0.5, 0.0)?.0;
                mix.place(&bus, &Sig::Mono(low), t, 0.18 * g, -0.2);
                continue;
            }
            if last_slot == Some(slot) {
                continue;
            }
            last_slot = Some(slot);
            let hp = ctx.hp_at(k, t) / 100.0;
            // sparser when hurt
            dropped = thin_37(slot, 3.0, hp, dropped) && tq - last_played < 0.5;
            if dropped {
                continue;
            }
            last_played = tq;
            let mut m = ctx.line_at(name, tq)?.ok_or_else(|| rest(name, tq))?;
            let chord = ctx.harmony.chord_at(tq);
            if (slot as i64).rem_euclid(2) == 1 {
                // STACCATO's 16th landings: a passing note
                let t2 = tq + s16;
                let nxt = ctx.line_at(name, t2)?.ok_or_else(|| rest(name, t2))?;
                let pas: Vec<i32> = ctx
                    .harmony
                    .scale_notes(m.min(nxt), m.max(nxt), Some(chord))
                    .map_err(bad)?
                    .into_iter()
                    .filter(|&x| x != m && x != nxt)
                    .collect();
                m = if pas.is_empty() { nxt } else { pas[pas.len() / 2] };
            }
            let vel = if back {
                0.92
            } else if hit {
                0.8
            } else {
                0.6
            };
            let (sig, g, lead) = self.note(ctx, f64::from(m), 0.085, vel, t, 1.0, 0.0)?;
            mix.place(&bus, &Sig::Mono(sig), tq, if hit { 0.46 } else { 0.30 } * g, -0.22);
            if lead > 0.05 && hit {
                // the leader: a sixth under (fuller)
                let under = ctx.harmony.chord_tones(m - 9, m - 5, chord).map_err(bad)?;
                if let Some(&u) = under.first() {
                    let sig = self.note(ctx, f64::from(u), 0.085, 0.6, t, 1.0, 0.0)?.0;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.16 * g, -0.3);
                }
            }
            if back {
                // a backstab: a grace up the chord, the stab
                let up = ctx.harmony.chord_tones(m + 3, m + 9, chord).map_err(bad)?;
                if let Some(&u) = up.first() {
                    let sig = self.note(ctx, f64::from(u), 0.06, 0.9, t, 1.0, 0.0)?.0;
                    mix.place(&bus, &Sig::Mono(sig), tq + 0.035, 0.26 * g, -0.15);
                }
                let air = synth::whoosh(0.07, 2000.0, 9000.0, WHOOSH_RISE, WHOOSH_SEED);
                mix.place("fx", &Sig::Mono(air), tq - 0.03, 0.05, 0.0);
            }
        }
        let cfg = ctx.cfg[k];
        for e in events {
            let t = e.t;
            if t > t_end {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "hook_open" => {
                    // BOCAL HOOK: a comic blat sliding up as it swings out
                    let (sig, g, _) = self.note(ctx, self.k0(ctx, 6), 0.16, 1.0, t, 1.0, 0.3)?;
                    mix.place(&bus, &Sig::Mono(gliss(&sig, 4.0, 0.12)?), ctx.q16(t), 0.40 * g, -0.2);
                }
                "hook" => {
                    let tq = ctx.q16(t);
                    let landed = event_number(H, e, 3)? != 0.0;
                    // self.clank(91, int(t * 7))
                    let ring = self.clank(91.0, noise_seed(H, t * 7.0)?)?;
                    mix.place("fx", &Sig::Mono(ring), tq, if landed { 0.16 } else { 0.08 }, 0.1);
                    if landed {
                        let air = synth::whoosh(0.3, 800.0, 6000.0, WHOOSH_RISE, WHOOSH_SEED);
                        mix.place("fx", &Sig::Mono(air), tq, 0.10, 0.0);
                    }
                }
                "ability" => {
                    let tq = ctx.q16(t);
                    if event_is_str(H, e, 3, &cfg.unlock1.name)? {
                        // STACCATO: a quick staccato run up
                        for (j, off) in [24, 27, 31, 36].into_iter().enumerate() {
                            let sig = self.note(ctx, self.k0(ctx, off), 0.06, 0.9, t, 1.0, 0.0)?.0;
                            mix.place(&bus, &Sig::Mono(sig), tq + j as f64 * s16, 0.30, -0.2);
                        }
                    } else if event_is_str(H, e, 3, &cfg.low.name)? {
                        // MOUNTAIN KING: a crash as the dance starts
                        mix.place("drums", &Sig::Mono(synth::crash(1.6)), tq, 0.08, 0.0);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// A falling blat, keys clattering.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        let x = self.bsn(ctx, self.k0(ctx, 12), 1.0, 1.0, 0.4)?;
        // M.bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x))
        let mut blat = bend(&x, -9.0, 0.05, 1.0)?;
        for (o, ramp) in blat.iter_mut().zip(dsp::linspace(1.0, 0.4, x.len())) {
            *o *= ramp;
        }
        mix.place(&bus, &Sig::Mono(blat), t_hs, 0.42, 0.0);
        for (j, m) in [86.0, 81.0, 91.0, 77.0, 84.0].into_iter().enumerate() {
            let ring = self.clank(m, 20 + j as u64)?;
            // pan=(j - 2) / 5
            mix.place(
                "fx",
                &Sig::Mono(ring),
                t_hs + 0.08 * j as f64,
                0.07,
                (j as f64 - 2.0) / 5.0,
            );
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let s16 = ctx.show.grid.s16;
        for (j, off) in [24, 27, 31, 36].into_iter().enumerate() {
            let x = self.bsn(ctx, self.k0(ctx, off), 0.08, 1.0, 0.0)?.to_vec();
            mix.place("solo", &Sig::Mono(x), tw + j as f64 * s16, 0.42, 0.0);
        }
        Ok(())
    }

    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let x = self.bsn(ctx, f64::from(m), py_max(0.07, d * grid.beat * 0.6), 0.95, 0.0)?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.52, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let run = [24, 27, 31, 34, 36, 39];
        for (j, off) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.bsn(ctx, self.k0(ctx, off), 0.08, 0.7, 0.0)?.to_vec();
            mix.place(&bus, &Sig::Mono(x), tj, 0.24, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grieg's opening is twenty notes, and the dance has twenty hops.
    #[test]
    fn the_mountain_king_theme_has_a_note_per_hop() {
        assert_eq!(MK_THEME.len(), 20);
        assert_eq!(&MK_THEME[..7], &MK_THEME[13..]);
        assert_eq!(MK_BASE + MK_THEME[4], 31);
    }

    /// The cache key rounds as Python's `round` and takes minus zero for zero.
    #[test]
    fn the_cache_key_is_pythons_rounding() {
        assert_eq!(key(52.0, 0.085, 0.8, 0.0), key(52.0004, 0.0854, 0.804, -0.0));
        assert_ne!(key(52.0, 0.085, 0.8, 0.0), key(52.0, 0.085, 0.8, 0.3));
        assert_ne!(key(52.0, 0.085, 0.8, 0.0), key(51.78, 0.085, 0.8, 0.0));
        // round(0.125, 2) is 0.12 in Python (half to even on the decimal value of the float)
        assert_eq!(key(1.0, 1.0, 0.125, 0.0), key(1.0, 1.0, 0.12, 0.0));
    }
}
