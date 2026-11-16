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
    /// The parts it plays in the series' kit configs ("bass", "riff"), each once; empty when the app knows none.
    pub parts: Vec<String>,
    /// Its traits for scoring themes ("brassy", "low", "folk"); empty when the app knows none.
    pub traits: Vec<String>,
    /// 1 heavy and stately to 3 fast and bright, for the recommended tempo; 0 when the app knows none.
    pub energy: u8,
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
    /// Its key's note name and its scale.
    pub key: String,
    pub scale: String,
    /// Each fighter's part as it writes it, left then right of the draft it was described for.
    pub roles: [String; 2],
    /// The theme options it sets, by name; an option it leaves out is at the theme's default.
    pub options: Vec<(String, OptionSet)>,
    /// Its arrangement (`classic` or `composed`; empty reads as `classic`).
    pub arrangement: String,
    /// The fighters whose lines it keeps as written under Composed.
    pub pinned: Vec<String>,
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
/// `known` are the parts the fighter plays in the series' kit configs: a part it has no music of its own for (the
/// TUBA given the TROMBONE's riff by a bass swap) is played as its top line, so it reads "the top line". The melody
/// and the bass are every part's own; with nothing known (the sample roster) the part is named as it is.
pub fn part_words(role: &str, known: &[String]) -> String {
    let own = known.is_empty() || matches!(role, "melody" | "bass" | "") || known.iter().any(|k| k == role);
    match role {
        _ if !own => "the top line".to_string(),
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
    /// The key and scale each chord loop comes with (those of the kit config the theme ships it in, else the
    /// theme's own), in the order of `chord_loops`.
    pub loop_keys: Vec<(String, String)>,
    /// The tempo range as bars that fill 61 s, lowest and highest.
    pub bars: (u32, u32),
    /// The roster fighters it cannot be played with (roster names): each one's part plays a hit its music lacks, so
    /// the kit would crash on any pair with it (ruling M3-R2). Empty for most themes.
    pub refuses: Vec<String>,
    /// Its options (the trailer's sequel, the drumline's bass drums, the plugged fighter): none for most themes.
    pub options: Vec<ThemeOptionEntry>,
    /// The theme picker's group it sits in ("Jazz & Latin"); empty puts it under "Other".
    pub category: String,
    /// The fighter traits it likes, each weighted 1 or 2.
    pub likes: Vec<(String, u8)>,
    /// The bpm range its recommended tempo is placed in; `(0.0, 0.0)` when the app knows none.
    pub tempo: (f64, f64),
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
    /// The pairs that start from an older episode's config, one entry for each order (handicap 1.0 each).
    pub episode_pairs: Vec<KitPair>,
}

/// The theme picker's groups, in order; a theme with a category not among them is listed under [`OTHER`].
pub use bf_content::fit::CATEGORIES;
/// The last group: themes with no known category.
pub const OTHER: &str = "Other";
/// How many themes the picker recommends.
pub const RECOMMENDED: usize = 3;
/// What a fighter's home theme adds to a theme's score: enough to put it among the recommended.
const HOME_BONUS: u32 = 100;

impl Roster {
    /// The themes the pair `fighters` may be set in, grouped as the picker lists them: [`CATEGORIES`] in order, then
    /// [`OTHER`], each with its themes in the roster's order. Groups with no theme are left out.
    pub fn grouped(&self, fighters: &[String; 2]) -> Vec<(String, Vec<&ThemeEntry>)> {
        let themes = self.themes_for(fighters);
        fn group_of(t: &ThemeEntry) -> &str {
            if CATEGORIES.contains(&t.category.as_str()) {
                t.category.as_str()
            } else {
                OTHER
            }
        }
        CATEGORIES
            .iter()
            .chain(std::iter::once(&OTHER))
            .map(|&g| {
                (
                    g.to_string(),
                    themes.iter().copied().filter(|t| group_of(t) == g).collect::<Vec<_>>(),
                )
            })
            .filter(|(_, ts)| !ts.is_empty())
            .collect()
    }

    /// The themes recommended for `fighters`, best first: the [`RECOMMENDED`] best scores among the themes the pair
    /// may be set in. A fighter's fit for a theme is the sum of the weights of the traits it has that the theme likes;
    /// a theme scores twice the weaker fighter's fit plus the stronger's, and [`HOME_BONUS`] for each fighter whose
    /// home theme it is. Ties keep the picker's order ([`Roster::grouped`]).
    pub fn recommended(&self, fighters: &[String; 2]) -> Vec<&ThemeEntry> {
        let entry = |name: &str| self.fighters.iter().find(|f| f.name == name);
        let pair = [entry(&fighters[0]), entry(&fighters[1])];
        let score = |t: &ThemeEntry| {
            let fit = |f: Option<&RosterEntry>| -> u32 {
                f.map_or(0, |f| {
                    t.likes
                        .iter()
                        .filter(|(trait_, _)| f.traits.contains(trait_))
                        .map(|(_, w)| u32::from(*w))
                        .sum()
                })
            };
            let (a, b) = (fit(pair[0]), fit(pair[1]));
            let home = pair
                .iter()
                .flatten()
                .filter(|f| f.home.as_ref().is_some_and(|h| h.theme == t.name))
                .count() as u32;
            2 * a.min(b) + a.max(b) + HOME_BONUS * home
        };
        let mut ordered: Vec<&ThemeEntry> = self.grouped(fighters).into_iter().flat_map(|(_, ts)| ts).collect();
        // a stable sort: ties keep the picker's order
        ordered.sort_by_key(|t| std::cmp::Reverse(score(t)));
        ordered.truncate(RECOMMENDED);
        ordered
    }

    /// The tempo recommended for `fighters` in the theme `theme`, in bpm: the theme's tempo range, from its low end
    /// for a pair of energy 1 to its high end for a pair of energy 3 (the two energies averaged). `None` when the
    /// theme is unknown or has no range; a fighter of unknown energy counts as 2.
    pub fn recommended_bpm(&self, fighters: &[String; 2], theme: &str) -> Option<f64> {
        let t = self.themes.iter().find(|t| t.name == theme)?;
        let (lo, hi) = t.tempo;
        if !(lo.is_finite() && hi.is_finite() && lo > 0.0 && lo < hi) {
            return None;
        }
        let energy = |name: &str| {
            self.fighters
                .iter()
                .find(|f| f.name == name)
                .filter(|f| (1..=3).contains(&f.energy))
                .map_or(2.0, |f| f64::from(f.energy))
        };
        let avg = f64::midpoint(energy(&fighters[0]), energy(&fighters[1]));
        Some(lo + (avg - 1.0) / 2.0 * (hi - lo))
    }

    /// The themes the pair `fighters` (left corner first) may be set in, in the order they are offered: those that
    /// refuse neither fighter (ruling M3-R2). The pickers list only these.
    pub fn themes_for(&self, fighters: &[String; 2]) -> Vec<&ThemeEntry> {
        self.themes
            .iter()
            .filter(|t| !fighters.iter().any(|f| t.refuses.contains(f)))
            .collect()
    }

    /// The series' frozen handicap of `fighters` (ruling M3-R6), left then right: their kit config's, swapped for the
    /// other order; `None` for a pair with no kit config.
    pub fn series_handicap(&self, fighters: &[String; 2]) -> Option<[f64; 2]> {
        let swapped = [fighters[1].clone(), fighters[0].clone()];
        if let Some(kit) = self.kit_pairs.iter().find(|k| k.fighters == *fighters) {
            return Some(kit.handicap);
        }
        let kit = self.kit_pairs.iter().find(|k| k.fighters == swapped)?;
        Some([kit.handicap[1], kit.handicap[0]])
    }

    /// Whether `fighters`, in this order, have a kit config of their own (HORN against TROMBONE has one for each
    /// order).
    pub fn has_own_config(&self, fighters: &[String; 2]) -> bool {
        self.kit_pairs.iter().any(|k| k.fighters == *fighters)
    }

    /// How a new match-up of `fighters` starts, as the engine's starter makes it (`bf_content::starter`): a kit
    /// pair from its kit config (the other order swaps the handicap and the parts); a pair with an older episode's
    /// config from that config ([`Roster::episode_pairs`], each order its own); any other pair from the left
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
        if let Some(ep) = self.episode_pairs.iter().find(|k| k.fighters == *fighters) {
            return Some(PairStart {
                chord_loop: index(&ep.theme, &ep.chord_loop),
                theme: ep.theme.clone(),
                handicap: ep.handicap,
                roles: ep.roles.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The sample roster with five themes: `(name, category, likes)`, a range of 100 to 140 bpm each.
    /// A test theme: its name, category and likes.
    type Spec<'a> = (&'a str, &'a str, &'a [(&'a str, u8)]);

    fn roster(themes: &[Spec<'_>]) -> Roster {
        let mut r = crate::seam::sample::roster();
        let base = r.themes[0].clone();
        r.themes = themes
            .iter()
            .map(|(name, category, likes)| ThemeEntry {
                name: name.to_string(),
                display: name.to_string(),
                category: category.to_string(),
                likes: likes.iter().map(|(t, w)| (t.to_string(), *w)).collect(),
                tempo: (100.0, 140.0),
                ..base.clone()
            })
            .collect();
        let set = |r: &mut Roster, name: &str, traits: &[&str], energy: u8| {
            let f = r.fighters.iter_mut().find(|f| f.name == name).unwrap();
            f.traits = traits.iter().map(|t| t.to_string()).collect();
            f.energy = energy;
        };
        set(&mut r, "FLUTE", &["airy", "high", "folk"], 3);
        set(&mut r, "TUBA", &["brassy", "low", "folk"], 1);
        r
    }

    fn pair() -> [String; 2] {
        ["FLUTE".to_string(), "TUBA".to_string()]
    }

    fn five() -> Roster {
        roster(&[
            ("a", "Rock & Punk", &[("rock", 2)]),
            ("b", "Jazz & Latin", &[("brassy", 2)]),
            ("c", "Folk & World", &[("folk", 2)]),
            ("d", "", &[("airy", 2), ("brassy", 2)]),
            ("e", "Jazz & Latin", &[("high", 1)]),
        ])
    }

    fn names(ts: &[&ThemeEntry]) -> Vec<String> {
        ts.iter().map(|t| t.name.clone()).collect()
    }

    #[test]
    fn groups_follow_the_category_order_with_other_last() {
        let r = five();
        let g: Vec<(String, Vec<String>)> = r.grouped(&pair()).into_iter().map(|(g, ts)| (g, names(&ts))).collect();
        let want = [
            ("Jazz & Latin", vec!["b", "e"]),
            ("Folk & World", vec!["c"]),
            ("Rock & Punk", vec!["a"]),
            ("Other", vec!["d"]),
        ];
        assert_eq!(
            g,
            want.map(|(g, ts)| (g.to_string(), ts.into_iter().map(String::from).collect()))
        );
    }

    #[test]
    fn three_suiting_both_are_recommended() {
        // c: both fit 2 → 6; d: 2 and 2 → 6; b: 0 and 2 → 2; e: 1 and 0 → 1; a: 0
        assert_eq!(names(&five().recommended(&pair())), ["c", "d", "b"]);
    }

    #[test]
    fn a_home_theme_is_always_recommended() {
        let mut r = five();
        r.fighters.iter_mut().find(|f| f.name == "TUBA").unwrap().home = Some(Home {
            theme: "a".to_string(),
            chord_loop: Vec::new(),
            role: "bass".to_string(),
        });
        assert_eq!(names(&r.recommended(&pair()))[0], "a");
    }

    #[test]
    fn only_playable_themes_are_recommended() {
        let mut r = five();
        for t in &mut r.themes[1..] {
            t.refuses = vec!["TUBA".to_string()];
        }
        assert_eq!(names(&r.recommended(&pair())), ["a"]);
    }

    #[test]
    fn no_shared_traits_still_gives_three_in_picker_order() {
        let r = five();
        let strangers = ["OBOE".to_string(), "HARP".to_string()];
        assert_eq!(names(&r.recommended(&strangers)), ["b", "e", "c"]);
    }

    #[test]
    fn the_recommended_tempo_follows_the_energy() {
        let mut r = five();
        assert_eq!(r.recommended_bpm(&pair(), "a"), Some(120.0));
        let both = |r: &mut Roster, e: u8| {
            for f in &mut r.fighters {
                f.energy = e;
            }
        };
        both(&mut r, 1);
        assert_eq!(r.recommended_bpm(&pair(), "a"), Some(100.0));
        both(&mut r, 3);
        assert_eq!(r.recommended_bpm(&pair(), "a"), Some(140.0));
        assert_eq!(r.recommended_bpm(&pair(), "nope"), None);
        r.themes[0].tempo = (0.0, 0.0);
        assert_eq!(r.recommended_bpm(&pair(), "a"), None);
    }
}
