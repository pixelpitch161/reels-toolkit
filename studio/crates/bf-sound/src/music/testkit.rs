//! A recording, a fight file, a small voice and a bed that plays nothing, for the unit tests of this crate.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bf_core::dmath::{self, PI};
use bf_core::fightfile::FightFile;
use bf_core::recording::Recording;
use bf_core::roster::{FighterCfg, parse_roster};
use bf_core::show::Show;

use super::{MusicCtx, MusicError};
use crate::SRF;
use crate::mix::{Mix, Sig};
use crate::synth;
use crate::traits::{Bed, Played};
use crate::voice::{PpNoise, Voice};

fn studio() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The example episode: its recording (seed 392), fight file, show and the two fighters' roster entries.
pub(crate) struct Episode {
    pub(crate) rec: Recording,
    pub(crate) ff: FightFile,
    pub(crate) show: Show,
    pub(crate) cfg: Vec<FighterCfg>,
}

pub(crate) fn episode() -> Episode {
    let rec =
        Recording::read(&studio().join("reference/fights/flute_vs_trombone/ep392.bfrec")).expect("the recording reads");
    let read = |rel: &str| std::fs::read_to_string(studio().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
    let ff = FightFile::parse(&read("fights/flute_vs_trombone.toml")).expect("the fight file parses");
    let show = Show::new(&rec, &ff).expect("the episode has a K.O.");
    let roster = parse_roster(&read("crates/bf-content/data/roster.json")).expect("the roster reads");
    let cfg = show
        .names
        .iter()
        .map(|n| roster.iter().find(|f| &f.name == n).expect("in the roster").clone())
        .collect();
    Episode { rec, ff, show, cfg }
}

/// A voice for tests, under the roster name `name`: a sine on the note's pitch for `dur + 0.1` s (so it can
/// hold a `dur` down to -0.1 s), plus `draws` normals from the noise stream at a hundredth of the level. It
/// remembers every call.
pub(crate) struct TestVoice {
    pub(crate) name: &'static str,
    pub(crate) draws: usize,
    pub(crate) calls: Mutex<Vec<(f64, f64, f64)>>,
}

impl TestVoice {
    pub(crate) fn new(name: &'static str, draws: usize) -> Arc<TestVoice> {
        Arc::new(TestVoice {
            name,
            draws,
            calls: Mutex::new(Vec::new()),
        })
    }

    pub(crate) fn calls(&self) -> Vec<(f64, f64, f64)> {
        self.calls.lock().expect("the call list").clone()
    }
}

impl Voice for TestVoice {
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64> {
        self.calls.lock().expect("the call list").push((midi, vel, dur));
        let n = ((dur + 0.1) * SRF) as usize;
        let breath = noise.nz(self.draws);
        let f = synth::mtof(midi);
        (0..n)
            .map(|i| {
                let hiss = breath.get(i).map_or(0.0, |b| 0.01 * b);
                vel * (dmath::sin(2.0 * PI * f * (i as f64 / SRF)) + hiss)
            })
            .collect()
    }

    fn register(&self) -> (i32, i32) {
        (60, 73)
    }

    fn name(&self) -> &str {
        self.name
    }

    fn can_hold(&self, dur: f64) -> bool {
        dur >= -0.1
    }
}

/// A bed for tests: two buses, a room, a compressor, and sections that play nothing. It remembers the time
/// of every clash it is asked for, and marks each with one sample of 1.0 on the bus `clash`.
pub(crate) struct TestBed {
    pub(crate) reverb: (f64, f64, f64),
    pub(crate) clashes: Mutex<Vec<f64>>,
    /// The optional hooks it has (`Boom` unless a test says otherwise).
    pub(crate) hooks: &'static [crate::traits::BedHook],
}

impl TestBed {
    pub(crate) fn new(reverb: (f64, f64, f64)) -> TestBed {
        TestBed {
            reverb,
            clashes: Mutex::new(Vec::new()),
            hooks: &[crate::traits::BedHook::Boom],
        }
    }
}

impl Bed for TestBed {
    fn hooks(&self) -> &'static [crate::traits::BedHook] {
        self.hooks
    }
    fn buses(&self) -> &[(&'static str, (f64, f64))] {
        &[("drums", (1.0, 0.15)), ("fx", (0.5, 0.1))]
    }
    fn reverb(&self) -> (f64, f64, f64) {
        self.reverb
    }
    fn comp(&self) -> (f64, f64) {
        (0.15, 0.62)
    }
    fn roar_gain(&self) -> f64 {
        0.15
    }
    fn doors_gain(&self) -> f64 {
        0.4
    }
    fn landing(&self) -> (f64, f64) {
        (0.07, 0.05)
    }
    fn pop(&self) -> (f64, f64, f64) {
        (95.0, 100.0, 0.1)
    }
    fn boom(&self, _: &MusicCtx, _: &mut Mix, _: f64, _: f64) -> Played {
        if self.has(crate::traits::BedHook::Boom) {
            Ok(())
        } else {
            crate::traits::no_hook(crate::traits::BedHook::Boom)
        }
    }
    fn clash(&self, _: &MusicCtx, mix: &mut Mix, t: f64) -> Played {
        if t < 0.0 {
            return Err(MusicError::new("clash", format!("a clash at {t} s")));
        }
        self.clashes.lock().expect("the clash list").push(t);
        mix.place("clash", &Sig::Mono(vec![1.0]), t, 1.0, 0.0);
        Ok(())
    }
    fn title(&self, _: &MusicCtx, _: &mut Mix, _: [f64; 3]) -> Played {
        Ok(())
    }
    fn vs(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
        Ok(())
    }
    fn title_rest(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn count_in(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn fight_start(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn fight(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
        Ok(())
    }
    fn ko(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
    fn stinger(&self, _: &MusicCtx, _: &mut Mix, _: f64) -> Played {
        Ok(())
    }
    fn after_ko(&self, _: &MusicCtx, _: &mut Mix, _: usize, _: f64, _: f64) -> Played {
        Ok(())
    }
    fn victory(&self, _: &MusicCtx, _: &mut Mix, _: usize) -> Played {
        Ok(())
    }
    fn last_bars(&self, _: &MusicCtx, _: &mut Mix) -> Played {
        Ok(())
    }
}

/// The episode's context with two test voices: the FLUTE's draws 100 normals a note, the TROMBONE's none.
pub(crate) fn context(ep: &Episode) -> (MusicCtx<'_>, [Arc<TestVoice>; 2]) {
    let voices = [TestVoice::new("FLUTE", 100), TestVoice::new("TROMBONE", 0)];
    let ctx = MusicCtx::new(
        &ep.show,
        &ep.rec,
        &ep.ff,
        [&ep.cfg[0], &ep.cfg[1]],
        [voices[0].clone(), voices[1].clone()],
    )
    .expect("the context builds");
    (ctx, voices)
}
