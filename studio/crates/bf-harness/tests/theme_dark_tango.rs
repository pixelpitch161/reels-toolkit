//! The dark tango against the Python kit: every gate of Template T (studio/docs/porting-a-fighter.md), with its one
//! voicing function (the bandoneon's right hand). Below, the tango stops (`stops.json`), which the bed alone never
//! makes (the kit's base Sound has no `stops`): the bed alone under two base Sounds whose `stops()` give fixed times,
//! then the times CELLO's own `stops()` gives on the config's run.

bf_harness::theme_suite!("dark_tango", [("voicing", bf_content::themes::dark_tango::voicing)]);

use bf_content::SoundBuild;
use bf_content::themes::dark_tango::DarkTangoClassic;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::Show;
use bf_harness::refs::{hexf, json, list, named_recording, report, text};
use bf_harness::sound::{compare, summarise, windows_against};
use bf_harness::theme::{bed_voices, config_fight_file};
use bf_sound::music::MusicCtx;
use bf_sound::traits::Sound;
use serde_json::Value;

const FOLDER: &str = "themes/dark_tango";

/// The kit's base `Sound` with a `stops()` that gives fixed times (the export's `s.stops = lambda: list(ts)`).
struct Stops {
    k: usize,
    times: Vec<f64>,
}

impl Sound for Stops {
    fn k(&self) -> usize {
        self.k
    }
    fn stops(&self, _: &MusicCtx) -> Vec<f64> {
        self.times.clone()
    }
}

fn times(v: &Value) -> Vec<f64> {
    list(v).iter().map(hexf).collect()
}

/// The reference config's recording and fight file (the harness's: with the config's LINES and ROLES, which the
/// bed's `theme()` reads).
fn config() -> (Value, Recording, FightFile) {
    let theme = json(FOLDER, "theme.json");
    let doc = json(FOLDER, "stops.json");
    let c = list(&theme["configs"])[0].clone();
    assert_eq!(c["config"], doc["config"]);
    let rec = named_recording(&c["recording"], None);
    let ff = config_fight_file(&c, &rec);
    (doc, rec, ff)
}

/// Each variant of `stops.json` against the bed alone with the two stop-giving Sounds: the times the bed keeps, the
/// theme line, then the soundtrack's summary (`stems`: the stem windows against the cache instead).
fn stop_variants(stems: bool) {
    let (doc, rec, ff) = config();
    let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("the show: {e}"));
    let mut failures = Vec::new();
    let mut n = 0;
    for v in list(&doc["variants"]) {
        let name = format!("stops {}", text(&v["name"]));
        let given: Vec<Vec<f64>> = list(&v["given"]).iter().map(times).collect();
        let mut build = SoundBuild::bed_alone(&show, &rec, &ff, bed_voices(&show, false))
            .unwrap_or_else(|e| panic!("the bed-alone build: {e}"));
        build.sounds = [
            Box::new(Stops {
                k: 0,
                times: given[0].clone(),
            }),
            Box::new(Stops {
                k: 1,
                times: given[1].clone(),
            }),
        ];
        // Bed.stop_times on the parts' stops, as the score hands them over
        let bed = DarkTangoClassic::new();
        build.ctx.set_fighter_stops(given.concat()).expect("the stops");
        let kept = bed.stop_times(&build.ctx).expect("stop_times");
        assert_eq!(kept, times(&v["kept"]), "{name}: the stops the bed keeps");
        let line = bed.theme(&build.ctx).map(str::to_string);
        assert_eq!(line.as_deref(), v["theme_line"].as_str(), "{name}: the theme line");
        build.ctx.set_fighter_stops(Vec::new()).expect("the stops");
        let s = summarise(&build);
        n += if stems {
            windows_against(&name, &s.st, &v["windows"], text(&v["stems_cache"]), &mut failures)
        } else {
            compare(&name, &s, v, &mut failures)
        };
    }
    assert!(
        list(&doc["variants"]).len() >= 2,
        "{FOLDER}: the fixed and the cello stops"
    );
    let (gate, what) = if stems {
        ("stems", "stop stem windows")
    } else {
        ("sound", "stop facts")
    };
    report(FOLDER, gate, "stops", what, failures, n);
}

/// The bed alone with stops on both sides: the 16-bit master, the buses, the master's RMS, the notes and the noise.
#[test]
fn bed_stops() {
    stop_variants(false);
}

/// The stem windows of the stop variants against the cache at -120 dBFS. Ignored.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache themes/<name>"]
fn bed_stops_stems() {
    stop_variants(true);
}

/// The `cello` variant's times are what the ported CELLO's own part asks for on this run.
#[test]
fn cello_stops_are_the_cellos() {
    let (doc, rec, ff) = config();
    let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("the show: {e}"));
    let build = SoundBuild::bed_alone(&show, &rec, &ff, bed_voices(&show, false)).expect("the build");
    let v = list(&doc["variants"])
        .iter()
        .find(|v| v["name"] == "cello")
        .expect("the cello variant");
    let cello = bf_content::fighters::sound("CELLO", 0, &ff).expect("CELLO's part is in this app");
    let want: Vec<Vec<f64>> = list(&v["given"]).iter().map(times).collect();
    assert!(!want[0].is_empty(), "the run has rope slams");
    assert_eq!(cello.stops(&build.ctx), want[0], "CELLO's stops on the config's run");
    println!("dark_tango: CELLO's {} stops equal the kit's", want[0].len());
}
