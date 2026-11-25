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

/// A one-voice note waiting for [`Mix::settle`]: its start sample and its frames.
type Waiting = (i64, Vec<[f64; 2]>);

/// Named stereo buses.
#[derive(Clone, Debug)]
pub struct Mix {
    dur: f64,
    ns: usize,
    buses: BTreeMap<String, Vec<[f64; 2]>>,
    /// The bus names in the order the buses were created.
    order: Vec<String>,
    /// The one-voice notes waiting for [`Mix::settle`], by bus: `(start sample, frames)`.
    queue: BTreeMap<String, Vec<Waiting>>,
}

/// How long a one-voice note takes to fade out when the next one begins ([`Mix::note`]), in seconds.
pub const VOICE_FADE: f64 = 0.03;
/// One-voice notes that begin within this many seconds of each other are one event (a chord, a doubled octave).
pub const SAME_EVENT: f64 = 0.001;

impl Mix {
    /// `Mix(dur)`: an empty mix of `dur` seconds (Python's default is 60.0).
    pub fn new(dur: f64) -> Mix {
        Mix {
            dur,
            ns: (SRF * dur) as usize,
            buses: BTreeMap::new(),
            order: Vec::new(),
            queue: BTreeMap::new(),
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

    /// Places a note of an instrument that sounds one note at a time (sound upgrade, M5: wind, brass, bowed
    /// strings under the Composed arrangement). With `one_voice` false this is [`Mix::place`], sample for sample.
    /// With it true the note waits until [`Mix::settle`], which lets every such note of the bus end when the next
    /// one begins: it fades to silence over [`VOICE_FADE`] from the next note's start, so two notes of a flute
    /// never ring through each other. Notes that begin within [`SAME_EVENT`] of each other are one event and do
    /// not end each other.
    ///
    /// # Panics
    /// As [`Mix::place`].
    pub fn note(&mut self, bus: &str, sig: &Sig, t: f64, gain: f64, pan: f64, one_voice: bool) {
        if !one_voice {
            return self.place(bus, sig, t, gain, pan);
        }
        self.bus_mut(bus);
        let start = t * SRF;
        assert!(start.is_finite(), "note: the time {t} s is not a finite number");
        let frames: Vec<[f64; 2]> = match sig {
            Sig::Mono(x) => {
                let a = (pan + 1.0) * dmath::PI / 4.0;
                let (left, right) = (dmath::cos(a), dmath::sin(a));
                x.iter().map(|&v| [v * left * gain, v * right * gain]).collect()
            }
            Sig::Stereo(x) => x.iter().map(|v| [v[0] * gain, v[1] * gain]).collect(),
        };
        self.queue
            .entry(bus.to_string())
            .or_default()
            .push((round_half_even(start) as i64, frames));
    }

    /// Whether one-voice notes are waiting for [`Mix::settle`].
    pub fn unsettled(&self) -> bool {
        self.queue.values().any(|q| !q.is_empty())
    }

    /// Puts the waiting one-voice notes ([`Mix::note`]) on their buses, each ended by the event after it. A score
    /// calls it after each stage that plays notes and before anything reads or reshapes a bus; with nothing
    /// waiting it does nothing.
    pub fn settle(&mut self) {
        let fade = (VOICE_FADE * SRF) as usize;
        let same = (SAME_EVENT * SRF) as i64;
        for (bus, mut notes) in std::mem::take(&mut self.queue) {
            notes.sort_by_key(|n| n.0);
            let mut i = 0;
            while i < notes.len() {
                // the event: every note that begins with this one; `j` is the first note of the next event
                let mut j = i + 1;
                while j < notes.len() && notes[j].0 - notes[i].0 <= same {
                    j += 1;
                }
                let next = notes.get(j).map(|n| n.0);
                for (start, frames) in &mut notes[i..j] {
                    if let Some(next) = next {
                        let cut = (next - *start).max(0) as usize;
                        frames.truncate(cut + fade);
                        for (k, f) in frames.iter_mut().enumerate().skip(cut) {
                            let g = 1.0 - (k - cut + 1) as f64 / fade as f64;
                            f[0] *= g;
                            f[1] *= g;
                        }
                    }
                    let b = self.buses.get_mut(&bus).expect("note made the bus");
                    let s0 = start.saturating_neg().max(0);
                    let i0 = (*start).max(0);
                    let n = (frames.len() as i64 - s0).min(b.len() as i64 - i0);
                    if n <= 0 {
                        continue;
                    }
                    let (s0, i0, n) = (s0 as usize, i0 as usize, n as usize);
                    for (o, v) in b[i0..i0 + n].iter_mut().zip(&frames[s0..s0 + n]) {
                        o[0] += v[0];
                        o[1] += v[1];
                    }
                }
                i = j;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M5: a one-voice note ends when the next one begins; notes of one instant are one event; without the flag a
    /// note is placed as `place` places it.
    #[test]
    fn a_one_voice_note_ends_when_the_next_begins() {
        let long = Sig::Mono(vec![1.0; 2 * SR]);
        let mut plain = Mix::new(4.0);
        plain.place("x", &long, 0.25, 0.7, 0.3);
        let mut same = Mix::new(4.0);
        same.note("x", &long, 0.25, 0.7, 0.3, false);
        assert_eq!(plain.bus("x"), same.bus("x"));
        assert!(!same.unsettled());

        let mut mix = Mix::new(4.0);
        mix.note("x", &long, 0.0, 1.0, -1.0, true);
        mix.note("x", &long, 0.5, 0.5, -1.0, true);
        // a chord with the second note, placed first: sorted by time, the three are two events
        mix.note("x", &long, 0.5 + 0.0005, 0.25, -1.0, true);
        assert!(mix.unsettled() && mix.bus("x").unwrap().iter().all(|f| *f == [0.0; 2]));
        mix.settle();
        assert!(!mix.unsettled());
        let b = mix.bus("x").unwrap();
        let at = |t: f64| b[(t * SRF) as usize][0];
        assert_eq!(at(0.4), 1.0);
        // 15 ms into the fade the first note is half gone, 40 ms after the second began it is silent
        assert!((at(0.515) - (0.5 + 0.25 + 0.5)).abs() < 0.01, "{}", at(0.515));
        assert!((at(0.54) - 0.75).abs() < 1e-12);
        // the last event rings out whole
        assert!((at(2.4) - 0.75).abs() < 1e-12 && at(2.6) == 0.0);
        // settling again changes nothing; a note before the start is cut as place cuts it
        let before = b.to_vec();
        mix.settle();
        assert_eq!(mix.bus("x").unwrap(), before);
        let mut mix = Mix::new(1.0);
        mix.note("y", &Sig::Stereo(vec![[1.0, 2.0]; 10]), -5.0 / SRF, 1.0, 0.0, true);
        mix.settle();
        assert_eq!(
            mix.bus("y").unwrap()[..6],
            [[1.0, 2.0], [1.0, 2.0], [1.0, 2.0], [1.0, 2.0], [1.0, 2.0], [0.0, 0.0]]
        );
        assert_eq!(mix.bus_names(), ["y"]);
    }

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
