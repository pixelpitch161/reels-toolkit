//! `Prepared`: a fight file made ready once, fighting seeds into candidates on the file's engine (milestone E6).

use bf_core::Engine;
use bf_core::fightfile::FightFile;
use bf_engine::{EngineError, Fought, Prepared, fight, prepare};

const EXAMPLE: &str = include_str!("../../../fights/flute_vs_trombone.toml");
const DUMMY: &str = include_str!("../../../fights/engine2/test_dummy.toml");

fn example() -> FightFile {
    FightFile::parse(EXAMPLE).expect("the example parses")
}

fn dummy() -> FightFile {
    FightFile::parse(DUMMY).expect("the dummy's file parses")
}

#[test]
fn prepared_is_send_and_sync() {
    fn both<T: Send + Sync>() {}
    both::<Prepared>();
}

#[test]
fn prepared_one_matches_bf_story_fight() {
    let ff = example();
    let p = prepare(&ff).expect("prepares");
    assert_eq!(p.engine(), Engine::One);
    let base = bf_engine::classic_setup(&ff, 0).expect("sets up");
    let sheet = bf_story::BeatSheet::series(base.fighters);
    assert_eq!(p.sheet(), &sheet);
    assert_eq!(p.colours(), base.fighters.map(|f| f.color));
    for seed in 1..=100 {
        let theirs = bf_story::fight(&bf_sim::Setup { seed, ..base.clone() }, &sheet).expect("fights");
        assert_eq!(p.candidate(seed, None).expect("fights"), theirs, "seed {seed}");
        let mut s = bf_sim::Setup { seed, ..base.clone() };
        s.rules.dmg_side = Some([1.2, 0.9]);
        let theirs = bf_story::fight(&s, &sheet).expect("fights");
        assert_eq!(
            p.candidate(seed, Some([1.2, 0.9])).expect("fights"),
            theirs,
            "seed {seed}, handicap"
        );
    }
}

#[test]
fn prepared_two_fights_the_dummy() {
    let ff = dummy();
    let p = prepare(&ff).expect("prepares");
    assert_eq!(p.engine(), Engine::Two);
    let colour = |k: usize| {
        bf_content::fighters2::def(&ff.fighters[k])
            .expect("a definition")
            .colour
    };
    assert_eq!(p.colours(), [colour(0), colour(1)]);
    let mut kos = 0;
    for seed in 1..=20 {
        let c = p.candidate(seed, None).expect("fights");
        assert_eq!(c.seed, seed);
        assert_eq!(c.story.seed, seed);
        if let Some(t) = c.story.t_ko {
            kos += 1;
            assert!((0.0..=61.0).contains(&t), "seed {seed}: K.O. at {t}");
            // The story's K.O. is on the video clock.
            assert_eq!(
                Some(t),
                bf_engine::ko_video_t(&ff, seed).expect("fights"),
                "seed {seed}"
            );
        } else {
            assert!(!c.verdict.fits, "seed {seed}: a fight with no K.O. never fits");
        }
    }
    assert!(kos > 0, "no dummy fight of 20 ends in a K.O.");
}

#[test]
fn prepared_two_handicap_moves_damage() {
    let ff = dummy();
    let p = prepare(&ff).expect("prepares");
    // With [1.0, 0.0] the right fighter deals no damage, so the left never loses HP and never loses.
    for seed in 1..=10 {
        let c = p.candidate(seed, Some([1.0, 0.0])).expect("fights");
        assert_ne!(c.story.winner, Some(1), "seed {seed}");
        let lead_left = c.story.lead.iter().all(|&(_, d)| d >= 0.0);
        assert!(lead_left, "seed {seed}: the left fighter lost HP: {:?}", c.story.lead);
        let ours = match fight(
            &FightFile {
                handicap: vec![1.0, 0.0],
                ..ff.clone()
            },
            seed,
            false,
        )
        .expect("fights")
        {
            Fought::Two(r) => r,
            Fought::One(_) => unreachable!(),
        };
        assert_eq!(ours.outcome.damage[1], 0.0, "seed {seed}");
        assert_eq!(c.story.winner, ours.outcome.winner, "seed {seed}");
    }
}

#[test]
fn fought_winner_and_ko_agree_with_candidate() {
    for ff in [example(), dummy()] {
        let p = prepare(&ff).expect("prepares");
        for seed in 1..=8 {
            let c = p.candidate(seed, None).expect("fights");
            let f = fight(&ff, seed, false).expect("fights");
            assert_eq!(f.winner(), c.story.winner, "{:?} seed {seed}", ff.engine);
            assert_eq!(f.t_ko_video(), c.story.t_ko, "{:?} seed {seed}", ff.engine);
            let recorded = fight(&ff, seed, true).expect("fights");
            assert_eq!(
                recorded.t_ko_video(),
                c.story.t_ko,
                "{:?} seed {seed}: recorded",
                ff.engine
            );
        }
    }
}

#[test]
fn prepare_refuses_an_engine2_file_without_definitions() {
    let ff = FightFile {
        fighters: vec!["TEST DUMMY".into(), "FLUTE".into()],
        ..dummy()
    };
    assert_eq!(prepare(&ff).err(), Some(EngineError::NotYet(Engine::Two)));
}

#[test]
fn judge_of_a_recorded_fight_is_the_candidate() {
    for ff in [example(), dummy()] {
        let p = prepare(&ff).expect("prepares");
        for seed in [1, 5, 26] {
            let recorded = fight(&ff, seed, true).expect("fights");
            assert_eq!(
                p.judge(&recorded).expect("the same engine"),
                p.candidate(seed, None).expect("fights"),
                "{:?} seed {seed}",
                ff.engine
            );
        }
    }
    let other = fight(&example(), 1, false).expect("fights");
    assert!(prepare(&dummy()).unwrap().judge(&other).is_err());
}
