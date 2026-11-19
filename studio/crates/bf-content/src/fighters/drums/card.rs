//! DRUMS's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: #28's README and `audio.py` (the drumhead pulses on every beat, DRUM ROLL, CRASH CYMBAL, DOUBLE KICK,
//! the pulses as the kit) and its roster entry (a pulse every beat, its reach +0.07 per landed pulse up to 2.3).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts.
const TAGS: [&str; 2] = ["#drums", "#drummer"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Drumhead pulse",
    text: "On every beat a drumhead ring blasts out of the drum ball, hitting anything it reaches and swatting enemy \
           darts and keys out of the air. Every pulse that lands pushes its reach a little further.",
    abilities: [
        "A drum roll: a pulse on every 16th for a bar",
        "A crash cymbal shields it for 6 beats",
        "A giant kick pedal stomps the enemy's spot, below 25 HP.",
    ],
    colour: "Orange",
    usually_plays: "The beat",
    music_title: "Drums, the pulses",
    music_text: "Every pulse is a drum: kick on 1 and 3, snare on 2 and 4, a snare roll for the drum roll and a tom \
                 fill for every pulse that lands. The beat thins out as the drums get hurt.",
    can_play_bass: false,
    emoji: "\u{1F941}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
