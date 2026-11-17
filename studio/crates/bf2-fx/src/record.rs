//! Effect records: what an effect is, where it is anchored, when it starts, how long it lives and how it leaves.
//!
//! A record is everything needed to draw its effect at any video time, so frames can be drawn in any order
//! (scrubbing, sub-frames) and always the same. Every record has a life and an exit: nothing just disappears.

use std::fmt;

/// How an effect leaves. There is no cut: every exit takes the record's release time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Exit {
    /// Fades out.
    Fade,
    /// Breaks up into noise as it fades.
    Dissolve,
    /// Swells and thins out as it fades.
    Burst,
    /// Shrinks to nothing.
    Shrink,
    /// Drifts up and away as it fades.
    FlyOff,
    /// Cools to a dark red, then fades.
    Ember,
}

/// An envelope over a record's life, in seconds: rise, hold, fall. When the three add up to more than the life
/// they are scaled down to fit; when less, the hold takes up the rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env {
    pub attack: f32,
    pub hold: f32,
    pub release: f32,
}

impl Env {
    pub const fn new(attack: f32, hold: f32, release: f32) -> Env {
        Env { attack, hold, release }
    }

    /// A fast hit: up in 30 ms, then a long fall over the rest of the life.
    pub const HIT: Env = Env::new(0.03, 0.0, 10.0);
}

/// The shortest rise or fall: a smoothstep this long moves at most 1.5 / 40 of its height per 1/240 s, which
/// keeps every effect below a quarter-height jump between motion-blur sub-steps.
pub const MIN_EDGE: f32 = 1.0 / 40.0;

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The rise and fall times `e` gives over `life`.
pub fn edges(e: &Env, life: f32) -> (f32, f32) {
    let sum = e.attack.max(0.0) + e.hold.max(0.0) + e.release.max(0.0);
    let k = if sum > life && sum > 0.0 { life / sum } else { 1.0 };
    ((e.attack * k).max(MIN_EDGE), (e.release * k).max(MIN_EDGE))
}

/// The envelope's height at `age` seconds into a `life`: 0 at age 0 and at `life` (and outside), up to 1 between.
/// A life shorter than its two edges never reaches 1; it still starts and ends at 0 without a jump. Mirrored by
/// `envelope` in the shaders.
pub fn envelope(e: &Env, life: f32, age: f32) -> f32 {
    if !(age > 0.0 && age < life) {
        return 0.0;
    }
    let (a, r) = edges(e, life);
    smooth(age / a).min(smooth((life - age) / r))
}

/// Where an effect is. Positions are sim space (x, y the floor, z height); the anchor follows the recording.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    /// A fixed point.
    World([f64; 3]),
    /// Fighter `k`'s ball centre plus `offset`.
    Body { k: usize, offset: [f64; 3] },
    /// The tip of fighter `k`'s bone.
    Bone { k: usize, bone: usize },
    /// The screen (screen effects).
    Screen,
}

/// What an effect draws. The definitions live with what draws them.
#[derive(Clone, Debug, PartialEq)]
pub enum FxKind {
    Burst(crate::particles::BurstDef),
    Trail(crate::trail::TrailDef),
    Ring(crate::meshfx::RingDef),
    Dome(crate::meshfx::DomeDef),
    Beam(crate::meshfx::BeamDef),
    Decal(crate::decal::DecalDef),
    Light(crate::lights::LightDef),
    Screen(crate::screen::ScreenDef),
}

/// One effect.
#[derive(Clone, Debug, PartialEq)]
pub struct FxRecord {
    /// Video seconds.
    pub t0: f64,
    /// Seconds; more than 0.
    pub life: f32,
    pub exit: Exit,
    pub env: Env,
    /// Every random choice the effect makes comes from this.
    pub seed: u32,
    pub anchor: Anchor,
    pub kind: FxKind,
}

impl FxRecord {
    /// Whether it is live at video time `t`: `t0 <= t < t0 + life`.
    pub fn live_at(&self, t: f64) -> bool {
        self.t0 <= t && t < self.t0 + f64::from(self.life)
    }

    /// Its age at `t`, in seconds.
    pub fn age(&self, t: f64) -> f32 {
        (t - self.t0) as f32
    }

    /// The envelope at `t`.
    pub fn env_at(&self, t: f64) -> f32 {
        envelope(&self.env, self.life, self.age(t))
    }

    /// A life above 0, a finite start and a sane envelope.
    pub fn check(&self) -> Result<(), FxError> {
        if !(self.life > 0.0 && self.life.is_finite()) {
            return Err(FxError(format!(
                "an effect needs a life above 0 seconds, not {}",
                self.life
            )));
        }
        if !self.t0.is_finite() {
            return Err(FxError(format!("an effect needs a finite start time, not {}", self.t0)));
        }
        let e = self.env;
        if ![e.attack, e.hold, e.release].iter().all(|v| v.is_finite() && *v >= 0.0) {
            return Err(FxError(format!(
                "an effect's envelope needs times of 0 or more, not {e:?}"
            )));
        }
        Ok(())
    }
}

/// An effect that can't be used, as a sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FxError(pub String);

impl fmt::Display for FxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FxError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn envs() -> Vec<(Env, f32)> {
        let mut out = Vec::new();
        for &life in &[0.01f32, 0.05, 0.2, 1.0, 6.0] {
            for &(a, h, r) in &[(0.0, 0.0, 0.0), (0.03, 0.0, 10.0), (0.2, 0.5, 0.3), (1.0, 1.0, 1.0)] {
                out.push((Env::new(a, h, r), life));
            }
        }
        out
    }

    #[test]
    fn envelope_starts_and_ends_at_zero() {
        for (e, life) in envs() {
            assert_eq!(envelope(&e, life, 0.0), 0.0);
            assert_eq!(envelope(&e, life, life), 0.0);
            assert_eq!(envelope(&e, life, -1.0), 0.0);
            assert_eq!(envelope(&e, life, life + 1.0), 0.0);
            assert!(envelope(&e, life, 1e-4) < 0.01, "{e:?} {life}");
            assert!(envelope(&e, life, life - 1e-4) < 0.01, "{e:?} {life}");
        }
        // A roomy life reaches full height.
        assert!((envelope(&Env::new(0.1, 0.0, 0.1), 1.0, 0.5) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn envelope_never_jumps() {
        let dt = 1.0 / 240.0;
        for (e, life) in envs() {
            let mut age = -dt;
            let mut last = 0.0;
            while age < life + 2.0 * dt {
                age += dt / 8.0;
                let v = envelope(&e, life, age);
                // Any two times one sub-step apart.
                let w = envelope(&e, life, age + dt);
                assert!((w - v).abs() <= 0.25, "{e:?} life {life}: {v} then {w}");
                assert!((0.0..=1.0).contains(&v));
                last = v;
            }
            assert_eq!(last, 0.0);
        }
    }
}
