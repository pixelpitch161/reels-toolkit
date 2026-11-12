//! The proving harness (milestone 3, Task 6): every gate a port of the Python kit must pass, written once.
//!
//! A port's whole test is one file of this crate's `tests/` folder that hands its name to a suite macro:
//!
//! ```ignore
//! // tests/fighter_tuba.rs
//! bf_harness::fighter_suite!("TUBA");
//! // tests/theme_heist.rs
//! bf_harness::theme_suite!("heist");
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
    ($name:literal) => {
        const NAME: &str = $name;

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
