//! SOPRANO SAX's card: what the Match-up corner card and the Music screen say about it. (No text names the instrument
//! by its second word alone: the fight file retires that word.)

use crate::cards::FighterCard;

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Hornet swarm",
    text: "Hornets circle the ball. Every bar some peel off and chase the enemy down. Every sting brings another \
           hornet into the swarm, and the bigger it gets, the more fly out each bar.",
    abilities: [
        "The whole swarm charges the enemy at once for 4 beats",
        "The swarm wraps the ball as armour for 4 beats, stinging anything that touches it",
        "A big queen hornet hunts the enemy for 8 beats and stings hard, below 25 HP",
    ],
    colour: "Signal green",
    usually_plays: "Melody",
    music_title: "Soprano, the hornet",
    music_text: "Bright, buzzing and quick: darting trills on every launch, a high stab on every sting, a tremolo buzz \
                 under the armour and a proud call for the queen.",
    can_play_bass: false,
    emoji: "\u{1F3B7}",
    tags: &["#sopranosax", "#jazz", "#bandkids"],
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
