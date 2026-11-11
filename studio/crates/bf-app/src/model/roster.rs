use std::sync::Arc;

/// A one-bit picture, such as a fighter's name in pixel letters. Row-major; `h` is 7 for one line of text.
#[derive(Clone, Debug, PartialEq)]
pub struct PixelMask {
    pub w: usize,
    pub h: usize,
    pub on: Vec<bool>,
}

/// A fighter's portrait, `w` x `h` pixels of RGBA.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    pub rgba: Arc<[u8]>,
}

/// A group of the roster, in the order the roster is shown. `Keys` is shown as "Keys and electronics".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Woodwinds,
    Brass,
    Strings,
    Keys,
    Percussion,
}

/// One of a fighter's three abilities.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityLine {
    /// "LV2".
    pub tag: String,
    /// As shown: "Flutter tongue".
    pub name: String,
    /// "Twice as fast for 8 beats".
    pub text: String,
    /// The roster's own name, "FLUTTER TONGUE": what an ability filter stores.
    pub key: String,
}

/// What the Match-up screen's corner card says about a fighter.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterCard {
    pub weapon: String,
    pub text: String,
    pub abilities: [AbilityLine; 3],
    /// The notes it plays, in words ("C5 to A6").
    pub range: String,
    pub usually_plays: String,
    pub colour_name: String,
    pub music_title: String,
    pub music_text: String,
    /// Its part can play the bass line.
    pub can_play_bass: bool,
    pub portrait: Option<Sprite>,
}

/// One fighter of the roster.
#[derive(Clone, Debug, PartialEq)]
pub struct RosterEntry {
    /// The roster's own name: "BASS CLARINET".
    pub name: String,
    /// As shown: "Bass clarinet".
    pub display: String,
    pub section: Section,
    pub color: [u8; 3],
    /// Built and ready to fight; the others are shown disabled.
    pub available: bool,
    /// Its notes are its weapon, so the chord loop and the music seed feed its fights.
    pub music_feeds_fight: bool,
    /// Its name in pixel letters.
    pub mask: PixelMask,
    pub card: Option<FighterCard>,
}

/// One theme: the music and look a fight is set in.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeEntry {
    /// The theme's own name: "big_top".
    pub name: String,
    /// As shown: "Big top".
    pub display: String,
    pub text: String,
    /// "E".
    pub key: String,
    /// "harmonic minor".
    pub scale: String,
    /// Each chord loop as its chord names.
    pub chord_loops: Vec<Vec<String>>,
    /// The tempo range as bars that fill 61 s, lowest and highest.
    pub bars: (u32, u32),
}

/// Everything the engine offers to fight with.
#[derive(Clone, Debug, PartialEq)]
pub struct Roster {
    pub fighters: Vec<RosterEntry>,
    pub themes: Vec<ThemeEntry>,
    /// "VS" in pixel letters.
    pub vs: PixelMask,
    /// The match-up a new draft starts from (roster names, left then right).
    pub start: [String; 2],
    pub app_version: String,
}
