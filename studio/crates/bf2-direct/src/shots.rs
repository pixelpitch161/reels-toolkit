//! Shot requests: what the events ask the camera for, with a priority each, and the rule that keeps big shots
//! apart so the minute breathes.
//!
//! The recording does not carry the moves' tags, so a move's weight is read from what it did: a move whose events
//! hold a `SlowMo` cue is a special; one with a `PunchIn` cue, a contact of strength 0.6 or more, a launch or a
//! slam is heavy; anything else is light.

use bf2_sim::{Cue, Ev, EvKind};

use crate::cfg::DirectCfg;
use crate::time::TimeMap;

/// A shot's priority: a higher one wins a clash and may cut into a big shot's spacing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Light,
    Heavy,
    Special,
    Ko,
}

/// The weight of the move each event belongs to (its fighter's move from the last `MoveStart`), `Light` for an
/// event outside any move.
pub fn move_class(events: &[Ev]) -> Vec<Priority> {
    // Each event's move instance: the index of its fighter's last MoveStart.
    let mut current: [Option<usize>; 2] = [None, None];
    let mut inst = vec![None; events.len()];
    let mut class = vec![Priority::Light; events.len()];
    for (i, e) in events.iter().enumerate() {
        let who = e.who.min(1);
        if matches!(e.kind, EvKind::MoveStart { .. }) {
            current[who] = Some(i);
        }
        inst[i] = current[who];
        let Some(m) = current[who] else { continue };
        let c = match e.kind {
            EvKind::Cue(Cue::SlowMo { .. }) => Priority::Special,
            EvKind::Cue(Cue::PunchIn) | EvKind::Launch { .. } | EvKind::Slam { .. } => Priority::Heavy,
            EvKind::Contact { strength, .. } if strength >= 0.6 => Priority::Heavy,
            _ => Priority::Light,
        };
        class[m] = class[m].max(c);
    }
    inst.iter().map(|m| m.map_or(Priority::Light, |m| class[m])).collect()
}

/// What a shot does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShotKind {
    /// Closer (distance × 0.62) and toward the attacker over the wind-up; holds through the strike.
    PunchIn,
    /// Roll this many degrees.
    Tilt(f64),
    /// Back to the base shot fast, with a 6 % overshoot.
    WhipBack,
    /// Follow the launched ball up, the camera 8 degrees lower.
    Launch,
    /// A low angle, 14 degrees lower, after a slam.
    Slam,
    /// A quick push-in to 0.9.
    Clash,
    /// The K.O.: push in on the loser and orbit 12 degrees through the freeze and slow, then pull out wide on the
    /// winner.
    Ko,
}

impl ShotKind {
    /// Big shots keep `big_shot_gap` apart.
    pub fn is_big(self) -> bool {
        matches!(self, ShotKind::PunchIn | ShotKind::Slam | ShotKind::Ko)
    }
}

/// One accepted shot, in video seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub kind: ShotKind,
    /// The ball it looks at: the attacker, the launched or slammed ball, or the K.O.'s loser.
    pub subject: usize,
    pub priority: Priority,
    pub t0: f64,
    /// When the build-up is done: a punch-in's strike, the end of the K.O.'s orbit.
    pub t_peak: f64,
    /// When it lets go (the camera springs back from there). The K.O. shot never lets go.
    pub t1: f64,
}

/// The step of the first event after index `i` by `who` that matches.
fn next_step(events: &[Ev], i: usize, who: usize, f: impl Fn(&EvKind) -> bool) -> Option<u32> {
    events[i + 1..]
        .iter()
        .find(|e| e.who == who && f(&e.kind))
        .map(|e| e.step)
}

/// Every shot the events ask for, with the big-shot spacing applied: a big shot within `cfg.big_shot_gap` of the
/// last accepted big one is refused unless its priority is higher, in which case it cuts the earlier one short.
/// A whip-back is kept only after an accepted punch-in of its fighter, and ends it. Nothing but the K.O. shot starts after the
/// K.O., and every other shot stops there.
pub fn plan(events: &[Ev], map: &TimeMap, cfg: &DirectCfg) -> Vec<Shot> {
    let class = move_class(events);
    let vt = |s: u32| map.step_video_t(s);
    let mut want: Vec<Shot> = Vec::new();
    for (i, e) in events.iter().enumerate() {
        let t0 = vt(e.step);
        let shot = |kind, subject, priority, t_peak: f64, t1: f64| Shot {
            kind,
            subject,
            priority,
            t0,
            t_peak: t_peak.max(t0),
            t1: t1.max(t_peak.max(t0)),
        };
        let p = class[i];
        let s = match e.kind {
            EvKind::Cue(Cue::PunchIn) => {
                let active = next_step(events, i, e.who, |k| matches!(k, EvKind::Active { .. })).map(vt);
                let rec = next_step(events, i, e.who, |k| matches!(k, EvKind::Recovery { .. })).map(vt);
                let t_peak = active.unwrap_or(t0 + 0.3);
                // Held through the strike and a beat after, so the push-in reads.
                let t1 = rec.unwrap_or(t_peak + 0.2).max(t_peak + 0.45);
                shot(ShotKind::PunchIn, e.who, p.max(Priority::Heavy), t_peak, t1)
            }
            EvKind::Cue(Cue::Tilt(a)) => {
                let rec = next_step(events, i, e.who, |k| matches!(k, EvKind::Recovery { .. })).map(vt);
                shot(ShotKind::Tilt(a), e.who, p, t0 + 0.15, rec.unwrap_or(t0 + 0.5))
            }
            EvKind::Cue(Cue::WhipBack) => shot(ShotKind::WhipBack, e.who, p, t0, t0 + 0.12),
            EvKind::Launch { target, .. } => {
                let land = next_step(events, i, target, |k| matches!(k, EvKind::Land)).map(vt);
                let t1 = land.unwrap_or(t0 + 1.2).min(t0 + 2.0);
                shot(ShotKind::Launch, target, p.max(Priority::Heavy), t0 + 0.1, t1)
            }
            EvKind::Slam { target } => shot(ShotKind::Slam, target, p.max(Priority::Heavy), t0 + 0.06, t0 + 0.6),
            EvKind::Clash { .. } => shot(ShotKind::Clash, e.who, p, t0 + 0.08, t0 + 0.25),
            EvKind::Ko { target } => {
                let fk = e.step as f64 * map.dt();
                let slow_end = map.video_t(fk + cfg.ko_slow_len);
                shot(ShotKind::Ko, target, Priority::Ko, slow_end, f64::INFINITY)
            }
            _ => continue,
        };
        want.push(s);
    }
    want.sort_by(|a, b| a.t0.total_cmp(&b.t0));

    let ko_t = want.iter().find(|s| s.kind == ShotKind::Ko).map(|s| s.t0);
    let mut out: Vec<Shot> = Vec::new();
    let mut last_big: Option<usize> = None;
    for s in want {
        if let Some(k) = ko_t
            && s.kind != ShotKind::Ko
            && s.t0 >= k
        {
            continue;
        }
        if s.kind == ShotKind::WhipBack {
            // Only after an accepted punch-in of its fighter, which it ends.
            let Some(p) = out
                .iter_mut()
                .rev()
                .find(|p| p.kind == ShotKind::PunchIn && p.subject == s.subject && p.t1 >= s.t0 - 0.3)
            else {
                continue;
            };
            p.t1 = p.t1.min(s.t0).max(p.t0);
            p.t_peak = p.t_peak.min(p.t1);
        }
        if s.kind.is_big() {
            if let Some(b) = last_big {
                let prev = out[b];
                if s.t0 - prev.t0 < cfg.big_shot_gap && s.priority <= prev.priority {
                    continue;
                }
                if prev.t1 > s.t0 {
                    out[b].t1 = s.t0;
                    out[b].t_peak = out[b].t_peak.min(s.t0);
                }
            }
            last_big = Some(out.len());
        }
        out.push(s);
    }
    if let Some(k) = ko_t {
        for s in out.iter_mut().filter(|s| s.kind != ShotKind::Ko) {
            s.t1 = s.t1.min(k);
            s.t_peak = s.t_peak.min(s.t1);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::*;

    #[test]
    fn moves_are_weighed_by_what_they_did() {
        let ev = [
            ev(0, 0, EvKind::MoveStart { mv: 0 }),
            ev(0, 0, EvKind::Windup { mv: 0 }),
            ev(5, 0, EvKind::Active { mv: 0 }),
            ev(10, 1, EvKind::MoveStart { mv: 1 }),
            ev(10, 1, EvKind::Cue(Cue::PunchIn)),
            ev(20, 0, EvKind::MoveStart { mv: 4 }),
            ev(50, 0, EvKind::Cue(Cue::SlowMo { scale: 0.3 })),
            ev(60, 1, EvKind::Land),
        ];
        let c = move_class(&ev);
        assert_eq!(&c[..3], &[Priority::Light; 3]);
        assert_eq!(&c[3..5], &[Priority::Heavy; 2]);
        assert_eq!(&c[5..7], &[Priority::Special; 2]);
        assert_eq!(c[7], Priority::Heavy, "an event of the fighter's current move");
    }
}
