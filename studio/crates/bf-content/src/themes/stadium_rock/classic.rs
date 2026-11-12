//! The stadium rock's classic arrangement: the `Bed` class of `projects/ballfight/kit/moods/stadium_rock.py`,
//! section by section as `kit.music.score()` calls them, each sound placed in the kit's order (sounds add up on a
//! bus in that order, buses come into being in it, and the cache below fills in it).
//!
//! **The kit's cache.** The kit's bed keeps every instrument's sound in a dict (`self._c`) under a key made when it
//! is first asked for, and hands that sound back for every later call with the same key; a duration in a key is
//! `round(dur, 3)`, so a later call that differs only past the rounding gets the first call's sound (the strum's
//! notes are asked for at its first call's `dur`). [`StadiumRockClassic`] keeps the same cache under the same keys,
//! filled in the same order (one bed per build, as the kit makes one bed per soundtrack). A velocity scales the
//! sound after the cache.
//!
//! **The strum** plays the ACOUSTIC's Pixel Pitch voice through the music context's `note` (its own cache and the
//! voice's level), `M.note('ACOUSTIC', int(m), dur, 0.5, 0.7)`, one note a string in strum order.
//!
//! **Plugged / unplugged.** `last_bars` ends with the lead envelope: the two arrangements' fight loudness (K-weighted,
//! [`lufs`]) is matched, then the plugged buses are scaled by `sin(w pi / 2)` and the unplugged by `cos(w pi / 2)`
//! times the match, `w` the plugged share at each sample ([`StadiumRockClassic::plugged_share`]).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::harmony::HarmonyError;
use bf_core::py::{py_max, py_min, round_half_even, round_nd};
use bf_core::show::{CAP_GAP, captions};
use bf_sound::SRF;
use bf_sound::dsp;
use bf_sound::mix::{Mix, Sig};
use bf_sound::music::{
    MusicCtx, MusicError, RISER_OVER, RISER_SEED, THUNDER_CRACK, event_int, event_number, noise_seed, riser, thunder,
};
use bf_sound::synth::{self, HAT_OPEN};
use bf_sound::traits::{Bed, BedHook, Played};

use super::{
    BUILD_BARS, BUSES, PLUG, UNPLUG, cajon, kick, lufs, pbass, pgtr, root, shaker, snare, strum_tones, tamb, tom, ubass,
};

/// The name the bed's errors carry.
const H: &str = "stadium_rock";

/// The ACOUSTIC's Pixel Pitch voice id, which the strum plays.
const ACOUSTIC: &str = "ACOUSTIC";

/// A key of the kit's `self._c`: the method's tag and its key fields, a duration as `round(dur, 3)` by its bits
/// (a rounded zero as +0.0, as Python's `-0.0 == 0.0`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// `('pg', root, round(dur, 3), mute, open_)`
    Pgtr(i32, u64, bool, bool),
    /// `('pb', m, round(dur, 3))`
    Pbass(i32, u64),
    /// `('ub', m, round(dur, 3))`
    Ubass(i32, u64),
    /// `('k',)`
    Kick,
    /// `('s',)`
    Snare,
    /// `('t', m)`
    Tom(i32),
    /// `('cj',)`
    Cajon,
    /// `('sh',)`
    Shaker,
    /// `('tb',)`
    Tamb,
    /// `('st', ch, up, round(dur, 3))`
    Strum(String, bool, u64),
}

/// `round(x, 3)` as a key.
fn r3(x: f64) -> u64 {
    let r = round_nd(x, 3);
    if r == 0.0 { 0.0f64.to_bits() } else { r.to_bits() }
}

/// Python's `int(x)` for a length: cut toward zero.
fn length(x: f64) -> usize {
    assert!(x > -1.0, "a length of {x} samples");
    x as usize
}

fn bad(e: HarmonyError) -> MusicError {
    MusicError::new(H, e.to_string())
}

fn scaled(x: &[f64], v: f64) -> Vec<f64> {
    x.iter().map(|a| a * v).collect()
}

fn put(mix: &mut Mix, bus: &str, x: Vec<f64>, t: f64, gain: f64) {
    mix.place(bus, &Sig::Mono(x), t, gain, 0.0);
}

/// `T.CHORDS[b]`.
fn chord<'a>(ctx: &'a MusicCtx, b: usize) -> Result<&'a str, MusicError> {
    let chords = &ctx.harmony.chords;
    chords.get(b).map(String::as_str).ok_or_else(|| {
        MusicError::new(
            H,
            format!("bar {b} is past the episode's last bar ({} bars)", chords.len()),
        )
    })
}

/// `range(a, int(round(x)))`'s end: Python's `round` (halves to even), from 0.
fn round_end(x: f64) -> usize {
    let r = round_half_even(x);
    if r > 0.0 { r as usize } else { 0 }
}

/// The stadium rock's bed as the kit plays it, with the kit's cache of sounds and the fight's plugged fighter.
#[derive(Debug, Default)]
pub struct StadiumRockClassic {
    /// The fight file's `plugged` (the kit's `PLUGGED`), or `None` for the left corner's fighter.
    plugged: Option<String>,
    cache: Mutex<BTreeMap<Key, Arc<Vec<f64>>>>,
}

impl StadiumRockClassic {
    /// The bed for the plugged fighter `plugged` (`None`: the left corner's), its cache empty.
    pub fn new(plugged: Option<String>) -> StadiumRockClassic {
        StadiumRockClassic {
            plugged,
            cache: Mutex::default(),
        }
    }

    /// `self._cache(key, fn)`: the sound under `key`, made by `make` the first time.
    fn cached(
        &self,
        key: Key,
        make: impl FnOnce() -> Result<Vec<f64>, MusicError>,
    ) -> Result<Arc<Vec<f64>>, MusicError> {
        if let Some(x) = self.cache.lock().expect("the cache").get(&key) {
            return Ok(Arc::clone(x));
        }
        let x = Arc::new(make()?);
        Ok(Arc::clone(
            self.cache.lock().expect("the cache").entry(key).or_insert(x),
        ))
    }

    fn root(&self, ctx: &MusicCtx, ch: &str) -> Result<i32, MusicError> {
        root(&ctx.harmony, ch).map_err(bad)
    }

    /// `self.pgtr(root, dur, mute, open_)`.
    fn pgtr(&self, r: i32, dur: f64, mute: bool, open: bool) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Pgtr(r, r3(dur), mute, open), || Ok(pgtr(r, dur, mute, open)))?;
        Ok(x.to_vec())
    }

    /// `self.pbass(m, dur)`.
    fn pbass(&self, m: i32, dur: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Pbass(m, r3(dur)), || Ok(pbass(m, dur)))?;
        Ok(x.to_vec())
    }

    /// `self.ubass(m, dur)`.
    fn ubass(&self, m: i32, dur: f64) -> Result<Vec<f64>, MusicError> {
        if m < 0 {
            return Err(MusicError::new(
                H,
                format!("the upright bass's note {m} is no noise seed"),
            ));
        }
        let x = self.cached(Key::Ubass(m, r3(dur)), || Ok(ubass(m, dur)))?;
        Ok(x.to_vec())
    }

    /// `self.kick(v)`.
    fn kick(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Kick, || Ok(kick()))?;
        Ok(scaled(&x, v))
    }

    /// `self.snare(v)`.
    fn snare(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Snare, || Ok(snare()))?;
        Ok(scaled(&x, v))
    }

    /// `self.tom(m, v)`.
    fn tom(&self, m: i32, v: f64) -> Result<Vec<f64>, MusicError> {
        if m < 0 {
            return Err(MusicError::new(H, format!("the tom's note {m} is no noise seed")));
        }
        let x = self.cached(Key::Tom(m), || Ok(tom(m)))?;
        Ok(scaled(&x, v))
    }

    /// `self.cajon(v)`.
    fn cajon(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Cajon, || Ok(cajon()))?;
        Ok(scaled(&x, v))
    }

    /// `self.shaker(v)`.
    fn shaker(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Shaker, || Ok(shaker()))?;
        Ok(scaled(&x, v))
    }

    /// `self.tamb(v)`.
    fn tamb(&self, v: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Tamb, || Ok(tamb()))?;
        Ok(scaled(&x, v))
    }

    /// `self.strum(ch, up, v, dur)`: the unplugged shimmer, a quiet strum of the chord's tones high up the neck
    /// (one ACOUSTIC note a string, 10 ms apart, down or up), darkened.
    fn strum(&self, ctx: &MusicCtx, ch: &str, up: bool, v: f64, dur: f64) -> Result<Vec<f64>, MusicError> {
        let x = self.cached(Key::Strum(ch.to_string(), up, r3(dur)), || {
            let mut ns = strum_tones(&ctx.harmony, ch).map_err(bad)?;
            if ns.is_empty() {
                return Err(MusicError::new(H, format!("{ch} has no tone to strum")));
            }
            if up {
                ns.reverse();
            }
            let d = length(0.010 * SRF);
            let mut parts = Vec::with_capacity(ns.len());
            for &m in &ns {
                parts.push(ctx.note(ACOUSTIC, f64::from(m), dur, 0.5, 0.7)?);
            }
            let longest = parts.iter().map(|p| p.len()).max().unwrap_or(0);
            let mut out = vec![0.0; longest + d * ns.len()];
            for (j, p) in parts.iter().enumerate() {
                for (o, v) in out[j * d..j * d + p.len()].iter_mut().zip(p.iter()) {
                    *o += v;
                }
            }
            let k = (ns.len() as f64).sqrt();
            Ok(
                synth::lp(&synth::hp(&out, 250.0, synth::FILTER_O), 3200.0, synth::FILTER_O)
                    .iter()
                    .map(|v| v / k)
                    .collect(),
            )
        })?;
        Ok(scaled(&x, v))
    }

    /// `Bed.bar(mix, b, dense, light)`: one bar of the song (bar index `b`) in both arrangements: the stadium kick
    /// (and the claps when it is big); plugged: chugs that open into power chords, pick bass 8ths, snare, hats,
    /// crashes and tom fills; unplugged: the strum's D . D U . U D U, the upright walking, cajon, tambourine and
    /// shaker. It thickens with `dense` (0 to 1); `light` thins it for the victory. Python's defaults are
    /// `dense = 0.0` and `light = False`.
    ///
    /// # Errors
    /// Where Python raises: a chord the harmony does not hold, a strum note the voice cannot make.
    pub fn bar(&self, ctx: &MusicCtx, mix: &mut Mix, b: usize, dense: f64, light: bool) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let t0 = b as f64 * bar;
        let ch = chord(ctx, b % grid.n_bars)?;
        let r = self.root(ctx, ch)?;
        let rb = r - 12;
        let e8 = beat / 2.0;
        let big = dense > 0.45 && !light;
        // both: the stadium kick (1 and 3, a push into 3 in the big sections)
        let kicks: &[f64] = if big { &[0.0, 2.0, 2.5] } else { &[0.0, 2.0] };
        for &bt in kicks {
            let v = if bt == 0.0 || bt == 2.0 { 1.0 } else { 0.7 };
            put(mix, "kick", self.kick(v)?, t0 + bt * beat, 0.55);
        }
        // plugged: palm-muted chugs -> open power chords, pick bass 8ths, the stadium kit
        for j in 0..8usize {
            let te = t0 + j as f64 * e8;
            if light && j % 2 == 1 {
                continue;
            }
            let open = dense > 0.3 && matches!(j, 0 | 3 | 6);
            if open {
                let dur = if j != 6 { 1.5 * e8 } else { 2.0 * e8 };
                put(mix, "p_gtr", self.pgtr(r, dur, false, dense > 0.7)?, te, 0.30);
            } else if !(dense > 0.3 && matches!(j, 1 | 4 | 7)) {
                put(mix, "p_gtr", self.pgtr(r, e8 * 0.9, true, false)?, te, 0.30);
            }
            put(mix, "p_bass", self.pbass(rb, e8 * 0.92)?, te, 0.42);
        }
        for bt in [1.0, 3.0] {
            put(mix, "p_drums", self.snare(1.0)?, t0 + bt * beat, 0.42);
        }
        let hat_n: usize = if dense > 0.6 && !light { 16 } else { 8 };
        for j in 0..hat_n {
            let acc = if j % (hat_n / 4) == 0 { 1.0 } else { 0.65 };
            let t = t0 + j as f64 * bar / hat_n as f64;
            put(mix, "p_drums", synth::hat(0.03, HAT_OPEN), t, 0.10 * acc);
        }
        if b % 4 == 2 || (big && b.is_multiple_of(2)) {
            put(mix, "p_drums", synth::crash(1.8), t0, 0.16);
        }
        if (b % 4 == 1 && dense > 0.2) || (big && b % 8 == 7) {
            for (j, m) in [50, 50, 45, 45, 40, 40].into_iter().enumerate() {
                put(
                    mix,
                    "p_drums",
                    self.tom(m, 1.0)?,
                    t0 + 2.5 * beat + j as f64 * s16,
                    0.26,
                );
            }
        }
        // unplugged: the strum shimmer, the upright bass, hand percussion
        let pat: &[usize] = if light { &[0, 3, 6] } else { &[0, 2, 3, 5, 6, 7] };
        for &j in pat {
            let s = self.strum(ctx, ch, matches!(j, 3 | 5 | 7), 1.0, 0.22)?;
            let g = if matches!(j, 0 | 2 | 6) { 0.22 } else { 0.16 };
            put(mix, "u_gtr", s, t0 + j as f64 * e8, g);
        }
        let walk = [rb, rb + 7, rb + 12, rb + 7];
        for (bt, &step) in walk.iter().enumerate() {
            let m = if dense > 0.3 {
                step
            } else if bt % 2 == 0 {
                rb
            } else {
                rb + 7
            };
            put(mix, "u_bass", self.ubass(m, beat)?, t0 + bt as f64 * beat, 0.55);
        }
        for bt in [1.0, 3.0] {
            put(mix, "u_perc", self.cajon(1.0)?, t0 + bt * beat, 0.42);
            put(mix, "u_perc", self.tamb(1.0)?, t0 + bt * beat, 0.20);
        }
        for j in 0..16usize {
            if light && j % 2 == 1 {
                continue;
            }
            let v = if j % 4 == 0 { 1.0 } else { 0.7 };
            put(mix, "u_perc", self.shaker(v)?, t0 + j as f64 * s16, 0.20);
        }
        if big {
            // claps on 2 and 4 in the big sections (both)
            for bt in [1.0, 3.0] {
                put(mix, "kick", synth::clap(), t0 + bt * beat, 0.28);
            }
        }
        Ok(())
    }

    /// `Bed.lead_times()`: the lead changes with the series' 5 HP hysteresis, `(t, leader)` (the leader's side), from
    /// the run's `heal` and `hit` events up to the K.O.
    ///
    /// # Errors
    /// An event that lacks an element the kit reads, or names no fighter 0 or 1.
    pub fn lead_times(&self, ctx: &MusicCtx) -> Result<Vec<(f64, usize)>, MusicError> {
        let mut hp = [100.0f64, 100.0];
        let mut lead: Option<usize> = None;
        let mut out = Vec::new();
        let side = |e, i| -> Result<usize, MusicError> {
            match event_int(H, e, i)? {
                0 => Ok(0),
                1 => Ok(1),
                k => Err(MusicError::new(H, format!("an event names fighter {k}"))),
            }
        };
        for e in &ctx.rec.events {
            if e.t > ctx.show.t_ko {
                break;
            }
            if e.kind == "heal" {
                hp[side(e, 2)?] += event_number(H, e, 3)?;
            } else if e.kind == "hit" {
                hp[side(e, 3)?] -= event_number(H, e, 4)?;
                let d = hp[0] - hp[1];
                let new = if d > 5.0 {
                    Some(0)
                } else if d < -5.0 {
                    Some(1)
                } else {
                    lead
                };
                if new != lead {
                    out.push((e.t, new.expect("a new leader")));
                    lead = new;
                }
            }
        }
        Ok(out)
    }

    /// The side of the plugged fighter: `M.NAMES.index(getattr(T, 'PLUGGED', M.NAMES[0]))`.
    ///
    /// # Errors
    /// The plugged fighter is not one of the fight's (Python's `ValueError`).
    fn plug(&self, ctx: &MusicCtx) -> Result<usize, MusicError> {
        let names = &ctx.show.names;
        let name = self.plugged.as_deref().unwrap_or(&names[0]);
        names.iter().position(|n| n == name).ok_or_else(|| {
            MusicError::new(
                H,
                format!("the plugged fighter {name:?} is not one of the fighters {names:?}"),
            )
        })
    }

    /// `Bed.plugged_share(n)`: the plugged share at each of `n` samples (1 plugged, 0 unplugged, 0.5 both): both up
    /// to FIGHT!, then each lead change a one-beat crossfade from the next 8th, the winner's arrangement from just
    /// after the K.O., back to both through the build (`np.interp` on the points, their times made non-decreasing).
    ///
    /// # Errors
    /// As [`StadiumRockClassic::lead_times`], and a plugged fighter that is not in the fight.
    pub fn plugged_share(&self, ctx: &MusicCtx, n: usize) -> Result<Vec<f64>, MusicError> {
        let grid = &ctx.show.grid;
        let plug = self.plug(ctx)?;
        let e8 = grid.beat / 2.0;
        let mut pts: Vec<(f64, f64)> = vec![(0.0, 0.5), (grid.t_fight, 0.5)];
        let mut w = 0.5;
        let mut go = |pts: &mut Vec<(f64, f64)>, t: f64, to: f64| {
            let last = pts.last().expect("a point").0;
            let ts = py_max((t / e8 - 1e-6).ceil() * e8, last);
            pts.push((ts, w));
            pts.push((ts + grid.beat, to));
            w = to;
        };
        for (t, lead) in self.lead_times(ctx)? {
            if t >= grid.t_fight {
                go(&mut pts, t, if lead == plug { 1.0 } else { 0.0 });
            }
        }
        // the winner's arrangement for the victory
        go(
            &mut pts,
            ctx.show.t_ko + 0.6,
            if ctx.show.win == plug { 1.0 } else { 0.0 },
        );
        // the build fades back to both for frame 0
        pts.push((grid.t_reassemble, w));
        pts.push((grid.t_glide, 0.5));
        pts.push((grid.dur + 10.0, 0.5));
        // np.maximum.accumulate (the times are numbers: no NaN to carry)
        let mut x: Vec<f64> = Vec::with_capacity(pts.len());
        for &(t, _) in &pts {
            let v = match x.last() {
                Some(&m) if m >= t => m,
                _ => t,
            };
            x.push(v);
        }
        let y: Vec<f64> = pts.iter().map(|p| p.1).collect();
        Ok(dsp::interp(&synth::tt(n), &x, &y))
    }

    /// `Bed.lead_envelope(mix)`: matches the two arrangements' loudness over the fight (each the sum of its buses
    /// at the class's levels, K-weighted from FIGHT! to the K.O.), then the equal-power crossfade that follows the
    /// leader: the plugged buses times `sin(w pi / 2)`, the unplugged times `cos(w pi / 2) * k_u`.
    ///
    /// # Errors
    /// As [`StadiumRockClassic::plugged_share`]; the mix has no plugged guitar bus (Python's `KeyError`).
    pub fn lead_envelope(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let level = |k: &str| {
            BUSES
                .iter()
                .find(|b| b.0 == k)
                .map(|b| b.1.0)
                .expect("a bus of the class")
        };
        let n = mix
            .bus("p_gtr")
            .ok_or_else(|| MusicError::new(H, "the mix has no p_gtr bus"))?
            .len();
        // sum(mix.buses[k] * g[k] for k in ... if k in mix.buses), from 0
        let sum_of = |mix: &Mix, keys: &[&str]| -> Result<Vec<[f64; 2]>, MusicError> {
            let mut acc: Option<Vec<[f64; 2]>> = None;
            for &k in keys {
                let Some(x) = mix.bus(k) else { continue };
                let g = level(k);
                let a = acc.get_or_insert_with(|| vec![[0.0; 2]; x.len()]);
                if a.len() != x.len() {
                    return Err(MusicError::new(H, format!("the bus {k} has another length")));
                }
                for (o, v) in a.iter_mut().zip(x) {
                    o[0] += v[0] * g;
                    o[1] += v[1] * g;
                }
            }
            acc.ok_or_else(|| MusicError::new(H, "an arrangement has no bus"))
        };
        let (t_fight, t_ko) = (ctx.show.grid.t_fight, ctx.show.t_ko);
        let lp = lufs(&sum_of(mix, &PLUG)?, t_fight, t_ko);
        let lu = lufs(&sum_of(mix, &UNPLUG)?, t_fight, t_ko);
        let k_u = dmath::pow(10.0, (lp - lu) / 20.0);
        let w = self.plugged_share(ctx, n)?;
        let gp: Vec<f64> = w.iter().map(|&w| dmath::sin(w * PI / 2.0)).collect();
        let gu: Vec<f64> = w.iter().map(|&w| dmath::cos(w * PI / 2.0) * k_u).collect();
        for (keys, gain) in [(&PLUG, &gp), (&UNPLUG, &gu)] {
            for &k in keys {
                if let Some(x) = mix.bus(k) {
                    let y: Vec<[f64; 2]> = x.iter().zip(gain.iter()).map(|(v, g)| [v[0] * g, v[1] * g]).collect();
                    if y.len() != x.len() {
                        return Err(MusicError::new(H, format!("the bus {k} is longer than the envelope")));
                    }
                    *mix.bus_mut(k) = y;
                }
            }
        }
        Ok(())
    }
}

impl Bed for StadiumRockClassic {
    /// The mood's class defines `boom` and `hit`, none of the other optional hooks.
    fn hooks(&self) -> &'static [BedHook] {
        &[BedHook::Boom, BedHook::Hit]
    }

    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &BUSES
    }

    /// The big room.
    fn reverb(&self) -> (f64, f64, f64) {
        (2.8, 0.75, 0.24)
    }

    fn comp(&self) -> (f64, f64) {
        (0.20, 0.5)
    }

    fn roar_gain(&self) -> f64 {
        0.24
    }

    fn doors_gain(&self) -> f64 {
        0.45
    }

    fn landing(&self) -> (f64, f64) {
        (0.08, 0.06)
    }

    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 102.0, 0.12)
    }

    /// The stadium kick.
    fn boom(&self, _: &MusicCtx, mix: &mut Mix, t: f64, g: f64) -> Played {
        put(mix, "kick", self.kick(1.0)?, t, g);
        Ok(())
    }

    /// A band hit on a chord in both arrangements (the envelope picks), the kick and a thud.
    fn hit(&self, ctx: &MusicCtx, mix: &mut Mix, t: f64, ch: &str, g: f64, ring: f64) -> Played {
        let r = self.root(ctx, ch)?;
        put(mix, "p_gtr", self.pgtr(r, ring, false, true)?, t, 0.42 * g);
        put(mix, "p_bass", self.pbass(r - 12, ring)?, t, 0.5 * g);
        put(mix, "p_drums", synth::crash(1.8), t, 0.20 * g);
        put(mix, "p_drums", self.snare(1.0)?, t, 0.36 * g);
        put(mix, "u_gtr", self.strum(ctx, ch, false, 1.0, 0.8)?, t, 0.40 * g);
        put(mix, "u_bass", self.ubass(r - 12, ring)?, t, 0.7 * g);
        put(mix, "u_perc", self.cajon(1.0)?, t, 0.5 * g);
        put(mix, "u_perc", self.tamb(1.0)?, t, 0.3 * g);
        put(mix, "kick", self.kick(1.0)?, t, 0.7 * g);
        let seed = noise_seed(H, t * 50.0)? + 3;
        put(mix, "fx", synth::thud(0.7, 110.0, 32.0, seed), t, 0.30 * g);
        Ok(())
    }

    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        let seed = noise_seed(H, t * 3.0)?;
        put(mix, "fx", synth::thud(0.15, 200.0, 90.0, seed), t, 0.06);
        Ok(())
    }

    /// The VS card at full level: the song's groove on the tonic and a band hit under each word.
    fn title(&self, ctx: &MusicCtx, mix: &mut Mix, times: [f64; 3]) -> Played {
        let tonic = chord(ctx, 0)?;
        self.bar(ctx, mix, 0, 1.0, false)?;
        for (tw, g) in times.into_iter().zip([0.85, 0.8, 0.8]) {
            self.hit(ctx, mix, tw, tonic, g, 0.4)?;
        }
        Ok(())
    }

    /// The thunder crack where it splits the divider.
    fn vs(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        put(mix, "fx", thunder(1.5, 91, THUNDER_CRACK)?, t, 0.55);
        Ok(())
    }

    /// Beat 4: a tom fill as the doors open.
    fn title_rest(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        for (j, m) in [45, 45, 40, 40].into_iter().enumerate() {
            let t = 3.0 * grid.beat + j as f64 * grid.s16;
            put(mix, "p_drums", self.tom(m + 5, 1.0)?, t, 0.26);
            put(mix, "u_perc", self.cajon(0.8 + 0.07 * j as f64)?, t, 0.26);
        }
        Ok(())
    }

    /// Bar 1: the band's count (sticks), dominant chugs, a fill into FIGHT!
    fn count_in(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar, s16) = (grid.beat, grid.bar, grid.s16);
        let ch = chord(ctx, 1)?;
        let r = self.root(ctx, ch)?;
        let stick = super::stick();
        // the groove keeps going under the count
        self.bar(ctx, mix, 1, 0.8, false)?;
        for &(tc, _) in &grid.count[..3] {
            put(mix, "fx", stick.clone(), tc, 0.30);
            put(mix, "kick", self.kick(1.0)?, tc, 0.55);
            put(mix, "p_gtr", self.pgtr(r, beat * 0.5, false, true)?, tc, 0.34);
            put(mix, "p_bass", self.pbass(r - 12, beat * 0.5)?, tc, 0.45);
            put(mix, "u_gtr", self.strum(ctx, ch, false, 1.0, 0.4)?, tc, 0.34);
            put(mix, "u_bass", self.ubass(r - 12, beat)?, tc, 0.6);
            put(mix, "u_perc", self.cajon(1.0)?, tc, 0.4);
        }
        // the fill into FIGHT!
        for j in 12..16usize {
            let t = bar + j as f64 * s16;
            let k = (j - 12) as f64;
            put(mix, "p_drums", self.tom([52, 50, 45, 40][j - 12], 1.0)?, t, 0.30);
            put(mix, "p_drums", self.snare(0.7 + 0.1 * k)?, t, 0.22);
            put(mix, "u_perc", self.cajon(0.7 + 0.1 * k)?, t, 0.34);
            put(mix, "kick", self.kick(0.6)?, t, 0.25);
            put(mix, "p_gtr", self.pgtr(r, s16 * 0.8, true, false)?, t, 0.26);
        }
        Ok(())
    }

    /// The drop.
    fn fight_start(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let ch = chord(ctx, ctx.harmony.loop_start)?;
        self.hit(ctx, mix, ctx.show.grid.t_fight, ch, 1.0, 1.2)
    }

    /// The song from FIGHT! to the K.O. (thicker bar by bar), a band hit under every captioned ability (the story's
    /// captions: `M.P.STORY['caps']`), a tape stop at the K.O.
    fn fight(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize) -> Played {
        for b in 2..=bar_ko {
            self.bar(ctx, mix, b, py_min(1.0, (b as f64 - 2.0) / 22.0), false)?;
        }
        for (t, _, _) in captions(&ctx.rec.events, CAP_GAP) {
            let tq = ctx.q16(t);
            put(mix, "p_drums", synth::crash(1.4), tq, 0.12);
            put(mix, "u_perc", self.tamb(1.0)?, tq, 0.25);
            put(mix, "kick", self.kick(1.0)?, tq, 0.4);
        }
        for bus in ["kick"].into_iter().chain(PLUG).chain(UNPLUG) {
            if let Some(x) = mix.bus(bus) {
                let stopped = synth::tape_stop(x, ctx.show.t_ko, 0.42);
                *mix.bus_mut(bus) = stopped;
            }
        }
        Ok(())
    }

    fn ko(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let t_ko = ctx.show.t_ko;
        self.hit(ctx, mix, t_ko, chord(ctx, 0)?, 1.0, 0.6)?;
        put(mix, "fx", thunder(2.2, 51, THUNDER_CRACK)?, t_ko, 0.45);
        Ok(())
    }

    fn stinger(&self, ctx: &MusicCtx, mix: &mut Mix, tw: f64) -> Played {
        self.hit(ctx, mix, tw, chord(ctx, 0)?, 1.0, 1.4)
    }

    /// The stadium holds the tonic until the solo: a ringing chord, the kick pulsing, cymbal swells (never quiet).
    fn after_ko(&self, ctx: &MusicCtx, mix: &mut Mix, bar_ko: usize, _tw: f64, t_solo: f64) -> Played {
        let grid = &ctx.show.grid;
        let (beat, bar) = (grid.beat, grid.bar);
        let tonic = chord(ctx, 0)?;
        let r = self.root(ctx, tonic)?;
        for b in bar_ko + 1..round_end(t_solo / bar) {
            let tq = b as f64 * bar;
            put(mix, "p_gtr", self.pgtr(r, bar * 0.95, false, true)?, tq, 0.30);
            put(mix, "p_bass", self.pbass(r - 12, bar * 0.95)?, tq, 0.40);
            put(mix, "u_gtr", self.strum(ctx, tonic, false, 1.0, 1.2)?, tq, 0.34);
            put(mix, "u_bass", self.ubass(r - 12, bar)?, tq, 0.6);
            for bt in 0..4u32 {
                let tb = tq + f64::from(bt) * beat;
                put(mix, "kick", self.kick(0.8)?, tb, 0.42);
                put(mix, "u_perc", self.shaker(1.0)?, tb + beat / 2.0, 0.22);
            }
            // A.crash(T.BAR)[::-1] * 0.5
            let sw: Vec<f64> = synth::crash(bar).iter().rev().map(|v| v * 0.5).collect();
            put(mix, "p_drums", sw, tq, 0.16);
        }
        Ok(())
    }

    /// The lighter bars under the solo, up to the build.
    fn victory(&self, ctx: &MusicCtx, mix: &mut Mix, b_solo: usize) -> Played {
        for b in b_solo..ctx.show.grid.n_bars.saturating_sub(BUILD_BARS) {
            self.bar(ctx, mix, b, 0.5, true)?;
        }
        Ok(())
    }

    /// The build into the title slam: the full song, a snare roll and toms into the seam, a riser; then the lead
    /// envelope over the whole song (both arrangements fade back to both for frame 0).
    fn last_bars(&self, ctx: &MusicCtx, mix: &mut Mix) -> Played {
        let grid = &ctx.show.grid;
        let (bar, s16, dur) = (grid.bar, grid.s16, grid.dur);
        let nb = grid.n_bars;
        for b in nb.saturating_sub(BUILD_BARS)..nb {
            self.bar(ctx, mix, b, 1.0, false)?;
        }
        let t_last = (nb as f64 - 1.0) * bar;
        for j in 0..16usize {
            let jf = j as f64;
            let v = 0.6 + 0.4 * jf / 15.0;
            let t = t_last + jf * s16;
            put(mix, "p_drums", self.snare(v)?, t, 0.20 + 0.12 * jf / 15.0);
            put(mix, "u_perc", self.cajon(v)?, t, 0.28 + 0.10 * jf / 15.0);
            if j >= 8 {
                let m = [52, 50, 47, 45, 43, 40, 38, 36][j - 8];
                put(mix, "p_drums", self.tom(m, 1.0)?, t, 0.22);
            }
        }
        put(mix, "fx", riser(dur - t_last, RISER_OVER, RISER_SEED)?, t_last, 0.20);
        self.lead_envelope(ctx, mix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_keys_are_the_kits() {
        // round(dur, 3): 0.1234 and 0.1226 share a key; -0.0001 rounds to the key of 0
        assert_eq!(r3(0.1234), r3(0.1226));
        assert_eq!(r3(-0.0001), r3(0.0));
        let bed = StadiumRockClassic::new(None);
        // a power chord is keyed by its rounded length: the first call's sound for both
        let first = bed.pgtr(40, 0.2, true, false).unwrap();
        assert_eq!(first, pgtr(40, 0.2, true, false));
        assert_eq!(bed.pgtr(40, 0.2004, true, false).unwrap(), first);
        assert_ne!(bed.pgtr(40, 0.2, false, false).unwrap(), first);
        // a velocity scales after the cache
        let k = bed.kick(1.0).unwrap();
        assert_eq!(bed.kick(0.6).unwrap(), scaled(&k, 0.6));
        assert!(bed.ubass(-1, 0.2).is_err() && bed.tom(-1, 1.0).is_err());
    }

    #[test]
    fn the_bed_has_boom_and_hit() {
        let bed = StadiumRockClassic::new(None);
        assert_eq!(bed.hooks(), &[BedHook::Boom, BedHook::Hit]);
        assert!(bed.has(BedHook::Boom) && !bed.has(BedHook::PressRoll));
    }

    #[test]
    fn python_rounds_the_solo_bar() {
        assert_eq!((round_end(2.5), round_end(3.5), round_end(-1.0)), (2, 4, 0));
    }
}
