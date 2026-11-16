//! The Studio's own themes (2026-11-16): for each, both arrangements build and play a whole song for the theme's
//! own pair (its preset's fighters, tempo, key and chords), the Composed one driven by the fight's story signal, and
//! the master passes the levels check: a sound, never silence, finite and under the ceiling, with every layer's bus
//! in the stems. No parity gates: judged by ear.

use bf_content::themes::{CLASSIC, COMPOSED, theme};
use bf_content::{SoundBuild, roster, starter, themes_for};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_sound::music::layers::{Cue, CueKind, Layer};
use bf_sound::music::signal::{MomentKind, StorySignal};

fn fight_file(name: &str) -> FightFile {
    let p = &theme(name).expect("the theme").card.presets[0];
    let [a, b] = p.fighters;
    assert!(themes_for(a, b).contains(&name), "{name} is offered for {a} v {b}");
    let mut ff = starter(a, b).expect("a starter");
    ff.music.theme = name.to_string();
    ff.music.theme_options.clear();
    ff.music.mix.clear();
    ff.tempo_bpm = p.bpm;
    ff.music.key = p.key.to_string();
    ff.music.scale = p.scale.to_string();
    ff.music.chords_intro = p.intro.iter().map(|c| c.to_string()).collect();
    ff.music.chord_loop = p.chord_loop.iter().map(|c| c.to_string()).collect();
    ff.music.chords_tail = p.tail.iter().map(|c| c.to_string()).collect();
    ff.music.lines.clear();
    ff.music.solos.clear();
    ff
}

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

fn plays(name: &str, layers: Vec<Layer>) {
    let ff = fight_file(name);
    bf_content::validate(&ff).unwrap_or_else(|e| panic!("{name}: {e}"));
    let rec = (1..=12u64)
        .map(|seed| {
            let setup = bf_sim::setup_from(&ff, roster(), seed).expect("a setup");
            bf_sim::run(&setup, true).expect("a fight")
        })
        .find(|rec| rec.t_ko.is_some())
        .unwrap_or_else(|| panic!("{name}: no K.O. in 12 seeds"));
    let show = Show::new(&rec, &ff).expect("a show");
    let t = theme(name).unwrap();
    for arrangement in [CLASSIC, COMPOSED] {
        let mut build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("{name}: {e}"));
        let signal = StorySignal::from(&build.ctx).expect("a story signal");
        let options = t.options_of(&ff).expect("the options");
        build.bed = t
            .bed_with(arrangement, &options, cues(&signal), ff.music.music_seed)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let mix = build.score().unwrap_or_else(|e| panic!("{name} {arrangement}: {e}"));
        let st = build.master(&mix);
        assert!(st.master.iter().flatten().all(|v| v.is_finite()), "{name}");
        let peak = st.master.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(peak > 0.3 && peak <= 1.0, "{name} {arrangement}: peak {peak}");
        for l in &layers {
            assert!(st.stems.contains_key(l.bus), "{name} {arrangement}: no {} stem", l.bus);
        }
        if let Ok(dir) = std::env::var("BF_COMPOSED_CLIPS") {
            let mut levels: Vec<String> = Vec::new();
            for (bus, x) in &st.stems {
                let rms = (x.iter().flatten().map(|v| v * v).sum::<f64>() / (2 * x.len().max(1)) as f64).sqrt();
                levels.push(format!("{bus} {rms:.3}"));
            }
            eprintln!("LEVELS {name} {arrangement} peak {peak:.2}: {}", levels.join(", "));
            let i0 = (show.grid.t_fight * bf_sound::SRF) as usize;
            let i1 = (i0 + 20 * bf_sound::SR).min(st.master.len());
            let path = std::path::Path::new(&dir).join(format!("{name}_{arrangement}.wav"));
            bf_sound::wav::write_wav16(&path, &st.master[i0..i1]).expect("the clip is written");
        }
    }
}

macro_rules! studio {
    ($($test:ident: $theme:ident),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                plays(stringify!($theme), bf_content::themes::$theme::layers());
            }
        )*
    };
}

studio! {
    the_surf_rock_plays: surf_rock,
    the_pep_band_punk_plays: pep_band_punk,
    the_drumline_metal_plays: drumline_metal,
    the_celtic_reel_plays: celtic_reel,
    the_mariachi_plays: mariachi,
    the_highlife_plays: highlife,
    the_samba_bateria_plays: samba_bateria,
    the_heroic_fanfare_plays: heroic_fanfare,
    the_liquid_dnb_plays: liquid_dnb,
}
