//! What the engine offers, in the interface's terms: the roster with its cards, portraits and pixel-letter names,
//! the themes, and the description of a match-up (its pair rules, drafted captions, VS title card, or why it cannot
//! fight).

use std::sync::{Arc, OnceLock};

use bf_content::themes::OptionKind;
use bf_content::{FighterCard as ContentCard, Section as ContentSection};
use bf_core::fightfile::{FightFile, OptionValue};
use bf_core::roster::FighterCfg;

use super::convert::{self, Unusable};
use crate::model::Gen;
use crate::model::draft::{CaptionTexts, Draft};
use crate::model::roster::{
    AbilityLine, FighterCard, Home, KitPair, OptionSet, OptionShape, PairMusic, PixelMask, Roster, RosterEntry,
    Section, Sprite, ThemeEntry, ThemeOptionEntry,
};
use crate::model::take::Frame;
use crate::seam::Event;
use crate::wording;

/// The corner card's portrait (ruling M2-R32): a picture this many pixels square, holding the fighter's ball of
/// this radius, drawn as the VS title card draws its balls.
const PORTRAIT_SIDE: u32 = 52;
const PORTRAIT_BALL: f64 = 24.0;
/// The glow of the title card's ball at rest (`0.25 + 0.5 * flare`, with no flare).
const PORTRAIT_GLOW: f64 = 0.25;
/// Fighters whose model, at the portrait's size, reads better turned another way than the title card's view:
/// (name, yaw, roll, sprite radius). At the roster view the TROMBONE is an edge-on line at 52 px; turned 60 degrees
/// and rolled 45 its slide and bell both show. A fighter with a close-up of its own (the FLUTE's) keeps it.
const PORTRAIT_VIEWS: [(&str, f64, f64, u32); 1] = [("TROMBONE", 60.0, 45.0, 22)];

/// The match-up a new draft starts from.
const START: [&str; 2] = ["FLUTE", "TROMBONE"];

/// The roster as the interface shows it: every roster fighter in score order with its section, colour and name in
/// pixel letters; the card (weapon, abilities, range, music, portrait) and home config of each fighter the app
/// offers; the themes with their options; "VS" in pixel letters; the starting match-up, the series' kit pairs and
/// this app's version.
///
/// It is made once and kept (making it takes about a tenth of a second: each fighter's themes and portrait), so
/// every later call is a copy.
pub fn roster() -> Roster {
    static ROSTER: OnceLock<Roster> = OnceLock::new();
    ROSTER.get_or_init(make_roster).clone()
}

fn make_roster() -> Roster {
    let cfgs = bf_content::roster();
    let available = bf_content::available();
    let fighters = bf_content::shown_order()
        .into_iter()
        .filter_map(|name| {
            let cfg = cfgs.iter().find(|f| f.name == name)?;
            let section = section(bf_content::section(name)?);
            let on = available.contains(&name);
            Some(RosterEntry {
                name: name.to_string(),
                display: wording::display_name(name),
                section,
                color: cfg.color,
                available: on,
                music_feeds_fight: bf_content::music_feeds_fight(name),
                mask: mask(name),
                card: if on {
                    bf_content::card(name).map(|c| card(cfg, c))
                } else {
                    None
                },
                home: if on { home(name) } else { None },
            })
        })
        .collect();
    // each roster fighter with the themes it can be played in (against itself: see `refused_by`)
    let plays: Vec<(&str, Vec<&str>)> = cfgs
        .iter()
        .map(|f| (f.name.as_str(), bf_content::themes_for(&f.name, &f.name)))
        .collect();
    let themes = bf_content::theme_names()
        .into_iter()
        .filter_map(|name| {
            let t = bf_content::theme_card(name)?;
            Some(ThemeEntry {
                name: name.to_string(),
                display: t.display.to_string(),
                text: t.text.to_string(),
                key: t.key.to_string(),
                scale: t.scale.to_string(),
                chord_loops: t
                    .chord_loops
                    .iter()
                    .map(|l| l.iter().map(|c| c.to_string()).collect())
                    .collect(),
                bars: t.bars,
                refuses: refused_by(name, &plays),
                options: theme_options(name),
            })
        })
        .collect();
    let kit_pairs = bf_content::kit_pair_fights()
        .into_iter()
        .filter_map(|(_, ff)| {
            let fighters = [ff.fighters.first()?.clone(), ff.fighters.get(1)?.clone()];
            Some(KitPair {
                roles: convert::written_parts(ff, &fighters),
                handicap: [*ff.handicap.first()?, *ff.handicap.get(1)?],
                fighters,
                theme: ff.music.theme.clone(),
                chord_loop: ff.music.chord_loop.clone(),
            })
        })
        .collect();
    Roster {
        fighters,
        themes,
        vs: mask("VS"),
        start: START.map(str::to_string),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        kit_pairs,
    }
}

/// A fighter's home config (`bf_content::home_config`): its theme, chord loop and its own part there.
fn home(name: &str) -> Option<Home> {
    let pair = bf_content::home_config(name)?;
    let (_, ff) = bf_content::kit_pair_fights().into_iter().find(|(p, _)| *p == pair)?;
    Some(Home {
        theme: ff.music.theme.clone(),
        chord_loop: ff.music.chord_loop.clone(),
        role: ff
            .music
            .roles
            .get(name)
            .cloned()
            .unwrap_or_else(|| "melody".to_string()),
    })
}

/// A theme's options as the Music screen words them. The words are the app's: "The sequel" (the brass trailer's
/// second episode), "No bass drums" and "Bass drums back after the K.O." (the drumline's), "Plugged: {Name}" (the
/// stadium rock's plugged fighter). An option this app has no words for is shown by its name when it is on.
fn theme_options(theme: &str) -> Vec<ThemeOptionEntry> {
    let Some(t) = bf_content::themes::theme(theme) else {
        return Vec::new();
    };
    t.options
        .iter()
        .map(|o| {
            let words = |w: &str| Some(w.to_string());
            let shape = match (o.kind, o.key) {
                (OptionKind::Fighter, "plugged") => OptionShape::Fighter {
                    label: "Plugged".to_string(),
                },
                (OptionKind::Fighter, key) => OptionShape::Fighter {
                    label: wording::display_name(&key.replace('_', " ")),
                },
                (OptionKind::Bool, key) => {
                    let default = !matches!(o.default, Some(OptionValue::Bool(false)));
                    let (on, off) = match key {
                        "sequel" => (words("The sequel"), None),
                        "bass_drums" => (None, words("No bass drums")),
                        "after_ko" => (words("Bass drums back after the K.O."), None),
                        other => (Some(wording::display_name(&other.replace('_', " "))), None),
                    };
                    OptionShape::Switch { default, on, off }
                }
            };
            ThemeOptionEntry {
                key: o.key.to_string(),
                shape,
            }
        })
        .collect()
}

/// What the fight file `base` says about its music for a draft of `fighters`: its theme, chord loop, the parts of
/// the two fighters as it writes them, and the theme options it sets.
pub fn pair_music(base: &FightFile, fighters: &[String; 2]) -> PairMusic {
    PairMusic {
        theme: base.music.theme.clone(),
        chord_loop: base.music.chord_loop.clone(),
        roles: convert::written_parts(base, fighters),
        options: base
            .music
            .theme_options
            .iter()
            .filter_map(|(key, value)| {
                let set = match value {
                    OptionValue::Bool(b) => OptionSet::Switch(*b),
                    OptionValue::Str(name) => OptionSet::Fighter(name.clone()),
                    OptionValue::Int(_) | OptionValue::Float(_) => return None,
                };
                Some((key.clone(), set))
            })
            .collect(),
    }
}

/// The roster fighters the theme `theme` cannot be played with (ruling M3-R2), in the roster's order. Whether a
/// theme fits a pair is a question about each fighter's part alone (its bed hooks), so a fighter is refused when
/// [`bf_content::themes_for`] leaves the theme out of the fighter against itself; the tests prove this gives
/// `themes_for` for every pair.
fn refused_by(theme: &str, plays: &[(&str, Vec<&str>)]) -> Vec<String> {
    plays
        .iter()
        .filter(|(_, themes)| !themes.contains(&theme))
        .map(|(name, _)| name.to_string())
        .collect()
}

/// The interface's section of a content section.
fn section(s: ContentSection) -> Section {
    match s {
        ContentSection::Woodwinds => Section::Woodwinds,
        ContentSection::Brass => Section::Brass,
        ContentSection::Strings => Section::Strings,
        ContentSection::Keys => Section::Keys,
        ContentSection::Percussion => Section::Percussion,
    }
}

/// `text` in the series' pixel font at scale 1: a pixel is on where the font's mask is not 0.
fn mask(text: &str) -> PixelMask {
    let m = bf_picture::font::text_mask(text, 1);
    PixelMask {
        w: m.w,
        h: m.h,
        on: m.data.iter().map(|&v| v != 0).collect(),
    }
}

/// The corner card of an offered fighter.
fn card(cfg: &FighterCfg, c: &ContentCard) -> FighterCard {
    let ability = |tag: &str, name: &str, text: &str| AbilityLine {
        tag: tag.to_string(),
        name: wording::display_name(name),
        text: text.to_string(),
        key: name.to_string(),
    };
    let (lo, hi) = cfg.register;
    FighterCard {
        weapon: c.weapon.to_string(),
        text: c.text.to_string(),
        abilities: [
            ability("LV2", &cfg.unlock1.name, c.abilities[0]),
            ability("LV3", &cfg.unlock2.name, c.abilities[1]),
            ability("LOW", &cfg.low.name, c.abilities[2]),
        ],
        range: format!("{} to {}", bf_content::note_name(lo), bf_content::note_name(hi)),
        usually_plays: c.usually_plays.to_string(),
        colour_name: c.colour.to_string(),
        music_title: c.music_title.to_string(),
        music_text: c.music_text.to_string(),
        can_play_bass: c.can_play_bass,
        portrait: ball_portrait(cfg),
    }
}

/// The fighter's ball with its instrument inside, as the VS title card draws it (`draw_big_ball`, with the same
/// close-up: the look's own, else the model at its roster view's yaw and roll at 0.8 of the ball's radius, or at
/// its `PORTRAIT_VIEWS` entry; the
/// look's glass and the fighter's colour), at rest, on a transparent square of `PORTRAIT_SIDE`. `None` for a
/// fighter without a model or a view.
fn ball_portrait(cfg: &FighterCfg) -> Option<Sprite> {
    let mesh = bf_content::mesh(&cfg.name)?;
    let view: Vec<f64> = cfg
        .raw
        .get("view")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let [yaw, _, roll] = view[..] else {
        return None;
    };
    let look = bf_content::look_for(&cfg.name);
    let (yaw, roll, r) = PORTRAIT_VIEWS
        .iter()
        .find(|(name, ..)| *name == cfg.name)
        .map_or((yaw, roll, (PORTRAIT_BALL * 0.8) as u32), |&(_, y, rl, r)| (y, rl, r));
    let closeup = look
        .title_closeup(r)
        .unwrap_or_else(|| bf_picture::sprite::render_sprite(&mesh, r, yaw, roll));
    Some(ball(&closeup, look.glass(), cfg.color))
}

/// A ball of `PORTRAIT_BALL` on a transparent square of `PORTRAIT_SIDE` with `closeup` inside, its glass `glass`
/// and its rim `color` (see [`ball_portrait`]).
fn ball(closeup: &bf_picture::img::Img<u8>, glass: [f64; 3], color: [u8; 3]) -> Sprite {
    use bf_picture::img::Img as Pic;
    let side = PORTRAIT_SIDE as usize;
    let mut layer: Pic<f64> = Pic::new(side, side, 3);
    let mut alpha: Pic<f64> = Pic::new(side, side, 1);
    let centre = (PORTRAIT_SIDE / 2) as i32;
    bf_picture::title::panels::draw_big_ball(
        &mut layer,
        &mut alpha,
        closeup,
        glass,
        f64::from(centre),
        f64::from(centre),
        PORTRAIT_BALL,
        PORTRAIT_GLOW,
        color.map(f64::from),
    );
    let rgba: Vec<u8> = layer
        .data
        .chunks(3)
        .zip(&alpha.data)
        .flat_map(|(rgb, &a)| {
            let px = |v: f64| v.clamp(0.0, 255.0).round() as u8;
            [px(rgb[0]), px(rgb[1]), px(rgb[2]), px(a * 255.0)]
        })
        .collect();
    Sprite {
        w: side,
        h: side,
        rgba: rgba.into(),
    }
}

/// What the Match-up screen shows about the draft's fighters, for generation `gen`: the pair rules in words (a rule
/// this app has no words for is shown by its key), the drafted captions and the VS title card when the two can
/// fight; otherwise no rules, empty captions, no card and the problem in words ([`cannot_fight`]).
///
/// The card is drawn at the tempo of the draft being described, and the interface keeps it until the fighters or
/// their order change: it is not redrawn on a tempo change (controller ruling M2-R28), since a take's own frames and
/// the video's cover come from the real fight file. A `title_preview` that fails leaves the card absent on purpose,
/// with no problem: the fight itself can still be used.
///
/// Drawing the card takes about 40 ms, so this runs on a thread of its own, never on the coordinator.
pub fn describe(r#gen: Gen, draft: &Draft) -> Event {
    let [left, right] = [draft.fighters[0].as_str(), draft.fighters[1].as_str()];
    match convert::build(draft) {
        Ok(ff) => Event::FightInfo {
            music: convert::base_of(draft)
                .ok()
                .map(|base| pair_music(&base, &draft.fighters)),
            r#gen,
            pair_rules: ff
                .pair_rules
                .iter()
                .filter(|(_, on)| **on != bf_core::event::Value::Bool(false))
                .map(|(key, _)| bf_content::pair_rule_text(key).map_or_else(|| key.clone(), str::to_string))
                .collect(),
            captions: convert::caption_texts(&bf_content::draft_captions(left, right)),
            title_card: bf_content::title_preview(&ff).ok().map(|img| Frame {
                w: img.w as u32,
                h: img.h as u32,
                rgb: Arc::from(img.data),
            }),
            problem: None,
        },
        Err(why) => cannot_fight(
            r#gen,
            match why {
                Unusable::NoMusic => convert::no_music_problem(&draft.fighters),
                Unusable::Other(text) => text,
            },
        ),
    }
}

/// The description of a pair that cannot fight, for generation `gen`: no rules, empty captions, no card, and
/// `problem`, so the interface offers no balance and no search for it.
pub fn cannot_fight(r#gen: Gen, problem: String) -> Event {
    Event::FightInfo {
        r#gen,
        pair_rules: Vec::new(),
        captions: CaptionTexts {
            instagram: String::new(),
            tiktok: String::new(),
            shorts: String::new(),
        },
        title_card: None,
        problem: Some(problem),
        music: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::tests::starting_draft;
    use crate::seam::sample;

    #[test]
    fn the_real_roster() {
        let r = roster();
        assert_eq!(r.fighters.len(), 20);
        let names: Vec<&str> = r.fighters.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, bf_content::shown_order());
        let on: Vec<&str> = r
            .fighters
            .iter()
            .filter(|f| f.available)
            .map(|f| f.name.as_str())
            .collect();
        // every fighter the registry offers, in the score order the roster is shown in
        let offered: Vec<&str> = bf_content::shown_order()
            .into_iter()
            .filter(|n| bf_content::PORTED.contains(n))
            .collect();
        assert_eq!(on, offered);
        assert_eq!(on.len(), bf_content::PORTED.len());
        assert!(on.contains(&"FLUTE") && on.contains(&"TROMBONE"));
        for f in &r.fighters {
            assert!(!bf_core::fightfile::RETIRED.contains(&f.name.as_str()), "{}", f.name);
            assert_eq!(f.card.is_some(), f.available, "{}", f.name);
            assert_eq!(f.display, wording::display_name(&f.name));
            assert_eq!(f.mask.on.len(), f.mask.w * f.mask.h);
            assert_eq!(f.mask.h, 7);
            // what the fighter's own folder declares (`MUSIC_FEEDS_FIGHT`: PIANO's roll), not a fixed list
            let declared = bf_content::fighters::entry(&f.name).is_some_and(|e| e.music_feeds_fight);
            assert_eq!(f.music_feeds_fight, declared, "{}", f.name);
        }
        let get = |name: &str| r.fighters.iter().find(|f| f.name == name).expect(name);
        // and the facts known: FLUTE's and TROMBONE's music never feeds their fights; PIANO's roll does
        assert!(!get("FLUTE").music_feeds_fight && !get("TROMBONE").music_feeds_fight);
        assert!(
            get("PIANO").available && get("PIANO").music_feeds_fight,
            "PIANO's roll is its music"
        );
        let flute = get("FLUTE");
        assert_eq!((flute.section, flute.display.as_str()), (Section::Woodwinds, "Flute"));
        let fc = flute.card.as_ref().expect("the flute's card");
        assert_eq!(fc.weapon, "Snake charmer");
        assert_eq!(fc.range, "C5 to A6");
        let abilities: Vec<(&str, &str)> = fc.abilities.iter().map(|a| (a.tag.as_str(), a.name.as_str())).collect();
        assert_eq!(
            abilities,
            [("LV2", "Flutter tongue"), ("LV3", "Snake dance"), ("LOW", "Piccolo")]
        );
        assert_eq!(fc.abilities[0].key, "FLUTTER TONGUE");
        assert_eq!(fc.abilities[0].text, "Twice as fast for 8 beats");
        assert_eq!(
            (fc.usually_plays.as_str(), fc.colour_name.as_str()),
            ("Melody", "Silver")
        );
        assert!(!fc.can_play_bass);
        let portrait = fc.portrait.as_ref().expect("the flute's portrait");
        assert_eq!((portrait.w, portrait.h, portrait.rgba.len()), (52, 52, 52 * 52 * 4));
        assert!(
            portrait.rgba.chunks(4).any(|p| p[3] == 255),
            "the portrait shows something"
        );
        let trombone = get("TROMBONE");
        assert_eq!(trombone.section, Section::Brass);
        let tc = trombone.card.as_ref().expect("the trombone's card");
        assert_eq!(tc.range, "E2 to F4");
        assert!(tc.can_play_bass);
        assert!(tc.portrait.is_some());
        assert_eq!(get("BASS DRUM").section, Section::Percussion);
        // every theme the registry offers, the big top (the starting pair's) first
        let names: Vec<&str> = r.themes.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, bf_content::theme_names());
        assert_eq!(names.len(), 14);
        let t = &r.themes[0];
        assert_eq!(
            (t.name.as_str(), t.display.as_str(), t.bars),
            ("big_top", "Big top", (24, 42))
        );
        assert_eq!((t.key.as_str(), t.scale.as_str()), ("E", "harmonic minor"));
        assert_eq!(t.chord_loops, [["Em", "Am", "B7", "Em", "C", "Am", "B7", "B7"]]);
        assert_eq!(r.start, ["FLUTE", "TROMBONE"]);
        assert_eq!(r.app_version, env!("CARGO_PKG_VERSION"));
    }

    /// Ruling M3-R2: the themes the interface offers a pair are the engine's `themes_for` that pair, for every
    /// ordered pair of offered fighters; the starting pair is not offered the two themes without a boom.
    #[test]
    fn the_roster_offers_each_pair_the_themes_it_can_play() {
        let r = roster();
        let offered: Vec<&str> = r
            .fighters
            .iter()
            .filter(|f| f.available)
            .map(|f| f.name.as_str())
            .collect();
        assert!(offered.len() >= 2);
        for &a in &offered {
            for &b in &offered {
                let got: Vec<&str> = r
                    .themes_for(&[a.to_string(), b.to_string()])
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect();
                assert_eq!(got, bf_content::themes_for(a, b), "{a} vs {b}");
            }
        }
        let start: Vec<&str> = r.themes_for(&r.start).iter().map(|t| t.name.as_str()).collect();
        assert!(!start.contains(&"heist") && !start.contains(&"dark_disco"), "{start:?}");
        assert_eq!(start[0], "big_top");
        let heist = r.themes.iter().find(|t| t.name == "heist").expect("heist is offered");
        let mut refuses = heist.refuses.clone();
        refuses.sort();
        assert_eq!(refuses, ["EUPHONIUM", "HORN", "ORGAN", "TROMBONE", "TUBA"]);
        assert!(
            r.themes
                .iter()
                .find(|t| t.name == "big_top")
                .unwrap()
                .refuses
                .is_empty()
        );
    }

    #[test]
    fn the_sample_font_is_the_series_font() {
        let r = roster();
        for name in ["FLUTE", "TROMBONE"] {
            let f = r.fighters.iter().find(|f| f.name == name).expect(name);
            assert_eq!(f.mask, sample::mask(name), "{name}");
        }
        assert_eq!(r.vs, sample::mask("VS"));
    }

    #[test]
    fn safe_zone_numbers_agree() {
        let tuple = |(a, b, c, d): (i32, i32, i32, i32)| [a, b, c, d];
        assert_eq!(crate::preview::scale::SAFE, tuple(bf_picture::config::SAFE));
        assert_eq!(
            crate::preview::scale::SAFE_BUTTONS,
            tuple(bf_picture::config::SAFE_BUTTONS)
        );
    }

    #[test]
    fn folder_names_agree() {
        let root = std::env::temp_dir().join(format!("bf-app-folder-names-{}", std::process::id()));
        assert!(!root.exists(), "{} is left from another run", root.display());
        for pair in [["FLUTE", "TROMBONE"], ["TROMBONE", "FLUTE"]] {
            let folder = bf_export::folder_name(pair, 7, &root);
            let last = folder.file_name().and_then(|n| n.to_str()).expect("a folder name");
            assert_eq!(last, wording::folder_stem(&pair.map(str::to_string)));
        }
    }

    #[test]
    fn a_fight_is_described() {
        let d = starting_draft();
        let Event::FightInfo {
            r#gen,
            pair_rules,
            captions,
            title_card,
            problem,
            music: _,
        } = describe(3, &d)
        else {
            panic!("not a description");
        };
        assert_eq!(r#gen, 3);
        assert_eq!(pair_rules, ["A live lance knocks a risen cobra flat."]);
        assert_eq!(
            captions,
            convert::caption_texts(&bf_content::draft_captions("FLUTE", "TROMBONE"))
        );
        let card = title_card.expect("a title card");
        assert_eq!((card.w, card.h, card.rgb.len()), (360, 640, 360 * 640 * 3));
        assert_eq!(problem, None);
    }

    /// The description carries what the fight file says about its music: the kit config's parts and theme options
    /// for a kit pair (in the draft's order), the starter's for any other pair, and nothing for a pair that cannot
    /// fight.
    #[test]
    fn a_description_says_what_the_fight_file_plays() {
        let music_of = |d: &Draft| match describe(1, d) {
            Event::FightInfo { music, .. } => music,
            other => panic!("not a description: {other:?}"),
        };
        let mut d = starting_draft();
        let m = music_of(&d).expect("the starting pair's music");
        assert_eq!(m.theme, "big_top");
        assert_eq!(m.roles, ["melody", "bass"]);
        assert!(m.options.is_empty());
        let pair = |a: &str, b: &str| [a.to_string(), b.to_string()];
        let set = |k: &str, v: OptionSet| (k.to_string(), v);
        for (fighters, theme, roles, options) in [
            (
                pair("BASS DRUM", "TUBA"),
                "dark_drumline",
                ["drums", "bass"],
                vec![
                    set("after_ko", OptionSet::Switch(true)),
                    set("bass_drums", OptionSet::Switch(false)),
                ],
            ),
            (
                pair("ACOUSTIC", "ELECTRIC"),
                "stadium_rock",
                ["rhythm", "riff"],
                vec![set("plugged", OptionSet::Fighter("ELECTRIC".to_string()))],
            ),
            (
                pair("HORN", "TROMBONE"),
                "brass_trailer",
                ["melody", "riff"],
                vec![set("sequel", OptionSet::Switch(true))],
            ),
        ] {
            d.fighters = fighters.clone();
            let m = music_of(&d).unwrap_or_else(|| panic!("{fighters:?}"));
            assert_eq!(
                (m.theme.as_str(), m.roles.clone(), m.options),
                (theme, roles.map(str::to_string), options)
            );
        }
        // a pair with no kit config: the left fighter's home config, and the parts of the starter's rule
        d.fighters = pair("CELLO", "FLUTE");
        let m = music_of(&d).expect("a crossover's music");
        let starter = bf_content::starter("CELLO", "FLUTE").expect("its starter");
        assert_eq!(m.theme, starter.music.theme);
        assert_eq!(m.chord_loop, starter.music.chord_loop);
        assert_eq!(m.roles, ["melody", "line"]);
        d.fighters[1] = "CELLO".to_string();
        assert_eq!(music_of(&d), None, "a fighter against itself cannot fight");
    }

    /// The roster knows how every new match-up starts exactly as the engine's starter makes it (theme, the place
    /// of its chord loop, handicap, parts), for all 380 ordered pairs of offered fighters, so the interface can set
    /// the draft at once when a fighter is picked.
    #[test]
    fn the_roster_starts_every_pair_as_the_engine_does() {
        let r = roster();
        let offered: Vec<String> = r
            .fighters
            .iter()
            .filter(|f| f.available)
            .map(|f| f.name.clone())
            .collect();
        assert_eq!(offered.len(), 20);
        assert_eq!(r.kit_pairs.len(), 18);
        let mut pairs = 0;
        for a in &offered {
            for b in &offered {
                if a == b {
                    continue;
                }
                pairs += 1;
                let fighters = [a.clone(), b.clone()];
                let ff = bf_content::starter(a, b).unwrap_or_else(|e| panic!("{a} vs {b}: {e}"));
                let theme = r
                    .themes
                    .iter()
                    .find(|t| t.name == ff.music.theme)
                    .expect("an offered theme");
                let place = theme
                    .chord_loops
                    .iter()
                    .position(|l| *l == ff.music.chord_loop)
                    .unwrap_or(theme.chord_loops.len());
                let want = crate::model::roster::PairStart {
                    theme: ff.music.theme.clone(),
                    chord_loop: place,
                    handicap: [ff.handicap[0], ff.handicap[1]],
                    roles: convert::written_parts(&ff, &fighters),
                };
                assert_eq!(r.start_of(&fighters), Some(want), "{a} vs {b}");
            }
        }
        assert_eq!(pairs, 380);
        // the kit's frozen handicaps, swapped with the sides
        let start = |a: &str, b: &str| r.start_of(&[a.to_string(), b.to_string()]).expect("a start");
        assert_eq!(start("FLUTE", "TROMBONE").handicap, [2.0, 0.4]);
        assert_eq!(start("TROMBONE", "FLUTE").handicap, [0.4, 2.0]);
        assert_eq!(start("CELLO", "FLUTE").handicap, [1.0, 1.0]);
    }

    /// The themes' options in the Music screen's words: those three kit themes have options, and each kit config
    /// that sets one reads as the plan's examples ("The sequel", "No bass drums", "Plugged: Electric").
    #[test]
    fn theme_options_are_in_words() {
        let r = roster();
        let with: Vec<&str> = r
            .themes
            .iter()
            .filter(|t| !t.options.is_empty())
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(with, ["brass_trailer", "dark_drumline", "stadium_rock"]);
        for t in &r.themes {
            let schema = bf_content::themes::theme(&t.name).expect("a theme").options;
            assert_eq!(t.options.len(), schema.len(), "{}", t.name);
        }
        let words = |pair: &str| {
            let (_, ff) = bf_content::kit_pair_fights()
                .into_iter()
                .find(|(p, _)| *p == pair)
                .expect(pair);
            let fighters = [ff.fighters[0].clone(), ff.fighters[1].clone()];
            let m = pair_music(ff, &fighters);
            let theme = r.themes.iter().find(|t| t.name == m.theme).expect("its theme");
            theme.option_words(&m.options, &fighters)
        };
        assert_eq!(words("horn_vs_trombone_rematch"), ["The sequel"]);
        assert!(words("trombone_vs_horn").is_empty());
        assert_eq!(
            words("bassdrum_vs_tuba"),
            ["No bass drums", "Bass drums back after the K.O."]
        );
        assert!(words("flute_vs_sousaphone").is_empty());
        assert_eq!(words("electric_vs_acoustic"), ["Plugged: Electric"]);
        // a fighter option left out is the left corner's fighter
        let rock = r
            .themes
            .iter()
            .find(|t| t.name == "stadium_rock")
            .expect("stadium rock");
        let pair = ["ACOUSTIC".to_string(), "ELECTRIC".to_string()];
        assert_eq!(rock.option_words(&[], &pair), ["Plugged: Acoustic"]);
        for words in r.themes.iter().flat_map(|t| t.option_words(&[], &pair)) {
            assert!(!words.contains('\u{2014}') && !words.contains('_'), "{words}");
        }
    }

    /// Ruling note (e) of Task 60: the roster takes about a tenth of a second to make, so it is made once.
    #[test]
    fn the_roster_is_made_once() {
        let first = roster();
        let asked = std::time::Instant::now();
        for _ in 0..20 {
            assert_eq!(roster(), first);
        }
        let took = asked.elapsed();
        // twenty copies take well under the time of making one (about 0.1 s)
        assert!(took < std::time::Duration::from_millis(500), "{took:?}");
    }

    #[test]
    fn a_pair_without_music_is_a_plain_problem() {
        // every pair of two kit fighters has music now (Task 59); a fighter against itself has none
        let mut d = starting_draft();
        d.fighters[1] = "FLUTE".to_string();
        let Event::FightInfo {
            r#gen,
            pair_rules,
            captions,
            title_card,
            problem,
            music: _,
        } = describe(5, &d)
        else {
            panic!("not a description");
        };
        assert_eq!(r#gen, 5);
        assert!(pair_rules.is_empty());
        assert_eq!(
            (captions.instagram, captions.tiktok, captions.shorts),
            (String::new(), String::new(), String::new())
        );
        assert!(title_card.is_none());
        assert_eq!(
            problem.as_deref(),
            Some(
                "No music is written for Flute vs Flute yet, so it cannot be balanced or rendered. Pick another pair."
            )
        );
        // any other refusal is a sentence too: a theme the pair cannot play (ruling M3-R2) in its own words
        d.fighters[1] = "TROMBONE".to_string();
        d.theme = "heist".to_string();
        let Event::FightInfo { problem, .. } = describe(6, &d) else {
            panic!("not a description");
        };
        assert_eq!(
            problem.as_deref(),
            Some(
                "This fight cannot be used: The theme: Heist has no boom (a big drum hit), which the TROMBONE's part plays; pick another theme."
            )
        );
        d.theme = "big_top".to_string();
        d.base = Some("format = 1".to_string());
        let Event::FightInfo { problem, .. } = describe(7, &d) else {
            panic!("not a description");
        };
        let text = problem.expect("a problem");
        assert!(
            text.starts_with("This fight cannot be used: ") && text.ends_with('.'),
            "{text}"
        );
    }
    #[test]
    fn a_portrait_is_the_fighters_ball_with_its_instrument() {
        let r = roster();
        for name in ["FLUTE", "TROMBONE"] {
            let f = r.fighters.iter().find(|f| f.name == name).expect("in the roster");
            let p = f.card.as_ref().and_then(|c| c.portrait.as_ref()).expect("a portrait");
            let px = |x: usize, y: usize| &p.rgba[(y * p.w + x) * 4..(y * p.w + x) * 4 + 4];
            // round: the corners are clear, the centre and the ball's edge are drawn
            assert_eq!(px(0, 0)[3], 0, "{name}");
            assert_eq!(px(51, 51)[3], 0, "{name}");
            assert_eq!(px(26, 26)[3], 255, "{name}");
            let drawn = p.rgba.chunks(4).filter(|q| q[3] == 255).count();
            // a disc of radius 24 or so: about 1,800 of the 2,704 pixels (the old sprite alone drew a thin line)
            assert!((1_500..=2_300).contains(&drawn), "{name}: {drawn}");
            // the rim is in the fighter's colour: some drawn pixel is close to it
            let near = |q: &[u8]| (0..3).all(|c| q[c].abs_diff(f.color[c]) < 40);
            assert!(p.rgba.chunks(4).any(|q| q[3] == 255 && near(q)), "{name}'s colour");
            // the instrument is inside: against the same ball with nothing in it, many pixels differ
            let empty = ball(
                &bf_picture::img::Img::new(1, 1, 4),
                bf_content::look_for(name).glass(),
                f.color,
            );
            let differ = p
                .rgba
                .chunks(4)
                .zip(empty.rgba.chunks(4))
                .filter(|(a, b)| a != b)
                .count();
            // (the TROMBONE at the roster view showed 66; turned, 93; the FLUTE's close-up more)
            assert!(differ > 80, "{name}: {differ} pixels show the instrument");
        }
    }
}
