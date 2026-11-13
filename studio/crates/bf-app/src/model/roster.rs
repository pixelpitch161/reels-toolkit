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
    /// The music a match-up with it in the left corner starts from when the pair has no kit config of its own (its
    /// home config), or `None` when the app knows none.
    pub home: Option<Home>,
}

/// A fighter's home config, as far as a new match-up needs it (milestone 3 Task 59: its newest kit config).
#[derive(Clone, Debug, PartialEq)]
pub struct Home {
    /// Its theme's own name.
    pub theme: String,
    /// Its chord loop.
    pub chord_loop: Vec<String>,
    /// The fighter's part in it ("melody", "bass", ...).
    pub role: String,
}

/// One of the series' kit configs: the fight file a match-up of these two starts from.
#[derive(Clone, Debug, PartialEq)]
pub struct KitPair {
    /// Roster names, left then right, as the kit config has them.
    pub fighters: [String; 2],
    pub theme: String,
    pub chord_loop: Vec<String>,
    /// The kit's frozen handicap, left then right (ruling M3-R6).
    pub handicap: [f64; 2],
    /// Each fighter's part, left then right.
    pub roles: [String; 2],
}

/// How a new match-up of two fighters starts (the engine's starter, as the interface needs it): the theme it is
/// written for, the place of its chord loop among that theme's loops (one past the last when it is none of them),
/// the handicap and each fighter's part, left then right.
#[derive(Clone, Debug, PartialEq)]
pub struct PairStart {
    pub theme: String,
    pub chord_loop: usize,
    pub handicap: [f64; 2],
    pub roles: [String; 2],
}

/// What a theme option is, for the words the Music screen shows (read-only) about it.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionShape {
    /// True or false, with its default and the words shown for each value (none for the value that is the plain
    /// theme: "The sequel" when `sequel` is on, "No bass drums" when `bass_drums` is off).
    Switch {
        default: bool,
        on: Option<String>,
        off: Option<String>,
    },
    /// One of the two fighters, the left corner's by default, shown as "{label}: {Name}" ("Plugged: Electric").
    Fighter { label: String },
}

/// One option of a theme.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeOptionEntry {
    /// Its name in a fight file ("sequel").
    pub key: String,
    pub shape: OptionShape,
}

/// The value a fight file gives a theme option.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionSet {
    Switch(bool),
    /// A fighter, by roster name.
    Fighter(String),
}

/// What the fight file a draft is laid over says about its music: the starter of a new match-up, or the file it was
/// opened from. The Music screen shows its parts and its theme's options from it, read-only (ruling M3-R8).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PairMusic {
    /// The theme it is written for.
    pub theme: String,
    /// Its chord loop.
    pub chord_loop: Vec<String>,
    /// Each fighter's part as it writes it, left then right of the draft it was described for.
    pub roles: [String; 2],
    /// The theme options it sets, by name; an option it leaves out is at the theme's default.
    pub options: Vec<(String, OptionSet)>,
}

/// The part that is the bass line.
pub const BASS_ROLE: &str = "bass";

/// The parts as a fight file writes them, after the one change the app makes to them (ruling M3-R8): when both
/// fighters' cards can play bass (`can`), one of the two plays bass and `bass` names the other, the two swap parts.
/// In every other case the parts stay as written.
pub fn parts_with_bass(written: &[String; 2], bass: Option<usize>, can: [bool; 2]) -> [String; 2] {
    let mut parts = written.clone();
    if let Some(side) = bass.filter(|&s| s < 2)
        && can == [true, true]
        && parts[side] != BASS_ROLE
        && parts[1 - side] == BASS_ROLE
    {
        parts.swap(0, 1);
    }
    parts
}

/// Whether the owner may say who plays bass: both cards can play it and one of the written parts is the bass.
pub fn bass_can_move(written: &[String; 2], can: [bool; 2]) -> bool {
    can == [true, true] && written.iter().any(|r| r == BASS_ROLE)
}

/// A part in words, for "Flute plays the melody.": "the melody", "the bass", "the answering line", "the riff".
pub fn part_words(role: &str) -> String {
    match role {
        "line" => "the answering line".to_string(),
        "" => "the melody".to_string(),
        other => format!("the {}", other.replace('_', " ")),
    }
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
    /// The roster fighters it cannot be played with (roster names): each one's part plays a hit its music lacks, so
    /// the kit would crash on any pair with it (ruling M3-R2). Empty for most themes.
    pub refuses: Vec<String>,
    /// Its options (the trailer's sequel, the drumline's bass drums, the plugged fighter): none for most themes.
    pub options: Vec<ThemeOptionEntry>,
}

impl ThemeEntry {
    /// The words for this theme's options in a fight of `fighters` (roster names, left first) whose fight file
    /// sets `set` (an option it leaves out is at its default, a fighter option's default being the left corner's
    /// fighter): one line per option that differs from the plain theme, in the theme's order ("The sequel", "No
    /// bass drums", "Plugged: Electric").
    pub fn option_words(&self, set: &[(String, OptionSet)], fighters: &[String; 2]) -> Vec<String> {
        self.options
            .iter()
            .filter_map(|o| {
                let given = set.iter().find(|(key, _)| *key == o.key).map(|(_, v)| v);
                match &o.shape {
                    OptionShape::Switch { default, on, off } => {
                        let value = match given {
                            Some(OptionSet::Switch(b)) => *b,
                            _ => *default,
                        };
                        if value { on.clone() } else { off.clone() }
                    }
                    OptionShape::Fighter { label } => {
                        let name = match given {
                            Some(OptionSet::Fighter(name)) => name.as_str(),
                            _ => fighters[0].as_str(),
                        };
                        Some(format!("{label}: {}", crate::wording::display_name(name)))
                    }
                }
            })
            .collect()
    }
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
    /// The series' kit configs: the pairs that start from their own fight file.
    pub kit_pairs: Vec<KitPair>,
}

impl Roster {
    /// The themes the pair `fighters` (left corner first) may be set in, in the order they are offered: those that
    /// refuse neither fighter (ruling M3-R2). The pickers list only these.
    pub fn themes_for(&self, fighters: &[String; 2]) -> Vec<&ThemeEntry> {
        self.themes
            .iter()
            .filter(|t| !fighters.iter().any(|f| t.refuses.contains(f)))
            .collect()
    }

    /// How a new match-up of `fighters` starts, as the engine's starter makes it (`bf_content::starter`): a kit
    /// pair from its kit config (the other order swaps the handicap and the parts); any other pair from the left
    /// fighter's home config, in its theme when the pair can play it, else the right fighter's home theme, else the
    /// first theme offered (the big top), with an even handicap, the left fighter's home part and the right one's
    /// own unless that is the same (then the bass when its card can play bass and the left's part is not the bass,
    /// else the melody unless the left plays it, else the answering line). `None` when the roster knows no start
    /// for them (a fighter without a home, as in the sample roster).
    pub fn start_of(&self, fighters: &[String; 2]) -> Option<PairStart> {
        let index = |theme: &str, chords: &[String]| {
            self.themes.iter().find(|t| t.name == theme).map_or(0, |t| {
                t.chord_loops
                    .iter()
                    .position(|l| l.as_slice() == chords)
                    .unwrap_or(t.chord_loops.len())
            })
        };
        // The config in this order first (HORN and TROMBONE have one for each order), else the other order's.
        let swapped = [fighters[1].clone(), fighters[0].clone()];
        let written = |pair: &[String; 2]| self.kit_pairs.iter().find(|k| k.fighters == *pair);
        if let Some(kit) = written(fighters) {
            return Some(PairStart {
                chord_loop: index(&kit.theme, &kit.chord_loop),
                theme: kit.theme.clone(),
                handicap: kit.handicap,
                roles: kit.roles.clone(),
            });
        }
        if let Some(kit) = written(&swapped) {
            return Some(PairStart {
                chord_loop: index(&kit.theme, &kit.chord_loop),
                theme: kit.theme.clone(),
                handicap: [kit.handicap[1], kit.handicap[0]],
                roles: [kit.roles[1].clone(), kit.roles[0].clone()],
            });
        }
        let entry = |name: &str| self.fighters.iter().find(|f| f.name == name);
        let (left, right) = (entry(&fighters[0])?, entry(&fighters[1])?);
        let (hl, hr) = (left.home.as_ref()?, right.home.as_ref()?);
        let offered: Vec<&str> = self.themes_for(fighters).iter().map(|t| t.name.as_str()).collect();
        let theme = [hl.theme.as_str(), hr.theme.as_str()]
            .into_iter()
            .find(|t| offered.contains(t))
            .or_else(|| offered.first().copied())?
            .to_string();
        let rl = hl.role.clone();
        let mut rr = hr.role.clone();
        if rr == rl {
            let can = right.card.as_ref().is_some_and(|c| c.can_play_bass);
            rr = if can && rl != BASS_ROLE {
                BASS_ROLE
            } else if rl != "melody" {
                "melody"
            } else {
                "line"
            }
            .to_string();
        }
        Some(PairStart {
            chord_loop: index(&theme, &hl.chord_loop),
            theme,
            handicap: [1.0, 1.0],
            roles: [rl, rr],
        })
    }
}
