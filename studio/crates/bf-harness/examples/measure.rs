//! Measures the sound upgrade's "done when" figures on fight files and prints one JSON line a fight:
//! `cargo run --release -p bf-harness --example measure -- <fight.toml>...`
//!
//! A fight under the Composed arrangement is scored once and mastered twice: as it was before the sound upgrade
//! (the Classic master's single compressor and squash, on the same mix) and as it is now. For each fighter, in
//! both: how far it stands over the band to the ear, what it loses on a phone, its rumble and piercing shares,
//! its level between 2 and 6 kHz against the other fighter, and how far the master's gain on it dips; for the
//! whole: the loudness, the true peak, the room's rumble and who holds the lows.

use bf_content::{SoundBuild, roster};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_sound::dsp::{FilterKind, butter_sos, sosfilt_stereo};
use bf_sound::master::{MASTER_CEILING, Soundtrack, master};
use bf_sound::{SRF, heard, polish};
use serde_json::{Value, json};

/// The mean of the levels of the blocks where the sound is on (within 20 dB of its loudest, above -65).
fn on_mean(levels: &[f64]) -> Option<f64> {
    let top = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let on: Vec<f64> = levels
        .iter()
        .copied()
        .filter(|&l| l > (top - 20.0).max(-65.0))
        .collect();
    (!on.is_empty()).then(|| on.iter().sum::<f64>() / on.len() as f64)
}

/// How far the gain the master put on a fighter dips while it plays: the median less the 5th percentile, in dB,
/// of `after / before` over 50 ms blocks.
fn dip(before: &[[f64; 2]], after: &[[f64; 2]]) -> Option<f64> {
    let block = 2_400;
    let rms = |x: &[[f64; 2]]| (x.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum::<f64>() / (2 * x.len()) as f64).sqrt();
    let n = before.len().min(after.len()) / block;
    let levels: Vec<f64> = (0..n).map(|i| rms(&before[i * block..(i + 1) * block])).collect();
    let top = levels.iter().copied().fold(0.0, f64::max);
    let mut gains: Vec<f64> = (0..n)
        .filter(|&i| levels[i] > top * 0.1)
        .map(|i| heard::db(rms(&after[i * block..(i + 1) * block]) / levels[i]))
        .collect();
    gains.sort_by(|a, b| a.total_cmp(b));
    (!gains.is_empty()).then(|| gains[gains.len() / 2] - gains[gains.len() / 20])
}

fn round(x: f64) -> Value {
    json!((x * 100.0).round() / 100.0)
}

fn opt(x: Option<f64>) -> Value {
    x.map_or(Value::Null, round)
}

/// The figures of one master.
fn figures(st: &Soundtrack, plan: &polish::Plan, raw: &[Vec<[f64; 2]>; 2]) -> Value {
    let (from, to) = plan.window;
    let band = polish::band(&st.stems, plan);
    let piercing = butter_sos(FilterKind::Bp, &[2000.0, 6000.0], 4, SRF);
    let stem = |k: usize| st.stems.get(&plan.parts[k].bus);
    let overall = |k: usize| stem(k).and_then(|x| on_mean(&heard::heard(heard::span(x, from, to))));
    let lows = |x: &[[f64; 2]]| {
        let s = heard::span(x, from, to);
        heard::share(s, heard::RUMBLE) * s.iter().map(|f| f[0] * f[0] + f[1] * f[1]).sum::<f64>()
    };
    let all_lows: f64 = st.stems.values().map(|x| lows(x)).sum();
    let fighters: Vec<Value> = (0..2)
        .map(|k| {
            let Some(x) = stem(k) else { return Value::Null };
            let fight = heard::span(x, from, to);
            let sting = on_mean(&heard::heard(&sosfilt_stereo(&piercing, fight)));
            json!({
                "bus": plan.parts[k].bus,
                "over_band": opt(heard::over_band(x, &band, from, to)),
                "phone_loss": round(heard::phone_loss(fight)),
                "rumble": round(heard::share(fight, heard::RUMBLE)),
                "piercing": round(heard::share(fight, heard::PIERCING)),
                "sting_over_other": opt(sting.zip(overall(1 - k)).map(|(a, b)| a - b)),
                "gain_dip": opt(dip(heard::span(&raw[k], from, to), fight)),
                "lows_held": round(if all_lows > 0.0 { lows(x) / all_lows } else { 0.0 }),
            })
        })
        .collect();
    // where the lows of the fight come from: each bus's part of the energy under 200 Hz, where it is 2% or more
    let lows_by_bus: serde_json::Map<String, Value> = st
        .stems
        .iter()
        .filter(|_| all_lows > 0.0)
        .map(|(name, x)| (name.clone(), lows(x) / all_lows))
        .filter(|(_, share)| *share >= 0.02)
        .map(|(name, share)| (name, json!(round(share))))
        .collect();
    let peak = st
        .master
        .iter()
        .flat_map(|f| [f[0].abs(), f[1].abs()])
        .fold(0.0, f64::max);
    json!({
        "fighters": fighters,
        "lufs": round(heard::integrated(&st.master)),
        "true_peak_db": round(heard::db(heard::true_peak(&st.master))),
        "peak": round(peak),
        "band_phone_loss": round(heard::phone_loss(heard::span(&band, from, to))),
        "lows_by_bus": lows_by_bus,
    })
}

fn main() {
    for path in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&path).expect("a fight file");
        let mut ff = FightFile::parse(&text).expect("a fight file that parses");
        // the figures are of the Composed arrangement, whatever the file was saved with
        ff.music.arrangement = bf_content::themes::COMPOSED.to_string();
        let seed = ff.seed.expect("a fight file with a seed");
        let setup = bf_sim::setup_from(&ff, roster(), seed).expect("a fight");
        let rec = bf_sim::run(&setup, true).expect("the fight runs");
        let show = Show::new(&rec, &ff).expect("a fight with a K.O.");
        let build = SoundBuild::new(&show, &rec, &ff).expect("the sound build");
        let mix = build.score().expect("the score");
        let plan = build.plan(None).expect("a fight under the Composed arrangement");
        let levels = build.levels();
        let raw: [Vec<[f64; 2]>; 2] = [0, 1].map(|k| {
            let gain = levels.get(&plan.parts[k].bus).map_or(1.0, |(g, _)| g);
            mix.bus(&plan.parts[k].bus)
                .map(|x| x.iter().map(|f| [f[0] * gain, f[1] * gain]).collect())
                .unwrap_or_default()
        });
        let today = master(&mix, &levels, build.bed.as_ref(), MASTER_CEILING);
        let (new, report) = build.master_report(&mix).expect("the Composed master");
        let (from, to) = plan.window;
        let pull_today = [0, 1].map(|k| {
            let after = today.stems.get(&plan.parts[k].bus).map_or(&[][..], Vec::as_slice);
            round(polish::pull(
                heard::span(&raw[k], from, to),
                heard::span(after, from, to),
                heard::span(&raw[1 - k], from, to),
            ))
        });
        let levels_check = {
            let r = bf_export::checks::levels(&bf_export::checks::as_wav16(&new.master));
            json!({"pass": r.pass, "summary": r.summary})
        };
        let line = json!({
            "fight": std::path::Path::new(&path).file_stem().and_then(|s| s.to_str()),
            "fighters": ff.fighters,
            "theme": ff.music.theme,
            "winner": show.win,
            "today": figures(&today, &plan, &raw),
            "new": figures(&new, &plan, &raw),
            "trims": report.trims.map(round),
            "held": report.held.map(round),
            "pull_today": pull_today,
            "pull_new": report.pulls.map(|p| p.map(round)),
            "over_band_before_trim": report.over_band.map(opt),
            "room_rumble": opt(report.room_rumble),
            "levels_check": levels_check,
            "kinds": plan.parts.iter().map(|p| json!({"low": p.low, "piercing": p.piercing, "saved": p.saved})).collect::<Vec<_>>(),
        });
        println!("{line}");
    }
}
