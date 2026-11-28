//! BARI SAX's part in the score: the `Sound` class of its reference kit module (`older/fighters/bari_sax.py`, #39
//! `audio.py`'s BARI SAX part; its notes follow the fight's key).
//!
//! Every swing one huge honk on the bass root of the bar it bottoms out in (SKANK's swings up the octave, their
//! bottoms on the off-beats), let go a 16th early, swelling to the bottom and bent like a doppler pass, the leader's
//! octave on top, a rush of air; the winch whirrs as the cable drops and the hook clanks home; a smash is a heavy
//! thud, a clank and grit (a crash cymbal when fast); GROWL a bent growl and grit; SKANK a crash; CABLE SNAP: the
//! winch strains up, the cable parts (two clanks, a whoosh, a falling whistle), the crash (thunder, a thud, the bed's
//! boom, the biggest bent honk); a rope it pressed twangs as it springs back; dropped on the ropes a low twang and
//! the slung-back whoosh; back in, a thud.
//!
//! Under the Composed arrangement (sound upgrade, 2026-11-25) the swing honk plays the written bass line's note
//! where the swing bottoms out, each honk ends the one before it (one voice), and nothing slides ("Bari sax: no
//! slide"): the pass keeps its swell without the doppler's pitch, the crash honk and GROWL are not bent, and every
//! note but the falling honk of `fall_apart` is made with a scoop of at most 1.2 %. Classic is the part above.
//!
//! The part calls the Pixel Pitch bari's tone itself ([`super::voice::bari_tone`], with a honk), fits each note to
//! its length plus 0.12 s with a 20 ms fade and levels it to an RMS of 0.16 over its first 0.12 s (`bari`), with the
//! module's cache `_vc` (two calls that round to one key get the note of whichever came first, as in Python).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::event::Event;
use bf_core::py::round_nd;
use bf_core::rng::Generator;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, bend, clank, fit, noise_seed};
use bf_sound::synth::{self, WHOOSH_RISE, WHOOSH_SEED};
use bf_sound::traits::{Bed, BedHook, Played, Sound};
use bf_sound::voice::engine2::play::Play;
use bf_sound::voice::pp;

use super::state::{RecSrc, Wreckage, wobbles};
use super::voice::{bari_tone_real, bari_tone_scooped};

/// How many variants a repeated Composed note takes turns among (sound upgrade, stage 3: no two repeats alike).
const REPEATS: u64 = 3;
/// The name the part's errors carry.
const H: &str = "BARI SAX";
/// #39's clank: `(frequency ratio, decay in seconds)`.
const CLANK_DECAYS: [(f64, f64); 3] = [(1.0, 0.09), (2.76, 0.045), (5.4, 0.02)];
/// The clank's length in seconds.
const CLANK_DUR: f64 = 0.3;
/// `A.SR`.
const SR: f64 = 48_000.0;

/// `(round(m, 3), round(dur, 3), round(vel, 2), round(honk, 2))`, compared as Python compares floats.
fn key(m: f64, dur: f64, vel: f64, honk: f64) -> [u64; 4] {
    [round_nd(m, 3), round_nd(dur, 3), round_nd(vel, 2), round_nd(honk, 2)].map(|x| (x + 0.0).to_bits())
}

/// `A.tt(n)`.
fn tt(n: usize) -> Vec<f64> {
    synth::tt(n)
}

/// `int(dur * SR)`.
fn n_of(dur: f64) -> usize {
    let n = dur * SR;
    if n > 0.0 { n as usize } else { 0 }
}

/// #39's `thunder(dur, seed, crack)`: a rolling rumble (no wobble) and a crack.
fn thunder(dur: f64, seed: u64, crack: bool) -> Vec<f64> {
    let n = n_of(dur);
    let t = tt(n);
    let mut r = Generator::new(seed);
    let noise: Vec<f64> = (0..n).map(|_| r.normal(0.0, 1.0)).collect();
    let rumble = synth::lp(&noise, 140.0, 2);
    let mut x: Vec<f64> = (0..n)
        .map(|i| rumble[i] * 3.0 * synth::clip(t[i] / 0.03, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.35)))
        .collect();
    if crack {
        let noise: Vec<f64> = (0..n).map(|_| r.normal(0.0, 1.0)).collect();
        let c = synth::bp(&noise, 1500.0, 9000.0, 2);
        for i in 0..n {
            x[i] += c[i] * dmath::exp(-t[i] / 0.03) * 0.9;
        }
    }
    x
}

/// #39's `grit(dur, seed)`: GROWL's gravel: a rough, low, crackling noise burst.
fn grit(dur: f64, seed: u64) -> Vec<f64> {
    let n = n_of(dur);
    let t = tt(n);
    let mut r = Generator::new(seed);
    let noise: Vec<f64> = (0..n).map(|_| r.normal(0.0, 1.0)).collect();
    let b = synth::bp(&noise, 120.0, 900.0, 2);
    (0..n)
        .map(|i| {
            let gate = 0.6 + 0.4 * sign(dmath::sin(2.0 * PI * 31.0 * t[i]));
            b[i] * gate * synth::clip(t[i] / 0.01, 0.0, 1.0) * dmath::exp(-t[i] / (dur * 0.4))
        })
        .collect()
}

/// `np.sign(x)`.
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// #39's `winch(dur, seed, rise)`: a geared whirr (a buzzy tone sweeping `rise` octaves) and a chain rattle.
fn winch(dur: f64, seed: u64, rise: f64) -> Vec<f64> {
    let n = n_of(dur);
    let t = tt(n);
    let f: Vec<f64> = t.iter().map(|&t| 110.0 * dmath::pow(2.0, rise * t / dur)).collect();
    let ph: Vec<f64> = dsp::cumsum(&f).iter().map(|&c| 2.0 * PI * c / SR).collect();
    let buzz: Vec<f64> = ph
        .iter()
        .map(|&p| sign(dmath::sin(p)) * 0.3 + dmath::sin(2.0 * p) * 0.2)
        .collect();
    let mut r = Generator::new(seed);
    let noise: Vec<f64> = (0..n).map(|_| r.normal(0.0, 1.0)).collect();
    let nb = synth::bp(&noise, 2500.0, 8000.0, 2);
    let bz = synth::bp(&buzz, 150.0, 3000.0, 2);
    (0..n)
        .map(|i| {
            let rattle = nb[i] * (0.5 + 0.5 * sign(dmath::sin(2.0 * PI * 34.0 * t[i])));
            let env = synth::clip(t[i] / 0.03, 0.0, 1.0) * synth::clip((dur - t[i]) / 0.03, 0.0, 1.0);
            bz[i] * env * 0.7 + rattle * env * 0.25
        })
        .collect()
}

/// #39's `doppler(x, t_bot, up, down, width)`: a pass: sharp coming in, through the pitch at `t_bot`, sliding flat
/// going away, swelling to the pass.
fn doppler(x: &[f64], t_bot: f64, up: f64, down: f64, width: f64) -> Vec<f64> {
    let n = x.len();
    let t = tt(n);
    // off = np.where(z < 0, up, down) * -np.tanh(z); rate = 2 ** (off / 12); pos = cumsum(rate) - rate[0]
    let rate: Vec<f64> = t
        .iter()
        .map(|&t| {
            let z = (t - t_bot) / width;
            let off = if z < 0.0 { up } else { down } * -dmath::np_tanh(z);
            dmath::pow(2.0, off / 12.0)
        })
        .collect();
    let c = dsp::cumsum(&rate);
    let pos: Vec<f64> = c.iter().map(|&v| v - rate[0]).collect();
    let at: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y = dsp::interp_lr(&pos, &at, x, None, Some(0.0));
    (0..n)
        .map(|i| {
            let d = (t[i] - t_bot) / (2.2 * width);
            y[i] * (0.5 + 0.5 * dmath::exp(-(d * d)))
        })
        .collect()
}

/// The key of a note in the part's cache: [`key`], and whether the note is a steady one ([`BariSaxSound::bari`]).
type NoteKey = ([u64; 4], bool, u64);

/// BARI SAX's part.
#[derive(Debug)]
pub struct BariSaxSound {
    k: usize,
    /// `_vc`: the notes made so far, by their [`NoteKey`].
    vc: Mutex<BTreeMap<NoteKey, Arc<[f64]>>>,
    /// How many times each Composed note has been asked for (its repeats take turns among [`REPEATS`] variants).
    uses: Mutex<BTreeMap<[u64; 4], u64>>,
    /// `_bc`: the rope twangs made so far, by `(round(m, 2), seed % 3, round(dur, 2))`.
    bc: Mutex<BTreeMap<[u64; 3], Arc<[f64]>>>,
}

impl BariSaxSound {
    /// The part of the fighter on side `k` (0 is the left corner).
    pub fn new(k: usize) -> BariSaxSound {
        BariSaxSound {
            k,
            vc: Mutex::new(BTreeMap::new()),
            uses: Mutex::new(BTreeMap::new()),
            bc: Mutex::new(BTreeMap::new()),
        }
    }

    /// `k0(off)`: semitones above the key's root in the C2 octave.
    fn k0(&self, ctx: &MusicCtx, off: i32) -> f64 {
        f64::from(ctx.harmony.key_root + off)
    }

    /// `bass_root(ch)`: the chord's root in the octave from a fourth under the key's root.
    fn bass_root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        ctx.harmony
            .root_above(ch, ctx.harmony.key_root - 5)
            .map_err(|e| MusicError::new(H, e.to_string()))
    }

    /// `honk_note(ch)`: the bass root, or its fifth when the root is below the bari's low C (its register's floor).
    fn honk_note(&self, ctx: &MusicCtx, ch: &str) -> Result<f64, MusicError> {
        let r = self.bass_root(ctx, ch)?;
        let floor = ctx.cfg[self.k]
            .raw
            .get("register")
            .and_then(|v| v.get(0))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| MusicError::new(H, "its roster entry has no register"))?;
        Ok(if f64::from(r) >= floor {
            f64::from(r)
        } else {
            f64::from(r + 7)
        })
    }

    /// The written bass line's note for a swing that bottoms out at `tb` (sound upgrade: under the Composed
    /// arrangement the swing honk plays the line). `None` under Classic and on a rest of the line: the bar's bass
    /// root stands then.
    fn written(&self, ctx: &MusicCtx, tb: f64) -> Result<Option<i32>, MusicError> {
        if ctx.composed_on() {
            ctx.play_line(self.name(ctx), tb)
        } else {
            Ok(None)
        }
    }

    /// `bari(m, dur, vel=0.9, honk=0.0)`: one bari note, fitted to `dur + 0.12` s with a 20 ms fade, levelled.
    ///
    /// The part passes its context: under the Composed arrangement the note is then the built bari's (Voice Engine 2,
    /// Bs2: no slide, a honk its accent, half the honk), cut and levelled the same way, its repeats taking turns; under
    /// Classic, or with no context (the breaking instrument), it is the kit's, to the sample.
    fn bari(&self, m: f64, dur: f64, vel: f64, honk: f64, ctx: Option<&MusicCtx>) -> Result<Arc<[f64]>, MusicError> {
        let steady = ctx.is_some_and(MusicCtx::composed_on);
        if let Some(ctx) = ctx
            && pp::has_len(dur + 0.16)
            && (pp::midi_hz(m) * 10.0).is_finite()
            && vel.is_finite()
            && honk.is_finite()
        {
            let play = Play {
                accent: round_nd(0.5 * honk.clamp(0.0, 1.0), 2),
                ..Play::plain(m, dur, vel)
            };
            let pp = &ctx.cfg[self.k].pp;
            let cap = round_nd(dur + 0.12, 3);
            if let Some(x) = crate::fighters::clarinet::sound::built_reed(ctx, pp, &play, cap, Some((0.12, 0.16)))? {
                return Ok(x);
            }
        }
        let variant = if steady {
            let mut uses = self
                .uses
                .lock()
                .map_err(|_| MusicError::new(H, "a note panicked earlier in this build"))?;
            let u = uses.entry(key(m, dur, vel, honk)).or_insert(0);
            *u += 1;
            (*u - 1) % REPEATS
        } else {
            0
        };
        let key = (key(m, dur, vel, honk), steady, variant);
        let mut vc = self
            .vc
            .lock()
            .map_err(|_| MusicError::new(H, "a note panicked earlier in this build"))?;
        if let Some(x) = vc.get(&key) {
            return Ok(Arc::clone(x));
        }
        if !pp::has_len(dur + 0.16) {
            return Err(MusicError::new(H, format!("a note cannot be held for {dur} s")));
        }
        let f = pp::midi_hz(m);
        if !(f * 10.0).is_finite() {
            return Err(MusicError::new(H, format!("{m} is not a pitch")));
        }
        let tone = if steady {
            bari_tone_real(f, dur, vel, honk, variant)
        } else {
            bari_tone_scooped(f, dur, vel, honk, None)
        };
        let x = fit(&pp::to_48k(&tone), dur + 0.12, 0.02)?;
        let head = ((0.12 * bf_sound::SRF) as usize).min(x.len());
        let sq: Vec<f64> = x[..head].iter().map(|&v| v * v).collect();
        let level = 0.16 / (dsp::mean(&sq).sqrt() + 1e-9);
        let x: Arc<[f64]> = x.iter().map(|&v| v * level).collect();
        vc.insert(key, Arc::clone(&x));
        Ok(x)
    }

    /// `colour(bari(...), t, drive_k)`: `(signal, gain, lead)`.
    #[allow(clippy::too_many_arguments)] // the Python call, and the context
    fn honk(
        &self,
        ctx: &MusicCtx,
        m: f64,
        dur: f64,
        vel: f64,
        honk: f64,
        t: f64,
        drive_k: f64,
    ) -> Result<(Vec<f64>, f64, f64), MusicError> {
        let x = self.bari(m, dur, vel, honk, Some(ctx))?;
        self.colour(ctx, &x, t, drive_k, &self.colour_args())
    }

    /// `rope_twang(m, seed, dur)`: a ring rope springing back from under the bari (#39's cache: keyed by the seed
    /// modulo 3).
    fn rope_twang(&self, m: f64, seed: u64, dur: f64) -> Result<Arc<[f64]>, MusicError> {
        let key = [
            (round_nd(m, 2) + 0.0).to_bits(),
            seed % 3,
            (round_nd(dur, 2) + 0.0).to_bits(),
        ];
        let mut bc = self
            .bc
            .lock()
            .map_err(|_| MusicError::new(H, "a twang panicked earlier in this build"))?;
        if let Some(x) = bc.get(&key) {
            return Ok(Arc::clone(x));
        }
        let n = n_of(dur);
        let t = tt(n);
        let f = synth::mtof(m);
        let mut x = vec![0.0; n];
        for k in 1..7 {
            let kf = f64::from(k);
            let a = 1.0 / dmath::pow(kf, 1.2);
            for i in 0..n {
                let s = dmath::sin(2.0 * PI * f * kf * t[i] * (1.0 + 0.01 * dmath::exp(-t[i] / 0.05)) + 0.3 * kf);
                x[i] += a * s * dmath::exp(-t[i] * (6.0 + 4.0 * kf));
            }
        }
        let mut r = Generator::new(seed + 31);
        let noise: Vec<f64> = (0..n).map(|_| r.normal(0.0, 1.0)).collect();
        let b = synth::bp(&noise, 300.0, 2500.0, 2);
        let y: Arc<[f64]> = (0..n)
            .map(|i| {
                let buzz = b[i] * dmath::exp(-t[i] / 0.02) * 0.4;
                dmath::np_tanh(1.5 * (x[i] + buzz)) * synth::clip(t[i] / 0.002, 0.0, 1.0)
            })
            .collect();
        bc.insert(key, Arc::clone(&y));
        Ok(y)
    }

    /// The bari's derivations (for the ropes it pressed).
    fn wreckage<'a>(&self, ctx: &'a MusicCtx) -> (Wreckage, RecSrc<'a>) {
        let src = RecSrc {
            rec: ctx.rec,
            t_fight: ctx.show.grid.t_fight,
            t_ko: ctx.show.t_ko,
            t_hs: ctx.show.t_hs,
            s16: ctx.show.grid.s16,
        };
        let w = Wreckage::new(&src, self.k, ctx.cfg[self.k]);
        (w, src)
    }
}

/// `SIDE_OF[(axis, sign)]`: a rope event's side.
fn side_of(e: &Event) -> usize {
    match (e.i(3), e.i(4)) {
        (1, 1) => 0,
        (0, 1) => 1,
        (1, -1) => 2,
        _ => 3,
    }
}

impl Sound for BariSaxSound {
    fn k(&self) -> usize {
        self.k
    }

    /// #39's mix (`bari`).
    fn bus_gain(&self) -> f64 {
        1.3
    }

    fn send(&self) -> f64 {
        0.2
    }

    /// #39's band check: its honks.
    fn band(&self) -> (f64, f64) {
        (60.0, 500.0)
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

    /// The crater's band hit (`M.bed.boom`).
    fn bed_calls(&self) -> &'static [BedHook] {
        &[BedHook::Boom]
    }

    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        self.bari(m, dur, vel, 0.0, Some(ctx))
    }

    /// A huge honk on the root and its octave.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let bus = self.bus(ctx);
        let (first, last) = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(hits) => match (hits.first(), hits.last()) {
                (Some(&a), Some(&b)) => (f64::from(a), f64::from(b)),
                _ => return Err(MusicError::new(H, "its title hits are an empty list")),
            },
            None => (self.k0(ctx, 0), self.k0(ctx, 12)),
        };
        mix.place(
            &bus,
            &Sig::Mono(self.bari(first, 0.5, 1.0, 1.0, Some(ctx))?.to_vec()),
            t,
            0.55,
            0.0,
        );
        mix.place(
            &bus,
            &Sig::Mono(self.bari(last, 0.4, 0.8, 0.6, Some(ctx))?.to_vec()),
            t + 0.02,
            0.20,
            0.0,
        );
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Played {
        let k = self.k;
        let bus = self.bus(ctx);
        let s16 = ctx.show.grid.s16;
        let t_ko = ctx.show.t_ko;
        let events = &ctx.rec.events;
        let cfg = ctx.cfg[k];
        // sound upgrade, Composed only: no slide (the honks keep their swell, their pitch stays put), and a bari
        // sax sounds one note at a time (a new swing honk ends the one before it)
        let steady = ctx.composed_on();
        let one = ctx.one_voice(self.name(ctx));
        for e in events {
            let t = e.t;
            if t > t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            match e.kind.as_str() {
                "cable" => {
                    // the cable drops (the winch), the hook clanks home
                    let tq = ctx.q16(t);
                    let whirr = winch(e.f(3) - t, noise_seed(H, t * 10.0)?, -0.8);
                    mix.place("fx", &Sig::Mono(whirr), tq, 0.10, 0.15);
                    let ring = clank(79.0, noise_seed(H, t * 13.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                    mix.place("fx", &Sig::Mono(ring), ctx.q16(e.f(3)), 0.12, 0.15);
                }
                "swing" => {
                    // the swing honk: the bass on the swings
                    let (ts0, ts1, sk) = (e.f(11), e.f(12), e.i(13) != 0);
                    let tb = ctx.q16((ts0 + ts1) / 2.0);
                    let ch = ctx.harmony.chord_at(tb + 0.01).to_string();
                    // Composed: the written bass line's note where the swing bottoms out; the kit's: the bass
                    // root of that bar
                    let m = match self.written(ctx, tb)? {
                        Some(m) => f64::from(m),
                        None => self.honk_note(ctx, &ch)?,
                    } + if sk { 12.0 } else { 0.0 };
                    let t_on = ctx.q16(ts0 - s16);
                    let dur = ts1 - t_on + 0.06;
                    let (sig, g, lead) = self.honk(ctx, m, dur, 0.95, if sk { 0.55 } else { 0.85 }, t, 0.6)?;
                    // the pass: sharp coming in, flat going away (Composed: only its swell)
                    let (up, down) = if steady {
                        (0.0, 0.0)
                    } else if sk {
                        (1.0, 1.6)
                    } else {
                        (1.6, 2.6)
                    };
                    let sig = doppler(&sig, tb - t_on, up, down, 0.09);
                    mix.note(&bus, &Sig::Mono(sig), t_on, if sk { 1.15 } else { 1.35 } * g, 0.2, one);
                    if lead > 0.05 && !sk {
                        // the leader: the octave honked on top (fuller)
                        let top = self.honk(ctx, m + 12.0, dur, 0.7, 0.4, t, 1.0)?.0;
                        let (up, down) = if steady { (0.0, 0.0) } else { (1.6, 2.6) };
                        let up8 = doppler(&top, tb - t_on, up, down, 0.09);
                        mix.note(&bus, &Sig::Mono(up8), t_on, 0.20 * g, 0.25, one);
                    }
                    let air = synth::whoosh(ts1 - ts0, 250.0, 3500.0, WHOOSH_RISE, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(air), ts0, if sk { 0.07 } else { 0.10 }, 0.1);
                }
                "smash" => {
                    // the wrecking ball connects
                    let tq = ctx.q16(t);
                    let sp = e.f(5);
                    let thud = synth::thud(0.45, 120.0, 38.0, noise_seed(H, t * 10.0)?);
                    mix.place("fx", &Sig::Mono(thud), tq, 0.22 + 0.22 * sp, 0.0);
                    let ring = clank(67.0, noise_seed(H, t * 11.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                    mix.place("fx", &Sig::Mono(ring), tq, 0.10 + 0.08 * sp, 0.1);
                    mix.place(
                        "fx",
                        &Sig::Mono(grit(0.25, noise_seed(H, t * 7.0)?)),
                        tq,
                        0.12 + 0.12 * sp,
                        0.0,
                    );
                    if sp > 0.6 {
                        mix.place("drums", &Sig::Mono(synth::crash(1.0)), tq, 0.06, 0.0);
                    }
                }
                "snap_aim" => {
                    // CABLE SNAP: the winch strains it up high
                    mix.place("fx", &Sig::Mono(winch(e.f(5) - t, 41, 1.3)), ctx.q16(t), 0.16, 0.1);
                }
                "cable_snap" => {
                    // the cable parts: a twang, the whip, a falling whistle
                    let tq = ctx.q16(t);
                    let a = clank(98.0, 5, CLANK_DUR, &CLANK_DECAYS)?;
                    let b = clank(86.0, 6, CLANK_DUR, &CLANK_DECAYS)?;
                    let both: Vec<f64> = a.iter().zip(&b).map(|(x, y)| x + y * 0.6).collect();
                    mix.place("fx", &Sig::Mono(both), tq, 0.22, 0.0);
                    let air = synth::whoosh(0.25, 3000.0, 9000.0, WHOOSH_RISE, WHOOSH_SEED);
                    mix.place("fx", &Sig::Mono(air), tq, 0.12, 0.0);
                    let fall = events
                        .iter()
                        .find(|q| q.kind == "crater" && q.who() == Some(k) && q.t >= t);
                    let tf = fall.map_or(0.4, |q| q.t - t);
                    let nf = n_of(tf);
                    let tt_ = tt(nf);
                    let fr: Vec<f64> = tt_.iter().map(|&u| 1800.0 * dmath::pow(2.0, -1.6 * u / tf)).collect();
                    let ph = dsp::cumsum(&fr);
                    let whistle: Vec<f64> = (0..nf)
                        .map(|i| dmath::sin(2.0 * PI * ph[i] / SR) * dmath::pow(tt_[i] / tf, 1.5) * 0.5)
                        .collect();
                    mix.place("fx", &Sig::Mono(whistle), tq, 0.10, 0.0);
                }
                "crater" => {
                    // the crash
                    let tq = ctx.q16(t);
                    mix.place("fx", &Sig::Mono(thunder(2.0, 77, true)), tq, 0.55, 0.0);
                    mix.place("fx", &Sig::Mono(synth::thud(0.7, 100.0, 32.0, 17)), tq, 0.40, 0.0);
                    bed.boom(ctx, mix, tq, 0.9)?;
                    let ch = ctx.harmony.chord_at(tq + 0.01).to_string();
                    let m = self.honk_note(ctx, &ch)?;
                    let (sig, g, _) = self.honk(ctx, m, 0.7, 1.0, 1.0, t, 0.5)?;
                    // the biggest honk sags a semitone and a half (Composed: unbent, its weight is growl and level)
                    let sig = if steady { sig } else { bend(&sig, -1.5, 0.3, 0.7)? };
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.70 * g, 0.0);
                    let top = self.honk(ctx, m + 12.0, 0.5, 0.7, 0.5, t, 1.0)?.0;
                    mix.place(&bus, &Sig::Mono(top), tq, 0.18 * g, 0.0);
                }
                "ability" => {
                    let tq = ctx.q16(t);
                    let name = e.s(3);
                    if name == cfg.unlock1.name && cfg.unlock1.kind.name() == "shockwave" {
                        // GROWL: a gravelly growl + grit
                        let ch = ctx.harmony.chord_at(tq).to_string();
                        let m = self.honk_note(ctx, &ch)?;
                        let (sig, g, _) = self.honk(ctx, m, 0.6, 1.0, 1.0, t, 0.8)?;
                        // bent a semitone down (Composed: unbent)
                        let sig = if steady { sig } else { bend(&sig, -1.0, 0.1, 0.6)? };
                        mix.place(&bus, &Sig::Mono(sig), tq, 0.50 * g, 0.2);
                        mix.place("fx", &Sig::Mono(grit(0.6, noise_seed(H, t * 10.0)?)), tq, 0.30, 0.0);
                    } else if name == cfg.unlock2.name && cfg.unlock2.kind.name() == "offbeat" {
                        // SKANK: a crash
                        mix.place("drums", &Sig::Mono(synth::crash(1.2)), tq, 0.08, 0.0);
                    }
                }
                _ => {}
            }
        }
        // a rope it pressed springs back: a low twang (ROPE_EV with the look's spring-backs, as Python sorts them)
        let (w, src) = self.wreckage(ctx);
        let mut ropes: Vec<(f64, usize, f64, f64, f64)> = events
            .iter()
            .filter(|e| e.kind == "rope")
            .map(|e| (e.t, side_of(e), e.f(5), e.f(6), e.f(7)))
            .collect();
        let more = wobbles(src.t_fight, src.t_ko, |t| w.over(&src, t));
        if !more.is_empty() {
            ropes.extend(more.into_iter().map(|(t, s, x, z)| (t, s, x, z, 3.2)));
            ropes.sort_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then(a.1.cmp(&b.1))
                    .then(a.2.total_cmp(&b.2))
                    .then(a.3.total_cmp(&b.3))
                    .then(a.4.total_cmp(&b.4))
            });
        }
        for (te, ..) in ropes.into_iter().filter(|r| r.4 == 3.2 && r.0 <= t_ko) {
            let ch = ctx.harmony.chord_at(te).to_string();
            let m = f64::from(self.bass_root(ctx, &ch)? + 12);
            let twang = self.rope_twang(m, noise_seed(H, te * 10.0)?, 0.45)?;
            mix.place("fx", &Sig::Mono(twang.to_vec()), ctx.q16(te), 0.20, 0.2);
        }
        // dropped on the ropes, slung back in
        for e in events {
            if e.t > t_ko {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            if e.kind == "wreck_out" {
                let ch = ctx.harmony.chord_at(e.t).to_string();
                let m = f64::from(self.bass_root(ctx, &ch)?);
                let twang = self.rope_twang(m, 7, 0.6)?;
                mix.place("fx", &Sig::Mono(twang.to_vec()), ctx.q16(e.t + 0.06), 0.30, 0.25);
                let air = synth::whoosh(e.f(8) - e.f(7), 400.0, 3000.0, false, WHOOSH_SEED);
                mix.place("fx", &Sig::Mono(air), e.t, 0.08, 0.0);
            } else if e.kind == "wreck_in" {
                let thud = synth::thud(0.35, 140.0, 50.0, noise_seed(H, e.t * 10.0)?);
                mix.place("fx", &Sig::Mono(thud), ctx.q16(e.t), 0.22, 0.0);
            }
        }
        Ok(())
    }

    /// A falling honk, keys clattering.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let t_hs = ctx.show.t_hs;
        // the instrument breaking: the kit's own note, scoop and fall, under either arrangement
        let x = self.bari(self.k0(ctx, 0), 1.0, 1.0, 0.8, None)?;
        let mut blat = bend(&x, -7.0, 0.05, 1.0)?;
        for (o, ramp) in blat.iter_mut().zip(dsp::linspace(1.0, 0.4, x.len())) {
            *o *= ramp;
        }
        mix.place(&bus, &Sig::Mono(blat), t_hs, 0.42, 0.0);
        for (j, m) in [86.0, 81.0, 91.0, 77.0, 84.0].into_iter().enumerate() {
            let ring = clank(m, 20 + j as u64, CLANK_DUR, &CLANK_DECAYS)?;
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
        mix.place(
            "solo",
            &Sig::Mono(self.bari(self.k0(ctx, 0), 0.8, 1.0, 1.0, Some(ctx))?.to_vec()),
            tw,
            0.50,
            0.0,
        );
        mix.place(
            "solo",
            &Sig::Mono(self.bari(self.k0(ctx, 12), 0.8, 0.8, 0.6, Some(ctx))?.to_vec()),
            tw + 0.02,
            0.22,
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
            let x = self.bari(f64::from(m), d * grid.beat * 0.9, 0.95, 0.7, Some(ctx))?;
            mix.place("solo", &Sig::Mono(x.to_vec()), ts, 0.55, 0.0);
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let bus = self.bus(ctx);
        let grid = &ctx.show.grid;
        let run = [0, 3, 7, 10, 12, 15];
        for (j, off) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.bari(self.k0(ctx, off), 0.10, 0.7, 0.0, Some(ctx))?.to_vec();
            mix.place(&bus, &Sig::Mono(x), tj, 0.24, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_key_is_pythons_rounding() {
        assert_eq!(key(40.0, 0.3, 0.95, 0.85), key(40.0004, 0.3004, 0.954, 0.854));
        assert_ne!(key(40.0, 0.3, 0.95, 0.85), key(40.0, 0.3, 0.95, 0.55));
    }

    #[test]
    fn a_doppler_pass_keeps_its_length() {
        let x: Vec<f64> = (0..4800).map(|i| dmath::sin(i as f64 * 0.05)).collect();
        assert_eq!(doppler(&x, 0.05, 1.6, 2.6, 0.09).len(), x.len());
    }
}
