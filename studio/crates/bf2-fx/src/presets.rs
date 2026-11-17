//! The hit presets: what a moment of the fight looks like as effect records. Each scales its particle counts,
//! sizes, lights, shake, bloom push and screen effects with the hit's `strength`, and the three hit tiers read
//! very differently:
//!
//! - **light** (strength under 0.4): a quick spray of about 600 sparks, a small ring, a short light, a little shake
//!   and no screen effects;
//! - **heavy** (0.4 to 0.85): about 6k sparks and 1.5k embers, a ring that bends the screen, a hot crack in the
//!   canvas, real shake, radial blur and a bloom push;
//! - **special** (above 0.85 or a `Tag::Special` move): about 40k sparks, embers and smoke, a dome, two rings, a
//!   scorch, a flash, a shockwave, radial blur, chroma, speed lines, hard shake and a long flickering light;
//! - **K.O.**: about 120k shards and embers, a big flash, an invert for two frames, the picture greyed but for the
//!   winner's colour, letterbox bars and a shockwave.
//!
//! Every record has a life and an exit, and every envelope falls for at least [`MIN_FALL`] seconds, so nothing
//! pops out.

use crate::decal::{DECAL_LIFE, DecalDef, DecalKind, Surface};
use crate::lights::LightDef;
use crate::meshfx::{DomeDef, RingDef};
use crate::particles::{BurstDef, Shape};
use crate::record::{Anchor, Env, Exit, FxKind, FxRecord};
use crate::screen::{ScreenDef, ScreenFx};
use crate::trail::{BALL, TrailDef, TrailStyle};

/// The shortest fall of any preset's envelope: long enough that a record is under 2 % of its peak one 1/240 s
/// sub-step before its end.
pub const MIN_FALL: f32 = 0.08;

/// The presets' light colours are written as their effect's hot colour; a point light that bright over the
/// canvas washes the frame out, so lights are given this share of it.
const LIGHT_SCALE: f32 = 0.3;

/// Where and how hard: the moment a preset is made for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Video seconds.
    pub t0: f64,
    /// The first record's seed; the others count up from it.
    pub seed: u32,
    /// Sim space.
    pub at: [f64; 3],
    /// Sim space, from the contact into the target (need not be unit length).
    pub normal: [f64; 3],
    /// The attacker's colour, linear 0..1.
    pub colour: [f32; 3],
    /// 0..1.
    pub strength: f32,
}

/// The tier a hit's strength reads as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Light,
    Heavy,
    Special,
}

impl Tier {
    pub fn of(strength: f32, special: bool) -> Tier {
        if special || strength > 0.85 {
            Tier::Special
        } else if strength >= 0.4 {
            Tier::Heavy
        } else {
            Tier::Light
        }
    }
}

struct Out<'a> {
    h: &'a Hit,
    recs: Vec<FxRecord>,
}

impl Out<'_> {
    fn add(&mut self, life: f32, exit: Exit, env: Env, anchor: Anchor, kind: FxKind) {
        let release = env.release.max(MIN_FALL);
        self.recs.push(FxRecord {
            t0: self.h.t0,
            life,
            exit,
            env: Env { release, ..env },
            seed: self.h.seed.wrapping_add(self.recs.len() as u32 * 7919),
            anchor,
            kind: tamed(kind),
        });
    }

    fn here(&self) -> Anchor {
        Anchor::World(self.h.at)
    }

    fn burst(&mut self, life: f32, def: BurstDef) {
        let a = self.here();
        self.add(life, Exit::Fade, Env::new(0.0, 0.0, life * 0.7), a, FxKind::Burst(def));
    }

    fn screen(&mut self, life: f32, env: Env, effect: ScreenFx, amount: f32) {
        self.add(
            life,
            Exit::Fade,
            env,
            Anchor::Screen,
            FxKind::Screen(ScreenDef { effect, amount }),
        );
    }

    fn light(&mut self, life: f32, colour: [f32; 3], range: f32, flicker: f32) {
        let a = self.here();
        self.add(
            life,
            Exit::Fade,
            Env::new(0.02, 0.0, life * 0.8),
            a,
            FxKind::Light(LightDef {
                colour: colour.map(|c| c * LIGHT_SCALE),
                range,
                flicker,
            }),
        );
    }
}

/// `v` normalised in render space (Y up), or straight up for a zero vector.
fn dir_of(v: [f64; 3]) -> [f32; 3] {
    let r = bf2_render::to_render(v);
    let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if l < 1e-6 { [0.0, 1.0, 0.0] } else { r.map(|c| c / l) }
}

/// White-hot `k` times brighter, tinted toward `c` by `tint`.
fn hot(c: [f32; 3], tint: f32, k: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| k * (1.0 + (c[i] - 1.0) * tint))
}

/// The hot amber that sparks lean toward, so they read as sparks (not a white cloud) over the pale canvas and a
/// white fighter.
const AMBER: [f32; 3] = [1.0, 0.45, 0.12];

/// A spark's colour, `k` times brighter: a third the fighter's colour, two thirds [`AMBER`].
fn spark(c: [f32; 3], k: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| k * (0.35 * c[i] + 0.65 * AMBER[i]))
}

fn on_floor(p: [f64; 3]) -> [f64; 3] {
    [p[0], p[1], 0.0]
}

/// The presets' colours are written as hot as their effect reads; additive particles, rings and domes overlapping
/// in the first frames sum far past white, so glowing records are given this share of it (and a burst of `n`
/// particles a further `800 / n` once it is denser than 800, so a dense burst spreads the light of 800 sparks).
const GLOW_GAIN: f32 = 0.15;

/// `kind` with its glow scaled down as [`GLOW_GAIN`] says.
fn tamed(kind: FxKind) -> FxKind {
    let k = |c: [f32; 3], g: f32| c.map(|x| x * g);
    match kind {
        FxKind::Burst(mut b) if !matches!(b.shape, Shape::Smoke | Shape::Debris) => {
            let g = GLOW_GAIN * (800.0 / b.count.max(1) as f32).min(1.0);
            b.colour = k(b.colour, g);
            b.colour_end = k(b.colour_end, g);
            FxKind::Burst(b)
        }
        FxKind::Ring(mut r) => {
            r.colour = k(r.colour, GLOW_GAIN);
            FxKind::Ring(r)
        }
        FxKind::Dome(mut d) => {
            d.colour = k(d.colour, GLOW_GAIN);
            FxKind::Dome(d)
        }
        other => other,
    }
}

fn sparks(count: u32, speed: (f32, f32), dir: [f32; 3], spread: f32, colour: [f32; 3], size: f32) -> BurstDef {
    BurstDef {
        count,
        speed,
        spread,
        dir,
        gravity: 6.0,
        drag: 2.6,
        size: (size, size * 0.3),
        colour,
        colour_end: [colour[0] * 0.25, colour[1] * 0.06, colour[2] * 0.02],
        stretch: 0.022,
        bounce: 0.3,
        shape: Shape::Spark,
        emit_over: 0.0,
    }
}

fn embers(count: u32, speed: (f32, f32), colour: [f32; 3], size: f32) -> BurstDef {
    BurstDef {
        count,
        speed,
        spread: std::f32::consts::PI,
        dir: [0.0, 1.0, 0.0],
        gravity: 1.5,
        drag: 1.8,
        size: (size, size * 0.4),
        colour,
        colour_end: [1.2, 0.15, 0.02],
        stretch: 0.0,
        bounce: 0.2,
        shape: Shape::Ember,
        emit_over: 0.05,
    }
}

fn smoke(count: u32, speed: (f32, f32), size: (f32, f32), colour: [f32; 3]) -> BurstDef {
    BurstDef {
        count,
        speed,
        spread: 1.35,
        dir: [0.0, 1.0, 0.0],
        gravity: -0.6,
        drag: 3.0,
        size,
        colour,
        colour_end: [colour[0] * 0.6, colour[1] * 0.6, colour[2] * 0.6],
        stretch: 0.0,
        bounce: 0.0,
        shape: Shape::Smoke,
        emit_over: 0.08,
    }
}

/// Smoke that rolls out low along the canvas (no rise toward the camera), so it rings the fighters instead of
/// covering them.
fn dust(count: u32, speed: (f32, f32), size: (f32, f32), colour: [f32; 3]) -> BurstDef {
    BurstDef {
        spread: 1.5,
        gravity: 0.0,
        drag: 3.5,
        emit_over: 0.03,
        ..smoke(count, speed, size, colour)
    }
}

fn ring(r: (f32, f32), thickness: f32, colour: [f32; 3], tilt: f32, distort: f32) -> FxKind {
    FxKind::Ring(RingDef {
        r,
        thickness,
        colour,
        tilt,
        distort,
    })
}

/// A light hit: a quick spray, a small ring, a short light, a little shake. No screen effects.
pub fn hit_light(h: &Hit) -> Vec<FxRecord> {
    let s = h.strength.clamp(0.0, 1.0);
    let mut o = Out { h, recs: Vec::new() };
    let d = dir_of(h.normal);
    o.burst(
        0.4,
        sparks(
            450 + (450.0 * s) as u32,
            (3.0, 8.0 + 6.0 * s),
            d,
            1.0,
            spark(h.colour, 16.0),
            0.1,
        ),
    );
    o.burst(0.3, embers(120, (0.5, 2.5), hot(h.colour, 0.6, 5.0), 0.05));
    let a = o.here();
    o.add(
        0.22,
        Exit::Fade,
        Env::new(0.02, 0.0, 0.2),
        a,
        ring((0.08, 0.55 + 0.4 * s), 0.16, hot(h.colour, 0.5, 6.0), 1.25, 0.0),
    );
    o.light(0.16, hot(h.colour, 0.5, 7.0), 2.5, 0.0);
    o.screen(0.2, Env::new(0.02, 0.0, 0.18), ScreenFx::Shake, 0.15);
    o.recs
}

/// A heavy hit: a dense spray and embers, a screen-bending ring, a crack in the canvas, shake, radial blur and a
/// bloom push.
pub fn hit_heavy(h: &Hit) -> Vec<FxRecord> {
    let s = ((h.strength - 0.4) / 0.45).clamp(0.0, 1.0);
    let mut o = Out { h, recs: Vec::new() };
    let d = dir_of(h.normal);
    let k = 1.0 + 0.5 * s;
    o.burst(
        0.7,
        sparks(
            (5_000.0 + 2_000.0 * s) as u32,
            (4.0, 16.0 * k),
            d,
            1.25,
            spark(h.colour, 14.0),
            0.07,
        ),
    );
    o.burst(
        1.2,
        embers(
            (1_300.0 + 500.0 * s) as u32,
            (1.0, 6.0 * k),
            hot(h.colour, 0.5, 8.0),
            0.08,
        ),
    );
    o.burst(0.9, smoke(220, (0.5, 2.0), (0.25, 0.7), [0.16, 0.15, 0.15]));
    let a = o.here();
    o.add(
        0.38,
        Exit::Burst,
        Env::new(0.02, 0.0, 0.3),
        a,
        ring((0.2, 1.6 + 1.0 * s), 0.3, hot(h.colour, 0.4, 7.0), 1.15, 1.0 + s),
    );
    o.add(
        DECAL_LIFE,
        Exit::Ember,
        Env::new(0.03, 0.0, 2.0),
        Anchor::World(on_floor(h.at)),
        FxKind::Decal(DecalDef {
            r: 0.7 + 0.4 * s,
            kind: DecalKind::Crack,
            colour: [0.03, 0.025, 0.02],
            on: Surface::Canvas,
        }),
    );
    o.light(0.35, hot(h.colour, 0.4, 10.0), 4.5, 0.3);
    o.screen(0.4, Env::new(0.02, 0.0, 0.35), ScreenFx::Shake, 0.5 + 0.2 * s);
    o.screen(
        0.28,
        Env::new(0.02, 0.0, 0.24),
        ScreenFx::RadialBlur {
            centre: Anchor::World(h.at),
        },
        0.3 + 0.15 * s,
    );
    o.screen(0.35, Env::new(0.02, 0.0, 0.3), ScreenFx::Bloom, 0.3);
    o.recs
}

/// A special's impact: everything, at once, and long.
pub fn special_impact(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    let d = dir_of(h.normal);
    let at = Anchor::World(h.at);
    o.burst(
        1.0,
        sparks(22_000, (6.0, 26.0), d, 1.6, spark(h.colour, 12.0), 0.04),
    );
    o.burst(
        0.8,
        sparks(
            6_000,
            (8.0, 30.0),
            [0.0, 1.0, 0.0],
            std::f32::consts::PI,
            spark(h.colour, 8.0),
            0.05,
        ),
    );
    o.burst(2.2, embers(8_000, (1.0, 9.0), hot(h.colour, 0.6, 6.0), 0.06));
    o.burst(1.6, dust(400, (1.5, 5.0), (0.3, 0.9), [0.24, 0.23, 0.24]));
    o.add(
        0.9,
        Exit::Burst,
        Env::new(0.04, 0.1, 0.5),
        at,
        FxKind::Dome(DomeDef {
            r: (0.3, 2.8),
            colour: hot(h.colour, 0.7, 3.0),
            fresnel: 0.85,
        }),
    );
    o.add(
        0.8,
        Exit::Burst,
        Env::new(0.02, 0.0, 0.6),
        Anchor::World(on_floor(h.at)),
        ring((0.3, 5.0), 0.5, hot(h.colour, 0.4, 9.0), 0.0, 2.0),
    );
    o.add(
        0.6,
        Exit::Burst,
        Env::new(0.02, 0.0, 0.45),
        at,
        ring((0.2, 3.2), 0.35, [14.0, 13.0, 12.0], 1.2, 1.5),
    );
    o.add(
        DECAL_LIFE,
        Exit::Ember,
        Env::new(0.04, 0.0, 2.5),
        Anchor::World(on_floor(h.at)),
        FxKind::Decal(DecalDef {
            r: 1.6,
            kind: DecalKind::Scorch,
            colour: [0.02, 0.015, 0.012],
            on: Surface::Canvas,
        }),
    );
    o.light(1.1, hot(h.colour, 0.6, 7.0), 7.5, 0.45);
    let flash = hot(h.colour, 0.2, 6.0);
    o.screen(0.1, Env::new(0.02, 0.0, 0.08), ScreenFx::Flash { colour: flash }, 0.8);
    o.screen(
        0.7,
        Env::new(0.02, 0.0, 0.6),
        ScreenFx::Shockwave { centre: at, speed: 1.3 },
        1.2,
    );
    o.screen(
        0.55,
        Env::new(0.03, 0.05, 0.4),
        ScreenFx::RadialBlur { centre: at },
        0.45,
    );
    o.screen(0.6, Env::new(0.02, 0.1, 0.45), ScreenFx::Chroma { centre: at }, 1.0);
    o.screen(
        0.7,
        Env::new(0.03, 0.15, 0.45),
        ScreenFx::SpeedLines {
            centre: at,
            colour: spark(h.colour, 2.5),
        },
        1.0,
    );
    o.screen(0.7, Env::new(0.02, 0.0, 0.6), ScreenFx::Shake, 1.0);
    o.screen(0.8, Env::new(0.02, 0.1, 0.6), ScreenFx::Bloom, 0.2);
    o.screen(0.5, Env::new(0.02, 0.2, 0.2), ScreenFx::MotionBlur, 0.6);
    o.recs
}

/// The hit `h` at its tier.
pub fn hit(h: &Hit, tier: Tier) -> Vec<FxRecord> {
    match tier {
        Tier::Light => hit_light(h),
        Tier::Heavy => hit_heavy(h),
        Tier::Special => special_impact(h),
    }
}

/// A launched ball: a ribbon behind fighter `k`'s ball, sparks kicked up from under it and speed lines.
pub fn launch_trail(h: &Hit, k: usize) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    let s = h.strength.clamp(0.0, 1.0);
    o.add(
        0.9,
        Exit::Fade,
        Env::new(0.03, 0.3, 0.4),
        Anchor::Body { k, offset: [0.0; 3] },
        FxKind::Trail(TrailDef {
            k,
            bone: BALL,
            width: (0.9, 0.05),
            length_s: 0.3,
            colour: hot(h.colour, 0.6, 2.5),
            core: [5.0, 4.5, 4.0],
            style: TrailStyle::Ribbon,
        }),
    );
    o.burst(
        0.6,
        sparks(
            1_500 + (1_000.0 * s) as u32,
            (3.0, 12.0),
            [0.0, 1.0, 0.0],
            0.6,
            spark(h.colour, 10.0),
            0.06,
        ),
    );
    o.burst(0.9, smoke(300, (1.0, 3.0), (0.2, 0.6), [0.2, 0.19, 0.18]));
    o.screen(
        0.5,
        Env::new(0.03, 0.1, 0.3),
        ScreenFx::SpeedLines {
            centre: Anchor::Body { k, offset: [0.0; 3] },
            colour: [2.5, 2.4, 2.2],
        },
        0.35,
    );
    o.recs
}

/// A ball landing: a low ring of dust.
pub fn land_dust(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    o.burst(1.0, dust(500, (1.5, 4.0), (0.15, 0.55), [0.24, 0.23, 0.22]));
    o.screen(0.15, Env::new(0.02, 0.0, 0.12), ScreenFx::Shake, 0.08);
    o.recs
}

/// A slam into the canvas: debris, dust, a cracked and smouldering crater, a ring that bends the screen.
pub fn slam_crater(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    let floor = Anchor::World(on_floor(h.at));
    let debris = BurstDef {
        count: 1_800,
        speed: (2.0, 9.0),
        spread: 0.9,
        dir: [0.0, 1.0, 0.0],
        gravity: 14.0,
        drag: 0.8,
        size: (0.07, 0.05),
        colour: [0.22, 0.21, 0.2],
        colour_end: [0.1, 0.1, 0.1],
        stretch: 0.0,
        bounce: 0.35,
        shape: Shape::Debris,
        emit_over: 0.0,
    };
    o.burst(1.3, debris);
    o.burst(1.4, smoke(400, (1.5, 4.5), (0.3, 0.9), [0.24, 0.23, 0.22]));
    o.burst(
        0.6,
        sparks(3_000, (3.0, 12.0), [0.0, 1.0, 0.0], 1.2, spark(h.colour, 12.0), 0.06),
    );
    o.add(
        0.6,
        Exit::Burst,
        Env::new(0.02, 0.0, 0.45),
        floor,
        ring((0.3, 3.2), 0.4, hot(h.colour, 0.4, 6.0), 0.0, 1.6),
    );
    for (i, (kind, r)) in [(DecalKind::Crack, 1.4), (DecalKind::Scorch, 1.0)]
        .into_iter()
        .enumerate()
    {
        o.add(
            DECAL_LIFE - i as f32,
            Exit::Ember,
            Env::new(0.03, 0.0, 2.0),
            floor,
            FxKind::Decal(DecalDef {
                r,
                kind,
                colour: [0.03, 0.025, 0.02],
                on: Surface::Canvas,
            }),
        );
    }
    o.light(0.4, hot(h.colour, 0.4, 9.0), 4.5, 0.3);
    o.screen(0.5, Env::new(0.02, 0.0, 0.45), ScreenFx::Shake, 0.8);
    o.screen(0.35, Env::new(0.02, 0.0, 0.3), ScreenFx::Bloom, 0.3);
    o.recs
}

/// Two hits meeting: a white starburst both ways and a hard little light.
pub fn clash_spark(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    o.burst(
        0.45,
        sparks(
            2_600,
            (5.0, 18.0),
            [0.0, 1.0, 0.0],
            std::f32::consts::PI,
            [16.0, 15.0, 13.0],
            0.05,
        ),
    );
    let a = o.here();
    o.add(
        0.25,
        Exit::Burst,
        Env::new(0.02, 0.0, 0.2),
        a,
        ring((0.1, 1.2), 0.15, [10.0, 10.0, 12.0], 1.4, 0.5),
    );
    o.light(0.18, [10.0, 10.0, 12.0], 3.0, 0.0);
    o.screen(0.25, Env::new(0.02, 0.0, 0.2), ScreenFx::Shake, 0.25);
    o.recs
}

/// A blocked hit: a shield flare and sparks glancing back.
pub fn block_flare(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    let a = o.here();
    o.add(
        0.32,
        Exit::Shrink,
        Env::new(0.02, 0.05, 0.22),
        a,
        FxKind::Dome(DomeDef {
            r: (0.35, 0.8),
            colour: [2.0, 3.5, 7.0],
            fresnel: 0.9,
        }),
    );
    let back = dir_of(h.normal).map(|c| -c);
    o.burst(0.35, sparks(800, (3.0, 9.0), back, 1.1, [4.0, 7.0, 14.0], 0.045));
    o.light(0.2, [2.5, 4.0, 9.0], 2.5, 0.0);
    o.recs
}

/// A K.O.: the biggest of everything.
pub fn ko_burst(h: &Hit, winner_colour: [f32; 3]) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    let at = Anchor::World(h.at);
    let shards = BurstDef {
        count: 4_000,
        speed: (2.0, 10.0),
        spread: std::f32::consts::PI,
        dir: [0.0, 1.0, 0.0],
        gravity: 14.0,
        drag: 1.4,
        size: (0.05, 0.03),
        colour: [0.3, 0.3, 0.33],
        colour_end: [0.18, 0.18, 0.2],
        stretch: 0.0,
        bounce: 0.4,
        shape: Shape::Debris,
        emit_over: 0.0,
    };
    o.burst(2.5, shards);
    o.burst(3.0, embers(66_000, (1.5, 10.0), hot(h.colour, 0.6, 14.0), 0.05));
    o.burst(
        1.2,
        sparks(
            30_000,
            (6.0, 24.0),
            [0.0, 1.0, 0.0],
            std::f32::consts::PI,
            spark(h.colour, 18.0),
            0.06,
        ),
    );
    o.burst(1.6, dust(300, (2.0, 6.0), (0.3, 0.9), [0.28, 0.27, 0.27]));
    o.add(
        1.2,
        Exit::Burst,
        Env::new(0.03, 0.2, 0.7),
        at,
        FxKind::Dome(DomeDef {
            r: (0.4, 4.0),
            colour: hot(winner_colour, 0.6, 3.0),
            fresnel: 0.8,
        }),
    );
    o.add(
        1.0,
        Exit::Burst,
        Env::new(0.02, 0.0, 0.8),
        Anchor::World(on_floor(h.at)),
        ring((0.4, 7.0), 0.6, [12.0, 11.0, 10.0], 0.0, 2.5),
    );
    o.add(
        DECAL_LIFE,
        Exit::Ember,
        Env::new(0.04, 0.0, 2.5),
        Anchor::World(on_floor(h.at)),
        FxKind::Decal(DecalDef {
            r: 2.0,
            kind: DecalKind::Scorch,
            colour: [0.02, 0.015, 0.012],
            on: Surface::Canvas,
        }),
    );
    o.light(2.0, hot(winner_colour, 0.6, 8.0), 9.0, 0.5);
    o.screen(
        0.12,
        Env::new(0.02, 0.02, 0.08),
        ScreenFx::Flash {
            colour: [6.0, 6.0, 6.0],
        },
        1.0,
    );
    o.screen(2.0 / 30.0, Env::new(0.0, 0.0, 0.03), ScreenFx::Invert, 1.0);
    o.screen(
        2.5,
        Env::new(0.15, 1.6, 0.6),
        ScreenFx::Desaturate { keep: winner_colour },
        1.0,
    );
    o.screen(1.5, Env::new(0.25, 0.8, 0.4), ScreenFx::Letterbox, 1.0);
    o.screen(
        1.0,
        Env::new(0.02, 0.0, 0.8),
        ScreenFx::Shockwave { centre: at, speed: 1.6 },
        1.6,
    );
    o.screen(0.9, Env::new(0.02, 0.0, 0.8), ScreenFx::Shake, 1.3);
    o.screen(1.2, Env::new(0.02, 0.2, 0.8), ScreenFx::Bloom, 0.5);
    o.screen(0.5, Env::new(0.02, 0.2, 0.2), ScreenFx::MotionBlur, 0.45);
    o.recs
}

/// A projectile thrown: a muzzle flash.
pub fn proj_spawn_flash(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    let d = dir_of(h.normal);
    o.burst(0.3, sparks(500, (2.0, 7.0), d, 0.7, spark(h.colour, 8.0), 0.04));
    o.light(0.15, hot(h.colour, 0.4, 6.0), 2.5, 0.0);
    o.recs
}

/// A projectile gone: a puff and some embers.
pub fn proj_expire_puff(h: &Hit) -> Vec<FxRecord> {
    let mut o = Out { h, recs: Vec::new() };
    o.burst(0.8, smoke(220, (0.5, 2.0), (0.15, 0.5), [0.2, 0.19, 0.18]));
    o.burst(0.6, embers(300, (0.5, 3.0), hot(h.colour, 0.5, 6.0), 0.05));
    o.recs
}

/// Every particle the records ask for.
pub fn particle_count(recs: &[FxRecord]) -> u32 {
    recs.iter()
        .map(|r| match &r.kind {
            FxKind::Burst(b) => b.count,
            _ => 0,
        })
        .sum()
}

/// The records that are screen effects (everything on the screen chain but camera shake).
pub fn screen_effects(recs: &[FxRecord]) -> usize {
    recs.iter()
        .filter(|r| matches!(&r.kind, FxKind::Screen(d) if d.effect != ScreenFx::Shake))
        .count()
}

/// A CPU estimate of how bright record `r` is at `t`, as a share of its peak (1 at full envelope): the envelope,
/// squared for flash and invert as the screen chain mixes them.
pub fn brightness(r: &FxRecord, t: f64) -> f32 {
    let e = r.env_at(t);
    match &r.kind {
        FxKind::Screen(ScreenDef {
            effect: ScreenFx::Flash { .. } | ScreenFx::Invert,
            ..
        }) => e * e,
        _ => e,
    }
}

#[cfg(test)]
#[path = "presets_tests.rs"]
mod tests;
