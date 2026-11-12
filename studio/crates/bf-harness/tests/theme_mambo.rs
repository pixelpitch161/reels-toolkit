//! The `mambo` theme (milestone 3, Task 39): every gate of Template T, and the ALTO SAX fight under it.

use bf_content::SoundBuild;
use bf_core::show::Show;
use bf_harness::refs::{cached_recording, json, report, text};
use bf_harness::sound::{compare, summarise, windows_against};
use bf_harness::theme::config_fight_file;

bf_harness::theme_suite!("mambo", [("tones", bf_content::themes::mambo::tones)]);

/// The ALTO SAX's part and the bed's alto draw from one generator, and the kit measures the ALTO SAX's level when
/// its part is made, before the bed's first alto note (`alto_sax.json`): the soundtrack of the config with the
/// TRUMPET's seat taken by the ALTO SAX, both real parts, against the kit's (16-bit master, buses, notes, noise,
/// RMS, stem windows). Ignored: it reads the cached recording and stems.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache themes/mambo"]
fn alto_sax_fight() {
    let folder = "themes/mambo";
    let doc = json(folder, "alto_sax.json");
    let rec = cached_recording(text(&doc["recording"]));
    assert_eq!(
        rec.hash(),
        text(&doc["rec_hash"]),
        "{folder}: the ALTO SAX fight's recording"
    );
    let ff = config_fight_file(&doc["config"], &rec);
    assert_eq!(ff.fighters, ["ALTO SAX", "TROMBONE"]);
    let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("the show: {e}"));
    let build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("the build: {e}"));
    let s = summarise(&build);
    let mut failures = Vec::new();
    let mut n = compare("ALTO SAX vs TROMBONE", &s, &doc, &mut failures);
    n += windows_against(
        "ALTO SAX vs TROMBONE",
        &s.st,
        &doc["windows"],
        text(&doc["stems_cache"]),
        &mut failures,
    );
    report(folder, "sound", "alto_sax", "ALTO SAX fight facts", failures, n);
}
