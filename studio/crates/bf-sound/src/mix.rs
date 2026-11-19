//! `reelkit.audio.Mix`: named stereo buses that sounds are placed on at given times.
//!
//! A bus is `dur` seconds long plus six seconds of room for tails, and comes into being the first time something
//! is placed on it. Buses are kept by name; [`Mix::bus_names`] lists them in the order they were created, which
//! is the order Python's `mix.buses` dict iterates in (the kit's master adds the buses up in that order).

use std::collections::BTreeMap;

use bf_core::dmath;
use bf_core::py::round_half_even;

use crate::{SR, SRF};

/// A sound to place: mono (panned when placed) or stereo (placed as it is).
#[derive(Clone, Debug, PartialEq)]
pub enum Sig {
    Mono(Vec<f64>),
    Stereo(Vec<[f64; 2]>),
}

impl Sig {
    /// The number of samples (frames, for a stereo sound).
    pub fn len(&self) -> usize {
        match self {
            Sig::Mono(x) => x.len(),
            Sig::Stereo(x) => x.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Named stereo buses.
#[derive(Clone, Debug)]
pub struct Mix {
    dur: f64,
    ns: usize,
    buses: BTreeMap<String, Vec<[f64; 2]>>,
    /// The bus names in the order the buses were created.
    order: Vec<String>,
}

impl Mix {
    /// `Mix(dur)`: an empty mix of `dur` seconds (Python's default is 60.0).
    pub fn new(dur: f64) -> Mix {
        Mix {
            dur,
            ns: (SRF * dur) as usize,
            buses: BTreeMap::new(),
            order: Vec::new(),
        }
    }

    /// The length the mix was made with, in seconds.
    pub fn dur(&self) -> f64 {
        self.dur
    }

    /// `NS`: the length of the mix in samples, `int(SR * dur)`. A bus is six seconds longer.
    pub fn ns(&self) -> usize {
        self.ns
    }

    /// The bus `name`, if something was placed on it (or [`Mix::bus_mut`] made it).
    pub fn bus(&self, name: &str) -> Option<&[[f64; 2]]> {
        self.buses.get(name).map(Vec::as_slice)
    }

    /// Python's `mix.bus(name)`: the bus `name`, made silent and `NS + 6 * SR` samples long if it is not there
    /// yet. Writing a whole new buffer through it replaces the bus, as `mix.buses[name] = ...` does.
    pub fn bus_mut(&mut self, name: &str) -> &mut Vec<[f64; 2]> {
        if !self.buses.contains_key(name) {
            self.order.push(name.to_string());
        }
        let len = self.ns + SR * 6;
        self.buses
            .entry(name.to_string())
            .or_insert_with(|| vec![[0.0; 2]; len])
    }

    /// The names of the buses, in the order they were created (as Python's `mix.buses` iterates).
    pub fn bus_names(&self) -> Vec<&str> {
        self.order.iter().map(String::as_str).collect()
    }

    /// `mix.place(bus, sig, t, gain, pan)`: adds `sig * gain` to the bus, starting at sample `round(t * SR)`
    /// (Python's `round`: halves go to the even sample). A mono sound is panned with equal power first: the left
    /// channel is `sig * cos(a)` and the right `sig * sin(a)` for `a = (pan + 1) * pi / 4`, so `pan` runs from -1
    /// (left) through 0 (centre) to 1 (right); a stereo sound ignores `pan`. What would fall before the start of
    /// the bus or after its end is cut off. Python's defaults are `gain = 1.0` and `pan = 0.0`.
    ///
    /// # Panics
    /// Where Python's `round` raises: `t` is NaN or infinite.
    pub fn place(&mut self, bus: &str, sig: &Sig, t: f64, gain: f64, pan: f64) {
        let b = self.bus_mut(bus);
        let start = t * SRF;
        assert!(start.is_finite(), "place: the time {t} s is not a finite number");
        let i0 = round_half_even(start) as i64;
        let s0 = i0.saturating_neg().max(0);
        let i0 = i0.max(0);
        let n = (sig.len() as i64 - s0).min(b.len() as i64 - i0);
        if n <= 0 {
            return;
        }
        let (s0, i0, n) = (s0 as usize, i0 as usize, n as usize);
        let into = &mut b[i0..i0 + n];
        match sig {
            Sig::Mono(x) => {
                let a = (pan + 1.0) * dmath::PI / 4.0;
                let (left, right) = (dmath::cos(a), dmath::sin(a));
                for (frame, &v) in into.iter_mut().zip(&x[s0..s0 + n]) {
                    frame[0] += v * left * gain;
                    frame[1] += v * right * gain;
                }
            }
            Sig::Stereo(x) => {
                for (frame, v) in into.iter_mut().zip(&x[s0..s0 + n]) {
                    frame[0] += v[0] * gain;
                    frame[1] += v[1] * gain;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::SRF;

    #[test]
    fn a_bus_appears_when_something_is_placed() {
        let mut mix = Mix::new(1.0);
        assert_eq!((mix.dur(), mix.ns()), (1.0, 48000));
        assert!(mix.bus("drums").is_none() && mix.bus_names().is_empty());
        mix.place("fx", &Sig::Mono(vec![1.0]), 0.0, 1.0, 0.0);
        mix.place("drums", &Sig::Mono(vec![1.0]), 0.0, 1.0, 0.0);
        mix.place("fx", &Sig::Mono(vec![1.0]), 0.0, 1.0, 0.0);
        // Even a sound that falls wholly outside makes its bus, as in Python.
        mix.place("air", &Sig::Mono(vec![1.0]), 100.0, 1.0, 0.0);
        assert_eq!(mix.bus_names(), ["fx", "drums", "air"]);
        // NS plus six seconds of room.
        assert_eq!(mix.bus("drums").unwrap().len(), 48000 + 6 * 48000);
        assert!(mix.bus("air").unwrap().iter().all(|f| *f == [0.0, 0.0]));
        assert_eq!(Mix::new(60.5).ns(), 2_904_000);
        assert_eq!(Sig::Stereo(vec![[0.0; 2]; 3]).len(), 3);
        assert!(Sig::Mono(Vec::new()).is_empty());
    }

    #[test]
    fn a_mono_sound_is_panned_with_equal_power() {
        for pan in [-1.0, -0.6, -0.25, 0.0, 0.3, 0.75, 1.0] {
            let mut mix = Mix::new(0.01);
            mix.place("x", &Sig::Mono(vec![1.0, -0.5]), 0.0, 1.0, pan);
            let a = (pan + 1.0) * dmath::PI / 4.0;
            let (l, r) = (dmath::cos(a), dmath::sin(a));
            let b = mix.bus("x").unwrap();
            assert_eq!(b[..3], [[l, r], [-0.5 * l, -0.5 * r], [0.0, 0.0]], "pan {pan}");
            // The same power wherever it sits.
            assert!((l * l + r * r - 1.0).abs() < 1e-15, "pan {pan}");
        }
        let at = |pan: f64| {
            let mut mix = Mix::new(0.01);
            mix.place("x", &Sig::Mono(vec![1.0]), 0.0, 1.0, pan);
            mix.bus("x").unwrap()[0]
        };
        // Hard left, the centre (3 dB down in each channel), hard right.
        assert_eq!(at(-1.0), [1.0, 0.0]);
        let [l, r] = at(0.0);
        assert!((l - 0.5f64.sqrt()).abs() < 1e-15 && (r - 0.5f64.sqrt()).abs() < 1e-15);
        let [l, r] = at(1.0);
        assert!(l.abs() < 1e-15 && r == 1.0);
    }

    #[test]
    fn gain_comes_after_the_pan_and_sounds_add_up() {
        let mut mix = Mix::new(0.01);
        let a = (0.3 + 1.0) * dmath::PI / 4.0;
        mix.place("x", &Sig::Mono(vec![0.7]), 0.0, 0.3, 0.3);
        // (sig * cos(a)) * gain, in that order.
        assert_eq!(
            mix.bus("x").unwrap()[0],
            [0.7 * dmath::cos(a) * 0.3, 0.7 * dmath::sin(a) * 0.3]
        );
        // A stereo sound keeps its channels: the pan is not used.
        let mut mix = Mix::new(0.01);
        let s = Sig::Stereo(vec![[1.0, 2.0], [3.0, 4.0]]);
        mix.place("x", &s, 0.0, 0.5, -1.0);
        mix.place("x", &s, 1.0 / SRF, 2.0, 1.0);
        assert_eq!(
            mix.bus("x").unwrap()[..4],
            [[0.5, 1.0], [3.5, 6.0], [6.0, 8.0], [0.0, 0.0]]
        );
    }

    #[test]
    fn the_start_is_rounded_half_to_even() {
        // 0.5, 1.5 and 2.5 samples in: Python's round gives 0, 2 and 2. (Times picked so that `t * SR` is exactly
        // the half: 3.5 / 48000 * 48000, for one, is not 3.5.)
        for (half, want) in [(0.5, 0), (1.5, 2), (2.5, 2), (4.5, 4), (5.5, 6), (2.4, 2), (2.6, 3)] {
            let t = half / SRF;
            assert_eq!(t * SRF, half, "{half} samples is not exact");
            let mut mix = Mix::new(0.01);
            mix.place("x", &Sig::Stereo(vec![[1.0, 1.0]]), t, 1.0, 0.0);
            let b = mix.bus("x").unwrap();
            assert_eq!(b.iter().position(|f| f[0] != 0.0), Some(want), "{half} samples");
        }
    }

    #[test]
    fn what_falls_outside_the_bus_is_cut_off() {
        let ramp = Sig::Stereo((1..=5).map(|i| [f64::from(i), -f64::from(i)]).collect());
        // Two samples before the start: the first two samples are dropped, the rest starts at 0.
        let mut mix = Mix::new(0.01);
        mix.place("x", &ramp, -2.0 / SRF, 1.0, 0.0);
        assert_eq!(
            mix.bus("x").unwrap()[..4],
            [[3.0, -3.0], [4.0, -4.0], [5.0, -5.0], [0.0, 0.0]]
        );
        // -1.5 samples rounds to -2 (half to even), -2.5 as well.
        for t in [-1.5 / SRF, -2.5 / SRF] {
            let mut mix = Mix::new(0.01);
            mix.place("x", &ramp, t, 1.0, 0.0);
            assert_eq!(mix.bus("x").unwrap()[0], [3.0, -3.0]);
        }
        // Wholly before the start: nothing.
        let mut mix = Mix::new(0.01);
        mix.place("x", &ramp, -5.0 / SRF, 1.0, 0.0);
        mix.place("x", &ramp, -1.0, 1.0, 0.0);
        mix.place("x", &ramp, -1e300, 1.0, 0.0);
        mix.place("x", &ramp, 1e300, 1.0, 0.0);
        assert!(mix.bus("x").unwrap().iter().all(|f| *f == [0.0, 0.0]));
        // Over the end: the bus keeps what fits.
        let mut mix = Mix::new(0.01);
        let len = mix.ns() + 6 * SR;
        mix.place("x", &ramp, (len - 2) as f64 / SRF, 1.0, 0.0);
        let b = mix.bus("x").unwrap();
        assert_eq!(b.len(), len);
        assert_eq!(b[len - 3..], [[0.0, 0.0], [1.0, -1.0], [2.0, -2.0]]);
    }

    #[test]
    #[should_panic(expected = "place: the time NaN s is not a finite number")]
    fn a_time_must_be_a_number() {
        Mix::new(0.01).place("x", &Sig::Mono(vec![1.0]), f64::NAN, 1.0, 0.0);
    }

    #[test]
    fn a_bus_can_be_replaced() {
        let mut mix = Mix::new(0.01);
        mix.place("x", &Sig::Mono(vec![1.0]), 0.0, 1.0, -1.0);
        let silent = vec![[0.0; 2]; mix.bus("x").unwrap().len()];
        *mix.bus_mut("x") = silent;
        assert_eq!(mix.bus("x").unwrap()[0], [0.0, 0.0]);
        assert_eq!(mix.bus_names(), ["x"]);
        // bus_mut makes a bus that is not there yet.
        assert_eq!(mix.bus_mut("y").len(), 480 + 6 * 48000);
        assert_eq!(mix.bus_names(), ["x", "y"]);
    }

    /// What `apply_stops` and `lead_envelope` (dark tango, stadium rock) need: the bus `place` made, to change in
    /// place, without a new bus for one the mix does not have (they skip those: `if k not in mix.buses`).
    #[test]
    fn bus_mut_is_the_bus_place_made() {
        let mut mix = Mix::new(0.01);
        mix.place("strings", &Sig::Mono(vec![0.5, 0.25]), 0.0, 2.0, 0.0);
        let placed = mix.bus("strings").unwrap().to_vec();
        let bus = mix.bus_mut("strings");
        assert_eq!(*bus, placed);
        for frame in bus.iter_mut().take(1) {
            frame[0] = 0.0;
        }
        assert_eq!(mix.bus("strings").unwrap()[0][0], 0.0);
        assert_eq!(mix.bus("strings").unwrap()[1], placed[1]);
        assert_eq!(mix.bus_names(), ["strings"]);
        // the bed asks first; a bus it does not have stays absent
        assert!(mix.bus("pad").is_none());
        assert_eq!(mix.bus_names(), ["strings"]);
    }
}
