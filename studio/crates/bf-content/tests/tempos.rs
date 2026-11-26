//! The example fight file at two other tempos (120 and 150: 30 and 38 bars instead of 34): the bed and both parts
//! follow the grid, the soundtrack is still whole AAC frames, and it is the kit's.

mod common;

use bf_content::{ContentError, SoundBuild, build_soundtrack, validate};
use bf_core::event::Value as EvValue;
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::show::{Show, ShowError};
use bf_sound::NA;

use common::{BUSES, EPISODE, block_db};

/// `rec` relabelled as fought on the grid of `ff`: its `rules.bpm`, `rules.beat`, `rules.t_fight` and `f0` are the
/// grid's, everything else (the run itself) is as it was.
fn on_grid(rec: &Recording, ff: &FightFile) -> Recording {
    let grid = ff.grid();
    let mut rec = rec.clone();
    for (key, v) in [("bpm", grid.bpm), ("beat", grid.beat), ("t_fight", grid.t_fight)] {
        rec.rules.insert(key.to_string(), EvValue::Float(v));
    }
    rec.f0 = bf_core::py::round_half_even(grid.t_fight * f64::from(rec.fps)) as usize;
    rec
}

/// What the kit made of the same run at another tempo: the bars, each stem's energy (the sum of its squared
/// samples, `float.hex()`), the master's, and the master's level in six 100 ms blocks.
struct OtherTempo {
    bpm: f64,
    bars: usize,
    energy: [(&'static str, &'static str); 7],
    master: &'static str,
    blocks: [(usize, f64); 6],
}

/// The fight file at `tempo_bpm = 120` and `150` (30 and 38 bars instead of 34): the bed and both parts follow
/// the grid, the soundtrack is still whole AAC frames, and it is the kit's.
///
/// The recording is the episode's own, fought at 132: a fight belongs to the tempo it was fought at (the engine
/// strikes on the beat), so a real video at another tempo needs a fight simulated at that tempo, and
/// `build_soundtrack` refuses the episode's recording under these fight files (`Show::new` compares the grids).
/// The kit has no such check: when an episode's `BPM` is changed after its run was made it plays the same run on
/// the new grid (it caches a run by its seed alone). The numbers are from exactly that, a copy of the example's
/// config with `BPM` replaced and one chord per bar of the new grid (the intro, then the loop over and over, as
/// `bf_core::harmony::chords_for` lays them out), through `kit.music.score()` and `master()`. Beyond these
/// numbers the kit's 16-bit master was compared once, by hand: the same samples at both tempos.
///
/// To compare with that, the parts are built through [`SoundBuild`] directly, from the episode's run relabelled
/// as fought on the new grid ([`on_grid`]): the score reads the run and the fight file's grid, never the
/// recording's own `rules` or `f0`, so this is the kit's stale run on the new grid, sample for sample.
#[test]
fn other_tempos_build() {
    #[rustfmt::skip]
    let cases = [
        OtherTempo {
            bpm: 120.0,
            bars: 30,
            energy: [
                ("drums", "0x1.ab892805681c8p+14"), ("calliope", "0x1.9721312b4ccf2p+13"),
                ("fx", "0x1.a209ec45f1e42p+12"), ("flute", "0x1.22d4facae6ebcp+15"),
                ("trombone", "0x1.81c9598f3270ap+16"), ("glock", "0x1.0896157e7479cp+5"),
                ("solo", "0x1.3f5e9d0f500f0p+13"),
            ],
            master: "0x1.421f3a1b2a7e6p+18",
            blocks: [
                (0, -10.268382592897261), (37, -11.381821917778698), (250, -11.985698527178094),
                (500, -9.95513838632431), (520, -12.373479082973988), (610, -11.433604111717937),
            ],
        },
        OtherTempo {
            bpm: 150.0,
            bars: 38,
            energy: [
                ("drums", "0x1.14e1452766800p+15"), ("calliope", "0x1.039ef06c140ccp+14"),
                ("fx", "0x1.b04f4f0b10994p+12"), ("flute", "0x1.89c3a73cce303p+15"),
                ("trombone", "0x1.77d0d515f4c7ep+16"), ("glock", "0x1.668d7d5125d2ep+5"),
                ("solo", "0x1.a9d36497fdaecp+13"),
            ],
            master: "0x1.6c9c189608643p+18",
            blocks: [
                (0, -9.91336100998854), (37, -11.77677961097145), (250, -12.612456059931102),
                (500, -9.16530826187168), (520, -12.02299463316066), (610, -11.23015564395333),
            ],
        },
    ];
    let energy = |x: &[[f64; 2]]| x.as_flattened().iter().map(|v| v * v).sum::<f64>();
    for case in cases {
        let mut ff = EPISODE.ff.clone();
        ff.tempo_bpm = case.bpm;
        assert_eq!(ff.grid().n_bars, case.bars, "tempo {}", case.bpm);
        assert_eq!(validate(&ff), Ok(()));
        let refused = build_soundtrack(&EPISODE.rec, &ff).unwrap_err();
        assert!(
            matches!(&refused, ContentError::Show(ShowError::Tempo(_))),
            "tempo {}: {refused:?}",
            case.bpm
        );
        let rec = on_grid(&EPISODE.rec, &ff);
        let show = Show::new(&rec, &ff).unwrap_or_else(|e| panic!("tempo {}: {e}", case.bpm));
        let build = SoundBuild::new(&show, &rec, &ff).unwrap_or_else(|e| panic!("tempo {}: {e}", case.bpm));
        let mix = build.score().unwrap_or_else(|e| panic!("tempo {}: {e}", case.bpm));
        let st = build.master(&mix);
        assert_eq!(st.master.len(), NA, "tempo {}", case.bpm);
        assert_eq!(st.stems.len(), BUSES.len(), "tempo {}", case.bpm);
        let mut worst = 0.0f64;
        for (bus, want) in case.energy {
            let stem = &st.stems[bus];
            assert_eq!(stem.len(), NA, "tempo {}: {bus}", case.bpm);
            let (got, want) = (energy(stem), bf_testdata::parse_hex_f64(want));
            let rel = ((got - want) / want).abs();
            assert!(
                rel < 1e-9,
                "tempo {}: {bus}'s energy is {got}, the kit's {want}",
                case.bpm
            );
            worst = worst.max(rel);
        }
        let (got, want) = (energy(&st.master), bf_testdata::parse_hex_f64(case.master));
        let rel = ((got - want) / want).abs();
        assert!(
            rel < 1e-9,
            "tempo {}: the master's energy is {got}, the kit's {want}",
            case.bpm
        );
        let levels = block_db(&st.master);
        assert_eq!(levels.len(), 611);
        for (b, want) in case.blocks {
            assert!(
                (levels[b] - want).abs() < 1e-6,
                "tempo {}: block {b} of the master is at {} dB, the kit's at {want}",
                case.bpm,
                levels[b]
            );
        }
        println!(
            "tempo {}: {} bars, {} samples; the stems' energies within {:.1e} of the kit's, the master's {rel:.1e}",
            case.bpm,
            case.bars,
            NA,
            worst.max(rel)
        );
    }
}
