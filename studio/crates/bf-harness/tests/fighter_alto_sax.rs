//! ALTO SAX against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md).

bf_harness::fighter_suite!("ALTO SAX");

/// The first sweep of the committed left fight is the kit's to the last bit: the `spot` event (the line through
/// the enemy's predicted spot, turned by the jitter draw and held inside the ropes), the first `hit` of kind
/// `burn` and its `burn` event (the light's eased place on its line at that 16th): the spotlight's own arithmetic,
/// checked here event by event. The whole fight after it (the knocks included, through the engine's fused `norm`)
/// is the kit's bit for bit too: the `prefix` gate finds every reference fight equal to its end.
#[test]
fn the_first_sweep_is_the_kits_to_the_last_bit() {
    use bf_harness::fight::{doc, fight_file, roster, traced};
    use bf_harness::refs::{recording_of, text};

    let d = doc(NAME);
    let left = &d["left"];
    let py = recording_of(left, Some(text(&left["hash"])));
    let ff = fight_file(&d, false);
    let setup = bf_sim::setup_from(&ff, roster(), py.seed).expect("the fight file sets up");
    let rs = traced(&bf_harness::sim!(), &setup, true, "ALTO SAX left");
    let first_burn = py
        .events
        .iter()
        .position(|e| e.kind == "burn")
        .expect("the left fight has a burn");
    let kinds: Vec<&str> = py.events[..=first_burn].iter().map(|e| e.kind.as_str()).collect();
    assert!(kinds.contains(&"spot") && kinds.contains(&"hit"), "{kinds:?}");
    // `Event`'s equality is bit for bit
    assert_eq!(rs.events[..=first_burn], py.events[..=first_burn]);
}
