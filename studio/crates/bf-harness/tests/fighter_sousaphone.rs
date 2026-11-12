//! SOUSAPHONE against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md), and two tests
//! of its own for what the suite's `left_fight` and `sound` gates cannot count on this fighter.

use bf_harness::refs::{committed_recording, fighter_folder, index, json, report, text};
use bf_sim::parity::{PREFIX_FLOOR_S, prefix_window};

bf_harness::fighter_suite!("SOUSAPHONE");

/// The committed `left` fight is the fight of the folder's fight file at its seed, and the engine agrees with it
/// for the prefix floor: what the suite's `left_fight` proves, with one difference in how the rules are compared.
///
/// The folder's fights run at the template's neutral handicap. The kit records that as `dmg_side = (1.0, 1.0)`; a
/// fight file with a handicap of 1.0 each is the Studio's "no handicap" and sets up rules without `dmg_side`
/// (`bf_sim::setup_from`: a damage times 1.0 is that damage, so no fight differs). The suite's gate compares the
/// two rules whole and so cannot pass at the neutral handicap; here a `dmg_side` of 1.0 each counts as none.
#[test]
fn left_fight_at_the_neutral_handicap() {
    let sim = bf_harness::sim!();
    let doc = bf_harness::fight::doc(NAME);
    let left = &doc["left"];
    let py = committed_recording(text(&left["file"]), Some(text(&left["hash"])));
    let ff = bf_harness::fight::fight_file(&doc, false);
    let setup = bf_sim::setup_from(&ff, bf_harness::fight::roster(), py.seed).expect("the fight file sets up");
    let mut like = bf_harness::fight::setup_like(&py);
    assert_eq!(
        like.rules.dmg_side,
        Some([1.0, 1.0]),
        "{NAME}: the left fight is no longer at the neutral handicap: the suite's `left_fight` gate covers it"
    );
    like.rules.dmg_side = None;
    assert_eq!(
        format!("{:?}", setup.rules),
        format!("{:?}", like.rules),
        "{NAME}: the fight file's rules"
    );
    let rs = bf_harness::fight::traced(&sim, &setup, true, &format!("{NAME} left"));
    let window = prefix_window(&rs, &py);
    let t_fight = setup.rules.t_fight;
    let lasts = window.agree_until_s - t_fight;
    println!(
        "{NAME} left: agrees for {lasts:.2} s of fighting ({} frames, {} events, max |d| {:.1e}); ended by {}",
        window.frames,
        window.events,
        window.worst,
        window.first_break.as_deref().unwrap_or("nothing (the whole fight)")
    );
    assert!(
        window.first_break.is_none() || lasts >= PREFIX_FLOOR_S,
        "{NAME} left: the window ends after {lasts:.3} s of fighting, before the floor of {PREFIX_FLOOR_S} s:\n {:?}",
        window.first_break
    );
    assert_eq!(
        (rs.winner, rs.t_ko.map(f64::to_bits)),
        (py.winner, py.t_ko.map(f64::to_bits))
    );
}

/// The left soundtrack and each role against the kit's summary, as the suite's `sound` gate compares them (the
/// 16-bit master's SHA-256, the buses, the stems, every 100 ms block of each listed bus, the first-time notes, the
/// noise stream's end), with this fighter's own minimum.
///
/// The suite's `sound` requires a first-time note of `kit.music.note` in the kit's left soundtrack. The kit's
/// SOUSAPHONE never calls it: every note of its part is its own module's `sousa_tone`, and the other side plays
/// the base `Sound`, so that count is 0 and there is no note to list. What proves that its part plays here is its
/// own bus: it must be in the kit's summary and louder than -40 dBFS in some block, and then every block of it is
/// compared like every other bus.
#[test]
fn sound_of_a_part_that_plays_its_own_tones() {
    let folder = fighter_folder(NAME);
    let doc = json(&folder, "sound.json");
    assert_eq!(text(&doc["fighter"]), NAME);
    let fd = bf_harness::fight::doc(NAME);
    let rec = committed_recording(text(&fd["left"]["file"]), Some(text(&fd["left"]["hash"])));
    assert_eq!(
        index(&doc["left"]["notes"]["count"]),
        0,
        "{folder}: the kit's left soundtrack now has first-time notes: the suite's `sound` gate covers it"
    );
    let bus = NAME.to_lowercase();
    let blocks = doc["left"]["rms"][&bus]
        .as_array()
        .unwrap_or_else(|| panic!("{folder}: sound.json has no RMS for the bus {bus:?}"));
    let loud = blocks
        .iter()
        .filter(|v| v.as_f64().is_some_and(|db| db > -40.0))
        .count();
    assert!(
        loud >= 100,
        "{folder}: the kit's {bus} bus passes -40 dBFS in {loud} of {} blocks: its part plays too little to compare",
        blocks.len()
    );
    let mut failures = Vec::new();
    let s = bf_harness::sound::fighter_soundtrack(NAME, &rec, false, None);
    let mut n = bf_harness::sound::compare("left", &s, &doc["left"], &mut failures);
    for (role, kit) in doc["roles"].as_object().expect("roles") {
        let s = bf_harness::sound::fighter_soundtrack(NAME, &rec, false, Some(role));
        n += bf_harness::sound::compare(&format!("role {role}"), &s, kit, &mut failures);
    }
    report(&folder, "sound", "sound", "soundtrack facts", failures, n);
}
