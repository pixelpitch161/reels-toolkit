//! The fighters' instrument voices: the trait every voice implements, the noise the Pixel Pitch voices draw
//! from, and what those voices have in common.
//!
//! A fighter plays in the voice of its instrument from the "Pixel Pitch" instrument-history series
//! (`series/instrument-history/<instrument>_lowpoly.py`), reached in the kit through
//! `projects/which_instrument_wins/pp_bridge.py`: `pp_bridge.voice(name, midi, vel, dur)` is one note, made by the
//! instrument's own `synth` at 44,100 Hz and resampled to 48,000 Hz. The instruments' synthesisers are ported in
//! `bf-content` (one `voice.rs` per fighter); this module holds what they share:
//!
//! - [`Voice`]: one note at 48 kHz, as `pp_bridge.voice`.
//! - [`PpNoise`]: the series' ONE shared noise generator, and the generators two of its modules keep for
//!   themselves ([`ModuleNoise`]). Every voice with breath or attack noise draws from one of them, so a note's
//!   samples depend on every noisy note made before it. A soundtrack build starts them once
//!   ([`PpNoise::start`]) and synthesises its notes one at a time, in the order the score first asks for them.
//! - [`pp`]: the series' sample rate and time base, its pitch formula, its own filters, the way to 48 kHz, and
//!   the synthesis blocks several kit voices share.

use bf_core::rng::Generator;

/// The Pixel Pitch voices' noise: the shared stream `drums60_lowpoly._rng = np.random.default_rng(77)` (`.0`),
/// advanced by every `nz(n)` call of every voice, and the generators of the modules that keep their own (`.1`).
///
/// In the kit each is a module's global, made once when the module is first imported, so a note that draws from
/// one moves it on for every later note of the soundtrack, whichever fighter or bed plays it. A soundtrack build
/// keeps them all together in this one value, which [`crate::music::MusicCtx`] holds and every [`Voice::note`]
/// is handed.
///
/// The generators are public so a test can resume one from a state recorded with a reference note
/// (`Generator::from_state`); nothing else should draw from them, or every later note changes.
#[derive(Clone, Debug)]
pub struct PpNoise(pub Generator, pub ModuleNoise);

/// The generators two Pixel Pitch modules keep for themselves, each a module global of a fixed seed. Importing
/// the kit and the voices draws nothing from either (checked by the `shared` export: `starts` in
/// `studio/reference/shared/pp_blocks.json`).
#[derive(Clone, Debug)]
pub struct ModuleNoise {
    /// `altosax60_lowpoly._RN`, `np.random.default_rng(23)` at import: the breath and slap noise of
    /// [`pp::alto::alto_tone`]. The ALTO SAX's voice and the alto voice the mambo bed plays both draw from it, and
    /// the ALTO SAX's part replaces it with a freshly seeded generator before each tone it makes itself
    /// ([`pp::alto::part_seed`], through [`crate::music::MusicCtx::with_noise`]); later draws go on from there.
    pub alto: Generator,
    /// `violin_lowpoly.rng = np.random.default_rng(9)`: one `random()` a note for the vibrato's phase in
    /// `bowed` (the CELLO's voice).
    pub violin: Generator,
}

impl ModuleNoise {
    /// The seed of `altosax60_lowpoly._RN`.
    pub const ALTO_SEED: u64 = 23;
    /// The seed of `violin_lowpoly.rng`.
    pub const VIOLIN_SEED: u64 = 9;

    /// Both generators where a soundtrack build starts: fresh generators of their seeds.
    pub fn start() -> ModuleNoise {
        ModuleNoise {
            alto: Generator::new(Self::ALTO_SEED),
            violin: Generator::new(Self::VIOLIN_SEED),
        }
    }
}

impl PpNoise {
    /// The seed of the series' generator.
    pub const SEED: u64 = 77;
    /// The generator's `state` where a soundtrack build starts (`studio/reference/sound/pp_noise_start.json`):
    /// after every import of the kit and before the first note. It is the state of a fresh `default_rng(77)`:
    /// importing the kit and the voices draws nothing.
    pub const START_STATE: u128 = 289533129354323354318955456298181112347;
    /// The generator's `inc`, which never changes.
    pub const START_INC: u128 = 336983293413220778415499640756163231851;

    /// The noise where a soundtrack build starts: the shared stream and both module generators. Call it once per
    /// build.
    pub fn start() -> PpNoise {
        PpNoise::from_state(Self::START_STATE, Self::START_INC)
    }

    /// The shared stream resumed from a recorded `state` and `inc` (a reference note stores the state before its
    /// call); the module generators start fresh ([`ModuleNoise::start`]). A test that resumes one of those sets
    /// it on `.1` itself.
    pub fn from_state(state: u128, inc: u128) -> PpNoise {
        PpNoise(Generator::from_state(state, inc), ModuleNoise::start())
    }

    /// `drums60_lowpoly.nz(n)`: `_rng.normal(0, 1, n)`, the next `n` normals of the shared stream.
    pub fn nz(&mut self, n: usize) -> Vec<f64> {
        let mut x = vec![0.0; n];
        self.0.fill_normal(0.0, 1.0, &mut x);
        x
    }
}

/// An instrument voice: `pp_bridge.voice(name, midi, vel, dur)` for one instrument.
pub trait Voice: Send + Sync {
    /// One note at 48,000 Hz: MIDI note `midi` (a whole number; the kit passes `int(midi)`), velocity `vel`, held
    /// for `dur` seconds. The note rings on past `dur` as the synth makes it (a release, echoes), so it is longer
    /// than `dur`. Noise, if the voice has any, is drawn from `noise`, which is left where the note left it.
    ///
    /// A voice gives samples for every finite `midi` and `vel` and every `dur` it [can hold](Voice::can_hold),
    /// however far outside the instrument (Python does too; far outside they may be NaN). A `dur` a little below
    /// zero is such a `dur`: the note is then its release alone, as in Python. Where Python raises (a `dur` so far
    /// below zero that the note has a negative number of samples, a `dur` that is not a number) a voice panics;
    /// [`crate::music::MusicCtx::note`] asks `can_hold` first and returns an error instead.
    fn note(&self, noise: &mut PpNoise, midi: f64, vel: f64, dur: f64) -> Vec<f64>;

    /// The voice's name in `pp_bridge`'s roster (`entrant(name)[0]`), which is what a fighter's roster entry
    /// names as its `pp`: `FLUTE`, `TROMBONE`. [`crate::music::MusicCtx::new`] checks it against each fighter's.
    fn name(&self) -> &str;

    /// Whether the voice has a note for a held length of `dur` seconds: false where Python raises for it (the
    /// note would have a negative number of samples; a voice rings on for its own release past `dur`, so each
    /// has its own limit a little below zero). The default says yes to every `dur`: right for a voice whose
    /// `note` never panics.
    fn can_hold(&self, dur: f64) -> bool {
        let _ = dur;
        true
    }

    /// The instrument's register in `pp_bridge`'s roster, lowest and highest MIDI note (`entrant(name)[3]`).
    /// `kit.music.inst_gain` measures the instrument's level on the note in its middle, `(lo + hi) // 2`. This
    /// is the Pixel Pitch entrant's register, which need not be the fighter's own (`FighterCfg::register`):
    /// TROMBONE is 45 to 67 here and 40 to 65 as a fighter.
    fn register(&self) -> (i32, i32);
}

pub mod pp;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_is_a_fresh_generator_of_seed_77() {
        let (mut a, mut b) = (PpNoise::start(), Generator::new(PpNoise::SEED));
        let mut want = vec![0.0; 64];
        b.fill_normal(0.0, 1.0, &mut want);
        assert_eq!(a.nz(64), want);
        // The stream goes on where the last call left it.
        b.fill_normal(0.0, 1.0, &mut want);
        assert_eq!(a.nz(64), want);
        assert!(a.nz(0).is_empty());
        // A copy draws on its own.
        let mut copy = a.clone();
        assert_eq!(copy.nz(8), a.nz(8));
    }

    #[test]
    fn the_module_generators_start_fresh_and_go_their_own_way() {
        let mut a = PpNoise::start();
        let (mut alto, mut violin) = (Generator::new(23), Generator::new(9));
        assert_eq!(a.1.alto.random().to_bits(), alto.random().to_bits());
        assert_eq!(a.1.violin.random().to_bits(), violin.random().to_bits());
        // Drawing from one leaves the others where they were.
        let mut fresh = Generator::new(PpNoise::SEED);
        let mut want = vec![0.0; 4];
        fresh.fill_normal(0.0, 1.0, &mut want);
        assert_eq!(a.nz(4), want);
        assert_eq!(a.1.alto.random().to_bits(), alto.random().to_bits());
        // A stream resumed from a state brings fresh module generators with it.
        let mut b = PpNoise::from_state(PpNoise::START_STATE, PpNoise::START_INC);
        assert_eq!(
            b.1.alto.random().to_bits(),
            Generator::new(ModuleNoise::ALTO_SEED).random().to_bits()
        );
        assert_eq!(
            b.1.violin.random().to_bits(),
            Generator::new(ModuleNoise::VIOLIN_SEED).random().to_bits()
        );
    }
}
