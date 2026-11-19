//! The Composed arrangements of the first 13 themes (milestone 5, Task 4): for each theme, a composed bed builds
//! and plays a whole song for its preset's pairing, and the master passes the levels check. The fight is the first
//! whose classic soundtrack passes it: the layered bed hands the K.O. and the bars after it to the classic bed.
//! Judged by ear otherwise: `BF_COMPOSED_CLIP_DIR` writes a 20 s clip and the whole song; `BF_COMPOSED_COMPARE`
//! prints each bed bus's loudness next to the classic bed's.

use bf_content::ContentError;
use bf_content::themes::{self, Preset, ThemeOptions};
use bf_content::{SoundBuild, roster, starter, validate};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_export::checks;
use bf_sound::NA;
use bf_sound::music::layers::{Cue, CueKind, LayeredBed};
use bf_sound::music::signal::{MomentKind, StorySignal};

type Composed = fn(&ThemeOptions, Vec<Cue>, u64) -> Result<LayeredBed, ContentError>;

/// The starter of the theme's first preset's fighters, on that preset's music.
fn fight_file(theme: &str) -> FightFile {
    let t = themes::theme(theme).unwrap_or_else(|| panic!("{theme} is not listed"));
    let p: &Preset = &t.card.presets[0];
    let [a, b] = p.fighters;
    assert!(
        themes::themes_for(a, b).contains(&theme),
        "{theme} is not offered for {a} v {b}"
    );
    let mut ff = starter(a, b).unwrap_or_else(|e| panic!("{a} v {b}: {e}"));
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
    m.arrangement = "classic".to_string();
    ff
}

/// The story moments the layered bed listens to.
fn cues(signal: &StorySignal) -> Vec<Cue> {
    signal
        .moments
        .iter()
        .filter_map(|m| {
            let kind = match m.kind {
                MomentKind::LeadChange { .. } => CueKind::LeadChange,
                MomentKind::Ability { .. } => CueKind::Ability,
                MomentKind::Desperation { .. } => CueKind::Desperation,
                MomentKind::FinalExchange => CueKind::FinalExchange,
                _ => return None,
            };
            Some(Cue {
                bar: m.bar,
                s16: (m.step % 16) as u32,
                kind,
            })
        })
        .collect()
}

fn rms_db(x: &[[f64; 2]]) -> f64 {
    let s: f64 = x.as_flattened().iter().map(|v| v * v).sum();
    10.0 * (s / (2 * x.len().max(1)) as f64 + 1e-20).log10()
}

fn composed_builds(theme: &str, composed: Composed) {
    let ff = fight_file(theme);
    validate(&ff).unwrap_or_else(|e| panic!("{theme}: {e}"));
    // the first fight whose classic soundtrack passes the levels check: the sections the layered bed hands to the
    // classic one (the K.O., the bars after it) are the classic's
    let (rec, classic) = (1..=40u64)
        .map(|seed| bf_sim::run(&bf_sim::setup_from(&ff, roster(), seed).unwrap(), true).unwrap())
        .filter(|r| r.t_ko.is_some())
        .find_map(|rec| {
            let classic = {
                let show = Show::new(&rec, &ff).unwrap();
                let build = SoundBuild::new(&show, &rec, &ff).unwrap();
                build.master(&build.score().unwrap())
            };
            checks::levels(&checks::as_wav16(&classic.master))
                .pass
                .then_some((rec, classic))
        })
        .unwrap_or_else(|| panic!("{theme}: no K.O. with an even classic soundtrack in 40 seeds"));
    let show = Show::new(&rec, &ff).unwrap();
    let options = themes::theme(theme).unwrap().options_of(&ff).unwrap();
    let mut build = SoundBuild::new(&show, &rec, &ff).unwrap();
    let signal = StorySignal::from(&build.ctx).unwrap();
    let bed = composed(&options, cues(&signal), rec.seed).unwrap_or_else(|e| panic!("{theme}: {e}"));
    let own: Vec<&str> = bf_sound::traits::Bed::buses(&bed).iter().map(|(b, _)| *b).collect();
    let own: Vec<String> = own.into_iter().map(String::from).collect();
    build.bed = Box::new(bed);
    let mix = build.score().unwrap_or_else(|e| panic!("{theme}: {e}"));
    let st = build.master(&mix);
    assert_eq!(st.master.len(), NA);
    assert!(st.master.as_flattened().iter().all(|v| v.is_finite() && v.abs() <= 1.0));
    let levels = checks::levels(&checks::as_wav16(&st.master));
    println!("{theme}: seed {}, {}", rec.seed, levels.summary);
    println!(
        "  bar {:.2} s, fight bar {}, K.O. {:.1} s, solo {:.1} s, bars {}",
        show.grid.bar, show.grid.fight_bar, show.t_ko, show.t_solo, show.grid.n_bars
    );
    if std::env::var_os("BF_COMPOSED_COMPARE").is_some() {
        let w = |x: &[[f64; 2]]| -> String {
            x.chunks(2 * bf_sound::SR)
                .map(|c| format!("{:.1}", rms_db(c)))
                .collect::<Vec<_>>()
                .join(" ")
        };
        println!(
            "  classic: {}\n  classic  {}\n  composed {}",
            checks::levels(&checks::as_wav16(&classic.master)).summary,
            w(&classic.master),
            w(&st.master)
        );
        for bus in &own {
            let c = classic.stems.get(bus.as_str()).map_or(-120.0, |x| rms_db(x));
            let i = (show.t_solo * bf_sound::SRF) as usize;
            let late = |s: &std::collections::BTreeMap<String, Vec<[f64; 2]>>| {
                s.get(bus.as_str()).map_or(-120.0, |x| rms_db(&x[i.min(x.len())..]))
            };
            let n = st.stems.get(bus.as_str()).map_or(-120.0, |x| rms_db(x));
            println!("  {bus:>10}: classic {c:6.1} dB, composed {n:6.1} dB");
            println!(
                "  {:>10}  after the solo: classic {:6.1} dB, composed {:6.1} dB",
                "",
                late(&classic.stems),
                late(&st.stems)
            );
        }
    }
    if let Some(dir) = std::env::var_os("BF_COMPOSED_CLIP_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let t0 = (show.grid.bar_t(show.grid.fight_bar as f64) * bf_sound::SRF) as usize;
        let clip = &st.master[t0..(t0 + 20 * bf_sound::SR).min(st.master.len())];
        bf_sound::write_wav16(&dir.join(format!("{theme}_composed_clip.wav")), clip).unwrap();
        bf_sound::write_wav16(&dir.join(format!("{theme}_composed.wav")), &st.master).unwrap();
    }
    assert!(levels.pass, "{theme}: {}", levels.summary);
}

#[test]
fn balkan_fire_composed() {
    composed_builds("balkan_fire", themes::balkan_fire::layers::composed);
}

#[test]
fn baroque_storm_composed() {
    composed_builds("baroque_storm", themes::baroque_storm::layers::composed);
}

#[test]
fn big_top_composed() {
    composed_builds("big_top", themes::big_top::layers::composed);
}

#[test]
fn bossa_noir_composed() {
    composed_builds("bossa_noir", themes::bossa_noir::layers::composed);
}

#[test]
fn brass_house_composed() {
    composed_builds("brass_house", themes::brass_house::layers::composed);
}

#[test]
fn brass_trailer_composed() {
    composed_builds("brass_trailer", themes::brass_trailer::layers::composed);
}

#[test]
fn dark_breakbeat_composed() {
    composed_builds("dark_breakbeat", themes::dark_breakbeat::layers::composed);
}

#[test]
fn dark_disco_composed() {
    composed_builds("dark_disco", themes::dark_disco::layers::composed);
}

#[test]
fn dark_drumline_composed() {
    composed_builds("dark_drumline", themes::dark_drumline::layers::composed);
}

#[test]
fn dark_tango_composed() {
    composed_builds("dark_tango", themes::dark_tango::layers::composed);
}

#[test]
fn drift_phonk_composed() {
    composed_builds("drift_phonk", themes::drift_phonk::layers::composed);
}

#[test]
fn gothic_horror_composed() {
    composed_builds("gothic_horror", themes::gothic_horror::layers::composed);
}

#[test]
fn heist_composed() {
    composed_builds("heist", themes::heist::layers::composed);
}
