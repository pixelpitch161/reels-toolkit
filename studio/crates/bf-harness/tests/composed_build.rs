//! Milestone 5, Task 7: the Composed arrangement wired into the build. A classic fight file builds as it always
//! did (nothing composed, the classic bed); a composed build of FLUTE vs TROMBONE passes the levels check and every
//! composed note on the beat is in key; a pinned fighter keeps its lines and solo as written.

use bf_content::SoundBuild;
use bf_core::fightfile::FightFile;
use bf_core::harmony::parse_chord;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_export::checks;
use bf_sound::music::signal::StorySignal;

fn episode() -> (Recording, FightFile) {
    let rec = Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec")).expect("the recording");
    let ff = FightFile::parse(include_str!("../../../fights/flute_vs_trombone.toml")).expect("the episode");
    (rec, ff)
}

#[test]
fn a_classic_fight_composes_nothing() {
    let (rec, ff) = episode();
    assert_eq!(ff.music.arrangement, "classic");
    let show = Show::new(&rec, &ff).unwrap();
    let build = SoundBuild::new(&show, &rec, &ff).unwrap();
    assert!(build.ctx.composed.is_empty());
    let classic = bf_content::themes::theme("big_top")
        .unwrap()
        .bed("classic", &Default::default())
        .unwrap();
    assert_eq!(build.bed.buses(), classic.buses());
}

#[test]
fn a_composed_flute_vs_trombone_is_even_and_in_key() {
    let (rec, mut ff) = episode();
    ff.music.arrangement = "composed".into();
    bf_content::validate(&ff).unwrap();
    let show = Show::new(&rec, &ff).unwrap();
    let build = SoundBuild::new(&show, &rec, &ff).unwrap();
    assert_eq!(build.ctx.composed.len(), 2);
    let h = &build.ctx.harmony;
    let scale: Vec<i32> = h.scale.iter().map(|s| (h.key_pc + s).rem_euclid(12)).collect();
    for (name, part) in &build.ctx.composed {
        assert_eq!(part.song.len(), show.grid.n_bars * part.per_bar, "{name}");
        for (i, m) in part.song.iter().enumerate() {
            // on the beat: in key (a passing or approach note between the beats may be chromatic)
            let Some(m) = m.filter(|_| i % 2 == 0) else { continue };
            let chord = parse_chord(&h.chords[i / part.per_bar]).unwrap().pcs();
            let pc = m.rem_euclid(12);
            assert!(
                scale.contains(&pc) || chord.contains(&pc),
                "{name}: note {m} at slot {i}"
            );
        }
        // the part plays the whole-song line, not the fight file's loop line
        let t = show.grid.bar_t(show.grid.fight_bar as f64 + 3.0);
        assert_eq!(build.ctx.line_at(name, t).unwrap(), part.note_at(t, show.grid.bar));
        assert_eq!(build.ctx.solo_entries(name).unwrap(), part.solo);
    }
    // every story moment, and so every band hit, sits on a sixteenth
    let sig = StorySignal::from(&build.ctx).unwrap();
    assert!(sig.moments.iter().all(|m| m.t == m.step as f64 * show.grid.s16));
    let st = build.master(&build.score().unwrap());
    assert!(st.master.as_flattened().iter().all(|v| v.is_finite() && v.abs() <= 1.0));
    let levels = checks::levels(&checks::as_wav16(&st.master));
    assert!(levels.pass, "{}", levels.summary);
    // the classic soundtrack of the same fight is another one
    let (_, classic_ff) = episode();
    let cb = SoundBuild::new(&show, &rec, &classic_ff).unwrap();
    assert_ne!(cb.master(&cb.score().unwrap()).master, st.master);
}

#[test]
fn a_pinned_fighter_keeps_its_lines() {
    let (rec, mut ff) = episode();
    ff.music.arrangement = "composed".into();
    ff.music.pinned = vec!["FLUTE".into()];
    bf_content::validate(&ff).unwrap();
    let show = Show::new(&rec, &ff).unwrap();
    let build = SoundBuild::new(&show, &rec, &ff).unwrap();
    assert!(!build.ctx.composed.contains_key("FLUTE") && build.ctx.composed.contains_key("TROMBONE"));
    let (_, classic) = episode();
    let cshow = Show::new(&rec, &classic).unwrap();
    let cb = SoundBuild::new(&cshow, &rec, &classic).unwrap();
    for i in 0..200 {
        let t = i as f64 * 0.3;
        assert_eq!(build.ctx.line_at("FLUTE", t), cb.ctx.line_at("FLUTE", t), "at {t}");
    }
    assert_eq!(build.ctx.solo_entries("FLUTE").unwrap(), ff.music.solos["FLUTE"]);
    // a pinned name that is not a fighter is refused
    ff.music.pinned = vec!["PIANO".into()];
    assert_eq!(
        bf_content::validate(&ff).unwrap_err().to_string(),
        r#"music.pinned: "PIANO" is not one of the fighters"#
    );
}
