//! Effect lights: point lights that live and fade with their records, so effects light the canvas, the ropes and
//! the balls. They outrank the stage's lights when a frame has more than the GPU's 64.

use bf2_render::{Light, to_render};

use crate::hash::h3;
use crate::record::{FxKind, FxRecord};

/// A light an effect gives off.
#[derive(Clone, Debug, PartialEq)]
pub struct LightDef {
    /// HDR, at full envelope.
    pub colour: [f32; 3],
    /// Where its light ends.
    pub range: f32,
    /// 0 steady, 1 flickering hard.
    pub flicker: f32,
}

impl Default for LightDef {
    fn default() -> Self {
        LightDef {
            colour: [6.0, 3.0, 1.0],
            range: 4.0,
            flicker: 0.0,
        }
    }
}

/// The effect light's priority in `bf2_render::Light`.
pub const EFFECT_PRIORITY: u8 = 1;

/// The light of `def` at video time `t` for a record of `seed`, as a share 0..1 of full (before the envelope).
pub fn flicker_at(def: &LightDef, seed: u32, t: f64) -> f32 {
    if def.flicker <= 0.0 {
        return 1.0;
    }
    // Smooth noise at 24 Hz: two hashed values blended.
    let x = t * 24.0;
    let i = x.floor();
    let f = (x - i) as f32;
    let f = f * f * (3.0 - 2.0 * f);
    let (a, b) = (h3(seed, i as u32, 3), h3(seed, i as u32 + 1, 3));
    1.0 - def.flicker.clamp(0.0, 1.0) * (a + (b - a) * f)
}

/// Adds the light of every live Light record (with its anchor resolved, sim space) at video time `t` to `out`.
pub fn lights_for(t: f64, recs: &[(&FxRecord, [f64; 3])], out: &mut Vec<Light>) {
    for (r, at) in recs {
        let FxKind::Light(def) = &r.kind else { continue };
        let k = r.env_at(t) * flicker_at(def, r.seed, t);
        if k <= 0.0 {
            continue;
        }
        out.push(Light {
            pos: to_render(*at),
            colour: def.colour.map(|c| c * k),
            range: def.range,
            priority: EFFECT_PRIORITY,
        });
    }
}
