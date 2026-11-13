//! The draft: everything the owner has chosen for the fight being made.

use serde::{Deserialize, Serialize};

/// The caption text for each platform.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptionTexts {
    pub instagram: String,
    pub tiktok: String,
    pub shorts: String,
}

/// The fight being made: a thin layer of choices over a fight file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    /// Roster names, left then right ("FLUTE", "TROMBONE").
    pub fighters: [String; 2],
    /// The theme's own name ("big_top").
    pub theme: String,
    /// The tempo, as the number of bars that fill 61 s.
    pub bars: u32,
    /// Each fighter's damage multiplier, left then right.
    pub handicap: [f64; 2],
    /// The chosen seed, if one is.
    pub seed: Option<u64>,
    /// Which of the theme's chord loops plays.
    pub chord_loop: usize,
    /// Which fighter (0 or 1) plays bass, if either can.
    pub bass: Option<usize>,
    pub music_seed: u64,
    pub captions: CaptionTexts,
    /// The owner has typed in a caption, so drafted captions no longer replace it.
    pub captions_edited: bool,
    /// The fight file this draft was opened from, as TOML.
    pub base: Option<String>,
}

impl Draft {
    /// The tempo in beats per minute: `bars` whole bars of four beats in 61 s, snapped by the engine's own rule
    /// (`bf_core::grid::loop_bpm`), so the app and the engine always agree on it.
    pub fn tempo_bpm(&self) -> f64 {
        bf_core::grid::loop_bpm(self.bars as f64 * 4.0 * 60.0 / 61.0, 61.0, 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(bars: u32) -> Draft {
        Draft {
            fighters: ["FLUTE".to_string(), "TROMBONE".to_string()],
            theme: "big_top".to_string(),
            bars,
            handicap: [1.0, 1.0],
            seed: None,
            chord_loop: 0,
            bass: Some(1),
            music_seed: 1,
            captions: CaptionTexts {
                instagram: String::new(),
                tiktok: String::new(),
                shorts: String::new(),
            },
            captions_edited: false,
            base: None,
        }
    }

    #[test]
    fn tempo_snaps_to_whole_bars() {
        assert!((draft(34).tempo_bpm() - 133.770_491_803_278_7).abs() < 1e-12);
        assert_eq!(draft(24).tempo_bpm(), 24.0 * 4.0 * 60.0 / 61.0);
    }
}
