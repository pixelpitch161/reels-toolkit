//! The ring's own sounds from the run: `shared_events`.

use bf_core::event::{Event, Value};

use super::MusicCtx;
use super::error::MusicError;
use super::fx::{CLANK_DECAYS, CLANK_DUR, clank};
use super::num::{clip, py_min};
use crate::mix::{Mix, Sig};
use crate::synth;
use crate::traits::Bed;

/// Element `i` of an event (Python's `e[i]`, `i >= 2`) as a number.
fn number(e: &Event, i: usize) -> Result<f64, MusicError> {
    match e.fields.get(i - 2) {
        Some(Value::Float(x)) => Ok(*x),
        Some(Value::Int(n)) => Ok(*n as f64),
        other => Err(MusicError::new(
            "shared_events",
            format!(
                "the {} event at {} s has no number as element {i} (it has {other:?})",
                e.kind, e.t
            ),
        )),
    }
}

/// Python's `int(v)` as a noise seed: numpy refuses a seed below zero.
fn seed(v: f64) -> Result<u64, MusicError> {
    if v.is_finite() && v > -1.0 {
        Ok(v as u64)
    } else {
        Err(MusicError::new("shared_events", format!("{v} is not a noise seed")))
    }
}

/// `shared_events(mix)`: the ring's own sounds from the run, up to the K.O.: a soft thud for every rope bounce
/// (louder the faster the ball hit, panned to where), the bed's own sound for a clash of the two balls, a
/// metallic clank for a blocked hit (on the 16th) and for a parry.
///
/// # Errors
/// An event of one of those kinds lacks a number the kit reads, or the bed's `clash` fails.
pub fn shared_events(ctx: &MusicCtx, bed: &dyn Bed, mix: &mut Mix) -> Result<(), MusicError> {
    for e in &ctx.rec.events {
        let t = e.t;
        if t > ctx.show.t_ko + 0.01 {
            break;
        }
        match e.kind.as_str() {
            "rope" => {
                let thud = synth::thud(0.22, 95.0, 55.0, seed(t * 10.0)?);
                let gain = 0.05 * py_min(1.5, number(e, 7)? / 2.6);
                let pan = clip(number(e, 5)? / 3.5, -0.6, 0.6);
                mix.place("fx", &Sig::Mono(thud), t, gain, pan);
            }
            "clash" => bed.clash(ctx, mix, t)?,
            "block" => {
                let ring = clank(84.0, seed(t * 5.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                mix.place("fx", &Sig::Mono(ring), ctx.q16(t), 0.06, 0.0);
            }
            "parry" => {
                let ring = clank(88.0, seed(t * 9.0)?, CLANK_DUR, &CLANK_DECAYS)?;
                mix.place("fx", &Sig::Mono(ring), t, 0.04, 0.0);
            }
            _ => {}
        }
    }
    Ok(())
}
