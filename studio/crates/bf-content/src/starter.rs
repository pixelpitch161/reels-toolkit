//! The fight a new match-up begins from, and the caption drafts (Task 26, milestone 3 Task 59; spec sections 3, 4
//! and 9).
//!
//! The fight file is the whole recipe, so a match-up starts from a written one. A kit pair (one of the series' kit
//! configs, written as `studio/fights/<pair>.toml`) starts from its fight file with the kit's frozen handicap and
//! damage scale (plan section 8, Q6; ruling M3-R6), swapped with the sides for the other order. Any other pair of two
//! kit fighters starts from the left fighter's home config: its theme, tempo, key, scale, chords and options, the
//! kit's sketch lines and solos, roles from both homes, a neutral handicap (ruling M3-R7). Both are configs the kit
//! itself plays, and `studio/reference/episodes/crossovers/` proves six of the second kind against the kit.

use std::sync::OnceLock;

use bf_core::fightfile::{Captions, FightFile, OptionValue};

use crate::themes::{PORTED_THEMES, themes_for};
use crate::{ContentError, available, card, pair_rules};

/// The series' kit configs as fight files (plan Table S4 and milestone 1's `flute_vs_trombone`): the pair's folder
/// name, the episode number of the kit config (the first line of its notes), whether it is a kit example (a config
/// rebuilt to check a port, which is a fighter's home only when it has no other) and the fight file.
const KIT_PAIRS: [(&str, u32, bool, &str); 18] = [
    (
        "altosax_vs_tenorsax",
        51,
        false,
        include_str!("../../../fights/altosax_vs_tenorsax.toml"),
    ),
    (
        "bassclarinet_vs_bassoon",
        55,
        false,
        include_str!("../../../fights/bassclarinet_vs_bassoon.toml"),
    ),
    (
        "bassclarinet_vs_euphonium",
        38,
        true,
        include_str!("../../../fights/bassclarinet_vs_euphonium.toml"),
    ),
    (
        "bassdrum_vs_tuba",
        56,
        false,
        include_str!("../../../fights/bassdrum_vs_tuba.toml"),
    ),
    (
        "cello_vs_contrabass",
        47,
        false,
        include_str!("../../../fights/cello_vs_contrabass.toml"),
    ),
    (
        "clarinet_vs_oboe",
        37,
        true,
        include_str!("../../../fights/clarinet_vs_oboe.toml"),
    ),
    (
        "electric_vs_acoustic",
        44,
        false,
        include_str!("../../../fights/electric_vs_acoustic.toml"),
    ),
    (
        "euphonium_vs_trombone",
        48,
        false,
        include_str!("../../../fights/euphonium_vs_trombone.toml"),
    ),
    (
        "flute_vs_clarinet",
        49,
        false,
        include_str!("../../../fights/flute_vs_clarinet.toml"),
    ),
    (
        "flute_vs_sousaphone",
        53,
        false,
        include_str!("../../../fights/flute_vs_sousaphone.toml"),
    ),
    (
        "flute_vs_trombone",
        41,
        true,
        include_str!("../../../fights/flute_vs_trombone.toml"),
    ),
    (
        "horn_vs_trombone_rematch",
        54,
        false,
        include_str!("../../../fights/horn_vs_trombone_rematch.toml"),
    ),
    (
        "oboe_vs_bassoon",
        52,
        false,
        include_str!("../../../fights/oboe_vs_bassoon.toml"),
    ),
    (
        "piano_vs_organ",
        45,
        false,
        include_str!("../../../fights/piano_vs_organ.toml"),
    ),
    (
        "trombone_vs_horn",
        34,
        true,
        include_str!("../../../fights/trombone_vs_horn.toml"),
    ),
    (
        "trombone_vs_tuba",
        36,
        true,
        include_str!("../../../fights/trombone_vs_tuba.toml"),
    ),
    (
        "trumpet_vs_clarinet",
        46,
        false,
        include_str!("../../../fights/trumpet_vs_clarinet.toml"),
    ),
    (
        "trumpet_vs_trombone",
        50,
        false,
        include_str!("../../../fights/trumpet_vs_trombone.toml"),
    ),
];

/// The theme a crossover falls back to when neither fighter's home theme has every bed hook both parts play.
const FALLBACK_THEME: &str = "big_top";

/// A kit config as a fight file: its pair's folder name, episode number, example flag and fight.
struct KitPair {
    pair: &'static str,
    episode: u32,
    example: bool,
    ff: FightFile,
}

fn kit_pairs() -> &'static [KitPair] {
    static PAIRS: OnceLock<Vec<KitPair>> = OnceLock::new();
    PAIRS.get_or_init(|| {
        KIT_PAIRS
            .iter()
            .map(|&(pair, episode, example, text)| KitPair {
                pair,
                episode,
                example,
                ff: FightFile::parse(text).unwrap_or_else(|e| panic!("studio/fights/{pair}.toml: {e}")),
            })
            .collect()
    })
}

/// The kit configs the pair gates prove, as `(pair folder name, fight file)`: every pair with written music.
pub fn kit_pair_fights() -> Vec<(&'static str, &'static FightFile)> {
    kit_pairs().iter().map(|p| (p.pair, &p.ff)).collect()
}

/// A fighter's home config (the music a crossover with it on the left starts from), by its pair folder name: its
/// newest kit config by episode number, an example only when it has no other. `None` for a name not offered that no
/// kit config has.
///
/// An offered fighter without a kit config of its own (an older fighter) starts from its own episode's config (the
/// newest one whose theme is offered); one with none offered yet borrows FLUTE's and plays the melody there.
pub fn home_config(name: &str) -> Option<&'static str> {
    own_home_config(name).or_else(|| {
        if available().contains(&name) {
            own_episode(name).map(|e| e.pair).or_else(|| own_home_config("FLUTE"))
        } else {
            None
        }
    })
}

/// A fighter's home config ([`home_config`]) as its fight file.
pub fn home_fight(name: &str) -> Option<&'static FightFile> {
    home(name)
}

/// An older episode's music as a kit config (plan m4 Task 5's `tools/studio_export/older/configs/<pair>/episode.py`):
/// its pair folder name, episode number, fighters, theme, tempo, key, scale, intro, loop, tail and the roles it names.
/// A config whose `CHORDS` no single loop gives (ORGAN vs HARP's three sections) keeps its first section's loop and
/// its last two bars as the tail.
struct Episode {
    pair: &'static str,
    episode: u32,
    fighters: [&'static str; 2],
    theme: &'static str,
    bpm: f64,
    key: &'static str,
    scale: &'static str,
    intro: [&'static str; 2],
    chord_loop: &'static [&'static str],
    tail: &'static [&'static str],
    roles: &'static [(&'static str, &'static str)],
}

/// Shorthand for an [`Episode`] row.
#[allow(clippy::too_many_arguments)]
const fn ep(
    pair: &'static str,
    episode: u32,
    fighters: [&'static str; 2],
    theme: &'static str,
    bpm: f64,
    (key, scale): (&'static str, &'static str),
    intro: [&'static str; 2],
    chord_loop: &'static [&'static str],
    tail: &'static [&'static str],
    roles: &'static [(&'static str, &'static str)],
) -> Episode {
    Episode {
        pair,
        episode,
        fighters,
        theme,
        bpm,
        key,
        scale,
        intro,
        chord_loop,
        tail,
        roles,
    }
}

/// The older episodes' configs (TROMBONE vs TUBA's #36 is left out: that pair keeps its kit config).
const EPISODES: [Episode; 10] = [
    ep(
        "drums_vs_violin",
        28,
        ["DRUMS", "VIOLIN"],
        "balkan_fire",
        140.0,
        ("D", "harmonic minor"),
        ["Dm", "A"],
        &["Dm", "Dm", "Gm", "A"],
        &["Gm", "A"],
        &[],
    ),
    ep(
        "bagpipes_vs_accordion",
        30,
        ["BAGPIPES", "ACCORDION"],
        "sea_shanty",
        122.0,
        ("A", "dorian"),
        ["Am", "E"],
        &["Am", "G", "Am", "Em"],
        &["E"],
        &[],
    ),
    ep(
        "turntables_vs_synth",
        31,
        ["TURNTABLES", "SYNTH"],
        "dark_breakbeat",
        138.0,
        ("F", "minor"),
        ["Fm", "C"],
        &["Fm", "Db", "Bbm", "C"],
        &["C"],
        &[],
    ),
    ep(
        "tuba_vs_flute",
        32,
        ["TUBA", "FLUTE"],
        "villain_march",
        126.0,
        ("C", "harmonic minor"),
        ["Cm", "G"],
        &["Cm", "Ab", "Db", "G"],
        &["Ab", "G"],
        &[("TUBA", "bass"), ("FLUTE", "melody")],
    ),
    ep(
        "ukulele_vs_acoustic",
        33,
        ["UKULELE", "ACOUSTIC"],
        "spaghetti_western",
        130.0,
        ("A", "minor"),
        ["Am", "E"],
        &["Am", "G", "F", "E"],
        &["Am", "F", "E"],
        &[("UKULELE", "melody"), ("ACOUSTIC", "rhythm")],
    ),
    ep(
        "organ_vs_harp",
        35,
        ["ORGAN", "HARP"],
        "gothic_horror",
        126.0,
        ("D", "minor"),
        ["Dm", "A"],
        &["Dm", "Dm", "Bb", "A"],
        &["Dm", "A"],
        &[],
    ),
    ep(
        "bassoon_vs_barisax",
        39,
        ["BASSOON", "BARI SAX"],
        "ska_punk",
        170.0,
        ("E", "minor"),
        ["Em", "B"],
        &["Em", "C", "D", "B"],
        &["B"],
        &[("BASSOON", "melody"), ("BARI SAX", "bass")],
    ),
    ep(
        "barisax_vs_bassclarinet",
        40,
        ["BARI SAX", "BASS CLARINET"],
        "industrial",
        126.0,
        ("G", "minor"),
        ["Gm", "D"],
        &["Gm", "Eb", "Cm", "D"],
        &["Eb", "D"],
        &[("BARI SAX", "bass"), ("BASS CLARINET", "counter")],
    ),
    // CONTRABASSOON's home config: a fighter made from scratch has no episode (number 0); its pipes suit
    // `industrial`, BASSOON is its natural rival.
    ep(
        "contrabassoon_vs_bassoon",
        0,
        ["CONTRABASSOON", "BASSOON"],
        "industrial",
        128.0,
        ("G", "minor"),
        ["Gm", "D"],
        &["Gm", "Gm", "Eb", "D"],
        &["Eb", "D"],
        &[("CONTRABASSOON", "bass"), ("BASSOON", "melody")],
    ),
    ep(
        "viola_vs_violin",
        42,
        ["VIOLA", "VIOLIN"],
        "orchestral_drill",
        140.0,
        ("C", "harmonic minor"),
        ["Cm", "G"],
        &["Cm", "Cm", "Ab", "Ab", "Fm", "Fm", "G", "G"],
        &["Ab", "G"],
        &[],
    ),
];

/// An older episode config as a fight file: its pair folder name, episode number and fight.
struct EpisodeFight {
    pair: &'static str,
    episode: u32,
    ff: FightFile,
}

/// The older episode configs whose theme is offered ([`PORTED_THEMES`]), as fight files: FLUTE's home config (what
/// an older fighter borrowed before) with the episode's fighters, theme, tempo, key, scale, chords and roles, the
/// kit's sketch lines and solos, no title hits, no mix, no title-card opt-ins and a neutral handicap.
fn episodes() -> &'static [EpisodeFight] {
    static FIGHTS: OnceLock<Vec<EpisodeFight>> = OnceLock::new();
    FIGHTS.get_or_init(|| {
        let flute = own_home_config("FLUTE")
            .and_then(|pair| kit_pairs().iter().find(|p| p.pair == pair))
            .map(|p| &p.ff)
            .expect("FLUTE's home config");
        let texts = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        EPISODES
            .iter()
            .filter(|e| PORTED_THEMES.contains(&e.theme))
            .map(|e| {
                let mut ff = flute.clone();
                ff.fighters = texts(&e.fighters);
                ff.handicap = vec![1.0, 1.0];
                ff.dmg_scale = 1.0;
                ff.tempo_bpm = e.bpm;
                let m = &mut ff.music;
                m.theme = e.theme.to_string();
                m.theme_options.clear();
                m.key = e.key.to_string();
                m.scale = e.scale.to_string();
                m.chords_intro = texts(&e.intro);
                m.chord_loop = texts(e.chord_loop);
                m.chords_tail = texts(e.tail);
                m.roles = e.roles.iter().map(|&(f, r)| (f.to_string(), r.to_string())).collect();
                m.lines.clear();
                m.solos.clear();
                m.title_hits.clear();
                m.mix.clear();
                let p = &mut ff.picture;
                p.title_lines.clear();
                p.title_balls = None;
                p.middle = None;
                EpisodeFight {
                    pair: e.pair,
                    episode: e.episode,
                    ff,
                }
            })
            .collect()
    })
}

/// The pairs that start from an older episode config (its theme offered), as written: left then right.
pub fn episode_pairs() -> Vec<[&'static str; 2]> {
    episodes()
        .iter()
        .filter_map(|e| {
            let [a, b] = [e.ff.fighters.first()?, e.ff.fighters.get(1)?];
            let names = available();
            let a = names.iter().copied().find(|n| n == a)?;
            let b = names.iter().copied().find(|n| n == b)?;
            episode_pair(a, b).map(|_| [a, b])
        })
        .collect()
}

/// An older fighter's own episode config: its newest one by episode number whose theme is offered.
fn own_episode(name: &str) -> Option<&'static EpisodeFight> {
    episodes()
        .iter()
        .filter(|e| e.ff.fighters.iter().any(|f| f == name))
        .max_by_key(|e| e.episode)
}

fn own_home_config(name: &str) -> Option<&'static str> {
    kit_pairs()
        .iter()
        .filter(|p| p.ff.fighters.iter().any(|f| f == name))
        .max_by_key(|p| (!p.example, p.episode))
        .map(|p| p.pair)
}

fn home(name: &str) -> Option<&'static FightFile> {
    let pair = home_config(name)?;
    kit_pairs()
        .iter()
        .find(|p| p.pair == pair)
        .map(|p| &p.ff)
        .or_else(|| episodes().iter().find(|e| e.pair == pair).map(|e| &e.ff))
}

/// The tags every Instagram draft ends with.
const INSTAGRAM_TAGS: &str = "#musicmemes #musician #pixelart #lowpoly #gamedev #satisfying #music";

/// The tags every TikTok draft ends with.
const TIKTOK_TAGS: &str = "#bandkid #musictok #pixelart #satisfying #bandtok #music";

/// The tags every Shorts draft ends with.
const SHORTS_TAGS: &str = "#music #pixelart";

/// The fight a new match-up of `left` (the left corner) and `right` begins from, for any ordered pair of two kit
/// fighters ([`available`]):
///
/// - a kit pair starts from its fight file (for HORN and TROMBONE, which have two, the one whose left fighter is
///   `left`); the other order swaps the fighters and the handicap, and every field that names a fighter keeps its
///   name. The handicap and damage scale are the kit's frozen `DMG_SIDE` and `DMG_SCALE` (the series' handicap);
/// - any other pair starts from the left fighter's home config ([`home_config`]): its theme (or, when the theme lacks
///   a bed hook a part plays, the right fighter's home theme, else the big top), tempo, key, scale, chords and that
///   theme's options (one that names a fighter not in the pair names the left fighter); both play the kit's sketch
///   line and solo, no title hits, no mix, no title-card opt-ins; the left keeps its home role, the right takes its
///   own unless that is the same, then `bass` if its card can play bass and the left's is not `bass`, else `melody`
///   unless the left plays the melody, else `line`; handicap 1.0 each, damage scale 1.0. Such a new pairing starts
///   on the `composed` arrangement; a kit pair and an older episode's pair keep `classic`.
///
/// Either way: no seed, no series tag, the kit's default particle seed, the pair's rules, drafted captions and this
/// app's version.
///
/// # Errors
/// For two names that are not two different kit fighters, [`ContentError::NoMusic`], which reads "No music is
/// written for {Left} vs {Right} yet." with both names in sentence case.
pub fn starter(left: &str, right: &str) -> Result<FightFile, ContentError> {
    let offered = available();
    if left == right || !offered.contains(&left) || !offered.contains(&right) {
        return Err(no_music(left, right));
    }
    let written = |a: &str, b: &str| kit_pairs().iter().find(|p| p.ff.fighters == [a, b]);
    let mut ff = if let Some(p) = written(left, right) {
        p.ff.clone()
    } else if let Some(p) = written(right, left) {
        let mut ff = p.ff.clone();
        ff.fighters.reverse();
        ff.handicap.reverse();
        ff
    } else if let Some(ff) = episode_pair(left, right) {
        ff
    } else {
        crossover(left, right)?
    };
    ff.app_version = env!("CARGO_PKG_VERSION").to_string();
    ff.seed = None;
    ff.series_tag = None;
    ff.picture.fx_seed = None;
    ff.pair_rules = pair_rules(left, right);
    ff.captions = draft_captions(left, right);
    Ok(ff)
}

fn no_music(left: &str, right: &str) -> ContentError {
    ContentError::NoMusic {
        left: sentence_case(left),
        right: sentence_case(right),
    }
}

/// The fight file of a pair without a kit config (see [`starter`]), before what every starter sets.
fn crossover(left: &str, right: &str) -> Result<FightFile, ContentError> {
    let (Some(hl), Some(hr)) = (home(left), home(right)) else {
        return Err(no_music(left, right));
    };
    let offered = themes_for(left, right);
    let theme = [hl.music.theme.as_str(), hr.music.theme.as_str()]
        .into_iter()
        .find(|t| offered.contains(t))
        .unwrap_or(FALLBACK_THEME);
    let (Some(rl), Some(rr)) = (role_of(hl, left), role_of(hr, right)) else {
        return Err(no_music(left, right));
    };
    let mut ff = arrange(hl.clone(), left, right, theme, rl, rr);
    // a new pairing starts on the composed arrangement (milestone 5); Classic is one click away
    ff.music.arrangement = crate::themes::COMPOSED.to_string();
    Ok(ff)
}

/// `name`'s part in its home config `h`: the one it names, else the melody for a fighter without a kit config.
fn role_of(h: &FightFile, name: &str) -> Option<String> {
    match h.music.roles.get(name) {
        Some(role) => Some(role.clone()),
        None if own_home_config(name).is_none() => Some("melody".to_string()),
        None => None,
    }
}

/// The fight file of a pair with an older episode config (in either order) whose theme the pair is offered: that
/// config's music with the sides as given, each fighter's part the one the config names, else its home part (the
/// same clash rule as a crossover). `None` for any other pair.
fn episode_pair(left: &str, right: &str) -> Option<FightFile> {
    let e = episodes()
        .iter()
        .find(|e| e.ff.fighters == [left, right] || e.ff.fighters == [right, left])?;
    let theme = e.ff.music.theme.as_str();
    if !themes_for(left, right).contains(&theme) {
        return None;
    }
    let part = |name: &str| {
        e.ff.music
            .roles
            .get(name)
            .cloned()
            .or_else(|| role_of(home(name)?, name))
    };
    Some(arrange(e.ff.clone(), left, right, theme, part(left)?, part(right)?))
}

/// `ff` (a home or episode config) as `left` vs `right` on `theme` with the parts `rl` and `rr` (see [`starter`]).
fn arrange(mut ff: FightFile, left: &str, right: &str, theme: &str, rl: String, mut rr: String) -> FightFile {
    if rr == rl {
        rr = if card(right).is_some_and(|c| c.can_play_bass) && rl != "bass" {
            "bass"
        } else if rl != "melody" {
            "melody"
        } else {
            "line"
        }
        .to_string();
    }
    ff.fighters = vec![left.to_string(), right.to_string()];
    ff.handicap = vec![1.0, 1.0];
    ff.dmg_scale = 1.0;
    let m = &mut ff.music;
    if theme != m.theme {
        m.theme = theme.to_string();
        m.theme_options.clear();
    }
    for v in m.theme_options.values_mut() {
        if let OptionValue::Str(name) = v
            && name != left
            && name != right
        {
            *name = left.to_string();
        }
    }
    m.roles = [(left.to_string(), rl), (right.to_string(), rr)].into_iter().collect();
    m.lines.clear();
    m.solos.clear();
    m.title_hits.clear();
    m.mix.clear();
    let p = &mut ff.picture;
    p.title_lines.clear();
    p.title_balls = None;
    p.middle = None;
    ff
}

/// The post texts drafted for `left` vs `right` from the fighters' emoji and tags (spec section 9). A fighter
/// without a card adds no emoji and no tags of its own.
pub fn draft_captions(left: &str, right: &str) -> Captions {
    let (l, r) = (sentence_case(left), sentence_case(right));
    let (lt, rt) = (name_tag(left), name_tag(right));
    let emoji: String = [left, right].iter().filter_map(|n| card(n)).map(|c| c.emoji).collect();
    let mut own: Vec<&str> = Vec::new();
    for tag in [left, right]
        .iter()
        .filter_map(|n| card(n))
        .flat_map(|c| c.tags.iter().copied())
    {
        if !own.contains(&tag) {
            own.push(tag);
        }
    }
    let mut instagram = format!("#ballfight {lt} {rt}");
    for tag in own {
        instagram.push(' ');
        instagram.push_str(tag);
    }
    instagram.push(' ');
    instagram.push_str(INSTAGRAM_TAGS);
    let mut caption = format!("{l} vs {r} ");
    if !emoji.is_empty() {
        caption.push_str(&emoji);
        caption.push(' ');
    }
    caption.push_str("Comment the next match-up \u{1F447}");
    Captions {
        caption,
        instagram,
        tiktok: format!("#fyp #ballfight {lt} {rt} {TIKTOK_TAGS}"),
        shorts: vec![
            format!("{l} vs {r}"),
            format!("{l} vs {r}, every hit in key. Comment the next match-up"),
            format!("#shorts #ballfight {lt} {rt} {SHORTS_TAGS}"),
        ],
    }
}

/// A roster name in sentence case: "BASS DRUM" is "Bass drum".
fn sentence_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect(),
        None => String::new(),
    }
}

/// A fighter's name tag: "#" and the name in lower case without spaces ("ALTO SAX" is "#altosax").
fn name_tag(name: &str) -> String {
    let mut tag = String::from("#");
    tag.extend(name.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase));
    tag
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_core::event::Value;

    #[test]
    fn the_starter_is_a_valid_fight() {
        // the series' handicap (the kit's DMG_SIDE), with the sides
        for (left, right, handicap) in [("FLUTE", "TROMBONE", [2.0, 0.4]), ("TROMBONE", "FLUTE", [0.4, 2.0])] {
            let ff = starter(left, right).unwrap_or_else(|e| panic!("{left} vs {right}: {e}"));
            assert_eq!(crate::validate(&ff), Ok(()), "{left} vs {right}");
            assert_eq!(ff.fighters, [left, right]);
            assert_eq!(ff.handicap, handicap);
            assert_eq!(ff.seed, None);
            assert_eq!(ff.series_tag, None);
            assert_eq!(ff.picture.fx_seed, None);
            assert_eq!(ff.pair_rules.get("swat_cobras"), Some(&Value::Bool(true)));
            assert_eq!(ff.pair_rules, pair_rules(left, right));
            assert_eq!(ff.captions, draft_captions(left, right));
            assert_eq!(ff.app_version, env!("CARGO_PKG_VERSION"));
            // a written file that reads back as the same fight
            assert_eq!(FightFile::parse(&ff.to_toml()).as_ref(), Ok(&ff));
        }
    }

    /// A new pairing starts on Composed; a kit pair keeps Classic. Both read back as written.
    #[test]
    fn a_new_pairing_starts_composed() {
        let kit = starter("FLUTE", "TROMBONE").unwrap();
        assert_eq!(kit.music.arrangement, "classic");
        let new = starter("FLUTE", "PIANO").unwrap();
        assert_eq!(new.music.arrangement, "composed");
        assert_eq!(crate::validate(&new), Ok(()));
        assert_eq!(FightFile::parse(&new.to_toml()).as_ref(), Ok(&new));
    }

    #[test]
    fn the_two_orders_are_different_fights() {
        let a = starter("FLUTE", "TROMBONE").expect("flute vs trombone");
        let b = starter("TROMBONE", "FLUTE").expect("trombone vs flute");
        assert_ne!(a.search_key(&[false, false]), b.search_key(&[false, false]));
    }

    #[test]
    fn a_pair_that_is_no_kit_pair_says_so() {
        let message = |left: &str, right: &str| match starter(left, right) {
            Err(e @ ContentError::NoMusic { .. }) => e.to_string(),
            other => panic!("{left} vs {right}: {other:?}"),
        };
        // every pair of two different kit fighters has a starter; a mirror match or a name that is no kit fighter
        // has none
        assert!(starter("FLUTE", "PIANO").is_ok() && starter("BASS DRUM", "ALTO SAX").is_ok());
        assert_eq!(message("FLUTE", "KAZOO"), "No music is written for Flute vs Kazoo yet.");
        assert_eq!(
            starter("KAZOO", "PIANO").unwrap_err(),
            ContentError::NoMusic {
                left: "Kazoo".into(),
                right: "Piano".into()
            }
        );
        assert_eq!(
            message("BASS DRUM", "BASS DRUM"),
            "No music is written for Bass drum vs Bass drum yet."
        );
        assert_eq!(message("FLUTE", "FLUTE"), "No music is written for Flute vs Flute yet.");
        assert_eq!(message("", ""), "No music is written for  vs  yet.");
        assert_eq!(
            message("flute", "trombone"),
            "No music is written for Flute vs Trombone yet."
        );
    }

    #[test]
    fn captions_are_drafted_from_emoji_and_tags() {
        let c = draft_captions("FLUTE", "TROMBONE");
        assert_eq!(
            c.caption,
            "Flute vs Trombone \u{1FA88}\u{1F3BA} Comment the next match-up \u{1F447}"
        );
        assert_eq!(
            c.instagram,
            "#ballfight #flute #trombone #bandkids #marchingband #musicmemes #musician #pixelart #lowpoly #gamedev \
             #satisfying #music"
        );
        assert_eq!(
            c.tiktok,
            "#fyp #ballfight #flute #trombone #bandkid #musictok #pixelart #satisfying #bandtok #music"
        );
        assert_eq!(
            c.shorts,
            [
                "Flute vs Trombone",
                "Flute vs Trombone, every hit in key. Comment the next match-up",
                "#shorts #ballfight #flute #trombone #music #pixelart"
            ]
        );
        // the other order swaps the names and the emoji
        let r = draft_captions("TROMBONE", "FLUTE");
        assert_eq!(
            r.caption,
            "Trombone vs Flute \u{1F3BA}\u{1FA88} Comment the next match-up \u{1F447}"
        );
        assert!(
            r.instagram
                .starts_with("#ballfight #trombone #flute #bandkids #marchingband #musicmemes")
        );
    }

    #[test]
    fn captions_for_fighters_without_cards() {
        // the two names are found at run time (roster fighters with no card yet), so a port never breaks this test;
        // once fewer than two are left, names the registry does not know take the same path (no card either)
        let mut names: Vec<&str> = crate::roster()
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| card(n).is_none())
            .take(2)
            .collect();
        if names.len() < 2 {
            println!(
                "only {} roster fighter(s) without a card: names the registry does not know fill in",
                names.len()
            );
            for unknown in ["NOT A FIGHTER", "NOR THIS ONE"] {
                assert!(card(unknown).is_none() && !crate::roster().iter().any(|f| f.name == unknown));
                if names.len() < 2 {
                    names.push(unknown);
                }
            }
        }
        let (l, r) = (names[0], names[1]);
        // the name forms the drafts use, on a fixed example
        assert_eq!(
            (sentence_case("ALTO SAX"), name_tag("ALTO SAX")),
            ("Alto sax".to_string(), "#altosax".to_string())
        );
        let (ls, rs, lt, rt) = (sentence_case(l), sentence_case(r), name_tag(l), name_tag(r));
        let c = draft_captions(l, r);
        // no emoji and no tags of their own
        assert_eq!(c.caption, format!("{ls} vs {rs} Comment the next match-up \u{1F447}"));
        assert_eq!(
            c.instagram,
            format!("#ballfight {lt} {rt} #musicmemes #musician #pixelart #lowpoly #gamedev #satisfying #music")
        );
        assert_eq!(c.shorts[2], format!("#shorts #ballfight {lt} {rt} #music #pixelart"));
    }
}
