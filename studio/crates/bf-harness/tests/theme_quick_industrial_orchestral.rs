//! The industrial bed (#40 v2) and the orchestral drill (#42), quick-ported (ruling M4-R16): each bed builds a whole
//! soundtrack for a few pairs (its episode's own and two others) on its preset's tempo, key and chords, without a
//! panic, every bed bus playing. No parity gates: the beds are checked by ear against the episodes.

use bf_content::themes::{self, Preset};
use bf_content::{build_soundtrack, roster, starter, validate};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_export::{BandPlan, checks};
use bf_sound::NA;

/// The starter of `a` vs `b` playing `theme` with its first preset's music.
fn fight_file(theme: &str, a: &str, b: &str) -> FightFile {
    let t = themes::theme(theme).unwrap_or_else(|| panic!("{theme} is not listed"));
    let p: &Preset = &t.card.presets[0];
    let mut ff = starter(a, b).unwrap_or_else(|e| panic!("{a} v {b}: {e}"));
    ff.music.arrangement = "classic".to_string();
    ff.tempo_bpm = p.bpm;
    let m = &mut ff.music;
    m.theme = theme.to_string();
    m.theme_options.clear();
    m.key = p.key.to_string();
    m.scale = p.scale.to_string();
    m.chords_intro = p.intro.iter().map(|c| c.to_string()).collect();
    m.chord_loop = p.chord_loop.iter().map(|c| c.to_string()).collect();
    m.chords_tail = p.tail.iter().map(|c| c.to_string()).collect();
    m.lines.clear();
    ff
}

/// Each pair's soundtrack under `theme`. `late`: bed buses that play only in the final exchange (silent when the
/// K.O. comes before it).
fn builds(theme: &str, pairs: [(&str, &str); 3], late: &[&str]) {
    let bed = themes::theme(theme)
        .unwrap()
        .bed("classic", &Default::default())
        .unwrap();
    let own: Vec<&str> = bed.buses().iter().map(|(b, _)| *b).collect();
    for (a, b) in pairs {
        let ff = fight_file(theme, a, b);
        validate(&ff).unwrap_or_else(|e| panic!("{theme}, {a} v {b}: {e}"));
        let rec = (1..=40u64)
            .map(|seed| {
                let setup = bf_sim::setup_from(&ff, roster(), seed).unwrap();
                bf_sim::run(&setup, true).unwrap()
            })
            .find(|r| r.t_ko.is_some())
            .unwrap_or_else(|| panic!("{theme}, {a} v {b}: no K.O. in 40 seeds"));
        let st = build_soundtrack(&rec, &ff).unwrap_or_else(|e| panic!("{theme}, {a} v {b}: {e}"));
        assert_eq!(st.master.len(), NA);
        assert!(st.master.as_flattened().iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        for bus in own.iter().filter(|b| !late.contains(b)) {
            let x = st.stems.get(*bus).unwrap_or_else(|| panic!("{theme}: no {bus} stem"));
            assert!(
                x.as_flattened().iter().any(|v| v.abs() > 1e-3),
                "{theme}, {a} v {b}: {bus} is silent"
            );
        }
        println!("{theme}, {a} v {b}: seed {} built", rec.seed);
        // the render's own sound checks, printed for the ear check (not gated: a quick port)
        let show = Show::new(&rec, &ff).unwrap();
        let plan = BandPlan::of(&ff).unwrap();
        let master = checks::as_wav16(&st.master);
        let sounds = [
            (plan.sounds[0].0.as_str(), plan.sounds[0].1),
            (plan.sounds[1].0.as_str(), plan.sounds[1].1),
        ];
        let bed_buses: Vec<&str> = plan.bed_buses.iter().map(String::as_str).collect();
        for r in [
            checks::levels(&master),
            checks::band(&st, &show, sounds, &bed_buses),
            checks::loudness(&master, &show),
            checks::loop_audio(&master),
        ] {
            println!("  {} {}: {}", if r.pass { "pass" } else { "FAIL" }, r.name, r.summary);
        }
        // a short clip to listen to (the card, the count-in and the fight's first bars), when asked for
        if let Some(dir) = std::env::var_os("BF_QUICK_CLIP_DIR") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).unwrap();
            let name = format!("{theme}_{}_v_{}", a.replace(' ', "_"), b.replace(' ', "_")).to_lowercase();
            let clip = &st.master[..(20 * bf_sound::SR).min(st.master.len())];
            bf_sound::write_wav16(&dir.join(format!("{name}_clip.wav")), clip).unwrap();
            bf_sound::write_wav16(&dir.join(format!("{name}.wav")), &st.master).unwrap();
        }
    }
}

#[test]
fn industrial_builds() {
    builds(
        "industrial",
        [("BARI SAX", "BASS CLARINET"), ("VIOLIN", "CELLO"), ("TUBA", "ORGAN")],
        &[],
    );
}

#[test]
fn orchestral_drill_builds() {
    builds(
        "orchestral_drill",
        [("VIOLA", "VIOLIN"), ("BARI SAX", "FLUTE"), ("TRUMPET", "HARP")],
        &[],
    );
}
