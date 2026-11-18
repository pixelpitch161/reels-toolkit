//! The TRUMPET's Engine 2 VS card panel: what the Match-up corner card says about it on Engine 2.

use crate::FighterCard;

/// Its card. The text names every move its brain picks; the ability lines describe the three moves of [`MOVES`], in
/// that order.
pub static CARD: FighterCard = FighterCard {
    weapon: "Valve combo+",
    text: "Valve combo+: two piston jabs, then all three slam out as one chord. Ta-ka combo is the same, \
           double-tongued, while Double tongue+ lasts; Herald line and Piston rockets are its specials.",
    abilities: [
        "Six banner poles fence the foe in, and nick it on every bounce",
        "Twice as fast for 4 s, every jab tongued twice",
        "The pistons fly off as homing steam rockets, below 25 HP",
    ],
    colour: "Yellow",
    usually_plays: "Melody",
    music_title: "Trumpet, the fanfare",
    music_text: "A note on every jab and a bright chord on every combo, a rising call for the herald line and ta-ka \
                 graces while it double-tongues. Give it a line and it takes the melody.",
    can_play_bass: false,
    emoji: "\u{1F3BA}",
    tags: &["#bandkids", "#marchingband"],
};

/// The roster names of the moves the card's three ability lines describe.
pub const MOVES: [&str; 3] = ["HERALD LINE", "DOUBLE TONGUE+", "PISTON ROCKETS"];

/// How far it picks its moves from, in words (its brain's nearest and farthest choice ranges).
pub const RANGE: &str = "1.1 to 5 m";
