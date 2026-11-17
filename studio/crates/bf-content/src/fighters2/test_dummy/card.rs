//! The TEST DUMMY's VS card panel (Engine 2 E6, Task 3): what the Match-up corner card says about it when the
//! developer switch offers it.

use crate::FighterCard;

/// The dummy's card. Its text names the five moves its brain picks; the ability lines describe the three moves of
/// [`MOVES`], in that order.
pub static CARD: FighterCard = FighterCard {
    weapon: "Fist on an arm",
    text: "Jab, Haymaker, Grab slam, Toss and Dummy buster: one of every kind of move, for testing Engine 2.",
    abilities: [
        "A hook that launches",
        "The meter special: two quick hits and an uppercut, in slow motion",
        "A guarded hop, below 25 HP",
    ],
    colour: "Grey",
    usually_plays: "Melody",
    music_title: "Test dummy, the stand-in",
    music_text: "A developer fighter. It plays the melody of whatever it is set against.",
    can_play_bass: false,
    emoji: "",
    tags: &[],
};

/// The roster names of the moves the card's three ability lines describe.
pub const MOVES: [&str; 3] = ["HAYMAKER", "DUMMY BUSTER", "SECOND WIND"];

/// How far the dummy picks its moves from, in words (its brain's nearest and farthest choice ranges).
pub const RANGE: &str = "1.1 to 9 m";
