//! The time map: one monotonic map between fight time and video time, built from the events.
//!
//! The map is a list of fight-time segments, each with a constant speed (fight seconds per video second): 1 is
//! normal, 0.25 is 4x slow, and 0 is a freeze that lasts a set video time at one fight time. Ramps are 6 constant
//! sub-segments, so the map stays exactly invertible. Hitstop is already in the fight (bodies freeze while fight
//! time runs), so this adds only slow-mo and the K.O. cinematic.
//!
//! Fight time is measured from the fight start (step `n` is at `n·dt`); video time is absolute (`t_fight`
//! included). Before the fight and after the last segment, the map is the identity shifted by what came before.

use bf2_sim::{Cue, Ev, EvKind, Recording2};

use crate::cfg::DirectCfg;

/// Sub-segments of each ramp.
const RAMP_STEPS: usize = 6;

/// One stretch of fight time at a constant speed. A freeze has `f0 == f1` and speed 0.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Seg {
    f0: f64,
    f1: f64,
    v0: f64,
    v1: f64,
    speed: f64,
}

/// The map between fight time and video time.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeMap {
    t_fight: f64,
    dt: f64,
    segs: Vec<Seg>,
    ko: Option<f64>,
}

/// One slow-mo: ramps in from `start` to `at`, holds `scale` to `hold`, ramps out to `end` (all fight seconds).
#[derive(Clone, Copy, Debug)]
struct Slow {
    start: f64,
    at: f64,
    hold: f64,
    end: f64,
    scale: f64,
    /// The cue's step, to drop the later of two equally weak cues first.
    step: u32,
}

impl Slow {
    /// The breakpoints of its constant pieces.
    fn breaks(&self, out: &mut Vec<f64>) {
        let n = RAMP_STEPS as f64;
        for i in 0..=RAMP_STEPS {
            out.push(self.start + (self.at - self.start) * i as f64 / n);
            out.push(self.hold + (self.end - self.hold) * i as f64 / n);
        }
    }

    /// Its speed at `f`, inside `start..end` (outside, 1). Each ramp piece takes the ramp's value at its middle.
    fn speed(&self, f: f64) -> f64 {
        let n = RAMP_STEPS as f64;
        let piece = |a: f64, b: f64| (((f - a) / (b - a) * n).floor()).clamp(0.0, n - 1.0);
        if f < self.start || f >= self.end {
            1.0
        } else if f < self.at {
            1.0 + (self.scale - 1.0) * (piece(self.start, self.at) + 0.5) / n
        } else if f < self.hold {
            self.scale
        } else {
            1.0 + (self.scale - 1.0) * (1.0 - (piece(self.hold, self.end) + 0.5) / n)
        }
    }
}

/// The elementary pieces `(f0, f1, speed)` covering `0..end` that the slow-mos give, the slowest winning where
/// they overlap, with equal neighbours merged.
fn pieces(slows: &[Slow], end: f64, breaks: &mut Vec<f64>) -> Vec<(f64, f64, f64)> {
    breaks.clear();
    breaks.push(0.0);
    breaks.push(end);
    for s in slows {
        s.breaks(breaks);
    }
    breaks.retain(|b| *b >= 0.0);
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    let mut out: Vec<(f64, f64, f64)> = Vec::with_capacity(breaks.len());
    for w in breaks.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mid = 0.5 * (a + b);
        let speed = slows.iter().map(|s| s.speed(mid)).fold(1.0, f64::min);
        match out.last_mut() {
            Some(last) if last.2 == speed && last.1 == a => last.1 = b,
            _ => out.push((a, b, speed)),
        }
    }
    out
}

/// The video seconds these pieces add.
fn added(pieces: &[(f64, f64, f64)]) -> f64 {
    pieces.iter().map(|&(a, b, s)| (b - a) * (1.0 / s - 1.0)).sum()
}

impl TimeMap {
    /// The map of a fight from its events alone (cheap: the seed search builds one per seed). `steps` is the
    /// fight's step count, `ko` the K.O. step.
    ///
    /// A `SlowMo { scale }` cue ramps in over `cfg.ramp_in` before its step, holds `scale` until its fighter's next
    /// `Recovery` plus `cfg.hold_after`, then ramps out over `cfg.ramp_out`. The K.O. freezes for `cfg.ko_freeze`
    /// video seconds, then runs at `cfg.ko_slow` for `cfg.ko_slow_len` and ramps out. Where slow-mos overlap the
    /// slowest wins. While the added video time is over `cfg.max_extra`, the weakest cue (the later of equals) is
    /// dropped; the K.O. cinematic never is.
    pub fn build(rec_events: &[Ev], steps: u32, dt: f64, t_fight: f64, ko: Option<u32>, cfg: &DirectCfg) -> TimeMap {
        let mut cues: Vec<Slow> = Vec::new();
        if cfg.slowmo {
            for (i, e) in rec_events.iter().enumerate() {
                let EvKind::Cue(Cue::SlowMo { scale }) = e.kind else {
                    continue;
                };
                if !(scale > 0.0 && scale < 1.0) {
                    continue;
                }
                let at = e.step as f64 * dt;
                let rec = rec_events[i..]
                    .iter()
                    .find(|r| r.who == e.who && matches!(r.kind, EvKind::Recovery { .. }))
                    .map_or(at, |r| r.step as f64 * dt);
                let hold = rec.max(at) + cfg.hold_after;
                cues.push(Slow {
                    start: at - cfg.ramp_in,
                    at,
                    hold,
                    end: hold + cfg.ramp_out,
                    scale: scale.max(0.05),
                    step: e.step,
                });
            }
        }
        let ko_f = ko.map(|k| k as f64 * dt);
        let ko_slow = ko_f.map(|fk| Slow {
            start: fk,
            at: fk,
            hold: fk + cfg.ko_slow_len,
            end: fk + cfg.ko_slow_len + cfg.ramp_out,
            scale: cfg.ko_slow.clamp(0.05, 1.0),
            step: u32::MAX,
        });
        let freeze = if ko_f.is_some() { cfg.ko_freeze.max(0.0) } else { 0.0 };
        let last_ev = rec_events.iter().map(|e| e.step).max().map_or(0.0, |s| s as f64 * dt);
        let end = (steps as f64 * dt).max(last_ev);

        // The weakest first, the later of equals first: the order cues are dropped in.
        let mut order: Vec<usize> = (0..cues.len()).collect();
        order.sort_by(|&a, &b| {
            cues[b]
                .scale
                .total_cmp(&cues[a].scale)
                .then(cues[b].step.cmp(&cues[a].step))
        });
        let mut keep = vec![true; cues.len()];
        let mut slows: Vec<Slow> = Vec::with_capacity(cues.len() + 1);
        let mut breaks = Vec::new();
        let mut dropped = 0;
        let pcs = loop {
            slows.clear();
            slows.extend(cues.iter().zip(&keep).filter(|(_, k)| **k).map(|(c, _)| *c));
            slows.extend(ko_slow);
            let pcs = pieces(&slows, end, &mut breaks);
            if freeze + added(&pcs) <= cfg.max_extra || dropped == order.len() {
                break pcs;
            }
            keep[order[dropped]] = false;
            dropped += 1;
        };

        let mut segs = Vec::with_capacity(pcs.len() + 1);
        let mut v = t_fight;
        let push_freeze = |segs: &mut Vec<Seg>, v: &mut f64, at: f64| {
            segs.push(Seg {
                f0: at,
                f1: at,
                v0: *v,
                v1: *v + freeze,
                speed: 0.0,
            });
            *v += freeze;
        };
        let mut frozen = ko_f.is_none() || freeze == 0.0;
        for &(a, b, speed) in &pcs {
            if !frozen && ko_f.is_some_and(|fk| a >= fk) {
                push_freeze(&mut segs, &mut v, a);
                frozen = true;
            }
            let v1 = v + (b - a) / speed;
            segs.push(Seg {
                f0: a,
                f1: b,
                v0: v,
                v1,
                speed,
            });
            v = v1;
        }
        if !frozen && let Some(fk) = ko_f {
            push_freeze(&mut segs, &mut v, fk.max(end));
        }
        TimeMap {
            t_fight,
            dt,
            segs,
            ko: ko_f,
        }
    }

    /// [`TimeMap::build`] from a whole recording; the K.O. step is its first `Ko` event.
    pub fn from_recording(rec: &Recording2, cfg: &DirectCfg) -> TimeMap {
        let ko = rec
            .events
            .iter()
            .find(|e| matches!(e.kind, EvKind::Ko { .. }))
            .map(|e| e.step);
        TimeMap::build(&rec.events, rec.steps.len() as u32, rec.dt, rec.t_fight, ko, cfg)
    }

    /// The video time at which fight time `fight_t` is shown. A freeze's fight time maps to the freeze's start.
    pub fn video_t(&self, fight_t: f64) -> f64 {
        let (Some(first), Some(last)) = (self.segs.first(), self.segs.last()) else {
            return self.t_fight + fight_t;
        };
        if fight_t < first.f0 {
            return first.v0 + (fight_t - first.f0);
        }
        if fight_t > last.f1 {
            return last.v1 + (fight_t - last.f1);
        }
        let s = &self.segs[self.segs.partition_point(|s| s.f1 < fight_t)];
        if s.speed == 0.0 {
            s.v0
        } else {
            (s.v0 + (fight_t - s.f0) / s.speed).min(s.v1)
        }
    }

    /// The fight time shown at video time `video_t`; constant through a freeze.
    pub fn fight_t(&self, video_t: f64) -> f64 {
        let (Some(first), Some(last)) = (self.segs.first(), self.segs.last()) else {
            return video_t - self.t_fight;
        };
        if video_t < first.v0 {
            return first.f0 + (video_t - first.v0);
        }
        if video_t > last.v1 {
            return last.f1 + (video_t - last.v1);
        }
        let s = &self.segs[self.segs.partition_point(|s| s.v1 < video_t)];
        (s.f0 + (video_t - s.v0) * s.speed).min(s.f1)
    }

    /// Fight seconds per video second at `video_t`: 1 normally, below 1 in slow-mo, 0 in a freeze. At a boundary
    /// it is the rate of the stretch that starts there.
    pub fn rate(&self, video_t: f64) -> f64 {
        let i = self.segs.partition_point(|s| s.v1 <= video_t);
        match self.segs.get(i) {
            Some(s) if video_t >= s.v0 => s.speed,
            _ => 1.0,
        }
    }

    /// Fight seconds per step.
    pub fn dt(&self) -> f64 {
        self.dt
    }

    /// The video time of step `step`.
    pub fn step_video_t(&self, step: u32) -> f64 {
        self.video_t(step as f64 * self.dt)
    }

    /// The video time of the K.O. (the start of its freeze).
    pub fn ko_video_t(&self) -> Option<f64> {
        self.ko.map(|f| self.video_t(f))
    }

    /// The video seconds slow-mo and the K.O. cinematic add, all together.
    pub fn extra(&self) -> f64 {
        self.segs.iter().map(|s| (s.v1 - s.v0) - (s.f1 - s.f0)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::*;
    use bf_core::rng::Generator;

    fn map(events: &[Ev], ko: Option<u32>) -> TimeMap {
        TimeMap::build(events, 6000, DT, t_fight(), ko, &DirectCfg::default())
    }

    #[test]
    fn identity_without_cues() {
        let m = map(&[recovery(50, 0)], None);
        let tf = t_fight();
        for f in [-1.0, 0.0, 0.3, 12.5, 49.99, 80.0] {
            assert!((m.video_t(f) - (tf + f)).abs() < 1e-12, "{f}");
            assert!((m.fight_t(tf + f) - f).abs() < 1e-12, "{f}");
            assert_eq!(m.rate(tf + f), 1.0);
        }
        assert_eq!(m.extra(), 0.0);
        assert_eq!(m.ko_video_t(), None);
        assert!((m.step_video_t(240) - (tf + 2.0)).abs() < 1e-12);
    }

    #[test]
    fn slowmo_stretches_video() {
        // A 0.25 cue at 5 s, its move's recovery 0.15 s later: held 0.4 fight-s.
        let (s, r) = (600, 618);
        let m = map(&[slowmo(s, 1, 0.25), recovery(r, 1)], None);
        let (fc, fe) = (s as f64 * DT, r as f64 * DT + 0.25);
        let held = m.video_t(fe) - m.video_t(fc);
        assert!((held - 1.6).abs() < 1e-9, "{held}");
        let extra = m.extra();
        assert!(extra > 1.2 && extra < 1.2 + 0.35 * 3.0, "{extra}");
        // ramping: slower and slower into the cue, never below 0.25
        let v = m.video_t(fc);
        assert!((m.rate(v + 0.01) - 0.25).abs() < 1e-12);
        let r1 = m.rate(m.video_t(fc - 0.14));
        let r2 = m.rate(m.video_t(fc - 0.02));
        assert!(r1 < 1.0 && r2 < r1 && r2 > 0.25, "{r1} {r2}");
        assert_eq!(m.rate(m.video_t(fc - 0.2)), 1.0);
        assert_eq!(m.rate(m.video_t(fe + 0.25)), 1.0);
        // normal speed resumes with the slow-mo's extra time behind it
        assert!((m.video_t(30.0) - (t_fight() + 30.0 + extra)).abs() < 1e-9);
    }

    fn sample_map() -> TimeMap {
        map(
            &[
                slowmo(600, 1, 0.25),
                recovery(618, 1),
                slowmo(1500, 0, 0.4),
                slowmo(1530, 1, 0.3),
                recovery(1560, 0),
                recovery(1590, 1),
                slowmo(4790, 0, 0.3),
                recovery(4800, 0),
            ],
            Some(4800),
        )
    }

    #[test]
    fn round_trips_everywhere() {
        let m = sample_map();
        let mut g = Generator::new(5);
        let ko = m.ko_video_t().unwrap();
        let end = m.video_t(55.0);
        for _ in 0..10_000 {
            let f = g.uniform(-1.0, 55.0);
            assert!((m.fight_t(m.video_t(f)) - f).abs() < 1e-9, "fight {f}");
            let v = g.uniform(t_fight() - 1.0, end);
            let back = m.video_t(m.fight_t(v));
            if v > ko && v < ko + 0.35 {
                // inside the freeze: its one fight time maps back to its start
                assert!((back - ko).abs() < 1e-9, "freeze {v}");
            } else {
                assert!((back - v).abs() < 1e-9, "video {v}");
            }
        }
        // monotonic
        let mut last = f64::NEG_INFINITY;
        for i in 0..20_000 {
            let v = t_fight() - 1.0 + i as f64 * 0.003;
            let f = m.fight_t(v);
            assert!(f >= last, "{v}");
            last = f;
        }
    }

    #[test]
    fn overlapping_cues_merge() {
        // Two cues 0.25 s apart, overlapping: never slower than the slowest, and the time adds less than the two
        // alone would.
        let a = [slowmo(1500, 0, 0.4), recovery(1560, 0)];
        let b = [slowmo(1530, 1, 0.3), recovery(1590, 1)];
        let both: Vec<Ev> = a.iter().chain(&b).cloned().collect();
        let (ma, mb, m) = (map(&a, None), map(&b, None), map(&both, None));
        assert!(m.extra() < ma.extra() + mb.extra());
        assert!(m.extra() >= ma.extra().max(mb.extra()));
        for i in 0..4000 {
            let v = t_fight() + 11.0 + i as f64 * 0.001;
            assert!(m.rate(v) >= 0.3 - 1e-12, "{v}");
        }
        // the K.O. slow and a special at the K.O. merge too
        let m = map(&[slowmo(4790, 0, 0.3), recovery(4800, 0)], Some(4800));
        let ko = m.ko_video_t().unwrap();
        assert!((m.rate(ko + 0.4) - 0.3).abs() < 1e-12);
        assert!(m.extra() <= DirectCfg::default().max_extra);
    }

    #[test]
    fn extra_time_is_capped() {
        let mut ev = Vec::new();
        for i in 0..20 {
            let s = 300 + i * 240;
            // the later ones weaker
            ev.push(slowmo(s, 0, 0.5 + i as f64 * 0.01));
            ev.push(recovery(s + 10, 0));
        }
        let m = map(&ev, Some(5400));
        let cap = DirectCfg::default().max_extra;
        assert!(m.extra() <= cap + 1e-9, "{}", m.extra());
        assert!(m.extra() > cap - 0.6, "{}", m.extra());
        // the first, strongest cue survived; the last, weakest one went
        assert!(m.rate(m.video_t(300.0 * DT + 0.1)) < 1.0);
        let last = (300 + 19 * 240) as f64 * DT;
        assert_eq!(m.rate(m.video_t(last + 0.1)), 1.0);
        // the K.O. is never dropped
        assert!(m.ko_video_t().is_some());
        let ko = m.ko_video_t().unwrap();
        assert_eq!(m.rate(ko + 0.1), 0.0);
    }

    #[test]
    fn ko_freezes_then_slows() {
        let m = map(&[], Some(4800));
        let ko = m.ko_video_t().unwrap();
        let fk = 4800.0 * DT;
        assert!((ko - (t_fight() + fk)).abs() < 1e-9);
        for i in 0..35 {
            let v = ko + 0.005 + i as f64 * 0.0098;
            assert_eq!(m.rate(v), 0.0, "{v}");
            assert!((m.fight_t(v) - fk).abs() < 1e-12);
        }
        assert!((m.rate(ko + 0.36) - 0.3).abs() < 1e-12);
        // 0.8 fight-s at 0.3x take 8/3 video-s
        let slow_end = ko + 0.35 + 0.8 / 0.3;
        assert!((m.rate(slow_end - 0.01) - 0.3).abs() < 1e-12);
        assert!((m.fight_t(slow_end) - (fk + 0.8)).abs() < 1e-9);
        assert!(m.rate(slow_end + 0.01) > 0.3);
        assert_eq!(m.rate(slow_end + 1.0), 1.0);
        assert!((m.step_video_t(4800) - ko).abs() < 1e-12);
    }

    #[test]
    fn no_ko_no_freeze() {
        let m = map(&[slowmo(600, 1, 0.25), recovery(618, 1)], None);
        assert_eq!(m.ko_video_t(), None);
        for i in 0..60_000 {
            let v = t_fight() + i as f64 * 0.001;
            assert!(m.rate(v) > 0.0, "{v}");
        }
    }

    #[test]
    fn rate_is_zero_only_in_freezes() {
        let m = sample_map();
        let ko = m.ko_video_t().unwrap();
        for i in 0..70_000 {
            let v = t_fight() - 1.0 + i as f64 * 0.001;
            let freeze = v >= ko && v < ko + 0.35;
            assert_eq!(m.rate(v) == 0.0, freeze, "{v}");
        }
    }

    #[test]
    fn slowmo_off_is_identity_but_ko() {
        let cfg = DirectCfg {
            slowmo: false,
            ..DirectCfg::default()
        };
        let ev = [slowmo(600, 1, 0.25), recovery(618, 1)];
        let m = TimeMap::build(&ev, 6000, DT, t_fight(), None, &cfg);
        assert_eq!(m.extra(), 0.0);
        assert!((m.video_t(20.0) - (t_fight() + 20.0)).abs() < 1e-12);
        let m = TimeMap::build(&ev, 6000, DT, t_fight(), Some(4800), &cfg);
        let ko = m.ko_video_t().unwrap();
        assert!((ko - (t_fight() + 40.0)).abs() < 1e-9, "no slow-mo before the K.O.");
        assert_eq!(m.rate(ko + 0.1), 0.0);
        assert!(m.extra() > 0.35);
    }

    #[test]
    fn from_recording_finds_the_ko() {
        let rec = dummy(4, false);
        let m = TimeMap::from_recording(&rec, &DirectCfg::default());
        match rec.outcome.t_ko {
            Some(t) => assert!(m.ko_video_t().unwrap() >= t),
            None => assert_eq!(m.ko_video_t(), None),
        }
        assert!(m.extra() <= 4.0 + 1e-9);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn builds_fast() {
        let rec = dummy(7, false);
        let ko = rec
            .events
            .iter()
            .find(|e| matches!(e.kind, EvKind::Ko { .. }))
            .map(|e| e.step);
        let steps = rec.events.last().map_or(0, |e| e.step + 1);
        let cfg = DirectCfg::default();
        let n = 1000;
        let t = std::time::Instant::now();
        let mut sink = 0.0;
        for _ in 0..n {
            let m = TimeMap::build(std::hint::black_box(&rec.events), steps, rec.dt, rec.t_fight, ko, &cfg);
            sink += m.extra();
        }
        let per = t.elapsed().as_secs_f64() / n as f64;
        std::hint::black_box(sink);
        assert!(per < 50e-6, "{} µs a build", per * 1e6);
    }
}
