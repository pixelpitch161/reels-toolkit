//! TRUMPET against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), and its part with a
//! line (the plan's Task 8: "test both with and without it").

bf_harness::fighter_suite!("TRUMPET");

/// The `left` soundtrack with a line for the TRUMPET (the line of `sound_line.json`, the export's own extra:
/// `tools/studio_export/fighter_trumpet.py`) and the base `Sound` for the other side; the kit's summary of it.
fn line_soundtrack() -> (
    bf_harness::sound::Summary,
    serde_json::Value,
    bf_core::recording::Recording,
) {
    use bf_harness::refs::{committed_recording, fighter_folder, json, list, text};

    let doc = json(&fighter_folder(NAME), "sound_line.json");
    assert_eq!(text(&doc["fighter"]), NAME);
    let fd = bf_harness::fight::doc(NAME);
    let rec = committed_recording(text(&fd["left"]["file"]), Some(text(&fd["left"]["hash"])));
    let mut ff = bf_harness::fight::fight_file(&fd, false);
    ff.seed = Some(rec.seed);
    assert!(
        !ff.music.lines.contains_key(NAME),
        "the base fight file has a line for {NAME}"
    );
    let line: Vec<Option<i32>> = list(&doc["line"])
        .iter()
        .map(|n| Some(i32::try_from(n.as_i64().expect("a note")).expect("a MIDI note")))
        .collect();
    assert_eq!(line.len(), 64, "8 notes a bar over the 8-bar loop");
    ff.music.lines.insert(NAME.to_string(), line);
    let show = bf_core::show::Show::new(&rec, &ff).expect("the show");
    let mut build = bf_content::SoundBuild::new(&show, &rec, &ff).expect("the sound build");
    assert_eq!(show.names[0], NAME);
    build.sounds[1] = Box::new(bf_sound::traits::BaseSound { k: 1 });
    (bf_harness::sound::summarise(&build), doc["left"].clone(), rec)
}

/// With a line in the fight file (`music.lines.TRUMPET`) the punches play that line instead of the valve
/// fingerings: the kit's soundtrack of the `left` fight with the base config's FLUTE line an octave down as the
/// TRUMPET's. The 16-bit master is the kit's sample for sample (its SHA-256), with the buses, the stems, the
/// first-time notes in the kit's order and the noise stream's end. The `sound` gate above is the part without a
/// line. (The RMS profiles are in the cache: `the_line_stems`.)
#[test]
fn the_line_takes_the_punches() {
    use bf_harness::refs::{index, list, sha_lines, text};
    use bf_harness::sound::{fighter_soundtrack, pcm_sha};

    let (s, kit, rec) = line_soundtrack();
    assert_eq!(
        pcm_sha(&s.st.master),
        text(&kit["master_sha256"]),
        "{NAME} with a line: the 16-bit master is not the kit's (run the_line_stems for the RMS profiles)"
    );
    let buses: Vec<&str> = list(&kit["buses"]).iter().map(text).collect();
    assert_eq!(s.buses, buses, "the buses in the score's order");
    let stems: Vec<&str> = list(&kit["stems"]).iter().map(text).collect();
    assert_eq!(s.st.stems.keys().map(String::as_str).collect::<Vec<_>>(), stems);
    let notes = &kit["notes"];
    assert!(index(&notes["count"]) > 0, "the kit's soundtrack has no note");
    assert_eq!(
        (s.notes.len(), sha_lines(&s.notes)),
        (index(&notes["count"]), text(&notes["sha256"]).to_string()),
        "the first-time notes: the first are {:?}, the kit's {:?}",
        &s.notes[..s.notes.len().min(8)],
        list(&notes["head"])
    );
    let end: u128 = text(&kit["noise"]["end"]).parse().expect("a state");
    let (mut a, mut b) = (
        s.noise.0.clone(),
        bf_core::rng::Generator::from_state(end, bf_sound::voice::PpNoise::START_INC),
    );
    assert!(
        (0..4).all(|_| a.random().to_bits() == b.random().to_bits()),
        "the noise stream does not end where the kit's did"
    );
    // and it is another soundtrack than the one without a line
    let plain = fighter_soundtrack(NAME, &rec, false, None);
    assert_ne!(
        pcm_sha(&s.st.master),
        pcm_sha(&plain.st.master),
        "the line changed nothing"
    );
    println!(
        "{NAME} with a line: the 16-bit master, {} buses, {} first-time notes and the noise stream equal the kit's",
        buses.len(),
        s.notes.len()
    );
}

/// The soundtrack with a line again, with the RMS of every stem and the master per 100 ms (cached) and every stem
/// window within -120 dBFS of the kit's. Ignored: reads the cache.
#[test]
#[ignore = "reads the pinned cache: python -m tools.studio_export cache fighters/trumpet"]
fn the_line_stems() {
    use bf_harness::refs::{fighter_folder, report, text};
    use bf_harness::sound::{compare, windows_against, with_cached_rms};

    let (s, kit, _) = line_soundtrack();
    let mut failures = Vec::new();
    let mut n = compare("with a line", &s, &with_cached_rms(&kit), &mut failures);
    n += windows_against(
        "with a line",
        &s.st,
        &kit["windows"],
        text(&kit["cache"]),
        &mut failures,
    );
    report(
        &fighter_folder(NAME),
        "stems",
        "line",
        "soundtrack facts and stem windows with a line",
        failures,
        n,
    );
}

/// The look's frame time (the plan's Task 8 extra, which Task 62 collects): a measurement that prints and checks
/// nothing. One thread, the `left` fight: the whole fight, the frames of a charging HIGH C and the frames of its
/// blast, each with the TRUMPET's look and with the bare ball for both fighters.
#[test]
#[ignore = "a measurement, run by name in release: cargo test --release -p bf-harness --test fighter_trumpet -- --ignored --exact look_frame_time --nocapture"]
fn look_frame_time() {
    use bf_harness::refs::{committed_recording, text};

    let fd = bf_harness::fight::doc(NAME);
    let rec = || committed_recording(text(&fd["left"]["file"]), None);
    let ff = bf_harness::fight::fight_file(&fd, false);
    let real = bf_harness::look::picture(NAME, rec(), &ff, false);
    let bare = bf_harness::look::picture("", rec(), &ff, false);
    let p = real.ctx();
    let frame_of = |t: f64| (t * 30.0).round() as usize;
    let (charge, fire) = {
        let at = |kind: &str| p.rec.events.iter().find(|e| e.kind == kind).map(|e| e.t);
        (at("charge").expect("a charge"), at("beam").expect("a blast"))
    };
    let sets: [(&str, Vec<usize>); 3] = [
        (
            "the fight, every 12th frame",
            (frame_of(p.show.grid.t_fight)..frame_of(p.t_ko)).step_by(12).collect(),
        ),
        (
            "HIGH C charging (the sight)",
            (frame_of(charge) + 1..frame_of(fire)).collect(),
        ),
        (
            "HIGH C's blast (to the end of its fade)",
            (frame_of(fire) + 1..frame_of(fire + 0.35 + 0.5)).collect(),
        ),
    ];
    let time = |pic: &bf_picture::ring::Picture, frames: &[usize]| {
        // once to fill the sprite caches, then the measured pass
        for &f in frames {
            std::hint::black_box(pic.frame(f));
        }
        let t0 = std::time::Instant::now();
        for &f in frames {
            std::hint::black_box(pic.frame(f));
        }
        t0.elapsed().as_secs_f64() * 1000.0 / frames.len() as f64
    };
    for (what, frames) in &sets {
        let (with, without) = (time(&real, frames), time(&bare, frames));
        println!(
            "{NAME} look frame time, {what} ({} frames): {with:.1} ms a frame with its look, {without:.1} ms with the \
             bare ball: {:+.1} ms",
            frames.len(),
            with - without
        );
    }
}
