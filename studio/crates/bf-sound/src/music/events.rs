//! The run's events as the score reads them: Python's `e[i]`, `e[i] == k`, `int(e[i])` and a noise seed made of
//! an event's time, each an error where Python raises; and the ring's own sounds, `shared_events`.

use bf_core::event::{Event, Value};

use super::MusicCtx;
use super::error::MusicError;
use super::fx::{CLANK_DECAYS, CLANK_DUR, clank};
use super::num::{clip, py_min};
use crate::mix::{Mix, Sig};
use crate::synth;
use crate::traits::Bed;

/// Python's `e[i]` for `i >= 2`: element `i` of an event (`e[0]` is its time, `e[1]` its kind).
///
/// # Errors
/// Where Python raises `IndexError`: the event has no element `i`. `helper` is the part that reads it.
pub fn event_at<'e>(helper: &'static str, e: &'e Event, i: usize) -> Result<&'e Value, MusicError> {
    i.checked_sub(2).and_then(|at| e.fields.get(at)).ok_or_else(|| {
        MusicError::new(
            helper,
            format!(
                "the {} event at {} s has no element {i} (it has {})",
                e.kind,
                e.t,
                2 + e.fields.len()
            ),
        )
    })
}

/// Element `i` of an event as a number (Python computes with it: a position, a speed, an angle).
///
/// # Errors
/// The event has no element `i`, or it is not a number.
pub fn event_number(helper: &'static str, e: &Event, i: usize) -> Result<f64, MusicError> {
    match i.checked_sub(2).and_then(|at| e.fields.get(at)) {
        Some(Value::Float(x)) => Ok(*x),
        Some(Value::Int(n)) => Ok(*n as f64),
        other => Err(MusicError::new(
            helper,
            format!(
                "the {} event at {} s has no number as element {i} (it has {other:?})",
                e.kind, e.t
            ),
        )),
    }
}

/// Python's `e[i] == k` for a fighter index `k`: whether element `i` is that number.
///
/// # Errors
/// The event has no element `i`.
pub fn event_is(helper: &'static str, e: &Event, i: usize, k: usize) -> Result<bool, MusicError> {
    Ok(match event_at(helper, e, i)? {
        Value::Int(n) => usize::try_from(*n) == Ok(k),
        Value::Float(x) => *x == k as f64,
        Value::Bool(b) => usize::from(*b) == k,
        _ => false,
    })
}

/// Python's `e[i] == 'text'`: whether element `i` is that string (a hit's kind, an ability's name).
///
/// # Errors
/// The event has no element `i`.
pub fn event_is_str(helper: &'static str, e: &Event, i: usize, text: &str) -> Result<bool, MusicError> {
    Ok(matches!(event_at(helper, e, i)?, Value::Str(s) if s == text))
}

/// Python's `int(e[i])`: element `i` as a whole number, a float cut toward zero.
///
/// # Errors
/// The event has no element `i`, or it is not a number, or not a finite one (Python parses a string of digits
/// too; no event carries one).
pub fn event_int(helper: &'static str, e: &Event, i: usize) -> Result<i64, MusicError> {
    match event_at(helper, e, i)? {
        Value::Int(n) => Ok(*n),
        Value::Bool(b) => Ok(i64::from(*b)),
        Value::Float(x) if x.is_finite() && x.abs() < 9.0e18 => Ok(*x as i64),
        other => Err(MusicError::new(
            helper,
            format!(
                "the {} event at {} s has no whole number as element {i} (it has {other:?})",
                e.kind, e.t
            ),
        )),
    }
}

/// Python's `int(v)` as a noise seed (`seed=int(t * 10)`): numpy refuses a seed below zero.
///
/// # Errors
/// `v` is below zero or not a finite number.
pub fn noise_seed(helper: &'static str, v: f64) -> Result<u64, MusicError> {
    if v.is_finite() && v > -1.0 {
        Ok(v as u64)
    } else {
        Err(MusicError::new(helper, format!("{v} is not a noise seed")))
    }
}

/// `shared_events(mix)`: the ring's own sounds from the run, up to the K.O.: a soft thud for every rope bounce
/// (louder the faster the ball hit, panned to where), the bed's own sound for a clash of the two balls, a
/// metallic clank for a blocked hit (on the 16th) and for a parry.
///
/// # Errors
/// An event of one of those kinds lacks a number the kit reads, or the bed's `clash` fails.
pub fn shared_events(ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Result<(), MusicError> {
    const H: &str = "shared_events";
    for e in &ctx.rec.events {
        let t = e.t;
        if t > ctx.show.t_ko + 0.01 {
            break;
        }
        match e.kind.as_str() {
            "rope" => {
                let thud = synth::thud(0.22, 95.0, 55.0, noise_seed(H, t * 10.0)?);
                let gain = 0.05 * py_min(1.5, event_number(H, e, 7)? / 2.6);
                let pan = clip(event_number(H, e, 5)? / 3.5, -0.6, 0.6);
                mix.place("fx", &Sig::Mono(thud), t, gain, pan);
            }
            "clash" => bed.clash(ctx, mix, t)?,
            "block" => {
                let ring = clank(84.0, noise_seed(H, t * 5.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                mix.place("fx", &Sig::Mono(ring), ctx.q16(t), 0.06, 0.0);
            }
            "parry" => {
                let ring = clank(88.0, noise_seed(H, t * 9.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                mix.place("fx", &Sig::Mono(ring), t, 0.04, 0.0);
            }
            _ => {}
        }
    }
    Ok(())
}
