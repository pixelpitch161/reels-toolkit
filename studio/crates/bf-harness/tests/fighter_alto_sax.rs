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

/// The kit makes the ALTO SAX's part before the score, and its `__init__` measures the voice's level
/// (`self.g0 = M.inst_gain(...)`: a note of the alto, drawn from the alto generator). So a build has drawn that note
/// once the parts are made, before any bed method. The kit's harness makes both real parts and only then swaps in
/// the silent stand-in (ruling M3-R49), so the note is drawn whichever side stands in. The kit, on this fight (seed
/// 4) with the ALTO SAX swapped out after `import kit.music`: `altosax60_lowpoly._RN.random()` is
/// `0x1.c0b3a96190b76p-2`.
#[test]
fn the_part_measures_its_level_when_it_is_made() {
    use bf_content::SoundBuild;
    use bf_core::show::Show;
    use bf_harness::fight::{doc, fight_file};
    use bf_harness::refs::{recording_of, text};

    let d = doc(NAME);
    let left = &d["left"];
    let rec = recording_of(left, Some(text(&left["hash"])));
    let mut ff = fight_file(&d, false);
    ff.seed = Some(rec.seed);
    let show = Show::new(&rec, &ff).expect("the show");
    let k = show.names.iter().position(|n| n == NAME).expect("the ALTO SAX fights");
    let next = |b: SoundBuild| b.ctx.noise().expect("the noise").1.alto.random();
    let kit = f64::from_bits(0x3FDC_0B3A_9619_0B76);
    assert_ne!(bf_sound::voice::PpNoise::start().1.alto.random(), kit);
    assert_eq!(next(SoundBuild::new(&show, &rec, &ff).expect("the build")), kit);
    for silent in [k, 1 - k] {
        let build = SoundBuild::with_stand_in(&show, &rec, &ff, silent).expect("the build");
        assert_eq!(next(build), kit, "side {silent} the stand-in");
    }
}
