//! The bar and beat grid of a 61 s episode (Task 5).
//!
//! A port of the tempo and time part of `build_timeline` in `projects/ballfight/kit/episode.py` and of
//! `reelkit/config.py`'s `loop_bpm`, with each expression in Python's operation order (an int times a float in Python
//! is the int converted to a float, then the product), plus the timeline helpers `bar_t`, `next_bar`, `beat_env` and
//! `beat_pulse`. The harmony helpers are in [`crate::harmony`].

use crate::{dmath, py};

/// `reelkit/config.py` `loop_bpm`: the nearest tempo at which `dur` seconds hold a whole number of bars (at least
/// one), so anything on the beat is in phase at the loop point. `round` ties to even, as Python's.
pub fn loop_bpm(bpm: f64, dur: f64, beats_per_bar: u32) -> f64 {
    let bpb = f64::from(beats_per_bar);
    let bars = py::round_half_even(dur * bpm / 60.0 / bpb);
    // max(1, round(...)): Python keeps the first argument unless the second is greater.
    let bars = if bars > 1.0 { bars } else { 1.0 };
    // `bars * beats_per_bar` is an exact int product in Python; it is exact here too for any sane tempo.
    bars * bpb * 60.0 / dur
}

/// Python's `max(a, b)` on floats: `a` unless `b` is greater.
fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// Python's `min(a, b)` on floats: `a` unless `b` is smaller.
fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// The episode's tempo and its fixed times, as `build_timeline` derives them (the timeline's upper-case names, in
/// lower case). Times are in seconds from frame 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    /// `FPS`: 30.
    pub fps: u32,
    /// `DUR`: 61.0 s.
    pub dur: f64,
    /// `NF`: `int(FPS * DUR)`, 1,830 frames.
    pub nf: usize,
    /// `BPM_BASE`: the config's `BPM`.
    pub bpm_base: f64,
    /// `BPM`: `loop_bpm(BPM_BASE, DUR)`, a whole number of bars in 61 s.
    pub bpm: f64,
    /// `BEAT`: `60.0 / BPM`.
    pub beat: f64,
    /// `BAR`: `4 * BEAT`.
    pub bar: f64,
    /// `N_BARS`: `int(round(DUR / BAR))`.
    pub n_bars: usize,
    /// `S16`: a sixteenth, `BEAT / 4`.
    pub s16: f64,
    /// The bar FIGHT! lands on (config `FIGHT_BAR`, 2 unless an old episode sets 1).
    pub fight_bar: usize,
    /// `T_FIGHT`: `fight_bar * BAR`.
    pub t_fight: f64,
    /// `COUNT`: the count-in on the bar before FIGHT!, one word a beat, then FIGHT! on the downbeat.
    pub count: [(f64, &'static str); 4],
    /// The times of `TITLE`'s three words (left name, VS, right name): `0.0`, `BEAT`, `2 * BEAT`. The words
    /// themselves come from the fighters, so they are not part of the grid. Not in the plan's struct: added because
    /// the title card needs these times.
    pub title: [f64; 3],
    /// `T_DOORS`: `(3 * BEAT + 0.10, BAR - 0.03)`.
    pub t_doors: (f64, f64),
    /// `T_CLOSE`: 0.30.
    pub t_close: f64,
    /// `T_REASSEMBLE`: `(N_BARS - 2) * BAR`, the loser's pieces fly back together.
    pub t_reassemble: f64,
    /// `T_GLIDE`: `(N_BARS - 1) * BAR`, the last bar: both glide to their corners.
    pub t_glide: f64,
    /// `T_REV`: `T_GLIDE + 1.05`.
    pub t_rev: f64,
}

impl Grid {
    /// The grid of a new episode (FIGHT! on bar 2) at the config tempo `bpm_base`.
    pub fn new(bpm_base: f64) -> Grid {
        Grid::with_fight_bar(bpm_base, 2)
    }

    /// The grid with FIGHT! on bar `fight_bar` (only an episode from before the VS card uses 1).
    pub fn with_fight_bar(bpm_base: f64, fight_bar: usize) -> Grid {
        let fps = 30u32;
        let dur = 61.0;
        let nf = (f64::from(fps) * dur) as usize;
        let bpm = loop_bpm(bpm_base, dur, 4);
        let beat = 60.0 / bpm;
        let bar = 4.0 * beat;
        let n_bars = py::round_half_even(dur / bar) as usize;
        let s16 = beat / 4.0;
        let fb = fight_bar as f64;
        let t_fight = fb * bar;
        let c0 = (fb - 1.0) * bar;
        let count = [
            (c0, "3"),
            (c0 + beat, "2"),
            (c0 + 2.0 * beat, "1"),
            (fb * bar, "FIGHT!"),
        ];
        let title = [0.0, beat, 2.0 * beat];
        let t_doors = (3.0 * beat + 0.10, bar - 0.03);
        let n = n_bars as f64;
        let t_reassemble = (n - 2.0) * bar;
        let t_glide = (n - 1.0) * bar;
        let t_rev = t_glide + 1.05;
        Grid {
            fps,
            dur,
            nf,
            bpm_base,
            bpm,
            beat,
            bar,
            n_bars,
            s16,
            fight_bar,
            t_fight,
            count,
            title,
            t_doors,
            t_close: 0.30,
            t_reassemble,
            t_glide,
            t_rev,
        }
    }

    /// `bar_t(b)`: the start of bar `b`, `b * BAR`.
    pub fn bar_t(&self, b: f64) -> f64 {
        b * self.bar
    }

    /// `next_bar(t)`: the first bar line at or after `t` (within a millionth of a bar), `ceil(t / BAR - 1e-6) * BAR`.
    pub fn next_bar(&self, t: f64) -> f64 {
        // math.ceil returns an int, which has no negative zero: `+ 0.0` turns a -0.0 into 0.0.
        let bars = (t / self.bar - 1e-6).ceil() + 0.0;
        bars * self.bar
    }

    /// `beat_env(t, decay=0.12)`: `exp(-((t % BEAT) / decay))`, 1 on each beat decaying to the next.
    pub fn beat_env(&self, t: f64, decay: f64) -> f64 {
        dmath::exp(-(py::fmod_py(t, self.beat) / decay))
    }

    /// `beat_pulse(t, decay=0.12, pre=0.12)`: a pulse on every beat that also ramps up (smoothstep) over the last
    /// `pre` seconds before it, so the beat on frame 0 does not snap at the loop seam.
    pub fn beat_pulse(&self, t: f64, decay: f64, pre: f64) -> f64 {
        let ph = py::fmod_py(t, self.beat);
        let up = py_min(1.0, py_max(0.0, (ph - (self.beat - pre)) / pre));
        py_max(dmath::exp(-ph / decay), up * up * (3.0 - 2.0 * up))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Python's `repr` of a float parsed back: the same bits.
    fn f(text: &str) -> f64 {
        text.parse().expect("a float")
    }

    /// Asserts two floats have the same bits (so `-0.0` and `0.0` differ).
    #[track_caller]
    fn same(got: f64, want: &str) {
        assert_eq!(got.to_bits(), f(want).to_bits(), "got {got:?}, Python gave {want}");
    }

    /// Asserts a value that ends in `exp` is within one unit in the last place of Python's. The argument of the `exp`
    /// matches Python bit for bit; `dmath::exp` (musl's) is not always correctly rounded where Windows' CRT is, so
    /// two of these differ by one ulp (t = 1.7 and t = 60.99 at decay 0.05).
    #[track_caller]
    fn near(got: f64, want: &str) {
        let (a, b) = (got.to_bits() as i64, f(want).to_bits() as i64);
        assert!((a - b).abs() <= 1, "got {got:?}, Python gave {want}");
    }

    #[test]
    fn loop_bpm_gives_a_whole_number_of_bars() {
        assert_eq!(loop_bpm(96.0, 61.0, 4), 24.0 * 4.0 * 60.0 / 61.0); // 94.43, 24 bars
        assert_eq!(loop_bpm(120.0, 61.0, 4), 30.0 * 4.0 * 60.0 / 61.0); // 118.03, 30 bars
        // round() ties to even: 146 bpm over 60 s is exactly 36.5 bars, which rounds to 36 (not 37).
        assert_eq!(loop_bpm(146.0, 60.0, 4), 144.0);
        // never fewer than one bar
        assert_eq!(loop_bpm(1.0, 61.0, 4), 4.0 * 60.0 / 61.0);
    }

    #[test]
    fn grid_matches_the_flute_vs_trombone_timeline() {
        let g = Grid::new(132.0);
        assert_eq!(g.n_bars, 34);
        assert_eq!(g.nf, 1830);
        assert_eq!(g.fps, 30);
        assert_eq!(g.dur, 61.0);
        assert_eq!(g.bpm_base, 132.0);
        assert_eq!(g.fight_bar, 2);
        assert!((g.bpm - 133.770_491_803_278_7).abs() < 1e-12);
        assert!((g.t_fight - 2.0 * g.bar).abs() == 0.0 && (g.t_fight - 3.588_235_294_117_647).abs() < 1e-12);
        assert_eq!(g.t_reassemble, 32.0 * g.bar);
        assert_eq!(g.t_glide, 33.0 * g.bar);
        assert_eq!(g.t_close, 0.30);
        assert_eq!(g.count.map(|c| c.1), ["3", "2", "1", "FIGHT!"]);
        assert_eq!(g.title, [0.0, g.beat, 2.0 * g.beat]);
    }

    /// Every derived time, as CPython 3.13 printed it (`repr`) from `build_timeline`'s own expressions on the
    /// reference PC, for five tempos: bars, bpm, beat, bar, s16, the four count-in times, the doors, reassemble,
    /// glide and the reveal.
    #[test]
    fn every_derived_time_matches_python_bit_for_bit() {
        #[rustfmt::skip]
        let cases: [(f64, usize, [&str; 13]); 5] = [
            (132.0, 34, ["133.7704918032787", "0.44852941176470584", "1.7941176470588234", "0.11213235294117646",
                "1.7941176470588234", "2.242647058823529", "2.691176470588235", "3.5882352941176467",
                "1.4455882352941176", "1.7641176470588233", "57.41176470588235", "59.205882352941174",
                "60.25588235294117"]),
            (96.0, 24, ["94.42622950819673", "0.6354166666666666", "2.5416666666666665", "0.15885416666666666",
                "2.5416666666666665", "3.177083333333333", "3.8125", "5.083333333333333", "2.00625",
                "2.5116666666666667", "55.916666666666664", "58.45833333333333", "59.508333333333326"]),
            (60.0, 15, ["59.01639344262295", "1.0166666666666666", "4.066666666666666", "0.25416666666666665",
                "4.066666666666666", "5.083333333333333", "6.1", "8.133333333333333", "3.15", "4.036666666666666",
                "52.86666666666666", "56.93333333333333", "57.98333333333333"]),
            (77.0, 20, ["78.68852459016394", "0.7625", "3.05", "0.190625", "3.05", "3.8125", "4.574999999999999",
                "6.1", "2.3874999999999997", "3.02", "54.9", "57.949999999999996", "58.99999999999999"]),
            (200.0, 51, ["200.65573770491804", "0.29901960784313725", "1.196078431372549", "0.07475490196078431",
                "1.196078431372549", "1.4950980392156863", "1.7941176470588234", "2.392156862745098",
                "0.9970588235294117", "1.166078431372549", "58.6078431372549", "59.80392156862745",
                "60.85392156862745"]),
        ];
        for (bpm, bars, want) in cases {
            let g = Grid::new(bpm);
            assert_eq!(g.n_bars, bars, "bpm {bpm}");
            let got = [
                g.bpm,
                g.beat,
                g.bar,
                g.s16,
                g.count[0].0,
                g.count[1].0,
                g.count[2].0,
                g.count[3].0,
                g.t_doors.0,
                g.t_doors.1,
                g.t_reassemble,
                g.t_glide,
                g.t_rev,
            ];
            for (g, w) in got.into_iter().zip(want) {
                same(g, w);
            }
            assert_eq!(g.t_fight.to_bits(), g.count[3].0.to_bits());
        }
    }

    /// The helpers on flute_vs_trombone's timeline, as Python printed them: beat_env (default decay and 0.05),
    /// beat_pulse (defaults and decay 0.2, pre 0.3), next_bar.
    #[test]
    fn helpers_match_python() {
        let g = Grid::new(132.0);
        #[rustfmt::skip]
        let cases: [(f64, [&str; 5]); 6] = [
            (0.0, ["1.0", "1.0", "1.0", "1.0", "0.0"]),
            (0.3, ["0.0820849986238988", "0.0024787521766663607", "0.0820849986238988", "0.5073527055958871",
                "1.7941176470588234"]),
            (1.7, ["0.05216041669824022", "0.0008348693652813541", "0.11949401059924251", "0.7664849869205664",
                "1.7941176470588234"]),
            (3.588, ["0.02385445435385442", "0.00012769290543851485", "0.9999884810517825", "0.9999981555208781",
                "3.5882352941176467"]),
            (12.34, ["0.1474574401381656", "0.010111138438879426", "0.1474574401381656", "0.3171027542389091",
                "12.558823529411764"]),
            (60.99, ["0.025876714603927355", "0.0001552322407040444", "0.980324074074102", "0.9967407407407455",
                "60.99999999999999"]),
        ];
        for (t, want) in cases {
            near(g.beat_env(t, 0.12), want[0]);
            near(g.beat_env(t, 0.05), want[1]);
            near(g.beat_pulse(t, 0.12, 0.12), want[2]);
            near(g.beat_pulse(t, 0.2, 0.3), want[3]);
            same(g.next_bar(t), want[4]);
        }
        same(g.bar_t(3.0), "5.38235294117647");
        same(g.bar_t(2.5), "4.485294117647058");
        // Python's math.ceil returns an int, so a bar just below zero is a positive zero.
        same(g.next_bar(-0.5), "0.0");
    }

    #[test]
    fn every_tempo_from_60_to_200_fills_61_s_with_whole_bars() {
        for bpm in 60..=200 {
            let g = Grid::new(f64::from(bpm));
            assert!(g.n_bars >= 1, "bpm {bpm}");
            assert!(
                (g.n_bars as f64 * g.bar - 61.0).abs() < 1e-9,
                "bpm {bpm}: {} bars of {}",
                g.n_bars,
                g.bar
            );
        }
    }

    #[test]
    fn an_old_episode_can_fight_from_bar_1() {
        let g = Grid::with_fight_bar(132.0, 1);
        assert_eq!(g.fight_bar, 1);
        assert_eq!(g.t_fight, g.bar);
        assert_eq!(g.count[0].0, 0.0);
        assert_eq!(g.count[3].0, g.bar);
    }
}
