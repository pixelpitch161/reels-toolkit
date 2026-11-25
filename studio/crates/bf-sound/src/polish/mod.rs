//! The mix and the polish of the Composed arrangement (sound upgrade, 2026-11-25; design
//! `docs/studio/2026-11-25-sound-upgrade-design.md`, stage 1). The Classic arrangement keeps [`crate::master`],
//! sample for sample; a fight under Composed is mastered here instead, in this order:
//!
//! 1. every bus times its gain: a fighter's is its saved level where the level table has one (M1, first layer);
//! 2. a low fighter's bus gets overtones a phone can play ([`lows`], M2), a piercing fighter's loses its sting
//!    ([`highs`], M3); the winner's solo bus is treated as the winner's;
//! 3. each fighter is trimmed to stand [`OVER_BAND`] dB over this fight's band to the ear, by at most [`TRIM`] dB
//!    either way (M1, second layer); the solo bus takes the winner's trim;
//! 4. the room ([`room`], F3): each bus sends by the kind of instrument, and the room lies [`room::UNDER`] LU
//!    under the dry mix;
//! 5. the fold onto the loop;
//! 6. each of three ranges evened out by itself ([`bands`], F2), and each fighter brought back to where step 3
//!    left it against the band, by at most [`HOLD`] dB ([`hold_balance`]);
//! 7. one loudness and a clean limiter ([`loud`], [`limit`], F1).
//!
//! The stems are the folded buses under the same gains, without the room, as the Classic master gives them.

use std::collections::BTreeMap;

use crate::master::{Levels, Soundtrack};
use crate::mix::Mix;
use crate::{NA, heard};

pub mod bands;
pub mod highs;
pub mod limit;
pub mod loud;
pub mod lows;
pub mod room;

/// How far over the band a fighter stands to the ear, in dB, and the most a render moves it to get there.
pub const OVER_BAND: f64 = 5.0;
pub const TRIM: f64 = 9.0;

/// The shared bus the winner's solo plays on.
pub const SOLO: &str = "solo";

/// A fighter in the mix.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// Its bus.
    pub bus: String,
    /// Its saved level (the gain that puts it [`OVER_BAND`] over the reference band), if the table has one.
    pub saved: Option<f64>,
    /// Most of its energy is rumble: it gets [`lows::treat`] and sends little to the room.
    pub low: bool,
    /// It stings: it gets [`highs::treat`] and sends less to the room.
    pub piercing: bool,
}

/// What the master is told about the fight.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// The two fighters, left corner first.
    pub parts: [Part; 2],
    /// From FIGHT! to the K.O., in seconds: where the fighters are measured.
    pub window: (f64, f64),
    /// The winner's corner (its solo bus follows its bus).
    pub winner: usize,
    /// A bass fighter holds the bottom in the fight (C6, `Polish::hand_bass`): the band's and the hits' buses make
    /// room for it there ([`lows::room_for_bass`]).
    pub bass_room: bool,
}

/// What the master did and reached.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// Each fighter's trim, in dB (0 for a fighter that never sounds in the fight).
    pub trims: [f64; 2],
    /// How far each fighter stood over the band before its trim, in dB.
    pub over_band: [Option<f64>; 2],
    /// How far each fighter was moved after F2 to hold the balance M1 set ([`hold_balance`]), in dB.
    pub held: [f64; 2],
    /// The loudness and the true peak reached.
    pub reached: loud::Reached,
    /// How far F2 pulls each fighter down while the other is loud ([`pull`]), in dB, where it was asked for.
    pub pulls: Option<[f64; 2]>,
    /// The part of the room's energy under 200 Hz during the fight, where it was asked for ([`master_detail`]).
    pub room_rumble: Option<f64>,
}

/// `x` times `g`.
fn scaled(x: &[[f64; 2]], g: f64) -> Vec<[f64; 2]> {
    x.iter().map(|f| [f[0] * g, f[1] * g]).collect()
}

fn add(to: &mut [[f64; 2]], x: &[[f64; 2]], g: f64) {
    for (o, f) in to.iter_mut().zip(x) {
        o[0] += f[0] * g;
        o[1] += f[1] * g;
    }
}

/// The first [`NA`] frames, with what lies past them added onto the start.
fn fold(x: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut y = x[..x.len().min(NA)].to_vec();
    if x.len() > NA {
        add(&mut y, &x[NA..], 1.0);
    }
    y
}

/// The buses of `mix` with their gains and the fighters' treatments, before any trim (steps 1 and 2), by name.
/// Split from [`master`] so the level table is measured on exactly what the master trims.
pub fn treated(mix: &Mix, levels: &Levels, plan: &Plan) -> BTreeMap<String, Vec<[f64; 2]>> {
    let (from, to) = plan.window;
    let mut buses = BTreeMap::new();
    for name in mix.bus_names() {
        let x = mix.bus(name).expect("a bus the mix lists");
        let hand = levels.get(name).map_or(1.0, |(gain, _)| gain);
        let part = plan.parts.iter().position(|p| p.bus == name);
        // the solo is the winner's voice: it follows the winner's level and treatment
        let like = part.or((name == SOLO).then_some(plan.winner));
        let mut gain = hand;
        if let Some(k) = like {
            let p = &plan.parts[k];
            let own = levels.get(&p.bus).map_or(1.0, |(g, _)| g);
            if let (Some(saved), true) = (p.saved, own > 0.0) {
                gain = hand * saved / own;
            }
        }
        let mut y = scaled(x, gain);
        if let Some(k) = like {
            let p = &plan.parts[k];
            if p.low {
                y = if part.is_some() {
                    lows::treat(&y, from, to)
                } else {
                    // the solo sounds after the K.O.: it is measured where it plays
                    lows::treat(&y, to, f64::INFINITY)
                };
            }
            if p.piercing {
                y = highs::treat(&y);
            }
        } else if plan.bass_room {
            y = lows::room_for_bass(&y, from, to);
        }
        buses.insert(name.to_string(), y);
    }
    buses
}

/// The band of a fight: every bus but the fighters' and the solo, summed.
pub fn band(buses: &BTreeMap<String, Vec<[f64; 2]>>, plan: &Plan) -> Vec<[f64; 2]> {
    let len = buses.values().map(Vec::len).max().unwrap_or(0);
    let mut sum = vec![[0.0; 2]; len];
    for (name, x) in buses {
        if name != SOLO && plan.parts.iter().all(|p| &p.bus != name) {
            add(&mut sum, x, 1.0);
        }
    }
    sum
}

/// The mix of a Composed score mastered: see the module notes. `levels` are the bus gains and sends
/// ([`Levels::of`]).
pub fn master(mix: &Mix, levels: &Levels, plan: &Plan) -> (Soundtrack, Report) {
    master_detail(mix, levels, plan, false)
}

/// [`master`]; with `detail` the report also holds what takes time to measure and no render needs.
pub fn master_detail(mix: &Mix, levels: &Levels, plan: &Plan, detail: bool) -> (Soundtrack, Report) {
    let (from, to) = plan.window;
    let mut buses = treated(mix, levels, plan);
    let len = buses.values().map(Vec::len).max().unwrap_or(0);

    // M1: each fighter to OVER_BAND over this fight's band, by at most TRIM either way.
    let band = band(&buses, plan);
    let over_band = [0, 1].map(|k| {
        buses
            .get(&plan.parts[k].bus)
            .and_then(|x| heard::over_band(x, &band, from, to))
    });
    let trims = over_band.map(|o| o.map_or(0.0, |o| (OVER_BAND - o).clamp(-TRIM, TRIM)));
    for (k, &trim) in trims.iter().enumerate() {
        let g = heard::gain(trim);
        let solo = (k == plan.winner).then_some(SOLO);
        for name in [Some(plan.parts[k].bus.as_str()), solo].into_iter().flatten() {
            if let Some(x) = buses.get_mut(name) {
                for f in x.iter_mut() {
                    f[0] *= g;
                    f[1] *= g;
                }
            }
        }
    }
    drop(band);

    // F3: what each bus sends to the room. A fighter sends by its kind; the band's buses keep their own sends.
    let mut send = vec![[0.0; 2]; len];
    let mut dry = vec![[0.0; 2]; len];
    for (name, x) in &buses {
        let part = plan.parts.iter().find(|p| &p.bus == name);
        let amount = match part {
            Some(p) => room::send(p.low, p.piercing),
            None => levels.get(name).map_or(0.0, |(_, send)| send),
        };
        add(&mut send, x, amount);
        add(&mut dry, x, 1.0);
    }
    let mut wet = room::wet(&send, room::SEED);
    drop(send);
    let (dry_level, wet_level) = (
        heard::integrated(heard::span(&dry, from, to)),
        heard::integrated(heard::span(&wet, from, to)),
    );
    let room_gain = if dry_level > -70.0 && wet_level > -70.0 {
        heard::gain(dry_level - room::UNDER - wet_level)
    } else {
        0.0
    };
    for f in &mut wet {
        f[0] *= room_gain;
        f[1] *= room_gain;
    }
    add(&mut dry, &wet, 1.0);
    let room_rumble = detail.then(|| heard::share(heard::span(&wet, from, to), heard::RUMBLE));
    drop(wet);

    // The fold, then F2 and F1 on the mix and, with the same gains, on the stems.
    let mut m = fold(&dry);
    drop(dry);
    let mut stems: BTreeMap<String, Vec<[f64; 2]>> = buses.iter().map(|(name, x)| (name.clone(), fold(x))).collect();
    drop(buses);
    let before: Option<[Vec<[f64; 2]>; 2]> =
        detail.then(|| [0, 1].map(|k| stems.get(&plan.parts[k].bus).cloned().unwrap_or_default()));
    bands::compress(&mut m, stems.values_mut());
    let held = hold_balance(&mut m, &mut stems, plan, over_band, trims);
    // how far F2 pulls each fighter down while the other is loud (F2's "done when"), from the stems either side
    let pulls = before.map(|before| {
        [0, 1].map(|k| {
            let after = stems.get(&plan.parts[k].bus).map_or(&[][..], Vec::as_slice);
            let other = &before[1 - k];
            let g = heard::gain(-held[k]);
            let after: Vec<[f64; 2]> = after.iter().map(|f| [f[0] * g, f[1] * g]).collect();
            pull(
                heard::span(&before[k], from, to),
                heard::span(&after, from, to),
                heard::span(other, from, to),
            )
        })
    });
    let mut each: Vec<&mut Vec<[f64; 2]>> = stems.values_mut().collect();
    let reached = loud::to_target(&mut m, &mut each, loud::TARGET, loud::PEAK);
    (
        Soundtrack { master: m, stems },
        Report {
            trims,
            over_band,
            held,
            reached,
            room_rumble,
            pulls,
        },
    )
}

/// The most [`hold_balance`] moves a fighter by, in dB.
pub const HOLD: f64 = 3.0;

/// After F2 each fighter is brought back to where M1 left it against the band, by at most [`HOLD`] dB: the three
/// ranges are compressed by the mix's level, which the band leads, so a fighter in other ranges than the band's
/// can come out a little louder or softer against it. The stem moves and the mix by the same sound (the mix
/// after F2 is the sum of its stems and the room, so this is exact). Returns each fighter's move, in dB.
fn hold_balance(
    m: &mut [[f64; 2]],
    stems: &mut BTreeMap<String, Vec<[f64; 2]>>,
    plan: &Plan,
    over_band: [Option<f64>; 2],
    trims: [f64; 2],
) -> [f64; 2] {
    let (from, to) = plan.window;
    let band = band(stems, plan);
    let mut held = [0.0; 2];
    for k in 0..2 {
        let (Some(before), Some(x)) = (over_band[k], stems.get(&plan.parts[k].bus)) else {
            continue;
        };
        let Some(now) = heard::over_band(x, &band, from, to) else {
            continue;
        };
        held[k] = (before + trims[k] - now).clamp(-HOLD, HOLD);
    }
    drop(band);
    for (k, &d) in held.iter().enumerate() {
        if d == 0.0 {
            continue;
        }
        let g = heard::gain(d);
        let solo = (k == plan.winner).then_some(SOLO);
        for name in [Some(plan.parts[k].bus.as_str()), solo].into_iter().flatten() {
            if let Some(x) = stems.get_mut(name) {
                for (o, f) in m.iter_mut().zip(x.iter_mut()) {
                    let (l, r) = (f[0] * g, f[1] * g);
                    o[0] += l - f[0];
                    o[1] += r - f[1];
                    *f = [l, r];
                }
            }
        }
    }
    held
}

/// How far a gain pulls a sound down while another one is loud, in dB: `after / before` of the sound in 50 ms
/// blocks where it sounds (within 20 dB of its loudest), its mean over the blocks where `other` is quiet (under its
/// median) less its mean where `other` is loud (its top quarter). 0 when either set is empty.
pub fn pull(before: &[[f64; 2]], after: &[[f64; 2]], other: &[[f64; 2]]) -> f64 {
    let block = 2_400;
    let n = before.len().min(after.len()).min(other.len()) / block;
    let rms = |x: &[[f64; 2]], i: usize| {
        let b = &x[i * block..(i + 1) * block];
        (b.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum::<f64>() / (2 * block) as f64).sqrt()
    };
    let mine: Vec<f64> = (0..n).map(|i| rms(before, i)).collect();
    let top = mine.iter().copied().fold(0.0, f64::max);
    let on: Vec<usize> = (0..n).filter(|&i| mine[i] > top * 0.1 && mine[i] > 0.0).collect();
    let theirs: Vec<f64> = on.iter().map(|&i| rms(other, i)).collect();
    if on.is_empty() {
        return 0.0;
    }
    let median = crate::dsp::percentile(&theirs, 50.0);
    let loud = crate::dsp::percentile(&theirs, 75.0);
    let gain = |i: usize| heard::db(rms(after, i) / mine[i]);
    let mean = |pick: &dyn Fn(f64) -> bool| {
        let g: Vec<f64> = on
            .iter()
            .zip(&theirs)
            .filter(|(_, t)| pick(**t))
            .map(|(&i, _)| gain(i))
            .collect();
        (!g.is_empty()).then(|| g.iter().sum::<f64>() / g.len() as f64)
    };
    match (mean(&|t| t < median), mean(&|t| t >= loud)) {
        (Some(q), Some(l)) => q - l,
        _ => 0.0,
    }
}
