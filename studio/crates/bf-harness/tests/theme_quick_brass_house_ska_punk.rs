//! The brass house and the ska punk, quick-ported (ruling M4-R16): each bed builds a whole soundtrack under three
//! pairs of fighters without a panic, every one of its buses sounds, and the master is whole and loud. No parity
//! gates: the ports are checked by ear against #36 and #39.

use bf_content::SoundBuild;
use bf_core::fightfile::FightFile;
use bf_core::show::Show;

/// The example fight `base` under `theme` in its episode's key and loop, fought on the first seed that ends in a
/// K.O.; the bed's buses that played, and the master's peak and RMS.
fn under_the_theme(
    base: &str,
    theme: &str,
    bpm: f64,
    key: &str,
    intro: [&str; 2],
    lp: [&str; 4],
) -> (Vec<String>, f64, f64) {
    let path = format!("{}/../../fights/{base}.toml", env!("CARGO_MANIFEST_DIR"));
    let mut ff = FightFile::parse(&std::fs::read_to_string(&path).expect("the fight file")).expect("it parses");
    ff.music.theme = theme.into();
    ff.music.theme_options.clear();
    ff.music.key = key.into();
    ff.music.scale = "minor".into();
    ff.music.chords_intro = intro.map(String::from).to_vec();
    ff.music.chord_loop = [lp, lp].concat().into_iter().map(String::from).collect();
    ff.music.chords_tail = vec![];
    ff.tempo_bpm = bpm;
    let roster = bf_content::roster();
    let rec = [ff.seed.unwrap_or(1)]
        .into_iter()
        .chain(1..40)
        .find_map(|seed| {
            let setup = bf_sim::setup_from(&ff, roster, seed).expect("a setup");
            let rec = bf_sim::run(&setup, true).expect("a fight");
            rec.t_ko.is_some().then_some(rec)
        })
        .unwrap_or_else(|| panic!("{base}: no K.O. in 40 seeds"));
    let show = Show::new(&rec, &ff).expect("a show");
    let build = SoundBuild::new(&show, &rec, &ff).expect("the build");
    let mix = build.score().unwrap_or_else(|e| panic!("{base}: {e}"));
    let st = build.master(&mix);
    let played: Vec<String> = build
        .bed
        .buses()
        .iter()
        .filter(|(b, _)| st.stems.get(*b).is_some_and(|x| x.iter().any(|f| f[0] != 0.0)))
        .map(|(b, _)| b.to_string())
        .collect();
    let peak = st.master.iter().fold(0.0f64, |a, f| a.max(f[0].abs()).max(f[1].abs()));
    let rms = (st.master.iter().map(|f| f[0] * f[0]).sum::<f64>() / st.master.len() as f64).sqrt();
    (played, peak, rms)
}

const PAIRS: [&str; 3] = ["flute_vs_clarinet", "trombone_vs_tuba", "piano_vs_organ"];

#[test]
fn brass_house_builds_under_other_pairs() {
    for base in PAIRS {
        let (played, peak, rms) =
            under_the_theme(base, "brass_house", 126.0, "F", ["Fm", "C"], ["Fm", "Db", "Bbm", "C"]);
        assert_eq!(played, ["kit", "pad", "stabs"], "{base}");
        assert!(peak.is_finite() && peak > 0.5 && peak <= 1.0, "{base}: peak {peak}");
        assert!(rms > 0.03, "{base}: rms {rms}");
    }
}

#[test]
fn ska_punk_builds_under_other_pairs() {
    for base in PAIRS {
        let (played, peak, rms) = under_the_theme(base, "ska_punk", 170.0, "E", ["Em", "B"], ["Em", "C", "D", "B"]);
        assert_eq!(played, ["kit", "skank", "bubble", "ropes"], "{base}");
        assert!(peak.is_finite() && peak > 0.5 && peak <= 1.0, "{base}: peak {peak}");
        assert!(rms > 0.03, "{base}: rms {rms}");
    }
}
