//! SOUSAPHONE's part in the score: the `Sound` class of `projects/ballfight/kit/fighters/sousaphone.py` (#53).
//!
//! Its footfalls are the bass line: the marching 8ths of its line while it marches a set (a low thud under each),
//! a note on every beat while it marks time between sets, a big blat on the root with its octave on every bell
//! blast; a landed trample adds a low thud, a trampled basket a higher one; HORN FLASH is a rip up into a blat;
//! DOUBLE TIME adds 16th chromatic approach notes; COMPANY FRONT plays the trio of The Stars and Stripes Forever
//! (Sousa, 1896, public domain) in low unison octaves, a half note a beat, in the fight's key.
//!
//! Every note is the tone of its own voice (`voice::sousa_tone`) with the part's own `blat`, resampled and brought
//! to the series' level (`M.inst_gain`), and kept by `(note, length, velocity, blat)` as the Python part keeps
//! them: two calls whose length differs only past the third decimal share the first one's samples.
//!
//! What the Python part reads off its look (`M.P.LOOKS[k].SETS`, `.FRONT`) it gets here from the recording's
//! events (`state.rs`).
//!
//! `prepare` (COMPANY FRONT under the enemy's desperation move: the enemy's melody gives way to the config's
//! `OBBLIGATO` line, through `M.line_override`) is opt-in in the kit: it does nothing unless the episode sets
//! `OBBLIGATO` for the enemy. No kit config does, and a fight file has no field for one, so the part is made
//! without an obbligato and `prepare` registers nothing; with one ([`SousaphoneSound::with_obbligato`]) it
//! registers the kit's windows.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::event::{Event, Value};
use bf_core::harmony::HarmonyError;
use bf_core::py::{imod_py, py_max, py_min, round_half_even, round_nd};
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{ColourArgs, MusicCtx, MusicError, Thinner, bend, noise_seed};
use bf_sound::synth;
use bf_sound::traits::{Bed, Played, Sound};
use bf_sound::voice::pp;

use super::state;
use super::voice::{VOICE_BLAT, sousa_tone};

/// The name the part's errors carry.
const H: &str = "SOUSAPHONE";

/// `TRIO`: The Stars and Stripes Forever, trio (Sousa, 1896, public domain), bars 39 to 46 of the Mutopia
/// Project's edition of the John Church piano score (in A-flat): `(half-note beat, semitones above the trio's
/// tonic, half-note beats)`.
pub const TRIO: [(f64, i32, f64); 18] = [
    (0.0, 7, 1.0),
    (1.0, 5, 0.5),
    (1.5, 4, 0.5),
    (2.0, 4, 1.0),
    (3.0, 3, 0.5),
    (3.5, 4, 0.5),
    (4.0, 4, 3.0),
    (7.0, 3, 0.5),
    (7.5, 4, 0.5),
    (8.0, 4, 1.0),
    (9.0, 3, 0.5),
    (9.5, 4, 0.5),
    (10.0, 7, 1.0),
    (11.0, 4, 0.75),
    (11.75, 7, 0.25),
    (12.0, 5, 2.0),
    (14.0, 2, 1.0),
    (15.0, 2, 1.0),
];

/// A line the enemy plays instead of its own: `(beat from the window's zero, midi note or a rest, beats)`.
pub type Obbligato = Vec<(f64, Option<i32>, f64)>;

/// The key `tone` caches by: `(int(m), round(dur, 3), round(vel, 2), round(blat, 2))`, the floats by their bits.
type ToneKey = (i32, u64, u64, u64);

fn bits(x: f64) -> u64 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

/// Python's `int(round(x))` for a count of beats or 16ths.
fn slot(x: f64) -> Result<i64, MusicError> {
    let r = round_half_even(x);
    if r.abs() < 9.0e15 {
        Ok(r as i64)
    } else {
        Err(MusicError::new(H, format!("{x} is not a place on the grid")))
    }
}

/// `fit_reg(m, lo, hi)`: `m` moved by octaves into `lo..=hi` (down first, then up, as the Python loops do).
pub fn fit_reg(mut m: i32, lo: i32, hi: i32) -> i32 {
    while m > hi {
        m -= 12;
    }
    while m < lo {
        m += 12;
    }
    m
}

/// `prepare`'s windows: for each of fighter `k`'s COMPANY FRONTs (from its start on the 16th grid to its end),
/// every time the enemy fires its low ability `low` inside it, `(q16(that time), min(the wall's end, T_KO), the
/// wall's start)`: what `M.line_override(enemy, t0, t1, tz, ob)` is called with.
pub fn obbligato_windows(
    events: &[Event],
    k: usize,
    low: &str,
    t_ko: f64,
    q16: impl Fn(f64) -> f64,
) -> Vec<(f64, f64, f64)> {
    let enemy = 1 - k;
    let mut out = Vec::new();
    for fr in state::fronts(events, k) {
        let (t0, t1) = (q16(fr.tf), fr.t1);
        for f in events {
            let fires = f.kind == "ability"
                && f.who() == Some(enemy)
                && matches!(f.fields.get(1), Some(Value::Str(name)) if name == low);
            if fires && t0 - 1e-6 <= f.t && f.t < t1 {
                out.push((q16(f.t), py_min(t1, t_ko), t0));
            }
        }
    }
    out
}

/// SOUSAPHONE's part.
#[derive(Debug)]
pub struct SousaphoneSound {
    k: usize,
    /// `T.OBBLIGATO.get(M.NAMES[self.enemy])`: the line the enemy's melody gives way to; `None` when the episode
    /// sets none.
    obbligato: Option<Obbligato>,
    /// `self._c`: the tones made so far.
    tones: Mutex<BTreeMap<ToneKey, Arc<[f64]>>>,
}

impl SousaphoneSound {
    /// The part of the fighter on side `k` (0 is the left corner), with no obbligato for the enemy.
    pub fn new(k: usize) -> SousaphoneSound {
        SousaphoneSound {
            k,
            obbligato: None,
            tones: Mutex::new(BTreeMap::new()),
        }
    }

    /// The part with the episode's `OBBLIGATO` line for the enemy (the kit's opt-in).
    pub fn with_obbligato(k: usize, obbligato: Obbligato) -> SousaphoneSound {
        SousaphoneSound {
            obbligato: Some(obbligato),
            ..SousaphoneSound::new(k)
        }
    }

    /// `self.pan`: a little to its own side.
    fn pan(&self) -> f64 {
        if self.k == 0 { -0.1 } else { 0.1 }
    }

    /// `self.lo, self.hi = M.CFG[k]['register']`.
    fn register(&self, ctx: &MusicCtx) -> (i32, i32) {
        ctx.cfg[self.k].register
    }

    /// `tone(m, dur, vel=0.9, blat=0.0)`: one note of its voice with the accent `blat`, at 48 kHz and at the
    /// series' level: `_resample(sousa_tone(midi_hz(int(m)), dur, vel, blat=blat)) * g0`, kept by
    /// `(int(m), round(dur, 3), round(vel, 2), round(blat, 2))`.
    ///
    /// # Errors
    /// An argument is not a number, or the note would have no samples (Python raises there); the voice's level
    /// cannot be measured.
    pub fn tone(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, blat: f64) -> Result<Arc<[f64]>, MusicError> {
        for (what, v) in [("dur", dur), ("vel", vel), ("blat", blat)] {
            if !v.is_finite() {
                return Err(MusicError::new(H, format!("a tone's {what} is {v}, not a number")));
            }
        }
        // self.g0 = M.inst_gain(M.CFG[k]['pp']), measured when the part is made: the voice draws no shared noise,
        // so measuring it here (once: the context keeps it) gives the same soundtrack
        let g0 = ctx.inst_gain(&ctx.cfg[self.k].pp)?;
        let key = (
            m,
            bits(round_nd(dur, 3)),
            bits(round_nd(vel, 2)),
            bits(round_nd(blat, 2)),
        );
        let mut tones = self
            .tones
            .lock()
            .map_err(|_| MusicError::new(H, "a tone panicked earlier"))?;
        if let Some(x) = tones.get(&key) {
            return Ok(Arc::clone(x));
        }
        if !(pp::has_len(dur + 0.14) && pp::len(dur + 0.14) > 0) {
            return Err(MusicError::new(
                H,
                format!("no tone for a held length (dur) of {dur} s: it would end before it starts"),
            ));
        }
        let x = sousa_tone(pp::midi_hz(f64::from(m)), dur, vel, blat);
        let out: Arc<[f64]> = pp::to_48k(&x).iter().map(|v| v * g0).collect();
        tones.insert(key, Arc::clone(&out));
        Ok(out)
    }

    /// [`SousaphoneSound::tone`] as a mono sound to place.
    fn sig(&self, ctx: &MusicCtx, m: i32, dur: f64, vel: f64, blat: f64) -> Result<Sig, MusicError> {
        Ok(Sig::Mono(self.tone(ctx, m, dur, vel, blat)?.to_vec()))
    }

    /// `line_note(t)`: its line's note at `t`, in the lower two octaves of its register.
    ///
    /// # Errors
    /// The line has a rest there (Python cannot move `None` by octaves), or as [`MusicCtx::line_at`].
    fn line_note(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let name = self.name(ctx);
        let lo = self.register(ctx).0;
        let m = ctx.line_at(name, t)?.ok_or_else(|| {
            MusicError::new(
                H,
                format!("{name}'s line has a rest at {t} s, where its bass needs a note"),
            )
        })?;
        Ok(fit_reg(m, lo, lo + 22))
    }

    /// `root_of(t)`: the root of the chord at `t`, the lowest at or above a fourth over its bottom note.
    fn root_of(&self, ctx: &MusicCtx, t: f64) -> Result<i32, MusicError> {
        let lo = self.register(ctx).0;
        ctx.harmony.root_above(ctx.harmony.chord_at(t), lo + 5).map_err(bad)
    }

    /// `minor()`: the key's scale has a minor third.
    fn minor(&self, ctx: &MusicCtx) -> bool {
        let pc = ctx.harmony.key_pc;
        let third = imod_py(i64::from(pc) + 3, 12);
        ctx.harmony
            .scale
            .iter()
            .any(|&s| imod_py(i64::from(pc) + i64::from(s), 12) == third)
    }

    /// `trio_notes(t0, beats)`: the trio (a half note per beat here) in the fight's key, as `(time, midi note,
    /// seconds)`: its tonic is the key's relative major when the key is minor (the trio's own major), in the
    /// sousaphone's register; cut off after `beats` beats.
    fn trio_notes(&self, ctx: &MusicCtx, t0: f64, beats: i64) -> Vec<(f64, i32, f64)> {
        let beat = ctx.show.grid.beat;
        let lo = self.register(ctx).0;
        let relative = if self.minor(ctx) { 3 } else { 0 };
        let tonic = fit_reg(ctx.harmony.key_pc + 36 + relative, lo + 2, lo + 14);
        let beats = beats as f64;
        let mut out = Vec::new();
        for (bt, semi, d) in TRIO {
            if bt >= beats - 1e-6 {
                break;
            }
            out.push((t0 + bt * beat, tonic + semi, py_min(d, beats - bt) * beat));
        }
        out
    }

    /// `col(x, t, **kw)`: `self.colour(x, t, low=True, **kw)`: a bass line keeps its fundamental.
    fn col(&self, ctx: &MusicCtx, x: &[f64], t: f64, drive_k: f64) -> Result<(Vec<f64>, f64), MusicError> {
        let args = ColourArgs {
            low: true,
            ..self.colour_args()
        };
        let (sig, g, _lead) = self.colour(ctx, x, t, drive_k, &args)?;
        Ok((sig, g))
    }

    /// The first loop of `play`: marking time between sets, a note on every beat (the line's beat notes).
    fn marking_time(&self, ctx: &MusicCtx, mix: &mut Mix, th: &mut Thinner) -> Played {
        let (k, bus, pan) = (self.k, self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let ev = &ctx.rec.events;
        // (t0, t_blast) of every set, and (start, end) of every wall
        let marching: Vec<(f64, f64)> = state::sets(ev, k).iter().map(|s| (s.t0, s.end())).collect();
        let fronts: Vec<(f64, f64)> = state::fronts(ev, k).iter().map(|f| (f.tf, f.t1)).collect();
        let marking = |t: f64| {
            !marching.iter().any(|&(a, b)| a - 1e-6 <= t && t < b - 1e-6)
                && !fronts.iter().any(|&(a, b)| a - 1e-6 <= t && t < b)
        };
        // nb0, nb1 = int(math.ceil(T.T_FIGHT / T.BEAT)), int(M.T_KO / T.BEAT)
        let nb0 = slot((grid.t_fight / grid.beat).ceil())?;
        let nb1 = slot((ctx.show.t_ko / grid.beat).trunc())?;
        for nb in nb0..=nb1 {
            let tq = nb as f64 * grid.beat;
            if !marking(tq) {
                continue;
            }
            if !th.keep((nb * 4) as f64, ctx.hp_at(k, tq) / 100.0) {
                continue;
            }
            let x = self.tone(ctx, self.line_note(ctx, tq)?, grid.beat * 0.5, 0.9, 0.0)?;
            let (sig, g) = self.col(ctx, &x, tq, 1.0)?;
            mix.place(&bus, &Sig::Mono(sig), tq, 0.48 * g, pan);
        }
        Ok(())
    }

    /// The second loop of `play`: its events up to the K.O.
    fn events(&self, ctx: &MusicCtx, mix: &mut Mix, th: &mut Thinner) -> Played {
        let (k, bus, pan) = (self.k, self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (beat, s16) = (grid.beat, grid.s16);
        let lo = self.register(ctx).0;
        let thud = |mix: &mut Mix, dur: f64, f0: f64, f1: f64, seed: f64, tq: f64, g: f64| -> Played {
            let x = synth::thud(dur, f0, f1, noise_seed(H, seed)?);
            mix.place("fx", &Sig::Mono(x), tq, g, 0.0);
            Ok(())
        };
        for e in &ctx.rec.events {
            let t = e.t;
            if t > ctx.show.t_ko + 0.01 {
                break;
            }
            if e.who() != Some(k) {
                continue;
            }
            let tq = ctx.q16(t);
            match e.kind.as_str() {
                "drill_step" => {
                    // a footfall = a note of the marching line
                    if !th.keep(round_half_even(tq / s16), ctx.hp_at(k, t) / 100.0) {
                        continue;
                    }
                    let on8 = (tq / (2.0 * s16) - round_half_even(tq / (2.0 * s16))).abs() < 1e-6;
                    let (m, dur) = if on8 {
                        (self.line_note(ctx, tq)?, s16 * 1.6)
                    } else {
                        // DOUBLE TIME's 16ths: chromatic approach notes
                        let nxt = self.line_note(ctx, tq + s16)?;
                        (if nxt > lo { nxt - 1 } else { nxt + 1 }, s16 * 0.9)
                    };
                    let x = self.tone(ctx, m, dur, 0.92, 0.0)?;
                    let (sig, g) = self.col(ctx, &x, t, 1.0)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.52 * g, pan);
                    thud(mix, 0.12, 90.0, 50.0, t * 13.0, tq, 0.05)?;
                }
                "blast" => {
                    // the bell blast: a big blat on the root + its octave
                    let r = self.root_of(ctx, tq)?;
                    let x = self.tone(ctx, r, beat * 1.4, 1.0, 1.0)?;
                    let (sig, g) = self.col(ctx, &x, t, 1.3)?;
                    mix.place(&bus, &Sig::Mono(sig), tq, 0.62 * g, pan);
                    mix.place(&bus, &self.sig(ctx, r + 12, beat * 1.2, 0.9, 0.8)?, tq, 0.30 * g, pan);
                    thud(mix, 0.5, 120.0, 45.0, t * 7.0, tq, 0.16)?;
                }
                "drill_trample" => thud(mix, 0.5, 110.0, 32.0, t * 17.0, tq, 0.30)?,
                "trample_basket" => thud(mix, 0.2, 260.0, 140.0, t * 19.0, tq, 0.10)?,
                "horn_flash" => {
                    // a rip up into a blat
                    let r = self.root_of(ctx, tq)?;
                    for (j, m) in [r, r + 4, r + 7, r + 10].into_iter().enumerate() {
                        let at = tq + j as f64 * s16 / 2.0;
                        mix.place(&bus, &self.sig(ctx, m, s16 * 0.8, 0.85, 0.0)?, at, 0.34, pan);
                    }
                    mix.place(&bus, &self.sig(ctx, r + 12, beat, 1.0, 1.0)?, tq + 2.0 * s16, 0.48, pan);
                }
                "company_front" => {
                    // the trio, in low unison octaves
                    let beats = slot((e.f(6) - e.f(5)) / beat)?;
                    for (ts, m, d) in self.trio_notes(ctx, tq, beats) {
                        if ts > ctx.show.t_ko + 0.05 {
                            break;
                        }
                        let x = self.tone(ctx, m, d * 0.92, 1.0, 0.5)?;
                        let (sig, g) = self.col(ctx, &x, ts, 0.6)?;
                        mix.place(&bus, &Sig::Mono(sig), ts, 0.50 * g, pan * 0.4);
                        mix.place(&bus, &self.sig(ctx, m + 12, d * 0.9, 0.9, 0.3)?, ts, 0.26 * g, -pan);
                    }
                }
                "front_step" => thud(mix, 0.25, 100.0, 40.0, t * 23.0, tq, 0.12)?,
                _ => {}
            }
        }
        Ok(())
    }
}

impl Sound for SousaphoneSound {
    fn k(&self) -> usize {
        self.k
    }

    fn bus_gain(&self) -> f64 {
        1.6
    }

    fn send(&self) -> f64 {
        0.2
    }

    /// The marching bass's fundamentals (the bed has no low brass).
    fn band(&self) -> (f64, f64) {
        (40.0, 300.0)
    }

    fn colour_args(&self) -> ColourArgs {
        ColourArgs {
            cut0: 40.0,
            cut_span: 90.0,
            drive: 2.8,
            ..ColourArgs::default()
        }
    }

    /// `voice(m, dur, vel=0.85)`: `self.tone(m, dur, vel, 0.25)`.
    fn voice(&self, ctx: &MusicCtx, m: f64, dur: f64, vel: f64) -> Result<Arc<[f64]>, MusicError> {
        if !m.is_finite() {
            return Err(MusicError::new(H, format!("a note's midi is {m}, not a number")));
        }
        self.tone(ctx, m.trunc() as i32, dur, vel, VOICE_BLAT)
    }

    /// COMPANY FRONT under the enemy's desperation move: the enemy's melody gives way to the episode's obbligato
    /// (opt-in: only when the part was made with one).
    fn prepare(&self, ctx: &MusicCtx, _: &dyn Bed) -> Played {
        let Some(ob) = self.obbligato.as_ref().filter(|ob| !ob.is_empty()) else {
            return Ok(());
        };
        let enemy = self.enemy();
        let low = &ctx.cfg[enemy].low.name;
        let windows = obbligato_windows(&ctx.rec.events, self.k, low, ctx.show.t_ko, |t| ctx.q16(t));
        for (t0, t1, tz) in windows {
            ctx.line_override(enemy, t0, t1, tz, ob)?;
        }
        Ok(())
    }

    /// A huge blat on the tonic with its octave, then a pickup of marching 8ths.
    fn title_blast(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t: f64) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let (lo, hi) = self.register(ctx);
        // hits = self.T.TITLE_HITS.get(self.name) or (self.root_of(0.0),)
        let hits = match ctx.ff.music.title_hits.get(self.name(ctx)) {
            Some(h) if !h.is_empty() => h.clone(),
            _ => vec![self.root_of(ctx, 0.0)?],
        };
        let r = fit_reg(hits[0], lo, hi);
        mix.place(&bus, &self.sig(ctx, r, 0.7, 1.0, 1.0)?, t, 0.62, pan);
        mix.place(&bus, &self.sig(ctx, r + 12, 0.6, 0.9, 0.8)?, t, 0.30, pan);
        for (j, &m) in hits.iter().skip(1).take(3).enumerate() {
            let at = t + grid.beat + j as f64 * grid.beat / 2.0;
            if at < grid.bar - 0.05 {
                let x = self.sig(ctx, fit_reg(m, lo, hi), grid.beat * 0.4, 0.9, 0.0)?;
                mix.place(&bus, &x, at, 0.42, pan);
            }
        }
        Ok(())
    }

    /// It marks time on the count-in: root on every beat, an 8th pickup into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let r = self.root_of(ctx, grid.bar)?;
        for j in 0..4 {
            let x = self.sig(ctx, r, grid.beat * 0.45, 0.95, 0.0)?;
            mix.place(&bus, &x, grid.bar + f64::from(j) * grid.beat, 0.48, pan);
        }
        let nxt = self.line_note(ctx, grid.t_fight)?;
        let pickup = if nxt - 2 >= lo { nxt - 2 } else { nxt + 2 };
        let x = self.sig(ctx, pickup, grid.beat * 0.4, 0.9, 0.0)?;
        mix.place(&bus, &x, grid.bar + 3.5 * grid.beat, 0.40, pan);
        Ok(())
    }

    fn play(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        // th = M.Thinner(530 + k): one thinner for the beat notes, then the footfalls
        let mut th = Thinner::new(530.0 + self.k as f64);
        self.marking_time(ctx, mix, &mut th)?;
        self.events(ctx, mix, &mut th)
    }

    /// The horn sags: a falling blat and the valves rattling.
    fn fall_apart(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let t0 = ctx.show.t_hs;
        let r = self.root_of(ctx, t0)?;
        let x = self.tone(ctx, r, 1.3, 1.0, 1.0)?;
        mix.place(
            &self.bus(ctx),
            &Sig::Mono(bend(&x, -9.0, 0.1, 1.2)?),
            t0,
            0.5,
            self.pan(),
        );
        for j in 0..5u64 {
            let rattle = synth::thud(0.06, 900.0, 600.0, 40 + j);
            mix.place("fx", &Sig::Mono(rattle), t0 + 0.06 + 0.07 * j as f64, 0.06, 0.0);
        }
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, tw: f64) -> Played {
        let s16 = ctx.show.grid.s16;
        let r = self.root_of(ctx, 0.0)?;
        mix.place("solo", &self.sig(ctx, r, 0.25, 1.0, 0.6)?, tw, 0.44, 0.0);
        mix.place(
            "solo",
            &self.sig(ctx, r + 7, 0.25, 1.0, 0.6)?,
            tw + s16 * 2.0,
            0.44,
            0.0,
        );
        mix.place(
            "solo",
            &self.sig(ctx, r + 12, 1.0, 1.0, 1.0)?,
            tw + s16 * 4.0,
            0.50,
            0.0,
        );
        Ok(())
    }

    /// The fight file's solo: a strutting marching-bass solo, the long notes blatted.
    fn solo(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix, t0: f64) -> Played {
        let grid = &ctx.show.grid;
        let (lo, hi) = self.register(ctx);
        for (bt, m, d) in ctx.solo(self.name(ctx))? {
            let ts = t0 + bt * grid.beat;
            if ts > grid.t_reassemble - 0.05 {
                break;
            }
            let dur = py_max(0.1, d * grid.beat * 0.9);
            let blat = if d >= 1.5 { 0.8 } else { 0.2 };
            mix.place(
                "solo",
                &self.sig(ctx, fit_reg(m, lo, hi), dur, 0.95, blat)?,
                ts,
                0.6,
                0.0,
            );
        }
        Ok(())
    }

    fn reassemble(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let r = self.root_of(ctx, 0.0)?;
        let run = [r, r + 4, r + 7, r + 12, r + 16, r + 19];
        for (j, m) in run.into_iter().enumerate() {
            let tj = ctx.q16(grid.t_reassemble + (grid.t_glide - grid.t_reassemble) * j as f64 / run.len() as f64);
            let x = self.sig(ctx, fit_reg(m, lo, lo + 28), 0.12, 0.85, 0.0)?;
            mix.place("solo", &x, tj, 0.3, 0.0);
        }
        Ok(())
    }

    /// Marching up into the title slam: 8ths climbing the dominant to the tonic on frame 0.
    fn last_bar(&self, ctx: &MusicCtx, _: &dyn Bed, mix: &mut Mix) -> Played {
        let (bus, pan) = (self.bus(ctx), self.pan());
        let grid = &ctx.show.grid;
        let lo = self.register(ctx).0;
        let t_last = (grid.n_bars as f64 - 1.0) * grid.bar;
        let tgt = fit_reg(self.root_of(ctx, 0.0)? + 12, lo + 7, lo + 22);
        let steps = [tgt - 5, tgt - 5, tgt - 3, tgt - 3, tgt - 2, tgt - 2, tgt - 1, tgt - 1];
        for (j, m) in steps.into_iter().enumerate() {
            let x = self.sig(ctx, m, grid.beat * 0.4, 0.95, 0.0)?;
            mix.place(&bus, &x, t_last + j as f64 * grid.beat / 2.0, 0.5, pan);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_moved_into_the_register_by_octaves() {
        assert_eq!(fit_reg(36, 28, 58), 36);
        assert_eq!(fit_reg(72, 28, 58), 48);
        assert_eq!(fit_reg(12, 28, 58), 36);
        // a window narrower than an octave: down first, then up, as the Python loops run
        assert_eq!(fit_reg(63, 30, 42), 39);
        assert_eq!(fit_reg(60, 35, 50), 48);
    }

    #[test]
    fn the_trio_is_sixteen_half_note_beats_in_order() {
        let mut at = 0.0;
        for (bt, semi, d) in TRIO {
            assert_eq!(bt, at, "each note starts where the last ended");
            assert!((2..=7).contains(&semi));
            at += d;
        }
        assert_eq!(at, 16.0);
    }

    fn ev(t: f64, kind: &str, who: i64, data: Vec<Value>) -> Event {
        let mut fields = vec![Value::Int(who)];
        fields.extend(data);
        Event {
            t,
            kind: kind.to_string(),
            fields,
        }
    }

    /// `prepare`: a window for every time the enemy's low ability fires while the wall marches, from that time
    /// (on the grid) to the wall's end or the K.O., counted from the wall's start.
    #[test]
    fn the_obbligato_windows_are_the_enemys_low_ability_inside_a_wall() {
        let ability = |t: f64, who: i64, name: &str| {
            ev(
                t,
                "ability",
                who,
                vec![Value::Str(name.into()), Value::Str("wave".into()), Value::Int(1)],
            )
        };
        let front = |t: f64, t1: f64| {
            ev(
                t,
                "company_front",
                1,
                vec![Value::Int(0), Value::Float(-1.0), Value::Float(t), Value::Float(t1)],
            )
        };
        let events = vec![
            ability(19.0, 0, "PICCOLO"),
            front(20.0, 24.0),
            ability(21.03, 0, "PICCOLO"),
            ability(22.0, 0, "FLUTTER TONGUE"),
            ability(22.5, 1, "PICCOLO"),
            ability(24.0, 0, "PICCOLO"),
            front(40.0, 44.0),
            ability(41.0, 0, "PICCOLO"),
        ];
        let q = |t: f64| (t * 8.0).round() / 8.0;
        let got = obbligato_windows(&events, 1, "PICCOLO", 42.5, q);
        assert_eq!(got, vec![(21.0, 24.0, 20.0), (41.0, 42.5, 40.0)]);
        // the other side has no wall
        assert!(obbligato_windows(&events, 0, "PICCOLO", 42.5, q).is_empty());
    }
}
