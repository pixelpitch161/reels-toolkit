//! BARI SAX's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of its reference kit module (`older/fighters/bari_sax.py`, from #39 v4), its roster entry
//! (every other bar a crane hoists it and swings it as a wrecking ball, harder the faster it swings, the cable growing
//! with every smash; GROWL a shockwave, SKANK a swing every bar for 8 beats, CABLE SNAP below 25 HP), the colour
//! table (coral) and its kit configs (`examples/bassoon_vs_barisax`, `examples/barisax_vs_bassclarinet`: role `bass`).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#barisax", "#saxophone"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Wrecking ball",
    text: "Every other bar a crane cable hooks its harness, hoists it back and swings it across the ring, smashing \
           what is in front of it. The faster the swing, the harder the hit; every smash lengthens the cable.",
    abilities: [
        "A gravelly shockwave rings out from the bari.",
        "For 8 beats it swings every bar, on the off-beats.",
        "Hoisted over the enemy's spot, the cable snaps and it crashes down, below 25 HP.",
    ],
    colour: "Coral",
    usually_plays: "Bass",
    music_title: "Bari sax, the wrecking ball",
    music_text: "One huge honk on every swing, bent like a doppler pass, a winch, a clank and a heavy crash.",
    can_play_bass: true,
    emoji: "\u{1F3B7}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
