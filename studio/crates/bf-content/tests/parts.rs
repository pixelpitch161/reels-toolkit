//! The hooks that the example episode's own soundtrack never runs, against the Python kit: the FLUTE as the
//! winner (its stinger and solo), the TROMBONE as the loser (it falls apart and flies home), the TROMBONE as a
//! riff (#48), the kit's sketch of a solo for a fight file that writes none, a light bar of the big top, and the
//! parts' and the bed's own instruments one by one.
//!
//! In the episode the TROMBONE wins and plays the bass, so `tests/stems.rs` cannot see these. The kit was run
//! for each scenario from an empty note cache and the start of the noise stream (`T.SOLO`, `T.ROLES` changed at
//! run time where the scenario says so), and of every bus it touched this file holds the first and the last
//! sounding frame, the energy (the sum of the squared samples) and one frame from the middle, as `float.hex()`.
//! The whole buses were compared once, by hand, when this was written: below -230 dBFS everywhere.

mod common;

use bf_content::SoundBuild;
use bf_content::fighters::flute::sound as flute;
use bf_content::themes::big_top::{self, BigTopClassic, CALLIOPE_ATT, CALLIOPE_VIB};
use bf_core::fightfile::FightFile;
use bf_core::show::Show;
use bf_sound::mix::Mix;
use bf_sound::traits::Bed;
use bf_testdata::parse_hex_f64;

use common::episode;

/// A bus the kit left: its first and last sounding frame, its energy, and the frame `mid` (left, right).
type Bus = (&'static str, usize, usize, &'static str, usize, [&'static str; 2]);

/// The energy of a bus against the kit's: the largest relative difference allowed.
const ENERGY_REL: f64 = 1e-9;
/// One frame against the kit's: the largest difference allowed (-180 dBFS).
const FRAME_ABS: f64 = 1e-9;

/// The mix has exactly the buses of `want`, each as the kit left it.
#[track_caller]
fn same(what: &str, mix: &Mix, want: &[Bus]) {
    let mut got: Vec<&str> = mix.bus_names();
    got.sort_unstable();
    let mut theirs: Vec<&str> = want.iter().map(|b| b.0).collect();
    theirs.sort_unstable();
    assert_eq!(got, theirs, "{what}: the buses");
    for &(bus, first, last, energy, mid, at_mid) in want {
        let x = mix.bus(bus).expect("a bus the mix lists");
        assert_eq!(
            (
                x.iter().position(|f| *f != [0.0, 0.0]),
                x.iter().rposition(|f| *f != [0.0, 0.0])
            ),
            (Some(first), Some(last)),
            "{what}, {bus}: the first and the last sounding frame"
        );
        let (got, kit) = (
            x.as_flattened().iter().map(|v| v * v).sum::<f64>(),
            parse_hex_f64(energy),
        );
        assert!(
            ((got - kit) / kit).abs() < ENERGY_REL,
            "{what}, {bus}: an energy of {got}, the kit's is {kit}"
        );
        for c in 0..2 {
            let kit = parse_hex_f64(at_mid[c]);
            assert!(
                (x[mid][c] - kit).abs() < FRAME_ABS,
                "{what}, {bus}: frame {mid}, channel {c} is {}, the kit's is {kit}",
                x[mid][c]
            );
        }
    }
}

/// `tw = q16(T_KO + 1.3)` of the episode, as the kit printed it.
const TW: &str = "0x1.9ada5a5a5a5a5p+5";

#[test]
fn the_flute_as_the_winner() {
    let ep = episode();
    let show = Show::new(&ep.rec, &ep.ff).unwrap();
    let b = SoundBuild::new(&show, &ep.rec, &ep.ff).unwrap();
    let mut mix = Mix::new(show.grid.dur + 4.0);
    let (flute, bed) = (b.sounds[0].as_ref(), b.bed.as_ref());
    flute.stinger(&b.ctx, bed, &mut mix, parse_hex_f64(TW)).unwrap();
    flute.solo(&b.ctx, bed, &mut mix, show.t_solo).unwrap();
    #[rustfmt::skip]
    same("the FLUTE's stinger and solo", &mix, &[
        ("solo", 2465118, 2664506, "0x1.0cc8a8f1ad39bp+11", 2564812, ["-0x1.feb0338660522p-5", "-0x1.feb0338660522p-5"]),
    ]);
    // The FLUTE has no part in the build into the title slam, and sets nothing up before the fight.
    let mut quiet = Mix::new(1.0);
    flute.last_bar(&b.ctx, bed, &mut quiet).unwrap();
    flute.prepare(&b.ctx, bed).unwrap();
    assert!(quiet.bus_names().is_empty());
}

#[test]
fn the_trombone_as_the_loser() {
    let ep = episode();
    let show = Show::new(&ep.rec, &ep.ff).unwrap();
    let b = SoundBuild::new(&show, &ep.rec, &ep.ff).unwrap();
    let mut mix = Mix::new(show.grid.dur + 4.0);
    let (trombone, bed) = (b.sounds[1].as_ref(), b.bed.as_ref());
    trombone.fall_apart(&b.ctx, bed, &mut mix).unwrap();
    trombone.reassemble(&b.ctx, bed, &mut mix).unwrap();
    #[rustfmt::skip]
    same("the TROMBONE falls apart and flies home", &mix, &[
        ("trombone", 2417835, 2838289, "0x1.34cf04e8a34a7p+7", 2628062, ["0x0.0p+0", "0x0.0p+0"]),
        ("fx", 2417835, 2451434, "0x1.01db6e3cda822p+6", 2434634, ["0x1.e7cf2b0ff467ep-9", "0x1.3acfb53c59419p-8"]),
    ]);
}

/// A fight file without solos: `T.default_solo(name)`, the kit's 2-bar motif on the first bar's chord.
#[test]
fn the_kits_sketch_of_a_solo() {
    let ep = episode();
    let mut ff = ep.ff.clone();
    ff.music.solos.clear();
    let show = Show::new(&ep.rec, &ff).unwrap();
    #[rustfmt::skip]
    let cases: [(usize, &str, Bus); 2] = [
        (0, "the FLUTE's sketched solo",
         ("solo", 2497412, 2674106, "0x1.0430707a3701cp+11", 2585759, ["0x1.17569d306b5a2p-3", "0x1.17569d306b5a2p-3"])),
        (1, "the TROMBONE's sketched solo",
         ("solo", 2497412, 2841630, "0x1.47d3018a2a5c4p+11", 2669521, ["0x1.4ed5f70b60b1ep-6", "0x1.4ed5f70b60b1ep-6"])),
    ];
    for (k, what, want) in cases {
        let b = SoundBuild::new(&show, &ep.rec, &ff).unwrap();
        let mut mix = Mix::new(show.grid.dur + 4.0);
        b.sounds[k].solo(&b.ctx, b.bed.as_ref(), &mut mix, show.t_solo).unwrap();
        same(what, &mix, &[want]);
    }
}

/// #48 (opt-in): the TROMBONE over someone else's bass. Its line is the kit's sketch (the fight file writes none
/// for it): every thrust is a stab of it, and after its solo it goes back to it; it does not climb in the last
/// bar.
#[test]
fn the_trombone_as_a_riff() {
    let ep = episode();
    let mut ff = ep.ff.clone();
    ff.music.roles.insert("TROMBONE".into(), "riff".into());
    let show = Show::new(&ep.rec, &ff).unwrap();
    let b = SoundBuild::new(&show, &ep.rec, &ff).unwrap();
    let (trombone, bed) = (b.sounds[1].as_ref(), b.bed.as_ref());
    assert_eq!((trombone.role(&b.ctx), trombone.band()), ("riff", (120.0, 1500.0)));
    let mut mix = Mix::new(show.grid.dur + 4.0);
    trombone.play(&b.ctx, bed, &mut mix).unwrap();
    #[rustfmt::skip]
    same("the riff's part", &mix, &[
        ("trombone", 172235, 2425488, "0x1.4d3e93eaf587ep+14", 1298861, ["-0x1.2209cbc628494p-6", "-0x1.8e5ab80833059p-6"]),
        ("fx", 215295, 2411088, "0x1.1e5c473c4d4d5p+10", 1313191, ["0x0.0p+0", "0x0.0p+0"]),
        // PEDAL TONE's stomp: the bed's drum and a crash.
        ("drums", 2110236, 2187035, "0x1.2205bcebab6b0p+9", 2148635, ["0x1.3bdd35255e7b3p-7", "0x1.3bdd35255e7b3p-7"]),
    ]);
    // Its solo, from a fresh build (the kit's numbers are from an empty note cache).
    let b = SoundBuild::new(&show, &ep.rec, &ff).unwrap();
    let (trombone, bed) = (b.sounds[1].as_ref(), b.bed.as_ref());
    let mut mix = Mix::new(show.grid.dur + 4.0);
    trombone.solo(&b.ctx, bed, &mut mix, show.t_solo).unwrap();
    trombone.last_bar(&b.ctx, bed, &mut mix).unwrap();
    #[rustfmt::skip]
    same("the riff's solo", &mix, &[
        ("solo", 2497412, 2841630, "0x1.9888f6cb65c2ep+11", 2669521, ["-0x1.7088d2e7fa736p-9", "-0x1.7088d2e7fa736p-9"]),
    ]);
}

/// `bed.bar(mix, 5, intensity=1.0, light=True)`, `bed.bar(mix, 8, intensity=1.3)`, `bed.bar(mix, 11,
/// intensity=1.26)`, `bed.boom(mix, 30.0, 0.5)`, `bed.clash(mix, 31.337)`: a light bar (the kit's own sections
/// never ask for one), a big bar on an even bar (with the glockenspiel) and on an odd one, the slam drum and a
/// clash.
#[test]
fn the_beds_bars() {
    let ep = episode();
    let show = Show::new(&ep.rec, &ep.ff).unwrap();
    let b = SoundBuild::new(&show, &ep.rec, &ep.ff).unwrap();
    let mut mix = Mix::new(show.grid.dur + 4.0);
    let bed = BigTopClassic::new();
    bed.bar(&b.ctx, &mut mix, 5, 1.0, true).unwrap();
    bed.bar(&b.ctx, &mut mix, 8, 1.3, false).unwrap();
    bed.bar(&b.ctx, &mut mix, 11, 1.26, false).unwrap();
    bed.boom(&b.ctx, &mut mix, 30.0, 0.5).unwrap();
    bed.clash(&b.ctx, &mut mix, 31.337).unwrap();
    #[rustfmt::skip]
    same("three bars, a boom and a clash", &mix, &[
        ("drums", 430588, 1457279, "0x1.0f9145f85503bp+11", 943933, ["0x0.0p+0", "0x0.0p+0"]),
        ("calliope", 430589, 1033890, "0x1.6a3a42b9d8ee5p+9", 732239, ["-0x1.6fb7f04d5bd93p-7", "-0x1.6fb7f04d5bd93p-7"]),
        ("glock", 688942, 727340, "0x1.1b7749e5cc2d3p+2", 708141, ["-0x1.eec13a03cc497p-9", "-0x1.eec13a03cc497p-9"]),
        ("fx", 1504176, 1523375, "0x1.01f098ca70659p+2", 1513775, ["-0x1.99a09931a9855p-8", "-0x1.99a09931a9855p-8"]),
    ]);
    // A bar past the episode's last is an error, not a panic.
    let mut mix = Mix::new(1.0);
    assert_eq!(
        bed.bar(&b.ctx, &mut mix, 34, 1.0, false).unwrap_err().to_string(),
        "big_top: bar 34 is past the episode's last bar (34 bars)"
    );
}

/// One sound: its length, its energy and its middle sample as the kit made them.
#[track_caller]
fn sound(what: &str, got: &[f64], len: usize, energy: &str, at_mid: &str) {
    assert_eq!(got.len(), len, "{what}: samples");
    let (e, kit) = (got.iter().map(|v| v * v).sum::<f64>(), parse_hex_f64(energy));
    assert!(
        ((e - kit) / kit).abs() < ENERGY_REL,
        "{what}: an energy of {e}, the kit's is {kit}"
    );
    let (mid, kit) = (got[len / 2], parse_hex_f64(at_mid));
    assert!(
        (mid - kit).abs() < FRAME_ABS,
        "{what}: the middle sample is {mid}, the kit's is {kit}"
    );
}

/// The FLUTE's own sounds: `breath(0.1, 5)`, `flutter(np.sin(np.arange(4800) * 0.05), 24.0)`, `shriek(1.7)`,
/// `shriek()`, `swat_sound(31)`, `swat_sound()`.
#[test]
fn the_flutes_own_sounds() {
    sound(
        "breath",
        &flute::breath(0.1, flute::BREATH_SEED).unwrap(),
        4800,
        "0x1.cea883503e19cp+7",
        "-0x1.a717074133b5ep-7",
    );
    let tone: Vec<f64> = (0..4800).map(|i| bf_core::dmath::sin(f64::from(i) * 0.05)).collect();
    sound(
        "flutter",
        &flute::flutter(&tone, flute::FLUTTER_RATE),
        4800,
        "0x1.5f4b7ccc69ad1p+10",
        "0x1.2945de7fc4772p-1",
    );
    sound(
        "shriek(1.7)",
        &flute::shriek(1.7, flute::SHRIEK_F0).unwrap(),
        81600,
        "0x1.c01210e86fd24p+13",
        "-0x1.9d6d0cf3b3dc7p-2",
    );
    sound(
        "shriek()",
        &flute::shriek(flute::SHRIEK_DUR, flute::SHRIEK_F0).unwrap(),
        76800,
        "0x1.a520169a70592p+13",
        "-0x1.9d9d6db70e60bp-2",
    );
    sound(
        "swat_sound(31)",
        &flute::swat_sound(31),
        14400,
        "0x1.9ef09eacf4555p+8",
        "0x1.e93da1052de30p-5",
    );
    sound(
        "swat_sound()",
        &flute::swat_sound(flute::SWAT_SOUND_SEED),
        14400,
        "0x1.a96793e10fcd6p+8",
        "-0x1.1633d4c6b1815p-5",
    );
    // Where Python raises (a sound of no samples cannot be filtered), these are errors.
    assert_eq!(
        flute::breath(0.0, 5).unwrap_err().to_string(),
        "breath: a length of 0 s"
    );
    assert_eq!(
        flute::shriek(f64::NAN, 440.0).unwrap_err().to_string(),
        "shriek: a length of NaN s"
    );
    // The flutter is the tone at full level and at a tenth of it in turn; at a zero of its sine, 0.55 of it.
    let ones = vec![1.0; 4800];
    let f = flute::flutter(&ones, 24.0);
    assert_eq!((f[0], f[500], f[1500]), (0.55, 1.0, 0.55 - 0.45));
}

/// The big top's instruments: `calliope((64, 67, 71), 0.13, 1.0, seed=9)`, `calliope((59, 63, 69), 1.74, 0.8,
/// seed=203, att=0.06, vib=0.006)`, `calliope((), 0.2, 1.0, seed=1)`, `bass_drum(0.95)`, `choke(0.9, 43)`,
/// `snare(0.8)`, `roll(T.BEAT, 0.5, 1.0)`, `tambourine(0.6, 41)`.
#[test]
fn the_big_tops_instruments() {
    let grid = episode().ff.grid();
    sound(
        "calliope",
        &big_top::calliope(&[64, 67, 71], 0.13, 1.0, 9, CALLIOPE_ATT, CALLIOPE_VIB),
        9120,
        "0x1.377158e9bd488p+10",
        "0x1.6c20dc37d8e88p-6",
    );
    sound(
        "the held organ",
        &big_top::calliope(&[59, 63, 69], 1.74, 0.8, 203, 0.06, 0.006),
        86400,
        "0x1.2bc7b2e8f4818p+13",
        "-0x1.7d6cd105fc5d7p-6",
    );
    sound(
        "a calliope of no pipes",
        &big_top::calliope(&[], 0.2, 1.0, 1, CALLIOPE_ATT, CALLIOPE_VIB),
        12480,
        "0x1.c00f52bbc2936p+1",
        "0x1.0f82bd2e6e1b6p-7",
    );
    sound(
        "bass_drum",
        &big_top::bass_drum(0.95),
        17280,
        "0x1.858cb55237642p+9",
        "0x1.dfae0d973c2fbp-4",
    );
    sound(
        "choke",
        &big_top::choke(0.9, 43),
        10560,
        "0x1.98241483444f6p+9",
        "0x1.27c9ef8347253p-6",
    );
    sound(
        "snare",
        &big_top::snare(0.8),
        16800,
        "0x1.7642a742ce1e0p+8",
        "-0x1.1ab26fff4f538p-5",
    );
    // The kit's default rate: a 32nd of the episode.
    sound(
        "roll",
        &big_top::roll(grid.beat, 0.5, 1.0, grid.s16 / 2.0),
        28729,
        "0x1.78de135667d4ap+10",
        "0x1.7471d9f5f14dap-2",
    );
    sound(
        "tambourine",
        &big_top::tambourine(0.6, 41),
        5760,
        "0x1.07eab53013f00p+7",
        "0x1.04d6ab723dc07p-11",
    );
}

/// What a fight file can ask of the parts that the kit's Python cannot play either: each is an error.
#[test]
fn a_fight_file_the_parts_cannot_play_is_an_error() {
    let ep = episode();
    let show = Show::new(&ep.rec, &ep.ff).unwrap();
    let play = |ff: &FightFile, k: usize, what: &str| -> String {
        let b = SoundBuild::new(&show, &ep.rec, ff).unwrap();
        let mut mix = Mix::new(show.grid.dur + 4.0);
        let (part, bed) = (b.sounds[k].as_ref(), b.bed.as_ref());
        let result = match what {
            "title_blast" => part.title_blast(&b.ctx, bed, &mut mix, 0.004),
            "play" => part.play(&b.ctx, bed, &mut mix),
            "solo" => part.solo(&b.ctx, bed, &mut mix, show.t_solo),
            _ => unreachable!(),
        };
        result.unwrap_err().to_string()
    };
    // No title hits at all (Python: `hits[-1]` of an empty tuple).
    let mut ff = ep.ff.clone();
    ff.music.title_hits.insert("FLUTE".into(), Vec::new());
    ff.music.title_hits.insert("TROMBONE".into(), Vec::new());
    assert_eq!(play(&ff, 0, "title_blast"), "FLUTE: the title hits are an empty list");
    assert_eq!(
        play(&ff, 1, "title_blast"),
        "TROMBONE: the title hits are an empty list"
    );
    // A rest in the FLUTE's line, which plays a note on every 8th (Python: `int(None)`).
    let mut ff = ep.ff.clone();
    ff.music.lines.get_mut("FLUTE").unwrap().fill(None);
    let e = play(&ff, 0, "play");
    assert!(e.starts_with("FLUTE: FLUTE's line has a rest at "), "{e}");
    // An empty solo for the TROMBONE (Python: `max()` of nothing); the FLUTE just plays none.
    let mut ff = ep.ff.clone();
    ff.music.solos.insert("TROMBONE".into(), Vec::new());
    assert_eq!(play(&ff, 1, "solo"), "TROMBONE: TROMBONE's solo has no notes");
    // A note no trombone can make (a MIDI note so high that no harmonic fits under 7 kHz).
    let mut ff = ep.ff.clone();
    ff.music.title_hits.insert("TROMBONE".into(), vec![40, 127]);
    let e = play(&ff, 1, "title_blast");
    assert!(
        e.starts_with("note: TROMBONE gives samples that are not numbers for MIDI note 127"),
        "{e}"
    );
}

/// A chord too small for the TROMBONE's bass would be an error in words, not a panic (`walk_note` needs a second
/// note above the root, the climb in `last_bar` at least one note): every chord type the kit names has three
/// notes or more, so through the public hooks each one walks and climbs.
#[test]
fn every_chord_type_walks_and_climbs() {
    use bf_content::fighters::trombone::TromboneSound;
    use bf_core::harmony::QUALITY;
    let ep = episode();
    for (quality, intervals) in QUALITY {
        let chord = format!("E{quality}");
        let mut ff = ep.ff.clone();
        ff.music.chords_intro.iter_mut().for_each(|c| *c = chord.clone());
        ff.music.chord_loop.iter_mut().for_each(|c| *c = chord.clone());
        let show = Show::new(&ep.rec, &ff).unwrap();
        let b = SoundBuild::new(&show, &ep.rec, &ff).unwrap_or_else(|e| panic!("{chord}: {e}"));
        let grid = &show.grid;
        // The third 8th of bar 4: `table[2]`, the root (E2, the register's bottom) plus the chord's second note.
        let t = grid.bar_t(4.0) + grid.beat;
        let walk = TromboneSound::new(1, "bass").walk_note(&b.ctx, t);
        assert_eq!(walk, Ok(40 + intervals[1]), "{chord}");
        let mut mix = Mix::new(grid.dur + 4.0);
        b.sounds[1]
            .last_bar(&b.ctx, b.bed.as_ref(), &mut mix)
            .unwrap_or_else(|e| panic!("{chord}: {e}"));
        let bus = mix
            .bus("trombone")
            .unwrap_or_else(|| panic!("{chord}: the climb played nothing"));
        assert!(bus.iter().any(|f| *f != [0.0, 0.0]), "{chord}: the climb is silent");
    }
}
