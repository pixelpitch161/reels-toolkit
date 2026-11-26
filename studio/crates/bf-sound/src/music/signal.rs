//! The story signal (milestone 5, Task 1): what the composed arrangement listens to. Per bar of the song, who
//! leads and by how much, and the fight's moments on the sixteenth grid: each lead change, each captioned
//! ability, each desperation move, the start of the final exchange, the K.O., the victory bars and the bars that
//! build back into frame 0.
//!
//! The lead follows the story's own rule (`kit/story.py`, `bf-story`): it is replayed over the `hit` and `heal`
//! events, a fighter takes the lead once it is more than 5 HP ahead, and a change counts from 4 s on. The HP gap
//! of a bar is read with [`MusicCtx::hp_at`] at the bar's end.

use bf_core::py::round_half_even;

use super::events::{event_int, event_is_str, event_number};
use super::{MusicCtx, MusicError};

/// A lead needs more than this many HP of difference.
const LEAD_HYST: f64 = 5.0;
/// Lead changes before this time (s) do not count.
const LEAD_FROM: f64 = 4.0;
/// The final exchange: this many bars before the K.O.
const FINAL_BARS: f64 = 2.0;

/// Which part of the song a bar belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// The title card and the count-in, before FIGHT!.
    Intro,
    /// The fight, up to the final exchange.
    Fight,
    /// The last 2 bars before the K.O.
    FinalExchange,
    /// From the K.O. to the winner's solo.
    Ko,
    /// The winner's solo.
    Victory,
    /// The last bars, which build back into frame 0.
    BuildBack,
}

/// One bar of the song.
#[derive(Clone, Debug, PartialEq)]
pub struct BarSignal {
    /// The bar's number, from 0.
    pub bar: usize,
    /// When it starts, in seconds.
    pub t: f64,
    /// Who leads at the bar's end (`None`: nobody yet).
    pub leader: Option<usize>,
    /// The left fighter's HP minus the right's, at the bar's end.
    pub gap: f64,
    /// The part of the song.
    pub phase: Phase,
}

/// What happens at a moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MomentKind {
    /// The lead passes to this fighter.
    LeadChange { to: usize },
    /// A captioned ability.
    Ability { who: usize, name: String },
    /// A use of a fighter's low (desperation) ability.
    Desperation { who: usize },
    /// The start of the last 2 bars before the K.O.
    FinalExchange,
    /// The K.O.
    Ko,
    /// The start of a bar of the winner's solo.
    Victory,
    /// The start of a bar that builds back into frame 0.
    BuildBack,
}

/// A moment on the sixteenth grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Moment {
    /// The sixteenth it lands on, counted from 0.
    pub step: usize,
    /// Its time, `step` sixteenths.
    pub t: f64,
    /// The bar it falls in.
    pub bar: usize,
    /// What happens.
    pub kind: MomentKind,
}

/// The story signal of one soundtrack build.
#[derive(Clone, Debug, PartialEq)]
pub struct StorySignal {
    /// Every bar of the song, in order.
    pub bars: Vec<BarSignal>,
    /// Every moment, in time order (moments at the same sixteenth keep the order of the list above).
    pub moments: Vec<Moment>,
}

impl StorySignal {
    /// The story signal of `ctx`'s run.
    ///
    /// # Errors
    /// A `hit`, `heal` or `ability` event lacks an element the signal reads.
    pub fn from(ctx: &MusicCtx) -> Result<StorySignal, MusicError> {
        const H: &str = "story_signal";
        let show = ctx.show;
        let grid = &show.grid;
        let n_bars = grid.n_bars;
        let step_of = |t: f64| round_half_even(t / grid.s16).max(0.0) as usize;
        let bar_of = |t: f64| ((t / grid.bar + 1e-9).floor().max(0.0) as usize).min(n_bars.saturating_sub(1));
        let mut moments: Vec<(u8, Moment)> = Vec::new();
        let mut push = |order: u8, t: f64, kind: MomentKind| {
            let step = step_of(t);
            let t = step as f64 * grid.s16;
            moments.push((
                order,
                Moment {
                    step,
                    t,
                    bar: bar_of(t),
                    kind,
                },
            ));
        };

        // the lead, replayed over the hits and heals up to the K.O.
        let mut hp = [100.0f64; 2];
        let mut lead: Option<usize> = None;
        let mut leads: Vec<(f64, usize)> = Vec::new();
        let who = |e, i| -> Result<usize, MusicError> {
            let k = event_int(H, e, i)?;
            usize::try_from(k)
                .ok()
                .filter(|&k| k < 2)
                .ok_or_else(|| MusicError::new(H, format!("fighter {k} at {} s", e.t)))
        };
        for e in &ctx.rec.events {
            if e.t > show.t_ko + 0.01 {
                break;
            }
            match e.kind.as_str() {
                "heal" => hp[who(e, 2)?] += event_number(H, e, 3)?,
                "hit" => {
                    hp[who(e, 3)?] -= event_number(H, e, 4)?;
                    let d = hp[0] - hp[1];
                    let new = if d > LEAD_HYST {
                        Some(0)
                    } else if d < -LEAD_HYST {
                        Some(1)
                    } else {
                        lead
                    };
                    if new != lead {
                        let to = new.expect("a lead is only ever taken, never dropped");
                        if lead.is_some() && e.t >= LEAD_FROM {
                            push(0, e.t, MomentKind::LeadChange { to });
                        }
                        lead = new;
                        leads.push((e.t, to));
                    }
                }
                "ability" => {
                    let k = who(e, 2)?;
                    if event_is_str(H, e, 3, &ctx.cfg[k].low.name)? {
                        push(2, e.t, MomentKind::Desperation { who: k });
                    }
                }
                _ => {}
            }
        }
        for (k, caps) in show.caps_ability.iter().enumerate() {
            for cap in caps {
                let name = cap.lines.first().cloned().unwrap_or_default();
                push(1, cap.t0, MomentKind::Ability { who: k, name });
            }
        }

        let t_final = show.t_ko - FINAL_BARS * grid.bar;
        push(3, t_final, MomentKind::FinalExchange);
        push(4, show.t_ko, MomentKind::Ko);
        let (solo_bar, back_bar) = (bar_of(show.t_solo), bar_of(grid.t_reassemble));
        let final_bar = bar_of(t_final);
        let ko_bar = bar_of(show.t_ko);
        let mut bars = Vec::with_capacity(n_bars);
        for bar in 0..n_bars {
            let t = bar as f64 * grid.bar;
            let end = t + grid.bar;
            let phase = if t < grid.t_fight {
                Phase::Intro
            } else if bar >= back_bar {
                Phase::BuildBack
            } else if bar >= solo_bar {
                Phase::Victory
            } else if bar >= ko_bar {
                Phase::Ko
            } else if bar >= final_bar {
                Phase::FinalExchange
            } else {
                Phase::Fight
            };
            match phase {
                Phase::Victory => push(5, t, MomentKind::Victory),
                Phase::BuildBack => push(6, t, MomentKind::BuildBack),
                _ => {}
            }
            let leader = if end > show.t_ko {
                Some(show.win)
            } else {
                leads.iter().take_while(|&&(lt, _)| lt < end).last().map(|&(_, k)| k)
            };
            let probe = end.min(grid.dur) - 1e-6;
            bars.push(BarSignal {
                bar,
                t,
                leader,
                gap: ctx.hp_at(0, probe) - ctx.hp_at(1, probe),
                phase,
            });
        }
        moments.sort_by(|a, b| a.1.step.cmp(&b.1.step).then(a.0.cmp(&b.0)));
        Ok(StorySignal {
            bars,
            moments: moments.into_iter().map(|(_, m)| m).collect(),
        })
    }

    /// The moments of one kind, by a test on the kind.
    pub fn moments_where(&self, f: impl Fn(&MomentKind) -> bool) -> impl Iterator<Item = &Moment> {
        self.moments.iter().filter(move |m| f(&m.kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::music::testkit::{context, episode};

    #[test]
    fn the_flute_vs_trombone_signal_follows_the_fight() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let sig = StorySignal::from(&ctx).expect("the signal");
        let grid = &ep.show.grid;
        assert_eq!(sig.bars.len(), grid.n_bars);

        // the story's lead changes (stories.json: 22.47157 and 42.8799), to the nearest sixteenth
        let lc: Vec<f64> = sig
            .moments_where(|k| matches!(k, MomentKind::LeadChange { .. }))
            .map(|m| m.t)
            .collect();
        assert_eq!(lc.len(), 2, "{lc:?}");
        for (got, want) in lc.iter().zip([22.47157, 42.8799]) {
            assert!((got - want).abs() <= grid.s16 / 2.0 + 1e-6, "{got} against {want}");
        }

        // the K.O. (stories.json: 50.0216 s, the TROMBONE wins) and its bar
        let ko: Vec<&Moment> = sig.moments_where(|k| *k == MomentKind::Ko).collect();
        assert_eq!(ko.len(), 1);
        assert!((ko[0].t - 50.021568627450975).abs() <= grid.s16 / 2.0 + 1e-6);
        assert_eq!(ko[0].bar, (50.021568627450975 / grid.bar) as usize);
        assert_eq!(ep.show.win, 1);
        assert_eq!(sig.bars.last().expect("bars").leader, Some(1));
        assert_eq!(sig.bars.last().expect("bars").phase, Phase::BuildBack);
        assert!(sig.moments_where(|k| *k == MomentKind::Victory).count() > 0);
        assert!(sig.moments_where(|k| matches!(k, MomentKind::Ability { .. })).count() > 0);

        // every moment on a sixteenth, in time order
        for m in &sig.moments {
            assert_eq!(m.t, m.step as f64 * grid.s16);
            assert_eq!(m.bar, (m.t / grid.bar + 1e-9) as usize);
        }
        assert!(sig.moments.windows(2).all(|w| w[0].step <= w[1].step));

        // the same on a second run
        assert_eq!(StorySignal::from(&ctx).expect("again"), sig);
    }
}
