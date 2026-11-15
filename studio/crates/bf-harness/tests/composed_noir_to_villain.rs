//! Milestone 5, Task 5: the Composed arrangement of the themes from the noir jazz to the villain march. For each,
//! the layered bed builds and plays a whole song for the theme's own pair (its first preset's fighters, tempo, key
//! and chords) driven by the fight's story signal, and the master passes the levels check: a sound, never silence,
//! finite and under the ceiling, with every layer's bus in the stems. No parity gates: the arrangements are judged
//! by ear.

use bf_content::themes::{ThemeOptions, theme};
use bf_content::{ContentError, SoundBuild, roster, starter, themes_for};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_sound::music::layers::{Cue, CueKind, Layer};
use bf_sound::music::signal::{MomentKind, StorySignal};
use bf_sound::traits::Bed;

type Composed = fn(&ThemeOptions, Vec<Cue>, u64) -> Result<Box<dyn Bed>, ContentError>;

/// The theme's first preset on its own pair, with the lines and solos left to the parts.
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

/// The story signal's moments as the layer engine's cues.
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

fn plays(name: &str, composed: Composed, layers: Vec<Layer>) {
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
    let mut build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("{name}: {e}"));
    let signal = StorySignal::from(&build.ctx).expect("a story signal");
    let options = theme(name).unwrap().options_of(&ff).expect("the options");
    build.bed = composed(&options, cues(&signal), ff.music.music_seed).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mix = build.score().unwrap_or_else(|e| panic!("{name}: {e}"));
    let st = build.master(&mix);
    assert!(st.master.iter().flatten().all(|v| v.is_finite()), "{name}");
    let peak = st.master.iter().flatten().fold(0.0f64, |m, v| m.max(v.abs()));
    assert!(peak > 0.3 && peak <= 1.0, "{name}: peak {peak}");
    for l in &layers {
        assert!(st.stems.contains_key(l.bus), "{name}: no {} stem", l.bus);
    }
    // The listening clip: 20 s from FIGHT!, written when `BF_COMPOSED_CLIPS` names a folder.
    if let Ok(dir) = std::env::var("BF_COMPOSED_CLIPS") {
        let i0 = (show.grid.t_fight * bf_sound::SRF) as usize;
        let i1 = (i0 + 20 * bf_sound::SR).min(st.master.len());
        let path = std::path::Path::new(&dir).join(format!("{name}_composed.wav"));
        bf_sound::wav::write_wav16(&path, &st.master[i0..i1]).expect("the clip is written");
    }
}

macro_rules! composed {
    ($($test:ident: $theme:ident),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                plays(stringify!($theme), bf_content::themes::$theme::layers::composed, bf_content::themes::$theme::layers::layers());
            }
        )*
    };
}

composed! {
    the_noir_jazz_plays_composed: noir_jazz,
    the_orchestral_drill_plays_composed: orchestral_drill,
    the_prog_duel_plays_composed: prog_duel,
    the_sea_shanty_plays_composed: sea_shanty,
    the_second_line_plays_composed: second_line,
    the_ska_punk_plays_composed: ska_punk,
    the_spaghetti_western_plays_composed: spaghetti_western,
    the_stadium_rock_plays_composed: stadium_rock,
    the_villain_march_plays_composed: villain_march,
}
