//! BASSOON's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of `kit/fighters/bassoon.py` (the only sneak), its roster entry (a hop every beat, a stomp
//! on every landing, 2.5 times from behind; STACCATO 8 beats, BOCAL HOOK when the enemy is close, MOUNTAIN KING
//! below 25 HP: 20 hops), the colour table of `docs/ballfight/RULES.md` (forest green) and its kit configs
//! (`ballfight_oboe_vs_bassoon` and `ballfight_bassclarinet_vs_bassoon`: role `line`, no tags of its own, the
//! double reeds' caption emoji).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts (the kit's configs give it none: the instrument's own).
const TAGS: [&str; 2] = ["#bassoon", "#doublereed"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Tiptoe",
    text: "It stands still, then hops behind the enemy on the beat and stomps where it lands. A stomp from behind \
           hits much harder. Every stomp that lands lets it hop further.",
    abilities: [
        "Hops twice as often for 8 beats",
        "Its crook hooks a close enemy, spins it round and sets up a hop behind it.",
        "20 faster and faster hops round the enemy, below 25 HP.",
    ],
    colour: "Forest green",
    usually_plays: "Counter-line",
    music_title: "Bassoon, the sneak",
    music_text: "A short staccato note on every landing, a comic low blat, Grieg's Mountain King for its last dance.",
    can_play_bass: false,
    // The series' caption for the double reeds uses the flute: there is no bassoon emoji.
    emoji: "\u{1FA88}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
