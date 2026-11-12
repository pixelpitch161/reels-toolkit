//! `kit/music.py master()`: from the mix of a score to the soundtrack.
//!
//! ONE master gain (no section automation: consistent loudness from frame 0), a compressor and a soft limiter;
//! the overflow past the loop point (tails, reverb) folds onto the start; [`NA`] samples (whole AAC frames), so
//! the file loops sample-perfectly. In Python's order:
//!
//! 1. every bus times its gain ([`Levels`]; a bus without a level keeps its own);
//! 2. the reverb send: the buses that have a level, each times its send, added up in the order of the levels;
//!    the bed's room ([`crate::synth::reverb`]) makes the wet signal of it, times the bed's `wet`;
//! 3. the buses added up in the order the score first touched them, plus the wet signal;
//! 4. the fold: what lies past sample `NA` is added onto the start;
//! 5. the compressor: an envelope follower (`lfilter([0.002], [1, -0.998], ...)` on the mean square of the two
//!    channels, started one second early on the END of the loop so that it is already settled at sample 0), and
//!    above the bed's threshold a gain of `(threshold / envelope) ** exponent`;
//! 6. one scale that puts the 99.97th percentile of the absolute samples at the ceiling, a `tanh` limiter, and
//!    one more scale that puts the peak at the ceiling.
//!
//! The stems are the folded buses under the same compressor gain and the same two scales (without the reverb,
//! which is in the master only).

use std::collections::BTreeMap;

use bf_core::dmath;

use crate::mix::Mix;
use crate::music::MusicCtx;
use crate::synth::{self, REVERB_SEED};
use crate::traits::{Bed, Sound};
use crate::{NA, SR, dsp};

/// `kit.music.BUS`: the `(gain, reverb send)` of the shared buses.
pub const BUS: [(&str, (f64, f64)); 2] = [("solo", (1.5, 0.4)), ("fx", (1.2, 0.25))];

/// `master`'s default ceiling.
pub const MASTER_CEILING: f64 = 0.89;

/// The bus levels of a master: each bus's `(gain, reverb send)`, kept in the order they were set, as Python's
/// `levels` dict keeps them. The order matters: the reverb send adds the buses up in it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Levels {
    rows: Vec<(String, (f64, f64))>,
}

impl Levels {
    /// No levels.
    pub fn new() -> Levels {
        Levels::default()
    }

    /// `levels[bus] = (gain, send)`: a new bus goes to the end; a bus already there keeps its place and takes
    /// the new values.
    pub fn set(&mut self, bus: &str, gain: f64, send: f64) {
        match self.rows.iter_mut().find(|(name, _)| name == bus) {
            Some(row) => row.1 = (gain, send),
            None => self.rows.push((bus.to_string(), (gain, send))),
        }
    }

    /// `levels.get(bus)`: the bus's `(gain, send)`, if it has a level.
    pub fn get(&self, bus: &str) -> Option<(f64, f64)> {
        self.rows.iter().find(|(name, _)| name == bus).map(|(_, level)| *level)
    }

    /// The levels in order: `(bus, (gain, send))`.
    pub fn iter(&self) -> impl Iterator<Item = (&str, (f64, f64))> {
        self.rows.iter().map(|(name, level)| (name.as_str(), *level))
    }

    /// The levels as the first lines of `master()` build them: the two fighters' buses (their `bus_gain` and
    /// `send`), then the shared buses ([`BUS`]), then the bed's, then `overrides` (the episode's `MIX`: bus gain
    /// and send overrides; no fight file of Milestone 1 has any).
    pub fn of(ctx: &MusicCtx, sounds: [&dyn Sound; 2], bed: &dyn Bed, overrides: &[(&str, (f64, f64))]) -> Levels {
        let mut levels = Levels::new();
        for s in sounds {
            levels.set(&s.bus(ctx), s.bus_gain(), s.send());
        }
        let shared = BUS.iter().chain(bed.buses()).chain(overrides);
        for &(bus, (gain, send)) in shared {
            levels.set(bus, gain, send);
        }
        levels
    }
}

/// A finished soundtrack: the master and every bus after it (the stems), each [`NA`] frames long.
#[derive(Clone, Debug, PartialEq)]
pub struct Soundtrack {
    pub master: Vec<[f64; 2]>,
    pub stems: BTreeMap<String, Vec<[f64; 2]>>,
}

/// `fold(x)`: the first [`NA`] frames, with what lies past them added onto the start.
fn fold(x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut y = x[..x.len().min(NA)].to_vec();
    if x.len() > NA {
        for (frame, tail) in y.iter_mut().zip(&x[NA..]) {
            frame[0] += tail[0];
            frame[1] += tail[1];
        }
    }
    y
}

/// `master(mix, ceiling)`: the mix of a score mastered, see the module notes. `levels` are the bus gains and
/// reverb sends ([`Levels::of`]); the bed gives the room and the compressor. Python's default `ceiling` is
/// [`MASTER_CEILING`].
///
/// # Panics
/// Where Python raises, none of which a score's mix can be: no bus of the mix has a level (there is nothing to
/// send to the reverb: every score plays on `fx`), a bus was replaced by one of another length or shorter than
/// a second, or the bed's reverb is under one sample long.
pub fn master(mix: &Mix, levels: &Levels, bed: &dyn Bed, ceiling: f64) -> Soundtrack {
    let names = mix.bus_names();
    let bus = |name: &str| mix.bus(name).expect("a bus the mix lists");
    let len = names.first().map_or(0, |name| bus(name).len());
    assert!(
        names.iter().all(|name| bus(name).len() == len),
        "master: the buses of the mix differ in length"
    );
    let gain_of = |name: &str| levels.get(name).map_or(1.0, |(gain, _)| gain);

    // send = sum(buses.get(k, 0) * levels[k][1] for k in levels): Python's sum starts from 0.
    let mut send = vec![[0.0; 2]; len];
    let mut sent = false;
    for (name, (gain, level)) in levels.iter() {
        let Some(b) = mix.bus(name) else { continue };
        sent = true;
        for (s, f) in send.iter_mut().zip(b) {
            s[0] += f[0] * gain * level;
            s[1] += f[1] * gain * level;
        }
    }
    assert!(
        sent,
        "master: no bus of the mix has a level (the mix has {names:?}): there is nothing to send to the reverb"
    );
    let (rdur, rdec, rwet) = bed.reverb();
    let wet = synth::reverb(&send, rdur, rdec, REVERB_SEED);
    drop(send);

    // full = sum(buses.values()) + wet, in the order the buses were made; each bus is folded for its stem.
    let mut full = vec![[0.0; 2]; len];
    let mut stems: BTreeMap<String, Vec<[f64; 2]>> = BTreeMap::new();
    for &name in &names {
        let gain = gain_of(name);
        let scaled: Vec<[f64; 2]> = bus(name).iter().map(|f| [f[0] * gain, f[1] * gain]).collect();
        for (o, f) in full.iter_mut().zip(&scaled) {
            o[0] += f[0];
            o[1] += f[1];
        }
        stems.insert(name.to_string(), fold(&scaled));
    }
    for (o, w) in full.iter_mut().zip(&wet) {
        o[0] += w[0] * rwet;
        o[1] += w[1] * rwet;
    }
    drop(wet);
    let mut m = fold(&full);
    drop(full);

    // The compressor: the envelope starts a second early, on the last second of the loop.
    let (thr, ex) = bed.comp();
    let n = m.len();
    assert!(n >= SR, "master: the mix is {n} frames, shorter than a second");
    let power: Vec<f64> = m[n - SR..]
        .iter()
        .chain(m.iter())
        .map(|f| (f[0] * f[0] + f[1] * f[1]) / 2.0)
        .collect();
    let gain: Vec<f64> = dsp::lfilter(&[0.002], &[1.0, -0.998], &power)[SR..]
        .iter()
        .map(|&p| {
            let env = p.sqrt();
            if env > thr {
                // (thr / np.maximum(env, 1e-9)) ** ex
                dmath::pow(thr / if env > 1e-9 { env } else { 1e-9 }, ex)
            } else {
                1.0
            }
        })
        .collect();
    drop(power);
    for (f, g) in m.iter_mut().zip(&gain) {
        f[0] *= g;
        f[1] *= g;
    }

    // The limiter: the 99.97th percentile to the ceiling, tanh, then the peak to the ceiling.
    let level: Vec<f64> = m.as_flattened().iter().map(|v| v.abs()).collect();
    let s1 = ceiling / (dsp::percentile(&level, 99.97) + 1e-9);
    drop(level);
    let top = dmath::np_tanh(1.1);
    let mut peak = 0.0f64;
    for v in m.as_flattened_mut() {
        *v = dmath::np_tanh(*v * s1 * 1.1) / top;
        // np.abs(m).max(): a NaN stays.
        if v.abs() > peak || v.is_nan() {
            peak = v.abs();
        }
    }
    let s2 = ceiling / (peak + 1e-9);
    for v in m.as_flattened_mut() {
        *v *= s2;
    }
    for stem in stems.values_mut() {
        for (f, g) in stem.iter_mut().zip(&gain) {
            f[0] = f[0] * g * s1 * s2;
            f[1] = f[1] * g * s1 * s2;
        }
    }
    Soundtrack { master: m, stems }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mix::Sig;
    use crate::music::testkit::{TestBed, context, episode};

    /// A bed with a short room: 50 ms of tail, silent for its first 20.
    fn room() -> TestBed {
        TestBed::new((0.05, 0.02, 0.2))
    }

    struct Part(usize);

    impl Sound for Part {
        fn k(&self) -> usize {
            self.0
        }
        fn send(&self) -> f64 {
            0.2 + 0.1 * self.0 as f64
        }
    }

    #[test]
    fn levels_keep_the_order_they_were_set_in() {
        let mut levels = Levels::new();
        levels.set("b", 1.0, 0.1);
        levels.set("a", 2.0, 0.2);
        levels.set("b", 3.0, 0.3);
        // Python's dict: an update keeps the key's place.
        assert_eq!(
            levels.iter().collect::<Vec<_>>(),
            [("b", (3.0, 0.3)), ("a", (2.0, 0.2))]
        );
        assert_eq!((levels.get("a"), levels.get("c")), (Some((2.0, 0.2)), None));

        // master()'s own: the fighters, the shared buses, the bed's (which may set a shared bus again), the
        // overrides.
        let ep = episode();
        let (ctx, _) = context(&ep);
        let levels = Levels::of(
            &ctx,
            [&Part(0), &Part(1)],
            &room(),
            &[("flute", (0.9, 0.0)), ("extra", (1.0, 1.0))],
        );
        assert_eq!(
            levels.iter().collect::<Vec<_>>(),
            [
                ("flute", (0.9, 0.0)),
                ("trombone", (1.5, 0.30000000000000004)),
                ("solo", (1.5, 0.4)),
                ("fx", (0.5, 0.1)),
                ("drums", (1.0, 0.15)),
                ("extra", (1.0, 1.0)),
            ]
        );
    }

    #[test]
    fn fold_adds_the_overflow_onto_the_start() {
        let mut x = vec![[0.0; 2]; NA + 3];
        x[0] = [1.0, 2.0];
        x[1] = [0.5, 0.5];
        x[NA - 1] = [7.0, 7.0];
        x[NA] = [10.0, 20.0];
        x[NA + 2] = [30.0, 40.0];
        let y = fold(&x);
        assert_eq!(y.len(), NA);
        assert_eq!(y[..4], [[11.0, 22.0], [0.5, 0.5], [30.0, 40.0], [0.0, 0.0]]);
        assert_eq!(y[NA - 1], [7.0, 7.0]);
        // A buffer that ends before the loop point has nothing to fold.
        assert_eq!(fold(&x[..10]), x[..10]);
        assert!(fold(&[]).is_empty());
    }

    /// A short mix (one second, plus the six of room every bus has): a tone on `drums` from sample 4,800 to
    /// 14,400, and the first half of it, quieter, on `odd` from 7,200 to 12,000.
    fn small_mix() -> Mix {
        let mut mix = Mix::new(1.0);
        let tone: Vec<f64> = (0..9600)
            .map(|i| dmath::sin(f64::from(i) * 0.06) * dmath::exp(-f64::from(i) / 3000.0))
            .collect();
        mix.place("odd", &Sig::Mono(tone[..4800].to_vec()), 0.15, 0.2, 0.5);
        mix.place("drums", &Sig::Mono(tone), 0.1, 0.8, -0.3);
        mix
    }

    /// A sample after both tones, inside the room's tail of the end of the `drums` tone.
    const LATE: usize = 14400 + 1200;

    fn peak(x: &[[f64; 2]]) -> f64 {
        x.as_flattened().iter().fold(0.0f64, |p, v| p.max(v.abs()))
    }

    #[test]
    fn the_master_peaks_at_the_ceiling_and_the_stems_share_its_gain() {
        let mix = small_mix();
        let mut levels = Levels::new();
        levels.set("drums", 2.0, 0.5);
        let st = master(&mix, &levels, &room(), MASTER_CEILING);
        // Shorter than the loop: nothing folds, the length is the buses'.
        let len = mix.bus("drums").unwrap().len();
        assert_eq!((len, st.master.len()), (7 * SR, len));
        // The mix lists its buses in the order they were made; the stems are kept by name.
        assert_eq!(mix.bus_names(), ["odd", "drums"]);
        assert_eq!(st.stems.keys().collect::<Vec<_>>(), ["drums", "odd"]);
        assert!(st.stems.values().all(|s| s.len() == len));
        assert!((peak(&st.master) - MASTER_CEILING).abs() < 1e-8, "{}", peak(&st.master));
        assert!((peak(&master(&mix, &levels, &room(), 0.5).master) - 0.5).abs() < 1e-8);
        // A stem is its bus times its level's gain (1.0 for a bus without a level) times the gain the master
        // had at that sample (the compressor's and the two scales), the same in both channels and for every bus.
        let (drums, odd) = (mix.bus("drums").unwrap(), mix.bus("odd").unwrap());
        let (stem, other) = (&st.stems["drums"], &st.stems["odd"]);
        let i = 4800 + 300;
        let factor = stem[i][0] / (drums[i][0] * 2.0);
        assert!(factor > 0.0 && (stem[i][1] / (drums[i][1] * 2.0) - factor).abs() < 1e-12 * factor);
        assert_eq!(other[i], [0.0, 0.0]);
        let j = 7200 + 300;
        let both = stem[j][0] / (drums[j][0] * 2.0);
        assert!((other[j][0] / odd[j][0] - both).abs() < 1e-12 * both, "{both}");
        // The compressor moves: that gain is not the same at the two samples.
        assert!((both - factor).abs() > 1e-3 * factor, "{both} {factor}");
        // Before anything sounds the master is silent (but for the rounding of the reverb's transform), and
        // after the tones the room rings on in the master alone.
        assert!(st.master[100][0].abs() < 1e-12);
        assert!(st.master[LATE][0].abs() > 1e-6, "{}", st.master[LATE][0]);
        assert!(stem[LATE] == [0.0, 0.0] && other[LATE] == [0.0, 0.0]);
    }

    #[test]
    fn the_reverb_send_follows_the_levels() {
        let mix = small_mix();
        // No send, no room: after the tones the master is silent to the last bit.
        let mut dry = Levels::new();
        dry.set("drums", 2.0, 0.0);
        let plain = master(&mix, &dry, &room(), MASTER_CEILING);
        assert_eq!(plain.master[LATE], [0.0, 0.0]);
        // A level for a bus the mix does not have changes nothing.
        let mut more = dry.clone();
        more.set("gone", 3.0, 1.0);
        assert!(master(&mix, &more, &room(), MASTER_CEILING) == plain);
        // With a send the room is in the master, and only there.
        let mut sent = Levels::new();
        sent.set("drums", 2.0, 0.5);
        let roomy = master(&mix, &sent, &room(), MASTER_CEILING);
        assert!(roomy.master[LATE][0].abs() > 1e-6);
        assert_eq!(roomy.stems["drums"][LATE], [0.0, 0.0]);
    }

    #[test]
    #[should_panic(expected = "no bus of the mix has a level (the mix has [\"odd\", \"drums\"])")]
    fn a_mix_needs_a_bus_with_a_level() {
        let mut levels = Levels::new();
        levels.set("fx", 1.2, 0.25);
        master(&small_mix(), &levels, &room(), MASTER_CEILING);
    }

    #[test]
    #[should_panic(expected = "the buses of the mix differ in length")]
    fn the_buses_must_be_one_length() {
        let mut mix = small_mix();
        mix.bus_mut("odd").truncate(1000);
        let mut levels = Levels::new();
        levels.set("drums", 1.0, 0.1);
        master(&mix, &levels, &room(), MASTER_CEILING);
    }

    #[test]
    #[should_panic(expected = "sosfilt: x is empty")]
    fn a_room_needs_a_tail() {
        let mut levels = Levels::new();
        levels.set("drums", 1.0, 0.1);
        let none = TestBed::new((0.0, 0.5, 0.2));
        master(&small_mix(), &levels, &none, MASTER_CEILING);
    }
}
