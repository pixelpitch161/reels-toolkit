//! Engine 2's shared sound effects (milestone E5): the impacts, launches, landings, slams, the hitstop "freeze"
//! swell and the special wind-up, read from the v2 detail a shadow recording carries (`bf2_direct::shadow`).
//!
//! A shadow recording's `hit` carries three trailing fields past the current engine's (strength, the move's weight
//! and its hitstop in seconds), and its v2-only events are named `v2.*`. No current-engine event has either, so on
//! a current-engine fight [`shared_events2`] places nothing: it asks for no note and draws no noise from the build's
//! stream, and the soundtrack is the same to the sample.

use bf_core::dmath;

use super::{MusicCtx, MusicError, event_at, event_number, noise_seed};
use crate::SRF;
use crate::mix::{Mix, Sig};
use crate::synth::{self, FILTER_O, WHOOSH_RISE, bp, clip};
use crate::traits::Bed;

/// The freeze swell's release after the hitstop, in seconds.
pub const FREEZE_RELEASE: f64 = 0.15;

/// Below this strength a hit is light; above [`SPECIAL_FROM`] it is a special.
pub const HEAVY_FROM: f64 = 0.4;

/// Above this strength a hit sounds as a special, whatever its move.
pub const SPECIAL_FROM: f64 = 0.85;

/// `n` samples of band-passed noise under an exponential decay of `tau` seconds, at peak 1.
fn burst(seed: u64, dur: f64, lo: f64, hi: f64, tau: f64) -> Vec<f64> {
    let n = (dur * SRF) as usize;
    let x = bp(&synth::noise(seed, n), lo, hi, FILTER_O);
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, v)| v * dmath::exp(-(i as f64 / SRF) / tau) * clip(i as f64 / (0.001 * SRF), 0.0, 1.0))
        .collect();
    unit(y)
}

/// `x` scaled to peak 1 (silence stays silence).
fn unit(mut x: Vec<f64>) -> Vec<f64> {
    let p = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    if p > 0.0 {
        for v in &mut x {
            *v /= p;
        }
    }
    x
}

/// `b * gain` added onto `a` from its start, `a` growing to fit.
fn add(a: &mut Vec<f64>, b: &[f64], gain: f64) {
    if a.len() < b.len() {
        a.resize(b.len(), 0.0);
    }
    for (x, y) in a.iter_mut().zip(b) {
        *x += gain * y;
    }
}

/// One impact, mono, peak at most 1, louder the stronger it is (strength is capped at 1):
///
/// - light (strength under 0.4): a short high thud and a 20 ms band-passed click at 2 to 6 kHz;
/// - heavy (0.4 to 0.85): a deeper thud and a 60 ms crack at 0.8 to 4 kHz;
/// - special (above 0.85, or `tag` `"special"`): the heavy one over a sub thud and a 0.4 s noise tail at 300 to
///   2,000 Hz.
pub fn impact(strength: f64, tag: &str, seed: u64) -> Vec<f64> {
    let st = clip(strength, 0.0, 1.0);
    let special = tag == "special" || st > SPECIAL_FROM;
    let mut x = if st < HEAVY_FROM && !special {
        let mut x = unit(synth::thud(0.12, 180.0, 90.0, seed));
        add(&mut x, &burst(seed.wrapping_add(1), 0.02, 2000.0, 6000.0, 0.004), 0.35);
        x
    } else {
        let mut x = unit(synth::thud(0.30, 120.0, 45.0, seed));
        add(&mut x, &burst(seed.wrapping_add(2), 0.06, 800.0, 4000.0, 0.012), 0.45);
        if special {
            add(&mut x, &unit(synth::thud(0.9, 70.0, 30.0, seed.wrapping_add(3))), 0.8);
            add(&mut x, &burst(seed.wrapping_add(4), 0.4, 300.0, 2000.0, 0.12), 0.3);
        }
        x
    };
    x = unit(x);
    let gain = 0.2 + 0.8 * st;
    for v in &mut x {
        *v *= gain;
    }
    x
}

/// The hitstop freeze: band-passed noise swelling over `dur` seconds and a held sine at `90 + 140·strength` Hz,
/// then a [`FREEZE_RELEASE`] fall to silence. Mono, peak at most 1, louder the stronger the hit.
pub fn freeze_swell(strength: f64, dur: f64, seed: u64) -> Vec<f64> {
    let st = clip(strength, 0.0, 1.0);
    let dur = dur.max(1.0 / SRF);
    let n = ((dur + FREEZE_RELEASE) * SRF) as usize;
    let hiss = unit(bp(&synth::noise(seed, n), 500.0, 2500.0, FILTER_O));
    let f = 90.0 + 140.0 * st;
    let x: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / SRF;
            let env = if t < dur {
                let u = t / dur;
                u * u
            } else {
                let r = 1.0 - (t - dur) / FREEZE_RELEASE;
                r.max(0.0) * r.max(0.0)
            };
            env * (0.6 * hiss[i] + 0.8 * dmath::sin(dmath::TAU * f * t))
        })
        .collect();
    let gain = 0.4 + 0.6 * st;
    unit(x).into_iter().map(|v| v * gain).collect()
}

/// The pan of a sound at ring position `x`.
fn pan(x: f64) -> f64 {
    clip(x / 3.5, -0.6, 0.6)
}

/// The Engine 2 sound effects of the run's v2 detail, on bus `"fx"`, up to the K.O. (`show.t_ko + 0.01`):
///
/// - a `hit` with the v2 trailing fields (at least 10): [`impact`] of its strength (`e[9]`) and weight (`e[10]`),
///   gain 0.10, panned to `e[5]`;
/// - `v2.freeze`: [`freeze_swell`] of its strength and hitstop, gain 0.05;
/// - `v2.launch`: a rising whoosh, gain 0.06;
/// - `v2.land`: a small thud, gain 0.05;
/// - `v2.slam`: a special [`impact`] at the bed's first caption-landing gain;
/// - `v2.windup` of a special: a whoosh that swells up to the move's Active step, gain 0.05.
///
/// Every noise seed is `noise_seed(t·1000)`. It asks for no note and reads none of the build's shared state, and it
/// never matches a current-engine event, so on a current-engine fight it places nothing.
///
/// # Errors
/// A v2 event lacks a number or a text it reads.
pub fn shared_events2(ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Result<(), MusicError> {
    const H: &str = "shared_events2";
    let text = |e: &bf_core::event::Event, i: usize| -> Result<String, MusicError> {
        match event_at(H, e, i)? {
            bf_core::event::Value::Str(s) => Ok(s.clone()),
            other => Err(MusicError::new(
                H,
                format!(
                    "the {} event at {} s has no text as element {i} (it has {other:?})",
                    e.kind, e.t
                ),
            )),
        }
    };
    for e in &ctx.rec.events {
        let t = e.t;
        if t > ctx.show.t_ko + 0.01 {
            break;
        }
        let mut put = |x: Vec<f64>, gain: f64, p: f64| mix.place("fx", &Sig::Mono(x), t, gain, p);
        match e.kind.as_str() {
            "hit" if e.fields.len() >= 10 => {
                let x = impact(event_number(H, e, 9)?, &text(e, 10)?, noise_seed(H, t * 1000.0)?);
                put(x, 0.10, pan(event_number(H, e, 5)?));
            }
            "v2.freeze" => {
                let x = freeze_swell(
                    event_number(H, e, 4)?,
                    event_number(H, e, 5)?,
                    noise_seed(H, t * 1000.0)?,
                );
                put(x, 0.05, pan(event_number(H, e, 6)?));
            }
            "v2.launch" => {
                let x = synth::whoosh(0.35, 300.0, 3000.0, WHOOSH_RISE, noise_seed(H, t * 1000.0)?);
                put(x, 0.06, 0.0);
            }
            "v2.land" => {
                let x = synth::thud(0.18, 110.0, 60.0, noise_seed(H, t * 1000.0)?);
                put(x, 0.05, pan(event_number(H, e, 3)?));
            }
            "v2.slam" => {
                let x = impact(1.0, "special", noise_seed(H, t * 1000.0)?);
                put(x, bed.landing().0, pan(event_number(H, e, 4)?));
            }
            "v2.windup" => {
                let dur = event_number(H, e, 5)?;
                // a whoosh needs 24 samples a band; a wind-up too short for one has none
                if text(e, 4)? == "special" && dur * SRF >= 48.0 {
                    // the rising half of a whoosh twice as long: it swells up to the Active step, loudest there
                    let mut x = synth::whoosh(2.0 * dur, 200.0, 6000.0, WHOOSH_RISE, noise_seed(H, t * 1000.0)?);
                    x.truncate((dur * SRF) as usize);
                    put(x, 0.05, 0.0);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mix::Mix;
    use crate::music::shared_events;
    use crate::music::testkit::{Episode, TestBed, TestVoice, context, episode};
    use bf_core::event::{Event, Value};
    use bf_core::recording::Recording;
    use std::sync::Arc;

    fn rms(x: &[f64], n: usize) -> f64 {
        let s: f64 = (0..n).map(|i| x.get(i).map_or(0.0, |v| v * v)).sum();
        (s / n as f64).sqrt()
    }

    fn peak(x: &[f64]) -> f64 {
        x.iter().fold(0.0, |m, v| m.max(v.abs()))
    }

    fn bus_bits(m: &Mix) -> Vec<(String, Vec<u64>)> {
        m.bus_names()
            .into_iter()
            .map(|b| {
                (
                    b.to_string(),
                    m.bus(b).unwrap().iter().flatten().map(|v| v.to_bits()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn v2_hook_is_silent_on_v1() {
        let ep = episode();
        let (ctx, _) = context(&ep);
        let bed = TestBed::new((0.3, 0.4, 0.2));
        let mut mix = Mix::new(ep.show.grid.dur + 4.0);
        shared_events(&ctx, &bed, &mut mix).unwrap();
        let (before, notes) = (bus_bits(&mix), ctx.first_notes().unwrap());
        let noise = ctx.noise().unwrap();
        shared_events2(&ctx, &bed, &mut mix).unwrap();
        assert_eq!(bus_bits(&mix), before);
        assert_eq!(ctx.first_notes().unwrap(), notes);
        assert_eq!(format!("{:?}", ctx.noise().unwrap()), format!("{noise:?}"));
    }

    #[test]
    fn v2_events_sound_on_fx() {
        let mut ep = episode();
        let t = 10.0;
        let f = Value::Float;
        let s = |x: &str| Value::Str(x.to_string());
        let i = Value::Int;
        let evs = [
            (
                "hit",
                vec![
                    i(0),
                    i(1),
                    f(4.0),
                    f(1.0),
                    f(0.0),
                    s("JAB"),
                    i(0),
                    f(0.7),
                    s("heavy"),
                    f(0.05),
                ],
            ),
            ("v2.freeze", vec![i(0), i(1), f(0.7), f(0.05), f(1.0)]),
            ("v2.launch", vec![i(0), i(1), f(5.0)]),
            ("v2.land", vec![i(1), f(-1.0)]),
            ("v2.slam", vec![i(0), i(1), f(0.5)]),
            ("v2.windup", vec![i(0), s("BIG ONE"), s("special"), f(0.2)]),
        ];
        let at = ep.rec.events.partition_point(|e| e.t <= t);
        for (k, (kind, fields)) in evs.into_iter().enumerate() {
            ep.rec.events.insert(
                at + k,
                Event {
                    t: t + k as f64,
                    kind: kind.to_string(),
                    fields,
                },
            );
        }
        let (ctx, _) = context(&ep);
        let bed = TestBed::new((0.3, 0.4, 0.2));
        let mut mix = Mix::new(ep.show.grid.dur + 4.0);
        shared_events2(&ctx, &bed, &mut mix).unwrap();
        assert_eq!(mix.bus_names(), vec!["fx"]);
        let fx = mix.bus("fx").unwrap();
        for k in 0..6 {
            let a = ((t + k as f64) * crate::SRF) as usize;
            let loud = fx[a..a + 4_800].iter().any(|s| s[0].abs() + s[1].abs() > 1e-4);
            assert!(loud, "event {k} sounds");
        }
        // nothing before the first of them
        let a = (t * crate::SRF) as usize;
        assert!(fx[..a].iter().all(|s| *s == [0.0, 0.0]));
        // a v2 hit with a weight that is not a text is a sentence, not a panic
        let mut bad = ep.rec.clone();
        bad.events[at].fields[8] = f(1.0);
        let ctx = ctx_on(&ep, &bad);
        let e = shared_events2(&ctx, &bed, &mut Mix::new(1.0)).unwrap_err().to_string();
        assert!(e.starts_with("shared_events2: "), "{e}");
    }

    /// The episode's context on another recording.
    fn ctx_on<'a>(ep: &'a Episode, rec: &'a Recording) -> MusicCtx<'a> {
        let voices: [Arc<dyn crate::voice::Voice>; 2] = [TestVoice::new("FLUTE", 0), TestVoice::new("TROMBONE", 0)];
        MusicCtx::new(&ep.show, rec, &ep.ff, [&ep.cfg[0], &ep.cfg[1]], voices, Vec::new()).expect("the context builds")
    }

    #[test]
    fn heavy_is_louder_than_light() {
        let n = (0.3 * crate::SRF) as usize;
        for seed in [1, 2, 3] {
            let light = impact(0.2, "light", seed);
            let heavy = impact(0.6, "heavy", seed);
            assert!(
                rms(&heavy, n) >= 2.0 * rms(&light, n),
                "seed {seed}: {} vs {}",
                rms(&heavy, n),
                rms(&light, n)
            );
        }
        // and the gain grows with strength
        assert!(peak(&impact(0.39, "light", 4)) > peak(&impact(0.1, "light", 4)));
    }

    #[test]
    fn special_is_longest() {
        let (l, h, s) = (
            impact(0.2, "light", 5),
            impact(0.6, "heavy", 5),
            impact(0.6, "special", 5),
        );
        assert!(l.len() < h.len() && h.len() < s.len());
        assert_eq!(s.len(), impact(0.9, "heavy", 5).len(), "above 0.85 is a special too");
        assert!(s.len() >= (0.9 * crate::SRF) as usize);
    }

    #[test]
    fn impacts_peak_at_most_one() {
        for seed in 0..6 {
            for (st, tag) in [
                (0.0, "light"),
                (0.39, "light"),
                (0.6, "heavy"),
                (0.85, "heavy"),
                (1.0, "special"),
            ] {
                let x = impact(st, tag, seed);
                assert!(peak(&x) <= 1.0, "{st} {tag} {seed}");
                assert!(peak(&x) > 0.05, "{st} {tag} {seed} sounds");
            }
            // strength past 1 is capped, not louder than 1
            assert!(peak(&impact(3.0, "special", seed)) <= 1.0);
        }
    }

    #[test]
    fn freeze_lasts_the_hitstop() {
        for dur in [4.0 / 120.0, 0.1, 0.3] {
            let x = freeze_swell(0.8, dur, 9);
            assert_eq!(x.len(), ((dur + FREEZE_RELEASE) * crate::SRF) as usize);
            // it rises through the hitstop: loudest at its end, then lets go
            let at = |t: f64| (t * crate::SRF) as usize;
            let w = at(0.004).max(1);
            let early = rms(&x[..w], w);
            let top = rms(&x[at(dur) - w..at(dur)], w);
            assert!(top > 2.0 * early, "{dur}: {top} vs {early}");
            assert!(x.last().unwrap().abs() < 1e-3);
            assert!(peak(&x) <= 1.0);
        }
        // a stronger hit holds a higher note: louder
        assert!(peak(&freeze_swell(1.0, 0.1, 2)) > peak(&freeze_swell(0.2, 0.1, 2)));
    }
}
