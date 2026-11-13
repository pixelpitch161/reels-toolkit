//! The proving harness (milestone 3, Task 6): every gate a port of the Python kit must pass, written once.
//!
//! A port's whole test is one file of this crate's `tests/` folder that hands its name to a suite macro:
//!
//! ```ignore
//! // tests/fighter_tuba.rs
//! bf_harness::fighter_suite!("TUBA");
//! // tests/theme_heist.rs
//! bf_harness::theme_suite!("heist", [("voice", bf_content::themes::heist::voice)]);
//! // tests/episode_oboe_vs_bassoon.rs
//! bf_harness::episode_suite!("oboe_vs_bassoon");
//! ```
//!
//! The macro writes the `#[test]` functions (their names are the gates': `coverage`, `look_frames`, `sound`, ...);
//! the suites here read the port's reference folder, `studio/reference/<area>/<name>/`, which its export wrote
//! (`python -m tools.studio_export fighter_<slug>` and `cache <area>/<name>`). The gates that read the pinned cache
//! are `#[ignore]`d and run by name (`-- --include-ignored`); the statistics run in release builds only.
//!
//! **No gate passes by doing nothing.** A missing reference folder or file fails with the export to run; a missing,
//! unpinned or changed cache file with the `cache` command ([`bf_testdata::cache_file`]); a socket stub reached in
//! any fight ([`fight::traced`]) names the stub; a hull fallback in a look's frame stops the task (ruling M3-R4); an
//! `exceptions.toml` entry without a controller ruling fails ([`refs::exceptions`]); a count of frames, notes or
//! rows below what the export wrote fails.
//!
//! The comparisons are milestone 1's: [`bf_sim::parity::prefix_window`] for the fights, the hashes and draw logs of
//! `flute_look.rs` for the looks, the RMS profile and the -120 dBFS windows of `stems.rs`, the 16-bit master of
//! `master_matches`, the voice bars of `voices.rs`, the bars of `checks_parity.rs` and of `stats_parity.rs`.
//!
//! Two minimums are stated as rules rather than counts. The fits ([`fight::fits_present`], ruling M3-R32): where
//! the kit's statistics have fits the port must have at least one, and wherever either has fits the two counts must
//! agree under a two-proportion test, |z| within [`fight::FIT_Z`] = 3, on top of the fit-rate bar. A part's sound
//! ([`sound::part_is_compared`]): the kit's soundtrack has first-time notes of the part, or (a part that plays only
//! through its own synth, the kit's count 0) its own bus is a stem sounding above -60 dBFS in at least 100 blocks.

pub mod checks;
pub mod episode;
pub mod fight;
pub mod look;
pub mod refs;
pub mod scan;
pub mod sound;
pub mod theme;
pub mod voice;

/// The engine's test entries a suite needs: the traced run (`bf_sim::run_traced`) and the margin of a knife-edge
/// clash (`bf_sim::parity::clash_margin`). They exist only in builds with `bf-sim`'s `testing` feature, which this
/// crate's tests turn on; the suite macros pass them in.
#[derive(Clone, Copy)]
pub struct Sim {
    pub run_traced: fight::Traced,
    pub clash_margin: fn(&bf_sim::Setup, f64) -> Option<f64>,
}

/// The [`Sim`] of this build (in a test file of this crate).
#[macro_export]
macro_rules! sim {
    () => {
        $crate::Sim {
            run_traced: ::bf_sim::run_traced,
            clash_margin: ::bf_sim::parity::clash_margin,
        }
    };
}

/// Every gate of Template F for the fighter `$name` (its roster name), as `#[test]` functions.
#[macro_export]
macro_rules! fighter_suite {
    ($name:literal) => {
        const NAME: &str = $name;

        // ---------------------------------------------------------------- the fight (always run, fast)
        #[test]
        fn reference_folder() {
            $crate::refs::fighter_folder_is_whole(NAME);
        }
        #[test]
        fn exceptions() {
            $crate::refs::exceptions_are_ruled(&$crate::refs::fighter_folder(NAME));
        }
        #[test]
        fn coverage() {
            $crate::fight::coverage(&$crate::sim!(), NAME);
        }
        #[test]
        fn replay() {
            $crate::fight::replay(&$crate::sim!(), NAME);
        }
        #[test]
        fn record_flag() {
            $crate::fight::record_flag(&$crate::sim!(), NAME);
        }
        #[test]
        fn left_fight() {
            $crate::fight::left_fight(&$crate::sim!(), NAME);
        }
        // ---------------------------------------------------------------- the fight against the cache
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
        fn prefix() {
            $crate::fight::prefix(&$crate::sim!(), NAME);
        }
        #[test]
        #[cfg_attr(debug_assertions, ignore = "2,000 fights: run with --release")]
        fn stats() {
            $crate::fight::stats(NAME, false);
        }
        #[test]
        #[cfg_attr(debug_assertions, ignore = "20,000 fights: run with --release")]
        fn stats_wide() {
            $crate::fight::stats(NAME, true);
        }
        // ---------------------------------------------------------------- the picture
        #[test]
        fn look_frames() {
            $crate::look::look_frames(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
        fn look_frames_cached() {
            $crate::look::look_frames_cached(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
        fn hull_points() {
            $crate::look::hull_points(NAME);
        }
        #[test]
        #[ignore = "needs local stage dumps: python -m tools.studio_export fighter_stages <slug> [left | right] <frame>..."]
        fn stages() {
            $crate::look::stages(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
        fn scenarios_draw() {
            $crate::look::scenarios_draw(NAME, 30);
        }
        #[test]
        fn title() {
            $crate::look::title(NAME);
        }
        #[test]
        fn sprites() {
            $crate::look::sprites(NAME);
        }
        #[test]
        fn setup_as() {
            $crate::look::setup_as(NAME);
        }
        #[test]
        fn looks_take_numbers_from_setup_as() {
            $crate::scan::looks_take_numbers_from_setup_as();
        }
        // ---------------------------------------------------------------- the sound
        #[test]
        fn voice() {
            $crate::voice::voice(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
        fn voice_notes() {
            $crate::voice::voice_notes(NAME);
        }
        #[test]
        fn sound() {
            $crate::sound::sound(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/<slug>"]
        fn sound_stems() {
            $crate::sound::sound_stems(NAME);
        }
        #[test]
        fn bed_calls() {
            $crate::sound::bed_calls(NAME);
        }
        // ---------------------------------------------------------------- the checks
        #[test]
        #[ignore = "slow (1 to 2 minutes: the soundtrack and about 680 frames); run by name in every review"]
        fn checks() {
            $crate::checks::fighter_checks(NAME);
        }
    };
}

/// Every gate of Template T for the theme `$name` (the kit's mood), as `#[test]` functions. The voicings are the
/// theme's own test (its functions are its own): `bf_harness::theme::voicings(NAME, &[("voice", f), ...])`.
#[macro_export]
macro_rules! theme_suite {
    ($name:literal, [$(($method:literal, $f:path)),* $(,)?]) => {
        const NAME: &str = $name;

        #[test]
        fn voicings() {
            $crate::theme::voicings(NAME, &[$(($method, $f)),*]);
        }
        #[test]
        fn bed_alone_stand_ins() {
            $crate::theme::bed_alone_stand_ins(NAME);
        }

        #[test]
        fn reference_folder() {
            $crate::refs::theme_folder_is_whole(NAME);
        }
        #[test]
        fn exceptions() {
            $crate::refs::exceptions_are_ruled(&format!("themes/{NAME}"));
        }
        #[test]
        fn bed_alone() {
            $crate::theme::bed_alone(NAME);
        }
        #[test]
        fn hooks() {
            $crate::theme::hooks(NAME);
        }
        #[test]
        fn card() {
            $crate::theme::card(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache themes/<name>"]
        fn bed_variants() {
            $crate::theme::bed_variants(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache themes/<name>"]
        fn bed_stems() {
            $crate::theme::bed_stems(NAME);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache themes/<name>"]
        fn hook_vectors() {
            $crate::theme::hook_vectors(NAME);
        }
    };
}

/// Every gate of Template P for the pair gate `$pair` (its folder under `studio/reference/episodes/`, and its fight
/// file `studio/fights/<pair>.toml`), as `#[test]` functions.
#[macro_export]
macro_rules! episode_suite {
    ($pair:literal) => {
        const PAIR: &str = $pair;

        #[test]
        fn reference_folder() {
            $crate::refs::episode_folder_is_whole(PAIR);
        }
        #[test]
        fn exceptions() {
            $crate::refs::exceptions_are_ruled(&format!("episodes/{PAIR}"));
        }
        #[test]
        fn fight_file_is_the_config() {
            $crate::episode::fight_file_is_the_config(PAIR);
        }
        #[test]
        fn recording() {
            $crate::episode::recording(&$crate::sim!(), PAIR);
        }
        #[test]
        fn golden_frames() {
            $crate::episode::golden_frames(PAIR);
        }
        #[test]
        fn title_card() {
            $crate::episode::title_card(PAIR);
        }
        #[test]
        fn soundtrack() {
            $crate::episode::soundtrack(PAIR);
        }
        #[test]
        #[ignore = "slow (1 to 2 minutes: the soundtrack and about 680 frames); run by name in every review"]
        fn checks() {
            $crate::checks::episode_checks(PAIR);
        }
        #[test]
        fn pair_links() {
            $crate::episode::pair_links(&$crate::sim!(), PAIR);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache episodes/<pair>"]
        fn stems() {
            $crate::episode::stems(PAIR);
        }
        #[test]
        #[ignore = "reads the pinned cache: python -m tools.studio_export cache episodes/<pair>"]
        fn prefix() {
            $crate::episode::prefix(&$crate::sim!(), PAIR);
        }
        #[test]
        #[cfg_attr(debug_assertions, ignore = "4,000 fights: run with --release")]
        fn statistics() {
            $crate::episode::statistics(PAIR);
        }
        #[test]
        #[ignore = "long: a search and a whole render (cargo test --release ... native_render -- --ignored)"]
        fn native_render() {
            $crate::episode::native_render(PAIR);
        }
    };
}

/// Mutations of the gates that read the cache or need a whole run, made on their comparison itself: each changes
/// one value the kit gave and the gate's comparison must refuse it.
#[cfg(test)]
mod mutations {
    use bf_core::event::Value;
    use bf_core::recording::Recording;

    fn sim() -> crate::Sim {
        crate::Sim {
            run_traced: bf_sim::run_traced,
            clash_margin: bf_sim::parity::clash_margin,
        }
    }

    fn seed1() -> Recording {
        Recording::read(&crate::refs::reference("fights/flute_vs_trombone/seed1.bfrec")).expect("seed1")
    }

    /// Ruling M3-R36: a left recording in the pinned cache is read from there, and a missing one fails with the
    /// command that makes it (a gate never passes by comparing nothing).
    #[test]
    #[should_panic(expected = "python -m tools.studio_export")]
    fn a_left_recording_missing_from_the_cache_fails_with_the_command() {
        crate::refs::recording_of(&serde_json::json!({"cache": "fighters/nobody/left.bfrec"}), None);
    }

    #[test]
    #[should_panic(expected = "exactly one of `file` and `cache`")]
    fn a_recording_entry_names_a_file_or_the_cache() {
        crate::refs::recording_of(
            &serde_json::json!({"file": "fights/flute_vs_trombone/seed1.bfrec", "cache": "fighters/x/left.bfrec"}),
            None,
        );
    }

    #[test]
    fn a_committed_recording_entry_reads_as_before() {
        let entry = serde_json::json!({"file": "fights/flute_vs_trombone/seed1.bfrec"});
        let rec = crate::refs::recording_of(&entry, Some(&seed1().hash()));
        assert_eq!(rec.seed, 1);
    }

    #[test]
    #[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/trombone"]
    fn a_cached_recording_entry_reads_and_checks_its_hash() {
        // TROMBONE's right fight is milestone 1's seed1.bfrec byte for byte (fight.json's same_as)
        let entry = serde_json::json!({"cache": "fighters/trombone/right.bfrec"});
        let rec = crate::refs::recording_of(&entry, Some(&seed1().hash()));
        assert_eq!(rec.seed, 1);
        let wrong = std::panic::catch_unwind(|| crate::refs::recording_of(&entry, Some("0000000000000000")));
        assert!(wrong.is_err(), "a cached recording with another hash was taken");
    }

    #[test]
    fn prefix_agrees_with_the_kits_own_fight() {
        let py = seed1();
        let setup = crate::fight::setup_like(&py);
        let rs = crate::fight::traced(&sim(), &setup, true, "seed1");
        assert!(crate::fight::window_ok(&sim(), "FLUTE", "seed1", &rs, &py, &setup) >= 10.0);
    }

    #[test]
    #[should_panic(expected = "before the floor")]
    fn prefix_refuses_one_event_moved() {
        let mut py = seed1();
        let i = py
            .events
            .iter()
            .position(|e| e.kind == "clash" && e.t > 6.0)
            .expect("an early clash");
        py.events[i].t += 1.0 / 240.0;
        let setup = crate::fight::setup_like(&py);
        let rs = crate::fight::traced(&sim(), &setup, true, "seed1");
        crate::fight::window_ok(&sim(), "FLUTE", "seed1 with one event moved", &rs, &py, &setup);
    }

    #[test]
    #[should_panic(expected = "rules.t_fight")]
    fn prefix_refuses_a_recording_without_t_fight() {
        let mut py = seed1();
        let setup = crate::fight::setup_like(&py);
        let rs = crate::fight::traced(&sim(), &setup, true, "seed1");
        py.rules.insert("t_fight".into(), Value::Null);
        crate::fight::window_ok(&sim(), "FLUTE", "seed1", &rs, &py, &setup);
    }

    fn stats() -> bf_story::Stats {
        bf_story::Stats {
            n: 2000,
            decided: 1850,
            win_share: 0.48,
            ko_median: 49.9,
            ko_p10: 42.6,
            ko_p90: 57.0,
            no_ko_share: 0.074,
            lead_changes_mean: 0.58,
            fit_rate: 0.0085,
            ko_hist: Default::default(),
        }
    }

    #[test]
    #[should_panic(expected = "win share")]
    fn statistics_refuse_a_pooled_figure_moved() {
        let mut rs = stats();
        rs.win_share += 0.031;
        crate::fight::assert_the_kits("mutation", &rs, &stats());
    }

    #[test]
    #[should_panic(expected = "no fits at all")]
    fn statistics_refuse_no_fits_where_the_kit_has_them() {
        let mut rs = stats();
        rs.n = 20000;
        rs.fit_rate = 0.0;
        let mut py = stats();
        py.fit_rate = 0.0025;
        crate::fight::assert_the_kits("mutation", &rs, &py);
    }

    #[test]
    fn a_part_with_no_kit_note_is_compared_by_its_own_bus() {
        use crate::sound::part_is_compared;
        let doc = crate::refs::json("fighters/flute", "sound.json");
        let left = &doc["left"];
        assert!(crate::refs::index(&left["notes"]["count"]) > 0);
        assert!(part_is_compared("FLUTE", left).is_ok());
        // the kit's count 0 (CLARINET: every note through its own reed cache): its bus sounds, so it passes
        let mut none = left.clone();
        none["notes"]["count"] = 0.into();
        assert!(part_is_compared("FLUTE", &none).is_ok());
        // ... but not when the bus is silent, missing from the stems or has no profile: nothing would be compared
        let mut silent = none.clone();
        let blocks = silent["rms"]["flute"].as_array().expect("flute rms").len();
        silent["rms"]["flute"] = serde_json::json!(vec![-120.0; blocks]);
        assert!(part_is_compared("FLUTE", &silent).unwrap_err().contains("fewer than"));
        let mut gone = none.clone();
        gone["stems"] = serde_json::json!(["master", "drums"]);
        assert!(
            part_is_compared("FLUTE", &gone)
                .unwrap_err()
                .contains("not one of its stems")
        );
        let mut unprofiled = none.clone();
        unprofiled["rms"].as_object_mut().expect("rms").remove("flute");
        assert!(part_is_compared("FLUTE", &unprofiled).is_err());
    }

    /// Ruling M3-R32 on the fit counts alone (n fights, fits).
    fn fits(n: usize, fits: usize) -> bf_story::Stats {
        bf_story::Stats {
            n,
            fit_rate: fits as f64 / n as f64,
            ..stats()
        }
    }

    #[test]
    fn rare_fits_are_compared_as_samples_of_one_rate() {
        use crate::fight::{fit_z, fits_present};
        // CLARINET (task 10): the kit's 5 in 2,000 against the port's 2 in 2,000 and 25 in 20,000: noise, passes
        assert!(fits_present(&fits(2000, 2), &fits(2000, 5)).is_ok());
        assert!(fits_present(&fits(20000, 25), &fits(2000, 5)).is_ok());
        assert!((fit_z(&fits(20000, 25), &fits(2000, 5)).abs() - 1.44).abs() < 0.01);
        // (milestone 1's fixed threshold, above 0.002 where the kit is, refused that: 25 / 20,000 = 0.00125)
        // no fits at all where the kit has them always fails, however rare
        assert!(fits_present(&fits(20000, 0), &fits(2000, 1)).is_err());
        assert!(fits_present(&fits(2000, 0), &fits(2000, 5)).is_err());
        // a rate several standard errors away fails, either way round
        assert!(fits_present(&fits(20000, 10), &fits(2000, 9)).is_err());
        assert!(fits_present(&fits(20000, 200), &fits(2000, 4)).is_err());
        assert!(fits_present(&fits(20000, 120), &fits(2000, 0)).is_err());
        // equal rates and no fits on either side pass
        assert!(fits_present(&fits(2000, 17), &fits(2000, 17)).is_ok());
        assert!(fits_present(&fits(20000, 0), &fits(2000, 0)).is_ok());
    }

    #[test]
    fn a_stem_window_off_by_1e_5_fails_the_bar() {
        let want: Vec<f32> = (0..96).map(|i| (i as f32 * 0.01).sin()).collect();
        let mut got: Vec<[f64; 2]> = want.chunks(2).map(|c| [f64::from(c[0]), f64::from(c[1])]).collect();
        assert_eq!(crate::sound::window_db(&got, &want), f64::NEG_INFINITY);
        got[7][1] += 1e-5;
        assert!(crate::sound::window_db(&got, &want) >= crate::sound::DB);
    }

    #[test]
    fn a_voice_note_off_by_2e_6_fails_the_bar() {
        let want: Vec<f64> = (0..480).map(|i| (f64::from(i) * 0.05).sin() * 0.3).collect();
        let mut got = want.clone();
        assert_eq!(crate::voice::note_gap(&got, &want), 0.0);
        got[100] += 2e-6;
        assert!(crate::voice::note_gap(&got, &want) > crate::voice::BAR);
        got[100] = f64::NAN;
        assert!(crate::voice::note_gap(&got, &want) > crate::voice::BAR);
    }

    #[test]
    fn a_neutral_handicap_recorded_or_not_is_the_same_rules() {
        let like = crate::fight::setup_like(&seed1());
        let mut none = like.rules.clone();
        none.dmg_side = None;
        let mut neutral = none.clone();
        neutral.dmg_side = Some([1.0, 1.0]);
        let text = crate::fight::rules_text;
        assert_ne!(
            format!("{none:?}"),
            format!("{neutral:?}"),
            "what the gates compared before"
        );
        assert_eq!(text(&none), text(&neutral));
        for other in [[1.0, 2.0], [2.0, 1.0], [0.4, 0.4]] {
            let mut r = none.clone();
            r.dmg_side = Some(other);
            assert_ne!(text(&none), text(&r), "{other:?}");
        }
        let mut faster = neutral.clone();
        faster.bpm += 1.0;
        assert_ne!(text(&neutral), text(&faster), "another rule must still differ");
    }

    #[test]
    fn a_hull_point_off_by_2e_12_fails_the_bar() {
        assert!(crate::look::point_gap((120.5, 33.25), (120.5, 33.25)) <= 1e-12);
        assert!(crate::look::point_gap((120.5, 33.25 + 2e-12), (120.5, 33.25)) > 1e-12);
        assert!(crate::look::point_gap((f64::NAN, 0.0), (1.0, 0.0)) > 1e-12);
    }
}
