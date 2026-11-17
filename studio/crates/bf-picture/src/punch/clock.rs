//! The micro hit-stop: the picture's clock slows to a crawl for a few frames on each hit and then catches up with
//! the audio's clock, so a hit lands with weight while the picture never drifts more than a fraction of a second.

use crate::config::FPS;

/// How fast the picture's clock runs while a hit holds (a fraction of real time).
const HOLD_RATE: f64 = 0.15;
/// The frames the picture takes to catch up after a hold.
const CATCH_FRAMES: f64 = 6.0;
/// No hold starts this close to the K.O. (the K.O. has its own hit-stop).
const KO_GUARD: f64 = 0.4;
/// A special's slow motion: the picture runs ahead of the sound for `SLOW_RUN` s, then runs at `SLOW_RATE` for
/// the last `SLOW_LEN` s into the impact, so the impact lands on its sound at full speed.
pub const SLOW_RUN: f64 = 1.1;
pub const SLOW_LEN: f64 = 0.6;
pub const SLOW_RATE: f64 = 0.3;

/// One hold: it starts at `t0`, lags by `peak` at `t0 + hold`, and is caught up at `t0 + hold + catch`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hold {
    t0: f64,
    hold: f64,
    peak: f64,
    catch: f64,
}

impl Hold {
    fn lag(&self, t: f64) -> f64 {
        let u = t - self.t0;
        if u <= 0.0 || u >= self.hold + self.catch {
            0.0
        } else if u < self.hold {
            u * (1.0 - HOLD_RATE)
        } else {
            self.peak * (1.0 - (u - self.hold) / self.catch)
        }
    }
}

/// The picture's clock of the Punchy look.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PunchClock {
    holds: Vec<Hold>,
    /// The specials' impacts that get a slow motion into them.
    slows: Vec<f64>,
}

/// How far a slow motion into an impact at `t_hit` puts the picture ahead at `t`.
fn slow_lead(t_hit: f64, t: f64) -> f64 {
    let tp = t_hit - SLOW_LEN;
    let ta = tp - SLOW_RUN;
    let peak = SLOW_LEN * (1.0 - SLOW_RATE);
    if t <= ta || t >= t_hit {
        0.0
    } else if t < tp {
        peak * crate::config::ss(ta, tp, t)
    } else {
        peak * (1.0 - (t - tp) / SLOW_LEN)
    }
}

impl PunchClock {
    /// The clock of `hits` (each `(time, strength 0..1)`), taking only the hits in `t_lo..t_hi - 0.4`.
    pub fn new(hits: &[(f64, f64)], t_lo: f64, t_hi: f64) -> PunchClock {
        PunchClock::with_slows(hits, &[], t_lo, t_hi)
    }

    /// The clock of `hits` with a slow motion into each impact of `slows` (dropped when it would start before
    /// `t_lo` or end too close to the K.O.); no hit holds while a slow motion runs.
    pub fn with_slows(hits: &[(f64, f64)], slows: &[f64], t_lo: f64, t_hi: f64) -> PunchClock {
        let frame = 1.0 / f64::from(FPS);
        let slows: Vec<f64> = slows
            .iter()
            .copied()
            .filter(|&th| th - SLOW_LEN - SLOW_RUN >= t_lo && th < t_hi - KO_GUARD)
            .collect();
        let in_slow = |t: f64| slows.iter().any(|&th| t >= th - SLOW_LEN - SLOW_RUN - 0.15 && t < th);
        let holds = hits
            .iter()
            .filter(|&&(t, s)| t >= t_lo && t < t_hi - KO_GUARD && s > 0.0 && !in_slow(t))
            .map(|&(t0, s)| {
                let hold = (1.0 + 3.0 * s.clamp(0.0, 1.0)).round() * frame;
                Hold {
                    t0,
                    hold,
                    peak: hold * (1.0 - HOLD_RATE),
                    catch: CATCH_FRAMES * frame,
                }
            })
            .collect();
        PunchClock { holds, slows }
    }

    /// How far the picture is behind real time at `t` (the largest lag of the holds at work).
    pub fn lag(&self, t: f64) -> f64 {
        self.holds.iter().map(|h| h.lag(t)).fold(0.0, f64::max)
    }

    /// The picture's time at real time `t`.
    pub fn at(&self, t: f64) -> f64 {
        t - self.lag(t) + self.slows.iter().map(|&th| slow_lead(th, t)).fold(0.0, f64::max)
    }

    /// How slow the picture runs at `t` (1 at full speed, [`SLOW_RATE`] deep in a slow motion), 0..1 of the
    /// slow motion's depth.
    pub fn slow_k(&self, t: f64) -> f64 {
        self.slows
            .iter()
            .map(|&th| if t >= th - SLOW_LEN && t < th { 1.0 } else { 0.0 })
            .fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_hits_it_is_real_time() {
        let c = PunchClock::new(&[], 3.0, 50.0);
        for t in [0.0, 3.5, 20.0, 61.0] {
            assert_eq!(c.at(t), t);
        }
    }

    #[test]
    fn it_holds_then_catches_up() {
        let c = PunchClock::new(&[(10.0, 1.0)], 3.0, 50.0);
        assert_eq!(c.at(9.99), 9.99);
        assert!(c.at(10.1) < 10.1 - 0.05, "held: {}", c.at(10.1));
        assert_eq!(c.at(10.5), 10.5, "caught up within a third of a second");
    }

    #[test]
    fn a_dense_burst_stays_monotonic_and_close() {
        let hits: Vec<(f64, f64)> = (0..40).map(|i| (10.0 + f64::from(i) * 0.05, 1.0)).collect();
        let c = PunchClock::new(&hits, 3.0, 50.0);
        let mut last = f64::MIN;
        for i in 0..3000 {
            let t = 9.0 + f64::from(i) / 1000.0;
            let tp = c.at(t);
            assert!(tp >= last, "monotonic at {t}");
            assert!(c.lag(t) <= 0.25, "lag {} at {t}", c.lag(t));
            last = tp;
        }
    }

    #[test]
    fn a_slow_motion_lands_on_time_and_stays_monotonic() {
        let c = PunchClock::with_slows(&[(20.0, 1.0), (19.8, 1.0)], &[20.0], 3.0, 50.0);
        assert_eq!(c.at(18.0), 18.0);
        assert!(c.at(19.4) > 19.4 + 0.3, "ahead before the slow part: {}", c.at(19.4));
        let mut last = f64::MIN;
        for i in 0..4000 {
            let t = 17.5 + f64::from(i) / 1000.0;
            let tp = c.at(t);
            assert!(tp >= last, "monotonic at {t}");
            last = tp;
        }
        // deep in it, the picture crawls
        let rate = (c.at(19.7) - c.at(19.6)) / 0.1;
        assert!((rate - SLOW_RATE).abs() < 1e-9, "rate {rate}");
    }

    #[test]
    fn no_hold_near_the_ko_or_before_the_fight() {
        let c = PunchClock::new(&[(2.0, 1.0), (49.8, 1.0)], 3.0, 50.0);
        assert_eq!(c.at(2.05), 2.05);
        assert_eq!(c.at(49.9), 49.9);
        assert_eq!(c.at(50.0), 50.0);
    }
}
