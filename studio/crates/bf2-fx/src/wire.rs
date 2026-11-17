//! Event wiring: the fight's events v2 become preset records on the timeline. Contacts by tier (a `Tag::Special`
//! move or a strength above 0.85 is a special; a multi-hit special impacts once, on its last contact, and its
//! earlier hits are heavy), blocks, clashes, launches, landings, slams, projectiles thrown and gone, the K.O., and
//! the `Flash` and `Shake` cues. Each record's seed comes from its event's step and index.

use bf2_sim::{EvKind, FighterDef, Recording2, Tag};

use crate::hash::seed_of;
use crate::presets::{self, Hit, Tier};
use crate::record::{Anchor, Env, Exit, FxError, FxKind, FxRecord};
use crate::screen::{ScreenDef, ScreenFx};
use crate::timeline::FxTimeline;

/// Fighter `k`'s ball centre at step `s` (sim space), held at the last step past the end.
fn centre(rec: &Recording2, defs: [&FighterDef; 2], k: usize, s: usize) -> [f64; 3] {
    match rec.steps.get(s).or(rec.steps.last()) {
        Some(st) => {
            let p = st.pos[k];
            [p[0], p[1], p[2] + defs[k].r]
        }
        None => [0.0; 3],
    }
}

fn floor_of(rec: &Recording2, k: usize, s: usize) -> [f64; 3] {
    match rec.steps.get(s).or(rec.steps.last()) {
        Some(st) => [st.pos[k][0], st.pos[k][1], 0.0],
        None => [0.0; 3],
    }
}

fn facing(rec: &Recording2, k: usize, s: usize) -> [f64; 3] {
    let f = rec.steps.get(s).or(rec.steps.last()).map_or(0.0, |st| st.facing[k]);
    [f.cos(), f.sin(), 0.0]
}

/// Projectile `id`'s last place at or before step `s`.
fn proj_at(rec: &Recording2, id: usize, s: usize) -> Option<[f64; 3]> {
    let s = s.min(rec.steps.len().checked_sub(1)?);
    (0..=s)
        .rev()
        .take(4)
        .find_map(|i| rec.steps[i].projs.iter().find(|p| p.0 as usize == id).map(|p| p.1))
}

/// Adds the records of every event of `rec` to `out` (video time from `step_to_t`), in the fighters' `colours`
/// (linear 0..1). `defs` tell a special move by its tag and give the ball sizes. Gives the indices (into
/// `rec.events`) of the events that made records, in order.
///
/// # Errors
/// A record a preset made that the timeline refuses (a fault in a preset).
pub fn wire(
    rec: &Recording2,
    defs: [&FighterDef; 2],
    colours: [[f32; 3]; 2],
    step_to_t: &dyn Fn(u32) -> f64,
    out: &mut FxTimeline,
) -> Result<Vec<usize>, FxError> {
    let mut made = Vec::new();
    for (i, e) in rec.events.iter().enumerate() {
        let s = e.step as usize;
        let who = e.who.min(1);
        let hit = |at: [f64; 3], normal: [f64; 3], colour: [f32; 3], strength: f32| Hit {
            t0: step_to_t(e.step),
            seed: seed_of(e.step, i, 0),
            at,
            normal,
            colour,
            strength,
        };
        let recs: Vec<FxRecord> = match &e.kind {
            EvKind::Contact {
                mv,
                point,
                normal,
                strength,
                ..
            } => {
                let special = defs[who].moves.get(*mv).is_some_and(|m| m.tags.contains(&Tag::Special));
                let tier = match Tier::of(*strength as f32, special) {
                    // a multi-hit special impacts once, on its last contact: the hits before it are heavy
                    Tier::Special if special && more_of_the_same(rec, i, *mv) => Tier::Heavy,
                    tier => tier,
                };
                presets::hit(&hit(*point, *normal, colours[who], *strength as f32), tier)
            }
            EvKind::Block { target, point } => {
                let n = centre(rec, defs, (*target).min(1), s);
                let d = [n[0] - point[0], n[1] - point[1], n[2] - point[2]];
                presets::block_flare(&hit(*point, d, colours[(*target).min(1)], 0.5))
            }
            EvKind::Clash { other, point } => {
                // Both sides report a clash: draw it once.
                let twin = rec.events.iter().any(|f| {
                    f.step == e.step
                        && f.who == *other
                        && matches!(f.kind, EvKind::Clash { other: o, .. } if o == e.who)
                });
                if twin && e.who > *other {
                    Vec::new()
                } else {
                    presets::clash_spark(&hit(*point, [0.0, 0.0, 1.0], [1.0; 3], 0.6))
                }
            }
            EvKind::Launch { target, up } => {
                let k = (*target).min(1);
                let strength = (*up as f32 / 10.0).clamp(0.3, 1.0);
                presets::launch_trail(&hit(floor_of(rec, k, s), [0.0, 0.0, 1.0], colours[who], strength), k)
            }
            EvKind::Land => presets::land_dust(&hit(floor_of(rec, who, s), [0.0, 0.0, 1.0], colours[who], 0.5)),
            EvKind::Slam { target } => {
                let k = (*target).min(1);
                presets::slam_crater(&hit(floor_of(rec, k, s), [0.0, 0.0, 1.0], colours[who], 0.8))
            }
            EvKind::Spawn { proj } => match proj_at(rec, *proj, s + 1) {
                Some(p) => presets::proj_spawn_flash(&hit(p, facing(rec, who, s), colours[who], 0.5)),
                None => Vec::new(),
            },
            EvKind::Expire { proj } => match proj_at(rec, *proj, s) {
                Some(p) => presets::proj_expire_puff(&hit(p, [0.0, 0.0, 1.0], colours[who], 0.5)),
                None => Vec::new(),
            },
            EvKind::Ko { target } => {
                let k = (*target).min(1);
                let winner = 1 - k;
                presets::ko_burst(
                    &hit(centre(rec, defs, k, s), [0.0, 0.0, 1.0], colours[winner], 1.0),
                    colours[winner],
                )
            }
            EvKind::Cue(bf2_sim::Cue::Flash) => vec![cue(
                &hit([0.0; 3], [0.0; 3], colours[who], 1.0),
                0.1,
                ScreenFx::Flash {
                    colour: [5.0, 5.0, 5.0],
                },
                0.35,
            )],
            EvKind::Cue(bf2_sim::Cue::Shake(x)) => {
                vec![cue(
                    &hit([0.0; 3], [0.0; 3], colours[who], 1.0),
                    0.45,
                    ScreenFx::Shake,
                    0.5 * (*x as f32),
                )]
            }
            _ => Vec::new(),
        };
        if !recs.is_empty() {
            made.push(i);
            out.extend(recs)?;
        }
    }
    Ok(made)
}

/// How close (in steps) two contacts of one special move by one fighter are to be one multi-hit special.
pub const SPECIAL_CHAIN_STEPS: u32 = 60;

/// Whether the same fighter lands move `mv` again within [`SPECIAL_CHAIN_STEPS`] after event `i`.
fn more_of_the_same(rec: &Recording2, i: usize, mv: usize) -> bool {
    let e = &rec.events[i];
    rec.events[i + 1..]
        .iter()
        .take_while(|f| f.step <= e.step + SPECIAL_CHAIN_STEPS)
        .any(|f| f.who == e.who && matches!(f.kind, EvKind::Contact { mv: m, .. } if m == mv))
}

fn cue(h: &Hit, life: f32, effect: ScreenFx, amount: f32) -> FxRecord {
    FxRecord {
        t0: h.t0,
        life,
        exit: Exit::Fade,
        env: Env::new(0.02, 0.0, life * 0.8),
        seed: h.seed,
        anchor: Anchor::Screen,
        kind: FxKind::Screen(ScreenDef { effect, amount }),
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
