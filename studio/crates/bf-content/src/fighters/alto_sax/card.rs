//! ALTO SAX's card: what the Match-up corner card and the Music screen say about it.
//!
//! Sources: the docstring of `kit/fighters/alto_sax.py`, its roster entry (a sweep every 4 beats, the light 0.03
//! wider and the burn 0.06 hotter per sweep that lands; OCTAVE KEY 8 beats, ALTISSIMO 8 beats, ENCORE 8 beats below
//! 25 HP), the colour table of `docs/ballfight/RULES.md` (orchid) and its kit config
//! `ballfight_altosax_vs_tenorsax` (role `melody`; the tags of its captions).

use crate::cards::FighterCard;

/// Its own tags, for the caption drafts (the kit's alto against tenor captions).
const TAGS: [&str; 2] = ["#altosax", "#saxophone"];

/// Its card.
pub static CARD: Option<FighterCard> = Some(FighterCard {
    weapon: "Spotlight solo",
    text: "Every bar a spotlight swings down and sweeps across the enemy's path, burning what it catches on every \
           16th note. Every sweep that lands makes the light wider and hotter.",
    abilities: [
        "A second light crosses the first for 8 beats",
        "A narrow, white-hot light for 8 beats",
        "The light chases the enemy for 8 beats and the phones in the stands join in, below 25 HP",
    ],
    colour: "Orchid",
    usually_plays: "Melody",
    music_title: "Alto sax, the spotlight",
    music_text: "The bright lead. Its line in eighths under every sweep, a squeal on every burn.",
    can_play_bass: false,
    emoji: "\u{1F3B7}",
    tags: &TAGS,
});

/// [`CARD`], as the app asks for it.
pub fn card() -> Option<&'static FighterCard> {
    CARD.as_ref()
}
