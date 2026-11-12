//! The horror trap against the Python kit: every gate of Template T (studio/docs/porting-a-fighter.md). The mood has
//! no method that voices one chord, so the macro gets none; its music box's 2-bar lullaby (`Bed.box_motif(ch,
//! ch2)`) is checked below for every chord the harmony parses, from the same `voicings.json`.

bf_harness::theme_suite!("horror_trap", []);

use bf_core::grid::Grid;
use bf_core::harmony::{Harmony, chords_for, scale};

/// `voicings.json`'s `box_motif`: for each of the 12 roots x 13 qualities `ch` and its `next`, the kit's bar 1 then
/// bar 2 against [`bf_content::themes::horror_trap::box_motif`].
#[test]
fn box_motif() {
    let path = bf_testdata::reference("themes/horror_trap/voicings.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let doc: serde_json::Value = serde_json::from_str(&text).expect("voicings.json");
    let table = doc["box_motif"].as_object().expect("voicings.json has a box_motif table");
    assert_eq!(table.len(), 12 * 13, "a lullaby for every chord");
    let g = Grid::new(132.0);
    for (ch, row) in table {
        let next = row["next"].as_str().expect("its next chord").to_string();
        let both = vec![ch.clone(), next.clone()];
        let h = Harmony::new(
            &g,
            "E",
            scale("minor").unwrap(),
            chords_for(&both, &both, &[], g.n_bars),
            Some(both.clone()),
            2,
        )
        .unwrap_or_else(|e| panic!("{ch}, {next}: {e}"));
        let want: Vec<i32> = row["notes"]
            .as_array()
            .expect("its notes")
            .iter()
            .map(|x| x.as_i64().unwrap() as i32)
            .collect();
        let (bar1, bar2) = bf_content::themes::horror_trap::box_motif(&h, ch, &next)
            .unwrap_or_else(|e| panic!("box_motif({ch}, {next}): {e}"));
        let got: Vec<i32> = bar1.iter().chain(&bar2).copied().collect();
        assert_eq!(got, want, "box_motif({ch}, {next})");
    }
    println!("horror_trap: {} lullabies equal the kit's", table.len());
}
