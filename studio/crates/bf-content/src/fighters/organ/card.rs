//! ORGAN's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of `kit/fighters/organ.py`, its roster entry (a row every 2 beats, 0.06 longer per
//! landed row; CATHEDRAL 6 beats, PULL ALL STOPS 8 beats, TOCCATA below 25 HP), the colour table of
//! `docs/ballfight/RULES.md` (wine) and its kit config `ballfight_piano_vs_organ` (role `bass`; the tags of its
//! captions).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts (the kit's piano against organ captions).
const TAGS: [&str; 2] = ["#pipeorgan", "#organist"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Pipe rows",
    text: "A row of organ pipes bursts out of the floor toward the enemy every second beat. Every row that lands \
           makes the next ones longer.",
    abilities: [
        "A dome that blocks every hit for 6 beats",
        "A row on every beat for 8 beats",
        "A forest of pipes slams down, below 25 HP",
    ],
    colour: "Wine",
    usually_plays: "Bass",
    music_title: "Organ, the pipes",
    music_text: "Short chord stabs on the beat, a low pedal under them, a toccata run.",
    can_play_bass: true,
    // The series uses the piano keys: there is no organ emoji.
    emoji: "\u{1F3B9}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
