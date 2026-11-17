//! Restraint: a fighter's way of turning the shared presets down for its own events (E7: Aedan asked for the TEST
//! DUMMY's attack effects "toned down by a lot"). The mechanism lives here; the numbers are each fighter's own
//! ([`crate::frame::LookFx::tone`]).

use crate::record::{FxKind, FxRecord};
use crate::screen::ScreenFx;

/// How far a fighter turns its events' preset records down. 1 everywhere (and `lens: true`) leaves them as they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Restraint {
    /// Particle counts (at least one particle is kept).
    pub particles: f32,
    /// The glow of sparks, rings, domes, trails, beams and lights.
    pub glow: f32,
    /// How long rings, domes and lights last.
    pub life: f32,
    /// How big rings and domes grow and how far lights reach.
    pub size: f32,
    /// The size of the marks left on the canvas (scorches, cracks, glows).
    pub marks: f32,
    /// Flashes, inverts, extra bloom, letterbox bars and shake.
    pub screen: f32,
    /// Whether the lens effects (speed lines, radial blur, chroma, shockwaves, desaturate, motion blur, and the
    /// rings' push on the screen) stay (scaled by `screen`); false drops them.
    pub lens: bool,
}

impl Restraint {
    /// No change.
    pub const NONE: Restraint = Restraint {
        particles: 1.0,
        glow: 1.0,
        life: 1.0,
        size: 1.0,
        marks: 1.0,
        screen: 1.0,
        lens: true,
    };

    /// `r` toned down, or `None` when it is dropped.
    pub fn apply(&self, mut r: FxRecord) -> Option<FxRecord> {
        let g = |c: [f32; 3]| c.map(|x| x * self.glow);
        match &mut r.kind {
            FxKind::Burst(b) => {
                b.count = ((b.count as f32 * self.particles).round() as u32).max(1);
                b.colour = g(b.colour);
                b.colour_end = g(b.colour_end);
            }
            FxKind::Ring(x) => {
                x.colour = g(x.colour);
                x.r = (x.r.0 * self.size, x.r.1 * self.size);
                x.distort *= if self.lens { self.screen } else { 0.0 };
                r.life = shorter(r.life, self.life);
            }
            FxKind::Dome(x) => {
                x.colour = g(x.colour);
                x.r = (x.r.0 * self.size, x.r.1 * self.size);
                r.life = shorter(r.life, self.life);
            }
            FxKind::Beam(x) => {
                x.colour = g(x.colour);
                x.core = g(x.core);
            }
            FxKind::Trail(x) => {
                x.colour = g(x.colour);
                x.core = g(x.core);
            }
            FxKind::Light(x) => {
                x.colour = g(x.colour);
                x.range *= self.size;
                r.life = shorter(r.life, self.life);
            }
            FxKind::Decal(d) => d.r *= self.marks,
            FxKind::Screen(s) => {
                let lens = matches!(
                    s.effect,
                    ScreenFx::SpeedLines { .. }
                        | ScreenFx::RadialBlur { .. }
                        | ScreenFx::Chroma { .. }
                        | ScreenFx::Shockwave { .. }
                        | ScreenFx::Desaturate { .. }
                        | ScreenFx::MotionBlur
                );
                if lens && !self.lens {
                    return None;
                }
                if !matches!(s.effect, ScreenFx::Grade { .. }) {
                    s.amount *= self.screen;
                }
            }
        }
        Some(r)
    }
}

/// `life` times `k`, but never under 0.15 s (or the life it had, if shorter), so a record still has time to fade.
fn shorter(life: f32, k: f32) -> f32 {
    (life * k).max(life.min(0.15))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::{Hit, Tier, hit, ko_burst, particle_count, screen_effects};

    fn h() -> Hit {
        Hit {
            t0: 1.0,
            seed: 7,
            at: [0.5, 0.0, 0.6],
            normal: [1.0, 0.0, 0.0],
            colour: [0.9, 0.9, 0.9],
            strength: 0.7,
        }
    }

    #[test]
    fn none_changes_nothing() {
        for r in hit(&h(), Tier::Special) {
            assert_eq!(Restraint::NONE.apply(r.clone()), Some(r));
        }
    }

    #[test]
    fn restraint_cuts_particles_glow_and_lens_effects() {
        let k = Restraint {
            particles: 0.25,
            glow: 0.5,
            life: 0.6,
            size: 0.5,
            marks: 0.6,
            screen: 0.3,
            lens: false,
        };
        let full = hit(&h(), Tier::Special);
        let toned: Vec<FxRecord> = full.iter().cloned().filter_map(|r| k.apply(r)).collect();
        let (a, b) = (particle_count(&full), particle_count(&toned));
        assert!(b <= a / 3 && b > 0, "{a} -> {b}");
        assert_eq!(screen_effects(&toned.iter().filter(|r| !matches!(&r.kind, FxKind::Screen(s) if matches!(s.effect, ScreenFx::Flash { .. } | ScreenFx::Bloom | ScreenFx::Invert))).cloned().collect::<Vec<_>>()), 0);
        let pairs = full.iter().filter_map(|f| k.apply(f.clone()).map(|t| (f, t)));
        for (f, t) in pairs {
            let t = &t;
            if let (FxKind::Screen(x), FxKind::Screen(y)) = (&f.kind, &t.kind)
                && x.effect == y.effect
                && !matches!(x.effect, ScreenFx::Grade { .. })
            {
                assert!((y.amount - 0.3 * x.amount).abs() < 1e-6);
            }
            assert!(t.life >= f.life.min(0.15));
            // rings and domes grow half as big, rings stop pushing the screen, lights reach half as far
            match (&f.kind, &t.kind) {
                (FxKind::Ring(x), FxKind::Ring(y)) => {
                    assert_eq!(y.r, (x.r.0 * 0.5, x.r.1 * 0.5));
                    assert_eq!(y.distort, 0.0);
                }
                (FxKind::Dome(x), FxKind::Dome(y)) => assert_eq!(y.r, (x.r.0 * 0.5, x.r.1 * 0.5)),
                (FxKind::Light(x), FxKind::Light(y)) => assert_eq!(y.range, x.range * 0.5),
                _ => {}
            }
        }
        // lens effects stay (scaled) when the restraint keeps them, as for the K.O.
        let ko = ko_burst(&h(), [1.0, 0.5, 0.1]);
        let keep = Restraint { lens: true, ..k };
        assert_eq!(ko.iter().cloned().filter_map(|r| keep.apply(r)).count(), ko.len());
    }
}
