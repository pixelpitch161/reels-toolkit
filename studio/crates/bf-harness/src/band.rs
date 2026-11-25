//! The band of each theme's Composed arrangement as a phone plays it (sound upgrade, stage 2: M6).
//!
//! [`measure`] renders a theme's fight (its preset pair, as the Classic lock picks it, under Composed, the first of
//! seeds 1 to 12 with a K.O.) and reads its band from FIGHT! to the K.O.: every bus but the two fighters', the
//! solo and the shared `fx`, at the master's gains and after the mix's treatments ([`polish::treated`]), before
//! the master's compressor. The figure that counts is [`BandFigures::phone_loss`]: the loudness the band loses
//! through a phone's speaker. M6 holds it at [`PHONE_TARGET`] dB or less.
//!
//! [`propose`] is the dev command's part (`cargo run --release -p bf-harness --example bands`): the bus levels that
//! would bring the band under the target, steered toward an even balance of its octaves, with its loudness kept.

use bf_content::themes::COMPOSED;
use bf_content::{SoundBuild, roster};
use bf_core::show::Show;
use bf_sound::SRF;
use bf_sound::dsp::fft::Fft;
use bf_sound::heard;
use bf_sound::master::BUS;
use bf_sound::polish;

use crate::golden::{self, GoldenFight};

/// The most a band may lose on a phone, in dB (M6).
pub const PHONE_TARGET: f64 = 5.0;

/// The centres of the octaves the balance is read in, Hz.
pub const OCTAVES: [f64; 8] = [63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];

/// The octave balance the levels are steered toward (each octave against the strongest, dB): a guide, not a gate.
pub const BALANCE: [f64; 8] = [0.0, -1.0, -3.5, -5.5, -7.5, -9.5, -11.5, -14.0];

/// One bus of a band.
#[derive(Clone, Debug)]
pub struct BusFigures {
    pub name: String,
    /// Its K-weighted energy over the fight, and that of what a phone plays of it.
    pub energy: f64,
    pub phone_energy: f64,
    /// Its power in each of [`OCTAVES`].
    pub octaves: [f64; 8],
}

/// A theme's band over its fight.
#[derive(Clone, Debug)]
pub struct BandFigures {
    pub theme: String,
    pub fighters: [String; 2],
    pub seed: u64,
    /// Whether the bed plays the theme's own band (its groove).
    pub grooved: bool,
    /// The loudness the band loses on a phone, dB.
    pub phone_loss: f64,
    /// The band's integrated loudness, LUFS.
    pub loudness: f64,
    pub buses: Vec<BusFigures>,
}

/// Each octave's power of `x` (its channels' mean).
fn octaves(x: &[[f64; 2]]) -> [f64; 8] {
    let mut out = [0.0; 8];
    if x.is_empty() {
        return out;
    }
    let n = x.len().next_power_of_two();
    let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
    for (r, f) in re.iter_mut().zip(x) {
        *r = (f[0] + f[1]) / 2.0;
    }
    Fft::new(n).forward(&mut re, &mut im);
    for k in 1..=n / 2 {
        let f = k as f64 * SRF / n as f64;
        if let Some(o) = OCTAVES
            .iter()
            .position(|&c| f >= c / 2f64.sqrt() && f < c * 2f64.sqrt())
        {
            out[o] += re[k] * re[k] + im[k] * im[k];
        }
    }
    out
}

fn energy(x: &[[f64; 2]]) -> f64 {
    x.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum()
}

/// The fight a theme's band is measured on: its pinned Classic fight's pair, tempo, key and chords.
pub fn fight_of(theme: &str) -> Result<GoldenFight, String> {
    golden::fights()
        .into_iter()
        .find(|g| g.theme == theme)
        .ok_or_else(|| format!("{theme}: no pair plays it"))
}

/// Measures the band of theme `theme`: see the module notes.
///
/// # Errors
/// The theme has no pair, no seed of 1 to 12 ends in a K.O., or the render fails.
pub fn measure(theme: &str) -> Result<BandFigures, String> {
    let g = fight_of(theme)?;
    let mut ff = golden::fight_file(&g)?;
    ff.music.arrangement = COMPOSED.to_string();
    let (seed, rec) = (1..=12u64)
        .find_map(|seed| {
            let setup = bf_sim::setup_from(&ff, roster(), seed).ok()?;
            let rec = bf_sim::run(&setup, true).ok()?;
            rec.t_ko.is_some().then_some((seed, rec))
        })
        .ok_or_else(|| format!("{theme}: no K.O. in 12 seeds"))?;
    let show = Show::new(&rec, &ff).map_err(|e| e.to_string())?;
    let build = SoundBuild::new(&show, &rec, &ff).map_err(|e| format!("{theme}: {e}"))?;
    let mix = build.score().map_err(|e| format!("{theme}: {e}"))?;
    let plan = build.plan(None).ok_or_else(|| format!("{theme}: not under Composed"))?;
    let buses = polish::treated(&mix, &build.levels(), &plan);
    let (from, to) = plan.window;
    let mut band = Vec::new();
    let mut figures = Vec::new();
    for (name, x) in &buses {
        let shared = BUS.iter().any(|(b, _)| b == name);
        if shared || plan.parts.iter().any(|p| &p.bus == name) {
            continue;
        }
        let x = heard::span(x, from, to);
        if band.is_empty() {
            band = vec![[0.0; 2]; x.len()];
        }
        for (s, f) in band.iter_mut().zip(x) {
            s[0] += f[0];
            s[1] += f[1];
        }
        figures.push(BusFigures {
            name: name.clone(),
            energy: energy(&heard::k_weight(x)),
            phone_energy: energy(&heard::k_weight(&heard::phone(x))),
            octaves: octaves(x),
        });
    }
    Ok(BandFigures {
        theme: theme.to_string(),
        fighters: [g.a, g.b],
        seed,
        grooved: build.bed.has_groove(),
        phone_loss: heard::phone_loss(&band),
        loudness: heard::integrated(&band),
        buses: figures,
    })
}

/// A bus loses this much on a phone or more, and holds at least [`LOWS_SHARE`] of the band: a candidate for the
/// groove's `lows` (M2's overtones), which bring a bus to about [`bf_sound::polish::lows::PHONE_LOSS`].
pub const LOWS_OVER: f64 = 12.0;
pub const LOWS_SHARE: f64 = 0.1;

/// The buses [`LOWS_OVER`] names.
pub fn lows_candidates(f: &BandFigures) -> Vec<String> {
    let all: f64 = f.buses.iter().map(|b| b.energy).sum();
    f.buses
        .iter()
        .filter(|b| b.energy >= LOWS_SHARE * all && b.loss() >= LOWS_OVER)
        .map(|b| b.name.clone())
        .collect()
}

/// `f` as the band would be with the buses `lows` given M2's overtones (each then losing what M2 leaves).
pub fn with_lows(f: &BandFigures, lows: &[String]) -> BandFigures {
    let mut f = f.clone();
    for b in f.buses.iter_mut().filter(|b| lows.contains(&b.name)) {
        let left = bf_sound::polish::lows::PHONE_LOSS.min(b.loss());
        b.phone_energy = b.energy / 10f64.powf(left / 10.0);
    }
    f
}

impl BusFigures {
    /// What the bus loses on a phone, dB.
    pub fn loss(&self) -> f64 {
        10.0 * (self.energy / self.phone_energy.max(1e-30)).log10()
    }
}

/// What [`propose`] found: each bus's change in dB, and the band's phone loss and octave balance it predicts.
#[derive(Clone, Debug)]
pub struct Proposal {
    pub changes: Vec<(String, f64)>,
    pub phone_loss: f64,
    pub balance_before: [f64; 8],
    pub balance_after: [f64; 8],
}

/// The band's phone loss with each bus at `gains` (power factors), from the buses' energies (the buses are taken
/// as unrelated; the real figure is measured again after).
fn predicted_loss(f: &BandFigures, gains: &[f64]) -> f64 {
    let all: f64 = f.buses.iter().zip(gains).map(|(b, g)| b.energy * g).sum();
    let phone: f64 = f.buses.iter().zip(gains).map(|(b, g)| b.phone_energy * g).sum();
    10.0 * (all / phone.max(1e-30)).log10()
}

fn balance(f: &BandFigures, gains: &[f64]) -> [f64; 8] {
    let mut p = [0.0; 8];
    for (b, g) in f.buses.iter().zip(gains) {
        for (o, v) in p.iter_mut().zip(b.octaves) {
            *o += v * g;
        }
    }
    let db = p.map(|v| 10.0 * (v + 1e-30).log10());
    let top = db.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    db.map(|v| v - top)
}

/// The bus changes (each within `reach` dB, so no two move more than 2 × `reach` apart) that bring the band to
/// [`PHONE_TARGET`] less half a dB, as close to the [`BALANCE`] guide as that allows and moving as little as it
/// can; the whole moved back to the band's loudness.
pub fn propose(f: &BandFigures, reach: f64) -> Proposal {
    let n = f.buses.len();
    let power = |db: f64| 10f64.powf(db / 10.0);
    let cost = |d: &[f64]| {
        let g: Vec<f64> = d.iter().map(|&v| power(v)).collect();
        let over = (predicted_loss(f, &g) - (PHONE_TARGET - 0.5)).max(0.0);
        let guide: f64 = balance(f, &g).iter().zip(BALANCE).map(|(a, b)| (a - b) * (a - b)).sum();
        100.0 * over * over + 0.02 * guide + 0.05 * d.iter().map(|v| v * v).sum::<f64>()
    };
    let mut d = vec![0.0; n];
    let steps: Vec<f64> = (0..=(4.0 * reach) as i32).map(|i| -reach + 0.5 * i as f64).collect();
    for _ in 0..40 {
        let before = d.clone();
        for k in 0..n {
            let best = steps
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    let mut da = d.clone();
                    da[k] = a;
                    let mut db = d.clone();
                    db[k] = b;
                    cost(&da).total_cmp(&cost(&db))
                })
                .unwrap_or(0.0);
            d[k] = best;
        }
        if d == before {
            break;
        }
    }
    // the band as loud as before
    let all: f64 = f.buses.iter().map(|b| b.energy).sum();
    let now: f64 = f.buses.iter().zip(&d).map(|(b, v)| b.energy * power(*v)).sum();
    let back = 10.0 * (all / now.max(1e-30)).log10();
    let d: Vec<f64> = d.iter().map(|v| ((v + back) * 2.0).round() / 2.0).collect();
    let g: Vec<f64> = d.iter().map(|&v| power(v)).collect();
    Proposal {
        changes: f.buses.iter().map(|b| b.name.clone()).zip(d.iter().copied()).collect(),
        phone_loss: predicted_loss(f, &g),
        balance_before: balance(f, &vec![1.0; n]),
        balance_after: balance(f, &g),
    }
}
