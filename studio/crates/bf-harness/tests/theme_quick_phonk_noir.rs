//! The drift phonk (#27) and the noir jazz (#29), quick-ported by ear (ruling M4-R16): each bed builds a whole
//! soundtrack for its stand-in pairs (today's fighters; the episodes' own pairs are not in this app) and one more,
//! under the pair's own harmony and under the theme's preset, without a panic, and the master is a sound, not
//! silence. No parity gates: the beds are checked by ear against their episodes.

use bf_content::themes::theme;
use bf_content::{build_soundtrack, roster, starter};
use bf_core::fightfile::FightFile;

/// The pair's starter under `name`; with `preset`, the theme's preset `p`'s tempo, key and chords too.
fn fight_file(a: &str, b: &str, name: &str, preset: Option<usize>) -> FightFile {
    let mut ff = starter(a, b).expect("a starter");
    ff.music.arrangement = "classic".to_string();
    ff.music.theme = name.to_string();
    ff.music.theme_options.clear();
    ff.music.mix.clear();
    if let Some(i) = preset {
        let p = &theme(name).expect("the theme").card.presets[i];
        ff.tempo_bpm = p.bpm;
        ff.music.key = p.key.to_string();
        ff.music.scale = p.scale.to_string();
        ff.music.chords_intro = p.intro.iter().map(|c| c.to_string()).collect();
        ff.music.chord_loop = p.chord_loop.iter().map(|c| c.to_string()).collect();
        ff.music.chords_tail = p.tail.iter().map(|c| c.to_string()).collect();
        // The starter's lines and solos follow its own harmony; the parts play their sketch under the preset's.
        ff.music.lines.clear();
        ff.music.solos.clear();
    }
    ff
}

fn builds(name: &str, extra: (&str, &str)) {
    let presets = theme(name).expect("the theme").card.presets;
    let mut runs: Vec<(&str, &str, Option<usize>)> = Vec::new();
    for (i, p) in presets.iter().enumerate() {
        runs.push((p.fighters[0], p.fighters[1], Some(i)));
    }
    runs.push((presets[0].fighters[0], presets[0].fighters[1], None));
    runs.push((extra.0, extra.1, None));
    for (a, b, preset) in runs {
        let ff = fight_file(a, b, name, preset);
        bf_content::validate(&ff).unwrap_or_else(|e| panic!("{name}, {a} v {b}: {e}"));
        let rec = (1..=12u64)
            .map(|seed| {
                let setup = bf_sim::setup_from(&ff, roster(), seed).expect("a setup");
                bf_sim::run(&setup, true).expect("a fight")
            })
            .find(|rec| rec.t_ko.is_some())
            .unwrap_or_else(|| panic!("{a} v {b}: no K.O. in 12 seeds"));
        let st = build_soundtrack(&rec, &ff).unwrap_or_else(|e| panic!("{name}, {a} v {b}: {e}"));
        let peak = st.master.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(st.master.iter().flatten().all(|v| v.is_finite()), "{name}, {a} v {b}");
        assert!(peak > 0.5 && peak <= 1.0, "{name}, {a} v {b}: peak {peak}");
        for bus in theme(name)
            .unwrap()
            .bed(bf_content::themes::CLASSIC, &Default::default())
            .unwrap()
            .buses()
        {
            assert!(st.stems.contains_key(bus.0), "{name}, {a} v {b}: no {} stem", bus.0);
        }
    }
}

#[test]
fn the_drift_phonk_builds() {
    builds("drift_phonk", ("TRUMPET", "TENOR SAX"));
}

#[test]
fn the_noir_jazz_builds() {
    builds("noir_jazz", ("PIANO", "ELECTRIC"));
}
