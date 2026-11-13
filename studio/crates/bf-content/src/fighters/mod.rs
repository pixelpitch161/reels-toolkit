//! The fighters: one folder per kit fighter (`<slug>/`, the kit's `name.lower().replace(' ', '_')`). Each holds
//! its instrument voice (`voice.rs`), its part in the score (`sound.rs`), its look (`look.rs`), its card
//! (`card.rs`) and what the look and the part both derive from the recording (`state.rs`), and offers the same
//! entry points from its `mod.rs`: `look()`, `voice()`, `sound(k, ff)`, `card()`, `MESH`, `MUSIC_FEEDS_FIGHT`
//! and `EXTRA_LINES`. The twenty kit fighters' folders are filled (milestone 3); the ten older fighters' folders
//! ([`OLDER`], milestone 4) are stubs whose entry points say "none" until their port fills them. [`entry`] is the
//! one place that names every folder, so a port changes only its own folder.
//!
//! [`shared`] holds the helpers several fighters' looks and voices call (milestone 3's Task 4; the older looks',
//! milestone 4's Task 4: [`shared::older`]).

use std::sync::Arc;

use bf_core::fightfile::FightFile;
use bf_picture::look::{DefaultLook, Look};
use bf_sound::traits::Sound;
use bf_sound::voice::Voice;

use crate::cards::FighterCard;

pub mod shared;

pub mod acoustic;
pub mod alto_sax;
pub mod bass_clarinet;
pub mod bass_drum;
pub mod bassoon;
pub mod cello;
pub mod clarinet;
pub mod contrabass;
pub mod electric;
pub mod euphonium;
pub mod flute;
pub mod horn;
pub mod oboe;
pub mod organ;
pub mod piano;
pub mod sousaphone;
pub mod tenor_sax;
pub mod trombone;
pub mod trumpet;
pub mod tuba;

// the older fighters (milestone 4)
pub mod accordion;
pub mod bagpipes;
pub mod bari_sax;
pub mod drums;
pub mod harp;
pub mod synth;
pub mod turntables;
pub mod ukulele;
pub mod viola;
pub mod violin;

/// The ten older fighters (spec section 10, plan m4 Table O1), in alphabetical order: each has its folder here, a
/// stub until its port, offered only once a register task lists it in `PORTED`.
pub const OLDER: [&str; 10] = [
    "ACCORDION",
    "BAGPIPES",
    "BARI SAX",
    "DRUMS",
    "HARP",
    "SYNTH",
    "TURNTABLES",
    "UKULELE",
    "VIOLA",
    "VIOLIN",
];

/// What one fighter's folder offers: its `mod.rs`'s entry points.
#[derive(Clone, Copy)]
pub struct Entry {
    /// `look()`: its look, or `None` while not ported.
    pub look: fn() -> Option<Box<dyn Look>>,
    /// `voice()`: its instrument voice, or `None`.
    pub voice: fn() -> Option<Arc<dyn Voice>>,
    /// `sound(k, ff)`: its part on side `k` of the fight `ff`, or `None`.
    pub sound: fn(usize, &FightFile) -> Option<Box<dyn Sound>>,
    /// `card()`: its card, or `None`.
    pub card: fn() -> Option<&'static FighterCard>,
    /// `MESH`: its model file's text, or `None`.
    pub mesh: Option<&'static str>,
    /// `MUSIC_FEEDS_FIGHT`.
    pub music_feeds_fight: bool,
    /// `EXTRA_LINES`.
    pub extra_lines: &'static [&'static str],
}

/// The entry points of one fighter's folder.
macro_rules! entry {
    ($m:ident) => {
        Entry {
            look: $m::look,
            voice: $m::voice,
            sound: $m::sound,
            card: $m::card,
            mesh: $m::MESH,
            music_feeds_fight: $m::MUSIC_FEEDS_FIGHT,
            extra_lines: $m::EXTRA_LINES,
        }
    };
}

/// The folder of the fighter `fighter` (its name in the roster, such as `BASS CLARINET`, or one of the [`OLDER`]
/// ten), or `None` for a name that is not one of the thirty (the retired ones, another spelling).
pub fn entry(fighter: &str) -> Option<Entry> {
    Some(match fighter {
        "ACOUSTIC" => entry!(acoustic),
        "ALTO SAX" => entry!(alto_sax),
        "BASS CLARINET" => entry!(bass_clarinet),
        "BASS DRUM" => entry!(bass_drum),
        "BASSOON" => entry!(bassoon),
        "CELLO" => entry!(cello),
        "CLARINET" => entry!(clarinet),
        "CONTRABASS" => entry!(contrabass),
        "ELECTRIC" => entry!(electric),
        "EUPHONIUM" => entry!(euphonium),
        "FLUTE" => entry!(flute),
        "HORN" => entry!(horn),
        "OBOE" => entry!(oboe),
        "ORGAN" => entry!(organ),
        "PIANO" => entry!(piano),
        "SOUSAPHONE" => entry!(sousaphone),
        "TENOR SAX" => entry!(tenor_sax),
        "TROMBONE" => entry!(trombone),
        "TRUMPET" => entry!(trumpet),
        "TUBA" => entry!(tuba),
        "ACCORDION" => entry!(accordion),
        "BAGPIPES" => entry!(bagpipes),
        "BARI SAX" => entry!(bari_sax),
        "DRUMS" => entry!(drums),
        "HARP" => entry!(harp),
        "SYNTH" => entry!(synth),
        "TURNTABLES" => entry!(turntables),
        "UKULELE" => entry!(ukulele),
        "VIOLA" => entry!(viola),
        "VIOLIN" => entry!(violin),
        _ => return None,
    })
}

/// The instrument voice of the fighter `fighter` (its name in the roster, such as `FLUTE`), or `None` for a
/// fighter whose voice is not ported. Every fighter the app offers ([`crate::PORTED`]) has one.
pub fn voice(fighter: &str) -> Option<Arc<dyn Voice>> {
    entry(fighter).and_then(|e| (e.voice)())
}

/// The part in the score of the fighter `fighter` on side `k` (0 is the left corner) of the fight `ff`: the
/// kit's `fighters.sound(name, M, k)`. `None` for a fighter whose part is not ported. Every fighter the app
/// offers ([`crate::PORTED`]) has one.
pub fn sound(fighter: &str, k: usize, ff: &FightFile) -> Option<Box<dyn Sound>> {
    entry(fighter).and_then(|e| (e.sound)(k, ff))
}

/// The look of the fighter `fighter` (its name in the roster, such as `FLUTE`): the kit's
/// `fighters.look(name, P, k)`. The picture hands the look its side and the run in
/// [`Look::setup`]. As in the kit, a fighter whose look is not ported draws as the bare ball
/// ([`DefaultLook`]).
pub fn look_for(fighter: &str) -> Box<dyn Look> {
    entry(fighter)
        .and_then(|e| (e.look)())
        .unwrap_or_else(|| Box::new(DefaultLook))
}

/// The card of the fighter `fighter`, or `None` while it has none.
pub fn card(fighter: &str) -> Option<&'static FighterCard> {
    entry(fighter).and_then(|e| (e.card)())
}

/// The model file of the fighter `fighter` (`data/models/<NAME>.json`, compiled in), or `None` while its look is
/// not ported.
pub fn mesh_json(fighter: &str) -> Option<&'static str> {
    entry(fighter).and_then(|e| e.mesh)
}

/// Whether the fighter's music settings feed its fight (PIANO's roll, once it is ported).
pub fn music_feeds_fight(fighter: &str) -> bool {
    entry(fighter).is_some_and(|e| e.music_feeds_fight)
}

/// The extra lines the fighter's part declares beside its own (HORN: `HORN THEME`); empty for every other.
pub fn extra_lines(fighter: &str) -> &'static [&'static str] {
    entry(fighter).map_or(&[], |e| e.extra_lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PORTED, RETIRED, roster};

    #[test]
    fn a_look_made_for_another_side_takes_that_fighters_numbers() {
        use bf_core::recording::Recording;
        // the episode's picture: FLUTE on side 0, TROMBONE on side 1
        let ff = FightFile::parse(include_str!("../../../../fights/flute_vs_trombone.toml")).expect("parses");
        let rec = Recording::read(&bf_testdata::reference("fights/flute_vs_trombone/ep392.bfrec")).expect("ep392");
        let pic = crate::build_picture(Arc::new(rec), &ff).expect("the picture");
        let p = pic.ctx();
        // for its own side, setup_as with its own numbers is setup
        let (mut a, mut b) = (trombone::TromboneLook::default(), trombone::TromboneLook::default());
        a.setup(p, 1);
        b.setup_as(p, 1, &p.cfg[1]);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let (mut a, mut b) = (flute::FluteLook::new(), flute::FluteLook::new());
        a.setup(p, 0);
        b.setup_as(p, 0, &p.cfg[0]);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        // EUPHONIUM's dup: TROMBONE's look made for side 0 with TROMBONE's numbers keeps side 0 (its events) and
        // TROMBONE's PEDAL TONE radius; made for side 0 with side 0's numbers it has FLUTE's PICCOLO radius
        let (radius, flute_radius) = (p.cfg[1].low.get("radius"), p.cfg[0].low.get("radius"));
        assert!(
            radius.is_some() && radius != flute_radius,
            "{radius:?} {flute_radius:?}"
        );
        let mut dup = trombone::TromboneLook::default();
        dup.setup_as(p, 0, &p.cfg[1]);
        let text = format!("{dup:?}");
        assert!(text.starts_with("TromboneLook { k: 0, thr_t: [],"), "{text}");
        assert!(text.ends_with(&format!("low_radius: {radius:?} }}")), "{text}");
        let mut own = trombone::TromboneLook::default();
        own.setup(p, 0);
        assert!(
            format!("{own:?}").ends_with(&format!("low_radius: {flute_radius:?} }}")),
            "{own:?}"
        );
        // FLUTE's look for side 1 with FLUTE's numbers: its roll, rise and PICCOLO are FLUTE's, its side is 1
        let mut dup = flute::FluteLook::new();
        dup.setup_as(p, 1, &p.cfg[0]);
        let mut own = flute::FluteLook::new();
        own.setup(p, 0);
        let field = |l: &flute::FluteLook, name: &str| {
            let t = format!("{l:?}");
            let i = t.find(&format!(" {name}: ")).expect("the field");
            t[i..].split(", ").next().expect("a value").to_string()
        };
        for name in ["roll", "rise", "low"] {
            assert_eq!(field(&dup, name), field(&own, name), "{name}");
        }
        assert_eq!(field(&dup, "k"), " k: 1");
    }

    #[test]
    fn the_ported_fighters_have_their_looks() {
        // kit/fighters/flute.py, `class Look`: its glass, its flare scale and its shatter parts.
        let flute = look_for("FLUTE");
        assert_eq!(flute.glass(), [150.0, 160.0, 196.0]);
        assert_eq!(flute.flare_scale(), 0.85);
        assert_eq!(flute.shards().len(), 13);
        assert!(flute.sprite(16, 0.0, 32.0).is_some() && flute.title_closeup(44).is_some());
        // kit/fighters/trombone.py, `class Look`: the base class's glass, its own shatter parts, a smaller sprite.
        let trombone = look_for("TROMBONE");
        assert_eq!(trombone.glass(), DefaultLook.glass());
        assert_eq!(trombone.shards().len(), 14);
        assert_eq!(trombone.sprite_k(), 0.80);
        assert_eq!(trombone.spark_flash(), ["blade", "slam"]);
        // Every fighter the app offers has a look of its own: its shatter parts tell it from the bare ball.
        for name in PORTED {
            assert!(!look_for(name).shards().is_empty(), "{name}");
        }
        // Anything else that is not ported draws as the bare ball, as the kit does.
        for name in RETIRED.iter().chain(&["flute", "", "KAZOO"]) {
            let look = look_for(name);
            assert_eq!(look.glass(), DefaultLook.glass(), "{name:?}");
            assert!(
                look.shards().is_empty() && look.sprite(16, 0.0, 0.0).is_none(),
                "{name:?}"
            );
        }
    }

    #[test]
    fn every_kit_fighter_has_a_folder() {
        // every roster fighter has its folder, and a fighter the app offers has everything in it; a folder a port
        // has filled before the register task offers it may hold anything (nothing here names its state)
        assert_eq!(roster().len(), 20);
        for f in roster() {
            let e = entry(&f.name).unwrap_or_else(|| panic!("{} has no folder", f.name));
            if PORTED.contains(&f.name.as_str()) {
                assert!((e.look)().is_some(), "{} is offered without a look", f.name);
                assert!((e.voice)().is_some(), "{} is offered without a voice", f.name);
                assert!((e.card)().is_some(), "{} is offered without a card", f.name);
                assert!(e.mesh.is_some(), "{} is offered without a model", f.name);
            }
            // the dispatch says what the folder says
            assert_eq!(mesh_json(&f.name), e.mesh, "{}", f.name);
            assert_eq!(music_feeds_fight(&f.name), e.music_feeds_fight, "{}", f.name);
            assert_eq!(extra_lines(&f.name), e.extra_lines, "{}", f.name);
            assert_eq!(card(&f.name).is_some(), (e.card)().is_some(), "{}", f.name);
        }
        for name in RETIRED.iter().chain(&["flute", "", "KAZOO", "NOT A FIGHTER"]) {
            assert!(
                entry(name).is_none() && card(name).is_none() && mesh_json(name).is_none(),
                "{name:?}"
            );
            assert!(!music_feeds_fight(name) && extra_lines(name).is_empty(), "{name:?}");
        }
    }

    /// Every older fighter has its folder (milestone 4, Task 4). While a folder is a stub it says "none" everywhere
    /// and the picture draws the fighter as the bare ball; none of the ten feeds its fight from the music, and a
    /// fighter is offered only with everything in its folder.
    #[test]
    fn every_older_fighter_has_a_folder() {
        let names: Vec<&str> = roster().iter().map(|f| f.name.as_str()).collect();
        let mut sorted = OLDER;
        sorted.sort_unstable();
        assert_eq!(sorted, OLDER);
        for name in OLDER {
            assert!(!RETIRED.contains(&name), "{name}");
            assert!(!names.contains(&name), "{name} is in the kit roster");
            let e = entry(name).unwrap_or_else(|| panic!("{name} has no folder"));
            assert!(!e.music_feeds_fight && e.extra_lines.is_empty(), "{name}");
            assert_eq!(mesh_json(name), e.mesh, "{name}");
            assert_eq!(card(name).is_some(), (e.card)().is_some(), "{name}");
            if PORTED.contains(&name) {
                assert!(
                    (e.look)().is_some() && (e.voice)().is_some() && (e.card)().is_some(),
                    "{name}"
                );
                assert!(e.mesh.is_some(), "{name}");
            } else if (e.look)().is_none() {
                assert_eq!(look_for(name).glass(), DefaultLook.glass(), "{name}");
                assert!(look_for(name).shards().is_empty(), "{name}");
            }
        }
    }

    #[test]
    fn every_ported_fighter_has_a_part() {
        let ff = FightFile::parse(include_str!("../../../../fights/flute_vs_trombone.toml")).expect("parses");
        for f in roster() {
            for k in 0..2 {
                let part = sound(&f.name, k, &ff);
                // offered implies a part; a folder filled before it is offered may have one too
                assert!(
                    part.is_some() || !PORTED.contains(&f.name.as_str()),
                    "{} has no part",
                    f.name
                );
                if let Some(part) = part {
                    assert_eq!(part.k(), k, "{}", f.name);
                }
            }
        }
        for name in RETIRED.iter().chain(&["flute", "", "KAZOO"]) {
            assert!(sound(name, 0, &ff).is_none(), "{name:?}");
        }
        // The class attributes of the two parts (kit/fighters/flute.py and trombone.py, `class Sound`).
        let flute = sound("FLUTE", 0, &ff).unwrap();
        assert_eq!(
            (flute.bus_gain(), flute.send(), flute.band()),
            (1.7, 0.45, (600.0, 3200.0))
        );
        let trombone = sound("TROMBONE", 1, &ff).unwrap();
        assert_eq!(
            (trombone.bus_gain(), trombone.send(), trombone.band()),
            (1.8, 0.2, (70.0, 700.0))
        );
        // #48: as a riff over someone else's bass the trombone is checked in the riff's band.
        let mut riff = ff.clone();
        riff.music.roles.insert("TROMBONE".into(), "riff".into());
        assert_eq!(sound("TROMBONE", 1, &riff).unwrap().band(), (120.0, 1500.0));
        riff.music.roles.clear();
        assert_eq!(sound("TROMBONE", 1, &riff).unwrap().band(), (70.0, 700.0));
    }

    #[test]
    fn every_ported_fighter_has_a_voice() {
        for name in PORTED {
            assert!(voice(name).is_some(), "{name} has no voice");
        }
        // The Pixel Pitch registers of pp_bridge's roster (not the fighters' own: TROMBONE fights from 40 to 65).
        assert_eq!(voice("FLUTE").unwrap().register(), (72, 93));
        assert_eq!(voice("TROMBONE").unwrap().register(), (45, 67));
        // Offered implies a voice; nothing outside the roster has one (not a retired name, not another spelling).
        for f in roster() {
            assert!(
                voice(&f.name).is_some() || !PORTED.contains(&f.name.as_str()),
                "{} has no voice",
                f.name
            );
        }
        for name in RETIRED.iter().chain(&["flute", "", "KAZOO"]) {
            assert!(voice(name).is_none(), "{name:?}");
        }
    }

    #[test]
    fn a_ported_fighter_plays_its_own_roster_voice() {
        // `kit.music.note` is asked for the roster's `pp` name (ELECTRIC's is an internal voice id, not its name).
        for name in PORTED {
            let cfg = roster().iter().find(|f| f.name == name).expect("in the roster");
            assert_eq!(voice(name).expect("a voice").name(), cfg.pp);
        }
    }
}
